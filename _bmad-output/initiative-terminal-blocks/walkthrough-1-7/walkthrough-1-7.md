# Walkthrough 1-7 — terminal-blocks 1.7 refactor sweep

- target: `3e1c754` — terminal-blocks 1.7: refactor sweep (comments, branch spelling, cfg-test)
- baseline: `a31e78d`
- plan: `epic-terminal-blocks/story-refactor-sweep-plan.md` (ticket 7, status built)
- diff: 代码 2 files, pane.rs 21 行 / blocks.rs 12 行（+ plan 文件本身）；行为零变更
- per-block status: `done` — 本文件为 review narrative，已走读确认通过
- 范围注记：区间内 walkthrough-1-6 的 narrative 文件（`walkthrough-1-6.md` / `walkthrough-1-6-log.md`）为已审记录，不在本次 walkthrough 范围

## Intent

Plan Intent（frozen 原文，一字未改）：

> **Problem:** 1.1–1.6 并行与回路留下的注释/卫生问题需要一遍清理，只做清理，不加功能不改行为。
>
> **Approach:** 以只读审计定清单：扫 1.1–1.6 diff 求死代码、过期注释、重复逻辑；只修注释与卫生；全量验证锁行为无变更。

Boundaries（frozen，walkthrough 只确认不扩）：

> **Always:** 清理仅限注释、死代码删除、格式；任何行为变更即停并回滚该项。
>
> **Never:** 功能变更；schema/协议变更；deferred-work.md 的 4 条功能级 postpones（那是新故事，不是 sweep 范围）；为 sweep 而改测试期望（测试只跑不改）。

## Broad strokes

只清不改，三处卫生：

1. 注释卫生（措辞/换行）：`clear_command_blocks` doc 重述 pending-open 语义（`D` 到后关空），`note_block_marks` 长行重排不断句；`note_b` 补"重绘仍是一轮"注。
2. 分支拼写统一：双端 B 分支 `is_none() + !is_some_and(has_c)` 合并为 `is_none_or(!has_c)`，语义不变。
3. `cfg(test)` + plan 行更正：`has_pending` 标 `#[cfg(test)]` 并注明生产侧读 `open_start`/`void_pending`；plan Code Map 更正"有生产调用"误述（Review Triage R1/R2/R3 落定）。

- status: pending

## Slices

### S1 — server 侧清理（pane.rs）

- status: pending
- files:
  - [pane.rs:2839](../../../crates/tty7-core/src/daemon/pane.rs#L2839) — `clear_command_blocks` doc（pending-open 语义重述：clear 后 `D` 关空）
  - [pane.rs:4305](../../../crates/tty7-core/src/daemon/pane.rs#L4305) — `note_block_marks` doc 重排（作废规则不断句）
  - [pane.rs:4345](../../../crates/tty7-core/src/daemon/pane.rs#L4345) — B 分支拼写统一（`is_none_or(!has_c)`，外层+嵌套；约 4345–4360）
- plan Tasks 对应：逐项改注释/拼写，语义不变
- AC 对应：任一清理项问"改行为吗"答案为否；`cargo test -p tty7-core --lib block` 全绿
- Confirmed-zero-behavior-change 依据：单测全绿（core lib block 组 31 passed / 0 failed，bin block 组 63 passed / 0 failed；分支合并为纯拼写等价）
- review 入口：先对两处 doc 改（措辞只澄清不清义），再对 B 两分支做旧拼写→新拼写真值表对照（None/无 C/有 C 三态一致），最后确认无其他 hunk

### S2 — client 侧清理（blocks.rs）

- status: pending
- files:
  - [blocks.rs:153](../../../src/terminal/blocks.rs#L153) — `note_b`（与 S1 同拼写统一 + 重绘注；约 153–168）
  - [blocks.rs:187](../../../src/terminal/blocks.rs#L187) — `has_pending` doc + `#[cfg(test)]`（生产侧读 `open_start`/`void_pending`）
- plan Tasks 对应：`has_pending` 仅单测用→`#[cfg(test)]`（"死代码删除"授权内）；嵌套分支拼写统一双端
- AC 对应：同上；`cargo test --bin tty7-app block` 全绿
- Confirmed-zero-behavior-change 依据：单测全绿（同上两组；`#[cfg(test)]` 只收窄非测试编译可见性，测试语义不变）
- review 入口：逐分支对照 S1 server 规则（刷新 vs 开嵌套 vs 作废三态一致），再确认 `has_pending` 无生产调用残留（搜调用点止于单测），最后确认 doc 指向的 `open_start`/`void_pending` 确为生产读取口

## Periphery

### P1 — plan 定稿（story-refactor-sweep-plan.md）

- status: pending
- files:
  - [story-refactor-sweep-plan.md](../epic-terminal-blocks/story-refactor-sweep-plan.md) — ticket 7 frozen Intent/Boundaries/矩阵/Tasks/AC（本 walkthrough 的 Intent 唯一来源；+75 新文件；Code Map 更正 + Review Triage R1/R2/R3）
- review 入口：确认 walkthrough Intent 与 frozen 原文一致，确认 R1/R2（has_pending 误述更正）与 R3（is_none_or 双端统一）是否均已落定、无行为项混入
