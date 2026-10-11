## Deferred from: code review (2026-10-08) of b06ab5c..HEAD (terminal-blocks 1.2-1.5)

- source_plan: `_bmad-output/initiative-terminal-blocks/epic-terminal-blocks/story-pane-plan.md`
  summary: handoff 照抄 record.next_block_id，未按 max+1 重算（外来 manifest 会 alias）。
  evidence: 同版本 blob 自洽故未复现；settles 需外来-manifest 证据或跨版本升级用例。
  revisit_when: 跨版本 handoff 用例出现，或 `handoff.rs:402` 附近重构；复验=构造 next<max 的 record 断言重算。
- source_plan: `_bmad-output/initiative-terminal-blocks/epic-terminal-blocks/story-pane-plan.md`
  summary: 新旧二进制混跑时 kind 64/65 在旧端整连接 break，需版本门控或降级策略。
  evidence: `break 'conn` 已确认；version handshake 存在但不管 pane 协议新 kind；属协议版本管辖，非本 epic 范围。
  revisit_when: 协议版本门控覆盖 pane 消息时；复验=旧 daemon 收 kind 64 不掉线。
- source_plan: `_bmad-output/initiative-terminal-blocks/epic-terminal-blocks/story-copy-plan.md`
  summary: 三复制 action 的剪贴板精确串缺回归单测（wiring 已人工验对：command→block_command_text、output→block_output_text、both→block_text）。
  evidence: 缺 block 覆盖（更正：`view.rs:11698` 有 TestAppContext harness，129 测试含 harness 用例，但三 action 无一使用；待补）。
  revisit_when: 用现成 harness latch 块后断言三 action 剪贴板精确串。
- source_plan: `_bmad-output/initiative-terminal-blocks/epic-terminal-blocks/story-ctrl-c-plan.md`
  summary: gutter marker (seq,folded,failed) 映射缺单测（wiring 已人工验对：is_failed() 直通第三元组）。
  evidence: 同上，待 harness-backed 映射用例（failed/success/None 三 span 第三元组）。
  revisit_when: 同上。
- source_plan: `_bmad-output/initiative-terminal-blocks/epic-terminal-blocks/story-perf-plan.md`
  summary: B 轨 GUI 真跑未跑——Xvfb 下 baseline 二进制 vs 现二进制 11MB cat 五次平均，无数据；A 轨已过门，不阻塞收 epic。
  evidence: bench harness 系 macOS-only（zsh driver `scripts/bench/run_one.sh`、`/Applications` 路径）；本容器无 zsh、无 release 二进制（仅 debug 产物），time-box 内未起构建。A 轨数据见 plan Implementation Notes。
  revisit_when: macOS 本机且 `scripts/bench/setup.sh` 语料就绪；复验=下述复现命令跑通并记录两组五次平均（门限：现二进制 ≤ 1.10 × baseline）。
  reproduce:
    - `scripts/bench/setup.sh`
    - `cargo build --release`
    - `git worktree add /tmp/tty7-perf-base 4f0e140` （1.1 前代码基线）
    - `cargo build --release --manifest-path /tmp/tty7-perf-base/Cargo.toml`
    - `scripts/bench/run_one.sh tty7 io` （现二进制 5 次平均 → `.bench/results/io-tty7.txt`）
    - `cp .bench/results/io-tty7.txt /tmp/io-head.txt`
    - `TTY7_BIN=/tmp/tty7-perf-base/target/release/tty7-app scripts/bench/run_one.sh tty7 io` （baseline → `.bench/results/io-tty7.txt`，与 `/tmp/io-head.txt` 对比）
    - `git worktree remove /tmp/tty7-perf-base`
- source_plan: `/ws/tty7/_bmad-output/initiative-terminal-blocks/plan-fold-scroll-jitter-fix.md`
  summary: toggle_fold_by_seq 本体接线（翻转前 anchor 读取、has_folds 门、alt/rows=0/锁争用跳过）缺单测
  evidence: 新纯函数单测全过但无一用例调用 toggle 本体；既有 view 单测皆为纯函数，TerminalView 方法需 gpui harness（同 mouse_grid_line 无 harness 条目）；接线回归仍靠真机验证
- source_plan: `/ws/tty7/_bmad-output/initiative-terminal-blocks/plan-fold-scroll-jitter-fix.md`
  summary: RESOLVED——toggle 接线单测已补（defer 理由不成立：gpui_tests harness 早已存在）
  evidence: view.rs gpui_tests 新增 toggle_fold_by_seq_keeps_the_top_anchor_stable_on_a_live_grid（含窗外折叠 want==offset 分支）与 toggle_fold_by_seq_falls_back_to_the_summary_row_on_a_live_grid；terminal::1139 全过。锁争用/alt-screen 跳过分支仍无单测（需精确制造争用，维持不测）。
  revisit_when: 无（关闭）
- source_plan: `_bmad-output/initiative-terminal-blocks/plan-fold-display-fix.md`
  summary: mouse_grid_line 快照验证/回退分支缺单测（函数本体需 TerminalView gpui harness）。
  evidence: view.rs:11698 有 TestAppContext harness，但 mouse 路径无一使用；fold_epoch/回退逻辑仅由 live-Term 映射单测间接覆盖。
  revisit_when: 同 P5/P6，harness 落地后补 mouse 事件级用例。
- source_plan: `_bmad-output/initiative-terminal-blocks/plan-host-ui-thread-panic-fix.md`
  summary: RESOLVED——Host call on UI thread panic（`manual_cwd_rejoins` 后台化，单飞+缓存）
  evidence: 远端 192.168.1.25 /tmp/tty7-app.log：thread 'main' panicked at crates/tty7-core/src/host/mod.rs:54:5；已单开 plan 修完待 25 机器验证
