# remote-instance-reuse — complete target specification

## 1. 实例推导（本地）

- 建链时读取本进程 `config_dir_path()`，经 `tty7_core::core::instance::memory_name_for` 得名。
- 得 `DEFAULT_SENTINEL`（默认实例）→ 不附加任何参数，行为与今天逐字一致。
- 具名实例（`memory_name_for` 只产出 `validate_name` 合法名）→ 远端目标实例即同名。

## 2. 命令与解析（远端）

- 远端 server 基命令（默认 `tty7-server --stdio` 及其安装路径变体）追加 `--config-dir <dir>`，其中 `<dir> = <remote-home>/.config/tty7-<name>`；`<remote-home>` 取环境探针的 `HOME`，与 `remote_control_socket` 同源；目录做 shell 引用。
- `RouteHeader.server_command` 显式给出时优先、原样执行，不拼接。
- socket 推导：用实例目录覆盖探针 `config_dir` 后走既有 `remote_control_socket` 规则（含过长 fallback）；StreamLocal 快路与 SessionExec 回退一致落在该实例。
- Windows 远端（固定 SessionExec）：同样拼接，server 侧 `--config-dir` 跨平台生效。

## 3. 连接复用隔离

- `SshConnection.remote_entry` memo 键加入实例维度；同 `user@host:port` 下 gyh/mca 双链各走各的 entry，先后顺序不影响归属。
- SSH 传输连接本身仍可复用（各 link 独立 session channel）。

## 4. 远端缺实例

- 远端同名目录不存在 → server 侧自动创建空实例并连上（与本地 picker 首次进入同语义），日志记录；不报错、不回退默认实例。

## 5. 不变项

- 默认实例链路行为不变；daemon/control 协议不变；pane/switcher 展示逻辑不变；`--as`/记忆文件格式不变；WSL/local-stdio/快连直连不带实例。
