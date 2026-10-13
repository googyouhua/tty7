---
title: 'mtime recency tracer'
type: 'feature'
ticket: 1
created: '2026-10-11'
status: 'built'
baseline_revision: 'a1a7a73b2bbea73aba185313ba9dc792bc000cdc'
route: 'oneshot'
route_source: 'auto'
risk: 'low'
review: 'quick'
review_source: 'pinned'
lenses_ran: [quick]
lenses_ran: []
review_loop_iteration: 0
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** The startup picker rows show bare usernames with no recency signal, and nothing yet proves the core-to-UI path for per-instance metadata.

**Approach:** Snapshot each instance dir mtime once per picker open, order newest-first, and render the relative-time meta on each row, reusing the existing relative-time formatter.

</frozen-after-approval>

## Implementation Notes

Oneshot route: two files, mechanical, ~75 lines — a core mtime query plus ordering with unit tests in `crates/tty7-core/src/core/instance.rs`, and a picker change in `src/ui/instance_picker.rs` carrying per-row mtime into the row render via the existing `home::relative_time` helper (no new i18n keys; `HomeTime*` arms already cover four locales).

Decisions made while implementing:
- New core API is `list_with_mtime(_in)` returning snapshot `(name, Option<SystemTime>)` pairs plus a pure `order_by_recency` (newest first, unknown oldest, name tie-break) so ordering is unit-testable without touching mtimes.
- Removed the now-unused `list_names()` (no callers left in-tree) rather than leaving a dead pub fn.
- `PickerState` carries `Vec<PickerRow { name, mtime }>`; render adds a right-aligned 12px meta child only when mtime exists (Default row never has meta). Reused `crate::core::config::unix_now()` for now, matching `settings/hosts.rs`.
- Verification gap (pre-existing, not caused here): `cargo test --bin tty7-app` does not compile on this branch — 24 E0252 duplicate-import errors in `src/terminal/view.rs` test module. Proved pre-existing by stashing this change and rebuilding the baseline (same 24 errors, none citing touched files). Core tests run green (7/7); bin normal-build `cargo check` green; picker `mod tests` (updated + new rows_retain_snapshot_mtimes) could not execute in this tree state — recorded in deferred-work.md.

## Review Triage Log

- quick lens, 5 findings, all real: 4 patched (snapshot test now asserts malformed-exclusion + all-Some + stamped newest-first order; PickerState doc comment fixed; rows_retain_snapshot_mtimes added; core verify expectations aligned), 1 deferred (bin test-target E0252 breakage predates this change). Evidence: `cargo test -p tty7-core core::instance` 7/7 green including the strengthened snapshot test; `cargo check --bin tty7-app` clean.

## Verification

**Commands:**
- `cargo test -p tty7-core core::instance` -- expected: green, including new ordering, tie-break, and omit cases
- `cargo test` picker-side unit tests for the extended picker state -- expected: green
