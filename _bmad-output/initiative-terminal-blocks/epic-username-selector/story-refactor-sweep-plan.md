---
title: 'Refactor sweep'
type: 'chore'
ticket: 6
created: '2026-10-11'
status: 'built'
baseline_revision: 'd6bae08a829d70b89fa11dfdc702f6b5db9d0967'
route: 'oneshot'
route_source: 'auto'
risk: 'low'
review: 'quick'
review_source: 'pinned'
lenses_ran: [quick]
lenses_ran: []
review_loop_iteration: 0
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Entries 1–5 may have left formatting drift, new lints, or uncommitted bookkeeping across the epic's touched files.

**Approach:** Audit the touched surface (fmt, clippy on touched files, green-suite re-run) and land only sweep-scoped fixes — no pushed-out scope, no pre-existing lint cleanup, no external breakage repair.

</frozen-after-approval>

## Implementation Notes

Sweep audit outcome: one fmt reflow in instance_picker.rs (Change-guard closure shape); zero new clippy findings on touched files (remaining warnings pre-existing: t_select unused, collapsible ifs, Windows-only passthrough field); core instance suite re-run green; no deferred code cleanups existed (all review findings patched inline in their entries). Also committing the inception-time tickets.toml epic registration that was still uncommitted.

## Review Triage Log

- quick lens: no findings (fmt-only diff). Verification all green: fmt clean, core 15/15, bin check clean, no new clippy on touched files.

## Verification

**Commands:**
- `cargo fmt --check -p tty7 -p tty7-core` -- expected: clean (verified)
- `cargo test -p tty7-core core::instance` -- expected: green (verified 15/15)
- `cargo check --bin tty7-app` -- expected: clean (verified)
