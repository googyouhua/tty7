# Review — Currency (reality-check of committed decisions)

Spine: `../architecture-username-selector.md` (status: draft, 2026-10-11)
Reviewer role: research-currency — every committed decision checked against
the repo and current toolchain, not training data.
Date: 2026-10-11. Toolchain observed: `rustc 1.98.1`.

## Verdict: PASS WITH FINDINGS (3 yellow, 1 red-adjacent)

No decision is built on a technology that no longer exists, and the stack
table matches the workspace pins. But AD-3 as literally written cannot be
built (it names the wrong probe primitive), AD-2's seam signature is novel
rather than conventional, AD-8 understates the locale count, and AD-4's
mtime proxy is weaker than "last-used" implies. Details below, ordered by
severity.

---

## F1 — AD-3 names an unbuildable probe (highest severity)

**Spine text:** "probe `<instance-dir>/daemon.lock` with a shared
non-blocking lock (new dir-scoped probe — `config_path("daemon.lock")`
resolves against the global pinned dir and cannot address another
instance)".

The parenthetical is correct about the problem but the Rule, read
literally, points at the broken construction:

- `singleton::lock_path()` (`crates/tty7-core/src/daemon/singleton.rs:51-53`)
  is `config::config_path("daemon.lock")`, and `config_path`
  (`crates/tty7-core/src/core/config.rs:1356-1358`) resolves through the
  process-global `config_dir()` — a `OnceLock` override that is
  **first-wins** (`set_config_dir`, `config.rs:1237-1239`: subsequent
  `set()` calls are silently ignored). There is no way to point it at a
  second instance's directory from the picker process, not even by
  temporarily swapping the override.
- The existing `holder_pid()` (`singleton.rs:167-193`) takes no path
  argument at all; it probes only the global dir. Reusing it (or
  `config_path`) for a per-instance probe is impossible without a new
  function.
- What AD-3 actually needs — and what the spine's own Consistency
  Conventions row hints at with "`holder_pid_in(dir)`-shaped" — is a new
  **dir-scoped** primitive, e.g. `holder_pid_in(dir: &Path)` (or a private
  `flock(LOCK_SH|LOCK_NB)` probe on `dir.join("daemon.lock")` inside the
  instance module). The Rule should name that, not `config_path`.

**Otherwise verified for AD-3:**

- `flock(LOCK_SH|LOCK_NB)` probe semantics are sound on this codebase's
  targets: the holder takes `LOCK_EX|LOCK_NB` (`singleton.rs:285`), the
  probe takes `LOCK_SH|LOCK_NB` (`singleton.rs:176`), shared conflicts
  with exclusive on both Linux and macOS (BSD `flock`). `EINTR` retry and
  `EWOULDBLOCK`-means-held handling are already the established pattern.
- `daemon.lock` lives directly in the config dir, so
  `<instance-dir>/daemon.lock` is the right path shape.
- Unix-only `holder_pid` confirmed: `#[cfg(unix)]` real implementation vs
  `#[cfg(not(unix))]` stub returning `None` (`singleton.rs:260-268`).
  The spine's "Non-unix: modal warns verification was skipped and allows
  the delete" matches the stub — but note this is **fail-open by design**,
  acknowledged, not verified-safe.
- The stop command in the report is real: `--as` is a global flag
  (`crates/tty7-cli/src/cli.rs`, `long = "as"`, routed per-instance) and
  `server stop` exists (`ServerCmd::Stop`, exercised in
  `cli.rs`/`commands.rs` tests). `tty7 --as <name> server stop` checks out.

**Unverified / unaddressed gaps the builder must close:**

1. **NFS/unsupported filesystems:** `flock` over NFS can return
   `ENOTSUP`; `holder_pid`'s match treats any errno other than
   `EWOULDBLOCK`/`EINTR` as `None` = "free" (`singleton.rs:180-184`) —
   i.e. fail-open to delete on precisely the shared-box setups this
   feature targets. The spine says nothing about this path.
2. **TOCTOU:** probe-then-`remove_dir_all` is not fenced by any held lock;
   a server can start in the window between probe and removal. No decision
   covers it (acceptable for v1 only if stated).
3. **Pid-report staleness:** the pid file is written at claim time; a seat
   held by a pre-recording build has stale/absent content
   (`singleton.rs:186-192` documents the `pid > 1` filter + caller identity
   check). The modal report should treat unparseable pid as "unknown
   holder", not as free.

## F2 — AD-2's seam signature is novel, not conventional (yellow)

**Spine text:** "`remove_instance_in(root, default_dir, name)` for tests,
following the existing `*_in` convention".

Verified against `crates/tty7-core/src/core/instance.rs`:

- Every existing `*_in` variant takes **exactly one** directory:
  `list_in(root)`, `read_last_in(default_dir)`,
  `write_last_in(default_dir, name)`. A two-directory
  `(root, default_dir)` signature has no precedent in the module.
- The two-arg shape is arguably justified (delete touches the instance
  root *and* the memory file in the default dir), but the spine should say
  so and should decide whether memory cleanup is inside `remove_instance`
  (then two dirs are needed) or left to the AD-7 caller (then
  `remove_instance_in(root, name)` suffices with one dir). As written, a
  builder following "the existing convention" will produce a different
  signature than the spine names.
- Remainder of AD-2 verified: no `remove_instance` exists today (grep
  confirms absence — it is genuinely new); validate-first-then-resolve
  matches `dir_for` (`instance.rs:85-89`); `io::Result` matches
  `ensure_dir`/`write_last_in`; the `[a-z0-9-]` + sentinel + leading-`-`
  rule cited by AD-6 is exactly `validate_name` (`instance.rs:37-50`).

## F3 — AD-8 understates the locale count (yellow)

**Spine text:** "gets an `L10nKey::InstancePicker*` entry with both `zh.rs`
and `en.rs` arms in the same change."

The repo has **four** locales, not two: `src/ui/i18n/{en,zh,ja,ru}.rs all
carry the full `InstancePicker*` family today (8 keys each, verified by
grep), and missing keys fall back to English (`translate` in
`src/ui/i18n/mod.rs:2053-2060` falls back to `translate_en`).

Requiring only zh+en is therefore shippable (ja/ru degrade to English,
never to a missing-key panic), but the spine should either require all
four arms or explicitly bless the English-fallback degradation for ja/ru.
As written, a builder satisfying the letter of AD-8 leaves two supported
locales untranslated with no recorded decision to do so.

## F4 — AD-4's mtime proxy is portable but semantically weak (yellow)

**Portability verified:**

- `metadata().modified()` is supported on all three target platforms for
  local filesystems; the spine's "omit the meta when the filesystem cannot
  supply it" matches the established repo idiom (`.ok()`/`unwrap_or`
  fallbacks in `src/core/ssh_config.rs:359`, `src/core/update.rs:1163`,
  `crates/tty7-core/src/core/machine.rs:1709`,
  `crates/tty7-core/src/daemon/mobile.rs:240`, and others). No new
  dependency, no migration — the "no new state file" claim holds.
- No existing mtime helper exists in the instance module; "mtime helper in
  instance module" (Capability Map) is correctly scoped as new code.

**Semantic weakness not acknowledged:**

- A **directory's** mtime reflects direct-child creation/removal/rename
  only — not nested writes, not file-content modification, and not reads.
  Steady-state session activity (scrollback, pane state, sockets in
  subdirectories) will *not* bump `tty7-<name>`'s mtime. What the column
  actually displays is closer to "last top-level churn" than "last used".
- Countervailing detail in the implementation's favor: claiming the
  singleton seat truncates and rewrites `daemon.lock`
  (`record_holder_pid`, `singleton.rs:140-152`), which *is* a top-level
  file — so each server start does bump the instance dir mtime. The proxy
  is therefore a reasonable "last started / last created" signal, but not
  "last used".
- The spine already scopes the damage ("display only", "mtime skew
  accepted", "hardening deferred"), so this is not a blocker — but the
  column label chosen from this value should avoid promising "last used"
  ordering semantics, and the deferred item should mention the nested-write
  blindness alongside restore/copy skew.

## Also verified (no findings)

- **Stack table:** `edition = "2024"` confirmed in root and
  `tty7-core` manifests; `gpui`/`gpui-component` workspace pins confirmed
  (`Cargo.toml:317-327` + `[patch]` fork indirection at `:469-474`).
  Resolved tree per `Cargo.lock`: gpui 0.2.2 from `l0ng-ai/zed` branch
  `tty7` (`#fb44723`), gpui-component 0.5.2 from `l0ng-ai/gpui-component`
  branch `tty7` (`#f5d9ab3`). One nit: the spine's "zed rev / tty7 branch"
  gloss hides the two-layer reality (upstream rev pin + fork-branch patch);
  bumping the rev means rebasing the fork per the manifest comment
  (`Cargo.toml:467-469`) — builders should read that comment before
  touching the pins. `InputState` (`gpui_component::input::InputState`)
  confirmed alive and already used by the picker (`instance_picker.rs:34`).
- **AD-1:** `tty7-core`'s manifest comments state the crate must not depend
  on gpui (verified; only mentions are "does *not* depend" comments), and
  all fs logic cited lives in the instance module. Holds.
- **AD-5:** `InputState`-owns-text matches current picker construction
  (`instance_picker.rs:174-188`); the Enter-typed-wins reversal and Esc
  stacking are behavior changes, not currency claims — no stale-fact risk
  found. Note the current `PickerState` (cursor-only, no filter field,
  `instance_picker.rs:101-104`) means the "derives" half of AD-5 is all new
  state, correctly scoped to `instance_picker.rs`.
- **AD-6:** rule text matches `validate_name` byte-for-byte (lowercase,
  digits, hyphen; rejects empty/sentinel/leading-`-`).
- **AD-7:** `write_last`/`write_last_in` signatures confirm the memory
  write path; sentinel semantics confirmed (`DEFAULT_SENTINEL =
  "default"`, `instance.rs:29`).
- **AD-9:** companion paths `../ux-username-selector/DESIGN.md` and
  `EXPERIENCE.md` exist (referenced as binding; content review is the
  companion reviewer's job, not this one).

## Currency checklist

| Claim | Status |
|---|---|
| Rust edition 2024 | confirmed (`Cargo.toml`, `tty7-core/Cargo.toml`) |
| gpui + gpui-component workspace pins | confirmed, with patch-indirection nit above |
| `InputState` exists in gpui-component | confirmed |
| `validate_name` rule | confirmed exact |
| `*_in` test-seam convention | exists but single-dir only — see F2 |
| `flock LOCK_SH\|LOCK_NB` probe sound (macOS/Linux) | sound on local FS; NFS gap — see F1 |
| `holder_pid` unix-only / Windows gap | confirmed; fail-open acknowledged — see F1 |
| `config_path("daemon.lock")` per-instance use | **not possible** — see F1 |
| `tty7 --as <name> server stop` | confirmed real |
| `metadata().modified()` portability | portable; semantic weakness — see F4 |
| zh+en locale arms | understated (4 locales) — see F3 |
| No `remove_instance` today | confirmed absent (genuinely new) |
