## 1. Labels and rows

- [x] 1.1 Add `PanelTabId`, `PanelPaneId`, `PanelOrdinal` i18n keys with en/zh-CN/ja strings and verify `cargo test -p tty7 i18n` (or the repo's locale consistency test) passes
- [x] 1.2 Render Tab ID and Pane ID rows in `render_panel_info` after the cwd row with raw-value copy payloads, and verify new panel unit tests assert row presence, copy values, and focus-following in a split tab (ordinal row covered by 2.1)

## 2. Ordinal parity and verification

- [ ] 2.1 Derive the machine-wide tab ordinal from the machine mirror with `EMPTY`-dash fallback when unavailable, and verify unit tests cover both the synced and unsynced cases
- [ ] 2.2 Manual pass: open two same-named tabs plus a split, confirm Info rows match `tty7 tab ls --json` / `tty7 pane ls` ids and ordinals, and confirm strip/sidebar/switcher show no new rows; run `cargo test` for the touched crates
