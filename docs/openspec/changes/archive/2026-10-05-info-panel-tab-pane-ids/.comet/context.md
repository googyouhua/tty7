# Comet Design Handoff

- Change: info-panel-tab-pane-ids
- Phase: design
- Mode: compact
- Context hash: 1e64e92fbaf706d7e887f1428c7596b0886d60e555ad9dc09605ff4456fb15e1

Generated-by: comet-handoff.sh
Task hash policy: task-content-v1. Read tasks.md for live completion; excerpts are design-time context.

OpenSpec remains the canonical capability spec. This handoff is a deterministic, source-traceable context pack, not an agent-authored summary.

## docs/openspec/changes/info-panel-tab-pane-ids/proposal.md

- Source: docs/openspec/changes/info-panel-tab-pane-ids/proposal.md
- Lines: 1-29
- SHA256: 29d350d26111502aaae07d6c9f194a7cc1b96155a1332072bef455321aa1cde7

```md
## Why

Tabs and panes with identical names cannot be told apart in the UI. Every tab already owns a unique `TabId` (uuid) and every pane a unique numeric id, but the UI never surfaces either value, so users must drop to the CLI (`tty7 tab ls --json`) to identify what they are looking at.

## What Changes

- Add identifier rows to the right Info panel (the panel that renders the focused pane's cwd):
  - Tab ID row: raw uuid of the active tab, with a copy button.
  - Pane ID row: raw numeric id of the focused pane, with a copy button.
  - Ordinal row: the CLI-style addresses (`@n` tab ordinal, `%n` pane address) for cross-referencing CLI output.
- Rows follow the focused pane: in a split tab they describe the last-focused leaf, exactly like the existing cwd/shell/branch rows.
- Out of scope: CLI changes, other Info rows, listing non-focused panes, tab-id exposure anywhere else (strip, sidebar, switcher).

## Capabilities

### New Capabilities

- `info-panel-ids`: the Info panel exposes the active tab's id, the focused pane's id, and their CLI ordinals as copyable rows.

### Modified Capabilities

- None. No existing spec changes behavior.

## Impact

- `src/ui/right_panel.rs` (`render_panel_info`, `InfoRow`/`InfoValue`): three new rows, copy affordance reuse.
- i18n labels in `src/ui/i18n/{en,zh,ja}.rs`.
- Ordinal source for `@n`: resolved in design (window tab index vs machine-wide CLI ordinal).
- Tests: panel unit tests for row presence, copy payloads, and focus-following in splits.

```

## docs/openspec/changes/info-panel-tab-pane-ids/design.md

- Source: docs/openspec/changes/info-panel-tab-pane-ids/design.md
- Lines: 1-48
- SHA256: eecac176ce31543592d9ec75c3f32d1a569457ccf5833ebd124d577fb49f8e66

```md
## Context

See proposal.md for motivation. The right Info panel is rendered by `render_panel_info`, which already resolves everything this change needs: the active tab (`self.tabs[self.active]`), the focused leaf (`tab.detail_pane`, i.e. the last-focused split), and `view.pane_id` (already read at the top of the function for the procs lookup, but never displayed). Rows are `InfoRow{label, value: InfoValue, copy, ...}` rendered as a label/value table with an optional hover copy tile.

## Goals / Non-Goals

**Goals:**

- Surface the three identifiers with zero new data plumbing: tab id from `tab.tree_id`, pane id from `view.pane_id`, ordinals derived alongside.
- Reuse the existing `InfoRow` copy affordance so behavior matches the cwd/branch rows.

**Non-Goals:**

- No new panel, tab, or toggle; no CLI output changes; no persistence of anything new.

## Decisions

### Row sources (no new plumbing)

- Tab ID: `tab.tree_id.get().to_string()` (uuid, `TabId`).
- Pane ID: `view.pane_id.to_string()` (u64, the same value the CLI spells `%n`).
- All three rows are built inside the existing `if let Some(leaf)` block, so they inherit the focus-following semantics of the cwd row for free. Placement: directly after the cwd row, before shell — identity first, then details.

### Ordinal semantics: machine-wide CLI parity

- The CLI's `@n` numbers tabs across all workspaces of a machine in tree order (`tab_index`), while the window only shows one workspace. Showing `self.active + 1` would look identical but address a different tab than the CLI — worse than showing nothing.
- Therefore the ordinal row is derived from the machine mirror for the pane's host (same enumeration order as the CLI), and falls back to hiding the ordinal cell (ids still shown) when the mirror has no data for that workspace. Alternative considered (window-local index) rejected for the CLI-mismatch reason above.

### Copy payload is the raw value

- `InfoValue::Text(raw)` with `copy: Some(raw.clone())`: the uuid / decimal string, not the decorated `@n` / `%n` spelling. Rationale: clipboard content must paste directly into CLI commands and bug reports without editing; the decorated form is display-only. This follows the cwd row precedent (compacted display, full path on clipboard).

### Labels via i18n, three locales

- New `L10nKey`s (e.g. `PanelTabId`, `PanelPaneId`, `PanelOrdinal`) with `en`/`zh-CN`/`ja` strings, following the existing `PanelCwd` pattern. No hard-coded English in the renderer.

## Risks / Trade-offs

- [Risk] Mirror unavailable (remote host not yet synced) → ordinal cell hidden → Mitigation: ids are always available locally, so the rows still render with the ordinal shown as the table's `EMPTY` dash.
- [Risk] Long uuid truncates in a narrow panel → Mitigation: `InfoValue::Text` already truncates from the tail and the copy tile carries the full value; same behavior as the branch row.

## Migration Plan

Not applicable: additive UI rows, no data migration, no rollback beyond revert.

## Open Questions

None. Ordinal fallback behavior is specified above; everything else reuses existing patterns.

```

## docs/openspec/changes/info-panel-tab-pane-ids/tasks.md

- Source: docs/openspec/changes/info-panel-tab-pane-ids/tasks.md
- Lines: 1-9
- SHA256: 5a9f30084142823d85b628483d6f11100c3b31d7d1c92d050e2d617cdd37af2e

```md
## 1. Labels and rows

- [ ] 1.1 Add `PanelTabId`, `PanelPaneId`, `PanelOrdinal` i18n keys with en/zh-CN/ja strings and verify `cargo test -p tty7 i18n` (or the repo's locale consistency test) passes
- [ ] 1.2 Render Tab ID, Pane ID, and ordinal rows in `render_panel_info` after the cwd row with raw-value copy payloads, and verify new panel unit tests assert row presence, copy values, and focus-following in a split tab

## 2. Ordinal parity and verification

- [ ] 2.1 Derive the machine-wide tab ordinal from the machine mirror with `EMPTY`-dash fallback when unavailable, and verify unit tests cover both the synced and unsynced cases
- [ ] 2.2 Manual pass: open two same-named tabs plus a split, confirm Info rows match `tty7 tab ls --json` / `tty7 pane ls` ids and ordinals, and confirm strip/sidebar/switcher show no new rows; run `cargo test` for the touched crates

```

## docs/openspec/changes/info-panel-tab-pane-ids/.openspec.yaml

- Source: docs/openspec/changes/info-panel-tab-pane-ids/.openspec.yaml
- Lines: 1-2
- SHA256: c956e2a907cf0aa23d948e300827f91e46ab77d64e288d98285dfe990bb0d989

```md
schema: spec-driven
created: 2026-10-05

```

## docs/openspec/changes/info-panel-tab-pane-ids/specs/info-panel-ids/spec.md

- Source: docs/openspec/changes/info-panel-tab-pane-ids/specs/info-panel-ids/spec.md
- Lines: 1-51
- SHA256: ca3d2620268746d6424a82eda33f14a9f03442cc1597c00e034ce6b8bcee069a

```md
## Purpose

The right Info panel exposes the unique identifiers of the active tab and the focused pane so users can distinguish same-named tabs and correlate the UI with CLI output.

## ADDED Requirements

### Requirement: Info panel shows the active tab identifier

The system SHALL display a Tab ID row in the right Info panel for the active tab, showing the tab's raw unique id with a copy affordance that copies the raw value.

#### Scenario: Active tab id is visible

- **WHEN** the user opens the right Info panel on any tab
- **THEN** a Tab ID row shows the active tab's unique id and its copy action copies that exact value

#### Scenario: Switching tabs updates the row

- **WHEN** the user activates a different tab
- **THEN** the Tab ID row shows the newly active tab's id

### Requirement: Info panel shows the focused pane identifier

The system SHALL display a Pane ID row in the right Info panel for the focused pane, showing the pane's raw numeric id with a copy affordance that copies the raw value. In a split tab the row SHALL follow the last-focused leaf, consistent with the existing cwd row.

#### Scenario: Focused pane id is visible

- **WHEN** the user opens the right Info panel
- **THEN** a Pane ID row shows the focused pane's numeric id and its copy action copies that exact value

#### Scenario: Split tab follows focus

- **WHEN** the user moves focus to another split within the same tab
- **THEN** the Pane ID row updates to the newly focused pane's id while the Tab ID row is unchanged

### Requirement: Info panel shows CLI ordinals

The system SHALL display the CLI-style ordinals alongside the ids so UI state can be correlated with `tty7 tab ls` / `tty7 pane ls` output.

#### Scenario: Ordinals match CLI addressing

- **WHEN** the user compares the Info panel with CLI output for the same workspace
- **THEN** the displayed ordinals address the same tab and pane as the CLI's `@n` and `%n` forms

### Requirement: Identifiers stay out of other surfaces

The system SHALL NOT add id rows to the tab strip, sidebar, or switcher in this change; identifier display is scoped to the right Info panel.

#### Scenario: Other surfaces unchanged

- **WHEN** the user looks at the tab strip, sidebar, or switcher
- **THEN** no tab or pane id rows appear there

```
