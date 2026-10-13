---
id: ""
type: story
title: "Markdown预览支持选中复制与全选"
parent: none
covers: []
after: []
assignee: ""
refined: true
hitl: false
risk: low
---

# Markdown预览支持选中复制与全选

## Description

Markdown 预览态支持鼠标拖选渲染文本。选中后右键菜单提供复制与全选，复制结果为选中段的渲染纯文本。无选中时复制置灰，全选后可一键复制全文渲染文本。

## Acceptance Criteria

1. **预览文本可选中**
   **Given** 一个 Markdown 文件处于预览态
   **When** 用户用鼠标拖过一段渲染文本
   **Then** 该段高亮选中，松开后选中保留
2. **右键复制选中段**
   **Given** 预览中有非空选中
   **When** 用户右键并选择复制
   **Then** 剪贴板为选中段的渲染纯文本，不含 Markdown 标记，且首尾空白已去除
3. **无选中时复制置灰**
   **Given** 预览中无任何选中
   **When** 用户右键打开菜单
   **Then** 复制项可见但置灰不可点
4. **全选后复制全文**
   **Given** 预览中有内容
   **When** 用户右键选择全选后再选择复制
   **Then** 剪贴板为全文渲染纯文本，顺序与屏显一致
5. **快捷键复制一致**
   **Given** 预览中有非空选中
   **When** 用户按复制快捷键
   **Then** 剪贴板与右键复制结果一致
6. **空预览不提供复制**
   **Given** 预览内容为空
   **When** 用户右键
   **Then** 复制与全选均置灰或隐藏，剪贴板不被改写

## Boundaries

- Must not change: 源 Markdown 文本内容，编辑态的右键菜单与快捷键，预览与编辑的切换行为。
- Must not change: 非 Markdown 文件的编辑器行为。

## References

- parent — none
- source — docs/reference/configuration.mdx, `editor_markdown_preview` 行（预览态定义）

## Notes

- Decision: 只做选中段复制，不做全文复制按钮与代码块一键复制（2026-10-13）。
- Decision: 复制对象为渲染纯文本，不含标记，首尾 trim（2026-10-13）。
- Decision: 入口为右键菜单，含全选（2026-10-13）。
- Risk: low —— 仅读剪贴板写入，不改文档与持久状态。
- 右键菜单复用编辑态已有的复制与全选文案与样式，无新设计；布局由构建者按既有菜单惯例实现。
