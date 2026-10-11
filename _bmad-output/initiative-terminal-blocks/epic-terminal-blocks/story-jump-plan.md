---
title: '跳块首与菜单折叠'
type: 'feature'
ticket: 2
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
context: []
warnings: []
deferred: []
---

<intent-contract>

## Intent

**Problem:** 1.1 菜单只有跳块首与复制两者，折叠/展开只能点 gutter 窄条；鼠标用户在块内右键无法折叠，且需保证悬停与 gutter 不遮挡输出首行。

**Approach:** 右键菜单补折叠/展开切换项，与 gutter 共用同一 toggle_fold_by_seq 路径、文案随折叠态切换；跳块首保持一次点击视口置块首（折叠/展开均可）；gutter 保持 marker 绘制、不加悬停遮挡。

</intent-contract>

## Tasks & Acceptance

**Execution (reference, oneshot):**
- [x] `src/terminal/view.rs` -- 新增 ToggleBlockFold action 并接通 on_action，菜单在 menu_block 存在时追加折叠/展开项（文案随 is_folded 切换），复用 toggle_fold_by_seq；跳块首逻辑不动 -- 菜单与 gutter 同行为
- [x] `src/ui/i18n/mod.rs` + `en.rs`/`zh.rs`/`ja.rs`/`ru.rs` -- 新增折叠/展开两个 L10nKey 及四语翻译 -- 菜单文案可本地化
- [x] 悬停/gutter -- 不新增 hover 背景与 tooltip，gutter 保持 marker paint only（BLOCK_GUTTER_W=14 覆盖网格首列约 6px，仅标块首行），折叠摘要行沿用 paint_folded_rows -- 不遮挡输出首行

**Acceptance Criteria:**
- Given 块内任意位置， when 右键选跳到块首， then 视口一次点击置于块首命令行（折叠态亦可）
- Given 块内右键菜单， when 点折叠/展开， then 与点 gutter marker 等效（同一 seq 切换）
- Given 无块光标下， when 右键， then 无块菜单项（沿 1.1 隐藏先例）
- Given gutter marker 悬停， when 观察输出首行， then 无遮挡（无 hover 背景/tooltip 盖住首列文本）

## Implementation Notes

- Route oneshot + reason: 预估改动约 60-80 行（action 1 + on_action 1 + 菜单约 12 行 + i18n 约 10 行 + 单测约 20 行），i18n 为机械增补；直接实现、不起 subagent。
- 连续性（ticket 1 done，基线 b06ab5c 直接读不重复验证）：block schema（id/exit_code/folded/truncated）与退出码行 `exit <code>` 由 1.1 拥有，本 ticket 只消费；jump/copy 经 menu_block latch，gutter 经 toggle_fold_by_seq，alt-screen 下 block_at_screen_row 为 None。
- 版本控制备注：本 worktree 用 symlink `_bmad -> /ws/tty7/_bmad`、`_bmad-output -> /ws/tty7/_bmad-output` 指向主检出统一 ticket 树，`git status` 在此 worktree 恒显示 `_bmad-output/*` 删除 + 未跟踪二 symlink；`git add --refresh -- .` 退出 0 但树不干净。此为环境预设（parent 指令），按 parent 覆盖推进、不 halt。
- 悬停语义备注（不确定处按 plan 推进）：ticket/spec 未定义专用 hover 浮层；现状 gutter 无 hover 背景/tooltip，link hover 仅改 underline 不盖文本。本 ticket 不新增任何 hover 浮层，以此满足“悬停不遮挡输出首行”；若另有 hover 浮层定义，留后续 ticket。
- Subagent 备注：full/review 要求起 subagent，本环境无 subagent 工具；按 parent 无人值守覆盖，oneshot 直接实现 + 自查。
- 矩阵审计（代码核查，2026-10-08）：
  - AC1 跳块首：`jump_to_block_start` 经 `block_span(seq)` 解析 latch 的 seq，与折叠态无关，一次 `scroll_display(Delta)` 置块首；实现未动。
  - AC2 菜单折叠等效 gutter：菜单 `ToggleBlockFold → toggle_menu_block_fold → toggle_fold_by_seq(seq)`；gutter marker 点击同调 `toggle_fold_by_seq(seq)`（同 seq），等效 by construction；文案按 `block_folded(seq)` 切 Fold/Unfold；新增 `fold_toggle_round_trip_returns_to_unfolded` 单测覆盖 toggle 路径。
  - AC3 无块隐藏：`menu_block: None` 分支原样返回 menu，无新增项（hide never grey 沿 1.1）。
  - AC4 悬停不遮挡：本 ticket 零 gutter/hover 改动（diff 仅菜单+i18n+单测）；`BLOCK_GUTTER_W=14` 不变，hover 机制仅 link underline，无新增浮层/tooltip。

## Auto Run Result

- `export CARGO_BUILD_JOBS=2; cargo check -p tty7` -- 成功（exit 0，2m25s；35 warnings 均为存量：LoopbackPlan/manual_cwd/t_select 等未触及行，本 ticket 零新增警告）
- `export CARGO_BUILD_JOBS=2; cargo test -p tty7 blocks` -- 全过（21 passed / 0 failed，含新增 `fold_toggle_round_trip_returns_to_unfolded`；另 2519 个非 blocks 测试被 filter 跳过）
- 手工项留 walkthrough：块内右键跳块首一次点击置块首；折叠/展开与 gutter 等效；无块处无此两项；marker 悬停不盖首行文本

## Plan Change Log

## Review Triage Log

## Verification

**Commands:**
- `export CARGO_BUILD_JOBS=2; cargo test -p tty7 blocks` -- expected: 全过（折叠切换逻辑回归）
- `export CARGO_BUILD_JOBS=2; cargo check -p tty7` -- expected: 成功（i18n key 全语言齐备，零警告外仅存量）

**Manual checks (if no CLI):**
- 块内右键：跳块首一次点击视口置块首；折叠/展开与 gutter 等效；无块处无此两项；marker 悬停不盖首行文本
