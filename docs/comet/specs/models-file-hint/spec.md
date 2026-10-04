# models-file-hint

文件模式格式 hint 行的完整目标行为。

## 展示规则

- 仅当模型来源切换为文件模式时，在来源行下方展示一行小字 hint。
- 文案（单键三语）：en `JSON: {flag, models[]} or a models.dev document — see docs/agents/models-load-file.md.` / zh `JSON：{flag, models[]} 或 models.dev 文档，见 docs/agents/models-load-file.md。` / ja `JSON: {flag, models[]} または models.dev ドキュメント。docs/agents/models-load-file.md 参照。`
- 切回 URL 模式时不展示；不影响加载逻辑。
