---
comet_change: info-panel-tab-pane-ids
role: technical-design
canonical_spec: openspec
archived-with: 2026-10-05-info-panel-tab-pane-ids
status: final
---

# Design Doc: Tab/Pane IDs in the Info Panel

## Context

The open-phase `design.md` fixes the approach: three identifier rows in `render_panel_info`, reusing `InfoRow` and the focused-leaf semantics. This doc refines the implementation mechanics: exact data sources, ordinal enumeration, row construction, labels, and tests.

## Data sources (all local, no new plumbing)

- Tab ID: `self.tabs[self.active].tree_id.get().to_string()` — `tree_id: Cell<TabId>` (`src/ui/app.rs:521`), uuid.
- Pane ID: `view.pane_id.to_string()` — `u64`, the same value the CLI spells `%n` (`src/terminal/view.rs:307`). Already read in `render_panel_info` (`src/ui/right_panel.rs:951`); this change displays it.
- Both live inside the existing `if let Some(leaf) = tab.detail_pane(window, cx)` block, so split tabs automatically report the last-focused leaf, matching the cwd/shell/branch rows.

## Ordinal derivation

- Enumerate `MachineMirrors::machine(cx, host).workspaces[].tabs[]` in order (the GUI mirror of the CLI's `tab_index`, `crates/tty7-cli/src/resolve.rs:13`); position+1 is `@n`. Host comes from the current workspace entry (`WorkspaceStore::all(cx).get(self.workspace).host_id()`, cf. `src/ui/switcher.rs:1050`).
- Fallback: when the mirror has no machine for the host (not yet synced), the ordinal cell renders the table `EMPTY` dash; the two ID rows still render.
- Copy payload for the ordinal row is the decorated `@n` string (paste-ready for `tty7 tab … @n`); ID rows copy raw values (uuid / decimal).

## Row construction

```rust
rows.push(InfoRow::text(t(L10nKey::PanelTabId), tab.tree_id.get().to_string()).copyable());
rows.push(InfoRow::text(t(L10nKey::PanelPaneId), view.pane_id.to_string()).copyable());
```

- Value type `InfoValue::Text` (tail truncation for the 36-char uuid in narrow panels; full value rides the copy tile — same contract as the branch row).
- No `reveal` (uuids/ids are not paths), no `edit_cwd`, no `toggle_follow`.
- Order: cwd row, follow-switch row (unchanged), then Tab ID, Pane ID, Tab `#`, then shell — identity block stays together at the top.

## Labels

New `L10nKey::PanelTabId` / `PanelPaneId` / `PanelOrdinal` with en / zh-CN / ja strings, following the `PanelCwd` pattern. Renderer code holds no literal label text.

## Edge cases

- No active tab / no leaf: existing early return (`rows.is_empty()` → `PanelNoSession`) unchanged; new rows only exist inside the leaf block, so they cannot appear without a session.
- Hibernated tab: rows render from the same `TerminalView` the cwd row uses; no special casing.
- Remote pane: ids are daemon-agnostic (`pane_id` is assigned per daemon but displayed as-is); ordinal enumeration uses the pane's host mirror.

## Test strategy

- Unit (`src/ui/right_panel.rs` tests module): three rows present with exact values for a fixture tab/pane; copy payloads equal raw values (`@n` decorated only on the ordinal row); refocus in a split updates Pane ID, leaves Tab ID; mirror-missing fixture shows `—` for ordinal with ids intact.
- Manual: two same-named tabs + one split; compare against `tty7 tab ls --json` / `tty7 pane ls`; verify strip/sidebar/switcher unchanged.
- Suite: `cargo test` on touched crates; `gpui::test` harness where the existing panel tests live.

## Risks

- Mirror enumeration order must stay identical to CLI `tab_index` (workspaces in stored order, tabs in stored order). Both read the same `Machine` shape; any future reordering must change both — noted here so a later reorder does not silently desync `@n`.
