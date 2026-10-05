# Review package — Task 1 (BASE 283dd1a..HEAD 0ab2d62)

## Commits
0ab2d62 feat(info-panel): add Tab ID / Pane ID / tab ordinal i18n keys

## Stat
 src/ui/i18n/en.rs  | 3 +++
 src/ui/i18n/ja.rs  | 3 +++
 src/ui/i18n/mod.rs | 3 +++
 src/ui/i18n/zh.rs  | 3 +++
 4 files changed, 12 insertions(+)

## Diff
diff --git a/src/ui/i18n/en.rs b/src/ui/i18n/en.rs
index d52d9e8..aa572cc 100644
--- a/src/ui/i18n/en.rs
+++ b/src/ui/i18n/en.rs
@@ -1431,20 +1431,23 @@ pub fn translate_en(key: L10nKey) -> &'static str {
             "Something here runs as another user, whose ports aren't visible."
         }
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
         L10nKey::ScmGroupMerge => "Merge Changes",
         L10nKey::ScmGroupStaged => "Staged Changes",
diff --git a/src/ui/i18n/ja.rs b/src/ui/i18n/ja.rs
index f11b2ee..ac24dc9 100644
--- a/src/ui/i18n/ja.rs
+++ b/src/ui/i18n/ja.rs
@@ -1495,20 +1495,23 @@ pub fn translate_ja(key: L10nKey) -> Option<&'static str> {
         L10nKey::PanelPortsRestricted => {
             "他のユーザーで動いているプロセスがあり、そのポートは見えません。"
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
         L10nKey::ScmGroupMerge => "マージの競合",
         L10nKey::ScmGroupStaged => "ステージされた変更",
diff --git a/src/ui/i18n/mod.rs b/src/ui/i18n/mod.rs
index 07ea9f8..a2af0ad 100644
--- a/src/ui/i18n/mod.rs
+++ b/src/ui/i18n/mod.rs
@@ -1111,20 +1111,23 @@ l10n_keys! {
     PanelPortsProbeFailed,
     PanelPortsRestricted,
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
     ScmGroupMerge,
     ScmGroupStaged,
diff --git a/src/ui/i18n/zh.rs b/src/ui/i18n/zh.rs
index 3b8aa65..4c654f0 100644
--- a/src/ui/i18n/zh.rs
+++ b/src/ui/i18n/zh.rs
@@ -1329,20 +1329,23 @@ pub fn translate_zh(key: L10nKey) -> Option<&'static str> {
         L10nKey::PanelPortsUnsupported => "对端的 tty7-server 太旧，列不出端口。",
         L10nKey::PanelPortsProbeFailed => "没能查出这个窗格在监听什么。",
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
         L10nKey::ScmGroupMerge => "合并冲突",
         L10nKey::ScmGroupStaged => "暂存的更改",
