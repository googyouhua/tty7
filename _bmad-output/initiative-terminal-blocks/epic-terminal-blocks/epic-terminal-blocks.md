---
tracker_id: ""
key: ""
type: epic
title: "命令块：折叠/跳转/复制/红标/持久化/远端一致"
parent: initiative-terminal-blocks
covers: ["CAP-1", "CAP-2", "CAP-3", "CAP-4", "CAP-5", "CAP-6"]
after: []
assignee: ""
risk: medium
---

# 命令块：折叠/跳转/复制/红标/持久化/远端一致

## Description

server 模式（OSC 133 有 marks）下的命令块：一次执行一单位，可折叠、可跳块首、可整块纯文本复制，失败红标，折叠随 pane 持久，远端一致。需求源为同目录外的 `spec-terminal-blocks/`（复用其 CAP-N，不另起 ids）。

## Outcome

跑构建/长日志的用户在万行输出内 2 次点击回到块首并复制出完整纯文本执行结果，不再手滚选区。

## Requirements

沿用 `spec-terminal-blocks/spec-terminal-blocks.md` 的 CAP-1~CAP-6（intent+success 见源，不复述）：CAP-1 手动折叠/展开；CAP-2 右键跳块首；CAP-3 整块纯文本复制（命令+输出+退出码行）；CAP-4 失败红标与 Ctrl-C 成块；CAP-5 折叠持久化；CAP-6 远端一致与嵌套分块。

## Done when

- 万行输出内 2 次点击回到块首并完成整块纯文本复制。
- 有 marks 的 pane 命令与输出不错位；Ctrl-C 与失败块标记正确。
- 同一 pane 重启后折叠态一致；scrollback 清除的块消失。
- 11MB `cat` 五次平均退化≤10%，折叠/跳块首 <100ms。

## Boundaries

单 epic：单 owner、单模块（server pane 状态 + gpui 视图）、共享 block-model 契约，不按 CAP 拆分。spec 非目标同样适用：无集成不猜块、无自动折叠、无快捷键、无 outline、无 ANSI 复制、无搜索联动。

## References

- parent — ../brief-terminal-blocks/brief-terminal-blocks.md
- spec — ../spec-terminal-blocks/spec-terminal-blocks.md (CAP-1~CAP-6)
- companion — ../spec-terminal-blocks/block-model.md
- companion — ../spec-terminal-blocks/interaction.md
- constraint — spec 性能条（11MB cat / 155×40 基线）

## Notes

- Decision: 2026-10-07 单 epic 吃 CAP-1~6，不拆分（单 owner 单模块同契约）。
- Decision: 2026-10-07 tracer 先行，entry-1 为穿全层最薄路径；block schema 与退出码行格式的 owner 是 entry-1。
- Decision: 2026-10-07 退出码行统一带（成功与失败均进剪贴板）。
- Decision: 2026-10-07 嵌套归属放 server 侧 pane 状态机，渲染只读。
- Decision: 2026-10-07 性能从严：退化≤10%、交互<100ms；性能门放最后测终态。
- Decision: 2026-10-07 保留 Refactor sweep；不单开 e2e suite，性能门即 closing verification。
- Source conflict: block-model.md“归属判定见 Open Questions” vs spec 约束与用户拍板（server 侧）——已按后者修正 block-model，见 spec memlog。
