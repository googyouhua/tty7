# role-model-refresh

Agent Role 表单的模型列表行为（归档后的完整目标）。

## 模型来源

- base 为 OpenCode 时，模型列表首选 live 获取：按序试 `opencode2`、`opencode`。
- 二进制查找：先 `PATH`（可执行校验，Windows 兼顾 `.exe`），后 `~/.opencode/bin`、`~/.local/bin`、`/usr/local/bin`、`/opt/homebrew/bin`。
- 运行 `<program> models`，8s 超时；stdout 按行 trim，非空行即一条 `provider/model`。
- 空列表视为获取失败。

## 回退链

1. live 获取成功 → 用 live 列表。
2. live 失败 → 用 `models.json` 对应 base 条目，无则用内置表（Claude/Codex/Gemini/OpenCode 四家，其余空）。
3. UI 场景下 live 失败保留旧表（打开/切换/刷新前的列表）并记录失败原因；非 UI 同步调用返回空，由调用方走第 2 步。

## 表单交互

- 打开表单：先以 fast 值（`models.json`+内置）秒填；仅 OpenCode 起后台 live 回填，成功替换、失败留旧 + 红字原因。
- 切换 base：清空已选 model，fast 值秒填，OpenCode 起后台回填；非 OpenCode 无加载态。
- 点击 refresh：旧表保留 + 加载态（按钮 `…`），后台完成后成功替换、失败留旧 + 红字 `Models refresh failed: <reason>`。
- 并发保护：每次切换/刷新递增代际号，只有最新代际的回填生效；表单关闭或 base 已变时丢弃旧回填。
- 已保存但列表不再含的 model 仍保留可选，不静默清空。

## 非目标

- 不含定时轮询；不含 models.dev 直连；不改 `models.json` 与内置表本身。
