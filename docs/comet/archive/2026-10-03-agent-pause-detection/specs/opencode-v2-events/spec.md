# opencode-v2-events

Complete target behavior after Archive: the OpenCode bridge maps opencode v2.0.18's actual event vocabulary onto tty7's unchanged `Working/Waiting/Done` semantics.

## 1. Mapping (the complete table)

Shared `presence()` tracker, `capture()` emits `session-start` on the first event naming a session id; subagent child sessions excluded via `session.created` parent links.

| opencode event | tty7 report | Meaning |
|---|---|---|
| `session.status.busy` | `prompt-submit` → Working | turn started (older 2.x) |
| `session.status.idle` | `stop` → Done | turn ended (older 2.x) |
| `session.idle` | `stop` → Done | idle fallback |
| `session.execution.started` | `prompt-submit` → Working | turn started (v2.0.18) |
| `session.execution.succeeded` / `.failed` | `stop` → Done | turn ended (v2.0.18) |
| `permission.asked` / `.v2.asked` | `permission-request` → Waiting | approval prompt |
| `permission.replied` / `.v2.replied` | `prompt-submit` → Working | approval answered |
| `question.asked` / `.v2.asked` / `form.created` | `question-asked` → Waiting | user question; `form.*` is TUI-local, absent from published SDK types |
| `question.replied` / `.rejected` (+ v2) / `form.answered` / `.rejected` / `.dismissed` | `prompt-submit` → Working | question answered |

Unknown event names stay ignored. Old names kept for earlier builds.

## 2. Scope notes

- v2 `setup` stays inert under the machine-wide `--service` server (sessions cannot be attributed to panes there); `opencode --standalone` (or a pane-owned `serve`) is the supported launch.
- `AgentStatus` transitions, socket/OSC transport, `free`/`NoAgent` split, and `tty7 wait` polling semantics unchanged.

## 3. Out of scope

`tui-idle`, dashboard kanban, the 6 hook-less agents, and a global on/off switch (dropped as redundant: per-agent Uninstall already covers it) remain future or rejected items.
