# Deferred work

- source_spec: `_bmad-output/specs/spec-container-remote-host/stories/1-container-transport-skeleton.md`
  summary: Story 1's four ATDD scaffolds remain `#[ignore]`d and CI never runs them, so its acceptance contract is unguarded.
  evidence: `crates/tty7-core/src/daemon/install/container/atdd.rs:23,58,85,111` carry `#[ignore = "ATDD RED: …"]`; `cargo test -p tty7-core --lib atdd` reports 0 passed, 4 ignored; `.github/workflows/ci.yml` runs `cargo test --locked` without `--ignored`.

- source_spec: `_bmad-output/specs/spec-container-remote-host/stories/1-container-transport-skeleton.md`
  summary: Story 1's acceptance tests restate implementation details instead of the promised behavior.
  evidence: `crates/tty7-core/src/daemon/install/container.rs:93-97,291-300` assert `docker_args` and one relocation identity arm; AC-2 seeds its own memo, so the writable+exec probe is never exercised.
