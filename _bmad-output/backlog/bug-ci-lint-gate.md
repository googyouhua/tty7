---
id: ""
type: bug
title: "CI lint gate red: E0252 duplicate imports plus warnings"
parent: none
covers: []
after: []
assignee: ""
refined: false
hitl: false
risk: medium
severity: P1
---

# CI lint gate red: E0252 duplicate imports plus warnings

## Description

Windows CI fails to compile the bin test target: 24 E0252 duplicate-import errors from a second `use super::{...}` block in the `src/terminal/view.rs` test module. The same log carries ~25 rustc warnings (unused imports, dead code, private interfaces, one deprecated iroh call). Until the E0252 is fixed, no bin tests can build or run on any platform.

## Reproduction

On `origin/main` (or any branch carrying it): `cargo test --bin tty7-app --no-run` fails with 24× E0252 at `src/terminal/view.rs:10183-10188` re-importing the block at `:10164-10169`. `cargo build --workspace --all-targets` emits the warning set from the CI log.

## Cause Hypothesis

The second import block looks like a bad merge duplicate — but it is NOT a pure duplicate: it adds `before_input` (used by tests) while the first block has `smooth_scroll_step_visible` (also used). Blindly deleting either block breaks tests. The warnings split into three groups (investigated 2026-10-12, evidence in Notes): truly dead code safe to remove; visibility/underscore/mut micro-fixes; and items that only LOOK dead because the E0252 hides their test-only callers or because their callers are cfg-gated to other platforms — those must be kept or gated, never deleted.

## Acceptance Criteria

1. **Bin test target compiles**
   **Given** a clean tree on the fix branch
   **When** `cargo test --bin tty7-app --no-run` runs
   **Then** it compiles with zero errors
2. **CI warning set is gone**
   **Given** the fixed tree
   **When** `cargo build --workspace --all-targets` runs
   **Then** no row from `ci-baseline-2026-10-12.md` (same folder) still warns
3. **Nothing that was alive got killed**
   **Given** the fix
   **When** the previously-green suites run (`core::instance`, `username_selector_lifecycle`, plus a compile of all test targets)
   **Then** all green, and the full bin suite runs (the E0252 that blocked it is fixed here)
4. **Windows-only blind spots flagged**
   **Given** files that cannot compile on Linux (`explorer_context_menu.rs`)
   **When** the fix touches them
   **Then** changes are whole-unused-item deletions only (no signature or logic edits), called out in Notes, with Windows CI as verifier

## References

- parent — none
- source — `ci-baseline-2026-10-12.md` (same folder; condensed from the chat-pasted Windows CI log, one row per warning/error; no run URL was provided)

## Notes

- Investigation decision log (2026-10-12, three parallel codegraph/grep passes — DO NOT "fix" these by deletion):
  - KEEP (test-only callers hidden by the breakage): `ReplayRing::adopted`/`tail_size` (pane.rs tests), `manual_cwd()` (view.rs tests), `responder_lock`, `util_linux_script`, `reported_title`, `STATUS` (unix tests), `address.rs` items (same-file tests), `set_filter`/`cancel`, `LinkRoots::local`, `known_status`, `fold_*` group, `ReviewState` ×3, `SearchTab::step`, `with_session`, `active_ssh_pane`.
  - KEEP (live cfg-gated callers): `duplex.rs` imports, `host/server.rs` `Path`, `SshInvocation` group, `claim_ids_past`, `reaper`, `is_supported_shell`, `cli_install` variants, `update.rs` `plain`, `Pending` (matched live in view.rs).
- As-built corrections during review (verified by green Windows-target + Linux checks, NOT reversions — the investigation above was imprecise on these points):
  - `Pending`: matched but never CONSTRUCTED anywhere → deleted variant + dead arm (zero behavior change; keeping it would leave the baseline row red).
  - `active_ssh_pane`: its sole caller was the deleted `active_connected_native_ssh_pane` → deleted as cascade.
  - `GridSnapshot.fold_epoch`: read nowhere (live or tests; `FrameMap` owns a separate field) → deleted field + dead assignments; `fold_map`/`fold_top`/`gutter_markers` kept under `allow(dead_code)` (live constructor sets them, tests read them).
  - right_panel `Host` import: compiler-proven unneeded on both targets (methods resolve without it) → deleted.
  - Gates beyond the plan's short list (each matched to caller cfgs; Windows-target check green is the proof): `adopted` → `any(test, unix)`; `tail_size`, `foreground_argv` (plus deleted Windows-only `platform_foreground_argv` stub), `claim_ids_past`, `reaper` → `unix`; `SshInvocation` struct/`parse`/`option_takes_value`, `is_supported_shell`, `update.rs` `plain` → `any(test, unix)`; `duplex.rs`/`host/server.rs` imports → `unix`; `cli_install` variants + match arms → `unix`; test-only fns (`set_filter`, `cancel`, `LinkRoots::local`, `known_status`, `ReviewState` ×3, `SearchTab::step`, `manual_cwd()`, `with_session`) → `#[cfg(test)]`.
  - Same-class extras absent from the pasted CI log but proven red on Windows-target check: `Spot::Search` variant + arm (never constructed), `ArenaClearNeeded` `let _ =` ×2 (test must-use sites).
  - FIX by deletion: `entry_count`, `probed`, `active_connected_native_ssh_pane`, `tile_trailing_inset_sm`, `leaf_title_and_home`, `t_select`, `review_actions` ×3, `Tk::k07` + init, `workspace_avatar`, `RegQuery*W` names, view `bottom` ×2.
  - FIX otherwise: E0252 = UNION of both import blocks; `LoopbackPlan`/`CwdEdit` → `pub(crate)`; `app.rs` remove `mut`; `panel_review.rs` `repo` → `_repo`; `edns_z_flag.rs` migrate `with_nameserver(addr, Udp)` → `add_nameserver_config(NameserverConfig::udp(ip).with_port(port))` (pinned iroh 1.3.0).
  - GATE to match caller cfgs (never delete): `responder_lock`, `util_linux_script`, `reported_title`, `STATUS`, `address.rs` items → `#[cfg(unix)]` or move into test module as appropriate.
- Parked (in Linux output but NOT in the CI set — follow-up, out of scope): `key`/`label`/`placeholder` methods, `sender`, `replace_entry_with`, `registrations`, `quoted_open_command`, `application_path`, `Location`, `DIRECTORY_KEY`, `BACKGROUND_KEY`, `Registration`, `windows_background_appearance`, `supported_backdrops_for`, `height`.
- Branch note: built on `dev/ci-lint-gate` but LANDED on `dev/username-selector`
  via cherry-pick (the epic PR needed it for CI); `dev/ci-lint-gate` was left
  at `origin/main` unpushed and can be dropped.
