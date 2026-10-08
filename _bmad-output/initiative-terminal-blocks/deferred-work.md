## Deferred from: code review (2026-10-08) of b06ab5c..HEAD (terminal-blocks 1.2-1.5)

- source_plan: `_bmad-output/initiative-terminal-blocks/epic-terminal-blocks/story-pane-plan.md`
  summary: handoff 照抄 record.next_block_id，未按 max+1 重算（外来 manifest 会 alias）。
  evidence: 同版本 blob 自洽故未复现；settles 需外来-manifest 证据或跨版本升级用例。
- source_plan: `_bmad-output/initiative-terminal-blocks/epic-terminal-blocks/story-pane-plan.md`
  summary: 新旧二进制混跑时 kind 64/65 在旧端整连接 break，需版本门控或降级策略。
  evidence: `break 'conn` 已确认；version handshake 存在但不管 pane 协议新 kind；属协议版本管辖，非本 epic 范围。
- source_plan: `_bmad-output/initiative-terminal-blocks/epic-terminal-blocks/story-copy-plan.md`
  summary: 三复制 action 的剪贴板精确串缺回归单测（wiring 已人工验对：command→block_command_text、output→block_output_text、both→block_text）。
  evidence: repo 无 view harness（296 view 测试全纯单元）；待 view TestApp harness 出现后补。
- source_plan: `_bmad-output/initiative-terminal-blocks/epic-terminal-blocks/story-ctrl-c-plan.md`
  summary: gutter marker (seq,folded,failed) 映射缺单测（wiring 已人工验对：is_failed() 直通第三元组）。
  evidence: 同上，待 view harness。
