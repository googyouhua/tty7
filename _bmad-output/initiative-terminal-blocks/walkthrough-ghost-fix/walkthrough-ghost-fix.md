# Walkthrough ghost-fix — ghost 历史补全时有时无

- target: `4ea4a0c8` — fix(ghost): demote failed history, suggest longer on exact match
- branch: `fix/ghost-demote-failed-exact-longer`
- baseline: `72a2e92a`（分支基线，以 `git log --oneline` 确认；本 review 只看 `4ea4a0c8` 单提交）
- diff: 仅 `src/terminal/view.rs`，+84 / −18
- per-block status: block 1 (Intent) accepted (checked); block 2 (Broad strokes) accepted (checked); S1（失败降级）accepted (checked); S2（等长找更长）accepted (checked); S3（测试与渲染口）accepted (checked); current: Periphery

## Intent

- status: accepted (checked) — 用户已确认通过，无修改、无发现。

起因：ghost 历史补全时有时无——用户在输入前缀时，灰字提示偶尔空白，明明历史里有更长的匹配命令。

本提交的修复意图（来自 commit message 三条）：

1. exit 非零条目降级到最后 tier 而不是隐藏（demote failed history）；
2. 等长匹配继续找更长扩展（suggest longer on exact match）；
3. 更新 + 新增 gpui ghost 测试锁定上述两条。

换句话说：旧逻辑遇到"最新匹配失败过"或"输入恰好等于某条历史"时直接给空，新逻辑把这两种"有时无"变成"有但排后 / 有更长的"。

## Broad strokes

三处，一一对应 commit message：

1. `ghost_suggestion` 核心改两点：失败降级四 tier + 等长（`<=`）跳过找更长：
   - [view.rs:5931](../../../src/terminal/view.rs#L5931) — 函数入口与守卫（空行/光标不在尾即空）
   - [view.rs:5945](../../../src/terminal/view.rs#L5945) — 等长跳过（c）
   - [view.rs:5948](../../../src/terminal/view.rs#L5948) — 失败判定（`history_meta.exit != 0`）
   - [view.rs:5973](../../../src/terminal/view.rs#L5973) — tier 落定 `elsewhere_ok / here_failed / elsewhere_failed`
2. 三个测试（1 更新 + 2 新增），共用 seed：
   - [view.rs:19744](../../../src/terminal/view.rs#L19744) — `seed_ghost_history`（未改，背景）
   - [view.rs:19827](../../../src/terminal/view.rs#L19827) — `ghost_demotes_commands_that_failed`（更新：原断言 None → 降级可见）
   - [view.rs:19857](../../../src/terminal/view.rs#L19857) — `ghost_success_anywhere_beats_failure_here`（新增）
   - [view.rs:19880](../../../src/terminal/view.rs#L19880) — `ghost_exact_match_suggests_a_longer_entry`（新增）
3. 渲染过滤点（未改，确认"有时无"是否到屏的必经口）：
   - [view.rs:8173](../../../src/terminal/view.rs#L8173) — ghost 取后缀 + 空串过滤渲染
   - 接受入口（同样未改，ghost 消费者）：[view.rs:3245](../../../src/terminal/view.rs#L3245)（→ 接受）、[view.rs:3315](../../../src/terminal/view.rs#L3315)（Ctrl+E）、[view.rs:3328](../../../src/terminal/view.rs#L3328)（Ctrl+F）

- status: accepted (checked) — 用户已确认通过，无修改、无发现。

## Slices

### S1 — 失败降级（demote, b）

- status: accepted (checked) — 用户已确认通过，无修改、无发现。
- files:
  - [view.rs:5948](../../../src/terminal/view.rs#L5948) — `failed` 判定（`history_meta.exit != 0`；`None`/0 视为成功）
  - [view.rs:5958](../../../src/terminal/view.rs#L5958) — 成功分支：同目录即返，否则记 `elsewhere_ok`
  - [view.rs:5963](../../../src/terminal/view.rs#L5963) — 失败分支：记 `here_failed` / `elsewhere_failed`
  - [view.rs:5973](../../../src/terminal/view.rs#L5973) — 落定序：`elsewhere_ok → here_failed → elsewhere_failed`
- 行为对照：旧逻辑失败即 `continue`（永不给）；新逻辑任一成功压过任一失败（目录只在成功 tier 内优先）。
- review 入口：先读 S1 四行判定 tier 划分，再对 `ghost_demotes_commands_that_failed` 的 `make t → make test` / `make l → make lint` 两断言，最后对 `ghost_success_anywhere_beats_failure_here` 确认跨目录 tier 序。
- open question：`history_meta` 缺条目（`None`）按成功处理是否符合"失败降级"本意？待用户确认。

### S2 — 等长找更长（exact-longer, c）

- status: accepted (checked) — 用户已确认通过，无修改、无发现。
- files:
  - [view.rs:5945](../../../src/terminal/view.rs#L5945) — `h.len() <= line.len()` 跳过自身找更长
  - [view.rs:5926](../../../src/terminal/view.rs#L5926) — doc（c）语义说明
- 行为对照：旧逻辑 `h.len() < line.len()`（或等价）下输入恰等于历史时给空；新逻辑跳过等长、继续找更长扩展；无更长时仍为空（truthful blank）。
- review 入口：先读 `<=` 一行，再走 `ghost_exact_match_suggests_a_longer_entry` 两断言（`git status → git status --short` / 最长时 None），最后看渲染点空串过滤确认等长本身不留残影。
- open question：恰等于最长历史时 blank 是否为期望（而非回退到次新短串）？待用户确认。

### S3 — 测试与渲染口（tests + render filter）

- status: accepted (checked) — 用户已确认通过，无修改、无发现。
- files:
  - [view.rs:19744](../../../src/terminal/view.rs#L19744) — `seed_ghost_history`（entries 最旧在前 + `rerank_history`）
  - [view.rs:19827](../../../src/terminal/view.rs#L19827) — 更新测试（失败降级可见）
  - [view.rs:19857](../../../src/terminal/view.rs#L19857) — 新增测试（成功压失败，跨目录）
  - [view.rs:19880](../../../src/terminal/view.rs#L19880) — 新增测试（等长找更长 + 最长为空）
  - [view.rs:8173](../../../src/terminal/view.rs#L8173) — 渲染过滤（`.filter(|r| !r.is_empty())`，未改）
- review 入口：先确认 seed 语义（`exit: None` 在 S3 测试里按成功计），再逐个跑三测试断言，最后确认渲染口与接受入口（→/Ctrl+E/Ctrl+F）无改、行为仅随 `ghost_suggestion` 变。
- 待跑：`cargo test` 相关 ghost 组（走读时未执行，留待用户确认是否需要实跑记录）。

## Periphery

- status: current — 本块为当前评审块，待用户确认。

明确不在本次 review 范围，看到也不扩：

- P1 — frecency/`history_counts`/`rerank_history` 本体：ghost 仍是纯 recency（doc 重申），Ctrl+R 侧排名不动；本提交只读它们。
- P2 — `↑` 回忆一致性测试 `ghost_names_what_up_arrow_recalls`（未改）：只作背景，不重审。
- P3 — 接受键（→/Ctrl+E/Ctrl+F）与多行/selection/marked 渲染守卫：未改，仅作消费者确认。
- P4 — CI/单测门相关提交（`72a2e92a` 等基线侧改动）：不在 `4ea4a0c8` 内，不审。
