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
  summary: RESOLVED——三复制 action 剪贴板精确串回归单测已补。
  evidence: view.rs gpui_tests 新增 three_copy_actions_write_their_exact_clipboard_strings（rooted_harness 真 daemon 输出 latch `record_menu_block(0)`，断言 command→`$ cmd\nexit 0`、output→`out1\nout2\nexit 0`、both→`$ cmd\nout1\nout2\nexit 0`）；terminal::1170 全过。
  revisit_when: 无（关闭）
- source_plan: `_bmad-output/initiative-terminal-blocks/epic-terminal-blocks/story-ctrl-c-plan.md`
  summary: RESOLVED——gutter marker (seq,folded,failed) 映射单测已补。
  evidence: view.rs gpui_tests 新增 gutter_markers_carry_seq_fold_and_failed_per_row（exit 2→`(1,false,true)`、exit 0→`(2,false,false)`、无 span 行→None）；terminal::1170 全过。
  revisit_when: 无（关闭）
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
  summary: RESOLVED——mouse_grid_line 快照验证/回退分支单测已补。
  evidence: view.rs gpui_tests 新增 mouse_grid_line_guards_the_window_and_the_alternate_screen（越界行→None、alt-screen 取 identity）与 mouse_grid_line_falls_back_live_when_a_fold_invalidates_the_slot（第二折叠致 slot 过期→live `visible_map`，且断言 live≠stale 防空转）；terminal::1170 全过。frame 短 map 空白行→None 与 try_lock 锁争用分支仍无单测（需精确制造，维持不测）。
  revisit_when: 无（关闭；空白行/锁争用分支维持不测）
- source_plan: `_bmad-output/initiative-terminal-blocks/plan-host-ui-thread-panic-fix.md`
  summary: RESOLVED——Host call on UI thread panic（`manual_cwd_rejoins` 后台化，单飞+缓存）
  evidence: 远端 192.168.1.25 /tmp/tty7-app.log：thread 'main' panicked at crates/tty7-core/src/host/mod.rs:54:5；已单开 plan 修完待 25 机器验证

- source_plan: `_bmad-output/initiative-terminal-blocks/epic-username-selector/story-mtime-recency-tracer-plan.md`
  summary: RESOLVED——bin 测试目标恢复编译（defer 理由不成立：E0252 已消失）。
  evidence: `cargo check --bin tty7-app --tests` 仅 warning 无报错；`cargo test --bin tty7-app instance_picker` 25/25 全绿（含 rows_retain_snapshot_mtimes）。
  revisit_when: 无（关闭）

- source_plan: `_bmad-output/initiative-terminal-blocks/epic-username-selector/story-core-delete-op-plus-probe-plus-memory-guard-plan.md`
  summary: 全量 tty7-core 套件 3 个失败（daemon shell_integration x2、spawn x1），与 instance 改动无关。
  evidence: 评审在基线 stash 下复现同败；diff 仅碰 instance.rs，被败模块不依赖 instance（仅 remote_link.rs 引用）。
  revisit_when: daemon 测试在 CI/本机稳定后；复验=cargo test -p tty7-core 全绿。

## Deferred from: code review of story-md-preview-copy-plan.md (2026-10-14)

- source_plan: `_bmad-output/backlog/story-md-preview-copy-plan.md`
  summary: 预览快捷键复制（Ctrl+C）与右键复制一致性未在真机验证
  evidence: 上游 selectable TextView 自带绑定但本分支只验证了右键路径；25 机选中后按 Ctrl+C 比对粘贴即可证伪或证实
- source_plan: `/ws/tty7/_bmad-output/initiative-terminal-blocks/plan-markdown-local-link.md`
  summary: 含图片/引用链接/HTML 的段落整段回退，段内本地链接仍走旧 open_url
  evidence: preview_link_unhandled 有意回退；日常文档图片与链接分段，命中率低
- source_plan: `/ws/tty7/_bmad-output/initiative-terminal-blocks/plan-markdown-local-link.md`
  summary: 接管段落拍平为纯文本（加粗/斜体/行内代码样式丢失，换行不保留）
  evidence: 词与链接目标保留、复制可用；恢复 marks 需重做内联渲染，非小补丁
- source_plan: `/ws/tty7/_bmad-output/initiative-terminal-blocks/plan-ws-unreachable-ui-freeze.md`
  summary: 全屏 occlude 遮罩（daemon-mismatch 无 cancel、auth sheet scrim）在无可操作内容时仍拦截全部点击。
  evidence: 本次 native 重拨已落为非模态 Failed；剩余 trap 为既有行为，本次 diff 未触及 prompt/scrim，属旧 bug/后续想法。
- source_plan: `/ws/tty7/_bmad-output/initiative-terminal-blocks/plan-ws-unreachable-ui-freeze.md`
  summary: 纯远端 workspace（零 native pane）打开不可达时是否仍冻结（maybe-false，若真则为 high）。
  evidence: 远端 new_terminal 分支本已异步，本次只修了 native 同步重拨；需复现命名 ws 类型才能判定，复验=按 ws 类型分别复现切换/配置可达性。
- source_plan: none
  summary: pane 链路从每 pane 一条 SSH session 改为 direct-streamlocal 到 daemon.sock，消除 MaxSessions 触顶与每 pane 一个 `--stdio --pane` 进程（`crates/tty7-core/src/daemon/ssh/mod.rs:496`）。
  evidence: 与“连接/认证卡死 UI”拆分：它不改变冻结点本身，只减少再次触发认证的前提（重连触顶后另拨链路），可独立评审与合并。
- source_plan: `/ws/tty7/_bmad-output/initiative-terminal-blocks/plan-ssh-connect-ui-freeze.md`
  summary: auth_sheet_window 的 `most_recent` 兜底不校验窗口可见性/焦点，密码框可能弹到无关窗口并抢焦点。
  evidence: 需要窗口可见性 API；已先按发起窗口→host 窗口→最近窗口的顺序收敛，普通单窗口场景不触发。
- source_plan: `/ws/tty7/_bmad-output/initiative-terminal-blocks/plan-ssh-connect-ui-freeze.md`
  summary: pane 级 auth sheet 现在始终 modal，但不标注来自哪个 tab，后台 tab 的提示会盖住当前窗口。
  evidence: 属于 UX 增项（来源 tab 文案 + 跳转），本轮只保证可见可答。
- source_plan: `/ws/tty7/_bmad-output/initiative-terminal-blocks/plan-ssh-connect-ui-freeze.md`
  summary: ssh banner 在 session 内累积（`clear()` 不清 `banners`），旧连接的安全提示会一直留在窗口里。
  evidence: 需要产品决定 banner 作用域/自动消失；当前每条都可单独 Dismiss。
- source_plan: `/ws/tty7/_bmad-output/initiative-terminal-blocks/plan-ssh-connect-ui-freeze.md`
  summary: 安装进行中时 strip/switcher 展示进度条，本窗口的 Cancel 入口被隐藏，慢安装无法取消。
  evidence: 需要改安装进度渲染分支并新增文案；取消逻辑本身已能清理进度与同意。
- source_plan: `/ws/tty7/_bmad-output/initiative-terminal-blocks/plan-ssh-connect-ui-freeze.md`
  summary: 已有 sheet 时新 prompt 仍走 GiveBack→parked；单窗口用户若现有 sheet 一直不答，仍要等 120s。
  evidence: 需要评估 per-host sheet 队列（同一窗口串行抬 sheet），本轮只保证无人可答时快速失败。
- source_plan: `/ws/tty7/_bmad-output/initiative-terminal-blocks/plan-ssh-connect-ui-freeze.md`
  summary: 缺 switcher Cancel band 的渲染测试与 overlay 图层顺序的点击穿透测试。
  evidence: 需要 switcher render harness 与 `simulate_mouse_down` + `debug_bounds`；本轮以 `.occlude()` 修复点击穿透，行为未锁测。
