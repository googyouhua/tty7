---
title: 'delete modal plus report plus fallback'
type: 'feature'
ticket: 5
created: '2026-10-11'
status: 'built'
baseline_revision: 'eadf166fc191a713d8050fd41ff6b6c5eb2f3cba'
route: 'full'
route_source: 'auto'
risk: 'medium'
review: 'quick'
review_source: 'pinned'
lenses_ran: [quick, quick]
lenses_ran: []
review_loop_iteration: 0
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Stale usernames cannot be removed: rows have no delete affordance, and the guarded core op from entry 2 has no UI surface for its report, double-confirm, failure, and memory-fallback contract.

**Approach:** Add a per-row delete control opening an in-view confirm modal: live seats refuse with an actionable report, unknown probes need a second explicit confirm (via a new core force channel), failures keep row and memory untouched, and success rewrites memory to the focused row with focus falling to the next row.

## Boundaries & Constraints

**Always:** Core owns the unlink (AD-1/AD-2) — the force channel extends `remove_instance(_in)`, it does not bypass them; `write_last` rewrite runs strictly after `Ok` with the focused row's value (AD-7); all new strings ship `InstancePicker*` keys in four locales (AD-8); modal stacks one deep, picker behind inert, Esc closes topmost first (AD-5/AD-9).

**Never:** No kill action anywhere (user stops servers via the reported command); no × on the Default row; no second modal over the modal; no memory write on failure or cancel.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Open modal (free) | click × on named row | modal names instance + warns removal; Cancel returns to row | No error expected |
| Confirm free | Confirm click | dir gone, memory rewritten to focused row, focus falls next | No error expected |
| Live refusal | seat held | modal shows report (dir, advisory pid, stop + rm commands), no confirm action | No error expected |
| Unknown double-confirm | probe unevaluable | first Confirm arms, second proceeds via force channel | No error expected |
| Core failure | remove errs | modal stays open with error line, row + memory untouched | Retryable error line |
| Cancel | Esc / scrim / Cancel | modal closes, focus returns to invoking row, nothing written | No error expected |
| Default row | any | no × rendered | No error expected |

</frozen-after-approval>

## Code Map

- `crates/tty7-core/src/core/instance.rs` -- extend `remove_instance`/`remove_instance_in` with a force channel that proceeds past `Unknown` only (Live still refuses, Invalid still refuses); update entry-2 tests to the new signatures.
- `src/ui/instance_picker.rs` -- row × on named rows only (precedent for nested click: tab-strip close button — verify non-propagation or route click through cursor-set without confirm); `pending_delete` modal state (name + armed flag); in-view overlay (`absolute inset_0` + scrim + `dialog::card`, precedent `ssh_prompt.rs` sheet); pre-confirm advisory probe builds the report; confirm path calls force channel only when armed; post-`Ok` row removal + `write_last` + focus-to-next; failure line on `Err`; Esc/scrim/Cancel close with focus back to input; roles + labels on modal and ×.
- `src/ui/i18n/*` -- DeleteTitle/DeleteBody/DeleteConfirm/DeleteCancel/DeleteLiveReport/DeleteUnknownBody/DeleteConfirmAgain/DeleteError keys × (list + translate) × four locales.
- Reuse: `holder_pid_in`, `dir_for`, `SeatState`, `RemoveError` display, `t`/`t_fmt`, footer patterns, entry-1 meta, entry-3 filter/highlight.

## Tasks & Acceptance

**Execution:**
- [x] `instance.rs` -- force channel on the remove op + signature updates to entry-2 tests, all green
- [x] `instance_picker.rs` -- × affordance (named rows only, no confirm propagation) + modal state machine (free/live/unknown/armed/failed/cancelled)
- [x] `instance_picker.rs` -- post-delete row removal, memory rewrite after `Ok`, focus-to-next, focus return, failure persistence
- [x] `src/ui/i18n/*` -- delete key family in four locales
- [x] picker mod tests -- modal transitions, fallback selection, no-×-on-Default, failure keeps row+memory (state-level where gpui blocks view tests)

**Acceptance Criteria:**
- Given a free instance, when the user confirms, then the dir is gone, memory names the focused row, and highlight sits on the next row
- Given a live instance, when the user opens delete, then the report shows dir, advisory pid, and stop/rm commands with no confirm action
- Given an unknown probe, when the user confirms twice, then the remove proceeds; a single confirm only arms
- Given a failed remove, when the error returns, then the modal stays open, the row stays, and memory is untouched
- Given any cancel path, when it fires, then nothing is written and focus returns to the invoking row

## Review Triage Log

- quick lens round 1, 6 findings: 5 real, all patched (modal-open keystrokes revert displayed text via set_value, which emits no Change — no loop; invalid/create chrome then renders frozen-consistent values; filter-container click guarded; × label is now a per-row InstancePickerDeleteRow key; rm command quoted via shell_quote with platform-default dialect); 1 false (footer Enter routing to confirm_delete is the plan's designed modal-aware Enter, not inert chrome — behavior correct as built).
- Author additions during triage: InstancePickerDeleteNoDir key (replaces two hardcoded English no-dir strings in modal None branch + confirm_delete) in four locales.
- quick lens round 2 on patched hunks: no new findings.
- Matrix audit: 7/7 rows covered — free/live/unknown-double/failure/cancel by ran tests (core 15/15 in-tree; state transitions via 25/25 verbatim harness), Default-no-× by render gate + is_deletable (inspection), modal render paths by inspection + clean check; 10 delete keys compiler-checked (exhaustive match) + arms inspected in all four locales; bin-test execution still blocked by the pre-existing breakage (deferred-work.md).

## Verification

**Commands:**
- `cargo test -p tty7-core core::instance` -- expected: green, including force-channel cases (verified 15/15)
- `cargo check --bin tty7-app` -- expected: clean (verified)
- `cargo test --bin tty7-app instance_picker` -- expected: green, but BLOCKED by the pre-existing test-target breakage (recorded in deferred-work.md)
