# Review Send Role Select — 完整目标规格

## 能力
代码 review 的“Send to new agent”在发送时可选择已添加的 Agent 角色，将 review 选中的内容发给指定的角色处理。

## 入口
- Diff overlay 评论框：`src/ui/diff_overlay.rs` 评论条中的 `Send to new agent` 按钮（`review_to_new=true` 路径，经 `review_send_to_new_agent` → `deliver_review_to_new_agent`）。
- Review Tab 草稿编辑器：`src/ui/panel_review.rs:221` 的 `Send to new agent` 按钮（经 `review_send_edit` → `deliver_review_to_new_agent`）。
- 两个入口共用同一选择与投递实现，行为一致。

## 选择菜单（A1/A2）
- 点击“Send to new agent”时不再直接发送，而是弹出一次选择菜单（`dropdown_menu` / `dropdown_menu_with_anchor`，与 New Tab 菜单和 branch 下拉相同的点击时构建语义）。
- 菜单内容在打开时现取：
  - 已添加角色：`offered_roles_here(cx)`（本地按 PATH 过滤，远端全量），按 `agent_frecency[role:<slug>]` frecency 排序，行显示角色名 + `role_launch_line`。
  - 可用 agent：`offered_agents(cx)` 按现有 frecency 排序，行显示 agent 名 + launch command。
- 默认高亮：上次 review 发送成功记住的目标（agent slug 或 `role:<slug>` 的 frecency `last_used` 最大者）；无历史时为现有 `most_recent` 结果。
- 菜单取消 = 不发送，不关评论框/不结束编辑，不记录 send。

## 投递（A3）
- 目标为 agent：保持现状 — `launch_agent(agent, NewTab)`，等待新 tab 出现 agent（30s）+ settle（5s / Waiting|Idle|Done 即发），`send_agent_prompt(prompt)`，`review.record_send{path, lines, comment_len}`。
- 目标为角色：`launch_role(role, NewTab)` 启动；同样等待新 tab 的 agent 出现（30s）+ settle（5s），然后发送 `build_review_attach_prompt(path, lines, diff, comment, source_label)` 生成的 review prompt，并同样 `record_send`。
- 角色自带 instructions 不丢失：
  - `PromptArg`：launch 命令行已含 instructions，review prompt 作为其后的独立一次 `send_agent_prompt`（与 TwoPhase 的 followup 语义一致，不合并进同一命令行）。
  - `TwoPhase`：`launch_role` 的 followup（instructions 粘贴）先行，review prompt 在其后、agent settle 后再发送；实现上复用同一等待循环，顺序为 launch → role followup → review prompt。
- prompt 内容不变：文件路径 + 行号区间 + 选中 diff 代码块 + 用户评论 + 来源 label（`capped 24KB` 截断规则不变）。
- 运行中的 agent 永不被触碰；只向新开 tab 投递。

## 回退（A4）
- `offered_roles_here` 为空：菜单仅列 agent；行为与现状一致（默认 `most_recent`）。
- agent 与 role 皆无：沿用现有桌面通知（本地 `AppNoAgentOnPath` / 远端 `AppNoAgentSeenHere`），返回 `Err("no-agent-offered")`，不打开新 tab，不记录 send。
- 选中 diff 为空：沿用 `Err("no-selection")`，不弹菜单。

## 记忆（A5）
- 发送成功（投递任务已排入新 tab，不等待 30s 结果）即更新 frecency：agent 写 `agent_frecency[slug].last_used=now`（沿用 `launch_agent` 内已有更新），角色写 `agent_frecency[role:<slug>].last_used=now`（沿用 `launch_role` 内已有更新）。
- 下次菜单打开时按更新后的 frecency 排序并高亮上次目标。

## 非目标
- “Attach to agent”不动。
- 角色的新增/编辑/删除、`plan_role_first_send`、starter 发送不动。
- 无多选/批量发送；无默认发送角色的全局设置项。

## 验收映射
- A1 → overlay 菜单；A2 → Review Tab 菜单；A3 → 指定角色新 tab 启动 + review prompt 投递 + instructions 顺序；A4 → 无角色回退与空提示；A5 → frecency 记忆与默认高亮。
