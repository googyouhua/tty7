# Addendum: terminal-blocks

用户在陪练中给出的下游直达材料，brief 放不下但 PRD/架构/设计要用的部分。

## 技术约束（架构输入）

- 块边界唯一来源：OSC 133 A/B/C/D；无 marks 的 pane 不分块。
- alt-screen 程序（vim、全屏 TUI）不算块、不折叠。
- 嵌套 ssh：端到端 marks 可达则分块，否则无块；需区分外层忙与内层 prompt（见 `protocol.rs:607` 防误触逻辑），实现细节留给 `bmad-architecture`。
- 远端直连 pane 靠 `sshd <shell> -c <bootstrap>` 注入集成；失败按无集成处理（与 `exec/wait` 现有判定一致）。
- 折叠态存 server 侧、跟 pane 生命周期；重启恢复；scrollback 清除则块消失。
- 复制为纯文本（`--plain` 同源语义），内容 = 命令行 + 输出 + 退出码行。
- 性能基线：11MB `cat` / 155×40 同机基准，块索引不能让交互可感知退化。

## 交互细节（UX 输入）

- v1 手动折叠，无自动折叠阈值；gutter 或右键折叠/展开。
- 右键菜单至少：跳到块首、折叠/展开、复制命令、复制输出、复制两者（含退出码行，两者为默认主项）。
- 失败块红色标记（含 Ctrl-C 非零情形）；双击选中行为与现有选区不冲突为前提。
- v1 无快捷键；outline 面板、ANSI 复制、多块多选明确延后。

##  parked

- 按块搜索、按块分享、agent 引用块 ID。
