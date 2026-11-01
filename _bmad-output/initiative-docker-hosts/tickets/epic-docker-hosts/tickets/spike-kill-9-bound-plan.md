---
title: "强杀客户端后容器内进程的退出上界 (kill-9 bound)"
ticket: 2
status: blocked
blocked_at: "2026-11-01"
blocked_reason: "本环境没有任何容器运行时：docker/podman/nerdctl/ctr/buildah 均不存在，无 /var/run/docker.sock，DOCKER_HOST 未设置。spike 1.2 必须真实起容器并 exec，无法在此环境执行或验证。"
---

## Auto Run Result

Status: blocked
Blocking condition: 本环境没有任何容器运行时。

### 已核实的环境事实（2026-11-01）

| 检查项 | 结果 |
| --- | --- |
| `docker` / `podman` / `nerdctl` / `ctr` / `buildah` | 全部不存在（`command -v` 无输出） |
| `/var/run/docker.sock` | 不存在 |
| `DOCKER_HOST` | 未设置 |
| `~/.docker/contexts` | 不存在 |

运行环境本身是一个容器：无 dockerd、无嵌套运行时、未挂载宿主 socket。

### 为什么不能绕过

1.2 的验收要求真实测量「容器内进程在通道被关闭 / 客户端被 SIGKILL 后多久退出」。这需要：起一个容器、把 `tty7-server` 放进去、用 `docker exec -i` 挂 stdio、在客户端侧制造三种终止、并对容器内进程轮询 `pgrep`。

没有容器运行时，上面每一步都无法发生。写成推测值等于伪造测量结果，而这条 spike 的全部价值就是那个数字。

### 同一条件还挡住谁

- **1.3 安装目录回退顺序**：同样要起四类容器（root / 非 root / 只读 rootfs / `noexec`）。
- **1.1 tracer**：hitl 步骤明确要求一台有 docker 的机器和一个无 sshd 的容器。
- **1.6–1.13**：全部经由 1.1 或 1.2 的前置。

也就是说，这条 epic 的验收工作整体需要一个有容器运行时的环境；只有代码落地部分（例如 1.4 的列表与拒绝远端、1.11 的部分错误分类）能在纯代码层面推进。

### 解除条件（任一）

1. 在宿主机（有 Docker Desktop 或 dockerd）上运行本 workflow。
2. 把宿主 socket 挂进来：`docker run -v /var/run/docker.sock:/var/run/docker.sock ...`，并安装 `docker` CLI。
3. 起一个 DinD 服务并把 `DOCKER_HOST` 指过去。
4. 提供一个远程 docker context（注意：epic 的 R11 明确不支持远端守护进程，那会与产品边界冲突——所以这条只适合做实验，不适合当验收环境）。

解除后执行：`tickets.py mark epic-docker-hosts 2 ready-for-dev`（或直接重新发起 build），本 ticket 无需改写。
