//! Per-user instances for machines where several people share one OS user.
//!
//! Everybody logging in as the same OS user (all root, say) shares the
//! default config directory, hence one server and one set of
//! workspaces/panes. An *instance* is an independent config directory —
//! `$ROOT/tty7-<name>` beside the default `$ROOT/tty7` — with its own
//! server, sockets, and state. The CLI's `--as <name>` and the GUI's
//! startup picker are both thin skins over this module; neither touches
//! the daemon/control protocol.
//!
//! The `*_in` variants take explicit directories so tests never touch the
//! real `$HOME`; the public wrappers resolve through the environment.

use std::path::{Path, PathBuf};

/// Override for the root under which instances live. Defaults to the parent
/// of the default config directory (`$HOME/.config` on unix).
pub const ENV_INSTANCE_ROOT: &str = "TTY7_AS_ROOT";

/// The file, inside the *default* config directory, remembering the last
/// instance the GUI entered. Stores `DEFAULT_SENTINEL` for the default
/// instance, otherwise the instance name. It lives outside every instance
/// on purpose: following the memory cannot depend on the remembered
/// instance still existing.
pub const MEMORY_FILE: &str = ".last-instance";

/// Memory value meaning the default instance. `validate_name` reserves it
/// so a real instance can never collide with it.
pub const DEFAULT_SENTINEL: &str = "default";

/// The directory prefix instances hang under the root.
const INSTANCE_PREFIX: &str = "tty7-";

/// A per-user instance name: lowercase letters, digits, hyphens. Rejects
/// `..`, `/`, a leading `-`, and the `default` sentinel — nothing that
/// could escape the `tty7-<name>` directory shape or read as a flag.
pub fn validate_name(s: &str) -> Result<String, String> {
    let ok = !s.is_empty()
        && s != DEFAULT_SENTINEL
        && !s.starts_with('-')
        && s.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-');
    if ok {
        Ok(s.to_string())
    } else {
        Err(format!(
            "'{s}' is not an instance name — use [a-z0-9-], e.g. alice"
        ))
    }
}

/// The instance root from injected values, for tests and the CLI's explicit
/// seam: `TTY7_AS_ROOT` when non-empty, else `$HOME/.config`. `home` is a
/// test double on every platform — the join is deliberately uniform rather
/// than platform-specific.
pub fn root_with(root_env: Option<&str>, home_env: Option<&str>) -> Option<PathBuf> {
    root_env
        .filter(|r| !r.is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            home_env
                .filter(|h| !h.is_empty())
                .map(|h| PathBuf::from(h).join(".config"))
        })
}

/// The live instance root: `TTY7_AS_ROOT`, else the parent of the default
/// config directory (platform-correct where `$HOME` is not the story).
pub fn default_root() -> Option<PathBuf> {
    let env = std::env::var(ENV_INSTANCE_ROOT).ok();
    root_with(
        env.as_deref(),
        std::env::var_os("HOME").as_deref().and_then(|h| h.to_str()),
    )
    .or_else(|| {
        crate::core::config::default_config_dir()?
            .parent()
            .map(Path::to_path_buf)
    })
}

/// The config directory for `name` under the live root, or `None` when no
/// root resolves. The name is validated first: an invalid name is an error,
/// never a surprising path.
pub fn dir_for(name: &str) -> Option<PathBuf> {
    validate_name(name)
        .ok()
        .and_then(|_| default_root().map(|r| r.join(format!("{INSTANCE_PREFIX}{name}"))))
}

/// Create an instance directory, locking the leaf to its owner. Parents are
/// left alone — on a shared box the parent stays shared.
pub fn ensure_dir(path: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))?;
    }
    Ok(())
}

/// Instance names present under `root`: subdirectories shaped exactly
/// `tty7-<valid-name>`, sorted. The default instance is implied, never
/// listed — it has no suffixed directory.
pub fn list_in(root: &Path) -> Vec<String> {
    let mut names = Vec::new();
    let Ok(entries) = std::fs::read_dir(root) else {
        return names;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(rest) = name.to_str().and_then(|n| n.strip_prefix(INSTANCE_PREFIX)) else {
            continue;
        };
        if entry.path().is_dir() && validate_name(rest).is_ok() {
            names.push(rest.to_string());
        }
    }
    names.sort();
    names
}

/// One snapshot row: the name plus its directory's last-modified time, for
/// recency display. The snapshot is taken once per listing and ordered by
/// [`order_by_recency`]: newest first, unknown mtimes as oldest, ties by
/// name. A missing mtime (`None`) renders no meta — never a placeholder.
pub fn list_with_mtime_in(root: &Path) -> Vec<(String, Option<std::time::SystemTime>)> {
    let mut rows: Vec<(String, Option<std::time::SystemTime>)> = list_in(root)
        .into_iter()
        .map(|name| {
            let mtime = root
                .join(format!("{INSTANCE_PREFIX}{name}"))
                .metadata()
                .and_then(|m| m.modified())
                .ok();
            (name, mtime)
        })
        .collect();
    order_by_recency(&mut rows);
    rows
}

/// The snapshot under the live root; empty when the root is missing.
pub fn list_with_mtime() -> Vec<(String, Option<std::time::SystemTime>)> {
    default_root()
        .map(|r| list_with_mtime_in(&r))
        .unwrap_or_default()
}

/// Newest mtime first; rows without a mtime sort as oldest; ties break by
/// name ascending. Pure so the order is testable without touching mtimes.
pub fn order_by_recency(rows: &mut [(String, Option<std::time::SystemTime>)]) {
    rows.sort_by(|a, b| match (a.1, b.1) {
        (Some(x), Some(y)) => y.cmp(&x).then_with(|| a.0.cmp(&b.0)),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => a.0.cmp(&b.0),
    });
}

/// The memory value for a resolved config directory: `DEFAULT_SENTINEL`
/// for the default instance, the instance name for `tty7-<name>` under the
/// live root, `None` for anything else (a foreign `--config-dir` is used
/// as-is but never remembered).
pub fn memory_name_for(dir: &Path) -> Option<String> {
    let default = crate::core::config::default_config_dir()?;
    if dir == default {
        return Some(DEFAULT_SENTINEL.to_string());
    }
    let root = default_root()?;
    if dir.parent() == Some(root.as_path()) {
        let name = dir.file_name()?.to_str()?;
        let rest = name.strip_prefix(INSTANCE_PREFIX)?;
        if validate_name(rest).is_ok() {
            return Some(rest.to_string());
        }
    }
    None
}

/// The remembered instance: `DEFAULT_SENTINEL` for the default instance,
/// otherwise a name. `None` when no memory exists, it is unreadable, or it
/// names something invalid — every one of those falls back to the picker.
pub fn read_last_in(default_dir: &Path) -> Option<String> {
    let text = std::fs::read_to_string(default_dir.join(MEMORY_FILE)).ok()?;
    let name = text.trim();
    if name == DEFAULT_SENTINEL {
        return Some(DEFAULT_SENTINEL.to_string());
    }
    validate_name(name).ok()
}

/// The remembered instance under the live default directory.
pub fn read_last() -> Option<String> {
    let dir = crate::core::config::default_config_dir()?;
    read_last_in(&dir)
}

/// Remember `name` (`DEFAULT_SENTINEL` for the default instance) in the
/// given default directory, creating it when needed.
///
/// The shape is enforced here so the no-dangling-memory guarantee does not
/// rest on call-site discipline (AD-7): only the sentinel or a
/// [`validate_name`]-clean name is written, anything else is an
/// `InvalidInput` error that leaves the directory and file untouched.
pub fn write_last_in(default_dir: &Path, name: &str) -> std::io::Result<()> {
    if name != DEFAULT_SENTINEL && validate_name(name).is_err() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("'{name}' is not an instance name — use [a-z0-9-], e.g. alice"),
        ));
    }
    std::fs::create_dir_all(default_dir)?;
    std::fs::write(default_dir.join(MEMORY_FILE), format!("{name}\n"))
}

/// Remember `name` under the live default directory.
pub fn write_last(name: &str) -> std::io::Result<()> {
    let Some(dir) = crate::core::config::default_config_dir() else {
        return Err(std::io::Error::other("no default config directory"));
    };
    write_last_in(&dir, name)
}

/// What the dir-scoped liveness probe found: the seat is free, held by a
/// live server, or could not be evaluated (in which case the caller must not
/// fail open — see [`remove_instance_in`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeatState {
    /// No server holds `<dir>/daemon.lock`.
    Free,
    /// A live server holds the seat. The pid is advisory/unverified: it is
    /// what the holder recorded, and `None` when it recorded nothing
    /// (a pre-recording build) or the record is unreadable. Either way the
    /// seat is held, so the directory must not be unlinked.
    Live { pid: Option<u32> },
    /// The probe could not be put to the kernel: non-unix (no probe
    /// primitive there), or a lock error such as NFS `ENOTSUP`.
    Unknown,
}

/// The seat holder for one instance directory, probed at
/// `<dir>/daemon.lock` — the same seat file the singleton claims, addressed
/// dir-scoped so the global pinned config dir is never consulted and
/// `daemon/singleton.rs` stays untouched.
///
/// Mirrors the singleton's `holder_pid` flock loop: a shared non-blocking
/// lock distinguishes a live holder (`EWOULDBLOCK`) from a free seat, and
/// `EINTR` retries rather than reporting either way. The lock file is never
/// created here: a seat file that does not exist has never had a holder, so
/// that reads as [`SeatState::Free`].
pub fn holder_pid_in(dir: &Path) -> SeatState {
    #[cfg(unix)]
    {
        use std::os::unix::io::AsRawFd as _;

        let path = dir.join("daemon.lock");
        let file = match std::fs::File::options().read(true).open(&path) {
            Ok(file) => file,
            // Never had a holder; anything else (permissions, ENOTDIR, …)
            // means the question could not be put to the kernel.
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return SeatState::Free,
            Err(_) => return SeatState::Unknown,
        };
        loop {
            // LOCK_SH is enough to conflict with a holder's LOCK_EX while
            // keeping the probe as weak as possible.
            if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_SH | libc::LOCK_NB) } == 0 {
                unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_UN) };
                return SeatState::Free;
            }
            match std::io::Error::last_os_error().raw_os_error() {
                Some(libc::EWOULDBLOCK) => break,
                Some(libc::EINTR) => continue,
                _ => return SeatState::Unknown,
            }
        }
        // The seat is held — the flock said so, which is authoritative. The
        // pid is advisory: every holder writes its pid the moment it claims,
        // so a missing or stale record still means "held, by someone unnamed".
        let pid = std::fs::read_to_string(&path)
            .ok()
            .and_then(|c| c.trim().parse::<u32>().ok())
            .filter(|&pid| pid > 1);
        SeatState::Live { pid }
    }
    #[cfg(not(unix))]
    {
        let _ = dir;
        // No probe primitive on this platform (mirrors the singleton's
        // unix-only `holder_pid`): the caller double-confirms instead of
        // deleting blind.
        SeatState::Unknown
    }
}

/// Why [`remove_instance_in`] refused or failed to remove an instance.
/// `Live` and `Unknown` never unlink without the force channel; the picker
/// (entry 5) renders the liveness report for `Live` and a second confirm
/// for `Unknown` (which then retries with `force = true`).
#[derive(Debug)]
pub enum RemoveError {
    /// The name is invalid or the sentinel, or the instance path is a
    /// symlink (never followed) or otherwise not a real directory.
    /// Nothing was touched, no path was built beyond the refusal.
    Invalid(String),
    /// A live server holds the seat. The pid is advisory/unverified (see
    /// [`SeatState::Live`]); the directory still exists.
    Live { pid: Option<u32> },
    /// The liveness probe could not be evaluated. The directory still
    /// exists; the caller double-confirms before proceeding.
    Unknown(String),
    /// The unlink itself (or resolving the root) failed.
    Io(std::io::Error),
}

impl std::fmt::Display for RemoveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RemoveError::Invalid(reason) => write!(f, "invalid instance: {reason}"),
            RemoveError::Live { pid: Some(pid) } => write!(
                f,
                "instance is live (server pid {pid}, advisory); stop that instance's server first"
            ),
            RemoveError::Live { pid: None } => write!(
                f,
                "instance is live (holder pid unknown); stop its server first"
            ),
            RemoveError::Unknown(reason) => write!(f, "liveness probe unavailable: {reason}"),
            RemoveError::Io(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for RemoveError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            RemoveError::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for RemoveError {
    fn from(e: std::io::Error) -> Self {
        RemoveError::Io(e)
    }
}

/// Remove the instance `name` under `root`: validate, run the authoritative
/// in-op liveness probe, then unlink — one op so the probe and the unlink
/// cannot disagree on the directory (AD-2).
///
/// - Invalid names and the `default` sentinel are refused before any path
///   is built; a symlinked `tty7-<name>` is refused without following it.
///   `force` never bypasses either: there is no directory a forced remove
///   could safely mean.
/// - A live seat ([`SeatState::Live`]) refuses without unlinking, forced or
///   not: the holder is authoritative for liveness, and only the user stops
///   it (no kill action anywhere).
/// - An unevaluable probe ([`SeatState::Unknown`]) refuses without unlinking
///   unless `force` is set — the picker's second explicit confirm — in which
///   case the unlink proceeds past the unknown seat.
/// - Otherwise the whole `tty7-<name>` directory is removed with
///   `remove_dir_all` (permanent, no trash).
///
/// The residual probe→unlink window is documented, not closed: a server
/// claiming between the probe and the unlink keeps its seat (the claim is
/// what is authoritative for serving), and the directory going missing in
/// the window surfaces as an `Io` error rather than silent success.
/// Memory is untouched throughout — rewriting it to the focused row after
/// `Ok` is the caller's ordered job (AD-7).
pub fn remove_instance_in(root: &Path, name: &str, force: bool) -> Result<(), RemoveError> {
    let clean = validate_name(name).map_err(|reason| {
        log::warn!("refusing to remove instance {name:?}: {reason}");
        RemoveError::Invalid(reason)
    })?;
    let dir = root.join(format!("{INSTANCE_PREFIX}{clean}"));
    // Resolve once and never follow a symlink: `symlink_metadata` does not
    // traverse the final component, so a symlinked instance dir is seen as
    // a symlink here and refused below. A missing path falls through —
    // the probe reads it as free and the unlink reports `NotFound`.
    match std::fs::symlink_metadata(&dir) {
        Ok(meta) if meta.file_type().is_symlink() => {
            log::warn!(
                "refusing to remove instance {clean:?}: {} is a symlink, not an instance directory",
                dir.display()
            );
            return Err(RemoveError::Invalid(format!(
                "'{clean}' is not an instance directory"
            )));
        }
        Ok(meta) if !meta.file_type().is_dir() => {
            log::warn!(
                "refusing to remove instance {clean:?}: {} is not a directory",
                dir.display()
            );
            return Err(RemoveError::Invalid(format!(
                "'{clean}' is not an instance directory"
            )));
        }
        _ => {}
    }
    match holder_pid_in(&dir) {
        SeatState::Live { pid } => {
            log::warn!(
                "refusing to remove live instance {clean:?} at {} (holder pid {})",
                dir.display(),
                pid.map_or("unknown".to_string(), |p| p.to_string())
            );
            return Err(RemoveError::Live { pid });
        }
        SeatState::Unknown if !force => {
            log::warn!(
                "refusing to remove instance {clean:?} at {}: liveness probe unavailable",
                dir.display()
            );
            return Err(RemoveError::Unknown(format!(
                "could not probe the server seat for '{clean}'"
            )));
        }
        SeatState::Unknown => {
            log::info!(
                "removing instance {clean:?} at {} with an unevaluable probe (second confirm)",
                dir.display()
            );
        }
        SeatState::Free => {}
    }
    match std::fs::remove_dir_all(&dir) {
        Ok(()) => {
            log::info!("removed instance {clean:?} at {}", dir.display());
            Ok(())
        }
        Err(e) => {
            log::warn!(
                "could not remove instance {clean:?} at {}: {e}",
                dir.display()
            );
            Err(RemoveError::Io(e))
        }
    }
}

/// Remove the instance `name` under the live root. `force` proceeds past an
/// unevaluable liveness probe only (the picker's second explicit confirm);
/// live seats and invalid names are still refused.
pub fn remove_instance(name: &str, force: bool) -> Result<(), RemoveError> {
    // Validate before resolving the root: an invalid name is a refusal,
    // never a path, even when no root resolves.
    if let Err(reason) = validate_name(name) {
        log::warn!("refusing to remove instance {name:?}: {reason}");
        return Err(RemoveError::Invalid(reason));
    }
    let Some(root) = default_root() else {
        log::warn!("could not remove instance {name:?}: no instance root");
        return Err(RemoveError::Unknown("no instance root".to_string()));
    };
    remove_instance_in(&root, name, force)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_take_a_narrow_shape() {
        assert_eq!(validate_name("alice").unwrap(), "alice");
        assert_eq!(validate_name("a-1").unwrap(), "a-1");
        for bad in [
            "", "Alice", "ALICE", "default", "../x", "a/b", "-x", "a b", "a_b", "%41",
        ] {
            assert!(
                validate_name(bad).is_err(),
                "'{bad}' must not read as an instance name"
            );
        }
    }

    #[test]
    fn root_prefers_the_override_then_home_config() {
        assert_eq!(
            root_with(Some("/r"), Some("/home/u")),
            Some(PathBuf::from("/r"))
        );
        assert_eq!(
            root_with(Some(""), Some("/home/u")),
            Some(PathBuf::from("/home/u/.config"))
        );
        assert_eq!(root_with(None, None), None);
    }

    #[test]
    fn listing_sees_only_well_shaped_instance_dirs() {
        let root = std::env::temp_dir().join(format!("tty7-inst-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("tty7-bob")).unwrap();
        std::fs::create_dir_all(root.join("tty7-alice")).unwrap();
        std::fs::create_dir_all(root.join("tty7")).unwrap();
        std::fs::create_dir_all(root.join("tty7-Default")).unwrap();
        std::fs::write(root.join("tty7-ghost"), "file, not dir").unwrap();
        assert_eq!(list_in(&root), vec!["alice".to_string(), "bob".to_string()]);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn recency_orders_newest_first_with_name_tiebreak() {
        use std::time::{Duration, UNIX_EPOCH};
        let t = |s: u64| UNIX_EPOCH + Duration::from_secs(s);
        let mut rows = vec![
            ("bob".to_string(), Some(t(20))),
            ("alice".to_string(), Some(t(30))),
            ("cara".to_string(), Some(t(30))),
            ("ghost".to_string(), None),
            ("zed".to_string(), None),
        ];
        order_by_recency(&mut rows);
        let names: Vec<&str> = rows.iter().map(|(n, _)| n.as_str()).collect();
        // Newest first, ties by name, unknown mtimes oldest (by name).
        assert_eq!(names, vec!["alice", "cara", "bob", "ghost", "zed"]);
    }

    #[test]
    fn snapshot_lists_only_well_shaped_dirs_with_mtimes_newest_first() {
        use std::time::{Duration, UNIX_EPOCH};
        let root = std::env::temp_dir().join(format!("tty7-mtime-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("tty7-bob")).unwrap();
        std::fs::create_dir_all(root.join("tty7-alice")).unwrap();
        // Malformed neighbors must not leak into the snapshot.
        std::fs::create_dir_all(root.join("tty7")).unwrap();
        std::fs::create_dir_all(root.join("tty7-Default")).unwrap();
        std::fs::write(root.join("tty7-ghost"), "file, not dir").unwrap();
        // Distinct mtimes so the order assertion is deterministic even on
        // coarse-clock filesystems. `File::open` on a directory fails on
        // Windows, so stamping is best-effort there: the exclusion and
        // `Some` assertions below still run, and `order_by_recency`'s pure
        // test owns the ordering contract everywhere.
        let t = |s: u64| UNIX_EPOCH + Duration::from_secs(s);
        let stamped = (|| -> std::io::Result<()> {
            std::fs::File::options()
                .read(true)
                .open(root.join("tty7-bob"))?
                .set_modified(t(1_000))?;
            std::fs::File::options()
                .read(true)
                .open(root.join("tty7-alice"))?
                .set_modified(t(2_000))?;
            Ok(())
        })()
        .is_ok();
        let rows = list_with_mtime_in(&root);
        assert_eq!(rows.len(), 2);
        assert!(rows.iter().all(|(_, m)| m.is_some()));
        if stamped {
            let names: Vec<&str> = rows.iter().map(|(n, _)| n.as_str()).collect();
            assert_eq!(names, vec!["alice", "bob"]);
        }
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn memory_names_resolve_from_directories() {
        // Read-only against the live environment: safe under parallel tests.
        if let Some(default) = crate::core::config::default_config_dir() {
            assert_eq!(memory_name_for(&default).as_deref(), Some(DEFAULT_SENTINEL));
        }
        // A foreign directory is used as-is but never remembered.
        assert_eq!(
            memory_name_for(Path::new("/elsewhere/tty7")).as_deref(),
            None
        );
        assert_eq!(
            memory_name_for(Path::new("/elsewhere/custom")).as_deref(),
            None
        );
    }

    #[test]
    fn memory_round_trips_and_rejects_garbage() {
        let dir = std::env::temp_dir().join(format!("tty7-mem-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(read_last_in(&dir), None);
        write_last_in(&dir, "alice").unwrap();
        assert_eq!(read_last_in(&dir).as_deref(), Some("alice"));
        write_last_in(&dir, DEFAULT_SENTINEL).unwrap();
        assert_eq!(read_last_in(&dir).as_deref(), Some(DEFAULT_SENTINEL));
        std::fs::write(dir.join(MEMORY_FILE), "../x\n").unwrap();
        assert_eq!(read_last_in(&dir), None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn memory_guard_rejects_garbage_and_leaves_the_file_alone() {
        let dir = std::env::temp_dir().join(format!("tty7-mem-guard-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        write_last_in(&dir, "alice").unwrap();
        for bad in ["", "-x", "../x", "a/b", "Alice", "a_b"] {
            assert!(
                write_last_in(&dir, bad).is_err(),
                "{bad:?} must not pass the memory guard"
            );
            // The remembered value is untouched: still alice, not garbage.
            assert_eq!(read_last_in(&dir).as_deref(), Some("alice"));
        }
        assert_eq!(
            std::fs::read_to_string(dir.join(MEMORY_FILE)).unwrap(),
            "alice\n"
        );
        // The sentinel stays writable: it is a memory value, not an instance.
        write_last_in(&dir, DEFAULT_SENTINEL).unwrap();
        assert_eq!(read_last_in(&dir).as_deref(), Some(DEFAULT_SENTINEL));
        // Garbage against a fresh directory fails before any fs mutation:
        // nothing is created.
        let fresh = std::env::temp_dir().join(format!("tty7-mem-fresh-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&fresh);
        assert!(write_last_in(&fresh, "../x").is_err());
        assert!(!fresh.exists(), "a refused write must not create anything");
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(&fresh);
    }

    #[test]
    fn removing_a_free_instance_unlinks_it() {
        let root = std::env::temp_dir().join(format!("tty7-rm-free-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let dir = root.join("tty7-gone");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("state.json"), "{}").unwrap();
        // No seat file at all: never had a holder, so free — except where
        // no probe primitive exists (non-unix reads Unknown by design).
        if cfg!(unix) {
            assert_eq!(holder_pid_in(&dir), SeatState::Free);
            assert!(remove_instance_in(&root, "gone", false).is_ok());
            assert!(!dir.exists(), "a free instance must be unlinked");
            // Forcing a free seat is a no-op: the same unlink runs.
            std::fs::create_dir_all(&dir).unwrap();
        } else {
            assert_eq!(holder_pid_in(&dir), SeatState::Unknown);
            assert!(matches!(
                remove_instance_in(&root, "gone", false),
                Err(RemoveError::Unknown(_))
            ));
            assert!(dir.is_dir(), "an unevaluable probe must not unlink");
        }
        assert!(remove_instance_in(&root, "gone", true).is_ok());
        assert!(!dir.exists());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn removing_a_missing_instance_reports_io_not_found() {
        let root = std::env::temp_dir().join(format!("tty7-rm-missing-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let before: Vec<_> = std::fs::read_dir(&root)
            .unwrap()
            .flatten()
            .map(|e| e.file_name())
            .collect();
        // Unix resolves the missing dir to Free-then-NotFound; without a
        // probe primitive the seat is unevaluable before the unlink runs.
        if cfg!(unix) {
            let err = remove_instance_in(&root, "ghost", false).unwrap_err();
            assert!(
                matches!(err, RemoveError::Io(ref e) if e.kind() == std::io::ErrorKind::NotFound),
                "a missing instance must surface NotFound, got {err:?}"
            );
        } else {
            assert!(matches!(
                remove_instance_in(&root, "ghost", false),
                Err(RemoveError::Unknown(_))
            ));
        }
        let after: Vec<_> = std::fs::read_dir(&root)
            .unwrap()
            .flatten()
            .map(|e| e.file_name())
            .collect();
        assert_eq!(before, after, "nothing may be created by a failed remove");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn removing_invalid_sentinel_or_symlinked_names_touches_nothing() {
        let root = std::env::temp_dir().join(format!("tty7-rm-bad-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("tty7-keep")).unwrap();
        std::fs::write(root.join("tty7-keep").join("state.json"), "{}").unwrap();
        for bad in ["", "default", "-x", "../x", "a/b", "Alice"] {
            assert!(
                matches!(
                    remove_instance_in(&root, bad, false),
                    Err(RemoveError::Invalid(_))
                ),
                "{bad:?} must be refused as invalid"
            );
            // Force never bypasses validity: there is no directory a forced
            // remove of a bad name could safely mean.
            assert!(
                matches!(
                    remove_instance_in(&root, bad, true),
                    Err(RemoveError::Invalid(_))
                ),
                "forced {bad:?} must still be refused as invalid"
            );
        }
        assert!(
            root.join("tty7-keep").is_dir(),
            "a refused remove must touch nothing"
        );
        assert_eq!(list_in(&root), vec!["keep".to_string()]);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[cfg(unix)]
    #[test]
    fn removing_a_symlinked_instance_dir_is_refused_without_following_it() {
        use std::os::unix::fs::symlink;

        let root = std::env::temp_dir().join(format!("tty7-rm-link-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let target = std::env::temp_dir().join(format!("tty7-rm-target-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&target);
        std::fs::create_dir_all(&target).unwrap();
        std::fs::write(target.join("precious"), "do not delete").unwrap();
        std::fs::create_dir_all(&root).unwrap();
        symlink(&target, root.join("tty7-link")).unwrap();
        assert!(
            matches!(
                remove_instance_in(&root, "link", false),
                Err(RemoveError::Invalid(_))
            ),
            "a symlinked instance dir must be refused"
        );
        // Forcing never follows a symlink either.
        assert!(
            matches!(
                remove_instance_in(&root, "link", true),
                Err(RemoveError::Invalid(_))
            ),
            "a forced remove must still refuse a symlink"
        );
        // Neither the link nor its target was followed or unlinked.
        assert!(
            std::fs::symlink_metadata(root.join("tty7-link"))
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert_eq!(
            std::fs::read_to_string(target.join("precious")).unwrap(),
            "do not delete"
        );
        // The probe itself never follows either: the lock resolves under
        // the link path, and with no seat file there it reads free — the
        // refusal above comes from the symlink check, not the probe.
        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_dir_all(&target);
    }

    #[cfg(unix)]
    #[test]
    fn removing_a_live_instance_is_refused_and_keeps_the_dir() {
        use std::os::unix::io::AsRawFd as _;

        let root = std::env::temp_dir().join(format!("tty7-rm-live-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let dir = root.join("tty7-busy");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("state.json"), "{}").unwrap();
        // A live server: pid recorded, seat held LOCK_EX.
        let pid = std::process::id();
        std::fs::write(dir.join("daemon.lock"), format!("{pid}\n")).unwrap();
        let held = std::fs::File::options()
            .read(true)
            .write(true)
            .open(dir.join("daemon.lock"))
            .unwrap();
        assert_eq!(
            unsafe { libc::flock(held.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) },
            0,
            "the test must hold the seat for the probe to see"
        );
        assert_eq!(holder_pid_in(&dir), SeatState::Live { pid: Some(pid) });
        match remove_instance_in(&root, "busy", false) {
            Err(RemoveError::Live { pid: seen }) => assert_eq!(seen, Some(pid)),
            other => panic!("a live seat must refuse with Live, got {other:?}"),
        }
        assert!(dir.is_dir(), "a refused remove must keep the directory");
        assert!(dir.join("state.json").exists());
        // Force never bypasses a live seat: only the user stops the server.
        match remove_instance_in(&root, "busy", true) {
            Err(RemoveError::Live { pid: seen }) => assert_eq!(seen, Some(pid)),
            other => panic!("a forced remove must still refuse a live seat, got {other:?}"),
        }
        assert!(dir.is_dir(), "a forced remove must keep a live directory");
        drop(held);
        // Once the seat is released the same directory removes cleanly.
        assert!(remove_instance_in(&root, "busy", false).is_ok());
        assert!(!dir.exists());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[cfg(unix)]
    #[test]
    fn force_proceeds_past_an_unknown_probe_but_live_still_refuses() {
        // `root` is a regular file, so `<root>/tty7-<name>/daemon.lock`
        // cannot be opened (ENOTDIR, never NotFound): the probe reads
        // Unknown however the remove is forced. This runs as any user —
        // permission tricks would not survive root — and touches nothing:
        // both attempts fail before any unlink could mean anything.
        let root = std::env::temp_dir().join(format!("tty7-rm-force-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::write(&root, "file, not dir").unwrap();
        let dir = root.join("tty7-busy");
        assert_eq!(holder_pid_in(&dir), SeatState::Unknown);
        match remove_instance_in(&root, "busy", false) {
            Err(RemoveError::Unknown(_)) => {}
            other => panic!("an unforced unknown probe must refuse, got {other:?}"),
        }
        // Forced, the op proceeds past the unknown seat and fails at the
        // unlink instead — the probe refusal is what force lifts, nothing
        // else.
        match remove_instance_in(&root, "busy", true) {
            Err(RemoveError::Io(_)) => {}
            other => panic!("a forced unknown probe must reach the unlink, got {other:?}"),
        }
        assert!(
            std::fs::read_to_string(&root).unwrap() == "file, not dir",
            "a forced unknown remove must touch nothing but its own unlink"
        );
        let _ = std::fs::remove_file(&root);
    }

    #[cfg(unix)]
    #[test]
    fn probe_reports_unknown_when_the_lock_cannot_be_put_to_the_kernel() {
        // `dir` is a regular file, so `<dir>/daemon.lock` cannot be opened:
        // ENOTDIR is not NotFound, so the probe must not read it as free.
        let root = std::env::temp_dir().join(format!("tty7-rm-unknown-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let file_as_dir = root.join("not-a-dir");
        std::fs::write(&file_as_dir, "file, not dir").unwrap();
        assert_eq!(holder_pid_in(&file_as_dir), SeatState::Unknown);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[cfg(not(unix))]
    #[test]
    fn probe_is_unknown_where_no_primitive_exists() {
        let root = std::env::temp_dir().join("tty7-rm-unknown");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("tty7-busy")).unwrap();
        assert_eq!(holder_pid_in(&root.join("tty7-busy")), SeatState::Unknown);
        assert!(matches!(
            remove_instance_in(&root, "busy", false),
            Err(RemoveError::Unknown(_))
        ));
        assert!(root.join("tty7-busy").is_dir());
        // The second confirm proceeds past the unknown seat: without a probe
        // primitive the forced unlink is the only way through.
        assert!(remove_instance_in(&root, "busy", true).is_ok());
        assert!(!root.join("tty7-busy").exists());
        let _ = std::fs::remove_dir_all(&root);
    }
}
