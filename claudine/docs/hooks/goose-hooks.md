---
prompt: >-
    Do online research on the hooks/event which can be leveraged when using the Goose Agentic CLI (https://block.github.io/goose/). Describe each hook, what data it returns, what kind of return type is expected and how that return type effects the agentic flow. in the end describe any known gotcha in working with this event/hook model and how people have gotten around these quirks or shortcomings.
---

# Goose CLI hooks and event stream

This summarizes the hook and event surfaces available when you run goose via the CLI. The key integration points are:

- A status hook you can point to a shell command.
- A streamable JSON event feed (`--output-format stream-json`) that emits messages, notifications, model changes, errors, and completion.
- MCP logging/progress notifications (including subagent and task-execution notifications) that are forwarded into the stream and/or rendered in the CLI.

## Hook: `GOOSE_STATUS_HOOK`

What it is

- A CLI status hook that runs a shell command when goose changes status.
- Implemented in `goose-cli` and invoked via `run_status_hook(status)`.

Data returned

- The hook is invoked with a single argument: a status string.
- Statuses seen in the CLI code are: `waiting` (idle, waiting for user input) and `thinking` (agent is processing).

Return type and effect on agentic flow

- Return type: a process exit code (from the shell command).
- The CLI spawns the hook asynchronously and discards stdout/stderr; the exit code is ignored.
- Effect: no effect on agent flow. It is fire-and-forget; you cannot block, cancel, or modify behavior with this hook.

Source: https://raw.githubusercontent.com/block/goose/main/crates/goose-cli/src/session/output.rs

## Event stream: `--output-format stream-json`

What it is

- A newline-delimited JSON stream for automation and real-time UI updates.
- The CLI emits one JSON object per event while a `goose run` session is executing.

Event types and payloads
All events are JSON objects with a `type` field (`snake_case`). The core event types are:

1) `message`

- Payload: `{ "type": "message", "message": <Message> }`
- `message` is the full goose message object (role + content array). Content can include text, tool requests/responses, action_required, etc.
- Effect: informational; consumers can render messages or detect `action_required` content to prompt a user.

1) `notification`

- Payload: `{ "type": "notification", "extension_id": "<id>", ... }`
- Two shapes are emitted (flattened in the event):
    - Log: `{ "log": { "message": "..." } }`
    - Progress: `{ "progress": { "progress": <float>, "total": <float|null>, "message": "<string|null>" } }`
- Effect: informational; used for progress bars, subagent tool logs, and task dashboards.

1) `model_change`

- Payload: `{ "type": "model_change", "model": "<model>", "mode": "<mode>" }`
- Effect: informational; indicates the active model or mode has changed (e.g., lead/worker or planner transitions).

1) `error`

- Payload: `{ "type": "error", "error": "<string>" }`
- Effect: informational; indicates an error in the agent loop. The CLI continues cleanup and exits the stream with `complete`.

1) `complete`

- Payload: `{ "type": "complete", "total_tokens": <int|null> }`
- Effect: informational; marks end of the run.

Return type and effect on agentic flow

- Return type: JSON objects printed to stdout (one per event).
- There is no return channel; parsing the stream does not affect the agent’s decisions.

Sources:

- https://block.github.io/goose/docs/guides/running-tasks/
- https://raw.githubusercontent.com/block/goose/main/crates/goose-cli/src/session/mod.rs

## MCP notification payloads worth handling

goose forwards MCP notifications into the CLI and stream-json output. These are emitted as `notification` events (log/progress) and are driven by the notification payloads sent by extensions or subagents.

### Subagent tool request notifications

What it is

- A logging notification emitted when a subagent issues a tool call.

Data returned

- Payload fields: `type: "subagent_tool_request"`, `subagent_id`, and `tool_call` with `name` and `arguments`.

Return type and effect on flow

- Return type: JSON object in the notification payload; in stream-json mode this is re-emitted as a `notification` log event with a formatted message string.
- Effect: informational only; it does not change agent behavior.

Source: https://raw.githubusercontent.com/block/goose/main/crates/goose/src/agents/subagent_handler.rs

### Task execution notifications (`type: "task_execution"`)

What it is

- Structured updates about subagent task execution and progress.

Data returned

- A notification object with `type: "task_execution"` and a `subtype` field:
    - `line_output`: `{ task_id, output }`
    - `tasks_update`: `{ stats, tasks }`
    - `tasks_complete`: `{ stats, failed_tasks }`
- `stats` fields include totals and counts (pending/running/completed/failed).
- `tasks` entries include `id`, `status`, `duration_secs`, `current_output`, `task_type`, `task_name`, `task_metadata`, `error`, and `result_data`.

Return type and effect on flow

- Return type: JSON object in the notification payload (serialized by the subagent system).
- In stream-json mode, the CLI currently converts these into formatted log strings (not the raw structured event).
- Effect: informational only; used for display and monitoring.

Sources:

- https://raw.githubusercontent.com/block/goose/main/crates/goose/src/agents/subagent_execution_tool/notification_events.rs
- https://raw.githubusercontent.com/block/goose/main/crates/goose-cli/src/session/task_execution_display/mod.rs

### Progress/log notifications

What it is

- Standard MCP progress and logging notifications from extensions.

Data returned

- Log notifications often include `message` and optional `type`, `subagent_id`, or `output` fields.
- Progress notifications include `progress`, `total`, and optional `message`.

Return type and effect on flow

- Return type: JSON object in the notification payload; in stream-json mode re-emitted as a `notification` event with `log` or `progress` data.
- Effect: informational; used for progress rendering and diagnostic logging.

Source: https://raw.githubusercontent.com/block/goose/main/crates/goose-cli/src/session/mod.rs

## Known gotchas and workarounds

1) Task execution notifications are flattened in stream-json

- The CLI formats `task_execution` events into log strings rather than emitting the raw structured event in `stream-json` mode.
- Workaround: consume MCP notifications at the extension or client layer if you need structured task updates; otherwise parse the formatted log output.

1) `GOOSE_STATUS_HOOK` is fire-and-forget

- It runs asynchronously with stdout/stderr suppressed, and exit codes are ignored. You cannot block or modify execution.
- Workaround: use the stream-json event feed for richer, synchronous state updates; in the hook, write to a file or external service if you need state persistence.

1) Event schemas are not versioned

- The stream-json event set is defined in code and may evolve; unknown fields or new event types can appear.
- Workaround: implement tolerant parsing (ignore unknown fields/types) and treat the stream as best-effort telemetry.

1) Tool output may be filtered in CLI rendering

- The CLI can suppress low-priority tool output depending on `GOOSE_CLI_MIN_PRIORITY` and debug settings.
- Workaround: run with `--debug` or set `GOOSE_CLI_MIN_PRIORITY=0` when you need full tool output in notifications.
