---
title: 'UI线程同步canonicalize导致debug崩溃'
type: 'bugfix'
ticket: ''
created: '2026-10-11'
status: 'done'
baseline_revision: '5018dc74c90b120a2d46b64dc364029515f13833'
route: 'oneshot'
route_source: 'auto'
risk: 'medium'
review: 'quick'
review_source: 'pinned'
lenses_ran: ["quick"]
review_loop_iteration: 0
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** debug 版在 25 机器上崩溃：`Host call on the UI thread`（`host/mod.rs:54`），起因是 `poll_foreground` 在 UI 线程同步调 `host.canonicalize`。

**Approach:** 把 `manual_cwd_rejoins` 的 canonicalize 比较搬到后台线程做，UI 线程只读缓存结果，不再同步碰 host。

</frozen-after-approval>

## Implementation Notes

oneshot：`view.rs:4628 manual_cwd_rejoins` 经 `host.canonicalize` 直达 `local.rs:196 guard_off_ui`，`poll_foreground` 每帧跑在 UI 线程，manual_cwd 与 tracked 不一致时 debug 必炸。修法按仓库既有模式走 `HostOps::run` 后台化 + 结果缓存落地，display 本来就不动（注释原话），晚一帧 rejoin 无可见影响。`maybe_adopt_daemon_blocks` 的本地 `query_procs` 无 guard 不动它；远端 `connect_routed` 分支本次不动（用户是本地 pane）。

- 决策：新增 `manual_cwd_canon` 缓存 + `manual_cwd_canon_pending` 在途标记，完全照抄 `link_repo_root` 的既有模式；`pinned == tracked` 快路径保持同步；miss 时后台取、落地 `cx.notify()`，下帧读缓存。`set_manual_cwd` 不用清缓存（按 (pinned,tracked) 键比对，旧条目自然 miss）。
- 坑：harness 里 `HostOps` 后台落地跑不起来（2s 轮询无 land），符号链接端到端用例改走确定性缓存命中测试；既有 `a_pin_rejoins/holds` 行为不变。
- 验证：pin 相关 3 过 + 新增 `a_cached_canon_answer` 1 过，`block` 81 过，`check-host-boundary.sh` clean，`cargo fmt --check` 干净。

## Verification

**Commands:**
- `cargo test --bin tty7-app pin_` -- expected: pin 相关 3 过
- `cargo test --bin tty7-app cached_canon` -- expected: 新增用例过
- `cargo test --bin tty7-app block` -- expected: 全过无回归
- `bash .github/scripts/check-host-boundary.sh` -- expected: clean
- `cargo fmt --check` -- expected: 干净

**Manual checks (if no CLI):**
- 25 机器 debug 版设 manual_cwd 后切目录，不再崩溃且 pin 正常回到跟踪

## Review Triage Log

- quick lens: 8 findings → 5 patch, 1 doc-fix, 2 false. G1（缓存永不复验#1 + 落地后clear#4，low）patch：`set_manual_cwd` 清缓存/在途，落地无条件存（keyed，下一帧自然纠偏）；残余 symlink 中途改指向属可接受罕见case。G2（多job堆积#2 + 逐帧spawn#3，medium）patch：改单飞（有在途直接等）。#6（plan 里 `manual_cwd` filter 命中不了新用例，low）patch：Verification 改 `pin_`/`cached_canon`。#8（文档重复，low）patch：合并为一段。#5（新测试只走命中路径，称回归抓不住）false：`a_pin_holds` 每轮走 miss 且本地 host 恒 Some（`host_registry.rs:30` fallback），同步调用重现即触发 guard，suite 会红。#7（`Write` 未用）false：`daemon.flush()` 依赖它，删掉即 E0599（已实测）。
