---
title: 'combobox filter plus select plus keyboard plus preselect'
type: 'feature'
ticket: 3
created: '2026-10-11'
status: 'built'
baseline_revision: '908a50d51cfa56eb1a32e84fccca83722f8526a8'
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

**Problem:** The picker still tiles every instance as rows that fill the window, and typing a name bypasses selection (typed-wins) instead of filtering it.

**Approach:** Turn the picker into an input-is-filter combobox: keystrokes narrow a capped scrollable dropdown, arrows move a wrapping highlight, Enter confirms the highlighted row only, Esc closes the dropdown before cancelling, with open-time preselect and four-locale strings for the new affordances.

## Boundaries & Constraints

**Always:** `validate_name` stays the only validity rule (AD-6, never re-encoded); memory writes go through `apply_selection` only (AD-7 producer); new strings join `InstancePicker*` in en/zh/ja/ru in the same change (AD-8); visual tokens from DESIGN.md (green highlight, 6px radii, compact window).

**Never:** No create button (entry 4); no delete affordance or modal (entry 5); no core changes; no new deps; no aria-live primitive (gpui has none — closest equivalent below).

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Filter narrows | types "b" over [alice, bob, libby] | dropdown shows Default + matches, highlight resets to first match | No error expected |
| Clear restores | clears input | full list back | No error expected |
| No matches | types "zzz" | no-match hint line, no rows (Default still shown) | No error expected |
| Arrows wrap | Down past last / Up past first | highlight wraps around visible rows | No error expected |
| Enter confirms | highlight on bob | enters bob, memory rewritten | No error expected |
| Enter never creates | valid unlisted "carol" typed | nothing happens (create is entry 4) | No error expected |
| Esc layers | dropdown open, then closed | first Esc closes dropdown (text kept), second cancels window | No error expected |
| Preselect fallback | memory missing, corrupt, or names deleted instance | Default highlighted | No error expected |

</frozen-after-approval>

## Code Map

- `src/ui/instance_picker.rs` -- owns everything: `PickerState` gains query-derived `visible()` (Default always on top + substring-filtered names, case-insensitive) with highlight-over-visible + wrap; view reorders to input-top / capped dropdown (`overflow_y_scroll` + `ScrollHandle`, max ~6-8 rows, reuse `scrollbar.rs` wrapper pattern) / footer; keybindings change to combobox ownership (arrows always move highlight; Enter confirms highlight only; Esc closes dropdown first); row keeps entry-1 meta child; roles + accessible labels on list/options plus a visible count line.
- `src/ui/i18n/mod.rs` -- add `InstancePickerNoMatch`, `InstancePickerMatchCount` to `l10n_keys!` (near other picker keys ~1467).
- `src/ui/i18n/{en,zh,ja,ru}.rs` -- both `translate_*` and `translate_variant_*` arms for the two keys (MatchCount needs one/other via existing `t_plural`); ja/ru drafted, user reviews at build review per epic Notes.
- Reuse, do not reinvent: `InputState` value/Change events, `home::relative_time`, `t`/`t_plural`, footer Close/Enter, `apply_selection` memory path.

## Tasks & Acceptance

**Execution:**
- [ ] `src/ui/instance_picker.rs` -- PickerState filter derivation (visible rows, highlight reset + wrap, confirm-from-highlight, deleted-memory fallback to Default)
- [ ] `src/ui/instance_picker.rs` -- view reorder + capped scrollable dropdown + combobox key ownership + count/no-match lines + roles/labels
- [ ] `src/ui/i18n/*` -- two keys × (list + translate + variant) × four locales
- [ ] `src/ui/instance_picker.rs` -- picker mod tests: filter narrows + resets, wrap, Enter-confirms-highlight, Esc layers, preselect fallbacks, rows retain mtimes through filter

**Acceptance Criteria:**
- Given several instances, when the user types a substring, then only Default plus matches show and highlight sits on the first match
- Given a highlight, when the user presses Enter, then that instance is entered and remembered; a valid unlisted name does nothing
- Given an open dropdown, when the user presses Esc twice, then the first press closes the dropdown keeping text and the second cancels the launch
- Given missing, corrupt, or deleted-name memory, when the picker opens, then Default is highlighted
- Given the four locale files, when the parity test runs, then the two new keys are present and non-empty everywhere

## Design Notes

gpui exposes `role()`/`accessible_label()` but no live-region primitive (verified: no aria-live anywhere in `src/`). The closest equivalent to the spec's live announcements is a visible count line (localized via the new MatchCount key) plus per-option accessible labels carrying position ("bob, 2 of 5"). If the user wants more than the platform allows, that is a spec-level change, not code.

## Review Triage Log

- quick lens round 1, 3 findings, all verified real and patched: click bypasses invalid-text veto (direct state.confirm+finish); set_query("") restores open-time preselect via new preselect field (verbatim-state harness green: restore, filter, wrap, confirm, esc); MatchCount added to plural_keys test list (arms already complete).
- Author patch: hardcoded "N of M" label fragment replaced with localized InstancePickerPosition key (mirrors established of-{total} family in all four locales) + t_fmt import; cargo check clean.
- quick lens round 2 on patched hunks: no new findings.
- Matrix audit: 8/8 rows covered by ran tests (standalone verbatim-state harness for state rows; in-tree picker tests written but blocked by pre-existing bin-target breakage per deferred-work.md); i18n parity compiler-checked (exhaustive match) + arms inspected in all four locales.

## Verification

**Commands:**
- `cargo check --bin tty7-app` -- expected: clean (verified green)
- `cargo test --bin tty7-app instance_picker` -- expected: green, but BLOCKED by the pre-existing test-target breakage (24 E0252 in `terminal/view.rs`, recorded in deferred-work.md); picker tests are written to run once that lands
