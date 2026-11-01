# Brainstorm Summary

- Change: review-panel-scroll-filter
- Date: 2026-10-06

## Confirmed Technical Approach

Follow the `github_shell` pattern (`src/ui/panel_github.rs:130`): pinned header (branch dropdowns, open row, filter input, section headers) plus two independent scroll regions. File region `flex_1` capped at ~60% of remaining height; draft region keeps a reserved minimum height (~120-160px) and scrolls internally. Dedicated `files_scroll` / `drafts_scroll` `ScrollHandle`s in `ReviewState`; filter is a render-time case-insensitive path-substring match over the existing `FileListCache.snapshot`, reset on repo/base/head change. Remove the 8-file truncation toggle (`unfolded`).

User decisions (all recommended options accepted): remove toggle, drafts stay global (no scope expansion), file-priority space split with draft floor.

## Key Trade-offs and Risks

- Two nested scrollers in a narrow panel can feel cramped → cap file region, guarantee draft minimum; tune against real data.
- Removing the truncation toggle changes existing UX → rows and click behavior unchanged, only toggle chrome disappears.
- Filter input focus vs panel shortcuts → `panel_search`-style well with `cleanable(true)`.

## Testing Strategy

- `cargo test -p tty7 review_state`, full `cargo test -p tty7`, `cargo clippy -p tty7`.
- Manual run with a many-file branch range: file list scrolls, filter narrows with `N of M` count, drafts visible and scrollable.

## Spec Patches

None. Existing delta spec acceptance scenarios already cover the confirmed behavior.
