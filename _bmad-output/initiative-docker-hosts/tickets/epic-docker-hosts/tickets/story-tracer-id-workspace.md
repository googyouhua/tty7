---
id: 1
type: story
title: "tracer: 按容器名或 ID 接入、确认装机、进 workspace、断开后进程退出"
parent: epic-docker-hosts
covers: [R1, R3, R9]
after: []
hitl: true
risk: high
---

# tracer: 按容器名或 ID 接入、确认装机、进 workspace、断开后进程退出

## Description

打通一条最薄的完整路径：手动输入容器名或 ID → 路由进容器 → 首次装机弹确认（拒绝时不写入）→ 进入 workspace → 关闭后容器内进程退出。列表、架构选择、目录回退这一轮都先走单一路径。需要一台有 docker 的机器，以及一个没有 sshd 的运行中容器（例如 `docker compose up -d` 起的开发容器）。

## Requirements

Verify: 在有 docker 的机器上按容器名接入一个无 sshd 的容器：出现装机确认，拒绝时容器内无文件写入；允许后能在 pane 里执行命令；关闭 workspace 后 `docker exec <容器> pgrep -f tty7-server` 无输出。同一个容器改用容器 ID 再接入一次，落到同一台主机、同一个 workspace。

## References

- parent — _bmad-output/initiative-docker-hosts/tickets/epic-docker-hosts/epic-docker-hosts.md

## Notes

- 高风险，验收之外再加一道检查：换一台**从没装过** `tty7-server` 的容器重跑一次，确认不是本机缓存掩盖了装机问题。
- Risk: high — 这是整条链路的骨架，接不通的话后面 12 条全部要重排；接错方向（例如把隧道做成一次性 exec）会让 perf 与回收两条要求同时报废。
- hitl 步骤：准备一台有 docker 的机器和一个没有 sshd 的运行中容器（`docker compose up -d` 或 `docker run -d`），这一步由人来跑。
