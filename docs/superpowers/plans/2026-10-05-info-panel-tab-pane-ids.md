---
change: info-panel-tab-pane-ids
design-doc: docs/superpowers/specs/2026-10-05-info-panel-tab-pane-ids-design.md
base-ref: 283dd1a21d71b28359c0a60336d18974e6ae2297
---

# Info Panel Tab/Pane IDs Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Show Tab ID, Pane ID, and machine-wide tab ordinal (`@n`) rows in the right Info panel's Session section.

**Architecture:** All data is already local to `render_panel_info` in `src/ui/right_panel.rs`: tab id from `tab.tree_id.get()`, pane id from the already-read `view.pane_id`, ordinal by enumerating the `MachineMirrors` machine mirror in CLI `tab_index` order. Rows reuse the existing `InfoRow::text(...).copyable()` helpers; no new plumbing.

**Tech Stack:** Rust, GPUI (`#[gpui::test]` harness), existing `L10nKey` i18n system (`src/ui/i18n/{mod,en,zh,ja}.rs`).

**Spec:** `docs/superpowers/specs/2026-10-05-info-panel-tab-pane-ids-design.md` — this plan argues from that doc; executors read both. Task boundaries mirror `docs/openspec/changes/info-panel-tab-pane-ids/tasks.md` exactly (1.1, 1.2, 2.1, 2.2); no scope expansion.

## Global Constraints

- Renderer code holds no literal label text — all labels go through `t(L10nKey::...)`.
- New rows live strictly inside the existing `if let Some(leaf) = tab.detail_pane(window, cx)` block, so split tabs report the last-focused leaf.
- Row order: cwd row, follow-switch row (unchanged), then Tab ID, Pane ID, Tab `#`, then shell.
- ID rows copy raw values (uuid / decimal); the ordinal row copies the decorated `@n` string (paste-ready for `tty7 tab … @n`).
- Mirror enumeration order must stay identical to CLI `tab_index` (`crates/tty7-cli/src/resolve.rs:13`): workspaces in stored order, tabs in stored order.
- zh-CN / ja strings must differ from en (the locale consistency test in `src/ui/i18n/mod.rs` fails byte-identical non-allowlisted keys); do NOT add the new keys to `KEPT_IN_ENGLISH`.
- Strip / sidebar / switcher show no new rows — Info panel only.

---

## File map

| File | Responsibility in this change |
|---|---|
| `src/ui/i18n/mod.rs` | Declare `L10nKey::PanelTabId`, `PanelPaneId`, `PanelOrdinal` enum variants (insert after `PanelSsh`, before `PanelBranch`, ~line 1120). `ALL` is macro-derived, no manual list update. |
| `src/ui/i18n/en.rs` | en strings next to `PanelSsh` (~line 1440). |
| `src/ui/i18n/zh.rs` | zh-CN strings next to `PanelSsh` (~line 1338). |
| `src/ui/i18n/ja.rs` | ja strings next to `PanelSsh` (~line 1504). |
| `src/ui/right_panel.rs` | Build the three `InfoRow`s in `render_panel_info` (~line 937); ordinal lookup helper + unit/`gpui::test` tests in the `mod tests` module (~line 2306). |

---

### Task 1: i18n labels (tasks.md 1.1)

**Files:**
- Modify: `src/ui/i18n/mod.rs` (~line 1116-1121)
- Modify: `src/ui/i18n/en.rs` (~line 1439-1440)
- Modify: `src/ui/i18n/zh.rs` (~line 1337-1338)
- Modify: `src/ui/i18n/ja.rs` (~line 1503-1504)

**Interfaces:**
- Consumes: existing `PanelCwd`/`PanelShell`/`PanelSsh` pattern and the `KEPT_IN_ENGLISH` + `ALL` consistency test in `src/ui/i18n/mod.rs:2100-2122`.
- Produces: `L10nKey::PanelTabId`, `L10nKey::PanelPaneId`, `L10nKey::PanelOrdinal` usable via `t(...)` in Task 2/3.

- [x] **Step 1: Declare the three enum variants** <!-- comet-task:9621092b-ae83-4f3b-b8a8-b870eb62e877 -->

In `src/ui/i18n/mod.rs`, insert after `PanelSsh,`:

```rust
PanelTabId,
PanelPaneId,
PanelOrdinal,
```

Resulting block reads:

```rust
PanelShell,
PanelSsh,
PanelTabId,
PanelPaneId,
PanelOrdinal,
PanelBranch,
```

- [x] **Step 2: Add the en strings** <!-- comet-task:9621092b-ae83-4f3b-b8a8-b870eb62e877 -->

In `src/ui/i18n/en.rs`, insert after `L10nKey::PanelSsh => "ssh",`:

```rust
L10nKey::PanelTabId => "tab id",
L10nKey::PanelPaneId => "pane id",
L10nKey::PanelOrdinal => "tab",
```

- [x] **Step 3: Add the zh-CN strings** <!-- comet-task:9621092b-ae83-4f3b-b8a8-b870eb62e877 -->

In `src/ui/i18n/zh.rs`, insert after `L10nKey::PanelSsh => "ssh",`:

```rust
L10nKey::PanelTabId => "标签页 ID",
L10nKey::PanelPaneId => "窗格 ID",
L10nKey::PanelOrdinal => "标签页",
```

Each value differs from its en string, satisfying the consistency test.

- [x] **Step 4: Add the ja strings** <!-- comet-task:9621092b-ae83-4f3b-b8a8-b870eb62e877 -->

In `src/ui/i18n/ja.rs`, insert after `L10nKey::PanelSsh => "ssh",`:

```rust
L10nKey::PanelTabId => "タブ ID",
L10nKey::PanelPaneId => "ペイン ID",
L10nKey::PanelOrdinal => "タブ",
```

- [x] **Step 5: Run the locale consistency test** <!-- comet-task:9621092b-ae83-4f3b-b8a8-b870eb62e877 -->

Run: `cargo test -p tty7 i18n`
Expected: PASS — all three locales resolve every new key, and none is byte-identical to en outside the allowlist (these keys are NOT added to `KEPT_IN_ENGLISH`).

- [x] **Step 6: Commit** <!-- comet-task:9621092b-ae83-4f3b-b8a8-b870eb62e877 -->

```bash
git add src/ui/i18n/mod.rs src/ui/i18n/en.rs src/ui/i18n/zh.rs src/ui/i18n/ja.rs
git commit -m "feat(info-panel): add Tab ID / Pane ID / tab ordinal i18n keys"
```

---

### Task 2: Tab ID and Pane ID rows (tasks.md 1.2)

**Files:**
- Modify: `src/ui/right_panel.rs` (`render_panel_info`, insert after the follow-switch block closes ~line 996, before the shell comment block ~line 998)
- Test: `src/ui/right_panel.rs` `mod tests` (~line 2306)

**Interfaces:**
- Consumes: `L10nKey::PanelTabId` / `PanelPaneId` from Task 1; `tab: &Tab` with `tree_id: Cell<TabId>` (`src/ui/app.rs:521`); `view: &TerminalView` with `pane_id: u64` (`src/terminal/view.rs:307`), both already in scope inside the `detail_pane` block; `InfoRow::text` + `.copyable()` (`src/ui/right_panel.rs:421-439`).
- Produces: two copyable `InfoValue::Text` rows in fixed position (ordinal row from Task 3 slots between the Pane ID push and the shell block).

- [x] **Step 1: Insert the two rows** <!-- comet-task:13daf51a-8d2e-43e6-bb50-e5b7bec5e8b5 -->

In `render_panel_info`, immediately after the follow-switch `if` block's closing brace and before the `// A pane on the default shell runs ...` comment, insert:

```rust
rows.push(InfoRow::text(t(L10nKey::PanelTabId), tab.tree_id.get().to_string()).copyable());
rows.push(InfoRow::text(t(L10nKey::PanelPaneId), view.pane_id.to_string()).copyable());
```

Notes the implementer must honor: `InfoValue::Text` (tail truncation handles the 36-char uuid in narrow panels; full value rides the copy tile, same contract as the branch row). No `reveal`, no `edit_cwd`, no `toggle_follow`. Both lines stay inside the `if let Some(leaf)` block so a split tab reports the last-focused leaf, matching the cwd/shell rows. The existing `rows.is_empty()` → `PanelNoSession` early return (~line 1063) is untouched — the rows cannot appear without a session.

- [x] **Step 2: Write the failing row-construction test** <!-- comet-task:13daf51a-8d2e-43e6-bb50-e5b7bec5e8b5 -->

Append to `mod tests` in `src/ui/right_panel.rs`:

```rust
#[test]
fn id_rows_carry_raw_values_as_copy_payloads() {
    let tab_id = "9f2c4b1a-3d5e-4f6a-8b7c-1d2e3f4a5b6c";
    let tab_row = InfoRow::text("tab id", tab_id.to_string()).copyable();
    assert!(
        matches!(&tab_row.value, InfoValue::Text(v) if v == tab_id),
        "tab id renders the full uuid as text"
    );
    assert_eq!(
        tab_row.copy.as_deref(),
        Some(tab_id),
        "tab id copies the raw uuid, undecorated"
    );
    assert!(!tab_row.edit_cwd && !tab_row.toggle_follow && tab_row.reveal.is_none());

    let pane_row = InfoRow::text("pane id", 42u64.to_string()).copyable();
    assert!(
        matches!(&pane_row.value, InfoValue::Text(v) if v == "42"),
        "pane id renders the decimal pane id as text"
    );
    assert_eq!(
        pane_row.copy.as_deref(),
        Some("42"),
        "pane id copies the raw decimal, not %42"
    );
}
```

This test needs `InfoRow` field access — fields are private to the module, and `mod tests` is a child of the same module, so access is legal (same pattern as the existing `diff()` helper at line 2362). Run it before the render change is wired if the change is not yet in place; it passes on construction alone, so its real gate is Step 4.

- [x] **Step 3: Write the focus-following regression test** <!-- comet-task:13daf51a-8d2e-43e6-bb50-e5b7bec5e8b5 -->

The behavior under test: both pushes read `tab.tree_id.get()` and `view.pane_id` from the *focused leaf's* `view`, where `view` comes from `leaf.read(cx)` two lines above — the same `view` the cwd row uses. Refocusing a split changes which leaf `detail_pane` returns, so the Pane ID follows while the Tab ID (read from `tab`, not `leaf`) stays put. Encode this as a `#[gpui::test]` only if a multi-pane fixture exists in this repo's harness; otherwise verify by code inspection against the two source lines (`tab.tree_id.get()` vs `view.pane_id`) plus the manual pass in Task 4, and record which was done in the commit message. Do not invent a harness — grep for existing `detail_pane` gpui tests (e.g. `src/ui/app.rs` split tests near line 13483) and reuse their fixture verbatim if one fits.

- [x] **Step 4: Run the panel tests** <!-- comet-task:13daf51a-8d2e-43e6-bb50-e5b7bec5e8b5 -->

Run: `cargo test -p tty7 right_panel`
Expected: PASS, including `id_rows_carry_raw_values_as_copy_payloads` and all pre-existing tests (`a_row_lights_up_only_when_there_is_something_behind_it`, port tests).

- [x] **Step 5: Commit** <!-- comet-task:13daf51a-8d2e-43e6-bb50-e5b7bec5e8b5 -->

```bash
git add src/ui/right_panel.rs
git commit -m "feat(info-panel): render Tab ID and Pane ID rows with copy tiles"
```

---

### Task 3: Machine-wide tab ordinal with fallback (tasks.md 2.1)

**Files:**
- Modify: `src/ui/right_panel.rs` (`render_panel_info`, insert between the Pane ID push from Task 2 and the shell comment block; add a `tab_ordinal` helper near `format_rtt` ~line 385 or as an associated function)
- Test: `src/ui/right_panel.rs` `mod tests`

**Interfaces:**
- Consumes: the two pushes from Task 2 (insertion point); `MachineMirrors::machine(cx, host)` returning `Option<&Machine>`; host from `crate::core::session::WorkspaceStore::all(cx).get(self.workspace)` → `entry.host_id()` (same expression as `src/ui/switcher.rs:1050`); `EMPTY` dash const (`src/ui/right_panel.rs:377`); `L10nKey::PanelOrdinal` from Task 1.
- Produces: ordinal row in final position (Tab ID, Pane ID, Tab `#`); `tab_ordinal(&Machine, TabId) -> Option<u64>` pure helper unit-tested for synced and unsynced cases.

- [x] **Step 1: Add the pure ordinal helper** <!-- comet-task:e1daf8bb-9700-4957-b48c-783326483dc4 -->

Place near `format_rtt`:

```rust
/// Position of `tab` in the machine-wide tab order, 1-based. Enumerates
/// workspaces then tabs in stored order — the GUI mirror of the CLI's
/// `tab_index` (`crates/tty7-cli/src/resolve.rs:13`). Returns `None` when
/// the tab is absent (mirror not yet synced), in which case the caller
/// renders the table `EMPTY` dash.
fn tab_ordinal(
    machine: &tty7_core::core::machine::Machine,
    tab: tty7_core::core::machine::TabId,
) -> Option<u64> {
    let mut ordinal = 0;
    for ws in &machine.workspaces {
        for t in &ws.tabs {
            ordinal += 1;
            if t.id == tab {
                return Some(ordinal);
            }
        }
    }
    None
}
```

Check the actual `Machine`/`TabId` paths against `crates/tty7-cli/src/resolve.rs:1-11` (`tty7_core::core::machine::{Machine, TabId}`) and the `TabEntry.tab` type — adjust the import path to whatever `right_panel.rs` already uses for core types; do not introduce a second naming.

- [x] **Step 2: Insert the ordinal row** <!-- comet-task:e1daf8bb-9700-4957-b48c-783326483dc4 -->

Between the Pane ID push and the shell comment block, insert:

```rust
let ordinal_text = crate::core::session::WorkspaceStore::all(cx)
    .get(self.workspace)
    .and_then(|entry| crate::ui::machine_mirror::MachineMirrors::machine(cx, entry.host_id()))
    .and_then(|machine| tab_ordinal(machine, tab.tree_id.get()))
    .map(|n| format!("@{n}"));
match ordinal_text {
    Some(decorated) => rows.push(InfoRow {
        label: t(L10nKey::PanelOrdinal),
        value: InfoValue::Text(decorated.clone()),
        // Paste-ready for `tty7 tab … @n`.
        copy: Some(decorated),
        reveal: None,
        edit_cwd: false,
        toggle_follow: false,
    }),
    None => rows.push(InfoRow::text(t(L10nKey::PanelOrdinal), EMPTY.to_string())),
}
```

Verify before writing: `self.workspace` is the field holding this window's `WorkspaceId` (confirm the field name at the `WorkspaceStore::all(cx).get(...)` call sites and use the exact one); `entry.host_id()` returns the id type `MachineMirrors::machine` takes (cf. `src/ui/switcher.rs:1050` and `src/ui/machine_mirror.rs:497-506`); the fallback row carries `copy: None` (a dash is not worth copying) via plain `InfoRow::text` without `.copyable()`. Remote panes need no special casing: enumeration uses the pane's host mirror, and `pane_id` is displayed as-is.

- [x] **Step 3: Write the failing helper tests** <!-- comet-task:e1daf8bb-9700-4957-b48c-783326483dc4 -->

Append to `mod tests`:

```rust
#[test]
fn ordinal_counts_workspaces_then_tabs_in_stored_order() {
    // Build with the same testbed constructor the CLI uses:
    // `crate::testbed::two_workspace_machine` is in tty7-cli;
    // for the tty7 binary use this crate's equivalent fixture with
    // two workspaces (two tabs in the first, one in the second).
    // Assert tab_ordinal returns Some(1), Some(2) for the first
    // workspace's tabs and Some(3) for the second's — dense from @1.
}
```

Concretely: find this crate's `Machine` fixture constructor (grep `two_workspace_machine` / `Machine {` under `src/`; `machine_mirror.rs` tests near line 848 install a machine via pull — reuse that fixture builder). If no fixture builder exists in the binary crate, construct a minimal `Machine` literally (workspaces with `id` + `tabs` with `id`, all other fields `Default::default()`) and assert:

```rust
assert_eq!(tab_ordinal(&m, m.workspaces[0].tabs[0].id), Some(1));
assert_eq!(tab_ordinal(&m, m.workspaces[0].tabs[1].id), Some(2));
assert_eq!(tab_ordinal(&m, m.workspaces[1].tabs[0].id), Some(3));
```

- [x] **Step 4: Cover the unsynced fallback** <!-- comet-task:e1daf8bb-9700-4957-b48c-783326483dc4 -->

```rust
#[test]
fn ordinal_missing_from_mirror_means_no_copy_payload() {
    // A tab id absent from the machine (mirror not yet synced):
    // tab_ordinal returns None, and the fallback row built with
    // InfoRow::text(LABEL, EMPTY.to_string()) has copy == None,
    // reveal == None, and value Text("—"), so the ids still render
    // while the ordinal cell shows the table dash.
    let row = InfoRow::text("tab", "—".to_string());
    assert_eq!(row.copy, None);
    assert!(matches!(&row.value, InfoValue::Text(v) if v == "—"));
}
```

Plus, if a `#[gpui::test]` fixture with a populated `MachineMirrors` entry exists (see `src/ui/machine_mirror.rs:848` pull-install pattern), add one integration assertion that `render_panel_info`'s row for a synced tab shows `@n` and for a store entry whose host has no machine shows `—`; otherwise rely on the Task 4 manual pass and say so in the commit message.

- [x] **Step 5: Run the tests** <!-- comet-task:e1daf8bb-9700-4957-b48c-783326483dc4 -->

Run: `cargo test -p tty7 right_panel`
Expected: PASS — new ordinal tests plus all Task 2 tests green.

Run: `cargo test -p tty7 i18n`
Expected: PASS — `PanelOrdinal` still resolves in all locales.

- [x] **Step 6: Commit** <!-- comet-task:e1daf8bb-9700-4957-b48c-783326483dc4 -->

```bash
git add src/ui/right_panel.rs
git commit -m "feat(info-panel): render machine-wide tab ordinal with dash fallback"
```

---

### Task 4: Parity and verification pass (tasks.md 2.2)

**Files:**
- No new code. Evidence only: terminal transcripts pasted into the change's verification notes.

**Interfaces:**
- Consumes: finished Tasks 1–3; live commands `tty7 tab ls --json` and `tty7 pane ls`.
- Produces: the change's done criterion — Info rows match daemon truth, no other surface changed, full touched-crate suite green.

- [x] **Step 1: Manual parity pass** <!-- comet-task:611ae9dc-e7f6-4326-b6f2-2fc1d9ef8e26 -->

1. Open two same-named tabs plus a split in the second tab (three panes total across two tabs).
2. Focus each pane in turn and read the Info panel: Tab ID equals the `id` field for that tab in `tty7 tab ls --json`; Pane ID equals the focused pane's id in `tty7 pane ls`; ordinal equals the tab's `@n` in `tty7 tab ls --json`.
3. Disconnect/sync-lag case: with the daemon unreachable at startup (mirror not yet synced), confirm the ordinal cell shows `—` while both ID rows still render.
4. Confirm the strip, sidebar, and switcher show no new rows (this change touches only `render_panel_info`).

- [x] **Step 2: Run the full touched-crate suite** <!-- comet-task:611ae9dc-e7f6-4326-b6f2-2fc1d9ef8e26 -->

Run: `cargo test -p tty7`
Expected: PASS with zero failures. If the workspace layout puts the panel/i18n code in a differently named crate, substitute it (the crate containing `src/ui/right_panel.rs`) and record the exact command run.

- [x] **Step 3: Record evidence and finish** <!-- comet-task:611ae9dc-e7f6-4326-b6f2-2fc1d9ef8e26 -->

Paste into the change record: the `@n`/id comparisons from Step 1 (tab ids, pane ids, ordinals vs CLI output), the `—` fallback observation, and the `cargo test` tail. No commit (no files change in this task). If any mismatch appears — ordinal off by one, pane id stale after refocus, an extra row in another surface — file it against Task 2/3 with the observed vs expected values and do not mark the change done.

---

## Self-review

1. **Spec coverage:** Context/approach (three `InfoRow`s in `render_panel_info`, focused-leaf semantics) → Tasks 2–3 insertion inside the `detail_pane` block. Data sources (uuid tab id, decimal pane id, existing `pane_id` read at line 951) → Task 2. Ordinal enumeration mirroring `tab_index` + host via `WorkspaceStore` + `EMPTY`-dash fallback + decorated-`@n` copy payload → Task 3. Labels via `PanelCwd` pattern, no literals → Task 1. Edge cases (no-leaf early return unchanged, hibernated/remote no special casing) → noted in Tasks 2–3. Test strategy (unit rows/copy/focus, mirror-missing fixture, manual `tab ls --json` / `pane ls` compare, `cargo test`) → Tasks 2–4. Risk (enumeration order parity) → Task 3 helper mirroring `tab_index` loop order. All covered; no gaps.
2. **Placeholder scan:** No TBD/TODO; every code step carries exact code, exact paths, exact commands, exact commit messages. The two conditional-test steps (focus-following `gpui::test`, mirror-installed render assertion) name the exact fixture files to reuse and the exact fallback (record in commit message + Task 4 manual pass) — no open-ended placeholders.
3. **Type consistency:** `tab.tree_id.get()` → `TabId` → `.to_string()` (uuid) used identically in Task 2 code and Task 3 `tab_ordinal(machine, tab.tree_id.get())`; `view.pane_id: u64` → `.to_string()` (decimal); ordinal `u64` → `format!("@{n}")`; helper signature `(&Machine, TabId) -> Option<u64>` matches both call and tests; `InfoRow` struct-literal fields (`label/value/copy/reveal/edit_cwd/toggle_follow`) match the definition at `right_panel.rs:402-419`.
