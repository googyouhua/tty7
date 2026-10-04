# Model load file

Agent Role 表单「模型来源 = 文件」模式读取的 JSON 格式：给 models.dev 没有收录的 base（自部署模型、自建 endpoint）手写模型列表。文件放在哪里都可以，表单里填路径后点刷新即加载；加载成功后 UI 会写 per-base 缓存，重开表单仍在。

## 格式

文件接受两种形状（顶层有无 `models` 数组决定走哪条路）：

### 形状一：手写 `{flag, models[]}`

| 字段 | 必填 | 说明 |
| --- | --- | --- |
| `models` | 是 | 字符串数组，每项一个模型 id。OpenCode 系请写 `provider/model` 全形（如 `"my-proxy/llama3.1"`）；其余 base 写裸 id。空字符串会被忽略。 |
| `flag` | 否 | 接受并校验（须以 `-` 开头，如 `"--model"`），目前仅作记录：启动时实际用的 flag 仍来自内置表。 |

```json
{
  "flag": "--model",
  "models": [
    "my-proxy/llama3.1-70b",
    "my-proxy/qwen3-32b"
  ]
}
```

另一个例子（Qwen 自部署直连写法，裸 id 即可）：

```json
{
  "models": ["qwen3:32b", "qwen3-coder:30b"]
}
```

### 形状二：models.dev 整份/切片文档

从 `https://models.dev/api.json` 下载的文件直接可用：顶层没有 `models` 数组时，文件被视为 provider 文档，按当前 base 的映射提取（与 URL 模式同规则）——`opencode`/`opencode-go` 的裸 key 自动加 `provider/` 前缀，其余 provider 用裸 id；无命中即失败。切片文档同样接受（只含需要的 provider 即可）。

## 校验规则

- 文件须 ≤ 1 MiB，形状一的 `models` 须为字符串数组且 ≤ 2000 项；`models` 存在但非法（非数组、含非字符串项、有效为 0、超限、`flag` 非法）报精确错误，不会静默换路。
- 形状二无命中（如切片里没有当前 base 的 provider）即失败。
- 失败时表单保留旧列表并红字说明原因（缺文件、解析失败、超限、无命中等），不会清空下拉。

## 优先级

文件加载成功 → 用文件列表；失败 → 读该文件上次成功时的缓存 → 本地 CLI/目录 → `models.json` → 内置表。无论列表里有没有，启动命令里手写的参数永远是最后兜底：如果启动行已经写了该 base 的 model flag（`--model xxx` 或 `--model=xxx`），下拉的值不再重复追加。
