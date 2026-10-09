---
epic: epic-terminal-blocks
date: 2026-10-09
verdict: accepted-with-open-items
criteria: declared
headless: false
---

# Retrospective: epic-terminal-blocks

## Epic summary

- Epic: `epic-terminal-blocks`（命令块：折叠/跳转/复制/红标/持久化/远端一致），initiative `initiative-terminal-blocks`。
- Tickets: 1.1–1.8 全 `done`（counts 8/8 done；`pending_tickets` 空；unpinned/undeclared/order 全空）。
- Plans: story-tracer/jump/copy/ctrl-c/pane/remote/refactor-sweep/perf，全 `done`。
- Range: `4f0e140..HEAD`（20 commits，3 merges 全 measured）。代码量级：`blocks.rs` 新文件 +1465、`pane.rs` +697、`remote.rs` +537、`view.rs` +439、`server.rs` +279、`scrollback.rs` +204、`element.rs` +220。
- Evidence: epic 文件、spec CAP-1~6、8 plans、diff 全量、两轮 code review（18+8 findings）、4 walkthroughs；session logs 即本会话；无 previous retro；behavior 无真机 GUI（headless，见 Behavior verification）。

## Findings

### R1 双端配对镜像（aggregate/duplication）—— accept
- daemon `note_block_marks`/`BlockSlot`（`pane.rs:4271`）与 client `note_b/note_d`/`BlockSlot`（`blocks.rs:98`）逐分支同构，有意为之（server 定稿、client 镜像，spec 约束）；同序列双端断言锁定（`command_blocks_pair_one_nested_layer` vs `one_nested_layer_builds_three_spans_in_close_order`）。
- 解析入口三处分叉（daemon 单 `parse_block_mark` vs client `parse_d_exit_code` + `command_cuts`）现状无害，接受；重构统一为远期候选，不立项。

### R2 分层与体积（aggregate/architecture）—— accept
- daemon 不引 GUI、无环；GUI→daemon 仅类型引用；`client/`、`host/` 未动。`view.rs` +439（18808→19247）两大 hunk 内聚；`blocks.rs` 为新内聚模块。无 god-class verdict。
- i18n 6 键中插截断 `TabContext` 簇：cosmetic，接受。

### R3 kind 64/65 无 FEATURE 门（aggregate/pattern）—— defer（已在 deferred-work.md，不重复）
- `protocol.rs:1040` 无门，旧端整连接 break；与 `FEATURE_FOLLOW_NESTED` 先例不一致。属协议版本管辖，已延期，接受现状。

### R4 spec 出入：嵌套语义被修订（aggregate/spec-reconciliation）—— propose spec update
- spec 原文“嵌套有 marks 则分块”（`spec-terminal-blocks.md:38`）vs as-built 两槽 1 层 + 2 层作废（`pane.rs:4345`、`story-remote-plan.md:24`）。需把 spec 的嵌套条目改写为 as-built（人来改，见 Action items）。

### R5 视图层三处无回归单测（aggregate/spec-reconciliation）—— defer（修正理由后接受）
- 跳块首逻辑、复制精确串、gutter 映射：实现存在、无测试。deferred-work 原理由“无 view harness”**失实**（`view.rs:11698` 有 `TestAppContext` harness），已更正为“缺 block 覆盖”，见 Action items。

### R6 1.8 补审 findings（diff-scope review of `a31e78d..HEAD` 代码部分）
- R6a perf share 无下界（负值空过）—— fix-now（测试加 `>= 0` 界）。
- R6b `as_millis` 截断失败信息 —— fix-now（改精确打印）。
- R6c “delta 即配对成本”注释夸大（实为配对+mark 字节，保守方向）—— fix-now（一行注释）。
- R6d plan:58 基线映射假陈述（`baseline_revision: 3e1c754` vs 复现命令 `4f0e140`）—— fix-now（一行计划行）。
- R6e review 范围误界（把 1.7 重构计入 1.8）—— 过程教训，无代码影响。

## Behavior verification

- 未做真机 GUI 演练（headless，无显示；plan manual checks 均记 deferred/留 walkthrough）。Proxy：全量相关单测绿（core block 31、bin block 63、server 65、scrollback 16、i18n 8），性能门 A 轨数据（增量 2.05–2.67%，fold ~2ms）。
- 以上为“测过但未端到端摸过”，如实记为 narrowed scope，非 clean。

## Previous-retro follow-through

首个 epic，无 previous retro 文件——无待跟进项。

## Action items

1. [fix-now] R6a/R6b/R6c/R6d 四处小修（测试界+打印+注释+计划行）。Owner: agent，待用户点头即做。
2. [fix-now] deferred-work P5/P6 理由更正（harness 存在，缺 block 覆盖）。Owner: agent，同上。
3. [spec-reconciliation] spec 嵌套条目改写为两槽 1 层 + 2 层作废。Owner: 用户（提议文本见 Open questions）。
4. [process] review 前先核 range 归属（R6e 教训）；claim“无 harness”前先 grep（R5 教训）。

## Acceptance verdict

**accepted-with-open-items**（criteria declared；pending 空；deferred 4+2 项已跟踪；行为未真机演练记为 scope 注脚）：
1. 2 次点击回块首+整块复制——代码+文本层单测全绿，缺端到端点击测试（open item）。
2. 不错位+中断/失败标记——双端单测覆盖，met。
3. 重启折叠一致+清除消失——server round-trip 单测覆盖，met。
4. 性能门——A 轨数据达标，B 轨 GUI deferred（open item）。

## Open questions

- spec 嵌套条目改写建议文本：“嵌套 ssh 只支持 1 层：外层 B/C 后内层 B/C/D 正常配块；第 2 层及以上出现作废内外两槽（无块）；内层无 marks 时外层整轮成一块。”是否采用？
- GUI 真机 eyeball（折叠手感、右键三项、万行）何时补？补完即关 open items。
