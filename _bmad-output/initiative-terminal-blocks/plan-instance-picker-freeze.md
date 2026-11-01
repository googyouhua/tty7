---
title: 'instance picker freeze fix'
type: 'bugfix'
ticket: ''
created: '2026-10-11'
status: 'built'
baseline_revision: 'e094a55106de5ec7b8fa0f03bcf7679e74fc1534'
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

**Problem:** 软件 bare 打开时的 username 选择 UI（startup instance picker）经常卡住：进程在、无窗口或窗口无响应，必须杀掉重开。

**Approach:** 只改 `src/ui/instance_picker.rs` 的 `run_picker` 生命周期：同步开窗、每条退出路径都落 result 并 quit、接住 OS 关窗、双重 confirm 只执行一次。

</frozen-after-approval>

## Implementation Notes

Oneshot：单文件、预估 <100 行，只动 picker 生命周期，不碰 daemon/配置协议；Intent 已足够，无需 full 规划开销。

实际改动（`src/ui/instance_picker.rs`）：
- `finish` 加已决 guard：Enter 经 Input PressEnter + 全局 Enter 键绑定会进两次，第二次直接返回，避免 double quit；Windows relaunch 失败不落 result，可重试。
- `run_picker` 改同步开窗：去掉 `cx.spawn(async { cx.update })`，与 `main.rs` 一致，消掉无窗空隙导致的 idle 无 UI。
- `with_quit_mode(LastWindowClosed)`：OS X 关窗在 macOS（Default=Explicit）原来会把 `application.run` 晾在零窗口上 hang，现在关窗即 quit 返回 None。
- `open_window` 失败落 `Some(None)` + `cx.quit()` + 日志：原来只落 result 不 quit，会 hang 在无窗事件循环里。

验证：
- `cargo check -p tty7` 通过（36 warnings，既有）。
- `cargo test -p tty7-core instance` 13 passed。
- `cargo test -p tty7` 编译失败是既有问题：baseline（stash 后）同样 25 个 E0252（`view.rs:10183` 重复 import），与本改动无关。
- `cargo fmt -- --check src/ui/instance_picker.rs` clean。

## Plan Change Log

## Review Triage Log

- Finding 1 (X-close 不写 outcome，`instance_picker.rs:385-390` vs `:429-439`) — verdict: low, rejected. Evidence: X-close 后 `result` 保持外层 `None`，`lock().clone().flatten()` 得 `None`；`open 失败` 分支写 `Some(None)` 后 flatten 同样得 `None`；`main.rs:730-738` 对两种 `None` 都是 quiet return，用户行为一致，无 hang。显式 close-handler 需要新 gpui observer 接线，超出本次 oneshot 的简单修正范围。
- Finding 2 (freeze 行为无 lifecycle 测试，`instance_picker.rs:468-524` 仅覆盖 `PickerState`) — verdict: low, rejected. Evidence: `finish` guard / 同步开窗 / `LastWindowClosed` / open-failure quit 确无 headless 覆盖；但 gpui `run_picker` 的 headless 生命周期测试需要 `TestAppContext` + 窗口 harness，非简单增补；本次已有 `cargo check` + `tty7-core instance` 13 passed + `fmt clean`，回归风险可接受。

## Verification

**Commands:**
- `cargo test instance_picker -- --nocapture` -- expected: pass
- `cargo test instance:: -- --nocapture` -- expected: pass
- `cargo fmt --check -- src/ui/instance_picker.rs` -- expected: clean (或 `cargo fmt` 后再查)
