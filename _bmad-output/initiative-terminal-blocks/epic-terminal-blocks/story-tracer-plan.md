---
title: '块模型 tracer：配对建块贯通折叠与菜单脚手架'
type: 'feature'
ticket: 1
created: '2026-10-07'
status: 'done'
baseline_revision: '4f0e140'
route: 'full'
route_source: 'auto'
risk: 'medium'
review: 'quick'
review_source: 'pinned'
lenses_ran: ['quick']
review_loop_iteration: 1
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** scrollback 是平的，没有“一次执行”的单位，长输出回翻定位与完整复制成本高。

**Approach:** server 侧以 OSC 133 B→D 配对建块表并下发客户端，GUI 以 gutter 折叠与右键两项（跳块首、复制两者）接通同一块 id，跑通最薄全链路。

## Boundaries & Constraints

**Always:** 边界唯一来源 OSC 133 B→D；无 marks 不建块；alt-screen 期间不建块；复制纯文本；block schema（id/exit_code/folded/truncated）与退出码行格式由本 ticket 拥有，后续 tickets 只消费。

**Never:** 自动折叠；快捷键；ANSI 复制；outline；正则猜 prompt；持久化接线（entry-5）；远端特殊处理（entry-6）。

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| 正常命令 | `B cmd … C output D;0` | 建一块，gutter 可折叠为一行，展开无损 | No error expected |
| Ctrl-C 中断 | `B cmd … D;130` | 成块， exit_code=130（红标在 entry-4） | No error expected |
| 无集成 | 无 marks 的输出 | 不建块，scrollback 原样 | No error expected |
| 全屏程序 | alt-screen 期 marks | 不建块 | No error expected |
| 未配对 | 有 B 无 D（命令仍在跑） | 不建块，D 到达才闭合 | No error expected |
| scrollback 清除 | 清除含块区域 | 块记录删除 | No error expected |

</frozen-after-approval>

## Code Map

- `crates/tty7-core/src/daemon/pane.rs` -- 建块表落点：OscSniffer/handle_osc133 附近（约 4190/4226），C 开块、D 闭合，存 PaneState；输出切片紧邻 ReplayRing/TerminalModes
- `crates/tty7-core/src/core/osc.rs` -- OscTokenizer::feed_at（约 40），payload 切分，已有，不改
- `crates/tty7-core/src/core/term_modes.rs` -- TerminalModes::feed（约 27），alt-screen 门控查询源
- `crates/tty7-core/src/daemon/protocol.rs` -- PaneContext（约 588），块表下发客户端的契约位置
- `src/terminal/remote.rs` -- ReaderCut/command_cuts（约 1347/1518/3649）与 CommandMark::parse 消费示例，客户端接块表参考
- `src/terminal/view.rs` -- actions!（约 113）、context_menu builder（约 8161）、gating（8056/8576），新增 2 菜单项与 Action
- `src/terminal/element.rs` -- 右键分流（约 2097）与绘制，gutter strip 与 marker 点击落点（参考 render_scrollbar 外挂 child 模式）
- `src/terminal/view.rs:3732 copy_selection` -- 块复制复用的纯文本路径（selection_to_string + 修剪 + clipboard）
- `src/terminal/blocks.rs`（新建） -- loopback 1：span 在 B 处打开、C 仅注释（与 daemon 对齐）；clear 武装 resync；adopt 只做丰富不做路由
- `src/terminal/remote.rs cut_anchor` -- loopback 1：锚点不用 display_offset（cursor 行即网格坐标，绝对行 = history + line）
- `src/terminal/view.rs block_text` -- loopback 1：经 term.bounds_to_string（与 screen.rs capture 同函数）取文本，WRAPLINE 由 alacritty 语义处理

## Tasks & Acceptance

**Execution:**
- [ ] `crates/tty7-core/src/daemon/pane.rs` -- 新增块表（id/exit_code/folded/truncated）与 B→D 配对开闭合，alt-screen 门控，无 marks 不建块 -- 真相源，entry-1 拥有 schema
- [ ] `crates/tty7-core/src/daemon/pane.rs` -- I/O 矩阵边角单测（中断/无集成/全屏/未配对/清除） -- 可验边角
- [ ] `crates/tty7-core/src/daemon/protocol.rs` -- 块表下发客户端契约（复用 PaneContext/现有帧，不另起通道） -- 全链路可达
- [ ] `src/terminal/view.rs` -- 新增 2 Action 与右键菜单项（跳到块首、复制两者），无块光标下隐藏 -- 菜单脚手架
- [ ] `src/terminal/element.rs` -- gutter marker 列绘制与单击折叠/展开（含折叠一行摘要） -- 可见折叠
- [ ] 块复制走 copy_selection 同源语义并附退出码行 -- 与 capture --plain 一致
- [ ] `src/terminal/blocks.rs` -- B 处开 span、C 仅注释（去掉 C 门，与 daemon 一致），补 B 无 C 配对单测 -- 收敛双边配对语义
- [ ] `src/terminal/remote.rs` -- cut_anchor 去掉 display_offset 项，补滚动中落 D 的锚点单测 -- 长输出滚动场景折叠/复制不错位
- [ ] `src/terminal/view.rs block_text` -- 改经 term.bounds_to_string 取段文本（与 screen.rs capture 同函数），保留退出码行与修剪，补换行包裹复制单测 -- 与 plain 语义逐字一致
- [ ] `src/terminal/blocks.rs` -- clear 武装 resync（下次 adopt 只追 watermark 不配对）+ spans 上限（与 daemon 200 同量级）；`pane.rs clear_command_blocks` 保持 tested helper，daemon 侧接线递延 entry-5 -- 无陈旧配对、无无界增长、无沉默缺口

**Acceptance Criteria:**
- Given 有 marks 的 pane 跑出万行命令， when 点 gutter marker， then 块折为一行，展开后内容无损
- Given 块内任意位置， when 右键选跳到块首， then 视口置于块首命令行
- Given 块内任意位置， when 右键复制两者， then 剪贴板得纯文本命令+输出+退出码行
- Given 无 marks 的 pane， when 跑命令， then 不建块、无 marker、无菜单项
- Given vim 全屏期间， when 有输出， then 不建块
- Given B→D 间无 C 字节， when D 到达， then 建一块并与首个 daemon id 配对（client/daemon 同语义）
- Given 输出含终端自动换行， when 复制整块， then 与同范围 bounds_to_string 逐字一致（无多余换行）
- Given 向上滚动中长命令落 D， when 折叠该块， then 折为一行且复制含尾部各行（锚点为真绝对行）
- Given 清除 scrollback 后跑新命令， when 下次 adoption， then 新 span 只与 clear 后 daemon id 配对（resync 丢弃陈旧快照）

## Design Notes

退出码行格式：`exit <code>` 独占末行（如 `exit 0`）。菜单无块光标下隐藏（沿 8161 空菜单抑制先例），不做 disabled 置灰。

 pairing 生命周期（loopback 1 增补）：client 侧 span 以本地 seq 与绝对行寻址，fold/jump/copy 只认本地 span；daemon 表是审计/持久真相，adoption 只是尽力而为的丰富（daemon_id/truncated 写入 span 但 tracer 不消费路由，entry-6 定义 epoch 协议前不依赖它）。clear 后首次 adopt 只做 resync（watermark 追到快照最大 id，不配对），避免陈旧配对；client spans 设与 daemon 同量级上限，避免无界增长；daemon 侧 clear 接线明确递延 entry-5（需新增 ClientMsg 通道）。

## Verification

**Commands:**
- `cargo test -p tty7-core block` -- expected: 全过，含新增边角单测
- `cargo clippy --workspace -- -D warnings` -- expected: 零警告
- `cargo build` -- expected: 成功

**Manual checks (if no CLI):**
- GUI 跑长命令验证折叠一行、跳块首、复制三段；无集成 pane 与 vim 无 marker

## Implementation Notes

- Subagent implemented 1.1 pre-restart (server B→D table in pane.rs, CommandBlock wire type, client BlockTracker in remote.rs + blocks.rs, gutter/paint/mouse in element.rs, menu/jump/copy in view.rs, i18n x4); dispatch aborted before report, resumed inline.
- Finished one leftover: blocks.rs parse test failed (`B` unhandled by CommandMark::parse) — fixed parse_command_mark to map `B`→Started; blocks tests 12/12 pass.
- Matrix audit: all 6 rows covered by ran+passed tests (10 pane block tests). Full core suite 1769 pass; 3 failures (spawn signal, 2 shell-integration bootstraps) reproduce on clean baseline — environment, not this change.
- cargo check --workspace clean (only pre-existing warnings).

## Review Triage Log

- F1 daemon-clear-unwired — medium — `clear_command_blocks` (pane.rs:942) has no production caller (only its own test); client `clear_blocks` drops spans but keeps the adoption watermark while the daemon keeps pre-clear rows, so post-clear spans can pair against stale daemon ids and `PaneContext.blocks` advertises rows the GUI no longer shows. GUI-visible clear itself works.
- F2 B-without-C asymmetry — medium — client `note_d` needs an open span (created only by `C`), daemon closes on `D` for any pending `B` (its own test asserts a C-less pair builds); `shell_integration.rs:516` documents commands running with no `133;C`, so a live pair yields a daemon block with no client span and shifts every later close-order pairing by one.
- F3 adoption-unconsumed — low — verified zero readers of `BlockSpan::daemon_id`/`span.truncated` outside `blocks.rs` tests; fold/jump/copy key off local seq/abs rows, so the 5s poll stores ids nothing consumes yet (entry-6 will).
- F4 eviction-divergence — medium — `CommandBlock` carries no offsets and the two sides prune differently (daemon: ring eviction + SNAPSHOT_CAP + 200-cap; client `prune(top_abs)` where `topmost_line()==-(history)` makes `top_abs` 0, i.e. client effectively retains everything), so once the daemon evicts a prefix the client keeps, close-order pairing misattributes ids/flags.
- F5 copy-wrap divergence — medium — `block_text` emits `\n` per grid row and only skips `WIDE_CHAR_SPACER`, while `line_to_string` joins `WRAPLINE` rows and handles tabs/wide/zero-width; wrapped lines (routine at 120 cols) copy with spurious newlines, contradicting the plan's same-as-plain acceptance criterion.
- F6 anchor understated while scrolled — high — `cut_anchor` subtracts `display_offset`, but cursor rows are grid coordinates (paint path uses `history + line`); `display_offset` survives output, so a `D` landing mid-scroll understates `end_abs` by the scroll distance and the fold leaks rows while copy drops trailing rows — exactly the long-output-while-scrolled headline scenario.

Grouping: G1 {F1,F3,F4} share one root cause (close-order pairing without shared anchors or epochs; half-built lifecycle) — routes bad_plan by highest verdict (medium). G2 {F2}, G3 {F5}, G4 {F6} are trivially fixable with no new surface — patch, moot on loopback and carried into the plan amendment below.

## Plan Change Log

- 2026-10-07 quick-lens loopback 1: review filed 6 findings; triage grouped G1 {daemon-clear-unwired, adoption-unconsumed, eviction-divergence} as bad_plan (close-order pairing without shared anchors/epochs, half-built lifecycle) and G2 {B-without-C asymmetry}, G3 {copy-wrap divergence}, G4 {scrolled anchor} as patch. Amended only non-frozen sections: +4 tasks/+4 AC below, Design Notes pairing paragraph, Code Map pointers. Known-bad avoided: stale post-clear adoption, C-gated spans diverging from daemon rows, hand-rolled copy text, display_offset in anchors. KEEP: daemon B→D table + its 10 matrix tests; CommandBlock wire shape; BlockTracker/FoldMap/gutter paint/mouse mapping; menu latch + jump/copy actions; i18n keys; `exit <code>` line format.

## Implementation Notes (loopback 1 re-derive)

- Reverted first implementation per bad_plan; subagent re-derived from amended plan. Notable shape changes: daemon stores CommandBlock rows directly (cap 200 as COMMAND_BLOCK_CAP); per-mark alt gating via scratch TerminalModes fold; client opens at B (C ignored entirely); block_text via Term::bounds_to_string; adoption resync flag + client cap; CommandBlock drops `command` (span keeps local text; daemon_id/truncated pair best-effort).
- Matrix audit: all 6 frozen rows + 4 amended ACs covered by ran+passed named tests (core 10 command_blocks_* incl. close_without_c, coalesce, unparsable_exit; client b_to_d with/without C, wrapped-lines copy, scrolled-D anchor, clear-resync). Full core suite 1769 pass + same 3 baseline-env failures; bin block 38 pass; fmt clean; clippy adds zero warnings (workspace -D fails on baseline too, pre-existing).
- Left for live GUI eyeball: fold-to-one-line paint, gutter alignment, click strip, jump/copy feel (headless env).

## Review Triage Log (loopback 2)

- G5 gutter-marker-unclickable — high — verified: strip at surface x 0..14, glyph centered ~x7; element hitbox starts at surface x=8 (GRID_PAD_X) and gates the handler, hit-test covers element-local x<6 (surface 8..14): only a ~2px sliver of the visible marker toggles, the rest just focuses. Clicking the marker does not fold — breaks a CAP-1 acceptance criterion. Patch.
- G6 stale-menu-latch — medium — verified: record_menu_block (view.rs:3803) maps to None correctly, but only runs inside the hitbox-gated element handler; right-clicks on the 8px padding strip open the surface menu with a stale menu_block (wrong jump/copy). Patch.
- G7 search-flags-count-hidden — low — verified (element.rs:2143-2147 flags set before per-line to_screen at 2148+): folded-away matches report in-view hits with nothing highlighted. Touches the spec non-goal (search linkage) but the change owns the inconsistency it introduced. Patch (set flags only when >=1 line maps).
- G8 cap-eviction-aliasing — medium — verified: history_size caps at scrolling_history (default 10000) while abs addressing assumes no eviction; prune floor (history+topmost) is always 0 so nothing ever prunes; past the cap, fold/copy/gutter alias live rows and block_text clamps instead of None. Headline-adjacent (AC is 10k lines). Patch (cap-overflow reuses the clear+resync path; small-scrollback tests): no new surface, guards demonstrated state — not bad_plan.
