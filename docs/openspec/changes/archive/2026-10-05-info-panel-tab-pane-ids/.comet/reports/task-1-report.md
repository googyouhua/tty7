# Task 1 Report: i18n labels (tasks.md 1.1)

## Status
GREEN — committed.

## Commit
- `0ab2d6291a7b2248aefc3f942710ce2911808f26`
- Message: `feat(info-panel): add Tab ID / Pane ID / tab ordinal i18n keys`

## Changed files (4, +12 lines)
- `src/ui/i18n/mod.rs` — added `L10nKey::PanelTabId`, `PanelPaneId`, `PanelOrdinal` variants after `PanelSsh`.
- `src/ui/i18n/en.rs` — `"tab id"`, `"pane id"`, `"tab"`.
- `src/ui/i18n/zh.rs` — `"标签页 ID"`, `"窗格 ID"`, `"标签页"`.
- `src/ui/i18n/ja.rs` — `"タブ ID"`, `"ペイン ID"`, `"タブ"`.
- `KEPT_IN_ENGLISH` untouched (new keys not added, per plan).

## TDD
- Skill `test-driven-development` loaded and followed.
- RED: added only the three enum variants first, then ran `cargo test -p tty7 i18n`.
  Failure: compile error — 3× `E0004` non-exhaustive patterns in the `translate_en` /
  `translate_zh` / `translate_ja` matches (`PanelTabId`, `PanelPaneId`, `PanelOrdinal`
  not covered), so the existing locale consistency test
  (`every_key_is_translated_in_every_locale`) could not build. This compile failure
  is the RED signal (as the plan anticipates).
- GREEN: added the exact en/zh/ja arms from the plan, then re-ran `cargo test -p tty7 i18n`.
  Pass: 7 passed, 0 failed (`english_is_the_complete_table`,
  `japanese_covers_every_key`, `explicit_languages_select_the_right_locale`,
  `wording_that_names_a_platform_names_this_one`, `chinese_covers_every_key`,
  `every_key_is_translated_in_every_locale`,
  `plural_and_select_branches_are_translated`; 2506 unrelated tests filtered out).
  The consistency test confirms all three locales resolve every new key and none is
  byte-identical to en outside the allowlist.
- No production code was written before the failing run; no other files touched;
  no plan/OpenSpec checkboxes touched.

## Concerns
none.

## Risk signals
no risk signals (4-file, 12-line additive i18n change; no cross-module logic,
no security surface, no concurrency, no migration, no public API change).
