---
title: '失败红标与 Ctrl-C 成块'
type: 'feature'
ticket: 4
created: '2026-10-08'
status: 'done'
baseline_revision: 'b06ab5cf36ba32d442132d64fb12aa2377ad6e87'
route: 'oneshot'
route_source: 'auto'
risk: 'low'
review: ''
review_source: ''
lenses_ran: []
review_loop_iteration: 0
followup_review_recommended: false
context:
  - _bmad-output/initiative-terminal-blocks/spec-terminal-blocks/interaction.md
  - _bmad-output/initiative-terminal-blocks/spec-terminal-blocks/block-model.md
warnings: []
deferred: []
---

<intent-contract>

## Intent

**Problem:** 失败与 Ctrl-C 中断的命令块在 gutter 上与成功块无视觉区分，用户无法一眼辨别；且失败提示的悬停形态必须保证不遮挡块输出首行（spec interaction.md 失败态条）。

**Approach:** BlockSpan 已有 exit_code（含 130 中断码，B→D 配对在 daemon 与 client 两侧均已闭合），本 change 只做表现层：非零退出块的 gutter marker 染红并在块首行加红条（块首定位），Ctrl-C（D;130）沿既有配对成块并同样标红；marker 悬停只做无覆盖式强调（不变色块、不弹遮挡首行的浮层），右键菜单项保持不变。

</intent-contract>

## Implementation Notes

- Route rationale (oneshot): 纯表现层改动，预计 prod 改动 <100 行（view.rs gutter 渲染 + blocks.rs 一个 failed 判定 helper），无协议、无状态机、无新通道；本环境无 subagent 可用，直接实现。
- 1.1 连续性（ticket 1，story-tracer-plan.md done）：复用 `BlockSpan.exit_code` 与 `exit <code>` 行格式，不改 schema；B→D 配对语义（含无 C 配对、130 中断成块）已由 1.1 覆盖，不重做。
- 环境备注：本 worktree 根的 `_bmad` / `_bmad-output` 为指向主仓的 symlink，`git status` 显示其 tracked 文件为 deleted，但 `crates/` `src/` `tests/` 干净；VCS 判定以代码树为准，不因此 halt。
- Ctrl-C 成块的传输事实：中断时 shell 集成下发 `D;130`，daemon `note_block_marks` 与 client `BlockTracker::note_d` 均已按任意 exit_code 闭合（1.1 单测 `interrupt_exit_code_is_kept`、`close_without_c` 为证）；本 ticket 不碰该路径，只加红标表现 + 补断言。
- 实现（wt/tb-1.4，未提交断点续做完成）：`src/terminal/blocks.rs` 新增 `BlockSpan::is_failed()`（`exit_code.is_some_and(|c| c != 0)`：非零即失败、含 130；`None` bare-D 永不标红）+ 单测 `only_nonzero_exit_reads_as_failed`（1/130→true，0/None→false）；`src/terminal/view.rs` `block_gutter_markers` 返回 `(seq, folded, failed)`，`render_block_gutter` 失败时 marker `text_color(danger)` + 条带左侧 3px `bg(danger)` 红条（`.when(failed)`，成功块零改动），hover 仅 `.underline()` glyph 自身、无 overlay（首行不被遮挡）；右键菜单零改动（diff 内无 menu 相关行，失败/成功菜单项一致）。
- 矩阵审计（CAP-4 × spec interaction.md 失败态）：块首红条 ✓（3px danger bar 定位块首 marker 行）、菜单项不变 ✓（menu_block 分支无失败特化）、悬停不遮挡首行 ✓（glyph-only underline）、Ctrl-C 成块且标红 ✓（配对路径未碰 + 130 走 is_failed）。`block_gutter_markers` 仅一处调用；`cx.theme().danger` 与代码库既有用例一致。

## Auto Run Result

- `cargo test -p tty7-core block`（CARGO_BUILD_JOBS=2）：ok，16 passed / 0 failed（含 daemon 侧 `command_blocks_record_interrupt_exit_codes`）。
- `cargo test --bin tty7-app block`：ok，42 passed / 0 failed（含新增 `only_nonzero_exit_reads_as_failed` 与既有 `interrupt_exit_code_is_kept`）。
- `cargo test --bin tty7-app gutter`：ok，17 passed / 0 failed，无回归。
- `--bin tty7-app` 编译成功（随 test 附带编译；plan Verification 允许以此为准）。
- 走读替代目检：`render_block_gutter` 失败红 marker + 块首红条、成功无变化、hover 无覆盖；GUI 目检不可行（headless），风险见遗留。

## Plan Change Log

## Review Triage Log

## Verification

**Commands (each run with `export CARGO_BUILD_JOBS=2` first):**
- `cargo test -p tty7-core block` -- expected: 全过（含 daemon 侧 D;130 成块既有单测）
- `cargo test --bin tty7-app block` -- expected: 全过（含新增：非零红标判定、130 标红、bare-D 不标红单测）
- `cargo test --bin tty7-app gutter` -- expected: 全过（gutter marker 相关既有单测无回归；若无此名测试则以 block 行为准）
- `cargo build --bin tty7-app` -- expected: 成功（若过重则以 test 附带的编译为准）

**Manual checks (headless，GUI 无法目检，以单测 + 代码走读代替):**
- 走读 `render_block_gutter`：失败块 marker 为红色、块首行有红条；成功块无变化；hover 态无覆盖首行的 overlay（只有 marker 自身强调或下偏 tooltip，首行 grid cell 不被遮挡）。
- 右键菜单：失败块与成功块菜单项一致（跳块首、复制两者），无新增/隐藏项。
