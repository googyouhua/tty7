# instance-picker-relaunch — complete target specification

## 1. Picker 确认后的继续方式（按平台）

- Linux/macOS：`run_picker()` 返回 `Some(selection)`，`main` 在进程内继续：`apply_selection`（建目录、写记忆、设 config dir）后走既有启动流程（daemon ensure、forward、主窗口）。
- Windows：确认 handler 内先重起自己再退出 picker 进程，不返回：
  - 解析并创建实例目录（同 `PickerSelection::config_dir`，leaf `700` 语义不变）。
  - `Command::new(current_exe).arg("--config-dir").arg(dir)` + 原启动参数（除 picker 触发条件外原样透传，如 `--open-path`/路径参数），detached（不继承控制台）拉起新进程；失败则报错并停留，不退出。
  - 新进程带显式 `--config-dir` 走老路：跳过 picker、`remember_current` 写记忆、正常进主 UI。
- 取消/Esc：所有平台直接退出，不拉新进程、不写记忆。

## 2. 不变项

- picker 界面、列表、预选中、校验规则不变；`--as`/记忆文件格式不变；daemon/control 协议不变。
- `wants_instance_picker` 门控条件不变。

## 3. 平台差异声明

- 该差异仅因 Windows `platform.run()` 以 `ExitProcess(0)` 结尾、无法在进程内继续；行为结果三平台一致：确认后进入所选实例主 UI。
