---
title: 'ws unreachable UI freeze fix'
type: 'bugfix'
ticket: ''
created: '2026-10-17'
status: 'built'
baseline_revision: 'fbbecad524c86f9a5e1a97d9e0e37969b0c25fb0'
route: 'oneshot'
route_source: 'auto'
risk: 'medium'
review: 'quick'
review_source: 'pinned'
lenses_ran: ['quick']
review_loop_iteration: 0
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** 打开不可达 ws（远端服务器 SSH 失败）后，当前界面卡死：无法切换 ws、无法打开配置，点击任意位置无响应。

**Approach:** 让连接失败落到非模态 Failed 状态并保留切换/配置入口，不让全屏 occlude 遮罩在无可操作内容时拦截全部点击，连接保持 off-UI-thread。

</frozen-after-approval>

## Implementation Notes

Oneshot：单目标、预估 <100 行，只动失败态 UI 生命周期（auth sheet / prompt scrim / pending pane / switcher band），不碰 daemon SSH 握手、config 真值、`vendor/simple-dns`；Intent 已足够，无需 full 规划开销。

实际改动（worktree `.worktrees/fix-ws-unreachable-ui-freeze`）：
- `src/ui/pending_pane.rs`：`PendingSpawn` 新增 `native_ssh` + `native_start_dir`，native 重拨可 parked 为 Connecting。
- `src/ui/app.rs`：`session_to_pane` 的 native SSH 分支不再同步 `new_terminal_native`（UI 线程阻塞 N*2s），改为立 `PendingPane::Connecting` + `start_native_spawn` 后台拨号；新增 `start_native_spawn` / `land_native_pane`（失败即 `fail(reason)` + Try Again，不 auto-retry）；`start_pane_spawn` 遇 native 转交；`pane_to_session` 的 Connecting 分支保留 `ssh_spec`/`cwd`，二次恢复不丢。
- `src/terminal/view.rs`：`NativeSshParts::pane_id` 改 `pub(crate)`，供 slot 关闭后的后台清理。
- 其余 `PendingSpawn` 构造点（tests/document_column/diff_overlay）补 `None`。
- 2026-10-17 rebase：`--onto origin/main` 丢掉与已合 #31 同内容的 `fbbecad5`，现基 `c40d31bc`，提交为 `453eb21` + `8c3d668`，已 force-with-lease 推；check + native 单测重过。

验证：
- `cargo check -p tty7` 通过（16 warnings，既有）。
- `cargo test -p tty7-core ssh` 175 passed。
- `cargo fmt --check` clean。

## Plan Change Log

## Review Triage Log

- Finding 1 (occlude overlays untouched, `prompt.rs:238`/`ssh_prompt.rs:709-729`) — verdict: defer. Evidence: parked native redial lands as inline `PendingPane::Failed` + Try Again, no overlay left up on this path; daemon-mismatch no-cancel trap is pre-existing and not triggered by SSH-unreachable. Deferred as old bug/idea for later.
- Finding 2 (workspace supervisor path unchanged) — verdict: maybe-false (would be high if true), defer. Evidence: remote `new_terminal` branch was already async `PendingPane` (`app.rs:11026-11044`); sync UI block found was native restore. What would prove it: repro naming ws type (remote workspace with zero native panes vs workspace holding native SSH panes) still freezing after this change.
- Finding 3 (`resolve_persisted_ssh_spec` sync on UI, keychain/fs) — verdict: low, rejected. Evidence: `ssh_connect.rs:392-427` reads are bounded/fast vs 2s dial moved off thread; moving `cx.global` resolve off thread adds complexity for rare stall. Normal use rarely hits.
- Finding 4 (Try Again stale creds + secrets in snapshot) — verdict: medium, patched. Evidence: parked `PendingSpawn.native_ssh` held resolved secrets, `pane_to_session` persisted them. Fix: Retry re-resolves on UI thread + `retrying()`; `pane_to_session` stores `without_secrets()`.
- Finding 5 (landed native drops agent wiring) — verdict: medium, patched. Evidence: `land_native_pane` missed `AgentSessionChanged`/`AgentDetected`, `agent_resume_command`, role followup vs `land_pane:4177-4210` + `build_terminal_view`. Fix: mirrored subscriptions + resume + followup.
- Finding 6 (double-click concurrent redials) — verdict: false. Evidence: same fire-and-forget pattern as workspace `start_pane_spawn`; loser killed off-thread. No new bug.
- Finding 7 (no new test for park/preserve/land round trip) — verdict: low, rejected. Evidence: headless lifecycle needs window harness beyond oneshot; covered by `cargo check` + `tty7-core ssh` 175 passed + `fmt clean`, same as prior `plan-instance-picker-freeze` precedent.
- Process (bare `cargo test` not run) — verdict: noted. Evidence: `cargo test -p tty7-core` 1828 passed / 3 failed in `shell_integration` + `spawn::signal_and_await_exit`; diff touches no `crates/` files (`git diff --name-only` shows only `src/ui/*` + `src/terminal/view.rs`), failures unrelated/environmental. Bare `cargo test` timed out at 120s (GUI build).

## Verification

**Commands:**
- `cargo fmt --check -- src/ui/ssh_prompt.rs src/ui/remote_workspace.rs src/ui/pending_pane.rs src/ui/prompt.rs` -- expected: clean
- `cargo test -p tty7-core ssh` -- expected: pass
- `cargo check -p tty7` -- expected: pass (36 warnings 既有除外)

## Code Review

### 2026-10-17

**Code review complete.** 0 `decision-needed`, 4 `patch`, 0 `defer`, 13 rejected.

- [x] [Review][Patch] Window-gone dial leaks far-side pane [src/ui/app.rs:11225] — `start_native_spawn` discards `update_in` result; window closed mid-dial drops dialed `pane_id` without `kill_pane_off_thread`. → fixed in `adce9630`: pane_id kept outside the closure, reaped off-thread on `update_in` failure.
- [x] [Review][Patch] No test pins native park/dial/retry round trip [src/ui/app.rs:10981] — reverting to sync `new_terminal_native` or dropping the `RetryRequested` subscription fails no checked test; add `gpui::test` asserting `Connecting` + preserved spec/start-dir + one background dial. → `a_native_ssh_restore_parks_connecting_off_thread` added, passes (park + background Failed landing).
- [x] [Review][Patch] No test observes landed native wiring [src/ui/app.rs:4222] — deleting agent subscriptions/resume/followup or stale-slot kill fails no checked test; add success/error/stale-slot `gpui::test`s. → `a_failed_native_land_fails_the_pending_pane` added (error path), passes. Success/stale-slot paths untestable without a `NativeSshParts` test ctor (`terminal`/`persist` private, live dial required) — documented gap, no new surface added for it.
- [x] [Review][Patch] No test pins stripped persist of parked redial [src/ui/app.rs:10621] — storing raw secrets or wrong cwd fails no checked test; extend connecting-pane test with native spawn asserting `without_secrets()` + `native_start_dir` cwd. → `a_parked_native_redial_saves_no_secrets` added, passes.

**Rejected**
- false: background dial has no timeout (Local 2s `spawn_reply` budget exists, `remote.rs:2362`).
- false: stale slot leaves orphaned Connecting UI (`!still_there` means leaf already removed).
- false: `native_start_dir` fallback loses far cwd (fallback only when original cwd was `None`).
- false: `restore_pane: None` loses affinity (old pane dead by construction; fresh dial intended).
- false: `start_native_spawn` None-guard spins forever (all 3 call sites guarantee `Some`).
- false: Retry with `None` stuck failed (same; unreachable state).
- false: stale-slot kill uses wrong route (`for_workspace(None)` = Local is correct; native always via local daemon).
- false: `replace_leaf_in` check-then-act races (single UI-thread closure, no interleave).
- false: `font_size` NaN renders invisible (finite from config/session; unshown state).
- low: concurrent Try Again dials race (wasted 2s dial, loser killed; same as workspace path) — not worth generation-counter state.
- low: error path skips `save_session` (matches workspace `land_pane`; transient state).
- low: duplicated land wiring without shared helper (no named divergence yet; refactor churn).
- low: empty user/host machine label (profiles require both; cosmetic, rare).
