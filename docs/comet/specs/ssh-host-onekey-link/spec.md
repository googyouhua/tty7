# SSH host ↔ OneKey link — complete target specification

## 1. Purpose

Let an SSH host reference one OneKey entry's username and password, so
connecting that host authenticates with the linked credential automatically.

## 2. Data model

- `SshProfile.onekey_entry_id: Option<String>` (`#[serde(default)]`,
  persisted in `servers.json` beside the host). `None` (including old files)
  means "no link": every existing behavior is unchanged.
- The id is the OneKey entry's stable uuid. Titles are display-only and never
  used for lookup, so renames never break the link.
- OneKey entries themselves are untouched by this change (no schema change to
  `OneKeyEntry` or `config.json`).

## 3. Host editor UI

- The SSH host editor (add and edit) gains a "OneKey" row: a dropdown listing
  all entries as `title (username, kind)` plus a "No link" option. Picking one
  stores its id on save; picking "No link" clears it.
- Host rows and the read-only details view show the linked entry title, or a
  "link broken" hint when the id matches no entry.
- The existing name/host/port/user/password fields keep working as the
  fallback credential and as the source of truth when unlinked.

## 4. Connect-time behavior

- In `build_spec_inner` (the single funnel for profile connect, restart,
  reconnect, jump chains, and remote-workspace specs), after the profile's
  own user is known: if `onekey_entry_id` resolves to an entry, the entry's
  non-empty username replaces the effective user and its password is used
  exactly where `spec.password` is used today (password auth for
  `Auto|Password`, all-password keyboard-interactive rounds).
- Empty entry username → keep the host's own user, override password only.
- Empty entry password → behave as if no link existed for the password
  (keychain/prompt chain untouched).
- Unlinked hosts: byte-for-byte the old behavior.

## 5. Dangling links

- Entry deleted (or id unknown): connect proceeds with the host's own
  credentials; the host list row and details show a broken-link hint naming
  the missing entry id's last known title where available. No error dialog,
  no failed-auth surprise from a stale secret, no crash.
- Re-linking or re-creating an entry does not resurrect old links (ids are
  uuids, never reused by the add form).

## 6. Security

- Linked passwords are read from `config.json` at connect time and handled
  like `spec.password`: never logged, never in `capture`/verification output,
  redacted in `Debug`.
- No new secret storage is introduced; the plaintext-at-rest property of
  OneKey entries is unchanged (user decision D5/D8 of the base change).

## 7. Non-behavior

- No change to key-passphrase handling, SFTP, port forwarding, `~/.ssh/config`
  import, or the keychain read/write paths.
- No PTY output scanning; no shell-integration protocol change.
