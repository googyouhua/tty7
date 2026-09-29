# 修复经 JumpServer 跳板 SSH 登录后终端无回显/阶梯输出

## 问题

tty7 经跳板机 `jumpserver-v3.yunsilicon.com:2222`（JumpServer/koko）SSH 连接资产机时，pane 内出现：

- 输入不可见（无回显）
- 输出阶梯状（`\n` 未被翻译成 `\r\n`，缺 ONLCR）
- `TERM=dumb`
- shell integration 的 C/D（命令开始/结束）标记缺失

**直连 IP（同一资产机，sshd :2022）一切正常**——差异只在 koko 这一层。

## 根因

tty7 的 SSH pane 流程：`request_pty → exec(bootstrap 脚本)`（同一通道，见 `ssh/mod.rs:307-320`）。
资产机侧证据表明 koko 把 **exec 会话以无 pty 方式代理**到资产机：

- exec 路径实测 `TERM=dumb`（pty-req 请求的是 xterm-256color，若 pty 兑现则不可能出现 dumb）
- 无 pty 时 sshd 用管道跑命令：无 ECHO（输入隐身）、无 ONLCR（阶梯）、无 job control（bash-preexec 静默 → C/D 缺失，见 `shell_integration.rs:294` 门控）
- bootstrap 本身工作正常（rcfile 落地、133;A/B/V + OSC 7 均回传）——排除脚本问题

即：**koko 未对 exec 会话兑现 pty-req，tty7 的 bootstrap exec 落地后 shell 没有 tty**。

### 实弹确认（2026-09-29，jms 一次性口令 loopback）

`ssh -tt -p 2222 JMS-<token>@jms '<exec 命令>'`（强制 pty-req + exec，复刻 tty7 请求序列）：

```
===MARKER===
sw-lab-ylf131          ← 路由正确
TTY_STATE=NO_TTY       ← stdin 非 tty
TERM=dumb              ← pty-req 的 xterm-256color 被丢弃
not a tty
stty: 'standard input': Inappropriate ioctl for device
SHELL=/bin/bash
```

客户端已明确请求 pty 的前提下，koko 代理的 exec 会话仍以纯管道落地——根因实锤。

## 修复目标

bootstrap 脚本在资产机侧自检：无 tty 时用 util-linux `script` 在**资产机本地**兜底分配 pty 再 exec 用户 shell。

- 回显恢复（内层 pty ECHO）
- 换行恢复正常（内层 pty ONLCR）
- 作业控制恢复 → bash-preexec 启用 → C/D 标记恢复
- 正常直连路径（有 pty）行为零变化；无 `script` 的环境退化为现状
