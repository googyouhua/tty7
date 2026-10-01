# OneKey Autofill — complete target specification

## 1. Purpose

Provide WindTerm-style OneKey autofill for every shell pane: user-saved credentials
(SSH-bound and generic account entries) can be picked manually and filled into the
active PTY as username and/or password.

## 2. Data model

- `OneKeyEntry`:
  - `id: String` (uuid v4, stable).
  - `kind: SshBound | Account`.
  - `title: String` (display label, non-empty, unique suggestion, max 64 chars).
  - `username: String` (may be empty for password-only entries).
  - `password: SecretString` (stored in config file per D5).
  - `host_binding: Option<String>` for `SshBound` (host/profile hint for filtering; not enforced).
- Stored in user config file (e.g. `config.json` `onekey_entries: Vec<OneKeyEntry>`);
  exact key name is implementation detail but must be documented in config reference.
- CRUD: add, edit (title/username/password/binding), delete, list. Duplicate titles
  rejected with visible error.

## 3. Trigger

Manual only. All three entry points open the same picker for the focused pane:

- Command palette item `OneKey: Autofill…` (`CommandKind::OneKeyAutofill`).
- Terminal right-click context menu row `OneKey Autofill…`.
- Shortcut `Ctrl-Shift-I` (user choice), registered in `shipped_bindings` and
  Keybindings settings page under Terminal group.

No automatic prompt detection. No background scanning of PTY output.

## 4. Picker and fill behavior

- Picker is anchored to the active `TerminalView` (like reverse-search/completion
  popup): filters by title/username/binding as user types, keyboard navigable
  (Up/Down + Enter to select, Esc to cancel).
- After entry selection, second step chooses `Username` / `Password` /
  `Username+Password`. Cancel at either step sends zero bytes to the PTY.
- Fill routing:
  - If inline prompt editor active (`input_active()`), username fill uses the
    bracketed-paste-safe `paste()` path (`CmdEditor::insert_pasted`); password
    fill uses the same path when editor active, else direct `send_to_pty`.
  - Never bypass `accepts_input` guards; never call `terminal.write` raw.
- `Username+Password` sequence: types username, presses `Enter`, types password,
  then submits the login with a final `Enter` (auto-submit per user request).
- Picker masks password values (never renders secret plaintext); title/username
  visible for selection.

## 5. Management UI

Settings page `OneKey` section plus picker-inline `Edit…` shortcut. Supports
add/edit/delete with masked password fields and show-all toggle, reusing
`ui::dialog` chrome and `SshPromptState`-style submit.

## 6. Security

- Settings + docs show WindTerm-derived warning: use only in SECURE environments;
  config-file secrets are readable by local file readers; listing/sending
  decrypts/exposes them; impersonating login screens can steal them.
- Secrets never logged, never included in `capture`/verification output, redacted
  in `Debug`. Stored plaintext in config file with warning (per Q3/D8).

## 7. Non-behavior

- Does not alter russh `spec.password` auto-supply, keychain `CredentialStore`,
  `~/.ssh/config` import, or SFTP flows.
- Does not add PTY output scanners, IME changes, or shell-integration protocol
  changes.
