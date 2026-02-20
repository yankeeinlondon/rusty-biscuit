---
homepage: https://github.com/openai/codex
docs: https://developers.openai.com/codex/cli/
hooks: https://developers.openai.com/codex/config-advanced/
---

# Codex CLI Hooks and Events

Homepage: https://github.com/openai/codex

Documentation: https://developers.openai.com/codex/cli/

## Scope

This document covers the hook and event surfaces available in the Codex CLI (OpenAI's agentic CLI). Codex provides two integration surfaces:

1. A **notify hook** that fires an external program after agent turns complete (and after tool use, internally).
2. A **JSONL event stream** (`codex exec --json`) for non-interactive automation and CI/CD pipelines.

The hooks system is **outbound-only** and **fire-and-forget**. There is no mechanism to block, modify, or approve tool calls through hooks. Community PRs proposing Claude Code-style blocking/steering hooks (PRs #2904, #9796, #11067) have been declined by OpenAI, who have stated they are designing a hooks system internally.

(https://github.com/openai/codex/issues/2109) (https://github.com/openai/codex/discussions/2150)

## Configuration

### Config file locations

Codex uses TOML configuration files. Multiple locations are supported, merged by priority.

| Location | Scope | Priority |
|----------|-------|----------|
| CLI flags and `--config` | Invocation | Highest |
| `--profile <name>` values | Per-profile | High |
| `.codex/config.toml` (project) | Single project (trusted projects only) | Medium-high |
| `~/.codex/config.toml` | All projects (user) | Medium |
| `/etc/codex/config.toml` (Unix) | System-wide | Low |
| Built-in defaults | Fallback | Lowest |

Project-level `.codex/config.toml` files are only loaded for trusted projects. Untrusted projects skip all project-scoped layers.

(https://developers.openai.com/codex/config-basic/)

### Notify hook configuration

The `notify` key in `config.toml` specifies an external command to run after each completed agent turn. The value is an argv array (the program and its fixed arguments). Codex appends one additional argument containing a JSON payload describing the event.

```toml
# ~/.codex/config.toml

# External notifier - fires after each agent turn completes
notify = ["notify-send", "Codex"]
```

Which Codex invokes as:

```shell
notify-send Codex '{"type":"agent-turn-complete","thread-id":"...","turn-id":"12345",...}'
```

If unset or empty, the feature is disabled.

### TUI notification configuration

Built-in TUI notifications are separate from the `notify` hook. They control terminal alerts when the window is unfocused.

```toml
[tui]
# Boolean or filtered event list. Default: true
# Examples: false | ["agent-turn-complete", "approval-requested"]
notifications = true

# Notification method: auto | osc9 | bel. Default: "auto"
notification_method = "auto"
```

| Method | Behavior |
|--------|----------|
| `auto` | Prefers OSC 9 escape sequences; falls back to BEL |
| `osc9` | Terminal escape sequence for desktop notifications |
| `bel` | ASCII bell character (`\x07`) |

### Example configurations

**macOS sound on turn complete:**

```toml
notify = ["bash", "-lc", "afplay /System/Library/Sounds/Blow.aiff"]
```

**Linux desktop notification:**

```toml
notify = ["notify-send", "Codex"]
```

**Webhook notification:**

```toml
notify = ["python3", "/path/to/notify.py"]
```

**TUI notifications filtered to approvals only:**

```toml
[tui]
notifications = ["approval-requested"]
notification_method = "osc9"
```

(https://developers.openai.com/codex/config-advanced/) (https://developers.openai.com/codex/config-sample/)

## Hook Events

Codex has two internal hook event types. Only `AfterAgent` is currently wired to the user-facing `notify` configuration. `AfterToolUse` is implemented internally but not yet exposed to user configuration (as of CLI 0.102.0, February 2026).

### AfterAgent (notify)

**Triggers:** When the agent finishes processing a user submission (a complete turn).

**User-facing:** Yes, via the `notify` config key.

**Delivery:** The configured command is spawned as a fire-and-forget process. Codex does not wait for it to complete and does not read its output. The JSON payload is passed as the final command-line argument (not stdin).

**Event payload:**

```json
{
  "type": "agent-turn-complete",
  "thread-id": "b5f6c1c2-1111-2222-3333-444455556666",
  "turn-id": "12345",
  "cwd": "/Users/example/project",
  "input-messages": ["Rename `foo` to `bar` and update the callsites."],
  "last-assistant-message": "Rename complete and verified `cargo build` succeeds."
}
```

| Field | Type | Description |
|-------|------|-------------|
| `type` | string | Always `"agent-turn-complete"` |
| `thread-id` | string (UUID) | Session identifier |
| `turn-id` | string | Turn identifier within the session |
| `cwd` | string | Current working directory when the turn completed |
| `input-messages` | string[] | User messages that initiated the turn |
| `last-assistant-message` | string or null | Final assistant response text |

**Event response:** None. The hook is fire-and-forget. Exit code and output are ignored (stdout, stderr, stdin are all connected to `/dev/null`).

**Gotchas:**

1. **Only one event type:** The `notify` system only fires on `agent-turn-complete`. It does not fire on approval requests, tool calls, session start/end, or errors. For approval alerts, use `tui.notifications = ["approval-requested"]` instead.
2. **Argument-based, not stdin:** The JSON payload is passed as a CLI argument, not piped to stdin. Scripts must read `$1`, not stdin.
3. **Fire-and-forget:** If the notifier command fails or hangs, Codex logs the spawn error at debug level but never blocks the agent. There is no retry mechanism.
4. **No flow control:** The hook cannot block, deny, modify, or steer the agent. It is purely observational.
5. **kebab-case keys:** The legacy payload uses kebab-case field names (`thread-id`, `turn-id`, `input-messages`, `last-assistant-message`), not snake_case.

### AfterToolUse (internal only)

**Triggers:** After any tool execution completes (success or failure).

**User-facing:** Not yet. As of CLI 0.100.0 (February 2026), this hook is implemented in the core runtime but is not wired to user configuration. The `HooksConfig` struct only accepts `legacy_notify_argv` and the `after_tool_use` hook vector is initialized empty.

(https://developers.openai.com/codex/changelog/) (https://github.com/openai/codex/pull/11335)

**Internal payload (for reference):**

```json
{
  "session_id": "0199a213-81c0-7800-8aa1-bbab2a035a53",
  "cwd": "/Users/example/project",
  "triggered_at": "2026-02-11T00:00:00Z",
  "hook_event": {
    "event_type": "after_tool_use",
    "turn_id": "turn-2",
    "call_id": "call-1",
    "tool_name": "local_shell",
    "tool_kind": "local_shell",
    "tool_input": {
      "input_type": "local_shell",
      "params": {
        "command": ["cargo", "fmt"],
        "workdir": "codex-rs",
        "timeout_ms": 60000,
        "sandbox_permissions": "use_default",
        "justification": null,
        "prefix_rule": null
      }
    },
    "executed": true,
    "success": true,
    "duration_ms": 42,
    "mutating": true,
    "sandbox": "none",
    "sandbox_policy": "danger-full-access",
    "output_preview": "ok"
  }
}
```

**Payload fields (hook_event):**

| Field | Type | Description |
|-------|------|-------------|
| `event_type` | string | Always `"after_tool_use"` |
| `turn_id` | string | Turn within the session |
| `call_id` | string | Unique identifier for this tool call |
| `tool_name` | string | Name of the tool (e.g., `local_shell`, `apply_patch`) |
| `tool_kind` | string | One of: `function`, `custom`, `local_shell`, `mcp` |
| `tool_input` | object | Tagged union by `input_type` (see below) |
| `executed` | boolean | Whether the tool actually ran |
| `success` | boolean | Whether the tool completed successfully |
| `duration_ms` | integer | Wall-clock execution time in milliseconds |
| `mutating` | boolean | Whether the tool modifies state |
| `sandbox` | string | Sandbox mode used (e.g., `"none"`, `"seatbelt"`) |
| `sandbox_policy` | string | Sandbox policy name |
| `output_preview` | string | Truncated/serialized tool output |

**Tool input variants (`tool_input.input_type`):**

| input_type | Fields | Description |
|------------|--------|-------------|
| `function` | `arguments` (string) | Function-call style tool |
| `custom` | `input` (string) | Custom tool (e.g., `apply_patch`) |
| `local_shell` | `params.command` (string[]), `params.workdir`, `params.timeout_ms`, `params.sandbox_permissions`, `params.justification`, `params.prefix_rule` | Shell command execution |
| `mcp` | `server` (string), `tool` (string), `arguments` (string) | MCP server tool call |

**Hook result types (internal):**

| Result | Behavior |
|--------|----------|
| `Success` | Hook completed, continue normally |
| `FailedContinue` | Hook failed, log warning, continue with remaining hooks and operation |
| `FailedAbort` | Hook failed, skip remaining hooks, abort the operation with a fatal error |

**Gotchas:**

1. **Not user-configurable yet:** You cannot register `AfterToolUse` hooks via `config.toml`. The infrastructure exists but is not exposed.
2. **Different payload format:** The internal hook payload uses snake_case and a nested `hook_event` structure, unlike the legacy `notify` payload which uses kebab-case and a flat structure.
3. **Abort capability:** Unlike `AfterAgent`, the `AfterToolUse` hook can abort operations via `FailedAbort`, which returns a fatal error to the agent. This is a significant distinction that will matter when user configuration is eventually added.

## JSONL Event Stream (`codex exec --json`)

Running Codex in non-interactive mode with `--json` streams a newline-delimited JSON event stream to stdout. This is the primary automation surface for CI/CD pipelines and external orchestrators.

```bash
codex exec --json "your task prompt" | jq
```

### Configuration flags

| Flag | Description |
|------|-------------|
| `--json` | Enable JSONL streaming to stdout |
| `-o <path>` / `--output-last-message <path>` | Write final assistant message to a file |
| `--output-schema <path>` | JSON Schema file; Codex validates the final response against it |

### Event types

Each JSONL line is a JSON object with a `type` field.

#### thread.started

Emitted once at the beginning of a run.

```json
{"type": "thread.started", "thread_id": "0199a213-81c0-7800-8aa1-bbab2a035a53"}
```

| Field | Type | Description |
|-------|------|-------------|
| `type` | string | `"thread.started"` |
| `thread_id` | string (UUID) | Correlation key for all subsequent events |

#### turn.started

Marks the beginning of a new agent turn.

```json
{"type": "turn.started"}
```

#### turn.completed

Signals successful completion of a turn with token usage.

```json
{"type": "turn.completed", "usage": {"input_tokens": 24763, "cached_input_tokens": 24448, "output_tokens": 122}}
```

| Field | Type | Description |
|-------|------|-------------|
| `usage.input_tokens` | integer | Total input tokens consumed |
| `usage.cached_input_tokens` | integer | Input tokens served from cache |
| `usage.output_tokens` | integer | Output tokens generated |

#### turn.failed

Signals a failed turn. Includes error details.

```json
{"type": "turn.failed", "error": {"message": "Rate limit exceeded"}}
```

#### item.started / item.updated / item.completed

Item lifecycle events. Each item has an `id` and a `type`-specific payload.

```json
{"type": "item.started", "item": {"id": "item_1", "type": "command_execution", "command": "bash -lc ls", "status": "in_progress"}}
{"type": "item.completed", "item": {"id": "item_3", "type": "agent_message", "text": "Repo contains docs, sdk, and examples directories."}}
```

#### error

Fatal or recoverable error event.

```json
{"type": "error", "error": {"message": "Connection timeout"}}
```

### Item types

| Item type | Description | Key fields |
|-----------|-------------|------------|
| `agent_message` | Assistant text response | `text` |
| `reasoning` | Model reasoning/thinking | `text` (can be suppressed via config) |
| `command_execution` | Shell command execution | `command`, `status`, `exit_code` |
| `file_change` | File modification | File path and change details |
| `mcp_tool_call` | MCP server tool invocation | Server and tool identifiers |
| `web_search` | Web search performed | Search query and results |
| `plan_update` | Agent plan modification | Updated plan content |

### Structured output (`--output-schema`)

For deterministic downstream parsing, provide a JSON Schema to validate the final response:

```bash
codex exec --json --output-schema schema.json "Analyze this repo"
```

Codex validates the final `agent_message` against the schema, making the output shape reliable for automation.

(https://developers.openai.com/codex/noninteractive/) (https://developers.openai.com/codex/cli/reference/)

## Telemetry Event Export (OpenTelemetry)

Codex can export structured telemetry events via OpenTelemetry. This is not a control hook but provides a structured event stream for observability.

```toml
[otel]
exporter = "otlp"  # Options: "none", "otlp"
```

When `exporter = "none"`, Codex records events internally but sends nothing. Exporters batch asynchronously and flush on shutdown.

**Representative telemetry event types:**

| Event | Description |
|-------|-------------|
| `codex.conversation_starts` | Session initiated |
| `codex.api_request` | API call made |
| `codex.sse_event` | Server-sent event received |
| `codex.user_prompt` | User prompt submitted (redacted by default) |
| `codex.tool_decision` | Tool call decision made |
| `codex.tool_result` | Tool execution result |

(https://developers.openai.com/codex/config-advanced/)

## Matcher System

Codex does **not** have a matcher system for hooks. The `notify` hook fires on every `agent-turn-complete` event unconditionally. There is no mechanism to filter by tool name, event subtype, or pattern.

The TUI notifications support a limited filter via an array of event type strings:

```toml
[tui]
notifications = ["agent-turn-complete", "approval-requested"]
```

This only controls which events trigger TUI desktop alerts, not which events invoke external hooks.

Community PRs have proposed pattern-matching systems (e.g., PR #11067 proposed `patterns = ["shell:*", "mcp:*"]`), but these have not been merged.

## Known Gotchas and Workarounds

### 1. No blocking or steering hooks

**Problem:** Unlike Claude Code, Codex hooks cannot block, deny, modify, or approve tool calls. The `notify` hook is purely observational and fire-and-forget.

**Workaround:** For programmatic control over Codex, use `codex exec --json` and build external orchestration that parses the JSONL stream and manages the workflow externally. Alternatively, use approval policies (`full_auto`, `auto_edit`, `ask_always`) configured in `config.toml` to control which actions require user approval.

### 2. JSON event schema is experimental

**Problem:** The `--json` flag is also exposed as `--experimental-json`, indicating the event schema can change between versions.

**Workaround:** Use tolerant JSON parsing. Ignore unknown fields. Do not hard-code exact payload shapes beyond `type`, `thread_id`, and `item.id`.

(https://developers.openai.com/codex/cli/reference/)

### 3. Notify payload is a CLI argument, not stdin

**Problem:** The `notify` JSON payload is passed as the last command-line argument, not piped to stdin. Scripts that read from stdin will hang.

**Workaround:** Read `$1` (first/last positional argument) in your notify script:

```bash
#!/bin/bash
PAYLOAD="$1"
echo "$PAYLOAD" | jq '.type'
```

### 4. JSONL stdout replaces human-readable output

**Problem:** In `--json` mode, stdout is JSONL only. The final human-readable message is not printed as plain text.

**Workaround:** Parse the final `agent_message` item from the JSONL stream, or use `-o/--output-last-message` to capture the final output directly to a file.

(https://developers.openai.com/codex/noninteractive/)

### 5. Item updates can be incremental

**Problem:** `item.updated` events contain partial data, not the full item state.

**Workaround:** Store per-item state keyed by `item.id` and merge updates until `item.completed`.

### 6. Reasoning events can be suppressed

**Problem:** Reasoning events may or may not appear depending on configuration (`hide_agent_reasoning` / `show_raw_agent_reasoning`). Do not depend on them unless you control the config.

**Workaround:** Check for the presence of reasoning items but do not require them for control flow.

(https://developers.openai.com/codex/config-advanced/)

### 7. Telemetry exporters drop events on abrupt termination

**Problem:** OTel exporters flush asynchronously on shutdown. Killing the process (SIGKILL) can lose events.

**Workaround:** Allow graceful shutdown (SIGTERM/SIGINT) or use external retries in your log collector.

### 8. AfterToolUse abort is a fatal error

**Problem:** When the internal `AfterToolUse` hook returns `FailedAbort`, it produces a `FunctionCallError::Fatal` that terminates the tool call pipeline. This behavior is not documented and not user-configurable yet, but developers building on the Codex core library should be aware.

**Workaround:** If using the Codex Rust library directly, prefer `FailedContinue` for non-critical hook failures.

### 9. TUI notifications vs notify are independent

**Problem:** Configuring `tui.notifications = false` does not disable the `notify` external command, and vice versa. They are separate systems.

**Workaround:** Configure each independently. Use `notify` for external integrations (webhooks, CI). Use `tui.notifications` for desktop alerts while working interactively.

## Sources

- Codex CLI homepage: https://github.com/openai/codex
- Codex CLI documentation: https://developers.openai.com/codex/cli/
- Codex CLI features: https://developers.openai.com/codex/cli/features/
- Codex CLI reference: https://developers.openai.com/codex/cli/reference/
- Codex config basics: https://developers.openai.com/codex/config-basic/
- Codex config advanced: https://developers.openai.com/codex/config-advanced/
- Codex config reference: https://developers.openai.com/codex/config-reference/
- Codex config sample: https://developers.openai.com/codex/config-sample/
- Codex non-interactive mode: https://developers.openai.com/codex/noninteractive/
- Codex changelog: https://developers.openai.com/codex/changelog/
- Event hooks feature request: https://github.com/openai/codex/issues/2109
- Hooks discussion: https://github.com/openai/codex/discussions/2150
- AfterToolUse hook PR (merged): https://github.com/openai/codex/pull/11335
- Allow hooks to error PR (merged): https://github.com/openai/codex/pull/11615
- Comprehensive hooks PR (closed, not merged): https://github.com/openai/codex/pull/11067
- Codex hooks source code: https://github.com/openai/codex/tree/main/codex-rs/hooks/src
