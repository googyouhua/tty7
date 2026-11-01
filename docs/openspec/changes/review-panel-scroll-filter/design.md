## Context

See proposal.md for motivation. Current state (`src/ui/panel_review.rs`):

- `render_panel_review` returns `v_flex(title, body)` with no scroll container, unlike Info (`panel_scroll`) or GitHub (`github_shell`).
- `review_files_section` truncates to 8 of N files behind a "Show all" toggle; when unfolded with many files the section grows unbounded and pushes drafts off-screen.
- Draft rows (up to 20) are appended to the same unbounded `body`, so they are only reachable if the file section above them stays short.

Constraints: GPUI layout — scrollable regions need `flex_1 + min_h_0 + overflow_y_scroll + track_scroll(ScrollHandle)` plus `with_vertical_scrollbar` for affordance. The tab's root is a `flex_1 + min_h_0` column (see `render_right_panel`), so nested `flex_1` children divide space correctly.

## Goals / Non-Goals

**Goals:**

- File list and draft list each scroll independently inside bounded regions.
- File list filterable by path substring.
- File section cannot push the draft section off-screen.

**Non-Goals:**

- No change to diff overlay, branch resolution, draft data model, or send flow.
- No virtualized rows; lists are tens-to-hundreds of rows, plain scroll suffices.
- No persisted filter text across repo/base/head switches (filter resets; avoids stale-filter confusion).

## Decisions

### 1. Two independent scrollers under a pinned header (over whole-panel scroll)

Follow the `github_shell` pattern (`src/ui/panel_github.rs:130`): pinned controls on top (branch dropdowns, open row, filter input, section headers), then two scroll regions sharing the remaining height via `flex` proportions with `min_h_0`.

- File region: `flex_1` (grows) with a `max_h` cap so it cannot take everything.
- Draft region: `flex_none` minimum reserved height plus `flex` share, so its header and first rows are always visible; scrolls internally beyond that.
- Alternative considered: wrap the whole tab in `panel_scroll` (Info pattern). Rejected — one shared scroller still lets a long file list push drafts far down; the requirement is drafts *visible*, not merely *reachable by scrolling*.

### 2. Dedicated `ScrollHandle`s in `ReviewState` (over reusing `right_panel.scroll`)

Add `files_scroll` and `drafts_scroll` handles (per panel, not per repo key — handles are view state, and per-key handles would reset scroll on every re-render lookup). Each region gets `overflow_y_scroll + track_scroll + with_vertical_scrollbar`.

- Alternative considered: reuse `right_panel.scroll`. Rejected — that handle belongs to Info's whole-body scroller; sharing it across tabs/regions couples scroll positions.

### 3. Filter as render-time substring match over probed snapshot

Store one `file_filter: String` in `ReviewState` (cleared on repo/base/head change). Render filters `snap.files` by case-insensitive path substring; header shows `Files (N of M)` plus a clear affordance. No new probe or cache key — filtering is a view over the existing `FileListCache.snapshot`.

- Alternative considered: cache filtered lists per filter text. Rejected — unnecessary; filtering a few hundred paths per frame is trivial.

### 4. Remove the 8-file truncation toggle

With a bounded scroller the "Show 8 / Show all" toggle (`unfolded` in `FileListCache`) no longer serves a purpose; show all (filtered) files in the scroller and delete the toggle path. Keeps one code path instead of two.

- Alternative considered: keep the toggle above the scroller. Rejected — redundant chrome; the scroller already solves the length problem.

## Risks / Trade-offs

- [Risk] Two nested scrollers in a narrow panel can feel cramped → Mitigation: file region capped (~55-60% of remaining height), draft region guaranteed minimum (~120-160px); tune constants against real data.
- [Risk] Removing the truncation toggle changes existing UX → Mitigation: scroller preserves the same rows and click behavior; only the toggle chrome disappears.
- [Risk] Filter input steals keyboard focus from panel shortcuts → Mitigation: use the same `panel_search`-stylewell pattern (as Files tab) with `cleanable(true)` so the filter is escapable/clearable.

## Migration Plan

No migration. Pure UI change behind existing tab; no config, data, or API changes. Rollback is a revert.
