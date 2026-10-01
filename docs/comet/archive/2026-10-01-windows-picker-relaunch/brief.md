# Outcome

Windows 上 picker 确认后能进入主 UI：确认后重起自己（带解出的 `--config-dir`），再退出 picker 进程。

# Scope

- 背景：Windows `platform.run()` 以 `ExitProcess(0)` 结尾，picker 的 `application.run()` 一返回进程即结束，`run_picker()` 回不到 `main`，主 UI 永远起不来（Linux/macOS 不受影响）。
- 修法（仅 Windows）：picker 确认 handler 内先 `Command::new(current_exe).arg("--config-dir").arg(dir) + 原启动参数` 拉起新进程（detached），再 `quit()`；新进程带显式 config 走老路（跳过 picker、写记忆）。
- Linux/macOS 保持进程内继续不变。
- 不改 daemon/control 协议，不动系统配置。

# Non-goals

- 不重构单 `application.run` 架构；不改 picker 界面本身；不碰 `--as`/记忆规则。

# Acceptance examples

- A1：Windows 裸启动 → picker → 输入用户名确认 → 主 UI 出现，进的是 `tty7-<name>` 实例。
- A2：带路径冷启动（`tty7-app <path>`）经 picker 确认后，主 UI 打开该路径（参数透传）。
- A3：Linux/macOS 行为不变（回归：已有无头常驻/显式/记忆证据 + 单测）。

# Constraints and invariants

- 重起只发生在 Windows 确认路径；取消/Esc 仍直接退出，不拉新进程。
- 原参数透传（picker 只在无显式 config 时出现，无冲突）。
- 子进程 detached，不继承控制台。

# Decisions

- 方案：Windows 确认内重起（小改），而非单 run 重构（风险大）。
- 工作区：独立 worktree `windows-picker-relaunch`。

# Open questions

无。用户已确认范围，进入 Build。

# Verification expectations

- `cargo fmt --check`、相关单测通过；Windows 真机：A1/A2 目检通过。
