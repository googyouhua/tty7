# Outcome
嵌套 shell（`sudo -s` / `su` / 内层 bash）切换目录后，Info/Review/Changes/GitHub 的 cwd 跟随更新，与外层 shell 一致；冻住可接受时有开关/手动手段。

# Scope
- 现状（已验证）：tty7 靠启动外层 shell 时注入 `--rcfile /tmp/tty7-bashrc-*/bashrc` 上报 cwd（daemon → `DaemonMsg::Cwd` → GUI `foreground_cwd` → `scm_pane_target`）。内层 shell（`sudo -s` 后的 root bash）不走 rcfile，只发标题不发 cwd，表现为左标题跟、右 cwd 全冻住；Windows 因无此嵌套场景而正常。
- 修方向（二选一，agent 定）：
  - A（推荐）：Linux/macOS 上 daemon 对本地 pane 以“前台进程的 cwd”为兜底（Linux `/proc/<pid>/cwd`，macOS `PROC_PIDVNODEPATHINFO`，都是 pgid→组内最深进程解析），shell 上报优先、proc 兜底。
  - B：环境变量穿透（`PROMPT_COMMAND` 继承），但 sudo 默认 `env_reset` 会剥离，需验证。
- 不动点：`DaemonMsg::Cwd` 协议、`foreground_cwd` 调用方（Info/Review/Changes/GitHub 共用，修好一处全好）。
- 远端 su/sudo（A5）：in-band ssh 进远端后再 `su`/sudo 到 root，远端内层 shell 零上报且本机 `/proc` 看不见远端进程，本地 proc 兜底无效。唯一跨过 su 边界传回来的信息是标题（Debian/Ubuntu 系 root 默认 PS1 `\u@\h: \w` 带全路径）。daemon 对远端 pane 加标题→路径严格解析兜底：仅 `user@host: /绝对路径` 格式，解析不出宁可冻住也不报错目录；仅无新鲜 OSC7 时采用，新鲜上报立刻赢回。本地路径不动。
- 手动覆盖（A6）：Info 行 cwd 目前只有复制/reveal（Linux 点 reveal 弹文件管理器选不了，Windows 没反应），无修改入口。加 per-pane 手动覆盖：点击 cwd 值变行内文本输入，回车提交、Esc 取消、清空恢复自动跟随；`effective_cwd()` 优先读覆盖值；Win/Linux 同一套纯文本输入，不用系统路径弹窗。

# Non-goals
- Windows 的 proc 兜底不做（ConPTY 无前台进程组概念，`foreground_cwd` 本来就返回 None）；不改 Review Tab 本身（codereview-mr-review 已验证跟随逻辑无 bug）。
- 远端标题兜底之外不做远端执行（不下发远端初始化、不轮询远端 /proc、不包装 sudo/su）；RHEL 系默认 root PS1 不设标题，那种机器上远端 su 后依然冻住（物理上限，写成已知限制）。
- 手动覆盖不做持久化（换 pane/重连后以自动跟随为准，覆盖值只活在当前 pane 会话）。

# Acceptance examples
- A1: 本地 pane 外层 bash `cd` 后 cwd 跟随（回归现有行为）。
- A2: `sudo -s` 后内层 root bash `cd /tmp`，Info/Review 显示 `/tmp`（误差 ≤ 1 次 prompt/轮询周期）。
- A3: 退出内层 shell 回到外层，cwd 恢复跟随外层目录。
- A4: 现有单测全过（daemon pane cwd 相关：`pane.rs:6394/6478/6582` 附近用例）。
- A5: 远端 pane（ssh 进服务器后 `su` 到 root）`cd /tmp`，Info 显示 `/tmp`（Debian/Ubuntu 系标题；无标题格式的机器保持冻住但不报错）。
- A6: Info 行 cwd 点击变文本输入，输入绝对路径回车后各面板跟随手动值；清空后恢复自动跟随；Win/Linux 一致，无系统弹窗。
- A7: Info 面板 cwd 下方独立“跟随”开关行，per-pane 默认开（=off 才冻住）；开关只门控标题兜底的本地部分（A5 远端常开不受影响）；手动 pin 追踪值回到同目录时自动解 pin、显示不动；会话级，不持久化。
- A8: bash rcfile 追加自包含 OSC7 上报尾巴并 export PROMPT_COMMAND：`su`（不带 `-`）与透传型 `sudo -s` 进来的内层 bash 自动精确上报；外层行为不变；`su -`/`sudo -i` 与 zsh 内层仍走标题/手动。

# Constraints and invariants
- shell 上报优先，proc 只是兜底（上报新鲜时不用 proc，避免与 agent/编辑器等前台进程的 cwd 打架）。
- 仅本地 pane 生效；远端/WSL/SSH pane 保持原逻辑。
- 轮询节流：沿用现有 status 刷新节奏，不新增高频 timer。

# Decisions
- 工作区：独立 worktree（`comet/nested-shell-cwd`）。
- 与 codereview-mr-review 独立成 change（后者 Verify 6/6 待 Archive，不动）。
- A5 标题格式优先级：Debian/Ubuntu 系 + 通用严格 `user@host: /绝对路径`；`~` 仅在标题用户为 root 时映射 `/root`，其余放弃；RHEL 系无标题保持冻住。
- A6 覆盖语义：per-pane 会话级，不持久化；`effective_cwd()` 最优先；清空=删除覆盖。
- A7 开关语义：per-pane 会话级默认开（=off 冻住）；只门控标题兜底（本地部分），A5 远端常开不受影响；GUI→daemon 走 per-pane 标志（参考 clipboard 权限先例的新 ClientMsg）；手动 pin 在追踪值自然到达同目录时自动解除。
- A8 穿透语义：bash 首版（zsh 中转器首个 precmd 即指回用户目录，内层够不到文件；fish/pwsh/nu/sh 无通道，均走标题/手动/开关）；上报尾巴自包含（仅 builtin+变量，零函数引用），外层链冻进函数、缺失时静默跳过；`su` 透传实测成立，`sudo -s` 默认被 `env_reset` 清掉（早前 T4/T5 为转义假阳性，已勘误），透传型 sudo 构建可透，透不过自动掉到标题/手动/开关。
- macOS 同理纳入：`foreground_pid_in` 是纯表逻辑（macOS 侧 `process_table` 已有 ppid/pgid），`foreground_cwd` macOS 臂做同样解析；macOS 无真机，靠同份单测 + CI；root 不可读同样诚实降级。

# Open questions
- Q1 已决：以内层 shell 跟随为验收核心（A2），Linux `/proc` 兜底优先。
- Q2 已决（用户 2026-10-03）：冻住默认可接受，但要有开关（A7 默认开）；远端 sudo 必须跟（A5）；zsh/fish 本轮不做。

# Verification expectations
- A1–A4：单测 + 在 192.168.1.25 实机验证（sudo -s 进出，Info/Review 跟随）。
- A5–A8：单测 + 25 实机 headline（`sudo -s` 精确跟随 yours; 开关行；手动输入）。
