---
homepage: https://block.github.io/goose/
docs: https://block.github.io/goose/docs/guides/config-files/
hooks: https://block.github.io/goose/docs/guides/config-files/
---

# Goose CLI Hooks and Event Stream

Homepage: https://block.github.io/goose/

Documentation: https://block.github.io/goose/docs/guides/config-files/

## Scope

This document covers the hook and event surfaces available in the Goose agentic CLI (Block's open-source AI agent). Goose does **not** have a traditional pre/post lifecycle hook system like Claude Code or Gemini CLI. Instead, it provides:

1. A **status hook** (`GOOSE_STATUS_HOOK`) that fires a shell command on agent state transitions.
2. A **streaming JSON event feed** (`--output-format stream-json`) that emits structured events during `goose run`.
3. A **batch JSON output** (`--output-format json`) for post-run consumption.
4. **MCP notification forwarding** for extension and subagent progress.

All integration points are **outbound-only** (observe, do not control). There is no mechanism to block, modify, or approve tool calls through hooks.

## Configuration

### Config file locations

Goose uses YAML configuration files. There is no project-scoped config file; all configuration is user-scoped.

| File | Path (macOS/Linux) | Purpose |
|------|-------------------|---------|
| `config.yaml` | `~/.config/goose/config.yaml` | Provider, model, extensions, status hook, and general settings |
| `permission.yaml` | `~/.config/goose/permission.yaml` | Tool permission levels (configured via CLI) |
| `secrets.yaml` | `~/.config/goose/secrets.yaml` | API keys (file-based fallback when keyring unavailable) |
| `tool_permissions.json` | `~/.config/goose/permissions/tool_permissions.json` | Runtime permission decisions |

On Windows, the base directory is `%APPDATA%\Block\goose\config\`.

Configuration priority (highest to lowest): environment variables, config file settings, default values.

### Goosehints (context injection, not hooks)

Goose has a `.goosehints` system for injecting context into the agent's system prompt. This is not a hook mechanism but is worth noting for completeness.

| Location | Scope |
|----------|-------|
| `~/.config/goose/.goosehints` | Global (all sessions) |
| `.goosehints` (any directory in hierarchy) | Project (local takes priority on conflict) |

Goosehints are loaded at session start and become part of every request's system prompt. They support `@filename.md` syntax to include referenced files. The `CONTEXT_FILE_NAMES` environment variable (JSON array) can configure alternative filenames; the default is `["AGENTS.md", ".goosehints"]`.

(https://block.github.io/goose/docs/guides/context-engineering/using-goosehints)

### Example config.yaml with status hook

```yaml
# Model configuration
GOOSE_PROVIDER: "anthropic"
GOOSE_MODEL: "claude-4.5-sonnet"
GOOSE_TEMPERATURE: 0.7

# Status hook - shell command called with status arg
GOOSE_STATUS_HOOK: "/path/to/my-status-handler.sh"

# Tool execution mode
GOOSE_MODE: "smart_approve"

# Extensions
extensions:
  developer:
    bundled: true
    enabled: true
    name: developer
    timeout: 300
    type: builtin
```

The `GOOSE_STATUS_HOOK` value is a shell command string. It can also be set as an environment variable.

## Hook Events

### `GOOSE_STATUS_HOOK`

The only true "hook" in Goose. A shell command executed when the agent transitions between states.

**Description:** Fires when the CLI transitions between waiting for user input and processing. The configured command is spawned in a background thread with the status string as an argument.

**Event payload:**

The hook command receives a single positional argument: the status string.

```
<hook_command> <status>
```

| Status | Meaning | When fired |
|--------|---------|------------|
| `waiting` | Agent is idle, awaiting user input | Before the input prompt is shown in interactive mode |
| `thinking` | Agent is processing | After user input is accepted, before agent response processing begins |

On Unix, the command is executed via `sh -c "<hook> <status>"`. On Windows, via `cmd /C "<hook> <status>"`.

**Event response:**

- Exit code: **ignored**. The CLI discards the result.
- stdout: **suppressed** (redirected to null).
- stderr: **suppressed** (redirected to null).
- Effect on flow: **none**. This is entirely fire-and-forget. You cannot block, modify, or influence the agent loop.

**Configuration:**

Set in `~/.config/goose/config.yaml`:

```yaml
GOOSE_STATUS_HOOK: "/path/to/script.sh"
```

Or as an environment variable:

```bash
export GOOSE_STATUS_HOOK="/path/to/script.sh"
```

**Example handler:**

```bash
#!/bin/bash
# my-status-handler.sh
STATUS="$1"
case "$STATUS" in
  thinking)
    # Update a status indicator
    echo "$(date -Iseconds) THINKING" >> /tmp/goose-status.log
    ;;
  waiting)
    echo "$(date -Iseconds) WAITING" >> /tmp/goose-status.log
    ;;
esac
```

**Gotchas:**

1. The hook spawns on a separate thread, so there is no ordering guarantee relative to the agent's next action.
2. stdout and stderr are both piped to null. If you need to capture output, write to a file or external service inside the script.
3. The hook reads its command from `Config::global().get_param::<String>("GOOSE_STATUS_HOOK")`, meaning it respects the standard config priority (env var overrides config file).

(Source: https://github.com/block/goose/blob/main/crates/goose-cli/src/session/output.rs)

### Stream-JSON event: `message`

**Description:** Emitted when the agent produces a response message (assistant turn) or when a tool response is generated.

**Event payload:**

```json
{
  "type": "message",
  "message": {
    "role": "assistant",
    "content": [
      { "type": "text", "text": "..." },
      { "type": "tool_request", "id": "...", "tool_name": "...", "arguments": {} },
      { "type": "tool_response", "id": "...", "output": "..." }
    ]
  }
}
```

The `message` object is the full Goose `Message` struct (role + content array). Content items can include text, tool requests, tool responses, `action_required` prompts, and thinking/reasoning blocks.

**Event response:** None. The stream is outbound-only; there is no return channel.

**Gotchas:** Message events are only emitted in `stream-json` mode (`goose run --output-format stream-json`). In interactive `session` mode, messages are rendered to the terminal instead.

### Stream-JSON event: `notification`

**Description:** Emitted when an MCP extension or subagent sends a logging or progress notification.

**Event payload (log variant):**

```json
{
  "type": "notification",
  "extension_id": "developer",
  "log": {
    "message": "Formatted log message string"
  }
}
```

**Event payload (progress variant):**

```json
{
  "type": "notification",
  "extension_id": "developer",
  "progress": {
    "progress": 0.5,
    "total": 1.0,
    "message": "Processing files..."
  }
}
```

The `extension_id` identifies which MCP extension or subagent produced the notification. The variant (`log` or `progress`) is determined by the MCP notification type (`LoggingMessageNotification` or `ProgressNotification`).

**Event response:** None. Outbound-only.

**Gotchas:**

- Subagent tool request notifications (`type: "subagent_tool_request"` in the MCP data) are re-emitted as `notification` log events with a formatted message string rather than the raw structured payload. To get structured subagent tool call data, consume MCP notifications at the extension/client layer.
- Task execution notifications (`type: "task_execution"`) with subtypes `line_output`, `tasks_update`, and `tasks_complete` are similarly flattened into formatted log strings rather than preserving the structured JSON. See the "Known gotchas" section below.

### Stream-JSON event: `model_change`

**Description:** Emitted when the active model or operating mode changes (e.g., switching between lead and worker models, or planner transitions).

**Event payload:**

```json
{
  "type": "model_change",
  "model": "claude-4.5-sonnet",
  "mode": "lead"
}
```

**Event response:** None. Outbound-only. Informational.

### Stream-JSON event: `error`

**Description:** Emitted when an error occurs in the agent loop. After emitting this event, the CLI performs cleanup and eventually emits a `complete` event.

**Event payload:**

```json
{
  "type": "error",
  "error": "Error description string"
}
```

**Event response:** None. Outbound-only.

**Gotchas:** Error events are rendered to stderr in non-stream-json modes. In stream-json mode, they appear in the JSON stream. A `complete` event always follows.

### Stream-JSON event: `complete`

**Description:** Marks the end of a `goose run` execution. Always the final event in the stream.

**Event payload:**

```json
{
  "type": "complete",
  "total_tokens": 1250
}
```

The `total_tokens` field is `null` if token tracking is unavailable for the session.

**Event response:** None. Outbound-only.

### Batch JSON output (`--output-format json`)

Not a stream, but included for completeness. When using `goose run --output-format json`, the CLI outputs a single JSON object after the run completes:

```json
{
  "messages": [
    { "role": "user", "content": [...] },
    { "role": "assistant", "content": [...] }
  ],
  "metadata": {
    "total_tokens": 1250,
    "status": "completed"
  }
}
```

This mode is best suited for CI pipelines where you want the final result rather than real-time events.

## MCP Notification Payloads

Goose forwards MCP notifications from extensions and subagents into both the CLI display and the stream-json output. These appear as `notification` stream events.

### Subagent tool request notifications

Emitted when a subagent issues a tool call. The raw MCP notification data includes:

```json
{
  "type": "subagent_tool_request",
  "subagent_id": "subagent-123",
  "tool_call": {
    "name": "developer__shell",
    "arguments": { "command": "ls -la" }
  }
}
```

In stream-json mode, this is re-emitted as a `notification` log event with a formatted message string (not the raw structured data).

(Source: https://github.com/block/goose/blob/main/crates/goose/src/agents/subagent_handler.rs)

### Task execution notifications

Structured updates about subagent task execution. The raw MCP notification data includes a `type: "task_execution"` field and a `subtype`:

| Subtype | Fields | Purpose |
|---------|--------|---------|
| `line_output` | `task_id`, `output` | Real-time output from a running task |
| `tasks_update` | `stats`, `tasks` | Periodic status update for all tasks |
| `tasks_complete` | `stats`, `failed_tasks` | Final completion summary |

The `stats` object contains: `total`, `pending`, `running`, `completed`, `failed` counts. Each `tasks` entry contains: `id`, `status`, `duration_secs`, `current_output`, `task_type`, `task_name`, `task_metadata`, `error`, `result_data`.

In stream-json mode, these are converted to formatted log strings rather than emitting the raw structured event.

(Sources: https://github.com/block/goose/blob/main/crates/goose/src/agents/subagent_execution_tool/notification_events.rs, https://github.com/block/goose/blob/main/crates/goose-cli/src/session/task_execution_display/mod.rs)

### Progress and log notifications

Standard MCP `ProgressNotification` and `LoggingMessageNotification` events from extensions. These are emitted as `notification` stream events with the `progress` or `log` variant respectively. See the `notification` event payload above.

## Matcher System

Goose does **not** have a matcher system. There is no mechanism to selectively fire hooks based on tool names, event types, or patterns. The `GOOSE_STATUS_HOOK` fires on every status transition. The stream-json events are emitted unconditionally when the output format is set to `stream-json`.

If you need to filter events, do so in your consuming script or application by inspecting the `type` field of each JSON event.

## Comparison with Other Agentic CLIs

| Feature | Goose | Claude Code | Gemini CLI |
|---------|-------|-------------|------------|
| Pre-tool-use hooks | No | Yes | Yes |
| Post-tool-use hooks | No | Yes | Yes |
| Block/approve tool calls | No | Yes | Yes |
| Modify tool input | No | Yes | Yes |
| Status change notification | Yes (fire-and-forget) | Yes (via hooks) | Yes (via hooks) |
| Streaming event output | Yes (stream-json) | No | No |
| Batch JSON output | Yes (json) | No | No |
| Config-based hook definition | Partial (status hook only) | Yes (JSON) | Yes (JSON) |
| Matcher/filter system | No | Yes (regex) | Yes (regex) |
| Return channel | No | Yes (stdin/stdout JSON) | Yes (stdin/stdout JSON) |

## Known Gotchas and Workarounds

### 1. Task execution notifications are flattened in stream-json

**Problem:** The CLI formats `task_execution` notifications into human-readable log strings rather than emitting the raw structured JSON in stream-json mode.

**Workaround:** Consume MCP notifications at the extension or client layer if you need structured task updates. Otherwise, parse the formatted log output with regex or string matching.

### 2. `GOOSE_STATUS_HOOK` is fire-and-forget

**Problem:** The hook runs asynchronously with stdout/stderr suppressed and exit codes ignored. You cannot block, modify, or respond to agent actions.

**Workaround:** Use the stream-json event feed for richer, synchronous state observation. In the hook, write to a file, named pipe, or external service if you need state persistence or need to trigger downstream actions.

### 3. Event schemas are not versioned

**Problem:** The stream-json event set is defined in Rust code and may change between releases. Unknown fields or new event types can appear without notice.

**Workaround:** Implement tolerant parsing: use `#[serde(deny_unknown_fields)]`-free deserialization, ignore unknown event types, and treat the stream as best-effort telemetry. The `StreamEvent` enum uses `#[serde(tag = "type", rename_all = "snake_case")]`, so the `type` field is the discriminator.

### 4. Tool output may be filtered in CLI rendering

**Problem:** The CLI can suppress low-priority tool output depending on `GOOSE_CLI_MIN_PRIORITY` and debug settings. Content items with priority below the threshold or with no priority (in non-debug mode) are skipped.

**Workaround:** Run with `--debug` or set `GOOSE_CLI_MIN_PRIORITY=0` in config to see all tool output. Note that this only affects CLI rendering, not stream-json events.

### 5. No project-scoped hook configuration

**Problem:** Unlike Claude Code (`.claude/settings.json`) or Gemini CLI (`.gemini/settings.json`), Goose has no project-scoped config for hooks. The `GOOSE_STATUS_HOOK` is always user-global.

**Workaround:** Use environment variables per project (e.g., via direnv or shell scripts), or have the hook script inspect the current working directory to apply project-specific logic.

### 6. Stream-json mode only works with `goose run`

**Problem:** The `--output-format stream-json` flag is only available on the `goose run` command (non-interactive task execution). Interactive `goose session` does not support structured event output.

**Workaround:** For interactive monitoring, use the `GOOSE_STATUS_HOOK` for state transitions, or run `goose run` in headless mode for full event streaming.

### 7. No hook for tool approval or permission decisions

**Problem:** Goose has a `GOOSE_MODE` setting (`auto`, `approve`, `chat`, `smart_approve`) that controls tool approval, but there is no hook to programmatically approve or deny individual tool calls.

**Workaround:** Set `GOOSE_MODE` globally. For fine-grained tool permissions, configure the `permission.yaml` file. There is no runtime hook-based approach.

## Sources

- Goose homepage: https://block.github.io/goose/
- Configuration files guide: https://block.github.io/goose/docs/guides/config-files/
- CLI commands reference: https://block.github.io/goose/docs/guides/goose-cli-commands/
- Running tasks (output formats): https://block.github.io/goose/docs/guides/running-tasks/
- Using goosehints: https://block.github.io/goose/docs/guides/context-engineering/using-goosehints
- Using extensions: https://block.github.io/goose/docs/getting-started/using-extensions/
- GitHub repository: https://github.com/block/goose
- Source: `run_status_hook` implementation: https://github.com/block/goose/blob/main/crates/goose-cli/src/session/output.rs
- Source: `StreamEvent` enum and event emission: https://github.com/block/goose/blob/main/crates/goose-cli/src/session/mod.rs
- Source: `AgentEvent` enum: https://github.com/block/goose/blob/main/crates/goose/src/agents/agent.rs
- Source: Subagent notifications: https://github.com/block/goose/blob/main/crates/goose/src/agents/subagent_handler.rs
- Source: Task execution notifications: https://github.com/block/goose/blob/main/crates/goose/src/agents/subagent_execution_tool/notification_events.rs
