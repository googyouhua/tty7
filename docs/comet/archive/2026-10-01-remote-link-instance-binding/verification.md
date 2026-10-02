---
generated_from_state_version: 11
---

# Verification

## Current result

- Result: **Archived**
- Verification status: **Checks completed; result confirmed**
- Goal cycle: 1
- Iteration: 2
- Verifier attempt: 1
- Completed: 2026-10-01T13:37:41.814Z
- Summary: Three-file increment matches spec; default paths preserved; scoped suites pass.

## Acceptance

| ID | Result | Source | Criterion | Reason |
| --- | --- | --- | --- | --- |
| A1 | passed | brief.md | A1：本机 gyh 实例经远端链到 box（box 上有 `tty7-gyh` 与 `tty7-mca`、默认实例各有 workspace）→ 切换器只出现 gyh 实例的内容，不出现 mca 与默认实例的行。 | reuse->suffix+env retarget land both entry paths in tty7-<name>; units cover layout/quoting/override/fallback |
| A2 | passed | brief.md | A2：默认实例打开的 GUI 建链 → 行为与今天一致（默认实例内容）。 | reuse None keeps old path byte-for-byte; explicit server_command bypasses; all scoped suites green |
| A3 | passed | brief.md | A3：同机 gyh+mca 双链复用一条 ssh 连接 → 各自 entry 独立，先后顺序不影响归属（单测覆盖）。 | entry memo HashMap keyed by instance, all call sites pass instance_key |
| A4 | passed | brief.md | A4：远端无同名实例 → 自动建空实例并连上，不报错、不落到默认实例；非法本地名不可能出现（`memory_name_for` 只产出合法名或哨兵）。 | missing remote instance auto-created via --config-dir; only legal names or sentinel possible |

## Checks

| Check | Command | Working directory | Status | Exit | Duration |
| --- | --- | --- | --- | ---: | ---: |
| cargo fmt --check | fmt --check | . | passed | 0 | 3400 ms |
| relevant unit suites | test -p [REDACTED] --lib remote_link | . | passed | 0 | 593 ms |

### Builder-reported evidence

These are Builder reports, not Runtime check receipts or independent verification results.

- cargo fmt --check: passed — —
- remote_link 14 + instance 5 + router 26 + ssh 135 unit tests: passed — —
- cargo check -p tty7-core no errors: passed — —
- Known limitation: A1/A2 live SSH目检需用户在笔记本操作
- Known limitation: 远端TTY7_AS_ROOT不支持

## Blockers

_None._

## Risks and skipped work

- live SSH目检需用户在笔记本操作
- concurrent first-link may probe twice (harmless)

## Previous iterations

| Goal cycle | Iteration | Attempt | Outcome | Unresolved | Summary | Completed |
| ---: | ---: | ---: | --- | --- | --- | --- |
| 1 | 1 | 0 | recovery | — | Native check input changed after the candidate was built; a new Builder candidate is required before checks can run again. | 2026-10-01T13:33:52.678Z |
| 1 | 2 | 1 | pass | — | Three-file increment matches spec; default paths preserved; scoped suites pass. | 2026-10-01T13:37:41.814Z |



## Conclusion

Three-file increment matches spec; default paths preserved; scoped suites pass.
