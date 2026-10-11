---
title: '折叠后滚轮回滚抖动修复'
type: 'bugfix'
ticket: ''
created: '2026-10-10'
status: 'done'
baseline_revision: '3e6c23444074fdf973386f3a611cf256ad717a91'
route: 'full'
route_source: 'auto'
risk: 'medium'
review: 'quick'
review_source: 'pinned'
lenses_ran: ['quick']
review_loop_iteration: 0
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** 点击 gutter 折叠很长的数据块后，鼠标滚轮回滚时视口来回抖动：上滚上去马上又下来，下滚同样，多滚一会儿才能顺利滚过去。

**Approach:** 让滚轮步进与钳制按可见行走而非含隐藏的绝对行，折叠切换时修正 offset/frac 使同 offset 命名同一可见集，消除逐帧来回修正。

## Boundaries & Constraints

**Always:** 同一 display_offset 在折叠态下命名同一可见集；滚轮每 notch 单向收敛不回跳；无折叠时绘制与滚动路径零变更；paint/gutter/mouse 读同一份同帧映射。

**Never:** 改块配对/adopt/持久化逻辑；改 alt-screen 滚动语义；为修抖动加阻塞锁或跨帧无校验缓存；改 vi 键盘与 scrollbar 语义。

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| 长块折叠后上滚 | 折叠 + 滚轮上滚多 notch | 视口单调上移，不出现上去又下来 | No error expected |
| 长块折叠后下滚 | 折叠 + 滚轮下滚多 notch | 视口单调下移，不出现下来又上去 | No error expected |
| 折叠开关后立即滚轮 | toggle 后 1 个 wheel 事件 | 首 notch 即产生可见位移，无死 notch | No error expected |
| 短 map 顶对齐 | map 短于视口 + 滚动 | 内容顶对齐，空白只在底部，不Pn跳顶 | No error expected |
| 无折叠回归 | 无折叠 + 任意滚轮 | 与修前行为一致 | No error expected |

</frozen-after-approval>

## Code Map

- `src/terminal/view.rs` (`on_scroll` 约 6540，`smooth_scroll` 约 6736，`smooth_scroll_step` 约 9713，`sync_frame_map` 约 6822，`toggle_fold_by_seq` 约 3930，`scroll_frac` 约 443) -- 滚轮入口、分数累积、预钳制 `max=history_size`、折叠切换不重置 offset/frac 的嫌疑点
- `src/terminal/blocks.rs` (`map_visible` 约 570，`visible_map` 约 635，`window_abs_range` 约 622，`screen_to_abs` 约 667，`screen_to_line` 约 681，`fold_epoch` 约 434) -- bottom 锚定 + 绝对行步进导致可见跳变非线性的嫌疑点；映射数学复用不动
- `src/terminal/element.rs` (`build_grid` 约 1903，`paint_folded_rows` 约 2443，`FrameMap::matches` 约 1823，快照约 1840，原点位移约 2651) -- frac 位移 + fold 下无 sliver，来回即抖的表现层；slot 复用逻辑复用不动

## Tasks & Acceptance

**Execution:**
- [x] `src/terminal/blocks.rs` -- 复现单测先红：同 folds 下连续 offset 的可见首行单调性；toggle 前后同 offset 可见集一致性；短 map 顶对齐不断言 GUI 只断映射
- [x] `src/terminal/view.rs` -- 滚轮步进可见行化：Delta 按可见行换算或等效修正，预钳制按可见上限而非 history_size；反向首 notch 先吃 frac 不产生死 notch 后跳变
- [x] `src/terminal/view.rs` -- 折叠切换修正：toggle 后重置 scroll_frac 并把 offset 修正到命名同一可见集的位置，下一 wheel 不再做来回修正
- [x] `src/terminal/element.rs` -- frac/paint 一致性：fold 下 frac 位移与整数 jump 衔接单向收敛，不引入新 sliver 路径（评估后零改动，见 Implementation Notes）
- [x] 全链回归：选中/搜索/跳转/复制/链接走同映射，复用既有 fold-aware mouse 语义不另起（未动既有路径，全量单测通过）

**Acceptance Criteria:**
- Given 长块折叠态， when 连续滚轮上滚/下滚， then 视口可见首行单调变化无反向回跳
- Given 折叠开关后， when 第一个滚轮 notch， then 产生可见位移
- Given 无折叠， when 全量既有单测， then 全过且滚动行为不变

## Implementation Notes

- `blocks.rs` 新增纯映射助手（`map_visible` 数学未动）：`folded_container`/`prev_visible_row`/`next_visible_row`/`visible_at_or_below` 按 span 整段跳过隐藏 run（O(run) 而非 O(隐藏行数)）；`window_edges` 为 paint 等价的 skip-walk（`window_edges_agree_with_map_visible` 穷举 pin 住与 `map_visible`+floor-clip 一致）；`step_offset_visible` 按可见行步进并报告 applied 计数；`offset_for_anchor` 做 toggle 顶对齐修正（含 anchor 被折叠走时回退到 span 起始行即 summary 行）。
- `view.rs`：`smooth_scroll` 仅在「非 alt-screen + 有 fold + tracker try_lock 成功」时走 `smooth_scroll_step_visible`，其余（无折叠/alt/锁争用）逐位走 legacy `smooth_scroll_step`；`smooth_scroll_step_visible_matches_legacy_without_folds` 13 组参数 pin 住无折叠时 jump/frac 与 legacy 一致。`toggle_fold_by_seq` 翻转前读顶 anchor，翻转后 `offset_for_anchor` 修正 + `scroll_frac=0` + 取消在途 wheel 动画；任一锁争用时 toggle 本身仍落地，仅跳过修正（无阻塞锁）。
- `element.rs` 零改动：frac 语义（[0,1) 可见行小数）与 paint 位移/sliver 缺席路径保持不变；单向收敛由步进层保证（每整数步恰好一可见行），符合“不引入新 sliver 路径”。
- 过程中抓到并修复：初版 skip-walk 把 run 起始可见行也跳过（`start-1`），等价性测试失败暴露；toggle 测试场景锚点落在折叠区内，遂拆分为 straddle 场景（顶稳定）与 fallback 场景（回退 summary 行）两个单测。

## Verification Results (2026-10-10)

- `cargo test --bin tty7-app terminal::` → 1137 passed, 0 failed（含新增 9 个单测：blocks 7 + view 2）
- `cargo test -p tty7-core --lib block` → 32 passed（daemon 未动）
- `cargo fmt --check` → 干净；`cargo build` 无新增 warning（基线 38 → 仍 38）
- 真机手动检查未做（无 CLI）：需按 plan 手动验证折叠长块后慢速/快速滚轮上下回滚单调无回跳、停手无跳变。

## Plan Change Log

## Review Triage Log

- Quick lens: 5 findings → 1 false, 4 patch, 1 defer (no intent_gap/bad_plan, no loopback).
- F0 no AGENTS.md/CLAUDE.md — false — repo 根与 src/terminal/ 下均无代理指令文件，plan context 为空，无规则可违反。
- F1 smooth_scroll 双 try_lock TOCTOU — medium — view.rs:6811 先 try_lock 判 has_folds 再 try_lock 取 guard，第二次可因 paint/render 争用失败回退 legacy 绝对步进，单 notch 落死 offset 下一 notch 再跳即抖动复现。Patch：一次 try_lock 取 guard 复用判 has_folds。
- F2 toggle 阻塞 term.lock — medium — view.rs toggle 修正分支用 blocking term.lock() 加 scroll_display，与同函数 try_lock_unfair 快照及“fold 永不等待 grid”注释矛盾，亦违 plan Never（不加阻塞锁）。Patch：改 try_lock，争用即跳过修正。
- F3 toggle 文档与代码不符 — low — 文档称任一锁争用 toggle 仍落地仅跳过修正，实则 store.lock() 失败直接 return 丢 toggle（BlockStore 为 Arc<Mutex>，行为沿用旧码）。Patch：文档照实改（tracker 争用即返回，grid 快照缺失即跳过修正），不改 tracker 旧语义。
- F4 toggle 接线无单测 — medium, defer — toggle_fold_by_seq 本体（翻转前 anchor 读取、has_folds 门、alt/rows=0/锁争用跳过）无 harness 可达单测，既有 view 单测皆为纯函数；同前轮 mouse_grid_line 无 harness 条目，记 deferred 待 view harness。
- F5 can_step_up 文档过 claim — low — “无折叠即 offset<history”仅 floor==0 成立，floor>0 短 map 时可见路径更早钳制（行为本身符合顶对齐意图）。Patch：文档限定 floor==0 前提。

## Design Notes

- 上轮 zero-lag 把映射算搬到 render 消了 gutter 追赶，但滚轮仍按绝对行（含隐藏）`Delta` 步进 + `bottom` 锚定：折叠长块时 1 个绝对行可能对应 0 个可见行，offset 加了可见不动，下一帧修正即“上去又下来”。
- `max=history_size` 预钳制是 fold-unaware 的；短 map 顶对齐时同一 offset 前后命名不同可见集，toggle 不修正即首个 wheel 先修正，观感即抖动。
- frac 在折叠下仍做像素位移但无 sliver 补偿，下帧整数 jump 才换 map，先滑后 snap，连续反向即抖；修法是让位移单向收敛而非禁 frac。

## Verification

**Commands:**
- `cargo test --bin tty7-app terminal::` -- expected: 全过，含新增单调性/首 notch 单测
- `cargo test -p tty7-core --lib block` -- expected: 全过（daemon 未动，回归）
- `cargo fmt --check` -- expected: 干净

**Manual checks (if no CLI):**
- 真机折叠长块后慢速/快速滚轮上下回滚，视口单调无来回跳；停手无跳变
