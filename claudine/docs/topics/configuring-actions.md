# Configuring Actions

Actions are the responses Claudine executes when an event fires during an agentic CLI session. Each event binding in `~/.claudine/config.json` specifies an ordered list of actions that run sequentially. Most actions are fire-and-forget; only `call` can return a response to influence agent behavior on blocking events.

## Event Binding Structure

Actions live inside event bindings under a provider's `events` map:

```json
{
  "version": "1.0",
  "settings": { ... },
  "providers": {
    "claude": {
      "events": {
        "before_tool": {
          "enabled": true,
          "actions": [
            { "type": "log" },
            { "type": "sound_effect", "name": "alert", "volume": 0.5 }
          ],
          "matcher": null
        }
      }
    }
  }
}
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | `bool` | `true` | Toggle the entire binding on/off |
| `actions` | `HookAction[]` | `[]` | Ordered list of actions to execute |
| `matcher` | `string?` | `null` | Optional regex filter (matches `tool_name` for tool events, `notification_type` for notifications) |

When `matcher` is set, the binding only fires if the regex matches the relevant field. For example, `"matcher": "^Bash$"` restricts the binding to Bash tool invocations.

## Action Types

Every action is a JSON object with a `"type"` discriminator. Claudine supports six action types.

### `sound_effect`

Play an embedded sound from the playa library. Fire-and-forget -- playback runs on a blocking thread and does not delay subsequent actions.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | `string` | (required) | Effect name from playa's 88 embedded effects |
| `volume` | `f32` | `1.0` | Playback volume (0.0 to 1.0) |
| `speed` | `f32` | `1.0` | Playback speed multiplier |

```json
{ "type": "sound_effect", "name": "success", "volume": 0.8, "speed": 1.2 }
```

### `speak`

Speak a message aloud using biscuit-speaks TTS. Fire-and-forget -- TTS is async and does not block the pipeline. Empty messages after template interpolation are silently skipped.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `message` | `string` | (required) | Template message with `{{variable}}` placeholders |

```json
{ "type": "speak", "message": "Tool {{tool_name}} completed on {{git.branch}}" }
```

### `log`

Write the event payload to a log target. When no target is specified, defaults to a daily-rotated JSONL file under `~/.claudine/logs/`.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `target` | `LogTarget?` | file (daily rotation) | Destination for log records |

**File target** (default):

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `path` | `string?` | `~/.claudine/logs/<date>.jsonl` | Explicit file path (supports `~`) |
| `rotate_daily` | `bool` | `true` | Rotate by local day boundary |

**Server target**:

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `url` | `string` | (required) | HTTP endpoint URL |
| `timeout_ms` | `u64` | `10000` | Request timeout in milliseconds |
| `headers` | `map?` | `null` | Additional HTTP headers |

```json
{ "type": "log" }
```

```json
{
  "type": "log",
  "target": {
    "type": "server",
    "url": "https://hooks.example.com/events",
    "timeout_ms": 5000,
    "headers": { "Authorization": "Bearer tok_xxx" }
  }
}
```

### `report`

Print event information to stdout, making it visible in the agent's output stream. When `handler` is omitted, defaults to `[EVENT] tool_name (provider)`.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `handler` | `ReportHandler?` | `null` | Output handler configuration |

**ReportHandler fields**:

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `format` | `ReportFormat` | (required) | Output format: `text`, `json`, or `compact` |
| `template` | `string?` | `null` | Custom template (overrides format when set) |
| `include_metadata` | `bool` | `false` | Append full event metadata JSON after template output |

**Format variants**:

| Format | Output |
|--------|--------|
| `text` | `Event: {event}, Provider: {provider}, Tool: {tool_name}` |
| `json` | Full event metadata as a JSON object |
| `compact` | `[EVENT] tool_name` |

```json
{ "type": "report" }
```

```json
{
  "type": "report",
  "handler": {
    "format": "compact",
    "template": "[TOOL] {{tool_name}}: executing on {{os.type}}"
  }
}
```

### `fire_and_forget`

Spawn an external command asynchronously without waiting for completion or inspecting output. Command and args support template interpolation. The command must be findable on `PATH`.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `command` | `string` | (required) | Command name or path to executable |
| `args` | `string[]?` | `null` | Optional arguments |

```json
{ "type": "fire_and_forget", "command": "notify-send", "args": ["Claudine", "{{event}} fired"] }
```

### `call`

Execute an external command synchronously and map its output to a `HookResponse`. This is the only action type that can influence agent behavior on blocking events (e.g., `before_tool`, `before_prompt`, `permission_request`).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `command` | `string` | (required) | Command name or path to executable |
| `args` | `string[]?` | `null` | Optional arguments |
| `timeout_ms` | `u64` | `60000` | Command timeout in milliseconds |
| `mapper` | `Mapper?` | `exit_code` | How to interpret command output |

**Mapper variants**:

| Type | Config | Behavior |
|------|--------|----------|
| `exit_code` | -- | Exit 0 = Allow, 2 = Deny, other = Allow. Stdout/stderr becomes `reason`. |
| `json_field` | `{ "field": "decision" }` | Parse stdout as JSON, extract a dot-separated path as the decision. Also reads `reason`. |
| `json_object` | -- | Parse stdout as a full `HookResponse` JSON object. |
| `regex` | `{ "pattern": "..." }` | Match stdout with named capture groups: `decision`, `reason`, `context`. |

**HookResponse fields** (returned to the agent on blocking events):

| Field | Type | Description |
|-------|------|-------------|
| `decision` | `HookDecision?` | `allow`, `deny`, `ask`, or `continue` |
| `reason` | `string?` | Human-readable reason for the decision |
| `updated_input` | `JSON?` | Modified tool input to substitute before execution |
| `additional_context` | `string?` | Context string to inject into the agent |
| `raw` | `JSON?` | Raw provider-specific response fields |

When multiple `call` actions appear in a binding, the last non-`continue` decision wins.

```json
{
  "type": "call",
  "command": "security-check",
  "args": ["--tool", "{{tool_name}}"],
  "timeout_ms": 4000,
  "mapper": { "type": "exit_code" }
}
```

```json
{
  "type": "call",
  "command": "policy-engine",
  "args": ["--event", "{{event}}"],
  "mapper": { "type": "json_field", "field": "result.decision" }
}
```

## Template Variables

All string fields in actions (`message`, `template`, `command`, `args`) support Handlebars-style interpolation.

### Event Variables

| Variable | Description | Available For |
|----------|-------------|---------------|
| `{{provider}}` | Provider slug (claude, codex, gemini, etc.) | all events |
| `{{event}}` | Event name (tool_error, turn_complete, etc.) | all events |
| `{{timestamp}}` | RFC 3339 timestamp | all events |
| `{{session_id}}` | Session or thread identifier | all events |
| `{{cwd}}` | Current working directory | all events |
| `{{tool_name}}` | Tool name (Bash, Read, etc.) | tool events |
| `{{error}}` | Error message | error events |
| `{{prompt}}` | User prompt text | before_prompt |
| `{{agent_type}}` | Agent/subagent type | subagent events |
| `{{notification_type}}` | Notification type | notification |

### Context Variables (auto-detected at runtime)

| Variable | Description |
|----------|-------------|
| `{{os.name}}` | OS name (macOS, Ubuntu, Windows) |
| `{{os.type}}` | OS type slug (macos, linux, windows) |
| `{{os.version}}` | OS version string |
| `{{os.hostname}}` | Machine hostname |
| `{{hardware.arch}}` | CPU architecture (aarch64, x86_64) |
| `{{hardware.cpu}}` | CPU model name |
| `{{hardware.cores}}` | Logical CPU core count |
| `{{git.branch}}` | Current git branch |
| `{{git.is_dirty}}` | Git dirty state (true/false) |
| `{{git.head_sha}}` | HEAD commit SHA |
| `{{git.head_message}}` | HEAD commit message |
| `{{git.remote}}` | Git remote name (usually origin) |
| `{{git.hosting}}` | Git hosting provider (github, gitlab, bitbucket) |
| `{{git.repo_name}}` | Repository name |
| `{{git.repo_org}}` | Organization/owner name |
| `{{project.language}}` | Primary project language |
| `{{project.is_monorepo}}` | Monorepo detection (true/false) |
| `{{project.monorepo_tool}}` | Monorepo tool (cargo_workspace, pnpm, nx) |

### Environment Variables

Shell environment variables are available via `{{env.VAR_NAME}}` with optional defaults:

```
{{env.SLACK_WEBHOOK}}
{{env.MY_VAR | "fallback_value"}}
```

## Full Example

A complete event binding with multiple actions:

```json
{
  "providers": {
    "claude": {
      "events": {
        "before_tool": {
          "enabled": true,
          "actions": [
            { "type": "log" },
            {
              "type": "call",
              "command": "security-check",
              "args": ["--tool", "{{tool_name}}", "--branch", "{{git.branch}}"],
              "timeout_ms": 5000,
              "mapper": { "type": "exit_code" }
            },
            {
              "type": "report",
              "handler": { "format": "compact" }
            }
          ],
          "matcher": "^Bash$"
        },
        "turn_complete": {
          "enabled": true,
          "actions": [
            { "type": "log" },
            { "type": "sound_effect", "name": "success" },
            { "type": "speak", "message": "Turn complete on {{git.branch}}" }
          ]
        },
        "tool_error": {
          "enabled": true,
          "actions": [
            { "type": "log" },
            { "type": "sound_effect", "name": "error", "volume": 0.6 },
            { "type": "speak", "message": "{{tool_name}} failed: {{error}}" }
          ]
        }
      }
    }
  }
}
```

This configuration logs all Bash tool invocations, runs a security check that can block execution, and reports the result. Turn completions get a sound and TTS announcement. Tool errors are logged with an alert sound and spoken error summary.
