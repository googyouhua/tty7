# Outcome
在 tty7 内实现类似 GitHub/Gitee 查看 MR 的 diff review 功能：在现有 GitHub PR 单文件 diff 展示之上（已验证可用），保留现有 GitHub PR 视图不动；新增与 Changes/GitHub 并列的 Review Tab（base/head 入口 + 草稿列表 + 发送记录），Range 连续 diff 覆盖 Gitee 等拉不到 MR 的场景；在 `diff_overlay` 内做行内本地草稿评论，并支持选中内容 attach 到 agent。

# Scope
- 现状已实现（用户已验证）：GitHub PR 详情拉取远端数据并可看到单文件 diff（`Detail{files: Vec<PrFile{patch}>}` → `DiffSource::Patch` → `diff_overlay` 单文件打开）。
  - 直连方式：直接接入 `https://api.github.com` REST（GUI 本机 `HttpTransport`，非 server 代发；token 本机解析）。
  - 现有 agent 投递能力（复用，不另起通道）：`attach_path_to_agent`（`@path` 粘贴，`file_tree.rs:1649`）、`send_selection_to_agent`（终端选中 → `build_selection_prompt` → `deliver_agent_prompt`，`app.rs:7007`）、`send_git_diff_to_agent`（工作区 diff → `build_diff_review_prompt`，`app.rs:7028`）。
- 本次新增：
  - A1 连续 diff（无独立 MR 视图）：复用 `diff_overlay` 多文件快照（PR 的 `Patch` 全量快照 + focus；本地 `Range{base,head}` 同一 overlay），文件可折叠/展开，unified/split 复用 `DiffViewMode`。为 Gitee 等无 MR 接口场景提供本地 Range 入口。
  - A1b Review Tab（与 Changes/GitHub 并列）：base/head 双下拉（`git branch -a` 真实列表，默认 head=当前分支、base=upstream 或 main/master/origin 对应），下拉下方直接列出两分支差异文件（状态徽标 + 文件名 + 目录 + +/-，与 GitHub PR 文件列表同风格，>10 默认折叠），点击文件即以该文件为 focus 打开 Range overlay。硬编码预设已删除：clone 无本地 main 时预设会 fatal（实测 `Needed a single revision` → DiffReadFailed）。
  - A1d 文件列表数据：随 base/head 选择自动 probe（TTL 缓存，per repo+base+head key），失败显示原因并保留旧列表；点文件复用列表快照打开（同文件必同内容，无二次 probe）。
  - A1e dock 高度：dock 列在真窗口中 flex 链不定高导致虚拟列表零高空白（最大化复现，小窗口 fill 正常，无头单测只能数 builder 调用故无法复现）；修法为 dock 表面视口定高 + 列表纯 flex，macOS 走原路径。切文件/快照变化回顶（同文件重复点击不再残留旧滚动）。
  - A1c 无效 ref 防护：base/head 为空、相同或非法（`-` 开头/控制字符）时按钮置灰并提示，不发 probe。
  - A2 行内本地草稿评论：点击 diff 行可写草稿，存本地、标注“未提交”，刷新/切换保留。
  - A3 本地离线 review：`git diff base...head` 进入同一视图，任何托管可用、无需网络。
  - A4 只读信息沿用：分支、+/-统计、checks、reviewers、comments 时间线。
  - A6 Attach to agent（新增）：在 review 视图中选中 diff 内容 + 附上用户评论，一起发给 agent 处理（复用 `deliver_agent_prompt`/`send_agent_prompt` 通道，prompt 含文件路径 + 行号 + 选中 diff + 用户评论）。

# Non-goals
- 首版不做远端写提交（不 POST review / review comments，不做 Approve/Request changes）。
- Gitee 首版不同做：仅预留统一 Forge 接口，二期再接。

# Acceptance examples
- A1: 现有 PR 详情 + 单文件弹窗保持可用；A1b: 双下拉 + 文件列表点进 diff；A1c: 非法置灰；A1d: 列表自动 probe + 快照复用；A1e: dock 最大化有内容。
- A2: 点击任意 diff 行可添加本地行内草稿评论，切换文件/刷新后仍在且标注未提交。
- A3: 支持 unified/split 切换，与现有单文件 diff 一致。
- A4: 本地 `base...head` 可进入同一 review 视图（离线可用）。
- A5: 无 token/匿名时远端只读部分降级提示，本地 diff 与草稿评论仍可用。
- A6: 选中 diff 行/块后点 “Attach to agent”，附上评论，可在运行中 agent 的 prompt 看到含路径+行号+diff+评论的内容；无运行中 agent 时明确提示。

# Constraints and invariants
- 复用 `diff_overlay`/`diff_rows`/`diff_list` 行渲染与 `DiffViewMode`，不另起渲染器。
- 复用 `agent_target_leaf` + `deliver_agent_prompt` 投递，不新增 agent 通道；超长按 `capped (24KB)` 截断规则。
- 远端刷新失败不丢弃已展示详情；草稿与远端线程解耦。
- token 缺失不阻塞本地能力。

# Decisions
- 工作区：独立 worktree（`comet/codereview-mr-review`）。
- Q1：GitHub 先行 + 本地对比，Gitee 预留二期。
- Q2 已确认：MR 级连续视图。
- Q3 已确认：仅本地草稿，不走远端提交。
- Q4 已确认：包含本地离线 review。
- Q5 已确认：行选区（复用 `DiffSelection`，带路径+行号）。
- Q6 已确认：拼 prompt（路径+行号+选中diff+用户评论，`capped 24KB`）。
- Q7 已确认：无运行中 agent 时提示并停留（沿用 `AppNoRunningCodingAgent`）。

# Open questions
- Q5 已决： 选中粒度是沿用 diff 行选区（`DiffSelection` 行级，已支持拖选复制）还是整文件？推荐行选区 + 自动带文件路径与行号。
- Q6 已决： 发给 agent 的内容格式是拼成 prompt（diff 代码块 + 评论，类似 `build_selection_prompt`）还是 `@path#L` 引用？推荐拼 prompt（review 上下文必须离线带过去，`@` 引用 agent 侧读不到远端 patch）。
- Q7 已决： 无运行中 agent 时是提示并停留（沿用 `AppNoRunningCodingAgent`），还是自动开新 agent pane？推荐前者（不擅自开 pane）。
- CONFIRM 已确认（2026-10-02）：A1–A6 范围确认，进入 Build。

# Verification expectations
- 按 A1–A6 验证：连续浏览、草稿持久、unified/split、本地对比、匿名降级、attach 到 agent（含无 agent 提示）。
