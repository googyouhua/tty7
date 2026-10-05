# Review package — Task 2 (BASE 9f3879d..HEAD 435decc)

## Commits
435decc fix(info-panel): show ID rows independent of cwd report
4d35d96 feat(info-panel): render Tab ID and Pane ID rows with copy tiles

## Stat
 src/ui/right_panel.rs | 29 +++++++++++++++++++++++++++++
 1 file changed, 29 insertions(+)

## Diff
diff --git a/src/ui/right_panel.rs b/src/ui/right_panel.rs
index 0112dd6..180947d 100644
--- a/src/ui/right_panel.rs
+++ b/src/ui/right_panel.rs
@@ -988,20 +988,22 @@ impl Tty7App {
                                 true => t(L10nKey::SettingsValueOn).to_string(),
                                 false => t(L10nKey::SettingsValueOff).to_string(),
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
@@ -2452,11 +2454,38 @@ mod tests {
                 .copy
                 .as_deref(),
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
 }
