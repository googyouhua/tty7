//! Review actions on the diff overlay: drafts and attach-to-agent.
//!
//! Both reuse the overlay's line-granular `DiffSelection`; the comment text
//! comes from the caller (overlay comment box or Review tab editor).

use gpui::{Context, Window};

use crate::terminal::git_diff::DiffSource;
use crate::ui::app::Tty7App;
use crate::ui::diff_overlay::DiffLoad;
use crate::ui::i18n::{L10nKey, t};
use crate::ui::review_state::{ReviewDraft, ReviewSend};

fn source_tag(source: &DiffSource) -> String {
    source.tag()
}

fn source_label(source: &DiffSource) -> String {
    match source {
        DiffSource::Patch { id, .. } => id.clone(),
        DiffSource::Range { base, head } => format!("{base}...{head}"),
        DiffSource::Commit { rev, .. } => rev.clone(),
        DiffSource::Worktree => "worktree".to_string(),
        DiffSource::Staged => "staged".to_string(),
        DiffSource::Head => "HEAD".to_string(),
    }
}

fn selection_range_label(sel: &crate::ui::diff_rows::DiffSelection) -> String {
    let (a, b) = match sel.anchor <= sel.head {
        true => (sel.anchor, sel.head),
        false => (sel.head, sel.anchor),
    };
    format!("hunk {}:{}-{}:{}", a.hunk, a.row, b.hunk, b.row)
}

impl Tty7App {
    /// Save the overlay's current selection as a local draft.
    pub(crate) fn review_save_selection_as_draft(&mut self, comment: &str, cx: &mut Context<Self>) -> bool {
        let overlay = match self.tabs.get(self.active).and_then(|t| t.diff_overlay.as_ref()) {
            Some(o) => o,
            None => return false,
        };
        let sel = match overlay.selection.as_ref() {
            Some(s) => s.clone(),
            None => return false,
        };
        let (source, hunks) = match &overlay.load {
            DiffLoad::Ready(snap) => {
                let h = snap
                    .files
                    .iter()
                    .find(|f| f.path == sel.path)
                    .map(|f| f.hunks.clone());
                (snap.source.clone(), h)
            }
            _ => return false,
        };
        let hunks = match hunks {
            Some(h) => h,
            None => match &overlay.preview {
                Some((held, Some(file))) if *held == sel.path => file.hunks.clone(),
                _ => return false,
            },
        };
        let diff = sel.text(&hunks);
        if diff.trim().is_empty() {
            return false;
        }
        self.review.upsert_draft(ReviewDraft {
            source_tag: source_tag(&source),
            source_label: source_label(&source),
            path: sel.path.clone(),
            lines: selection_range_label(&sel),
            diff,
            comment: comment.trim().to_string(),
        });
        cx.notify();
        true
    }

    /// Send the overlay's current selection plus comment to the running agent.
    pub(crate) fn review_send_selection_to_agent(
        &mut self,
        comment: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<(), &'static str> {
        let overlay = match self.tabs.get(self.active).and_then(|t| t.diff_overlay.as_ref()) {
            Some(o) => o,
            None => return Err("no-selection"),
        };
        let sel = match overlay.selection.as_ref() {
            Some(s) => s.clone(),
            None => return Err("no-selection"),
        };
        let (source, hunks) = match &overlay.load {
            DiffLoad::Ready(snap) => {
                let h = snap
                    .files
                    .iter()
                    .find(|f| f.path == sel.path)
                    .map(|f| f.hunks.clone());
                (snap.source.clone(), h)
            }
            _ => return Err("no-selection"),
        };
        let hunks = match hunks {
            Some(h) => h,
            None => match &overlay.preview {
                Some((held, Some(file))) if *held == sel.path => file.hunks.clone(),
                _ => return Err("no-selection"),
            },
        };
        let diff = sel.text(&hunks);
        if diff.trim().is_empty() {
            return Err("no-selection");
        }
        let lines = selection_range_label(&sel);
        let prompt = crate::core::agent_prompt::build_review_attach_prompt(
            &sel.path,
            &lines,
            &diff,
            comment,
            Some(&source_label(&source)),
        )
        .ok_or("no-selection")?;
        if self.agent_target_leaf(cx).is_none() {
            crate::terminal::notify_desktop(Some("tty7"), t(L10nKey::AppNoRunningCodingAgent));
            return Err("no-agent");
        }
        let comment_len = comment.trim().len();
        let path = sel.path.clone();
        self.deliver_agent_prompt(&prompt, window, cx);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        self.review.record_send(ReviewSend {
            path,
            lines,
            comment_len,
            at: now,
        });
        Ok(())
    }
}
