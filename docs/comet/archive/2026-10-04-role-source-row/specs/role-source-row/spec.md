# role-source-row

Role 表单模型来源行的完整目标行为。

## 行布局

- 标题 `模型来源`，注释为单行短文本（en `URL list or local JSON file.` / zh `URL 列表或本地 JSON 文件。` / ja `URL の一覧またはローカル JSON。`），常规宽度下不换行。
- 控件竖排两行、右对齐：第一行 URL|File 下拉，第二行路径/URL 文本输入。
- 加载、缓存、回退、兜底行为沿用 `models-dev-fetch`，本 spec 只定行布局与文案。
