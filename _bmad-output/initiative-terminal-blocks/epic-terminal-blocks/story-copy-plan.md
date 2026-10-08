---
title: '整块纯文本复制三项'
type: 'feature'
ticket: 3
created: '2026-10-08'
status: 'built'
baseline_revision: 'b06ab5cf36ba32d442132d64fb12aa2377ad6e87'
route: 'full'
route_source: 'auto'
risk: 'low'
review: ''
review_source: ''
lenses_ran: []
review_loop_iteration: 0
followup_review_recommended: false
context: []
warnings: []
deferred: []
---

<intent-contract>

## Intent

**Problem:** 右键目前只有“复制两者”一项，缺复制命令、复制输出两项，且默认主项语义未定；折叠态必须复制全块而非可见行。

**Approach:** 在 1.1 的 `block_text`（已与 `capture --plain` 同源 `bounds_to_string`）上拆出命令/输出/两者三项，菜单补齐三项（两者为默认主项），三项统一带退出码行，无 ANSI。

## Boundaries & Constraints

**Always:** 纯文本经 `Term::bounds_to_string`（与 `capture --plain` 同函数）；折叠态复制全块（读 grid 绝对行，不读可见映射）；无 ANSI；无块光标下三项隐藏（沿 1.1 空菜单抑制先例，不做 disabled 置灰）；block schema 与退出码行格式沿用 1.1（`exit <code>`，未知为 `exit ?`）。

**Never:** ANSI 复制；快捷键；outline；搜索联动；自动折叠；持久化接线（entry-5）；远端特殊处理（entry-6）；改动 daemon 侧配对语义。

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| 复制两者（默认主项） | 块内右键选两者 | 剪贴板得命令+输出+退出码行 | 无块时菜单无此项 |
| 复制命令 | 块内右键选命令 | 剪贴板得首逻辑行+退出码行 | 无块时菜单无此项 |
| 复制输出 | 块内右键选输出 | 剪贴板得首行之后各行+退出码行 | 无输出时得仅退出码行；无块时无此项 |
| 折叠态复制 | 块已折叠后复制任一项 | 与展开态逐字一致（全块） | No error expected |
| 换行包裹 | 命令/输出含终端自动换行 | 与同范围 `bounds_to_string` 逐字一致 | No error expected |
| 已淘汰块 | span 行已不在 grid | 不写剪贴板 | 返回 None，不 panic |

</intent-contract>

## Code Map

- `src/terminal/blocks.rs` (`block_text` 约 462，`trim_trailing_spaces` 约 493) -- 拆分落点：新增 `block_command_text` / `block_output_text`（或等价 `block_parts`），复用 `block_text` 的 range+trim+exit 逻辑；命令=网格文本首逻辑行，输出=其余行；`bounds_to_string` 保证包裹行已连接，拆分按 `\n` 切首行即包裹安全。
- `src/terminal/view.rs` (`actions!` 约 119，`copy_block` 约 3854，菜单约 8531，`on_action` 约 8403) -- 新增 `CopyBlockCommand` / `CopyBlockOutput` Action 与 `copy_block_command` / `copy_block_output` 方法（与 `copy_block` 同 latch 语义），菜单在 `menu_block.is_some()` 分支补齐三项，两者排首为默认主项。
- `src/ui/i18n/mod.rs` (约 1396) + `src/ui/i18n/en.rs`/`ja.rs`/`zh.rs`/`ru.rs` -- 新增 `TerminalBlockCopyCommand` / `TerminalBlockCopyOutput` 键，四语言各一行。
- `crates/tty7-cli/src/screen.rs` (`render_segment` 约 85) -- 只读约束：`capture --plain` 同源证据（`bounds_to_string`），不改。
- `src/terminal/remote.rs` (`note_block_cut` 约 3891) -- 只读约束：B 开/D 闭语义已由 1.1 拥有，不改。

## Tasks & Acceptance

**Execution:**
- [x] `src/terminal/blocks.rs` -- 新增命令/输出拆分 helper（复用 `block_text` 的 clamp/range/trim/exit 路径，命令取首逻辑行、输出取其余，三项统一附退出码行，evicted 返回 None） -- 三项同源、无 ANSI、折叠无关
- [x] `src/terminal/blocks.rs` -- I/O 矩阵边角单测（两者/命令/输出含退出码行、空输出仅退出码行、包裹行逐字一致、折叠态与展开态一致、evicted 为 None） -- 可验边角
- [x] `src/terminal/view.rs` -- 新增 2 Action 与 2 copy 方法（同 `menu_block` latch、无块返回、空文本不写剪贴板、trim 取 `clipboard_trim_trailing_spaces`） -- 菜单行为与既有项一致
- [x] `src/terminal/view.rs` -- 右键菜单补齐三项（两者排首为默认主项，无块隐藏） -- CAP-3 入口完整
- [x] `src/ui/i18n/*` -- 新增两键四语言文案 -- 无缺键、无回退英文

**Acceptance Criteria:**
- Given 块内任意位置， when 右键复制两者， then 剪贴板得纯文本命令+输出+退出码行
- Given 块内任意位置， when 右键复制命令， then 剪贴板得命令首行+退出码行
- Given 块内任意位置， when 右键复制输出， then 剪贴板得输出各行+退出码行（无输出时仅退出码行）
- Given 块已折叠， when 复制任一项， then 与展开态逐字一致
- Given 输出含终端自动换行， when 复制整块， then 与同范围 `bounds_to_string` 逐字一致
- Given 无块光标处， when 右键， then 无三项（隐藏非置灰）

## Implementation Notes

- full 路由但无 subagent 可用（本环境无派发工具），按顶层“绝不 halt”指令由主会话直接实现，等价 oneshot 执行；plan 内容未动 intent-contract。
- 切分实现：`block_grid_text` 抽取既有 range+trim 逻辑，`block_text`/`block_command_text`/`block_output_text` 皆经它再统一 `with_exit_line` 附退出码行；切分按已连接包裹后的逻辑行切首行，包裹命令不断裂。
- `view.rs` 三项共享 `copy_block_with` latch（同 `menu_block`、空文本不写剪贴板）；菜单两者排首为默认主项；新增 `CopyBlockCommand`/`CopyBlockOutput` Action。
- i18n 两键×四语言（en/zh/ja/ru；全仓仅此四种 locale 文件）。
- 文件：`src/terminal/blocks.rs`、`src/terminal/view.rs`、`src/ui/i18n/{mod,en,zh,ja,ru}.rs`。
- 版本卫生：worktree `_bmad-output` symlink 致 git 恒显 14×D + 2×??，先于本变更存在；commit 时仅显式 `git add` 上述 7 个代码文件。

- 版本卫生异常（先于本 plan 存在，非本变更引入）：worktree 内 `_bmad-output` 为指向主仓共享 ticket 树的 symlink，而 git 基线将其内容作为普通文件跟踪，故 `git status` 在本分支恒显示 14 个 `D _bmad-output/...` + `?? _bmad`/`?? _bmad-output`；`git add --refresh -- .` 退出 0。本 plan 按顶层指令推进不 halt，此异常不纳入本次 commit（只提交代码文件）。
- 语义拍板（ticket 措辞“统一带退出码行”）：三项统一附退出码行（含成功 `exit 0`），而非仅两者带；单项同样可独立粘贴出完整可执行证据。
- 默认主项：两者排首位；gpui 原生菜单无 default-item API，不另造视觉强调。
- 命令/输出切分：按 `bounds_to_string` 已连接包裹后的逻辑行切首行，而非按网格行切，保证包裹的命令不断裂。
- 1.1 连续性：直接复用其 `block_text` 路径与 `menu_block` latch，不重复验证 1.1。
- 自动执行结果（2026-10-08，wt/tb-1.3 接手续跑）：`export CARGO_BUILD_JOBS=2; cargo test -p tty7 blocks` → 23 passed / 0 failed，EXIT 0（含 3 新增：`block_parts_split_…`、`block_output_without_output_rows_…`、`block_copies_of_a_folded_span_…`）；`cargo test -p tty7 i18n` → 8 passed / 0 failed，EXIT 0（其余 warning 均为无关文件的 pre-existing 项）。`cargo build -p tty7` → EXIT 0（重启后补跑通过）。
- 矩阵审计结论：I/O 六行全部命中实现+单测（两者/命令/输出三项同源退出码行；空输出仅退出码行；包裹命令不断裂；折叠态绝对行读取；evicted 全 None 且 `copy_block_with` 不写剪贴板；无块隐藏）。Never 约束干净：无 ANSI（`bounds_to_string`）、无快捷键（改动仅 7 文件、无 keymap）、`screen.rs`/`remote.rs` 只读未改、daemon 未碰。
- 过程备注：两次 server 重启分别中断了前台验证（编译超时）与后台验证（被 cancel）；最终以后台重跑 EXIT 0 为准，target 缓存使 i18n 轮次快速通过。

## Plan Change Log

- 2026-10-08：实现完成并验证通过，status in-progress → built；5 执行任务全勾选。

## Review Triage Log

## Design Notes

切分示例（grid 文本 `"$ cmd\nout1\nout2"`，exit 0）：两者=`"$ cmd\nout1\nout2\nexit 0"`；命令=`"$ cmd\nexit 0"`；输出=`"out1\nout2\nexit 0"`；无输出时输出项=`"exit 0"`。

## Verification

**Commands:**
- `export CARGO_BUILD_JOBS=2; cargo test -p tty7 blocks` -- expected: 全过，含新增拆分单测
- `export CARGO_BUILD_JOBS=2; cargo test -p tty7 i18n` -- expected: 全过（如存在该目标；否则以编译通过为准）
- `export CARGO_BUILD_JOBS=2; cargo build -p tty7` -- expected: 成功

**Manual checks (if no CLI):**
- GUI 块内右键见三项、两者首位；粘贴含命令、输出与退出码行；折叠后复制一致
