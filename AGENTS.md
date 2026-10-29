<!-- bmad:context -->
<!-- Verified 2026-10-14 against 72a2e92a. Managed by bmad-project-context; edits inside this block are replaced on refresh. Keep anything you want preserved outside the markers. -->

## tty7

Terminal workbench: persistent sessions, remote work, agents. Pure Rust, GPU rendering on Zed's gpui, VT core from Alacritty. Planning lives in `openspec/` and `docs/`, tickets in `_bmad-output/` per active initiative.

## Policy

- Never push to main; open a PR and get at least one approval.
- Never casually edit `vendor/simple-dns/` — it is a local patch of upstream (see `Cargo.toml.orig`); a change forks upstream.
- Route secrets through the system keychain/OneKey entry; never write plaintext credentials to disk or logs.
- Before implementation work, ask whether to isolate in a worktree under `.worktrees/` — state the current worktree status first, and never create, switch, or reuse one without the user's explicit consent (auto/unattended runs may create directly).

## Where things are

- Workspace members in `crates/`: core (headless protocol/daemon/PTY/SSH, no gpui), server (headless proof), cli (thin client), gateway + mobile-proto/mobile-client (phone pairing over iroh); `mobile/src-tauri/` is its own workspace.
- Config truth is `crates/tty7-core/src/core/config.rs` (`CoreConfig`); GUI `src/core/config.rs` is a thin Global wrapper; `docs/reference/configuration.mdx` is a hand-kept mirror.
- UI chrome lives in `src/ui/` and hosts `TerminalView`; VT grid/render/input lives in `src/terminal/`; i18n keys in `src/ui/i18n/` (en, zh-CN, ja-JP, ru-RU).
- `ui::`/`terminal::` must reach the filesystem only via `src/ui/host_ops.rs` (`Host`); exemptions live in `.github/scripts/check-host-boundary.sh`.

## Running and verifying

- Iterate with `cargo test -p <tty7-core|tty7-cli|tty7-server>`; touch the updater only with `cargo test --features updater --bin tty7-updater`; pre-PR run bare `cargo test` (covers default-members only, gateway/mobile excluded).
- CI pins `cargo build/test --locked`; nightly/release stamping rewrites `Cargo.toml` and must not use `--locked` — do not copy flags across.
- `cargo fmt --check` is a required check; `host-boundary` stays non-required but keep it clean via the script above.
- Linux builds need the system deps in `.github/workflows/ci.yml` (pkg-config, cmake, clang, wayland/x11, krb5) with `LIBGSSAPI_IMPL=mit`.

## Conventions that differ from defaults

- `tty7-core` never depends on gpui; `tty7-server` proves headless; only the GUI crate enables gssapi/remote-install/github/iroh/keyring.
- `mobile-proto`/`mobile-client` stay off `tty7-core` for iOS/Android cross-compile; `tty7-gateway` translates.
- Docs-only changes skip Rust jobs through the `changes` job in CI; never use workflow-level `paths-ignore` (pends required checks).

## Known pitfalls

- Bare `cargo test` is not the full workspace; `--workspace` unifies features and hides `-p tty7-core` gaps — use bare `cargo test` as the green signal.
- In `src/ui`/`src/terminal`, `std::fs::`, `Command::new("git")`, `.canonicalize()`, `.is_absolute()` fail the boundary grep unless allowlisted — use `Host` methods instead.

<!-- /bmad:context -->
