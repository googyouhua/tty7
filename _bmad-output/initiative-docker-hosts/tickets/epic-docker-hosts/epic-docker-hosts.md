---
type: epic
title: "容器就是一台主机"
parent: initiative-docker-hosts
covers: ["R1", "R2", "R3", "R4", "R5", "R6", "R7", "R8", "R9", "R10", "R11"]
sources: ["_bmad-output/initiative-docker-hosts/initiative-docker-hosts.md"]
companions: ["_bmad-output/initiative-docker-hosts/prd-docker-containers/addendum.md"]
after: []
refined: true
risk: medium
---

# 容器就是一台主机

## Description

把本机的 Docker 容器接成 tty7 的第三种主机类型，与 SSH 远端、WSL 发行版同级。容器里没有 sshd、不要求出网，宿主机上有 `docker` CLI 即可。交付的是完整 workspace，不是裸终端。

## Outcome

一个用 compose 起开发容器的开发者，在发布构建里选中该容器就能完整干活；容器重建后零提问地回到原布局，断开后容器里不留进程。

## Requirements

1. **发现与接入** (parent R1)：本机运行中的容器出现在连接器里，可与 SSH 主机、WSL 发行版并列搜索，按容器名或镜像名命中；按名字或 ID 手动指定也能接入；容器主机的连接标识不与同名 WSL 发行版互相命中。
2. **完整 workspace** (parent R2)：接入后文件树、git 状态、diff、搜索都作用在容器内文件系统上；tab、分组、pane 布局按既有规则持久化并在重开后恢复。
3. **首次装机确认** (parent R3)：首次向某个容器写入服务端程序前必须得到用户确认；确认过一次后，同一容器（含重建后）的后续接入不再询问。
4. **按容器架构选资产** (parent R4)：装机使用容器架构对应的静态 musl 资产，宿主与容器架构不同也能成功；架构无法识别或没有对应资产时连接中止，并说明是哪一侧不受支持。
5. **安装位置回退** (parent R5)：容器内 `$HOME`、rootfs 不可写或挂载为 `noexec` 时，装机按优先级回退到可写位置；全部不可写时列出尝试过的路径。
6. **接入开销有界** (parent R6)：典型镜像上首次装机从确认到可进入不超过 15 秒；已装好且版本匹配时接入不产生额外等待；交互开销不随打开的 pane 数量线性增长。
7. **身份是容器名** (parent R7)：`docker compose down && up` 之后连接器里仍只有一行；该 workspace 的 tab 与布局保留，pane 显示为容器已重建且已停止，不显示为空白。
8. **停止与删除后的记录** (parent R8)：容器停止或 docker 未运行时条目保留并标记不可用；容器恢复后条目自动回到可用，且无需重新确认装机；容器被彻底删除时条目保留并标记为找不到。
9. **断开不留下孤儿** (parent R9)：关闭 workspace、关闭 tty7 或断开后，该次连接的桥进程在 1 秒内退出；客户端被强杀（SIGKILL）同样在 1 秒内退出。容器内允许存在且只存在一个常驻 daemon，并被后续连接复用，不随连接次数累积。
10. **失败分类且可操作** (parent R10)：容器未运行、docker 未运行、容器内无可用 `/bin/sh`、容器架构无对应资产、无可写安装目录、装机被用户拒绝、装机后服务端无法启动——七类失败各自给出可读原因与用户可执行的下一步，且都不表现为无限等待。
11. **拒绝远端 docker 守护进程** (parent R11)：`DOCKER_HOST` / docker context 指向远端守护进程时明确拒绝并说明 v1 只支持本机容器；该配置下本机容器仍然可用。

## Constraints

- **C1**：容器内不得安装 sshd，也不得要求容器具备出网能力。
- **C2**：只支持有 POSIX shell 的容器；`distroless` / `scratch` 给可读错误，不降级。
- **C3**：沿用 `docker exec` 默认用户（镜像 `USER`），v1 不做降权或用户选择。
- **C4**：复用既有本地 exec 通道与远端装机抽象，不新增线上协议。
- **C5**：不起停容器、不构建镜像、不解析 compose 项目。
- **C6**：SSH 与 WSL 主机类型的既有行为不变。
- **C7**：容器列表刷新不得变成常驻高频 `docker ps` 轮询。

## Done when

- 一个真实开发容器（compose 起的）在发布构建里从连接器接入，文件、git、diff、多 tab 全部在容器内生效。
- 容器重建后重连零提问，连接器条目恒为一，tab 与布局保留，pane 明示已停止。
- 100 次「接入 → 关闭」循环后，容器内桥进程数为 0，常驻 daemon 数恒为 1。
- 七类失败各自给出可读原因与可执行下一步，无无限等待。
- 以上随发布构建可用，不是只在开发构建里成立。

## Boundaries

一个容器就是一台主机。不涉及编排、不起停容器、不构建镜像、不聚合 compose 项目、不支持 Podman / nerdctl（prd §5、§6.2）。

**v1 不含 Windows。** Windows 上的 `docker` CLI 发现与同名 `docker-desktop` WSL 发行版的区分后置 v2；R1 中「不与同名 WSL 发行版互相命中」这一句随之延后，v1 的验收不含它。

- Touch point: `RemoteTarget` / `RouteHeader` / `RemoteOps` — 本地 exec 通道与远端装机抽象；本 epic 拥有
- Touch point: `RouteTarget::origin_key` — 容器主机身份键 `docker:<name>`
- Touch point: 连接器发现逻辑 — 容器列表来源与容器/发行版区分
- Touch point: 装机资产选择 — 按容器而非宿主架构选 musl 资产

## References

- parent — `_bmad-output/initiative-docker-hosts/initiative-docker-hosts.md`, section Requirements
- prd — `_bmad-output/initiative-docker-hosts/prd-docker-containers/prd-docker-containers.md`, §4 Features、§5 Non-Goals、§7 Success Metrics
- companion — `_bmad-output/initiative-docker-hosts/prd-docker-containers/addendum.md`, 接缝清单、被否决的路线、外部证据

## Notes

- Decision: 2026-11-01 — 本 epic 一次交付 R1-R11，不切第二条第 epic；唯一站得住的切点是 R7/R8，被拒。
- Decision: 2026-11-01 — R11 留在本 epic 内交付，不后置。
- Decision: 2026-11-01 — 切 story 之前先钉死 Q2（安装目录回退顺序）与 Q3（强杀后退出上界）：两者都是 R5/R9 的验收阈值，只能由代码回答，各为一个 spike。
- Decision: 2026-11-01 — tracer：手动按容器名或 ID 接入 → 首次确认装机 → 进入 workspace → 断开回收；发现与列表面板另设 entry 4，不在 tracer 里。
- Decision: 2026-11-01 — v1 只做 macOS / Linux，Windows 后置 v2；代价是 R1 的「同名 WSL 发行版不互相命中」不在 v1 验收里。理由：Windows 的 docker 生态（Docker Desktop + WSL 后端）需要单独设计，混进来会把 tracer 拖长。
