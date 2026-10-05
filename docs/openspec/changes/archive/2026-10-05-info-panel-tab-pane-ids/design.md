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
