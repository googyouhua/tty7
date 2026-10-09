# Walkthrough 1-6 — terminal-blocks 1.6 one-level nested pairing plus routed adopt

- target: `a31e78d` — terminal-blocks 1.6: one-level nested pairing plus routed adopt
- baseline: `a6dc93c`
- plan: `epic-terminal-blocks/story-remote-plan.md` (ticket 6, status built)
- diff: 6 files, +833 / -69
- per-block status: `done` — 本文件为 review narrative，Intent 已确认

## Intent

Plan Intent（frozen 原文，一字未改）：

> **Problem:** 远端 pane 的块与本地不一致：嵌套 ssh 下内外层 marks 错配（内层首个 D 把外层提前闭合，外层 ssh 整轮丢失）；直连远端的 adopt/push-back 读不到远端 daemon 表（query 本地），折叠改了对不回来。
>
> **Approach:** 只支持 1 层嵌套：两槽（外层+一层嵌套）+C 分水岭——C 前的 B 是重绘（刷新同槽），C 后的 B 开嵌套槽；第二层及以上（嵌套槽被占时再来 B）直接作废（含外层），只丢块不记错块；adopt 查询按 pane 路由走。

Boundaries（frozen，walkthrough 只确认不扩）：

> **Always:** 归属判定在 server 侧 pane 状态机，client 镜像同规则；无 marks 不建块；alt-screen 触及的槽作废；adopt 只做丰富不做路由（1.1 设计延续）；只支持 1 层嵌套——第 2 层及以上出现（嵌套槽被占时再来 B）作废内外两槽，只丢块不记错块。
>
> **Never:** 无界栈/多层配对；正则猜嵌套；改本地已验行为（B 无 C 照关、孤 D 忽略、clear/resync 语义）；持久化格式变更；为测而造的 ssh 真机依赖（单测止于 harness 可达处）。

Design Notes 落点：只保 1 层（无界栈砍掉，C 分水岭保留）；两槽各记 has_c/saw_alt/tainted；C 是分水岭（C 前 B 重绘刷新同槽，C 后 B 开嵌套槽）；嵌套槽被占时再来 B 即第 2 层出现，作废内外两槽（该区无块，外层 D 到亦丢，之后新 B 重开）——宁缺不错块；daemon 表行仍扁平，wire 不变；本地行为零变更的理由：本地字节流除 C 前双 B 外不会出现"pending 中 B"（此前 bool 重配即等价刷新），唯一行为变化是 C 后 B 从"重配外层"变为"开嵌套"，正是此前错配根因。

Review Triage 遗留（plan 内 G1/G2，审查时注意是否已合入本 commit）：G1 单 batch 瞬态 alt 分歧（medium，reader 侧 modes 折叠镜像补丁）；G2 server 注释误述作废规则（low，改注释）。

## Broad strokes

两槽一层嵌套配对 + adopt 按路由走：

1. server `note_block_marks` bool 改两槽（外层+嵌套，各记 has_c），B/C/D/taint/clear 全按同规则走。
2. client `BlockTracker` 同规则镜像（两槽存 start_abs/fp），同字节流双端同结果。
3. reader 接线：`note_block_cut` C 处理 + per-cut `alt_void_flags`/`void_blocks_on_touch` + 沿用 `void_blocks_on_alt`；`query_procs_routed` 经 pane route 查远端表，不可达回 default。
4. `maybe_adopt` 按 `is_local` 分流：本地走老路，远端走 routed。
5. `term_modes::feed_alt_touched` helper：双端共用的 alt 触及折叠原语。

## Slices

### S1 — server 两槽配对（pane.rs）

- status: done
- files:
  - [pane.rs:4271](../../../crates/tty7-core/src/daemon/pane.rs#L4271) — `BlockSlot { has_c }`（两槽单元）
  - [pane.rs:784](../../../crates/tty7-core/src/daemon/pane.rs#L784) — `block_outer` 落点（外层 pending B）
  - [pane.rs:787](../../../crates/tty7-core/src/daemon/pane.rs#L787) — `block_nested` 落点（一层嵌套 pending B）
  - [pane.rs:4319](../../../crates/tty7-core/src/daemon/pane.rs#L4319) — `note_block_marks`（B：无外层开外层 / 外层无 C 刷新外层 / 嵌套空开嵌套 / 嵌套无 C 刷新嵌套 / 否则作废两槽；C 注顶层；D 关顶层；alt 触及段作废；约 4319–4397）
  - [pane.rs:4400](../../../crates/tty7-core/src/daemon/pane.rs#L4400) — `clear_command_blocks` 全复位（表 + 两槽）
- plan Tasks 对应：bool 改两槽 + B/C/D/taint/clear 规则 + 嵌套矩阵单测
- AC 对应：外B,C,内B,C,D,内B,C,D,外D 得 3 块（id/exit 对号）；嵌套槽被占时再来 B 则该区无块、外层亦丢、新 B 重开；重绘 B,B,C,D 得 1 块；孤 D 不建块；本地已有行为回归全过
- review 入口：先看两槽形状与 `has_c`，再看 B 五分支（开/刷新/开嵌套/刷新嵌套/作废），最后看 C 注顶层 / D 关顶层 / per-segment alt 作废 / clear 全复位；对照 G2 注释是否已修正（被占且有 C 才作废）

### S2 — client 镜像（blocks.rs）

- status: done
- files:
  - [blocks.rs:98](../../../src/terminal/blocks.rs#L98) — `BlockSlot { start_abs, has_c }`（client 镜像槽）
  - [blocks.rs:153](../../../src/terminal/blocks.rs#L153) — `note_b`（与 server 同五分支）
  - [blocks.rs:172](../../../src/terminal/blocks.rs#L172) — `note_c`（注顶层）
  - [blocks.rs:213](../../../src/terminal/blocks.rs#L213) — `note_d`（关顶层成 span，无 pending 则忽略）
  - [blocks.rs:272](../../../src/terminal/blocks.rs#L272) — `prune_before`（丢全落线下槽 + spans）
- plan Tasks 对应：同规则镜像 + 双端一致单测（同字节流同结果）
- AC 对应：与 S1 同矩阵的 client 侧结果；连续内层命令各自成块；第 2 层出现后新 B 重新开始
- review 入口：逐分支对照 S1 的 server 规则（刷新 vs 开嵌套 vs 作废），再看 `void_pending`/`has_pending`/`open_start` 是否只服务接线、最后看 prune 是否连槽带 span 一起丢

### S3 — reader 接线与路由查询（remote.rs）

- status: done
- files:
  - [remote.rs:4058](../../../src/terminal/remote.rs#L4058) — `note_block_cut`（B/C/D 经 tracker；alt-screen 端点门控；C 处理）
  - [remote.rs:4095](../../../src/terminal/remote.rs#L4095) — `void_blocks_on_alt`（端点 mode 版作废，batch 尾）
  - [remote.rs:4114](../../../src/terminal/remote.rs#L4114) — `alt_void_flags`（per-cut modes 折叠，含尾段；G1 补丁落点）
  - [remote.rs:4136](../../../src/terminal/remote.rs#L4136) — `void_blocks_on_touch`（per-cut 作废，cut 前先 void）
  - [remote.rs:1292](../../../src/terminal/remote.rs#L1292) — live batch `alt_probe` 起点
  - [remote.rs:1386](../../../src/terminal/remote.rs#L1386) — live batch `alt_voids` 折叠 + `note_block_cut`（约 1386–1434）
  - [remote.rs:1576](../../../src/terminal/remote.rs#L1576) — replay batch `alt_voids` 折叠 + `note_block_cut`（约 1576–1610）
  - [remote.rs:2674](../../../src/terminal/remote.rs#L2674) — `query_procs`（本地直连老路，不变）
  - [remote.rs:2692](../../../src/terminal/remote.rs#L2692) — `query_procs_routed`（经 `self.route` 查远端表，不可达回 default）
- plan Tasks 对应：`query_procs_routed` 经 route 查表 + 不可达回 default 单测
- AC 对应：单 batch 内进出 alt 且无 marks 时双端同结论（G1）；远端表可达即读到、链路断静默回 default
- review 入口：先看 `alt_void_flags` 折叠顺序（cut 前段 + 尾段）与 probe 跨 batch 携带，再看两处 batch 接线（live/replay 是否对称：cut 前 `void_blocks_on_touch`、尾段 void、再 `void_blocks_on_alt`），最后看 `query_procs_routed` 错误语义是否与 `query_procs` 同（silent default）

### S4 — adopt 分流（view.rs）

- status: done
- files:
  - [view.rs:3948](../../../src/terminal/view.rs#L3948) — `maybe_adopt_daemon_blocks`（adopt 节流 + 表消费 + push-back 不动）
  - [view.rs:3962](../../../src/terminal/view.rs#L3962) — `is_local` 分流（本地 `query_procs` / 非本地 `query_procs_routed`，约 3962–3964）
- plan Tasks 对应：maybe_adopt 按 is_local 分流，本地行为零变更
- AC 对应：远端 pane 链路可达读到远端表、链路断回 default；本地 pane 路径与 1.1/1.5 一致无行为变更
- review 入口：确认分流只换表来源（其余 adopt/push-back 逻辑不动），确认失败语义仍是 silence（无错无 stall）

### S5 — term_modes helper（feed_alt_touched）

- status: done
- files:
  - [term_modes.rs:159](../../../crates/tty7-core/src/core/term_modes.rs#L159) — `feed`（chunk 折叠，跨 chunk 序列携带）
  - [term_modes.rs:172](../../../crates/tty7-core/src/core/term_modes.rs#L172) — `feed_alt_touched`（报告折叠中 alt 是否触及过，含起始态；server per-segment 与 client per-cut 共用原语）
- plan Tasks 对应：双端 alt 作废的共享语义基础（server probe / reader probe 同函数）
- AC 对应：G1 单 batch 瞬态（起止间开关过即算触及）双端一致
- review 入口：确认"起始态计入 + 只看 h 完成切换即精确"的论断，再看 split 序列跨 chunk 携带是否与 grid 解析一致

## Periphery

### P1 — plan 定稿（story-remote-plan.md）

- status: done
- files:
  - [story-remote-plan.md](../epic-terminal-blocks/story-remote-plan.md) — ticket 6 frozen Intent/Boundaries/矩阵/Tasks/AC（本 walkthrough 的 Intent 唯一来源；+87）
- review 入口：确认 walkthrough Intent 与 frozen 原文一致，确认 G1/G2 triage 是否已在本 commit 落定

### P2 — 单测矩阵（双端 + 路由回退）

- status: done
- files:
  - [pane.rs:6465](../../../crates/tty7-core/src/daemon/pane.rs#L6465) — server 嵌套矩阵单测（约 6465–6610：1 层 3 块 / 连续内层 / 第 2 层作废 / 重绘 / 孤 D）
  - [blocks.rs:732](../../../src/terminal/blocks.rs#L732) — client 双端一致与 prune 单测（约 732–1000）
  - [remote.rs:5601](../../../src/terminal/remote.rs#L5601) — `query_procs_routed` 不可达回 default 断言
- plan Verification 对应：`cargo test -p tty7-core --lib block` / `cargo test --bin tty7-app block` 全过；`cargo fmt --check` 干净
- review 入口：先跑两组 block 单测，再看新增用例是否覆盖矩阵每一行（尤其第 2 层作废后新 B 重开、单 batch 瞬态 alt）
