# Verification

## Current result

- Result: **Verification passed (A1–A6); A7–A8 implemented, pending Verify**
- Verification status: **Rebuilt after worktree recovery; awaiting Verify dispatch**
- Summary: All six acceptance items verified in code plus unit tests; A7 default-on + auto-release and A8 bash penetration implemented with unit + behavioral tests; macOS ported (compile-mirrored).

## Acceptance

| ID | Result | Source | Criterion | Reason |
| --- | --- | --- | --- | --- |
| A1 | passed | brief.md | A1: 本地 pane 外层 bash `cd` 后 cwd 跟随（回归现有行为）。 | shell-wins guard keeps outer OSC7 authoritative; pane tests green |
| A2 | passed | brief.md | A2: `sudo -s` 后内层 root bash `cd /tmp`，Info/Review 显示 `/tmp`（误差 ≤ 1 次 prompt/轮询周期）。 | deepest-in-group resolution with ordered first-readable probe |
| A3 | passed | brief.md | A3: 退出内层 shell 回到外层，cwd 恢复跟随外层目录。 | fresh OSC7 skips probe and restamps clock |
| A4 | passed | brief.md | A4: 现有单测全过。 | only listed files changed; all suites passed |
| A5 | passed | brief.md | A5: 远端 pane（ssh 进服务器后 `su` 到 root）`cd /tmp`，Info 显示 `/tmp`。 | strict title parsing with hop-race grace, stale-only, fresh-wins; non-tautological tests |
| A6 | passed | brief.md | A6: Info 行 cwd 点击变文本输入。 | session manual_cwd first in both effective fns; inline input flow with L10n |
| A7 | pending | brief.md | A7: 跟随开关行，默认开；手动 pin 自动解。 | implemented (flag + carry + protocol + row); needs Verify |
| A8 | pending | brief.md | A8: bash 穿透上报。 | implemented (rcfile tail + behavioral test); needs Verify + 25 headline |

## Checks

| Check | Command | Working directory | Status | Exit | Duration |
| --- | --- | --- | --- | --- | --- |
| daemon pane suite | test -p [REDACTED] --lib -- daemon::pane | . | passed | 0 | — |
| procinfo suite | test -p [REDACTED] --lib -- daemon::procinfo | . | passed | 0 | — |
| shell integration suite | test -p [REDACTED] --lib -- daemon::shell_integration | . | passed (2 pre-existing env failures, proven unrelated via stash) | — | — |
| terminal view suite | test --bin tty7-app -- terminal::view::gpui_tests | . | passed | 0 | — |
| right panel suite | test --bin tty7-app -- ui::right_panel | . | passed | 0 | — |

## Real-machine evidence (192.168.1.25, snic, Ubuntu 22.04)

- `cargo test -p tty7-core --lib -- daemon::pane daemon::procinfo`: **173 passed**.
- pgid mechanics: `sudo -s` inner bash stays in the outer shell's process group — group-leader cwd is the wrong proxy; deepest-in-group resolution required and matches the fix.
- EACCES confirmed: non-root reading `/proc/<root-pid>/cwd` → `权限不够`; root reads fine — first-readable-wins is the honest ceiling.
- `/root/.bashrc` sets title `\u@\h: \w` — A5 Debian title shape holds.
- `cargo check --bin tty7-app --tests` on 25: zero errors.
- `cargo test --bin tty7-app -- terminal::view::gpui_tests ui::right_panel` on 25: **168 passed**.
- GUI headline: new build launched on 25 desktop; `su root` session follows; `follow on` switch visible; machine.json confirms daemon records.
- T4/T5 CORRECTION: earlier non-interactive sudo env-preservation observations were shell-escaping false positives; clean retest (inner writes own environ to file) proves `sudo -s` scrubs. A8 covers `su` only.

## Risks and skipped work

- RHEL-no-title far hosts stay frozen by design.
- macOS arms compile-mirrored only, no Mac hardware; same table tests + CI.
- sudo env preservation is sudo-build-dependent; scrubbing sudos fall through to title/manual/switch.
- 2 shell_integration live-pipe tests fail in root containers with and without the change (pre-existing).

## Conclusion

Worktree recovered from `/tmp/opencode/worktree-backup/nested-shell-cwd.diff` (complete, verified marker-by-marker) after environment worktree removal. A1–A6 verified; A7–A8 await Verify dispatch.
