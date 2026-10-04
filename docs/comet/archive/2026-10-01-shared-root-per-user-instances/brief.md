# Outcome

服务器多人共用同一系统用户（均为 root 登录）时，提供一种可配置、可试用的隔离方法：每个用户只能 attach / 看到 / 操作自己创建的 session，看不到别人的 workspace 与 pane。

# Scope

- 背景：默认 `TTY7_CONFIG_DIR` 为 `$HOME/.config/tty7`，同为 root 登录时所有人共享同一个 server 与 `control.sock`，`ls/send/capture/pane close` 可操作任意 pane（仅靠“别碰别人的 pane”约定，无强制隔离）。
- 本次试用独立实例方案：每人独立 `TTY7_CONFIG_DIR` + 独立 server/socket/state，实现 pane 级控制与 workspace 级可见性两层隔离。
- 覆盖 CLI（内建 `--as <name>` 参数）与 GUI（启动实例选择器 + `--config-dir` 直达）两条链路。
- 交付物：CLI 内建 `--as` 全局参数 + GUI 启动实例选择器 + 文档，不新增 wrapper 脚本，不改 daemon/control 协议，不改系统配置（无 `.desktop`、无系统目录安装）。

# Non-goals

- 不做同一 server 内的按创建者鉴权（需改协议，工程量大，已排除）。
- 不提供内核级硬安全：同为 root 的用户理论上仍可读对方目录/进程，本方案是诚实隔离而非防 root 攻击。
- 不做 CPU/内存/端口配额隔离。

# Acceptance examples

- A1：`tty7 --as alice ls` 看不到 `tty7 --as bob` 的 workspace；反之亦然。
- A2：`tty7 --as bob capture %1`（alice 的 pane）被拒，寻址不到对方 pane id。
- A3：GUI 以 `tty7-app --config-dir ~/.config/tty7-alice` 启动后只 attach 自己的实例，切换器中无别人的 workspace。
- A4：按文档可复现：两用户并行各起实例、互不可见；停掉 A 的 server 不影响 B。
- A5：GUI 不带参数启动必弹实例选择器（默认实例 + 已有 `tty7-*` + 手输新名），上次选择预选中；确认后进入对应实例并记住选择；带 `--config-dir`/`TTY7_CONFIG_DIR` 启动跳过选择器。

# Constraints and invariants

- 不改 `daemon/control` 线协议；复用现有 `--config-dir` / `TTY7_CONFIG_DIR` 解析优先级（flag > env > portable > 默认）。
- socket 权限语义不变（`0600`，父目录 `0700`，`umask 077`）。
- 同 UID 下目录权限靠自觉 `chmod 700`；root 仍可旁路，必须作为已知约束写进文档。
- 方法可逆：删掉独立目录即回到共享默认实例。

# Decisions

- 方案形态：每人独立实例（`TTY7_CONFIG_DIR` + 独立 server），而非同服鉴权或纯口头约束。
- 隔离粒度：pane 级控制 + workspace 级可见性，两层都要。
- 身份区分：CLI 侧靠环境变量或参数；GUI 侧靠用户客户端配置后带过去（用户已确认无更好办法，先按此试）。
- 工作区：独立 worktree `shared-root-per-user-instances`，不碰主目录未提交修改。
- 映射形式：CLI 内建全局 `--as <name>` 参数（`tty7 --as alice status`），不新增 wrapper 脚本；`scripts/tty7-as` 在 Build 中删除，文档改写为 `--as` 用法。
- GUI 启动实例选择器：`tty7-app` 不带 `--config-dir`（且无 `TTY7_CONFIG_DIR`）启动时，先弹实例选择器再进主窗口；界面只录 `username`——下拉选已有实例名、可手输新名（同 `--as` 名字规则），路径由程序按统一根目录自动拼 `${TTY7_AS_ROOT:-$HOME/.config}/tty7-<username>`，用户不碰路径。
- 记忆位置：默认目录下固定文件（如 `~/.config/tty7/.last-instance`，存名字/`default`），不跟实例走；记忆指向的目录被删则回退选择器；显式参数启动跳过选择器但仍更新记忆。
- `--as` 语义：名字规则 `[a-z0-9-]+`（非法直接报错退出 `2`）；映射到 `${TTY7_AS_ROOT:-$HOME/.config}/tty7-<name>`，首次使用建目录并 `chmod 700`；CLI 内优先级 `--as` > `TTY7_CONFIG_DIR`（`--config-dir` 只存在于 `tty7-app`/`tty7-server`）。
- 实例目录：`~/.config/tty7-<name>`，与默认同级分散存放。
- 旧 Repair 回合结论（wrapper 的两处修复）作废，随 wrapper 删除而失效；等价语义由 `--as` 实现继承（`TTY7_AS_ROOT` 免 `$HOME`、缺 tty7 不建目录不适用、仅 leaf `700`）。

# Open questions

无。A5 细化（界面只录 username、下拉选已有/手输新名、路径自动拼）已写入 Decisions，待用户最终确认后进入 Build。

# Verification expectations

- 按 A1–A4 在同一台机器双用户并行手工验证（`tty7 ls/ws tree/pane ls --all/capture` 互不可见 + GUI 切换器检查）。
- `cargo test` 相关单测（如 config dir 解析、socket 绑定）回归通过。
- Live 证据（2026-09-29，真二进制 `target/debug/tty7` + `tty7-server`，`TTY7_AS_ROOT=/tmp/opencode/live-test`）：alice/bob 各起 server（socket 分离），alice `new` 建 workspace+pane %1 后 bob `ls` 为空、`pane ls --all` 无 pane、`capture %1` 被拒（`no such pane 1`，exit 1）；停 alice 后 bob 仍存活且不受影响。GUI 切换器目检仍需用户在桌面环境按文档手动确认。
