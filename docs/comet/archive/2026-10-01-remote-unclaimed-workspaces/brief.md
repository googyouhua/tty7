# 目标

CLI 在远端 daemon 上创建的 workspace（含 tab/pane），GUI 切换器在对应机器分组下实时可见、可打开，与本地 unclaimed workspaces 行为一致。不再需要重连才能看到。

# 范围

- 切换器面板（随处搜索 workspace 面与切换器同源 `switcher_groups`，改一处即覆盖两处）。
- Remote 分组追加 machine mirror 中存在、但本 GUI 无 view 的 workspace 行，走现有 adopt/open 流程（`Group::merge` + `RemoteWorkspaceRow` 语义）。
- 实时性：`WorkspaceCreated` / `WorkspaceRenamed` / `WorkspaceDeleted` delta 经 mirror 实时反映在行上，无需手动刷新或重连。

# 非目标

- Orphan pane、远端文件树、代理/跳转等其他面。
- 改 daemon 协议与事件推送（`WorkspaceCreated` 广播已完备，`tty7 events` 可观测）。
- 离线机器的快照（snapshot）逻辑不动；mirror 不存在时不虚构行。

# 验收示例

- A1：CLI 在已连接远端新建 workspace → 切换器该机器分组出现同名行，无需重连或手动刷新。
- A2：点击该行 → 走现有 adopt/open 流程正常打开，tab/pane 与远端一致且可交互。
- A3：CLI 重命名远端 workspace → 切换器行名同步更新。
- A4：CLI 删除远端 workspace → 切换器对应行消失，不残留 adopt 行。
- A5：回归：本地 unclaimed 行、已打开/已保存的远端行、离线快照行行为不变；相关 `switcher` / `machine_mirror` 单测全过。

# 约束与不变量

- 只读 `MachineMirrors` + `WorkspaceStore` views 派生行；不改 daemon，不改 Control 协议，不加新事件。
- 行去重：store 已有 view 的 workspace 不重复出 adopt 行（沿用 `listed` / `known` 逻辑）。
- 远端行打开必须走现有 `open_remote_workspace` adopt 流程，不另起控制路径。

# 决策

- D1 数据源：复用 `MachineMirrors`（与本地 `unclaimed_local_workspaces` 同源）；remote group 追加 mirror 有、store 无 view 的行。
- D2 行形态：复用 `RemoteWorkspaceRow` + adopt 语义，走 `Group::merge` 既有路径（灰态、无 tab 列，占位说明与既有 adopt 行一致）。
- D3 触发：live delta 驱动 mirror，切换器每帧读取；mirror 不存在时不展示，离线沿用 snapshot 行。
- D4 范围：仅切换器面板；随处搜索同源自动覆盖，不单独立项。

# 待解决问题

（无；2026-10-01 用户已确认目标、范围、验收 A1–A5 与非目标。）

# 验证预期

- 单测：`switcher` / `machine_mirror` 单元测试覆盖出现（A1）、改名（A3）、删除（A4），参考既有 `a_deleted_remote_workspace_leaves_the_switcher` 用例风格。
- 联调：以本地 daemon 驱动的单测/逻辑验证为主；真远端 GUI 目检（用户已连接的 GUI 上确认 A1–A4）记为用户验收。
