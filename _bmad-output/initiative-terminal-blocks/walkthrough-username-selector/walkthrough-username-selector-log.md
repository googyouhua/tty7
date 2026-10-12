# Review log: walkthrough-username-selector

Target: username-selector epic on branch `dev/sync-upstream-20261011`, commit range `78cb43b0..05b789f5` (epic work `2a90a78a` through `05b789f5` plus post-build crash fix `05b789f5`; pre-epic picker fixes `78cb43b0`, `a1a7a73b` as context)

<!--
Append-only record of outcomes/decisions.
Main purpose: persisted context for resuming or analyzing
the review process. You're writing this log for consumption
by another LLM session.

Record decisions and their reasons, user constraints, unresolved
findings, and references needed to recover evidence. Link to large
results rather than copying them here.
This is NOT a transcript or internal deliberation.
The review narrative (the human-facing file) owns review blocks
and their statuses.

Log entry:
Use format below. Omit empty fields. In Result, give each finding's
disposition: fixed, accepted as-is, deferred, or open. When a
disposition changes, append a new entry; do not edit earlier ones.
Record the actual session ID supplied by the environment and an ISO
8601 timestamp with local timezone for each entry. If the session ID
is unavailable, write "unavailable" rather than inventing one.

Token budget: up to 150 tokens per entry.
The smaller the better, but do not sacrifice precision.
-->

## 1 — Orientation — scope and pace agreed

Session: ses_ecfc09912ffebioVivPN4tLO3d · Timestamp: 2026-10-12T13:12:00Z

- Action: Oriented on epic folder `_bmad-output/initiative-terminal-blocks/epic-username-selector/` (7 ticket entries 2.1–2.7 plus crash fix `05b789f5`); user set scope = full epic and pace = all at once.
- Result: Scope locked to full epic 2.1–2.7 + Surfaces-global crash fix; pre-epic picker fixes context only.
- Evidence: `epic-username-selector.md`, `tickets.toml`, commit range `78cb43b0..05b789f5`
- Open: None; proceed to narrative then present Block 1 Intent.

## 2 — Narrative — created in mandated layout

Session: ses_ecfc09912ffebioVivPN4tLO3d · Timestamp: 2026-10-12T13:14:00Z

- Action: Created `walkthrough-username-selector/walkthrough-username-selector.md` with blocks Intent / Broad strokes / Slices / Periphery, all unchecked, current-block marker on Block 1; relocated from `epic-username-selector/walkthrough-narrative.md` + `walkthrough-log.md` and deleted the old files.
- Result: Narrative ready for review; no blocks presented, no findings, no user decisions yet.
- Evidence: `walkthrough-username-selector/walkthrough-username-selector.md`
- Open: Present Block 1 Intent.

## 3 — Block 1 Test move executed 2026-10-12

Session: ses_ecfc09912ffebioVivPN4tLO3d · Timestamp: 2026-10-12T00:00:00Z

- Action: Automated re-run (core::instance 15/15, e2e 1/1, both green) plus remote GUI re-verification on 192.168.1.25 (fresh 12-user sandbox, pristine compact state with judy preselect + just-now metas re-screenshot; filter "al" narrows to Default+alice with 1-match line; sandbox cleaned after).
- Result: Block 1 verified. Observation for the record (not a finding): primary-button accent rendered blue this run vs black in the previous session — app-theme-owned, both self-consistent, no action.
- Evidence: automated re-run + remote GUI on 192.168.1.25
- Open: None for Block 1.

## 4 — Block 1 accepted by user 2026-10-12 after Test move

Session: ses_ecfbbbf9affeiTyVXQ3mc9ETtO · Timestamp: 2026-10-12T00:00:00Z

- Action: Block 1 Intent accepted by user after Test move.
- Result: Block 1 accepted; Block 2 Broad strokes now current.
- Evidence: walkthrough-username-selector.md Block 1 checked
- Open: Present Block 2.

## 5 — Block 2 Test move executed 2026-10-12

Session: ses_ecfba8c72ffeywXcd2yIX2nub7 · Timestamp: 2026-10-12T13:20:25Z

- Action: Entry-point suites re-run: core::instance 15/15 green (local), lifecycle e2e 1/1 green (local), bin instance_picker 25/25 green (remote 192.168.1.25, transplanted tree).
- Result: All green, no findings.
- Evidence: local cargo test + remote run on 192.168.1.25
- Open: None for Block 2 Test move.

## 6 — Block 2 accepted by user 2026-10-12 after Test move

Session: ses_ecfb9f653ffeoSqUrEAUZGvNVJ · Timestamp: 2026-10-12T00:00:00Z

- Action: Block 2 Broad strokes accepted by user after Test move.
- Result: Block 2 accepted; Slice 1 filter+select now current; Slices 1-4 pending (filter+select, guarded delete, memory fallback, i18n+a11y).
- Evidence: walkthrough-username-selector.md Block 2 checked, Slice 1 marked current
- Open: Present Slice 1.
Slice 1 Test move 2026-10-12 — remote filtered run covering slice-1 behaviors (filter narrows/reset/case-insensitive, highlight wraps visible-only, no-match Default-only, invalid-from-validate_name-only, esc-dropdown-layer, confirm-follows-highlight): 27 passed 0 failed (includes some scm/settings/sftp tests matched by the same filter substrings; all picker slice-1 tests green). No findings.

## 7 — Slice 1 accepted by user 2026-10-12 after Test move

Session: ses_ecfb898b0ffe7mQgXEySXQGT3L · Timestamp: 2026-10-12T00:00:00Z

- Action: Slice 1 filter+select accepted by user after Test move.
- Result: Slice 1 accepted; Slice 2 guarded delete now current.
- Evidence: walkthrough-username-selector.md Slice 1 checked, Slice 2 marked current
- Open: Present Slice 2.

## 8 — Slice 2 Test move 2026-10-12

Session: ses_ecfb7c0d9ffe5V0exgLeRAE957 · Timestamp: 2026-10-12T00:00:00Z

- Action: Slice 2 Test move 2026-10-12 — core::instance 15/15 green locally AND on remote; remote delete-modal state tests all green (complete falls next/previous/Default-landing, cancel returns + writes nothing, never-on-Default, opens unarmed, tracks under filter, esc-modal-first, unknown-arms-only, failed-keeps-row+memory). Combined with the screenshot-verified live/free modals from the entry-7 GUI session.
- Result: No findings.
- Evidence: local + remote cargo test, entry-7 GUI screenshots
- Open: None for Slice 2 Test move.

## 9 — Slice 2 accepted by user 2026-10-12 after Test move

Session: ses_ecfb76dabffeuu2jENPIfNBmuE · Timestamp: 2026-10-12T00:00:00Z

- Action: Slice 2 guarded delete accepted by user after Test move.
- Result: Slice 2 accepted; Slice 3 memory fallback now current.
- Evidence: walkthrough-username-selector.md Slice 2 checked, Slice 3 marked current
- Open: Present Slice 3.

## 10 — Slice 3 Test move 2026-10-12

Session: ses_ecfb6359affecqZpp4qR8DU2X9 · Timestamp: 2026-10-12T00:00:00Z

- Action: Slice 3 Test move 2026-10-12 — core memory 3/3 green locally; remote memory-related picker tests green within a 20-pass run (remembered row, missing/corrupt/deleted fallback, clear-restores-highlight, selections values, complete-names-live, failed-keeps).
- Result: No findings.
- Evidence: local cargo test (core memory 3/3) + remote 20-pass run on 192.168.1.25
- Open: None for Slice 3 Test move.

## 11 — Slice 3 accepted by user 2026-10-12 after Test move

Session: ses_ecfb6359affecqZpp4qR8DU2X9 · Timestamp: 2026-10-12T00:00:00Z

- Action: Slice 3 memory fallback accepted by user after Test move.
- Result: Slice 3 accepted; Slice 4 i18n+a11y now current.
- Evidence: walkthrough-username-selector.md Slice 3 checked, Slice 4 marked current
- Open: Present Slice 4.

## 12 — Slice 4 Test pre-run 2026-10-12

Session: ses_ecfb6359affecqZpp4qR8DU2X9 · Timestamp: 2026-10-12T00:00:00Z

- Action: Slice 4 Test pre-run 2026-10-12 — i18n suite 8/8 green on remote (parity, plural branches incl. new MatchCount, per-locale coverage).
- Result: No findings; ready to present Slice 4.
- Evidence: remote i18n run on 192.168.1.25
- Open: Present Slice 4.

## 13 — Slice 4 accepted by user 2026-10-12 after Test move

Session: ses_ecfb570b9ffehdihmow1t8GPYf · Timestamp: 2026-10-12T00:00:00Z

- Action: Slice 4 i18n+a11y accepted by user after Test move.
- Result: Slice 4 accepted; Block 4 Periphery now current.
- Evidence: walkthrough-username-selector.md Slice 4 checked, Periphery marked current
- Open: Present Periphery.
