# instance-title-badge — complete target specification

## 1. 徽标文案（纯函数，可单测）

- 输入：当前进程 `config_dir_path()`。
- `instance::memory_name_for(dir)` 得 `Some(name)`：
  - `name == DEFAULT_SENTINEL` → 显示 `t(InstancePickerDefault)`。
  - 否则显示 `name` 原文（`memory_name_for` 只产出合法名）。
- `None`（无 config dir 的极端情况）→ 显示 `t(InstancePickerDefault)`，永不空不 panic。
- Tooltip 文本：config dir 全路径（`display()`），无路径时回退显示文案本身。

## 2. 渲染位置

- `Tty7App::workspace_head`（`src/ui/tab_strip.rs`）chip 行内、下拉箭头后追加徽标元素；徽标纯展示，不拦截 chip 点击/下拉/重命名。
- 样式：小尺寸次级 badge，主题自适应；复用现有 tooltip 模式，hover 显示 config dir 全路径。

## 3. 不变项

- OS 窗口标题不动；picker/daemon/远端/switcher 逻辑不动；标题栏按钮、拖拽、tab 布局不动。
