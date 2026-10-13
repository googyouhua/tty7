---
tracker_id: ""
key: ""
type: epic
title: "用户名选择器：下拉+过滤+删除"
parent: initiative-terminal-blocks
covers: ["CAP-1", "CAP-2", "CAP-3", "CAP-4", "CAP-5", "CAP-6", "CAP-7"]
after: []
assignee: ""
risk: medium
---

# 用户名选择器：下拉+过滤+删除

## Description

把裸启动实例选择器从平铺用户名墙换成紧凑 combobox（输入即过滤、上限滚动下拉），新建走显式按钮，每行可删且删除受 liveness 守卫 + 报告保护。需求源是 `spec-username-selector`（CAP-1–CAP-7），视觉与行为由 UX 双 spine 绑定，结构由架构脊柱 AD-1–AD-9 绑定。

## Outcome

共享机裸启动只见紧凑选择器：能过滤、能进入、能新建、删旧用户时活 server 删不掉且有报告可照着停服。

## Requirements

Source: `spec-username-selector/spec-username-selector.md` (ids reused verbatim).

- CAP-1 filter — combobox substring narrowing, highlight reset, never creates.
- CAP-2 select-enter — confirm highlighted row, resolve dir, rewrite memory, enter.
- CAP-3 explicit create — button-gated creation, remembered next launch, Enter never creates.
- CAP-4 guarded delete — live refuses with report, free deletes with fallback, unknown double-confirms, Default never deletable, failures keep row + memory.
- CAP-5 preselect-memory — open-time preselect with Default fallback; memory never unvalidated/deleted.
- CAP-6 recency meta — relative last-used per row, stable newest-first order.
- CAP-7 keyboard-a11y — pointer-free operation, focus trap + return, live announcements.

## Done when

- 多用户盒子渲染紧凑窗口，recency meta 稳定有序。
- 过滤-进入-新建-删除全通，含 live 拒绝报告与删后记忆回落。
- 新增字符串 en/zh/ja/ru 四 locale 同改。
- `cargo test` 绿，含新增 core 删除/probe/记忆顺序用例。

## Boundaries

picker UI（`src/ui/instance_picker.rs`）+ core instance 模块（`crates/tty7-core/src/core/instance.rs`，probe 辅助同模块）。spec 非目标即边界外：多机盘点、文件报告、拼音/模糊、回收站语义、picker 代停服。

## References

- spec — `spec-username-selector/spec-username-selector.md`, CAP-1–CAP-7
- design — `ux-username-selector/DESIGN.md` (visual deltas, binding per AD-9)
- experience — `ux-username-selector/EXPERIENCE.md` (behavior, binding per AD-9)
- constraint — `architecture-username-selector/architecture-username-selector.md`, AD-1–AD-9

## Notes

- Decision 2026-10-11: tracer = entry 1 (mtime meta, thinnest core→UI path).
- Decision 2026-10-11: i18n keys fold into entries 3/4/5 (four locales each); ja/ru drafted by builder, user reviews at build review.
- Decision 2026-10-11: closing = Refactor sweep + end-to-end suite (both).
- Decision 2026-10-11: open-time preselect fallback belongs to entry 3; entry 5 owns post-delete focus fallback only.
- Decision 2026-10-11: entry 5 after entry 4 — single picker-view lane runs sequential (no parallel same-file work).
- Sequencing: same-file `instance.rs` (1→2); shared picker view (1→3→4, 5 last).
- Split risk noted: entries 3 and 5 are each borderline one-session; split entry 5 into modal-paths vs report-fallback-a11y if the build strains.
