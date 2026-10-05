# Task 4 Review — info-panel-tab-pane-ids parity & verification pass

## Verdict: APPROVED

## Checks

1. **Suite result unambiguous** — report records `cargo test -p tty7` tail:
   `2514 passed; 0 failed; 2 ignored`, plus `cargo test -p tty7-cli resolve`
   7 passed / 0 failed. Zero failures on both; counts explicit. (Tail taken
   as recorded per reviewer rule; full suite not re-run.) — PASS
2. **Cited file:lines exist (spot-checked via grep, no code written)** —
   `src/ui/right_panel.rs:1019/1020` (ID rows), `:1030/:1038` (ordinal row +
   dash fallback), `:384` (`tab_ordinal`), `:2506` + `:2533` + `:2561`
   (three unit tests), `src/ui/i18n/mod.rs:1121-1123` + `en.rs:1441-1443` /
   `zh.rs:1339-1341` / `ja.rs:1505-1507` (keys + strings),
   `crates/tty7-cli/src/resolve.rs:13/:29` (`tab_index` / `ordinal_of`).
   One trivial drift: report cites the ordinal-order test at `:2532`, actual
   `fn` line is `:2533` (attribute vs fn line) — immaterial. — PASS
3. **No other surface gained rows** — repo-wide grep for
   `PanelTabId|PanelPaneId|PanelOrdinal` under `src/ui` matches only
   `right_panel.rs` + the four i18n files; no hits in `tab_strip.rs`,
   `tab_sidebar.rs`, or `switcher.rs`. — PASS
4. **Residual-for-verify matches plan exactly** — report §4 items 1–4 are the
   four Task 4 Step 1 manual sub-steps verbatim (two same-named tabs + split
   parity vs `tty7 tab ls --json` / `tty7 pane ls`; daemon-unreachable `—`
   fallback; strip/sidebar/switcher no-new-rows). Step 2 (headless
   `cargo test`) was executed, not skipped; code-inspection parity for
   focused-leaf/row-order/no-literals/`tab_ordinal`≡`tab_index`/allowlist is
   documented as verified, honestly separated from live-GUI residual. Nothing
   headless-executable was deferred. — PASS
5. **No code modified, no commit** — review package
   `task-4-review-package.md` shows empty range `BASE 0d00289..HEAD 0d00289`
   with no commits, as expected for this evidence-only task. — PASS

## Residual for verify phase (unchanged from report §4)

1. Open two same-named tabs plus a split in the second tab (three panes total).
2. Focus each pane; compare Tab ID / Pane ID / ordinal against
   `tty7 tab ls --json` and `tty7 pane ls`.
3. Daemon-unreachable-at-startup: ordinal shows `—`, ID rows still render.
4. Confirm strip, sidebar, switcher show no new rows.
