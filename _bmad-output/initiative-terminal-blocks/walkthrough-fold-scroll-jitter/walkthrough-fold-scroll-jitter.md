# Walkthrough：折叠后滚轮回滚抖动修复

目标：本地提交 3cc5d1a（fix）+ 9b14f99（chore）。计划见 plan-fold-scroll-jitter-fix.md（状态 done，用户已真机验收通过）。

当前块：Periphery。

- [x] Block 1 — Intent（已走读）
- [x] Block 2 — Broad strokes（已走读）
- [x] Slice A — 可见行步进（已走读）
- [x] Slice B — 折叠切换顶对齐修正（已走读）
- [x] Slice C — Review 补丁（已走读）
- [x] Slice D — 测试覆盖（已走读）
- [ ] Periphery（待走读）

## Block 1 — Intent

> **Problem:** 点击 gutter 折叠很长的数据块后，鼠标滚轮回滚时视口来回抖动：上滚上去马上又下来，下滚同样，多滚一会儿才能顺利滚过去。
>
> **Approach:** 让滚轮步进与钳制按可见行走而非含隐藏的绝对行，折叠切换时修正 offset/frac 使同 offset 命名同一可见集，消除逐帧来回修正。

（逐字引自 plan-fold-scroll-jitter-fix.md 的冻结 Intent 区。）

## Block 2 — Broad strokes

滚轮按绝对行（含隐藏行）步进、窗口按 bottom 锚定：折叠长块时 1 个绝对行可能对应 0 个可见行，offset 加了可见不动，下一帧再修正，看上去就是“上去又下来”。本次改动三处入口：

- [smooth_scroll（view.rs，折叠感知分发）](../../../src/terminal/view.rs#L6800)：有折叠走可见行步进，无折叠/alt-screen/锁争用逐位走老路径。
- [toggle_fold_by_seq（view.rs，翻转后顶对齐修正）](../../../src/terminal/view.rs#L3939)：翻转前读窗口顶 anchor，翻转后把 offset 搬到同一可见集，重置 scroll_frac 并取消在途滚轮动画。
- [step_offset_visible / offset_for_anchor（blocks.rs，纯可见行数学）](../../../src/terminal/blocks.rs#L691)：前者按可见行步进 offset 并报告实际步数，后者算出让 anchor 仍居窗口顶的 offset。

## Slice A — 可见行步进

每一步整数行必落在可见行上，整段隐藏 run 一步跳过，所以 wheel notch 不可能走空（dead notch），连续 notch 单调无回跳；步数不够（撞界）时余量按老钳制语义吃掉 frac。无折叠时行为与老函数逐位一致（有 13 组参数的等价测试 pin 住）。

- [prev/next_visible_row 与 window_edges（blocks.rs）](../../../src/terminal/blocks.rs#L572)：按 span 整段跳过隐藏 run；window_edges 与 paint 用的 map_visible 穷举对拍，步进决策不可能和绘制行打架。
- [step_offset_visible（blocks.rs）](../../../src/terminal/blocks.rs#L691)：返回新 offset + 实际步数。
- [smooth_scroll_step_visible（view.rs）](../../../src/terminal/view.rs#L9819)：frac + delta 累积取整步；反向首 notch 先吃 frac。
- [smooth_scroll 分发（view.rs）](../../../src/terminal/view.rs#L6800)：单次 try_lock 取 guard 复用，既判 has_folds 又借给步进（review 补丁消掉的双锁 TOCTOU 见 Slice C）。

## Slice B — 折叠切换顶对齐修正

toggle 前后同一 offset 命名的是不同可见集（短 map 顶对齐时更明显），不修正的话下一个 wheel 先做来回修正。修正方法是：翻转前记住窗口顶可见行 anchor，翻转后用 offset_for_anchor 把 offset 搬到 anchor 仍居顶的位置；anchor 本身被折叠走时回退到该 span 起始行（即摘要行）。

- [toggle_fold_by_seq（view.rs）](../../../src/terminal/view.rs#L3939)：快照只读 try_lock；修正用 try_lock，grid 争用即跳过修正（toggle 本身照常落地）；tracker 争用则直接返回（沿用旧语义，文档已照实写）。
- [offset_for_anchor（blocks.rs）](../../../src/terminal/blocks.rs#L765)：结果钳在 [0, history] 内。

## Slice C — Review 补丁

quick review 5 项发现：1 false（无 AGENTS.md 可对），4 patch 当场修完，无 loopback。

- 单 guard：smooth_scroll 原先 try_lock 两次（先判折叠再取 guard），第二次可能因 paint/render 争用失败而回退老路径，单 notch 重现抖动；现一次加锁复用。
- 非阻塞修正：toggle 修正分支原用 blocking term.lock()，与“fold 永不等待 grid”注释及计划 Never 相违；现改 try_lock_unfair，争用跳过修正。
- 文档照实：toggle 文档“任一锁争用仍落地”不实（tracker 争用直接返回），can_step_up“无折叠即 offset < history”仅 floor == 0 成立；两处注释已收窄。

## Slice D — 测试覆盖

新增 9 个单测（blocks 7 + view 2），矩阵 5 行全部覆盖且跑过：上滚/下滚单调（visible_steps_walk_monotonically_across_a_fold）、首 notch 生效（visible_step_up_heals_a_dead_offset_in_one_notch）、toggle 顶稳定与回退（offset_for_anchor 两个单测）、短 map 钳制（visible_steps_clamp_at_live_edge_and_birth）、无折叠回归（smooth_scroll_step_visible_matches_legacy_without_folds）。另有 window_edges 与 map_visible 穷举一致性测试。

验证：cargo test --bin tty7-app terminal:: 1137 全过；cargo test -p tty7-core --lib block 32 全过；cargo fmt --check 干净。真机手动检查由用户在 192.168.1.25 完成并验收通过。

## Periphery

- deferred-work.md 记一项：toggle_fold_by_seq 本体接线缺单测（TerminalView 方法需 gpui harness，同既往 mouse 项），待 view harness 落地后补。
- 计划文件 plan-fold-scroll-jitter-fix.md 状态 done；element.rs 零改动（frac 语义与无 sliver 路径保持不变）。
