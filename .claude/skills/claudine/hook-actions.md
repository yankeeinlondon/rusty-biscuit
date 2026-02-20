# Hook Actions

Actions are the responses Claudine executes when an event fires during an agentic CLI session. Each event binding in `~/.claudine/config.json` specifies an ordered list of actions. Actions are executed sequentially in declaration order. Most actions are fire-and-forget, but `Call` can return a `HookResponse` to influence agent behavior on blocking events.

## Supported Actions

### `sound_effect`

Play an embedded sound effect from the `playa` library.

**Fields:**

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | `string` | (required) | Effect name from playa's 88 embedded effects |
| `volume` | `f32` | `1.0` | Playback volume (0.0 to 1.0) |
| `speed` | `f32` | `1.0` | Playback speed multiplier |

**Behavior:** Fire-and-forget (`tokio::spawn_blocking`). Playback runs on a blocking thread and does not delay subsequent actions. Warns if the effect name is unknown.

**Return payload:** None.

**Example:**
```json
{ "type": "sound_effect", "name": "success", "volume": 0.8, "speed": 1.2 }
```

---

### `speak`

Speak a message aloud using biscuit-speaks TTS. The message supports Handlebars-style template interpolation.

**Fields:**

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `message` | `string` | (required) | Template message with `{{variable}}` placeholders |

**Behavior:** Fire-and-forget (`tokio::spawn`). TTS playback is async and does not block the event pipeline. Empty messages after interpolation are silently skipped.

**Return payload:** None.

**Example:**
```json
{ "type": "speak", "message": "Tool {{tool_name}} completed on {{git.branch}}" }
```

---

### `log`

Write the event payload to a configured log target (file or HTTP server).

**Fields:**

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `target` | `LogTarget` | File (daily rotation) | Destination for log records |

**LogTarget variants:**

**`file`** -- Append JSONL records to a local file.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `path` | `string?` | `~/.claudine/logs/<date>.jsonl` | Explicit file path (supports `~` expansion) |
| `rotate_daily` | `bool` | `true` | Rotate log file by local day boundary |

**`server`** -- POST structured hook records to an HTTP endpoint.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `url` | `string` | (required) | Endpoint URL |
| `timeout_ms` | `u64` | `10000` | Request timeout in milliseconds |
| `headers` | `map?` | None | Optional additional HTTP headers |

**Behavior:** Synchronous for file targets (creates parent directories as needed). Non-fatal for server targets -- POST failures are logged as warnings but do not halt execution. When the target is a file with no explicit path, falls back to the global `settings.default_log_target` if configured.

**Return payload:** None.

**Example (file):**
```json
{ "type": "log" }
```

**Example (server):**
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

---

### `report`

Print event information to stdout, making it visible in the agent's output stream.

**Fields:**

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `handler` | `ReportHandler?` | None | Report output handler |

When `handler` is omitted, defaults to: `[EVENT] tool_name (provider)`.

**ReportHandler fields:**

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `format` | `ReportFormat` | (required) | Output format: `text`, `json`, or `compact` |
| `template` | `string?` | None | Custom template string (overrides format when set) |
| `include_metadata` | `bool` | `false` | Append full event metadata JSON after the template output |

**ReportFormat variants:**
- `text` -- `Event: {event}, Provider: {provider}, Tool: {tool_name}`
- `json` -- Full event metadata as a JSON object
- `compact` -- `[EVENT] tool_name`

**Behavior:** Synchronous. Writes to stdout via `println!`.

**Return payload:** None.

**Example:**
```json
{
  "type": "report",
  "handler": {
    "format": "compact",
    "template": "[TOOL] {{tool_name}}: executing on {{os.type}}"
  }
}
```

---

### `fire_and_forget`

Execute an external command asynchronously without waiting for it to complete or inspecting its output. Command and args support template interpolation.

**Fields:**

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `command` | `string` | (required) | Command name or path to executable |
| `args` | `string[]?` | None | Optional arguments |

**Behavior:** Fire-and-forget (`tokio::spawn`). The command is spawned as a child process. Failures are logged as warnings. The command must be findable on `PATH` (verified via `sniff::programs::find_program`).

**Return payload:** None.

**Example:**
```json
{ "type": "fire_and_forget", "command": "notify-send", "args": ["Claudine", "{{event}} fired"] }
```

---

### `call`

Execute an external command synchronously and map its output to a `HookResponse`. This is the only action type that can influence agent behavior on blocking events.

**Fields:**

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `command` | `string` | (required) | Command name or path to executable |
| `args` | `string[]?` | None | Optional arguments |
| `timeout_ms` | `u64?` | `60000` (60s) | Command timeout in milliseconds |
| `mapper` | `Mapper?` | `exit_code` | How to interpret command output |

**Mapper variants:**

| Mapper | Description |
|--------|-------------|
| `exit_code` | Exit code 0 = Allow, 2 = Deny, other = Allow. Stdout/stderr becomes `reason`. |
| `json_field` | Parse stdout as JSON, extract `field` (dot-separated path) as decision. Also reads `reason` field. |
| `json_object` | Parse stdout as a full `HookResponse` JSON object. Falls back to storing raw JSON if shape doesn't match. |
| `regex` | Match stdout against a regex with named capture groups: `decision`, `reason`, `context`. |

**Behavior:** Blocking. Waits for the command to complete (up to `timeout_ms`). Timeouts and failures are logged as warnings and produce no response. When multiple `Call` actions appear in a binding, the last non-`Continue` decision wins (a `Continue` response is replaced by any subsequent non-`Continue` decision).

**Return payload:** `HookResponse` with:

| Field | Type | Description |
|-------|------|-------------|
| `decision` | `HookDecision?` | `allow`, `deny`, `ask`, or `continue` |
| `reason` | `string?` | Human-readable reason for the decision |
| `updated_input` | `JSON?` | Modified tool input to substitute before execution |
| `additional_context` | `string?` | Context string to inject into the agent |
| `raw` | `JSON?` | Raw provider-specific response fields |

**Note:** `HookResponse` values are only communicated back to the agent when the event supports blocking (e.g., `before_tool`, `before_prompt`, `permission_request`). On non-blocking events, call responses are discarded with a debug log.

**Example:**
```json
{
  "type": "call",
  "command": "security-check",
  "args": ["--tool", "{{tool_name}}"],
  "timeout_ms": 4000,
  "mapper": { "type": "json_field", "field": "decision" }
}
```

## Context Variables

Context variables are available for you to use in all string parameters of the actions you configure.

### Template Variables

Use these in speak messages and report templates: `"Tool {{tool_name}} failed: {{error}}"`

| Variable | Description | Available For |
|----------|-------------|---------------|
| `{{provider}}` | Provider slug (claude, codex, gemini, etc.) | all events |
| `{{event}}` | Event name (tool_error, turn_complete, etc.) | all events |
| `{{timestamp}}` | RFC3339 timestamp | all events |
| `{{session_id}}` | Session ID | all events |
| `{{cwd}}` | Current working directory | all events |
| `{{tool_name}}` | Tool name (Bash, Read, etc.) | tool events |
| `{{error}}` | Error message | error events |
| `{{prompt}}` | User prompt text | before_prompt |
| `{{agent_type}}` | Agent/subagent type | subagent events |
| `{{notification_type}}` | Notification type | notification |

### Context Variables (auto-detected at runtime)

| Variable | Description |
|----------|-------------|
| **OS** | |
| `{{os.name}}` | OS name (macOS, Ubuntu, Windows) |
| `{{os.type}}` | OS type slug (macos, linux, windows) |
| `{{os.version}}` | OS version string |
| `{{os.hostname}}` | Machine hostname |
| **Hardware** | |
| `{{hardware.arch}}` | CPU architecture (aarch64, x86_64) |
| `{{hardware.cpu}}` | CPU model name |
| `{{hardware.cores}}` | Logical CPU core count |
| **Git** | |
| `{{git.branch}}` | Current git branch |
| `{{git.is_dirty}}` | Git dirty state (true/false) |
| `{{git.head_sha}}` | HEAD commit SHA |
| `{{git.head_message}}` | HEAD commit message |
| `{{git.remote}}` | Git remote name (usually origin) |
| `{{git.hosting}}` | Git hosting provider (github, gitlab, bitbucket) |
| `{{git.repo_name}}` | Repository name |
| `{{git.repo_org}}` | Organization/owner name |
| **Project** | |
| `{{project.language}}` | Primary project language |
| `{{project.is_monorepo}}` | Monorepo detection (true/false) |
| `{{project.monorepo_tool}}` | Monorepo tool (cargo_workspace, pnpm, nx) |

### Environment Variables

Shell environment variables are supported via `{{env.VAR_NAME}}` with optional defaults: `{{env.MY_VAR | "fallback"}}`.

### Example

```json
{
  "type": "speak",
  "message": "Tool {{tool_name}} failed on {{git.branch}}: {{error}}"
}
```
