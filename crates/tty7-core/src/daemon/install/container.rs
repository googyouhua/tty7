//! Installing and reaching a tty7 server inside a running container.
//!
//! A container is treated as a remote host whose transport is the host's
//! `docker` CLI: every operation is `docker exec -i <ctr> …`, and the daemon
//! protocol rides that process's stdin/stdout over the **host's** docker socket.
//! The container needs no network route back to the GUI and exposes no listener,
//! so a `--network none` container works exactly like any other.
//!
//! This is the non-SSH twin of [`super::wsl`]: it implements the same
//! [`RemoteOps`] seam over a different transport, and reuses the generic
//! [`Installer`] — arch detection, checksums, the `$HOME/.local/share/tty7/bin`
//! layout, and the reuse notes — unchanged.

use std::io;
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::io::AsyncWriteExt as _;

use super::wsl::BundledServerBinary;
use super::{ExecOutput, Installer, RemoteOps, RemoteStat, install_confirm, shell_quote};

/// The runtime CLI every container operation goes through. It is resolved on
/// `PATH` of the machine running tty7 — the docker socket lives on the host and
/// is never mounted into the container.
pub const DOCKER_EXE: &str = "docker";

const MARKER: &str = "__tty7_container__";

const COMMAND_TIMEOUT: Duration = Duration::from_secs(60);
const LAUNCH_TIMEOUT: Duration = Duration::from_secs(30);
const PUT_TIMEOUT: Duration = Duration::from_secs(180);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContainerNameError {
    Empty,
    LeadingDash(String),
    Control(String),
    TooLong(usize),
}

impl std::fmt::Display for ContainerNameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => write!(f, "a container name cannot be empty"),
            Self::LeadingDash(name) => write!(
                f,
                "{name:?} cannot be a container name: a leading `-` would be read as a docker option"
            ),
            Self::Control(name) => write!(
                f,
                "{name:?} cannot be a container name: it contains a control character"
            ),
            Self::TooLong(len) => write!(f, "a container name of {len} bytes is not real"),
        }
    }
}

impl std::error::Error for ContainerNameError {}

impl From<ContainerNameError> for io::Error {
    fn from(e: ContainerNameError) -> io::Error {
        io::Error::new(io::ErrorKind::InvalidInput, e.to_string())
    }
}

const MAX_CONTAINER_NAME: usize = 255;

/// Refuse anything that could be read as a `docker` option or smuggle a second
/// command onto the argument line. A name or id is one argv word handed to
/// `docker exec`, so this is the whole of the injection surface.
pub fn validate_container(name: &str) -> Result<(), ContainerNameError> {
    if name.is_empty() {
        return Err(ContainerNameError::Empty);
    }
    if name.len() > MAX_CONTAINER_NAME {
        return Err(ContainerNameError::TooLong(name.len()));
    }
    if name.starts_with('-') {
        return Err(ContainerNameError::LeadingDash(name.to_string()));
    }
    if name.chars().any(|c| c.is_control()) {
        return Err(ContainerNameError::Control(name.to_string()));
    }
    Ok(())
}

/// `docker exec -i <ctr> …` — the argv for one operation or for the stdio link.
/// `-i` keeps stdin open (scripts and the daemon protocol are piped in); there
/// is deliberately no `-t`, so no terminal is allocated and stdout stays a
/// clean byte stream.
pub fn docker_args(container: &str, argv: &[&str]) -> Vec<String> {
    let mut args = vec!["exec".to_string(), "-i".to_string(), container.to_string()];
    args.extend(argv.iter().map(|a| (*a).to_string()));
    args
}

pub fn host_label(container: &str) -> String {
    format!("container:{container}")
}

/// The shell every operation runs through. `docker exec` hands argv to the
/// container without a shell, so the script itself is fed on stdin to `sh -s` —
/// exactly the shape [`super::wsl`] uses.
const HOME_SCRIPT: &str = concat!(
    "h=${HOME-}\n",
    // `docker exec` does not set HOME for a non-login shell the way an
    // interactive login does, so fall back to the account's entry in
    // /etc/passwd before giving up.
    "if [ -z \"$h\" ]; then\n",
    "u=$(id -un 2>/dev/null) || u=\n",
    "if [ -n \"$u\" ]; then\n",
    "while IFS=: read -r n _ _ _ _ home _; do\n",
    "if [ \"$n\" = \"$u\" ]; then h=\"$home\"; break; fi\n",
    "done < /etc/passwd 2>/dev/null\n",
    "fi\n",
    "fi\n",
    "printf '__tty7_container__ home=%s\\n' \"$h\"\n",
);

/// Where the server can live inside the container, asked only when `$HOME`
/// cannot hold an executable.
///
/// Each candidate is a *home-shaped* base; the probe tests
/// `<base>/.local/share/tty7/bin`, the directory the installer will actually
/// use, and it writes **and runs** a throwaway script there. Writability alone
/// says nothing about a noexec mount, which is the case this exists for.
///
/// The order is deliberate: the real home first (so an ordinary container never
/// relocates), the runtime directory next, then the conventional writable
/// temporary roots. `/tmp` is late because many containers mount it noexec
/// too; a stated failure beats a binary that cannot start.
const PLACE_SCRIPT: &str = concat!(
    "probe() {\n",
    "  d=$1\n",
    "  mkdir -p \"$d\" 2>/dev/null || return 1\n",
    "  [ -w \"$d\" ] || return 1\n",
    "  p=\"$d/.tty7-exec-probe.$$\"\n",
    "  printf '#!/bin/sh\\nexit 0\\n' > \"$p\" 2>/dev/null || return 1\n",
    "  chmod 700 \"$p\" 2>/dev/null\n",
    "  \"$p\" >/dev/null 2>&1 || { rm -f \"$p\"; return 1; }\n",
    "  rm -f \"$p\"\n",
    "  return 0\n",
    "}\n",
    "uid=$(id -u 2>/dev/null) || uid=0\n",
    "for b in \"${HOME:-}\" \"${XDG_RUNTIME_DIR:-}\" \"/run/user/$uid\" \"/dev/shm/tty7-$uid\" \"/tmp/tty7-$uid\" \"/var/tmp/tty7-$uid\"; do\n",
    "  case \"$b\" in /*) ;; *) continue ;; esac\n",
    "  if probe \"$b/.local/share/tty7/bin\"; then printf '__tty7_container__ base=%s\\n' \"$b\"; exit 0; fi\n",
    "done\n",
    "printf '__tty7_container__ noplace\\n'\n",
);

const SETTLE_TRIES: u32 = 25;
const SETTLE_STEP: &str = "0.2";
const SETTLE_GRACE: &str = "0.3";

pub(super) fn launch_settle(binary: &str) -> String {
    settle_script(binary, SETTLE_TRIES, SETTLE_STEP, SETTLE_GRACE)
}

fn settle_script(binary: &str, tries: u32, step: &str, grace: &str) -> String {
    let bin = shell_quote(binary);
    format!(
        "__tty7_settle=0\n\
         while [ \"$__tty7_settle\" -lt {tries} ]; do\n\
         if {bin} --stdio --bridge < /dev/null > /dev/null 2>&1; then break; fi\n\
         __tty7_settle=$((__tty7_settle + 1))\n\
         sleep {step}\n\
         done\n\
         sleep {grace}\n"
    )
}

fn stat_script(path: &str) -> String {
    let p = shell_quote(path);
    format!(
        "p={p}\n\
         if [ -d \"$p\" ]; then printf '{MARKER} stat=dir 0 0\\n'\n\
         elif [ -e \"$p\" ]; then\n\
         if [ -x \"$p\" ]; then m=755; else m=644; fi\n\
         s=$(wc -c < \"$p\" 2>/dev/null) || s=0\n\
         printf '{MARKER} stat=file %s %s\\n' \"$m\" \"$s\"\n\
         else printf '{MARKER} stat=none 0 0\\n'\n\
         fi\n"
    )
}

fn parse_stat(stdout: &str) -> Result<Option<RemoteStat>, String> {
    for line in stdout.lines() {
        let Some(rest) = marked(line, "stat=") else {
            continue;
        };
        let mut words = rest.split_whitespace();
        let kind = words.next().unwrap_or_default();
        let mode = words
            .next()
            .and_then(|m| u32::from_str_radix(m, 8).ok())
            .unwrap_or(0);
        let size = words
            .next()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(0);
        return Ok(match kind {
            "none" => None,
            "dir" => Some(RemoteStat {
                size: 0,
                mode: 0o755,
                is_dir: true,
            }),
            "file" => Some(RemoteStat {
                size,
                mode,
                is_dir: false,
            }),
            other => return Err(format!("the container answered stat={other:?}")),
        });
    }
    Err(format!(
        "the container did not answer the stat probe (got {:?})",
        truncate(stdout)
    ))
}

fn list_script(dir: &str) -> String {
    let d = shell_quote(dir);
    format!(
        "d={d}\n\
         if [ -d \"$d\" ]; then\n\
         printf '{MARKER} listing\\n'\n\
         ls -A -- \"$d\" 2>/dev/null | while IFS= read -r e; do printf '{MARKER} entry=%s\\n' \"$e\"; done\n\
         else printf '{MARKER} nolisting\\n'\n\
         fi\n"
    )
}

fn parse_list(stdout: &str) -> Result<Option<Vec<String>>, String> {
    let mut saw_dir = false;
    let mut entries = Vec::new();
    for line in stdout.lines() {
        let Some(rest) = marked(line, "") else {
            continue;
        };
        if rest == "nolisting" {
            return Ok(None);
        }
        if rest == "listing" {
            saw_dir = true;
        } else if let Some(name) = rest.strip_prefix("entry=") {
            entries.push(name.to_string());
        }
    }
    if saw_dir {
        Ok(Some(entries))
    } else {
        Err(format!(
            "the container did not answer the directory probe (got {:?})",
            truncate(stdout)
        ))
    }
}

/// The candidate the place probe accepted, if any. `noplace` is a real answer
/// (there is nowhere to install); silence or nonsense is an error.
fn parse_place(stdout: &str) -> Result<Option<String>, String> {
    for line in stdout.lines() {
        let Some(rest) = marked(line, "") else {
            continue;
        };
        if rest == "noplace" {
            return Ok(None);
        }
        if let Some(base) = rest.strip_prefix("base=") {
            if !base.starts_with('/') {
                return Err(format!(
                    "the container offered {base:?} as an install base, which is not absolute"
                ));
            }
            return Ok(Some(base.to_string()));
        }
    }
    Err(format!(
        "the container did not answer the install-location probe (got {:?})",
        truncate(stdout)
    ))
}

/// Turn the probe's answer into what [`Installer`] needs: `None` when the real
/// home is good, `Some(base)` when it must relocate, and an error naming the
/// home when there is nowhere usable at all.
fn relocation(home: &str, place: Option<String>) -> Result<Option<String>, String> {
    match place {
        Some(base) if base == home => Ok(None),
        Some(base) => Ok(Some(base)),
        None => Err(format!(
            "the home directory {home} and the usual runtime and temporary directories were \
             all unwritable or mounted noexec"
        )),
    }
}

fn marked<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    line.trim()
        .strip_prefix(MARKER)?
        .trim_start()
        .strip_prefix(key)
}

fn truncate(s: &str) -> String {
    let s = s.trim();
    if s.chars().count() <= 200 {
        return s.to_string();
    }
    s.chars().take(200).collect::<String>() + "…"
}

pub struct ContainerRemoteOps {
    container: String,
}

impl ContainerRemoteOps {
    pub fn new(container: impl Into<String>) -> Self {
        Self {
            container: container.into(),
        }
    }

    fn sh(&self, script: &str, timeout: Duration) -> Result<ExecOutput, String> {
        self.invoke(
            &["sh", "-s"],
            Some(script.as_bytes().to_vec()),
            false,
            timeout,
        )
    }

    fn sh_ok(&self, script: &str) -> Result<(), String> {
        let out = self.sh(script, COMMAND_TIMEOUT)?;
        if out.success() {
            Ok(())
        } else {
            Err(out.failure_reason())
        }
    }

    fn invoke(
        &self,
        argv: &[&str],
        input: Option<Vec<u8>>,
        discard_stdout: bool,
        timeout: Duration,
    ) -> Result<ExecOutput, String> {
        let args = docker_args(&self.container, argv);
        crate::daemon::ssh::SshManager::global()
            .handle()
            .block_on(async move { invoke_async(&args, input, discard_stdout, timeout).await })
    }
}

async fn invoke_async(
    args: &[String],
    input: Option<Vec<u8>>,
    discard_stdout: bool,
    timeout: Duration,
) -> Result<ExecOutput, String> {
    let mut cmd = tokio::process::Command::new(DOCKER_EXE);
    cmd.args(args)
        .stdin(if input.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(if discard_stdout {
            Stdio::null()
        } else {
            Stdio::piped()
        })
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    crate::core::proc::hide_console_tokio(&mut cmd);

    let mut child = cmd.spawn().map_err(|e| match e.kind() {
        io::ErrorKind::NotFound => format!(
            "the {DOCKER_EXE} CLI is not available on this machine, so containers cannot be used"
        ),
        _ => format!("could not start {DOCKER_EXE}: {e}"),
    })?;

    let stdin = child.stdin.take();
    let feed = async move {
        if let (Some(mut stdin), Some(bytes)) = (stdin, input) {
            stdin.write_all(&bytes).await?;
            stdin.flush().await?;
            stdin.shutdown().await?;
        }
        Ok::<(), io::Error>(())
    };

    let both = async { tokio::join!(feed, child.wait_with_output()) };
    let (written, waited) = tokio::time::timeout(timeout, both)
        .await
        .map_err(|_| format!("the container did not answer within {timeout:?}"))?;

    let write_err = written.err();
    let out = waited.map_err(|e| format!("{DOCKER_EXE} failed: {e}"))?;

    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    if let Some(e) = write_err
        && out.status.success()
    {
        return Err(format!("could not send input to the container: {e}"));
    }

    Ok(ExecOutput {
        status: out.status.code().map(|c| c as u32),
        stdout,
        stderr,
    })
}

impl RemoteOps for ContainerRemoteOps {
    fn home_dir(&self) -> Result<String, String> {
        let out = self.sh(HOME_SCRIPT, COMMAND_TIMEOUT)?;
        if !out.success() {
            return Err(out.failure_reason());
        }
        match out.stdout.lines().find_map(|l| marked(l, "home=")) {
            Some(home) if home.starts_with('/') => Ok(home.to_string()),
            Some(home) => Err(format!(
                "the container resolved its home to {home:?}, which is not absolute"
            )),
            None => Err(format!(
                "the container did not report a home directory (got {:?})",
                truncate(&out.stdout)
            )),
        }
    }

    fn install_home(&self) -> Result<Option<String>, String> {
        // Resolve home as the shell would first: `docker exec` can leave `$HOME`
        // unset, and the probe's first candidate has to be the real home so an
        // ordinary container never relocates. Feeding it back also means the
        // probe and the comparison agree on one spelling.
        let home = self.home_dir()?;
        let script = format!("HOME={}\n{PLACE_SCRIPT}", shell_quote(&home));
        let out = self.sh(&script, COMMAND_TIMEOUT)?;
        if !out.success() {
            return Err(out.failure_reason());
        }
        match relocation(&home, parse_place(&out.stdout)?)? {
            Some(base) => {
                log::info!(
                    "container: {home} cannot hold an executable tty7-server; installing under {base}"
                );
                Ok(Some(base))
            }
            None => Ok(None),
        }
    }

    fn run(&self, cmd: &str) -> Result<ExecOutput, String> {
        self.sh(cmd, COMMAND_TIMEOUT)
    }

    fn spawn_detached(&self, cmd: &str) -> Result<(), String> {
        self.sh(cmd, LAUNCH_TIMEOUT).map(|_| ())
    }

    fn launch_settle(&self, binary: &str) -> Option<String> {
        Some(launch_settle(binary))
    }

    fn stat(&self, path: &str) -> Result<Option<RemoteStat>, String> {
        let out = self.sh(&stat_script(path), COMMAND_TIMEOUT)?;
        if !out.success() {
            return Err(out.failure_reason());
        }
        parse_stat(&out.stdout)
    }

    fn mkdir(&self, path: &str) -> Result<(), String> {
        self.sh_ok(&format!("mkdir -p {}\n", shell_quote(path)))
    }

    fn chmod(&self, path: &str, mode: u32) -> Result<(), String> {
        self.sh_ok(&format!("chmod {mode:o} {}\n", shell_quote(path)))
    }

    fn put(&self, path: &str, bytes: &[u8]) -> Result<(), String> {
        // `tee` receives the path as one argv word, so nothing re-parses it.
        let out = self.invoke(&["tee", path], Some(bytes.to_vec()), true, PUT_TIMEOUT)?;
        if !out.success() {
            return Err(out.failure_reason());
        }
        match self.stat(path)? {
            Some(stat) if stat.size == bytes.len() as u64 => Ok(()),
            Some(stat) => Err(format!(
                "wrote {} bytes but the container reports {} at that path",
                bytes.len(),
                stat.size
            )),
            None => Err("the file was not there after writing it".to_string()),
        }
    }

    fn rename(&self, from: &str, to: &str) -> Result<(), String> {
        self.sh_ok(&format!(
            "mv -f {} {}\n",
            shell_quote(from),
            shell_quote(to)
        ))
    }

    fn remove_file(&self, path: &str) -> Result<(), String> {
        self.sh_ok(&format!("rm -f {}\n", shell_quote(path)))
    }

    fn list_dir(&self, path: &str) -> Result<Option<Vec<String>>, String> {
        let out = self.sh(&list_script(path), COMMAND_TIMEOUT)?;
        if !out.success() {
            return Err(out.failure_reason());
        }
        parse_list(&out.stdout)
    }
}

static INSTALL_LOCKS: Mutex<Vec<(String, Arc<Mutex<()>>)>> = Mutex::new(Vec::new());

fn install_lock(container: &str) -> Arc<Mutex<()>> {
    let mut locks = INSTALL_LOCKS
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some((_, lock)) = locks.iter().find(|(c, _)| c == container) {
        return lock.clone();
    }
    let lock = Arc::new(Mutex::new(()));
    locks.push((container.to_string(), lock.clone()));
    lock
}

/// Where a container's server was last proved to be, so the next pane can skip
/// the proving. The reasoning is [`super::wsl`]'s almost word for word — the
/// probe is several `docker exec` round trips, and the answer barely rots — with
/// one difference: a container can be **recreated** by a fresh `docker run`,
/// leaving the same name pointing at a new filesystem. Nothing here expires on a
/// timer, so that case is caught the way a deleted binary is: the router forgets
/// the container when a bridge closes without ever answering, and the next pane
/// proves it again.
static READY: Mutex<Vec<(String, Proved)>> = Mutex::new(Vec::new());

#[derive(Clone)]
struct Proved {
    binary: String,
    mismatch: Option<super::MismatchedRemoteDaemon>,
}

fn remembered(container: &str) -> Option<Proved> {
    READY
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .iter()
        .find(|(c, _)| c == container)
        .map(|(_, proved)| proved.clone())
}

fn remember(container: &str, proved: Proved) {
    let mut ready = READY
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    match ready.iter_mut().find(|(c, _)| c == container) {
        Some((_, known)) => *known = proved,
        None => ready.push((container.to_string(), proved)),
    }
}

/// Where this container's server was last proved to be, or `None` if the next
/// pane would have to go and ask. A hint for callers deciding whether a failure
/// is worth re-proving; the answer itself comes from [`ensure_container_server`].
pub fn remembered_container_server(container: &str) -> Option<String> {
    remembered(container).map(|proved| proved.binary)
}

/// Drop what we thought we knew about a container, so the next
/// [`ensure_container_server`] proves it again the long way.
pub fn forget_container_server(container: &str) {
    READY
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .retain(|(c, _)| c != container);
}

pub fn ensure_container_server(container: &str) -> io::Result<String> {
    validate_container(container)?;

    let lock = install_lock(container);
    let _held = lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(proved) = remembered(container) {
        if let Some(mismatch) = proved.mismatch {
            super::record_remote_mismatches(vec![mismatch]);
        }
        return Ok(proved.binary);
    }

    let ops = ContainerRemoteOps::new(container);
    let source = BundledServerBinary::discover();
    let confirm = install_confirm();
    let host = host_label(container);
    let report = Installer::with_source(&ops, &source, confirm.as_ref(), host.clone()).run()?;

    log::info!(
        "{host}: {} at {} ({}{})",
        if report.installed {
            "installed the bundled tty7-server"
        } else {
            "tty7-server already present"
        },
        report.paths.binary,
        if report.launched {
            "daemon launched"
        } else {
            "daemon already running"
        },
        if report.mismatch.is_some() {
            ", build mismatch recorded"
        } else {
            ""
        },
    );
    remember(
        container,
        Proved {
            binary: report.paths.binary.clone(),
            mismatch: report.mismatch,
        },
    );
    Ok(report.paths.binary)
}

pub fn restart_container_daemon(container: &str) -> io::Result<()> {
    validate_container(container)?;
    let ops = ContainerRemoteOps::new(container);
    let source = BundledServerBinary::discover();
    let confirm = install_confirm();
    let lock = install_lock(container);
    let _held = lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    forget_container_server(container);
    Installer::with_source(&ops, &source, confirm.as_ref(), host_label(container))
        .restart_daemon()?;
    Ok(())
}

/// Restart the container's daemon after making sure the binary it launches is
/// this build's. The bundled server is already on this machine, so the copy is
/// the whole install.
pub fn replace_container_server(container: &str) -> io::Result<()> {
    put_server_in_container(container, false)
}

/// Copy this build's server in over the one already there, even when it speaks
/// our dialect, and restart it. See [`Installer::replace_forced`].
pub fn update_container_server(container: &str) -> io::Result<()> {
    put_server_in_container(container, true)
}

fn put_server_in_container(container: &str, force: bool) -> io::Result<()> {
    validate_container(container)?;
    let ops = ContainerRemoteOps::new(container);
    let source = BundledServerBinary::discover();
    let confirm = install_confirm();
    let lock = install_lock(container);
    let _held = lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    forget_container_server(container);
    let installer = Installer::with_source(&ops, &source, confirm.as_ref(), host_label(container));
    if force {
        installer.replace_forced()?;
    } else {
        installer.replace()?;
    }
    Ok(())
}

// ATDD red-phase acceptance scaffolds for story 1. Test-only, no production
// code; see the module header for why they live here instead of `tests/`.
#[cfg(test)]
mod atdd;

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn real_container_names_and_ids_are_accepted() {
        // The one argv word handed to `docker exec`. Names and ids are the
        // intended input; anything else is docker's to refuse, and its answer
        // is what the prompt shows, so there is no regex to keep in sync here.
        for name in [
            "web",
            "my-app_1",
            "a1b2c3d4e5f6",
            "tty7-test-container",
            "容器",
            "registry.example.com:5000/team/app",
        ] {
            assert_eq!(
                validate_container(name),
                Ok(()),
                "{name:?} is passed to docker"
            );
        }
    }

    #[test]
    fn names_that_could_not_be_passed_safely_are_refused() {
        assert_eq!(validate_container(""), Err(ContainerNameError::Empty));
        assert!(matches!(
            validate_container("--privileged"),
            Err(ContainerNameError::LeadingDash(_))
        ));
        assert!(matches!(
            validate_container("-i"),
            Err(ContainerNameError::LeadingDash(_))
        ));
        assert!(matches!(
            validate_container("web\0host"),
            Err(ContainerNameError::Control(_))
        ));
        assert!(matches!(
            validate_container("web\nrm -rf /"),
            Err(ContainerNameError::Control(_))
        ));
        assert!(matches!(
            validate_container(&"x".repeat(MAX_CONTAINER_NAME + 1)),
            Err(ContainerNameError::TooLong(_))
        ));
    }

    #[test]
    fn the_transport_command_line_is_the_designs() {
        assert_eq!(
            docker_args("web", &["sh", "-s"]),
            vec!["exec", "-i", "web", "sh", "-s"]
        );
        assert_eq!(
            docker_args(
                "web",
                &[
                    "/root/.local/share/tty7/bin/tty7-server-c8p6",
                    "--stdio",
                    "--pane"
                ]
            ),
            vec![
                "exec",
                "-i",
                "web",
                "/root/.local/share/tty7/bin/tty7-server-c8p6",
                "--stdio",
                "--pane",
            ]
        );
        // `-i` but never `-t`: a terminal would mangle the byte stream.
        assert!(
            !docker_args("web", &["sh", "-s"]).iter().any(|a| a == "-t"),
            "no pseudo-terminal may be allocated"
        );
    }

    #[test]
    fn the_host_label_is_the_connection_key() {
        for container in ["web", "a1b2c3d4e5f6"] {
            let target = crate::core::session::RemoteTarget::Container {
                container: container.to_string(),
            };
            assert_eq!(host_label(container), target.connection_key());
        }
    }

    #[cfg(unix)]
    fn real_sh(script: &str) -> (String, bool) {
        use std::io::Write as _;
        let mut child = std::process::Command::new("sh")
            .arg("-s")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("every unix has /bin/sh");
        child
            .stdin
            .take()
            .unwrap()
            .write_all(script.as_bytes())
            .unwrap();
        let out = child.wait_with_output().unwrap();
        (
            String::from_utf8_lossy(&out.stdout).into_owned(),
            out.status.success(),
        )
    }

    #[cfg(unix)]
    fn sh_scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("tty7-ctr-sh-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn the_stat_probe_round_trips_its_three_answers() {
        assert_eq!(
            parse_stat("__tty7_container__ stat=none 0 0\n").unwrap(),
            None
        );

        let dir = parse_stat("__tty7_container__ stat=dir 0 0\n")
            .unwrap()
            .unwrap();
        assert!(dir.is_dir);

        let exe = parse_stat("__tty7_container__ stat=file 755 6291456\n")
            .unwrap()
            .unwrap();
        assert!(!exe.is_dir);
        assert_eq!(exe.size, 6_291_456);
        assert_ne!(exe.mode & 0o100, 0, "must read as executable");

        let half = parse_stat("__tty7_container__ stat=file 644 6291456\n")
            .unwrap()
            .unwrap();
        assert_eq!(
            half.mode & 0o100,
            0,
            "must read as executable only if it is"
        );
    }

    #[test]
    fn the_stat_probe_ignores_noise_but_not_silence() {
        let noisy = "WARNING: some docker chatter\n__tty7_container__ stat=file 755 10\n";
        assert_eq!(parse_stat(noisy).unwrap().unwrap().size, 10);
        assert!(parse_stat("").is_err());
        assert!(parse_stat("Error response from daemon: No such container: web\n").is_err());
        assert!(parse_stat("__tty7_container__ stat=weird 0 0\n").is_err());
    }

    #[test]
    fn a_listing_distinguishes_empty_from_absent() {
        assert_eq!(parse_list("__tty7_container__ nolisting\n").unwrap(), None);
        assert_eq!(
            parse_list("__tty7_container__ listing\n").unwrap(),
            Some(Vec::new())
        );
        assert_eq!(
            parse_list(
                "__tty7_container__ listing\n\
                 __tty7_container__ entry=tty7-server-c8p6\n\
                 __tty7_container__ entry=.tty7-server-c8p6.tmp\n"
            )
            .unwrap(),
            Some(vec![
                "tty7-server-c8p6".to_string(),
                ".tty7-server-c8p6.tmp".to_string()
            ])
        );
        assert_eq!(
            parse_list("__tty7_container__ listing\n__tty7_container__ entry=a b c\n").unwrap(),
            Some(vec!["a b c".to_string()])
        );
        assert!(parse_list("nothing at all\n").is_err());
    }

    #[test]
    fn script_paths_are_shell_quoted() {
        let script = stat_script("/home/o'brien/my dir/tty7-server-c8p6");
        assert!(
            script.contains(r"p='/home/o'\''brien/my dir/tty7-server-c8p6'"),
            "{script}"
        );
        let script = list_script("/home/a b/bin");
        assert!(script.contains("d='/home/a b/bin'"), "{script}");
        assert!(!list_script("/x").contains("\"/x\""));
    }

    #[test]
    fn the_place_probe_reads_its_three_answers() {
        assert_eq!(
            parse_place("WARNING: docker chatter\n__tty7_container__ base=/tmp/tty7-0\n").unwrap(),
            Some("/tmp/tty7-0".to_string())
        );
        assert_eq!(parse_place("__tty7_container__ noplace\n").unwrap(), None);
        assert!(parse_place("").is_err(), "silence is not an answer");
        assert!(
            parse_place("__tty7_container__ base=relative/dir\n").is_err(),
            "an install base has to be absolute"
        );
    }

    #[test]
    fn the_install_base_is_home_unless_the_probe_moved_it() {
        assert_eq!(
            relocation("/root", Some("/root".to_string())),
            Ok::<_, String>(None)
        );
        assert_eq!(
            relocation("/root", Some("/tmp/tty7-0".to_string())),
            Ok(Some("/tmp/tty7-0".to_string()))
        );
        let err = relocation("/root", None).expect_err("nowhere usable is a stated failure");
        assert!(err.contains("/root"), "{err}");
        assert!(err.contains("noexec"), "{err}");
    }

    #[cfg(unix)]
    #[test]
    fn the_place_probe_relocates_past_a_home_that_cannot_hold_a_bin_directory() {
        use std::io::Write as _;
        let dir = sh_scratch("relocate");
        // A regular file where the home should be: `mkdir -p` under it fails,
        // which is the same answer a read-only home gives the probe.
        let blocked_home = dir.join("home");
        std::fs::write(&blocked_home, b"not a directory").unwrap();
        let runtime = dir.join("run");
        let mut child = std::process::Command::new("sh")
            .arg("-s")
            .env("HOME", &blocked_home)
            .env("XDG_RUNTIME_DIR", &runtime)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("every unix has /bin/sh");
        child
            .stdin
            .take()
            .unwrap()
            .write_all(PLACE_SCRIPT.as_bytes())
            .unwrap();
        let out = child.wait_with_output().unwrap();
        assert!(out.status.success());
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert_eq!(
            parse_place(&stdout).unwrap(),
            Some(runtime.display().to_string()),
            "the first usable base should be the runtime directory: {stdout}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_remembered_server_is_reused_until_it_is_forgotten() {
        let container = "tty7-test-memo";
        let binary = "/root/.local/share/tty7/bin/tty7-server-c8p6";
        forget_container_server(container);
        assert_eq!(remembered_container_server(container), None);
        remember(
            container,
            Proved {
                binary: binary.to_string(),
                mismatch: None,
            },
        );
        assert_eq!(
            remembered_container_server(container).as_deref(),
            Some(binary),
            "the second pane of the same container must not re-push the server"
        );
        forget_container_server(container);
        assert_eq!(remembered_container_server(container), None);
    }

    #[test]
    fn a_daemon_refusal_keeps_the_reason_it_gave() {
        // `docker`'s answer for a name that is not running is the whole of the
        // user-facing diagnosis, so it must survive to the strip verbatim.
        let out = ExecOutput {
            status: Some(1),
            stdout: String::new(),
            stderr: "Error response from daemon: No such container: web\n".into(),
        };
        assert!(!out.success());
        assert!(
            out.failure_reason().contains("No such container: web"),
            "{}",
            out.failure_reason()
        );
        let denied = ExecOutput {
            status: Some(126),
            stdout: String::new(),
            stderr: "permission denied while trying to connect to the Docker daemon socket\n"
                .into(),
        };
        assert!(
            denied.failure_reason().contains("permission denied"),
            "{}",
            denied.failure_reason()
        );
    }

    #[test]
    fn the_settle_asks_the_control_dialect_with_the_quoted_binary() {
        let script = settle_script("/home/a b/tty7-server-c8p6", 4, "0.2", "0.3");
        assert!(
            script.contains("'/home/a b/tty7-server-c8p6' --stdio --bridge < /dev/null"),
            "{script}"
        );
        assert!(!script.contains("--pane"), "{script}");
        assert!(script.contains("-lt 4"), "{script}");
    }

    #[test]
    fn the_settle_budget_fits_inside_the_launch_timeout() {
        let step: f64 = SETTLE_STEP.parse().expect("a number for `sleep`");
        let grace: f64 = SETTLE_GRACE.parse().expect("a number for `sleep`");
        let worst = f64::from(SETTLE_TRIES) * step + grace;
        assert!(
            worst < LAUNCH_TIMEOUT.as_secs_f64() / 2.0,
            "{worst}s of settle inside a {LAUNCH_TIMEOUT:?} launch budget"
        );
    }

    #[cfg(unix)]
    #[test]
    fn the_stat_script_really_runs_and_answers_correctly() {
        use std::os::unix::fs::PermissionsExt as _;
        let dir = sh_scratch("stat");

        let (out, ok) = real_sh(&stat_script(dir.to_str().unwrap()));
        assert!(ok, "{out}");
        assert!(parse_stat(&out).unwrap().unwrap().is_dir);

        let absent = dir.join("not-here");
        let (out, ok) = real_sh(&stat_script(absent.to_str().unwrap()));
        assert!(ok, "{out}");
        assert_eq!(parse_stat(&out).unwrap(), None);

        let plain = dir.join("plain");
        std::fs::write(&plain, b"0123456789").unwrap();
        let (out, ok) = real_sh(&stat_script(plain.to_str().unwrap()));
        assert!(ok, "{out}");
        let stat = parse_stat(&out).unwrap().unwrap();
        assert!(!stat.is_dir);
        assert_eq!(stat.size, 10);
        assert_eq!(stat.mode & 0o100, 0, "not executable yet");

        std::fs::set_permissions(&plain, std::fs::Permissions::from_mode(0o755)).unwrap();
        let (out, ok) = real_sh(&stat_script(plain.to_str().unwrap()));
        assert!(ok, "{out}");
        assert_ne!(parse_stat(&out).unwrap().unwrap().mode & 0o100, 0);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(unix)]
    #[test]
    fn the_list_script_really_runs_and_answers_correctly() {
        let dir = sh_scratch("list");

        let (out, ok) = real_sh(&list_script(dir.to_str().unwrap()));
        assert!(ok, "{out}");
        assert_eq!(parse_list(&out).unwrap(), Some(Vec::new()), "empty");

        let (out, ok) = real_sh(&list_script(dir.join("nope").to_str().unwrap()));
        assert!(ok, "{out}");
        assert_eq!(parse_list(&out).unwrap(), None, "absent");

        std::fs::write(dir.join("tty7-server-c8p6"), b"x").unwrap();
        std::fs::write(dir.join(".tty7-server-c8p6.tmp"), b"x").unwrap();
        std::fs::write(dir.join("a b c"), b"x").unwrap();
        let (out, ok) = real_sh(&list_script(dir.to_str().unwrap()));
        assert!(ok, "{out}");
        let mut got = parse_list(&out).unwrap().unwrap();
        got.sort();
        assert_eq!(
            got,
            vec![".tty7-server-c8p6.tmp", "a b c", "tty7-server-c8p6"],
            "`ls -A` includes dotfiles, and a space is not a separator"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(unix)]
    #[test]
    fn the_home_script_really_reports_an_absolute_path() {
        let (out, ok) = real_sh(HOME_SCRIPT);
        assert!(ok, "{out}");
        let home = out.lines().find_map(|l| marked(l, "home=")).expect("home");
        assert!(home.starts_with('/'), "{home:?}");
    }

    #[cfg(unix)]
    #[test]
    fn the_home_script_falls_back_to_the_passwd_entry_without_a_home_env() {
        // Simulate a `docker exec` that never set HOME: the shell falls back to
        // the account's /etc/passwd entry rather than reporting nothing.
        let script = format!("HOME=\n{tmp}\n", tmp = HOME_SCRIPT);
        let (out, ok) = real_sh(&script);
        assert!(ok, "{out}");
        let home = out.lines().find_map(|l| marked(l, "home=")).expect("home");
        assert!(home.starts_with('/'), "{home:?}");
    }

    #[cfg(unix)]
    #[test]
    fn the_mutating_scripts_really_run() {
        let dir = sh_scratch("mutate");
        let nested = dir.join("a b").join("c'\''d");
        let q = |p: &PathBuf| p.to_str().unwrap().to_string();

        let (_, ok) = real_sh(&format!("mkdir -p {}\n", shell_quote(&q(&nested))));
        assert!(ok);
        assert!(nested.is_dir(), "mkdir did not create {nested:?}");

        let from = nested.join(".tty7-server-c8p6.tmp");
        let to = nested.join("tty7-server-c8p6");
        std::fs::write(&from, b"payload").unwrap();

        let (_, ok) = real_sh(&format!("chmod {:o} {}\n", 0o755, shell_quote(&q(&from))));
        assert!(ok);
        let (out, ok) = real_sh(&stat_script(&q(&from)));
        assert!(ok);
        assert_ne!(parse_stat(&out).unwrap().unwrap().mode & 0o100, 0);

        let (_, ok) = real_sh(&format!(
            "mv -f {} {}\n",
            shell_quote(&q(&from)),
            shell_quote(&q(&to))
        ));
        assert!(ok);
        assert!(to.is_file() && !from.exists(), "mv did not publish");

        let (_, ok) = real_sh(&format!("rm -f {}\n", shell_quote(&q(&to))));
        assert!(ok);
        assert!(!to.exists(), "rm did not remove");

        let _ = std::fs::remove_dir_all(&dir);
    }
}
