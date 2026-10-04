# Outcome

Agent Role 表单的模型下拉不再依赖手写配置与本地 agent 二进制：表单内「文件 | URL」二选一加载（默认 `https://models.dev/api.json`，允许自定义 URL），结果进下拉并由 UI 写 per-base 缓存、重开仍在；models.dev 有收录的 base 按映射取、无收录的不发起 HTTP；失败保留旧表 + 红字原因且 UI 不冻结；最后兜底永远是 launch cmd 手写参数（launch 行已含 flag 时下拉不再追加）。

# Scope

- `crates/tty7-core/src/core/agent_roles.rs`：`fetch_models` 扩展到 26 个 base，新增 models.dev HTTP 源（HTTP 优先、本地 CLI/catalog 兜底）；映射表初始覆盖 `opencode`→`opencode,opencode-go`、`claude`→`anthropic`、`codex`→`openai`、`gemini`→`google`、`grok`→`xai`、`qwen`→`alibaba`、`kimi`→`moonshotai`、`copilot`→`github-copilot`，其余为空则跳过 HTTP。
- 加载双源（用户在表单二选一）：文件模式读用户指定路径的 JSON（格式复用 `{flag, models[]}`，与 `models.json` 条目同形，文件说明见新增文档）；URL 模式拉用户填写的地址（默认 models.dev，可改自建镜像/额外地址并按映射 provider 提取后合并）。
- 缓存：加载成功由 UI 写 per-base 缓存文件（config dir 下，打开/切 base/点加载即用），重开表单仍在；HTTP/文件失败时读缓存，再走 `models.json`/内置。
- `src/ui/settings/agent_roles.rs`：Role 表单加来源切换（文件 | URL）+ 路径/URL 文本输入（无原生文件对话框）+ 加载按钮；有映射或用户指定源的 base 起后台任务（沿用代际号防旧覆盖），无映射且无指定源的 base 无后台任务；失败保留旧表 + 红字原因；加载态语义不变。
- 新增文件说明文档 `docs/agents/models-load-file.md`：格式、字段、示例、校验规则（大小/条数上限、失败表现），表单给出指向该说明的提示。
- 兜底语义：`role_launch_argv` 改为 launch 行已含该 base 的 model flag（含 `flag=value` 形）时不再追加下拉值；下拉选空 + launch 手写即最终兜底；已保存但列表不再含的 model 仍保留可选。
- ID 形态：沿用现有格式——OpenCode 系保持 `provider/model`（models.dev/文件内裸 key 加 `opencode/` 前缀，已含 `/` 则原样），其余 base 用裸 model id；不改 `role.json` 的 `model` 语义。

# Non-goals

- 不做启动预拉取与定时轮询（仍只在打开/切 base/点加载时拉取）。
- 不改 `models.json` 读取机制本身（仍为最后回退之一），但用户不再需要手写它。
- 不改各 base 的 `model_flag` 表内容与 `role.json` 结构（`role.json` 不存加载源选择之外的多余字段）。
- 不为 models.dev 无 provider 的小众 base 捏造模型列表；不做原生文件对话框。

# Acceptance examples

- A1：URL 模式默认 models.dev、无本地 `opencode2`/`opencode` 但有网时，base=opencode 的表单加载后下拉出现 `opencode/*` 完整列表（含 `opencode-go` 源模型），不再只有 2 条内置。
- A2：有映射的 base（如 claude/codex/gemini）在无本地 catalog/CLI 时，下拉出现 models.dev 对应 provider 列表（如 anthropic/openai/google 的裸 id），数量>内置表。
- A3：无映射的 base（如 goose/droid）且未指定文件/URL 时行为不变：不发起 HTTP，有 models.json 则用之、无则内置/空，不报错不卡顿。
- A4：断网/8s 超时/URL 解析失败/文件缺失或非法时，下拉保留旧列表并显示红字失败原因，UI 不冻结；有 CLI 时回退本地 CLI 列表仍可用；缓存存在时离线可读缓存。
- A5：`cargo test -p tty7-core agent_roles` 全过（含新增映射、解析、缓存回退、去重单测）；`cargo check --bins` 通过；无新增重型依赖。
- A6：文件模式：按 `docs/agents/models-load-file.md` 手写一份 JSON，表单选「文件」填路径点加载，下拉出现文件内列表；重开表单仍在；非法文件报红字且保留旧表。
- A7：自定义 URL 与 launch 兜底：填自建镜像地址可加载；launch 行手写 `--model xxx` 时下拉不再追加（无重复传参，argv 只含一处 flag）；下拉选空 + launch 手写即兜底生效。

# Constraints and invariants

- 复用 `ureq`（`remote-install` feature 门控，GUI 已开）；feature 关闭时 HTTP 源自动禁用、走文件/本地/内置链路，保证 `cargo test -p tty7-core`（默认 feature）仍编译通过。
- 单次 HTTP 8s 超时；文件读取设上限（如 1MiB / models 2000 条，超限按失败处理）；空列表/解析失败视为失败，走回退链，不清空下拉。
- `model_choices` 同步语义保留；UI 路径走后台任务 + 代际号，不同步阻塞。
- 去重规则：launch 按空白分词后含 `flag` 或以 `flag=` 开头即视为已含，不再追加下拉值。
- Rust 2024：不用 `gen` 作标识符；错误信息英文、面向用户可读。
- models.dev 全量 JSON 只取映射 provider 的 `models` keys，不过度解析；未知字段忽略，保证结构变化时不 panic。

# Decisions

- 覆盖范围：26 个 base 尽量覆盖，models.dev 有则取、无则算了（用户确认 Q1）。
- 来源优先级：HTTP 优先、本地 CLI/catalog 兜底（用户确认 Q2）。
- 拉取策略：按需后台 + 缓存文件，离线读缓存（用户确认 Q3）。
- ID 形态：沿用现有格式，不改 role.json 与启动参数语义（用户确认 Q4）。
- 自部署模型：不靠手写配置，表单内「文件 | URL」二选一 + UI 写 per-base 缓存（用户确认 Q6）。
- URL：默认 models.dev 且允许自定义（用户确认 Q7）。
- 文件格式：复用 `{flag, models[]}` 并提供文件说明文档（用户确认 Q8）。
- 兜底：launch 行已含 flag 时不再追加，launch 手写优先（用户确认 Q9）。
- 二进制优先级与查找路径沿用既有（`opencode2` 先于 `opencode`，先 PATH 后已知安装位）；Claude catalog 缓存读取保留为 Claude 的本地兜底。

# Open questions

（无；Shape 已由用户确认，进入 Build。）

# Verification expectations

- `cargo test -p tty7-core agent_roles` 全过。
- `cargo check --bins` 通过。
- 真机：有网无 CLI 时 A1/A2 可复现；断网/坏文件/坏 URL 时 A4/A6 文案可复现；无映射 base 按 A3 保持旧行为；A7 去重可用 argv 单测 + 真机 launch 验证。
- 独立只读 Verifier 按 A1–A7 逐项判定。
