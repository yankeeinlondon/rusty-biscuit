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

Every action is a JSON object with a `"type"` discriminator. Claudine supports seven action types.

### `sound_effect`

Play an embedded sound from the Playa library. Claudine durably publishes the
job to Playa's private per-user spool and returns without waiting for playback.
The global scheduler serializes it with lifecycle, speech, and other Playa jobs.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | `string` | (required) | Effect name from playa's 88 embedded effects |
| `volume` | `f32` | `1.0` | Playback volume (0.0 to 1.0) |
| `speed` | `f32` | `1.0` | Playback speed multiplier |

```json
{ "type": "sound_effect", "name": "success", "volume": 0.8, "speed": 1.2 }
```

### `speak`

Speak a message aloud using biscuit-speaks TTS. Claudine reserves or publishes
an ordered detached job and returns without waiting for synthesis or playback.
Cache misses keep their sequence while a private helper prepares them. Empty
messages after template interpolation are silently skipped.

Both audio actions are native-first where available and best-effort after
durable handoff. A handoff failure logs one warning; Claudine does not attempt a
blocking fallback or start a second scheduler. Playa's journal/status views
redact private preparation text.

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

### `message`

Send a message to the configured messaging destination (Slack, Discord, Signal, WhatsApp, or webhooks). Fire-and-forget -- delivery is async and does not block the pipeline. Empty messages after template interpolation are silently skipped.

Requires messaging configuration in `settings.messaging` (see [Messaging Configuration](#messaging-configuration) below).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `message` | `string` | (required) | Template message with `{{variable}}` placeholders (rendered as Markdown where supported) |
| `image` | `string?` | `null` | File path to a raster image attachment (Discord only in v1; ignored with a warning for other providers) |

```json
{ "type": "message", "message": "**{{provider}}** `{{event}}` in `{{cwd}}`" }
```

```json
{
  "type": "message",
  "message": "Build artifact ready",
  "image": "{{cwd}}/.claudine/artifacts/last-run.png"
}
```

Image path resolution: absolute paths stay absolute, `~/` expands to the home directory, relative paths resolve from `{{cwd}}` then the repo root.

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

## Conditional Execution (`when`)

Every action supports an optional `when` field that controls whether the action runs for a given event. The `when` value is a Darkmatter condition expression evaluated against the live event payload. If the condition evaluates to truthy, the action runs; if falsy or invalid, the action is skipped non-fatally and the rest of the binding continues.

### Available Paths

The following paths can be referenced in `when` expressions:

| Path Prefix | Description |
|-------------|-------------|
| `env.*` | Shell environment variables (e.g. `env.CI`) |
| `extra.*` | Arbitrary extra data attached to the event |
| `tool_input.*` | Nested fields from the tool input JSON |
| `tool_response.*` | Nested fields from the tool response JSON |
| `os.*` | Operating system context (`os.name`, `os.type`, `os.version`, `os.hostname`) |
| `hardware.*` | Hardware context (`hardware.arch`, `hardware.cpu`, `hardware.cores`) |
| `git.*` | Git context (`git.branch`, `git.is_dirty`, `git.head_sha`, etc.) |
| `project.*` | Project context (`project.language`, `project.is_monorepo`, `project.monorepo_standard`, `project.monorepo_orchestrators`, `project.monorepo_tool`) |
| `ctx.*` | Auto-captured runtime context (`ctx.today`, `ctx.year`, etc.) |
| Top-level fields | `provider`, `event`, `timestamp`, `session_id`, `cwd`, `tool_name`, `error`, `prompt`, `agent_type`, `notification_type`, `notification_message` |

### Examples

Only speak when on the `main` branch:

```json
{
  "type": "speak",
  "message": "Deploying to production",
  "when": "git.branch == 'main'"
}
```

Only call a policy engine when `ctx.today` is available:

```json
{
  "type": "call",
  "command": "policy-engine",
  "when": "ctx.today != ''"
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
| `{{project.monorepo_standard}}` | Monorepo authority standard (cargo-workspace, pnpm-workspaces, etc.) |
| `{{project.monorepo_orchestrators}}` | Orchestrators on the primary monorepo layer (nx, turborepo, lerna) |
| `{{project.monorepo_tool}}` | Deprecated alias for `{{project.monorepo_standard}}` |

### Environment Variables

Shell environment variables are available via `{{env.VAR_NAME}}` with optional defaults:

```
{{env.SLACK_WEBHOOK}}
{{env.MY_VAR || "fallback_value"}}
```

The single-pipe `|` form is no longer supported (see the migration note in unified-events.md §3.7).

## Messaging Configuration

The `message` action requires a destination configured under `settings.messaging` in your `config.json`. Configuration supports both user scope (`~/.claudine/config.json`) and repo scope (`.claudine/config.json`).

### Named Config Pattern

Multiple provider configurations are stored under a `configs` map keyed by user-chosen names. An `active` field selects which named config is currently in use. Setting `active` to `null` or omitting it disables messaging for that scope.

Scope resolution: repo-scope `active` overrides user-scope when present. If repo scope sets `active` to `null`, the user-scope active config is used as fallback.

### Provider Configs

Secrets can be provided inline (e.g., `bot_token`) or via an environment variable name (e.g., `bot_token_env`). Inline values take precedence. Default env var names follow the `messenger` CLI conventions.

**Discord:**

```json
{
  "provider": "discord",
  "channel_id": "123456789012345678",
  "bot_token_env": "DISCORD_BOT_TOKEN"
}
```

**Slack:**

```json
{
  "provider": "slack",
  "channel_id": "C012345ABC",
  "bot_token_env": "SLACK_BOT_TOKEN"
}
```

**Discord Webhook:**

```json
{
  "provider": "discord_webhook",
  "webhook_url": "https://discord.com/api/webhooks/123456789/abcdef...",
  "webhook_url_env": "DISCORD_WEBHOOK_URL"
}
```

**Slack Webhook:**

```json
{
  "provider": "slack_webhook",
  "webhook_url": "https://hooks.slack.com/services/T00/B00/XXXX",
  "webhook_url_env": "SLACK_WEBHOOK_URL"
}
```

**Signal:**

```json
{
  "provider": "signal",
  "recipient": "+15551234567",
  "rpc_url_env": "SIGNAL_RPC_URL",
  "account_env": "SIGNAL_ACCOUNT"
}
```

Recipients starting with `+` are treated as phone numbers; other values are treated as Signal group IDs.

**WhatsApp:**

```json
{
  "provider": "whatsapp",
  "recipient": "+15559876543",
  "access_token_env": "WHATSAPP_ACCESS_TOKEN",
  "phone_number_id_env": "WHATSAPP_PHONE_NUMBER_ID"
}
```

### Env-Only Webhook Configuration

Webhook routes can omit the inline `webhook_url` and rely solely on an environment variable:

```json
{
  "provider": "slack_webhook",
  "webhook_url_env": "DEPLOY_SLACK_WEBHOOK_URL"
}
```

This is useful for shared repo configs where the webhook secret should not be committed.

### Desktop Notifications

Desktop notifications are **zero-config** and are not managed through `claudine config`. They are triggered via the `notify` field in composition lifecycle frontmatter only. See [Lifecycle Notifications](lifecycle.md).

### Example Settings Block

```json
{
  "settings": {
    "messaging": {
      "active": "work-slack",
      "configs": {
        "work-slack": {
          "provider": "slack",
          "channel_id": "C012345ABC",
          "bot_token_env": "SLACK_BOT_TOKEN"
        },
        "personal-discord": {
          "provider": "discord",
          "channel_id": "123456789012345678",
          "bot_token_env": "DISCORD_BOT_TOKEN"
        },
        "deploy-webhook": {
          "provider": "slack_webhook",
          "webhook_url_env": "DEPLOY_SLACK_WEBHOOK_URL"
        },
        "alerts-webhook": {
          "provider": "discord_webhook",
          "webhook_url_env": "DISCORD_WEBHOOK_URL"
        }
      }
    }
  }
}
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
            { "type": "speak", "message": "Turn complete on {{git.branch}}" },
            { "type": "message", "message": "Turn complete on **{{git.branch}}**" }
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
