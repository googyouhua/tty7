# Outcome
代码 review 时“Send to new agent”支持选择已添加的 Agent 角色，将 review 选中的内容（文件路径 + 行号区间 + 选中 diff + 用户评论）发给指定的角色处理，在新 tab 中启动该角色并投递 review prompt。

# Scope
- 现状已查明（`src/ui/review_actions.rs:348` `deliver_review_to_new_agent`）：overlay 选中评论框（`src/ui/diff_overlay.rs:586`）与 Review Tab 草稿编辑器（`src/ui/panel_review.rs:221`）的“Send to new agent”都走同一投递函数，当前只按 `most_recent(offered_agents, agent_frecency)` 选默认 agent 并 `launch_agent(NewTab)`，30s 等待 + 5s settle 后 `send_agent_prompt`。
- 角色体系已查明（`src/ui/agent_launch.rs:80` `offered_roles` / `385` `offered_roles_here` / `418` `launch_role`，`src/core/agent_roles`）：已添加角色按 PATH 可用性 + `agent_frecency[role:<slug>]` 排序；启动分 `PromptArg`（指令拼 launch 命令行）与 `TwoPhase`（先 launch 行、后贴 instructions）两种。
- 本次新增：
  - 两个入口统一改为“点击 Send 弹出菜单选择目标”：菜单列出 `offered_roles_here` 已添加角色 + 现有 offered agents（默认 agent 项），按 frecency 排序，上次 review 选择默认高亮。
  - 选角色 → `launch_role(NewTab)` 启动该角色，复用相同的 30s/5s 等待后投递 `build_review_attach_prompt(path, lines, diff, comment, source_label)` 生成的 review prompt；选 agent → 保持现有 `launch_agent` 路径。
  - 记住上次选择：发送成功后更新对应 frecency（agent 沿用现有 `agent_frecency[slug].last_used`，角色沿用 `role_frecency_key`），下次菜单默认高亮该项。
  - 无角色时回退：`offered_roles_here` 为空时菜单仅列 agent，行为与现状一致；无任何 agent/role 时沿用现有桌面通知（`AppNoAgentOnPath` / `AppNoAgentSeenHere`）并返回 `no-agent-offered`。

# Non-goals
- 不改“Attach to agent”（发给运行中 agent）的行为。
- 不改角色本身的新增/编辑/删除与启动指令规划（`plan_role_first_send` 原样复用）。
- 不做 review 内容发给多个角色/批量发送；一次只选一个目标。
- 不改 Review 草稿的保存/编辑/删除逻辑。

# Acceptance examples
- A1（overlay 菜单）：在 diff overlay 选中 diff 行块打开评论框，点“Send to new agent”弹出菜单，列出已添加角色与可用 agent。
- A2（Review Tab 菜单）：在 Review Tab 草稿点 Edit 再点“Send to new agent”，同样弹出角色/agent 选择菜单。
- A3（发给指定角色）：选中任一角色后，新 tab 启动该角色的 launch line，待其 ready 后 review prompt（含 path + lines + diff + comment + source）出现在该 pane 并被记录为一次 send；角色自带 instructions 不丢失（PromptArg 合并 / TwoPhase 先后顺序正确）。
- A4（无角色回退）：未添加任何角色时，菜单仅列 agent 或直接走现有 most_recent 路径，发送成功；无 agent 也无 role 时给出已有提示且不崩溃。
- A5（记住选择）：发送成功后下次打开菜单时上次所选角色/agent 为默认高亮项。

# Constraints and invariants
- 复用 `build_review_attach_prompt` 拼 prompt（含 path/lines/diff/comment/source，`capped 24KB` 规则不变）。
- 复用 `launch_agent` / `launch_role` 机械启动路径与 `agent_target` 等待/投递通道，不新增 agent 通道；运行中的 agent 永不被触碰。
- 菜单在点击时现取 `offered_roles_here` + `offered_agents`（render 时不缓存），与现有 New Tab 菜单/branch 下拉保持一致的现取语义。
- 只提交本 change 的实现与正式工件，不动用户其他改动。

# Decisions
- 工作区：独立 worktree（`comet/review-send-role-select`，target `main`）。
- Q1 已确认（2026-10-04）：两个入口都改（overlay 评论框 + Review Tab 草稿），共用 `deliver_review_to_new_agent`，行为一致。
- Q2 已确认（2026-10-04）：点击弹出菜单选（列出已添加角色 + 默认 agent），对现有按钮改动最小，无角色时直接回退。
- Q3 已确认（2026-10-04）：记住上次选择，用现有 `agent_frecency`（agent key 与 `role:<slug>` key）实现，下次默认高亮。

# Open questions
- 无未决问题；待最终确认后进入 Build。

# Verification expectations
- 按 A1–A5 验证：两个入口菜单列出角色、选中角色新 tab 启动并收到 review prompt（含 instructions 顺序正确）、无角色回退与空提示、frecency 记住上次选择；`cargo test` 相关单测通过。
