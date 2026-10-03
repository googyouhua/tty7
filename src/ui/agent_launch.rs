//! Quick launch for the coding agents this machine can run (#955).
//!
//! Launching an agent never types into a pane that already exists. It opens a
//! new shell — a tab in the active tab's directory, or a split when the entry
//! point was asked for one — and types the agent's command line into *that*
//! shell once it is there. The agent then runs the way a hand-typed one does:
//! the daemon detects it from the pane's foreground, status and resume work as
//! usual, and quitting it leaves the shell behind.
//!
//! Which agents are offered: on this computer, those whose launch program is
//! on `PATH`. A remote workspace's `PATH` is not something the `Host` trait can
//! answer — it has no way to run a command or read the far shell's
//! environment — so there the list is the agents seen running in that
//! workspace before (`WindowView::seen_agents`). Either way the order is
//! frecency, recorded the way saved SSH hosts' is.

use std::collections::HashMap;
use std::sync::LazyLock;

use gpui::{App, Axis, Context, Entity, Window};
use gpui_component::WindowExt as _;

use crate::core::agent_roles::{AgentRole, role_launch_program};
use crate::core::cli_agent::{CLIAgent, launch_program, program_on_path};
use crate::core::config::{Config, ProfileUsage, unix_now};
use crate::core::session::WorkspaceStore;
use crate::ui::app::{SpawnAs, SpawnWhere, Tty7App, join_shell_args};
use crate::ui::i18n::{L10nKey, t_fmt};
use crate::ui::pane::PaneSlot;

/// How the keymap spells "launch this agent": `LaunchAgent:claude`.
const LAUNCH_ACTION_PREFIX: &str = "LaunchAgent:";

/// One keymap action name per agent, in [`CLIAgent::ALL`] order.
pub(crate) fn launch_action_names() -> &'static [(CLIAgent, String)] {
    static NAMES: LazyLock<Vec<(CLIAgent, String)>> = LazyLock::new(|| {
        CLIAgent::ALL
            .into_iter()
            .map(|agent| (agent, format!("{LAUNCH_ACTION_PREFIX}{}", agent.slug())))
            .collect()
    });
    &NAMES
}

pub(crate) fn launch_action_name(agent: CLIAgent) -> &'static str {
    launch_action_names()
        .iter()
        .find(|(a, _)| *a == agent)
        .map(|(_, name)| name.as_str())
        .expect("every agent has a launch action")
}

pub(crate) fn agent_for_launch_action(action: &str) -> Option<CLIAgent> {
    action
        .strip_prefix(LAUNCH_ACTION_PREFIX)
        .and_then(CLIAgent::from_slug)
}

/// How the keymap spells "launch this role": `LaunchRole:<slug>`.
pub(crate) const LAUNCH_ROLE_PREFIX: &str = "LaunchRole:";

/// The keymap action name that launches the role `slug` belongs to.
pub(crate) fn role_action_name(slug: &str) -> String {
    format!("{LAUNCH_ROLE_PREFIX}{slug}")
}

pub(crate) fn role_for_launch_action(action: &str, roles: &[AgentRole]) -> Option<AgentRole> {
    let slug = action.strip_prefix(LAUNCH_ROLE_PREFIX)?;
    roles
        .iter()
        .find(|r| r.slug.eq_ignore_ascii_case(slug.trim()))
        .cloned()
}

pub(crate) fn offered_roles(
    roles: Vec<AgentRole>,
    path: &std::ffi::OsStr,
    seen_bases: Option<&[String]>,
) -> Vec<AgentRole> {
    roles
        .into_iter()
        .filter(|r| match seen_bases {
            Some(seen) => seen.iter().any(|s| s.eq_ignore_ascii_case(r.base.slug())),
            None => role_launch_program(r)
                .is_some_and(|p| crate::core::cli_agent::program_on_path(&p, path)),
        })
        .collect()
}

pub(crate) fn role_launch_line(role: &AgentRole) -> String {
    role.launch.trim().to_string()
}

pub(crate) fn role_resume_line(role: &AgentRole, session_id: &str) -> Option<String> {
    let argv: Vec<String> = role.launch.split_whitespace().map(str::to_string).collect();
    role.base.resume_command(session_id, Some(&argv))
}

/// How a role's instructions reach the agent on a fresh launch.
pub(crate) enum RoleFirstSend {
    /// The base takes an initial prompt argument (Claude, Codex, Gemini):
    /// the whole command line as shell words, instructions last.
    PromptArg(Vec<String>),
    /// Anything else: type the launch line, then paste the instructions
    /// into the agent once it is up.
    TwoPhase { launch: String, followup: String },
}

pub(crate) fn plan_role_first_send(role: &AgentRole) -> Option<RoleFirstSend> {
    let text = role.instructions.trim();
    if text.is_empty() {
        return None;
    }
    if let Some(mut argv) = role.base.prompt_args(text) {
        // Words, not the whole line: joining quotes each word, so the
        // launch flags stay flags and only the prompt gets quoted.
        let mut full: Vec<String> = role
            .launch
            .split_whitespace()
            .map(str::to_string)
            .collect();
        full.append(&mut argv);
        return Some(RoleFirstSend::PromptArg(full));
    }
    Some(RoleFirstSend::TwoPhase {
        launch: role_launch_line(role),
        followup: text.to_string(),
    })
}

/// How long a fresh agent gets to reach its input before the role's
/// instructions are pasted in. Best-effort (V1): anything earlier risks
/// typing into a TUI that is still booting; the pane-side guard below is
/// what keeps a late agent from eating the paste silently.
const ROLE_FOLLOWUP_DELAY: std::time::Duration = std::time::Duration::from_secs(6);

/// How many times a missed followup is retried before the pane is told it
/// never landed: one retry, so a slow agent gets a second chance and a dead
/// pane gets told rather than retried forever.
const ROLE_FOLLOWUP_ATTEMPTS: usize = 2;

/// Paste `followup` into the agent running in `view`, after
/// [`ROLE_FOLLOWUP_DELAY`]. Each attempt checks the pane is actually running
/// an agent first — pasting instructions into a bare shell would execute
/// them as commands — and a pane that never gets there is told so, with a
/// pointer at the starters menu that sends them by hand.
pub(crate) fn send_role_followup_later(
    view: Entity<crate::terminal::view::TerminalView>,
    followup: String,
    window: &mut Window,
    cx: &mut Context<Tty7App>,
) {
    let missed = crate::ui::i18n::t(L10nKey::SettingsRoleFollowupMissed).to_string();
    let view = view.downgrade();
    cx.spawn_in(window, async move |_this, cx| {
        for _ in 0..ROLE_FOLLOWUP_ATTEMPTS {
            cx.background_executor().timer(ROLE_FOLLOWUP_DELAY).await;
            let sent = view
                .update_in(cx, |view, _, _| {
                    if view.agent().is_none() {
                        return false;
                    }
                    view.send_agent_prompt(&followup);
                    true
                })
                .unwrap_or(false);
            if sent {
                return;
            }
        }
        log::warn!("role instructions never reached a running agent");
        crate::terminal::notify_desktop(Some("tty7"), &missed);
    })
    .detach();
}

/// One keymap action name per role on disk, in slug order
/// (`LaunchRole:<slug>`) — the slots `default_bindings` offers beside the
/// agents', so a role takes a key the same way any other action does.
///
/// The binding table holds `&'static str`, so the names are cached as leaked
/// strings: the set is re-read on every call and only re-leaked when it
/// actually changed, which is how a role added or removed while the app runs
/// shows up after the next rebuild.
pub(crate) fn launch_role_action_names() -> Vec<&'static str> {
    static SLOTS: std::sync::LazyLock<std::sync::Mutex<Vec<&'static str>>> =
        std::sync::LazyLock::new(|| std::sync::Mutex::new(Vec::new()));
    let fresh: Vec<String> = crate::core::agent_roles::roles_dir()
        .map(|dir| crate::core::agent_roles::load_roles(&dir))
        .unwrap_or_default()
        .into_iter()
        .map(|role| role_action_name(&role.slug))
        .collect();
    let mut slots = SLOTS.lock().expect("role action slots");
    let same = slots.len() == fresh.len() && slots.iter().zip(fresh.iter()).all(|(a, b)| *a == b);
    if !same {
        *slots = fresh
            .into_iter()
            .map(|name| &*Box::leak(name.into_boxed_str()) as &str)
            .collect();
    }
    slots.clone()
}

/// The role `slug` names, if it is still on disk. Keybindings and palette
/// rows outlive the files they were built from, so both dispatch paths
/// resolve through here and degrade to a warning when the role is gone.
pub(crate) fn find_role(slug: &str) -> Option<AgentRole> {
    let dir = crate::core::agent_roles::roles_dir()?;
    let roles = crate::core::agent_roles::load_roles(&dir);
    role_for_launch_action(&role_action_name(slug), &roles)
}

/// The `agent_frecency` key a role sorts and stamps under. Agent slugs never
/// contain a colon, so the two families share the map without colliding.
pub(crate) fn role_frecency_key(slug: &str) -> String {
    format!("role:{slug}")
}

/// The agents whose launch program is on `path`, in [`CLIAgent::ALL`] order.
/// An agent with an `agent_launch` override is looked up by the override's
/// program, since that is what the launch will run.
pub(crate) fn installed_on(
    path: &std::ffi::OsStr,
    overrides: &HashMap<String, String>,
) -> Vec<CLIAgent> {
    CLIAgent::ALL
        .into_iter()
        .filter(|agent| {
            launch_program(&agent.launch_command(overrides))
                .is_some_and(|program| program_on_path(&program, path))
        })
        .collect()
}

/// `agents` most-used-first by frecency score; agents with the same score
/// keep the order they came in.
pub(crate) fn by_frecency(
    agents: impl IntoIterator<Item = CLIAgent>,
    usage: &HashMap<String, ProfileUsage>,
    now: u64,
) -> Vec<CLIAgent> {
    let score = |agent: &CLIAgent| usage.get(agent.slug()).map(|u| u.score(now)).unwrap_or(0.0);
    let mut agents: Vec<CLIAgent> = agents.into_iter().collect();
    agents.sort_by(|a, b| {
        score(b)
            .partial_cmp(&score(a))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    agents
}

/// The agent "New Agent Tab" opens: of `offered`, the one launched or seen
/// most recently; the first offered when none has been used yet.
pub(crate) fn most_recent(
    offered: &[CLIAgent],
    usage: &HashMap<String, ProfileUsage>,
) -> Option<CLIAgent> {
    let last_used = |agent: &CLIAgent| usage.get(agent.slug()).map_or(0, |u| u.last_used);
    offered
        .iter()
        .copied()
        .filter(|agent| last_used(agent) > 0)
        // `max_by_key` keeps the last of equals; reversed, that is the first.
        .rev()
        .max_by_key(last_used)
        .or_else(|| offered.first().copied())
}

/// The `agent_launch` line "Set Current Launch Args as Default" writes for a
/// pane that `agent` is running in with `argv`.
pub(crate) fn launch_line_to_save(agent: CLIAgent, argv: &[String]) -> Option<String> {
    agent
        .launch_argv_for_default(argv)
        .map(|argv| join_shell_args(&argv))
}

/// Make `line` the command `agent` is launched with from now on.
pub(crate) fn remember_launch_line(cfg: &mut Config, agent: CLIAgent, line: String) {
    // One entry per agent, whatever case an existing key was written in.
    cfg.agent_launch
        .retain(|slug, _| CLIAgent::from_slug(slug) != Some(agent));
    cfg.agent_launch.insert(agent.slug().to_string(), line);
}

/// The line that resumes `agent`'s session `session_id`, carrying the flags
/// `agent_launch` gives the agent: a session started with
/// `--dangerously-skip-permissions` comes back with it.
pub(crate) fn resume_line(
    agent: CLIAgent,
    session_id: &str,
    overrides: &HashMap<String, String>,
) -> Option<String> {
    let launch = agent.launch_command(overrides);
    let argv: Vec<String> = launch.split_whitespace().map(str::to_string).collect();
    agent.resume_command(session_id, Some(&argv))
}

/// The line that forks `agent`'s session `session_id` into a new one, with
/// the same flags [`resume_line`] carries. `None` for an agent that cannot
/// fork.
pub(crate) fn fork_line(
    agent: CLIAgent,
    session_id: &str,
    overrides: &HashMap<String, String>,
) -> Option<String> {
    let launch = agent.launch_command(overrides);
    let argv: Vec<String> = launch.split_whitespace().map(str::to_string).collect();
    agent.fork_command(session_id, Some(&argv))
}

/// How long a new shell is given to reach its first prompt before a queued
/// command is typed anyway.
const PROMPT_WAIT: std::time::Duration = std::time::Duration::from_secs(3);
const PROMPT_POLL: std::time::Duration = std::time::Duration::from_millis(50);

/// Type `command` into the shell `slot` holds — now if it is up, or the
/// moment it lands if it is still connecting.
///
/// A shell that is up is not yet at its prompt: typed straight in, the line
/// was echoed by the tty above everything the shell prints while it starts (a
/// banner, `fastfetch`) and read only afterwards. It waits for the shell's
/// first prompt instead — up to [`PROMPT_WAIT`], for a shell with no
/// integration to say when that is.
pub(crate) fn run_when_ready(slot: &PaneSlot, command: String, cx: &mut App) {
    match slot {
        PaneSlot::Ready(view) => {
            let view = view.downgrade();
            cx.spawn(async move |cx| {
                let deadline = std::time::Instant::now() + PROMPT_WAIT;
                loop {
                    let ready = view
                        .read_with(cx, |view, _| view.terminal.at_prompt())
                        .unwrap_or(true);
                    if ready || std::time::Instant::now() >= deadline {
                        break;
                    }
                    cx.background_executor().timer(PROMPT_POLL).await;
                }
                let _ = view.read_with(cx, |view, _| view.run_command_line(&command));
            })
            .detach();
        }
        PaneSlot::Connecting(pending) => {
            pending.update(cx, |pending, _| pending.spawn.run_on_land = Some(command));
        }
    }
}

impl Tty7App {
    /// The agents this window's quick launch offers, most likely first.
    pub(crate) fn offered_agents(&self, cx: &App) -> Vec<CLIAgent> {
        let cfg = cx.global::<Config>();
        let candidates = match WorkspaceStore::all(cx)
            .get(self.workspace)
            .filter(|view| view.is_remote())
        {
            Some(view) => view
                .seen_agents
                .iter()
                .filter_map(|slug| CLIAgent::from_slug(slug))
                .collect(),
            None => installed_on(
                &std::env::var_os("PATH").unwrap_or_default(),
                &cfg.agent_launch,
            ),
        };
        by_frecency(candidates, &cfg.agent_frecency, unix_now())
    }

    /// The roles this window's quick launch offers, most-used-first — the
    /// mirror of [`Self::offered_agents`]. On this computer that is the roles
    /// whose launch program is on `PATH`; on a remote workspace, whose `PATH`
    /// cannot be asked, the roles whose base agent was seen running there.
    pub(crate) fn offered_roles_here(&self, cx: &App) -> Vec<AgentRole> {
        let Some(dir) = crate::core::agent_roles::roles_dir() else {
            return Vec::new();
        };
        let loaded = crate::core::agent_roles::load_roles(&dir);
        let candidates = match WorkspaceStore::all(cx)
            .get(self.workspace)
            .filter(|view| view.is_remote())
        {
            Some(view) => offered_roles(loaded, "".as_ref(), Some(&view.seen_agents)),
            None => offered_roles(loaded, &std::env::var_os("PATH").unwrap_or_default(), None),
        };
        let cfg = cx.global::<Config>();
        let now = unix_now();
        let score = |role: &AgentRole| {
            cfg.agent_frecency
                .get(&role_frecency_key(&role.slug))
                .map(|u| u.score(now))
                .unwrap_or(0.0)
        };
        let mut candidates = candidates;
        candidates.sort_by(|a, b| {
            score(b)
                .partial_cmp(&score(a))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        candidates
    }

    /// Open a new shell for `role` and start its launch line there — the
    /// mechanical half of launching. Instructions, starters and session
    /// resume belong to the role launcher this dispatches for.
    pub(crate) fn launch_role(
        &mut self,
        role: AgentRole,
        at: SpawnWhere,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // Instructions ride the launch when the base takes a prompt
        // argument, and follow a few seconds later otherwise; no
        // instructions means the bare launch line.
        let first_send = plan_role_first_send(&role);
        let command = match &first_send {
            None => role_launch_line(&role),
            Some(RoleFirstSend::PromptArg(parts)) => join_shell_args(parts),
            Some(RoleFirstSend::TwoPhase { launch, .. }) => launch.clone(),
        };
        let slot = match at {
            SpawnWhere::NewTab => {
                let cwd = self.tabs.get(self.active).and_then(|t| {
                    t.pane
                        .focused_or_first(window, cx)
                        .and_then(|leaf| leaf.read(cx).spawnable_cwd())
                });
                self.new_tab_slot(cwd, None, window, cx)
            }
            SpawnWhere::Split => {
                self.split_slot(Axis::Horizontal, Some(SpawnAs::Shell(None)), window, cx)
            }
        };
        // Same failure mode as [`Self::launch_agent`]: nowhere to type into,
        // and the reason is already on screen.
        let Some(slot) = slot else {
            log::warn!("no pane opened for role {}; not launching it", role.slug);
            return;
        };
        // Tag the pane with the role: the slug for display and restore, the
        // base agent for detection-independent bookkeeping, and the launch
        // argv the resume derives its flags from. A pane that is still
        // connecting carries it on its spawn; one already up (a local shell)
        // is detected as its base agent from here on — the live tag belongs
        // to the display-side work, not this spawn metadata.
        if let PaneSlot::Connecting(pending) = &slot {
            let argv: Vec<String> =
                role.launch.split_whitespace().map(str::to_string).collect();
            let (slug, base) = (role.slug.clone(), role.base);
            pending.update(cx, |pending, _| {
                pending.spawn.role = Some(slug);
                pending.spawn.agent = Some(base);
                pending.spawn.agent_launch_argv = Some(argv);
            });
        }
        // The delayed paste only fits a pane that is already up: one still
        // connecting gets its followup at the arrival transition instead,
        // which recomputes the same plan from the role file.
        match (&slot, first_send) {
            (PaneSlot::Ready(view), Some(RoleFirstSend::TwoPhase { followup, .. })) => {
                run_when_ready(&slot, command, cx);
                send_role_followup_later(view.clone(), followup, window, cx);
            }
            _ => run_when_ready(&slot, command, cx),
        }
        // Recency only, under the role's own key — see `role_frecency_key`.
        self.update_config(cx, |cfg| {
            cfg.agent_frecency
                .entry(role_frecency_key(&role.slug))
                .or_default()
                .last_used = unix_now();
        });
    }

    /// Open a new shell for `agent` and start it there.
    pub(crate) fn launch_agent(
        &mut self,
        agent: CLIAgent,
        at: SpawnWhere,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let command = agent.launch_command(&cx.global::<Config>().agent_launch);
        let slot = match at {
            SpawnWhere::NewTab => {
                let cwd = self.tabs.get(self.active).and_then(|t| {
                    t.pane
                        .focused_or_first(window, cx)
                        .and_then(|leaf| leaf.read(cx).spawnable_cwd())
                });
                self.new_tab_slot(cwd, None, window, cx)
            }
            SpawnWhere::Split => {
                self.split_slot(Axis::Horizontal, Some(SpawnAs::Shell(None)), window, cx)
            }
        };
        // Nothing opened (the spawn failed, or this workspace cannot host a
        // shell right now), and the reason is already on screen. The command
        // goes nowhere rather than into whatever pane was focused before.
        let Some(slot) = slot else {
            log::warn!("no pane opened for {}; not launching it", agent.slug());
            return;
        };
        run_when_ready(&slot, command, cx);
        // Recency only: the run is counted when the pane reports the agent
        // actually running, so a launch and its detection are one use, not two.
        self.update_config(cx, |cfg| {
            cfg.agent_frecency
                .entry(agent.slug().to_string())
                .or_default()
                .last_used = unix_now();
        });
    }

    /// Reopen `agent`'s session `session_id` in a new tab in `cwd`, the
    /// directory it ran in — an agent keys its history by directory, and
    /// `--resume` elsewhere finds nothing. The resume carries the flags the
    /// agent is configured to launch with (`agent_launch`), so a session
    /// started with them comes back with them.
    ///
    /// With `fork`, the session is branched instead: the agent starts a new
    /// one from its history and the original is left as it was.
    pub(crate) fn resume_session(
        &mut self,
        agent: CLIAgent,
        session_id: &str,
        cwd: Option<std::path::PathBuf>,
        fork: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let launch = &cx.global::<Config>().agent_launch;
        let command = match fork {
            true => fork_line(agent, session_id, launch),
            false => resume_line(agent, session_id, launch),
        };
        let Some(command) = command else {
            window.push_notification(
                t_fmt(
                    L10nKey::AppSessionNotResumable,
                    &[("name", agent.display_name())],
                ),
                cx,
            );
            return;
        };
        // A directory that is gone cannot hold the session either; say so
        // rather than resume into a history the agent will not find. Only
        // asked of this machine: a remote session's directory is not here.
        if self.spawn_host(cx).is_local()
            && let Some(dir) = cwd.as_ref().filter(|dir| !dir.is_dir())
        {
            window.push_notification(
                t_fmt(
                    L10nKey::AppSessionDirectoryGone,
                    &[("path", &dir.display().to_string())],
                ),
                cx,
            );
            return;
        }
        let Some(slot) = self.new_tab_slot(cwd, None, window, cx) else {
            log::warn!("no pane opened to resume {} {session_id}", agent.slug());
            return;
        };
        run_when_ready(&slot, command, cx);
    }

    /// "New Agent Tab": launch the agent used most recently.
    pub(crate) fn new_agent_tab(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let offered = self.offered_agents(cx);
        match most_recent(&offered, &cx.global::<Config>().agent_frecency) {
            Some(agent) => self.launch_agent(agent, SpawnWhere::NewTab, window, cx),
            None => {
                let remote = WorkspaceStore::remote_ref(cx, self.workspace).is_some();
                window.push_notification(
                    crate::ui::i18n::t(if remote {
                        L10nKey::AppNoAgentSeenHere
                    } else {
                        L10nKey::AppNoAgentOnPath
                    }),
                    cx,
                );
            }
        }
    }

    /// A pane in this window started running `agent`.
    pub(crate) fn note_agent_detected(&mut self, agent: CLIAgent, cx: &mut Context<Self>) {
        self.update_config(cx, |cfg| {
            let entry = cfg
                .agent_frecency
                .entry(agent.slug().to_string())
                .or_default();
            entry.count = entry.count.saturating_add(1);
            entry.last_used = unix_now();
        });
        if WorkspaceStore::remote_ref(cx, self.workspace).is_some() {
            WorkspaceStore::record_agent_seen(cx, self.workspace, agent);
        }
    }

    /// "Set Current Launch Args as Default": make the focused pane's agent
    /// launch the way this one was started.
    pub(crate) fn save_agent_launch_args(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(view) = self
            .tabs
            .get(self.active)
            .and_then(|t| t.pane.focused_or_first(window, cx))
        else {
            return;
        };
        let view = view.read(cx);
        let Some(agent) = view.agent() else {
            window.push_notification(crate::ui::i18n::t(L10nKey::AppPaneNoCodingAgent), cx);
            return;
        };
        let name = agent.display_name();
        let Some(line) = view
            .agent_session()
            .and_then(|s| s.launch_argv)
            .and_then(|argv| launch_line_to_save(agent, &argv))
        else {
            window.push_notification(
                t_fmt(L10nKey::AppAgentLaunchArgsUnknown, &[("name", name)]),
                cx,
            );
            return;
        };
        let saved = line.clone();
        self.update_config(cx, |cfg| remember_launch_line(cfg, agent, saved));
        window.push_notification(
            t_fmt(
                L10nKey::AppAgentLaunchSaved,
                &[("name", name), ("command", &line)],
            ),
            cx,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn usage(pairs: &[(&str, u32, u64)]) -> HashMap<String, ProfileUsage> {
        pairs
            .iter()
            .map(|(slug, count, last_used)| {
                (
                    slug.to_string(),
                    ProfileUsage {
                        count: *count,
                        last_used: *last_used,
                    },
                )
            })
            .collect()
    }

    fn argv(parts: &[&str]) -> Vec<String> {
        parts.iter().map(|s| s.to_string()).collect()
    }

    const NOW: u64 = 1_800_000_000;
    const DAY: u64 = 86_400;

    #[test]
    fn agents_are_offered_most_used_first() {
        let used = usage(&[
            ("codex", 3, NOW - DAY),
            ("claude", 12, NOW - 2 * DAY),
            // Used a lot, but so long ago that it has decayed below Codex.
            ("gemini", 20, NOW - 200 * DAY),
        ]);
        let offered = by_frecency(
            [
                CLIAgent::Aider,
                CLIAgent::Gemini,
                CLIAgent::Codex,
                CLIAgent::Claude,
                CLIAgent::Amp,
            ],
            &used,
            NOW,
        );
        assert_eq!(
            offered,
            vec![
                CLIAgent::Claude,
                CLIAgent::Codex,
                CLIAgent::Gemini,
                // Never used: they keep the order they were found in.
                CLIAgent::Aider,
                CLIAgent::Amp,
            ]
        );
    }

    #[test]
    fn new_agent_tab_opens_the_most_recently_used_agent() {
        let offered = [CLIAgent::Claude, CLIAgent::Codex, CLIAgent::Gemini];
        // Claude is used more, but Codex was the last one run or launched.
        let used = usage(&[
            ("claude", 40, NOW - DAY),
            ("codex", 1, NOW - 60),
            ("aider", 5, NOW),
        ]);
        assert_eq!(most_recent(&offered, &used), Some(CLIAgent::Codex));
        // Aider was more recent still, but it is not offered here.
        // Nothing used yet: the first one offered.
        assert_eq!(
            most_recent(&offered, &HashMap::new()),
            Some(CLIAgent::Claude)
        );
        assert_eq!(most_recent(&[], &used), None);
        // A launch that only stamped recency counts as the most recent.
        let launched = usage(&[("claude", 40, NOW - DAY), ("gemini", 0, NOW)]);
        assert_eq!(most_recent(&offered, &launched), Some(CLIAgent::Gemini));
    }

    #[test]
    fn saved_launch_args_survive_a_round_trip_through_the_command_line() {
        let run = argv(&[
            "/opt/homebrew/bin/claude",
            "--append-system-prompt",
            "be terse, don't guess",
            "--add-dir",
            "/Users/me/My Projects",
            "--model",
            "opus",
            "--resume",
            "abc-123",
        ]);
        let line = launch_line_to_save(CLIAgent::Claude, &run).unwrap();
        assert_eq!(
            crate::ui::app::split_shell_args(&line).unwrap(),
            argv(&[
                "claude",
                "--append-system-prompt",
                "be terse, don't guess",
                "--add-dir",
                "/Users/me/My Projects",
                "--model",
                "opus",
            ])
        );
        // And the saved line is recognised as the agent it launches.
        assert_eq!(
            CLIAgent::detect_from_command_with(&line, &HashMap::new()),
            Some(CLIAgent::Claude)
        );
    }

    #[test]
    fn set_current_launch_args_as_default_writes_the_agent_launch_entry() {
        let mut cfg = Config::default();
        cfg.agent_launch
            .insert("Claude".to_string(), "claude --old".to_string());
        cfg.agent_launch
            .insert("codex".to_string(), "codex --model o3".to_string());
        let line = launch_line_to_save(
            CLIAgent::Claude,
            &argv(&["claude", "--dangerously-skip-permissions", "--continue"]),
        )
        .unwrap();
        remember_launch_line(&mut cfg, CLIAgent::Claude, line);

        let json: serde_json::Value = serde_json::to_value(&cfg.0).unwrap();
        assert_eq!(
            json["agent_launch"],
            serde_json::json!({
                "claude": "claude --dangerously-skip-permissions",
                "codex": "codex --model o3",
            })
        );
        assert_eq!(
            CLIAgent::Claude.launch_command(&cfg.agent_launch),
            "claude --dangerously-skip-permissions"
        );
    }

    #[test]
    fn role_action_round_trips_by_slug() {
        let role = AgentRole {
            slug: "frontend-reviewer".into(),
            name: "Frontend Reviewer".into(),
            base: CLIAgent::Claude,
            description: String::new(),
            launch: "claude --model opus".into(),
            instructions: String::new(),
            starters: vec![],
        };
        let action = format!("{LAUNCH_ROLE_PREFIX}{}", role.slug);
        assert_eq!(
            role_for_launch_action(&action, &[role.clone()]).map(|r| r.slug),
            Some("frontend-reviewer".to_string())
        );
        assert!(role_for_launch_action("LaunchRole:nobody", &[role]).is_none());
    }

    #[test]
    fn instructions_use_prompt_arg_where_supported() {
        let role = AgentRole {
            slug: "r".into(),
            name: "R".into(),
            base: CLIAgent::Claude,
            description: String::new(),
            launch: "claude --model opus".into(),
            instructions: "Be terse.".into(),
            starters: vec![],
        };
        // Flags stay words of their own; only the prompt may be quoted.
        match plan_role_first_send(&role) {
            Some(RoleFirstSend::PromptArg(parts)) => assert_eq!(
                parts,
                vec!["claude", "--model", "opus", "Be terse."]
            ),
            _ => panic!("expected PromptArg"),
        };
        let opencode = AgentRole {
            base: CLIAgent::OpenCode,
            launch: "opencode".into(),
            ..role
        };
        assert!(matches!(
            plan_role_first_send(&opencode),
            Some(RoleFirstSend::TwoPhase { .. })
        ));
        let silent = AgentRole {
            instructions: "   ".into(),
            ..opencode
        };
        assert!(plan_role_first_send(&silent).is_none());
    }

    #[test]
    fn only_roles_whose_launch_program_is_on_path_are_offered() {
        let dir = tempfile::TempDir::new().unwrap();
        let bin = dir.path().join("cc");
        std::fs::write(&bin, "#!/bin/sh\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        let role = |slug: &str, base: CLIAgent, launch: &str| AgentRole {
            slug: slug.into(),
            name: slug.into(),
            base,
            description: String::new(),
            launch: launch.into(),
            instructions: String::new(),
            starters: vec![],
        };
        let roles = vec![
            role("good", CLIAgent::Claude, "cc --fast"),
            role("missing", CLIAgent::Claude, "nope --fast"),
        ];
        let offered = offered_roles(roles, dir.path().as_os_str(), None);
        assert_eq!(
            offered.iter().map(|r| r.slug.as_str()).collect::<Vec<_>>(),
            vec!["good"]
        );
        // A remote workspace cannot answer for its PATH: the roles whose
        // base agent was seen there are offered instead.
        let roles = vec![
            role("seen", CLIAgent::Codex, "nope"),
            role("unseen", CLIAgent::Claude, "nope"),
        ];
        let offered = offered_roles(roles, "".as_ref(), Some(&["codex".to_string()]));
        assert_eq!(
            offered.iter().map(|r| r.slug.as_str()).collect::<Vec<_>>(),
            vec!["seen"]
        );
    }

    #[test]
    fn every_agent_has_a_bindable_launch_action() {
        for agent in CLIAgent::ALL {
            let name = launch_action_name(agent);
            assert_eq!(agent_for_launch_action(name), Some(agent), "{name}");
        }
        assert_eq!(agent_for_launch_action("LaunchAgent:nobody"), None);
        assert_eq!(agent_for_launch_action("NewAgentTab"), None);
    }

    #[test]
    fn only_agents_whose_launch_program_is_on_path_are_offered() {
        let dir = tempfile::TempDir::new().unwrap();
        for name in ["codex", "cc"] {
            let path = dir.path().join(name);
            std::fs::write(&path, "#!/bin/sh\n").unwrap();
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt as _;
                std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
            }
        }
        let path = dir.path().as_os_str();
        assert_eq!(installed_on(path, &HashMap::new()), vec![CLIAgent::Codex]);
        // With an override, the override's program is what has to be there.
        let overrides: HashMap<String, String> = [
            ("claude".to_string(), "cc --fast".to_string()),
            ("codex".to_string(), "codex-nightly".to_string()),
        ]
        .into_iter()
        .collect();
        assert_eq!(installed_on(path, &overrides), vec![CLIAgent::Claude]);
    }

    #[test]
    fn a_resume_carries_the_configured_launch_flags() {
        let none = HashMap::new();
        assert_eq!(
            resume_line(CLIAgent::Claude, "abc-1", &none).as_deref(),
            Some("claude --resume abc-1")
        );
        assert_eq!(
            resume_line(CLIAgent::Codex, "0199-x", &none).as_deref(),
            Some("codex resume 0199-x")
        );
        let flagged = HashMap::from([(
            "claude".to_string(),
            "claude --dangerously-skip-permissions".to_string(),
        )]);
        assert_eq!(
            resume_line(CLIAgent::Claude, "abc-1", &flagged).as_deref(),
            Some("claude --dangerously-skip-permissions --resume abc-1")
        );
    }

    #[test]
    fn an_id_that_could_smuggle_a_command_is_not_resumed() {
        let none = HashMap::new();
        assert_eq!(resume_line(CLIAgent::Claude, "x; rm -rf ~", &none), None);
        assert_eq!(resume_line(CLIAgent::Claude, "", &none), None);
    }
}
