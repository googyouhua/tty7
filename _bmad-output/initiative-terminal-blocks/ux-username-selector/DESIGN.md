---
name: username-selector
description: Startup instance picker redesign — compact combobox with filter and delete for tty7
status: final
updated: 2026-10-11
colors:
  selection: '#2E7D5B'
  selection-foreground: '#FFFFFF'
  danger: '#C0392B'
typography:
  title:
    fontSize: 15px
    fontWeight: '700'
    lineHeight: '1.3'
  hint:
    fontSize: 12px
    fontWeight: '400'
    lineHeight: '1.4'
  row:
    note: 'App default body size — picker rows inherit the gpui app text size; no override.'
  meta:
    fontSize: 12px
    fontWeight: '400'
    lineHeight: '1.4'
    note: 'Last-used relative time; secondary color inherited from app tokens.'
  invalid:
    fontSize: 12px
    fontWeight: '400'
    lineHeight: '1.4'
rounded:
  sm: 6px
spacing:
  window-pad: 20px
  stack: 12px
  row-x: 10px
  row-y: 7px
  actions-gap: 8px
components:
  combobox-input:
    radius: '{rounded.sm}'
  dropdown-viewport:
    radius: '{rounded.sm}'
  dropdown-row:
    padding-x: '{spacing.row-x}'
    padding-y: '{spacing.row-y}'
    radius: '{rounded.sm}'
  dropdown-row-selected:
    background: '{colors.selection}'
    foreground: '{colors.selection-foreground}'
    radius: '{rounded.sm}'
  row-delete:
    foreground: '{colors.selection-foreground}'
    note: 'Visible on the highlighted/hovered named row only; inherits row text color otherwise.'
  create-button:
    background: '{colors.selection}'
    foreground: '{colors.selection-foreground}'
    radius: '{rounded.sm}'
  delete-modal:
    radius: '{rounded.sm}'
  invalid-text:
    foreground: '{colors.danger}'
    fontSize: '{typography.invalid.fontSize}'
---

## Brand & Style

tty7's startup instance picker is a small desktop gate, not a destination: it answers "who is using tty7?" and gets out of the way. The existing gpui desktop picker sets the language — plain window, bold title, one-line hint, quiet rows, green confirm — and this redesign keeps that voice intact. The only change is posture: from tiled rows that grow with the user count to a compact combobox that stays the same size no matter how many instances exist.

The aesthetic rule is restraint. One brand green means *this is the confirmed choice*. One red means *this name cannot be used*. Everything else — window chrome, input surface, dropdown surface, light/dark values — is inherited from the app's existing gpui tokens. This DESIGN.md specifies only the deltas: combobox, dropdown viewport, delete affordance, create button, delete modal.

## Colors

All surfaces except the two deltas below inherit the app's existing gpui light/dark tokens. [ASSUMPTION] The exact light/dark surface, border, input-fill, and muted-text values are owned by the app theme and are not restated here; do not invent palette values for them.

- **Selection green (`{colors.selection}` `#2E7D5B` with `{colors.selection-foreground}` white text)** is the single brand signal. It fills the highlighted dropdown row, the combobox confirm affordance, and the explicit create button. It already means "selected" in the current picker (`instance_picker.rs` render); the redesign keeps that meaning and extends it to the combobox highlight. Never used for decoration, never used for delete.
- **Invalid red (`{colors.danger}` `#C0392B`)** is reserved for the `InstancePickerInvalid` hint line and the relaunch-failed line. It already serves this role in the current picker; unchanged. Inherits the shipped picker's verified contrast (green/white selection fill and red-on-surface invalid text ship today; verify ratios at build). Never used for row fills.
- **Delete ×** inherits the row's current text color — `{colors.selection-foreground}` on the highlighted row, app default otherwise. It has no color of its own; its danger is communicated by the modal, not the glyph.

Avoid: new accent colors, gradient fills, tinted dropdown surfaces, red row fills for deletable rows.

## Typography

Type roles inherit the app default face. Only sizes/weights below are pinned; all come from the current picker render.

- **Title** — `{typography.title.fontSize}` bold. The existing `InstancePickerTitle` string ("Who is using tty7?"). One line, top of window.
- **Hint** — `{typography.hint.fontSize}` regular. The existing `InstancePickerHint` string. One line under the title.
- **Row label** — app default body size. Default row uses the existing `InstancePickerDefault` string; named rows are the bare instance name in `[a-z0-9-]`. No truncation rule changes; long names clip per app default.
- **Row meta** — `{typography.meta.fontSize}` secondary. Last-used relative time (e.g. 3天前) right-aligned in the row when available. Color and exact secondary token inherited from app theme.
- **Invalid line** — `{typography.invalid.fontSize}` in `{colors.danger}`. The existing `InstancePickerInvalid` string; new combobox/create/delete strings follow the same size and red.

## Layout & Spacing

The window stays compact and fixed. Today it opens at 400px wide (`size(px(400.), px(430.))`); the redesign keeps the ~400px width and a fixed compact height that no longer grows with the user count. Internal scale, from the current render:

- Window padding `{spacing.window-pad}` (20px); vertical stack gap `{spacing.stack}` (12px).
- Dropdown rows pad `{spacing.row-x}` × `{spacing.row-y}` (10px × 7px).
- Footer action row right-aligned with gap `{spacing.actions-gap}` (8px): Close (ghost) + Enter (primary), unchanged.

Structure top to bottom: title → hint → combobox input (filter) → dropdown viewport → invalid line (conditional) → create-button row (conditional/enabled state) → footer actions. The dropdown viewport is a fixed-height list showing max 6–8 rows with internal scroll; overflow never expands the window — it scrolls inside the viewport. This is the core delta: the old `(0..rows)` tiled column is gone.

→ Composition reference: `mockups/key-picker.html` (left: filtering with `b` highlighting `bob`; right: delete-confirm modal). Spine wins on conflict.

## Elevation & Depth

Inherited from the app's gpui surfaces. The redesign adds no new shadow language. Two rules:

- The open dropdown viewport sits above the footer content as a single floating list layer; it does not cast a new shadow style — it uses whatever popover/list elevation the app theme already provides.
- The delete-confirm modal is the only true overlay: one level deep, centered over the picker, dimming the picker behind it per the app's existing modal treatment. Never stacks a second modal.

## Shapes

`{rounded.sm}` (6px) everywhere the picker draws its own corners — dropdown rows, combobox input, create button, modal. This matches the current row radius (`rounded(px(6.))`) and is now the picker-wide rule. No pills, no full-round avatars, no per-row shape variance. The window's own outer corner is owned by the OS/app theme, not this spine.

## Components

Visual specs for the deltas only. Footer actions (Close ghost + Enter primary) inherit app defaults unchanged — no delta. Behavior lives in `EXPERIENCE.md`. Token refs resolve against this file's frontmatter. Spines win over mockups on conflict.

- **Combobox input** — App input surface with `{rounded.sm}` corners. Placeholder is the existing `InstancePickerNewPlaceholder` string when empty. Typing filters the dropdown; the input itself shows no success/error fill — validity is communicated by the create button's enabled state plus the `{colors.danger}` invalid line.
- **Dropdown viewport** — Fixed-height list, max 6–8 visible rows, internal scroll beyond that, `{rounded.sm}` container. Closed by default except on open-with-preselect; opens on click/arrow when rows exist.
- **Dropdown row** — Padding `{spacing.row-x}` / `{spacing.row-y}`, `{rounded.sm}`. Anatomy: label left, last-used meta right (when available), delete × far right on named rows. Highlighted row uses `{components.dropdown-row-selected.background}` fill with `{components.dropdown-row-selected.foreground}` text — the inherited green selection. Non-highlighted rows are transparent app surface.
- **Row delete ×** — Small glyph button pinned to the right end of each named row. No × on the Default row (never deletable). Invisible or muted until the row is highlighted/hovered; on the highlighted row it renders in `{colors.selection-foreground}`. Click target must be comfortably larger than the glyph itself.
- **Create button** — Separate explicit action below the dropdown (not Enter-wins). `{components.create-button.background}` fill, `{components.create-button.foreground}` text, `{rounded.sm}`. Enabled only when the trimmed input passes `validate_name`; disabled state inherits the app's disabled-button treatment. Label is a new i18n key (e.g. create-new-instance), not the Enter key.
- **Delete-confirm modal** — `{components.delete-modal.radius}` card, centered, one level. Destructive confirm button + cancel. Names the instance and warns that all sessions/state under `tty7-<name>` will be removed. Button labels are new i18n keys.
- **Invalid text** — `{components.invalid-text.foreground}` at `{components.invalid-text.fontSize}`, shown under the combobox only when the trimmed non-empty input fails `validate_name`. Reuses the existing `InstancePickerInvalid` string.

New i18n keys (combobox affordances, create button, modal title/body/confirm/cancel, no-match hint) follow the existing `InstancePicker*` key family in `src/ui/i18n/{zh,en}.rs`; existing keys and their wording are kept.

## Do's and Don'ts

| Do | Don't |
|---|---|
| Keep the window compact (~400px, fixed height); overflow scrolls inside the dropdown viewport | Let rows tile and grow the window with the user count |
| Use `{colors.selection}` fill + `{colors.selection-foreground}` text only for the highlighted/confirmable thing | Use green for decoration, delete, or secondary info |
| Use `{colors.danger}` only for the invalid / relaunch-failed line | Fill rows red or invent new accent colors |
| Show delete × on named rows only; never on the Default row | Add any delete affordance to Default |
| `{rounded.sm}` (6px) on input, rows, create button, modal | Introduce pills or mixed radii |
| Inherit app light/dark surfaces; mark unknowns [ASSUMPTION] | Invent light/dark palette hex values |
| Keep existing `InstancePicker*` i18n keys and add new ones in-family | Rename or reword existing keys |
