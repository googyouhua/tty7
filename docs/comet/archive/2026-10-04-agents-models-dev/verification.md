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
- Completed: 2026-10-04T08:34:08.330Z
- Summary: All 7 pass after A6 repair (last-source persistence plus cache seed on open). No regressions.

## Acceptance

| ID | Result | Source | Criterion | Reason |
| --- | --- | --- | --- | --- |
| A1 | passed | brief.md | A1：URL 模式默认 models.dev、无本地 `opencode2`/`opencode` 但有网时，base=opencode 的表单加载后下拉出现 `opencode/*` 完整列表（含 `opencode-go` 源模型），不再只有 2 条内置。 | Default URL, opencode mapping, prefixing parse and auto-load in code; unit tests pass. |
| A2 | passed | brief.md | A2：有映射的 base（如 claude/codex/gemini）在无本地 catalog/CLI 时，下拉出现 models.dev 对应 provider 列表（如 anthropic/openai/google 的裸 id），数量>内置表。 | Per-base provider mapping with parse/load tests passing. |
| A3 | passed | brief.md | A3：无映射的 base（如 goose/droid）且未指定文件/URL 时行为不变：不发起 HTTP，有 models.json 则用之、无则内置/空，不报错不卡顿。 | Unmapped base plus default URL skips HTTP; no background task; fallback chain untouched. |
| A4 | passed | brief.md | A4：断网/8s 超时/URL 解析失败/文件缺失或非法时，下拉保留旧列表并显示红字失败原因，UI 不冻结；有 CLI 时回退本地 CLI 列表仍可用；缓存存在时离线可读缓存。 | Failure keeps old list with red-text reason, 8s timeouts, cache read on failure, background task. |
| A5 | passed | brief.md | A5：`cargo test -p tty7-core agent_roles` 全过（含新增映射、解析、缓存回退、去重单测）；`cargo check --bins` 通过；无新增重型依赖。 | 23/23 agent_roles tests, bins check, feature-gated build pass; no new heavy deps. |
| A6 | passed | brief.md | A6：文件模式：按 `docs/agents/models-load-file.md` 手写一份 JSON，表单选「文件」填路径点加载，下拉出现文件内列表；重开表单仍在；非法文件报红字且保留旧表。 | Last-used source persists per base, form restores toggle+text on open, list seeds from source-keyed cache; new unit tests pass. |
| A7 | passed | brief.md | A7：自定义 URL 与 launch 兜底：填自建镜像地址可加载；launch 行手写 `--model xxx` 时下拉不再追加（无重复传参，argv 只含一处 flag）；下拉选空 + launch 手写即兜底生效。 | Custom URL triggers load; launch dedup for bare and flag=value forms tested. |

## Checks

_No Runtime checks were recorded._

### Builder-reported evidence

These are Builder reports, not Runtime check receipts or independent verification results.

- cargo test -p tty7-core agent_roles: passed — 23/23 pass (added source_from_parts, last-source roundtrip)
- cargo check -p tty7-core --features remote-install: passed — ok
- cargo check --bins: passed — no errors, no new warnings in touched files
- GUI agent_roles_page test: passed — 1 passed
- Known limitation: Load-file flag field informational; launch flag still from model_flag table
- Known limitation: No native file dialog: file path is a text input
- Known limitation: Unmapped bases skip default-URL HTTP; custom URL still attempted
- Known limitation: Builds without remote-install disable HTTP fetch

## Blockers

_None._

## Risks and skipped work

- Live models.dev shape unverified without network; mismatch degrades to error with fallback.
- GUI offline-reopen judged by inspection plus headless tests.

## Previous iterations

| Goal cycle | Iteration | Attempt | Outcome | Unresolved | Summary | Completed |
| ---: | ---: | ---: | --- | --- | --- | --- |
| 1 | 1 | 1 | fail | A6 | A1-A5 and A7 pass; A6 fails on file-list reopen persistence. Next Build: persist last-used source per base and seed form from cache on open. | 2026-10-04T08:29:56.325Z |
| 1 | 2 | 1 | pass | — | All 7 pass after A6 repair (last-source persistence plus cache seed on open). No regressions. | 2026-10-04T08:34:08.330Z |



## Conclusion

All 7 pass after A6 repair (last-source persistence plus cache seed on open). No regressions.
