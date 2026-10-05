# Final review package — whole branch (BASE 283dd1a..HEAD 63a11e1)

## Commits
63a11e1 chore(info-panel-tab-pane-ids): check off Task 4 (verification)
0d00289 chore(info-panel-tab-pane-ids): check off Task 3 (ordinal)
ec13eff feat(info-panel): render machine-wide tab ordinal with dash fallback
a7383c8 chore(info-panel-tab-pane-ids): scope 1.2 to Tab/Pane rows, ordinal stays in 2.1
f45751f chore(info-panel-tab-pane-ids): check off OpenSpec 1.2
1ac5a50 chore(info-panel-tab-pane-ids): check off Task 2 (ID rows)
435decc fix(info-panel): show ID rows independent of cwd report
4d35d96 feat(info-panel): render Tab ID and Pane ID rows with copy tiles
9f3879d chore(info-panel-tab-pane-ids): check off Task 1 (i18n keys)
0ab2d62 feat(info-panel): add Tab ID / Pane ID / tab ordinal i18n keys

## Stat
 .../changes/info-panel-tab-pane-ids/tasks.md       |   9 +
 .../plans/2026-10-05-info-panel-tab-pane-ids.md    | 351 +++++++++++++++++++++
 src/ui/i18n/en.rs                                  |   3 +
 src/ui/i18n/ja.rs                                  |   3 +
 src/ui/i18n/mod.rs                                 |   3 +
 src/ui/i18n/zh.rs                                  |   3 +
 src/ui/right_panel.rs                              | 119 +++++++
 7 files changed, 491 insertions(+)

## Diff
diff --git a/src/ui/i18n/en.rs b/src/ui/i18n/en.rs
index d52d9e8..aa572cc 100644
--- a/src/ui/i18n/en.rs
+++ b/src/ui/i18n/en.rs
@@ -1433,16 +1433,19 @@ pub fn translate_en(key: L10nKey) -> &'static str {
         L10nKey::PanelPortsEmpty => "No forwarded ports",
         L10nKey::PanelLatency => "latency",
         L10nKey::PortAutoForwarded => "Remote :{port} is now http://localhost:{local}",
         L10nKey::PanelCwd => "cwd",
         L10nKey::PanelCwdEditHint => "Type a path, Enter to pin it",
         L10nKey::PanelFollowNested => "follow",
         L10nKey::PanelShell => "shell",
         L10nKey::PanelSsh => "ssh",
+        L10nKey::PanelTabId => "tab id",
+        L10nKey::PanelPaneId => "pane id",
+        L10nKey::PanelOrdinal => "tab",
         L10nKey::PanelBranch => "branch",
         L10nKey::PanelChangesRow => "changes",
         L10nKey::PanelAgentWorking => "working",
         L10nKey::PanelAgentWaiting => "waiting",
         L10nKey::PanelAgentDone => "done",
         L10nKey::PanelRevealInFinder => "Reveal in Finder",
         L10nKey::PanelOpenFolder => "Open Folder",
         L10nKey::PanelOpenInBrowser => "Open in Browser",
diff --git a/src/ui/i18n/ja.rs b/src/ui/i18n/ja.rs
index f11b2ee..ac24dc9 100644
--- a/src/ui/i18n/ja.rs
+++ b/src/ui/i18n/ja.rs
@@ -1497,16 +1497,19 @@ pub fn translate_ja(key: L10nKey) -> Option<&'static str> {
         }
         L10nKey::PanelPortsEmpty => "転送中のポートはありません",
         L10nKey::PortAutoForwarded => "リモートの :{port} は http://localhost:{local} で開けます",
         L10nKey::PanelCwd => "作業ディレクトリ",
         L10nKey::PanelCwdEditHint => "パスを入力して Enter で固定",
         L10nKey::PanelFollowNested => "追従",
         L10nKey::PanelShell => "シェル",
         L10nKey::PanelSsh => "ssh",
+        L10nKey::PanelTabId => "タブ ID",
+        L10nKey::PanelPaneId => "ペイン ID",
+        L10nKey::PanelOrdinal => "タブ",
         L10nKey::PanelBranch => "ブランチ",
         L10nKey::PanelChangesRow => "変更",
         L10nKey::PanelAgentWorking => "作業中",
         L10nKey::PanelAgentWaiting => "待機中",
         L10nKey::PanelAgentDone => "完了",
         L10nKey::PanelRevealInFinder => "Finder で表示",
         L10nKey::PanelOpenFolder => "フォルダを開く",
         L10nKey::PanelOpenInBrowser => "ブラウザで開く",
diff --git a/src/ui/i18n/mod.rs b/src/ui/i18n/mod.rs
index 07ea9f8..a2af0ad 100644
--- a/src/ui/i18n/mod.rs
+++ b/src/ui/i18n/mod.rs
@@ -1113,16 +1113,19 @@ l10n_keys! {
     PanelPortsEmpty,
     PanelLatency,
     PortAutoForwarded,
     PanelCwd,
     PanelCwdEditHint,
     PanelFollowNested,
     PanelShell,
     PanelSsh,
+    PanelTabId,
+    PanelPaneId,
+    PanelOrdinal,
     PanelBranch,
     PanelChangesRow,
     PanelAgentWorking,
     PanelAgentWaiting,
     PanelAgentDone,
     PanelRevealInFinder,
     PanelOpenFolder,
     PanelOpenInBrowser,
diff --git a/src/ui/i18n/zh.rs b/src/ui/i18n/zh.rs
index 3b8aa65..4c654f0 100644
--- a/src/ui/i18n/zh.rs
+++ b/src/ui/i18n/zh.rs
@@ -1331,16 +1331,19 @@ pub fn translate_zh(key: L10nKey) -> Option<&'static str> {
         L10nKey::PanelPortsEmpty => "没有转发的端口",
         L10nKey::PanelPortsRestricted => "这里有以其他用户身份运行的进程，看不到它们的端口。",
         L10nKey::PortAutoForwarded => "远程 :{port} 现在是 http://localhost:{local}",
         L10nKey::PanelCwd => "工作目录",
         L10nKey::PanelCwdEditHint => "输入路径，回车固定",
         L10nKey::PanelFollowNested => "跟随",
         L10nKey::PanelShell => "shell",
         L10nKey::PanelSsh => "ssh",
+        L10nKey::PanelTabId => "标签页 ID",
+        L10nKey::PanelPaneId => "窗格 ID",
+        L10nKey::PanelOrdinal => "标签页",
         L10nKey::PanelBranch => "分支",
         L10nKey::PanelChangesRow => "变更",
         L10nKey::PanelAgentWorking => "进行中",
         L10nKey::PanelAgentWaiting => "等待中",
         L10nKey::PanelAgentDone => "已完成",
         L10nKey::PanelRevealInFinder => "在访达中显示",
         L10nKey::PanelOpenFolder => "打开文件夹",
         L10nKey::PanelOpenInBrowser => "在浏览器中打开",
diff --git a/src/ui/right_panel.rs b/src/ui/right_panel.rs
index 0112dd6..6eb4c55 100644
--- a/src/ui/right_panel.rs
+++ b/src/ui/right_panel.rs
@@ -371,16 +371,37 @@ enum InfoValue {
         open: Option<(crate::ui::host_ops::HostId, PathBuf)>,
     },
 }
 
 /// The table convention for a cell with nothing in it. Needs no translating,
 /// and is shorter to read than any of the sentences it stands in for.
 const EMPTY: &str = "—";
 
+/// Position of `tab` in the machine-wide tab order, 1-based. Enumerates
+/// workspaces then tabs in stored order — the GUI mirror of the CLI's
+/// `tab_index` (`crates/tty7-cli/src/resolve.rs:13`). Returns `None` when
+/// the tab is absent (mirror not yet synced), in which case the caller
+/// renders the table `EMPTY` dash.
+fn tab_ordinal(
+    machine: &tty7_core::core::machine::Machine,
+    tab: tty7_core::core::machine::TabId,
+) -> Option<u64> {
+    let mut ordinal = 0;
+    for ws in &machine.workspaces {
+        for t in &ws.tabs {
+            ordinal += 1;
+            if t.id == tab {
+                return Some(ordinal);
+            }
+        }
+    }
+    None
+}
+
 /// A round trip, at the precision the number is worth reading to.
 ///
 /// Whole milliseconds up to a second: tenths of a millisecond on a link that
 /// varies by whole ones is noise dressed as measurement. Past a second the
 /// millisecond stops mattering and the second is the unit anyone would say it
 /// in.
 fn format_rtt(rtt: std::time::Duration) -> String {
     let ms = rtt.as_secs_f64() * 1000.;
@@ -990,16 +1011,37 @@ impl Tty7App {
                             }),
                             copy: None,
                             reveal: None,
                             edit_cwd: false,
                             toggle_follow: true,
                         });
                     }
                 }
+                rows.push(InfoRow::text(t(L10nKey::PanelTabId), tab.tree_id.get().to_string()).copyable());
+                rows.push(InfoRow::text(t(L10nKey::PanelPaneId), view.pane_id.to_string()).copyable());
+                let ordinal_text = crate::core::session::WorkspaceStore::all(cx)
+                    .get(self.workspace)
+                    .and_then(|entry| {
+                        crate::ui::machine_mirror::MachineMirrors::machine(cx, entry.host_id())
+                    })
+                    .and_then(|machine| tab_ordinal(machine, tab.tree_id.get()))
+                    .map(|n| format!("@{n}"));
+                match ordinal_text {
+                    Some(decorated) => rows.push(InfoRow {
+                        label: t(L10nKey::PanelOrdinal),
+                        value: InfoValue::Text(decorated.clone()),
+                        // Paste-ready for `tty7 tab … @n`.
+                        copy: Some(decorated),
+                        reveal: None,
+                        edit_cwd: false,
+                        toggle_follow: false,
+                    }),
+                    None => rows.push(InfoRow::text(t(L10nKey::PanelOrdinal), EMPTY.to_string())),
+                }
                 // A pane on the default shell runs whatever the server picked,
                 // which is not always the login shell the inventory names: a
                 // server started from bash spawns bash. The process at the root
                 // of the pane's tree is the shell itself, so it has the say
                 // once the tree is in.
                 let root = self
                     .procs(pane_id)
                     .and_then(|p| p.procs.first())
@@ -2454,9 +2496,86 @@ mod tests {
             Some("box")
         );
         assert_eq!(
             diff(3, 1, true).copyable().copy,
             None,
             "there is no sensible clipboard form of two coloured numbers"
         );
     }
+
+    #[test]
+    fn id_rows_carry_raw_values_as_copy_payloads() {
+        let tab_id = "9f2c4b1a-3d5e-4f6a-8b7c-1d2e3f4a5b6c";
+        let tab_row = InfoRow::text("tab id", tab_id.to_string()).copyable();
+        assert!(
+            matches!(&tab_row.value, InfoValue::Text(v) if v == tab_id),
+            "tab id renders the full uuid as text"
+        );
+        assert_eq!(
+            tab_row.copy.as_deref(),
+            Some(tab_id),
+            "tab id copies the raw uuid, undecorated"
+        );
+        assert!(!tab_row.edit_cwd && !tab_row.toggle_follow && tab_row.reveal.is_none());
+
+        let pane_row = InfoRow::text("pane id", 42u64.to_string()).copyable();
+        assert!(
+            matches!(&pane_row.value, InfoValue::Text(v) if v == "42"),
+            "pane id renders the decimal pane id as text"
+        );
+        assert_eq!(
+            pane_row.copy.as_deref(),
+            Some("42"),
+            "pane id copies the raw decimal, not %42"
+        );
+    }
+
+    #[test]
+    fn ordinal_counts_workspaces_then_tabs_in_stored_order() {
+        // The GUI mirror of the CLI's `tab_index`
+        // (`crates/tty7-cli/src/resolve.rs:13`): workspaces in stored
+        // order, tabs in stored order, dense from @1.
+        let t1 = tty7_core::core::machine::Tab::leaf(1);
+        let t2 = tty7_core::core::machine::Tab::leaf(2);
+        let t3 = tty7_core::core::machine::Tab::leaf(3);
+        let m = tty7_core::core::machine::Machine {
+            workspaces: vec![
+                tty7_core::core::machine::Workspace {
+                    id: crate::core::session::WorkspaceId::new(),
+                    tabs: vec![t1.clone(), t2.clone()],
+                    ..tty7_core::core::machine::Workspace::default()
+                },
+                tty7_core::core::machine::Workspace {
+                    id: crate::core::session::WorkspaceId::new(),
+                    tabs: vec![t3.clone()],
+                    ..tty7_core::core::machine::Workspace::default()
+                },
+            ],
+            panes: Vec::new(),
+        };
+        assert_eq!(super::tab_ordinal(&m, t1.id), Some(1));
+        assert_eq!(super::tab_ordinal(&m, t2.id), Some(2));
+        assert_eq!(super::tab_ordinal(&m, t3.id), Some(3));
+    }
+
+    #[test]
+    fn ordinal_missing_from_mirror_means_no_copy_payload() {
+        // A tab id absent from the machine (mirror not yet synced):
+        // tab_ordinal returns None, and the fallback row built with
+        // InfoRow::text(LABEL, EMPTY.to_string()) has copy == None,
+        // reveal == None, and value Text("—"), so the ids still render
+        // while the ordinal cell shows the table dash.
+        assert_eq!(
+            super::tab_ordinal(
+                &tty7_core::core::machine::Machine {
+                    workspaces: Vec::new(),
+                    panes: Vec::new(),
+                },
+                tty7_core::core::machine::TabId::new()
+            ),
+            None
+        );
+        let row = InfoRow::text("tab", "—".to_string());
+        assert_eq!(row.copy, None);
+        assert!(matches!(&row.value, InfoValue::Text(v) if v == "—"));
+    }
 }
