---
title: '远端一致与嵌套归属'
type: 'feature'
ticket: 6
created: '2026-10-08'
status: 'done'
baseline_revision: 'a6dc93c'
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

**Problem:** 远端 pane 的块与本地不一致：嵌套 ssh 下内外层 marks 错配（内层首个 D 把外层提前闭合，外层 ssh 整轮丢失）；直连远端的 adopt/push-back 读不到远端 daemon 表（query 本地），折叠改了对不回来。

**Approach:** 只支持 1 层嵌套：两槽（外层+一层嵌套）+C 分水岭——C 前的 B 是重绘（刷新同槽），C 后的 B 开嵌套槽；第二层及以上（嵌套槽被占时再来 B）直接作废（含外层），只丢块不记错块；adopt 查询按 pane 路由走。

## Boundaries & Constraints

**Always:** 归属判定在 server 侧 pane 状态机，client 镜像同规则；无 marks 不建块；alt-screen 触及的槽作废；adopt 只做丰富不做路由（1.1 设计延续）；只支持 1 层嵌套——第 2 层及以上出现（嵌套槽被占时再来 B）作废内外两槽，只丢块不记错块。

**Never:** 无界栈/多层配对；正则猜嵌套；改本地已验行为（B 无 C 照关、孤 D 忽略、clear/resync 语义）；持久化格式变更；为测而造的 ssh 真机依赖（单测止于 harness 可达处）。

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| 嵌套 1 层 | 外B,C,内B,C,D,内B,C,D,外D | 3 块：两内层 + 外层（id/exit 对号） | No error expected |
| 连续内层命令 | 内 D 闭合后又来内 B,C,D | 各自成块（槽已空） | No error expected |
| 第 2 层出现 | 嵌套槽被占时再来 B | 作废内外两槽：该区无块，外层 D 到亦丢；之后新 B 重新开始 | No error expected，不记错块 |
| 重绘 prompt | B,B,C,D（C 前双 B） | 1 块（第二 B 刷新同槽） | No error expected |
| 孤 D | 无 B 先 D | 不建块 | No error expected |
| 内层无集成 | 仅外层 C…D，内层原文输出 | 外层成一块，内层无块 | No error expected |
| 内层无集成 | 仅外层 C…D，内层原文输出 | 外层成一块，内层无块 | No error expected |
| 远端 adopt | 远端 pane + 远 daemon 有表 | 经链路读到表并配对；链路不通回 default | 无错无 stall（fire-and-forget 延续） |
| 本地 adopt | 本地 pane | 走老路，行为不变 | No error expected |

</frozen-after-approval>

## Code Map

- `crates/tty7-core/src/daemon/pane.rs` (`note_block_marks` 约 4291，`block_pending: bool` 约 782，`clear_command_blocks` 约 4332) -- bool 改两槽：外槽+嵌套槽（各记 has_c/saw_alt/tainted）；B：无外层开外层，外层无 C 刷新外层，嵌套空开嵌套，否则作废两槽；C 注释顶层开槽；D 关顶层（taint 则静丢）；clear 全复位
- `src/terminal/blocks.rs` (`open: Option<i64>` 约 100，`note_b/note_d` 约 124/146) -- 同规则镜像：两槽存 start_abs/fp/saw_alt/tainted；prune 丢全落线下槽；fingerprint/verify 作用顶层
- `src/terminal/remote.rs` (`query_procs` 约 2624，`connect_routed` 约 3434，`route` 字段约 649) -- 新增 `query_procs_routed(&self)`：经 self.route 查远端表，不可达回 default（与 query_procs 同错语义）
- `src/terminal/view.rs` (`maybe_adopt_daemon_blocks` 约 3948，`host_id.is_local()` 约 2110) -- 本地走 `query_procs`，非本地走 `terminal.query_procs_routed()`；其余 adopt/push-back 逻辑不动

## Tasks & Acceptance

**Execution:**
- [ ] `crates/tty7-core/src/daemon/pane.rs` -- bool 改两槽 + B/C/D/taint/clear 规则 + 嵌套矩阵单测 -- server 侧归属判定
- [ ] `src/terminal/blocks.rs` -- 同规则镜像 + 双端一致单测（同字节流同结果） -- client 归属与 server 一致
- [ ] `src/terminal/remote.rs` -- `query_procs_routed(&self)` 经 route 查表 + 不可达回 default 单测 -- 远端表可达
- [ ] `src/terminal/view.rs` -- maybe_adopt 按 is_local 分流 -- 本地行为零变更

**Acceptance Criteria:**
- Given 外B,C,内B,C,D,内B,C,D,外D 字节流， when 双端配对， then 得 3 块（两内层 + 外层，id/exit 对号）
- Given 嵌套槽被占时再来 B， when 后续 D 到达， then 该区无块、外层亦丢，新 B 重新开始（不记错块）
- Given 本地已有行为（重绘/孤D/无C/alt/clear）， when 回归单测， then 全过无改动
- Given 远端 pane 且链路可达， when adopt， then 读到远端表；链路断时静默回 default
- Given 本地 pane， when adopt， then 路径与断言与 1.1/1.5 一致（无行为变更）

## Design Notes

只保 1 层（无界栈砍掉，C 分水岭保留）。两槽：外层 + 一层嵌套，各记 has_c/saw_alt/tainted。C 是分水岭：C 前的 B 是重绘（刷新同槽），C 后的 B 开嵌套槽；嵌套槽被占时再来 B 等于第 2 层出现，作废内外两槽（该区无块，外层 D 到亦丢，之后新 B 重开）——宁缺不记错块。daemon 表行仍扁平，wire 不变。本地行为零变更的理由：本地字节流里除 C 前双 B 外不会出现“pending 中 B”（此前 bool 重配即等价刷新），唯一行为变化是 C 后 B 从“重配外层”变为“开嵌套”，这正是此前错配的根因。

## Verification

**Commands:**
- `export CARGO_BUILD_JOBS=2; cargo test -p tty7-core --lib block` -- expected: 全过，含新增嵌套单测
- `export CARGO_BUILD_JOBS=2; cargo test --bin tty7-app block` -- expected: 全过，含跨端一致与路由回退单测
- `cargo fmt --check` -- expected: 干净

**Manual checks (if no CLI):**
- 真机嵌套 ssh 跑两命令验证三块；远端直连 pane 切折叠后重连看保持（需 ssh 环境，CI 外）

## Review Triage Log

- G1 single-batch-transient-alt-divergence — medium — client `void_blocks_on_alt`/cut-gate 只看端点 mode，单 batch 内进出 alt 且无 marks 时不 void；server 经 `feed_alt_touched` 会 void。同一 D 双端结论相反，破“同字节流同结果”AC。Patch：reader 侧对 batch 字节做 modes 折叠镜像（复用 `term_modes`），另起单测锁定单 batch 瞬态。
- G2 server-doc-misstates-void-rule — low — `note_block_marks` 注释称嵌套槽被占即作废，代码实为被占且有 C 才作废（与 client 注释、plan 一致）。Patch：改注释。
