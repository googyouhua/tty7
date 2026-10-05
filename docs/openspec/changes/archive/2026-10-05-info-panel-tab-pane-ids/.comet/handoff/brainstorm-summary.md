# Brainstorm Summary

- Change: info-panel-tab-pane-ids
- Date: 2026-10-05

## Confirmed Technical Approach

Three new `InfoRow`s in `render_panel_info` (`src/ui/right_panel.rs`), placed after the cwd row: Tab ID (`tab.tree_id.get()` uuid, raw + copy), Pane ID (`view.pane_id`, raw + copy), Tab `#` (machine-wide `@n` enumerated from `MachineMirrors::machine(host)` in CLI `tab_index` order, copies `@n`; `EMPTY` dash fallback when mirror lacks the workspace). Focus-following via existing `detail_pane` semantics. New `L10nKey`s in en/zh-CN/ja.

## Key Trade-offs and Risks

- Window-local index rejected: looks identical to `@n` but addresses a different tab than the CLI.
- Mirror lag only degrades the ordinal cell; ids are always local.
- Narrow-panel uuid truncation handled by existing `InfoValue::Text` tail truncation + full-value copy.

## Testing Strategy

- Unit tests in `right_panel.rs`: row presence, copy payloads, split focus-following, ordinal synced/unsynced.
- Manual cross-check against `tty7 tab ls --json` / `tty7 pane ls`; confirm no new rows on strip/sidebar/switcher; `cargo test` touched crates.

## Spec Patches

None.
