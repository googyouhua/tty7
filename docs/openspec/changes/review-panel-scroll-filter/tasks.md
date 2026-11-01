## 1. State and layout scaffolding

- [ ] 1.1 Add `files_scroll`, `drafts_scroll`, and `file_filter` to `ReviewState` with filter reset on repo/base/head change, and verify `cargo test -p tty7 review_state` passes
- [ ] 1.2 Restructure `review_branch_body` into pinned header plus bounded file and draft scroll regions, and verify `cargo check -p tty7` passes

## 2. File list scroll and filter

- [ ] 2.1 Render all (filtered) files inside the file scroll region with `overflow_y_scroll + track_scroll + with_vertical_scrollbar`, removing the 8-file truncation toggle, and verify `cargo check -p tty7` passes
- [ ] 2.2 Add the file filter input with case-insensitive path substring match, `Files (N of M)` count, and clear affordance, and verify filtering behavior in a manual run with a many-file range

## 3. Draft list reservation and scroll

- [ ] 3.1 Give the draft section a reserved minimum height and render draft rows inside the draft scroll region, and verify drafts stay visible with a long file list in a manual run
- [ ] 3.2 Run `cargo test -p tty7` and `cargo clippy -p tty7` and verify no new failures
