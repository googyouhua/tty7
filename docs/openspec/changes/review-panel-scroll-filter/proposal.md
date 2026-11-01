## Why

The Review tab becomes unusable when a branch range touches many files: the diff file list grows without scrolling, pushes the draft section off-screen, and drafts become unreachable. Users need to scan a long file list and still reach their unsent drafts.

## What Changes

- The diff file list scrolls inside its own bounded region instead of growing the whole panel.
- The diff file list gains a text filter (path substring match) so users can narrow long lists.
- The Review tab reserves visible space for the draft list; the file list no longer monopolizes the panel.
- The draft list scrolls inside its own bounded region so drafts stay reachable with many entries.

## Capabilities

### New Capabilities

- `review-panel`: Review tab file-list scrolling and filtering, file/draft space allocation, and draft-list scrolling behavior.

### Modified Capabilities

- None. No existing spec requirements change.

## Impact

- Affected code: `src/ui/panel_review.rs` (file section, draft section, tab layout), possibly `src/ui/review_state.rs` (filter state), `src/ui/right_panel.rs` (scroll container integration).
- No API, schema, or cross-module changes. No data-model changes to drafts.
