# Task 4 Report — info-panel-tab-pane-ids parity & verification pass

## 1. Full touched-crate suite

Command: `cargo test -p tty7`

Result: **PASS — 2514 passed; 0 failed; 2 ignored** (finished in 19.47s). Tail:

```
test ui::settings::gpui_tests::agent_roles_page_adds_edits_and_deletes ... ok

test result: ok. 2514 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 19.47s
```

## 2. Headless parity checks (code inspection)

- **Focused-leaf reads** — `src/ui/right_panel.rs:969-972`: rows are built inside
  `if let Some(tab) = self.tabs.get(self.active)` → `if let Some(leaf) = tab.detail_pane(window, cx)`,
  with `let view = leaf.read(cx)` at :971. `src/ui/right_panel.rs:1019` reads the Tab ID from the
  outer `tab` (`tab.tree_id.get().to_string()` — stable across splits); `:1020` reads the Pane ID
  from the focused leaf's `view` (`view.pane_id.to_string()` — follows refocus, same `view` the cwd
  row at :976 uses). Both pushes sit inside the leaf block, so the existing `rows.is_empty()` →
  `PanelNoSession` early return (`:1105`) is untouched — PASS.
- **Row order** — cwd (`:984`), follow-switch (`:1006`), Tab ID (`:1019`), Pane ID (`:1020`),
  Tab `#` ordinal (`:1028-1039`), shell (`:1055`), per plan — PASS.
- **No literal labels** — all three rows go through `t(L10nKey::PanelTabId / PanelPaneId / PanelOrdinal)`
  (`:1019`, `:1020`, `:1030`, `:1038`) — PASS.
- **`tab_ordinal` ≡ CLI `tab_index`** — `src/ui/right_panel.rs:384-398` enumerates
  `machine.workspaces` in stored order, `ws.tabs` in stored order, 1-based dense counting, returning
  `None` when absent. This is line-for-line the same loop as `crates/tty7-cli/src/resolve.rs:13-27`
  (`tab_index`); `ordinal_of` (`resolve.rs:29-34`) is the same find-first semantics. Host lookup
  (`right_panel.rs:1021-1027`) uses `WorkspaceStore::all(cx).get(self.workspace)` →
  `entry.host_id()` → `MachineMirrors::machine`, and the `None` fallback renders the `EMPTY` dash
  (`:1038`) with `copy: None` (plain `InfoRow::text`, not `.copyable()`), while ID rows still render.
  Ordinal copy payload is the decorated `@n` (`:1029-1033`); ID rows copy raw values — PASS.
- **No other surface gained rows** — grep for `PanelTabId|PanelPaneId|PanelOrdinal|tab_ordinal`
  in `src/ui/tab_strip.rs`, `src/ui/tab_sidebar.rs`, `src/ui/switcher.rs`: **no matches in any of
  the three**. The new keys appear only in `src/ui/right_panel.rs` (`:1019`, `:1020`, `:1030`, `:1038`)
  plus `src/ui/i18n/mod.rs:1121-1123`, `src/ui/i18n/en.rs:1441-1443`, `src/ui/i18n/zh.rs:1339-1341`,
  `src/ui/i18n/ja.rs:1505-1507` — PASS.
- **i18n allowlist untouched** — `KEPT_IN_ENGLISH` (`src/ui/i18n/mod.rs:2039-2101`) contains no new
  key; zh/ja strings differ from en, satisfying the locale consistency test (covered by the green
  suite) — PASS.
- **Unit tests present and green** (suite above): `id_rows_carry_raw_values_as_copy_payloads`
  (`src/ui/right_panel.rs:2505-2530`), `ordinal_counts_workspaces_then_tabs_in_stored_order`
  (`:2532-2558`), `ordinal_missing_from_mirror_means_no_copy_payload` (`:2560-2580`) — PASS.

## 3. CLI-side cross-check

Command: `cargo test -p tty7-cli resolve` → **PASS — 7 passed; 0 failed** (resolve unit tests,
including `tab_ordinals_number_the_machine_in_tree_order`, `a_tab_resolves_by_ordinal_or_full_id`).
The `cli_e2e` binary also ran green in the same invocation (all listed tests `ok`, EXIT:0).

## 4. RESIDUAL-FOR-VERIFY (needs a live GUI — not verifiable headless)

1. Open two same-named tabs plus a split in the second tab (three panes total across two tabs).
2. Focus each pane in turn and read the Info panel: Tab ID equals the `id` field for that tab in
   `tty7 tab ls --json`; Pane ID equals the focused pane's id in `tty7 pane ls`; ordinal equals the
   tab's `@n` in `tty7 tab ls --json`.
3. Disconnect/sync-lag case: with the daemon unreachable at startup (mirror not yet synced), confirm
   the ordinal cell shows `—` while both ID rows still render.
4. Confirm the strip, sidebar, and switcher show no new rows (this change touches only
   `render_panel_info`).

## 5. Notes

- No files modified, no commit made, no plan/OpenSpec tasks checked off (evidence only).
- HEAD verified: Tasks 1–3 commits present (`0d00289` chore Task 3, `ec13eff` ordinal,
  `435decc` ID-row fix, `4d35d96` ID rows, plus i18n commit below).
