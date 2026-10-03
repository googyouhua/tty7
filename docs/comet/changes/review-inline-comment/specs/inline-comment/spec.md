# Inline review comment — 完整目标规格

## 能力
diff 行选区右键评论，一键发新建 agent 会话。

## 入口
- `diff_overlay` 行右键菜单（`copy_menu` 处）在有 `DiffSelection` 时追加 “Review selection…” 项；无选区不出现。

## 评论框
- 复用 overlay 内联 `review_box`（`InputState`）：打开、聚焦、回车/按钮确认即发送。单行；多行后续。

## 发送
- 内容：`build_review_attach_prompt(path, lines, diff, comment, source)`（路径 + 行号区间 + 选中 diff 代码块 + 用户评论 + 来源）。
- 目标：恒新建 agent 会话（`NewAgentTab`/agent_launch 通道），prompt 直接提交；不复用、不打断已运行会话；无运行中 agent 也不提示阻断。
- 记录：`ReviewSend` 照记；本地草稿保留（发送不删草稿）。

## 草稿重编与发送
- Review Tab 每条草稿行可展开编辑器（预填原正文）：保存更新正文；发送用（文件路径 + 行号 + 存档 diff + 编辑后正文 + 来源）走新建 agent 流程（与 overlay 一致：30s 就绪等待、超时通知、记发送、不删草稿）。
- 存草稿成功（overlay 保存按钮）后自动关闭评论框；取消/Esc 原有关闭不变。

## 草稿跳转
- 草稿存其仓库（host+root）与来源；点击草稿行（新增 Jump 按钮）在同来源 overlay 以该文件为 focus 打开：可 probe 来源（Range/Commit/工作区）重读进入；Patch 来源用存档的单文件快照经 `open_supplied_diff` 打开（存档草稿创建时的单文件 FileDiff）。
- 跳转不改草稿内容。

## 非目标
- 远端发表、多行编辑、行内线程展示、Review Tab 改动。

## 验收映射
- B1 → 入口显隐；B2 → 新会话 prompt；B3 → 旧会话无扰；B4 → 零 agent 可用；B5 → 重编发送 + 存后关框；B6 → 跳转。
