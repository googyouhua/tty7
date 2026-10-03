# Outcome
修复 opencode v2.0.18 的状态感知：旧映射表只认 `session.status.busy/idle`，而 v2.0.18 实测只发 `session.execution.started/succeeded` + `form.created`，导致 v2 pane 永远 idle、三色圆点不转。补映射后恢复 working→waiting→done。

# Scope
## 背景
- 用户原话“终端如何感知agent是否暂停”，已确认 Orca 范式；tty7 hook 通道 20/26 俱全。
- 联调（DESKTOP-9AMG9BD，opencode v2.0.18，standalone）用 249 行实测事件流定位：`session.status.*` 在流里一次都没出现过；`form.created`（TUI 提问表单）连 SDK 类型里都没有。

## 做什么
- `opencode_plugin_js` 映射表补 v2.0.18 事件名（execution/form/question.v2/permission.v2），旧名保留兼容老版本；未知事件名保持忽略。
- 钉死测试：现有 opencode 桥断言追加新映射条目。
- `docs/agents/status.mdx` 无需改（行为回到文档已承诺的三色）。

## 不做什么（本次）
- tui-idle 新等待语义、Dashboard 看板、补齐 6 种无 hook agent、全局总开关（已明确删掉，逐个 Uninstall 已够用）、endpoint 落盘（已明确删掉，Unix socket 路径确定性已够用），本次都不做。

# Non-goals
- 不改 waiting/done 语义方向（permission/question/stop 映射的方向不变，只是把 v2.0.18 的新事件名接到同一语义上）。
- 不做 PTY 文本正则/ML 推断；不改 `free` 进程树判定。

# Acceptance examples
- A1：现有 20 种 hook 的 waiting/done 行为无回归（单测全过；抽查 Claude/Codex 任一：permission→waiting、stop→done）。
- A2：opencode v2.0.18 真机三色：standalone 会话跑任务依次出现 working（蓝）→waiting（amber，表单提问时）→done（绿）；`tty7 agents --json` 显示 turns 递增、status 流转。

# Constraints and invariants
- 不新增运行时依赖；改动仅 `crates/tty7-core/src/core/agent_hooks.rs`（映射表 + 断言）。
- `--service` 共享模式插件主动不加载（归属不清），standalone 为支持方式。

# Decisions
- D1 真机联调驱动（用户 2026-10-02/03）：从“看不到圆点”一路查到 v2 事件改名，实测为准。
- D2 F1 总开关、F2 endpoint 落盘先后删除（用户确认多余）：Unix socket 确定性已覆盖重启场景，逐个 Uninstall 已覆盖关闭场景。
- D3 `--service` 模式不做归属猜测（用户只要求程序可读状态：走 `/api`，另议）。
- D4 隔离沿用 `.worktrees/agent-pause-detection`（comet/agent-pause-detection → main）。

# Open questions
- CONFIRMED：用户确认范围=A1-A2（映射修复），进入 Build（代码已就位）。

# Verification expectations
- `cargo test -p tty7-core --lib -- core::agent_hooks` 通过；`cargo test -p tty7-cli --bin tty7` 通过。
- A2 真机验证已完成（用户目视三色 + `agents --json` turns 递增）。
- Verifier 独立复核 A1-A2（只读）。
