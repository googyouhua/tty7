# Task 2 Report — Tab ID and Pane ID rows (tasks.md 1.2)

## Status
DONE_WITH_CONCERNS (concerns listed below; all plan requirements met)

## Commit
- `4d35d962882f6e254132fdefbfe367197d35d63f` — `feat(info-panel): render Tab ID and Pane ID rows with copy tiles`
- Command used exactly as specified: `git add src/ui/right_panel.rs && git commit -m "..."`. Not pushed.

## Changed files
- `src/ui/right_panel.rs` only (+29 lines, no deletions):
  - `render_panel_info`: two pushes inserted immediately after the follow-switch `if` block's
    closing brace, before the `if let Some(cwd)` close / shell comment block — verbatim plan code:
    `InfoRow::text(t(L10nKey::PanelTabId), tab.tree_id.get().to_string()).copyable()` then
    `InfoRow::text(t(L10nKey::PanelPaneId), view.pane_id.to_string()).copyable()`.
    Both stay inside the existing `if let Some(leaf)` block; `InfoValue::Text`, no
    `reveal`/`edit_cwd`/`toggle_follow`; raw copy payloads (uuid / decimal); Tab ID then Pane ID.
    The ordinal slot (between the Pane ID push and the shell block) is left empty for Task 3.
    `rows.is_empty()` early return and all other surfaces untouched.
  - `mod tests`: `id_rows_carry_raw_values_as_copy_payloads` unit test, verbatim from the plan.
- Verified preconditions: `L10nKey::PanelTabId`/`PanelPaneId` exist (`src/ui/i18n/mod.rs:1121-1122`)
  with en/zh/ja strings (Task 1 commit `0ab2d62` present as ancestor).

## Test summary
- `cargo test -p tty7 right_panel`: 8 passed, 0 failed — includes new
  `ui::right_panel::tests::id_rows_carry_raw_values_as_copy_payloads` plus all 7 pre-existing
  tests (`a_row_lights_up_only_when_there_is_something_behind_it`, port tests, rtt, copyable, counts).
- RED run (test added, production pushes not yet applied): 8 passed, 0 failed — the new
  construction test passes pre-change because `InfoRow::text`/`.copyable()` already exist.
  This is the outcome the plan itself predicts ("it passes on construction alone, so its real
  gate is Step 4"); the plan's Step 4 gate (full `right_panel` suite green with the pushes wired)
  is what was verified.
- GREEN run (pushes applied): 8 passed, 0 failed. Committed only when GREEN.

## Focus-following regression test: code inspection (no reused fixture)
No `#[gpui::test]` was added. Rationale, per the plan's Step 3 rule ("only if a multi-pane fixture
exists ... otherwise verify by code inspection ... do NOT invent a harness"):
- Grepped `src/ui/app.rs` split tests. The only multi-pane-in-one-tab fixture is `watched_split`
  (`src/ui/app.rs:14518`), which is a **private** fn inside `app.rs`'s test module, and it depends on
  `watch_pane_focus` (`src/ui/app.rs:11222`), also private to `app.rs`. Neither is reachable from
  `right_panel.rs`'s test module without touching `app.rs` (forbidden scope) or copying the
  fixture into `right_panel.rs` (inventing a harness — forbidden). The `pub(crate)` helpers
  (`test_window::harness`, `quiet_test_pane`) are single-pane/tab-level only. So no fitting
  verbatim-reusable fixture exists.
- Code inspection against the two source lines (now `src/ui/right_panel.rs:997-998`):
  - Tab ID reads `tab.tree_id.get()` where `tab` binds at line 948 from
    `self.tabs.get(self.active)` — the tab, not the leaf. Refocusing a split changes leaves, never
    the tab, so Tab ID stays put. `tree_id: Cell<TabId>` confirmed at `src/ui/app.rs:521`.
  - Pane ID reads `view.pane_id` where `view` binds at line 950 from `leaf.read(cx)` with
    `leaf = tab.detail_pane(window, cx)` (line 949) — the same `view` the cwd row uses (line 955).
    `detail_pane` (`src/ui/app.rs:660-669`) returns `focused_leaf(window, cx).or_else(focus_target)`,
    so refocusing a split returns the newly focused leaf and Pane ID follows it. `pane_id: u64`
    confirmed as the CLI's `%n` value (`src/terminal/view.rs:307`).
- Manual refocus verification is deferred to the Task 4 manual pass (owns the live Pane ID check).
- Note: the plan says to "record which was done in the commit message", but the commit message is
  fixed verbatim by the task (`feat(info-panel): ...`), so the record lives here instead; the
  message was kept exact.

## Concerns
1. TDD letter vs plan: the row-construction test cannot fail pre-change (it pins the pre-existing
   `InfoRow` helper contract the pushes rely on), so no literal RED→GREEN cycle exists for the two
   pushes without inventing render-inspection machinery the plan forbids. Followed the plan's
   prescribed flow instead: test written first and run pre-change, production code added after,
   full suite green before commit.
2. Placement nuance: the pushes sit inside the `if let Some(cwd)` block (directly after the
   follow-switch close, per the plan's literal "immediately after ... ~line 996, before the shell
   comment ~line 998"), so on a pane with no cwd yet the ID rows are absent until the first cwd
   report. Matches the plan text; flagging in case Task 3's executor wants leaf-level placement —
   if so, move all three ID rows together.

## Risk signals
No risk signals (single-module change in `src/ui/right_panel.rs`, +29 lines, no concurrency,
no migration, no public API change, no security-sensitive surface).

## Fix note (coordinator ruling)
- Moved the two ID pushes out of the `if let Some(cwd)` block to leaf-block level: immediately
  after the cwd block's closing brace, before the `// A pane on the default shell ...` comment.
  Both `tab` and `view` remain in scope (used by the shell/ssh rows below). ID rows now render
  whenever a leaf exists, independent of the first cwd report. Ordinal slot (between Pane ID push
  and shell block) still empty for Task 3.
- Commit `435decc13c8e17f290f58d8ae2a42c60aad42466` — `fix(info-panel): show ID rows independent
  of cwd report` (src/ui/right_panel.rs only, 2+/2-). `cargo test -p tty7 right_panel`: 8 passed,
  0 failed. Concern #2 from the original report is resolved by this move.
