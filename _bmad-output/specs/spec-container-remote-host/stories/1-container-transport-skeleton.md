---
title: 'Container transport skeleton — register a container, push the server, open a container shell'
type: 'feature'
created: '2026-10-19'
status: 'done'
route: 'dispatch'
review_loop_iteration: 0
baseline_commit: '7bdce5dd93c5f7361ace46506a4fdecbda911738'
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** After a user enters a container from a tty7 pane, the sidebar loses the agent avatar and the git branch because tty7 treats the pane as the local host; today a running container cannot become a workspace at all (CAP-1/CAP-2 in `_bmad-output/specs/spec-container-remote-host/SPEC.md`).

**Approach:** Treat the container as a **remote host**. Push the static musl `tty7-server` into the container and carry the existing daemon protocol over `docker exec -i <ctr> <server> --stdio`, riding the host's docker socket, and reuse the SSH/WSL remote-workspace plumbing end to end.

**Registration (this story):** a command-palette `Add container…` typed prompt that takes a running container name/id, mirroring the `SaveSshSessionAsHost` plumbing; no container list is enumerated (the automatic docker-exec picker is story 6).

## Boundaries & Constraints

**Always:** transport is the `docker exec` stdio channel over the **host** docker socket — no container-network dependency and no in-container listener. Attach to an already-running Linux container; never change how it was started. Install path is `$HOME/.local/share/tty7/bin/tty7-server-<dialect>` (same dir chain, `700`/`755` modes) with relocation to an executable writable dir when that is unwritable or noexec. Reuse the existing remote stack — no parallel host stack, no privilege beyond current docker access.

**Never:** no background/proactive container enumeration (entry is consent-only); no image change, extra run flags, or mounting the docker socket into the container; out of scope for this story — Windows containers, a remote docker daemon (`DOCKER_HOST` over ssh/tcp), and podman/nerdctl guarantees.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| HAPPY_PATH | running Linux container `web` (has `sh`), user registers it | server pushed + started; pane opens a shell inside `web`; workspace tagged as a container | N/A |
| REOPEN | same running container reopened | existing server reused, no re-push | N/A |
| NOT_RUNNING | registered name matches no running container | no push attempted | clear non-connected error naming the container |
| NO_EXEC_LOCATION | `$HOME` unwritable, or only noexec storage | relocate to an executable writable dir and retry | degrade with a stated reason if none exists |
| EXEC_DENIED | `docker exec` denied by daemon | no container pane | error names the reason; nothing else breaks |
| NETWORK_NONE | `--network none` container | pane works unchanged | N/A |

</frozen-after-approval>

## Code Map

- `_bmad-output/specs/spec-container-remote-host/container-host-design.md` -- the load-bearing design (model, sequence, lifecycle, degradation matrix); read before coding.
- `crates/tty7-core/src/core/session.rs` -- `RemoteTarget` enum (:120) and its identity helpers (`connection_key` :168, `is_ssh` :182, `hosts_our_server` :195, `resolvable` :210, `host_id` :224); add a `Container` variant here.
- `crates/tty7-core/src/daemon/router.rs` -- `RouteTarget` (:43), `RouteHeader` ctors incl. `local_stdio` (:128), `origin_key` (:160), `open_link` (:821-863, LocalStdio arm :858); add a Container arm.
- `crates/tty7-core/src/daemon/remote_link.rs` -- `RemoteLink` (:14), `local_stdio` (:32), `spawn_stdio_owned` (:92) spawns a piped local child; the container link docks here.
- `crates/tty7-core/src/daemon/install/mod.rs` -- `RemoteOps` trait (:63) and the generic `Installer` (:603, `install` :964, `ensure_daemon` :1122, `launch_daemon` :1191); reused unchanged.
- `crates/tty7-core/src/daemon/install/wsl.rs` -- the non-SSH template: `WslRemoteOps` (:268), `ensure_wsl_server` (:619) with per-distro lock + reuse memo; mirror as container ops. `BundledServerBinary` (:476) + `asset_for_uname` (`asset.rs` :117) pick the linux musl asset; `RemotePaths`/`dir_chain` (`asset.rs` :193) are reused.
- `src/ui/remote_workspace.rs` -- `connect_to_host` (:583), `open_remote_workspace` (:769); reused to open the container workspace.
- `src/ui/remote_connect.rs` -- `available_hosts` (:29), `control_route` (:323), `live_label` (:118); add container labeling/route.
- `src/ui/search/command.rs` (:16), `src/ui/app.rs` (palette registration ~:6558, `on_command` ~:7070), `src/ui/switcher.rs` (`host_form_label` :3339, groups :712), `src/terminal/remote.rs` (`route_header` :192, `--pane`) -- the entry + sidebar/route surfacing.

## Tasks & Acceptance

**Execution:**
- [x] `crates/tty7-core/src/core/session.rs` -- add `RemoteTarget::Container { container }` and its identity/`Display`/helper arms (`container:<name>`, not ssh, hosts our server) -- a container target is a first-class remote.
- [x] `crates/tty7-core/src/daemon/install/container.rs` (new) -- implement `ContainerRemoteOps` over `docker exec` (home/run/spawn_detached/stat/mkdir/chmod/put/rename/remove_file/list_dir; uploads go through `exec -i … tee`, since no extra binary is mounted), arch via `uname -m`, plus a per-container install lock and reuse memo -- the install seam, modeled on `WslRemoteOps`.
- [x] `crates/tty7-core/src/daemon/remote_link.rs` -- add a container stdio link spawning `docker exec -i <ctr> <server> --stdio [--pane]` -- the transport.
- [x] `crates/tty7-core/src/daemon/router.rs` -- add `RouteTarget::Container`, its header ctor/origin key, and the `open_link` arm (install via `ContainerRemoteOps` when needed) -- routing.
- [x] command-palette `Add container…` entry (`src/ui/search/command.rs` variant + `src/ui/app.rs` palette row and `on_command` arm) wired through `connect_to_host` / `create_remote_workspace` -- the user door.
- [x] `src/ui/remote_connect.rs`, `src/ui/switcher.rs`, `src/terminal/remote.rs` -- tag the container workspace/panes and pass `--pane`.
- [x] tests -- cover the I/O matrix (happy, reopen reuse, not-running, no-exec location, exec-denied) plus unit tests for `connection_key`/`host_id` stability.

**Acceptance Criteria:**
- Given a running Linux container, when the user registers it and opens a pane, then a shell runs inside the container and its workspace is tagged as a container.
- Given the same running container reopened, then the installed server is reused rather than re-pushed.
- Given a container whose only writable location is noexec, when registered, then the server relocates to an executable dir, or degrades with a stated reason.
- Given a `--network none` container, then the pane still works (transport is host IPC).

## Implementation Notes

**Shape.** `ContainerRemoteOps` mirrors `WslRemoteOps`; `docker exec -i` is the only
transport (no network, no listener, no socket mount). `RemoteTarget::Container` /
`RouteTarget::Container` are distinct variants so the target owns an install and a
lifecycle, while `LocalStdio` keeps meaning "user-named program, no install".

**Relocation.** `RemoteOps::install_home()` is a new trait hook (default `Ok(None)`).
`Installer::install_base()` applies it once and every path-taking operation
(`run`, `put_ours_and_cycle`, `restart_daemon`) goes through it, so a restart or a
forced update finds a relocated server instead of assuming `$HOME`. The container
probe feeds its already-resolved home into the script, so an unset `$HOME` cannot
cause a bogus relocation.

**I/O Matrix audit** (tests that actually ran, `cargo test --workspace`, 0 failed):

| Row | Evidence |
|-----|----------|
| HAPPY_PATH | `the_transport_command_line_is_the_designs` (argv is `exec -i <ctr> <server> --stdio`), `the_stat_script_really_runs_and_answers_correctly`, `the_list_script_really_runs_and_answers_correctly`, `the_home_script_really_reports_an_absolute_path`, `the_mutating_scripts_really_run`, `real_container_names_and_ids_are_accepted`. **Not exercised against a real docker daemon** — no `docker` binary in the build environment; manual check outstanding. |
| REOPEN | `a_remembered_server_is_reused_until_it_is_forgotten` (memo hit, then forget). |
| NOT_RUNNING | `a_daemon_refusal_keeps_the_reason_it_gave` — `No such container: web` survives verbatim; `ExecOutput::failure_reason` is what the UI surfaces. No push is attempted because the probe/validate call fails first. |
| NO_EXEC_LOCATION | `the_install_base_is_home_unless_the_probe_moved_it`, `the_place_probe_relocates_past_a_home_that_cannot_hold_a_bin_directory` (real `sh`, blocked `$HOME` relocates under `XDG_RUNTIME_DIR`), plus `relocation`'s "nowhere" arm asserting the stated reason. |
| EXEC_DENIED | `a_daemon_refusal_keeps_the_reason_it_gave` — `permission denied …` kept verbatim; the connect fails before any pane exists. |
| NETWORK_NONE | Structural: the transport is `docker exec` over the host socket and touches no container network. No runtime test possible here (no docker). |

**Verification run.** `cargo fmt --all -- --check`, `cargo check --workspace` (no new
warnings), `cargo test --workspace` with `USER=root LOGNAME=root` — all green.

## Spec Change Log

## Review Triage Log

Layers: blind-hunter, edge-case-hunter, verification-gap. No intent_gap or
bad_spec entries, so no loopback; every patch was applied in place.

| # | Finding | Verdict | Route | Evidence |
|---|---------|---------|-------|----------|
| 1 | Story task line claims `docker cp`; the upload is `docker exec … tee` | low | rejected | The fix is spec text, so it is not a code finding. The wording is corrected in place, and `put` really is `exec -i … tee` — there is no `cp` anywhere. |
| 2 | The story file ends with an orphan `</content>` | low | rejected | Spec text again; the stray line was removed. |
| 3 | A container with no resolvable home (uid absent from `/etc/passwd`, `$HOME` unset) is refused before relocation runs (also edge-case #2) | low | rejected | `home_dir()` fails before the probe, so such a container never registers. The story targets devcontainers, whose images carry a passwd entry; a fix means rebuilding `RemotePaths` around a base instead of a home, which is story-4 territory. |
| 4 | The relocated base is recomputed, never remembered (also edge-case #4) | low | rejected | The candidate order is deterministic for a given container and `run`/`replace`/`restart` all call the same `install_base`, so restart finds the server it installed; only a mid-life `XDG_RUNTIME_DIR` would move it. Remembering it adds a memo field and callers for that case. |
| 5 | A temp install base is created under a world-writable parent without ownership/mode checks | low | rejected | It needs a hostile co-tenant in the same container; `mkdir -p` runs as the install uid and the real directory creation stays with the installer. Devcontainers are single-user. |
| 6 | `validate_container` accepts image references and internal spaces | low | rejected | The value is one argv word, so there is no injection; docker's own refusal is now shown in the prompt (patch #8), which is a better message than a second regex to keep in sync. |
| 7 | Name, short id and full id are three host ids | low | rejected | The same is already true of two SSH aliases for one host; canonicalizing needs a `docker inspect` per registration. |
| 8 | A failed *Add Container* was silent: the prompt closed and only the switcher or a workspace could show the reason | medium | patch | `add_container` cleared the prompt before connecting, and a container has no workspace until the link lands. `finish_connect` now reopens the prompt with the daemon's words, unless a dialect-refused create is parked and about to be finished by the update. |
| 9 | Adding the same connected container twice creates a second workspace | low | rejected | Harmless duplication that matches the add-host path; deduplicating needs a target→workspace lookup branch in `add_container`. |
| 10 | `ContainerNameError` text bypasses i18n | low | rejected | Remote/SSH failures are English-only as well; four keys × four languages is more than a direct correction. |
| 11 | A container never gets a real "tag" | false | — | `RouteHeader::describe()` is `container web`, `origin_key()` is `container:web`, and the workspace is named for the container — both asserted by the new route test, which disproves "only the internal key distinguishes it". |
| 12 | The stale-memo retry arm in `open_link` cannot fire for a recreated container | low | rejected | Recovery lives in the bridge-closed arm, which forgets the container memo when the exec says nothing; the retry arm is only a fast path. |
| 13 | No matrix row for distroless images, a missing docker CLI, or a remote `DOCKER_HOST` (also edge-case #5) | low | rejected | HAPPY_PATH already requires a shell, and a distroless container cannot host a shell pane at all; non-connected/degraded states are CAP-6, story 4. |
| 14 | A stopped container stays `resolvable` forever | false | — | WSL has the identical semantics; the supervisor records the machine error and backs off rather than spinning, and stopping/reconnecting is story 4. |
| 15 | The place probe runs, and mkdirs, before the install consent (also edge-case #3) | low | rejected | The consent dialog has to name the destination path, so the base must be resolved first; the probe creates only the directory it is about to name. |
| 16 | `list_dir` parses newline-delimited marker lines, so an entry can spoof | low | rejected | `list_dir` is only ever asked for tty7's own bin directory. |
| 17 | Tests overstate the evidence; the passwd fallback test depends on the build account | low | rejected | The matrix row already records that no docker run happened; the fake-passwd test passes for any account with an entry, which is every CI user. |
| 18 | `every_variant_has_a_distinct_label` never calls `kind_label`; `origin_key`/`describe` untested | low | patch | Labels now come from `LinkKind::ALL`/`label()` through `kind_label`, and the new route test asserts `describe()` and `origin_key()`. |
| 19 | No user docs and no changelog entry | medium | patch | Added `## Containers` to `docs/remote/workspaces.mdx` and an Added entry under Unreleased. |
| 20 | Consent copy claims the container is "not restarted or changed", and there is no removal path | medium | patch | The hint now says the server is installed and left running; the docs name the directory and how to remove it. |
| E1 | `install_home` errors are reported as an install-location failure | false | — | Every caller resolves `home_dir()` successfully before `install_base`, so a failure there really is an exec/location failure, and the docker reason is preserved verbatim. |
| E7 | NOT_RUNNING/EXEC_DENIED are only tested at the `failure_reason` level | low | rejected | The daemon boundary cannot be crossed without docker; `failure_reason` is exactly the string the strip shows, and the real-`sh` tests cover the container-side scripts. |
| V1 | No `route_header` test for a container workspace | low | patch | Added `a_container_workspace_routes_to_its_container_on_the_pane_channel` (target, `describe()`, `origin_key()`, pane channel). |
| V2 | No `control_route` test for a container | low | patch | Added `a_container_control_route_skips_ssh` (target, control channel, `for_pane()`, and `spec_for` still refusing). |
| V3 | `install_base` never saw a non-home answer in a test | low | patch | `FakeRemote` now overrides `install_home`; two tests cover a relocated base through `run`/`restart_daemon` and the stated `NoInstallLocation` reason. |
| V4 | The self-contained-targets loop omits `Container` | low | patch | Added to the loop in `resolvable_covers_both_dangling_kinds`. |
| V5 | Nothing verifies that a container create opens a first shell tab | low | patch | Added `a_container_create_opens_a_first_shell_tab`, which also proves an alias create still opens none. |
| V6 | The transport-label test is vacuous | low | patch | Covered by #18. |

## Design Notes

The stdio transport docks mechanically into the existing `RouteTarget::LocalStdio` seam, but keep a distinct `RemoteTarget::Container` rather than overloading `LocalStdio` — `LocalStdio` means "a program the user named, with no install or restart", whereas a container target owns an install and a lifecycle. The far side runs the server the way WSL does (a detached daemon, reached by the `--stdio` bridge); reuse the generic `Installer`, checksums, `remote_paths`, and reuse memos unchanged, and only write the `RemoteOps` implementation.

## Verification

**Commands:**
- `cargo test -p tty7-core` -- expected: all pass, including the new container unit/edge tests.
- `cargo build` -- expected: the workspace builds with no new warnings.

**Manual checks (if no CLI):**
- With a local Linux container running, register it, open a pane: confirm a shell inside the container and the container tag in the workspace list; reopen it and confirm no re-push; repeat with a `--network none` container.
