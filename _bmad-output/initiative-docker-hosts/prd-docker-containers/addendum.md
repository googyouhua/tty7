# Addendum: Docker 容器作为 tty7 主机

配套文档：`prd-docker-containers.md`。这里放技术机制、备选路线、被否决的理由与外部证据。PRD 正文只写能力，不写这些。

## 1. 现有接缝（为什么这不是新协议）

远端能力的实现已经抽象到"一条字节流上跑控制协议"，主机类型差异只体现在**怎么拿到这条流**。

| 接缝 | 位置 | 说明 |
| --- | --- | --- |
| `RemoteTarget::LocalStdio { program, args }` | `crates/tty7-core/src/core/session.rs:139` | 任意程序 + `--stdio` 即一台主机；容器只需换成 `docker exec -i <c> <server> --stdio` |
| `RemoteTarget::Wsl { distro }` | 同上 `:136` | 本地 exec 通道的第一个实例，Docker 是第二个 |
| `RouteHeader::local_stdio` | `crates/tty7-core/src/daemon/router.rs:128` | 路由不经 SSH、不需要端口 |
| `RouteTarget::origin_key` | `crates/tty7-core/src/daemon/router.rs:160` | 主机身份键：`wsl:<distro>`、`local-stdio:<program>`，容器需要 `docker:<name>` |
| `tty7-server --stdio` | `crates/tty7-server/src/main.rs:9` | 容器内唯一需要的入口，不需要 daemon、不需要 root |
| `RemoteOps`（`WslRemoteOps` 实现） | `crates/tty7-core/src/daemon/install/wsl.rs:369` | 远端装机的完整实现：探测、上传、chmod、rename、拉起 |
| `asset_for_uname` | `crates/tty7-core/src/daemon/install/asset.rs:117` | `uname -sm` → 资产名，已按远端架构选 |
| musl 发布矩阵 | `.github/workflows/ci.yml:243` | `x86_64-unknown-linux-musl` + `aarch64-unknown-linux-musl` 静态二进制 |
| 连接器发现 + 列表 | `src/ui/remote_connect.rs:194,218` | `sweep_wsl` / `wsl_choices` 是容器发现的模板 |

**推论**：PRD 的 FR 全部落在"主机类型 + 发现 + 装机 + 生命周期"四件事上，协议层、pane、文件、git 都不动。

## 2. 机制选择

### 采用：stdio 服务端 over `docker exec`

- 客户端在宿主启动 `docker exec -i <container> <path>/tty7-server --stdio`，把容器内的进程当成远端主机。
- 二进制从宿主送进容器（`docker cp`，或经 exec 的 stdin 写入临时名再 rename）。
- 容器不参与网络：没有监听端口，没有入站连接，不需要出网。

### 已被否决的备选

| 方案 | 否决理由 |
| --- | --- |
| 在容器里起 sshd | 正是本功能要消除的前置条件；还要处理端口、密钥、镜像修改 |
| 把容器端口映射到宿主再走 SSH | 需要容器出网/宿主端口管理，且要求镜像里有 sshd |
| 走 docker socket 直接 attach | 需要宿主 docker socket 权限（等同 root），信任模型过重；且需要自己实现 exec 复用 |
| 停在 `docker exec -it` 的裸终端 | 没有文件、git、持久化的能力，等于不做 |

### 与同类产品的对照

Zed 与 DevPod 采用与本方案相同的形状（stdio 服务端 over `docker exec`，宿主侧推送二进制，无 sshd、无容器网络）；VS Code Dev Containers 同样是"本地下载、`docker cp` 进容器"。这构成外部验证。

## 3. 外部证据（研究摘要）

来源以官方文档、项目仓库、issue tracker 为主。

### 3.1 同类实现

- **VS Code Dev Containers**：传输走 "Docker's configured communication channel (via `docker exec`)"；服务端二进制"always downloaded locally and transferred into the container"，安装日志可见 `docker cp <ctr>:/etc/passwd -` 与 `docker exec ... mkdir`（[FAQ](https://code.visualstudio.com/docs/remote/faq)，[vscode-remote-release#1692](https://github.com/microsoft/vscode-remote-release/issues/1692)）。
- **DevPod**：注入自己的 agent，然后通过 `docker exec` 运行 `devpod helper ssh-server --stdio`——在进程内跑 SSH 服务，不装 sshd 包（[pkg/tunnel/container.go](https://github.com/loft-sh/devpod/blob/5a0efcbf/pkg/tunnel/container.go)）；agent 按架构在本地取（[inject.go](https://github.com/loft-sh/devpod/blob/5a0efcbf/pkg/agent/inject.go)）。
- **Zed**（v0.218，2026-01）：stdio server over `docker exec`，`upload_binary_over_docker_exec`（[docker.rs](https://github.com/zed-industries/zed/blob/main/crates/remote/src/transport/docker.rs)），musl 静态服务端（[blog](https://zed.dev/blog/dev-containers)）。
- **JetBrains Gateway**：把 IDE 后端装进容器，走 RD 协议；远端 Docker 场景要求**宿主**有 OpenSSH，容器内不需要（[blog, 2024-07](https://blog.jetbrains.com/idea/2024/07/using-dev-containers-in-jetbrains-ides-part-1/)）。本机 Docker 的通道实现未见公开文档，未验证。
- **Tabby / MobaXterm**：不注入服务端，profile 直接跑 `docker exec -it`，因此没有 workspace 能力；MobaXterm 用户报告交互破损（[tabby-docker](https://github.com/Eugeny/tabby-docker)，[SO](https://stackoverflow.com/questions/55403730/)）。
- **Cursor**：沿用 VS Code 模型，Alpine 自 v0.50.5（2025-05）才支持（[forum](https://forum.cursor.com/t/failed-to-download-vs-code-server-on-alpine-linux/45914)）。

### 3.2 失败模式与硬约束

- **无信号代理**：Docker API 没有 kill exec 进程的手段；Ctrl-C 只杀 `docker exec` 客户端（[docker/cli#1841](https://github.com/docker/cli/pull/1841)、[docker-py#3155](https://github.com/docker/docker-py/issues/3155)）。→ 支撑 FR-11，并使"EOF 即关闭"成为唯一的回收语义。
- **无 TTY、无 SIGWINCH**：无 tty 时初始窗口为 `0 0`，也没有窗口变化信号（[moby#33794](https://github.com/moby/moby/issues/33794)）。→ tty7 的 pane 尺寸走自己的协议、由服务端分配 PTY，不受此限；但控制通道必须按无 TTY 设计。
- **exec 成本**：单次 `docker exec` 约 0.4 秒（100 次 44 秒，原生 0.08 秒，[docker/for-linux#466](https://github.com/docker/for-linux/issues/466)）。→ FR-12；装机探测必须合并成少量调用。
- **重建身份**：服务端与状态都在容器的可写层，重建即新 ID、需重新投放（VS Code 同现象）。→ FR-9 以名字为身份，并在重建后静默重装（FR-5）。
- **非 root 且 `$HOME` 不可写**：[vscode-remote-release#1692](https://github.com/microsoft/vscode-remote-release/issues/1692)；挂载卷属主为 root：[#9931](https://github.com/microsoft/vscode-remote-release/issues/9931)、[#5296](https://github.com/microsoft/vscode-remote-release/issues/5296)、rootless [#4646](https://github.com/microsoft/vscode-remote-release/issues/4646)。→ FR-7。
- **架构/libc**：错架构直接 `exec format error`（[buildkit 多平台文档](https://github.com/moby/buildkit/blob/master/docs/multi-platform.md)）；glibc ≥2.28 是 VS Code 自 1.99 起的地板，Alpine 需要 musl 构建（[linux 文档](https://code.visualstudio.com/docs/remote/linux)）。→ 本仓库已有 musl 双架构资产，风险解除。
- **无 shell**：distroless/scratch 跑不了 `/bin/sh`，exec 必须直接传 argv（[distroless#891](https://github.com/GoogleContainerTools/distroless/issues/891)）。→ §5 Non-Goal；同时 `RemoteOps` 全部是 `sh -c` 脚本（`install/mod.rs:1611`），这条边界与实现一致。
- **`docker cp` 的规模与模式**：文档标注大文件性能下降，远端 TCP 下有截断问题（[docker cp 文档](https://docs.docker.com/reference/cli/docker/container/cp/)）。→ 12MB 量级二进制在安全区间。
- **`DOCKER_HOST` 指向远端**：每条命令重新拉 SSH，约 3–4 秒（[forum](https://forums.docker.com/t/setting-docker-host-to-ssh-results-in-slow-workflow-can-ssh-connection-be-reused/98754)）；部分 Linux 客户端 `-i` 报 `/dev/stdin EAGAIN`（[cli#1707](https://github.com/docker/cli/issues/1707)）。→ FR-14 明确拒绝。

### 3.3 用户诉求

- Zed [#11473](https://github.com/zed-industries/zed/issues/11473)："you need to run an ssh server in your container… it's more setup than needed"；"support connecting zed to docker containers the same way we can with ssh"。
- VS Code [#1186](https://github.com/microsoft/vscode-dev-containers/issues/1186)："SSH is not in any of these images by default and VS Code does not use it"。
- [#1725](https://github.com/microsoft/vscode-remote-release/issues/1725)："I didn't want to jump through extra configuration hoops"。

## 4. 开放的技术问题（对应 PRD §8）

1. **安装目录回退优先级**：需要在一组真实镜像（root / 非 root / 只读 rootfs / `noexec`）上实测，才能定下 FR-7 的验收。候选：`$HOME/.local/share/tty7/bin` → `/tmp/tty7-<uid>/bin` → `/dev/shm/...`，最后一项是否可执行需要在目标内核上验证。
2. **强杀后的自愈时间上界**：依赖 `--stdio` 会话在 stdin EOF 后的退出路径，需要在 `docker exec` 下实测，才能给 FR-11 一个数字。
3. **单条长驻 exec 的吞吐**：控制通道 + 全部 pane 共用一条流，需要验证在高输出场景（`cat` 大文件、构建日志）下的上限，作为架构阶段的门槛。
4. **`docker ps` 的调用节流**：连接器打开即探测，需要避免常驻轮询（SM-C3）。
5. **Windows 上 docker CLI 的发现**：PATH、Docker Desktop 安装路径、WSL 内 docker 三种来源；以及与同名 WSL 发行版 `docker-desktop` 的区分（FR-4）。
