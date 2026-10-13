# Spine Pair Review — username-selector

## Overall verdict

Adequate — consumable as a downstream contract with small repairs. All load-bearing behavior decisions (combobox-is-filter, explicit create, modal delete, fixed viewport, Enter/Escape rules) are committed with named code anchors, and every token reference resolves. Before story-dev, close the orphaned visual reference, step the create-success path, and promote the deletion-failure state from a flow aside into State Patterns.

## 1. Flow coverage — adequate

Checked: EXPERIENCE.md frontmatter `sources: []` → no UJ/requirement names to extract, so coverage is judged on the spine's own scope (pick + filter + create + delete). Flow 1 has a named protagonist (Xiao Lin), numbered steps (6), a bolded climax beat (window never changes size), and three failure paths (invalid input, modal cancel, core deletion failure).

### Findings

- **[medium]** Create-success path has no numbered steps — only a failure aside ("She corrects to `bob-2`; the button enables and she creates explicitly"). The explicit create button replaces the old typed-wins rule, so its success sequence (enable → click → enter instance → memory write) is load-bearing and should be stepped, not implied. (EXPERIENCE.md Key Flows, Flow 1 step 6 / Failure para 1). *Fix:* add 2–3 numbered create steps or a short Flow 2 for first-time creation.
- **[low]** `sources: []` is empty; traceability rests on inline code anchors (`src/ui/instance_picker.rs`, `list_in`/`validate_name`, i18n keys) and `.memlog.md` decisions. Resolves vacuously but leaves no requirement inventory for a consumer to check off. (EXPERIENCE.md frontmatter). *Fix:* either keep `[]` with a one-line note that code + memlog are the sources, or list the render fn and memlog as sources.

## 2. Token completeness — adequate

Checked: extracted all frontmatter tokens (colors: `selection`, `selection-foreground`, `danger`; typography: `title`, `hint`, `row[note-only]`, `meta`, `invalid`; rounded: `sm`; spacing: `window-pad`, `stack`, `row-x`, `row-y`, `actions-gap`; components: 8 entries) and all `{path.to.token}` references in both spines (~25 in DESIGN.md, 6 in EXPERIENCE.md). Every reference resolves. All three color tokens carry hex (`#2E7D5B`, `#FFFFFF`, `#C0392B`). `typography.row` note-only is spec-legal (any subset).

### Findings

- **[low]** No numeric contrast target stated for the load-bearing combos (selection-green/white fill, danger-red invalid text). Both spines declare contrast inherited from the shipped picker / app theme (DESIGN.md Colors; EXPERIENCE.md Accessibility Floor), which is defensible for a gpui desktop delta, but a consumer cannot verify AA without opening the app. (DESIGN.md Colors; EXPERIENCE.md Accessibility Floor). *Fix:* add one line per combo: measured ratio or "inherits shipped-picker ratio, verified at build."

## 3. Component coverage — adequate

Checked: extracted every component name used anywhere. DESIGN.md Components rows (7): Combobox input, Dropdown viewport, Dropdown row, Row delete ×, Create button, Delete-confirm modal, Invalid text. EXPERIENCE.md Component Patterns rows (7): Combobox input, Dropdown viewport, Dropdown row, Row delete ×, Create button, Delete modal, Footer Close / Enter. All rows carry real rules.

### Findings

- **[medium]** `Invalid text` has a DESIGN.md visual row but no EXPERIENCE.md Component Patterns row — its behavior lives only in State Patterns (`Invalid typed input`) and the Create-button row. Per the rubric each component needs a row in both tables. (DESIGN.md Components; EXPERIENCE.md Component Patterns). *Fix:* add an `Invalid text` row to Component Patterns (show condition, string source, no-modal rule) or record why State Patterns suffices.
- **[low]** `Footer Close / Enter` has an EXPERIENCE.md row but no DESIGN.md row. Omission is defensible (explicitly "unchanged") and matches the as-is pattern from the shadcn example, but DESIGN.md never says so — a consumer may read it as forgotten. (EXPERIENCE.md Component Patterns; DESIGN.md Components). *Fix:* one line in DESIGN.md Components intro: "Footer actions inherit app defaults unchanged, no delta."
- **[low]** Naming drift: `Delete-confirm modal` (DESIGN) vs `Delete modal` (EXPERIENCE); `Create action` (IA table) vs `Create button` (patterns). Behavior is unambiguous but names are not verbatim. (DESIGN.md Components; EXPERIENCE.md IA + Component Patterns). *Fix:* pick one name per component and use it everywhere.

## 4. State coverage — adequate

Checked: walked all six IA surfaces (Picker window, Combobox input, Dropdown viewport, Create action, Delete modal, Footer). State Patterns covers 9 states: open-with-memory (+fallback), filtering, no-matches, invalid input, valid-new-name, delete-confirm-open, after-delete, empty-except-Default, relaunch-failed. Cold-load/offline/permission-denied are correctly N/A for a pre-init local picker except as noted below.

### Findings

- **[medium]** Filesystem deletion failure (core `remove_instance` errors) appears only as a Flow 1 failure aside, not as a State Patterns row — yet it dictates modal persistence, error-line key, no-memory-rewrite, and retry. Load-bearing for story-dev. (EXPERIENCE.md State Patterns vs Key Flows Failure para 3). *Fix:* add a `Delete failed` state row (surface: modal; treatment: modal stays open, retryable error line, no row removal, no memory rewrite).
- **[low]** Dropdown-closed state (after first-Esc: input keeps text, highlight retained) is specified in Component Patterns and Interaction Primitives but has no State Patterns row. Minor since behavior is fully stated elsewhere. (EXPERIENCE.md Component Patterns + Interaction Primitives). *Fix:* add a one-line `Dropdown closed` state row or leave as-is.

## 5. Visual reference coverage — thin

Checked: filesystem has no `mockups/`, no `wireframes/`, empty `imports/`; the only visual is `.working/key-picker.html` (not yet promoted). Listed every candidate and searched both spines for links.

### Findings

- **[medium]** `.working/key-picker.html` is an orphan: neither spine links to it inline, and neither spine names what it illustrates (filtering state + delete modal). Its header comment already declares "Spine wins on conflict," but the rubric requires the spines to state that, and EXPERIENCE.md's "Spine wins on conflict" (line 18) scopes only to the code composition reference, not to visuals. (Both spines; `.working/key-picker.html`). *Fix:* promote to `mockups/key-picker.html` and add one inline link per spine at the relevant section (DESIGN.md Components; EXPERIENCE.md Component/State Patterns), or delete the file if superseded.
- **[low]** No spines-win-on-conflict sentence covering visual references exists in either spine. (Both spines). *Fix:* add once, e.g. in EXPERIENCE.md Foundation or DESIGN.md intro: "Spines win over mockups on conflict."

## 6. Bloat & overspecification — strong

Checked: DESIGN.md editorial voice stays tied to decisions (restraint rule, two-color discipline, [ASSUMPTION] marking for inherited tokens); no pixel specs beyond the inherited render values; no persona/FR restatement; tables where tables serve. EXPERIENCE.md prose is behavioral throughout.

### Findings

None.

## 7. Inheritance discipline — adequate

Checked: `sources` resolves vacuously (`[]`); glossary terms (`validate_name`, `list_in`/`read_last`, `tty7-<name>`, `default` sentinel, `InstancePicker*` keys, combobox/viewport/row/modal/footer) are identical across spines, memlog, and mock; all EXPERIENCE.md token refs (`{spacing.window-pad}`, `{spacing.stack}`, `{colors.danger}`, `{typography.invalid.fontSize}`, `{colors.selection}`, `{colors.selection-foreground}`) resolve to DESIGN.md frontmatter by name.

### Findings

- **[low]** Component-name drift noted in §3 (`Delete-confirm modal`/`Delete modal`, `Create action`/`Create button`) is the only inheritance break; token and glossary inheritance is clean. *Fix:* same rename fix as §3.

## 8. Shape fit — strong

Checked: DESIGN.md sections follow canonical order exactly (Brand & Style → Colors → Typography → Layout & Spacing → Elevation & Depth → Shapes → Components → Do's and Don'ts). EXPERIENCE.md contains all 8 required defaults (Foundation, IA, Voice and Tone, Component Patterns, State Patterns, Interaction Primitives, Accessibility Floor, Key Flows). Dropped-optionals defensible: no Responsive section (single fixed ~400px desktop window, no breakpoints); no Inspiration section (no reference products in memlog/sources — rejects recorded inline as Banned lists). No invented sections.

### Findings

None.

## Mechanical notes

- Frontmatter: DESIGN.md has `name`/`description`/`status` + full token blocks, no `sources`/`updated` (not required by design-md-spec); EXPERIENCE.md has `name`/`status`/`sources: []`/`updated: 2026-10-11`. Names match (`username-selector`); both `status: draft`.
- Cross-refs: all `{path.to.token}` refs resolve; no broken file links (spines reference `src/ui/instance_picker.rs`, `src/ui/i18n/{zh,en}.rs`, `crates/tty7-core/src/core/instance.rs` — paths exist per directory listing of `src/ui/`; core path not verified here).
- No Mermaid diagrams present; nothing to syntax-check.
- Directory note: task brief cited `mockups/` and `imports/` content, but the workspace has no `mockups/` dir, `imports/` is empty, and the only visual lives in `.working/` — §5 findings cover the resulting orphans.
