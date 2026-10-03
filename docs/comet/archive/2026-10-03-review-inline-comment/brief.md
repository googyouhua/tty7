# Outcome
diff 选中行右键输入评论，评论 + 选中内容发给新建的 agent 会话处理。

# Scope
- 触发：在 diff_overlay 行选区（`DiffSelection`，拖选行）上右键，菜单出现评论入口（复用现有 copy_menu 行菜单，加一项）。
- 输入：行内评论框（复用 `review_box` 的 `InputState` 形态；多行编辑为后续项，不在本期）。
- 发送：`build_review_attach_prompt` 拼装（路径 + 行号 + 选中 diff + 用户评论 + 来源）→ 开一个**新 agent 会话**承载并直接提交，不复用已运行 agent（覆盖旧 Q7 的“提示并停留”——仅本流程如此，旧的 Attach 按钮行为不变）。
- 数据沿用：`ReviewDraft`/`ReviewSend` 照记（发送记录保留，草稿保留）。

# Non-goals
- 不做远端发表（仍本地草稿 + 发 agent）。
- 不做多行编辑器、不做行内线程展示（后续 change）。
- 不动 Review Tab 文件列表与 dropdown。

# Acceptance examples
- B1: 拖选 diff 行后右键，菜单有评论入口；无选区时右键无此入口。
- B2: 输入评论确认后，新开一个 agent 会话，其 prompt 含路径 + 行号 + 选中 diff + 评论。
- B3: 已运行的 agent 会话不受打扰（不注入、不切换）。
- B4: 当前无运行中 agent 也可直接用（新建，不报错）。
- B5: Review Tab 草稿可重编正文并发送给新建 agent；存草稿成功后评论框自动关闭。
- B6: 点击草稿行跳转到对应文件的 diff 位置（同来源 overlay 聚焦该文件；远端 Patch 草稿用存档单文件快照打开）。

# Constraints and invariants
- 复用 `DiffSelection`/`build_review_attach_prompt`/`deliver` 通道族，不新增 agent 协议；超长按 `capped (24KB)` 截断。
- 新 agent 会话走现有 `NewAgentTab`/agent_launch 通道，不手造 pane。

# Decisions
- 落点：复用 `comet/codereview-mr-review` 基，新 change `review-inline-comment`（worktree 独立目录）。
- 右键菜单入口（Q2 已确认）；发送对象恒为新建 agent（Q2“新建”已确认，覆盖旧 Q7 仅本流程）。

# Open questions
- CONFIRM 待用户确认（B1–B4 范围已定）：确认后进入 Build。

# Verification expectations
- B1–B5：右键入口、发送、草稿重编、存后关框。
