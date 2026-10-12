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
//!   a name that matches nothing never creates it — creation happens only
//!   through the explicit create button, which enables solely for a valid,
//!   unlisted name. The
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
use gpui_component::{Disableable as _, h_flex, v_flex};
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

/// The pending delete-confirm modal: one deep at most, over the picker.
///
/// `name` is the invoking row (and the only row the modal can act on);
/// `armed` turns true on the first confirm of an unevaluable probe, and the
/// second confirm then proceeds through the core force channel; `error` is
/// the retryable failure line, set when the core op returns `Err` (the row
/// and the memory stay untouched while it is set).
#[derive(Debug, Clone)]
pub struct PendingDelete {
    name: String,
    armed: bool,
    error: Option<String>,
}

impl PendingDelete {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn is_armed(&self) -> bool {
        self.armed
    }

    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }
}

/// The selectable rows: the default instance first, then existing names
/// with their snapshot mtimes for recency display.
///
/// The picker is a combobox: the input filters these rows (case-insensitive
/// substring; the Default row always stays on top) and the highlight moves
/// over the visible rows only, wrapping at the ends.
///
/// The delete-confirm modal hangs off the same state: at most one pending
/// delete, and while it is set the picker behind is inert (the view routes
/// Enter/arrows to the modal and the scrim swallows clicks).
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
    pending_delete: Option<PendingDelete>,
}

/// What the next Esc press should do: layers close topmost-first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EscLayer {
    CloseDeleteModal,
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
            pending_delete: None,
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

    /// The raw filter text the list is currently narrowed by.
    pub fn query(&self) -> &str {
        &self.query
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

    /// Esc closes the open delete modal first (highlight returns to the
    /// invoking row), then the dropdown (text and highlight are kept by
    /// the caller); only a third press with both closed cancels the window.
    pub fn esc_layer(&mut self) -> EscLayer {
        if self.pending_delete.is_some() {
            self.cancel_delete();
            return EscLayer::CloseDeleteModal;
        }
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

    /// The explicit create candidate: the trimmed query when it is a
    /// valid, unlisted instance name, otherwise `None`. Validity comes
    /// only from [`instance::validate_name`]; unlisted means absent from
    /// the snapshot rows. The view enables the create button exactly on
    /// `Some`, and the click path revalidates through this same method.
    pub fn create_candidate(&self, query: &str) -> Option<String> {
        let trimmed = query.trim();
        if trimmed.is_empty() {
            return None;
        }
        let clean = instance::validate_name(trimmed).ok()?;
        if self.rows.iter().any(|r| r.name == clean) {
            return None;
        }
        Some(clean)
    }

    /// Confirm the highlighted row. The typed filter never wins and never
    /// creates: a valid but unlisted name is not entered here (the explicit
    /// create path owns that). Validity itself still comes only from
    /// [`instance::validate_name`], enforced by the view's invalid line.
    pub fn confirm(&self) -> PickerSelection {
        self.current()
    }

    /// Whether the global row `index` carries a delete affordance: every
    /// named row does, the Default row (0, no directory to delete) never
    /// does. The view renders the × exactly on this rule.
    pub fn is_deletable(&self, index: usize) -> bool {
        index != 0 && index <= self.rows.len()
    }

    pub fn pending_delete(&self) -> Option<&PendingDelete> {
        self.pending_delete.as_ref()
    }

    /// Open the delete modal for the global row `index`. Returns false and
    /// touches nothing when the row is not deletable (Default, out of
    /// range) — the × that would call this is never rendered there, so
    /// false is a caller bug, not a user error. Opening moves the highlight
    /// to the invoking row and starts unarmed with no error line.
    pub fn open_delete(&mut self, index: usize) -> bool {
        if !self.is_deletable(index) {
            return false;
        }
        let name = self.rows[index - 1].name.clone();
        if let Some(position) = self.visible().iter().position(|&g| g == index) {
            self.cursor = position;
        }
        self.pending_delete = Some(PendingDelete {
            name,
            armed: false,
            error: None,
        });
        true
    }

    /// Arm the pending modal after its first confirm over an unevaluable
    /// probe. The second confirm then proceeds through the core force
    /// channel; a single confirm only arms and removes nothing.
    pub fn arm_delete(&mut self) {
        if let Some(pending) = self.pending_delete.as_mut() {
            pending.armed = true;
        }
    }

    /// Record a failed core remove: the modal stays open with the retryable
    /// error line, and the row and the memory stay untouched by
    /// construction — this writes only the line.
    pub fn fail_delete(&mut self, message: String) {
        if let Some(pending) = self.pending_delete.as_mut() {
            pending.error = Some(message);
        }
    }

    /// Close the modal without removing anything: the highlight returns to
    /// the invoking row (or its visible position, when the filter still
    /// shows it) and nothing is written anywhere.
    pub fn cancel_delete(&mut self) {
        if let Some(pending) = self.pending_delete.take() {
            if let Some(position) = self.visible().iter().position(|&g| {
                g != 0 && self.rows.get(g - 1).is_some_and(|r| r.name == pending.name)
            }) {
                self.cursor = position;
            }
        }
        self.clamp_cursor();
    }

    /// Finish a confirmed delete whose core op returned `Ok`: drop the
    /// invoking row, close the modal, and fall the highlight to the next
    /// row (or the previous one when the last row went). Returns the
    /// focused selection so the caller can rewrite memory to it strictly
    /// after `Ok` — never a deleted name, `DEFAULT_SENTINEL` when focus
    /// lands on Default.
    ///
    /// Call this only after the core op succeeds; on `Err` use
    /// [`PickerState::fail_delete`] instead so the row survives.
    pub fn complete_delete(&mut self) -> PickerSelection {
        if let Some(pending) = self.pending_delete.take() {
            if let Some(global) = self.rows.iter().position(|r| r.name == pending.name) {
                self.rows.remove(global);
                let removed = global + 1;
                // The preselect tracked a live row: rows after the gap slide
                // down one, and a preselect that pointed at the deleted row
                // now points at whatever slid into its place (clamped to the
                // previous row when the last one went).
                if self.preselect > removed {
                    self.preselect -= 1;
                }
                self.preselect = self.preselect.min(self.rows.len());
            }
        }
        // The cursor sat on the invoking row (see `open_delete`), so holding
        // the visible position lands on the next row; clamping covers the
        // deleted-last case.
        self.clamp_cursor();
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
                    // The picker behind an open delete modal is inert:
                    // keystrokes still land in the focused input, so put
                    // the displayed text back to the applied query instead
                    // of re-filtering (set_value emits no Change, so this
                    // cannot loop). The box then shows only what the list
                    // was narrowed by, before and after cancel.
                    if this.state.pending_delete().is_some() {
                        let applied = this.state.query().to_string();
                        this.input.update(cx, |input, cx| {
                            input.set_value(applied, window, cx)
                        });
                        cx.notify();
                        return;
                    }
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
        // With the delete modal up, Enter belongs to it — the launch must
        // not confirm underneath (this also covers the Input PressEnter
        // path, which bypasses the keybind action below).
        if self.state.pending_delete().is_some() {
            self.confirm_delete(window, cx);
            return;
        }
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

    /// Explicit creation: the button enables only on
    /// [`PickerState::create_candidate`]; the click revalidates through the
    /// same method and enters the new instance. Directory creation and memory
    /// write happen downstream in `apply_selection`, so this stays a pure
    /// selection like every other confirm path.
    fn create(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // The picker behind the modal is inert: no creating over it.
        if self.state.pending_delete().is_some() {
            return;
        }
        let typed = self.input.read(cx).value().to_string();
        let Some(name) = self.state.create_candidate(&typed) else {
            cx.notify();
            return;
        };
        self.finish(Some(PickerSelection::Named(name)), window, cx);
    }

    fn cancel(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // Esc layers: first the delete modal (highlight back on the invoking
        // row), then the dropdown (text and highlight kept), then the
        // launch itself.
        match self.state.esc_layer() {
            EscLayer::CloseDeleteModal => {
                self.focus_input(window, cx);
                cx.notify();
                return;
            }
            EscLayer::CloseDropdown => {
                cx.notify();
                return;
            }
            EscLayer::CancelWindow => {}
        }
        self.finish(None, window, cx);
    }

    /// Keyboard focus back to the filter input (the modal's focus returns
    /// there on every close path; the highlight is owned by `PickerState`).
    fn focus_input(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let handle = self.input.read(cx).focus_handle(cx);
        window.focus(&handle, cx);
    }

    /// Open the delete modal for the global row `index` (the × click).
    /// The click must not select or confirm the row — the caller stops
    /// propagation first (the tab-strip close-button precedent).
    fn open_delete(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if self.state.open_delete(index) {
            // The modal renders over the picker; keep keyboard focus where
            // typing already goes so Esc/Enter reach the modal paths below.
            self.focus_input(window, cx);
            cx.notify();
        }
    }

    /// Close the delete modal with nothing removed and nothing written,
    /// highlight back on the invoking row.
    fn cancel_delete(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.state.cancel_delete();
        self.focus_input(window, cx);
        cx.notify();
    }

    /// Run the modal's confirm path for the pending row:
    ///
    /// - live seat (advisory probe): no confirm action exists — Enter is
    ///   inert here, exactly like the missing button.
    /// - unevaluable probe: the first confirm arms, the second proceeds
    ///   through the core force channel.
    /// - free seat: the core op runs unforced.
    ///
    /// On `Ok` the row drops, memory is rewritten to the focused row, and
    /// the highlight falls to the next row; on `Err` the modal stays open
    /// with the error line and the row and the memory are untouched.
    fn confirm_delete(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let (name, armed) = match self.state.pending_delete() {
            Some(pending) => (pending.name().to_string(), pending.is_armed()),
            None => return,
        };
        let Some(dir) = instance::dir_for(&name) else {
            self.state.fail_delete(
                t_fmt(L10nKey::InstancePickerDeleteNoDir, &[("name", name.as_str())])
                    .to_string(),
            );
            cx.notify();
            return;
        };
        match instance::holder_pid_in(&dir) {
            instance::SeatState::Live { .. } => {
                // No confirm action: the report is the whole modal.
            }
            instance::SeatState::Unknown if !armed => {
                self.state.arm_delete();
                cx.notify();
            }
            seat => {
                let force = seat == instance::SeatState::Unknown;
                match instance::remove_instance(&name, force) {
                    Ok(()) => {
                        let focused = self.state.complete_delete();
                        // Strictly after `Ok`, to the focused row (AD-7):
                        // never a deleted name.
                        let _ = instance::write_last(focused.memory_name());
                        self.focus_input(window, cx);
                        cx.notify();
                    }
                    Err(e) => {
                        self.state.fail_delete(e.to_string());
                        cx.notify();
                    }
                }
            }
        }
    }

    /// Advisory pre-confirm probe behind the modal report: the directory
    /// for the pending name plus its seat. The in-op probe inside
    /// `remove_instance` stays authoritative; this one only decides which
    /// report the modal renders.
    fn delete_probe(&self) -> Option<(PathBuf, instance::SeatState)> {
        let name = self.state.pending_delete()?.name().to_string();
        let dir = instance::dir_for(&name)?;
        let seat = instance::holder_pid_in(&dir);
        Some((dir, seat))
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
        let column = v_flex()
            .on_action(
                cx.listener(|this, _: &ConfirmInstance, window, cx| this.confirm(window, cx)),
            )
            .on_action(cx.listener(|this, _: &CancelInstance, window, cx| this.cancel(window, cx)))
            // The combobox owns the arrows: they always move the highlight,
            // even with the caret sitting in filtered text. A closed
            // dropdown reopens on either key. With the delete modal up the
            // picker behind is inert: both keys are swallowed.
            .on_action(cx.listener(|this, _: &PickerUp, _window, cx| {
                if this.state.pending_delete().is_some() {
                    return;
                }
                this.state.open_dropdown();
                this.state.move_up();
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &PickerDown, _window, cx| {
                if this.state.pending_delete().is_some() {
                    return;
                }
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
                        // Behind the modal the picker is inert: no
                        // reopening the dropdown under the overlay.
                        if this.state.pending_delete().is_some() {
                            return;
                        }
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
                            let option_label = self.row_accessible_label(index, position, total);
                            // Named rows only: the Default row has no
                            // directory to delete, so it never gets a ×.
                            let deletable = self.state.is_deletable(index);
                            let delete_name = (index != 0)
                                .then(|| self.state.rows[index - 1].name.clone());
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
                                    // Inert behind the modal (the scrim
                                    // normally swallows this first).
                                    if this.state.pending_delete().is_some() {
                                        return;
                                    }
                                    // A click selects the row outright: stale
                                    // invalid filter text vetoes Enter and
                                    // creation, never a direct selection.
                                    this.state.cursor = position;
                                    let choice = this.state.confirm();
                                    this.finish(Some(choice), window, cx);
                                }))
                                .child(label)
                                .children(meta.map(|m| div().text_size(px(12.)).child(m)))
                                .when(deletable, |row| {
                                    row.child(
                                        div()
                                            .id(("instance-picker-delete", index))
                                            .role(gpui::Role::Button)
                                            .aria_label(t_fmt(
                                                L10nKey::InstancePickerDeleteRow,
                                                &[(
                                                    "name",
                                                    delete_name.as_deref().unwrap_or_default(),
                                                )],
                                            ))
                                            // A larger hit target than the
                                            // glyph, per the touch floor.
                                            .px(px(8.))
                                            .py(px(4.))
                                            .rounded(px(6.))
                                            .cursor_pointer()
                                            .text_size(px(14.))
                                            .child("×")
                                            .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| {
                                                cx.stop_propagation()
                                            })
                                            .on_click(cx.listener(move |this, _, window, cx| {
                                                // The × opens the modal
                                                // only: holding the press
                                                // back keeps it from
                                                // reaching the row
                                                // behind, which would
                                                // confirm it (the
                                                // tab-strip close-button
                                                // precedent).
                                                cx.stop_propagation();
                                                this.open_delete(index, window, cx);
                                            })),
                                    )
                                })
                        })),
                    &self.list_scroll,
                ))
            })
            .when(dropdown_open && filtering && match_count > 0, |column| {
                column.child(div().text_size(px(12.)).child(t_plural(
                    L10nKey::InstancePickerMatchCount,
                    match_count,
                    &[],
                )))
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
            .when(filtering, |column| {
                // The explicit create path: enabled only for a valid,
                // unlisted name; otherwise a disabled affordance that
                // explains nothing beyond its label.
                let typed = input.read(cx).value().to_string();
                let candidate = self.state.create_candidate(&typed);
                column.child(
                    Button::new("instance-picker-create")
                        .label(t(L10nKey::InstancePickerCreate))
                        .primary()
                        .disabled(candidate.is_none())
                        .on_click(cx.listener(|this, _, window, cx| this.create(window, cx))),
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
            .track_focus(&self.focus);
        // The delete modal stacks one deep over the picker (never a modal
        // over the modal): an in-view scrim + card overlay, leaving the
        // window size untouched.
        let modal = self
            .state
            .pending_delete()
            .cloned()
            .map(|pending| self.render_delete_modal(&pending, cx));
        div().relative().w_full().child(column).children(modal)
    }
}

impl InstancePicker {
    /// The delete-confirm modal: an in-view overlay (scrim + dialog card,
    /// the ssh-prompt sheet precedent). Scrim, Esc, and Cancel close it
    /// with nothing removed and nothing written; Confirm runs
    /// [`InstancePicker::confirm_delete`]. Live seats get the actionable
    /// report with no confirm action; unknown probes need the double
    /// confirm; failures surface as a retryable error line with the modal
    /// staying open.
    fn render_delete_modal(
        &self,
        pending: &PendingDelete,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let name = pending.name().to_string();
        let armed = pending.is_armed();
        let error = pending.error().map(str::to_string);
        let title = t_fmt(
            L10nKey::InstancePickerDeleteTitle,
            &[("name", name.as_str())],
        );
        let rungs = crate::ui::dialog::popover_rungs(cx);

        // Advisory only: the in-op probe inside `remove_instance` is what
        // refuses or unlinks. `None` means the directory stopped resolving
        // mid-modal — then there is no way through at all.
        let probe = self.delete_probe();
        let seat = probe.as_ref().map(|(_, seat)| *seat);
        let confirmable = matches!(
            seat,
            Some(instance::SeatState::Free) | Some(instance::SeatState::Unknown)
        );

        let mut body = crate::ui::dialog::body();
        match (&probe, seat) {
            (Some((dir, instance::SeatState::Live { pid })), _) => {
                let dir = dir.display().to_string();
                // "?" is locale-neutral: no locale has a word for an
                // unrecorded pid, and the flock outcome (held) is what is
                // authoritative here, not the number.
                let pid = pid
                    .map(|p| p.to_string())
                    .unwrap_or_else(|| "?".to_string());
                let stop = format!("tty7 --as {name} server stop");
                let rm = format!(
                    "rm -rf {}",
                    tty7_core::core::shell_quote::quote_for_shell(&dir, None)
                );
                body = body.child(div().text_size(px(12.)).child(t_fmt(
                    L10nKey::InstancePickerDeleteLiveReport,
                    &[
                        ("dir", dir.as_str()),
                        ("pid", pid.as_str()),
                        ("stop", stop.as_str()),
                        ("rm", rm.as_str()),
                    ],
                )));
            }
            (Some((dir, instance::SeatState::Unknown)), _) => {
                let _ = dir;
                body = body.child(div().text_size(px(12.)).child(t_fmt(
                    L10nKey::InstancePickerDeleteUnknownBody,
                    &[("name", name.as_str())],
                )));
            }
            (Some((dir, instance::SeatState::Free)), _) => {
                let dir = dir.display().to_string();
                body = body.child(div().text_size(px(12.)).child(t_fmt(
                    L10nKey::InstancePickerDeleteBody,
                    &[("dir", dir.as_str())],
                )));
            }
            (None, _) => {
                body = body.child(div().text_size(px(12.)).child(t_fmt(
                    L10nKey::InstancePickerDeleteNoDir,
                    &[("name", name.as_str())],
                )));
            }
        }
        // The retryable failure line: the modal stays open, the row stays,
        // and memory is untouched (see `confirm_delete`).
        if let Some(message) = error {
            body = body.child(
                div()
                    .text_size(px(12.))
                    .text_color(gpui::rgb(0xC0392B))
                    .child(t_fmt(
                        L10nKey::InstancePickerDeleteError,
                        &[("error", message.as_str())],
                    )),
            );
        }

        let mut footer = crate::ui::dialog::footer(cx).child(crate::ui::dialog::button(
            "instance-picker-delete-cancel",
            t(L10nKey::InstancePickerDeleteCancel),
            crate::ui::dialog::Tone::Secondary,
            true,
            rungs,
            cx,
            cx.listener(|this, _, window, cx| this.cancel_delete(window, cx)),
        ));
        if confirmable {
            // The first unknown confirm only arms (primary); the armed
            // re-confirm and the free confirm remove permanently (danger).
            let (id, label, tone) = match (seat, armed) {
                (Some(instance::SeatState::Unknown), false) => (
                    "instance-picker-delete-confirm",
                    t(L10nKey::InstancePickerDeleteConfirm).to_string(),
                    crate::ui::dialog::Tone::Primary,
                ),
                (Some(instance::SeatState::Unknown), true) => (
                    "instance-picker-delete-confirm-again",
                    t(L10nKey::InstancePickerDeleteConfirmAgain).to_string(),
                    crate::ui::dialog::Tone::Danger,
                ),
                _ => (
                    "instance-picker-delete-confirm",
                    t(L10nKey::InstancePickerDeleteConfirm).to_string(),
                    crate::ui::dialog::Tone::Danger,
                ),
            };
            footer = footer.child(crate::ui::dialog::button(
                id,
                label,
                tone,
                true,
                rungs,
                cx,
                cx.listener(|this, _, window, cx| this.confirm_delete(window, cx)),
            ));
        }

        let card = crate::ui::dialog::card("instance-picker-delete-card", title.clone(), 340., cx)
            .child(crate::ui::dialog::header(title, cx))
            .child(body)
            .child(footer);
        div()
            .absolute()
            .inset_0()
            .bg(crate::ui::presets::scrim_fill(cx))
            // The scrim backs out (the worktree-prompt precedent); the card
            // occludes, so clicks on it never reach here.
            .on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(|this, _, window, cx| this.cancel_delete(window, cx)),
            )
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .p(px(16.))
            .child(card)
            .into_any_element()
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
        let names = vec![("bob".to_string(), None), ("alice".to_string(), None)];
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
        let names = vec![("alice".to_string(), None), ("bob".to_string(), None)];
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
        assert_eq!(state.current(), PickerSelection::Named("libby".to_string()));
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
        assert_eq!(state.confirm(), PickerSelection::Named("bob".to_string()));
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
    fn delete_modal_opens_unarmed_on_the_invoking_row() {
        let names = vec![("alice".to_string(), None), ("bob".to_string(), None)];
        let mut state = PickerState::new(names, None);
        assert_eq!(state.current(), PickerSelection::Default);
        assert!(state.open_delete(2));
        let pending = state.pending_delete().expect("modal must be pending");
        assert_eq!(pending.name(), "bob");
        assert!(!pending.is_armed());
        assert_eq!(pending.error(), None);
        // The highlight moved to the invoking row with it.
        assert_eq!(state.current(), PickerSelection::Named("bob".to_string()));
    }

    #[test]
    fn delete_modal_never_opens_on_default_or_air() {
        let names = vec![("alice".to_string(), None)];
        let mut state = PickerState::new(names, None);
        assert!(!state.is_deletable(0), "the Default row has no directory");
        assert!(state.is_deletable(1));
        assert!(!state.is_deletable(2), "out of range is not deletable");
        assert!(!state.is_deletable(99));
        assert!(!state.open_delete(0));
        assert!(!state.open_delete(99));
        assert!(state.pending_delete().is_none());
        assert_eq!(state.current(), PickerSelection::Default);
    }

    #[test]
    fn esc_closes_the_delete_modal_before_the_dropdown() {
        let names = vec![("alice".to_string(), None), ("bob".to_string(), None)];
        let mut state = PickerState::new(names, Some("alice"));
        assert!(state.open_delete(2));
        assert!(state.is_dropdown_open());
        // Modal first: it closes, the highlight is back on the invoking
        // row, and the dropdown stays exactly as it was.
        assert_eq!(state.esc_layer(), EscLayer::CloseDeleteModal);
        assert!(state.pending_delete().is_none());
        assert_eq!(state.current(), PickerSelection::Named("bob".to_string()));
        assert!(state.is_dropdown_open());
        assert_eq!(state.esc_layer(), EscLayer::CloseDropdown);
        assert_eq!(state.esc_layer(), EscLayer::CancelWindow);
    }

    #[test]
    fn unknown_first_confirm_only_arms() {
        let names = vec![("alice".to_string(), None)];
        let mut state = PickerState::new(names, None);
        assert!(state.open_delete(1));
        assert!(!state.pending_delete().unwrap().is_armed());
        state.arm_delete();
        assert!(state.pending_delete().unwrap().is_armed());
        // Arming removes nothing: the row is still there.
        assert_eq!(state.rows.len(), 1);
    }

    #[test]
    fn failed_delete_keeps_the_row_and_the_memory_value() {
        let names = vec![("alice".to_string(), None), ("bob".to_string(), None)];
        let mut state = PickerState::new(names, Some("bob"));
        assert!(state.open_delete(2));
        state.arm_delete();
        state.fail_delete(
            "instance is live (server pid 123, advisory); stop that instance's server first"
                .to_string(),
        );
        // The modal stays open with the retryable line…
        let pending = state.pending_delete().expect("modal must stay open");
        assert!(pending.is_armed());
        assert!(pending.error().is_some());
        // …the row stays, and the focused memory value is untouched.
        assert_eq!(state.rows.len(), 2);
        assert_eq!(state.current(), PickerSelection::Named("bob".to_string()));
        assert_eq!(
            state.current().memory_name(),
            "bob",
            "memory would still name the live row, never a deletion"
        );
    }

    #[test]
    fn complete_delete_falls_to_next_and_names_live_memory() {
        let names = vec![
            ("alice".to_string(), None),
            ("bob".to_string(), None),
            ("cara".to_string(), None),
        ];
        let mut state = PickerState::new(names, Some("alice"));
        assert!(state.open_delete(1));
        let focused = state.complete_delete();
        // The row is gone, the modal with it, and the highlight sits on the
        // next row — whose value memory must take.
        assert_eq!(focused, PickerSelection::Named("bob".to_string()));
        assert_eq!(state.current(), focused);
        assert_eq!(focused.memory_name(), "bob");
        assert!(state.pending_delete().is_none());
        assert_eq!(state.visible(), vec![0, 1, 2]);
    }

    #[test]
    fn complete_delete_at_the_end_falls_to_previous() {
        let names = vec![("alice".to_string(), None), ("bob".to_string(), None)];
        let mut state = PickerState::new(names, Some("bob"));
        assert!(state.open_delete(2));
        let focused = state.complete_delete();
        assert_eq!(focused, PickerSelection::Named("alice".to_string()));
        assert_eq!(state.current(), focused);
        assert_eq!(focused.memory_name(), "alice");
    }

    #[test]
    fn complete_delete_of_the_last_named_row_lands_on_default() {
        let names = vec![("alice".to_string(), None)];
        let mut state = PickerState::new(names, Some("alice"));
        assert!(state.open_delete(1));
        let focused = state.complete_delete();
        // Nothing dangles: focus is Default and memory takes the sentinel.
        assert_eq!(focused, PickerSelection::Default);
        assert_eq!(focused.memory_name(), DEFAULT_SENTINEL);
        assert_eq!(state.visible(), vec![0]);
    }

    #[test]
    fn cancel_delete_returns_to_the_invoking_row_and_writes_nothing() {
        let names = vec![("alice".to_string(), None), ("bob".to_string(), None)];
        let mut state = PickerState::new(names, None);
        assert!(state.open_delete(1));
        // The highlight drifts (arrows are inert behind the modal in the
        // view, but the state restores regardless); cancelling puts it back
        // on the invoking row with the row intact.
        state.move_down();
        state.cancel_delete();
        assert!(state.pending_delete().is_none());
        assert_eq!(state.current(), PickerSelection::Named("alice".to_string()));
        assert_eq!(state.rows.len(), 2);
    }

    #[test]
    fn delete_modal_tracks_the_invoking_row_under_a_filter() {
        let names = vec![
            ("alice".to_string(), None),
            ("bob".to_string(), None),
            ("libby".to_string(), None),
        ];
        let mut state = PickerState::new(names, None);
        state.set_query("b");
        assert_eq!(state.visible(), vec![0, 2, 3]);
        // libby is global row 3; opening its modal highlights it.
        assert!(state.open_delete(3));
        assert_eq!(state.current(), PickerSelection::Named("libby".to_string()));
        // Completing removes libby while the filter still holds: the
        // highlight stays valid and names a live row.
        let focused = state.complete_delete();
        assert_eq!(state.rows.len(), 2);
        assert!(matches!(
            focused,
            PickerSelection::Named(_) | PickerSelection::Default
        ));
    }

    #[test]
    fn create_candidate_accepts_only_valid_unlisted_names() {
        let names = vec![("alice".to_string(), None), ("bob".to_string(), None)];
        let state = PickerState::new(names, None);
        // Fresh and valid: the button enables.
        assert_eq!(state.create_candidate("carol"), Some("carol".to_string()));
        assert_eq!(
            state.create_candidate("  carol-2  "),
            Some("carol-2".to_string())
        );
        // Listed, empty, or invalid: no candidate, button stays disabled.
        assert_eq!(state.create_candidate("alice"), None);
        assert_eq!(state.create_candidate("bob"), None);
        assert_eq!(state.create_candidate(""), None);
        assert_eq!(state.create_candidate("   "), None);
        for bad in ["Bob!", "default", "-x", "../x", "a/b"] {
            assert_eq!(
                state.create_candidate(bad),
                None,
                "{bad:?} must never be a create candidate"
            );
        }
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
            vec![("alice".to_string(), Some(t)), ("bob".to_string(), None)],
            None,
        );
        state.set_query("a");
        assert_eq!(state.visible(), vec![0, 1]);
        // Filtering reorders nothing and drops no metadata: the snapshot
        // mtimes survive behind the visible rows.
        assert_eq!(state.rows[0].mtime, Some(t));
        assert_eq!(state.rows[1].mtime, None);
        assert_eq!(state.current(), PickerSelection::Named("alice".to_string()));
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
