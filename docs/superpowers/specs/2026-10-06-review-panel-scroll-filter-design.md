---
comet_change: review-panel-scroll-filter
role: technical-design
canonical_spec: openspec
---

# Review Panel Scroll + Filter — Technical Design

## Context

The Review tab (`src/ui/panel_review.rs`) renders branch picker, diff file list, and draft list in one unbounded `v_flex` with no scroll container. `review_files_section` truncates to 8 files behind a toggle; unfolded long lists push drafts off-screen. Draft state lives in `ReviewState` (`src/ui/review_state.rs`): `file_lists` cache per `(repo, base, head)`, global `drafts` map. Reference patterns: `github_shell` (`src/ui/panel_github.rs:130`, pinned + independent scroller with own `ScrollHandle`) and `panel_scroll` (`src/ui/right_panel.rs:915`, whole-body scroll).

## Architecture

Restructure `review_branch_body` into three vertical parts inside the tab's `flex_1 + min_h_0` column:

1. **Pinned header** (`flex_none`): repo/head line, base/head dropdowns, open row, file filter well, section headers.
2. **File region** (`flex_1`, `min_h_0`, `max_h` ~60% of remaining): `overflow_y_scroll + track_scroll(files_scroll)` wrapped in `with_vertical_scrollbar`.
3. **Draft region** (reserved `min_h` ~120–160px + remaining `flex` share): same scroll wrapping with `drafts_scroll`.

No changes to `render_right_panel` or the tab root; nesting `flex_1` children inside the existing column divides space correctly.

## Components

- `ReviewState`: add `files_scroll: ScrollHandle`, `drafts_scroll: ScrollHandle`, `file_filter: String`. Filter is view state, one per panel; reset it wherever selection changes (`set_selection`) so a stale filter never hides a fresh range's files.
- `review_files_section`: filter `snap.files` by case-insensitive substring on `file.path` at render; header becomes `Files (N of M)`; delete the `shown`/`unfolded` truncation and the `panel-review-files-toggle` row. Row rendering and click-to-open logic unchanged.
- Draft section: extract from the shared `body` builder into its own function returning a bounded scroller; keep the existing 20-item cap, editing box, and send flow untouched.
- Filter input: reuse the `panel_search` well idiom (`Input`, `appearance(false)`, `xsmall`, `cleanable(true)`) for consistent focus/clear behavior.

## Data Flow

Probe path unchanged (`ensure_review_files` → `FileListCache.snapshot`). Filter applies downstream at render: `snapshot.files → filter → rows`. No new cache keys, no extra git processes. Draft flow (`upsert_draft` / `remove_draft` / `all_drafts_sorted`) untouched.

## Edge Cases

- Empty snapshot / loading / probe error: same placeholders as today, rendered above the scrollers (pinned area), so they never depend on scroll position.
- Filter matches nothing: show a muted "No files match" line in the file region instead of an empty box.
- Few files / few drafts: regions collapse to content; no phantom scrollbars (GPUI only scrolls overflowing content).
- Panel resize: `flex` proportions adapt; `max_h` cap keeps the draft floor intact at small heights.

## Testing

- Unit: `cargo test -p tty7 review_state` covers filter-reset-on-selection-change (new test for `set_selection` clearing `file_filter`).
- Static: `cargo check -p tty7`, `cargo clippy -p tty7` clean.
- Manual: branch range with 50+ files — file list scrolls, typing in the filter narrows with correct `N of M`, clearing restores; drafts header plus first rows visible without scrolling the panel; 20 drafts scroll inside the draft region.

## Risks

- Cramped dual scrollers at narrow widths → tuned caps and minimums; follow-up tuning is a constants-only change.
- Toggle removal surprises muscle memory → accepted by user in brainstorming; rows and clicks behave identically.
