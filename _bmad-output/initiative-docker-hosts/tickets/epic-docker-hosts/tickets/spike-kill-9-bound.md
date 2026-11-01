---
id: 2
type: spike
title: "强杀客户端后容器内进程的退出上界 (kill-9 bound)"
parent: epic-docker-hosts
covers: [R9]
after: []
refined: true
hitl: false
risk: high
---

# 强杀客户端后容器内进程的退出上界 (kill-9 bound)

## Description

问题：客户端没有干净关闭就消失时，容器内的 `tty7-server` 多久退出，还是根本不退出？

为什么是开的：Docker 在协议层杀不掉 exec 进程（addendum §3.2「无信号代理」），回收只能靠通道的 EOF 语义。仓库里没有实测数字，epic 的 R9 因此只能写成「有限时间」，等于没有阈值。

谁在等：entry 10（断开回收：容器内不留孤儿）的验收，以及 epic 中 R9 那一条的阈值。

## Approach

1. 用 `docker run` 起一个最小容器（debian 系），把当前发布版的 `tty7-server` 放进去。
2. 用 `docker exec -i <c> tty7-server --stdio` 挂上一条 stdio 通道，另一端用脚本驱动握手。
3. 三种情形各跑一遍：正常关闭 stdin；对 `docker` CLI 发 SIGTERM；对 `docker` CLI 发 SIGKILL。
4. 每种情形每 200ms 用 `docker exec <c> pgrep -f tty7-server` 采样，记录进程消失的时间；观察上限 120 秒。
5. 每种情形重复 5 次，取最大值，并记录是否出现「观察上限内不退出」。

时间盒：半天。够好的标准：三种情形都有数字或明确的「不退出」，并且没有靠人工看屏得出结论。
签收：答案由负责 entry 10 的人确认能直接写进验收；决策者（本 initiative 的 owner）点头后本 ticket 关闭。

## Acceptance Criteria

1. **三种情形各自有一个时间上界（或明确的「不退出」）**
   Verify: 结果以秒数写进本 ticket 的 Notes，含重复 5 次的最大值与原始采样。
2. **出现「不退出」时有兜底方案的可验证描述**
   Verify: 写清下次接入靠什么识别残留、如何清理，以及清理失败的可见后果。
3. **R9 的阈值落到等待它的那张 ticket 上**
   Verify: entry 10 的 verify 与 epic 的 Requirements 同时写上具体秒数。
4. **答案被要用它的人接受**
   Verify: 负责 entry 10 的人与 initiative owner 在本 ticket 上各留一句确认，含日期。

## References

- parent — _bmad-output/initiative-docker-hosts/tickets/epic-docker-hosts/epic-docker-hosts.md
- evidence — _bmad-output/initiative-docker-hosts/prd-docker-containers/addendum.md, §3.2「无信号代理」

## Notes

- Assumption: 容器内进程的退出只取决于 stdin EOF，与容器是否还有别的 exec 无关；若采样显示相反，先记录再调整。
- 高风险，验收之外再加一道检查：用 `docker exec -i` **不**接任何客户端脚本、只把 stdin 打开再关闭，确认容器内进程同样退出——排除「是我们的驱动脚本杀掉了它」这种假阳性。
- Risk: high — 阈值决定 entry 10 是「靠 EOF 自然回收」还是「必须自己做兜底清理」，两者的实现量差一个数量级；数字错了，story 会白做。
