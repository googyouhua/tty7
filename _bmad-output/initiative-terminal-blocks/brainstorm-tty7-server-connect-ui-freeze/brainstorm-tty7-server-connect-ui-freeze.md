# tty7 连接 tty7-server 时 GUI 卡死 — 意图文档

状态：brainstorm 完成，待进入 brief / spec
范围：tty7（Rust 终端工作台，gpui GUI + headless crates）

## 1. 问题与影响

在某一台特定机器上，GUI 连接 tty7-server 主机时会无可恢复地冻结：窗口失去响应，没有任何退出入口，用户只能重启整个应用。该问题在此机器上必现，直接阻塞日常工作。此前两次修复（`eed2994f`、`e9c94b68`）都只补了 ssh 密码 sheet 的停放路径，冻结仍复现，说明还有别的门或状态在持有输入。

## 2. 目标与非目标

**目标不变量：连接过程中 GUI 永不失去可操作性。** 任何时刻都存在可点击、可键盘触达的取消/退出路径，且当前阶段与已等待时长始终可见。

非目标：

- 不重写 ssh 协议，不改远端 tty7-server 的协议与行为；
- 不以"连接更快"为目标，性能优化不是本议题；
- 不接受"换机器 / 换网络 / 少开窗口"的绕过方案；
- 不以关闭远程连接功能为代价换取稳定。

## 3. 已确认的技术事实

| 事实 | 含义 |
| --- | --- |
| `connect_blocking` 运行在 gpui 的 `background_executor` 上 | UI 线程没有被直接阻塞，卡死更像 UI 状态 / 弹层 / 门控问题，而非 CPU 忙等 |
| `ControlClient::call`（handshake 与 `list_workspaces` 共用）没有读超时 | 对端静默时握手线程永久挂起，`ConnectFlow` 永远是 `Connecting` |
| modal gate / sheet 是当前唯一的输入通道 | overlay 的 scrim 吞掉全部输入且没有 Esc 分支，视觉上等价于冻结 |
| 两次先前修复都落在 sheet 停放路径 | 缺陷在"弹层是唯一输入通道"这一结构，而不是某一条具体路径 |

## 4. 根因假设（按优先级）

1. **所有权缺失** —— "卡死不是「慢」，而是没有任何东西拥有超时与所有权：RPC 无 deadline、gate 无租约、attempt 无终点。" 后台任务可以无限等待，且没有主体对"结束"负责。
2. **可见性缺失** —— "连接阶段、谁持有 gate、等了多少秒全都没暴露，所以任何一次静默都表现为不可区分的冻结。" 用户与开发者都无法区分"在等"与"已死"。
3. **复发结构** —— "两次修复都在补 sheet 停放路径，说明缺陷在「弹层是唯一输入通道」这个架构，而不是某一条路径。" 只要输入仍被单一 gate 独占，就还会有第三条未被覆盖的路径。

环境相关性（仅本机复现）最可能落在五个外部依赖上，且必须用注入排除而非猜测：keychain/dbus、ssh-agent、DNS、GPU 首帧、proxy/ProxyCommand。

## 5. 方案：今天层（当天可发，直接解阻塞）

- `ControlClient::call` 加 20s 读超时，超时归为可重试错误；
- 全局脱困快捷键：一次取消所有 attempt、释放所有 sheet、清空 gate；
- `Connecting` 超过 45s 自动转 `Failed`，并给出重试与取消；spinner 显示已等待秒数，超阈值变色提示可取消；
- 连接状态从 modal 降级为窗口顶栏的连接条，永不吞输入；每个 sheet 都能 Esc 关闭且带默认安全动作；
- CLI 逃生口：`tty7 disconnect` / `tty7 status`，绕过 UI 恢复；
- 每个阶段打一行带 attempt id 与时间戳的 log；`TTY7_DEBUG_CONNECT=1` 打印连接阶段机；失败页提供"导出诊断"（dump 线程栈与状态表）。

## 6. 方案：架构层（防复发）

- 删掉全局 modal gate，改为每窗口独立的非阻塞任务坞；
- gate 从手工计数改为 RAII 租约，带自动超时释放；
- 删掉"等待用户输入"的隐式阻塞，改为可取消的 pending question 队列；
- 每个 attempt 拥有独立生命周期，窗口销毁即取消其名下 attempt，不再有 supervisor 单点与窗口死亡后任务继续的分离；
- 零 UI 线程 I/O：keychain、dbus、ssh 全部移出，改后台加超时；
- machine tree 改增量加载，不在 UI 线程全量反序列化；
- 错误一律非阻塞通知；spinner 由 UI 自己计时，不依赖后台汇报；重启后不继承"连接中"状态。

## 7. 验收标准

- 任何连接在 45s 内进入可取消状态，取消后 UI 可继续操作；
- CI 断言 UI 线程单次阻塞小于 16ms；
- 毒化 / 故障注入用例进 CI：iptables DROP 黑洞、SIGSTOP 伪造 ssh 挂起、删除 ssh-agent socket、无 dbus 环境、5000 workspace fixture、50MB 畸形响应、DNS 30s 超时、daemon 延迟 60s；
- 重启后不继承"连接中"状态，无需重启即可从任何失败态恢复；
- 存在 headless gpui 的 UI 级连接测试，覆盖不止 core 层。

## 8. 下一步（取证优先）

1. **取 hang 现场**：线程栈 + gate/attempt 状态 dump + 阶段 log，确认卡住时最后一条 log 停在解析、ssh 认证、握手、列 workspace 还是建 pane。
2. **隔离复现**：iptables 黑洞、`SIGSTOP` ssh、无 dbus 三种注入，逐一比对是否复现同一现场。
3. **最后才改代码**：先用第 5 节的最小集解阻塞，再按第 6 节替换结构。

五个外部依赖嫌疑人：keychain/dbus（密钥读取阻塞、密码 sheet 不出现）、ssh-agent socket 指向失效路径、DNS（`/etc/hosts`、mDNS 解析慢或失败）、GPU 首帧绘制、proxy/ProxyCommand 介入。
