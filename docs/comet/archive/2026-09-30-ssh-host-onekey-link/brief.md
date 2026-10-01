# Outcome

SSH 面板新增 host（及编辑已有 host）时，可关联一条 OneKey 条目的用户名和密码；连接该 host 时自动使用关联凭证鉴权，无需再弹密码框。

# Scope

- SSH host 编辑器（新增 + 编辑）增加 OneKey 关联选择：下拉列出所有 OneKey 条目（title + username），可选“不关联”解除。
- Host 以 live 引用方式存储 `onekey_entry_id`；OneKey 条目改名、改用户名、改密码后，下次连接自动跟随，无需改 host。
- 连接时（`build_spec_inner` 及经由它的全部调用方：直连、重连、jump、remote workspace）用关联条目的 username/password 覆盖本次鉴权，行为与现有 `spec.password` 一致（含全密码 keyboard-interactive）。
- 关联条目被删除（或 id 对不上）时：回退到 host 自身凭证连接，host 行/详情给出链接失效提示，不崩溃、不静默用错密码。
- `onekey_entry_id: Option<String>` 随 `servers.json` 持久化，老文件缺字段即 `None`。

# Non-goals

- Key passphrase（密钥口令）的关联——只做 username/password。
- SFTP 面板、端口转发的凭证复用。
- 改动现有 keychain 密码读写链路；未关联的 host 行为零变化。
- PTY 内密码提示的自动检测（手动 OneKey picker 已覆盖）。

# Acceptance examples

- A1: SSH 新增/编辑 host 表单有 OneKey 关联下拉；选定条目保存后，host 行与详情页显示已关联的条目 title。
- A2: 连接已关联 host（password/auth-auto）时直接用条目用户名密码鉴权成功，全程不弹密码框。
- A3: 去 OneKey 里改该条目的密码（或改名），不碰 host，再次连接用新密码成功（live 引用）。
- A4: 删除被关联的 OneKey 条目后，连接回退到 host 自身凭证；host 行显示链接失效提示；无崩溃。
- A5: 解除关联后恢复纯 host 凭证行为，与从未关联一致。

# Constraints and invariants

- Secrets 永不进日志、`capture` 输出与 verification 报告；`Debug` 延续红线（OneKeyEntry 已脱敏；SshProfile 打印不得带出关联密码明文）。
- 未关联 host 的连接链路逐字节不变；关联只在 `build_spec_inner` 的 password/user 解析处覆盖。
- 空 username 的条目：username 回退用 host 自己的 user，只覆盖 password。
- 空 password 的条目：等同未关联密码，走原 keychain/ prompting 链路。

# Decisions

- D1 Workspace: 新 worktree `.worktrees/ssh-host-onekey-link`，分支 `comet/ssh-host-onekey-link`，base 为 `comet/shell-onekey-autofill`（用户指定）。
- D2 关联语义：live 引用（存 entry id），非快照复制。
- D3 关联内容：username + password（用户原话明确两者）。
- D4 连接行为：自动使用关联凭证鉴权，不预填、不二次确认。
- D5 悬空处理：回退 host 自身凭证 + 行内失效提示。

# Open questions

None. 2026-09-30 user confirmed workspace, live-reference, auto-use, fallback+warn; username+password scope from request text. Ready to enter Build after explicit confirmation.

# Verification expectations

- `cargo test` 相关单测：link 解析/回退单测、`servers.json` 含新字段 round-trip、host 编辑器表单单测。
- GUI 手工：新增 host→关联→连接免密；改 OneKey 密码→重连成功；删条目→回退+提示；解关联→恢复。
- 无 secret 落盘到日志；未关联 host 回归不受影响。
