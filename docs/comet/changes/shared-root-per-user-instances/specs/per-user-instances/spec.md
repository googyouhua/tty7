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

## 3. 文档

- `docs/remote/shared-root.mdx`：双人并行开实例、互不可见检查清单（A1–A4）、GUI 配置、停服/清理、已知约束（同 UID 下 root 可旁路、端口/CPU 共享）。
- 明确禁止：`pane close --orphans` 或 `server stop` 不带 `--as` 在共享机器上使用（裸调用作用于默认实例）。
- CLI 参考（`docs/cli/reference.mdx` Global flags）：`--as <name>` 一行。

## 4. 安全与约束

- socket 语义不变：`0600`，父目录 `0700`，`umask 077`；实例目录 `700`。
- 不改 daemon/control 线协议；`--as` 只是在 CLI 进程内把名字解析为 config dir 后走既有链路。
- 同为 root 时本方案为诚实隔离：恶意 root 仍可读对方文件/进程，文档必须声明。
