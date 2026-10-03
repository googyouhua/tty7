//! Review actions on the diff overlay: drafts and attach-to-agent.
//!
//! Both reuse the overlay's line-granular `DiffSelection`; the comment text
//! comes from the caller (overlay comment box or Review tab editor).

use gpui::{AppContext as _, Context, Window};

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

impl Tty7App {
    /// Save the overlay's current selection as a local draft.
    pub(crate) fn review_save_selection_as_draft(
        &mut self,
        comment: &str,
        cx: &mut Context<Self>,
    ) -> bool {
        let overlay = match self
            .tabs
            .get(self.active)
            .and_then(|t| t.diff_overlay.as_ref())
        {
            Some(o) => o,
            None => return false,
        };
        let sel = match overlay.selection.as_ref() {
            Some(s) => s.clone(),
            None => return false,
        };
        let host_id = overlay.host_id;
        let overlay_cwd = overlay.cwd.clone();
        let (source, file) = match &overlay.load {
            DiffLoad::Ready(snap) => {
                let f = snap.files.iter().find(|f| f.path == sel.path).cloned();
                let root = snap.root.clone();
                (snap.source.clone(), f.map(|f| (root, f)))
            }
            _ => return false,
        };
        let (root, file) = match file {
            Some((root, f)) => (root, Some(f)),
            None => match &overlay.preview {
                Some((held, Some(file))) if *held == sel.path => {
                    (overlay_cwd, Some(file.as_ref().clone()))
                }
                _ => return false,
            },
        };
        let hunks = file.as_ref().map(|f| f.hunks.clone()).unwrap_or_default();
        let diff = sel.text(&hunks);
        if diff.trim().is_empty() {
            return false;
        }
        let lines = sel.line_label(&hunks);
        self.review.upsert_draft(ReviewDraft {
            source_tag: source_tag(&source),
            source_label: source_label(&source),
            path: sel.path.clone(),
            lines,
            diff,
            comment: comment.trim().to_string(),
            repo: crate::ui::scm::state::RepoKey {
                host: host_id,
                root,
            },
            source,
            file,
            sel_mode: sel.mode,
            sel_side: sel.side,
            sel_anchor: sel.anchor,
            sel_head: sel.head,
        });
        cx.notify();
        true
    }

    /// The prompt for the overlay's current selection plus comment, with the
    /// file path, line range and source label for the send record. `None`
    /// when there is no usable selection.
    fn review_selection_prompt(
        &self,
        comment: &str,
    ) -> Option<(String, String, String, usize, String)> {
        let overlay = self.tabs.get(self.active)?.diff_overlay.as_ref()?;
        let sel = overlay.selection.as_ref()?.clone();
        let (source, hunks) = match &overlay.load {
            DiffLoad::Ready(snap) => {
                let h = snap
                    .files
                    .iter()
                    .find(|f| f.path == sel.path)
                    .map(|f| f.hunks.clone());
                (snap.source.clone(), h)
            }
            _ => return None,
        };
        let hunks = match hunks {
            Some(h) => h,
            None => match &overlay.preview {
                Some((held, Some(file))) if *held == sel.path => file.hunks.clone(),
                _ => return None,
            },
        };
        let diff = sel.text(&hunks);
        if diff.trim().is_empty() {
            return None;
        }
        let lines = sel.line_label(&hunks);
        let label = source_label(&source);
        let prompt = crate::core::agent_prompt::build_review_attach_prompt(
            &sel.path,
            &lines,
            &diff,
            comment,
            Some(&label),
        )?;
        Some((sel.path.clone(), lines, prompt, comment.trim().len(), label))
    }

    /// Send the overlay's current selection plus comment to the running agent.
    pub(crate) fn review_send_selection_to_agent(
        &mut self,
        comment: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<(), &'static str> {
        let Some((path, lines, prompt, comment_len, _)) = self.review_selection_prompt(comment)
        else {
            return Err("no-selection");
        };
        if self.agent_target_leaf(cx).is_none() {
            crate::terminal::notify_desktop(Some("tty7"), t(L10nKey::AppNoRunningCodingAgent));
            return Err("no-agent");
        }
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

    /// Send the overlay's current selection plus comment to a NEW agent tab.
    ///
    /// Opens the most recently used agent in a new tab and delivers the
    /// prompt once the agent reports itself running (30s budget). Running
    /// agents are never touched.
    pub(crate) fn review_send_to_new_agent(
        &mut self,
        comment: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<(), &'static str> {
        let Some((path, lines, prompt, comment_len, label)) = self.review_selection_prompt(comment)
        else {
            return Err("no-selection");
        };
        self.deliver_review_to_new_agent(&path, &lines, &prompt, comment_len, &label, window, cx)
    }

    /// Jump to a draft's file: reopen its source overlay focused on the
    /// draft's path. Git-backed sources re-probe; a `Patch` source rebuilds
    /// the saved single-file snapshot instead. Returns false when the draft
    /// is gone or cannot be opened.
    pub(crate) fn review_jump_to_draft(
        &mut self,
        source_tag: &str,
        path: &str,
        lines: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        use crate::terminal::git_diff::DiffSource;

        let draft = match self.review.draft(source_tag, path, lines).cloned() {
            Some(d) => d,
            None => return false,
        };
        match &draft.source {
            DiffSource::Patch { .. } => {
                let Some(file) = draft.file.clone() else {
                    return false;
                };
                let snapshot = std::sync::Arc::new(crate::terminal::git_diff::DiffSnapshot {
                    root: draft.repo.root.clone(),
                    source: draft.source.clone(),
                    branch: String::new(),
                    files: vec![file],
                    untracked: Vec::new(),
                    untracked_total: 0,
                    read_failed: false,
                });
                self.open_supplied_diff(
                    draft.repo.host,
                    snapshot,
                    Some(draft.path.clone()),
                    window,
                    cx,
                );
            }
            _ => {
                self.open_diff_overlay(
                    draft.repo.host,
                    draft.repo.root.clone(),
                    draft.source.clone(),
                    Some(draft.path.clone()),
                    window,
                    cx,
                );
            }
        }
        // Restore the saved highlight when the current view speaks the same
        // coordinates; otherwise the file-level focus is all we can offer.
        let same_mode = draft.sel_mode == crate::ui::diff_overlay::view_mode(cx);
        if same_mode {
            if let Some(overlay) = self
                .tabs
                .get_mut(self.active)
                .and_then(|t| t.diff_overlay.as_mut())
            {
                overlay.selection = Some(crate::ui::diff_rows::DiffSelection {
                    path: draft.path.clone(),
                    mode: draft.sel_mode,
                    side: draft.sel_side,
                    anchor: draft.sel_anchor,
                    head: draft.sel_head,
                });
                overlay.reveal_selection = true;
            }
            cx.notify();
        }
        true
    }

    /// Toggle the Review tab editor for a draft, prefilled with its comment.
    pub(crate) fn review_edit_draft(
        &mut self,
        source_tag: &str,
        path: &str,
        lines: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let key = (source_tag.to_string(), path.to_string(), lines.to_string());
        if self.review.editing_key().as_ref() == Some(&key) {
            self.review.end_edit();
            cx.notify();
            return;
        }
        let comment = self
            .review
            .draft(source_tag, path, lines)
            .map(|d| d.comment.clone())
            .unwrap_or_default();
        let box_entity = cx.new(|cx| {
            use gpui_component::input::InputState;
            InputState::new(window, cx)
                .placeholder("Comment on the selected diff…")
                .default_value(comment)
        });
        box_entity.update(cx, |state, cx| state.focus(window, cx));
        self.review.begin_edit(key, box_entity);
        cx.notify();
    }

    /// Save the Review tab editor text back into its draft.
    pub(crate) fn review_save_edit(&mut self, cx: &mut Context<Self>) {
        let (Some(key), Some(box_entity)) = (self.review.editing_key(), self.review.edit_box())
        else {
            return;
        };
        let comment = box_entity.read(cx).value().trim().to_string();
        self.review
            .update_draft_comment(&key.0, &key.1, &key.2, comment);
        self.review.end_edit();
        cx.notify();
    }

    /// Send the Review tab editor text with its draft's diff to a new agent.
    pub(crate) fn review_send_edit(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<(), &'static str> {
        let (Some(key), Some(box_entity)) = (self.review.editing_key(), self.review.edit_box())
        else {
            return Err("no-selection");
        };
        let comment = box_entity.read(cx).value().trim().to_string();
        let draft = self.review.draft(&key.0, &key.1, &key.2).cloned();
        let Some(draft) = draft else {
            return Err("no-selection");
        };
        if draft.diff.trim().is_empty() {
            return Err("no-selection");
        }
        let prompt = crate::core::agent_prompt::build_review_attach_prompt(
            &draft.path,
            &draft.lines,
            &draft.diff,
            &comment,
            Some(&draft.source_label),
        )
        .ok_or("no-selection")?;
        self.review
            .update_draft_comment(&key.0, &key.1, &key.2, comment.clone());
        let r = self.deliver_review_to_new_agent(
            &draft.path,
            &draft.lines,
            &prompt,
            comment.len(),
            &draft.source_label,
            window,
            cx,
        );
        if r.is_ok() {
            self.review.end_edit();
        }
        r
    }

    /// Deliver already-built review content to a NEW agent tab. Shared by the
    /// overlay selection flow and the Review tab draft editor.
    pub(crate) fn deliver_review_to_new_agent(
        &mut self,
        path: &str,
        lines: &str,
        prompt: &str,
        comment_len: usize,
        _source_label: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<(), &'static str> {
        use crate::ui::agent_launch::most_recent;
        use crate::ui::app::SpawnWhere;

        let offered = self.offered_agents(cx);
        let agent_frecency = cx
            .global::<crate::core::config::Config>()
            .agent_frecency
            .clone();
        let Some(agent) = most_recent(&offered, &agent_frecency) else {
            let remote =
                crate::core::session::WorkspaceStore::remote_ref(cx, self.workspace).is_some();
            crate::terminal::notify_desktop(
                Some("tty7"),
                t(if remote {
                    L10nKey::AppNoAgentSeenHere
                } else {
                    L10nKey::AppNoAgentOnPath
                }),
            );
            return Err("no-agent-offered");
        };
        self.launch_agent(agent, SpawnWhere::NewTab, window, cx);
        let tab_idx = self.active;
        let (path, lines, prompt) = (path.to_string(), lines.to_string(), prompt.to_string());
        cx.spawn(async move |this, cx| {
            use tty7_core::core::cli_agent::AgentStatus;

            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
            // Stage 1: wait for the agent to appear in the new tab.
            let leaf = loop {
                let found = this
                    .update(cx, |this, cx| {
                        this.tabs.get(tab_idx).and_then(|t| {
                            t.pane
                                .terminals()
                                .into_iter()
                                .find(|leaf| leaf.read(cx).agent().is_some())
                        })
                    })
                    .ok()
                    .flatten();
                if let Some(leaf) = found {
                    break leaf;
                }
                if std::time::Instant::now() >= deadline {
                    crate::terminal::notify_desktop(Some("tty7"), t(L10nKey::ReviewAgentTimeout));
                    log::warn!("review: new agent tab did not report running in time");
                    return;
                }
                cx.background_executor()
                    .timer(std::time::Duration::from_millis(500))
                    .await;
            };
            // Stage 2: don't type into a still-booting TUI — its startup
            // output eats the paste. Send once it stops Working, or after a
            // short settle grace so a fresh idle agent never waits forever.
            let settled = std::time::Instant::now() + std::time::Duration::from_secs(5);
            loop {
                let status = this
                    .update(cx, |_, cx| leaf.read(cx).agent_session().map(|s| s.status))
                    .ok()
                    .flatten();
                match status {
                    Some(AgentStatus::Waiting)
                    | Some(AgentStatus::Idle)
                    | Some(AgentStatus::Done) => break,
                    _ => {}
                }
                if std::time::Instant::now() >= settled || std::time::Instant::now() >= deadline {
                    break;
                }
                cx.background_executor()
                    .timer(std::time::Duration::from_millis(500))
                    .await;
            }
            let _ = this.update(cx, |_, cx| {
                leaf.read(cx).send_agent_prompt(&prompt);
            });
            let _ = this.update(cx, |this, cx| {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs() as i64)
                    .unwrap_or(0);
                this.review.record_send(ReviewSend {
                    path: path.clone(),
                    lines: lines.clone(),
                    comment_len,
                    at: now,
                });
                cx.notify();
            });
        })
        .detach();
        Ok(())
    }
}
