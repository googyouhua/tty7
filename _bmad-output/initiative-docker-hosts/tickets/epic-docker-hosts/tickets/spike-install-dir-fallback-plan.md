---
title: "安装目录回退顺序 (install dir fallback)"
ticket: 3
status: built
baseline_revision: 02e163ea
---

## Intent

回答一个问题：在各类容器里，`tty7-server` 写到哪里、并且还能执行？优先级该是什么顺序？答案要给 entry 7（安装位置回退）和 epic 的 R5 一个可核对的顺序。

环境：192.168.1.25（Docker 29.1.3, amd64），镜像 `node:alpine`，二进制为发布版 `tty7-server-c12p6`（x86-64 静态链接）。二进制经宿主 bind mount 只读提供，避开 `docker cp` 写 tmpfs 的问题。

## Implementation Notes

- 第一版探针用 `docker cp` 把二进制送进容器，四类全部判成 `WRITE_FAIL`——`docker cp` 写不进 tmpfs 挂载点。改成 `-v /tmp/tty7probe:/probe:ro` 后结果才有意义。
- 每个候选做三件事：`mkdir -p`、复制二进制、`chmod +x`、然后真的执行 `--help`。只 stat 不算——`noexec` 挂载下文件可写可执行位齐全，就是跑不起来。
- 五类容器：root、非 root（uid 1000，有家目录）、非 root 且无家目录（uid 65534，`HOME=/`）、只读 rootfs、`/tmp` 与 `/dev/shm` 挂 `noexec`。

## Results

| 候选目录 | root | 非 root (1000) | 无人用户 (65534) | 只读 rootfs | noexec /tmp+shm |
| --- | --- | --- | --- | --- | --- |
| `$HOME/.local/share/tty7/bin` | OK | OK | MKDIR_FAIL (`/`) | MKDIR_FAIL | OK |
| `/tmp/tty7-<uid>/bin` | OK | OK | OK | MKDIR_FAIL | EXEC_FAIL |
| `/dev/shm/tty7-<uid>/bin` | EXEC_FAIL | EXEC_FAIL | EXEC_FAIL | EXEC_FAIL | EXEC_FAIL |
| `/run/user/<uid>/tty7/bin` | OK | MKDIR_FAIL | MKDIR_FAIL | MKDIR_FAIL | OK |

三条结论：

1. **`/dev/shm` 必须从候选里删掉。** Docker 默认以 `noexec` 挂载它，五类容器无一例外。ticket 里写的「`/dev/shm` 默认允许执行」这条 `Assumption:` 是错的。
2. **固定顺序**：`$HOME/.local/share/tty7/bin` → `/tmp/tty7-<uid>/bin` → `/run/user/<uid>/tty7/bin`。前两个覆盖了除只读 rootfs 之外的全部情形；第三个在 root 下可用，在非 root 容器里通常建不出来，但没有代价。
3. **只读 rootfs 且没有显式 exec tmpfs 时，没有可用位置。** 这不是实现缺口，是容器的物理现实：`/dev/shm` 是唯一还写得的 tmpfs，而它 noexec。此时必须报错并列出尝试过的路径，不能降级到「装不上也照样连」。

## Verification

- 五类容器各跑一遍，每个候选都在容器内真实执行过二进制，不是看权限位。
- 每类的失败原因被区分成 `MKDIR_FAIL` / `WRITE_FAIL` / `CHMOD_FAIL` / `EXEC_FAIL`，可以直接变成错误文案里的分类。
- 反证：`/dev/shm` 在所有类中都失败，说明探针确实在测执行而不只是测写权限。

## Consequences for the tree

R5 与 entry 7 需要按此改写（由 Sofia 落地）：

1. 顺序固定为 `$HOME/.local/share/tty7/bin` → `/tmp/tty7-<uid>/bin` → `/run/user/<uid>/tty7/bin`。
2. `/dev/shm` 不进入候选，理由写进 R5，避免以后有人「顺手加回来」。
3. 只读 rootfs 无 exec tmpfs 的场景进入 R10 的「无可写安装目录」一类，错误必须列出全部候选与各自原因。
4. entry 7 的验收用三类容器：只读 rootfs、`noexec` 的 `/tmp`、非 root 且无家目录。

## Notes

- 本次测量跑在远程 docker 主机（192.168.1.25）上；被测行为发生在容器内本地文件系统，与 ssh 无关。
- 未测：NFS / 9p 挂载的家目录、SELinux 强制模式。两者都可能改变结论，若用户报告相关环境再补。
