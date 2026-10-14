---
title: 'Markdown预览支持选中复制与全选'
type: 'feature'
ticket: 'story-md-preview-copy'
created: '2026-10-13'
status: 'built'
baseline_revision: '5c79639b372f778e0c42e7d2927652c2edef5ed4'
route: 'oneshot'
route_source: 'auto'
risk: 'low'
review: 'quick'
review_source: 'pinned'
lenses_ran: [quick]
review_loop_iteration: 0
followup_review_recommended: false
context: []
warnings: []
deferred: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Markdown预览态不可选中、无右键菜单，用户无法拷走预览里的某段渲染文本。

**Approach:** 预览TextView打开可选，容器加右键菜单（复制选中段渲染纯文本，无选中置灰；全选），快捷键行为随上游已有绑定保持一致。

</frozen-after-approval>

## Implementation Notes

- Oneshot路由：单文件约60行改动（结构字段+创建+文本同步+渲染菜单+菜单构造），无新依赖、无schema、无跨模块契约。
- 实现（2026-10-13）：仅改 `src/ui/code_editor.rs`。`OpenFile` 新增 `preview_state: Option<Entity<TextViewState>>`（Markdown 才 Some）；`editor_new_buffer` 创建；`editor_note_edit` 在 preview 可见时同步（`set_text` 相同即返）；`toggle_document_preview` 切回预览时先同步（覆盖源码态编辑期）；渲染改用 `TextView::new(&state).selectable(true)` + 外层 `context_menu`（复制/全选，复用 `AppMenuCopy`/`AppMenuSelectAll` 文案键，无 i18n 新增）。复制读 `selected_text().trim()`（与上游 Copy 语义一致），无选中/空源分别置灰；空写剪贴板直接返回。
- 未新增单测：菜单接线属 GUI wiring（本仓同类菜单亦无单测先例），由 `cargo check` + 既有 `code_editor` 81 项 + `i18n` 8 项全过 + 手工 GUI 验收覆盖。
- 跟进修复（2026-10-14，25 机真机验收）：右键复制粘出旧剪贴板内容。根因：点菜单项的左键在 capture 阶段先清全部可选选中，`on_click` 时再读已为空而直接返回。改为菜单构建时快照选中文本进闭包（commit `fix(editor): 预览右键复制在菜单打开时快照选中`）。25 机复验通过。
- 文件：`src/ui/code_editor.rs`。

## Plan Change Log

## Review Triage Log

- Rename/Save-As staleness (medium, patched): `preview_state` created once from creation-time language; verified `language()` derives from current path (`code_editor.rs:161`) while `editor_save_to`/`editor_path_moved` only re-pointed path+highlighter — untitled→saved-as-.md then preview fell into the non-copy fallback. Fixed with `editor_sync_preview_state` (Some exactly when Markdown) called from both paths.
- Select-All enabled on source that renders to nothing (low, rejected): only hits Markdown whose source is non-empty yet renders empty; click is a harmless no-op and Copy stays correctly disabled. Fix would need a non-public rendered-text accessor — more than a simple correction.

## Verification

**Commands:**
- `cargo check -p tty7` -- expected: 成功，无新增warning
- `cargo test -p tty7 code_editor` -- expected: 全过
- `cargo test -p tty7 i18n` -- expected: 全过（复用既有文案键，应无缺键）

**Manual checks (if no CLI):**
- GUI打开.md进Preview：拖选高亮，右键见复制/全选；复制粘贴得渲染纯文本无标记；无选中右键复制置灰；全选后复制得全文；⌘C/Ctrl+C与右键一致

## Code Review

### 2026-10-14

**Code review complete.** 0 `decision-needed`, 3 `patch`, 1 `defer`, 6 rejected. Findings written to `_bmad-output/backlog/story-md-preview-copy-plan.md`.

- [ ] [Review][Patch] 改扩展名离开 Markdown 时退出预览态 [`src/ui/code_editor.rs:editor_sync_preview_state`] — `preview_state` 清掉但 `preview` 保留，非 md 文件继续显示渲染窗格
- [ ] [Review][Patch] 预览文本同步补 gpui 单测 [`src/ui/code_editor.rs`] — 打开/编辑/切回/Save-As 断言 `preview_state` 文本
- [ ] [Review][Patch] 预览右键菜单补 gpui 单测 [`src/ui/code_editor.rs`] — 快照复制、置灰谓词、Select-All 接线
- [x] [Review][Defer] 快捷键复制与右键一致性的人工确认 [`src/ui/code_editor.rs`] — deferred: 上游绑定存在但未在真机验证，需在 25 机选中后按 Ctrl+C 比对

Rejected:
- `false` 改名/Save-As/重载导致预览 stale（`editor_reload_from_disk:3018` 显式调 `editor_note_edit`；其余路径经 toggle 回切同步覆盖）
- `false` 空菜单弹窗（`ContextMenu` 对零 item 菜单直接跳过渲染，有终端侧注释为证）
- `false` 复制 trim 去掉缩进是缺陷（ticket Decision 定死首尾 trim，与上游 ⌘C 语义一致）
- `false` 菜单打开期间选中可变致快照过期（modal 菜单打开时无法改选中）
- `low` 右键菜单两次全量 `to_string` 分配（单次点击级开销，可忽略；修法徒增复杂度）
- `low` 源码非空但渲染为空时全选可用（已在 triage 否决：需非公开 accessor 才能修）
