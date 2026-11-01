---
title: "连接器列出本机运行中的容器，并拒绝远端 docker"
ticket: 4
status: done
baseline_revision: 8c1b9124
---

## Auto Run Result

Status: done

这一条的范围随实现分散落了几处，记录在此，免得以后翻不出来。

### 交付

| 需求 | 落在哪 | 提交 |
| --- | --- | --- |
| 列表只列运行中的容器，随起停增减 | `remote_connect::sweep_docker`，打开切换器时探一次，30 秒 TTL | `a34083e0`、`19ee9127` |
| 按容器名或镜像名命中 | 两条路径都已覆盖：切换器的 `filter_hosts` 同时打分 label 与 detail；命令面板 Hosts 标签的 `item_score` 在副标题上做整词匹配 | 本次补测 |
| 按名字或 ID 手动接入 | `docker:<名字>` 一行「连接到容器」，走 `CommandKind::ConnectTarget` | `56589583` |
| 拒绝远端 docker 守护进程 | `control_route` 在路由前拒绝，列表也不列远端容器；`DOCKER_HOST` 优先于 context，context 查询缓存 30 秒 | `8c1b9124`、`5ae85e4f` |
| 刷新不得变成轮询 | 只在打开切换器时探；TTL 30 秒；`remote_endpoint` 的 context 查询同样有 30 秒缓存 | 同上 |

### 验证

- `a_container_is_found_by_name_or_by_image`：给两个容器（`api-dev`/`node:22-alpine`、`db`/`postgres:16`），按名字、按镜像都能命中唯一一行，按不存在的镜像返回空。
- `a_container_is_found_by_its_image_in_the_hosts_tab`：命令面板那条路，副标题 `Docker · node:22-alpine` 能被 `node`、`node:22` 命中，`postgres` 不命中。
- `docker_host_decides_before_the_context_is_asked`：`DOCKER_HOST` 为 ssh/tcp 时判为远端，`unix://` 判为本机，空白落到 context。
- `cargo test --bin tty7-app`：2750 通过。

### 未做（有意）

- **与同名 WSL 发行版的区分**：v1 不做 Windows，已在 epic 的 Boundaries 记 `Decision`。这一条随 Windows 一起进 v2。
- **容器条的右键菜单**（新建工作区/编辑主机）：`host_form_label` 对容器返回 `None`，所以不出现「编辑主机」——容器没有可编辑的 profile。新建工作区的入口在切换器表单里。

### 备注

列表能力在 tracer（1.1）里就已经落地，因为 tracer 没有入口就无法演示。那部分记在 1.1 的 plan 里，这里不重复计算工作量。
