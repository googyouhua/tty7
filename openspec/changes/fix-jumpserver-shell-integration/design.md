# 设计：bootstrap 的 pty 兜底（koko 无 pty exec 场景）

## 方案（单一方案）

`remote::bootstrap_command` 生成的三个 bootstrap 脚本（zsh/bash/fish）结尾的
`exec <shell> …` 改为条件分发：

```sh
if [ -t 0 ] || ! script -qfec true /dev/null 2>/dev/null; then
  exec '<shell>' …            # 现状不变（有 pty，或无 script 可用）
else
  export TERM=xterm-256color COLORTERM=truecolor
  exec script -qfec '<payload>' /dev/null
fi
```

- `[ -t 0 ]`：exec 通道 stdin 是 pty（正常 sshd 兑现了 pty-req）→ 原路径，
  且**短路在 probe 之前**（健康主机零探测开销）。
- `||` 链每项一个 `!`——**禁止把探测写成 `A && B` 再 `!` 之**：shell 的
  `&&`/`||` 同优先级左结合，会静默反转"无 script 机器"的分支，把退路变成
  `exec` 一个不存在的 `script` 而杀死 pane（独立审查抓到的 CRITICAL）。
- 探测用与 exec 完全相同的旗标（`-qfec`）：`-e` 让 shell 退出码穿透
  （普通 exec 本来就有此语义），busybox 等不支持 `-e` 的实现会探测失败
  → 落回普通 exec，而不是在 exec 时死掉。
- payload 经各自方言的 quoter 整体转义后交给 `script -c`。
  **注意：`script -c` 的中间解释器是 `$SHELL`（登录 shell），不是 `/bin/sh`**：
  bash/zsh 的 payload 是 POSIX（两种 shell 同构）；fish 的 payload 就是
  else 分支的同一条 fish 行（stty 前缀为 fish 3+ 语法），fish-quote 一层即可。

## payload 构成

`stty rows R cols C 2>/dev/null; exec <原 exec 目标（去掉 exec 字）>`

- R/C：会话打开时 pane 的 cols/rows，由 `ssh/mod.rs` 传入（仅 >0 时加 stty）。
  `script` 兜底 pty 初始为 0x0，不设尺寸会让 vim/readline 布局错乱。
- bash 的 rcfile 路径含运行时 `$$`：外层 bootstrap 先 `export TTY7_RM_DIR`，
  payload 内以 `"$TTY7_RM_DIR/bashrc"` 引用（内层 shell 展开）——不额外导出
  `__tty7_d`，健康路径的环境不受污染。
- zsh：payload 仅 `exec '<zsh>' -l`，ZDOTDIR 已在外层 export，经 script 继承。
- fish：payload 与 else 分支同一条 fish 行，整体 fish-quote。

## 各 shell 结尾形态（生成后）

```sh
# bash/zsh（POSIX if/else）
if [ -t 0 ] || ! script -qfec true /dev/null 2>/dev/null; then
exec '/bin/bash' --rcfile "$TTY7_RM_DIR/bashrc" -i
else
export TERM=xterm-256color COLORTERM=truecolor
exec script -qfec 'stty rows 24 cols 80 2>/dev/null; exec '\''/bin/bash'\'' --rcfile "$TTY7_RM_DIR/bashrc" -i' /dev/null
fi
```

fish 同构（fish 语法的 `if not test -t 0; and script -qfec …`，payload 为
else 分支同行加 stty 前缀，整体 fish-quote）。

## 兼容与退化

| 环境 | 行为 |
|---|---|
| 正常 sshd（pty 兑现） | `[ -t 0 ]` 真 → 与现状逐字节一致 |
| koko（exec 无 pty）+ util-linux script | 内层 pty 兜底，全功能恢复 |
| 无 script / BSD script（`-qc` 探测失败） | 退化为现状（裸 exec，无集成收益但不出错） |

## 已知限制

- 会话中途 resize 不再同步到内层 pty（koko 无 pty 即无 winch 通道）；初始尺寸由 stty 修复。
- `script` 不存在时无法兜底（维持现状，不恶化）。

## 不做的事

- 不改 probe（exec 通道纯文本，无需 pty）
- 不动 `request_shell` 路径（shell_integration=false 场景）
- 不改 koko/jms 侧配置（无权限；若 koko 对 request_shell 也剥 pty 属服务端问题，另行反馈）

## 验证计划

1. 单测（CI 可跑）：管道 stdin 下执行生成的 bootstrap → 断言内层进程 `tty` 非 "not a tty"、
   回显开启、133 标记齐全；有 pty 路径回归不变。
2. 实弹：经 jms 一次性口令 loopback 验证 exec 路径 TERM/回显/标记（需用户当场发口令）。
