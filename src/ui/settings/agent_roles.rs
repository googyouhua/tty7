//! Custom agent roles: the named jobs in `roles/<slug>/role.json`, each bound
//! to one of the built-in agents.

use super::kit::{self, BtnKind, Tk, fs};
use super::*;

use crate::core::agent_roles::{
    AgentRole, RoleStarter, delete_role, load_roles, roles_dir, save_role, slug_from_name,
    unique_slug,
};
use crate::core::cli_agent::CLIAgent;

/// The add/edit form for one role. `None` in `SettingsState::role_form`
/// means the list is showing.
pub(crate) struct AgentRoleForm {
    /// Slug being edited; `None` adds a new role (slug minted on save).
    editing: Option<String>,
    name: Entity<InputState>,
    launch: Entity<InputState>,
    description: Entity<InputState>,
    instructions: Entity<InputState>,
    starter_labels: [Entity<InputState>; 3],
    starter_prompts: [Entity<InputState>; 3],
    base: CLIAgent,
    error: Option<String>,
    _subs: Vec<Subscription>,
}

impl Tty7App {
    /// Every role on this machine, slug order — the list behind this page,
    /// the palette rows and every dispatch path.
    pub(crate) fn load_agent_roles() -> Vec<AgentRole> {
        roles_dir()
            .map(|dir| load_roles(&dir))
            .unwrap_or_default()
    }

    fn role_form_mut(&mut self) -> Option<&mut AgentRoleForm> {
        self.active_settings_mut()
            .and_then(|s| s.role_form.as_mut())
    }

    fn seed_role_input(
        window: &mut Window,
        cx: &mut Context<Self>,
        value: &str,
        subs: &mut Vec<Subscription>,
    ) -> Entity<InputState> {
        let input = super::seed_input(window, cx, value, false);
        subs.push(cx.subscribe_in(&input, window, |this, _i, ev: &InputEvent, _w, cx| {
            if matches!(ev, InputEvent::Change) {
                let _ = this.role_form_mut();
                cx.notify();
            }
        }));
        input
    }

    fn seed_role_form(
        &mut self,
        editing: Option<&AgentRole>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let mut subs = Vec::new();
        let mut seed = |value: &str, subs: &mut Vec<Subscription>| {
            Self::seed_role_input(window, cx, value, subs)
        };
        let name = seed(
            editing.map(|r| r.name.as_str()).unwrap_or(""),
            &mut subs,
        );
        let launch = seed(
            editing.map(|r| r.launch.as_str()).unwrap_or(""),
            &mut subs,
        );
        let description = seed(
            editing.map(|r| r.description.as_str()).unwrap_or(""),
            &mut subs,
        );
        let instructions = seed(
            editing.map(|r| r.instructions.as_str()).unwrap_or(""),
            &mut subs,
        );
        let starter_labels = std::array::from_fn(|i| {
            seed(
                editing
                    .and_then(|r| r.starters.get(i))
                    .map(|s| s.label.as_str())
                    .unwrap_or(""),
                &mut subs,
            )
        });
        let starter_prompts = std::array::from_fn(|i| {
            seed(
                editing
                    .and_then(|r| r.starters.get(i))
                    .map(|s| s.prompt.as_str())
                    .unwrap_or(""),
                &mut subs,
            )
        });
        let form = AgentRoleForm {
            editing: editing.map(|r| r.slug.clone()),
            base: editing.map(|r| r.base).unwrap_or(CLIAgent::Claude),
            name,
            launch,
            description,
            instructions,
            starter_labels,
            starter_prompts,
            error: None,
            _subs: subs,
        };
        if let Some(s) = self.active_settings_mut() {
            s.role_form = Some(form);
        }
        cx.notify();
    }

    /// Open a blank add form.
    pub(crate) fn start_role_add(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.seed_role_form(None, window, cx);
    }

    /// Open the edit form for `slug`.
    pub(crate) fn start_role_edit(
        &mut self,
        slug: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let role = Self::load_agent_roles()
            .into_iter()
            .find(|r| r.slug == slug);
        self.seed_role_form(role.as_ref(), window, cx);
    }

    pub(crate) fn cancel_role_form(&mut self, cx: &mut Context<Self>) {
        if let Some(s) = self.active_settings_mut() {
            s.role_form = None;
        }
        cx.notify();
    }

    pub(crate) fn set_role_base(&mut self, index: usize, cx: &mut Context<Self>) {
        if let Some(base) = CLIAgent::ALL.get(index) {
            if let Some(form) = self.role_form_mut() {
                form.base = *base;
            }
        }
        cx.notify();
    }

    /// Validate the open form and write it to `roles/<slug>/role.json`.
    pub(crate) fn save_role_form(&mut self, cx: &mut Context<Self>) {
        let Some(dir) = roles_dir() else {
            if let Some(form) = self.role_form_mut() {
                form.error = Some("Configuration directory is unavailable.".to_string());
            }
            cx.notify();
            return;
        };
        // Read everything first so the borrow ends before validating.
        let (editing, name, launch, description, instructions, starters, base) = {
            let Some(form) = self.role_form_mut() else {
                return;
            };
            let val = |e: &Entity<InputState>| e.read(cx).value().trim().to_string();
            let starters: Vec<RoleStarter> = form
                .starter_labels
                .iter()
                .zip(form.starter_prompts.iter())
                .filter_map(|(label, prompt)| {
                    let prompt = prompt.read(cx).value().trim().to_string();
                    if prompt.is_empty() {
                        return None;
                    }
                    let label = label.read(cx).value().trim().to_string();
                    let label = match label.is_empty() {
                        true => prompt.chars().take(40).collect(),
                        false => label,
                    };
                    Some(RoleStarter { label, prompt })
                })
                .collect();
            (
                form.editing.clone(),
                val(&form.name),
                val(&form.launch),
                val(&form.description),
                val(&form.instructions),
                starters,
                form.base,
            )
        };
        if name.is_empty() {
            if let Some(form) = self.role_form_mut() {
                form.error = Some(t(L10nKey::SettingsRoleNameRequired).to_string());
            }
            cx.notify();
            return;
        }
        if launch.is_empty() {
            if let Some(form) = self.role_form_mut() {
                form.error = Some(t(L10nKey::SettingsRoleLaunchRequired).to_string());
            }
            cx.notify();
            return;
        }
        let slug = match editing {
            Some(slug) => slug,
            None => {
                let slugs: Vec<String> = Self::load_agent_roles()
                    .into_iter()
                    .map(|r| r.slug)
                    .collect();
                unique_slug(&slugs, &slug_from_name(&name))
            }
        };
        let role = AgentRole {
            slug,
            name,
            base,
            description,
            launch,
            instructions,
            starters,
        };
        if let Err(e) = save_role(&dir, &role) {
            log::warn!("could not save agent role {}: {e}", role.slug);
            if let Some(form) = self.role_form_mut() {
                form.error = Some(e.to_string());
            }
            cx.notify();
            return;
        }
        if let Some(s) = self.active_settings_mut() {
            s.role_form = None;
        }
        cx.notify();
    }

    /// Delete `roles/<slug>/` outright. A failure leaves the row on screen
    /// with the reason on the page, so nothing silently survives.
    pub(crate) fn remove_agent_role(&mut self, slug: &str, cx: &mut Context<Self>) {
        let Some(dir) = roles_dir() else {
            return;
        };
        if let Err(e) = delete_role(&dir, slug) {
            log::warn!("could not delete agent role {slug}: {e}");
            if let Some(s) = self.active_settings_mut() {
                s.save_error = Some(e.to_string());
            }
        }
        cx.notify();
    }

    pub(crate) fn render_settings_agent_roles(&self, cx: &mut Context<Self>) -> AnyElement {
        if self
            .active_settings()
            .is_some_and(|s| s.role_form.is_some())
        {
            return Self::settings_page([self.render_role_form(cx)]);
        }
        let roles = Self::load_agent_roles();
        let mut rows: Vec<AnyElement> = vec![
            h_flex().pb(px(4.)).child(
                kit::button("role-new", t(L10nKey::SettingsRoleNew), BtnKind::Primary).on_click(
                    cx.listener(|this, _ev: &gpui::ClickEvent, window, cx| {
                        this.start_role_add(window, cx);
                    }),
                ),
            ).into_any_element(),
        ];
        if roles.is_empty() {
            let tk = Tk::of(cx);
            rows.push(
                div()
                    .py(px(12.))
                    .text_size(fs(12.5))
                    .text_color(tk.k35)
                    .child(t(L10nKey::SettingsRolesEmpty))
                    .into_any_element(),
            );
        }
        for (i, role) in roles.iter().enumerate() {
            let slug = role.slug.clone();
            let edit = kit::button(
                ("role-edit", i),
                t(L10nKey::SettingsRoleEditAction),
                BtnKind::Secondary,
            )
            .on_click(cx.listener(move |this, _ev: &gpui::ClickEvent, window, cx| {
                this.start_role_edit(&slug, window, cx);
            }));
            let slug = role.slug.clone();
            let delete = kit::button(("role-delete", i), t(L10nKey::Delete), BtnKind::Danger)
                .on_click(cx.listener(move |this, _ev: &gpui::ClickEvent, _window, cx| {
                    this.remove_agent_role(&slug, cx);
                }));
            rows.push(
                self.settings_row(
                    crate::ui::app::role_display(Some(&role.slug), Some(role.base)),
                    match role.description.trim().is_empty() {
                        true => role.launch.clone(),
                        false => role.description.clone(),
                    },
                    h_flex().gap(px(8.)).child(edit).child(delete).into_any_element(),
                    cx,
                )
                .into_any_element(),
            );
        }
        Self::settings_page([self.settings_group(
            Some(t(L10nKey::SettingsRolesIntro)),
            Some(t(L10nKey::SettingsRolesIntroDesc).to_string()),
            rows,
            cx,
        )])
    }

    fn render_role_form(&self, cx: &mut Context<Self>) -> AnyElement {
        let Some(form) = self.active_settings().and_then(|s| s.role_form.as_ref()) else {
            return div().into_any_element();
        };
        let title = match form.editing.is_some() {
            true => t(L10nKey::SettingsRoleEdit),
            false => t(L10nKey::SettingsRoleNew),
        };
        let base_names: Vec<&str> = CLIAgent::ALL
            .iter()
            .map(|a| a.display_name())
            .collect();
        let base_selected = CLIAgent::ALL
            .iter()
            .position(|a| *a == form.base)
            .unwrap_or(0);
        let base_choice = self.settings_choice(
            "role-base",
            &base_names,
            base_selected,
            cx,
            |this, ix, _w, cx| {
                this.set_role_base(ix, cx);
            },
        );
        const W: f32 = 320.;
        let mut rows: Vec<AnyElement> = vec![
            self.settings_row(
                t(L10nKey::SettingsRoleName),
                t(L10nKey::SettingsRoleNameDesc),
                self.settings_text_input(&form.name, W, form.error.is_some(), cx)
                    .into_any_element(),
                cx,
            )
            .into_any_element(),
            self.settings_row(
                t(L10nKey::SettingsRoleBase),
                t(L10nKey::SettingsRoleBaseDesc),
                base_choice,
                cx,
            )
            .into_any_element(),
            self.settings_row(
                t(L10nKey::SettingsRoleLaunch),
                t(L10nKey::SettingsRoleLaunchDesc),
                self.settings_text_input(&form.launch, W, form.error.is_some(), cx)
                    .into_any_element(),
                cx,
            )
            .into_any_element(),
            self.settings_row(
                t(L10nKey::SettingsRoleDescription),
                t(L10nKey::SettingsRoleDescriptionDesc),
                self.settings_text_input(&form.description, W, false, cx)
                    .into_any_element(),
                cx,
            )
            .into_any_element(),
            self.settings_row(
                t(L10nKey::SettingsRoleInstructions),
                t(L10nKey::SettingsRoleInstructionsDesc),
                self.settings_text_input(&form.instructions, W, false, cx)
                    .into_any_element(),
                cx,
            )
            .into_any_element(),
        ];
        for i in 0..3 {
            rows.push(
                self.settings_row(
                    format!("{} {}", t(L10nKey::SettingsRoleStarters), i + 1),
                    t(L10nKey::SettingsRoleStartersDesc),
                    v_flex()
                        .gap(px(6.))
                        .child(self.settings_text_input(&form.starter_labels[i], W, false, cx))
                        .child(self.settings_text_input(&form.starter_prompts[i], W, false, cx))
                        .into_any_element(),
                    cx,
                )
                .into_any_element(),
            );
        }
        if let Some(error) = form.error.as_ref() {
            let tk = Tk::of(cx);
            rows.push(
                div()
                    .text_size(fs(12.))
                    .text_color(tk.danger)
                    .child(error.clone())
                    .into_any_element(),
            );
        }
        rows.push(
            h_flex()
                .gap(px(8.))
                .child(
                    kit::button("role-save", t(L10nKey::Save), BtnKind::Primary).on_click(
                        cx.listener(|this, _ev: &gpui::ClickEvent, _w, cx| {
                            this.save_role_form(cx);
                        }),
                    ),
                )
                .child(
                    kit::button("role-cancel", t(L10nKey::Cancel), BtnKind::Secondary).on_click(
                        cx.listener(|this, _ev: &gpui::ClickEvent, _w, cx| {
                            this.cancel_role_form(cx);
                        }),
                    ),
                )
                .into_any_element(),
        );
        Self::settings_page([self.settings_group(Some(title), None, rows, cx)])
    }
}
