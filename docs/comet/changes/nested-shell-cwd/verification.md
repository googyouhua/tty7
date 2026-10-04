---
generated_from_state_version: 12
---

# 验证

## 当前结果

- 结果: **验收通过，可归档**
- 验证情况: **已完成检查，验证结果已确认**
- 目标周期: 2
- 迭代: 2
- 验证器尝试次数: 1
- 完成时间: 2026-10-04T02:13:10.067Z
- 摘要: All eight acceptance items verified in code plus unit/behavioral tests by independent read-only inspection; real-machine behavior confirmed on 25.

## 验收

| 编号 | 结果 | 来源 | 验收项 | 原因 |
| --- | --- | --- | --- | --- |
| A1 | passed | brief.md | A1: 本地 pane 外层 bash `cd` 后 cwd 跟随（回归现有行为）。 | shell-wins guard keeps outer OSC7 authoritative; info fallback intact |
| A2 | passed | brief.md | A2: `sudo -s` 后内层 root bash `cd /tmp`，Info/Review 显示 `/tmp`（误差 ≤ 1 次 prompt/轮询周期）。 | deepest-in-group resolution with ordered first-readable probe, Linux+macOS |
| A3 | passed | brief.md | A3: 退出内层 shell 回到外层，cwd 恢复跟随外层目录。 | fresh OSC7 skips probe/title and restamps clock |
| A4 | passed | brief.md | A4: 现有单测全过（daemon pane cwd 相关：`pane.rs:6394/6478/6582` 附近用例）。 | only listed files changed; live tests intact; suites green |
| A5 | passed | brief.md | A5: 远端 pane（ssh 进服务器后 `su` 到 root）`cd /tmp`，Info 显示 `/tmp`（Debian/Ubuntu 系标题；无标题格式的机器保持冻住但不报错）。 | strict title parsing, remote always-on, hop-race grace, stale-only, fresh-wins |
| A6 | passed | brief.md | A6: Info 行 cwd 点击变文本输入，输入绝对路径回车后各面板跟随手动值；清空后恢复自动跟随；Win/Linux 一致，无系统弹窗。 | session manual_cwd first in both effective fns; inline input flow with L10n |
| A7 | passed | brief.md | A7: Info 面板 cwd 下方独立“跟随”开关行，per-pane 默认开（=off 才冻住）；开关只门控标题兜底的本地部分（A5 远端常开不受影响）；手动 pin 追踪值回到同目录时自动解 pin、显示不动；会话级，不持久化。 | follow defaults on everywhere; pin auto-releases on tracked rejoin |
| A8 | passed | brief.md | A8: bash rcfile 追加自包含 OSC7 上报尾巴并 export PROMPT_COMMAND：`su`（不带 `-`）与透传型 `sudo -s` 进来的内层 bash 自动精确上报；外层行为不变；`su -`/`sudo -i` 与 zsh 内层仍走标题/手动。 | rcfile chain freeze plus self-contained export; env-cleared bash proves OSC7 and silence |

## 检查

| 检查 | 命令 | 工作目录 | 状态 | 退出码 | 耗时 |
| --- | --- | --- | --- | ---: | ---: |
| daemon pane suite | test -p [REDACTED] --lib -- daemon::pane | . | passed | 0 | 1337 ms |
| procinfo suite | test -p [REDACTED] --lib -- daemon::procinfo | . | passed | 0 | 840 ms |
| terminal view suite | test --bin tty7-app -- terminal::view::gpui_tests | . | passed | 0 | 37997 ms |
| right panel suite | test --bin tty7-app -- ui::right_panel | . | passed | 0 | 3752 ms |

### Builder 报告的证据

以下为 Builder 报告，不等同于 Runtime 检查凭据或独立验收结果。

- daemon pane/procinfo/handoff/protocol/server suites: passed — 232 passed
- shell_integration suite: passed — 72 passed; 2 pre-existing env failures proven unrelated via stash (live-pipe tests fail pristine too)
- terminal view + right_panel suites: passed — 168 passed incl. manual_cwd/follow_ui tests
- 已知限制: macOS arms compile-mirrored only, no Mac hardware here; same table tests + CI
- 已知限制: sudo -s scrubs env (clean retest overrode earlier false-positive); su inherits; scrubbing sudos fall through to title/manual/switch
- 已知限制: RHEL-no-title far hosts stay frozen by design
- 已知限制: 2 shell_integration live-pipe tests fail in this root container with and without the change
- 已知限制: shell_integration suite excluded from Runtime plan: 2 live-pipe tests fail in root containers with and without the change (stash-proven); covered as builder-reported evidence

## 阻塞项

_无。_

## 风险与跳过的工作

- macOS arms compile-mirrored only, no Mac hardware
- RHEL-no-title far hosts stay frozen by design
- 2 pre-existing shell_integration live-pipe failures in root containers

## 之前的迭代

| 目标周期 | 迭代 | 尝试 | 结果 | 未解决项 | 摘要 | 完成时间 |
| ---: | ---: | ---: | --- | --- | --- | --- |
| 1 | 1 | 1 | blocked | — | recovered after worktree loss; re-verify required | 2026-10-04T01:42:43.000Z |
| 1 | 1 | 1 | recovery | — | Native confirmed acceptance criteria changed | 2026-10-04T01:42:49.420Z |
| 2 | 1 | 0 | recovery | — | Builder handoff Runtime checks failed: shell-suite | 2026-10-04T02:04:25.744Z |
| 2 | 2 | 1 | pass | — | All eight acceptance items verified in code plus unit/behavioral tests by independent read-only inspection; real-machine behavior confirmed on 25. | 2026-10-04T02:13:10.067Z |



## 结论

All eight acceptance items verified in code plus unit/behavioral tests by independent read-only inspection; real-machine behavior confirmed on 25.
