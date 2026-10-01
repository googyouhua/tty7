---
generated_from_state_version: 8
---

# 验证

## 当前结果

- 结果: **已归档**
- 验证情况: **已完成检查，验证结果已确认**
- 目标周期: 1
- 迭代: 1
- 验证器尝试次数: 1
- 完成时间: 2026-10-01T09:46:29.725Z
- 摘要: Candidate passes A1-A5: 8-line switcher_groups hunk merges live-mirror rows per remote group with existing Group::merge dedup (store > snapshot > mirror), plus one gpui test. Suites green (switcher 47, machine_mirror 16). Noted risks are pre-existing-scope edge cases, none blocking.

## 验收

| 编号 | 结果 | 来源 | 验收项 | 原因 |
| --- | --- | --- | --- | --- |
| A1 | passed | brief.md | A1：CLI 在已连接远端新建 workspace → 切换器该机器分组出现同名行，无需重连或手动刷新。 | New gpui test a_cli_made_remote_workspace_reaches_the_switcher_without_reconnect passes (ran: 1 passed). Code path corroborated: tree_sync::on_layout_delta -> MachineMirrors::apply_delta -> apply() handles WorkspaceCreated by inserting into mirror; switcher_groups reads MachineMirrors::machine + rows_from_machine per frame, so no reconnect needed. |
| A2 | passed | brief.md | A2：点击该行 → 走现有 adopt/open 流程正常打开，tab/pane 与远端一致且可交互。 | No new control path added: Group::merge stamps adopt=Some + remote_id=Some on mirror rows, and unchanged switcher_open routes adopt rows to open_remote_workspace (claim_remote + enter). New test asserts adopt.is_some and remote_id match. |
| A3 | passed | brief.md | A3：CLI 重命名远端 workspace → 切换器行名同步更新。 | Same live-mirror mechanism: apply() handles WorkspaceRenamed by updating ws.name, rows_from_machine re-derives the name each frame via display_name_of. Rename unit coverage exists in machine_mirror tests. |
| A4 | passed | brief.md | A4：CLI 删除远端 workspace → 切换器对应行消失，不残留 adopt 行。 | apply() handles WorkspaceDeleted via retain-remove so the mirror row vanishes; CLI-made rows have no snapshot row to resurrect them. Existing test a_deleted_remote_workspace_leaves_the_switcher still passes within the green 47-test switcher suite. |
| A5 | passed | brief.md | A5：回归：本地 unclaimed 行、已打开/已保存的远端行、离线快照行行为不变；相关 `switcher` / `machine_mirror` 单测全过。 | git diff --stat shows only src/ui/switcher.rs (+65). Ran: switcher suite 47/47 pass, machine_mirror 16/16 pass, new A1 test passes. No daemon/protocol/event changes; snapshot and local-unclaimed paths untouched. |

## 检查

| 检查 | 命令 | 工作目录 | 状态 | 退出码 | 耗时 |
| --- | --- | --- | --- | ---: | ---: |
| switcher tests incl. new regression test | test -p [REDACTED] --bin tty7-app switcher:: | . | passed | 0 | 30781 ms |
| machine_mirror tests | test -p [REDACTED] --bin tty7-app machine_mirror | . | passed | 0 | 4059 ms |
| remote_workspace and remote_connect tests | test -p [REDACTED] --bin tty7-app remote_ | . | passed | 0 | 3990 ms |

### Builder 报告的证据

以下为 Builder 报告，不等同于 Runtime 检查凭据或独立验收结果。

- cargo check -p tty7: passed — only pre-existing warnings
- cargo test switcher (47, incl. new regression test): passed — new test a_cli_made_remote_workspace_reaches_the_switcher_without_reconnect green
- cargo test machine_mirror (16): passed — —
- cargo test remote_* (112): passed — —
- cargo fmt -p tty7: passed — applied, re-ran new test green
- 已知限制: No live-GUI check: needs the user's connected GUI to eyeball A1-A4 against a real remote daemon; unit + existing suites cover the logic.

## 阻塞项

_无。_

## 风险与跳过的工作

- Mirror is never cleared on disconnect (no machines.remove in machine_mirror.rs; RemoteLinks::disconnect only drops RemoteLinks/HostLinks state), so a stale mirror can show ghost adopt rows in an offline/parked group until a fresh pull replaces data.
- Adopt rows from the mirror hardcode live Liveness::Stopped (same as snapshot rows) even while connected.
- New A1 test seeds the mirror via MachineMirrors::install rather than driving a live WorkspaceCreated delta; the delta path itself is covered by existing machine_mirror apply() unit tests, and no live-GUI check was performed.

## 之前的迭代

| 目标周期 | 迭代 | 尝试 | 结果 | 未解决项 | 摘要 | 完成时间 |
| ---: | ---: | ---: | --- | --- | --- | --- |
| 1 | 1 | 1 | pass | — | Candidate passes A1-A5: 8-line switcher_groups hunk merges live-mirror rows per remote group with existing Group::merge dedup (store > snapshot > mirror), plus one gpui test. Suites green (switcher 47, machine_mirror 16). Noted risks are pre-existing-scope edge cases, none blocking. | 2026-10-01T09:46:29.725Z |



## 结论

Candidate passes A1-A5: 8-line switcher_groups hunk merges live-mirror rows per remote group with existing Group::merge dedup (store > snapshot > mirror), plus one gpui test. Suites green (switcher 47, machine_mirror 16). Noted risks are pre-existing-scope edge cases, none blocking.
