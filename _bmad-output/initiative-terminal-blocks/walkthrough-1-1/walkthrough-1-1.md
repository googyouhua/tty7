# Walkthrough 1-1 — terminal-blocks 1.1 tracer

- target: `6e51bf8` — terminal-blocks 1.1: block model tracer with fold, jump-to-start and whole-block copy
- baseline: `4f0e140` — terminal-blocks: brief + spec + epic with 8 tickets (CAP-1~6)
- plan: `epic-terminal-blocks/story-tracer-plan.md` (ticket 1, status built)
- diff: 14 files, +2127 / -78
- per-block status: `done` — 2026-10-08 用户审查通过

## Intent

Plan Intent（frozen）：scrollback 是平的，没有"一次执行"的单位，长输出回翻定位与完整复制成本高。
Approach：server 侧以 OSC 133 B→D 配对建块表并下发客户端，GUI 以 gutter 折叠与右键两项（跳块首、复制两者）接通同一块 id，跑通最薄全链路。

Boundaries（frozen，walkthrough 只确认不扩）：
Always：边界唯一来源 OSC 133 B→D；无 marks 不建块；alt-screen 期间不建块；复制纯文本；block schema（id/exit_code/folded/truncated）与退出码行格式由本 ticket 拥有。
Never：自动折叠；快捷键；ANSI 复制；outline；正则猜 prompt；持久化接线（entry-5）；远端特殊处理（entry-6）。

Design Notes 落点：退出码行格式 `exit <code>` 独占末行；菜单无块光标下隐藏（沿空菜单抑制先例），不做 disabled 置灰；client span 以本地 seq 与绝对行寻址，daemon 表是审计/持久真相，adoption 只做丰富不做路由；clear 后首次 adopt 只 resync 不配对；client spans 与 daemon 同量级上限；daemon 侧 clear 接线递延 entry-5。

## Broad strokes

最薄全链路（server 建表 → 协议下发 → client 跟踪 → GUI 折叠/菜单）：

1. server 在 OscSniffer/handle_osc133 处以 B→D 配对建块，alt-screen 门控，无 marks 不建块。
2. 块表经 PaneContext 下发客户端（复用现有帧，不另起通道）。
3. client 侧 BlockTracker 在 B 处开 span（C 仅注释/忽略，与 daemon 对齐），D 闭合，锚点用真绝对行。
4. GUI：gutter marker 列绘制 + 单击折叠/展开（一行摘要），右键两项跳块首 / 复制两者，块复制走 copy_selection 同源语义 + `exit <code>` 尾行。

## Slices

### S1 — daemon 建块表（真相源，entry-1 拥有 schema）

- status: done
- files:
  - [pane.rs:774](../../../crates/tty7-core/src/daemon/pane.rs#L774) — PaneState 新增块表落点
  - [pane.rs:4193](../../../crates/tty7-core/src/daemon/pane.rs#L4193) — SniffSignals / OscSniffer B→D 配对开闭合（含 alt-screen 门控，约 4193–4311/4322）
  - [pane.rs:6218](../../../crates/tty7-core/src/daemon/pane.rs#L6218) — I/O 矩阵边角单测（中断 D;130 / 无集成 / 全屏 / 未配对 / 清除，约 6218–6406）
  - [pane.rs:942](../../../crates/tty7-core/src/daemon/pane.rs#L942) — `clear_command_blocks` tested helper（daemon 侧接线递延 entry-5，loopback 1 结论）
- plan Tasks 对应：块表 + B→D 配对开闭合 + alt 门控 + 无 marks 不建块；边角单测；clear 武装 resync 的 daemon 侧（保持 helper，接线递延）
- AC 对应：万行折叠一行且展开无损；Ctrl-C exit_code=130；无 marks 不建块；vim 全屏不建块；有 B 无 D 不建块、D 到达才闭合；B→D 间无 C 字节仍建块并与首个 daemon id 配对
- review 入口：先看 SniffSignals 形状（id/exit_code/folded/truncated），再看 B 开 / D 闭合 / alt 门控三分支，最后看 tests 矩阵 6 行是否一一有名

### S2 — 块表下发契约（全链路可达）

- status: done
- files:
  - [protocol.rs:622](../../../crates/tty7-core/src/daemon/protocol.rs#L622) — PaneContext 块表下发位置（约 622–660，+37）
- plan Tasks 对应：复用 PaneContext/现有帧，不另起通道
- AC 对应：全链路可达（client 能见到 daemon 表；消费路由不依赖它，entry-6 前只做丰富）
- review 入口：确认 wire 类型 CommandBlock 形状（无 offsets，id/exit_code/folded/truncated），确认无新通道

### S3 — client 跟踪语义（B 开 / 锚点 / 复制文本 / clear resync）

- status: done
- files:
  - [blocks.rs:1](../../../src/terminal/blocks.rs#L1) — 新建 BlockTracker（808 行：B 处开 span、C 仅注释，clear 武装 resync + spans 上限与 daemon 200 同量级，adopt 只丰富不路由）
  - [mod.rs:1](../../../src/terminal/mod.rs#L1) — 模块挂载（+1）
  - [remote.rs:116](../../../src/terminal/remote.rs#L116) — ReaderSignals / RemoteTerminal 接线（约 116/644/963–1258）
  - [remote.rs:1388](../../../src/terminal/remote.rs#L1388) — ReaderCut / command_cuts 消费示例（约 1388/1547）
  - [remote.rs:2090](../../../src/terminal/remote.rs#L2090) — adopt / cut_anchor（去掉 display_offset，绝对行 = history + line，约 2090 起；chunking/block 单测约 3670–3850）
- plan Tasks 对应：B 处开 span、C 仅注释（去 C 门）；cut_anchor 去 display_offset；block_text 经 bounds_to_string；clear 武装 resync + 上限
- AC 对应：B 无 C 配对与 daemon 同语义；换行包裹复制与 bounds_to_string 逐字一致；滚动中落 D 折叠/复制不错位；清除后新 span 只与 clear 后 id 配对
- review 入口：先看 blocks.rs 开/闭合语义（B 开、D 闭、C 忽略），再看 cut_anchor 锚点公式，最后看 resync（watermark 追快照最大 id、不配对）与 cap

### S4 — gutter 折叠可见链路

- status: done
- files:
  - [element.rs:1855](../../../src/terminal/element.rs#L1855) — gutter marker 列绘制与单击折叠/展开（含折叠一行摘要，约 1855–2493；paint/mouse 映射）
- plan Tasks 对应：gutter marker 列绘制与单击折叠/展开
- AC 对应：点 marker 块折为一行，展开后内容无损
- review 入口：先看 marker 落点（gutter strip / marker 点击），再看折叠一行摘要绘制，最后看 mouse 分流

### S5 — 右键菜单 + 跳块首 + 复制两者

- status: done
- files:
  - [view.rs:133](../../../src/terminal/view.rs#L133) — actions! 新增 2 Action（约 133）
  - [view.rs:413](../../../src/terminal/view.rs#L413) — TerminalView 状态（menu latch / block，约 413/450）
  - [view.rs:3769](../../../src/terminal/view.rs#L3769) — block_text + 跳块首/复制两者实现（经 term.bounds_to_string，与 screen.rs capture 同函数；保留退出码行与修剪；约 3769–3908；copy_selection 同源约 3732 上下文）
  - [view.rs:6696](../../../src/terminal/view.rs#L6696) — context_menu builder 新增两项 + 无块隐藏（约 6696–6866；gating/空菜单抑制约 8342–8550 上下文）
  - [view.rs:1935](../../../src/terminal/view.rs#L1935) — 接线/调用点（约 1935/1953/4338/4432）
- plan Tasks 对应：新增 2 Action 与右键菜单项，无块光标下隐藏；块复制走 copy_selection 同源语义并附退出码行；block_text 改经 bounds_to_string
- AC 对应：块内右键跳块首视口置块首命令行；复制两者得纯文本命令+输出+退出码行；无 marks 下无菜单项
- review 入口：先看 actions 定义，再看 menu builder 隐藏逻辑（None 抑制 vs disabled），最后看 jump/copy 与 block_text（WRAPLINE 由 alacritty 语义处理）

## Periphery

### P1 — i18n 两菜单项 x5 语言

- status: done
- files:
  - [mod.rs:1396](../../../src/ui/i18n/mod.rs#L1396) — l10n_keys 新增两键（+2）
  - [en.rs:2324](../../../src/ui/i18n/en.rs#L2324) — 英文（+2）
  - [ja.rs:2395](../../../src/ui/i18n/ja.rs#L2395) — 日文（+2）
  - [ru.rs:1865](../../../src/ui/i18n/ru.rs#L1865) — 俄文（+2）
  - [zh.rs:2165](../../../src/ui/i18n/zh.rs#L2165) — 中文（+2）
- review 入口：确认 5 边键名一致、无缺译（4 语言返回 Option 侧）

### P2 — CLI 侧测试桩 + plan 文档本身

- status: done
- files:
  - [commands.rs:4034](../../../crates/tty7-cli/src/commands.rs#L4034) — tests（约 4034/4062/4085，+3）
  - [story-tracer-plan.md:1](../../../_bmad-output/initiative-terminal-blocks/epic-terminal-blocks/story-tracer-plan.md#L1) — 本 walkthrough 的 plan 基准（133 行：Intent/Tasks/AC/Design Notes/Verification/Review Triage G1–G8）
- review 入口：commands.rs 仅确认测试桩无行为变更；plan 文件确认 narrative 引用与其 Intent/AC 一致（plan 本身是输入，不是被 review 的代码）
