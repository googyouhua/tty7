//! Docker containers as hosts: the third local-exec channel beside WSL.
//!
//! A container is reached with `docker exec`, which needs no sshd and no
//! network inside it. Everything else — routing, provisioning, the workspace —
//! is the machinery that already serves SSH and WSL.

use std::collections::HashMap;
use std::io;
use std::process::Stdio;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use tokio::io::AsyncWriteExt as _;

use super::wsl::{HOME_SCRIPT, launch_settle, list_script, marked, parse_list, parse_stat, stat_script};
use super::{
    BundledOrRelease, ExecOutput, Installer, MismatchedRemoteDaemon, RemoteOps, RemoteStat,
    default_fetcher, install_confirm, shell_quote,
};

pub const DOCKER_EXE: &str = "docker";

const COMMAND_TIMEOUT: Duration = Duration::from_secs(60);
const LAUNCH_TIMEOUT: Duration = Duration::from_secs(30);
const PUT_TIMEOUT: Duration = Duration::from_secs(300);

/// A container name or id that is safe to hand to `docker` as one argument.
///
/// The name never goes through a shell — it is an argv element — but a leading
/// `-` would be read as a docker option, and a control character would break
/// the `docker exec` line the user sees in a log.
pub fn validate_container(name: &str) -> io::Result<()> {
    if name.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "a docker container name cannot be empty",
        ));
    }
    if name.len() > 255 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("a docker container name of {} bytes is not real", name.len()),
        ));
    }
    if name.starts_with('-') {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{name:?} cannot be a docker container name: a leading `-` would be read as an option"),
        ));
    }
    if name.chars().any(|c| c.is_control() || c.is_whitespace()) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{name:?} cannot be a docker container name: it contains whitespace or a control character"),
        ));
    }
    Ok(())
}

/// Endpoints that are this machine's own daemon. Everything else — `ssh://`,
/// `tcp://`, `http://` — is a daemon somewhere else, which this transport
/// cannot serve: `docker exec` would run on that other machine while tty7's
/// files, panes and workspace state all assume the machine the user is sitting
/// at.
fn is_local_endpoint(host: &str) -> bool {
    let host = host.trim();
    host.starts_with("unix://")
        || host.starts_with("npipe://")
        || host.starts_with("fd://")
        || host.starts_with("\0")
}

/// The remote daemon this machine's docker CLI is pointed at, or `None` when it
/// is pointed at the local one.
///
/// `DOCKER_HOST` wins over the context, exactly as the docker CLI reads it. A
/// context that cannot be inspected is treated as local: refusing on a failed
/// `docker context inspect` would take the feature away from every machine
/// whose CLI is fine but whose context command is not.
pub fn remote_endpoint() -> Option<String> {
    remote_endpoint_from(std::env::var("DOCKER_HOST").ok().as_deref())
}

/// The decision itself, split out so it can be tested without touching the
/// process environment every other test shares.
fn remote_endpoint_from(docker_host: Option<&str>) -> Option<String> {
    if let Some(host) = docker_host {
        let host = host.trim();
        if !host.is_empty() {
            return (!is_local_endpoint(host)).then(|| host.to_string());
        }
    }
    let out = std::process::Command::new(DOCKER_EXE)
        .args([
            "context",
            "inspect",
            "--format",
            "{{.Endpoints.docker.Host}}",
        ])
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let host = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if host.is_empty() || is_local_endpoint(&host) {
        None
    } else {
        Some(host)
    }
}

/// `docker exec -i <container> <argv...>`.
///
/// `-i` keeps stdin open, which is the only thing carrying this channel: the
/// control connection and every pane travel through it. No shell is involved,
/// so a container without `/bin/sh` fails at the server, with a message, rather
/// than at an unparseable command line.
pub fn exec_args(container: &str, argv: &[String]) -> Vec<String> {
    let mut out = vec!["exec".to_string(), "-i".to_string(), container.to_string()];
    out.extend_from_slice(argv);
    out
}

pub struct DockerRemoteOps {
    container: String,
}

impl DockerRemoteOps {
    pub fn new(container: &str) -> Self {
        Self {
            container: container.to_string(),
        }
    }

    fn sh(&self, script: &str, timeout: Duration) -> Result<ExecOutput, String> {
        self.invoke(
            &["sh".to_string(), "-c".to_string(), script.to_string()],
            None,
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
        argv: &[String],
        input: Option<Vec<u8>>,
        discard_stdout: bool,
        timeout: Duration,
    ) -> Result<ExecOutput, String> {
        let args = exec_args(&self.container, argv);
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
        io::ErrorKind::NotFound => {
            format!("{DOCKER_EXE} is not available on this machine, so containers cannot be used")
        }
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

impl RemoteOps for DockerRemoteOps {
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
                out.stdout.chars().take(120).collect::<String>()
            )),
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
        let out = self.invoke(
            &["tee".to_string(), path.to_string()],
            Some(bytes.to_vec()),
            true,
            PUT_TIMEOUT,
        )?;
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

// ---- what we already know about a container's server --------------------

#[derive(Clone)]
struct Proved {
    binary: String,
    mismatch: Option<MismatchedRemoteDaemon>,
}

fn ready() -> &'static Mutex<HashMap<String, Proved>> {
    static READY: OnceLock<Mutex<HashMap<String, Proved>>> = OnceLock::new();
    READY.get_or_init(|| Mutex::new(HashMap::new()))
}

fn locks() -> &'static Mutex<HashMap<String, Arc<Mutex<()>>>> {
    static LOCKS: OnceLock<Mutex<HashMap<String, Arc<Mutex<()>>>>> = OnceLock::new();
    LOCKS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn install_lock(container: &str) -> Arc<Mutex<()>> {
    let mut map = locks()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    map.entry(container.to_string())
        .or_insert_with(|| Arc::new(Mutex::new(())))
        .clone()
}

fn remembered(container: &str) -> Option<Proved> {
    ready()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .get(container)
        .cloned()
}

fn remember(container: &str, proved: Proved) {
    ready()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .insert(container.to_string(), proved);
}

pub fn remembered_docker_server(container: &str) -> Option<String> {
    remembered(container).map(|proved| proved.binary)
}

/// Drop what we thought we knew about a container, so the next connection
/// proves it again the long way.
pub fn forget_docker_server(container: &str) {
    ready()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .remove(container);
}

fn host_label(container: &str) -> String {
    format!("docker:{container}")
}

/// Prove (or install) where this container's server is.
pub fn ensure_docker_server(container: &str) -> io::Result<String> {
    validate_container(container)?;

    let lock = install_lock(container);
    let _held = lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(proved) = remembered(container) {
        if let Some(mismatch) = proved.mismatch {
            super::record_remote_mismatches(vec![mismatch]);
        }
        return Ok(proved.binary);
    }

    let ops = DockerRemoteOps::new(container);
    // A container with no shell cannot host panes at all, and the installer is
    // a chain of shell scripts. Say that plainly here instead of letting the
    // first script fail with "exec: /bin/sh: not found".
    match ops.run(":") {
        Ok(out) if out.success() => {}
        Ok(out) => {
            return Err(io::Error::other(format!(
                "this container has no usable /bin/sh ({}); tty7 needs a POSIX shell inside it to run panes",
                out.failure_reason()
            )));
        }
        Err(reason) => {
            return Err(io::Error::other(format!(
                "this container could not be reached ({reason}); is it still running?"
            )));
        }
    }
    let fetch = default_fetcher();
    let source = BundledOrRelease::discover(fetch.as_ref());
    let confirm = install_confirm();
    let host = host_label(container);
    let report = Installer::with_source(&ops, &source, confirm.as_ref(), host.clone())
        .run()
        .map_err(io::Error::other)?;

    log::info!(
        "{host}: {} at {} ({}{})",
        if report.installed {
            "installed tty7-server"
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

pub fn restart_docker_daemon(container: &str) -> io::Result<()> {
    validate_container(container)?;
    let ops = DockerRemoteOps::new(container);
    let fetch = default_fetcher();
    let source = BundledOrRelease::discover(fetch.as_ref());
    let confirm = install_confirm();
    let host = host_label(container);
    let lock = install_lock(container);
    let _held = lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    Installer::with_source(&ops, &source, confirm.as_ref(), host)
        .restart_daemon()
        .map_err(io::Error::other)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_container_name_cannot_smuggle_an_option() {
        assert!(validate_container("api-dev").is_ok());
        assert!(validate_container("a1b2c3d4e5").is_ok());
        assert!(validate_container("--rm").is_err());
        assert!(validate_container("").is_err());
        assert!(validate_container("a b").is_err());
        assert!(validate_container("a\nb").is_err());
    }

    #[test]
    fn docker_host_decides_before_the_context_is_asked() {
        assert_eq!(
            remote_endpoint_from(Some("ssh://root@10.0.0.5")).as_deref(),
            Some("ssh://root@10.0.0.5")
        );
        assert_eq!(remote_endpoint_from(Some("tcp://10.0.0.5:2375")).as_deref(), Some("tcp://10.0.0.5:2375"));
        // An explicit local endpoint means local, whatever the context says.
        assert_eq!(remote_endpoint_from(Some("unix:///var/run/docker.sock")), None);
        assert_eq!(remote_endpoint_from(Some("   ")), None);
    }

    #[test]
    fn only_this_machines_endpoints_count_as_local() {
        for local in ["unix:///var/run/docker.sock", "npipe:////./pipe/docker_engine", "fd://3"] {
            assert!(is_local_endpoint(local), "{local}");
        }
        for remote in ["ssh://root@10.0.0.5", "tcp://10.0.0.5:2375", "http://10.0.0.5:2376"] {
            assert!(!is_local_endpoint(remote), "{remote}");
        }
    }

    #[test]
    fn exec_args_keep_the_name_and_the_argv_apart() {
        let argv = vec!["/bin/tty7-server".to_string(), "--stdio".to_string()];
        assert_eq!(
            exec_args("api-dev", &argv),
            vec!["exec", "-i", "api-dev", "/bin/tty7-server", "--stdio"]
        );
    }
}
