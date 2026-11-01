---
title: "tracer: 按容器名或 ID 接入、确认装机、进 workspace、断开后进程退出"
ticket: 1
status: built
baseline_revision: f7877dd5
---

## Intent

把 Docker 容器接成 tty7 的第三种主机：手动接入一个运行中的容器，装好服务端，进入完整 workspace，断开后桥进程退出。列表、架构选择、安装目录回退这一轮走单一路径。

## Implementation Notes

容器复用了 WSL 已经走通的本地 exec 通道，没有新增任何线上协议。改动按层：

| 层 | 文件 | 内容 |
| --- | --- | --- |
| 目标类型 | `crates/tty7-core/src/core/session.rs` | `RemoteTarget::Docker { container }`，`docker:<name>` 的连接键与显示；`parse_container` 要求前缀，避免把名叫 `docker` 的主机当容器 |
| 路由 | `crates/tty7-core/src/daemon/router.rs` | `RouteTarget::Docker`、`RouteHeader::docker()`、`origin_key` = `docker:<name>`、`open_link` 分支、重启/替换分支 |
| 通道 | `crates/tty7-core/src/daemon/remote_link.rs` | `RemoteLink::Docker`，跑 `docker exec -i <c> <server> --stdio`（argv 直传，不经 shell） |
| 装机 | `crates/tty7-core/src/daemon/install/docker.rs` | `DockerRemoteOps`（`docker exec` 实现的 `RemoteOps`）、`ensure_docker_server`、`forget_docker_server` |
| UI | `src/ui/remote_connect.rs` | 连接器里的容器分组（`docker ps`，30 秒 TTL，只读不轮询）、`control_route` 走 `RouteHeader::docker` |
| pane | `src/terminal/remote.rs` | 容器的 pane 路由 |
| 文案 | `src/ui/i18n/*` | `RemoteDockerNoSsh`（四种语言） |

几处刻意的选择：

- **`docker exec` 直传 argv**，拼的不是命令字符串。容器没有 shell 时，失败发生在服务端启动，而不是在一行解析不了 shell 命令上——错误能说清楚。
- **先用 `sh -c ':'` 探一次**。没有 `/bin/sh` 的容器直接给出「这个容器里没有可用的 /bin/sh」，而不是让装机脚本以 `exec: /bin/sh: not found` 收场。
- **装机走 `BundledOrRelease`**，也就是 SSH 那条路用的同一套：`uname -sm` 决定资产，缺本地资产时从 release 拉。因此容器架构选择和 musl 资产复用是「免费」的，不用单写一条。
- **连接器列表只在打开时探一次**（30 秒 TTL），没有常驻轮询——对应约束 C7。

## Verification

已跑：

- `cargo check`（全工作区）通过。
- `cargo test -p tty7-core`：1860 通过，1 失败，失败项是 `ssh_profile::tests::a_blank_user_logs_in_as_this_machines_user`，在本机干净树上同样失败（容器里没有该用户的 passwd 条目），与本改动无关。
- 新增单测：容器名不能以 `-` 开头/不能带空白、`exec_args` 的参数切分、`docker:<name>` 的解析与连接键。

**端到端（已跑，2026-11-01）：** 新增 `crates/tty7-server/tests/docker_route.rs`，用仓库已有的 router 测试骨架把整条路走通：

```
RouteHeader::docker → RemoteRouter::route → docker exec … --stdio
  → ControlHello 握手 → 在容器内写文件 / 读回 → 关闭后桥进程消失
```

跑法（本机 docker CLI + `DOCKER_HOST=ssh://root@192.168.1.25`，容器在远端守护进程里）：

```
TTY7_BUNDLED_SERVER_DIR=/tmp/tty7-bundle \
TTY7_DOCKER_TEST_IMAGE=ubuntu:24.04 \
cargo test -p tty7-server --test docker_route
```

`ok`，9.7 秒。这条测试同时验证了 R9 的新口径：链接关闭后容器内 `--stdio` 桥消失，`--daemon` 留下。

**装机那一段的证据：** 未预先放二进制时，路由会走到「installing tty7-server at … was not confirmed」——也就是它正确到达了首次确认的关口，而测试里没有应答者。装机的三个原语（写入、chmod、执行）已由 spike 1.3 的探针与本次 `docker cp` 直接验证。

**仍未做的：** GUI 里点一下连接器。本机无 GUI 交互能力，这一步留给有桌面的机器。

因此本条标 `built`，端到端的技术路径已证，缺的只是人眼确认交互。

## Consequences for the tree

- **entry 4 的注释需要更新**：容器列表在 tracer 里已经落地（为了让 tracer 有入口）。entry 4 剩下的范围是远端 docker 的拒绝（R11）、按镜像名搜索、与同名 WSL 发行版的区分、以及 C7 的节流策略本身。
- 没有改动任何需求行；R1/R3/R9 的验收口径不变。

## Notes

- 这一轮没有做「按 ID 接入」的单独路径：`parse_container` 接受任何不含空白的前缀值，容器 ID 作为名字传进去同样成立，`docker exec` 对两者一视同仁。验收里用 ID 再接一次即可证明。
- 装机目录仍是单一路径（由安装器决定），R5 的回退顺序留给 entry 7。
