---
stepsCompleted: ['step-01-preflight-and-context', 'step-02-generation-mode', 'step-03-test-strategy', 'step-04c-aggregate', 'step-05-validate-and-complete']
lastStep: 'step-05-validate-and-complete'
lastSaved: '2026-10-22'
workflowType: 'testarch-atdd'
storyId: '2'
storyKey: '2-container-sidebar-agent-cwd'
storyFile: '_bmad-output/specs/spec-container-remote-host/stories/2-container-sidebar-agent-cwd.md'
atddChecklistPath: '_bmad-output/test-artifacts/atdd-checklist-2-container-sidebar-agent-cwd.md'
generatedTestFiles:
  - 'src/ui/tab_sidebar/atdd.rs'
  - 'src/ui/tab_sidebar.rs' # #[cfg(test)] mod atdd; wiring; remove both together
inputDocuments:
  - '_bmad-output/specs/spec-container-remote-host/stories/2-container-sidebar-agent-cwd.md'
  - '_bmad-output/specs/spec-container-remote-host/SPEC.md'
  - '_bmad-output/specs/spec-container-remote-host/container-host-design.md'
  - '_bmad-output/specs/spec-container-remote-host/stories.yaml'
  - 'Cargo.toml'
  - '_bmad-output/specs/spec-container-remote-host/stories/1-container-transport-skeleton.md'
  - '.agents/skills/bmad-testarch-atdd/resources/knowledge/data-factories.md'
  - '.agents/skills/bmad-testarch-atdd/resources/knowledge/component-tdd.md'
  - '.agents/skills/bmad-testarch-atdd/resources/knowledge/test-quality.md'
  - '.agents/skills/bmad-testarch-atdd/resources/knowledge/test-healing-patterns.md'
  - '.agents/skills/bmad-testarch-atdd/resources/knowledge/test-levels-framework.md'
  - '.agents/skills/bmad-testarch-atdd/resources/knowledge/test-priorities-matrix.md'
  - '.agents/skills/bmad-testarch-atdd/resources/knowledge/ci-burn-in.md'
acceptanceCriteria:
  - id: 'AC-1'
    idSource: generated
    text: 'Given a container pane whose far daemon reports Claude Working, when the sidebar reads the row, then the row pairs Claude''s avatar with the Working dot, and the host-side docker exec argv contributes nothing.'
  - id: 'AC-2'
    idSource: generated
    text: 'Given a container pane, when its sidebar hover card is built, then the card names the machine as container web; a local row still has no host line and an SSH/WSL row still names its own target.'
  - id: 'AC-3'
    idSource: generated
    text: 'Given the container shell reports /workspace/app and the container home is /workspace, when the row and Info panel draw the directory, then it reads /workspace/app and ~/app respectively — never shortened against this machine''s home.'
  - id: 'AC-4'
    idSource: generated
    text: 'Given an active container pane at /workspace/app, when the user opens a new tab or split, then the new shell starts at /workspace/app through the container route.'
---

# ATDD Checklist - Epic 0, Story 2: Container pane's agent avatar and cwd on the sidebar

**Date:** 2026-10-22
**Author:** hua
**Primary Test Level:** Component/UI integration (Rust, `gpui::test`) + unit

---

## Story Summary

Make a container pane's sidebar row show the agent and status the in-container
daemon reports, name the container in the row's hover card, and pin that the
cwd is the container shell's own directory — shortened against the container's
home and inherited by new tabs and splits.

**As a** tty7 user working inside a container
**I want** the sidebar to show the in-container agent and directory
**So that** a container pane behaves like a local pane's sidebar row

---

## Acceptance Criteria

| ID   | ID source | Acceptance criterion |
| ---- | --------- | -------------------- |
| AC-1 | generated | Given a container pane whose far daemon reports Claude `Working`, when the sidebar reads the row, then the row pairs Claude's avatar with the Working dot, and the host-side `docker exec` argv contributes nothing. |
| AC-2 | generated | Given a container pane, when its sidebar hover card is built, then the card names the machine as `container web`; a local row still has no host line and an SSH/WSL row still names its own target. |
| AC-3 | generated | Given the container shell reports `/workspace/app` and the container home is `/workspace`, when the row and Info panel draw the directory, then it reads `/workspace/app` and `~/app` respectively — never shortened against this machine's home. |
| AC-4 | generated | Given an active container pane at `/workspace/app`, when the user opens a new tab or split, then the new shell starts at `/workspace/app` through the container route. |

---

## Preflight & Context

### Stack detection

`config.test_stack_type` is `auto`; the story's surface is the Rust GUI crate
`tty7` (`Cargo.toml`, bin `tty7-app`, `gpui::test`), no Playwright/Cypress and
no mobile manifest in the code path. **Detected stack: `backend`** (component
tests run in-process under `cargo test`; there is no browser/E2E runner).

### Prerequisites

- [x] Story with clear acceptance criteria — authored first at
  `_bmad-output/specs/spec-container-remote-host/stories/2-container-sidebar-agent-cwd.md`
  (no supplied AC ids; ids generated in source order).
- [x] Test framework configured — `#[gpui::test]` + `cargo test -p tty7 --bin tty7-app`.
- [x] Development environment — Rust workspace in `.worktrees/container-remote-host`.

### Criterion registry notes

- All four criteria were unnamed; `AC-1`…`AC-4` were generated deterministically
  in source order and are stable for identical story content.
- Story 2 was **not implemented at ATDD generation time**; unlike story 1 this
  was a true pre-implementation ATDD run. It is implemented now — see
  “Activation (after implementation)” below.

### TEA config flags read

| Flag | Value | Effect here |
| ---- | ----- | ----------- |
| `tea_use_playwright_utils` | `true` | N/A: no `@seontechnologies/playwright-utils`, no Playwright runner. |
| `tea_use_pactjs_utils` | `true` | N/A: no Pact artifacts, contract testing not requested. |
| `tea_pact_mcp` | `mcp` | No contract branch opened, so not probed. |
| `tea_browser_automation` | `auto` | N/A: no browser surface. |
| `test_stack_type` | `auto` | Resolved to `backend`. |
| `tea_execution_mode` | `auto` | Resolved to **sequential**: backend has one worker (no E2E/API HTTP worker). |

### Knowledge fragments loaded

Core: `data-factories.md`, `component-tdd.md`, `test-quality.md`,
`test-healing-patterns.md`.
Backend: `test-levels-framework.md`, `test-priorities-matrix.md`,
`ci-burn-in.md`.
Not selected (conditions false): frontend `selector-resilience.md` /
`timing-debugging.md`; all Playwright-utils, Playwright CLI, Pact.js and
Pact MCP fragments.

### Affected components

`src/terminal/view.rs` (machine identity, `display_home`, `spawnable_cwd`),
`src/ui/tab_sidebar.rs` (`SidebarInfo.host`, `sidebar_info`),
`src/ui/app.rs` (`Tab::agent_row`), `src/terminal/remote.rs`
(`PaneWorkspace`, `PaneRoute`, `DaemonMsg::{Agent,AgentStatus,Cwd}`),
`src/ui/remote_connect.rs` (`HostLinks::home`), `src/ui/tab_strip.rs`
(`abbreviate_home`).

---

## Generation Mode (Step 2)

**Chosen: AI generation.** `{detected_stack}` is `backend`, so recording mode is
skipped by the workflow. Scenarios come from the story's I/O matrix, the design
companion, and the implemented seams (`harness_with_tabs`, `set_workspace`,
`DaemonMsg` frames, `Tab::agent_row`, `sidebar_info`, `PaneRoute`).

---

## Test Strategy (Step 3)

### Level selection

Backend rules apply: **unit** for pure formatters and route decisions,
**component/integration** for the sidebar row built on the gpui test harness.
**No E2E** (no browser surface).

### Coverage plan

| AC | Priority | Level | Primary red-phase scaffold (one per criterion) | Criterion-defining assertion (first that can fail) | Secondary / green-phase branches |
| -- | -------- | ----- | ---------------------------------------------- | -------------------------------------------------- | -------------------------------- |
| AC-1 | P0 | Component | `ac1_the_sidebar_pairs_the_in_container_agent_with_its_status` | `Tab::agent_row` returns `(Claude, Working)` after the far daemon's `Agent`/`AgentStatus` frames — with no agent named anywhere on the host side. | Avatar/dot rendering already reads that pair; add a screenshot/manual check with a live container agent. |
| AC-2 | P0 | Component | `ac2_a_container_row_names_its_container_in_the_hover_card` | `sidebar_info` returns `Some`, and `info.host == Some("container web")`. | Local row keeps `host == None`; SSH/WSL row keeps its existing target; a split tab reads the focused leaf's machine. |
| AC-3 | P1 | Component + unit | `ac3_container_cwd_is_what_the_shell_reported_and_shortens_against_the_container_home` | `effective_cwd()` equals `/workspace/app` after the container shell's `Cwd` frame. | `abbreviate_home("/workspace/app", Some("/workspace")) == "~/app"`; `HostLinks::home` supplies the container home once the link is up; Info panel uses the same pair. |
| AC-4 | P1 | Unit/structural | `ac4_a_new_tab_in_a_container_workspace_starts_in_the_container_cwd` | `spawnable_cwd()` equals `/workspace/app` for a container pane. | `PaneRoute::for_workspace` is `Remote` (not `Local`), so `new_tab_with_shell` → `new_tab_slot` carries the path to the container spawn; split path uses the same cwd. |

### Red-phase confirmation

- Each declared criterion has exactly one ignored leaf; one criterion, one id.
- **AC-2 is genuinely red today**: `sidebar_info` derives the host only from
  `remote_context()`, which a container pane never sets, and returns `None` for
  an otherwise-empty card. Verified: `cargo test … atdd -- --ignored` fails on
  `the card has a line to draw`.
- AC-1, AC-3, AC-4 are **characterisation pins** — story 1's transport already
  makes the in-container daemon the source of agent and cwd reports, so they
  pass before this story. They are the regression contract for the story's
  promise, not fake red leaves; a second fake would test a shim, not the system.

---

## Red-Phase Test Scaffolds Created (Step 4)

**Execution mode:** sequential (backend stack; one worker; no E2E).
**Red-phase marker:** `#[ignore = "ATDD RED: …"]` (Rust's equivalent of `test.skip()`).

**File:** `src/ui/tab_sidebar/atdd.rs`
**Module declaration:** `src/ui/tab_sidebar.rs` → `#[cfg(test)] mod atdd;` (test-only).

| # | Test | Criterion | Priority | Status | Verifies |
| - | ---- | --------- | -------- | ------ | -------- |
| 1 | `ac1_the_sidebar_pairs_the_in_container_agent_with_its_status` | AC-1 | P0 | RED — `#[ignore]` | `Tab::agent_row` = `(Claude, Working)` from far-daemon frames only. |
| 2 | `ac2_a_container_row_names_its_container_in_the_hover_card` | AC-2 | P0 | **RED — fails until implemented** | The hover card's `host` line is `container web`. |
| 3 | `ac3_container_cwd_is_what_the_shell_reported_and_shortens_against_the_container_home` | AC-3 | P1 | RED — `#[ignore]` (passes today) | `effective_cwd` follows the container `Cwd` frame; `~` shortens against the container home. |
| 4 | `ac4_a_new_tab_in_a_container_workspace_starts_in_the_container_cwd` | AC-4 | P1 | RED — `#[ignore]` (passes today) | `spawnable_cwd` + a `Remote` container route carry the in-container directory. |

### Data factories / fixtures

`harness_with_tabs(n)` gives tabs whose panes are `quiet_test_pane`s plus the
daemon-side `Stream`; `attach_container` sets a `PaneWorkspace` with
`RemoteTarget::Container` on the pane. Daemon reports are written as
`DaemonMsg::{Agent,AgentStatus,Cwd}` frames and polled through the pane's
reader, exactly as the production link delivers them. No docker shim.

---

## Mock Requirements

### docker CLI / daemon

Not mocked. The story's contract is about what the client does with daemon
frames and the workspace target, and those are delivered in-process. A live
container remains a manual check (register, run `claude`, `cd`, split).

### Host filesystem

The container's reported home is planted through the test-only
`HostLinks::set_home_for_test` (no `RemoteHost` is stood up), so AC-3 can resolve
`display_home` without touching this machine's `$HOME` or the real registry.

---

## Required data-testid Attributes

N/A — Rust/gpui component tests address widgets by entity and code path, not
DOM test ids.

---

## Implementation Checklist

### Test 1 — AC-1 (`P0`) `ac1_the_sidebar_pairs_the_in_container_agent_with_its_status`

**File:** `src/ui/tab_sidebar/atdd.rs`

- [ ] Remove the `#[ignore]` for AC-1.
- [ ] Confirm the row's `agent_row` still reads every leaf and pairs name with status from that leaf.
- [ ] Run: `cargo test -p tty7 --bin tty7-app atdd::ac1`
- [ ] ✅ Green: avatar + Working dot come from the far daemon.
- [ ] Manual (needs docker): start an agent in a container pane and confirm the avatar/dot.

### Test 2 — AC-2 (`P0`) `ac2_a_container_row_names_its_container_in_the_hover_card`

**File:** `src/ui/tab_sidebar/atdd.rs`

- [ ] Add the container machine accessor on `TerminalView` (workspace target → `container <name>`).
- [ ] Have `sidebar_info` fall back to it when `remote_context()` is `None`.
- [ ] Remove the `#[ignore]` for AC-2.
- [ ] Run: `cargo test -p tty7 --bin tty7-app atdd::ac2`
- [ ] ✅ Green: `info.host == Some("container web")`.
- [ ] Re-run the local/SSH row tests to confirm their host line is unchanged.

### Test 3 — AC-3 (`P1`) `ac3_container_cwd_is_what_the_shell_reported_and_shortens_against_the_container_home`

- [ ] Remove the `#[ignore]` for AC-3.
- [ ] Confirm the `Cwd` frame is the pane's `effective_cwd` and `display_home` resolves via `HostLinks`.
- [ ] Run: `cargo test -p tty7 --bin tty7-app atdd::ac3`
- [ ] ✅ Green: `/workspace/app` on the row/Info, `~/app` when shortened.

### Test 4 — AC-4 (`P1`) `ac4_a_new_tab_in_a_container_workspace_starts_in_the_container_cwd`

- [ ] Remove the `#[ignore]` for AC-4.
- [ ] Confirm `new_tab_with_shell` → `spawnable_cwd` → `new_tab_slot` keeps the path, and the route is the container's.
- [ ] Run: `cargo test -p tty7 --bin tty7-app atdd::ac4`
- [ ] ✅ Green: a new tab opens at `/workspace/app` inside `web`.

---

## Validation & Completion (Step 5)

- [x] Prerequisites satisfied (story authored with Given/When/Then ACs; `cargo` test surface present).
- [x] Test file compiles; four leaves listed, all ignored at generation time (`-- --list` verified).
- [x] Checklist maps one criterion to one scaffold id.
- [x] Criterion ids generated deterministically, collision-free, recorded in frontmatter.
- [x] Red-phase marker is Rust's `#[ignore]`, with the criterion id in every reason.
- [x] AC-2 red verified by running the ignored set; AC-1/3/4 green with the same command (characterisation pins, disclosed).
- [x] Secondary branches recorded as green-phase checklist work, not extra ignored leaves.
- [x] No browsers/CLI sessions started; no temp artifacts outside `_bmad-output/test-artifacts/`.
- [x] Story file carries the ATDD artifact paths and run command.

**Completion summary**

- Test file: `src/ui/tab_sidebar/atdd.rs` (wired by `#[cfg(test)] mod atdd;`).
- Checklist: `_bmad-output/test-artifacts/atdd-checklist-2-container-sidebar-agent-cwd.md`.
- Handoff: `_bmad-output/specs/spec-container-remote-host/stories/2-container-sidebar-agent-cwd.md`.
- Key risk: the only new production behavior is the hover-card machine line; the
  rest is a regression contract, so the build must not invent a second agent/cwd
  path.
- Next: `bmad-build` for story 2 (dev-story), then `bmad-testarch-automate` after
  implementation.

## Activation (after implementation)

The red-phase record above is the ATDD generation state. After implementation,
all eight leaves in `src/ui/tab_sidebar/atdd.rs` are unignored and green:

- `cargo test -p tty7 --bin tty7-app atdd` — 9 passed, 0 failed, 0 ignored.
- `cargo test -p tty7 --bin tty7-app atdd -- --ignored` — 0 selected; no
  `#[ignore]` remains.
- `cargo test -p tty7 --bin tty7-app` — 2687 passed, 0 failed, 2 pre-existing
  ignored.
- `USER=root LOGNAME=root cargo test --workspace` — 4923 passed, 0 failed,
  11 ignored.
