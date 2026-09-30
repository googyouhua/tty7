# Outcome

Shell terminal (any local/remote pane) provides OneKey autofill: a manual credential picker that fills a chosen username and/or password into the active PTY, modeled on WindTerm OneKey/Autofill.

# Scope

## Source coverage

Source: https://kingtoolbox.github.io/2024/01/20/onekey_autofill/ (user-supplied requirements source, fetched 2026-09-29).

| Unit | Source location | Read status | Retained semantics | Spec location | Acceptance | Coverage | Reason / replacement |
|---|---|---|---|---|---|---|---|
| U1 Quick Start: autofill login credentials; usernames+passwords defined in SSH OneKey and Account OneKey support autofill | page `#Quick-Start` paragraph | complete | Two credential kinds (SSH-bound + generic account); picker fills username/password into shell | specs/onekey-autofill/spec.md §2,§4 | A1,A2,A3 | covered | — |
| U2 Security warning: use only in SECURE env; OneKeys encrypted at rest but decrypted when listing/sending; impersonating logins can steal | page blockquote | complete | Security constraint: warn user; storage choice recorded explicitly | specs/onekey-autofill/spec.md §6; brief Constraints | A5 | covered | User chose config-file storage (see D5); warning becomes load-bearing |
| U3 Intro video demo of picker interaction | page `#Intro-Video` `/img/onekey_autofill.gif` | partial | Interaction example only; exact keys/clicks not readable from gif | — | — | background | Reason: user chose manual picker (D3); video not used as executable semantics |
| U0 User request: shell终端提供onekey功能 | /comet invocation args | complete | Manual picker in shell terminal for both SSH and generic entries, username/password choice | specs/onekey-autofill/spec.md §2–§4 | A1–A4 | covered | — |

# Non-goals

- Auto-detection of `password:` / `login:` prompts in PTY output (user chose manual picker only).
- Changing existing SSH russh auto-supply flow (`spec.password` silent use + sheet on reject); OneKey is an additional manual fill path.
- OS keychain storage for OneKey entries (user chose config file).
- SFTP panel / port-forwarding credential reuse in this change.

# Acceptance examples

- A1: In an active shell pane, invoking OneKey picker shows saved entries (both SSH-bound and generic account kinds); picking an entry and choosing `Username` types the username into the PTY input (via prompt-editor-aware paste path).
- A2: Same picker choosing `Password` fills the password without echoing it into visible UI beyond masked picker; works when prompt editor is active and when raw PTY owns input.
- A3: Picker offers `Username+Password` (username, then Enter/Tab equivalent, then password per spec) as one action; user can cancel with Esc without sending bytes.
- A4: Entries are manageable (add/edit/delete) and persist across restarts via config file; `tty7 ls`-style restart does not lose entries.
- A5: Docs/settings surface the WindTerm-derived security warning (secure-environment use; config-file secrets are recoverable by local readers).

# Constraints and invariants

- Never write secrets to logs, `capture` output, or verification reports; Debug redaction follows existing `without_secrets` pattern.
- Fill path must respect `prompt_editor` / `shell_owns_prompt`: use `paste()`-equivalent routing when inline editor active, direct `send_to_pty` only for password-prompt case; no raw `terminal.write` bypass.
- Config-file storage is user-explicit and insecure by design: secrets recoverable by anyone reading the file; UI must warn. No silent migration to keychain in this change.
- Existing SSH `CredentialStore`/keychain flows untouched.

# Decisions

- D1 Workspace: new worktree (`.worktrees/shell-onekey-autofill`, branch `comet/shell-onekey-autofill`) — another active change + uncommitted work.
- D2 Scope: Both — SSH-bound entries + generic account entries (mirrors WindTerm SSH OneKey + Account OneKey).
- D3 Trigger: manual picker only — shortcut + command palette + terminal context menu; no PTY prompt auto-detect.
- D4 Fill: per-pick choice of Username / Password / Username+Password.
- D5 Storage: config file (user choice, overrides recommendation of OS keychain). Warning in A5/C constraints applies.
- D6 Management UI: Settings `OneKey` section + picker-inline edit (Q1 Settings section).
- D7 Shortcut: `Ctrl-Shift-I` opens picker (user-typed; palette name `OneKey: Autofill…`).
- D8 At-rest protection: plaintext in config.json with security warning (Q3).
- D9 Sequence: username, Enter, password, final Enter auto-submit (changed per user request on 2026-09-30; was Enter-no-submit).

# Open questions

None. 2026-09-29 user confirmed outcome/scope/decisions D1–D9/acceptance A1–A5/non-goals; ready to enter Build.

# Verification expectations

- `cargo test` (workspace) + manual picker flow in dev GUI: open shell pane, trigger picker, fill username/password/sequence, cancel path sends zero bytes.
- Restart persistence check: entries survive app restart from config file.
- No secret leakage in test output or `capture`.
