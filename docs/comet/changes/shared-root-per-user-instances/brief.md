# Outcome

服务器多人共用同一系统用户（均为 root 登录）时，提供一种可配置、可试用的隔离方法：每个用户只能 attach / 看到 / 操作自己创建的 session，看不到别人的 workspace 与 pane。

# Scope

- 背景：默认 `TTY7_CONFIG_DIR` 为 `$HOME/.config/tty7`，同为 root 登录时所有人共享同一个 server 与 `control.sock`，`ls/send/capture/pane close` 可操作任意 pane（仅靠“别碰别人的 pane”约定，无强制隔离）。
- 本次试用独立实例方案：每人独立 `TTY7_CONFIG_DIR` + 独立 server/socket/state，实现 pane 级控制与 workspace 级可见性两层隔离。
- 覆盖 CLI（环境变量/参数）与 GUI（客户端配置透传）两条链路。
- 交付物为可配置方法：wrapper/约定 + 文档，优先不改 daemon/control 协议。

# Non-goals

- 不做同一 server 内的按创建者鉴权（需改协议，工程量大，已排除）。
- 不提供内核级硬安全：同为 root 的用户理论上仍可读对方目录/进程，本方案是诚实隔离而非防 root 攻击。
- 不做 CPU/内存/端口配额隔离。

# Acceptance examples

- A1：用户 A `export TTY7_CONFIG_DIR=~/.config/tty7-a` 后 `tty7 ls` 看不到用户 B 的 workspace；反之亦然。
- A2：用户 A 无法 `capture/send` 用户 B 的 pane（不同 socket，寻址不到对方 pane id）。
- A3：GUI 按文档配置后只 attach 自己的实例，切换器中无别人的 workspace。
- A4：按文档可复现：两用户并行各起实例、互不可见；停掉 A 的 server 不影响 B。

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
- 映射形式：先试 wrapper 脚本（`tty7-as <name>` → 独立 `TTY7_CONFIG_DIR`），验证有效再考虑内建 `--as` 参数。
- 实例目录：`~/.config/tty7-<name>`，与默认同级分散存放。
- Repair 回合：`TTY7_AS_ROOT` 已设置时不再要求 `$HOME`；`tty7` 缺失时先报错、不建目录；仅 leaf 目录 `700`、父目录不动（已写入脚本注释与文档约束）。

# Open questions

无。2026-09-29 用户已确认上述目标/范围/决策/验收/非目标，进入 Build。

# Verification expectations

- 按 A1–A4 在同一台机器双用户并行手工验证（`tty7 ls/ws tree/pane ls --all/capture` 互不可见 + GUI 切换器检查）。
- `cargo test` 相关单测（如 config dir 解析、socket 绑定）回归通过。
- Live 证据（2026-09-29，真二进制 `target/debug/tty7` + `tty7-server`，`TTY7_AS_ROOT=/tmp/opencode/live-test`）：alice/bob 各起 server（socket 分离），alice `new` 建 workspace+pane %1 后 bob `ls` 为空、`pane ls --all` 无 pane、`capture %1` 被拒（`no such pane 1`，exit 1）；停 alice 后 bob 仍存活且不受影响。GUI 切换器目检仍需用户在桌面环境按文档手动确认。
