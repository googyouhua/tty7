# Walkthrough — ws-unreachable UI freeze fix

Target: commit `1d660469` on branch `fix/ws-unreachable-ui-freeze` (fix: native SSH pane restore off UI thread)
Plan: [plan-ws-unreachable-ui-freeze.md](../plan-ws-unreachable-ui-freeze.md)
Initiative: `initiative-terminal-blocks`

Intent (from plan): 打开不可达 ws（远端服务器 SSH 失败）后当前界面卡死：无法切换 ws、无法打开配置，点击任意位置无响应。Approach: 连接失败落到非模态 Failed 状态并保留切换/配置入口，不让全屏 occlude 遮罩在无可操作内容时拦截全部点击，连接保持 off-UI-thread。

## Block 1 — Intent (pending)

Status: pending.

## Block 2 — Broad strokes (pending)

Status: pending.

## Block 3 — Slices (pending)

Status: pending.

Key files (links relative to this narrative):

- [app.rs](../../../src/ui/app.rs)
- [pending_pane.rs](../../../src/ui/pending_pane.rs)
- [view.rs](../../../src/terminal/view.rs)

## Block 4 — Periphery (pending)

Status: pending.
