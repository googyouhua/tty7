use tty7_core::core::onekey::{OneKeyEntry, OneKeyFill};

/// Manual OneKey picker state, anchored to a `TerminalView` like
/// reverse-search: filters saved entries, then a fill choice. Cancel at any
/// step sends zero bytes (A3).
#[derive(Debug, Clone)]
pub struct OneKeyPicker {
    entries: Vec<OneKeyEntry>,
    filter: String,
    cursor: usize,
    chosen: Option<usize>,
    stage: Stage,
}

/// Which step the picker is on: picking an entry, then picking what to fill.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    PickEntry,
    PickFill,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OneKeyAction {
    /// Keep interacting (filter/cursor changed).
    Pending,
    /// Cancelled — caller must send zero bytes.
    Cancelled,
    /// Ready to fill with these bytes.
    Fill(Vec<u8>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiAction {
    Redraw,
    Cancel,
    Fill(OneKeyFill),
}

impl OneKeyPicker {
    pub fn new(mut entries: Vec<OneKeyEntry>) -> Self {
        entries.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));
        Self {
            entries,
            filter: String::new(),
            cursor: 0,
            chosen: None,
            stage: Stage::PickEntry,
        }
    }

    pub fn stage(&self) -> Stage {
        self.stage
    }

    pub fn filter_text(&self) -> &str {
        &self.filter
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn push_filter_char(&mut self, ch: &str) {
        self.filter.push_str(ch);
        self.cursor = 0;
        self.chosen = None;
    }

    pub fn pop_filter(&mut self) {
        self.filter.pop();
        self.cursor = 0;
        self.chosen = None;
    }

    pub fn set_filter(&mut self, filter: String) {
        self.filter = filter;
        self.cursor = 0;
        self.chosen = None;
        self.stage = Stage::PickEntry;
    }

    pub fn filtered(&self) -> Vec<usize> {
        self.entries
            .iter()
            .enumerate()
            .filter(|(_, e)| e.matches_filter(&self.filter))
            .map(|(i, _)| i)
            .collect()
    }

    /// The entries the overlay paints, in display order.
    pub fn filtered_entries(&self) -> Vec<&OneKeyEntry> {
        let list = self.filtered();
        list.into_iter()
            .filter_map(|i| self.entries.get(i))
            .collect()
    }

    pub fn move_cursor(&mut self, delta: isize) {
        let n = self.filtered().len();
        if n == 0 {
            self.cursor = 0;
            return;
        }
        let cur = self.cursor.min(n - 1) as isize;
        let next = (cur + delta).rem_euclid(n as isize);
        self.cursor = next as usize;
    }

    /// Confirm the highlighted entry; returns false when the list is empty.
    pub fn choose(&mut self) -> bool {
        let list = self.filtered();
        if list.is_empty() {
            return false;
        }
        let idx = list[self.cursor.min(list.len() - 1)];
        self.chosen = Some(idx);
        self.stage = Stage::PickFill;
        true
    }

    /// Step back from the fill choice to the entry list.
    pub fn unchoose(&mut self) {
        self.chosen = None;
        self.stage = Stage::PickEntry;
    }

    pub fn chosen_entry(&self) -> Option<&OneKeyEntry> {
        self.chosen.and_then(|i| self.entries.get(i))
    }

    pub fn fill(&self, fill: OneKeyFill) -> OneKeyAction {
        match self.chosen_entry() {
            Some(e) => OneKeyAction::Fill(e.fill_bytes(fill)),
            None => OneKeyAction::Cancelled,
        }
    }

    pub fn cancel() -> OneKeyAction {
        OneKeyAction::Cancelled
    }

    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    /// Drive one keystroke while the picker owns the pane's keys, modeled on
    /// [`super::reverse_search::ReverseSearch::handle_key`]. Printable filter
    /// text is pushed by the caller (it owns `key_char` filtering); this owns
    /// navigation, choice, fill selection, and cancel.
    pub fn handle_key(&mut self, ks: &gpui::Keystroke) -> UiAction {
        let m = &ks.modifiers;
        let key = ks.key.as_str();
        if (m.control && (key == "g" || key == "c")) || key == "escape" {
            return UiAction::Cancel;
        }
        match self.stage {
            Stage::PickEntry => {
                if key == "up" || (m.control && key == "p") {
                    self.move_cursor(-1);
                    UiAction::Redraw
                } else if key == "down" || (m.control && key == "n") {
                    self.move_cursor(1);
                    UiAction::Redraw
                } else if key == "enter" || (m.control && (key == "j" || key == "m")) {
                    self.choose();
                    UiAction::Redraw
                } else if key == "backspace" {
                    self.pop_filter();
                    UiAction::Redraw
                } else {
                    UiAction::Redraw
                }
            }
            Stage::PickFill => {
                if m.control || m.platform || m.alt {
                    UiAction::Redraw
                } else {
                    match key {
                        "u" => UiAction::Fill(OneKeyFill::Username),
                        "p" => UiAction::Fill(OneKeyFill::Password),
                        "b" | "enter" => UiAction::Fill(OneKeyFill::UsernameAndPassword),
                        "backspace" => {
                            self.unchoose();
                            UiAction::Redraw
                        }
                        _ => UiAction::Redraw,
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tty7_core::core::onekey::OneKeyKind;

    fn entries() -> Vec<OneKeyEntry> {
        vec![
            OneKeyEntry {
                id: "1".into(),
                kind: OneKeyKind::Account,
                title: "prod".into(),
                username: "alice".into(),
                password: "pw1".into(),
                host_binding: None,
            },
            OneKeyEntry {
                id: "2".into(),
                kind: OneKeyKind::SshBound,
                title: "staging".into(),
                username: "bob".into(),
                password: "pw2".into(),
                host_binding: Some("10.0.0.5".into()),
            },
        ]
    }

    #[test]
    fn filter_narrows_and_cursor_wraps() {
        let mut p = OneKeyPicker::new(entries());
        assert_eq!(p.filtered().len(), 2);
        p.set_filter("prod".into());
        assert_eq!(p.filtered().len(), 1);
        p.move_cursor(1);
        assert_eq!(p.cursor, 0);
    }

    #[test]
    fn choose_then_fill_username_password_sequence() {
        let mut p = OneKeyPicker::new(entries());
        p.set_filter("stag".into());
        assert!(p.choose());
        assert_eq!(p.chosen_entry().unwrap().username, "bob");
        match p.fill(OneKeyFill::Username) {
            OneKeyAction::Fill(b) => assert_eq!(b, b"bob"),
            _ => panic!("expected fill"),
        }
        match p.fill(OneKeyFill::Password) {
            OneKeyAction::Fill(b) => assert_eq!(b, b"pw2"),
            _ => panic!("expected fill"),
        }
        match p.fill(OneKeyFill::UsernameAndPassword) {
            OneKeyAction::Fill(b) => assert_eq!(b, b"bob\rpw2"),
            _ => panic!("expected fill"),
        }
    }

    #[test]
    fn cancel_sends_zero_bytes() {
        match OneKeyPicker::cancel() {
            OneKeyAction::Cancelled => {}
            _ => panic!("expected cancel"),
        }
        let p = OneKeyPicker::new(entries());
        match p.fill(OneKeyFill::Password) {
            OneKeyAction::Cancelled => {}
            _ => panic!("no choice means cancel"),
        }
    }

    #[test]
    fn empty_list_cannot_choose() {
        let mut p = OneKeyPicker::new(vec![]);
        assert!(!p.choose());
    }

    fn key(spec: &str) -> gpui::Keystroke {
        gpui::Keystroke::parse(spec).expect("valid keystroke spec")
    }

    #[test]
    fn arrows_navigate_and_enter_moves_to_fill_stage() {
        let mut p = OneKeyPicker::new(entries());
        assert_eq!(p.stage(), Stage::PickEntry);
        assert!(matches!(p.handle_key(&key("down")), UiAction::Redraw));
        assert_eq!(p.cursor(), 1);
        assert!(matches!(p.handle_key(&key("up")), UiAction::Redraw));
        assert_eq!(p.cursor(), 0);
        assert!(matches!(p.handle_key(&key("enter")), UiAction::Redraw));
        assert_eq!(p.stage(), Stage::PickFill);
        // Sorted by title: prod (alice) first.
        assert_eq!(p.chosen_entry().unwrap().username, "alice");
    }

    #[test]
    fn fill_stage_keys_select_what_to_fill() {
        let mut p = OneKeyPicker::new(entries());
        p.handle_key(&key("enter"));
        assert!(matches!(
            p.handle_key(&key("u")),
            UiAction::Fill(OneKeyFill::Username)
        ));
        assert!(matches!(
            p.handle_key(&key("p")),
            UiAction::Fill(OneKeyFill::Password)
        ));
        assert!(matches!(
            p.handle_key(&key("b")),
            UiAction::Fill(OneKeyFill::UsernameAndPassword)
        ));
        assert!(matches!(
            p.handle_key(&key("enter")),
            UiAction::Fill(OneKeyFill::UsernameAndPassword)
        ));
    }

    #[test]
    fn backspace_in_fill_stage_returns_to_entry_list() {
        let mut p = OneKeyPicker::new(entries());
        p.handle_key(&key("enter"));
        assert_eq!(p.stage(), Stage::PickFill);
        assert!(matches!(p.handle_key(&key("backspace")), UiAction::Redraw));
        assert_eq!(p.stage(), Stage::PickEntry);
        assert!(p.chosen_entry().is_none());
    }

    #[test]
    fn escape_cancels_from_either_stage() {
        let mut p = OneKeyPicker::new(entries());
        assert!(matches!(p.handle_key(&key("escape")), UiAction::Cancel));
        let mut p = OneKeyPicker::new(entries());
        p.handle_key(&key("enter"));
        assert!(matches!(p.handle_key(&key("escape")), UiAction::Cancel));
        let mut p = OneKeyPicker::new(entries());
        assert!(matches!(p.handle_key(&key("ctrl-g")), UiAction::Cancel));
        assert!(matches!(p.handle_key(&key("ctrl-c")), UiAction::Cancel));
    }

    #[test]
    fn filter_chars_narrow_and_backspace_restores() {
        let mut p = OneKeyPicker::new(entries());
        for ch in ["s", "t", "a", "g"] {
            p.push_filter_char(ch);
        }
        assert_eq!(p.filtered().len(), 1);
        assert_eq!(p.filtered_entries()[0].username, "bob");
        for _ in 0..4 {
            p.pop_filter();
        }
        assert_eq!(p.filtered().len(), 2);
    }
}
