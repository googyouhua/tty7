# Outcome

Agent Role 表单的模型下拉 refresh 能自动同步 opencode 支持的模型：同时支持 `opencode2`（v2）与 `opencode`（v1）两种二进制名；失败时保留旧表并提示原因；全程不冻结 UI。

# Scope

- 覆盖 `crates/tty7-core/src/core/agent_roles.rs` 的 opencode 模型获取：二进制查找（PATH + 已知安装位）、`models` 子进程调用（8s 超时）、输出解析。
- 覆盖 `src/ui/settings/agent_roles.rs` 的 Role 表单：打开、切换 base、点击 refresh 三条路径改为 fast 值先填 + 后台 live 回填；失败保留旧表 + 红字原因；加载态。
- 用户已确认的三决策（见 Decisions）全部在范围内。

# Non-goals

- 不改 `models.json` 覆盖机制与内置兜底表（仍为最后回退）。
- 不直连 models.dev HTTP（`opencode models` CLI 已含 `opencode-go/*` 自定义源，是官方完整来源）。
- 不做定时后台轮询刷新（只在打开/切 base/点 refresh 时拉取）。
- 不碰其他 agent（Claude catalog 等）逻辑。

# Acceptance examples

- A1：装有 `opencode2` 或 `opencode` 任一（本机均为 v2.0.18）时，base=opencode 的表单 refresh 后下拉出现完整列表（含 `opencode-go/*`），不再只有 2 条内置。
- A2：CLI 缺失/执行失败/8s 超时/返回空列表时，下拉保留旧列表，页面出现红字失败原因（如 ``no `opencode2` or `opencode` binary found on PATH``）。
- A3：打开表单、切换 base、点击 refresh 时 UI 不冻结；刷新中有加载态（按钮 `…`）；快速连点/切换时旧慢请求不覆盖新结果（代际号）。
- A4：`cargo test -p tty7-core agent_roles` 全过（含新增 v2 优先、解析、真机 live 用例）；`cargo check --bins` 通过。

# Constraints and invariants

- 不新增第三方依赖（超时用 std `try_wait` 轮询实现）。
- `model_choices` 同步语义保留（供非 UI 调用方），UI 路径不再同步阻塞。
- 空列表视为失败（按 A2 处理），不清空下拉。
- Rust 2024：不用 `gen` 作标识符。

# Decisions

- 模型来源：继续用本地 `opencode models` CLI 并做 robust（用户确认，含关键信息“v2 叫 opencode2，两个都要支持”）。
- 失败表现：保留旧表 + 报错（用户确认）。
- 卡顿范围：改后台异步 + 超时（用户确认）。
- 二进制优先级：`opencode2` 先于 `opencode`；查找先 PATH（可执行校验）后 `~/.opencode/bin`、`~/.local/bin`、`/usr/local/bin`、`/opt/homebrew/bin`。
- 超时 8s；输出按行 trim 非空即 `provider/model`。

# Open questions

（无；Shape 已由用户确认进入 Build。）

# Verification expectations

- `cargo test -p tty7-core agent_roles` 全过。
- `cargo check --bins` 通过。
- 真机：有 opencode CLI 时 A1 可复现；无 CLI/杀 CLI 时 A2 文案可复现。
- 独立只读 Verifier 按 A1–A4 逐项判定。
