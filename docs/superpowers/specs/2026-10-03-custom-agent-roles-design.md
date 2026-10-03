# Custom Agent Roles — Design

Date: 2026-10-03
Scope: V1 implements items 1–6 only. Skills, execution settings (concurrency/env/MCP),
Access, copy/archive are deferred. Roles always bind to one of the 26 built-in agents.

## 1. Background

`CLIAgent` (`crates/tty7-core/src/core/cli_agent.rs:6`) is a hardcoded enum of 26
agents. Configurable today: `agent_commands` (wrapper alias) and `agent_launch`
(launch line) (`crates/tty7-core/src/core/config.rs:436,442`). There is no way to
define a new agent kind. Requirement: user-defined roles where each role has its
own function (Multica-style: name + instructions + starters), bound to an existing
base agent so detection / status / resume / history are reused.

Reference: Multica agent = name/avatar/description + instructions (every run) +
up to 3 conversation starters + skills + runtime/model + access + execution
settings (concurrency, env, CLI args, MCP). tty7 runs the real CLI in a PTY and
does not host models, so the role maps onto: identity + launch + prompts.

## 2. Storage and schema

- Location: `<config-dir>/roles/<slug>/role.json`. One directory per role (not a
  single `<slug>.json`) to reserve room for future `assets/` / `SKILL.md` without
  migration. Config dir resolution follows existing `config_dir()`
  (`TTY7_CONFIG_DIR` / `--config-dir` / portable / platform default).
- Local and remote machines each read their own roles directory. V1 does no
  auto-sync (documents the same caveat as `agent_launch` wrappers on remote).
- `role.json` fields (V1):
  - `slug`: lowercase `a-z0-9-`, must equal directory name, immutable after creation.
  - `name`: display name, freely editable.
  - `base`: one of the 26 built-in slugs (`claude`, `codex`, …). Anything else →
    ignore the whole file with a log line (same lenient style as config).
  - `description`: display-only, never enters the prompt.
  - `launch`: single command line, e.g. `codex --profile review`. Model / thinking
    level fold into this line; must be non-empty after trim.
  - `instructions`: first-message / persona text sent on every fresh launch.
  - `starters`: max 3 `{label, prompt}` objects; extras truncated.
- Loading: re-scanned with mtime-based caching (same pattern as
  `agent_detection_aliases`), so edits apply without restart. A broken file is
  skipped individually; it never fails the whole list.

## 3. Settings UI

- New top-level section `SettingsSection::AgentRoles` (nav label `Agents` /
  `Agent 角色`), alongside existing `SettingsSection::Agents` (`Integrations`,
  `src/ui/settings.rs:206,219`). Integrations keeps hooks + CLI-on-PATH; roles
  get their own page so the hooks list does not grow unbounded.
- Page content: role list (`name + base + description`) + `New role` + detail
  form editing all 7 fields. `base` is a dropdown of the 26 built-ins; starters
  are 3 `label + prompt` inputs.
- Slug auto-generated from name (lowercase, spaces → `-`, collision → `-2`);
  renaming `name` never changes `slug`. Delete removes `roles/<slug>/`.
- i18n: add `SettingsNavAgentRoles` + list/form strings to `en` / `zh-CN` / `ja`,
  following the existing `L10nKey` flow.

## 4. Launch, pane tagging, prompt delivery

- Quick launch: Search Everywhere Terminals tab gains `Role: <Name>` rows next to
  `Agent: …`, ranked by existing `agent_frecency` under `role:<slug>` keys.
  Bindable as `LaunchRole:<slug>` (mirrors `LaunchAgent:<slug>` in
  `src/ui/agent_launch.rs:30`). New Tab menu unchanged in V1.
- Offering rule mirrors `installed_on` (`src/ui/agent_launch.rs:58`): a role is
  offered only when its launch program is on local `PATH`; on remote workspaces
  only when its base was seen there.
- Pane tagging: spawn carries `role: Option<slug>`. The daemon still detects the
  base `CLIAgent` from the PTY; the UI shows `RoleName (BaseDisplay)` with the
  base avatar. Status dots, notifications, `tty7 wait`, past-session resume/fork
  all reuse the base path unchanged.
- Reboot restore: resume line = base `resume_command` + role launch flags
  (same flag-replay as `resume_line` in `agent_launch.rs:124`); `instructions`
  are not resent into a continued session.
- `instructions` delivery:
  - Base supports `prompt_args` (Claude/Codex/Gemini, `cli_agent.rs:725`):
    append instructions as the initial prompt arg — single send.
  - Others: two-phase — launch via existing `run_when_ready` (waits for shell
    prompt), then send `instructions` via `submit_bytes` (bracketed paste +
    Enter, `src/core/agent_prompt.rs:53`) with the swallowed-Enter check
    (`capture` + resend Enter). Best-effort in V1; a missed first send can be
    resent from starters.
- `starters`: pane right-click menu `Send starter: <label>` (≤3) + palette
  entries; manual send only via `submit_bytes`, never automatic.

## 5. Validation, errors, docs, tests

- Validation: slug charset + dir match, base membership, non-empty launch,
  starters ≤ 3. Failures skip that file with a log; the list hides it (no error
  row in V1).
- Dangling references: a pane whose role was deleted falls back to the base
  agent display; restore without the role file falls back to base resume.
- Docs: new Roles section in `docs/agents/overview.mdx` (fields + remote
  caveat); note in `docs/reference/configuration.mdx` that roles live under
  `roles/`, not in `config.json`.
- Tests: role load/validation (good file, bad base, slug mismatch, starters
  truncation); instructions two paths (prompt_args vs two-phase); `LaunchRole`
  parsing. Remote non-sync is documentation-only.
- Explicitly out of V1: skills, env/MCP/concurrency fields, Access, copy/archive
  UI, CLI verbs for roles, auto-sync of `roles/` to remotes.
