# per-user-instances — complete target specification

## 1. 实例模型

- 每个自然人 exactly 一个 tty7 实例：独立 `TTY7_CONFIG_DIR`（`~/.config/tty7-<name>`）、独立 server 进程、独立 `control.sock`、独立 workspace/pane/scrollback/history。
- 默认实例（`~/.config/tty7`）保留，可作为公共/无人值守实例或停用；不强制迁移。
- 实例间默认互不可见：`tty7 ls/ws tree/pane ls/agents/capture/send` 只作用于解析到的 socket。

## 2. `tty7 --as <name>`（CLI 全局参数）

- 用法：`tty7 --as alice status`、`tty7 --as alice new ~/code/api`、`tty7 --as alice run -- cargo test`。`--as` 接受在子命令前后（全局参数）。
- `<name>` 规则：非空、`[a-z0-9-]+` 且不以 `-` 开头；非法直接报错退出 `2`，不启动任何 server、不建目录。
- 映射到 `${TTY7_AS_ROOT:-$HOME/.config}/tty7-<name>`；`TTY7_AS_ROOT` 已设置时不要求 `$HOME`。
- 首次使用自动建目录并 `chmod 700`（仅 leaf 目录，父目录不动）。
- 优先级（CLI 内）：`--as` > `TTY7_CONFIG_DIR` > portable > 默认。`--config-dir` 只存在于 `tty7-app` / `tty7-server` 二进制上，不在 CLI；GUI 启动时 `--config-dir`（flag）优先于一切。
- `server start/stop/status/doctor` 等全部 verb 透传，只影响该用户实例。
- 无 wrapper 脚本：`scripts/tty7-as` 不存在。
- GUI：`tty7-app --config-dir ~/.config/tty7-alice`（GUI 保持 `--config-dir`，不加 `--as`；注意二进制是 `tty7-app`，`tty7` CLI 没有该 flag）；远端链路按用户分别建连接，客户端配置决定 attach 到哪个实例。

## 3. GUI 启动实例选择器

- 触发：`tty7-app` 启动时既无 `--config-dir` 也无 `TTY7_CONFIG_DIR`，先弹实例选择器（gpui 首窗口），用户确认后再按既有流程进主窗口。无参数启动必弹（已有窗口在跑也不例外）；带路径启动优先转发给已在跑的窗口，无窗口时同样先过选择器、再在所选实例中打开该路径。
- 列表：默认实例 + 已有 `tty7-*` 实例（扫 `${TTY7_AS_ROOT:-$HOME/.config}`）+ 手输新名（同 `--as` 名字规则）；界面只录 `username`（下拉选已有、可手输），路径自动拼，用户不碰路径。
- 上次选择预选中；常驻等待，无倒计时、无自动进入。
- 确认后：记住选择（见记忆），按选中实例的 config dir 继续启动（等价于带 `--config-dir` 启动）。
- 记忆：默认目录下固定文件（如 `~/.config/tty7/.last-instance`，存名字/`default`）；记忆失效（目录被删）回退选择器；显式参数启动跳过选择器但仍更新记忆。

## 4. 文档

- `docs/remote/shared-root.mdx`：双人并行开实例、互不可见检查清单（A1–A5）、GUI 选择器 + `--config-dir` 配置、停服/清理、已知约束（同 UID 下 root 可旁路、端口/CPU 共享）。
- 明确禁止：`pane close --orphans` 或 `server stop` 不带 `--as` 在共享机器上使用（裸调用作用于默认实例）。
- CLI 参考（`docs/cli/reference.mdx` Global flags）：`--as <name>` 一行。

## 5. 安全与约束

- socket 语义不变：`0600`，父目录 `0700`，`umask 077`；实例目录 `700`。
- 不改 daemon/control 线协议；`--as` 只是在 CLI 进程内把名字解析为 config dir 后走既有链路。
- 同为 root 时本方案为诚实隔离：恶意 root 仍可读对方文件/进程，文档必须声明。
