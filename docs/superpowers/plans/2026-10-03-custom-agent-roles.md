# Custom Agent Roles Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add user-defined agent roles bound to the 26 built-in agents, stored under `roles/<slug>/role.json`, manageable from a new Settings page, launchable from Search, with instructions/starters delivery.

**Architecture:** New `core::agent_roles` module owns loading/validation; `agent_launch` gains role offering plus `LaunchRole` plumbing reusing base-agent detection/status/resume; pane spawn carries an optional role slug with fallback to base display; Settings page edits the same files.

**Tech Stack:** Rust (existing workspace: `tty7-core` lib + GUI binary), gpui settings UI, existing `L10nKey` i18n flow, `cargo test`.

**Spec:** `docs/superpowers/specs/2026-10-03-custom-agent-roles-design.md`

## Global Constraints

- Roles always bind to one of the 26 built-in `CLIAgent` slugs; anything else ignores that file with a log line.
- `slug` is lowercase `a-z0-9-`, equals its directory name, immutable after creation.
- `starters` max 3 entries; extras truncated.
- `launch` must be non-empty after trim.
- Broken role files are skipped individually and never fail the whole list.
- A pane whose role is missing/deleted falls back to base-agent display and base resume.
- No new third-party dependencies; follow the existing lenient-config style.
- V1 excludes: skills, env/MCP/concurrency fields, Access, copy/archive UI, CLI verbs, auto-sync to remotes.

---

## File Structure

- Create: `crates/tty7-core/src/core/agent_roles.rs` — `AgentRole` / `RoleStarter` types, `roles_dir()`, `load_roles()`, slug helpers, launch-program helper. Single responsibility: roles on disk.
- Modify: `crates/tty7-core/src/core/mod.rs` — register `agent_roles`.
- Modify: `crates/tty7-core/src/core/config.rs` — frecency helpers accept `role:<slug>` keys (no schema change; keys are plain strings).
- Modify: `src/ui/agent_launch.rs` — role offering (`offered_roles`, `role_for_launch_action`, `LAUNCH_ROLE_PREFIX`), role launch + resume-line builders reusing base `CLIAgent` methods.
- Modify: `src/core/actions.rs` — new `LaunchRole { slug }` action mirroring `LaunchAgent`.
- Modify: `src/ui/keymap.rs` + `src/ui/app.rs` — wire `LaunchRole:<slug>` keybindings, palette `Role: <Name>` rows, `CommandKind::LaunchRole` dispatch.
- Modify: `src/ui/pending_pane.rs` — `PendingSpawn.role: Option<String>` plumbed through spawn/restore paths.
- Modify: `src/ui/app.rs` + `src/ui/tree_sync.rs` + `src/terminal/view.rs` (display only) — show `RoleName (Base)` with base avatar, fallback to base.
- Create: `src/ui/settings/agent_roles.rs` — new Settings page (list + new/edit/delete) reading/writing `roles/<slug>/role.json`.
- Modify: `src/ui/settings.rs` + `src/ui/settings/shell.rs` — register `SettingsSection::AgentRoles`, nav, dispatch.
- Modify: `src/ui/i18n/mod.rs` + `en.rs` + `zh.rs` + `ja.rs` — new `L10nKey`s.
- Modify: `docs/agents/overview.mdx` + `docs/reference/configuration.mdx` — Roles docs.
- Test: unit tests live next to code (`agent_roles.rs`, `agent_launch.rs` tests); Settings regressions covered by running the existing `settings` test suites.

---

### Task 1: Role storage module

**Files:**
- Create: `crates/tty7-core/src/core/agent_roles.rs`
- Modify: `crates/tty7-core/src/core/mod.rs`
- Test: `crates/tty7-core/src/core/agent_roles.rs` (inline `#[cfg(test)]`)

**Interfaces:**
- Consumes: `CLIAgent::from_slug` (`crates/tty7-core/src/core/cli_agent.rs`), `config_dir_path()` (`crates/tty7-core/src/core/config.rs`).
- Produces: `pub struct RoleStarter { pub label: String, pub prompt: String }`, `pub struct AgentRole { pub slug: String, pub name: String, pub base: CLIAgent, pub description: String, pub launch: String, pub instructions: String, pub starters: Vec<RoleStarter> }`, `pub fn roles_dir() -> Option<PathBuf>`, `pub fn load_roles(dir: &Path) -> Vec<AgentRole>`, `pub fn slug_from_name(name: &str) -> String`, `pub fn slug_valid(slug: &str) -> bool`, `pub fn role_launch_program(role: &AgentRole) -> Option<String>` used by Tasks 2, 4, 5.

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn bad_base_slug_skips_only_that_file() {
    let dir = tempfile::TempDir::new().unwrap();
    let good = dir.path().join("good").join("role.json");
    std::fs::create_dir_all(good.parent().unwrap()).unwrap();
    std::fs::write(&good, r#"{"slug":"good","name":"Good","base":"claude","launch":"claude"}"#).unwrap();
    let bad = dir.path().join("bad").join("role.json");
    std::fs::create_dir_all(bad.parent().unwrap()).unwrap();
    std::fs::write(&bad, r#"{"slug":"bad","name":"Bad","base":"nope","launch":"nope"}"#).unwrap();
    let roles = load_roles(dir.path());
    assert_eq!(roles.iter().map(|r| r.slug.as_str()).collect::<Vec<_>>(), vec!["good"]);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p tty7-core core::agent_roles::tests::bad_base_slug_skips_only_that_file -v`
Expected: FAIL with "no function or associated item named `load_roles` found" (module does not exist yet).

- [ ] **Step 3: Write minimal implementation**

```rust
use std::path::{Path, PathBuf};
use crate::core::cli_agent::CLIAgent;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoleStarter { pub label: String, pub prompt: String }

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentRole {
    pub slug: String,
    pub name: String,
    pub base: CLIAgent,
    pub description: String,
    pub launch: String,
    pub instructions: String,
    pub starters: Vec<RoleStarter>,
}

pub fn slug_valid(slug: &str) -> bool {
    !slug.is_empty()
        && slug.len() <= 64
        && slug.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

pub fn slug_from_name(name: &str) -> String {
    let mut s: String = name.to_lowercase().chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' }).collect();
    while s.contains("--") { s = s.replace("--", "-"); }
    let s = s.trim_matches('-').to_string();
    if s.is_empty() { "role".to_string() } else { s }
}

pub fn roles_dir() -> Option<PathBuf> {
    crate::core::config::config_dir_path().map(|d| d.join("roles"))
}

fn load_one(dir: &Path, slug: &str) -> Option<AgentRole> {
    let text = std::fs::read_to_string(dir.join("role.json")).ok()?;
    let v: serde_json::Value = serde_json::from_str(&text).ok()?;
    let file_slug = v.get("slug").and_then(|s| s.as_str()).unwrap_or("");
    if file_slug != slug || !slug_valid(file_slug) { return None; }
    let base = v.get("base").and_then(|s| s.as_str())
        .and_then(CLIAgent::from_slug)?;
    let launch = v.get("launch").and_then(|s| s.as_str()).unwrap_or("").trim().to_string();
    if launch.is_empty() { return None; }
    let mut starters: Vec<RoleStarter> = v.get("starters").and_then(|s| s.as_array())
        .map(|a| a.iter().filter_map(|e| Some(RoleStarter {
            label: e.get("label").and_then(|l| l.as_str()).unwrap_or("").trim().to_string(),
            prompt: e.get("prompt").and_then(|p| p.as_str()).unwrap_or("").to_string(),
        })).filter(|s| !s.label.is_empty() && !s.prompt.trim().is_empty()).collect())
        .unwrap_or_default();
    starters.truncate(3);
    Some(AgentRole {
        slug: file_slug.to_string(),
        name: v.get("name").and_then(|s| s.as_str()).unwrap_or(file_slug).to_string(),
        base,
        description: v.get("description").and_then(|s| s.as_str()).unwrap_or("").to_string(),
        launch,
        instructions: v.get("instructions").and_then(|s| s.as_str()).unwrap_or("").to_string(),
        starters,
    })
}

pub fn load_roles(dir: &Path) -> Vec<AgentRole> {
    let Ok(entries) = std::fs::read_dir(dir) else { return Vec::new(); };
    let mut out: Vec<AgentRole> = entries.flatten()
        .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
        .filter_map(|e| e.file_name().to_str().map(|s| s.to_string()))
        .filter_map(|slug| load_one(&dir.join(&slug), &slug))
        .collect();
    out.sort_by(|a, b| a.slug.cmp(&b.slug));
    out
}

pub fn role_launch_program(role: &AgentRole) -> Option<String> {
    crate::core::cli_agent::launch_program(&role.launch)
}
```

Register in `crates/tty7-core/src/core/mod.rs`:

```rust
pub mod agent_roles;
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p tty7-core core::agent_roles -v`
Expected: PASS (add slug-mismatch, empty-launch, starters-truncation cases in the same module following the Step 1 pattern).

- [ ] **Step 5: Commit**

```bash
git add crates/tty7-core/src/core/agent_roles.rs crates/tty7-core/src/core/mod.rs
git commit -m "feat: agent role storage under roles/<slug>/role.json"
```

### Task 2: Offering, keybinding, palette rows

**Files:**
- Modify: `src/ui/agent_launch.rs`
- Modify: `src/core/actions.rs`
- Modify: `src/ui/keymap.rs`
- Modify: `src/ui/app.rs` (palette rows + dispatch only)
- Test: `src/ui/agent_launch.rs` (inline tests)

**Interfaces:**
- Consumes: Task 1 `AgentRole`, `load_roles`, `role_launch_program`; `CLIAgent::from_slug`; `program_on_path`.
- Produces: `pub const LAUNCH_ROLE_PREFIX: &str`, `pub fn role_for_launch_action(action: &str, roles: &[AgentRole]) -> Option<AgentRole>`, `pub fn offered_roles(roles: Vec<AgentRole>, path: &OsStr, seen_bases: Option<&[String]>) -> Vec<AgentRole>`, `pub fn role_launch_line(role: &AgentRole) -> String`, `pub fn role_resume_line(role: &AgentRole, session_id: &str) -> Option<String>` used by Tasks 3–4.

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn role_action_round_trips_by_slug() {
    let role = AgentRole {
        slug: "frontend-reviewer".into(), name: "Frontend Reviewer".into(),
        base: CLIAgent::Claude, description: String::new(),
        launch: "claude --model opus".into(), instructions: String::new(),
        starters: vec![],
    };
    let action = format!("{LAUNCH_ROLE_PREFIX}{}", role.slug);
    assert_eq!(role_for_launch_action(&action, &[role.clone()]).map(|r| r.slug),
        Some("frontend-reviewer".to_string()));
    assert!(role_for_launch_action("LaunchRole:nobody", &[role]).is_none());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --bin tty7-app agent_launch::tests::role_action_round_trips_by_slug -v` (use the repo's actual GUI binary target name; if it differs, run the `agent_launch` module tests for that target).
Expected: FAIL with `LAUNCH_ROLE_PREFIX` / `role_for_launch_action` not found.

- [ ] **Step 3: Write minimal implementation**

```rust
use tty7_core::core::agent_roles::{AgentRole, role_launch_program};

pub(crate) const LAUNCH_ROLE_PREFIX: &str = "LaunchRole:";

pub(crate) fn role_for_launch_action(action: &str, roles: &[AgentRole]) -> Option<AgentRole> {
    let slug = action.strip_prefix(LAUNCH_ROLE_PREFIX)?;
    roles.iter().find(|r| r.slug.eq_ignore_ascii_case(slug.trim())).cloned()
}

pub(crate) fn offered_roles(
    roles: Vec<AgentRole>,
    path: &std::ffi::OsStr,
    seen_bases: Option<&[String]>,
) -> Vec<AgentRole> {
    roles.into_iter().filter(|r| match seen_bases {
        Some(seen) => seen.iter().any(|s| s.eq_ignore_ascii_case(r.base.slug())),
        None => role_launch_program(r)
            .is_some_and(|p| crate::core::cli_agent::program_on_path(&p, path)),
    }).collect()
}

pub(crate) fn role_launch_line(role: &AgentRole) -> String {
    role.launch.trim().to_string()
}

pub(crate) fn role_resume_line(role: &AgentRole, session_id: &str) -> Option<String> {
    let argv: Vec<String> = role.launch.split_whitespace().map(str::to_string).collect();
    role.base.resume_command(session_id, Some(&argv))
}
```

Wire `src/core/actions.rs` (mirror `LaunchAgent`):

```rust
pub struct LaunchRole {
    pub slug: String,
}
```

Wire `src/ui/keymap.rs`: register `LaunchRole:<slug>` names from loaded roles next to `launch_action_names()`; dispatch in `src/ui/app.rs` palette builder as `Item::new("Role: {name}", CommandKind::LaunchRole(slug))` with subtitle `role.launch` and base-agent avatar, and handle `CommandKind::LaunchRole` by calling the Task 3 role launcher.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --bin tty7-app agent_launch -v`
Expected: PASS (existing `agent_launch` tests plus the new role test).

- [ ] **Step 5: Commit**

```bash
git add src/ui/agent_launch.rs src/core/actions.rs src/ui/keymap.rs src/ui/app.rs
git commit -m "feat: offer roles in palette with LaunchRole actions"
```

### Task 3: Pane role tag with base fallback

**Files:**
- Modify: `src/ui/pending_pane.rs`
- Modify: `src/ui/app.rs` (launch path, display label, restore path)
- Modify: `src/ui/tree_sync.rs` (carry `role` through sync/restore structs)
- Test: extend `src/ui/app.rs` tests pattern for spawn metadata

**Interfaces:**
- Consumes: Task 2 `role_launch_line`, `role_resume_line`; existing `PendingSpawn.agent_launch_argv`, `run_when_ready`.
- Produces: `PendingSpawn.role: Option<String>`; `fn launch_role(role: &AgentRole, ...)`; `fn role_display(role_slug: Option<&str>, agent: Option<CLIAgent>) -> String` (resolves the slug via `find_role`; unknown/deleted slug reads as no role) used by Task 4.

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn deleted_role_falls_back_to_base_display() {
    assert_eq!(role_display(Some("gone"), Some(CLIAgent::Claude)), "Claude Code");
    assert_eq!(
        role_display(Some("frontend-reviewer"), Some(CLIAgent::Claude)),
        "Frontend Reviewer (Claude Code)"
    );
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --bin tty7-app role_display -v`
Expected: FAIL with `role_display` not found.

- [ ] **Step 3: Write minimal implementation**

```rust
// pending_pane.rs
pub struct PendingSpawn {
    // ... existing fields ...
    pub role: Option<String>,
}

// app.rs
pub(crate) fn role_display(role_slug: Option<&str>, agent: Option<CLIAgent>) -> String {
    let name = role_slug
        .map(str::trim)
        .filter(|slug| !slug.is_empty())
        .and_then(|slug| find_role(slug).map(|role| role.name));
    match (name, agent) {
        (Some(name), Some(a)) => format!("{name} ({})", a.display_name()),
        (Some(name), None) => name,
        (None, Some(a)) => a.display_name().to_string(),
        (None, None) => String::from("Shell"),
    }
}
```

Launch: new-tab slot + `run_when_ready(slot, role_launch_line(role))`, storing `PendingSpawn { role: Some(slug), agent: Some(base), agent_launch_argv }`. Restore: `role_resume_line(role, id)` when the role file still exists, else base `resume_line`; `instructions` never resent on restore. `tree_sync.rs`: add matching `role: Option<String>` fields wherever `agent_launch_argv` is cloned so restores keep the tag.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --bin tty7-app role_display -v`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/ui/pending_pane.rs src/ui/app.rs src/ui/tree_sync.rs
git commit -m "feat: tag panes with role slug, fall back to base display"
```

### Task 4: Instructions and starters delivery

**Files:**
- Modify: `src/ui/agent_launch.rs` (send planner)
- Modify: `src/ui/app.rs` (pane context menu + palette starters)
- Modify: `src/core/agent_prompt.rs` (reuse `submit_bytes`)
- Test: `src/ui/agent_launch.rs` tests

**Interfaces:**
- Consumes: Task 1–3 (`AgentRole`, `role_launch_line`, pane role tag); `CLIAgent::prompt_args`; `submit_bytes` (`src/core/agent_prompt.rs:53`).
- Produces: `pub enum RoleFirstSend { PromptArg(Vec<String>), TwoPhase { launch: String, followup: String } }`, `pub fn plan_role_first_send(role: &AgentRole) -> Option<RoleFirstSend>`; context-menu senders call existing `run_when_ready` + `submit_bytes`.

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn instructions_use_prompt_arg_where_supported() {
    let role = AgentRole {
        slug: "r".into(), name: "R".into(), base: CLIAgent::Claude,
        description: String::new(), launch: "claude".into(),
        instructions: "Be terse.".into(), starters: vec![],
    };
    assert!(matches!(plan_role_first_send(&role), Some(RoleFirstSend::PromptArg(_))));
    let opencode = AgentRole { base: CLIAgent::OpenCode, launch: "opencode".into(), ..role };
    assert!(matches!(plan_role_first_send(&opencode), Some(RoleFirstSend::TwoPhase { .. })));
    let silent = AgentRole { instructions: "   ".into(), ..opencode };
    assert!(plan_role_first_send(&silent).is_none());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --bin tty7-app plan_role_first_send -v`
Expected: FAIL with `plan_role_first_send` / `RoleFirstSend` not found.

- [ ] **Step 3: Write minimal implementation**

```rust
use tty7_core::core::agent_roles::AgentRole;

pub(crate) enum RoleFirstSend {
    PromptArg(Vec<String>),
    TwoPhase { launch: String, followup: String },
}

pub(crate) fn plan_role_first_send(role: &AgentRole) -> Option<RoleFirstSend> {
    let text = role.instructions.trim();
    if text.is_empty() {
        return None;
    }
    if let Some(mut argv) = role.base.prompt_args(text) {
        let mut full = vec![role_launch_line(role)];
        full.append(&mut argv);
        return Some(RoleFirstSend::PromptArg(full));
    }
    Some(RoleFirstSend::TwoPhase {
        launch: role_launch_line(role),
        followup: text.to_string(),
    })
}
```

`TwoPhase` execution: send `launch` via `run_when_ready`; queue `followup` via a second `run_when_ready`-style send using `submit_bytes(&followup)` after a short delay, then `capture --plain` check and resend Enter when the line is still sitting on the prompt (the swallowed-Enter rule from `skills/tty7/SKILL.md`). Starters: pane right-click `Send starter: <label>` rows plus palette entries, each sending `submit_bytes(&starter.prompt)`. The starters submenu is headed with the pane's role via `role_display(view.role(), view.agent())` — this is `role_display`'s first production caller (it ships tested-but-uncalled from Task 3); a pane with no role shows no header, only the starter rows.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --bin tty7-app agent_launch -v`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/ui/agent_launch.rs src/ui/app.rs src/core/agent_prompt.rs
git commit -m "feat: deliver role instructions and starters"
```

### Task 5: Settings Agents page

**Files:**
- Create: `src/ui/settings/agent_roles.rs`
- Modify: `src/ui/settings.rs` (new `SettingsSection::AgentRoles`, `ALL`, title, icon, profile label)
- Modify: `src/ui/settings/shell.rs` (dispatch + search wiring)
- Modify: `src/ui/i18n/mod.rs`, `en.rs`, `zh.rs`, `ja.rs`
- Test: extend `agent_roles` core tests + GUI smoke test following `settings/agents.rs` patterns

**Interfaces:**
- Consumes: Task 1 (`load_roles`, `slug_from_name`, `slug_valid`).
- Produces: Settings page reading/writing `roles/<slug>/role.json`; no other task depends on it.

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn slug_from_name_is_stable_and_unique() {
    assert_eq!(slug_from_name("Frontend Reviewer"), "frontend-reviewer");
    assert_eq!(slug_from_name("  !!  "), "role");
}
```

Uniqueness (`-2` suffix) is implemented in the Settings layer against `load_roles` output; cover with a second test:

```rust
#[test]
fn duplicate_names_get_numeric_suffix() {
    let existing = vec!["frontend-reviewer".to_string()];
    assert_eq!(unique_slug(&existing, "frontend-reviewer"), "frontend-reviewer-2");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p tty7-core core::agent_roles -v`
Expected: FAIL with `unique_slug` not found (implement it in `agent_roles.rs`).

- [ ] **Step 3: Write minimal implementation**

```rust
pub fn unique_slug(existing: &[String], want: &str) -> String {
    if !existing.iter().any(|s| s == want) {
        return want.to_string();
    }
    for n in 2..1000 {
        let cand = format!("{want}-{n}");
        if !existing.iter().any(|s| s == &cand) {
            return cand;
        }
    }
    format!("{want}-new")
}
```

Settings page (`src/ui/settings/agent_roles.rs`): list rows (`name + base + description`), `New role` form, edit form with the 7 approved fields (`slug` read-only after creation, `base` dropdown of 26, `launch`, `description`, `instructions`, 3 `label + prompt` pairs), delete removing `roles/<slug>/`. Register the section in `src/ui/settings.rs`:

```rust
AgentRoles,
// ALL: [..., SettingsSection::Agents, SettingsSection::AgentRoles, ...]
SettingsSection::AgentRoles => L10nKey::SettingsNavAgentRoles,
SettingsSection::AgentRoles => "icons/settings/agents.svg",
SettingsSection::AgentRoles => "settings:agent-roles",
```

and dispatch in `src/ui/settings/shell.rs` next to `SettingsSection::Agents => self.render_settings_agents(cx)`:

```rust
SettingsSection::AgentRoles => self.render_settings_agent_roles(cx),
```

Add `L10nKey::SettingsNavAgentRoles` plus list/form keys to all three locales.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p tty7-core core::agent_roles -v`
Expected: PASS. Then: `cargo test --bin tty7-app settings -v`
Expected: PASS (no regressions in existing settings suites).

- [ ] **Step 5: Commit**

```bash
git add src/ui/settings/agent_roles.rs src/ui/settings.rs src/ui/settings/shell.rs src/ui/i18n/ crates/tty7-core/src/core/agent_roles.rs
git commit -m "feat: settings Agents page for custom roles"
```

### Task 6: Docs and final verification

**Files:**
- Modify: `docs/agents/overview.mdx`
- Modify: `docs/reference/configuration.mdx`
- Test: full suite

**Interfaces:**
- Consumes: Tasks 1–5 user-visible behavior.
- Produces: documented Roles feature; no code interface.

- [ ] **Step 1: Write the docs check**

Add a `Roles` section to `docs/agents/overview.mdx` after `Your own wrapper`: fields, `roles/<slug>/role.json` location, `Role:` palette rows, `LaunchRole:<slug>` binding, `instructions`/`starters` behavior, remote caveat (per-machine definitions, remote needs the same CLI installed). Add a `roles/` note to `docs/reference/configuration.mdx` Agents table.

- [ ] **Step 2: Run the verification suite**

Run: `cargo test -p tty7-core core::agent_roles core::cli_agent -v`
Expected: PASS.

Run: `cargo test --bin tty7-app agent_launch settings -v`
Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add docs/agents/overview.mdx docs/reference/configuration.mdx
git commit -m "docs: custom agent roles usage and remote caveat"
```

---

### Task 7: Model selection (added post-V1, user-approved 2026-10-03)

**Files:**
- Modify: `crates/tty7-core/src/core/agent_roles.rs` (`model` field, `models.json` read, fetchers, `model_flag`, `model_choices`, `role_launch_argv`)
- Modify: `src/ui/agent_launch.rs` (argv via `role_launch_argv`, model append test)
- Modify: `src/ui/settings/agent_roles.rs` (form `model`/`models`, base-change reload, refresh, save, model dropdown row)
- Modify: `src/ui/i18n/mod.rs`, `en.rs`, `zh.rs`, `ja.rs` (`SettingsRoleModel[Desc]`, `SettingsRoleModelNone`, `SettingsRoleRefresh`)
- Modify: spec §6 + `docs/agents/overview.mdx` roles section (model line)

**Interfaces:**
- Consumes: Tasks 1–5 (`AgentRole`, `role_launch_line`, resume replay, form pattern).
- Produces: `role_launch_argv(role)`, `model_choices(base)`, `model_flag(base)`; no other task depends on it.

**Steps:** core field + sources + tests → GUI form + launch wiring → i18n → suites → commit.
