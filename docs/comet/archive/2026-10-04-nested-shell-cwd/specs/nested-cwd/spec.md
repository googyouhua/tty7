# Nested-shell cwd — 完整目标规格

## 能力
本地 pane 在嵌套 shell（`sudo -s`/`su`/内层 bash）中 `cd` 后，cwd 照常跟随。

## 行为
- daemon 对本地 pane 维护 cwd：shell 上报（现有 `DaemonMsg::Cwd`）优先；当上报停滞（嵌套 shell 不走 rcfile）时，Linux 用 `/proc/<前台进程>/cwd` 兜底，macOS 用 `PROC_PIDVNODEPATHINFO` 兜底，两边都是 pgid→组内最深进程解析。
- 前台进程判定：沿用 daemon 已有 phosphor/procinfo 能力（`daemon/procinfo.rs`），取 PTY 前台进程组。
- GUI 侧零改动：`foreground_cwd`（`terminal/remote.rs:2015`）与各面板调用方不变。
- 退出内层 shell 后恢复以外层上报为准（上报恢复新鲜即切换回来，不与 proc 打架的规则要在实现中定死并写单测）。

## 非目标
- Windows proc 兜底、远端 pane 的标题兜底之外、本次不动 Review Tab。
- 手动覆盖不持久化。

## 远端标题兜底（A5）
- 仅远端 pane（`st.remote.is_some()`）：标题严格解析出绝对路径，且自上次 OSC7 上报起超过 staleness 阈值（实现中定死，单测锁定），才采用为 cwd 并下发 `DaemonMsg::Cwd`。
- 新鲜 OSC7 永远优先：远端用户 shell 恢复上报（退出 su）后立刻赢回。
- `~` 仅标题用户为 root 时映射 `/root`；相对路径、一行多义、格式不符一律放弃。

## 手动覆盖（A6）
- `TerminalView` 新增会话级 `manual_cwd: Option<PathBuf>`；`effective_cwd()` 最优先返回它。
- Info 行 cwd 值可点击进入行内文本输入：回车提交（绝对路径才接受，远端 pane 接受远端绝对路径）、Esc 取消、提交空值清除覆盖。
- Win/Linux 同一套纯文本输入，不调用系统路径弹窗；需新增 L10n 文案。

## 跟随开关（A7）
- Info 面板 cwd 下方独立开关行（toggle 样式）：per-pane，会话级，默认开（=off 才不跟随）；手动 pin 在追踪值回到同目录时自动解除、显示不动（roach-motel 不出现）。
- 开关只门控标题兜底的本地部分；A5 远端常开不受影响。
- GUI→daemon 经 per-pane 标志下发（`PaneState` 新增字段 + 新 `ClientMsg`，参考 clipboard 权限先例）；daemon `apply_title_cwd` 本地分支要求开关开。

## 穿透上报（A8，bash）
- bash rcfile 末尾：把组装好的 PROMPT_COMMAND 冻成字符串藏进 `__tty7_outer_chain` 函数，export 的 PROMPT_COMMAND 只含 `__tty7_outer_chain 2>/dev/null` + 自包含 OSC7 单行（`builtin printf` + `$HOSTNAME`/`$PWD`，零函数引用）；字符串/array 统一处理。
- 外层行为不变（同一批命令同顺序执行，多一行同值上报，去重吃掉）；`su`（不带 `-`）与透传型 `sudo` 的内层 bash 静默跳过缺失函数后精确上报；`su -`/`sudo -i`/fish 等内层仍走标题/手动/开关。
- zsh 不做：中转器首个 precmd 即把 ZDOTDIR 指回用户目录（既定设计），内层 zsh 够不到我们的文件，无搭车通道；fish/pwsh/nu/sh 同理无通道。
- 单测：rcfile 内容断言（export 存在、尾巴零函数引用、外层链被包裹）+ 真 bash 函数真空 behavioral 测试 + 现有集成单测回归；25 实机以 `sudo -s` 为 headline 验证。

## 验收映射
- A1 → 外层回归；A2 → 本地内层跟随；A3 → 退出恢复；A4 → 现有单测；A5 → 远端标题兜底；A6 → 手动覆盖；A7 → 跟随开关；A8 → 穿透上报。
