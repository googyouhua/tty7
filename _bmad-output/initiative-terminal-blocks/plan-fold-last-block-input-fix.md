---
title: '末块折叠不再吞新输入prompt行'
type: 'bugfix'
ticket: ''
created: '2026-10-11'
status: 'done'
baseline_revision: '3bd5790a6c5cb5ab95fa7e28c686a83b8fa367f2'
route: 'oneshot'
route_source: 'auto'
risk: 'low'
review: 'quick'
review_source: 'pinned'
lenses_ran: ["quick"]
review_loop_iteration: 0
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** 折叠最后一个执行命令块时，新命令输入的 prompt 行也消失了，被当成折叠内部行藏起。

**Approach:** 把边界豁免从“已闭合块的起始行”扩大到“pending 中命令的起始行”，末块折叠时活输入行始终可见。

</frozen-after-approval>

## Implementation Notes

oneshot 小修：上一轮 `is_next_block_start` 豁免只认已闭合 span 的 `start_abs`，而新 prompt 只是 pending 的 `B` 槽（`outer`/`nested`，未进 `spans`），`D` 行与新 `B` 同行时活输入行仍被判隐藏。需在同一判定里加 pending 槽检查（仅 `abs == end_abs` 边界，严格内部不动，嵌套链不断），并补回归单测。

- 决策：`is_next_block_start` 加 `outer`/`nested` 槽检查；`hidden_abs_in`/`folded_container`/`next_visible_row`/`map_visible` 全经同一判定，自动一致，无需动 paint 侧（`cursor_abs` 在 map 里即 painting）。
- 改动：`src/terminal/blocks.rs` 判定 + `folded_last_leaves_pending_input_row_visible` 回归单测（span 100-110 折叠 + `note_b(110)` pending，110 可见、109 隐藏、map 含 100 与 110）。
- 验证：`cargo test --bin tty7-app block` 81 过，`cargo test -p tty7-core --lib block` 32 过，`cargo fmt --check` 干净。

## Verification

**Commands:**
- `cargo test --bin tty7-app block` -- expected: 全过，含新增末块+pending 输入行回归单测
- `cargo test -p tty7-core --lib block` -- expected: 全过
- `cargo fmt --check` -- expected: 干净

## Review Triage Log

- quick lens: no findings. Boundary-only pending exemption verified through single predicate; 81 + 32 passed, fmt clean.
