# Outcome

文件模式路径框下加一行格式 hint（两种形状 + 文档名），选择器本次不做。

# Scope

- `src/ui/settings/agent_roles.rs`：仅文件模式时在来源行下方加一行小字 hint。
- `src/ui/i18n/{en,zh,ja}.rs` + `mod.rs`：新键 `SettingsRoleModelsFileHint`：en `JSON: {flag, models[]} or a models.dev document — see docs/agents/models-load-file.md.` / zh `JSON：{flag, models[]} 或 models.dev 文档，见 docs/agents/models-load-file.md。` / ja `JSON: {flag, models[]} または models.dev ドキュメント。docs/agents/models-load-file.md 参照。`
- 不碰加载逻辑、选择器（不做）、其他行。

# Non-goals

- 不做文件选择器；不改解析/缓存/回退；不动其他行布局。

# Acceptance examples

- A1：文件模式下路径框下方可见一行格式 hint；切回 URL 模式 hint 消失。
- A2：三语键齐全；`cargo check --bins` 通过。

# Constraints and invariants

- 复用现有小字灰字样式；Rust 2024 不用 `gen` 作标识符。

# Decisions

- 选择器无现成组件，本次不做（用户确认）；只加一行 hint（用户确认）。

# Open questions

（无；Shape 已由用户确认，进入 Build。）

# Verification expectations

- `cargo check --bins` 通过；真机（.25）肉眼确认 hint 出現/消失。
- 独立只读 Verifier 按 A1–A2 判定。
