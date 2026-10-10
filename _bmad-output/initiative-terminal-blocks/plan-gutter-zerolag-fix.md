---
title: ' gutter 零 lag：render 算 paint 复用'
type: 'bugfix'
ticket: ''
created: '2026-10-10'
status: 'built'
baseline_revision: 'f0328040'
route: 'full'
route_source: 'auto'
risk: 'medium'
review: 'quick'
review_source: 'pinned'
lenses_ran: []
review_loop_iteration: 0
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** gutter marker 读 paint-1 快照，滚动手势中每帧落后，肉眼可见一跳一跳（静止收敛不够）。

**Approach:** 映射计算上移到 render（同帧新鲜），paint 校验 generation 后复用，不符则现算；gutter 与 grid 读同一份同帧映射。

## Boundaries & Constraints

**Always:** 同帧内 gutter/grid/点击三方同映射（render 产出、paint 校验复用）；grid 锁失败即清 slot（空 markers + paint 现算，不留上帧）； paint 无 render 产物可用时（如 retained tree 重绘）回退现算；并发输出落进 render→paint 缝隙时允许一帧分叉、次帧收敛（accepted residual）。

**Never:** 改映射数学（screen_to_abs 等不动）；改配对/adopt/持久化；阻塞锁；跨帧缓存无校验使用。

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| 滚动手势中 | 连续帧 scroll | marker 与文字同帧同行，无追赶跳跃 | No error expected |
| 静止 | 无事件 | 与现状一致 | No error expected |
| paint 无 render 产物 | retained 重绘 | 回退现算，结果正确 | No error expected |
| 锁竞争 | try_lock 失败 | 空 markers/回退，结果正确 | No error expected |

</frozen-after-approval>

## Code Map

- `src/terminal/view.rs` (`render_block_gutter` 约 6795，`mouse_grid_line` 约 6939，`block_gutter_markers` 约 6900) -- render 内算 map+markers 存 per-view slot（含 generation：history/offset/rows/fold_epoch/alt）；gutter 与 mouse 读 slot
- `src/terminal/element.rs` (snapshot 写入约 2623，`build_grid` 约 1875) -- paint 先验 slot generation，命中复用、不中现算并回写；快照旧字段保留或收敛（实现定）
- `src/terminal/blocks.rs` -- 映射函数不动，只用

## Tasks & Acceptance

**Execution:**
- [ ] per-view map slot（map+top+markers+generation）+ render 内计算
- [ ] paint 校验复用/回退现算；gutter/mouse 读 slot
- [ ] 单测：slot 新鲜命中、失配回退、retained 重绘路径（harness 可达部分）；无 harness 部分走 review

**Acceptance Criteria:**
- Given 滚动中任意帧， when 读 gutter markers 与 grid 行， then 同一映射、无追赶
- Given retained 重绘（无 render）， when paint， then 回退现算结果正确
- Given 无折叠， when 全量既有单测， then 全过

## Design Notes

- 上一轮“≤1 帧 lag，静止收敛”被真机否决：手势中每帧落后即肉眼可见一跳一跳。本轮消灭 lag 而非收敛 lag。
- generation = history + offset + rows + fold_epoch + alt；任一不等即失配。

## Verification

**Commands:**
- `cargo test --bin tty7-app terminal::` -- expected: 全过
- `cargo test -p tty7-core --lib block` -- expected: 全过（daemon 未动）
- `cargo fmt --check` -- expected: 干净

**Manual checks (if no CLI):**
- 真机慢速/快速滚动，marker 与文字同行；停手无跳变

## Review Triage Log (round 2)

- S1 grid 锁失败留旧 slot — medium — gutter 只验长度即画上帧 markers。Patched：失败即清 slot（空 markers + paint 现算）。
- S2/S4 注释 overclaim（双路径描述错、“永不分叉”）— low — 注释照实改；并发输出缝隙记 accepted residual（Always 已修订）。
- S3 同上，注释改。
