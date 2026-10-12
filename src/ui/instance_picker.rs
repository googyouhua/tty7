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
//!   `tty7-<name>` already on disk, with a filter field that narrows the
//!   dropdown to substring matches. The highlight alone is entered: typing
//!   a name that matches nothing never creates it. The
//!   path is derived, never typed: `${ROOT}/tty7-<username>`.
//! - the last choice is preselected and remembered in the default
//!   directory's memory file; the picker waits for a confirm and never
//!   auto-enters. Closing it quits the launch.
//!
//! On Windows the platform runner ends in `ExitProcess`, so continuing in
//! this process after the picker is impossible: confirming relaunches this
//! executable with the resolved `--config-dir` (plus the original launch
//! arguments) and quits. Other platforms continue in-process. Either way
//! the follow-up launch carries an explicit config, so it skips the picker
//! and remembers the choice for the next bare launch.

use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use gpui::{
    App, AppContext as _, Bounds, Context, Entity, FocusHandle, Focusable, KeyBinding, Render,
    ScrollHandle, SharedString, Subscription, Window, WindowBounds, WindowOptions, actions, div,
    prelude::*, px, size,
};
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::{h_flex, v_flex};
use tty7_core::core::instance::{self, DEFAULT_SENTINEL};

use crate::ui::i18n::{L10nKey, set_locale, t, t_fmt, t_plural};

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

/// The selectable rows: the default instance first, then existing names
/// with their snapshot mtimes for recency display.
#[derive(Debug, Clone)]
pub struct PickerRow {
    name: String,
    mtime: Option<SystemTime>,
}

/// The selectable rows: the default instance first, then existing names
/// with their snapshot mtimes for recency display.
///
/// The picker is a combobox: the input filters these rows (case-insensitive
/// substring; the Default row always stays on top) and the highlight moves
/// over the visible rows only, wrapping at the ends.
#[derive(Debug, Clone)]
pub struct PickerState {
    rows: Vec<PickerRow>,
    query: String,
    /// Position inside [`PickerState::visible`], not a global row index.
    cursor: usize,
    /// The open-time preselect (global row index, equal to a visible
    /// position while the query is empty): clearing the filter restores
    /// the highlight here instead of dropping it on Default.
    preselect: usize,
    dropdown_open: bool,
}

/// What the next Esc press should do: layers close topmost-first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EscLayer {
    CloseDropdown,
    CancelWindow,
}

impl PickerState {
    pub fn new(names: Vec<(String, Option<SystemTime>)>, last: Option<&str>) -> Self {
        let rows: Vec<PickerRow> = names
            .into_iter()
            .map(|(name, mtime)| PickerRow { name, mtime })
            .collect();
        let cursor = match last {
            None | Some(DEFAULT_SENTINEL) => 0,
            Some(name) => rows
                .iter()
                .position(|r| r.name == name)
                .map(|i| i + 1)
                .unwrap_or(0),
        };
        Self {
            rows,
            query: String::new(),
            cursor,
            preselect: cursor,
            dropdown_open: true,
        }
    }

    /// Row count including the default row (the unfiltered total).
    #[cfg(test)]
    pub fn row_count(&self) -> usize {
        self.rows.len() + 1
    }

    /// Global row indices currently shown: the Default row (0) always on
    /// top, then the named rows whose name contains the trimmed query,
    /// case-insensitive. An empty query shows everything.
    pub fn visible(&self) -> Vec<usize> {
        let q = self.query.trim().to_lowercase();
        let mut out = vec![0];
        if q.is_empty() {
            out.extend(1..=self.rows.len());
            return out;
        }
        out.extend(
            self.rows
                .iter()
                .enumerate()
                .filter(|(_, r)| r.name.to_lowercase().contains(&q))
                .map(|(i, _)| i + 1),
        );
        out
    }

    /// Named matches under the current filter (excludes the Default row).
    pub fn match_count(&self) -> usize {
        self.visible().len().saturating_sub(1)
    }

    /// Whether the input currently narrows the list.
    pub fn has_query(&self) -> bool {
        !self.query.trim().is_empty()
    }

    /// Re-filter on the latest input text. The highlight resets to the
    /// first named match — the always-visible Default row never steals it
    /// from a real match — or to the Default row when nothing matches.
    /// Clearing restores the open-time preselect highlight.
    pub fn set_query(&mut self, query: &str) {
        self.query = query.to_string();
        if !self.has_query() {
            self.cursor = self.preselect.min(self.visible().len().saturating_sub(1));
            return;
        }
        self.cursor = self.visible().iter().position(|&g| g != 0).unwrap_or(0);
    }

    pub fn is_dropdown_open(&self) -> bool {
        self.dropdown_open
    }

    pub fn open_dropdown(&mut self) {
        self.dropdown_open = true;
    }

    pub fn close_dropdown(&mut self) {
        self.dropdown_open = false;
    }

    /// Esc closes the open dropdown first (text and highlight are kept by
    /// the caller); only a second press with it closed cancels the window.
    pub fn esc_layer(&mut self) -> EscLayer {
        if self.dropdown_open {
            self.close_dropdown();
            EscLayer::CloseDropdown
        } else {
            EscLayer::CancelWindow
        }
    }

    fn clamp_cursor(&mut self) {
        let n = self.visible().len().max(1);
        self.cursor = self.cursor.min(n - 1);
    }

    pub fn move_up(&mut self) {
        let n = self.visible().len().max(1);
        self.clamp_cursor();
        self.cursor = (self.cursor + n - 1) % n;
    }

    pub fn move_down(&mut self) {
        let n = self.visible().len().max(1);
        self.clamp_cursor();
        self.cursor = (self.cursor + 1) % n;
    }

    /// The row under the highlight.
    pub fn current(&self) -> PickerSelection {
        match self.visible().get(self.cursor).copied().unwrap_or(0) {
            0 => PickerSelection::Default,
            i => PickerSelection::Named(self.rows[i - 1].name.clone()),
        }
    }

    /// Confirm the highlighted row. The typed filter never wins and never
    /// creates: a valid but unlisted name is not entered here (the explicit
    /// create path owns that). Validity itself still comes only from
    /// [`instance::validate_name`], enforced by the view's invalid line.
    pub fn confirm(&self) -> PickerSelection {
        self.current()
    }
}

pub struct InstancePicker {
    state: PickerState,
    input: Entity<InputState>,
    list_scroll: ScrollHandle,
    focus: FocusHandle,
    relaunch_failed: bool,
    passthrough: Vec<OsString>,
    result: Arc<Mutex<Option<Option<PickerSelection>>>>,
    _sub: Subscription,
}

impl InstancePicker {
    fn new(
        state: PickerState,
        passthrough: Vec<OsString>,
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
                    // Input is filter: every keystroke re-narrows the
                    // dropdown, resets the highlight, and reopens it.
                    let value = this.input.read(cx).value().to_string();
                    this.state.set_query(&value);
                    this.state.open_dropdown();
                    cx.notify();
                }
                _ => {}
            },
        );
        let focus = cx.focus_handle();
        Self {
            state,
            input,
            list_scroll: ScrollHandle::new(),
            focus,
            relaunch_failed: false,
            passthrough,
            result,
            _sub,
        }
    }

    fn finish(
        &mut self,
        choice: Option<PickerSelection>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // Enter reaches here twice (Input PressEnter + the Enter keybind):
        // the second call must not quit twice. A failed Windows relaunch
        // leaves `result` unset so the user can retry.
        if self
            .result
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .is_some()
        {
            return;
        }
        #[cfg(windows)]
        if let Some(choice) = &choice {
            // `platform.run` never returns on Windows — it ends in
            // `ExitProcess` — so hand the resolved config to a fresh process
            // (which skips the picker and remembers the choice) and quit.
            // Anywhere else the caller continues in-process below.
            if let Err(e) = relaunch_with(choice, &self.passthrough) {
                log::error!("instance picker relaunch failed: {e:#}");
                // Stay: quitting here would lose the launch with no window.
                self.relaunch_failed = true;
                cx.notify();
                return;
            }
        }
        *self.result.lock().unwrap_or_else(|e| e.into_inner()) = Some(choice);
        // Close the picker window first: `quit` alone stops the event loop
        // but leaves the native surface mapped, and the compositor then
        // reports the leftover window as "not responding" next to the main
        // window the follow-up launch opens in-process.
        window.remove_window();
        cx.quit();
    }

    fn confirm(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // Enter confirms the highlighted row only. A non-empty filter that
        // fails `validate_name` keeps the window open with the invalid line;
        // a valid but unlisted name simply confirms the highlight — it is
        // never created here.
        let typed = self.input.read(cx).value().to_string();
        let trimmed = typed.trim();
        if !trimmed.is_empty() && instance::validate_name(trimmed).is_err() {
            cx.notify();
            return;
        }
        let choice = self.state.confirm();
        self.finish(Some(choice), window, cx);
    }

    fn cancel(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // Esc layers: first close the dropdown (text and highlight kept),
        // then cancel the launch.
        if self.state.esc_layer() == EscLayer::CloseDropdown {
            cx.notify();
            return;
        }
        self.finish(None, window, cx);
    }

    fn row_label(&self, index: usize) -> SharedString {
        if index == 0 {
            t(L10nKey::InstancePickerDefault).into()
        } else {
            self.state.rows[index - 1].name.clone().into()
        }
    }

    /// Screen-reader label carrying the option's position in the visible
    /// list. The position fragment is a localized `InstancePickerPosition`
    /// template, mirroring the established "{x} of {total}" key family.
    fn row_accessible_label(&self, index: usize, position: usize, total: usize) -> String {
        let label = if index == 0 {
            t(L10nKey::InstancePickerDefault).to_string()
        } else {
            self.state.rows[index - 1].name.clone()
        };
        let position = t_fmt(
            L10nKey::InstancePickerPosition,
            &[
                ("position", &(position + 1).to_string()),
                ("total", &total.to_string()),
            ],
        );
        format!("{label}, {position}")
    }

    /// Recency meta for a named row, or `None` for the default row and for
    /// rows whose mtime the snapshot could not supply (renders nothing).
    fn row_meta(&self, index: usize) -> Option<String> {
        if index == 0 {
            return None;
        }
        let mtime = self.state.rows[index - 1].mtime?;
        let now = crate::core::config::unix_now();
        let then = mtime.duration_since(UNIX_EPOCH).ok()?.as_secs();
        Some(crate::ui::home::relative_time(now, then))
    }
}

impl Focusable for InstancePicker {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for InstancePicker {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl gpui::IntoElement {
        let visible = self.state.visible();
        let total = visible.len();
        let cursor = self.state.cursor.min(total.saturating_sub(1));
        let dropdown_open = self.state.is_dropdown_open();
        let match_count = self.state.match_count();
        let filtering = self.state.has_query();
        // The invalid line is live: any trimmed non-empty input that fails
        // the single validity rule explains itself as you type.
        let invalid = {
            let typed = self.input.read(cx).value().to_string();
            let trimmed = typed.trim().to_string();
            !trimmed.is_empty() && instance::validate_name(&trimmed).is_err()
        };
        let relaunch_failed = self.relaunch_failed;
        let input = self.input.clone();
        v_flex()
            .on_action(
                cx.listener(|this, _: &ConfirmInstance, window, cx| this.confirm(window, cx)),
            )
            .on_action(cx.listener(|this, _: &CancelInstance, window, cx| this.cancel(window, cx)))
            // The combobox owns the arrows: they always move the highlight,
            // even with the caret sitting in filtered text. A closed
            // dropdown reopens on either key.
            .on_action(cx.listener(|this, _: &PickerUp, _window, cx| {
                this.state.open_dropdown();
                this.state.move_up();
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &PickerDown, _window, cx| {
                this.state.open_dropdown();
                this.state.move_down();
                cx.notify();
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
            .child(
                // Clicking back into the filter reopens a dismissed dropdown.
                div()
                    .id("instance-picker-filter")
                    .w_full()
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.state.open_dropdown();
                        cx.notify();
                    }))
                    .child(Input::new(&input).cleanable(true)),
            )
            .when(dropdown_open, |column| {
                column.child(crate::ui::scrollbar::over_vertical_scroll(
                    "instance-picker-list",
                    div()
                        .id("instance-picker-list")
                        .role(gpui::Role::ListBox)
                        .aria_label(t(L10nKey::InstancePickerTitle))
                        .track_scroll(&self.list_scroll)
                        .max_h(px(240.))
                        .overflow_y_scroll()
                        .children(visible.iter().enumerate().map(|(position, &index)| {
                            let selected = position == cursor;
                            let label = self.row_label(index);
                            let meta = self.row_meta(index);
                            let option_label =
                                self.row_accessible_label(index, position, total);
                            div()
                                .id(("instance-picker-row", index))
                                .role(gpui::Role::ListBoxOption)
                                .aria_label(option_label)
                                .aria_selected(selected)
                                .px(px(10.))
                                .py(px(7.))
                                .rounded(px(6.))
                                .cursor_pointer()
                                .flex()
                                .justify_between()
                                .items_center()
                                .when(selected, |row| {
                                    row.bg(gpui::rgb(0x2E7D5B)).text_color(gpui::white())
                                })
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    // A click selects the row outright: stale
                                    // invalid filter text vetoes Enter and
                                    // creation, never a direct selection.
                                    this.state.cursor = position;
                                    let choice = this.state.confirm();
                                    this.finish(Some(choice), window, cx);
                                }))
                                .child(label)
                                .children(meta.map(|m| {
                                    div().text_size(px(12.)).child(m)
                                }))
                        })),
                    &self.list_scroll,
                ))
            })
            .when(dropdown_open && filtering && match_count > 0, |column| {
                column.child(
                    div().text_size(px(12.)).child(t_plural(
                        L10nKey::InstancePickerMatchCount,
                        match_count,
                        &[],
                    )),
                )
            })
            .when(dropdown_open && filtering && match_count == 0, |column| {
                column.child(
                    div()
                        .text_size(px(12.))
                        .child(t(L10nKey::InstancePickerNoMatch)),
                )
            })
            .when(invalid, |column| {
                column.child(
                    div()
                        .text_size(px(12.))
                        .text_color(gpui::rgb(0xC0392B))
                        .child(t(L10nKey::InstancePickerInvalid)),
                )
            })
            .when(relaunch_failed, |column| {
                column.child(
                    div()
                        .text_size(px(12.))
                        .text_color(gpui::rgb(0xC0392B))
                        .child(t(L10nKey::InstancePickerRelaunchFailed)),
                )
            })
            .child(
                h_flex()
                    .justify_end()
                    .gap(px(8.))
                    .child(
                        Button::new("instance-picker-close")
                            .label(t(L10nKey::InstancePickerClose))
                            .ghost()
                            .on_click(cx.listener(|this, _, window, cx| this.cancel(window, cx))),
                    )
                    .child(
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
///
/// `passthrough` is the original launch arguments (minus the program name).
/// On Windows the confirmed selection relaunches this executable with the
/// resolved `--config-dir` plus these arguments instead of returning — the
/// platform runner ends in `ExitProcess`, so returning would end the launch
/// with no window. Cancel still returns `None` everywhere.
pub fn run_picker(passthrough: &[OsString]) -> Option<PickerSelection> {
    let names = instance::list_with_mtime();
    let last = instance::read_last();
    let state = PickerState::new(names, last.as_deref());
    let result: Arc<Mutex<Option<Option<PickerSelection>>>> = Arc::new(Mutex::new(None));
    let outcome = result.clone();

    // Locale of the default config: the picker runs before Config loads.
    let language = crate::core::config::Config::load().gui_language.clone();
    // LastWindowClosed everywhere: the OS close button (X) must end the
    // launch with `None` instead of leaving `application.run` parked with
    // zero windows (macOS Default is Explicit and would hang there).
    let application = gpui_platform::application()
        .with_assets(crate::ui::assets::Assets)
        .with_quit_mode(gpui::QuitMode::LastWindowClosed);
    let passthrough = passthrough.to_vec();
    application.run(move |cx| {
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
        // Synchronous open like `main`: opening via `cx.spawn(async { cx.update })`
        // leaves a windowless gap where the foreground executor may never
        // deliver the future and the launch idles with no UI.
        let result = outcome.clone();
        let opened = cx.open_window(options, |window, cx| {
            let view = cx.new(|cx| {
                InstancePicker::new(
                    state.clone(),
                    passthrough.clone(),
                    result.clone(),
                    window,
                    cx,
                )
            });
            let handle = view.read(cx).input.read(cx).focus_handle(cx);
            window.focus(&handle, cx);
            cx.new(|cx| gpui_component::Root::new(view, window, cx))
        });
        if let Err(e) = opened {
            log::error!("instance picker failed to open a window: {e:#}");
            *outcome.lock().unwrap_or_else(|e| e.into_inner()) = Some(None);
            cx.quit();
        }
    });
    result
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
        .flatten()
}

/// Relaunch this executable with the resolved `--config-dir` plus the
/// original launch arguments. Windows-only: the platform runner ends in
/// `ExitProcess`, so the picker cannot hand control back to `main`.
#[cfg(windows)]
fn relaunch_with(choice: &PickerSelection, passthrough: &[OsString]) -> anyhow::Result<()> {
    let dir = choice
        .config_dir()
        .ok_or_else(|| anyhow::anyhow!("no directory resolves for this instance"))?;
    let exe = std::env::current_exe()?;
    let mut cmd = std::process::Command::new(exe);
    cmd.arg("--config-dir").arg(&dir);
    // The picker only runs with no explicit config, so nothing here can
    // collide with the `--config-dir` above — an open path included.
    cmd.args(passthrough);
    cmd.stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    use std::os::windows::process::CommandExt as _;
    // Detached, no console: the picker process exits right after spawning.
    const CREATE_NO_WINDOW: u32 = 0x08000000;
    const DETACHED_PROCESS: u32 = 0x00000008;
    cmd.creation_flags(CREATE_NO_WINDOW | DETACHED_PROCESS);
    cmd.spawn()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_starts_on_the_remembered_row() {
        let names = vec![
            ("bob".to_string(), None),
            ("alice".to_string(), None),
        ];
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
    fn missing_corrupt_and_deleted_memory_preselect_default() {
        let names = vec![("alice".to_string(), None), ("bob".to_string(), None)];
        // Missing memory, the sentinel, an empty (corrupt) value, and a name
        // whose instance is gone all land on Default.
        for last in [None, Some(DEFAULT_SENTINEL), Some(""), Some("ghost")] {
            assert_eq!(
                PickerState::new(names.clone(), last).current(),
                PickerSelection::Default,
                "last={last:?}"
            );
        }
        // A live remembered name still preselects its own row.
        assert_eq!(
            PickerState::new(names, Some("bob")).current(),
            PickerSelection::Named("bob".to_string())
        );
    }

    #[test]
    fn filter_narrows_to_default_plus_matches_and_clear_restores() {
        let names = vec![
            ("alice".to_string(), None),
            ("bob".to_string(), None),
            ("libby".to_string(), None),
        ];
        let mut state = PickerState::new(names, None);
        assert_eq!(state.visible(), vec![0, 1, 2, 3]);
        state.set_query("b");
        // Default always stays on top; only the matches follow it.
        assert_eq!(state.visible(), vec![0, 2, 3]);
        assert_eq!(state.match_count(), 2);
        // The highlight resets to the first match, skipping Default.
        assert_eq!(state.current(), PickerSelection::Named("bob".to_string()));
        // Clearing restores the full list and the open-time highlight.
        state.set_query("");
        assert_eq!(state.visible(), vec![0, 1, 2, 3]);
        assert!(!state.has_query());
    }

    #[test]
    fn clearing_the_filter_restores_the_remembered_highlight() {
        let names = vec![
            ("alice".to_string(), None),
            ("bob".to_string(), None),
        ];
        let mut state = PickerState::new(names, Some("bob"));
        assert_eq!(state.current(), PickerSelection::Named("bob".to_string()));
        state.set_query("x");
        state.set_query("");
        assert_eq!(
            state.current(),
            PickerSelection::Named("bob".to_string()),
            "clearing must restore the preselect, not drop to Default"
        );
    }

    #[test]
    fn filter_is_case_insensitive() {
        let names = vec![("alice".to_string(), None), ("bob".to_string(), None)];
        let mut state = PickerState::new(names, None);
        state.set_query("BOB");
        assert_eq!(state.visible(), vec![0, 2]);
        assert_eq!(state.current(), PickerSelection::Named("bob".to_string()));
    }

    #[test]
    fn no_match_leaves_default_only() {
        let names = vec![("alice".to_string(), None)];
        let mut state = PickerState::new(names, None);
        state.set_query("zzz");
        assert_eq!(state.visible(), vec![0]);
        assert_eq!(state.match_count(), 0);
        assert!(state.has_query());
        assert_eq!(state.current(), PickerSelection::Default);
    }

    #[test]
    fn cursor_wraps_around_the_rows() {
        let mut state = PickerState::new(vec![("alice".to_string(), None)], None);
        assert_eq!(state.row_count(), 2);
        state.move_up();
        assert_eq!(state.current(), PickerSelection::Named("alice".to_string()));
        state.move_down();
        assert_eq!(state.current(), PickerSelection::Default);
    }

    #[test]
    fn highlight_wraps_over_the_visible_rows_only() {
        let names = vec![
            ("alice".to_string(), None),
            ("bob".to_string(), None),
            ("libby".to_string(), None),
        ];
        let mut state = PickerState::new(names, None);
        state.set_query("b");
        assert_eq!(state.current(), PickerSelection::Named("bob".to_string()));
        state.move_up();
        // Wrapping runs over every visible row, Default on top included.
        assert_eq!(state.current(), PickerSelection::Default);
        state.move_up();
        assert_eq!(
            state.current(),
            PickerSelection::Named("libby".to_string())
        );
        state.move_down();
        assert_eq!(state.current(), PickerSelection::Default);
        state.move_down();
        assert_eq!(state.current(), PickerSelection::Named("bob".to_string()));
    }

    #[test]
    fn confirm_follows_the_highlight_and_never_creates() {
        let names = vec![("alice".to_string(), None), ("bob".to_string(), None)];
        let mut state = PickerState::new(names, None);
        state.set_query("b");
        assert_eq!(
            state.confirm(),
            PickerSelection::Named("bob".to_string())
        );
        // A valid but unlisted name is not entered: with nothing matching,
        // the highlight sits on Default and that is what confirms.
        state.set_query("carol");
        assert!(instance::validate_name("carol").is_ok());
        assert_eq!(state.confirm(), PickerSelection::Default);
    }

    #[test]
    fn invalid_input_only_comes_from_validate_name() {
        // The view blocks Enter on exactly this rule — never a re-encoded
        // copy of it.
        assert!(instance::validate_name("Bob!").is_err());
        assert!(instance::validate_name("").is_err());
        assert!(instance::validate_name("bob-2").is_ok());
    }

    #[test]
    fn esc_closes_the_dropdown_before_cancelling() {
        let mut state = PickerState::new(vec![("alice".to_string(), None)], None);
        assert!(state.is_dropdown_open());
        assert_eq!(state.esc_layer(), EscLayer::CloseDropdown);
        assert!(!state.is_dropdown_open());
        assert_eq!(state.esc_layer(), EscLayer::CancelWindow);
    }

    #[test]
    fn rows_retain_snapshot_mtimes() {
        use std::time::Duration;
        let t = UNIX_EPOCH + Duration::from_secs(1_000);
        let state = PickerState::new(vec![("alice".to_string(), Some(t))], None);
        assert_eq!(state.rows[0].mtime, Some(t));
        let state = PickerState::new(vec![("bob".to_string(), None)], None);
        assert_eq!(state.rows[0].mtime, None);
    }

    #[test]
    fn rows_retain_snapshot_mtimes_through_filter() {
        use std::time::Duration;
        let t = UNIX_EPOCH + Duration::from_secs(1_000);
        let mut state = PickerState::new(
            vec![
                ("alice".to_string(), Some(t)),
                ("bob".to_string(), None),
            ],
            None,
        );
        state.set_query("a");
        assert_eq!(state.visible(), vec![0, 1]);
        // Filtering reorders nothing and drops no metadata: the snapshot
        // mtimes survive behind the visible rows.
        assert_eq!(state.rows[0].mtime, Some(t));
        assert_eq!(state.rows[1].mtime, None);
        assert_eq!(
            state.current(),
            PickerSelection::Named("alice".to_string())
        );
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
