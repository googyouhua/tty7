# per-user-instances — complete target specification

## 1. 实例模型

- 每个自然人 exactly 一个 tty7 实例：独立 `TTY7_CONFIG_DIR`（`~/.config/tty7-<name>`）、独立 server 进程、独立 `control.sock`、独立 workspace/pane/scrollback/history。
- 默认实例（`~/.config/tty7`）保留，可作为公共/无人值守实例或停用；不强制迁移。
- 实例间默认互不可见：`tty7 ls/ws tree/pane ls/agents/capture/send` 只作用于 `TTY7_CONFIG_DIR` 解析到的 socket。

## 2. `tty7-as` wrapper

- 用法：`tty7-as <name> [--] <tty7 args...>`，等价于 `TTY7_CONFIG_DIR=~/.config/tty7-<name> exec tty7 <args...>`。
- `<name>` 规则：小写 `[a-z0-9-]`，非空；非法直接报错退出 `2`，不启动任何 server。
- 首次使用自动 `chmod 700` 实例目录；已存在则校验后复用。
- 透传退出码与 `--json` 输出；`tty7-as <name> server start/stop/status/doctor` 只影响该用户实例。
- GUI：`tty7 --config-dir ~/.config/tty7-<name>`；远端链路按用户分别建连接，客户端配置决定 attach 到哪个实例。

## 3. 文档

- 新增运维文档：双人并行开实例、互不可见检查清单（A1–A4）、GUI 配置、停服/清理、已知约束（同 UID 下 root 可旁路、端口/CPU 共享）。
- 明确禁止：`pane close --orphans` 跨实例使用、`server stop` 未加 `tty7-as` 前缀。

## 4. 安全与约束

- socket 语义不变：`0600`，父目录 `0700`，`umask 077`；实例目录 `700`。
- 不改 daemon/control 线协议；`--config-dir` > `TTY7_CONFIG_DIR` > portable > 默认的优先级不变。
- 同为 root 时本方案为诚实隔离：恶意 root 仍可读对方文件/进程，文档必须声明。
