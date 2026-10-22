---
title: "Container pane's agent avatar and cwd on the sidebar"
type: 'feature'
created: '2026-10-22'
status: 'done'
route: 'dispatch'
review_loop_iteration: 0
baseline_commit: 'de99fa05eccb124ecd0c217bffeec8ac787917be'
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Story 1 made a container a first-class remote host, so the in-container `tty7-server` is now the only thing that sees the agent process and the shell's directory. The sidebar surfaces were never taught that: a container row still gets its machine identity from `TerminalView::remote_context()`, which only SSH and WSL panes set, so the row's hover card cannot name the container; and nothing pins the promise that the avatar, status dot, and cwd really come from inside the container rather than from the host-side `docker exec` argv (CAP-3, CAP-5 in `_bmad-output/specs/spec-container-remote-host/SPEC.md`).

**Approach:** Read a pane's machine from the pane's **workspace target** when `remote_context()` is silent, add the missing `container <name>` line to the sidebar hover card, and pin the in-container agent/cwd promises at the seams that carry them (`Tab::agent_row`, `TerminalView::effective_cwd`/`spawnable_cwd`, `display_home`).

## Boundaries & Constraints

**Always:** the container line names the target exactly as the router does (`container <name>`, from `RouteTarget::Container`'s `describe()` spelling). Agent identity and status come from the far daemon's `Agent`/`AgentStatus` frames; cwd comes from the container shell's `Cwd` report. Keep local, SSH, WSL, and local-stdio rows byte-identical. `display_home` for a container resolves through `HostLinks`, i.e. the container's reported home, never this machine's `$HOME`.

**Never:** no host-side `/proc`, argv, or title heuristic for a container pane. No new pane or workspace model, no per-container branch in the agent/cwd polling path, no sidebar redesign, no container enumeration.

</frozen-after-approval>

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| AGENT_IN_CONTAINER | container pane; far daemon reports `Agent(Claude)` + `AgentStatus(Working)` | sidebar row shows Claude's avatar and the Working dot | N/A |
| HOST_ARGV_SILENT | host-side pane argv is `docker exec …` with no agent name | row still shows the in-container agent (no host fallback) | N/A |
| HOVER_IDENTITY | container pane with no `remote_context()` | hover card's host line reads `container web` | N/A |
| HOVER_LOCAL | local or SSH/WSL row | host line unchanged (`None` locally; the SSH/WSL target as today) | N/A |
| CWD_TRACKS | container shell reports `cd /workspace/app` | row/Info cwd is `/workspace/app`; with container home `/workspace` it reads `~/app` | N/A |
| NEW_TAB_IN_PLACE | active container pane at `/workspace/app` | new tab/split spawns through the container route with `/workspace/app` as start dir | falls back to container home only when the shell has not reported a cwd |

## Code Map

- `_bmad-output/specs/spec-container-remote-host/container-host-design.md` -- read the "Integration points" row for agent avatar + cwd before coding.
- `src/terminal/view.rs` -- `remote_context()` (:2134), `machine_label()` (:2152), `paths_are_local()` (:2168), `display_home()` (:2180), `spawnable_cwd()` (:2187), `set_workspace()` (:2203), `effective_cwd()` (:2431). Add the machine label the sidebar asks for; do not touch the polling path.
- `src/ui/app.rs` -- `Tab::agent_row()` (:781), `Tab::agent()` (:767), `agent_status()` (:813), `tab_ssh_dot()` (:8450). The row's agent pair already reads leaves only.
- `src/ui/tab_sidebar.rs` -- `SidebarInfo.host` (:216), `sidebar_info()` (:2128) derives the host from `remote_context()` only; add the container fallback here so the same leaf answers.
- `src/terminal/remote.rs` -- `PaneWorkspace` (:165) carries `target` (the container variant); `PaneRoute::for_workspace` (:252) decides remote vs local spawn; `DaemonMsg::Agent`/`AgentStatus`/`Cwd` land at (:1847)/(:1853)/(:1763).
- `src/ui/remote_connect.rs` -- `HostLinks::home()` (:512) supplies the container home for `~` shortening; `connect_blocking()` (:343) already registers it.
- `src/ui/tab_strip.rs` -- `abbreviate_home()` (:62) is the one `~` formatter the row uses.

## Tasks & Acceptance

**Execution:**
- [x] `src/terminal/view.rs` -- expose the pane's machine identity for a workspace pane (`container <name>` for `RemoteTarget::Container`; `None` when `remote_context()` already answers or the pane is local), read off `workspace()` -- one source for the card and every other surface.
- [x] `src/ui/tab_sidebar.rs` -- `sidebar_info()` asks that accessor when `remote_context()` is `None`, so a container row's hover card names its container; leave every other row's answer identical.
- [x] tests -- pin the I/O matrix: in-container `Agent`/`AgentStatus` frames drive `Tab::agent_row`; a container row's card names `container web`; a `Cwd` frame is the row's/Info's directory and shortens against the container home; `spawnable_cwd` + the container `PaneRoute` carry a new tab to `/workspace/app`.
- [x] docs/changelog -- record the container machine line on the sidebar card if the docs describe remote-row identity.

**Acceptance Criteria:**
- Given a container pane whose far daemon reports Claude `Working`, when the sidebar reads the row, then the row pairs Claude's avatar with the Working dot, and the host-side `docker exec` argv contributes nothing.
- Given a container pane, when its sidebar hover card is built, then the card names the machine as `container web`; a local row still has no host line and an SSH/WSL row still names its own target.
- Given the container shell reports `/workspace/app` and the container home is `/workspace`, when the row and Info panel draw the directory, then it reads `/workspace/app` and `~/app` respectively — never shortened against this machine's home.
- Given an active container pane at `/workspace/app`, when the user opens a new tab or split, then the new shell starts at `/workspace/app` through the container route.

## Implementation Notes

**Why the card, not the row.** The row already draws the agent avatar, the status dot, and the cwd from the in-container frames; the one piece of container identity it cannot draw is the machine, because the avatar only shows a dot for SSH. The hover card is where every other remote row spells its machine, so the container joins it there rather than growing a new row shape.

**Why the workspace target.** `remote_context()` describes a *shell that ssh'd onward*; a container pane's shell runs in the far daemon's own pty and reports no SSH/WSL context. The workspace target is the only thing that knows the pane is a container, and the same value already drives `host_id`, `display_home`, and the route.

**What this story is not.** No new agent detection: the container daemon already runs `foreground_agent` inside the container. No new cwd plumbing: the container shell already reports OSC 7. The tests pin those promises; if one fails, fix the seam it names rather than adding a second path.

**I/O Matrix audit** (tests that actually ran, `cargo test --workspace`, 0 failed):

| Row | Evidence |
|-----|----------|
| AGENT_IN_CONTAINER | `ac1_the_sidebar_pairs_the_in_container_agent_with_its_status` — the far daemon's `Agent(Claude)` + `AgentStatus(Working)` frames drive `Tab::agent_row` to `(Claude, Working)`. |
| HOST_ARGV_SILENT | The same test: nothing in the host process names an agent, and the only `Agent` frame comes from the daemon-side stream. `machine_label` reads the workspace target, never argv or a title. |
| HOVER_IDENTITY | `ac2_a_container_row_names_its_container_in_the_hover_card` — `SidebarInfo.host == Some("container web")`, spelled by `RouteHeader::container(..).describe()`. |
| HOVER_LOCAL | `ac2_a_local_row_has_no_host_line`, `ac2_an_ssh_workspace_row_has_no_host_line`, `ac2_a_shell_that_sshd_onward_keeps_its_own_target`, and `ac2_a_container_shell_that_sshd_onward_names_the_further_machine` — local and SSH workspace rows keep `None`; a shell's own `RemoteContext` still wins. |
| CWD_TRACKS | `ac3_container_cwd_is_what_the_shell_reported_and_shortens_against_the_container_home` — the container `Cwd` frame is `effective_cwd()`; `display_home` is `None` before the link and `Some("/workspace")` once `HostLinks` holds the container's home; `abbreviate_home("/workspace/app", Some("/workspace")) == "~/app"`. |
| NEW_TAB_IN_PLACE | `ac4_a_new_tab_in_a_container_workspace_starts_in_the_container_cwd` — `spawnable_cwd() == "/workspace/app"` and `PaneRoute::for_workspace` is remote (the container route). |

**Verification run.** `cargo fmt --all -- --check`, `cargo test -p tty7 --bin tty7-app atdd`
(9 passed, 0 ignored), `cargo test -p tty7 --bin tty7-app` (2687 passed, 0 failed),
and `USER=root LOGNAME=root cargo test --workspace` (4923 passed, 0 failed, 11 ignored).
Manual docker checks are outstanding: this environment has no docker CLI.

## Verification

**Commands (run in the worktree):**
- `cargo fmt --all -- --check` -- clean.
- `cargo test -p tty7 --bin tty7-app atdd` -- 9 passed, 0 failed, 0 ignored. The four criterion leaves plus five companion pins now run in the normal suite.
- `cargo test -p tty7 --bin tty7-app atdd -- --ignored` -- 0 selected; no scaffold is skipped any more.
- `cargo test -p tty7 --bin tty7-app` -- 2687 passed, 0 failed, 2 pre-existing ignored.
- `USER=root LOGNAME=root cargo test --workspace` -- 4923 passed, 0 failed, 11 ignored.

**Manual checks (if docker is available):**
- Add a running container, open a pane, run `claude`: avatar + Working dot in the sidebar, `container <name>` in the row's hover card.
- `cd /workspace/app`; confirm the row and Info cwd; open a split and confirm it starts there.

## ATDD Artifacts

- Checklist: `_bmad-output/test-artifacts/atdd-checklist-2-container-sidebar-agent-cwd.md`
- Acceptance tests (active, no `#[ignore]`): `src/ui/tab_sidebar/atdd.rs`
- Module wiring: `src/ui/tab_sidebar.rs` (`#[cfg(test)] mod atdd;`)
- Criteria: AC-1…AC-4, plus companion pins for local/SSH/WSL/local-stdio/profile/alias rows and the onward-hop precedence.
- Run: `cargo test -p tty7 --bin tty7-app atdd` (9 passed, 0 ignored)
- Red state: AC-2 was red until the hover-card machine line landed; AC-1/AC-3/AC-4 were green from the start and stand as regression pins. All are green now.

## Review Triage Log

Findings from the three review layers: **B** = blind hunter, **E** = edge-case
hunter, **V** = verification-gap reviewer. Verdicts are the triage decision;
`reject` means the finding did not survive (refuted, cosmetic, or only fixable
by rewriting this build's spec).

| # | Finding | Verdict | Route | Evidence |
|---|---------|---------|-------|----------|
| B1 | `machine_label` has one caller; `tab_ssh_dot`, `new_tab`, `panel_search` stay container-blind. | false | reject | Those sites render an SSH phase dot, a redial decision, or native-SSH search semantics — not a machine label. The hover card is the only machine-label surface and the accessor is its one source. |
| B2 | SSH workspace rows get no host line and the test locks the asymmetry in. | false | reject | Their card had no host line before this change, and the frozen boundary requires local/SSH/WSL/local-stdio rows to stay byte-identical. The test pins that boundary rather than a new gap. |
| B3 | AC-2 says an SSH/WSL row names its target, but the test asserts an SSH workspace row names nothing. | low | reject | The HOVER_LOCAL matrix says “the SSH/WSL target as today”, and the tests distinguish a shell that ssh’d (names it) from a workspace row (never had one). The only fix edits the spec, which triage rejects. |
| B4 | WSL, `LocalStdio`, `Profile`, and `Alias` targets are untested. | low | patch | Added `ac2_only_a_container_workspace_gets_a_machine_line`, a table test over all five non-container variants. |
| B5 | Two spellings reach the user: `container web` on the card, `container:web` from `RemoteTarget::Display`. | low | reject | The card spelling is the router’s (`describe()`), required by the frozen boundary. `Display` is story 1’s spelling and unifying it is a separate cosmetic change. |
| B6 | The changelog says the avatar “only dots” the host, and `SidebarInfo.host` says the same. | low | patch | Reworded the changelog bullet and the `SidebarInfo.host` comment to describe the machine line the card carries. |
| B7 | Docs say the card names the container even when the shell ssh’d onward. | low | patch | The docs paragraph now states that an onward `ssh` names the farther machine. |
| B8 | The new docs paragraph splits the install description from its undo step. | low | patch | Moved the paragraph after “To undo it…”. |
| B9 | The changelog says the label avoids the host-side `docker exec` argv, implying the old source was argv. | low | patch | Reworded to “read from the pane’s workspace and spelled the way the router already names it.” |
| B10 | AC-3 tests getters, not the sidebar row or Info panel. | low | reject | Both consumers are unchanged and use the tested pair (`effective_cwd` + `display_home` → `abbreviate_home`); the formatter has unit tests. A render test is beyond a direct correction and no wrong rendering was demonstrated. |
| B11 | `set_home_for_test` writes only `homes`, a state production `insert` never leaves. | low | reject | `HostLinks::home` reads only `homes`; the production `insert` path is unchanged and covered elsewhere. The helper is test-only and does not claim to exercise registration. |
| B12 | The “host argv contributes nothing” half of AC-1 is not observable in the fixture. | low | reject | The code path reads only daemon `Agent`/`AgentStatus` frames; absence of a host heuristic cannot be asserted by a fixture that carries no argv. No bad outcome is reachable. |
| B13 | AC-4 asserts two getters and never opens a tab or split. | low | reject | The test pins the two inputs the real paths consume (`spawnable_cwd`, remote `PaneRoute`); a live spawn needs a container link. No wrong behavior was demonstrated in `new_tab_with_shell`/`split_slot`. |
| B14 | `spawnable_cwd` is `None` for a container pane whose shell ssh’d onward. | false | reject | That shell is on a third machine; its directory must not be handed to the container route. `None` is the correct start directory. |
| B15 | Negative tests assert `info.is_none()` instead of `host.is_none()`. | low | patch | Both now assert `info.and_then(\|i\| i.host).is_none()`. |
| B16 | `attach_container` mints a fresh workspace and registers neither registry; fixtures repeat `SidebarRowShown`. | low | reject | Test-fixture style only; the fresh workspace is intentional isolation and no behavior depends on registration here. |
| B17 | `poll_until` busy-waits and returns `last` on timeout. | low | reject | The tests finish in 0.08 s; a timeout still fails with the got/want values. Improving the harness is not needed for this story. |
| B18 | Story 1’s four scaffolds keep `#[ignore]` and CI never runs them. | low | defer | Pre-existing story-1 artifact; appended to `_bmad-output/implementation-artifacts/deferred-work.md`. |
| B19 | Story 1’s acceptance tests restate implementation details. | low | defer | Pre-existing story-1 test quality; appended to the same ledger. |
| B20 | Story 1’s checklist has a stray “Generated by” line and `[!]`. | low | reject | Pre-existing cosmetic formatting in a historical artifact; no user or developer action depends on it. |
| B21 | Story 1’s checklist calls the scaffolds RED while they pass when activated. | low | reject | Historical red-phase terminology; the activation state is recorded separately. |
| B22 | Checklist 2 says “four leaves, all ignored” and “not implemented” as current state. | low | patch | Marked those as generation-time state and added an “Activation (after implementation)” section with the green command results. |
| B23 | Story 2’s Verification/ATDD sections still say the scaffolds stay skipped. | low | patch | Updated both sections to 8 active leaves, 0 ignored, and the current verification results. |
| B24 | `stories.yaml` has no `done_checkpoint` for story 2. | false | reject | `done_checkpoint` was present in the initial stories.yaml commit as an orchestration flag; no build step writes it, and the story status plus commit record delivery. |
| B25 | Code Map line numbers are pre-change. | low | patch | Refreshed `machine_label` 2152, `paths_are_local` 2168, `display_home` 2180, `spawnable_cwd` 2187, `set_workspace` 2203, `effective_cwd` 2431. |
| B26 | `generatedTestFiles` omits the `mod atdd;` wiring, so cleanup would orphan it. | low | patch | Added the wiring file to both checklists’ `generatedTestFiles`. |
| B27 | Story 1 prefixes commands with `USER=root LOGNAME=root` without explanation; story 2 omits it. | false | reject | Story 2’s workspace command already includes `USER=root LOGNAME=root` (Verification line 82). The story-1 prefix is pre-existing. |
| B28 | The matrix omits container-name edge cases (empty, spaces/colons, long id). | false | reject | Names reach the UI only through the Add Container prompt, which rejects empty input; docker names cannot contain spaces or colons, and long names only truncate visually. |
| E1 | AC-2 says an SSH/WSL row names its target, but `machine_label` skips `Direct`. | false | reject | Same as B2: workspace rows never had a host line, and the frozen boundary requires them unchanged. |
| E2 | AC-3 says the row shows `/workspace/app` and the Info panel `~/app`, but both abbreviate. | low | reject | The matrix and implementation agree that both surfaces shorten with the container home; the AC’s “respectively” is imprecise. The only fix edits the spec. |
| V1 | Story 2’s verification sections and checklist contradict the active eight-leaf file. | low | patch | Same correction as B22/B23. |
| V2 | Story 1’s scaffolds are permanently skipped. | low | defer | Same as B18. |
| V3 | AC-3 never builds the row or Info panel. | low | reject | Same as B10. |
| V4 | AC-4 never opens a tab or split. | low | reject | Same as B13. |
| V5 | AC-1’s “host argv contributes nothing” is not observable. | low | reject | Same as B12. |

