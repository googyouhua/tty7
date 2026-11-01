## Purpose

The Review tab keeps long branch-diff file lists navigable and unsent review drafts reachable by scrolling each list in its own bounded region and filtering files by path.

## ADDED Requirements

### Requirement: Diff file list scrolls in a bounded region

The system SHALL render the Review tab diff file list inside a vertically scrollable region with a bounded height, so a large number of files does not push content below it off-screen.

#### Scenario: Many files stay scrollable

- **WHEN** the branch range contains more files than fit the visible area
- **THEN** the file list region scrolls vertically and entries beyond the visible area remain reachable

#### Scenario: Few files show without clipping

- **WHEN** the branch range contains only a few files
- **THEN** all files are visible without scrolling and no empty scroll region is shown

### Requirement: Diff file list can be filtered by path

The system SHALL provide a text filter for the Review tab diff file list that narrows visible files by path substring match.

#### Scenario: Filter narrows the list

- **WHEN** the user types text into the file filter
- **THEN** only files whose path contains the filter text remain visible

#### Scenario: Clearing the filter restores the list

- **WHEN** the user clears the filter text
- **THEN** the full file list is visible again

### Requirement: File list shares space with the draft list

The system SHALL allocate Review tab vertical space so the diff file list does not monopolize the panel: the draft section keeps a visible reserved portion even when the file list is long.

#### Scenario: Drafts stay visible with a long file list

- **WHEN** the file list is long enough to need scrolling
- **THEN** the draft section header and at least the first draft rows remain visible without scrolling the whole panel

### Requirement: Draft list scrolls in a bounded region

The system SHALL render the Review tab draft list inside a vertically scrollable region with a bounded height, so many drafts remain reachable.

#### Scenario: Many drafts stay reachable

- **WHEN** there are more drafts than fit the draft region
- **THEN** the draft region scrolls vertically and every draft remains reachable
