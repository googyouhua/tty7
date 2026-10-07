# block-model

块的数据模型与生命周期（CAP-1、CAP-4、CAP-5、CAP-6 的展开；实现细节归架构）。

## 边界

- 起：命令行的 `B` mark；止：对应的 `D;<code>` mark；`A`/`C` 保留用于 prompt/输出区分。
- 无 `B→D` 配对不成块；配对跨 scrollback 清除即失效。
- alt-screen 期间的 marks 不建块。

## 状态

- 每块：`id`（pane 内稳定）、`exit_code`、`folded: bool`、`truncated: bool`（输出超 ring 时）。
- `folded` 存 server 侧 pane 状态，随持久会话恢复；`scrollback` 清除删除块。

## 远端

- 直连远端 pane：远端 `sshd <shell> -c <bootstrap>` 成功即与本地同模型；失败即无块。
- 嵌套 ssh：以外层忙为前提，内层端到端 marks 可达才建块；归属由 server 侧 pane 状态机判定，渲染只读。
