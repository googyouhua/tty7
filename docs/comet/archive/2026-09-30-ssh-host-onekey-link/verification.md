---
generated_from_state_version: 9
---

# Verification

## Current result

- Result: **Archived**
- Verification status: **Checks completed; result confirmed**
- Goal cycle: 1
- Iteration: 1
- Verifier attempt: 1
- Completed: 2026-09-30T14:09:13.798Z
- Summary: All 5 acceptance items verified in code plus passing suites: tty7-core core::onekey (13), tty7-app ssh_connect incl. 3 link tests (16+ pass), tty7-app settings (53). Read-only verification, no code modified.

## Acceptance

| ID | Result | Source | Criterion | Reason |
| --- | --- | --- | --- | --- |
| A1 | passed | brief.md | A1: SSH 新增/编辑 host 表单有 OneKey 关联下拉；选定条目保存后，host 行与详情页显示已关联的条目 title。 | Host editor has OneKey dropdown (onekey_link_row in src/ui/settings/hosts.rs, No-link + title (user) options wired to set_ssh_onekey_link; SshProfileForm.onekey_link seed/collect in settings.rs); saved rows suffix 'OneKey: {title}' and details show 'title (user)'. |
| A2 | passed | brief.md | A2: 连接已关联 host（password/auth-auto）时直接用条目用户名密码鉴权成功，全程不弹密码框。 | build_spec_inner overrides effective user and spec.password from linked_credential; native_ssh_spec_for_profile and resolve_persisted_ssh_spec thread cfg.onekey_entries; auth.rs try_password + all-password keyboard-interactive consume spec.password first (prompt only on auth failure). Test linked_host_uses_onekey_username_and_password passes. |
| A3 | passed | brief.md | A3: 去 OneKey 里改该条目的密码（或改名），不碰 host，再次连接用新密码成功（live 引用）。 | linked_credential resolves entry id live at connect time (no snapshot); empty-username/empty-password edge rules keep host user / keychain chain. Test link_follows_entry_edits_without_touching_the_host passes. |
| A4 | passed | brief.md | A4: 删除被关联的 OneKey 条目后，连接回退到 host 自身凭证；host 行显示链接失效提示；无崩溃。 | Unknown id resolves to None so connect falls back to host credentials; is_link_broken drives row hint 'OneKey link broken' and details 'link broken (entry missing)'; no error dialog path. Tests dangling_link_resolves_to_nothing_and_reports_broken and dangling_link_falls_back_to_host_credentials pass. |
| A5 | passed | brief.md | A5: 解除关联后恢复纯 host 凭证行为，与从未关联一致。 | onekey_entry_id None (incl. 'No link' + blank-id + old-file serde default) yields linked_credential None and is_link_broken false, i.e. byte-identical old behavior. Test unlinked_host_ignores_entries passes. |

## Checks

_No Runtime checks were recorded._

### Builder-reported evidence

These are Builder reports, not Runtime check receipts or independent verification results.

- cargo check (workspace): passed — warnings only
- tty7-core onekey tests (13): passed — incl. 5 new link resolver tests
- tty7-core ssh_profile tests (15): passed — —
- tty7-core config tests (60): passed — —
- bin ssh_connect tests (16): passed — incl. 3 new link override/fallback/unlink tests
- bin settings tests (53): passed — —
- Known limitation: Live GUI connect against a real SSH server not exercised in unit tests; override verified at spec-build level

## Blockers

_None._

## Risks and skipped work

_None reported._

## Previous iterations

| Goal cycle | Iteration | Attempt | Outcome | Unresolved | Summary | Completed |
| ---: | ---: | ---: | --- | --- | --- | --- |
| 1 | 1 | 1 | pass | — | All 5 acceptance items verified in code plus passing suites: tty7-core core::onekey (13), tty7-app ssh_connect incl. 3 link tests (16+ pass), tty7-app settings (53). Read-only verification, no code modified. | 2026-09-30T14:09:13.798Z |



## Conclusion

All 5 acceptance items verified in code plus passing suites: tty7-core core::onekey (13), tty7-app ssh_connect incl. 3 link tests (16+ pass), tty7-app settings (53). Read-only verification, no code modified.
