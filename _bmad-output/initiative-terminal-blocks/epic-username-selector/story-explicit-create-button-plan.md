---
title: 'explicit create button'
type: 'feature'
ticket: 4
created: '2026-10-11'
status: 'built'
baseline_revision: 'd7861f16726a1cb36f957a6e5f37fa7cd45c8962'
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

**Problem:** A valid new username typed in the filter has no way to enter the product: Enter correctly refuses to create, but no explicit create path exists yet.

**Approach:** Add an explicit create button below the dropdown, enabled only for valid unlisted names, that enters the new instance through the existing finish/apply_selection path (dir creation + memory write already live downstream).

</frozen-after-approval>

## Implementation Notes

Oneshot route: picker view + one i18n key, ~70 lines. Pure `PickerState::create_candidate(query) -> Option<String>` (Some only for trimmed validate_name-clean names absent from rows) drives enablement so it is unit-testable; click revalidates defensively then finishes Named. Invalid non-empty input keeps the existing live invalid line; empty input shows no button affordance change. New `InstancePickerCreate` key in en/zh/ja/ru (ja/ru builder-drafted, user reviews).

Decisions made while implementing:
- Creation reuses the existing finish/apply_selection path (dir creation + memory write already live downstream); no new core API.
- `Button::disabled` comes from the `Disableable` trait (added to imports); gpui-component Button has no inherent disabled method.
- Button renders only while filtering; empty input shows no create affordance.

## Review Triage Log

- quick lens, 1 finding, real and patched: module doc contradicted the new create path ("typing matches nothing never creates"); doc now names the explicit-button exception. Matrix audit: candidate Some/None rows verified by standalone harness with the real validate_name body; create-click path and i18n parity verified by inspection + compiler exhaustiveness (bin-test block pre-existing, see deferred-work.md).

## Verification

**Commands:**
- `cargo check --bin tty7-app` -- expected: clean
- `cargo test -p tty7-core core::instance` -- expected: green (no core changes; regression only)
