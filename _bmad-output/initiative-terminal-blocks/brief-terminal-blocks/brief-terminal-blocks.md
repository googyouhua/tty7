---
title: terminal-blocks
status: complete
created: 2026-10-07
updated: 2026-10-07
---

# Product Brief: terminal-blocks

## Executive Summary

tty7 在 server 模式（shell 集成成功、收到 OSC 133 marks）下引入命令块：一个块就是一次命令的执行结果（命令 + 输出 + 退出态）。用户可折叠长输出、右键跳到块首、一键复制整个块（纯文本，命令+输出+退出码行）。无集成时保持现有 scrollback，不猜块。这是 WindTerm fold/outline 思路在 tty7 上的精确版：以 OSC 为准而不是正则猜 prompt。

## The Problem

长输出场景下回翻和定位成本高：向上翻很麻烦，翻到命令执行开始的地方困难；想复制一次执行的完整结果时，要手滚选区，容易错行漏行。现有 scrollback 是平的，没有“一次执行”的概念，通知和退出码也和屏幕内容脱节。

## The Solution

只在有可靠边界时分块（OSC 133 A/B/C/D）。每个块提供：折叠/展开（手动，gutter 或右键）、右键跳到块首、复制整个块（纯文本，命令+输出+退出码行）。Ctrl-C 中断也算一块；非零退出红色标记。折叠态存 server 侧、跟 pane 走，重启恢复；scrollback 清除则块消失。alt-screen（vim/全屏 TUI）不算块、不折叠。

## What Makes This Different

- 以 OSC 为准，不用正则猜：windterm 靠 lexer 正则，自定义提示符就失效；tty7 有 marks 就精确，没 marks 就不分块，不误伤。
- 与现有能力同源：prompt marks、cwd、退出码、command-finished 通知已在集成里，块是同一数据源的自然延伸。
- server 拥有块：持久会话、重启恢复、远端一致都落在 server 侧，而不是 GUI 本地状态。

## Who This Serves

- 主：在 tty7 里跑构建/测试/长日志的本地用户，server 模式常开。
- 次：直连远端 workspace（ssh）的用户，远端 bootstrap 成功时体验与本地一致。

## Success Criteria

- 万行输出内，2 次点击内回到块首并完成整块复制，不再手滚选区。
- 有 marks 的 pane 分块准确率：命令与输出不错位；Ctrl-C 与失败块标记正确。
- 折叠态重启后恢复；11MB 级 `cat` 基准无可感知的交互退化（以后续实测为准）。

## Scope

In（v1）：
- 有 marks 才分块：本地 pane + 远端直连 pane（远端 bootstrap 成功）。
- 嵌套 ssh：端到端有 marks 则分块，无则无块。
- 手动折叠/展开、右键跳块首、复制整块（纯文本命令+输出+退出码行）、失败红标、Ctrl-C 成块、alt-screen 排除、折叠态持久化。

Out（v1 明确不做）：
- 无集成的启发式/正则分块。
- 自动折叠、快捷键、outline/命令面板。
- ANSI 富文本复制、搜索与块的联动。

## Vision

块成为 tty7 回看与协作的基本单位：先是折叠/跳转/复制，之后可长出按块搜索、按块分享、多块多选复制，以及 agent 回看执行结果时的稳定引用。
