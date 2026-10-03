# Outcome

username picker（实例选择 UI）在主分支基线上拥有可见、可点的“关闭”按钮；此前窗口弹出后无明确关闭出口。点击关闭后仅退出本次启动（不再进入主 UI），不杀死已运行实例/守护进程，不 `process::exit(1)` 报错退出。

# Scope

- 基线：本变更 worktree 基于 `main` 创建后，已合并 `comet/windows-picker-relaunch` 尖端（`5c1b82a`，Verify 中 2/3 通过、仅 A1 待 Windows 真机），picker 基座（`instance.rs`、`instance_picker.rs`、`main.rs` 接入、记忆/校验/i18n、Windows 确认内重起）已就绪；本变更只做关闭按钮增量，不重复实现 picker。
- 关闭按钮行为：
  - UI 底部与“进入”并排的次级“关闭”按钮，可鼠标点击；快捷键 `Esc` 保持原取消语义。
  - 点击/ Esc / 窗口 X 均走 `finish(None)` → `run_picker() -> None` → `main` 直接 `return`，不拉新进程、不写记忆、不报错。
  - “仅退出本次启动”指结束本次 `tty7-app` 进程；已运行的 GUI/daemon/tray 不受影响。

# Non-goals

- 不重构单 `application.run` 架构；不改 daemon/control 协议；不动 `--as`/记忆文件格式。
- 不做最小化到托盘、隐藏后唤回等额外语义。
- Linux/macOS 既有启动路径回归范围外的新功能不做。

# Acceptance examples

- A1：裸启动（无显式 config）弹出 picker，可见“关闭”按钮；点击关闭后进程正常结束（exit 0，安静退出），不进入主 UI，不弹错。
- A2：Esc 与窗口 X 与关闭按钮同效：均退出本次启动，不写记忆、不拉新进程。
- A3：确认路径不受影响：输入合法用户名确认后进入 `tty7-<name>` 实例主 UI（Windows 需含既有 detached 重起语义，否则确认后无法进主 UI——见 Constraints）。
- A4：Linux/macOS 回归：显式 `--config-dir` 跳过 picker；已有单测通过。

# Constraints and invariants

- 关闭是安静退出：`None` 分支直接 `return`，不得 `process::exit(1)`、不得打印错误、不得杀 daemon。
- Windows 确认内重起语义随基线合入已具备（`5c1b82a`），A3 三平台同测；关闭路径所有平台一致为直接退出、不拉新进程。
- 增量范围锁定为 picker 关闭按钮（UI + i18n + 单测），不顺手合入其他分支杂项。
- `cargo fmt --check` 通过。

# Decisions

- 工作区：独立 worktree `picker-close-button`，基于 `main` 创建后合入 picker 基线（用户更新：relaunch 分支已完成、可基于它实现）。
- 关闭语义：退出本次启动（用户已选），不做“仅关窗不退出进程”。

# Open questions

无。2026-10-01 用户已确认 Outcome/Scope/A1–A4/Constraints/Non-goals，进入 Build。

# Verification expectations

- `cargo fmt --check`、picker/instance 相关单测通过。
- 人工目检（Linux 即可，Windows 真机沿用 relaunch 分支结论）：A1/A2 关闭三路同效且静默退出；A3 确认进实例。
