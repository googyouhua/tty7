# Base agent opencode2
Complete target behavior after Archive.

## Base list
- `CLIAgent` 新增 `OpenCode2` 变体，位于枚举末尾（discriminant 稳定）；`ALL` 中紧随 `OpenCode`（Base 下拉是不可滚动的普通 popover，排末尾会点不到）。
- `slug()` = `opencode2`；`aliases()` = `["opencode2"]`；`binary()` = `opencode2`。
- `display_name()` = `OpenCode2`。
- `accent_rgb()` 与 OpenCode 相同（`0x6E56CF`）；`icon_rgb()` = `0xFFFFFF`；`icon_path()` = `icons/agents/opencode2.svg`（与 `opencode.svg` 同字节，遵循 `icons/agents/{slug}.svg` 约定）。

## Launch / resume / fork
- 空 launch 的 role：`role_launch_program` / `role_launch_argv` 首词为 `opencode2`（opencode 仍为 `opencode`）。
- `resume_command(session)` = `opencode2{flags} --session {id}`；`fork_command` = `opencode2{flags} --session {id} --fork`。
- `fork_label()` = `Some("Fork Session")`。
- `session_free_tail` stale 与 OpenCode 相同：`--session/-s/--continue/-c/--fork`（含 `=` 形式）。
- `launch_argv_for_default`、`detect_from_argv`（别名 `opencode2`）、`launch_command` overrides（key `opencode2`）与 OpenCode 同规则。

## Hooks / status / resume bookkeeping
- `HookAgent::of_detected(OpenCode2)` = `Some(HookAgent::OpenCode)`；安装目标、事件桥、状态语义与 OpenCode 完全一致。
- daemon 检测、quick launch `installed_on`、frecency（key `opencode2`）、`agent_launch` overrides（key `opencode2`）与 OpenCode 同规则。

## Composer parity
- 以下判断中 `OpenCode2` 与 `OpenCode` 同分支：`!` shell 前缀、bracketed paste、图片粘贴为附件、`input_top`（`opencode_input_top`）、`held_lines`、mention 查找、submit 计划。
- 实现方式：所有 `matches!(…, OpenCode)` 改为 `OpenCode | OpenCode2`（或等价 helper）。

## Roles / models
- `model_flag(OpenCode2)` = `--model`；`builtin_models(OpenCode2)` 与 OpenCode 相同。
- `fetch_models(OpenCode2)` = `fetch_opencode_models()`（与 OpenCode 同源：`opencode2` 优先、`opencode` 兜底）。
- `model_choices_fast` / `model_choices` 对两者同规则。
- Role 表单：`seed`、`set_role_base`、`spawn_live_models` 的 `matches!(OpenCode)` 改为包含 `OpenCode2`。
- Base 下拉与 model 下拉一致，使用可搜索可滚动的 `settings_search_dropdown`（普通 popover 不可滚动，27 项点不到底部）。
- `role_to_json` / `load_roles`：`base` 读写 slug `opencode2`。

## History
- 两者共享同一 DB（`opencode*.db` + `$OPENCODE_DB`）；同一会话在 opencode 与 opencode2 下都展示（`session_key` 按各自 slug 区分）。

## Docs / mobile
- `mobile/src/icons.ts`、`docs/index.mdx` 增加 `opencode2` 条目（与 opencode 同名色）。
- `docs/agents/*` 中 agent 列表提及 opencode 处补充 opencode2（二进制列分别为 `opencode` / `opencode2`）。
