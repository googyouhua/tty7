---
title: 'end-to-end picker suite'
type: 'feature'
ticket: 7
created: '2026-10-11'
status: 'built'
baseline_revision: 'cfa5ecb7cc6079eee2341ec295a1edb8891f308e'
route: 'oneshot'
route_source: 'auto'
risk: 'low'
review: 'quick'
review_source: 'pinned'
lenses_ran: [quick, quick]
lenses_ran: []
review_loop_iteration: 0
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Entries 1–5 are verified piecemeal (unit tests, harnesses, inspection); nothing exercises the real public-API lifecycle the picker drives — sandbox-rooted list, select-memory, create-enter, guarded delete with a live seat, and memory fallback — in one scripted pass.

**Approach:** Add a core integration test that sandboxes `TTY7_AS_ROOT`/`HOME` and walks the full lifecycle through the public API only, plus a short manual GUI checklist for the visual/keyboard paths automation cannot reach headless.

</frozen-after-approval>

## Implementation Notes

Oneshot route: one new test file (~130 lines) + this manual checklist. Single #[test] fn (no inter-test env races; env saved/restored inline). Unix-only live-seat section; rest portable.

Manual GUI checklist (multi-user box, bare launch):
- Window stays compact with 10+ users; dropdown scrolls internally.
- Type substring: list narrows, highlight resets; clear restores + preselect.
- Arrows wrap; Enter enters highlight; valid-unlisted Enter does not create.
- Create button enables only for valid unlisted; click creates + next launch preselects.
- × on named rows only; modal names instance; live shows report with stop/rm and no confirm; unknown needs two confirms; failed delete keeps row + memory; cancel returns focus.
- NoMatch/invalid lines appear at the right states; four-locale spot-check optional.

## Review Triage Log

- quick lens round 1, 6 findings, all verified real and patched: Sandbox Drop guard (stale-dirs cleaned, 3 subsequent green runs litter-free); live-API memory guard (InvalidInput + byte-identical) + read-side fallback (garbage→None, sentinel round-trip); Invalid variant matches; OsStr env values; flock-failure branches to Unknown-refusal; fallback covered write+read side.
- Author note: caught a real order flake mid-build (asserted alphabetical over recency-ordered list); fixed by asserting presence, ordering stays the unit tests' contract.
- quick lens round 2 on patched file: no new findings.
- Matrix/AC audit: lifecycle green 3×, fmt clean, zero warnings, core unit suite 15/15 untouched-green; GUI half remains manual per plan checklist (headless cannot reach it).

## Verification

**Commands:**
- `cargo test -p tty7-core --test username_selector_lifecycle` -- expected: green
- `cargo fmt --check -p tty7-core` -- expected: clean
