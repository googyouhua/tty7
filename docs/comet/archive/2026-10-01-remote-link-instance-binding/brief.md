# Outcome

远端链自动复用 GUI 打开时的 username：本地以 gyh 进入，SSH 到任何机器都只看到该机器上 `tty7-gyh` 实例的内容，看不到别的 username（含默认实例）的 workspace。默认实例打开则行为与今天完全一致。

# Scope

- 实例推导：建链时取本进程 config dir → `instance::memory_name_for` 得名；`default` 哨兵表示默认实例，不拼任何参数（现状）。
- 命令拼接：具名实例时，远端 server 基命令追加 `--config-dir <remote-home>/.config/tty7-<name>`；`<remote-home>` 取 probe 的 `HOME`（与现有 `remote_control_socket` 同源）。显式自定义 server 命令（`RouteHeader.server_command`）优先、原样不动。
- 解析同步：probe 的 socket 推导改用实例目录（等价于用实例目录覆盖 `RemoteEnv.config_dir` 后走现有规则），StreamLocal 快路与 SessionExec 回退都落在同一实例。
- Entry 隔离：`SshConnection` 上的 `remote_entry` memo 按 instance 区分；同机 gyh+mca 双链复用一条 ssh 连接不串 entry。
- 远端同名不存在：server 侧自动建空实例（与本地 picker 首次进入同语义），日志留一条；不报错、不回退到默认实例（回退即泄漏）。

# Non-goals

- WSL / local-stdio / 快连直连：不带实例（local-stdio 继承本机进程环境，本就同实例；WSL 单用户为主）。
- 不动 daemon/control 协议；不动 pane/switcher 展示逻辑。
- 远端 `TTY7_AS_ROOT` 自定义根：暂不支持，只认默认根（约束见下）。
- 不提供逐 host 覆盖字段（方案一：无例外口；将来需要另起 change）。

# Acceptance examples

- A1：本机 gyh 实例经远端链到 box（box 上有 `tty7-gyh` 与 `tty7-mca`、默认实例各有 workspace）→ 切换器只出现 gyh 实例的内容，不出现 mca 与默认实例的行。
- A2：默认实例打开的 GUI 建链 → 行为与今天一致（默认实例内容）。
- A3：同机 gyh+mca 双链复用一条 ssh 连接 → 各自 entry 独立，先后顺序不影响归属（单测覆盖）。
- A4：远端无同名实例 → 自动建空实例并连上，不报错、不落到默认实例；非法本地名不可能出现（`memory_name_for` 只产出合法名或哨兵）。

# Constraints and invariants

- 后端：`tty7-server --stdio` 原生支持 `--config-dir`（`crates/tty7-server/src/main.rs`），拼接命令须正确 shell 引用实例目录。
- 远端实例根固定为 `<remote-home>/.config`；远端若自定 `TTY7_AS_ROOT`，本 change 不跟随（约束，后续需要另起）。
- `cargo fmt --check` 通过；现有 `remote_link`/`router`/`ssh` 单测全过。
- 本 change 分支另含仓库卫生提交：删除 main 上遗留的两个无效变更残留目录（`docs/comet/changes/shared-root-per-user-instances|windows-picker-relaunch`，缺 state 导致任何新 change 建不出来；原件已备份 `/tmp/opencode/stale-comet-backup`）。

# Decisions

- 方案一：纯复用打开时的 username，无 UI、无配置（用户已选）；host 覆盖字段不做。
- 工作区：独立 worktree `remote-link-instance-binding`，基于 main 尖端（含 PR #5/#6）。
- 未填/默认实例 = 现状行为，不做强制下拉（兼容所有现有链路）。
- 残留目录处理：用户明确选择直接删除（已备份）。

# Open questions

无。用户已确认 Outcome/Scope/A1–A4/Constraints/Non-goals，进入 Build。

# Verification expectations

- `cargo fmt --check`、新增与现有 `remote_link`/`router`/`ssh` 单测通过。
- 真机目检（需用户在笔记本上操作）：A1（gyh 链 box 只见 gyh）、A2（默认链行为不变）；A3/A4 走单测。
