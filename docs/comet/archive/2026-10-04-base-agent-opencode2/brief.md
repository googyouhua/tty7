# Outcome
agents（自定义 Agent Role 表单）的 Base agent 增加 `opencode2` 选项；除启动二进制不同外，其他行为与 `opencode` 一致：`opencode2` 启动 `opencode2`，`opencode` 启动 `opencode`。

# Scope
- Base agent 下拉（`CLIAgent::ALL`）新增 `opencode2`：slug `opencode2`，二进制 `opencode2`，检测别名 `opencode2`。
- 启动 / 恢复 / fork 命令：`opencode2 --session <id>`、`opencode2 --session <id> --fork`，flags 回放与 `opencode` 同规则（`--session/-s/--continue/-c/--fork` 为 stale）。
- 状态检测、quick launch、role launch / resume、hooks（复用 OpenCode 插件）、composer 输入区 / 粘贴 / `@` 行为、图标与颜色、模型下拉（live + models.json + 内置）与 `opencode` 一致。
- `role.json` 的 `base: "opencode2"` 可加载保存；已有 `base: "opencode"` 不变。

# Non-goals
- 不改 `opencode` 现有行为（仍启动 `opencode`）。
- 不新增独立图标设计；`opencode2.svg` 与 `opencode.svg` 同字节（仅满足 `icons/agents/{slug}.svg` 约定）。
- 不改 hooks 插件目标路径（仍为 `opencode/plugins/tty7.js` 兼容 1.x/2.x）。

# Acceptance examples
- A1: Role 表单 Base 下拉同时出现 `OpenCode` 与 `OpenCode2`，选择 `opencode2` 可保存并回显。
- A2: `base=opencode2` 且 launch 为空时实际启动 `opencode2`；`base=opencode` 仍启动 `opencode`（`role_launch_argv` / `role_launch_program` 级别可验证）。
- A3: `opencode2 --session s-1` 被检测为 opencode2；resume/fork 生成 `opencode2 --session <id>` / `--fork` 形式。
- A4: opencode2 与 opencode 共享 composer 输入区定位、粘贴、hooks 映射与模型列表行为（单测覆盖）。

# Constraints and invariants
- `CLIAgent` 新增变体必须放末尾（daemon 消息序列化 discriminant 不可移动）。
- `CLIAgent::ALL` 长度 +1；`every_agent_icon_resolves`、`every_agent_has_a_bindable_launch_action` 必须通过。
- 不破坏现有 `opencode` 的检测、resume、fork 单测。

# Decisions
- 新增 `CLIAgent::OpenCode2` 变体（slug `opencode2`，aliases `["opencode2"]`），binary 自然为 `opencode2`。原因：只有独立变体才能在 Base 下拉中二选一且各自启动各自二进制。
- Hooks 复用 `HookAgent::OpenCode`（`of_detected(OpenCode2) → OpenCode`）。原因：同一插件同时兼容 1.x/2.x，无需第二安装目标。
- 图标复用 `icons/agents/opencode.svg`，accent 与 OpenCode 相同。原因：用户要求“其他都一样”，独立图标不在本次范围。
- Q1（已确认）：历史共享展示——同一 DB 会话在 opencode 与 opencode2 下都可见（各自 `session_key` 区分，All 视图可能重复，接受）。
- Q2（已确认）：显示名 `OpenCode2`（与二进制同名）。
- Q3（已确认）：模型 live 获取共享优先级——两者都按 `opencode2` 优先、`opencode` 兜底（现有逻辑不变）。

# Open questions
None. Shape confirmed by user.

# Verification expectations
- `cargo test -p tty7-core cli_agent agent_roles agent_history` 通过；新增 opencode2 单测通过。
- `cargo test -p tty7 agent_launch composer assets` 通过（如有相关用例）。
- 手动：Role 表单切换 base 为 opencode2，launch 为空保存后启动验证为 `opencode2`。
