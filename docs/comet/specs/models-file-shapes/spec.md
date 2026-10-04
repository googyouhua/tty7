# models-file-shapes

模型加载文件的完整目标行为（文件模式双形状）。

## 形状一：手写 `{flag, models[]}`

- 顶层 `models` 为字符串数组（≤2000 项），`flag` 可选、出现时须以 `-` 开头（仅记录，启动 flag 仍来自内置表）。
- 文件 ≤1MiB；`models` 缺失则看形状二；存在但非法（非数组、含非字符串、有效为 0、超限、flag 非法）即失败并报精确原因。

## 形状二：models.dev 整份/切片文档

- 顶层无 `models` 数组时，文档被视为 provider 映射：仅提取 `models_dev_providers(base)` 覆盖的 provider 下 `models` 的 keys。
- `opencode`/`opencode-go` 的裸 key 加 `provider/` 前缀（已含 `/` 原样），其余 provider 用裸 id；排序去重；无命中即失败。
- 大小上限适用 provider 文档 32MiB（文件模式仍先过 1MiB 文件上限）。

## 回退与失败

- 任一形状成功即用其列表并写 source-keyed 缓存；失败（非法形状、无命中）保留旧表 + 红字原因，再走缓存/本地/`models.json`/内置。
