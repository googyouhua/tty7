# Task 3 Report: Machine-wide tab ordinal with fallback

## Status
DONE (commit `ec13eff`), tests GREEN. No plan/OpenSpec checkboxes touched.

## What changed
`src/ui/right_panel.rs` only (+90 lines, no deletions):
- Pure helper `tab_ordinal(&Machine, TabId) -> Option<u64>` placed directly
  above `format_rtt`, enumerating `machine.workspaces[].tabs[]` in stored
  order (GUI mirror of CLI `tab_index`, `crates/tty7-cli/src/resolve.rs:13`).
- Ordinal row inserted between the Pane ID push (Task 2) and the shell
  comment block in `render_panel_info`, exactly per plan: synced tab renders
  `@n` with copy payload `@n` (paste-ready for `tty7 tab … @n`); missing
  mirror entry renders `InfoRow::text(label, EMPTY)` with NO copy payload.
- Two unit tests in `mod tests`: `ordinal_counts_workspaces_then_tabs_in_stored_order`
  (Some(1)/Some(2)/Some(3) across two workspaces via `Tab::leaf` + struct
  literals) and `ordinal_missing_from_mirror_means_no_copy_payload`
  (None on empty machine + fallback row has `copy == None`, value `—`).

## Pre-code verifications (all confirmed in repo, not assumed)
- `self.workspace: WorkspaceId` — `render_panel_info` is a `Tty7App` method
  and `Tty7App.workspace` exists at `src/ui/app.rs:1110`.
- `WorkspaceStore::all(cx).get(self.workspace)` returns `Option<&WindowView>`;
  `WindowView::host_id(&self) -> HostId` matches `MachineMirrors::machine(cx, host: HostId)`
  (`src/ui/machine_mirror.rs:36`) — cf. `src/ui/switcher.rs:1050` pattern.
- `Machine`/`TabId` naming: `right_panel.rs` had no existing machine imports;
  used the crate-wide fully-qualified `tty7_core::core::machine::{Machine, TabId}`
  (same as `src/ui/machine_mirror.rs`, `src/ui/reorder.rs`, `src/ui/windows.rs`) —
  no second naming introduced.
- Row order Tab ID, Pane ID, Tab `#` preserved; remote panes need no special
  casing (enumeration uses the pane host's mirror).

## TDD record
- RED: tests added first; `cargo test -p tty7 right_panel` failed to compile
  with `error[E0425]: cannot find function tab_ordinal in module super` (4 sites) —
  fails for exactly the right reason (helper missing, not a typo).
- GREEN: helper + row insertion added; both new tests pass, all pre-existing pass.
- gpui-integration assertion: NOT added — no existing `render_panel_info` gpui
  fixture with a populated `MachineMirrors` entry exists in this repo
  (`machine_mirror.rs` fixtures cover pull/seed flows only, not panel rendering).
  Per plan fallback: rely on Task 4 manual pass (`tty7 tab ls --json` `@n` compare
  + disconnected-startup `—` check). Inventing a harness was explicitly out of scope.

## Test summary
- `cargo test -p tty7 right_panel` → ok, 10 passed / 0 failed (includes both new
  ordinal tests + Task 2 `id_rows_carry_raw_values_as_copy_payloads`).
- `cargo test -p tty7 i18n` → ok, 7 passed / 0 failed (`PanelOrdinal` resolves in all locales).
- Two remaining warnings (`unused import Host` at test line 2141, `CwdEdit` privacy
  at line 309) are pre-existing and untouched by this change (confirmed via stash).

## Concerns
none

## Risk signals
no risk signals (single-file change, +90 lines, no cross-module/security/concurrency/
migration/API-surface impact)
