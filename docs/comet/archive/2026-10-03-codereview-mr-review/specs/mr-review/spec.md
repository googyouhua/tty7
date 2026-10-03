# MR Review — 完整目标规格

## 能力
MR 级 diff review：文件树 + 连续 diff 浏览、行内本地草稿评论、本地 base...head 离线 review、选中内容 attach 到 agent。

## 入口
- 远端：GitHub Tab → PR 详情 → “Review” 进入 MR 级视图。数据源为 `Detail{files: Vec<PrFile{patch}>}`（`core/github/api.rs:detail`），经 `DiffSource::Patch{id: owner/repo#n}` 复用行渲染。
- 本地：工作区选择 `base...head`（`DiffSource::Range{base,head}`）进入同一视图，无需网络。

## 视图（A1/A1b/A3）

- 无独立 MR 视图：沿用 `diff_overlay`（PR `Patch` 全量快照 + focus；本地 `Range{base,head}` 同一 overlay）。
- 新增 Review Tab（与 Changes/GitHub 并列，`RightPanelTab` 新变体）：base/head 双下拉 → Range overlay；草稿列表；发送记录。
- 分支列表：`git branch -a --format=%(refname:short)` 经 HostOps 按 repo 缓存（TTL 10s，同 github_branch），短名去重（`origin/x` 与本地 `x` 并存时都列出，`origin/HEAD` 剔除）。
- 默认值：head=当前分支（拿不到则 HEAD），base=upstream → 有本地则 main/master → origin/main（origin/HEAD 指向的远端默认分支）→ 第一个远端分支。
- 点击 Review 时现取 pane 目录（已实现）；所选 base/head 存 per-repo（host+root key），切换仓库回来保留上次选择。
- 非法选择（空/相同/非法字符）按钮置灰；probe 层 `revs_are_arguments` 已有最终兜底。
- 左侧文件树：按路径分组，显示每文件 +add/−del 与状态；点击定位到右侧对应文件块。
- 右侧连续 diff：多文件连续渲染，每文件可折叠/展开；文件数 >10 时默认只展开前 8（沿用 `FILES_SHOWN_FOLDED` 语义）。
- unified/split 切换复用 `DiffViewMode`，与现有单文件 diff 一致。
- 头部沿用只读信息：`head→base`、总数 +add/−del、commits、readiness、checks、reviewers。

## 行内草稿评论（A2）
- 点击任意 diff 行打开行内评论框；输入后保存为本地草稿，行旁显示标记。
- 草稿存本地（随 change 分支/工作区持久，至少 session 级保留），切换文件/刷新/重进视图后仍在，并标注“本地草稿、未提交”。
- 首版不 POST 远端；与远端 comment 线程解耦。

## 本地离线（A4/A5）
- `Range{base,head}` 进入同一视图；无 token/匿名时远端部分降级提示，本地 diff + 草稿 + attach 仍可用。
- 远端刷新失败不丢弃已展示详情（沿用现有 error 保留策略）。

## Attach to agent（A6）
- 在 review 视图中复用 `DiffSelection` 行选区；选中后操作 “Attach to agent” 打开发送框，用户可附评论。
- 发送内容拼成 prompt（`core/agent_prompt.rs` 新增 `build_review_attach_prompt`，`capped 24KB`）：
  - 文件路径 + 新旧行号区间 + 选中 diff 代码块 + 用户评论 + 来源（PR 号或 base...head）。
- 投递复用 `agent_target_leaf` + `deliver_agent_prompt` → `send_agent_prompt`；无运行中 agent 时按 `AppNoRunningCodingAgent` 提示并停留，不自动开 pane。
- 发送后保留草稿评论（attach 不删除草稿）。

## 非目标
- 不 POST GitHub review/review comments；不做 Approve/Request changes。
- Gitee 不同做，仅保证 `RepoSlug`/`Detail`/`PrFile` 抽象可复用。

## 验收映射
- A1 → 视图；A1b → 双下拉+文件列表；A1c → 非法防护；A1d → 列表 probe+快照复用；A1e → dock 高度；A2 → 草稿；A3 → 视图切换；A4 → 本地离线；A5 → 降级；A6 → attach。
