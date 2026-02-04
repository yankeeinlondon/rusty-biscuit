# Codex CLI hooks and events

## Scope
This document focuses on the hook and event surfaces available in the Codex Agentic CLI (Codex CLI). The primary integration surface is the JSONL event stream from `codex exec --json`, with auxiliary hooks for notifications and telemetry export. Sources are cited inline.

## JSONL event stream (`codex exec --json`)
Running Codex in non-interactive mode with `--json` streams a newline-delimited JSON (JSONL) event stream to stdout. Each line is a JSON object representing a state change while the agent runs. This mode is intended for automation and CI pipelines. The final answer is emitted as an item event, not as plain text on stdout. Use `-o/--output-last-message` if you want the final message written to a file in addition to the JSONL stream. (https://developers.openai.com/codex/noninteractive/)

### Return type

- Type: JSON Lines (one JSON object per line).
- Direction: outbound only. The CLI does not accept responses or acknowledgements from the consumer; your code simply reads and reacts to events.
- Schema note: the CLI flag is exposed as `--json` and `--experimental-json`, which implies the event schema can change. Use tolerant parsing. (https://developers.openai.com/codex/cli/reference/)

### Event types
Event objects include a top-level `type` and optional fields per event. Documented event types include:

1) `thread.started`

- Data: includes a `thread_id` string in the sample output. (https://developers.openai.com/codex/noninteractive/)
- Effect on flow: treat this as the correlation key for all subsequent events. External orchestrators typically create a run context keyed by `thread_id` and can use it to resume runs later.

1) `turn.started`

- Data: at minimum, the `type` field.
- Effect on flow: marks the start of a new turn. Consumers typically open a new aggregation window for `item.*` events until a matching `turn.completed` or `turn.failed` arrives. (https://developers.openai.com/codex/noninteractive/)

1) `turn.completed`

- Data: includes a `usage` object with token counts (for example `input_tokens`, `cached_input_tokens`, `output_tokens`). (https://developers.openai.com/codex/noninteractive/)
- Effect on flow: signals a successful end of the turn. External workflows usually treat this as the completion signal for the run and finalize any aggregation of items.

1) `turn.failed`

- Data: includes error details (documented in non-interactive mode). (https://developers.openai.com/codex/noninteractive/)
- Effect on flow: signals a failed turn. External automation should treat the run as failed and decide whether to retry or to resume with `codex exec resume`.

1) `item.started`, `item.updated`, `item.completed`

- Data: includes an `item` object with an `id` and a type-specific payload (see Item types below). (https://docs.onlinetool.cc/codex/docs/exec.html)
- Effect on flow: use `item.started` to initialize per-item state, `item.updated` for incremental updates, and `item.completed` to finalize and persist the item result.

1) `error`

- Data: includes error details (documented in non-interactive mode). (https://developers.openai.com/codex/noninteractive/)
- Effect on flow: treat as a fatal or recoverable error depending on your orchestration strategy.

### Item types (documented)
The JSONL stream includes item events for specific item types. Official docs list the following categories: agent messages, reasoning, command executions, file changes, MCP tool calls, web searches, and plan updates. (https://developers.openai.com/codex/noninteractive/)

Below is a practical mapping of each item type to the data you can expect and how it affects orchestration. The payload shape can vary by item type; implement tolerant parsing and key off the item `type`.

- Agent message (`agent_message` in the sample output)
    - Data: typically includes `text` with the assistant response. (https://developers.openai.com/codex/noninteractive/)
    - Return type effect: this is the primary output payload for automation. If you need structured data, pair `--json` with `--output-schema` to enforce JSON output. (https://developers.openai.com/codex/noninteractive/)

- Reasoning (`reasoning`)
    - Data: a summary of the agent's thinking. (https://developers.openai.com/codex/noninteractive/)
    - Return type effect: optional and can be suppressed; do not rely on it for control flow unless you explicitly enable it in config. (https://developers.openai.com/codex/config-advanced/)

- Command execution (`command_execution`)
    - Data: includes the `command` and a `status` in the official sample; mirrors often include `exit_code` and aggregated output when the command completes. (https://developers.openai.com/codex/noninteractive/) (https://docs.onlinetool.cc/codex/docs/exec.html)
    - Return type effect: downstream systems commonly gate on command success (for example, exit code) before triggering the next step.

- File change (`file_change`)
    - Data: indicates a file modification occurred; exact fields are not fully specified in the CLI docs, so treat the payload as opaque unless your integration is pinned to a known version.
    - Return type effect: external systems can use this as a signal to run formatters, tests, or a diff review stage.

- MCP tool call (`mcp_tool_call`)
    - Data: indicates an MCP tool was invoked. (https://developers.openai.com/codex/noninteractive/)
    - Return type effect: use this as an audit marker for external tool usage.

- Web search (`web_search`)
    - Data: indicates a web search was performed. (https://developers.openai.com/codex/noninteractive/)
    - Return type effect: helpful for traceability and for validating that a run used cached vs live results (controlled via `--search`). (https://developers.openai.com/codex/cli/features/)

- Plan update (`plan_update`)
    - Data: indicates the plan changed during the run. (https://developers.openai.com/codex/noninteractive/)
    - Return type effect: useful for progress tracking in dashboards, but should not be required for completion logic.

## Structured final outputs (`--output-schema`)
If your automation needs a stable return type for the final result, provide a JSON Schema using `--output-schema`. Codex validates the final response against the schema, making downstream parsing deterministic. This is the only place where you explicitly specify an expected return type for the agent's final output. (https://developers.openai.com/codex/noninteractive/) (https://developers.openai.com/codex/cli/reference/)

## Notification hook (`notify`)
Codex supports a simple hook for running an external program when specific events occur. Configure it via `notify` in `config.toml`. Currently, the only supported event is `agent-turn-complete`. The hook receives a single JSON argument with fields such as `type`, `thread-id`, `turn-id`, `cwd`, `input-messages`, and `last-assistant-message`. (https://developers.openai.com/codex/config-advanced/)

Return type and flow impact:

- Return type: there is no structured return payload; the hook is invoked as an external process.
- Flow impact: the hook is a side-channel for notifications (desktop alerts, webhooks, CI logs). It does not alter the agent's internal flow; use it to trigger external actions.

## Telemetry event export (OTel)
Codex can export structured telemetry events via OpenTelemetry. This is not a control hook, but it is a structured event stream useful for observability. Configure exporters in `[otel]`; when `exporter = "none"`, Codex records events but sends nothing. Exporters batch asynchronously and flush on shutdown. Representative event types include `codex.conversation_starts`, `codex.api_request`, `codex.sse_event`, `codex.user_prompt` (redacted by default), `codex.tool_decision`, and `codex.tool_result`. (https://developers.openai.com/codex/config-advanced/)

Return type and flow impact:

- Return type: structured log events emitted to the configured exporter.
- Flow impact: no effect on agent decisions; use for monitoring, analytics, and compliance.

## Known gotchas and workarounds

- JSON event schema is flagged as experimental: prefer tolerant parsing, ignore unknown fields, and avoid hard-coding exact payload shapes beyond `type`, `thread_id`, and `item.id`. (https://developers.openai.com/codex/cli/reference/)
- In `--json` mode, stdout is JSONL; the final human-readable message is not printed as plain stdout. Workaround: parse the final `agent_message` item or use `-o/--output-last-message` to capture the final output directly to a file. (https://developers.openai.com/codex/noninteractive/)
- Item updates can be incremental (`item.updated`). Workaround: store per-item state keyed by `item.id` and merge updates until `item.completed`. (https://docs.onlinetool.cc/codex/docs/exec.html)
- Reasoning events can be suppressed (`hide_agent_reasoning`) or surfaced (`show_raw_agent_reasoning`), so do not depend on them unless you control the config. (https://developers.openai.com/codex/config-advanced/)
- The only built-in hook for interactive sessions is `notify`, and it currently fires only on `agent-turn-complete`. For richer control, run in non-interactive mode with `codex exec --json` and orchestrate externally. (https://developers.openai.com/codex/config-advanced/) (https://developers.openai.com/codex/noninteractive/)
- Telemetry exporters flush on shutdown; abrupt termination can drop events. Workaround: allow graceful shutdown or use external retries in your log collector. (https://developers.openai.com/codex/config-advanced/)

## Sources

- https://developers.openai.com/codex/noninteractive/
- https://developers.openai.com/codex/cli/reference/
- https://developers.openai.com/codex/cli/features/
- https://developers.openai.com/codex/config-advanced/
- https://docs.onlinetool.cc/codex/docs/exec.html
