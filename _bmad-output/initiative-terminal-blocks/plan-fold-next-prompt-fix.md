---
title: '折叠越界修复：不折叠下一块prompt行'
type: 'bugfix'
ticket: ''
created: '2026-10-11'
status: 'built'
baseline_revision: 'adadb54d87537ecc3789551ae467ac099fb6daef'
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

**Problem:** 折叠一个执行命令块时，会把相邻下一个块的 prompt 那行也折叠掉，只剩下相邻块的执行结果。

**Approach:** 修正折叠隐藏范围判定，让折叠只隐藏本块内部行，下一块的 B/prompt 行始终可见。

</frozen-after-approval>

## Implementation Notes

oneshot 小修：相邻块 D 行与下一块 B 行同行时 inclusive end 导致越界，需在 blocks.rs 收窄隐藏判定并补回归单测。

- 决策：保留 `end_abs` inclusive 语义（折叠仍只留首行），仅在 `abs == end_abs` 且该行同时是另一 span 的 `start_abs` 时豁免隐藏。严格内部（`abs < end`）仍隐藏，嵌套折叠链不断。
- 改动：`src/terminal/blocks.rs` `hidden_abs_in` + `folded_container` 共用 `is_next_block_start` 判定；`span_at` 不动（rev 查找已归属新区快）。
- 新增：`folded_prev_leaves_next_block_prompt_visible` 回归单测（100-110 与 110-120 相邻，折叠前者，110 可见、map 含 100 与 110）。
- 验证：`cargo test --bin tty7-app block` 80 过，`cargo test -p tty7-core --lib block` 32 过，`cargo fmt --check` 干净。

## Verification

**Commands:**
- `cargo test --bin tty7-app block` -- expected: 全过，含新增相邻块回归单测
- `cargo test -p tty7-core --lib block` -- expected: 全过
- `cargo fmt --check` -- expected: 干净

## Review Triage Log

- quick lens: 2 findings, 1 group, 0 false. Group 1 (next_visible_row wholesale skip + offset_for_anchor inheritance) — medium — patch: `next_visible_row` jumped `end+1` over shared-boundary visible prompt, diverging from `map_visible`; fixed to step onto shared `end` and let next lap return it. Evidence: `folded_prev_leaves_next_block_prompt_visible` now asserts `next_visible_row(100)==110`; block suite 80 过.
