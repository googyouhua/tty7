---
title: '性能门：吞吐与交互验收'
type: 'chore'
ticket: 8
created: '2026-10-09'
status: 'done'
baseline_revision: '3e1c754'
route: 'oneshot'
route_source: 'auto'
risk: 'low'
review: ''
review_source: ''
lenses_ran: []
review_loop_iteration: 0
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** 块索引不能拖慢热路径：spec 约束要求 11MB cat 五次平均相对基线退化≤10%、折叠/跳块首 <100ms，终态一次实测说了算。

**Approach:** 双轨：A) 确定性 Rust 级基准（daemon 11MB feed 开/关配对耗时比 + 200-span fold 构建耗时），CI 可跑，即判即定；B) GUI 真跑（Xvfb 下 baseline 二进制 vs 现二进制 `cat`）time-box 尝试，不成则记 deferred 手工项，不阻塞收 epic。

## Boundaries & Constraints

**Always:** 基线映射诚实记录：A 轨测“增量开销占 feed 总耗时比≤10%”（等价 spirit：用户感知的只有增量部分）；B 轨沿 `scripts/bench` 方法论（11MB、5 次）。

**Never:** 为过门调 bench 参数；改产品代码降开销（超门即报，不修）；在 headless 断言 GUI 像素/手感。

</frozen-after-approval>

## Tasks & Acceptance

**Execution:**
- [x] `crates/tty7-core/src/daemon/pane.rs` tests -- 11MB 级合成输出 feed：有 marks 配对 vs 无 marks，断言配对增量 ≤10% 总耗时（5 次取平均，打印数据）
- [x] `src/terminal/blocks.rs` tests -- 200 span 上 FoldMap 构建 + toggle 计时，断言 <100ms（实为 μs 级，冒烟）
- [x] B 轨：Xvfb 下试跑 GUI io（baseline 取 1.1 前代码构建）；跑通则记录数据，跑不通则 deferred-work 记精确复现命令
- [x] 数据落盘：bench 数据记入 plan Implementation Notes（数字，不是“通过”二字）

**Acceptance Criteria:**
- A 轨两项测试通过且打印数据满足 ≤10% / <100ms
- B 轨有结论：数据或 deferred 手工项，二选一

## Implementation Notes

2026-10-09 A 轨实测（本机 Linux 容器，debug test profile，即判即定）：

- daemon 11MB feed（`perf_gate_block_pairing_overhead_within_ten_percent`，64KiB 分块走 reader 热路径 sniff→pair→record→apply，5 次平均）：
  plain 0.068s，marked（含 200 组 B/C/D、全部配对，`command_blocks.len()==200` 已断言）0.069s，
  增量 0.001s，占 feed 总耗时 2.05% ≤ 10%，过门。
- client 200-span fold（`fold_map_build_over_full_span_table_is_under_100ms`，半数 fold + 2 次 `map_visible` + 全量 unfold）：
  2.094ms < 100ms，过门（冒烟，余量 50×）。

B 轨结论：deferred 手工项（本容器跑不通：harness 系 macOS-only、无 zsh、无 release 二进制），
精确复现命令已记入 `initiative-terminal-blocks/deferred-work.md`（source_plan 为本 plan）。
基线映射：frontmatter `baseline_revision: 3e1c754` 即 B 轨复现命令中的 baseline；A 轨无跨版本基线，
其“基线”是同负载无 marks 的 feed（诚实记录：测的是增量占比，不是跨版本回归）。

## Verification

**Commands:**
- `cargo test -p tty7-core --lib perf_gate` -- expected: 通过并打印 5 次平均与占比
- `cargo test --bin tty7-app fold_map_build` -- expected: 通过并打印耗时
