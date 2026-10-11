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
pub fn write_last_in(default_dir: &Path, name: &str) -> std::io::Result<()> {
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
}
