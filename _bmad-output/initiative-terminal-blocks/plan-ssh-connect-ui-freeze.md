---
title: 'ssh connect auth prompt freeze fix'
type: 'bugfix'
ticket: ''
created: '2026-10-29'
status: 'built'
baseline_revision: '4ae165ec88435c849eba9a3205fc34f97ae61aca'
route: 'full'
route_source: 'auto'
risk: 'high'
review: 'quick'
review_source: 'pinned'
lenses_ran: ['quick']
review_loop_iteration: 1
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** 通过 SSH 连接机器时，密码提示可能永远不显示：该 host 没有已打开的 workspace、pane 级 auth sheet 占着唯一的 model、或 banner 提示泄漏了 host gate，连接只会干等 120 秒然后以 "authentication cancelled" 失败；期间界面停在 Connecting、输入被禁用且没有取消入口。服务端实测：两次重连都在 `USERAUTH_FAILURE` 后静默 120 秒被 `LoginGraceTime` 踢掉。

**Approach:** 认证提示的投递不再依赖“该 host 已有打开 workspace”，改为落到发起连接的那个窗口；任何投递不出去的路径立即失败并给出可见原因，而不是占着 modal 空转 120 秒；auth sheet 的 host gate 不得泄漏；Connecting 状态必须可取消。

## Boundaries & Constraints

**Always:** 提示显示在发起连接的窗口（无则取该 host 的窗口，再退到最近窗口）；答案只经 `PendingAuth::answer` 回到它自己的 request_id；凭证仍只走现有 keychain 路径；新增文案四个 locale 同步（en、zh-CN、ja-JP、ru-RU）。

**Never:** 不改 SSH 认证算法、凭证存储或 `broker::PROMPT_TIMEOUT` 数值；不动 pane 链路设计（每 pane 一条 SSH session 已拆到 deferred-work）；不把密码写进配置或日志；不加明文“稍后重试”式掩盖。

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| SWITCHER_NO_WS | 切换器连接该 host，尚无该 host 的 open workspace | 密码 sheet 在发起窗口抬起，输入后连接继续 | 无错误 |
| QUEUED_HOSTS | 两个 host 同时要密码 | 先到者持 sheet，后者排队；关掉 sheet 后后者立即抬起 | 不出现无 pending 的 holder |
| BANNER | 收到 `AuthPromptKind::Banner` | banner 显示且 host gate 立即释放 | 后续 host 的 sheet 不被卡住 |
| NO_WINDOW | 任何窗口都拿不到 | 连接 <2s 内失败并显示可见原因 | 不占 modal、不空转 120s |
| CANCEL | 用户在 Connecting 点取消 | 流清空、无残留遮罩，迟到结果被丢弃 | 后台失败只记日志 |
| STALE_QUEUE | 排队的 prompt 已被放弃 | 该 host 不再被提升为 holder | 其他 host 正常抬 sheet |

## Decisions

- Connecting 提供取消入口：strip 按钮 + 切换器动作 + 四个 locale 文案；取消即清空流并丢弃迟到结果。
- pane 级 auth sheet 始终渲染（`model.is_some()` 即显示）；其 pane 已消失时清 model。

</frozen-after-approval>

## Code Map

- `src/ui/remote_workspace.rs` -- `connect_to_host` :583（记录发起窗口）、`pump_tick` :1583、`pump_auth_sheets` :2426、`raise_auth_sheet` :2496、`AuthSheetQueue` :355、`PARKED` :2520、`RemoteStatus`/`action_label` :96/:130、`finish_connect` :660、`finish_attempt` :~2140；测试 :3047-:3516
- `src/ui/ssh_prompt.rs` -- `raise_routed_auth` :411（已有 sheet 时 GiveBack）、overlay :682（pane 失焦时 :695 隐藏）、`dismiss_and_advance` :595、keychain 写入 :626
- `src/ui/remote_connect.rs` -- `PendingAuth` :664（`answer`/`is_abandoned`）、`ask_routed` :787、`AUTH_MAILBOX` :728
- `src/ui/windows.rs` -- `open_windows` :50、`window_for` :59、`app_for` :110、`register`/`unregister` :156/:169
- `src/ui/app.rs` -- remote strip 渲染 :9243、`remote_retry` :~9254
- `src/ui/i18n/{en,zh-CN,ja-JP,ru-RU}.toml` -- 新文案 key
- 复用：`PendingAuth::for_test`、`claim_mailbox`、`fake_connected` :2771、`machine` :3208、`host` :3047

## Tasks & Acceptance

**Execution:**
- [ ] `src/ui/remote_workspace.rs` -- `RemoteLinks` 增加 `auth_window: HashMap<HostId, WorkspaceId>`，`connect_to_host` 记录 `self.workspace`，`finish_connect`/`finish_attempt` 清理 -- 让提示知道该落在哪个窗口
- [ ] `src/ui/remote_workspace.rs` -- `raise_auth_sheet` 改为：host 的 open workspace → 记录的发起窗口 → 最近窗口；都无则立即 `pending.answer(Cancelled)` 并返回 `Lost` -- 消除无窗口空转
- [ ] `src/ui/remote_workspace.rs` -- `pump_auth_sheets` 对 `Banner` 不持 gate；`is_abandoned` 丢弃时同步 `auth.withdraw(host)` -- 关掉两处 gate 泄漏
- [ ] `src/ui/ssh_prompt.rs` -- 去掉 pane 失焦隐藏（`model.is_some()` 即渲染），pane 已消失时清 model -- 让 pane sheet 不再静默堵住 routed 提示
- [ ] `src/ui/remote_workspace.rs` + `src/ui/app.rs` + i18n -- `RemoteStatus::Connecting` 增加取消动作，清 `connect` 并 withdraw 该 host -- 卡住时用户有出口
- [ ] `src/ui/remote_workspace.rs` -- 新增 6 个 gpui 测试覆盖上表六个场景 -- 锁住行为

**Acceptance Criteria:**
- Given 该 host 没有 open workspace，when 从切换器发起连接并收到密码提示，then sheet 出现在发起窗口且输入后连接继续
- Given 一个 banner 提示刚被处理，when 另一台机器随后索取密码，then 后者在下一个 tick 抬起，不等 120s
- Given 排队的 prompt 已放弃（asker 释放），when 当前 sheet 关闭，then 不会出现新的 holder，第三台机器可抬 sheet
- Given 没有可用窗口，when 提示到达，then 连接在 2s 内失败并显示可见原因
- Given 用户处于 Connecting，when 点取消，then 流清空且窗口可继续操作

## Implementation Notes

## Plan Change Log

## Review Triage Log

### 2026-10-29

**Code review complete.** quick lens: 6 findings — 0 high / 2 medium / 2 low / 0 false / 0 maybe-false carried to patch; 2 low rejected.

- [x] [Review][Patch] sheet painted under the switcher — `src/ui/app.rs:10455-10458` adds the ssh prompt overlay before `.children(self.render_switcher(...))`, and the switcher keeps its full-window scrim up during connect → the sheet and its focused input are invisible. Fix: render the sheet above the switcher.
- [x] [Review][Patch] cancel leaves a pending create — `cancel_connect` (`src/ui/remote_workspace.rs:595`) clears only `connect`; `finish_connect`→`take_create_waiting_on` later consumes `pending_create`/`parked_create`, so the next connect creates the workspace that was called off. Fix: clear both slots (invariant already in `switcher_disconnect`, `src/ui/switcher.rs:1223-1234`).
- [x] [Review][Patch] late result credited to the current flow — `finish_connect` (`src/ui/remote_workspace.rs:728-742`) reads `self.connect`; cancel A then connect B marks B failed and removes B's `auth_window`. Fix: identify the attempt by host and ignore a mismatch, logged.
- [x] [Review][Patch] new dead-code warning — `src/ui/ssh_prompt.rs:298` `is_up`/`routed_host`/`banners` used only by tests; `cargo check` reports 17 warnings vs the 16 baseline while the plan expects none new. Fix: `#[cfg(test)]`.
- [Rejected][low] `auth_window` keyed by host (`src/ui/remote_workspace.rs:1450,658-660,685-689,739-741`) — concurrent attempts to one host can overwrite or cross-cancel. Evidence: needs two simultaneous connects to the same machine (the switcher disables Connect for a host already Connecting in this window); the supervisor/manual overlap falls back to `workspaces_on`/`most_recent`, which still raises. Unlikely in everyday use and the fix needs attempt identity.
- [Rejected][low] tests hand-build `auth_window` — SWITCHER_NO_WS and CANCEL insert the map and call `raise_auth_sheet`/`cancel_connect` directly, so `connect_to_host` recording and switcher routing are untested. Evidence: all six matrix rows are covered; end-to-end coverage needs a fake route (`FakeSshd` is `#[cfg(test)]` inside tty7-core, not compiled into the GUI crate). Fix beyond a direct correction.

### 2026-10-29 — bmad-review round (adversarial + edge-case + verification-gap, 33 findings)

**Verdicts:** 3 high, 9 medium, 16 low, 5 rejected. Grouped by root cause below; ids are `A`=adversarial, `E`=edge-case, `V`=verification-gap.

- [Patch][high] `A1,A2,E1,E2` + `A5,A6,E3` — no attempt identity: stale result lands on a new flow, an older future deletes the newer `auth_window`, and prompt cancellation is host-wide. Fix: `AttemptId`, tagged `PendingAuth`, attempt-scoped finish/cancel/retire.
- [Patch][high] `E9` — the ssh scrim has no `.occlude()`, so after moving it above the switcher a click passes through to the switcher rows. Fix: `.occlude()` (as `src/ui/prompt.rs:238`).
- [Patch][medium] `A3,A4` — `withdraw` never releases the holder; `cancel_auth_prompts` releases by host without owning the sheet. Fix: release on abandon; release only this attempt's sheet.
- [Patch][medium] `A12,E7,A17` — cancel leaves install progress and a pending install consent, and does not refresh other windows. Fix: clear progress, drop the consent, `refresh_windows`.
- [Patch][medium] `A13,E5` — `disconnect` and leaving the workspace drop `auth_window` but leave the prompts waiting. Fix: cancel that attempt/machine.
- [Patch][medium] `A10,E6` — orphaned pane sheet cleared with `clear()`, skipping focus restore and the next pane. Fix: `dismiss_and_advance`.
- [Patch][low] `A8` — a missing `WindowRegistry` is treated as "no window ever". Fix: GiveBack while the registry is absent.
- [Patch][low] `A16` — `RemoteActionCancel` duplicates `L10nKey::Cancel`. Fix: reuse the existing key.
- [Patch][low] `A14` — `status_of` comment still says Connecting has no action button. Fix: update it.
- [Patch][medium] `V1,V3,V4,V5,A15` — untested: mailbox/parked cancel, strip Cancel gate, `pending_create` clearing, orphan cleanup through the pump, cancel→same-host retry. Fix: add those tests.
- [Defer] `A7` — `most_recent` fallback ignores visibility and can steal focus; needs a window-visibility API.
- [Defer] `A9` — a background pane's prompt is now modal with no cue about which tab asked; UX addition.
- [Defer] `A11` — banners accumulate for the session; product decision on scope/auto-dismiss.
- [Defer] `E8` — an install in progress hides both Cancel affordances; render-path follow-up.
- [Defer] `E10` — fail-fast degrades to parked when a window exists but already shows another sheet; needs a per-host sheet queue review.
- [Defer] `V2,V6` — no render test for the switcher Cancel band; no click-through test for the overlay z-order. Test-harness work.
- [Rejected][low] `A7`-adjacent focus-steal claim is unverified beyond the above deferral.

## Design Notes

本轮卡死不是网络：服务端在认证阶段静默满 120s。GUI 侧有三条独立死路：`raise_auth_sheet` 要求 host 已有 open workspace；`raise_routed_auth` 在已有 sheet 时返回 GiveBack（pane sheet 无超时且会因失焦隐藏）；banner 分支返回 `Raised` 却不设 `routed_host`，gate 永不释放。修法是让“抬起提示”有确定的宿主窗口、让投递失败快速可见，并保证 gate 与 pending 同生共死。

## Verification

**Commands:**
- `USER=root LOGNAME=root cargo test -p tty7 remote_workspace` -- expected: all pass
- `USER=root LOGNAME=root cargo test -p tty7 ssh_prompt` -- expected: all pass
- `cargo check -p tty7` -- expected: pass, no new warnings
- `cargo fmt --check` -- expected: clean

**Manual checks (if no CLI):**
- 断网后从切换器连接一台未打开过的机器：密码框应出现在当前窗口；关闭窗口前的连接应 2s 内失败，而不是转 120s


### 2026-10-29 — bmad-review round 2 (36 findings)

- [Patch][high] `A1,E12` — `finish_connect` removes `auth_window` without the attempt guard every other site uses.
- [Patch][high] `A10` — the scrim now occludes even when only banners are up, so one notice blocks the whole window. Fix: occlude only when a model is shown.
- [Patch][high] `A4,E5` — a cancelled attempt can still raise a prompt. Fix: cancelled-attempt set consulted before posting and by the pump.
- [Patch][medium] `A5,E1,A9,E2` — cancel/disconnect/enter close only this window's sheet. Fix: sweep the open windows for the same host/attempt.
- [Patch][medium] `A6,E7` — cancel is undone by the supervisor's next dial. Fix: suspend the host when the manual attempt had not landed.
- [Patch][medium] `A9,E2,E3` — disconnect leaves install consent and progress behind.
- [Patch][medium] `A11,E11` — a background pane sheet steals focus and `pane_id` is write-only. Fix: focus only when its pane is the focused one.
- [Patch][medium] `A13,E9` — orphaned pane sheet is cleared without answering the pane.
- [Patch][medium] `A14,E8` — the switcher's Cancel is hidden behind the install banner.
- [Patch][low] `A16` — `pump_tick`'s auth_window early return skips per-host cleanup.
- [Patch][low] `E8` — a banner whose raise gives back is dropped instead of parked.
- [Patch][medium] `A17,V3,V4,V5` — tests for the finish guard, the cancelled set, cross-window sweeping, disconnect cleanup, gate release through the pump, and the note/retire bookkeeping.
- [Defer] `A2,E10` — `ATTEMPT_BY_HOST`/`auth_window` still hold one slot per host; two live attempts to one machine can still collide.
- [Defer] `A3` — `ask_routed` stamps a supervisor/pane prompt with whichever manual attempt is on the wire for that host.
- [Defer] `A7` — install progress is keyed by host; one attempt's cleanup clears another's.
- [Defer] `A8,V1` — a consent modal already raised cannot be cancelled; only the mailbox window is caught.
- [Defer] `A12` — search and the notification layer render above the sheet.
- [Defer] `A15` — focus is not restored to the pre-sheet handle while the switcher stays open.
