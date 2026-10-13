//! End-to-end lifecycle for the username selector, through the public API
//! only — the same calls the picker drives: sandbox-rooted list, select
//! memory, create-enter, guarded delete (free, live, invalid), and memory
//! fallback. One test function so the process-global env sandbox needs no
//! inter-test locking; env is saved and restored inline.
//!
//! The GUI half (compact window, highlight, modal visuals, keyboard) cannot
//! run headless; its checklist lives in the entry-7 plan.

use std::path::PathBuf;
use tty7_core::core::instance::*;

// `tempfile` is a dev-dependency of tty7-core; use std here to stay free of
// new deps: one sandbox root per run, removed by `Sandbox` on drop so a
// mid-test failure cannot litter the temp dir.
struct Sandbox {
    path: PathBuf,
}

impl Sandbox {
    fn tagged(tag: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "tty7-e2e-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        Self { path }
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

struct EnvGuard {
    keys: Vec<&'static str>,
    saved: Vec<Option<std::ffi::OsString>>,
}

impl EnvGuard {
    fn set(keys: &[(&'static str, &std::ffi::OsStr)]) -> Self {
        let names: Vec<&'static str> = keys.iter().map(|(k, _)| *k).collect();
        let saved = names.iter().map(std::env::var_os).collect::<Vec<_>>();
        for (k, v) in keys {
            // `unsafe` because this process owns its env here: the whole
            // lifecycle runs inside this one test function.
            unsafe { std::env::set_var(k, v) };
        }
        Self { keys: names, saved }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (k, v) in self.keys.iter().zip(self.saved.iter()) {
            match v {
                Some(value) => unsafe { std::env::set_var(k, value) },
                None => unsafe { std::env::remove_var(k) },
            }
        }
    }
}

#[test]
fn username_selector_end_to_end_lifecycle() {
    let root = Sandbox::tagged("root");
    let home = Sandbox::tagged("home");
    let root = &root.path;
    let home = &home.path;
    let _env = EnvGuard::set(&[
        (ENV_INSTANCE_ROOT, root.as_os_str()),
        // Unix reads HOME, Windows reads APPDATA/USERPROFILE: sandbox all
        // three so the memory file never touches the real user dirs.
        ("HOME", home.as_os_str()),
        ("APPDATA", home.as_os_str()),
        ("USERPROFILE", home.as_os_str()),
    ]);

    // List starts empty (Default is implied, never listed).
    assert_eq!(list_with_mtime(), Vec::new());

    // Create-enter: two instances through the same calls the picker uses.
    for name in ["alice", "bob"] {
        let dir = dir_for(name).expect("sandbox root resolves");
        ensure_dir(&dir).unwrap();
    }
    let mut listed: Vec<String> = list_with_mtime()
        .into_iter()
        .map(|(name, _)| name)
        .collect();
    // Presence, not order: recency ordering is the unit tests' contract
    // (order_by_recency + the stamped snapshot test); the filesystem clock
    // decides which of the two fresh dirs reads newer here.
    listed.sort();
    assert_eq!(listed, vec!["alice".to_string(), "bob".to_string()]);

    // Select-memory: entering bob is remembered; next open preselects it.
    write_last("bob").unwrap();
    assert_eq!(read_last().as_deref(), Some("bob"));

    // Create-enter: carol appears and becomes the remembered row.
    ensure_dir(&dir_for("carol").unwrap()).unwrap();
    write_last("carol").unwrap();
    assert!(list_with_mtime().iter().any(|(n, _)| n == "carol"));
    assert_eq!(read_last().as_deref(), Some("carol"));

    // Guarded delete, free seat: dir gone, memory falls back to bob.
    // Without a probe primitive (non-unix) the seat is unevaluable, so the
    // flow takes the product's double-confirm path: refuse unforced, remove
    // forced — same directory outcome, same memory rewrite after.
    if cfg!(unix) {
        remove_instance("carol", false).unwrap();
    } else {
        assert!(matches!(
            remove_instance("carol", false),
            Err(RemoveError::Unknown(_))
        ));
        assert!(root.join("tty7-carol").is_dir());
        remove_instance("carol", true).unwrap();
    }
    assert!(!root.join("tty7-carol").exists());
    write_last("bob").unwrap();
    assert_eq!(read_last().as_deref(), Some("bob"));

    // Guarded delete, invalid names: refused as Invalid (not a masked
    // broken sandbox), nothing touched.
    assert!(matches!(
        remove_instance("default", false),
        Err(RemoveError::Invalid(_))
    ));
    assert!(matches!(
        remove_instance("../x", false),
        Err(RemoveError::Invalid(_))
    ));
    assert!(root.join("tty7-alice").is_dir());
    assert_eq!(read_last().as_deref(), Some("bob"));

    // Memory guard on the live path: garbage is rejected with InvalidInput
    // and the remembered value is byte-identical afterwards.
    let before = read_last();
    let err = write_last("../x").unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
    assert_eq!(read_last(), before);

    // Memory fallback on the live read path: garbage in the file reads as
    // nothing (the picker then falls back to Default), and the sentinel
    // round-trips through the same live calls.
    let mem = tty7_core::core::config::default_config_dir()
        .expect("sandboxed home resolves")
        .join(MEMORY_FILE);
    std::fs::write(&mem, "../x\n").unwrap();
    assert_eq!(read_last(), None);
    write_last(DEFAULT_SENTINEL).unwrap();
    assert_eq!(read_last().as_deref(), Some(DEFAULT_SENTINEL));
    write_last("bob").unwrap();

    #[cfg(unix)]
    {
        // Guarded delete, live seat: refusal keeps dir + memory; release
        // then removes cleanly — the full modal contract at the API level.
        // A filesystem without lock support cannot hold the seat: then the
        // probe is honestly Unknown and the unforced remove must refuse
        // with Unknown instead of unlinking.
        use std::os::unix::io::AsRawFd as _;
        let dir = dir_for("alice").unwrap();
        std::fs::write(dir.join("daemon.lock"), format!("{}\n", std::process::id())).unwrap();
        let held = std::fs::File::options()
            .read(true)
            .write(true)
            .open(dir.join("daemon.lock"))
            .unwrap();
        if unsafe { libc::flock(held.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } == 0 {
            assert!(matches!(
                remove_instance("alice", false),
                Err(RemoveError::Live { .. })
            ));
            assert!(dir.is_dir());
            drop(held);
            remove_instance("alice", false).unwrap();
            assert!(!dir.exists());
        } else {
            drop(held);
            assert!(matches!(
                remove_instance("alice", false),
                Err(RemoveError::Unknown(_))
            ));
            assert!(dir.is_dir(), "an unevaluable probe must not unlink");
        }
    }
}
