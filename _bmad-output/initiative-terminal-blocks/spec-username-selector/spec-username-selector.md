---
id: SPEC-username-selector
companions:
  - ../ux-username-selector/DESIGN.md
  - ../ux-username-selector/EXPERIENCE.md
  - ../architecture-username-selector/architecture-username-selector.md
sources: []
---

> **Canonical contract.** This SPEC and the files in `companions:` are the complete, preservation-validated contract for what to build, test, and validate. Source documents listed in frontmatter are for traceability — consult them only if you need narrative rationale or prose color this contract intentionally omits.

# Username Selector

## Why

A pain to solve: on shared machines the startup instance picker tiles every username as rows that fill the window, and there is no way to remove a stale user. Anyone on a box with a dozen users pays this on every bare launch. This spec replaces the wall of rows with a compact filter-and-pick gate plus a guarded delete.

## Capabilities

- **CAP-1**
  - **intent:** User types in the combobox to narrow the instance list to substring matches.
  - **success:** Typing filters case-insensitively per keystroke with highlight reset to the first match; clearing restores the full list; filtering alone never creates anything.
- **CAP-2**
  - **intent:** User confirms the highlighted row to enter that instance.
  - **success:** Enter or row click resolves the config dir, rewrites last-instance memory, and enters; cancel closes with no selection.
- **CAP-3**
  - **intent:** User creates a new instance through an explicit action.
  - **success:** Create button enables only for a valid, unlisted name; clicking creates `tty7-<name>`, enters it, and the next bare launch preselects it; Enter alone never creates.
- **CAP-4**
  - **intent:** User removes a stale instance without endangering a running server.
  - **success:** Live server refuses the delete and reports instance, dir, pid, stop command, and follow-up rm command; free server deletes the dir, rewrites memory to the focused row, and moves focus to the next row; unknown probe state requires a second explicit confirm; Default is never deletable; failed deletes leave row and memory untouched.
- **CAP-5**
  - **intent:** Picker reopens where the user left off.
  - **success:** Opens preselected on the remembered instance, falling back to Default when memory is missing, invalid, or names a deleted instance; memory is never written with unvalidated or deleted names.
- **CAP-6**
  - **intent:** User sees recency per row to recognise their account.
  - **success:** Each row shows relative last-used time when available and omits it otherwise; ordering is stable within one open (newest first).
- **CAP-7**
  - **intent:** User operates the whole picker without a pointer.
  - **success:** Arrows move and wrap highlight, first Esc closes the dropdown, second cancels, modal traps and returns focus, and result counts plus selection are announced.

## Constraints

- Filesystem mutation and name validation live in `tty7-core/src/core/instance.rs`; the gpui view renders and calls, never deleting paths itself.
- `validate_name` is the single validation authority; the view never re-encodes the `[a-z0-9-]` rule.
- Delete is one core op (validate, authoritative probe, unlink) behind a dir-scoped tri-state probe; symlinked instance dirs are refused and the `default` sentinel is never a path.
- Every new user-visible string ships `InstancePicker*` keys in all four locales (en, zh, ja, ru) in the same change.
- Surface detail (tokens, viewport cap, keyboard micro-rules, a11y floor, voice) is bound by the adopted UX companions; the architecture spine wins only on structural conflict.

## Non-goals

- Multi-machine server inventory and file-exported delete reports (modal-inline report only).
- Pinyin or fuzzy matching (substring only).
- Trash or recoverable delete (permanent removal semantics).
- The picker stopping or killing servers (user stops them via the reported command).

## Success signal

- A bare launch on a box with many users shows a compact picker: filter narrows, Enter enters, a new name creates through the button, and a stale user deletes only when its server is stopped — with `cargo test` covering the new core remove/probe/memory-ordering paths.
