//! The typed prompt that lets a running container into tty7.
//!
//! Containers enter only when the user names one here: there is no listing, no
//! `docker ps` behind the scenes, and no row offered because a container
//! happened to be running. The name is validated before anything is spawned,
//! then the same connect-and-create path the switcher uses takes over.

use gpui::{
    AnyElement, Context, Div, Entity, Hsla, Subscription, Window, div, prelude::*, px, rems,
};
use gpui_component::ActiveTheme as _;
use gpui_component::input::{Input, InputEvent, InputState};

use crate::core::session::RemoteTarget;
use crate::ui::app::Tty7App;
use crate::ui::dialog::{self, Tone};
use crate::ui::i18n::{L10nKey, t};
use crate::ui::remote_connect::{HostChoice, HostLinks};
use crate::ui::right_panel::META_MONO;

pub(crate) struct ContainerPrompt {
    name: Entity<InputState>,
    /// Why the last submit did not go through, said under the field it is about.
    error: Option<String>,
    _sub: Subscription,
}

impl Tty7App {
    pub(crate) fn open_container_prompt(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let name = cx.new(|cx| {
            InputState::new(window, cx).placeholder(t(L10nKey::ContainerPromptPlaceholder))
        });
        name.update(cx, |state, cx| state.focus(window, cx));
        let sub = cx.subscribe_in(
            &name,
            window,
            |this, _, ev: &InputEvent, window, cx| match ev {
                InputEvent::PressEnter { .. } => this.submit_container_prompt(window, cx),
                InputEvent::Change => {
                    if let Some(p) = this.container_prompt.as_mut() {
                        p.error = None;
                    }
                    cx.notify();
                }
                _ => {}
            },
        );
        self.container_prompt = Some(ContainerPrompt {
            name,
            error: None,
            _sub: sub,
        });
        cx.notify();
    }

    /// Put the prompt back after a connect failed, carrying the reason and the
    /// name it was typed with. A container has no switcher row and no workspace
    /// until its link lands, so this dialog is the only place its failure can
    /// be read — closing it on submit would leave the user with nothing at all.
    pub(crate) fn reopen_container_prompt(
        &mut self,
        container: String,
        error: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.open_container_prompt(window, cx);
        if let Some(p) = self.container_prompt.as_mut() {
            p.error = Some(error);
            let name = p.name.clone();
            name.update(cx, |state, cx| state.set_value(&container, window, cx));
        }
        cx.notify();
    }

    fn cancel_container_prompt(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.container_prompt.take().is_some() {
            self.focus_active(window, cx);
            cx.notify();
        }
    }

    fn submit_container_prompt(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(p) = self.container_prompt.as_ref() else {
            return;
        };
        let container = p.name.read(cx).value().trim().to_string();
        if container.is_empty() {
            if let Some(p) = self.container_prompt.as_mut() {
                p.error = Some(t(L10nKey::ContainerPromptNeedsName).to_string());
            }
            cx.notify();
            return;
        }
        self.add_container(&container, window, cx);
    }

    /// The one door containers enter by. A container already connected is
    /// entered at its home; a fresh one connects first, and the pending create
    /// it parks is spent by `finish_connect` once the link lands.
    fn add_container(&mut self, container: &str, window: &mut Window, cx: &mut Context<Self>) {
        if let Err(e) = tty7_core::daemon::install::container::validate_container(container) {
            if let Some(p) = self.container_prompt.as_mut() {
                p.error = Some(e.to_string());
            }
            cx.notify();
            return;
        }
        let target = RemoteTarget::Container {
            container: container.to_string(),
        };
        let label = tty7_core::daemon::install::container::host_label(container);
        self.container_prompt = None;
        if let Some(home) = HostLinks::home(cx, target.host_id()) {
            self.create_remote_workspace(target, home, window, cx);
            // The workspace is the container; its first pane is the shell the
            // user asked for, opened on the container's own home.
            self.new_tab(window, cx);
            return;
        }
        self.pending_create = Some(crate::ui::switcher::PendingCreate {
            target: target.clone(),
            name: Some(container.to_string()),
        });
        self.connect_to_host(
            HostChoice {
                target,
                label,
                detail: String::new(),
            },
            cx,
        );
    }

    pub(crate) fn render_container_prompt_overlay(
        &self,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let p = self.container_prompt.as_ref()?;
        let rungs = dialog::popover_rungs(cx);
        let muted = cx.theme().muted_foreground;
        let danger = cx.theme().danger;
        let note = |color: Hsla, text: String| -> Div {
            div()
                .text_size(rems(META_MONO))
                .text_color(color)
                .child(text)
        };
        let card = dialog::card(
            "container-prompt-card",
            t(L10nKey::ContainerPromptTitle),
            440.,
            cx,
        )
        .child(dialog::header(t(L10nKey::ContainerPromptTitle), cx))
        .child(
            dialog::body().child(
                dialog::labelled(t(L10nKey::ContainerPromptName), Input::new(&p.name), cx)
                    .child(note(muted, t(L10nKey::ContainerPromptHint).to_string()))
                    .children(p.error.as_ref().map(|e| note(danger, e.clone()))),
            ),
        )
        .child(
            dialog::footer(cx)
                .child(dialog::button(
                    "container-cancel",
                    t(L10nKey::Cancel),
                    Tone::Secondary,
                    true,
                    rungs,
                    cx,
                    cx.listener(|this, _, window, cx| this.cancel_container_prompt(window, cx)),
                ))
                .child(dialog::button(
                    "container-add",
                    t(L10nKey::ContainerPromptAdd),
                    Tone::Primary,
                    !p.name.read(cx).value().trim().is_empty(),
                    rungs,
                    cx,
                    cx.listener(|this, _, window, cx| this.submit_container_prompt(window, cx)),
                )),
        );
        Some(
            div()
                .absolute()
                .inset_0()
                .bg(crate::ui::presets::scrim_fill(cx))
                .on_key_down(cx.listener(|this, ev: &gpui::KeyDownEvent, window, cx| {
                    if ev.keystroke.key == "escape" {
                        this.cancel_container_prompt(window, cx);
                    }
                }))
                .on_mouse_down(
                    gpui::MouseButton::Left,
                    cx.listener(|this, _: &gpui::MouseDownEvent, window, cx| {
                        this.cancel_container_prompt(window, cx)
                    }),
                )
                .flex()
                .flex_col()
                .items_center()
                .justify_start()
                .pt(px(crate::ui::switcher::CARD_TOP))
                .child(card)
                .into_any_element(),
        )
    }
}
