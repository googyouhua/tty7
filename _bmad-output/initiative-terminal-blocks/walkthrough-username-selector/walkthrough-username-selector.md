# Username-selector epic — walkthrough narrative

Branch `dev/sync-upstream-20261011` · commit range `78cb43b0..05b789f5` · epic [epic-username-selector.md](../epic-username-selector/epic-username-selector.md) · tickets [tickets.toml](../epic-username-selector/tickets.toml)
Status legend: each block carries an unchecked box `- [ ]` until presented; the checked state lives here, the record lives in [walkthrough-username-selector-log.md](./walkthrough-username-selector-log.md). `▶ CURRENT` marks the block under review.

## Block 1 — Intent

- [x] Status: accepted

Bare-launch on a shared box stops showing a wall of usernames and shows one compact combobox instead: type to filter, Enter to enter, an explicit button to create, per-row delete guarded by a liveness probe with a stop/rm report. That outcome is stated in [epic-username-selector.md](../epic-username-selector/epic-username-selector.md), bound by CAP-1–CAP-7 in [spec-username-selector.md](../spec-username-selector/spec-username-selector.md), visual/behavioral deltas in [DESIGN.md](../ux-username-selector/DESIGN.md) and [EXPERIENCE.md](../ux-username-selector/EXPERIENCE.md), structure in [architecture-username-selector.md](../architecture-username-selector/architecture-username-selector.md) (AD-1–AD-9).

Out of scope on purpose (spec non-goals, restated in the epic boundaries): multi-machine inventory, file reports, pinyin/fuzzy matching, recycle-bin semantics, picker-driven server shutdown. Code under review lives in two places: the picker view in [instance_picker.rs](../../../src/ui/instance_picker.rs) and the core instance module in [instance.rs](../../../crates/tty7-core/src/core/instance.rs) (probe helper in the same module).

## Block 2 — Broad strokes

- [x] Status: accepted

Seven ticket entries in [tickets.toml](../epic-username-selector/tickets.toml), built in order, plus one post-build crash fix:

1. Tracer first (entry 1, commit `2a90a78a`): mtime recency meta, thinnest core→UI path — plan [story-mtime-recency-tracer-plan.md](../epic-username-selector/story-mtime-recency-tracer-plan.md).
2. Core delete op before any delete UI (entry 2, commit `908a50d5`): guarded remove + tri-state probe + memory guard in [instance.rs](../../../crates/tty7-core/src/core/instance.rs) — plan [story-core-delete-op-plus-probe-plus-memory-guard-plan.md](../epic-username-selector/story-core-delete-op-plus-probe-plus-memory-guard-plan.md).
3. Picker lane sequential through the shared view (entries 3→4→5, commits `d7861f16`, `eadf166f`, `d6bae08a`): combobox in [instance_picker.rs](../../../src/ui/instance_picker.rs), then create button, then delete modal last — plans [story-combobox-filter-plus-select-plus-keyboard-plus-preselect-plan.md](../epic-username-selector/story-combobox-filter-plus-select-plus-keyboard-plus-preselect-plan.md), [story-explicit-create-button-plan.md](../epic-username-selector/story-explicit-create-button-plan.md), [story-delete-modal-plus-report-plus-fallback-plan.md](../epic-username-selector/story-delete-modal-plus-report-plus-fallback-plan.md).
4. Closing pair (entries 6–7, commits `cfa5ecb7`, `6b0e210c`): refactor sweep, then the lifecycle suite in [username_selector_lifecycle.rs](../../../crates/tty7-core/tests/username_selector_lifecycle.rs) — plans [story-refactor-sweep-plan.md](../epic-username-selector/story-refactor-sweep-plan.md), [story-end-to-end-picker-suite-plan.md](../epic-username-selector/story-end-to-end-picker-suite-plan.md).
5. Post-build crash fix (commit `05b789f5`): theme applied headless so the delete modal always has the Surfaces global it renders against — diff in [instance_picker.rs](../../../src/ui/instance_picker.rs).

Two pre-epic picker fixes (`78cb43b0` hang on startup, `a1a7a73b` close-on-confirm) are context, not epic scope.

## Block 3 — Slices

- [ ] Status: pending

### 3.1 Mtime recency tracer (entry 1 · `2a90a78a`)

- [ ] Status: pending

Core snapshots `(name, mtime)` pairs once per open in [instance.rs](../../../crates/tty7-core/src/core/instance.rs), orders mtime-descending with name-ascending tie-break, omits meta when the filesystem cannot supply it; the picker renders the relative-time row meta in [instance_picker.rs](../../../src/ui/instance_picker.rs). Covers CAP-6. Detail: [story-mtime-recency-tracer-plan.md](../epic-username-selector/story-mtime-recency-tracer-plan.md).

### 3.2 Core delete op + probe + memory guard (entry 2 · `908a50d5`)

- [ ] Status: pending

Adds `remove_instance` / `remove_instance_in` (validate, authoritative probe, unlink; refuses symlinked dirs and the default sentinel), the dir-scoped tri-state holder probe, shape validation inside `write_last_in`, and logging (warn on refused/failed removes, info on success) — all in [instance.rs](../../../crates/tty7-core/src/core/instance.rs). Covers CAP-4 core half. Detail: [story-core-delete-op-plus-probe-plus-memory-guard-plan.md](../epic-username-selector/story-core-delete-op-plus-probe-plus-memory-guard-plan.md).

### 3.3 Combobox filter + select + keyboard + preselect (entry 3 · `d7861f16`)

- [ ] Status: pending

Input-is-filter over `validate_name` (never re-encoded) in [instance_picker.rs](../../../src/ui/instance_picker.rs): capped viewport, highlight with wrap, Enter confirms the highlighted row only with memory rewritten via `apply_selection`, Esc layering, click select, open-time preselect with Default fallback on missing/invalid/deleted memory, plus `InstancePicker` keys in [en.rs](../../../src/ui/i18n/en.rs), [zh.rs](../../../src/ui/i18n/zh.rs), [ja.rs](../../../src/ui/i18n/ja.rs), [ru.rs](../../../src/ui/i18n/ru.rs) (wired in [mod.rs](../../../src/ui/i18n/mod.rs)). Covers CAP-1, CAP-2, CAP-5, CAP-7. Detail: [story-combobox-filter-plus-select-plus-keyboard-plus-preselect-plan.md](../epic-username-selector/story-combobox-filter-plus-select-plus-keyboard-plus-preselect-plan.md).

### 3.4 Explicit create button (entry 4 · `eadf166f`)

- [ ] Status: pending

Button enabled only for valid unlisted names in [instance_picker.rs](../../../src/ui/instance_picker.rs); click creates `tty7-<name>`, enters it, preselected next launch; Enter alone never creates. New create/invalid keys across [en.rs](../../../src/ui/i18n/en.rs), [zh.rs](../../../src/ui/i18n/zh.rs), [ja.rs](../../../src/ui/i18n/ja.rs), [ru.rs](../../../src/ui/i18n/ru.rs). Covers CAP-3. Detail: [story-explicit-create-button-plan.md](../epic-username-selector/story-explicit-create-button-plan.md).

### 3.5 Delete modal + report + fallback (entry 5 · `d6bae08a`)

- [ ] Status: pending

Per-row delete affordance on named rows only, never Default, in [instance_picker.rs](../../../src/ui/instance_picker.rs): modal with the liveness report (stop and rm commands, advisory pid, no kill action) including second confirm on unknown probe state; failures keep row + memory; memory rewritten to the focused row after Ok; focus trap with return; live announcements. Modal/report/delete-error keys across [en.rs](../../../src/ui/i18n/en.rs), [zh.rs](../../../src/ui/i18n/zh.rs), [ja.rs](../../../src/ui/i18n/ja.rs), [ru.rs](../../../src/ui/i18n/ru.rs); logging per architecture conventions. Covers CAP-4, CAP-5, CAP-7. Detail: [story-delete-modal-plus-report-plus-fallback-plan.md](../epic-username-selector/story-delete-modal-plus-report-plus-fallback-plan.md).

### 3.6 Refactor sweep (entry 6 · `cfa5ecb7`)

- [ ] Status: pending

Cleanup only from deferred review findings; no pushed-out scope. Touchpoints are the same two files: [instance_picker.rs](../../../src/ui/instance_picker.rs) and [instance.rs](../../../crates/tty7-core/src/core/instance.rs). Detail: [story-refactor-sweep-plan.md](../epic-username-selector/story-refactor-sweep-plan.md).

### 3.7 End-to-end picker suite (entry 7 · `6b0e210c`)

- [ ] Status: pending

Closing scripted bare-launch walkthrough (filter, select, create, guarded delete) as the core lifecycle suite in [username_selector_lifecycle.rs](../../../crates/tty7-core/tests/username_selector_lifecycle.rs), following the spec success signal. Detail: [story-end-to-end-picker-suite-plan.md](../epic-username-selector/story-end-to-end-picker-suite-plan.md).

### 3.8 Crash fix — Surfaces global (post-build · `05b789f5`)

- [ ] Status: pending

Theme applied headless so the delete modal in [instance_picker.rs](../../../src/ui/instance_picker.rs) always renders against a present Surfaces global instead of crashing on the missing theme lookup.

## Slice 1 — filter+select

- [x] Status: accepted

Combobox state + keyboard ownership in [instance_picker.rs](../../../src/ui/instance_picker.rs): type-to-filter over `validate_name`, capped viewport, highlight with wrap, Enter confirms highlighted row only, Esc layering, click select. Covers CAP-1/CAP-2 keyboard half.

## Slice 2 — guarded delete

- [x] Status: accepted

Probe + modal + report: tri-state liveness probe and `remove_instance` in [instance.rs](../../../crates/tty7-core/src/core/instance.rs), per-row delete modal with stop/rm report and second confirm on unknown state in [instance_picker.rs](../../../src/ui/instance_picker.rs). Covers CAP-4.

## Slice 3 — memory fallback

- [x] Status: accepted

Preselect + post-delete rewrite + guards: open-time preselect with Default fallback, memory rewritten via `apply_selection` and to focused row after Ok delete, shape validation in `write_last_in` in [instance.rs](../../../crates/tty7-core/src/core/instance.rs) + [instance_picker.rs](../../../src/ui/instance_picker.rs). Covers CAP-5.

## Slice 4 — i18n+a11y

- [x] Status: accepted

Key family, roles, count/position labels: `InstancePicker` keys across [en.rs](../../../src/ui/i18n/en.rs), [zh.rs](../../../src/ui/i18n/zh.rs), [ja.rs](../../../src/ui/i18n/ja.rs), [ru.rs](../../../src/ui/i18n/ru.rs) (wired in [mod.rs](../../../src/ui/i18n/mod.rs)), modal focus trap with return, live announcements, row count/position labels in [instance_picker.rs](../../../src/ui/instance_picker.rs). Covers CAP-7.

## Block 4 — Periphery ▶ CURRENT

- [ ] Status: pending

- Locales: every user-visible picker string landed in four locales at once (entries 3/4/5) — [en.rs](../../../src/ui/i18n/en.rs), [zh.rs](../../../src/ui/i18n/zh.rs), [ja.rs](../../../src/ui/i18n/ja.rs), [ru.rs](../../../src/ui/i18n/ru.rs), key registration in [mod.rs](../../../src/ui/i18n/mod.rs). ja/ru drafted by the builder; user review of wording happens at build review.
- Architecture spine: AD-1–AD-9 in [architecture-username-selector.md](../architecture-username-selector/architecture-username-selector.md) stayed binding; deferred items (if any) go to [deferred-work.md](../deferred-work.md).
- Verification bar from the epic: multi-user box renders the compact window with stable recency order; filter→enter→create→delete all pass including the live-refusal report and post-delete memory fallback; `cargo test` green including the new core delete/probe/memory-order cases in [username_selector_lifecycle.rs](../../../crates/tty7-core/tests/username_selector_lifecycle.rs) and unit tests in [instance.rs](../../../crates/tty7-core/src/core/instance.rs).
