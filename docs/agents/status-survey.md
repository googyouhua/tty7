# Agent 状态感知横向调研（2026-10-03）

问题：终端/编辑器如何知道 agent 在 working、等输入（waiting），还是做完了（done）。

结论先行：只有三类架构。第一方内嵌（状态是内部变量，不存在“感知”问题）；结构化协议（ACP 这类，状态是协议消息）；终端外挂（hook 主动上报 + OSC 转义 + 轮询，tty7/Orca/Warp 都在这类）。

## 1. tty7（本项目）

- 主通道 hook 主动上报：20/26 种 CLI 有 hook（`session-start/prompt-submit/permission-request/question-asked/tool-complete/stop` → `Idle/Working/Waiting/Done`），daemon socket 优先、`OSC 777;tty7://cli-agent` 兜底。
- 弱兜底：桌面通知 `OSC 9/99/777`（无富事件时置 waiting）、中断猜测、30min stale。
- 查询：`tty7 wait` 500ms 轮询（ flour 状态、无状态设计），`tty7 agents` 快照。
- 缺口：6 种无 hook 只识别无状态；`--service` 这类共享后台导致无法归属 pane 的一律不报（乱报不如不报）。

## 2. Orca（stablyai/orca，开源，MIT）

- 与 tty7 同构：`OSC title` 管身份存活 + hook 管 waiting/done 语义（`Settings → Agents → Agent status hooks` 总开关 + `orca agent hooks status|on|off`）。
- hook 回调地址落盘（`{userData}/agent-hooks/endpoint.env`，每次调用 re-source），app 重启后长会话不断报——这正是 tty7 之前没有、后又证明不需要的两级之一（Orca 用 TCP 端口，重启变址；tty7 用确定性 Unix socket，无需文件）。
- 状态：spinner working / 琥珀问号 waiting / 绿勾 done / 红 blocked / 灰 idle；`working→idle` 即发 agent-finished 通知；Agent Dashboard 看板四列。
- 出处：https://www.onorca.dev/docs/model/agents-sessions 、https://www.onorca.dev/docs/agents/hooks-memory 、https://www.onorca.dev/docs/notifications 、https://www.onorca.dev/docs/terminal

## 3. Warp（闭源终端）

- 语义：通知分三类 `Complete / Request / Error`；tab 状态图标 working/blocked/completed/errored + 未读 badge；toast + 通知邮箱 + 桌面通知。
- 第三方 CLI 同样逐个适配装插件：Warp Agent 原生；Claude notification plugin（一键安装）；Codex 原生配置（`notification_condition = "always"`）；OpenCode 插件（`@warp-dot-dev/opencode-warp` 进 config `plugin` 数组）。
- 编排：父/子会话状态机 `INPROGRESS/SUCCEEDED/BLOCKED/FAILED/ERROR/CANCELLED`，mailbox 只看父，子状态看 pill bar。
- 出处：https://docs.warp.dev/agents/capabilities/agent-notifications/

## 4. Zed

- 自家 Agent Panel：第一方，状态是内部变量（spiner/审批行/错误行），无感知问题。
- 外部 CLI 走 **ACP（Agent Client Protocol，开放标准，JetBrains/Google/GitHub 等采用）**：agent 起独立进程 over ACP，Zed 给原生体验——tool approval、status indicator、update（zed discussion #49206 原话）；调 `dev: open acp logs` 可看全量协议消息。
- 备选 Terminal Threads：直接在终端跑 CLI/TUI——这条路下 Zed 和普通终端一样，靠 shell integration，无 agent 语义。
- 出处：https://zed.dev/docs/ai/external-agents 、https://zed.dev/acp 、https://github.com/zed-industries/zed/discussions/49206

## 5. VS Code（Copilot agent mode）

- 第一方内嵌：权限级别下拉（auto/manual）、工具调用逐项确认；状态即 UI 状态，无需感知。
- 旁证“通知不是状态”：社区长期有人要“做完后桌面通知”（discussion #157286），说明完成信号要靠 IDE 自己发，外部程序看不到。
- 出处：https://code.visualstudio.com/docs/agents/run/approvals 、https://github.com/orgs/community/discussions/157286

## 6. Cursor

- 第一方：Agents Window 侧栏按 Status 分组，`Needs Attention` 分区收拢所有 blocked 的 agent；完成音效/通知开关。
- 第三方绕法（AgentNotch/Pushary 等）：本质是屏幕/剪贴板/无障碍事件嗅探 + 手机推送，走的是“人肉轮询”，和终端的 hook 完全不是一回事。
- 出处：https://forum.cursor.com/t/agents-approval-window/170388 、https://forum.cursor.com/t/not-getting-notifications-for-sub-agents-allow-deny-request/166194

## 7. Windsurf（Cascade）

- 第一方：agentic pane 内计划、改文件、调工具，per-step approval；Automated Command 白名单自动放行低风险命令。
- 状态同样是内部变量；第三方有 AgentApprove 这类手机代点审批的，原理同 Cursor 绕法。
- 出处：https://www.digitalapplied.com/blog/windsurf-2-deep-dive-cascade-agents-flows-2026 、 cascade prompt 中 `WAITING for user approval` 语义

## 8. Ghostty（纯终端参照物）

- 只有命令级感知：`OSC 133` prompt 标记 + `notify-on-command-finish`（长命令跑完桌面通知）。无 agent 语义，也不可能有——shell integration 只知道命令起止。
- 出处：https://ghostty.org/docs/features/shell-integration 、https://ghostty.org/docs/install/release-notes/1-3-0

## 对比表

| 产品 | working | waiting | done | 通道 | 第三方 CLI |
|---|---|---|---|---|---|
| tty7 | hook | hook | hook | hook+OSC+轮询 | 逐个装 hook（20/26） |
| Orca | hook | hook | hook | OSC title+hook | 总开关+按机安装 |
| Warp | 插件/原生 | 插件/原生 | 插件/原生 | 插件+通知 | 逐个装插件（Claude/Codex/OpenCode） |
| Zed | ACP/内部 | ACP/内部 | ACP/内部 | 开放协议 | ACP registry 安装 |
| VS Code | 内部 | 内部 | 内部 | 第一方 | 不适用 |
| Cursor | 内部 | 内部 | 内部 | 第一方 | 不适用（绕法是嗅探） |
| Windsurf | 内部 | 内部 | 内部 | 第一方 | 不适用 |
| Ghostty | 无 | 无 | 命令级通知 | OSC 133 | 无（只能等命令退出） |

## 给 tty7 的结论

1. “hook 主动上报 + OSC 传输 + 可编程查询”是终端类产品的通用范式，不是某一家独创；语义事件必须逐 agent 适配，没有通用协议（ACP 是 IDE 侧的答案，终端侧没有对应物）。
2. 通知嗅探当状态用：没有任何一家当主力，全是弱兜底——tty7 的定位与业界一致。
3. Warp/Orca 的实践佐证了 tty7 的两个工程选择：逐 agent 装插件（OpenCode 两家都是插件）、共享后台不归属就不报（Warp 编排里子会话状态也不进 mailbox）。
4. Ghostty 的位置标定了下限：只有命令起止、没有 agent 语义——这正是 `tty7 wait --until free` 与 `--until waiting,done` 的分界线。
