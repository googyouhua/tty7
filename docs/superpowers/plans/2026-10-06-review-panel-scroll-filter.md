---
change: review-panel-scroll-filter
design-doc: docs/superpowers/specs/2026-10-06-review-panel-scroll-filter-design.md
base-ref: 3bbc70dcd81a877687fb8be6b7853ed5047374d2
---

# Review Panel Scroll + Filter Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the Review tab usable with long branch diffs by giving the file list and draft list their own bounded scroll regions plus a file path filter.

**Architecture:** Restructure `review_branch_body` into a pinned header plus two independent GPUI scrollers (file region and draft region), following the `github_shell` pattern in `src/ui/panel_github.rs:130-165`. Filter state lives in `ReviewState` as a plain `String` (reset in `set_selection`) synced with a lazily created `InputState` box, following the `scm_filter_row` pattern in `src/ui/scm/panel.rs:1471-1511`.

**Tech Stack:** Rust, GPUI (`overflow_y_scroll` + `track_scroll(ScrollHandle)`), `gpui-component` `Input`, `crate::ui::scrollbar::over_vertical_scroll`, existing `FileListCache.snapshot` probe (no new git processes).

**Spec:** `docs/superpowers/specs/2026-10-06-review-panel-scroll-filter-design.md` (technical design), `docs/openspec/changes/review-panel-scroll-filter/design.md` (decisions), `docs/openspec/changes/review-panel-scroll-filter/specs/review-panel/spec.md` (requirements), tasks bounded by `docs/openspec/changes/review-panel-scroll-filter/tasks.md`.

## Global Constraints

- No change to diff overlay, branch resolution, draft data model, or send flow (design Non-Goals).
- No new git probe or cache key: filtering is a render-time view over the existing `FileListCache.snapshot`.
- Filter resets on repo/base/head change (stale filter must never hide a fresh range's files).
- No virtualized rows; lists are tens-to-hundreds of rows, plain scroll suffices.
- Row rendering and click-to-open behavior stay identical; only the toggle chrome disappears.
- Hardcoded English UI strings match `src/ui/panel_review.rs` existing convention (`"Sent to agent: {}"`, `"Reading changed files…"`); no new `L10nKey` entries (out of scope).
- Verify with `cargo test -p tty7 review_state`, `cargo check -p tty7`, `cargo test -p tty7`, `cargo clippy -p tty7`.
- Frequent commits, one per task.

## File Structure

- Modify `src/ui/review_state.rs`: add `files_scroll`, `drafts_scroll`, `file_filter`, `filter_input`, `filter_sub` to `ReviewState`; reset filter in `set_selection`; add `matches_file_filter` pure helper; add unit tests. Remove `Clone, Debug` from `ReviewState` derive (becomes `#[derive(Default)]`, matching `GitHubPanelState` / `ScmPanelState` precedent — `ScrollHandle`, `Entity`, and `Subscription` are not `Clone`/`Debug`; grep confirms nothing clones or debug-formats `ReviewState` itself, only `ReviewDraft`/`FileListCache` which keep their derives).
- Modify `src/ui/panel_review.rs`: thread `window` into `review_branch_body`; split it into pinned header + file scroll region + draft scroll region; add `review_filter_row` + `review_file_filter_query`; rewrite `review_files_section` to render filtered rows with `Files (N of M)` header data and no toggle; extract `review_drafts_section`; delete `toggle_unfolded` call path and `unfolded` field usage.
- Modify `src/ui/review_state.rs` (same file, Task 3 step): delete `unfolded` field on `FileListCache` and `toggle_unfolded` method (dead after toggle removal; both used only by the toggle path).

## Coverage Map (spec requirement -> task)

- "Diff file list scrolls in a bounded region" -> Task 3 (file scroller).
- "Diff file list can be filtered by path" (narrows + clearing restores) -> Task 4 (filter row + render filter + clear affordance via `cleanable(true)`).
- "File list shares space with the draft list" (drafts visible with long file list) -> Task 2 (layout) + Task 5 (draft floor).
- "Draft list scrolls in a bounded region" -> Task 5.
- Edge cases (loading/error pinned, "No files match", collapse-to-content, no new probes) -> Tasks 2, 3, 4.
- Design testing section (`review_state` unit test, check, clippy, manual runs) -> Tasks 1, 3, 4, 5, 6.

---

### Task 1: State scaffolding — scroll handles and filter string (tasks.md 1.1)

**Files:**
- Modify: `src/ui/review_state.rs:57-67` (`ReviewState` struct + derive)
- Modify: `src/ui/review_state.rs:168-170` (`set_selection`)
- Test: `src/ui/review_state.rs:287-401` (`mod tests`)

**Interfaces:**
- Consumes: nothing new (uses `gpui::ScrollHandle`, `gpui::Entity`, `gpui::Subscription` by full path, no imports needed).
- Produces (used by Tasks 2-5):
  - `ReviewState::files_scroll: gpui::ScrollHandle`
  - `ReviewState::drafts_scroll: gpui::ScrollHandle`
  - `ReviewState::file_filter: String`
  - `ReviewState::filter_input: Option<gpui::Entity<gpui_component::input::InputState>>`
  - `ReviewState::filter_sub: Option<gpui::Subscription>`
  - `ReviewState::set_selection` clears `file_filter`
  - `ReviewState::matches_file_filter(path: &str, needle: &str) -> bool`

- [ ] **Step 1: Write the failing tests**

Append to `mod tests` in `src/ui/review_state.rs` (after `draft_comment_can_be_updated`, before the module's closing brace):

```rust
#[test]
fn changing_selection_clears_the_file_filter() {
    let mut s = ReviewState::new();
    let repo = repo();
    s.set_selection(&repo, "main".into(), "feat/x".into());
    s.file_filter = "widget".to_string();
    s.set_selection(&repo, "main".into(), "feat/y".into());
    assert!(
        s.file_filter.is_empty(),
        "a stale filter must never hide a fresh range's files"
    );
}

#[test]
fn file_filter_matches_case_insensitive_substring() {
    assert!(ReviewState::matches_file_filter(
        "src/ui/panel_review.rs",
        "panel"
    ));
    assert!(ReviewState::matches_file_filter(
        "src/ui/panel_review.rs",
        "PANEL"
    ));
    assert!(ReviewState::matches_file_filter(
        "src/ui/panel_review.rs",
        "ui/panel"
    ));
    assert!(!ReviewState::matches_file_filter(
        "src/ui/panel_review.rs",
        "github"
    ));
    assert!(ReviewState::matches_file_filter("src/ui/panel_review.rs", ""));
    assert!(ReviewState::matches_file_filter(
        "src/ui/panel_review.rs",
        "   "
    ));
}
```

Note: the existing `repo()` helper at `src/ui/review_state.rs:341-346` is reused as-is.

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p tty7 review_state 2>&1 | tail -20`
Expected: FAIL — `file_filter` and `matches_file_filter` do not exist (`error[E0609]` / `cannot find function`).

- [ ] **Step 3: Write minimal implementation**

In `src/ui/review_state.rs`, replace the struct derive and fields (lines 57-67):

```rust
#[derive(Default)]
pub struct ReviewState {
    drafts: HashMap<(String, String, String), ReviewDraft>,
    sends: Vec<ReviewSend>,
    branches: HashMap<RepoKey, BranchList>,
    selection: HashMap<RepoKey, (String, String)>,
    file_lists: HashMap<(RepoKey, String, String), FileListCache>,
    /// The draft open in the Review tab editor, if any, with its input box.
    editing: Option<(String, String, String)>,
    edit_box: Option<Entity<InputState>>,
    /// Independent scroll positions for the Review tab's two bounded regions.
    /// Per panel, not per repo key: handles are view state, and per-key
    /// handles would reset scroll on every re-render lookup.
    pub files_scroll: gpui::ScrollHandle,
    pub drafts_scroll: gpui::ScrollHandle,
    /// The file list filter text. Cleared by `set_selection` so a stale
    /// filter never hides a fresh range's files.
    pub file_filter: String,
    /// The filter box. `None` until the panel has been rendered once: an
    /// `InputState` needs a real window to be created in, and this struct is
    /// built by `Default` alongside the rest of `Tty7App`.
    pub filter_input: Option<Entity<InputState>>,
    pub filter_sub: Option<gpui::Subscription>,
}
```

Why the derive changes from `#[derive(Clone, Debug, Default)]` to `#[derive(Default)]`: `gpui::ScrollHandle`, `Entity<InputState>`, and `gpui::Subscription` are not `Clone`/`Debug`. This matches `GitHubPanelState` (`src/ui/github/mod.rs:138-139`, `#[derive(Default)]` with two `ScrollHandle`s) and `ScmPanelState` (`src/ui/scm/state.rs:55-56`, `#[derive(Default)]` with `Entity<InputState>` + `Subscription` + `ScrollHandle`). Grep confirms no caller clones or debug-formats `ReviewState` itself (only `ReviewDraft` and `FileListCache`, which keep their derives).

In `set_selection` (`src/ui/review_state.rs:168-170`), clear the filter:

```rust
pub fn set_selection(&mut self, repo: &RepoKey, base: String, head: String) {
    self.selection.insert(repo.clone(), (base, head));
    self.file_filter.clear();
}
```

Note: `set_selection` has no `Window`, so it cannot touch the `filter_input` entity text. The render-time filter row (Task 4) heals the box by writing `file_filter` back into the input when they diverge after a reset.

Add the pure match helper next to `pair_is_valid` (`src/ui/review_state.rs:271-276`):

```rust
/// Whether `path` survives the file list filter: a case-insensitive
/// substring match. Blank (empty or whitespace-only) matches everything.
pub fn matches_file_filter(path: &str, needle: &str) -> bool {
    let needle = needle.trim().to_lowercase();
    needle.is_empty() || path.to_lowercase().contains(&needle)
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p tty7 review_state 2>&1 | tail -10`
Expected: PASS — all `review_state` tests green, including the two new ones.

- [ ] **Step 5: Commit**

```bash
git add src/ui/review_state.rs
git commit -m "feat(review): add scroll handles and file filter state with reset on selection change"
```

---

### Task 2: Layout scaffolding — pinned header plus two bounded regions (tasks.md 1.2)

**Files:**
- Modify: `src/ui/panel_review.rs:32-57` (`render_panel_review` root)
- Modify: `src/ui/panel_review.rs:99-132` (`review_branch_body` signature + structure)

**Interfaces:**
- Consumes (from Task 1): `self.review.files_scroll`, `self.review.drafts_scroll`.
- Produces (used by Tasks 3-5): the three-part column layout with element ids `panel-review-files` / `panel-review-drafts` and scrollbar ids `panel-review-files-scrollbar` / `panel-review-drafts-scrollbar`; `review_branch_body` takes `window: &mut Window` (needed by Task 4's filter box creation).

- [ ] **Step 1: Make the tab root bounded**

In `render_panel_review` (`src/ui/panel_review.rs:56`), the root must be bounded or nested `flex_1` scrollers size to content and nothing scrolls (see `github_shell` at `src/ui/panel_github.rs:154-156`, whose root is `v_flex().flex_1().min_h_0()`). Change:

```rust
v_flex().flex_1().min_h_0().child(title).child(body).into_any_element()
```

Also thread `window` into the body call on line 52, since `review_branch_body` will need it for the filter input:

```rust
self.review_branch_body(host, repo, window, cx)
```

And change the `review_branch_body` signature (line 99-104) to accept it:

```rust
fn review_branch_body(
    &mut self,
    host: SharedHost,
    repo: RepoKey,
    window: &mut Window,
    cx: &mut Context<Self>,
) -> AnyElement {
```

- [ ] **Step 2: Restructure the body into pinned header + two scroll regions**

Replace the single `let mut body = v_flex().gap(px(8.)).p(px(12.));` accumulation with a three-part column. The pinned children (repo/head line, dropdowns, open row) stay as-is; the file section call and the draft loop move into scroll-regioninners built by Tasks 3-5. For this task, keep rendering the *existing* `review_files_section` output and draft rows inside the new scrollers so the change is layout-only and compiles:

```rust
let mut pinned = v_flex().flex_none().gap(px(4.));
pinned = pinned.child(
    div()
        .text_sm()
        .text_color(cx.theme().muted_foreground)
        .child(format!("{} · head: {}", repo.root.to_string_lossy(), head)),
);
pinned = pinned.child(
    v_flex()
        .gap(px(4.))
        .child(self.review_branch_dropdown(&repo, true, base.clone(), &branches, cx))
        .child(self.review_branch_dropdown(&repo, false, head.clone(), &branches, cx)),
);
pinned = pinned.child(self.review_open_row(&repo, &base, &head, valid, cx));

// Existing content in the new scrollers; layout only, no behavior change.
let file_inner: AnyElement = match valid {
    true => self.review_files_section(host.clone(), &repo, &base, &head, cx),
    false => v_flex().into_any_element(),
};
let file_scroll = div()
    .id("panel-review-files")
    .flex_1()
    .min_h_0()
    .overflow_y_scroll()
    .track_scroll(&self.review.files_scroll)
    .child(file_inner);
let file_region = crate::ui::scrollbar::over_vertical_scroll(
    "panel-review-files-scrollbar",
    file_scroll,
    &self.review.files_scroll,
);
```

`over_vertical_scroll` (not `with_vertical_scrollbar`) is the correct wrapper here: `with_vertical_scrollbar` takes `flex_1` itself and would stretch past the region's share, while `over_vertical_scroll` only lays the bar over a scroll area that already carries its own height (`src/ui/scrollbar.rs:59-84`). The draft region follows the same shape with a `min_h` floor (Task 5 fills it; here wrap the existing draft rows verbatim).

The body root becomes bounded:

```rust
v_flex()
    .flex_1()
    .min_h_0()
    .gap(px(8.))
    .p(px(12.))
    .child(pinned)
    .child(file_region)
    .child(draft_region)
    .into_any_element()
```

No `render_right_panel` changes (design Architecture: nesting `flex_1` children inside the existing column divides space correctly once the tab root is bounded).

- [ ] **Step 3: Run check to verify it compiles**

Run: `cargo check -p tty7 2>&1 | tail -5`
Expected: clean (zero errors; warnings acceptable if pre-existing).

- [ ] **Step 4: Commit**

```bash
git add src/ui/panel_review.rs
git commit -m "feat(review): split review tab into pinned header plus bounded file and draft regions"
```

---

### Task 3: File list scrolls, toggle removed (tasks.md 2.1)

**Files:**
- Modify: `src/ui/panel_review.rs:463-626` (`review_files_section`)
- Modify: `src/ui/review_state.rs:69-78` (delete `unfolded` field on `FileListCache`)
- Modify: `src/ui/review_state.rs:177-180` (delete `toggle_unfolded`)
- Test: manual compile via `cargo check -p tty7`

**Interfaces:**
- Consumes (from Task 2): the `panel-review-files` scroller in `review_branch_body`.
- Produces (used by Task 4): `review_files_section` renders *all* snapshot files (unfiltered for now; Task 4 adds the filter) with unchanged row rendering and click-to-open logic; header data (`total` count) available for Task 4's `Files (N of M)` header.

- [ ] **Step 1: Rewrite `review_files_section` without truncation**

Keep the loading placeholder, error line, and snapshot-missing early return exactly as they are (`src/ui/panel_review.rs:477-498`). Replace everything from `let total = snap.files.len();` through the toggle block (lines 499-624) with a loop over all files:

```rust
let mono = cx.theme().mono_font_family.clone();
for (i, file) in snap.files.iter().enumerate() {
    // ... row rendering verbatim as today (lines 504-597), unchanged ...
}
section.into_any_element()
```

Concretely: delete `let unfolded = entry.unfolded;`, change `.take(shown)` to iterate all files, and delete the entire `if total > shown { ... "panel-review-files-toggle" ... }` block (lines 599-624). The per-row code (badge, name/dir split, +/- counts, `on_click` opening the cached snapshot via `open_ready_range_diff` with `open_diff_overlay` fallback) stays byte-for-byte identical.

- [ ] **Step 2: Delete the now-dead `unfolded` state**

In `src/ui/review_state.rs`, remove `pub unfolded: bool,` from `FileListCache` and delete the whole `toggle_unfolded` method:

```rust
pub fn toggle_unfolded(&mut self, repo: &RepoKey, base: &str, head: &str) {
    let entry = self.file_list_entry(repo, base, head);
    entry.unfolded = !entry.unfolded;
}
```

Grep first to confirm no other users: `toggle_unfolded` and `.unfolded` appear only in `src/ui/review_state.rs` and the deleted `src/ui/panel_review.rs` toggle block.

- [ ] **Step 3: Run check to verify it passes**

Run: `cargo check -p tty7 2>&1 | tail -5`
Expected: clean — in particular, no `never read` / `never used` dead-code errors for `unfolded`.

- [ ] **Step 4: Commit**

```bash
git add src/ui/panel_review.rs src/ui/review_state.rs
git commit -m "feat(review): scroll all diff files, remove 8-file truncation toggle"
```

---

### Task 4: File filter input with `Files (N of M)` (tasks.md 2.2)

**Files:**
- Modify: `src/ui/panel_review.rs` (add `review_filter_row` + `review_file_filter_query`, filter in `review_files_section`, `Files (N of M)` header pinned above the file scroller)
- Test: unit test from Task 1 (`matches_file_filter`) + manual run with a many-file range

**Interfaces:**
- Consumes (from Task 1): `file_filter`, `filter_input`, `filter_sub`, `matches_file_filter`, `set_selection` reset.
- Produces: `review_filter_row(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement`; `review_file_filter_query(&self, cx: &gpui::App) -> Option<String>`; filtered rows + pinned `Files (N of M)` header element.

- [ ] **Step 1: Add the filter row and query, mirroring `scm_filter_row`**

Add these methods next to `review_files_section`, following `src/ui/scm/panel.rs:1471-1516` exactly (lazy creation + `Change` subscription that repaints):

```rust
/// The filter over the diff file list, drawn in the pinned header so it
/// stays in reach however far down the list has been scrolled.
fn review_filter_row(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
    let input = match self.review.filter_input.clone() {
        Some(input) => input,
        None => {
            let input = cx.new(|cx| {
                gpui_component::input::InputState::new(window, cx)
                    .placeholder(t(L10nKey::SearchFiles))
            });
            // An `InputState` is its own entity; without this the list
            // would never hear about the text typed into it.
            self.review.filter_sub =
                Some(cx.subscribe_in(&input, window, |this, input, ev, _window, cx| {
                    if matches!(
                        ev,
                        gpui_component::input::InputEvent::Change
                    ) {
                        this.review.file_filter =
                            input.read(cx).value().to_string();
                        cx.notify();
                    }
                }));
            self.review.filter_input = Some(input.clone());
            input
        }
    };
    // `set_selection` clears `file_filter` without a `Window`, so it cannot
    // clear the box itself. Heal the divergence here: after a repo/base/head
    // switch the box shows the fresh (empty) filter instead of stale text.
    // The clone goes into a local first: `input.update` borrows through
    // `self`, so reading `self.review` inside the closure would not compile.
    let current = self.review.file_filter.clone();
    if input.read(cx).value().to_string() != current {
        input.update(cx, |state, cx| state.set_value(&current, window, cx));
    }
    self.panel_search(&input, cx)
}
And the query helper, mirroring `scm_filter_query`:

```rust
/// The filter box's text, lowercased once, or `None` when there is nothing
/// left to filter by.
fn review_file_filter_query(&self, cx: &gpui::App) -> Option<String> {
    let input = self.review.filter_input.as_ref()?;
    let needle = input.read(cx).value().trim().to_lowercase();
    (!needle.is_empty()).then_some(needle)
}
```

Required imports at the top of `src/ui/panel_review.rs`: the file already imports `gpui::{... Window ...}` and `L10nKey, t`. `L10nKey::SearchFiles` is verified to exist (used at `src/ui/sftp.rs:208-209`). `Input`/`InputState` are referenced by full path (`gpui_component::input::...`) so no new `use` is needed; `panel_search` is an existing `Tty7App` method (`src/ui/right_panel.rs:856-913`) rendering `Input::new(input).appearance(false).xsmall().cleanable(true)` — the `panel_search` well idiom with focus/clear behavior, exactly what the design prescribes.

Placement: draw `self.review_filter_row(window, cx)` in the pinned header from Task 2 (after the open row), only when `valid` is true (no range, no filter).

- [ ] **Step 2: Filter the rows and add the `Files (N of M)` header**

In `review_files_section`, after `let Some(snap) = entry.snapshot else ...`, compute:

```rust
let total = snap.files.len();
let needle = self.review_file_filter_query(cx);
let shown: Vec<_> = snap
    .files
    .iter()
    .filter(|f| match &needle {
        Some(n) => f.path.to_lowercase().contains(n),
        None => true,
    })
    .collect();
```

The inline loop above lowercases the needle once and each path once, which is equivalent to the unit-tested `matches_file_filter` rule and also handles the `None` (no filter) case. Keep the loop inline as written.

Loop `for (i, file) in shown.iter().enumerate()` with unchanged row code. The inline loop lowercases the needle once and each path once; the unit-tested `matches_file_filter` in Task 1 documents the same case-insensitive-substring rule. When the filter matches nothing, render this muted line inside the scroll region instead of an empty box:

```rust
if shown.is_empty() {
    section = section.child(
        div()
            .text_sm()
            .px(px(8.))
            .text_color(cx.theme().muted_foreground)
            .child("No files match the filter."),
    );
}
```

Hardcoded English matches this file's existing convention.

The header goes in the pinned area (Task 2's `pinned` block), above the file scroller, so the count never scrolls away:

```rust
pinned = pinned.child(
    div()
        .flex_none()
        .text_sm()
        .px(px(8.))
        .text_color(cx.theme().muted_foreground)
        .child(format!("Files ({} of {})", shown_count, total_count)),
);
```

`shown_count`/`total_count` come from reading the cached snapshot without probing: `self.review.file_list(&repo, &base, &head).and_then(|e| e.snapshot.clone())`, applying the same filter. Only when `valid` and a snapshot exists; otherwise omit the header (loading/error placeholders already cover those states, rendered in the pinned area per the design edge cases).

- [ ] **Step 3: Run check and unit tests**

Run: `cargo check -p tty7 2>&1 | tail -5`
Expected: clean.
Run: `cargo test -p tty7 review_state 2>&1 | tail -5`
Expected: PASS.

- [ ] **Step 4: Manual verification with a many-file range**

Open a branch range with 50+ files. Confirm: file list scrolls inside its region; typing in the filter narrows the list with a correct `Files (N of M)` count; the `x` clear affordance (from `cleanable(true)`) restores the full list; switching base/head clears the filter text.

- [ ] **Step 5: Commit**

```bash
git add src/ui/panel_review.rs
git commit -m "feat(review): filter diff files by path with Files (N of M) count"
```

---

### Task 5: Draft list reservation and scroll (tasks.md 3.1)

**Files:**
- Modify: `src/ui/panel_review.rs:133-289` (extract draft loop into `review_drafts_section`, wrap in draft scroller with reserved minimum height)

**Interfaces:**
- Consumes (from Tasks 1-2): `self.review.drafts_scroll`, the `panel-review-drafts` region slot.
- Produces: `review_drafts_section(&mut self, cx: &mut Context<Self>) -> AnyElement` returning the existing draft rows (20-item cap, editing box, send flow) unchanged; draft region keeps header + first rows visible with a long file list.

- [ ] **Step 1: Extract the draft rows into `review_drafts_section`**

Move the `for (i, d) in self.review.all_drafts_sorted().iter().take(20).enumerate() { ... }` loop (currently `src/ui/panel_review.rs:133-278`, including the `is_editing` box, Save / Send-to-new-agent flow, and per-row jump/edit/delete handlers) verbatim into a new method returning the rows in a `v_flex`:

```rust
/// The draft rows behind the draft scroller: up to 20, with the inline
/// editor and send flow exactly as before. Only the wrapper scrolls; the
/// rows themselves are untouched.
fn review_drafts_section(&mut self, cx: &mut Context<Self>) -> AnyElement {
    let mut section = v_flex().gap(px(2.));
    for (i, d) in self.review.all_drafts_sorted().iter().take(20).enumerate() {
        // ... loop body verbatim, `section = section.child(row)` ...
    }
    section.into_any_element()
}
```

Keep the existing 20-item cap, editing box, and send flow untouched (design Components). The `"Sent to agent: {}"` line stays pinned at the bottom of the tab (flex_none), not inside the scroller.

- [ ] **Step 2: Wrap it in a reserved-height scroll region**

In `review_branch_body`, build the draft region with a guaranteed floor so the file list can never push it off-screen, mirroring the file region's shape:

```rust
let draft_scroll = div()
    .id("panel-review-drafts")
    .flex_1()
    .min_h(px(140.))
    .overflow_y_scroll()
    .track_scroll(&self.review.drafts_scroll)
    .child(self.review_drafts_section(cx));
let draft_region = crate::ui::scrollbar::over_vertical_scroll(
    "panel-review-drafts-scrollbar",
    draft_scroll,
    &self.review.drafts_scroll,
);
```

`min_h(px(140.))` is the design's 120-160px reserved floor; `flex_1` lets it take its share of remaining space; `over_vertical_scroll` overlays the bar without changing the measured layout. Few drafts collapse to content (GPUI only scrolls overflowing content, so no phantom scrollbars); 20 drafts scroll inside the region.

- [ ] **Step 3: Run check**

Run: `cargo check -p tty7 2>&1 | tail -5`
Expected: clean.

- [ ] **Step 4: Manual verification**

With a long file list on screen, confirm the draft section header plus first rows are visible without scrolling the panel, and that 20 drafts scroll inside the draft region.

- [ ] **Step 5: Commit**

```bash
git add src/ui/panel_review.rs
git commit -m "feat(review): reserve draft space and scroll drafts independently"
```

---

### Task 6: Full test suite and clippy (tasks.md 3.2)

**Files:**
- Fix only what `cargo test -p tty7` or `cargo clippy -p tty7` newly flags in `src/ui/review_state.rs` / `src/ui/panel_review.rs`. No refactoring, no scope expansion.

**Interfaces:**
- Consumes: all Tasks 1-5.
- Produces: green `cargo test -p tty7` and `cargo clippy -p tty7` with no new failures.

- [ ] **Step 1: Run the full test suite**

Run: `cargo test -p tty7 2>&1 | tail -15`
Expected: PASS across the crate. If a failure appears, check whether it exists on the base ref (`git stash` + rerun, or `git status` to confirm the failing area is untouched by this change); only fix failures caused by Tasks 1-5.

- [ ] **Step 2: Run clippy**

Run: `cargo clippy -p tty7 2>&1 | tail -15`
Expected: clean for the touched files. Likely lints to watch for: unused imports after the toggle deletion, needless borrows in the filter heal block, `len() == 0` vs `is_empty()` in tests.

- [ ] **Step 3: Commit any fixes (or nothing if green)**

```bash
git add -u
git commit -m "chore(review): fix test and clippy findings for scroll and filter"
```

Skip this commit if both commands were already green.
