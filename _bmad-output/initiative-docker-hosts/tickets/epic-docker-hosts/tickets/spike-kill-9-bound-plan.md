---
title: "强杀客户端后容器内进程的退出上界 (kill-9 bound)"
ticket: 2
status: built
baseline_revision: 0f259733d91a9a10258de951eca723299c584e50
blocked_at: ""
blocked_reason: ""
---

## Intent

回答一个问题：客户端没有干净关闭就消失时，容器内的 `tty7-server` 多久退出，还是根本不退出？答案要给 entry 10（断开回收）和 epic 的 R9 提供可测量的阈值。

环境：192.168.1.25（Docker 29.1.3, amd64），测试容器 `node:alpine`，二进制为发布版 `tty7-server-c12p6`（x86-64 静态链接，与容器 libc 无关）。

## Implementation Notes

### 先纠正一个测量错误

第一版探针把「容器内存在任何 `tty7-server` 进程」当作连接仍活着，结果三种情形全部 30 秒不退出。真实原因是进程结构：

| 进程 | 启动方 | 生命周期 |
| --- | --- | --- |
| `tty7-server --stdio` | `docker exec` 拉起，ppid 0 | 一次连接的桥，应当随连接结束而退出 |
| `tty7-server --daemon --config-dir …` | 桥在发现没有本地控制服务时自己起的 | 常驻，被后续连接复用 |

第二次连接只新增了桥，没有新增 daemon——daemon 数量始终为 1。所以「不得残留」的对象是**桥**，不是 daemon。

第二版探针按 `comm == tty7-server` 且命令行含 `--stdio` 判定桥，把 daemon 排除在外。

### 测量方法

每条用例：`mkfifo` 拿到一个不说话的 stdin，`docker exec -i <c> /tmp/tty7-server --stdio < fifo` 起桥，保持写端打开 1.5 秒，然后分别制造三种终止；随后每 50 ms 在容器内探测桥是否还在，上限 30 秒。每例重复 5 次。

## Results

| 情形 | 5 次测量 (ms) | 最大值 |
| --- | --- | --- |
| 正常关闭 stdin | 53, 56, 53, 54, 52 | **56 ms** |
| 杀 `docker exec` 客户端 (SIGTERM) | 55, 54, 52, 55, 53 | **55 ms** |
| 杀 `docker exec` 客户端 (SIGKILL) | 52, 53, 53, 54, 52 | **54 ms** |

- 三种情形**全部退出**，没有出现「不退出」。
- 15 次连接、3 种终止方式之后，daemon 数量仍为 1（开始时 0）。
- 结论：桥依赖通道 EOF 语义退出，且 `docker exec` 在客户端被 SIGKILL 后确实把 EOF 传到了容器内。

## Verification

- 每条用例的「桥是否仍在」由容器内 `/proc` 扫描判定，不依赖人工观察。
- 重复 5 次取最大值，三种情形一致在 52–56 ms。
- 反证：改用「存在任何 tty7-server」判定时，结果变成全部不退出——说明探针能区分两种进程，不是恒真。
- daemon 计数在 15 次连接前后均为 1，证明连接不累积进程。

## Consequences for the tree

R9 与 entry 10 需要按此改写（由 Sofia 落地，build 不改 ticket）：

1. 阈值：桥在 **1 秒**内退出即合格（实测上限 56 ms，留一个数量级余量给繁忙主机）。
2. 措辞：R9 的「不残留由该次连接启动的进程」不准确——第一次连接会起一个 daemon，它按设计常驻。应改为「不残留该次连接的桥；daemon 可以有且只能有一个，并被后续连接复用」。
3. 验收：N 次连接后 daemon 数量恒为 1，是比「零进程」更真实、也更容易测的检查。
4. 容器被删除/重建时 daemon 随之消失，不需要额外清理路径——这一点由 daemon 住在容器可写层直接推出，无需再测。

## Notes

- 本次测量跑在远程 docker 主机（192.168.1.25）上，被测的 `docker exec` 通道在**该主机本地**（CLI 与 daemon 同机），与产品在宿主机上使用时的形态一致；ssh 只是我们发起命令的方式，不穿过被测通道。
- 产品明确不支持远端 `DOCKER_HOST`（R11），所以这台机器只作实验环境，不作产品验收环境。
