---
title: 'Refactor sweep'
type: 'refactor'
ticket: 7
created: '2026-10-09'
status: 'done'
baseline_revision: 'a31e78d'
route: 'oneshot'
route_source: 'auto'
risk: 'low'
review: 'quick'
review_source: 'pinned'
lenses_ran: []
review_loop_iteration: 0
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** 1.1–1.6 并行与回路留下的注释/卫生问题需要一遍清理，只做清理，不加功能不改行为。

**Approach:** 以只读审计定清单：扫 1.1–1.6 diff 求死代码、过期注释、重复逻辑；只修注释与卫生；全量验证锁行为无变更。

## Boundaries & Constraints

**Always:** 清理仅限注释、死代码删除、格式；任何行为变更即停并回滚该项。

**Never:** 功能变更；schema/协议变更；deferred-work.md 的 4 条功能级 postpones（那是新故事，不是 sweep 范围）；为 sweep 而改测试期望（测试只跑不改）。

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| 全量回归 | 全部相关套件 | 与 sweep 前一致通过（3 基线环境失败除外） | 有新失败即停修 |
| clippy/fmt | 触及 crates | 零新增警告，fmt 干净 | 同上 |

</frozen-after-approval>

## Code Map

- Audit scope: `git diff b06ab5c..HEAD -- crates/ src/`（1.1–1.6 全部，五到六文件，约 3000 行）
- Known-clean already: 无 TODO/FIXME；无新增 `allow(dead_code)`；`void_pending`/`open_start` 有生产调用（`has_pending` 仅单测用，见下方 Change Log，已 `#[cfg(test)]`）；`truncated` 注释与行为一致；无 untitled 残留 plan

## Tasks & Acceptance

**Execution:**
- [x] 读审计 diff，列清理清单（注释过期/措辞/重复），逐项改
- [x] `cargo clippy` 触及 crates 零新增警告；`cargo fmt --check` 干净
- [x] 全量相关套件通过（core lib、bin），与基线一致

**Acceptance Criteria:**
- Given sweep 完成， when 跑全量相关套件， then 结果与 sweep 前一致（仅 3 基线环境失败）
- Given 任一清理项， when 问“改行为吗”， then 答案为否（否则回滚该项）

## Implementation Notes

## Verification

**Commands:**
- `cargo clippy -p tty7-core --lib` -- expected: ticket hunks 零新增警告
- `cargo test -p tty7-core --lib` -- expected: 1786+ 通过，仅 3 基线环境失败
- `cargo test --bin tty7-app` -- expected: 全过
- `cargo fmt --check` -- expected: 干净

## Plan Change Log

- 2026-10-09 review loopback 1 (quick): 3 hygiene findings, no behavior bugs. (1) `has_pending` 仅单测用——Code Map 上文已更正，`#[cfg(test)]` 落在其“死代码删除”授权内；(2) 同上；(3) 嵌套分支统一为 `is_none_or` 拼写（双端）。KEEP: 分支合并语义不变、单测全绿。

## Review Triage Log

- R1 cfg(test)-scope — low — plan Code Map 误称 has_pending 有生产调用，已更正；`#[cfg(test)]` 属死代码删除授权内。Patched（plan 行）。
- R2 同上。Patched。
- R3 nested-is_none_or 拼写统一 — low — 双端已改，单测全绿。Patched。
