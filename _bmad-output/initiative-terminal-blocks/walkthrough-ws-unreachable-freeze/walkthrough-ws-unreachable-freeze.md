# Walkthrough — ws-unreachable UI freeze fix

Target: commit `1d660469` on branch `fix/ws-unreachable-ui-freeze` (5 files, 199+/9-).
Plan: [plan-ws-unreachable-ui-freeze.md](../plan-ws-unreachable-ui-freeze.md) · Current block: Block 6 — Periphery.

## Block 1 — Intent (done)

意图原文（来自 plan 文件 `<frozen-after-approval>`，逐字粘贴）：

**Problem:** 打开不可达 ws（远端服务器 SSH 失败）后，当前界面卡死：无法切换 ws、无法打开配置，点击任意位置无响应。

**Approach:** 让连接失败落到非模态 Failed 状态并保留切换/配置入口，不让全屏 occlude 遮罩在无可操作内容时拦截全部点击，连接保持 off-UI-thread。

出处注：取自 [plan-ws-unreachable-ui-freeze.md](../plan-ws-unreachable-ui-freeze.md) 的 Intent 节；实现范围收敛为 native SSH 恢复路径（远端 workspace 分支本已异步）。

- [x] Block 1 — 用户确认意图无误即勾选

## Block 2 — Broad strokes (in progress)

按顺序打开这几处即可理解整套机制：

1. 恢复入口 park 住 pending：[session_to_pane native 分支](../../../src/ui/app.rs) —— 同步 `new_terminal_native` 改为立 `PendingPane::Connecting`。
2. 后台拨号与落态：[start_native_spawn / land_native_pane](../../../src/ui/app.rs) —— `background_executor` 拨号，失败 `fail`，成功建 view。
3. Pending 携带 spec：[PendingSpawn 新增字段](../../../src/ui/pending_pane.rs) —— `native_ssh` + `native_start_dir`。
4. 快照不存密钥：[pane_to_session Connecting 分支](../../../src/ui/app.rs) —— 存 `without_secrets()`。
5. 清理可见性：[NativeSshParts::pane_id](../../../src/terminal/view.rs) —— `pub(crate)` 供 slot 关闭后后台 kill。

- [x] Block 2 — 粗览无误即勾选

## Block 3 — Slice A：不阻塞 UI（pending + 后台拨号）(in progress)

机制：恢复时零同步拨号。`session_to_pane` 的 native 分支直接 park 为 Connecting 并 `start_native_spawn`；`start_pane_spawn` 遇 native 转交；Retry 订阅先在 UI 线程重解 profile/keychain（`resolve_persisted_ssh_spec`）并 `retrying()` 再打。拨号本身在 `background_executor`，UI 可切 ws / 开配置。

- [session_to_pane native park](../../../src/ui/app.rs) —— park + 订阅 Retry + 立即后台打一次
- [start_native_spawn](../../../src/ui/app.rs) —— 后台 `spawn_native_ssh_terminal`，经 `update_in` 落态
- [start_pane_spawn 转交](../../../src/ui/app.rs) —— 遇 `native_ssh.is_some()` 即转交，不走 workspace 拨号
- [PendingSpawn](../../../src/ui/pending_pane.rs) —— 新增两字段的定义

语义：单次制，无 auto-retry（`restore_pane: None`）；`Connecting` 态无取消键，关 tab/pane 或切 ws 即弃槽。

- [x] Slice A — 接受即勾选

## Block 4 — Slice B：落态正确（失败/成功/清理）(in progress)

机制：失败只 `fail(reason)` 留 `Failed` + Try Again（见 [PendingPane Failed 渲染](../../../src/ui/pending_pane.rs)）；成功建原生 view 并补齐 `ChildExited` / `AgentSessionChanged` / `AgentDetected` / `AuthPromptReady` 订阅、`agent_resume_command`、role followup；`pane_to_session` 存剥离版 spec；slot 已不在则后台 kill。

- [land_native_pane](../../../src/ui/app.rs) —— 失败/成功/清理三路
- [pane_to_session](../../../src/ui/app.rs) —— `without_secrets()` 存快照
- [PendingPane](../../../src/ui/pending_pane.rs) —— Failed + Try Again 语义

- [x] Slice B — 接受即勾选

## Block 5 — Slice C：构造点补齐与焦点 (in progress)

机制：同为一组机械改动。既有 `PendingSpawn` 构造点补 `None`；新 pending 照旧 `watch_pane_focus`，落态后 `focus_leaf` 恢复焦点。

- [测试构造点](../../../src/ui/app.rs) —— 单测内两处补 `None`
- [document_column 构造点](../../../src/ui/document_column.rs) —— harness 用例补 `None`
- [diff_overlay 构造点](../../../src/ui/diff_overlay.rs) —— harness 用例补 `None`

- [x] Slice C — 接受即勾选

## Block 6 — Periphery (in progress)

- [plan 文件](../plan-ws-unreachable-ui-freeze.md) —— 意图 + Implementation Notes + Review Triage Log（7 findings：2 patched、2 deferred、3 rejected/false）
- [deferred-work.md](../deferred-work.md) —— 全屏遮罩旧 trap；纯远端 ws 待复现确认
- [view.rs](../../../src/terminal/view.rs) —— 单行可见性小改

- [x] Periphery — 接受即勾选
