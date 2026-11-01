---
type: epic
title: "容器就是一台主机"
parent: initiative-docker-hosts
covers: ["R1", "R2", "R3", "R4", "R5", "R6", "R7", "R8", "R9", "R10", "R11"]
sources: ["_bmad-output/initiative-docker-hosts/initiative-docker-hosts.md"]
companions: ["_bmad-output/initiative-docker-hosts/prd-docker-containers/addendum.md"]
after: []
refined: false
risk: medium
---

# 容器就是一台主机

## Description

把本机的 Docker 容器接成 tty7 的第三种主机类型，与 SSH 远端、WSL 发行版同级。容器里没有 sshd、不要求出网，宿主机上有 `docker` CLI 即可。交付的是完整 workspace，不是裸终端。

## Outcome

一个用 compose 起开发容器的开发者，在发布构建里选中该容器就能完整干活；容器重建后零提问地回到原布局，断开后容器里不留进程。

## Done when

- 一个真实开发容器（compose 起的）在发布构建里从连接器接入，文件、git、diff、多 tab 全部在容器内生效。
- 容器重建后重连零提问，连接器条目恒为一，tab 与布局保留，pane 明示已停止。
- 100 次「接入 → 关闭」循环后，容器内残留服务端进程数为 0。
- 七类失败各自给出可读原因与可执行下一步，无无限等待。
- 以上随发布构建可用，不是只在开发构建里成立。

## Boundaries

一个容器就是一台主机。不涉及编排、不起停容器、不构建镜像、不聚合 compose 项目、不支持 Podman / nerdctl（prd §5、§6.2）。

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
- Decision: 2026-11-01 — tracer：发现容器 → 首次确认装机 → 进入 workspace → 断开回收。
