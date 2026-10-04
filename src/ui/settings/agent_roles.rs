//! Custom agent roles: the named jobs in `roles/<slug>/role.json`, each bound
//! to one of the built-in agents.

use super::kit::{self, BtnKind, Tk, fs};
use super::shell::SearchOption;
use super::*;

use crate::core::agent_roles::{
    AgentRole, RoleStarter, delete_role, fetch_live_models, load_roles,
    model_choices_fast, models_dev_providers, read_last_models_source, read_models_cache,
    roles_dir, save_role, slug_from_name, source_from_parts, unique_slug,
    write_last_models_source, MODELS_DEV_URL,
};
use crate::core::cli_agent::CLIAgent;

/// The add/edit form for one role. `None` in `SettingsState::role_form`
/// means the list is showing.
pub(crate) struct AgentRoleForm {
    /// Slug being edited; `None` adds a new role (slug minted on save).
    editing: Option<String>,
    pub(crate) name: Entity<InputState>,
    pub(crate) launch: Entity<InputState>,
    pub(crate) description: Entity<InputState>,
    pub(crate) instructions: Entity<InputState>,
    pub(crate) starter_labels: [Entity<InputState>; 3],
    pub(crate) starter_prompts: [Entity<InputState>; 3],
    base: CLIAgent,
    /// Selected model override, empty for none. The dropdown options live
    /// alongside so a refresh never loses the typed value.
    model: String,
    models: Vec<String>,
    /// Last refresh failure, if any. The old list is kept; this only
    /// explains why it did not update.
    models_error: Option<String>,
    /// A refresh is running off the UI thread.
    models_loading: bool,
    /// Guards overlapping refreshes; only the latest generation applies.
    models_generation: u64,
    /// Model-list source toggle: false loads the URL text as a
    /// models.dev-compatible list (custom mirrors allowed), true loads the
    /// path text as a `{flag, models[]}` JSON file. Session-only; the loaded
    /// list persists through the UI-written per-base cache.
    models_from_file: bool,
    /// The URL or file path the list loads from, depending on the toggle.
    models_source: Entity<InputState>,
    error: Option<String>,
    _subs: Vec<Subscription>,
}

impl Tty7App {
    /// Every role on this machine, slug order — the list behind this page,
    /// the palette rows and every dispatch path.
    pub(crate) fn load_agent_roles() -> Vec<AgentRole> {
        roles_dir().map(|dir| load_roles(&dir)).unwrap_or_default()
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
        subs.push(
            cx.subscribe_in(&input, window, |this, _i, ev: &InputEvent, _w, cx| {
                if matches!(ev, InputEvent::Change) {
                    let _ = this.role_form_mut();
                    cx.notify();
                }
            }),
        );
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
        let name = seed(editing.map(|r| r.name.as_str()).unwrap_or(""), &mut subs);
        let launch = seed(editing.map(|r| r.launch.as_str()).unwrap_or(""), &mut subs);
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
        let base = editing.map(|r| r.base).unwrap_or(CLIAgent::Claude);
        // Restore the last-used source for this base so a reopen keeps the
        // file-loaded list (via the cache seed below) instead of resetting
        // to the default URL.
        let (from_file, last_text) = read_last_models_source(base)
            .unwrap_or((false, MODELS_DEV_URL.to_string()));
        let models_source = seed(&last_text, &mut subs);
        let source_opt = source_from_parts(from_file, &last_text);
        // Fast path first so opening the form never blocks on a hung
        // source; the cached list seeds instantly and the live list fills
        // in on a background task below.
        let mut initial = model_choices_fast(base);
        if let Some(ref source) = source_opt {
            let cached = read_models_cache(base, &source.cache_key());
            if !cached.is_empty() {
                initial = cached;
            }
        }
        let auto_load = source_opt.is_some()
            && Self::models_should_load(base, from_file, &last_text);
        let form = AgentRoleForm {
            editing: editing.map(|r| r.slug.clone()),
            base,
            model: editing.map(|r| r.model.clone()).unwrap_or_default(),
            models: initial,
            models_error: None,
            models_loading: auto_load,
            models_generation: 0,
            models_from_file: from_file,
            models_source,
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
        // Live fill for bases with a source (mapped providers, or a custom
        // URL the user typed): success replaces the fast list, failure keeps
        // it and records the reason instead of silently showing stale data.
        if auto_load {
            self.spawn_live_models(0, cx);
        }
    }

    /// Whether opening the form (or switching to `base`) should kick off a
    /// background load: mapped bases always do; unmapped ones only when the
    /// user pointed at an explicit source (a file path, or a non-default URL).
    fn models_should_load(base: CLIAgent, from_file: bool, source_text: &str) -> bool {
        if from_file {
            return !source_text.trim().is_empty();
        }
        let url = source_text.trim();
        !models_dev_providers(base).is_empty() || (!url.is_empty() && url != MODELS_DEV_URL)
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
            let (generation, auto_load) = if let Some(form) = self.role_form_mut() {
                form.base = *base;
                // A new base means a new model list; the old pick rarely
                // survives the switch, so reset rather than keep a stale one.
                // The source toggle stays: it is the user's choice, not the
                // base's.
                form.model.clear();
                form.models = model_choices_fast(*base);
                form.models_error = None;
                let auto_load = Self::models_should_load(
                    *base,
                    form.models_from_file,
                    &form.models_source.read(cx).value().to_string(),
                );
                form.models_loading = auto_load;
                form.models_generation = form.models_generation.wrapping_add(1);
                (form.models_generation, auto_load)
            } else {
                return;
            };
            if auto_load {
                self.spawn_live_models(generation, cx);
            }
        }
        cx.notify();
    }

    /// Switch the list source between a URL and a JSON file. The text field
    /// keeps its value; an empty file path loads nothing until one is typed.
    pub(crate) fn set_role_models_source(&mut self, index: usize, cx: &mut Context<Self>) {
        let generation = if let Some(form) = self.role_form_mut() {
            form.models_from_file = index == 1;
            if form.models_from_file && form.models_source.read(cx).value().trim().is_empty() {
                form.models_loading = false;
                form.models_error = None;
                cx.notify();
                return;
            }
            form.models_generation = form.models_generation.wrapping_add(1);
            form.models_loading = true;
            form.models_error = None;
            form.models_generation
        } else {
            return;
        };
        cx.notify();
        self.spawn_live_models(generation, cx);
    }

    /// Re-run the model fetch for the form's base (the dropdown's refresh).
    /// Never blocks the UI: the old list stays until the background fetch
    /// completes; a failure keeps the list and records the reason.
    pub(crate) fn refresh_role_models(&mut self, cx: &mut Context<Self>) {
        let generation = if let Some(form) = self.role_form_mut() {
            form.models_generation = form.models_generation.wrapping_add(1);
            form.models_loading = true;
            form.models_error = None;
            form.models_generation
        } else {
            return;
        };
        cx.notify();
        self.spawn_live_models(generation, cx);
    }

    /// Background model load for the form's current base and selected
    /// source (URL or file). Only `generation` applies, so rapid base
    /// switches or refresh clicks cannot let a stale slow fetch overwrite a
    /// newer one.
    fn spawn_live_models(&mut self, generation: u64, cx: &mut Context<Self>) {
        let (base, source) = match self.role_form_mut() {
            Some(form) => {
                let text = form.models_source.read(cx).value().trim().to_string();
                let Some(source) = source_from_parts(form.models_from_file, &text) else {
                    form.models_loading = false;
                    return;
                };
                // Remember the effective source so a reopen restores it.
                write_last_models_source(form.base, form.models_from_file, &text);
                (form.base, source)
            }
            None => return,
        };
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { fetch_live_models(base, &source) })
                .await;
            let _ = this.update(cx, |this, cx| {
                let Some(form) = this.role_form_mut() else {
                    return;
                };
                if form.models_generation != generation || form.base != base {
                    return;
                }
                form.models_loading = false;
                match result {
                    Ok(models) => {
                        form.models = models;
                        form.models_error = None;
                    }
                    Err(e) => {
                        // Keep the old (fast/fallback) list; surface why it
                        // did not update instead of silently going stale.
                        if form.models.is_empty() {
                            form.models = model_choices_fast(form.base);
                        }
                        form.models_error = Some(e);
                    }
                }
                cx.notify();
            });
        })
        .detach();
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
        let (editing, name, launch, description, instructions, starters, base, model) = {
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
                form.model.clone(),
            )
        };
        if name.is_empty() {
            if let Some(form) = self.role_form_mut() {
                form.error = Some(t(L10nKey::SettingsRoleNameRequired).to_string());
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
            model,
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
            h_flex()
                .pb(px(4.))
                .child(
                    kit::button("role-new", t(L10nKey::SettingsRoleNew), BtnKind::Primary)
                        .on_click(cx.listener(|this, _ev: &gpui::ClickEvent, window, cx| {
                            this.start_role_add(window, cx);
                        })),
                )
                .into_any_element(),
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
            .on_click(
                cx.listener(move |this, _ev: &gpui::ClickEvent, window, cx| {
                    this.start_role_edit(&slug, window, cx);
                }),
            );
            let slug = role.slug.clone();
            let delete = kit::button(("role-delete", i), t(L10nKey::Delete), BtnKind::Danger)
                .on_click(
                    cx.listener(move |this, _ev: &gpui::ClickEvent, _window, cx| {
                        this.remove_agent_role(&slug, cx);
                    }),
                );
            rows.push(
                self.settings_row(
                    crate::ui::app::role_display(Some(&role.slug), Some(role.base)),
                    match role.description.trim().is_empty() {
                        true => role.launch.clone(),
                        false => role.description.clone(),
                    },
                    h_flex()
                        .gap(px(8.))
                        .child(edit)
                        .child(delete)
                        .into_any_element(),
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
        let base_names: Vec<&str> = CLIAgent::ALL.iter().map(|a| a.display_name()).collect();
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
        // Model options: "launch line only" first, then whatever the base
        // offers; a saved model the list no longer names stays selectable
        // as-is rather than silently clearing. A search dropdown like the
        // font menus: model lists run long and the plain popover cannot
        // scroll.
        let mut model_values: Vec<String> = vec![String::new()];
        model_values.extend(form.models.iter().cloned());
        if !form.model.is_empty() && !model_values[1..].contains(&form.model) {
            model_values.push(form.model.clone());
        }
        let model_labels: Vec<SharedString> = model_values
            .iter()
            .enumerate()
            .map(|(i, m)| {
                if i == 0 {
                    SharedString::from(t(L10nKey::SettingsRoleModelNone))
                } else {
                    SharedString::from(m.clone())
                }
            })
            .collect();
        let model_selected = model_values
            .iter()
            .position(|m| *m == form.model)
            .filter(|_| !form.model.is_empty())
            .unwrap_or(0);
        let model_options = std::rc::Rc::new(
            model_labels
                .into_iter()
                .map(|label| SearchOption { label, font: None })
                .collect::<Vec<_>>(),
        );        let model_values = std::rc::Rc::new(model_values);
        let model_choice = self.settings_search_dropdown(
            "role-model",
            model_options[model_selected].label.clone(),
            model_options,
            Some(model_selected),
            std::rc::Rc::new(move |this, ix, _w, cx| {
                if let Some(form) = this.role_form_mut() {
                    form.model = model_values.get(ix).cloned().unwrap_or_default();
                }
                cx.notify();
            }),
            cx,
        );
        const W: f32 = 320.;
        let models_loading = form.models_loading;
        let models_error = form.models_error.clone();
        let source_selected = match form.models_from_file {
            true => 1,
            false => 0,
        };
        let source_choice = self.settings_choice(
            "role-models-source",
            &["URL", "File"],
            source_selected,
            cx,
            |this, ix, _w, cx| {
                this.set_role_models_source(ix, cx);
            },
        );
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
                t(L10nKey::SettingsRoleModel),
                t(L10nKey::SettingsRoleModelDesc),
                h_flex()
                    .gap(px(8.))
                    .child(model_choice)
                    .child(
                        kit::button(
                            "role-models-refresh",
                            match models_loading {
                                true => format!("{}…", t(L10nKey::SettingsRoleRefresh)),
                                false => t(L10nKey::SettingsRoleRefresh).to_string(),
                            },
                            BtnKind::Link,
                        )
                        .on_click(cx.listener(
                            |this, _ev: &gpui::ClickEvent, _w, cx| {
                                this.refresh_role_models(cx);
                            },
                        )),
                    )
                    .into_any_element(),
                cx,
            )
            .into_any_element(),
            self.settings_row(
                t(L10nKey::SettingsRoleModelsSource),
                t(L10nKey::SettingsRoleModelsSourceDesc),
                v_flex()
                    .gap(px(6.))
                    .items_end()
                    .child(source_choice)
                    .child(self.settings_text_input(&form.models_source, W, false, cx))
                    .into_any_element(),
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
        if let Some(error) = models_error.as_ref() {
            let tk = Tk::of(cx);
            rows.push(
                div()
                    .text_size(fs(12.))
                    .text_color(tk.danger)
                    .child(format!("Models refresh failed: {error}"))
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
