# 切换器 workspace 列表（含远端未认领项）完整规格

## 数据源

切换器一帧的行只来自三处，按顺序合并：

1. `WorkspaceStore` views：本 GUI 已打开（open）或已保存的 workspace，本地与远端均含。
2. `host_snapshots`：每次 fresh connect 时的远端全量快照（`finish_connect` 写入），离线时仍展示。
3. Machine mirror 派生的未认领行：
   - 本地：`unclaimed_local_workspaces`（mirror 有、store 无 view），既有行为，保持不变。
   - 远端（本 change 新增）：对每个已建立 mirror 的远端 host，mirror 中存在、store 无对应 view 的 workspace，生成 adopt 行并入该机器分组。

## 远端未认领行的形态

- 与快照合并行一致：`RemoteWorkspaceRow { id, name, panes, last_active }` + `adopt` 标记，`remote_id` 为远端 workspace id。
- 点击走现有 `open_remote_workspace` adopt 流程；打开成功后该 view 进入 store，后续帧不再出 adopt 行（去重规则见下）。
- tab 列沿用 adopt 行既有占位（本机无 tab 详情可挂）。

## 实时更新

- `WorkspaceCreated`：mirror 插入，下一帧切换器出现该行（A1）。
- `WorkspaceRenamed`：mirror 更新，行名同步（A3）。
- `WorkspaceDeleted`：mirror 移除，行消失；若该 workspace 正被 store view 打开，走既有删除处理，不经快照复活（A4）。
- 无需重连、无需手动刷新；mirror 不存在（从未 pull 成功）时不产生行，离线展示仍由 snapshot 行承担。

## 去重规则

同一 workspace id 在 store views、snapshot 行、mirror 派生行中只出现一次，优先级：store view > snapshot 行 > mirror 派生行。store view 出现后，对应的 snapshot/派生行不再展示。

## 非目标（明确不做）

- 不新增 daemon 请求、事件类型与 Control 协议字段。
- 不展示远端 orphan pane；不改文件树、搜索文件等其他面板。
- 不改变离线快照的写入与清理时机。
