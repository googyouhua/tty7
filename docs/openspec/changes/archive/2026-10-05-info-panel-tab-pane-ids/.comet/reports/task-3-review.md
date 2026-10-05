# Task 3 Review: Machine-wide tab ordinal with fallback

## Verdict
APPROVED

## Spec compliance (plan Task 3 brief, all MUSTs)
- Helper: `tab_ordinal(&Machine, TabId) -> Option<u64>` placed directly above
  `format_rtt`, enumerating `machine.workspaces` then `ws.tabs` in stored
  order with 1-based dense counting — verbatim loop order matches CLI
  `tab_index` (`crates/tty7-cli/src/resolve.rs:13-17`). Returns `None` on
  absence. Matches Step 1 code exactly.
- Row insertion: between Pane ID push and shell comment block
  (`src/ui/right_panel.rs:1021-1039`), inside the existing block, order
  Tab ID / Pane ID / Tab `#` preserved. Synced case renders decorated
  `@n` with `copy: Some(decorated)` (paste-ready); fallback renders
  `InfoRow::text(t(PanelOrdinal), EMPTY)` with `copy: None` (plain
  `text`, no `.copyable()`). Matches Step 2 semantics (multi-line closure
  formatting only, no behavior change).
- API chain verified against call sites: `WorkspaceStore::all(cx).get(...) ->
  Option<&WindowView>` (`src/core/session.rs:24`, core `session.rs:483`);
  `WindowView::host_id() -> HostId` (core `session.rs:447`); 
  `MachineMirrors::machine(cx, host: HostId) -> Option<&Machine>`
  (`src/ui/machine_mirror.rs:36`); pattern consistent with
  `src/ui/switcher.rs:1050`. Fully-qualified `tty7_core::core::machine::`
  paths match `machine_mirror.rs`/`reorder.rs`/`windows.rs` convention —
  no second naming. `Tab::leaf` fixture constructor exists
  (core `machine.rs:264`); `InfoRow::text` sets `copy: None, reveal: None`
  (`right_panel.rs:443-452`), so fallback carries no copy payload.
- Tests: `ordinal_counts_workspaces_then_tabs_in_stored_order` asserts
  Some(1)/Some(2)/Some(3) across two workspaces via literal `Machine`
  construction (plan-authorized fallback when no binary-crate fixture
  exists); `ordinal_missing_from_mirror_means_no_copy_payload` asserts
  `None` on empty machine plus fallback row `copy == None` and value
  `Text("—")`. Covers 1/2/3 order + missing-tab None.
- Missing `#[gpui::test]` render assertion: explicitly permitted by plan
  Step 4 conditional ("otherwise rely on the Task 4 manual pass and say
  so"); report documents no suitable `render_panel_info` + populated-mirror
  fixture exists and defers to Task 4. No harness invented. Acceptable.

## Task quality
- YAGNI: no extra abstractions; struct-literal row only where decorated
  copy differs from displayed value would otherwise diverge — justified.
- Test hygiene: pure-helper unit tests, no invented harness, no duplication.
- No API misuse found: all field/method/signature usages verified above.

## RED/GREEN evidence
Checked as reported in `task-3-report.md` (not re-run per review
instructions): RED `error[E0425]: cannot find function tab_ordinal`
(helper missing — right reason); GREEN `cargo test -p tty7 right_panel`
10 passed / 0 failed, `cargo test -p tty7 i18n` 7 passed / 0 failed.
Remaining warnings identified as pre-existing via stash. No reason to doubt.

## Global constraints
All confirmed: labels via `t(L10nKey::...)` only; ordinal row between Pane
ID push and shell block (Tab ID, Pane ID, Tab `#` order); enumeration
order identical to CLI; `@n` decorated value+copy with `EMPTY`-dash
fallback row and no copy; no remote special casing; `src/ui/right_panel.rs`
only (+90/-0, strip/sidebar/switcher untouched).
