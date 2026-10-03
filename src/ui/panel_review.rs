//! The right panel's Review tab: branch-diff entry plus local drafts.
//!
//! Kept deliberately narrow: the diff itself stays in `diff_overlay`, this
//! tab only picks `base...head`, lists unsent drafts and recent sends.
//!
//! The pane target is resolved at click time, not at render time: the panel
//! has no refresh subscription of its own, so a closure that captured the
//! render-time directory would keep opening the previous repository after
//! the user switched panes.

use std::time::{Duration, Instant};

use gpui::{AnyElement, Context, Window, div, prelude::*, px, rems};
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::menu::{DropdownMenu as _, PopupMenuItem};
use gpui_component::{ActiveTheme as _, Sizable as _, h_flex, v_flex};

use crate::terminal::git_diff::{DiffRequest, DiffSource};
use crate::ui::app::Tty7App;
use crate::ui::host_ops::SharedHost;
use crate::ui::i18n::{L10nKey, t};
use crate::ui::right_panel::{META, META_MONO, ROW_INSET, TEXT, git_badge};
use crate::ui::scm::panel::RepoLookup;
use crate::ui::scm::path::split_display_path;
use crate::ui::scm::state::RepoKey;
use crate::ui::scm::status::{status_color, status_glyph};

/// How long a loaded branch list is trusted before it is re-read.
const BRANCH_TTL: Duration = Duration::from_secs(10);

impl Tty7App {
    pub(crate) fn render_panel_review(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let title = self.panel_title(t(L10nKey::PanelReviewTitle), None, None, window, cx);
        let body = match self.scm_pane_target(window, cx) {
            None => self.panel_empty(t(L10nKey::PanelNoWorkingDirectory), None, cx),
            Some((host, cwd)) => match self.scm_repo_root(&host, &cwd, cx) {
                RepoLookup::Pending => self.panel_empty(t(L10nKey::PanelLoading), None, cx),
                RepoLookup::NotARepo => self.panel_empty(
                    t(L10nKey::PanelNotAGitRepo),
                    Some(t(L10nKey::PanelNotAGitRepoHint)),
                    cx,
                ),
                RepoLookup::Root(root) => {
                    let repo = RepoKey {
                        host: host.id(),
                        root,
                    };
                    self.review_branch_body(host, repo, cx)
                }
            },
        };
        v_flex().child(title).child(body).into_any_element()
    }

    /// Branch names for `repo`, re-read every [`BRANCH_TTL`].
    fn review_branches(
        &mut self,
        host: SharedHost,
        repo: &RepoKey,
        cx: &mut Context<Self>,
    ) -> Vec<String> {
        let entry = self.review.branch_list_mut(repo);
        let due = !entry.loading && entry.read_at.is_none_or(|t| t.elapsed() > BRANCH_TTL);
        let branches = entry.branches.clone();
        if due {
            entry.loading = true;
            let root = repo.root.clone();
            let key = repo.clone();
            crate::ui::host_ops::HostOps::run(
                host,
                cx,
                move |h| {
                    h.git(&root, &["branch", "-a", "--format=%(refname:short)"])
                        .ok()
                        .filter(|o| o.success())
                        .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
                },
                move |this, out, cx| {
                    let entry = this.review.branch_list_mut(&key);
                    entry.loading = false;
                    entry.read_at = Some(Instant::now());
                    if let Some(raw) = out {
                        let branches = crate::ui::review_state::BranchList::parse(&raw);
                        if entry.branches != branches {
                            entry.branches = branches;
                            cx.notify();
                        }
                    }
                },
            );
        }
        branches
    }

    fn review_branch_body(
        &mut self,
        host: SharedHost,
        repo: RepoKey,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let branches = self.review_branches(host.clone(), &repo, cx);
        let head_info = self.github_branch(host.clone(), &repo, cx);
        let current = head_info.as_ref().map(|h| h.branch.as_str());
        let upstream = head_info.as_ref().and_then(|h| h.upstream.as_deref());
        let (base, head) = self
            .review
            .resolve_selection(&repo, current, upstream)
            .unwrap_or_default();
        let valid = !branches.is_empty()
            && crate::ui::review_state::ReviewState::pair_is_valid(&base, &head);

        let mut body = v_flex().gap(px(8.)).p(px(12.));
        body = body.child(
            div()
                .text_sm()
                .text_color(cx.theme().muted_foreground)
                .child(format!("{} · head: {}", repo.root.to_string_lossy(), head)),
        );
        body = body.child(
            v_flex()
                .gap(px(4.))
                .child(self.review_branch_dropdown(&repo, true, base.clone(), &branches, cx))
                .child(self.review_branch_dropdown(&repo, false, head.clone(), &branches, cx)),
        );
        body = body.child(self.review_open_row(&repo, &base, &head, valid, cx));
        if valid {
            body = body.child(self.review_files_section(host.clone(), &repo, &base, &head, cx));
        }
        for (i, d) in self.review.all_drafts_sorted().iter().take(20).enumerate() {
            let (tag, path, lines) = (d.source_tag.clone(), d.path.clone(), d.lines.clone());
            let is_editing = self
                .review
                .editing_key()
                .is_some_and(|k| k == (tag.clone(), path.clone(), lines.clone()));
            let accent = cx.theme().accent;
            let (jump_tag, jump_path, jump_lines) = (tag.clone(), path.clone(), lines.clone());
            let mut row = h_flex()
                .id(("panel-review-draft", i))
                .items_center()
                .gap(px(8.))
                .px(px(8.))
                .py(px(2.))
                .child(
                    div()
                        .id(("panel-review-draft-jump", i))
                        .flex_1()
                        .min_w_0()
                        .text_sm()
                        .truncate()
                        .cursor_pointer()
                        .hover(move |s| s.text_color(accent))
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.review_jump_to_draft(
                                &jump_tag,
                                &jump_path,
                                &jump_lines,
                                window,
                                cx,
                            );
                        }))
                        .child(format!("{} {} — {}", d.path, d.lines, d.comment)),
                );
            {
                let (tag, path, lines) = (tag.clone(), path.clone(), lines.clone());
                row = row.child(
                    div()
                        .id(("panel-review-draft-edit", i))
                        .text_sm()
                        .px(px(6.))
                        .rounded(px(4.))
                        .cursor_pointer()
                        .hover(|s| s.bg(cx.theme().accent.opacity(0.12)))
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.review_edit_draft(&tag, &path, &lines, window, cx);
                        }))
                        .child(if is_editing { "Close" } else { "Edit" }),
                );
            }
            {
                let (tag, path, lines) = (tag.clone(), path.clone(), lines.clone());
                row = row.child(
                    div()
                        .id(("panel-review-draft-del", i))
                        .text_sm()
                        .px(px(6.))
                        .rounded(px(4.))
                        .cursor_pointer()
                        .hover(|s| s.bg(cx.theme().accent.opacity(0.12)))
                        .on_click(cx.listener(move |this, _, _window, cx| {
                            this.review.remove_draft(&tag, &path, &lines);
                            cx.notify();
                        }))
                        .child("Delete"),
                );
            }
            body = body.child(row);
            if is_editing {
                if let Some(input) = self.review.edit_box() {
                    body = body.child(
                        v_flex()
                            .gap(px(6.))
                            .px(px(8.))
                            .py(px(4.))
                            .child(gpui_component::input::Input::new(&input))
                            .child(
                                h_flex()
                                    .gap(px(8.))
                                    .child(
                                        Button::new(("panel-review-draft-save", i))
                                            .label("Save")
                                            .on_click(cx.listener(move |this, _, _window, cx| {
                                                this.review_save_edit(cx);
                                            })),
                                    )
                                    .child(
                                        Button::new(("panel-review-draft-send", i))
                                            .label("Send to new agent")
                                            .on_click(cx.listener(move |this, _, window, cx| {
                                                this.review_send_edit(window, cx);
                                            })),
                                    ),
                            ),
                    );
                }
            }
        }
        body = body.child(
            div()
                .text_sm()
                .text_color(cx.theme().muted_foreground)
                .child(format!(
                    "Sent to agent: {}",
                    self.review.recent_sends().len()
                )),
        );
        body.into_any_element()
    }

    fn review_branch_dropdown(
        &mut self,
        repo: &RepoKey,
        is_base: bool,
        current: String,
        branches: &[String],
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let app = cx.entity().downgrade();
        let repo = repo.clone();
        let current = current;
        let label = if branches.is_empty() {
            format!(
                "{}: reading branches…",
                if is_base { "base" } else { "head" }
            )
        } else {
            format!("{}: {}", if is_base { "base" } else { "head" }, current)
        };
        Button::new(("panel-review-pick", is_base as usize))
            .ghost()
            .small()
            .dropdown_caret(true)
            .child(div().flex_1().min_w_0().truncate().child(label))
            .w_full()
            .dropdown_menu_with_anchor(gpui::Anchor::TopLeft, move |menu, _window, cx| {
                // Read at open time, not at render time: the render-time
                // list may still have been loading when this button drew.
                let fresh = app
                    .update(cx, |this, _| {
                        this.review
                            .branch_list(&repo)
                            .map(|l| l.branches.clone())
                            .unwrap_or_default()
                    })
                    .unwrap_or_default();
                let mut menu = menu.min_w(px(220.));
                if fresh.is_empty() {
                    return menu.item(PopupMenuItem::label("No branches loaded yet"));
                }
                for b in &fresh {
                    let name = b.clone();
                    menu = menu.item(
                        PopupMenuItem::new(name.clone())
                            .checked(name == current)
                            .on_click({
                                let app = app.clone();
                                let repo = repo.clone();
                                move |_, _window, cx| {
                                    let _ = app.update(cx, |this, cx| {
                                        let (mut base, mut head) =
                                            this.review.selection_for(&repo).unwrap_or_default();
                                        if base.is_empty() || head.is_empty() {
                                            // First pick seeds from the resolved pair.
                                            let resolved = this
                                                .review
                                                .resolve_selection(&repo, None, None)
                                                .unwrap_or_default();
                                            base = resolved.0;
                                            head = resolved.1;
                                        }
                                        if is_base {
                                            base = name.clone();
                                        } else {
                                            head = name.clone();
                                        }
                                        this.review.set_selection(&repo, base, head);
                                        cx.notify();
                                    });
                                }
                            }),
                    );
                }
                menu
            })
            .into_any_element()
    }

    fn review_open_row(
        &mut self,
        repo: &RepoKey,
        base: &str,
        head: &str,
        valid: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let hint = if valid {
            format!("Review {base}...{head}")
        } else {
            "Pick two different branches".to_string()
        };
        let row = h_flex()
            .id("panel-review-open")
            .w_full()
            .py(px(6.))
            .px(px(8.))
            .rounded(px(6.))
            .child(div().text_sm().child(hint));
        if !valid {
            return row
                .text_color(cx.theme().muted_foreground)
                .into_any_element();
        }
        let base = base.to_string();
        let head = head.to_string();
        row.cursor_pointer()
            .hover(|s| s.bg(cx.theme().accent.opacity(0.12)))
            .on_click(cx.listener(move |this, _, window, cx| {
                // Resolved now, not at render time (see module docs).
                let Some((host, cwd)) = this.scm_pane_target(window, cx) else {
                    return;
                };
                let source = DiffSource::Range {
                    base: base.clone(),
                    head: head.clone(),
                };
                this.open_diff_overlay(host.id(), cwd, source, None, window, cx);
            }))
            .into_any_element()
    }

    /// Probe the `base...head` snapshot behind the file list, cached per
    /// `(repo, base, head)`. A failed probe keeps the previous list and
    /// records the error rather than clearing the section.
    fn ensure_review_files(
        &mut self,
        host: SharedHost,
        repo: &RepoKey,
        base: &str,
        head: &str,
        cx: &mut Context<Self>,
    ) {
        let entry = self.review.file_list_entry(repo, base, head);
        let due = !entry.loading && entry.read_at.is_none_or(|t| t.elapsed() > BRANCH_TTL);
        if !due {
            return;
        }
        entry.loading = true;
        let root = repo.root.clone();
        let key = (repo.clone(), base.to_string(), head.to_string());
        let source = DiffSource::Range {
            base: base.to_string(),
            head: head.to_string(),
        };
        crate::ui::host_ops::HostOps::run(
            host,
            cx,
            move |h| {
                let req = DiffRequest {
                    source: source.clone(),
                    ..Default::default()
                };
                crate::terminal::git_diff::probe_diff(h, &root, &req)
            },
            move |this, snap, cx| {
                let entry = this.review.file_list_entry(&key.0, &key.1, &key.2);
                entry.loading = false;
                entry.read_at = Some(Instant::now());
                match snap {
                    Some(snap) if !snap.read_failed => {
                        entry.snapshot = Some(std::sync::Arc::new(snap));
                        entry.error = None;
                    }
                    _ => {
                        entry.error = Some("Could not read the range diff.".to_string());
                    }
                }
                cx.notify();
            },
        );
    }

    fn review_files_section(
        &mut self,
        host: SharedHost,
        repo: &RepoKey,
        base: &str,
        head: &str,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        self.ensure_review_files(host.clone(), repo, base, head, cx);
        let entry = match self.review.file_list(repo, base, head) {
            Some(e) => e.clone(),
            None => Default::default(),
        };
        let mut section = v_flex().gap(px(2.));
        if entry.snapshot.is_none() && entry.loading {
            section = section.child(
                div()
                    .text_sm()
                    .px(px(8.))
                    .text_color(cx.theme().muted_foreground)
                    .child("Reading changed files…"),
            );
            return section.into_any_element();
        }
        if let Some(err) = &entry.error {
            section = section.child(
                div()
                    .text_sm()
                    .px(px(8.))
                    .text_color(cx.theme().danger)
                    .child(err.clone()),
            );
        }
        let Some(snap) = entry.snapshot else {
            return section.into_any_element();
        };
        let total = snap.files.len();
        let unfolded = entry.unfolded;
        let shown = if total > 10 && !unfolded { 8 } else { total };
        let mono = cx.theme().mono_font_family.clone();
        for (i, file) in snap.files.iter().take(shown).enumerate() {
            let deco = crate::ui::diff_overlay::deco_status(file.status);
            let (name, dir) = split_display_path(&file.path);
            let path = file.path.clone();
            let repo = repo.clone();
            let host_id = host.id();
            let base = base.to_string();
            let head = head.to_string();
            section = section.child(
                h_flex()
                    .id(("panel-review-file", i))
                    .items_center()
                    .gap(px(8.))
                    .min_h(px(26.))
                    .w_full()
                    .min_w_0()
                    .px(px(ROW_INSET))
                    .py(px(3.))
                    .rounded(px(6.))
                    .cursor_pointer()
                    .hover(|s| s.bg(cx.theme().accent.opacity(0.12)))
                    .on_click(cx.listener(move |this, _, window, cx| {
                        // Stable repo root, not the pane's live directory.
                        // Prefer the tab's own probed snapshot so the click
                        // opens exactly what the list showed; fall back to a
                        // fresh probe when the cache has nothing.
                        let source = DiffSource::Range {
                            base: base.clone(),
                            head: head.clone(),
                        };
                        let cached = this
                            .review
                            .file_list(&repo, &base, &head)
                            .and_then(|e| e.snapshot.clone());
                        match cached {
                            Some(snap) => this.open_ready_range_diff(
                                host_id,
                                snap,
                                Some(path.clone()),
                                window,
                                cx,
                            ),
                            None => this.open_diff_overlay(
                                host_id,
                                repo.root.clone(),
                                source,
                                Some(path.clone()),
                                window,
                                cx,
                            ),
                        }
                    }))
                    .child(git_badge(status_glyph(deco), status_color(deco, cx), &mono))
                    .child(
                        div()
                            .flex_shrink(1.)
                            .min_w(px(40.))
                            .truncate()
                            .text_size(rems(TEXT))
                            .child(name.to_string()),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .text_ellipsis_start()
                            .text_right()
                            .text_size(rems(META))
                            .text_color(cx.theme().muted_foreground)
                            .child(dir.to_string()),
                    )
                    .child(
                        h_flex()
                            .flex_none()
                            .gap(px(4.))
                            .text_size(rems(META_MONO))
                            .font_family(mono.clone())
                            .when(file.added > 0, |d| {
                                d.child(
                                    div()
                                        .text_color(cx.theme().success)
                                        .child(format!("+{}", file.added)),
                                )
                            })
                            .when(file.removed > 0, |d| {
                                d.child(
                                    div()
                                        .text_color(cx.theme().danger)
                                        .child(format!("−{}", file.removed)),
                                )
                            }),
                    ),
            );
        }
        if total > shown {
            let repo = repo.clone();
            let base = base.to_string();
            let head = head.to_string();
            let text = if unfolded {
                "Show less".to_string()
            } else {
                format!("Show all {} files", total)
            };
            section = section.child(
                div()
                    .id("panel-review-files-toggle")
                    .text_sm()
                    .px(px(8.))
                    .py(px(4.))
                    .rounded(px(4.))
                    .cursor_pointer()
                    .text_color(cx.theme().muted_foreground)
                    .hover(|s| s.bg(cx.theme().accent.opacity(0.12)))
                    .on_click(cx.listener(move |this, _, _window, cx| {
                        this.review.toggle_unfolded(&repo, &base, &head);
                        cx.notify();
                    }))
                    .child(text),
            );
        }
        section.into_any_element()
    }
}
