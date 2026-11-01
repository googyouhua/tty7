---
type: initiative
title: "Docker 容器作为 tty7 主机"
parent: none
sources:
  - _bmad-output/initiative-docker-hosts/prd-docker-containers/prd-docker-containers.md
companions:
  - _bmad-output/initiative-docker-hosts/prd-docker-containers/addendum.md
after: []
risk: medium
---

# Docker 容器作为 tty7 主机

## Description

让 tty7 把本机的 Docker 容器当作一台完整的主机接进来：文件树、git 状态、diff、多 tab、持久 pane 都在容器内生效，和 SSH 远端、WSL 发行版是同一档体验。前提是容器里没有 sshd、不保证有出网，宿主机上 `docker` CLI 可用。服务对象是有长期开发容器（devcontainer / compose）的开发者，不是一次性调试容器。需求来源是 PRD，技术接缝与备选方案在 addendum。

## Outcome

宿主机上的开发者不经容器内 sshd、不要求容器出网，就能把运行中的容器当作完整主机接入并持续使用；信号是容器重建后重连零提问、条目数恒为一，且 100 次接入/断开循环后容器内无残留进程。

## Requirements

1. **发现与接入** (prd FR-1, FR-2, FR-4)：本机运行中的容器出现在连接器里，可与 SSH 主机、WSL 发行版并列搜索，按容器名或镜像名命中；按名字或 ID 手动指定也能接入；容器主机的连接标识不与同名 WSL 发行版互相命中。
2. **完整 workspace** (prd FR-3)：接入后文件树、git 状态、diff、搜索都作用在容器内文件系统上；tab、分组、pane 布局按既有规则持久化并在重开后恢复。
3. **首次装机确认** (prd FR-5)：首次向某个容器写入服务端程序前必须得到用户确认；确认过一次后，同一容器（含重建后）的后续接入不再询问。
4. **按容器架构选资产** (prd FR-6)：装机使用容器架构对应的静态 musl 资产，宿主与容器架构不同也能成功；架构无法识别或没有对应资产时连接中止，并说明是哪一侧不受支持。
5. **安装位置回退** (prd FR-7)：容器内 `$HOME`、rootfs 不可写或挂载为 `noexec` 时，装机按优先级回退到可写位置；全部不可写时列出尝试过的路径。
6. **接入开销有界** (prd FR-8, FR-12)：典型镜像上首次装机从确认到可进入不超过 15 秒；已装好且版本匹配时接入不产生额外等待；交互开销不随打开的 pane 数量线性增长。
7. **身份是容器名** (prd FR-9)：`docker compose down && up` 之后连接器里仍只有一行；该 workspace 的 tab 与布局保留，pane 显示为容器已重建且已停止，不显示为空白。
8. **停止与删除后的记录** (prd FR-10)：容器停止或 docker 未运行时条目保留并标记不可用；容器恢复后条目自动回到可用，且无需重新确认装机；容器被彻底删除时条目保留并标记为找不到。
9. **断开不留下孤儿** (prd FR-11)：关闭 workspace、关闭 tty7 或断开后，容器内不残留由该次连接启动的进程；客户端被强杀后，残留进程在有限时间内自行退出或被下次接入清理。
10. **失败分类且可操作** (prd FR-13)：容器未运行、docker 未运行、容器内无可用 `/bin/sh`、容器架构无对应资产、无可写安装目录、装机被用户拒绝、装机后服务端无法启动——七类失败各自给出可读原因与用户可执行的下一步，且都不表现为无限等待。
11. **拒绝远端 docker 守护进程** (prd FR-14)：`DOCKER_HOST` / docker context 指向远端守护进程时明确拒绝并说明 v1 只支持本机容器；该配置下本机容器仍然可用。

## Constraints

- **C1** (prd §5)：容器内不得安装 sshd，也不得要求容器具备出网能力。二进制从宿主送进去。
- **C2** (prd §5)：只支持有 POSIX shell 的容器；`distroless` / `scratch` 不支持，给可读错误而不是降级或静默失败。
- **C3** (prd §5, §6.2)：容器内执行沿用 `docker exec` 的默认用户（镜像 `USER`）；v1 不做降权，也不提供用户选择。
- **C4** (addendum §1)：复用既有的本地 exec 通道与远端装机抽象，不新增线上协议；容器主机是这条通道的第三个实例。
- **C5** (prd §5)：tty7 不起停容器、不构建镜像、不解析 compose 项目。
- **C6** (prd §5)：SSH 与 WSL 主机类型的既有行为不变。
- **C7** (prd §7 SM-C3)：容器列表的刷新不得变成常驻高频 `docker ps` 轮询。

## Done when

- 在连接器里选中一个运行中的容器，不需容器内 sshd、不需容器出网，即可进入完整 workspace 并在 pane 中执行命令。
- 首次装机需确认；此后同一容器（含重建后）接入零提问。
- 连续 10 次 `docker compose down && up` 后，连接器条目数恒为 1，tab 与布局保留。
- 100 次「接入 → 关闭」循环后，容器内残留服务端进程数为 0。
- 上列七类失败各自给出可读原因与下一步，且无无限等待。

## Boundaries

能力边界：一个容器就是一台主机。不涉及容器编排、不替用户起停容器、不做镜像构建、不聚合 compose 项目、不支持 Podman / nerdctl 等其它运行时（prd §5、§6.2）。非目标见 PRD §5。

- Touch point: `RemoteTarget` / `RouteHeader` / `RemoteOps` — 本地 exec 通道与远端装机抽象；owner: epic-docker-hosts
- Touch point: `RouteTarget::origin_key` — 容器主机的身份键 `docker:<name>`，R7 与 R1 都挂在它上面；owner: epic-docker-hosts
- Touch point: 连接器发现逻辑（`sweep_wsl` 同族）— 容器列表来源与容器/发行版区分；owner: epic-docker-hosts
- Touch point: 装机资产选择（`asset_for_uname`）— 按容器而非宿主架构选 musl 资产；owner: epic-docker-hosts
- 不改动：`tty7-server --stdio` 与 musl 发布矩阵（addendum §1），容器场景直接复用现有产物。

Tracer path：发现容器 → 首次确认装机 → 进入完整 workspace → 断开回收，这条路径的第一个 epic 必须整条走通。

## References

- prd — `_bmad-output/initiative-docker-hosts/prd-docker-containers/prd-docker-containers.md`，§2 用户与旅程、§4 Features、§5 Non-Goals、§7 Success Metrics
- companion — `_bmad-output/initiative-docker-hosts/prd-docker-containers/addendum.md`，接缝清单、被否决的路线、外部证据
- constraint — prd §6.2 Out of Scope for MVP

## Notes

- Assumption: A1 — 同名不同镜像的容器不额外提示，按同一主机处理；待可用性验证。
- Assumption: A2 — pane 默认工作目录是容器内该用户的 `HOME`，不可用时回退 `/`。
- Assumption: A3 — 容器被彻底删除后条目保留，由用户决定清理，v1 不提供删除入口。
- Assumption: A4 — 现有 musl 双架构资产已覆盖容器场景，不为容器新增构建矩阵。
- Open question: Q2 — 安装目录回退优先级的具体顺序，需在一组真实镜像上实测；blocker 是 R5。
- Open question: Q3 — 客户端被强杀后容器内进程自行退出的时间上界，决定 R9 的可测阈值；blocker 是 R9。
- Open question: Q4 — 同名不同镜像是否真的无需提示；blocker 是 R7（与 A1 同一条判断）。
- Open question: Q5 — 是否需要「删除容器主机记录」的显式入口；blocker 是 R8。
- Open question: Q6 — 已后置 v2：Windows 上 `docker` CLI 的发现方式；见 epic Boundaries。
- Decision: 2026-11-01 — 场景为「桌面端在宿主机、接入容器」，排除「tty7 跑在容器里连出去」与「server 在容器里被外部连入」两种拓扑。
- Decision: 2026-11-01 — 要完整 workspace，不接受只做终端；容器定位为长期开发环境。
- Decision: 2026-11-01 — 容器身份是容器名；重建即复用 workspace、保留 tab 与布局、pane 明示停止。
- Decision: 2026-11-01 — 首次确认后静默装机，容器重建不再询问。
- Decision: 2026-11-01 — 从 `initiative-terminal-blocks` 独立出来，Docker 主机能力与命令块无共同边界。
- Decision: 2026-11-01 — 只切一条 epic。理由：R1-R11 落在同一批文件（`session.rs` 的 `RemoteTarget`、`router.rs` 的 `RouteTarget`/`origin_key`、`remote_connect.rs` 的发现、`install/` 的装机），没有部署边界，且只有一起交付才满足本 initiative 的 Outcome；把 R7/R8 单切会让第一条 epic 交付一个「每天重建就丢布局」的容器主机，正是 PRD 要消除的体验。
- Decision: 2026-11-01 — 不设独立的平台基线 epic。理由：没有独立部署边界；基线（`docker` CLI 发现、容器枚举管道）由这条 epic 的头两条 story 承担。
- Decision: 2026-11-01 — tracer 路径：发现容器 → 首次确认装机 → 进入 workspace → 断开回收；整条走通前不切后续 story。
- Decision: 2026-11-01 — v1 只做 macOS / Linux。Q1（同名 `docker-desktop` 与容器并存）与 Q6（Windows 上 `docker` CLI 发现）一并后置 v2，不再是 R1 的 v1 阻塞项；判据是 epic 的 Boundaries。
