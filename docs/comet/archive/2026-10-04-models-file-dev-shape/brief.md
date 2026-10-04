# Outcome

文件模式同时接受两种 JSON：`{flag, models[]}` 手写形状，以及 models.dev 整份/切片文档（无 `models` 数组时按 base 映射提取，与 URL 模式同规则）。

# Scope

- `crates/tty7-core/src/core/agent_roles.rs`：`load_models_file` 在顶层无 `models` 数组时回退到 `parse_models_dev`（同 provider 映射、前缀、排序去重、空即失败）；`models` 键存在但非法时仍报原精确错误，不静默换路。
- `docs/agents/models-load-file.md`：补第二形状说明与示例（整份下载直接可用）。
- 单测：整份/切片文档解析、切片无映射 provider 即失败、`models` 非法仍报原错。

# Non-goals

- 不改 URL 模式、缓存、回退链、launch 兜底；不改 UI。

# Acceptance examples

- A1：把 `https://models.dev/api.json` 下载存文件，表单文件模式加载后对应 base 出现映射 provider 列表（如 opencode 出现 `opencode/*`）。
- A2：只含无关 provider 的切片（如仅 `deepinfra`）对 claude 加载失败并红字，不清空旧表。
- A3：`{"models": ["ok", 7]}` 仍报原非字符串错误，不被 provider 回退吞掉。
- A4：`cargo test -p tty7-core agent_roles` 全过。

# Constraints and invariants

- 回退仅当顶层缺 `models` 键；解析上限沿用（文件 1MiB 走先，provider 文档 32MiB 上限同样适用——文件模式下以 1MiB 为准）。
- Rust 2024 不用 `gen` 作标识符。

# Decisions

- 双形状兼容（用户确认）。

# Open questions

（无；Shape 已由用户确认，进入 Build。）

# Verification expectations

- `cargo test -p tty7-core agent_roles` 全过。
- 真机（.25）用下载的 api.json 文件加载验证 A1。
- 独立只读 Verifier 按 A1–A4 判定。
