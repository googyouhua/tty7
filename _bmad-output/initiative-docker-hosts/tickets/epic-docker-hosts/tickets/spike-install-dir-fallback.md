---
id: 3
type: spike
title: "安装目录回退顺序 (install dir fallback)"
parent: epic-docker-hosts
covers: [R5]
after: []
refined: true
hitl: false
risk: medium
---

# 安装目录回退顺序 (install dir fallback)

## Description

问题：在各类容器里，`tty7-server` 写到哪里、并且还能执行？优先级该是什么顺序？

为什么是开的：容器可能是 root、非 root、只读 rootfs、`noexec` 挂载，`$HOME` 不一定可写。现在的实现只有一条路径，R5 的验收没有可核对的顺序。

谁在等：entry 7（安装位置在没有写权限时回退）的验收，以及 epic 中 R5 那一条。

## Approach

1. 准备四类运行方式：root 用户、非 root 用户、只读 rootfs（`--read-only`）、`/tmp` 与 `/dev/shm` 分别以 `noexec` 挂载。
2. 候选目录按此顺序验证：`$HOME/.local/share/tty7/bin` → `/tmp/tty7-<uid>/bin` → `/dev/shm/tty7-<uid>/bin` → `/run/user/<uid>/tty7/bin`。
3. 每个候选做三件事：写入一个测试文件、`chmod +x`、执行它（不是只 stat）。
4. 记录每类容器命中哪个候选、前几个为什么失败（写不进 / 写进但执行被拒）。
5. 把结果整理成一张表，作为实现时的固定顺序。

时间盒：半天。够好的标准：四类容器、每个候选都有测试结果，没有「理论上应该可以」。
签收：顺序由负责 entry 7 的人确认能直接实现；initiative owner 点头后本 ticket 关闭。

## Acceptance Criteria

1. **四类容器与候选目录的交叉结果成表**
   Verify: 表写进本 ticket 的 Notes，每个格子是「可写可执行 / 写不进 / 执行被拒」之一。
2. **顺序固定且能解释每一次跳过**
   Verify: 顺序写进 epic 的 R5 行与 entry 7 的 verify，每个跳过都有原因。
3. **全部不可写时的错误消息有定义**
   Verify: 写明消息里要出现的字段（尝试过的路径、每条的失败原因），并有一个真实镜像上复现的输出。
4. **答案被要用它的人接受**
   Verify: 负责 entry 7 的人与 initiative owner 在本 ticket 上各留一句确认，含日期。

## References

- parent — _bmad-output/initiative-docker-hosts/tickets/epic-docker-hosts/epic-docker-hosts.md
- evidence — _bmad-output/initiative-docker-hosts/prd-docker-containers/addendum.md, §3.2「非 root 且 $HOME 不可写」

## Notes

- Assumption: `/dev/shm` 在目标内核上默认允许执行；若被 `noexec` 挂载，该候选直接删除并重排。
- 本 ticket 的结论会改写 epic 的 R5 行与 entry 7 的 verify——这是它的交付物，不是副作用。
- Risk: medium — 顺序错了的表现是「某些镜像装不上」，可以事后加目录绕过，不会伤数据；但会让 R5 的验收一直悬着。
