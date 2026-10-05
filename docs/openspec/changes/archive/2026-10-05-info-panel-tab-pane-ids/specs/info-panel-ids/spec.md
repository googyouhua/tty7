## Purpose

The right Info panel exposes the unique identifiers of the active tab and the focused pane so users can distinguish same-named tabs and correlate the UI with CLI output.

## ADDED Requirements

### Requirement: Info panel shows the active tab identifier

The system SHALL display a Tab ID row in the right Info panel for the active tab, showing the tab's raw unique id with a copy affordance that copies the raw value.

#### Scenario: Active tab id is visible

- **WHEN** the user opens the right Info panel on any tab
- **THEN** a Tab ID row shows the active tab's unique id and its copy action copies that exact value

#### Scenario: Switching tabs updates the row

- **WHEN** the user activates a different tab
- **THEN** the Tab ID row shows the newly active tab's id

### Requirement: Info panel shows the focused pane identifier

The system SHALL display a Pane ID row in the right Info panel for the focused pane, showing the pane's raw numeric id with a copy affordance that copies the raw value. In a split tab the row SHALL follow the last-focused leaf, consistent with the existing cwd row.

#### Scenario: Focused pane id is visible

- **WHEN** the user opens the right Info panel
- **THEN** a Pane ID row shows the focused pane's numeric id and its copy action copies that exact value

#### Scenario: Split tab follows focus

- **WHEN** the user moves focus to another split within the same tab
- **THEN** the Pane ID row updates to the newly focused pane's id while the Tab ID row is unchanged

### Requirement: Info panel shows CLI ordinals

The system SHALL display the CLI-style ordinals alongside the ids so UI state can be correlated with `tty7 tab ls` / `tty7 pane ls` output.

#### Scenario: Ordinals match CLI addressing

- **WHEN** the user compares the Info panel with CLI output for the same workspace
- **THEN** the displayed ordinals address the same tab and pane as the CLI's `@n` and `%n` forms

### Requirement: Identifiers stay out of other surfaces

The system SHALL NOT add id rows to the tab strip, sidebar, or switcher in this change; identifier display is scoped to the right Info panel.

#### Scenario: Other surfaces unchanged

- **WHEN** the user looks at the tab strip, sidebar, or switcher
- **THEN** no tab or pane id rows appear there
