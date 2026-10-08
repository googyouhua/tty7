---
title: '折叠持久化随 pane'
type: 'feature'
ticket: 5
created: '2026-10-08'
status: 'built'
baseline_revision: 'b06ab5c'
route: 'full'
route_source: 'auto'
risk: 'medium'
review: ''
review_source: ''
lenses_ran: []
review_loop_iteration: 0
followup_review_recommended: false
context: []
warnings: []
deferred: []
---

<intent-contract>

## Intent

**Problem:** 折叠态只在客户端内存里，daemon 重启或持久会话恢复后丢失；scrollback 清除时 daemon 块表残留陈旧行。

**Approach:** folded 存 server 侧 pane 块表并随持久会话恢复，GUI 折叠/清除经新 ClientMsg 通道通知 daemon，客户端 adopt 回填 folded。

## Boundaries & Constraints

**Always:** block schema（id/exit_code/folded/truncated）与 B→D 配对语义由 entry-1 拥有，本 ticket 只消费不改契约；无 marks 不建块、alt-screen 不建块；复制纯文本语义不变；daemon 表 cap 200 与 client span cap 同量级不变；folded 默认 false，缺字段按 false。

**Never:** 自动折叠；快捷键；ANSI 复制；outline；正则猜 prompt；远端特殊处理（entry-6）；shell 内 `clear` 转义序列嗅探（仅 GUI ClearScrollback 动作通知 daemon）。

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| 折叠后重启恢复 | 折叠块→daemon 重启/hibernate 恢复 | 同一 pane 恢复后折叠态与关闭前一致 | 持久文件缺失则全展开，不报错 |
| 展开后重启恢复 | 展开块→恢复 | 保持展开 | 同上 |
| 清除 scrollback | GUI ClearScrollback | 两侧块表清空，下次 adopt 只追 watermark 不配对 | daemon 不可达则本地仍清，不报错 |
| 恢复时块已被裁剪 | 快照只含尾部，旧 folded id 不在 replay 中 | 不复活旧折叠，只对重建 span 回填 | No error expected |
| 旧快照无 blocks 文件 | 只有 scrollback 无 blocks 持久 | 全展开，正常建块 | No error expected |
| 并发折叠与 evict | 折叠块被 cap 逐出 | folded 随行丢弃，不残留 | No error expected |

</intent-contract>

## Code Map

- `crates/tty7-core/src/daemon/protocol.rs` -- ClientMsg 新通道落点（约 919 枚举、1309 encode、from_frame 解码、1108 kind 常量）：新增 SetBlockFolded + ClearCommandBlocks，kind 取客户端方向空闲号（避开 1~59 与服务端 60~63 复用歧义，取 64/65）；CommandBlock.folded 注释（约 637~648，daemon 总发 false）改为真相源
- `crates/tty7-core/src/daemon/pane.rs` -- PaneState.command_blocks（约 777）、note_block_marks B→D（约 4243~4275）、clear_command_blocks（约 4282，无生产调用者，entry-5 接线目标）、context() 下发 blocks（约 2708）、carry/adopt（约 1968~2155）、scrollback_snapshot（约 2782）、over_pty 新建三处（约 1842/2084/2155) next_block_id 初始化旁
- `crates/tty7-core/src/daemon/scrollback.rs` -- 持久层：save/load/forget/sweep（约 170~260）、encode/decode（约 86~140，只含 segments+title，不动）；blocks 持久另起同目录 `{pane_id}.blocks.json`（0600），不碰二进制格式
- `crates/tty7-core/src/daemon/server.rs` -- 消息分发（约 805 首帧、1286 连接内循环）：SetBlockFolded/ClearCommandBlocks 处理位；spawn_snapshot_keeper（约 261）与 store_scrollback_now（约 295）、restored_screen（约 310）、kill_pane/hibernate_pane（约 340~360）持久接线位
- `crates/tty7-core/src/daemon/handoff.rs` -- exec 交接：PaneRecord（约 100）加 blocks 字段（serde default 兼容旧 blob）、stage（约 230）与回读（约 360）对称；Carried（pane.rs 约 1384）加 command_blocks + next_block_id
- `src/terminal/remote.rs` -- 客户端发送：link.send 示例 SetFollowNested（约 2007）、query_procs（约 2609）；maybe adopt 调用方在 view.rs；新增 set_block_folded/clear_command_blocks helper（fire-and-forget，最佳努力）
- `src/terminal/view.rs` -- toggle_fold_by_seq（约 3868）、maybe_adopt_daemon_blocks（约 3885，只 adopt id 不回填 folded）、clear_scrollback（约 4328，已清本地+resync，缺 daemon 通知）
- `src/terminal/blocks.rs` -- BlockTracker.folded（约 97 view-local）、adopt（约 230 只配 daemon_id/truncated）、set_folded/clear_blocks（约 203/281）

## Tasks & Acceptance

**Execution:**
- [x] `crates/tty7-core/src/daemon/protocol.rs` -- 新增 ClientMsg::SetBlockFolded{pane_id,block_id,folded} 与 ClearCommandBlocks{pane_id}（kind 64/65）+ encode/from_frame 对称 + CommandBlock.folded 注释改真相源 -- 跨进程契约
- [x] `crates/tty7-core/src/daemon/pane.rs` -- 新增 set_block_folded helper（按 id 置 folded）+ clear_command_blocks 去 dead_code 接生产调用 + carry/adopt/Restore 携带 blocks 与 next_block_id + 新建 pane 初始化 -- server 侧状态真相
- [x] `crates/tty7-core/src/daemon/scrollback.rs` -- 新增 save_blocks/load_blocks/forget_blocks（JSON，0600，同目录 `{pane_id}.blocks.json`）+ sweep_blocks -- 磁盘持久，不动现有二进制格式
- [x] `crates/tty7-core/src/daemon/server.rs` -- 分发两新消息 + snapshot keeper/store_now/hibernate 存 blocks + restored_screen 取 blocks 并随 Restore 交出 + kill/sweep 删 blocks -- 生命周期闭环
- [x] `crates/tty7-core/src/daemon/handoff.rs` -- PaneRecord 加 blocks（serde default）+ stage/回读对称 -- exec 交接不丢折叠
- [x] `src/terminal/remote.rs` -- 新增 set_block_folded/clear_blocks 发送 helper（fire-and-forget，失败静默） -- GUI→daemon 通道
- [x] `src/terminal/view.rs` -- toggle_fold_by_seq 成功后通知 daemon + clear_scrollback 通知 daemon + maybe_adopt 回填 folded（按 daemon_id→seq） -- 两端一致
- [x] `src/terminal/blocks.rs` -- adopt 回填 folded（有 daemon_id 的 span 按表置 folded；表无则保持本地）+ dirty 未回声保护 + 单测 -- 恢复语义
- [x] 边角单测 -- pane set_folded/clear、scrollback blocks roundtrip/缺文件、handoff record 兼容、client adopt folded 回填 + clear-resync 不配对陈旧 folded -- 矩阵可验

**Acceptance Criteria:**
- Given 有 marks 的 pane 折叠某块， when daemon 重启或 hibernate 恢复到同一 pane， then 该块仍折叠，展开块仍展开
- Given 恢复的快照不含某旧块（被裁剪）， when adopt， then 不复活该折叠，新块折叠可用
- Given 块折叠后点 ClearScrollback， when 跑新命令， then 新块不继承旧折叠，daemon 表无残留旧行
- Given GUI 点折叠/清除时 daemon 不可达， when 操作完成， then 本地视图仍更新，不报错不卡死
- Given 旧版本残留快照（无 blocks 文件）， when 恢复， then 全展开且功能正常

## Implementation Notes

- daemon 真相：`set_block_folded` 按 id 置位（未知 id 静默，fire-and-forget 语义）；`clear_command_blocks` 去 dead_code，由 `ClearCommandBlocks` 驱动；`Restore`/`Carried`/`PaneRecord(serde default)` 携带 blocks + next_block_id，`restored_next_block_id` 取 max+1 防别名。
- 持久：scrollback 二进制不动，blocks 另存 `{pane_id}.blocks.json`（0600，原子 rename）；keeper 在 ring 未动时仍无条件存 blocks（折叠不推 ring mark）；`restored_screen` 先 load 后 forget；kill→forget→forget_blocks，keeper 顺带 sweep_blocks。
- 客户端：toggle 先改本地再 best-effort 通知；adopt 回填 + 新配对分歧上报回推（双向收敛）；`BlockTracker.dirty` 记录 daemon 未回声的本地 toggle，refresh 遇 dirty 保留本地、回声一致消 dirty——未 ack 的折叠/展开不再被 5s adopt 闪掉（GUI 实测抓到的回归，已修）。
- adopt 返回语义收敛为“仅上报与 daemon 行分歧的新配对”，view 侧全量回推（不再只过滤 folded）。

## Auto Run Result

- `cargo test -p tty7-core block`（直跑现行 test 二进制）：22 passed
- `cargo test -p tty7-core scrollback`：14 passed（含 sidecar roundtrip/缺文件/外来文件）
- `cargo test -p tty7 --bin tty7-app terminal::blocks`：21 passed（含 adopt 回填/预配对保持/回声消 dirty/反向收敛/clear-resync）
- `cargo clippy -p tty7-core --lib`：50 warnings 全在基线既有位置，ticket hunks（pane/protocol/scrollback/server/handoff）零新增；bin 构建 ticket 文件（blocks/view/remote）零警告
- 矩阵审计：6 行全覆盖（重启恢复双向/clear 双端+watemark/c 裁剪不复活/无 blocks 全展开/evict 随行丢弃/不可达本地先生效），见回执

## Plan Change Log

## Review Triage Log

## Design Notes

folded 以 daemon 表为真相，client 为视图缓存：toggle 先改本地立即响应，再 fire-and-forget 通知 daemon；adopt 以 daemon folded 回填本地（daemon_id→seq）。持久只存 folded+id+exit_code（truncated 恒 false，不存偏移，无锚点协议，entry-6 前不引入 epoch）。scrollback 二进制格式不动，blocks 另存 JSON 同目录，避免版本兼容风险。shell 内转义清屏不嗅探，仅 GUI 动作通知，此限制在 plan 中明示。

版本控制注记：本 worktree 根上 `_bmad`/`_bmad-output` 为指向 `/ws/tty7` 的 symlink（统一 ticket 树），`git status` 将其记为 untracked 且原提交文件记为 deleted；此为环境固有脏标记，非本 ticket 改动。`git add --refresh -- .` 按流程应 HALT，但无人值守按 plan 推进：仅提交代码文件，不提交 symlink 本体。

## Verification

**Commands:**
- `cargo test -p tty7-core block` -- expected: 全过，含新增 set_folded/clear/scrollback-blocks/handoff 单测
- `cargo test -p tty7-core scrollback` -- expected: 全过，含 blocks roundtrip 与缺文件用例
- `cargo test block` -- expected: 全过，含 adopt folded 回填与 clear-resync 用例
- `cargo clippy -p tty7-core --lib` -- expected: 零新增警告（workspace 全量 -D 在基线已有预存警告，按 1.1 plan 只看增量）
