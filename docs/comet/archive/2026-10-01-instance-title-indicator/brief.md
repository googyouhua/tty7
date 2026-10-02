# Outcome

每个 GUI 窗口左栏 workspace chip 右侧常驻实例徽标：具名实例显示其 username（`gyh`），默认实例显示“默认”。一眼可确认当前 workspace 的归属，进错实例立刻可见。

# Scope

- 位置：左栏 workspace chip 右侧（下拉箭头后），小徽标样式；hover tooltip 为完整 config dir 路径。
- 文案：具名显示内存名原文；默认实例复用 `InstancePickerDefault` 三语（默认/Default/デフォルト），不新增 i18n key。
- 数据源：`config_dir_path()` + `instance::memory_name_for`；纯函数可单测，渲染层只读。
- 全平台一致（chip 无平台分支）；侧栏隐藏时徽标同进退（与 ws 名一致）。

# Non-goals

- 不改 OS 窗口标题/任务栏文本（gpui 未设置 OS title，本 change 不引入）。
- 不做点击交互（纯展示，无下拉/切换）。
- 不动 picker、daemon、远端链路逻辑。

# Acceptance examples

- A1：gyh 实例窗口左栏 ws chip 右侧可见 `gyh` 徽标，hover 显示其 config dir 全路径。
- A2：默认实例窗口显示“默认”徽标（中/英/日随语言）。
- A3：回归：chip 点击/下拉、侧栏布局不受影响；`cargo fmt` + 相关单测通过。

# Constraints and invariants

- 徽标只读当前进程 config，不触发 IO/网络；渲染开销可忽略。
- 样式跟随主题（沿用现有 badge/tooltip 模式，不引入新色板）。
- `cargo fmt --check` 通过。

# Decisions

- 工作区：用户指定当前目录（`remote-link` worktree）。
- 形态：始终显示，默认实例显示“默认”（用户已选）。
- 位置：左栏 workspace chip 右侧（用户已选；标题栏不动；侧栏隐藏时徽标同进退）。

# Open questions

无。用户已确认 Outcome/Scope/A1–A3/Constraints/Non-goals 及位置（左栏 workspace chip 右侧），进入 Build。

# Verification expectations

- `cargo fmt --check`、徽标文案单测、现有 `app`/`windows` 相关单测通过。
- 真机目检（用户）：gyh/默认窗口各看一次徽标（A1/A2）；拖拽与按钮无异常（A3）。
