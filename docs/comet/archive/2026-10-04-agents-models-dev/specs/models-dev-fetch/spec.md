# models-dev-fetch

Agent Role 表单的模型列表行为（归档后的完整目标，含 UI 双源加载与 launch 兜底）。

## 模型来源

- 每个 `CLIAgent` 经映射表对应 0–n 个 models.dev provider id；无映射的 base 跳过 HTTP，直接走本地链路。
- 初始映射：`opencode`→`opencode,opencode-go`；`claude`→`anthropic`；`codex`→`openai`；`gemini`→`google`；`grok`→`xai`；`qwen`→`alibaba`；`kimi`→`moonshotai`；`copilot`→`github-copilot`；其余 base 映射为空。
- URL 模式：地址默认为 `https://models.dev/api.json`，用户可在表单改为自建镜像/额外地址；单次 8s 超时，仅取映射 provider 下 `models` 的 keys；OpenCode 系裸 key 加 `opencode/` 前缀（已含 `/` 则原样），其余 base 用裸 id；去重并排序；空结果视为失败。
- 文件模式：用户在表单填本地 JSON 路径并点加载；格式复用 `{flag, models[]}`（与 `models.json` 条目同形，详见 `docs/agents/models-load-file.md`）；设大小/条数上限，超限或非法视为失败。
- HTTP 能力门控于 `remote-install` feature（`ureq`）：feature 关闭时 HTTP 源自动禁用，行为等同文件/本地链路。
- 成功由 UI 写 per-base 缓存文件（config dir 下）；HTTP/文件失败时读缓存，缓存无/空才继续回退。

## 回退链

1. 表单所选源（文件/URL，含缓存）成功 → 用其列表。
2. 本地 live：OpenCode 按序试 `opencode2`、`opencode`（先 `PATH` 可执行校验，后 `~/.opencode/bin`、`~/.local/bin`、`/usr/local/bin`、`/opt/homebrew/bin`），运行 `<program> models` 8s 超时，stdout 按行 trim 非空即一条；Claude 读 `$CLAUDE_CONFIG_DIR`（无则 `~/.claude`）下 `cache/model-catalog` 的 `catalog.config.models[].id`；其余 base 本地 live 为空。
3. `models.json` 对应 base 条目（最后的手写回退之一，用户不再需要手写），无则用内置表（Claude/Codex/Gemini/OpenCode 四家，其余空）。
4. UI 场景下失败保留旧表并记录失败原因；非 UI 同步调用返回空，由调用方走第 3 步。

## 表单交互

- 打开表单：先以 fast 值（`models.json`+内置，必要时叠加缓存）秒填；有映射或用户指定源的 base 起后台 live 回填，成功替换、失败留旧 + 红字原因；无映射且无指定源的 base 无后台任务、无加载态。
- 来源切换：表单内「文件 | URL」二选一 + 文本输入（路径/URL，无原生文件对话框）+ 加载按钮；切换或点加载递增代际号，只有最新代际的回填生效。
- 点击加载：旧表保留 + 加载态（按钮 `…`），后台完成后成功替换、失败留旧 + 红字原因（含文件缺失/非法、URL 超时/解析失败的区分文案）。
- 并发保护：每次切换/加载递增代际号，只有最新代际的回填生效；表单关闭或 base 已变时丢弃旧回填。
- 已保存但列表不再含的 model 仍保留可选，不静默清空。

## 启动兜底

- `role_launch_argv`：launch 按空白分词后已含该 base 的 model flag（含 `flag=value` 形）时，不再追加下拉值；否则 model 非空时追加 `{flag} {model}`。
- 下拉首项恒为"launch line only"（空 model）；下拉选空 + launch 手写 `--model xxx` 即最终兜底。
- `model_flag` 表内容与 `role.json` 结构不变。

## 非目标

- 不含启动预拉取与定时轮询；不改 `models.json` 读取机制与内置表本身；不改 `model_flag` 表内容；不为无 provider 的 base 捏造列表；不做原生文件对话框。
