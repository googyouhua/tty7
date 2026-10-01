# 任务

- [x] 1. `shell_integration.rs`：`bootstrap_command(shell, path, cols, rows)` 增加无 pty 兜底分支（zsh/bash/fish 三脚本结尾条件分发 + util-linux `script` 探测 + TERM/COLORTERM 导出 + stty 初始尺寸）
- [x] 2. `ssh/mod.rs`：把会话 `size` 传入 `remote_bootstrap` → `bootstrap_command`
- [x] 3. 单测：更新既有 bootstrap 断言（新签名/新分支）；新增"管道 stdin → 内层有 pty + 集成标记齐全"回归测试
- [x] 4. `cargo fmt` + 相关测试 + 构建通过，逐任务提交（注：本机无 github 访问，构建验证在 tty7-core 独立 workspace 完成——1524 测试全过；含 gpui 的完整 `--locked` 构建由 CI/用户机器执行）
- [x] 5. 实弹验证（2026-09-29 完成）：修复前 `-tt` exec 实测 NO_TTY/TERM=dumb（根因留档）；修复后 bootstrap 14826B 经 koko 单次 exec 投放——内层 pty `rows 30 cols 120`（stty 生效）、`INNER_TERM=xterm-256color`、输入回显正常、`133;A/B/C/D`+`OSC 7`+`133;V` 全套回传
- [x] 6. 独立代码审查 + 修复：CRITICAL（`&&`/`||` 同优先级导致无 script 机器 pane 死亡，De Morgan 重写）、IMPORTANT（`script -c` 中间层实为 `$SHELL`，fish 改两层 quote）、SUGGESTION（探测对齐 exec 旗标 `-qfec`+`-e` 退出码穿透、移除多余 `__tty7_d` 导出）——新增 2 个回归测试钉死
