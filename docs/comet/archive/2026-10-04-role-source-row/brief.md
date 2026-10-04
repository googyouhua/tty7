# Outcome

Role 表单的模型来源行干净一行：短注释单行展示，下拉与路径输入竖排放在注释下方。

# Scope

- `src/ui/settings/agent_roles.rs`：模型来源行的控件由横排 `h_flex` 改为竖排 `v_flex`（URL|File 下拉一行，路径/URL 输入一行，右对齐）。
- `src/ui/i18n/{en,zh,ja}.rs`：`SettingsRoleModelsSourceDesc` 缩短为单行：en `URL list or local JSON file.` / zh `URL 列表或本地 JSON 文件。` / ja `URL の一覧またはローカル JSON。`；完整格式说明仍只在 `docs/agents/models-load-file.md`。
- 不改加载逻辑、缓存、回退链与 `role.json`。

# Non-goals

- 不改模型来源的功能语义；不动其他行布局；不改 `models-load-file.md` 内容。

# Acceptance examples

- A1：模型来源行注释为一行短文本，不换行（常规宽度下）；下拉与路径输入竖排两行右对齐。
- A2：`cargo check --bins` 通过；中/英/日三语键齐全。

# Constraints and invariants

- 复用现有 `v_flex`/`settings_choice`/`settings_text_input` 模式；Rust 2024 不用 `gen` 作标识符。

# Decisions

- 注释缩短 + 控件竖排（用户确认方向，待 CONFIRM 落定）。

# Open questions

（无；Shape 已由用户确认，进入 Build。）

# Verification expectations

- `cargo check --bins` 通过。
- 真机（.25）肉眼确认一行的注释与两行控件。
- 独立只读 Verifier 按 A1–A2 判定。
