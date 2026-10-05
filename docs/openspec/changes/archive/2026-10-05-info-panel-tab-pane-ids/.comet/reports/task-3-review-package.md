# Review package — Task 3 (BASE a7383c8..HEAD ec13eff)

## Commits
ec13eff feat(info-panel): render machine-wide tab ordinal with dash fallback

## Stat
 src/ui/right_panel.rs | 90 +++++++++++++++++++++++++++++++++++++++++++++++++++
 1 file changed, 90 insertions(+)

## Diff
diff --git a/src/ui/right_panel.rs b/src/ui/right_panel.rs
index 180947d..6eb4c55 100644
--- a/src/ui/right_panel.rs
+++ b/src/ui/right_panel.rs
@@ -369,20 +369,41 @@ enum InfoValue {
         added: u32,
         removed: u32,
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
     if ms < 1. {
         // Loopback and a peer on the same LAN both land here. Rounding to
@@ -990,20 +1011,39 @@ impl Tty7App {
                             }),
                             copy: None,
                             reveal: None,
                             edit_cwd: false,
                             toggle_follow: true,
                         });
                     }
                 }
                 rows.push(InfoRow::text(t(L10nKey::PanelTabId), tab.tree_id.get().to_string()).copyable());
                 rows.push(InfoRow::text(t(L10nKey::PanelPaneId), view.pane_id.to_string()).copyable());
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
                     .filter(|p| p.depth == 0)
                     .map(|p| p.name.trim_start_matches('-').to_string())
@@ -2481,11 +2521,61 @@ mod tests {
         assert!(
             matches!(&pane_row.value, InfoValue::Text(v) if v == "42"),
             "pane id renders the decimal pane id as text"
         );
         assert_eq!(
             pane_row.copy.as_deref(),
             Some("42"),
             "pane id copies the raw decimal, not %42"
         );
     }
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
