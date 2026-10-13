# CI baseline 2026-10-12 (Windows runner, `D:\a\tty7\tty7`)

Pasted log condensed to one row per distinct warning/error. The fix must clear
every row below; anything else warning on Linux only is out of scope (see
ticket Notes/Parked).

## Errors (blocking)

- E0252 ×24: `src/terminal/view.rs:10183-10188` second `use super::{...}` block
  re-imports the block at `:10164-10169`. Names: description_budget,
  drag_scroll_step, elide, encode_mouse, expand_file_command_template,
  fallback_chain, fig_icon_emoji, fig_icon_glyph, focus_report_bytes,
  highlight_runs, input_cells, input_char_positions, input_overflow_shift,
  input_overlay_rows, input_start, menu_layout, paste_bytes, select_end_copy,
  should_show_context_menu, smooth_scroll_step, submit_bytes,
  trim_trailing_spaces, wheel_route, wrapped_click_index.

## Warnings

- `crates/tty7-core/src/daemon/duplex.rs:2` unused import `Mutex`
- `crates/tty7-core/src/daemon/duplex.rs:175` unused import `super::*`
- `crates/tty7-core/src/daemon/duplex.rs:176` unused imports `Read as _`, `Write as _`
- `crates/tty7-core/src/host/server.rs:3` unused import `Path`
- `crates/tty7-core/src/daemon/pane.rs:3301/3315` `adopted`, `tail_size` never used
- `crates/tty7-core/src/daemon/remote.rs:6/11/98/124/214` `SshInvocation`,
  `parse_ssh_invocation`, `option_takes_value`, `foreground_argv`,
  `platform_foreground_argv` never used
- `crates/tty7-core/src/daemon/server.rs:34/618` `claim_ids_past`, `reaper`
  never used
- `crates/tty7-core/src/daemon/spawn.rs:1279` `is_supported_shell` never used
- `crates/tty7-core/src/core/github/token.rs:386` const `STATUS` never used
- `crates/tty7-core/src/daemon/router.rs:1021` `responder_lock` never used
- `crates/tty7-core/src/daemon/shell_integration.rs:2638/3039`
  `util_linux_script`, `reported_title` never used
- `crates/tty7-cli/src/address.rs:20/36` `ENV_INSTANCE_ROOT`, `instance_dir`
  never used
- `tests/edns_z_flag.rs:47` deprecated `iroh::dns::Builder::with_nameserver`
- `src/core/explorer_context_menu.rs:161` unused imports `RegQueryInfoKeyW`,
  `RegQueryValueExW`
- `src/terminal/view.rs:2422` method `manual_cwd` never used
- `src/terminal/view.rs:672/658` `LoopbackPlan` more private than
  `port_route_of`
- `src/ui/right_panel.rs:2132` unused import `Host`
- `src/ui/right_panel.rs:221/295` `CwdEdit` more private than `cwd_edit`
- `src/ui/app.rs:3393` unused `mut` on `failure`
- `src/ui/panel_review.rs:371` unused variable `repo`
- `src/terminal/view.rs:16228/16330` unused variable `bottom` (×2)
- `src/terminal/onekey.rs:25/75/139/143` `Pending`, `set_filter`, `cancel`,
  `entry_count` never used
- `src/terminal/search.rs:957` `LinkRoots::local` never used
- `src/terminal/git_data.rs:405/415` `known_status`, `probed` never used
- `src/terminal/element.rs:1860-1874` `fold_map`, `fold_epoch`, `fold_top`,
  `gutter_markers` never read
- `src/core/cli_install.rs:50/56` variants `InstalledOffPath`, `Occupied`
  never constructed
- `src/core/update.rs:2081` `PackageOffer::plain` never used
- `src/ui/app.rs:297/693/1469/8524/8539` `tile_trailing_inset_sm`,
  `leaf_title_and_home`, `with_session`, `active_ssh_pane`,
  `active_connected_native_ssh_pane` never used
- `src/ui/i18n/mod.rs:2040` `t_select` never used
- `src/ui/review_actions.rs:197/329/375` `review_send_to_new_agent`,
  `review_send_edit`, `deliver_review_to_new_agent` never used
- `src/ui/review_state.rs:106/127/137` `new`, `drafts_for_source`,
  `draft_count_for_source` never used
- `src/ui/search/mod.rs:104` `step` never used
- `src/ui/settings/kit.rs:37` field `k07` never read
- `src/ui/tab_strip.rs:1287` `workspace_avatar` never used
