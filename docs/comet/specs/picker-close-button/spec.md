# picker-close-button — complete target specification

## 1. Picker 基座（来自 windows-picker-relaunch 基线，已合入）

- `tty7_core::core::instance`：用户名校验、根目录解析、已存在 `tty7-<name>` 列表、`last` 记忆读写（默认目录记忆文件），单测覆盖。
- `src/ui/instance_picker.rs`：默认行 + 已存在名列表 + 新用户名输入框，上/下、Enter 确认、Esc 取消；输入非空时胜出并校验；非法名行内报错不退出。
- `src/main.rs`：`wants_instance_picker` 门控（无显式 `--config-dir`/`TTY7_CONFIG_DIR`、非 `--daemon`/`--stop-daemon`、非 explorer 菜单动作）；`run_picker()`：
  - `Some(sel)` → Windows 先 detached 重起新进程再 `quit()`，Linux/macOS 进程内 `apply_selection` 后继续；
  - `None`（取消/关闭/X）→ 直接 `return`，静默退出。
- 显式 config 启动跳过 picker 但更新 preselect 记忆；冷 `--open-path` 无运行窗口时经 picker 再打开。

## 2. 关闭按钮（本变更增量）

- 位置：picker 底部操作行，与主“进入”按钮同行右侧（或次级样式），文案复用 i18n：`InstancePickerClose`（en `Close` / zh `关闭` / ja `閉じる`）。
- 触发：鼠标点击关闭按钮 → `cancel()` → `finish(None)`；与 `Esc`、`CancelInstance` action、窗口 X 完全同效。
- 结果：`run_picker() == None`，`main` 直接返回，进程 exit 0；不写记忆、不拉新进程、不 `eprintln`、不 `exit(1)`、不触碰 daemon/已运行 GUI。
- 可达性：按钮可聚焦、Enter/Space 可激活；关闭按钮永远可用，不受输入校验状态影响。

## 3. 平台差异

- Linux/macOS：确认后进程内继续；关闭后进程结束。
- Windows：确认需先 detached 重起新进程（`--config-dir <dir>` + 原参数透传）再 `quit()`，否则确认后无法进主 UI；关闭路径所有平台一致为直接退出、不拉新进程。

## 4. 不变项

- picker 列表、预选中、校验规则、`--as`/记忆格式、daemon/control 协议不变。
