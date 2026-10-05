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
