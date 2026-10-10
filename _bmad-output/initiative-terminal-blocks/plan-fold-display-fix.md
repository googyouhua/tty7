---
title: '折叠显示修复：marker 对齐与顶部空白'
type: 'bugfix'
ticket: ''
created: '2026-10-09'
status: 'built'
baseline_revision: 'bc23b14'
route: 'full'
route_source: 'auto'
risk: 'medium'
review: 'quick'
review_source: 'pinned'
lenses_ran: []
review_loop_iteration: 1
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** 折叠块在滚动时 gutter 箭头与摘要行错位（双箭头不同步）；折叠高块时视口顶部大面积空白。

**Approach:** 网格绘制与 gutter 取同一份 per-frame fold 映射（单源，消分歧）；摘要行去 `▸` 前缀（单箭头，只留 gutter）；fold 映射短于视口时从顶部填充（空白只许在底部）。

## Boundaries & Constraints

**Always:** gutter marker 行 == 摘要行，任何滚动态；短映射顶对齐；无块/无折叠时绘制路径零变更（`visible_map` 返回 None 即老路）；mouse 路径经单行换算函数，快照失配现算回退。

**Never:** 改配对/adopt/持久化任一逻辑；改 prompt 稳定性之外的滚动语义；为修对齐加新锁或跨帧缓存（paint 用快照，超 1 帧 lag 即 bug）；非连续选区（alacritty Selection 不支持，另起故事）；vi 键盘选择路径。

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| 滚动中折叠块 | 折叠 + 任意 display_offset | marker 行 == 摘要行，无错位 | No error expected |
| 折叠高块 | map 短于视口 | 内容顶对齐占满，空白仅底部 | No error expected |
| 无折叠 | 任何滚动 | 与修前逐像素一致 | No error expected |
| 短映射交互 | 选中/搜索/跳转/复制 | 与网格行一致（同函数、同 skip 语义） | No error expected |

</frozen-after-approval>

## Code Map

- `src/terminal/blocks.rs` (`visible_map` 约 525，`screen_to_abs` 约 553，`map_visible` 约 463) -- 分歧嫌疑点：gutter（`view.rs:6880 block_gutter_markers` 自算一份）与 grid（`element.rs:1905` 快照内一份）各算一次、两次 `try_lock`，输出中途到达即分叉；另 `skip = rows - len` bottom-anchor 是顶部空白的直接来源
- `src/terminal/element.rs` (`paint_folded_rows` 约 2320，`paint_summary_row` 约 2373，快照约 1890) -- 网格侧：skip 语义、摘要 `▸ ` 前缀（`fold_summary` 在 `blocks.rs:581`）
- `src/terminal/view.rs` (`render_block_gutter` 约 6795) -- gutter 侧：改读快照里的同一份 map，不再自算；marker 点击行换算同步
- 复现抓手（先写再修）：纯映射单测——同 spans+同 display_offset 下 gutter 行 vs 网格行逐行比对；短 map 断言顶对齐无顶部空白。headless 可跑，不依赖 gpui 真渲染

## Tasks & Acceptance

**Execution:**
- [ ] 复现单测（先红）：滚动+折叠下 marker 行 vs 摘要行；短 map 顶部空白
- [ ] 单源映射：快照产出 map，gutter 只读不用算；两处不再各锁各算
- [ ] 摘要去 `▸` 前缀（gutter 留箭头）；短 map 顶对齐（删 skip bottom-anchor，文档同步改）
- [ ] 交互回归：选中/搜索/跳转/复制/折叠开关走同映射（单测不断言 GUI，只断映射一致）
- [ ] 折叠感知 mouse（扩 scope，用户已批语义）：单行换算函数接全部 mouse 入口（选中起/拖/滚、链接 hover/click、右键 latch、jump 定点 walk 经 hidden_abs_in）；真 Term+scroll 单测锁映射；vi 路径不动

**Acceptance Criteria:**
- Given 折叠块 + 任意滚动偏移， when 取 marker 行与摘要行， then 相等
- Given map 短于视口， when 绘制， then 顶部无空白行（内容顶对齐）
- Given 无折叠， when 逐帧比对修前， then 网格输出一致（既有单测全过即证）

## Design Notes

- bottom-anchor 改顶对齐的影响面：map 满时 skip=0，新老完全一致；仅短 map 时行为变（prompt 上移但仍可见，无内容被藏）。注释里“prompt stays”论述同步改。
- 单箭头取舍：gutter marker 是点击目标必须留；摘要 `▸` 与之重复且是错位显眼包，去之。

## Verification

**Commands:**
- `cargo test --bin tty7-app block` -- expected: 全过，含新增对齐/顶对齐单测
- `cargo test -p tty7-core --lib block` -- expected: 全过（daemon 未动，回归）
- `cargo fmt --check` -- expected: 干净

**Manual checks (if no CLI):**
- 真机滚动+折叠目检 marker/摘要同行；折叠高块顶部无空白

## Review Triage Log

- F1 selection identity — pre-existing (unfolded 映射本 repo 从未存在，非回归）。路由 intent_gap：矩阵 row 4 点名选中，正确行为需人类定（折叠上拖选到底选中什么？可见行还是含隐藏？）。
- F2 link identity — 同上，intent_gap（并入 F1 决策）。
- F3 jump identity — 同上，intent_gap（并入 F1 决策）。三者修其一即新功能，非 bugfix。
- F4 snapshot 跨帧 — trade-off：单帧共享即要求 render 读 paint-1 快照，天然 ≤1 帧 lag，静止收敛；旧式双锁分叉是系统性错行。路由：改 Never 措辞为“≤1 帧 lag，静止收敛”（需人类点头）。
- F5 首帧空 markers — reject（transient + 有注释 + 比旧错行安全）。
- F6 测试绕开 display_offset — patch（用真 Term+scroll 的 visible_map 重做 scroll 断言；paint buffer 循环 hors scope，行号算术已同函数覆盖）。

## Review Triage Log

- F1/F2/F3 selection/link/jump identity — pre-existing gaps, user approved scope expansion 2026-10-10: fold-aware mouse semantics (single shared row function; snapshot validated, live fallback; selection reads grid verbatim incl. hidden = parity; links land painted rows; jump fixed-point via hidden_abs_in; vi keyboard untouched).
- F4 snapshot lag — amended Never to "≤1 帧 lag，静止收敛" (user-approved reading of single-frame sharing).
- F5 首帧空 markers — rejected (transient + documented + safer than wrong rows).
- F6 weak scroll tests — patch (real-Term visible_map + display_offset assertions).

## Review Triage Log (round 2)

- Q1 snapshot 未验 fold 态 — medium — toggle 夹在 paint 与 click 之间即 stale。Patch：snapshot 存 spans_len + folded 集合，mouse path 全量比对；失配走 live（已有锁）或 None。
- Q2 contention 文档失实 — low — tracker 忙时回 identity 再查 span_at 可能锁错块，与“latches nothing”矛盾。Patch：忙即 None（transient，下一事件即好），注释照实。
- Q3 mouse/jump 无单测 — patch：fold_aware_jump_target 取 Term 即 mutant 可测（live Term + folds + scroll 断言定点）；mouse_grid_line 本体无 harness，沿 P5/P6 记 deferred（同“缺 block 覆盖”条）。

## Review Triage Log (round 2 fixes applied)

- Q1/Q2 fixed: fold_epoch（spans len + folded seqs）存快照，mouse 全量比对；忙即 None；docstring 照实。terminal:: 1125 全绿。
- Q3 fixed: jump walk 提纯为 BlockTracker::jump_target_for + live-Term 定点单测；mouse_grid_line 本体无 harness，记入 deferred（与 P5/P6 同条，待 view harness）。
