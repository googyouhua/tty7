# 验证报告：fix-jumpserver-shell-integration

日期：2026-09-29 · 分支 `fix/jumpserver-shell-integration` · commits `f402eae0` + `39263cdd` · verify_mode：light（覆盖记录：hotfix 预设、实现 2 文件、无 delta spec；规模评估机械判 full 仅因任务计数含 2 个验证/工作流活动）

## 六项检查（lightweight）

| # | 检查 | 结果 | 证据 |
|---|---|---|---|
| 1 | tasks.md 全部完成 | PASS | 未完成任务数 0（6/6 `[x]`） |
| 2 | 变更文件与任务一致 | PASS | `git diff main...HEAD --stat`：仅 `shell_integration.rs`（+313/-47）与 `ssh/mod.rs`（+27/-7），对应任务 1/2/3/6 |
| 3 | 构建通过 | PASS（有备注） | 本机无 github 访问，完整 `--locked` 构建（含 gpui）不可行；验证在 tty7-core 独立 workspace（vendored russh 补齐 `MethodKind::GssapiWithMic`）完成，编译零错误。完整构建由 CI/用户机器执行 |
| 4 | 相关测试通过 | PASS | `cargo test -p tty7-core --lib -- --test-threads=16`：**1525 passed / 0 failed**（含 2 个新增回归测试）。备注：256 核本机默认并发下曾出现随机资源耗尽型偶发（`WouldBlock` 线程耗尽、失败模块逐次漂移），16 并发全绿，真实 CI（2-4 核）不受影响 |
| 5 | 无安全隐患 | PASS | diff 无密钥/新增 unsafe；脚本注入审查通过（对抗 term/路径值全部原样穿透，rows/cols 为 u16 数字） |
| 6 | 轻量代码审查 | PASS（修复后） | 首轮独立审查发现 1 CRITICAL + 1 IMPORTANT + 3 SUGGESTION，全部修复并补回归测试（见下） |

## 审查发现与处置（verify-fail 一轮后修复）

- **CRITICAL**：POSIX 守卫把探测写成 `! command -v script && script -qc …`，`&&`/`||` 同优先级左结合 → 无 util-linux `script` 的机器（macOS/BSD/精简镜像）落入 else 分支 `exec` 不存在的 `script`，**pane 直接死亡**。修复：`||` 链逐项拆开；`[ -t 0 ]` 短路先于探测（健康主机零探测开销）。回归测试：失败 `script` 桩 + 真 pty → 必须拿到完整 prompt 周期。
- **IMPORTANT**：`script -c` 的中间解释器是 `$SHELL`（登录 shell）而非 `/bin/sh`；fish 分支改为两层 fish-quote（payload 与 else 分支同一 fish 行）。
- **SUGGESTION（采纳）**：探测旗标与 exec 完全一致（`-qfec`，`-e` 令退出码穿透、busybox 探测失败即安全退回）；移除多余 `__tty7_d` 导出（payload 改读 `$TTY7_RM_DIR`）。

## 实弹验证（经 jumpserver-v3.yunsilicon.com:2222，一次性口令 loopback）

- 修复前（`ssh -tt` 强制 pty-req + exec）：`NO_TTY` / `TERM=dumb` / `not a tty` —— 根因实锤留档
- 修复后（14826B bootstrap 单次 exec 投放）：内层 pty `rows 30 columns 120`（stty 生效）、`INNER_TERM=xterm-256color`、输入回显恢复、`133;A/B/C/D` + `OSC 7` + `133;V` 全套回传 —— 与用户症状（阶梯/输入隐身/TERM=dumb）逐项对应消除

## 黑盒回归（本机，模拟 koko 无 pty）

- 管道 stdin 投喂 bootstrap → `script` 兜底 → 完整 `133;A/B/C/D` 周期（修复前该场景无 C/D 且无回显）
- 失败 `script` 桩 + 真 pty → 完整 prompt 周期（钉死 CRITICAL：无 script 机器不死 pane）

## 确认项

- [x] 全部测试通过（1525/0，16 并发）
- [x] 无硬编码密钥、无安全问题
- [x] 已知限制已记录：koko 兜底路径的会话中途 resize 不传播（无 winch 通道，初始尺寸已修）；fish 2.x（2016 年停更）与 busybox `script` 保持原行为
