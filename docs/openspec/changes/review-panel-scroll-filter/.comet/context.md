# Comet Design Handoff

- Change: review-panel-scroll-filter
- Phase: design
- Mode: compact
- Context hash: 272c933d372597bd48eeb04b113b615a5548897c426e56688540822bff03034c

Generated-by: comet-handoff.sh
Task hash policy: task-content-v1. Read tasks.md for live completion; excerpts are design-time context.

OpenSpec remains the canonical capability spec. This handoff is a deterministic, source-traceable context pack, not an agent-authored summary.

## docs/openspec/changes/review-panel-scroll-filter/proposal.md

- Source: docs/openspec/changes/review-panel-scroll-filter/proposal.md
- Lines: 1-25
- SHA256: b31a13eb43768cd85d442ed578e29c183c51898d6133f1eec8b29f38955283ca

```md
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

```

## docs/openspec/changes/review-panel-scroll-filter/design.md

- Source: docs/openspec/changes/review-panel-scroll-filter/design.md
- Lines: 1-61
- SHA256: 08efc27c4034e9eca4db67e40c43c98416dbef56fc0be30c75442cd0d8170c53

```md
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

```

## docs/openspec/changes/review-panel-scroll-filter/tasks.md

- Source: docs/openspec/changes/review-panel-scroll-filter/tasks.md
- Lines: 1-14
- SHA256: 9029344ff1de86c74230cd73e3bc3caf66aec5a39a92cc8f9ecefc377a5c8f79

```md
## 1. State and layout scaffolding

- [ ] 1.1 Add `files_scroll`, `drafts_scroll`, and `file_filter` to `ReviewState` with filter reset on repo/base/head change, and verify `cargo test -p tty7 review_state` passes
- [ ] 1.2 Restructure `review_branch_body` into pinned header plus bounded file and draft scroll regions, and verify `cargo check -p tty7` passes

## 2. File list scroll and filter

- [ ] 2.1 Render all (filtered) files inside the file scroll region with `overflow_y_scroll + track_scroll + with_vertical_scrollbar`, removing the 8-file truncation toggle, and verify `cargo check -p tty7` passes
- [ ] 2.2 Add the file filter input with case-insensitive path substring match, `Files (N of M)` count, and clear affordance, and verify filtering behavior in a manual run with a many-file range

## 3. Draft list reservation and scroll

- [ ] 3.1 Give the draft section a reserved minimum height and render draft rows inside the draft scroll region, and verify drafts stay visible with a long file list in a manual run
- [ ] 3.2 Run `cargo test -p tty7` and `cargo clippy -p tty7` and verify no new failures

```

## docs/openspec/changes/review-panel-scroll-filter/.openspec.yaml

- Source: docs/openspec/changes/review-panel-scroll-filter/.openspec.yaml
- Lines: 1-2
- SHA256: d71302f07db183bc30ff75198cd90cb17632c1d016043d0f43248f117bdc2e26

```md
schema: spec-driven
created: 2026-10-06

```

## docs/openspec/changes/review-panel-scroll-filter/specs/review-panel/spec.md

- Source: docs/openspec/changes/review-panel-scroll-filter/specs/review-panel/spec.md
- Lines: 1-51
- SHA256: 9ac5bee171a4f2eb358d744cb2419a3d195af500c72d28b3eebc89de44617d6d

```md
## Purpose

The Review tab keeps long branch-diff file lists navigable and unsent review drafts reachable by scrolling each list in its own bounded region and filtering files by path.

## ADDED Requirements

### Requirement: Diff file list scrolls in a bounded region

The system SHALL render the Review tab diff file list inside a vertically scrollable region with a bounded height, so a large number of files does not push content below it off-screen.

#### Scenario: Many files stay scrollable

- **WHEN** the branch range contains more files than fit the visible area
- **THEN** the file list region scrolls vertically and entries beyond the visible area remain reachable

#### Scenario: Few files show without clipping

- **WHEN** the branch range contains only a few files
- **THEN** all files are visible without scrolling and no empty scroll region is shown

### Requirement: Diff file list can be filtered by path

The system SHALL provide a text filter for the Review tab diff file list that narrows visible files by path substring match.

#### Scenario: Filter narrows the list

- **WHEN** the user types text into the file filter
- **THEN** only files whose path contains the filter text remain visible

#### Scenario: Clearing the filter restores the list

- **WHEN** the user clears the filter text
- **THEN** the full file list is visible again

### Requirement: File list shares space with the draft list

The system SHALL allocate Review tab vertical space so the diff file list does not monopolize the panel: the draft section keeps a visible reserved portion even when the file list is long.

#### Scenario: Drafts stay visible with a long file list

- **WHEN** the file list is long enough to need scrolling
- **THEN** the draft section header and at least the first draft rows remain visible without scrolling the whole panel

### Requirement: Draft list scrolls in a bounded region

The system SHALL render the Review tab draft list inside a vertically scrollable region with a bounded height, so many drafts remain reachable.

#### Scenario: Many drafts stay reachable

- **WHEN** there are more drafts than fit the draft region
- **THEN** the draft region scrolls vertically and every draft remains reachable

```
