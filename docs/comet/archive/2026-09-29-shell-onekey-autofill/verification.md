---
generated_from_state_version: 18
---

# Verification

## Current result

- Result: **Archived**
- Verification status: **Checks completed; result confirmed**
- Goal cycle: 1
- Iteration: 4
- Verifier attempt: 1
- Completed: 2026-09-29T14:01:39.632Z
- Summary: Iteration 4 repairs verified: A1 overlay+keys and A4 add/edit now present in code with green suites (core onekey 8, bin onekey 11, settings 53, terminal::view 288). OneKeyEntry Debug redaction confirmed with passing test.

## Acceptance

| ID | Result | Source | Criterion | Reason |
| --- | --- | --- | --- | --- |
| A1 | passed | brief.md | A1: In an active shell pane, invoking OneKey picker shows saved entries (both SSH-bound and generic account kinds); picking an entry and choosing `Username` types the username into the PTY input (via prompt-editor-aware paste path). | render_onekey_menu (view.rs:7443) wired into render via .children(onekey_menu) at :7879; early on_key_down routing at :2532 and handle_onekey_key at :5305 drive Stage/handle_key/filtered_entries (onekey.rs). cargo test --bin tty7-app onekey: 11 pass incl. 2 view e2e tests. |
| A2 | passed | brief.md | A2: Same picker choosing `Password` fills the password without echoing it into visible UI beyond masked picker; works when prompt editor is active and when raw PTY owns input. | fill_onekey (view.rs:3284) routes username via paste and password direct to PTY; overlay rows paint title/username/kind only (:7501-7508), never password. View e2e fills pw2 to raw PTY with nothing in editor state. |
| A3 | passed | brief.md | A3: Picker offers `Username+Password` (username, then Enter/Tab equivalent, then password per spec) as one action; user can cancel with Esc without sending bytes. | fill_bytes emits user\rpass with no final submit (core/onekey.rs:78-83); Esc/Ctrl-G/Ctrl-C map to Cancel (onekey.rs:154). Unit cancel_sends_zero_bytes + view onekey_escape_cancels_without_touching_the_pty both pass. |
| A4 | passed | brief.md | A4: Entries are manageable (add/edit/delete) and persist across restarts via config file; `tty7 ls`-style restart does not lose entries. | OneKeyForm (settings.rs:1440) rendered in pages.rs render_onekey_group (:1110) with Add/Edit form, 1-64 char + unique-title validation (save_onekey_form :2254), per-row Edit/Delete (:1281/:1286), persistence via update_config (:2298) and delete_onekey_entry (app.rs:3032). settings suite 53 pass; terminal::view 288 pass. |
| A5 | passed | brief.md | A5: Docs/settings surface the WindTerm-derived security warning (secure-environment use; config-file secrets are recoverable by local readers). | ONEKEY_SECURITY_WARNING defined core/onekey.rs:104, surfaced in Settings group (pages.rs:1123) and docs/reference/configuration.mdx:123. |

## Checks

_No Runtime checks were recorded._

### Builder-reported evidence

These are Builder reports, not Runtime check receipts or independent verification results.

- cargo check (workspace): passed — warnings only
- tty7-core onekey unit tests (8): passed — incl. Debug redaction
- tty7-core config tests (60): passed — persistence/sanitize
- bin onekey picker tests (9): passed — state machine incl. handle_key stages
- bin onekey view tests (2): passed — end-to-end keys to PTY bytes + cancel-zero-bytes
- bin terminal::view suite (288): passed — no regressions
- bin keymap tests (50): passed — —
- bin search tests (169): passed — —
- bin settings tests (53): passed — —
- Known limitation: Picker rows are keyboard-driven only; mouse clicks fall through to the grid (same as completion menu)

## Blockers

_None._

## Risks and skipped work

_None reported._

## Previous iterations

| Goal cycle | Iteration | Attempt | Outcome | Unresolved | Summary | Completed |
| ---: | ---: | ---: | --- | --- | --- | --- |
| 1 | 1 | 0 | recovery | — | Native check input changed after the candidate was built; a new Builder candidate is required before checks can run again. | 2026-09-29T13:40:13.711Z |
| 1 | 2 | 0 | recovery | — | Native check input changed after the candidate was built; a new Builder candidate is required before checks can run again. | 2026-09-29T13:41:14.429Z |
| 1 | 3 | 1 | fail | A1, A4 | 3/5 passed (A2, A3, A5). A1 fails: picker state + triggers wired but never rendered or keyboard-drivable in the GUI. A4 fails: persistence + delete work, but Settings add/edit UI required by spec section 5 is missing (config-file hand-edit only). All executed suites green: tty7-core onekey 7/7, bin onekey 4/4, keymap 50/50, config 60/60, settings 53/53, search 169/169. | 2026-09-29T13:46:50.453Z |
| 1 | 4 | 1 | pass | — | Iteration 4 repairs verified: A1 overlay+keys and A4 add/edit now present in code with green suites (core onekey 8, bin onekey 11, settings 53, terminal::view 288). OneKeyEntry Debug redaction confirmed with passing test. | 2026-09-29T14:01:39.632Z |



## Conclusion

Iteration 4 repairs verified: A1 overlay+keys and A4 add/edit now present in code with green suites (core onekey 8, bin onekey 11, settings 53, terminal::view 288). OneKeyEntry Debug redaction confirmed with passing test.
