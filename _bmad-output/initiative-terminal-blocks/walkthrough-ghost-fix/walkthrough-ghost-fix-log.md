# Walkthrough ghost-fix review log (append-only)

- 2026-10-16T00:00:00Z — session start: target `4ea4a0c8` on branch `fix/ghost-demote-failed-exact-longer` (ghost 历史补全时有时无；单文件 `src/terminal/view.rs` +84/−18). Wrote `walkthrough-ghost-fix/walkthrough-ghost-fix.md` 初版 (Intent 来自起因 + commit message 三条 / Broad strokes 3 项：ghost_suggestion、三个测试、渲染过滤点 / Slices S1–S3 / Periphery P1–P4, all blocks pending). Sources: `git show 4ea4a0c8` (doc + tier + `<=` + 1更新2新增测试), 现文件行号实测 (`ghost_suggestion` 5931 / 渲染过滤 8173 / seed 19744 / 三测试 19827·19857·19880 / 接受入口 3245·3315·3328). 未执行 `cargo test`；等用户逐块确认/发现后更新 narrative 与本 log。
- 2026-10-16 — Block 1 (Intent) 用户确认通过，无修改、无发现。narrative：Intent 标 accepted (checked)，当前块指向 Block 2 (Broad strokes)。
- 2026-10-16 — Block 2 (Broad strokes) 用户确认通过，无修改、无发现。narrative：Broad strokes 标 accepted (checked)，当前块指向 S1（失败降级）。
- 2026-10-16 — S1（失败降级）用户确认通过，无修改、无发现。narrative：S1 标 accepted (checked)，当前块指向 S2（等长找更长）。
- 2026-10-16 — S2（等长找更长）用户确认通过，无修改、无发现。narrative：S2 标 accepted (checked)，当前块指向 S3（测试与渲染口）。
- 2026-10-16 — S3（测试与渲染口）用户确认通过，无修改、无发现。narrative：S3 标 accepted (checked)，当前块指向 Periphery。
- 2026-10-16 — rebase：分支 `fix/ghost-demote-failed-exact-longer` 已 rebase 到最新 main 并 force-push，ghost 提交由旧哈希变为 `4ea4a0c8`（同题同 stat：单文件 `src/terminal/view.rs` +84/−18，b/c 改动内容不变，测试仍全绿）。narrative 与本 log 中的旧哈希已替换为新哈希；既有各块 accepted 状态不受影响。
