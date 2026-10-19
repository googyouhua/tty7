---
id: SPEC-container-remote-host
companions: [container-host-design.md]
sources: []
---

> **Canonical contract.** This SPEC and the files in `companions:` are the complete, preservation-validated contract for what to build, test, and validate. Source documents listed in frontmatter are for traceability — consult them only if you need narrative rationale or prose color this contract intentionally omits.

# Container panes as remote hosts

## Why

A developer running the tty7 desktop app on the host and doing agent work inside a container is stuck: after entering the container the sidebar loses the agent avatar and the git branch, and the files/diff panels describe the host, not the code the agent is editing. tty7 already solves this shape for `ssh` and WSL panes by treating the far machine as a **remote host** served by its own `tty7-server`; a container pane is instead mistaken for the local host, so the in-container agent and repository are never consulted. This is an opportunity to close a conspicuous gap in the now-standard devcontainer workflow by extending the existing remote-host model to containers.

## Capabilities

- **CAP-1**
  - **intent:** A user can register a running container as a workspace and open panes whose shell runs inside that container.
  - **success:** After adding the container it appears in the host switcher tagged as a container, and a new pane opens a shell in that container.
- **CAP-2**
  - **intent:** On first open, tty7 installs and starts its server inside the container so the container becomes a full remote host, with no manual step.
  - **success:** A version-matched `tty7-server` is present and running in the container, and reopening the same running container reuses it rather than reinstalling.
- **CAP-3**
  - **intent:** The sidebar shows each container pane's agent with avatar and status exactly as for a local pane, independent of the host-side runtime argv.
  - **success:** Starting Claude Code or OpenCode inside a container pane shows the brand avatar and the working/waiting/done dot in the sidebar.
- **CAP-4**
  - **intent:** Branch, working-tree diff, file tree, and diffs reflect the repository inside the container for bind-mount and in-container-clone alike.
  - **success:** For each mount shape (bind same path, bind different path, in-container clone) the sidebar branch and diff counts equal what `git` reports when run inside the container.
- **CAP-5**
  - **intent:** Shell integration works in the container shell, so the working directory tracks and new tabs/splits open in place.
  - **success:** `cd` inside the container updates the pane cwd and Info row, and a new split opens in that container directory.
- **CAP-6**
  - **intent:** tty7 handles container stop, restart, and unsupported containers without breaking, degrading to host-side behavior with a visible reason.
  - **success:** A stopped container shows a clear non-connected state; a container without exec permission, a non-Linux container, or a noexec-only writable location leaves the pane usable with a stated reason instead of crashing.
- **CAP-7**
  - **intent:** A user whose container runtime lives in a WSL distro — docker inside a WSL distro, or the Docker Desktop WSL2 backend — can add and use containers exactly as on a native Linux host.
  - **success:** On Windows, with the runtime in a WSL distro or on the Windows host backed by Docker Desktop WSL2, adding a running container yields a connected workspace whose sidebar shows the agent avatar, status dot, branch, and diff.
- **CAP-8**
  - **intent:** When the user enters a container from a local pane (`docker exec`), tty7 detects it and offers a one-click way to bring that container into tty7 as a workspace.
  - **success:** Running `docker exec -it <ctr>` in a local pane surfaces an entry, and accepting it re-homes that pane into a connected container workspace (keeping its scrollback) whose sidebar shows agent avatar, status dot, branch, and diff.

## Constraints

- No pre-install may be required inside the container image; the server binary is pushed by tty7 (static musl).
- No change to how the container is started (`docker run`/compose/devcontainer) and no added flags; tty7 attaches to an already-running container.
- The host docker socket stays on the host and must never be mounted into the container; the host-side `docker` CLI is its only user.
- The server install path inside the container mirrors the remote convention (`$HOME/.local/share/tty7/bin/tty7-server-<dialect>`, same dir chain and modes); when that location is unwritable or noexec, tty7 relocates the server to an executable writable directory and retries before degrading.
- Containers enter tty7 only by explicit user consent through the entry picker; tty7 never enumerates or adopts containers in the background.
- The container must be Linux and permit `docker exec`; the runtime CLI must be reachable and honor `DOCKER_HOST`/context.
- Reuse the existing remote-workspace plumbing; do not build a parallel host stack.
- The transport must not depend on the container network: the protocol rides the `docker exec` stdio channel over the host's docker socket, so network isolation (`bridge`/`host`/`none`) is irrelevant and no in-container listener is allowed. A runtime whose daemon is itself remote (`DOCKER_HOST` over ssh/tcp) is out of scope for v1.
- No privilege escalation beyond the user's existing container-runtime access.
- Docker is required for v1; podman and nerdctl are best-effort.

## Non-goals

- Not a `devcontainer.json` feature runner or image builder: tty7 does not build, manage, or configure images.
- Windows containers are out of scope for v1.
- tty7 does not install or elevate a container runtime.

## Success signal

On a host running the tty7 app, add a running devcontainer as a workspace, open a pane, start the agent inside it, and the sidebar shows the avatar, status dot, branch, and diff identically to a local pane — with nothing installed into the image.

## Assumptions

- The container is a persistent, running Linux container, not a one-shot `docker run --rm` that exits.
- The container runtime is reachable directly on the host or through a WSL distro; the transport runs wherever the runtime CLI does.
- The host build can supply a static musl `tty7-server` matching the container architecture (x86_64/aarch64), as release assets already do.
- The Docker CLI is on the host `PATH` (or, for a WSL-hosted runtime, on the distro's `PATH`) and the user can list and exec into containers.
- A writable and executable location exists inside the container once tty7 relocates past a noexec one; otherwise it degrades with a stated reason.

## Open Questions

<!-- None outstanding; all resolved decisions are in .memlog.md and reflected above. -->

