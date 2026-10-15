---
title: 'Markdown 预览本地文件跳转'
type: 'feature'
ticket: ''
created: '2026-10-14'
status: 'built'
baseline_revision: '7c905b0a9ed134237ec32f0db060df513c9ed4d7'
route: 'full'
route_source: 'auto'
risk: 'medium'
review: 'quick'
review_source: 'pinned'
lenses_ran: ['quick']
review_loop_iteration: 0
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** 编辑器 Markdown 预览中点击本地文件链接（相对路径如 `./other.md`、绝对路径如 `/var/log/app.log`）当前走 `cx.open_url`，打不开本地文件。

**Approach:** 新增纯路径解析 + 预览链接拦截，本地文件走编辑器优先打开（文本进内置编辑器，二进制/目录走系统默认），http(s)/mailto/纯锚点保持原 `open_url` 行为。

## Boundaries & Constraints

**Always:**
- 相对路径以当前 md 文件所在目录为基准解析，规范化 `./`、`../`，不穿越宿主边界（SFTP 文件不解析为本地路径）。
- 文本文件进内置编辑器（二进制/目录/打不开走系统默认或提示），与 `open_linked_file`、`open_outside_the_editor` 语义一致。
- 外链（http/https/mailto）和纯 `#锚点` 保持原 `cx.open_url` 行为不变。

**Never:**
- 不改终端链接识别（`terminal::search::link_at`）、不改 GitHub 面板 markdown 安全过滤。
- 不引入 fork（gpui-component）改动，全部在 tty7 仓内完成；不支持 TextView 内联插件时用块级 parser/renderer 覆盖含本地链接的段落。
- 不支持远程宿主（SFTP）文件的本地跳转。

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| 相对文件链接 | 预览点击 `[文](./other.md)`，base=/repo/docs/a.md | 在编辑器打开 /repo/docs/other.md | 文件不存在则通知提示，不崩溃 |
| 父目录链接 | 点击 `[配](../config.json)` | 打开 /repo/config.json | 同上 |
| 绝对路径链接 | 点击 `[志](/var/log/app.log)` | 文本进编辑器，二进制走系统默认 | 打不开则通知 `LinkFileOpenFailed` |
| file:// 链接 | 点击 `[f](file:///tmp/x.md)` | 同绝对路径 | 非法 file URL 则忽略并提示 |
| 描点剥离 | 点击 `[s](./other.md#标题)` | 打开 other.md（忽略锚点） | — |
| 外链不变 | 点击 `[w](https://example.com)` | 走系统浏览器 | — |
| 纯锚点不变 | 点击 `[a](#section)` | 走原 open_url/忽略 | — |
| SFTP 文件 | base host 非本地 | 本地跳转禁用，走原行为 | — |

</frozen-after-approval>

## Code Map

- `src/ui/code_editor.rs:3412-3455` -- 预览渲染入口（`render_editor_body`，`TextView::new`/`TextView::markdown` + `markdown_style`），拦截点加在这里
- `src/ui/code_editor.rs:1082-1097` -- 复用 `open_linked_file(path, line, column, is_dir)`，目录走树、文件进编辑器
- `src/ui/code_editor.rs:1221-1265` -- 复用 `open_outside_the_editor`/`open_with`（二进制/系统默认打开 + 失败通知 `LinkFileOpenFailed`）
- `src/ui/github/detail.rs:1031-1088` -- 反例：`renders_safely` + `is_safe_target` 只放行 web/mail，不可照搬（预览需要放行本地）
- `src/terminal/search.rs:974-1010` -- 反例：终端 `link_at` 的 URL 优先于文件判定，可借鉴判定顺序但不改动
- 新文件 `src/ui/code_editor/markdown_link.rs` -- 纯函数解析器，无 gpui 依赖，可单测

## Tasks & Acceptance

**Execution:**
- [ ] `src/ui/code_editor/markdown_link.rs` -- 新增纯解析 `resolve_markdown_link(base, url)`：剥离锚点/query、识别并跳过 http(s)/mailto/纯锚点、支持 file:// 与绝对/相对路径、规范化 join -- 覆盖矩阵全场景单测
- [ ] `src/ui/code_editor.rs` -- 新增 `open_markdown_link`：本地走 `open_linked_file`（文本）/`open_outside_the_editor` 语义，外链走 `cx.open_url`；预览 `TextView` 加 `markdown_block_parser`/`markdown_block_renderer` 拦截含本地链接的段落并渲染可点击 span
- [ ] `src/ui/code_editor.rs` -- 补预览链接集成测试/单测：相对/绝对/file:// 解析到编辑器打开，外链仍 open_url

**Acceptance Criteria:**
- Given 打开 /repo/docs/a.md 预览， when 点击 `./other.md`， then /repo/docs/other.md 在编辑器打开
- Given 点击 `https://example.com`， when 预览， then 走系统浏览器（行为不变）
- Given 点击不存在的本地路径， when 预览， then 通知提示且不崩溃
- Given SFTP host 文件， when 点击相对链接， then 不做本地解析（走原行为）

## Review Triage Log

- quick-01 SFTP takeover rendering — verdict: low — SFTP 预览段落被自定义块接管（tooltip 显示本地路径），点击仍走外部打开；仅渲染与计划“走原行为”不一致 — route: patch
- quick-02 Windows 盘符被判外部 — verdict: low — `C:/foo` 的 `C` 被当 URI scheme，Windows 下本地绝对路径走外部打开；日常相对链接不受影响 — route: patch
- quick-03 缺失文件未通知 LinkFileOpenFailed — verdict: false — 缺失路径走 `editor_open_on_host`→`EditorCantOpen` 通知，满足矩阵“通知提示不崩溃”；LinkFileOpenFailed 仅是二进制分支语义 — route: reject
- quick-04 未复用 open_linked_file — verdict: low — 直调 `std::fs::metadata`+`editor_open_on_host`，行为与 `open_linked_file` 一致但绕过 Host 抽象与计划复用点 — route: patch
- quick-05 Heading 被接管丢样式 — verdict: low — 标题仅加粗，大字号丢失；计划限定段落拦截 — route: patch
- quick-06 含图片/引用/HTML 整段回退 — verdict: low — 同段含本地链接+图片/引用/HTML 时本地链接仍走旧 open_url；属已知回退 — route: defer
- quick-07 未处理内联被静默丢弃 — verdict: false — 接管段落已排除 image/ref/HTML，其余分支均有处理；catch-all 为防御性死代码 — route: reject
- quick-08 加粗/换行丢失 — verdict: low — 接管段落拍平为纯文本，flex 行内 `\n` 不换行；词与链接目标保留 — route: defer
- quick-09 链接元素 ID 跨块重复 — verdict: medium — `(“tty7-md-local-link”, ix)` 仅段内序号，多接管块重复可能错乱点击 — route: patch
- quick-10 集成测试未覆盖点击/全矩阵 — verdict: low — 绝对/file:// /外链仅单元覆盖，集成仅直调相对与缺失 — route: patch

## Design Notes

解析示例（`resolve_markdown_link("/repo/docs/a.md", "./other.md#t")` -> `/repo/docs/other.md`）：
- 先 trim，空/`#...`/`http://`/`https://`/`mailto:` -> None（外链）
- 剥 `#` 锚点与 `?` query，`file://` 用 `Url::to_file_path`，以 `/` 开头为绝对，否则以 base.parent() 为基准 join 后 normalise `.`/`..`
- 仅本地 host 生效；存在性检查留给打开阶段（与终端 `link_at` 不同，预览允许指向待创建文件，打开失败再提示）

## Verification

**Commands:**
- `cargo test -p tty7 markdown_link` -- expected: 解析单测全过
- `cargo test -p tty7 code_editor` -- expected: 预览相关单测全过（含新增）
- `cargo fmt --check` -- expected: 干净
- `cargo clippy -- -D warnings` -- expected: 无新增警告（按仓内 lint 门控执行）

**Remote re-verification (192.168.1.25 `/home/snic/tty7-verify-md-link`, branch `verify/md-local-link`):**
- `markdown_link` 11 passed, `code_editor` 99 passed, `fmt --check` clean
- 为腾空间删除远端 4 个无未提交改动的旧 checkout（`tty7-build`/`tty7-test`/`tty7-onekey`/`tty7-link`）；`tty7_2` 含未提交改动予以保留

**CI fixes (PR #29):**
- `check-host-boundary.sh` 禁 `ui::` 直接 `.is_absolute()`（Windows 下对 `/home/me` 也报 false）：resolver 改走平台无关的 `/` 字符串语义，`file:` 自行解析（`Url::to_file_path` 同样平台相关：Windows 下 `file://host` 变 UNC、`file:///tmp` 反而非法）
- Windows 单测按统一语义钉死（仅远端-host 的 `file:` URL 保留 cfg：Windows 走 UNC，别处判不可读）
- 打开类集成测试加有界等待（CI 负载下异步落盘抖动）
- 评审项同步修：三函数递归抽取 `preview_link_children`、两 Link 分支抽取 `preview_link_run`、(app,host,base) 扎 `PreviewLinkContext`、`open_markdown_link` 加本地守卫、补 file:///目录/二进制集成测试
