//! Local review drafts and attach-to-agent send records.
//!
//! Drafts are keyed by the diff source tag plus file path plus a line key, so
//! a PR patch (`owner/repo#12`) and a local `Range{base,head}` never share
//! entries. Records remember what was last sent to the running agent.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use gpui::Entity;
use gpui_component::input::InputState;

use super::scm::state::RepoKey;
use crate::core::config::DiffViewMode;
use crate::terminal::git_diff::{DiffSnapshot, DiffSource, FileDiff};
use crate::ui::diff_rows::{RowId, Side};

/// One unsent review note on a diff hunk.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReviewDraft {
    /// `DiffSource::tag()` of the overlay it was written in.
    pub source_tag: String,
    /// Human source label (`owner/repo#12` or `base...head`).
    pub source_label: String,
    pub path: String,
    /// Line key such as `new 120-126` or `old 10-14`.
    pub lines: String,
    /// The selected diff text the draft was written against.
    pub diff: String,
    pub comment: String,
    /// Where it was written, for jumping back to the file.
    pub repo: RepoKey,
    /// The overlay source, for reopening it.
    pub source: DiffSource,
    /// The focused file as saved, so a supplied (`Patch`) snapshot can be
    /// rebuilt without refetching. `None` for preview-synthesized files.
    pub file: Option<FileDiff>,
    /// The selection as saved, for restoring the highlight and scroll on
    /// jump. Coordinates are only valid in `sel_mode`.
    pub sel_mode: DiffViewMode,
    pub sel_side: Option<Side>,
    pub sel_anchor: RowId,
    pub sel_head: RowId,
}

/// One attach-to-agent send.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReviewSend {
    pub path: String,
    pub lines: String,
    pub comment_len: usize,
    /// Unix seconds of the send.
    pub at: i64,
}

#[derive(Clone, Debug, Default)]
pub struct ReviewState {
    drafts: HashMap<(String, String, String), ReviewDraft>,
    sends: Vec<ReviewSend>,
    branches: HashMap<RepoKey, BranchList>,
    selection: HashMap<RepoKey, (String, String)>,
    file_lists: HashMap<(RepoKey, String, String), FileListCache>,
    /// The draft open in the Review tab editor, if any, with its input box.
    editing: Option<(String, String, String)>,
    edit_box: Option<Entity<InputState>>,
}

/// The probed `base...head` snapshot behind the file list, plus load state.
/// `Arc` is cloned, never re-read, on every render.
#[derive(Clone, Debug, Default)]
pub struct FileListCache {
    pub snapshot: Option<Arc<DiffSnapshot>>,
    pub loading: bool,
    pub read_at: Option<Instant>,
    pub error: Option<String>,
    pub unfolded: bool,
}

/// `git branch -a` for one repository, re-read on a TTL like the GitHub
/// branch lookup. `origin/HEAD` pointers are dropped at parse time.
#[derive(Clone, Debug, Default)]
pub struct BranchList {
    pub branches: Vec<String>,
    pub loading: bool,
    pub read_at: Option<Instant>,
}

impl BranchList {
    pub fn parse(raw: &str) -> Vec<String> {
        let mut out = Vec::new();
        for line in raw.lines() {
            let name = line.trim().trim_start_matches("* ").trim();
            if name.is_empty() || name.ends_with("/HEAD") || name.contains("->") {
                continue;
            }
            if !out.iter().any(|b| b == name) {
                out.push(name.to_string());
            }
        }
        out
    }
}

impl ReviewState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn upsert_draft(&mut self, draft: ReviewDraft) {
        self.drafts.insert(
            (
                draft.source_tag.clone(),
                draft.path.clone(),
                draft.lines.clone(),
            ),
            draft,
        );
    }

    pub fn remove_draft(&mut self, source_tag: &str, path: &str, lines: &str) -> bool {
        self.drafts
            .remove(&(source_tag.to_string(), path.to_string(), lines.to_string()))
            .is_some()
    }

    pub fn drafts_for_source(&self, source_tag: &str) -> Vec<&ReviewDraft> {
        let mut out: Vec<&ReviewDraft> = self
            .drafts
            .values()
            .filter(|d| d.source_tag == source_tag)
            .collect();
        out.sort_by(|a, b| (&a.path, &a.lines).cmp(&(&b.path, &b.lines)));
        out
    }

    pub fn draft_count_for_source(&self, source_tag: &str) -> usize {
        self.drafts
            .values()
            .filter(|d| d.source_tag == source_tag)
            .count()
    }

    pub fn record_send(&mut self, send: ReviewSend) {
        self.sends.push(send);
        if self.sends.len() > 20 {
            let excess = self.sends.len() - 20;
            self.sends.drain(..excess);
        }
    }

    pub fn recent_sends(&self) -> &[ReviewSend] {
        &self.sends
    }

    pub fn branch_list(&self, repo: &RepoKey) -> Option<&BranchList> {
        self.branches.get(repo)
    }

    pub fn branch_list_mut(&mut self, repo: &RepoKey) -> &mut BranchList {
        self.branches.entry(repo.clone()).or_default()
    }

    pub fn selection_for(&self, repo: &RepoKey) -> Option<(String, String)> {
        self.selection.get(repo).cloned()
    }

    pub fn set_selection(&mut self, repo: &RepoKey, base: String, head: String) {
        self.selection.insert(repo.clone(), (base, head));
    }

    pub fn file_list(&self, repo: &RepoKey, base: &str, head: &str) -> Option<&FileListCache> {
        self.file_lists
            .get(&(repo.clone(), base.to_string(), head.to_string()))
    }

    pub fn toggle_unfolded(&mut self, repo: &RepoKey, base: &str, head: &str) {
        let entry = self.file_list_entry(repo, base, head);
        entry.unfolded = !entry.unfolded;
    }

    pub fn draft(&self, source_tag: &str, path: &str, lines: &str) -> Option<&ReviewDraft> {
        self.drafts
            .get(&(source_tag.to_string(), path.to_string(), lines.to_string()))
    }

    pub fn update_draft_comment(
        &mut self,
        source_tag: &str,
        path: &str,
        lines: &str,
        comment: String,
    ) -> bool {
        match self
            .drafts
            .get_mut(&(source_tag.to_string(), path.to_string(), lines.to_string()))
        {
            Some(d) => {
                d.comment = comment;
                true
            }
            None => false,
        }
    }

    pub fn editing_key(&self) -> Option<(String, String, String)> {
        self.editing.clone()
    }

    pub fn edit_box(&self) -> Option<Entity<InputState>> {
        self.edit_box.clone()
    }

    pub fn begin_edit(&mut self, key: (String, String, String), box_entity: Entity<InputState>) {
        self.editing = Some(key);
        self.edit_box = Some(box_entity);
    }

    pub fn end_edit(&mut self) {
        self.editing = None;
        self.edit_box = None;
    }

    pub fn file_list_entry(
        &mut self,
        repo: &RepoKey,
        base: &str,
        head: &str,
    ) -> &mut FileListCache {
        self.file_lists
            .entry((repo.clone(), base.to_string(), head.to_string()))
            .or_default()
    }

    /// Pick a valid `(base, head)` from the loaded list: stored choices win
    /// while they still exist, then the current branch/upstream, then common
    /// default branches. Returns `None` until the list has loaded.
    pub fn resolve_selection(
        &self,
        repo: &RepoKey,
        current: Option<&str>,
        upstream: Option<&str>,
    ) -> Option<(String, String)> {
        let list = self.branch_list(repo)?;
        if list.branches.is_empty() {
            return None;
        }
        let has = |b: &str| list.branches.iter().any(|x| x == b);
        let (stored_base, stored_head) = self
            .selection_for(repo)
            .map(|(b, h)| (Some(b), Some(h)))
            .unwrap_or((None, None));
        let head = stored_head
            .filter(|h| has(h))
            .or_else(|| current.filter(|c| has(c)).map(str::to_string))
            .unwrap_or_else(|| "HEAD".to_string());
        let base = stored_base.filter(|b| has(b)).or_else(|| {
            upstream.filter(|u| has(u)).map(str::to_string).or_else(|| {
                ["main", "master", "origin/main", "origin/master"]
                    .into_iter()
                    .find(|b| has(b))
                    .map(str::to_string)
                    .or_else(|| list.branches.iter().find(|b| *b != &head).cloned())
            })
        })?;
        Some((base, head))
    }

    /// Whether the pair may be handed to git: non-empty, different, and not
    /// shaped like an option. Mirrors `DiffSource::revs_are_arguments`.
    pub fn pair_is_valid(base: &str, head: &str) -> bool {
        let ok = |rev: &str| {
            !rev.is_empty() && !rev.starts_with('-') && !rev.contains(|c: char| c.is_control())
        };
        ok(base) && ok(head) && base != head
    }

    pub fn all_drafts_sorted(&self) -> Vec<&ReviewDraft> {
        let mut out: Vec<&ReviewDraft> = self.drafts.values().collect();
        out.sort_by(|a, b| {
            (&a.source_tag, &a.path, &a.lines).cmp(&(&b.source_tag, &b.path, &b.lines))
        });
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn draft() -> ReviewDraft {
        ReviewDraft {
            source_tag: "patch".to_string(),
            source_label: "owner/repo#12".to_string(),
            path: "src/a.rs".to_string(),
            lines: "new 1-3".to_string(),
            diff: "+x".to_string(),
            comment: "fix".to_string(),
            repo: RepoKey {
                host: crate::ui::host_ops::HostId::LOCAL,
                root: std::path::PathBuf::from("/repo"),
            },
            source: DiffSource::Head,
            file: None,
            sel_mode: DiffViewMode::Split,
            sel_side: None,
            sel_anchor: RowId { hunk: 0, row: 0 },
            sel_head: RowId { hunk: 0, row: 0 },
        }
    }

    #[test]
    fn drafts_are_scoped_by_source() {
        let mut s = ReviewState::new();
        s.upsert_draft(draft());
        let mut other = draft();
        other.source_tag = "range".to_string();
        other.path = "src/b.rs".to_string();
        s.upsert_draft(other);
        assert_eq!(s.draft_count_for_source("patch"), 1);
        assert_eq!(s.drafts_for_source("range").len(), 1);
        assert!(s.remove_draft("patch", "src/a.rs", "new 1-3"));
        assert_eq!(s.draft_count_for_source("patch"), 0);
    }

    #[test]
    fn sends_keep_only_the_latest_twenty() {
        let mut s = ReviewState::new();
        for i in 0..25 {
            s.record_send(ReviewSend {
                path: "a.rs".to_string(),
                lines: format!("new {i}"),
                comment_len: 1,
                at: i,
            });
        }
        assert_eq!(s.recent_sends().len(), 20);
        assert_eq!(s.recent_sends().last().unwrap().at, 24);
    }

    fn repo() -> RepoKey {
        RepoKey {
            host: crate::ui::host_ops::HostId::LOCAL,
            root: std::path::PathBuf::from("/repo"),
        }
    }

    #[test]
    fn branch_parse_drops_head_pointers_and_dupes() {
        let branches = BranchList::parse(
            "* main\n  origin/HEAD -> origin/main\n  origin/main\n  main\n  feat/x\n",
        );
        assert_eq!(branches, vec!["main", "origin/main", "feat/x"]);
    }

    #[test]
    fn resolve_prefers_stored_then_upstream_then_defaults() {
        let mut s = ReviewState::new();
        let repo = repo();
        s.branch_list_mut(&repo).branches =
            vec!["main".into(), "feat/x".into(), "origin/main".into()];
        // No stored choice: head=current, base=upstream.
        assert_eq!(
            s.resolve_selection(&repo, Some("feat/x"), Some("origin/main")),
            Some(("origin/main".to_string(), "feat/x".to_string()))
        );
        // Stored choices win while they exist.
        s.set_selection(&repo, "main".into(), "feat/x".into());
        assert_eq!(
            s.resolve_selection(&repo, Some("feat/x"), Some("origin/main")),
            Some(("main".to_string(), "feat/x".to_string()))
        );
        // A stored branch that vanished falls back to defaults.
        s.set_selection(&repo, "gone".into(), "feat/x".into());
        assert_eq!(
            s.resolve_selection(&repo, Some("feat/x"), Some("gone-up")),
            Some(("main".to_string(), "feat/x".to_string()))
        );
    }

    #[test]
    fn pair_validation_rejects_empty_same_and_option_like() {
        assert!(ReviewState::pair_is_valid("main", "feat/x"));
        assert!(!ReviewState::pair_is_valid("main", "main"));
        assert!(!ReviewState::pair_is_valid("", "feat/x"));
        assert!(!ReviewState::pair_is_valid("--output=x", "feat/x"));
    }

    #[test]
    fn draft_comment_can_be_updated() {
        let mut s = ReviewState::new();
        s.upsert_draft(draft());
        assert!(s.update_draft_comment("patch", "src/a.rs", "new 1-3", "better".to_string()));
        assert_eq!(
            s.draft("patch", "src/a.rs", "new 1-3").unwrap().comment,
            "better"
        );
        assert!(!s.update_draft_comment("patch", "gone.rs", "new 1-3", "x".to_string()));
        assert!(s.editing_key().is_none());
    }
}
