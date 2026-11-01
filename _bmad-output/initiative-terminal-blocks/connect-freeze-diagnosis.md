# 连接 tty7-server 时 UI 卡死 — 取证报告

日期：2026-10-30
机器：本容器（`/ws/tty7`，root，无 dbus/keyring/ssh-agent）
关联：`brainstorm-tty7-server-connect-ui-freeze/`（memlog + HTML + intent doc）

## 1. 结论摘要

1. **本机的 tty7-server 是健康的**：`tty7 doctor` / `status` / `ls` 全部在 7–17ms 内返回，build 26.9.4，dialect control v12 / protocol v6，9 个 pane，已运行 5 天。**不是远端无响应或版本不匹配。**
2. **客户端存在无 deadline 的等待**：`router::negotiate` 的 `read_frame` 循环、`ControlClient::call`（hello / `MachineGet`）都没有读超时。对端只要沉默，`connect_blocking` 就永远不会返回 —— 这是"卡住且无法恢复"的机制性基础。
3. **认证提示是有上限的**（120s `PROMPT_TIMEOUT`），所以**超过 120s 仍不恢复的冻结**，最可能停在认证之前（ssh 启动/路由）或之后（hello / `MachineGet`）的静默等待上，而不是等人点密码。
4. **本机缺三样外部依赖**：dbus、keyring、ssh-agent 全都没有；sshd 在 **2222** 而不是 22。任何依赖这些的路径（凭据读取、agent 认证、默认端口的机器配置）在这台机器上都会走降级分支 —— 这正是"只有这台机器复现"的高概率入口。

## 2. 已核实的代码事实

| 位置 | 事实 |
| --- | --- |
| `src/ui/remote_workspace.rs:711` | `connect_blocking` 通过 `cx.background_executor().spawn(...)` 执行 —— UI 线程没有被直接阻塞，所以卡死是**状态/门控**而非 CPU 忙等 |
| `crates/tty7-core/src/daemon/router.rs:298` | `negotiate` 写 header 后进入 `loop { read_frame(...) }`，**没有 set_read_timeout，也没有 deadline**；并在同一线程同步调用 `RoutePrompt` 处理器（auth / install / mismatch / progress） |
| `crates/tty7-core/src/host/remote.rs:121`、`daemon/control.rs:1232` | `ControlClient::call` 只做 write + read，**无超时**；`list_workspaces` 的 `MachineGet` 走这条路 |
| `src/ui/remote_connect.rs:857` | `ask_routed` 用 `rx.recv_timeout(AUTH_TIMEOUT)`，`AUTH_TIMEOUT = PROMPT_TIMEOUT`（120s）—— 认证提问本身有上限 |
| `src/ui/remote_connect.rs:342` | `connect_blocking` 顺序：`ensure_running` → `transport::connect` → `negotiate` → `handshake` → `list_workspaces` → `refresh_agent_hooks_once`（后者在独立线程） |

由此得到冻结的三种可区分形态：

- **A（认证前静默）**：`negotiate` 卡在 `read_frame` —— ssh 连上了但对面不给 route ack（远端 shell 启动卡住、非要交互的东西、端口/ProxyCommand 指到别处）。无上限。
- **B（认证中）**：在 `ask_routed` 里等 sheet —— 上限 120s，到点变 `authentication cancelled`。
- **C（认证后静默）**：`handshake` / `MachineGet` 卡在 `ControlClient::call` —— 远端 daemon 活着但不回复（例如被别的连接/锁拖住）。无上限。

用户描述的是"无法恢复"，因此重点应在 **A 和 C**；B 只是"很久"，不会永久。

## 3. 本机环境事实（实测）

```
tty7 doctor       → server ok (build 26.9.4), dialect ok (control v12, protocol v6),
                    status pid 3205, up 444933s, 9 panes, machine links 0 known 0 connected
tty7 status       → 7ms
tty7 ls           → 13ms，5 个 workspace（wsl-docker 9 panes）
ssh               → sshd 监听 :2222；:22 直接 connection refused（16ms）
~/.ssh/config     → 只有 github.com 的 ProxyCommand（ssh.github.com:443）
ssh-agent         → SSH_AUTH_SOCK 未设置，ssh-add 失败
dbus / keyring    → DBUS_SESSION_BUS_ADDRESS 未设置；secret-tool / gnome-keyring-daemon / dbus-launch 均不存在
GPU               → 无 /dev/dri（软件渲染）；GUI 实例跑在 DISPLAY=:99
服务端二进制      → ~/.local/share/tty7/bin/ 同时存在 tty7-server-c10p6 与 -c12p6（c12p6 在跑）
持久化窗口        → ~/.config/tty7/views.json 里 window.x = -80（负坐标，open: true）
```

要点：

- **端口 22 vs 2222**：如果 GUI 里"这台机器"的 profile 用了默认 22，连接会立刻失败（不是卡死）；如果用了 2222，才会进入正常的 ssh 流程。值得先确认 profile 用的是哪个。
- **两个服务端二进制**：`c10p6` / `c12p6` 并存意味着历史上换过 dialect；远端拉起/安装时选错二进制会落到 mismatch → install 分支，那是另一条容易停住的路径。
- **无 dbus/keyring**：凭据只能靠密码 sheet 或密钥文件。密码 sheet 的上限是 120s，因此这解释不了"永久"。但它会让每次连接都不得不弹 sheet，从而把 A/C 的静默暴露成"卡在连接中"。
- **负坐标窗口**：`views.json` 里 x=-80 会随窗口恢复被复用。若用户看到的是"窗口在但点不动/看不到"，先排除这一项。

## 4. 下次复现时的取证方案

`capture-connect-hang.sh`（同目录）在卡住时抓一份现场，包含：

- UI 线程与全部线程的 wchan / 栈（`/proc/<pid>/task/*/stack`、`gdb -p` 的 `thread apply all bt`）
- 该进程打开的控制连接、ssh 子进程及其状态
- 本地 daemon 的 `tty7 status`、`tty7 ls`（判断是"daemon 也卡"还是"只有客户端卡"）
- `~/.config/tty7/views.json`、`machine.json` 摘要、最近日志

判读规则：

- daemon 的 `status` 也卡 → **C 类**，问题在 daemon 侧（锁/线程），客户端只是没有超时。
- daemon 秒回、客户端线程停在 `unix_stream_read` / `socket read` → **A 或 C 的静默等待**，用 `strace -p <tid>` 看是否有字节到达。
- 客户端线程停在 futex/condvar 且主线程在 `run_until_parked` 之外的等待 → 门控问题，直接导出 `AUTH_MAILBOX` / gate 状态。

## 5. 修复优先级（与 intent doc 一致）

**今天层（解阻塞）**

1. `negotiate` 与 `ControlClient::call` 加读超时（20s），超时映射为可重试错误；
2. `Connecting` 45s 自动转 `Failed`，spinner 显示已等秒数；
3. 全局脱困快捷键（取消所有 attempt、释放 sheet、清 gate）；
4. 连接阶段可视化 + `TTY7_DEBUG_CONNECT=1` 阶段日志；
5. `tty7 disconnect` / `status` 作为 UI 之外的逃生口。

**架构层（防复发）**：任务坞替代全局 modal gate、gate RAII 租约、可取消 question 队列、每 attempt 独立生命周期、UI 线程零 I/O。

## 6. 尚未确认的两件事

1. **A 还是 C** —— 需要一次真实卡住的现场（见第 4 节），或让 GUI 在 Xvfb :99 上复现后用脚本抓取。
2. **GUI 里那台机器的 profile 到底连的是 22 还是 2222、用密码还是密钥** —— 这决定了它是否每次都必须走 sheet，从而把 A/C 的静默放大成"永远在连接中"。
