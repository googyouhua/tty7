---
title: '折叠后输入光标跟随输入行'
type: 'bugfix'
ticket: ''
created: '2026-10-11'
status: 'done'
baseline_revision: '41530220305bd078612b36ddfddf6f8529b116c5'
route: 'oneshot'
route_source: 'auto'
risk: 'medium'
review: 'quick'
review_source: 'pinned'
lenses_ran: ["quick"]
review_loop_iteration: 0
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** 折叠末块后输入行保住了，但输入光标还停在折叠前的位置，输入条、补全菜单、点击映射全错位。

**Approach:** 让 `cursor_cell` 走折叠映射（复用 paint 那份 frame 槽位，失配回退恒等），输入相关定位一次全对齐。

</frozen-after-approval>

## Implementation Notes

oneshot：网格光标（`element.rs:2124`）和鼠标（`mouse_grid_line`）早已走折叠映射，唯独 `view.rs:5178 cursor_cell` 还是 `line+display_offset` 恒等坐标；它的调用方（输入条 7843、补全 8092、点击 5239、scroll 5210）于是全停在折叠前的位置。修法照抄 `mouse_grid_line`：优先 frame 槽位（`matches` 校验），失配现算 `visible_map`，无折叠即恒等。col 不动；paint 光标、FrameMap 形状、锁顺序（grid→tracker）都不动。

- 坑：首版测试 history 太长，窗口 bottom-anchored、光标在底行，恒等==映射，复现不了；改短 history（map 不满顶部对齐）才暴露。红绿验证：强制恒等分支则红。
- 验证：`cursor` 57 过，`input_` 54 过（`write_sends_input_frames` 在基线同样红：碰真实 `/root/.config` 的环境问题），`block` 81 过，boundary clean，fmt 干净。

## Verification

**Commands:**
- `cargo test --bin tty7-app cursor` -- expected: 新增用例过
- `cargo test --bin tty7-app input_` -- expected: 全过无回归
- `bash .github/scripts/check-host-boundary.sh` -- expected: clean
- `cargo fmt --check` -- expected: 干净

**Manual checks (if no CLI):**
- 25 机器折叠末块后光标落在输入行上，输入条跟随，打字位置正确

## Review Triage Log

- quick lens: no findings. Caller隐式覆盖确认（输入条/补全/点击/scroll 全继承）；`write_sends_input_frames` 基线同红（环境），非回归。
