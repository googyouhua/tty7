---
stepsCompleted: ['step-01-preflight-and-context', 'step-02-generation-mode', 'step-03-test-strategy', 'step-04c-aggregate', 'step-05-validate-and-complete']
lastStep: 'step-05-validate-and-complete'
lastSaved: '2026-10-22'
workflowType: 'testarch-atdd'
storyId: '1'
storyKey: '1-container-transport-skeleton'
storyFile: '_bmad-output/specs/spec-container-remote-host/stories/1-container-transport-skeleton.md'
atddChecklistPath: '_bmad-output/test-artifacts/atdd-checklist-1-container-transport-skeleton.md'
generatedTestFiles:
  - 'crates/tty7-core/src/daemon/install/container/atdd.rs'
  - 'crates/tty7-core/src/daemon/install/container.rs' # #[cfg(test)] mod atdd; wiring; remove both together
inputDocuments:
  - '_bmad-output/specs/spec-container-remote-host/stories/1-container-transport-skeleton.md'
  - '_bmad-output/specs/spec-container-remote-host/SPEC.md'
  - '_bmad-output/specs/spec-container-remote-host/container-host-design.md'
  - 'Cargo.toml'
  - 'crates/tty7-core/Cargo.toml'
  - 'crates/tty7-core/tests/username_selector_lifecycle.rs'
  - '.agents/skills/bmad-testarch-atdd/resources/knowledge/data-factories.md'
  - '.agents/skills/bmad-testarch-atdd/resources/knowledge/component-tdd.md'
  - '.agents/skills/bmad-testarch-atdd/resources/knowledge/test-quality.md'
  - '.agents/skills/bmad-testarch-atdd/resources/knowledge/test-healing-patterns.md'
  - '.agents/skills/bmad-testarch-atdd/resources/knowledge/test-levels-framework.md'
  - '.agents/skills/bmad-testarch-atdd/resources/knowledge/test-priorities-matrix.md'
  - '.agents/skills/bmad-testarch-atdd/resources/knowledge/ci-burn-in.md'
acceptanceCriteria:
  - id: 'AC-1'
    idSource: 'generated'
    text: 'Given a running Linux container, when the user registers it and opens a pane, then a shell runs inside the container and its workspace is tagged as a container.'
  - id: 'AC-2'
    idSource: 'generated'
    text: 'Given the same running container reopened, then the installed server is reused rather than re-pushed.'
  - id: 'AC-3'
    idSource: 'generated'
    text: 'Given a container whose only writable location is noexec, when registered, then the server relocates to an executable dir, or degrades with a stated reason.'
  - id: 'AC-4'
    idSource: 'generated'
    text: 'Given a --network none container, then the pane still works (transport is host IPC).'
---

# ATDD Checklist - Epic 0, Story 1: Container transport skeleton — register a container, push the server, open a container shell

**Date:** 2026-10-22
**Author:** hua
**Primary Test Level:** Integration (backend/Rust; `cargo test`)

---

## Story Summary

Register a running Linux container as a tty7 workspace, push the version-matched `tty7-server` into it over `docker exec`, and carry the existing daemon protocol on that exec stdio channel so a container pane is an ordinary remote pane.

**As a** tty7 user working inside a container
**I want** the container registered as a remote host and a shell opened inside it
**So that** the sidebar, cwd and remote-workspace behaviour keep working in container panes

---

## Acceptance Criteria

| ID   | ID source | Acceptance criterion |
| ---- | --------- | -------------------- |
| AC-1 | generated | Given a running Linux container, when the user registers it and opens a pane, then a shell runs inside the container and its workspace is tagged as a container. |
| AC-2 | generated | Given the same running container reopened, then the installed server is reused rather than re-pushed. |
| AC-3 | generated | Given a container whose only writable location is noexec, when registered, then the server relocates to an executable dir, or degrades with a stated reason. |
| AC-4 | generated | Given a `--network none` container, then the pane still works (transport is host IPC). |

---

## Preflight & Context

### Stack detection

`config.test_stack_type` is `auto`; `{project-root}` carries `Cargo.toml`, no `package.json`/`playwright.config.*`, no mobile manifests. **Detected stack: `backend`.** Test runner: `cargo test` (workspace). Per the workflow, backend uses AI generation and has no browser/E2E level.

### Prerequisites

- [x] Story with clear acceptance criteria — `_bmad-output/specs/spec-container-remote-host/stories/1-container-transport-skeleton.md` (the four bullets in **Tasks & Acceptance → Acceptance Criteria**; no supplied AC ids, so ids were generated in source order).
- [x] Test framework configured — `crates/tty7-core` unit tests (`#[cfg(test)]`) and `crates/tty7-core/tests/*.rs` integration tests run under `cargo test`.
- [x] Development environment — Rust workspace in `.worktrees/container-remote-host`; `target/` present.
- [!] No `docker` daemon/CLI in this environment. The unit boundary (`ContainerRemoteOps`) and the pure routing/memo/relocation seams are testable; the live `docker exec` round-trip remains a manual check (already recorded in the story's I/O matrix audit).

### Criterion registry notes

- All four criteria were **unnamed** in the story; ids `AC-1`…`AC-4` were generated deterministically in source order and are stable for identical story content.
- The story is already implemented (`status: done`, commit `de99fa05`). This run is a **retrofit**: the scaffolds pin the story's promised behaviour, and are ignored by default so they cannot mask a regression until a developer activates them.

### TEA config flags read

| Flag | Value | Effect here |
| ---- | ----- | ----------- |
| `tea_use_playwright_utils` | `true` | Not applicable: no `@seontechnologies/playwright-utils`, no Playwright runner, backend stack. |
| `tea_use_pactjs_utils` | `true` | Not applicable: no `@seontechnologies/pactjs-utils`, no Pact artifacts, contract testing not requested. |
| `tea_pact_mcp` | `mcp` | No contract branch opened, so not probed. |
| `tea_browser_automation` | `auto` | Not applicable: backend has no browser surface. |
| `test_stack_type` | `auto` | Resolved to `backend`. |
| `tea_execution_mode` | `auto` | Resolved to **sequential** for this run: the backend stack has one worker (no E2E/API HTTP worker, no Playwright recording). |

### Knowledge fragments loaded

Core: `data-factories.md`, `component-tdd.md`, `test-quality.md`, `test-healing-patterns.md`.
Backend: `test-levels-framework.md`, `test-priorities-matrix.md`, `ci-burn-in.md`.
Not selected (conditions false): frontend `selector-resilience.md` / `timing-debugging.md`; all Playwright-utils, Playwright CLI, Pact.js and Pact MCP fragments.

### Affected components

`crates/tty7-core/src/core/session.rs` (`RemoteTarget::Container`), `crates/tty7-core/src/daemon/router.rs` (`RouteTarget::Container`, `RouteHeader`), `crates/tty7-core/src/daemon/remote_link.rs` (`RemoteLink::container`), `crates/tty7-core/src/daemon/install/container.rs` (`ContainerRemoteOps`, memo, relocation), `src/ui/remote_workspace.rs` / `remote_connect.rs` / `switcher.rs` (entry + labeling), `src/terminal/remote.rs` (`--pane`).

---

## Generation Mode (Step 2)

**Chosen: AI generation.** `{detected_stack}` is `backend`, so recording mode is skipped by the workflow. Scenarios were derived from the story's Acceptance Criteria and I/O & Edge-Case Matrix, the design companion (`container-host-design.md`), and the implemented public seams (`docker_args`, `RouteHeader`, `RemoteTarget`, `ensure_container_server`, `relocation`). Recording (Playwright CLI/MCP) is not applicable: there is no browser surface.

---

## Test Strategy (Step 3)

### Level selection

Backend rules apply: **unit** for pure functions/edge branches, **integration** for seam interactions; **no E2E**. The live `docker exec` daemon boundary cannot be crossed in this environment (no docker), so the container-side effects are pinned through the transport argv, the routing/tagging seam, the reuse memo, and the relocation decision — exactly the seams the story's own audit names.

### Coverage plan

| AC | Priority | Level | Primary red-phase scaffold (one per criterion) | Criterion-defining assertion (first that can fail) | Secondary / green-phase branches |
| -- | -------- | ----- | ---------------------------------------------- | -------------------------------------------------- | -------------------------------- |
| AC-1 | P0 | Integration | `ac1_registering_a_running_container_opens_a_shell_inside_it` | `docker_args("web", [… "--stdio", "--pane"])` equals `exec -i web <server> --stdio --pane` — the pane's process runs **inside** the container. | Tag assertions: `RouteHeader::container("web").for_pane().describe() == "container web"`, `origin_key() == "container:web"`, `RemoteTarget::Container.connection_key() == "container:web"`, `hosts_our_server()`, `!is_ssh()`, `LinkKind::Container.label() == "container-stdio"`. Green-phase: real registration + pane open against a live container (manual, needs docker). |
| AC-2 | P1 | Integration | `ac2_reopening_the_same_container_reuses_the_proved_server` | `ensure_container_server(<remembered>)` returns `Ok(<same binary>)` **with no docker on the host** — a re-push would fail. | Green-phase: forget → re-prove; recreated container drops the memo on bridge close; concurrent first opens share one install lock. |
| AC-3 | P1 | Unit | `ac3_a_noexec_home_relocates_the_server_or_states_why_it_cannot` | `relocation("/home/dev", Some("/run/user/1000"))` returns `Ok(Some("/run/user/1000"))` — home rejected, executable base chosen. | Degradation arm: `relocation(home, None)` is an `Err` naming the home and “unwritable or mounted noexec”. Green-phase: the real probe writes+runs a throwaway script on each candidate; `$HOME` unset falls back to `/etc/passwd`; restart/replace find the relocated base. |
| AC-4 | P1 | Unit (structural) | `ac4_a_network_none_container_pane_uses_the_host_docker_channel` | `docker_args("air-gapped", [server, "--stdio"])` equals `exec -i air-gapped <server> --stdio` — the protocol rides the host docker socket. | Negative assertions: no `-t`, no `--network`, no `run`; `host_label == "container:air-gapped"`. Green-phase: live `--network none` container pane (manual, needs docker). |

### Red-phase confirmation

- Each criterion has exactly one ignored red-phase leaf; secondary branches are checklist work, not extra skipped leaves.
- Each leaf's first potentially failing assertion isolates the exact promised status/scalar: the transport argv (AC-1), `Ok(reused_binary)` without docker (AC-2), the relocation answer (AC-3), the transport argv minus network flags (AC-4).
- Setup is unasserted and opaque: `forget_container_server` / `remember` state is installed before the criterion assertion and never branched on; the no-docker condition is the observable that makes AC-2's reuse assertion meaningful.
- **Retrofit caveat:** because story 1 is already implemented, activating a scaffold today passes; these are characterisation scaffolds, not currently-failing ones. A genuinely red scaffold for AC-1/AC-4 would require a docker shim; that was rejected as it would test the shim, not the system (see Notes).

---

## Red-Phase Test Scaffolds Created (Step 4)

**Execution mode:** sequential (backend stack; one worker; E2E worker N/A — no browser surface, per Step 3's "no E2E for pure backend").
**Red-phase marker:** `#[ignore = "ATDD RED: …"]` (Rust's equivalent of `test.skip()`), with the criterion id in the reason.

### API / Integration Tests (4 tests, all ignored)

**File:** `crates/tty7-core/src/daemon/install/container/atdd.rs`
**Module declaration:** `crates/tty7-core/src/daemon/install/container.rs` → `#[cfg(test)] mod atdd;` (test-only; no production code).

| # | Test | Criterion | Priority | Status | Verifies |
| - | ---- | --------- | -------- | ------ | -------- |
| 1 | `ac1_registering_a_running_container_opens_a_shell_inside_it` | AC-1 | P0 | RED — `#[ignore]` | Pane process is `docker exec -i web <server> --stdio --pane`; workspace/target is tagged `container:web`, owns an install, is not SSH, and reports `container-stdio`. |
| 2 | `ac2_reopening_the_same_container_reuses_the_proved_server` | AC-2 | P1 | RED — `#[ignore]` | `ensure_container_server` returns the memoized binary before any docker probe (a re-push/re-probe would fail on this host). |
| 3 | `ac3_a_noexec_home_relocates_the_server_or_states_why_it_cannot` | AC-3 | P1 | RED — `#[ignore]` | A rejected `$HOME` relocates to the executable base; with no base, degradation names the home and the “unwritable or mounted noexec” reason. |
| 4 | `ac4_a_network_none_container_pane_uses_the_host_docker_channel` | AC-4 | P1 | RED — `#[ignore]` | Transport is exactly `exec -i <ctr> <server> --stdio`, with no `-t`, no `--network*` and no `run`; host label is `container:<ctr>`. |

### E2E / Component Tests

N/A — backend stack, no browser/component surface. Live container behaviour is a documented manual check (see Mock Requirements).

### Data factories / fixtures

N/A — no database, HTTP payload or random data. Fixtures are the production seams themselves:
`docker_args`, `RouteHeader::container`, `RemoteTarget::Container`, the process-global install memo (pid-unique key, forgotten before/after), and `relocation`. This avoids adding a docker shim that would test the shim rather than the system.

---

## Mock Requirements

### docker CLI / daemon

**Commands exercised:** `docker exec -i <ctr> …` only.

**Success:** not mocked. The scaffolds stop at the transport argv, memo and relocation decisions, because a fake `docker` would have to emulate the installer's stat/put/launch protocol and would assert the fake, not tty7.

**Failure:** the absence of a docker CLI is itself the discriminator in AC-2 (a memo miss reaches a probe and fails).

**Notes:** live evidence remains manual:
- register a running Linux container → pane opens a shell inside it, workspace tagged as a container;
- reopen → no re-push;
- `--network none` container → pane still works;
- `$HOME` read-only/noexec → relocated install or a stated reason.

---

## Required data-testid Attributes

N/A — backend/Rust; no UI test surface in this story. (Sidebar labeling is asserted at the routing/target layer in AC-1.)

---

## Implementation Checklist

> Retrofit caveat: story 1 is already implemented (`de99fa05`). These tasks are the **characterisation/green verification** path: activate a scaffold, confirm it still passes, and only treat it as red if you are re-implementing the seam.

### Test 1 — AC-1 (`P0`) `ac1_registering_a_running_container_opens_a_shell_inside_it`

**File:** `crates/tty7-core/src/daemon/install/container/atdd.rs`

- [ ] Remove the `#[ignore]` for AC-1.
- [ ] Confirm `docker_args` still composes `exec -i <ctr> <server> --stdio --pane`.
- [ ] Confirm `RemoteTarget::Container` / `RouteHeader::container` still tag `container:<ctr>` and `!is_ssh()`.
- [ ] Run: `cargo test -p tty7-core --lib atdd::ac1`
- [ ] ✅ Green: shell runs inside the container and the workspace is tagged as a container.
- [ ] Manual (needs docker): register `web`, open a pane, confirm a shell inside `web`.

### Test 2 — AC-2 (`P1`) `ac2_reopening_the_same_container_reuses_the_proved_server`

- [ ] Remove the `#[ignore]` for AC-2.
- [ ] Confirm `ensure_container_server` still short-circuits on `remembered_container_server`.
- [ ] Confirm the memo is forgotten on a bridge close that never answered (recreated container).
- [ ] Run: `cargo test -p tty7-core --lib atdd::ac2`
- [ ] ✅ Green: reopen returns the proved binary; no re-push.
- [ ] Green-phase add: a failing `RemoteOps` injection to prove the install path is never entered.

### Test 3 — AC-3 (`P1`) `ac3_a_noexec_home_relocates_the_server_or_states_why_it_cannot`

- [ ] Remove the `#[ignore]` for AC-3.
- [ ] Confirm `relocation(home, Some(base))` returns the base when it differs from home.
- [ ] Confirm `relocation(home, None)` fails with the home and the noexec/unwritable reason.
- [ ] Run: `cargo test -p tty7-core --lib atdd::ac3`
- [ ] ✅ Green: relocates or degrades with a stated reason.
- [ ] Green-phase add: probe-level coverage that writes+runs the throwaway script (existing `the_place_probe_relocates_past_a_home_that_cannot_hold_a_bin_directory`).

### Test 4 — AC-4 (`P1`) `ac4_a_network_none_container_pane_uses_the_host_docker_channel`

- [ ] Remove the `#[ignore]` for AC-4.
- [ ] Confirm `docker_args` remains `exec -i` with no `-t`, no `--network*`, no `run`.
- [ ] Confirm `host_label` is `container:<ctr>`.
- [ ] Run: `cargo test -p tty7-core --lib atdd::ac4`
- [ ] ✅ Green: transport is host IPC, independent of container networking.
- [ ] Manual (needs docker): repeat the pane check in a `--network none` container.

---

## Running Tests

```bash
# Red-phase baseline: all four scaffolds skipped
USER=root LOGNAME=root cargo test -p tty7-core --lib atdd

# Activate every scaffold (retrofit verification)
USER=root LOGNAME=root cargo test -p tty7-core --lib atdd -- --ignored

# One criterion at a time
USER=root LOGNAME=root cargo test -p tty7-core --lib atdd::ac1

# Whole crate, for regression beside the existing container tests
USER=root LOGNAME=root cargo test -p tty7-core --lib container
```

---

## Test Execution Evidence

**Command:** `USER=root LOGNAME=root cargo test -p tty7-core --lib atdd`

```text
running 4 tests
test daemon::install::container::atdd::ac1_registering_a_running_container_opens_a_shell_inside_it ... ignored, ATDD RED: activate when implementing AC-1 (registering a running container opens a shell inside it)
test daemon::install::container::atdd::ac2_reopening_the_same_container_reuses_the_proved_server ... ignored, ATDD RED: activate when implementing AC-2 (reopening the same container reuses the server)
test daemon::install::container::atdd::ac3_a_noexec_home_relocates_the_server_or_states_why_it_cannot ... ignored, ATDD RED: activate when implementing AC-3 (noexec home relocates or states why not)
test daemon::install::container::atdd::ac4_a_network_none_container_pane_uses_the_host_docker_channel ... ignored, ATDD RED: activate when implementing AC-4 (a --network none container pane still works)

test result: ok. 0 passed; 0 failed; 4 ignored; 0 measured; 1859 filtered out; finished in 0.00s
```

**Command:** `USER=root LOGNAME=root cargo test -p tty7-core --lib atdd -- --ignored`

```text
running 4 tests
test daemon::install::container::atdd::ac1_registering_a_running_container_opens_a_shell_inside_it ... ok
test daemon::install::container::atdd::ac2_reopening_the_same_container_reuses_the_proved_server ... ok
test daemon::install::container::atdd::ac3_a_noexec_home_relocates_the_server_or_states_why_it_cannot ... ok
test daemon::install::container::atdd::ac4_a_network_none_container_pane_uses_the_host_docker_channel ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 1859 filtered out; finished in 0.00s
```

**Summary:** 4 tests, 4 skipped by default, 0 failed; 4/4 pass when activated (retrofit — the implementation already exists).

---

## Knowledge Base References Applied

- `test-quality.md` — behavioural names, explicit assertions, no tautologies, one concern per leaf, documented skips with a condition that re-enables them.
- `test-levels-framework.md` — unit for the pure relocation decision, integration for the transport/tag/memo seams; no E2E for a backend.
- `test-priorities-matrix.md` — AC-1 P0 (the journey is unusable without a working transport); AC-2/3/4 P1 (reuse, degradation, network isolation).
- `data-factories.md` — no random data; deterministic names/paths, pid-unique memo key, unasserted setup.
- `test-healing-patterns.md` — failure signatures mapped to seams (stale memo, argv drift, copy drift in the degradation message).
- `ci-burn-in.md` — the suite is fast/deterministic; burn-in not needed for these pure/byte-level tests.
- `component-tdd.md` — red→green→refactor loop applied at the seam level (no component runner in this stack).

---

## Notes & Known Gaps

1. **Retrofit, not true red.** The story is implemented; activating a scaffold passes today. Treat these as characterisation contracts. A genuinely failing AC-1/AC-4 scaffold needs a docker daemon, which this environment does not have.
2. **AC-2 seeds the memo through the crate-private `remember`.** It is not a tautology — deleting the memo branch makes `ensure_container_server` fall through to a probe and fail — but it duplicates the pre-existing unit test's seam rather than adding end-to-end evidence.
3. **AC-1/AC-4 stop at `docker_args`.** The production glue `RemoteLink::container` / `container_link_argv` is pinned indirectly (same argv shape, `--pane` asserted); a regression in that glue alone would not fail these leaves.
4. **AC-3 tests the pure relocation decision, not the shell probe.** The probe has existing real-`sh` coverage in `container.rs`; the degradation wording assertion is intentionally coupled to the user-visible message.
5. **No contract tests.** No Pact artifacts/dependencies and no contract-testing request; the Pact branches did not open.
6. **Language tooling caveat:** Rust identifiers cannot contain `-`, so each leaf carries `AC-<n>` in its doc comment and `#[ignore]` reason rather than in the test title. The aggregation check maps functions to criteria explicitly.

---

## Next Steps

1. This checklist is linked from the story's `## ATDD Artifacts` section.
2. Run the red-phase baseline command above to confirm the four scaffolds stay ignored.
3. For a future change to this seam, activate one scaffold at a time (remove its `#[ignore]`), confirm it fails first, then implement.
4. Keep the manual docker checks in the story's I/O matrix audit alongside these scaffolds.

---

**Generated by BMad TEA Agent** - 2026-10-22

---

## Validation (Step 5)

Validated against `checklist.md`, adapted for a backend/Rust stack (the Playwright/E2E rows are N/A):

- [x] Story loaded and four acceptance criteria extracted.
- [x] All criteria were unnamed; `AC-1`…`AC-4` assigned in source order and persisted with `id`/`idSource`/text.
- [x] Affected components and constraints documented.
- [x] Test framework (cargo test) and existing patterns inspected (`username_selector_lifecycle.rs`, in-crate container tests).
- [x] Required backend knowledge fragments loaded; frontend/Playwright/Pact branches correctly closed.
- [x] Test level chosen per criterion (integration for transport/tag/memo, unit for relocation); no duplicate coverage across levels.
- [x] Priorities assigned (AC-1 P0; AC-2/3/4 P1).
- [x] Exactly one red-phase leaf per criterion; each leaf's ignore reason names its id once.
- [x] Criterion-defining assertion is the first potentially failing assertion in every leaf; setup is unasserted/opaque.
- [x] No placeholder, self-comparison, focus, or unreachable assertions.
- [x] `cargo test -p tty7-core --lib atdd` → 4 ignored / 0 failed; `-- --ignored` → 4 passed / 0 failed.
- [x] Checklist links back into the story (`## ATDD Artifacts`).
- [x] No orphaned browser/CLI sessions (none launched); no temp artifacts outside the checklist.

### Polish notes

- Consolidated the four step outputs into one document; metadata (`generatedTestFiles`) reflects the actual scaffold path.
- Terminology is consistent: red-phase = `#[ignore]`; “activation” = removing that attribute.

### Completion summary

| Item | Value |
| ---- | ----- |
| Test file | `crates/tty7-core/src/daemon/install/container/atdd.rs` |
| Wiring | `crates/tty7-core/src/daemon/install/container.rs` (`#[cfg(test)] mod atdd;`) |
| Checklist | `_bmad-output/test-artifacts/atdd-checklist-1-container-transport-skeleton.md` |
| Story | `_bmad-output/specs/spec-container-remote-host/stories/1-container-transport-skeleton.md` (linked) |
| Criteria | AC-1 (P0), AC-2, AC-3, AC-4 (P1) |
| Baseline | 4 ignored, 0 failed |
| Activated (retrofit) | 4 passed, 0 failed |

**Key risks / assumptions**

- Story 1 is already implemented, so this is a retrofit: activation passes rather than fails.
- No docker daemon here: live container registration, reopen and `--network none` remain manual checks.
- AC-2 uses the crate-private memo setter; it proves the read/short-circuit path, not the install write path.
- Rust cannot put `AC-<n>` in a function name; ids live in doc comments/ignore reasons and this table.

**Next recommended workflow:** continue with the story's manual docker checks; for new work start `dev-story`/`bmad-build` with these scaffolds as the contract. `automate` comes after implementation (already here), and `test-review` can be run on the four leaves.
