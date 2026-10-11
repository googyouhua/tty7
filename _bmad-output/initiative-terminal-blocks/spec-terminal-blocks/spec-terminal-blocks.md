---
id: SPEC-terminal-blocks
companions:
  - block-model.md
  - interaction.md
sources:
  - ../brief-terminal-blocks/brief-terminal-blocks.md
  - ../brief-terminal-blocks/addendum.md
---

> **Canonical contract.** This SPEC and the files in `companions:` are the complete, preservation-validated contract for what to build, test, and validate. Source documents listed in frontmatter are for traceability — consult them only if you need narrative rationale or prose color this contract intentionally omits.

# terminal-blocks

## Why

长输出终端里回翻定位与完整复制一次执行结果的成本过高：用户翻不到命令起点，复制要手滚选区。tty7 已有 shell 集成（OSC 133 prompt marks、cwd、退出码、command-finished 通知），但 scrollback 仍是平的，没有“一次执行”的单位。引入命令块把同一数据源变成可折叠、可跳转、可复制的单位。

## Capabilities

- **CAP-1**
  - **intent:** 用户可将任一命令块手动折叠为一行摘要并展开恢复，以收起长输出。
  - **success:** 万行输出块经 gutter 或右键折叠后只占一行，展开后内容无损。
- **CAP-2**
  - **intent:** 用户在块内可一跳定位到该块命令起始行，不再手滚。
  - **success:** 右键菜单“跳到块首”一次点击将视口置于块首命令行动画无丢失。
- **CAP-3**
  - **intent:** 用户可一次复制整个块的纯文本（命令 + 输出 + 退出码行）。
  - **success:** 复制结果与 `capture --plain` 同源语义，粘贴含完整命令、输出与退出码行。
- **CAP-4**
  - **intent:** 用户可一眼辨别失败与中断的块。
  - **success:** 非零退出块红色标记；Ctrl-C 中断也成块并标记。
- **CAP-5**
  - **intent:** 用户重启后折叠态仍在。
  - **success:** 同一 pane 重启恢复后折叠与展开态与关闭前一致；scrollback 被清除的块不再出现。
- **CAP-6**
  - **intent:** 用户在远端 pane 获得与本地一致的块。
  - **success:** 远端直连 pane（远端集成成功）折叠/跳转/复制/红标与本地一致；嵌套 ssh 只支持 1 层：外层 B/C 后内层 B/C/D 正常配块，第 2 层及以上出现作废内外两槽（无块），内层无 marks 时外层整轮成一块。

## Constraints

- 块边界唯一来源是 OSC 133 A/B/C/D；无 marks 的 pane 不分块，不用正则猜 prompt。
- alt-screen 程序（vim、全屏 TUI）不算块、不折叠。
- 复制为纯文本；成功与失败块统一带退出码行；v1 无 ANSI 富文本复制。
- 块级 gutter 与右键菜单 v1 新做，两端一致；v1 无自动折叠、无快捷键。
- 嵌套 ssh 的内外层归属由 server 侧 pane 状态机判定，渲染只读；只支持 1 层嵌套（两槽 + C 分水岭），第 2 层及以上作废两槽只丢块不记错块。
- 11MB `cat` 五次平均相对基线退化≤10%，折叠/跳块首 <100ms。

## Non-goals

- 无集成的启发式或正则分块。
- outline / 命令面板、多块多选复制。
- 按块搜索、按块分享、agent 引用块 ID（见 brief Vision parked）。
- 快捷键体系与搜索联动。

## Success signal

用户在万行输出内 2 次点击回到块首并复制出完整纯文本执行结果；有 marks 的 pane 命令与输出不错位，折叠重启可恢复。

## Assumptions

- 退出码行统一带（成功 `exit 0` 与失败均进剪贴板），具体前缀格式由渲染定。
