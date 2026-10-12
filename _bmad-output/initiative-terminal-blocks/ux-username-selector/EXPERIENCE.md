---
name: username-selector
status: final
sources: []
updated: 2026-10-11
---

# username-selector — Experience Spine

> Startup instance picker redesign: from tiled username rows that fill the window to a compact combobox (input-is-filter) with a capped dropdown, per-row delete, and an explicit create path. Visual identity lives in `DESIGN.md`; this spine is the behavior.

## Foundation

Desktop gpui picker window, ~400px wide, fixed compact height. Runs before anything instance-specific initializes (no daemon, no window store). Stack: `src/ui/instance_picker.rs` (gpui view, `PickerState`, combobox + dropdown + modal) over `crates/tty7-core/src/core/instance.rs` (`list_in`, `validate_name` `[a-z0-9-]`, memory file `.last-instance` with `default` sentinel).

Data sources: instance list from `list_in` (subdirectories shaped exactly `tty7-<valid-name>`, sorted; Default implied, never listed); preselect from `read_last` (memory value or `default` sentinel, fallback Default when missing/unreadable/invalid or when the remembered name no longer exists). [ASSUMPTION] Only case-insensitive substring filtering ships in v1; pinyin/fuzzy matching is deferred. Last-used time comes from the instance dir mtime (adopted per architecture AD-4); rows render without meta only when the filesystem cannot supply it. Deletion follows architecture AD-2/AD-3/AD-7: core `remove_instance(name)` validates, runs the authoritative liveness probe, and unlinks (refusing a live seat) — it does not touch memory; the picker rewrites memory to the focused row strictly after `Ok`. The UI never deletes paths itself.

Composition reference: `src/ui/instance_picker.rs` render fn (~283–375) and `mockups/key-picker.html` (filtering state + delete-confirm modal). Spines win over mockups and code sketches on conflict. Scope sources: shipped picker code + `.memlog.md` decisions (no PRD/brief for this scope).

## Information Architecture

| Surface | Reached from | Purpose |
|---|---|---|
| Picker window | Bare launch (no `--config-dir` / `TTY7_CONFIG_DIR`) | Choose who is using tty7; confirm or cancel the launch |
| Combobox input | Top of picker, focused on open | Filter existing instances by substring; holds a candidate new name |
| Dropdown viewport | Expands under the input | Capped (6–8 rows) scrollable list: Default row first, then sorted names |
| Create button | Below dropdown, enabled on valid new input | Explicitly create and enter a new instance |
| Delete-confirm modal | Row × on a named row | Confirm destructive removal of one instance |
| Footer | Bottom of picker | Close (cancel, ghost) / Enter (confirm, primary); unchanged |

Modal stacks one level deep (delete modal over picker, never a modal over a modal). No other surfaces. Window never grows with row count — overflow scrolls inside the dropdown viewport (`{spacing.window-pad}`, `{spacing.stack}` layout in `DESIGN.md`).

## Voice and Tone

Microcopy. Visual type specs live in `DESIGN.md`.

| Do | Don't |
|---|---|
| "Who is using tty7?" (existing title) | "Select your identity! 🎉" |
| "Usernames keep everyone's sessions apart." (existing hint) | "Pick wisely — this changes everything." |
| "Usernames use [a-z0-9-]." (existing invalid line, shown only for trimmed non-empty invalid input) | Technical regex dumps or blame ("You typed garbage") |
| Modal names the instance plainly and warns all sessions/state under `tty7-<name>` will be removed | Vague "Are you sure?" with no consequences |
| New keys (combobox affordance, create button, modal, no-match hint) match the `InstancePicker*` family tone in `zh`/`en` | New keys in a different voice per locale |

Existing `InstancePicker*` keys in `src/ui/i18n/zh.rs` / `en.rs` are kept; new keys are added in-family.

## Component Patterns

Behavioral. Visual specs live in `DESIGN.md.Components` (or app defaults when inherited).

| Component | Use | Behavioral rules |
|---|---|---|
| Combobox input | Picker top, autofocused | Input-is-filter: every keystroke live-filters the dropdown (case-insensitive substring). [ASSUMPTION] No pinyin/fuzzy in v1. Clearing the input restores the full list. Filter input alone never creates an instance. |
| Dropdown viewport | Under input | Fixed height, max 6–8 visible rows, internal scroll beyond. Opens on picker open (showing preselect), on input click, on Up/Down. Closes on Esc (first Esc), on selection confirm, on empty result set it shows the no-match hint instead of rows. |
| Dropdown row | Viewport list | Default row first (label `InstancePickerDefault`), then sorted names. Row shows label + last-used meta when available ([ASSUMPTION] source TBD; omit meta until agreed). Click row = select + confirm immediately. Highlight follows filter + arrows. |
| Row delete × | Named rows only | × on every named row; never on the Default row (no directory to delete). Click opens the delete modal; the click must not select/confirm the row (stop propagation). Keyboard focus never lands on × implicitly — arrows move row highlight only. |
| Create button | Below dropdown | Separate explicit action. Enabled only when the trimmed input passes `validate_name` (`[a-z0-9-]`, not `default`, no leading `-`) AND names no existing row. Click = create + enter that instance. Disabled otherwise; invalid non-empty input additionally shows the `InstancePickerInvalid` line in `{colors.danger}`. |
| Delete-confirm modal | Overlay | Destructive confirm: names the instance, warns all sessions/state under `tty7-<name>` will be removed. Confirm deletes the directory via core (`remove_instance`), then on success the picker rewrites memory to the focused row so no dangling preselect remains; closes modal, focus falls to the next row. Cancel closes the modal and returns focus to the invoking row. |
| Invalid text | Below input | Shown only when trimmed non-empty input fails `validate_name`. Reuses the existing `InstancePickerInvalid` string in `{colors.danger}` at `{typography.invalid.fontSize}`. Never modal, never toast; create button stays disabled while shown. |
| Footer Close / Enter | Picker bottom | Unchanged: Close cancels the launch (no select); Enter confirms per the Enter rules below. |

## State Patterns

| State | Surface | Treatment |
|---|---|---|
| Open with memory | Picker | Preselect last-used row (`read_last`); highlight on it; dropdown open showing it. Fallback: Default row when memory is missing/unreadable/invalid or names a deleted instance. |
| Filtering | Dropdown | Substring match (case-insensitive) against names; highlight resets to first match. Meta line still shows when available. |
| No matches | Dropdown | No rows; hint line (new i18n key, e.g. no instance matches — refine or create below). Create button state decides the path: valid input → create enabled; invalid → `InstancePickerInvalid` in `{colors.danger}`. |
| Invalid typed input | Below input | `InstancePickerInvalid` line in `{colors.danger}` whenever trimmed non-empty input fails `validate_name`. Create button disabled. No modal, no toast. |
| Valid new name | Create row | Create button enabled; dropdown shows no-match hint (name matches nothing existing). Enter does NOT create — only the button or footer Enter on a highlighted row confirms; see Interaction Primitives. |
| Delete-confirm modal open | Modal | Picker behind is inert. Confirm → remove + memory rewrite + focus-to-next-row. Cancel → back to invoking row, no state change. |
| Delete failed | Delete-confirm modal (stays open) | Core `remove_instance` error ([ASSUMPTION]): modal stays open with a retryable error line (new delete-error key, `InstancePickerRelaunchFailed` pattern), row not removed, memory not rewritten. |
| Dropdown closed | Picker (after first Esc) | Input keeps its text, highlight retained; Up/Down or click reopens. Second Esc cancels the window. |
| After delete | Dropdown | Deleted row gone; highlight/focus falls to the next row (or previous if last). If the deleted name was the remembered one, memory is rewritten so the next open preselects a live row — never a dangling preselect. |
| Empty except Default | Dropdown | Single Default row; every named row deletable means this state is reachable. Create path still available. |
| Relaunch failed (Windows) | Below input | `InstancePickerRelaunchFailed` in `{colors.danger}`; picker stays open for retry. Unchanged semantics. |

## Interaction Primitives

**Standard combobox keyboard.** Focus starts in the combobox input on open.

- `Up` / `Down` — Move row highlight; wraps at ends. Always available; when the input holds text the keys still move the highlight (the old caret-steals-arrows rule is gone — the combobox owns the keys).
- `Esc` — First press closes the dropdown (input keeps its text, highlight retained). Second press cancels the window (no select, launch quits). Esc inside the delete modal cancels the modal first.
- `Enter` — Confirms the highlighted row. When the dropdown is closed, Enter confirms the combobox value/highlight. Enter never creates from raw input: a valid-but-unlisted typed name requires the explicit create button. (This replaces the old typed-wins `PickerState::confirm` rule.)
- Click row = select + confirm. Click × = open delete modal only; must not confirm (stop propagation).
- Type-to-filter is live per keystroke; highlight resets to the first match after each filter change.

**Banned:** Enter-wins-as-new-user, filter-input-alone creates, delete without modal, × on the Default row, dropdown growing the window, modal stacks deeper than one.

## Accessibility Floor

Behavioral. Visual contrast lives in `DESIGN.md` (selection `{colors.selection}` on `{colors.selection-foreground}` inherited from the shipped picker; app theme owns remaining ratios).

- Full keyboard operation: open → filter → arrows → Enter, and open → ×-adjacent keyboard path → modal → confirm/cancel, all without a pointer. `Tab` order: input → dropdown rows → create button → Close → Enter → (modal trap when open).
- Screen reader announces the picker title on open, the preselected row ("alice, selected, 1 of N"), filtered result counts as they change (`aria-live`), and modal title + consequences on open.
- `Esc` always closes the topmost layer first (modal, then dropdown, then window) — never skips a layer.
- Highlighted row is exposed as the active descendant; highlight and input value are announced distinctly (a typed filter is not a selection).
- Delete-confirm modal traps focus until confirmed or cancelled; focus returns to the invoking row (or its successor after deletion).
- Touch: rows and × meet the app's minimum target treatment; × target is larger than its glyph.

## Key Flows

### Flow 1 — Monday pick, filter, and ghost cleanup (Xiao Lin, shared workstation, Monday 9:02am)

1. Xiao Lin opens tty7 bare on the shared box. The picker opens, input focused, dropdown showing the remembered `alice` row preselected (memory `.last-instance` read; fallback would be Default).
2. She needs `bob` today. She types `b`. The dropdown live-filters (case-insensitive substring) to the matching names; highlight resets to the first match, `bob`.
3. She presses Down once to check the next match, Up to return to `bob`, then Enter. The picker confirms `bob`: directory resolved, memory rewritten to `bob`, launch enters `bob`'s sessions. No new instance is created — the filter only narrowed the list.
4. Later she reopens the picker to clean up. A stale `ghost` row is still listed. She highlights it and clicks its × (Default has no ×). The delete modal opens, naming `ghost` and warning that all sessions/state under `tty7-ghost` will be removed.
5. She confirms. Core removes the directory (`remove_instance("ghost")`, refusing a live seat); on success the picker rewrites memory to the focused row. The modal closes and focus falls to the next row. Had `ghost` been the remembered name, the rewrite still leaves a live preselect — nothing dangles.
6. **Climax:** through a dozen-name box, Monday filtering, and a deletion, the picker window never changes size — the dropdown scrolled internally at its 6–8-row cap while the compact window stayed put. The tiled wall of rows that used to fill the screen is gone for good.

Failure: she types `Bob!` — trimmed input fails `validate_name`. The `InstancePickerInvalid` line appears in `{colors.danger}` (`{typography.invalid.fontSize}`), the create button stays disabled, and Enter does nothing but keep the hint. She corrects to `bob-2`; the button enables and she creates explicitly.

Failure: she clicks × on `ghost` but cancels the modal — no deletion, focus returns to the `ghost` row, memory untouched.

Failure: core deletion fails (`remove_instance` errors, including live-seat refusal) — the modal stays open with a retryable error line (existing `InstancePickerRelaunchFailed` pattern, new delete-error key), the row is not removed, memory is not rewritten.

### Flow 2 — First-time creation (Xiao Lin, new teammate onboarding)

1. Xiao Lin opens the picker for a new hire `carol`. She types `carol`; the dropdown shows the no-match hint and the create button enables (trimmed input passes `validate_name` and matches nothing existing).
2. She clicks the create button (explicit — Enter alone would do nothing). The instance directory `tty7-carol` is created, memory rewritten to `carol`, and the launch enters `carol`'s fresh sessions.
3. **Climax:** next bare launch preselects `carol` — the explicitly created user is now a first-class remembered row with last-used meta, not a one-off typed string.

→ Visual reference: `mockups/key-picker.html` illustrates Flow 1 steps 2–5 (filter highlight + modal).
