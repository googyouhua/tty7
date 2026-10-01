//! Startup instance picker: who is using tty7?
//!
//! On a machine where several people share one OS user, one config
//! directory means one server and everybody's sessions. The picker runs
//! before anything instance-specific initializes — no daemon, no window
//! store — and resolves the config directory for this launch:
//!
//! - launched with `--config-dir` / `TTY7_CONFIG_DIR`: the picker never
//!   runs; explicit always wins.
//! - launched bare: the picker lists the default instance plus every
//!   `tty7-<name>` already on disk, with a field for a new username. The
//!   path is derived, never typed: `${ROOT}/tty7-<username>`.
//! - the last choice is preselected and remembered in the default
//!   directory's memory file; the picker waits for a confirm and never
//!   auto-enters. Closing it quits the launch.
//!
//! The main flow continues in-process afterwards: picking only sets the
//! config directory, and every downstream step (daemon ensure, window
//! forward, first window) already resolves it live.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use gpui::{
    App, AppContext as _, Bounds, Context, Entity, FocusHandle, Focusable, KeyBinding, Render,
    SharedString, Subscription, Window, WindowBounds, WindowOptions, actions, div, prelude::*, px,
    size,
};
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::{h_flex, v_flex};
use tty7_core::core::instance::{self, DEFAULT_SENTINEL};

use crate::ui::i18n::{L10nKey, set_locale, t};

actions!(
    instance_picker,
    [ConfirmInstance, CancelInstance, PickerUp, PickerDown]
);

/// What the picker resolved: the default instance, or a named one.
#[derive(Debug, Clone, PartialEq)]
pub enum PickerSelection {
    Default,
    Named(String),
}

impl PickerSelection {
    /// The config directory for the selection, creating an instance's on
    /// first use. `None` only when no root resolves at all.
    pub fn config_dir(&self) -> Option<PathBuf> {
        match self {
            PickerSelection::Default => crate::core::config::default_config_dir(),
            PickerSelection::Named(name) => {
                let dir = instance::dir_for(name)?;
                instance::ensure_dir(&dir).ok()?;
                Some(dir)
            }
        }
    }

    /// The memory value for the selection.
    pub fn memory_name(&self) -> &str {
        match self {
            PickerSelection::Default => DEFAULT_SENTINEL,
            PickerSelection::Named(name) => name,
        }
    }
}

/// Apply a confirmed selection for this launch: resolve (and create) its
/// directory, remember it, and point the process at it. Best-effort memory:
/// a failed write never blocks entering.
pub fn apply_selection(selection: &PickerSelection) -> anyhow::Result<PathBuf> {
    let dir = selection
        .config_dir()
        .ok_or_else(|| anyhow::anyhow!("no directory resolves for this instance"))?;
    let _ = instance::write_last(selection.memory_name());
    crate::core::config::set_config_dir(dir.clone());
    Ok(dir)
}

/// Remember the explicitly chosen instance (flag or environment) for the
/// next bare launch's preselect. Best-effort: unmapped directories and
/// failed writes never block entering.
pub fn remember_current() {
    let Some(dir) = crate::core::config::config_dir_path() else {
        return;
    };
    if let Some(name) = instance::memory_name_for(&dir) {
        let _ = instance::write_last(&name);
    }
}

/// The selectable rows: the default instance first, then existing names.
#[derive(Debug, Clone)]
pub struct PickerState {
    names: Vec<String>,
    cursor: usize,
}

impl PickerState {
    pub fn new(names: Vec<String>, last: Option<&str>) -> Self {
        let cursor = match last {
            None | Some(DEFAULT_SENTINEL) => 0,
            Some(name) => names
                .iter()
                .position(|n| n == name)
                .map(|i| i + 1)
                .unwrap_or(0),
        };
        Self { names, cursor }
    }

    /// Row count including the default row.
    pub fn row_count(&self) -> usize {
        self.names.len() + 1
    }

    pub fn move_up(&mut self) {
        self.cursor = self.cursor.checked_sub(1).unwrap_or(self.row_count() - 1);
    }

    pub fn move_down(&mut self) {
        self.cursor = (self.cursor + 1) % self.row_count();
    }

    /// The row under the cursor.
    pub fn current(&self) -> PickerSelection {
        if self.cursor == 0 {
            PickerSelection::Default
        } else {
            PickerSelection::Named(self.names[self.cursor - 1].clone())
        }
    }

    /// Confirm: typed text wins as a new username, otherwise the cursor row.
    /// `Err` means the typed name is invalid and names the message to show.
    pub fn confirm(&self, typed: &str) -> Result<PickerSelection, &'static str> {
        let typed = typed.trim();
        if typed.is_empty() {
            return Ok(self.current());
        }
        match instance::validate_name(typed) {
            Ok(name) => Ok(PickerSelection::Named(name)),
            Err(_) => Err("invalid"),
        }
    }
}

pub struct InstancePicker {
    state: PickerState,
    input: Entity<InputState>,
    focus: FocusHandle,
    error: bool,
    result: Arc<Mutex<Option<Option<PickerSelection>>>>,
    _sub: Subscription,
}

impl InstancePicker {
    fn new(
        state: PickerState,
        result: Arc<Mutex<Option<Option<PickerSelection>>>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let input = cx.new(|cx| {
            InputState::new(window, cx).placeholder(t(L10nKey::InstancePickerNewPlaceholder))
        });
        let _sub = cx.subscribe_in(
            &input,
            window,
            |this: &mut Self, _, ev: &InputEvent, window, cx| match ev {
                InputEvent::PressEnter { .. } => this.confirm(window, cx),
                InputEvent::Change => {
                    this.error = false;
                    cx.notify();
                }
                _ => {}
            },
        );
        let focus = cx.focus_handle();
        Self {
            state,
            input,
            focus,
            error: false,
            result,
            _sub,
        }
    }

    fn finish(&self, choice: Option<PickerSelection>, cx: &mut Context<Self>) {
        *self.result.lock().unwrap_or_else(|e| e.into_inner()) = Some(choice);
        cx.quit();
    }

    fn confirm(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let typed = self.input.read(cx).value().to_string();
        match self.state.confirm(&typed) {
            Ok(choice) => self.finish(Some(choice), cx),
            Err(_) => {
                self.error = true;
                cx.notify();
            }
        }
        let _ = window;
    }

    fn cancel(&self, cx: &mut Context<Self>) {
        self.finish(None, cx);
    }

    /// Up/Down move the rows, unless the new-name field holds the focus
    /// with text in it — there they are the caret's own keys. Empty input
    /// has no caret business, so the rows stay reachable from the keyboard.
    fn input_holds_text(&self, cx: &mut Context<Self>) -> bool {
        !self.input.read(cx).value().is_empty()
    }

    fn input_focused(&self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        window.focused(cx) == Some(self.input.read(cx).focus_handle(cx))
    }

    fn row_label(&self, index: usize) -> SharedString {
        if index == 0 {
            t(L10nKey::InstancePickerDefault).into()
        } else {
            self.state.names[index - 1].clone().into()
        }
    }
}

impl Focusable for InstancePicker {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for InstancePicker {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl gpui::IntoElement {
        let rows = self.state.row_count();
        let cursor = self.state.cursor;
        let invalid = self.error;
        let input = self.input.clone();
        v_flex()
            .on_action(
                cx.listener(|this, _: &ConfirmInstance, window, cx| this.confirm(window, cx)),
            )
            .on_action(cx.listener(|this, _: &CancelInstance, _, cx| this.cancel(cx)))
            .on_action(cx.listener(|this, _: &PickerUp, window, cx| {
                if !this.input_focused(window, cx) || !this.input_holds_text(cx) {
                    this.state.move_up();
                    cx.notify();
                }
            }))
            .on_action(cx.listener(|this, _: &PickerDown, window, cx| {
                if !this.input_focused(window, cx) || !this.input_holds_text(cx) {
                    this.state.move_down();
                    cx.notify();
                }
            }))
            .p(px(20.))
            .gap(px(12.))
            .w_full()
            .child(
                div()
                    .text_size(px(15.))
                    .font_weight(gpui::FontWeight::BOLD)
                    .child(t(L10nKey::InstancePickerTitle)),
            )
            .child(
                div()
                    .text_size(px(12.))
                    .child(t(L10nKey::InstancePickerHint)),
            )
            .children((0..rows).map(|index| {
                let selected = index == cursor;
                let label = self.row_label(index);
                div()
                    .id(("instance-picker-row", index))
                    .px(px(10.))
                    .py(px(7.))
                    .rounded(px(6.))
                    .cursor_pointer()
                    .when(selected, |row| {
                        row.bg(gpui::rgb(0x2E7D5B)).text_color(gpui::white())
                    })
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.state.cursor = index;
                        this.confirm(window, cx);
                    }))
                    .child(label)
            }))
            .child(Input::new(&input).cleanable(true))
            .when(invalid, |column| {
                column.child(
                    div()
                        .text_size(px(12.))
                        .text_color(gpui::rgb(0xC0392B))
                        .child(t(L10nKey::InstancePickerInvalid)),
                )
            })
            .child(
                h_flex().justify_end().child(
                    Button::new("instance-picker-enter")
                        .label(t(L10nKey::InstancePickerEnter))
                        .primary()
                        .on_click(cx.listener(|this, _, window, cx| this.confirm(window, cx))),
                ),
            )
            .track_focus(&self.focus)
    }
}

/// Run the picker as this process's gpui application. Returns the confirmed
/// selection, or `None` when the user closed it without choosing.
pub fn run_picker() -> Option<PickerSelection> {
    let names = instance::list_names();
    let last = instance::read_last();
    let state = PickerState::new(names, last.as_deref());
    let result: Arc<Mutex<Option<Option<PickerSelection>>>> = Arc::new(Mutex::new(None));
    let outcome = result.clone();

    // Locale of the default config: the picker runs before Config loads.
    let language = crate::core::config::Config::load().gui_language.clone();
    let application = gpui_platform::application().with_assets(crate::ui::assets::Assets);
    application.run(move |cx| {
        cx.spawn(async move |cx| {
            let _ = cx.update(|cx| {
                gpui_component::init(cx);
                crate::register_bundled_fonts(cx);
                set_locale(&language);
                cx.bind_keys([
                    KeyBinding::new("enter", ConfirmInstance, None),
                    KeyBinding::new("escape", CancelInstance, None),
                    KeyBinding::new("up", PickerUp, None),
                    KeyBinding::new("down", PickerDown, None),
                ]);
                let options = WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                        None,
                        size(px(400.), px(430.)),
                        cx,
                    ))),
                    app_id: Some("tty7".to_owned()),
                    ..Default::default()
                };
                let result = outcome.clone();
                let opened = cx.open_window(options, |window, cx| {
                    let view =
                        cx.new(|cx| InstancePicker::new(state.clone(), result.clone(), window, cx));
                    let handle = view.read(cx).input.read(cx).focus_handle(cx);
                    window.focus(&handle, cx);
                    cx.new(|cx| gpui_component::Root::new(view, window, cx))
                });
                if opened.is_err() {
                    *outcome.lock().unwrap_or_else(|e| e.into_inner()) = Some(None);
                }
            });
        })
        .detach();
    });
    result
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
        .flatten()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_starts_on_the_remembered_row() {
        let names = vec!["bob".to_string(), "alice".to_string()];
        assert_eq!(
            PickerState::new(names.clone(), Some("alice")).current(),
            PickerSelection::Named("alice".to_string())
        );
        assert_eq!(
            PickerState::new(names.clone(), Some(DEFAULT_SENTINEL)).current(),
            PickerSelection::Default
        );
        assert_eq!(
            PickerState::new(names.clone(), None).current(),
            PickerSelection::Default
        );
        // A remembered instance that no longer exists falls back to default.
        assert_eq!(
            PickerState::new(names, Some("ghost")).current(),
            PickerSelection::Default
        );
    }

    #[test]
    fn cursor_wraps_around_the_rows() {
        let mut state = PickerState::new(vec!["alice".to_string()], None);
        assert_eq!(state.row_count(), 2);
        state.move_up();
        assert_eq!(state.current(), PickerSelection::Named("alice".to_string()));
        state.move_down();
        assert_eq!(state.current(), PickerSelection::Default);
    }

    #[test]
    fn typed_text_wins_over_the_cursor_row() {
        let state = PickerState::new(vec!["alice".to_string()], None);
        assert_eq!(state.confirm("").unwrap(), PickerSelection::Default);
        assert_eq!(
            state.confirm("  bob ").unwrap(),
            PickerSelection::Named("bob".to_string())
        );
        assert!(state.confirm("../x").is_err());
        assert!(state.confirm("default").is_err());
    }

    #[test]
    fn selections_map_to_memory_values() {
        assert_eq!(PickerSelection::Default.memory_name(), DEFAULT_SENTINEL);
        assert_eq!(
            PickerSelection::Named("alice".to_string()).memory_name(),
            "alice"
        );
    }
}
