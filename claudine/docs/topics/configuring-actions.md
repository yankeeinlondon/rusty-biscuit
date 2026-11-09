---
kind: canonical-documentation
---

# Configuring Actions

Actions are the responses Claudine executes when an event fires during an agentic CLI session. Each event binding in `~/.claudine/config.json` specifies an ordered list of actions that run sequentially. Most actions do not block the pipeline; only `call` can return a response to influence agent behavior on blocking events.

## Event Binding Structure

Actions live under the top-level `actions` map, keyed by canonical event name. Each value is the ordered list of actions for that event, and the same bindings apply to every provider:

```json
{
  "actions": {
    "before_tool": [
      { "type": "report" },
      { "type": "sound_effect", "effect": "space-alarm", "volume": 0.5 }
    ]
  },
  "matchers": {
    "before_tool": "^Bash$"
  }
}
```

| Key | Type | Default | Description |
|-------|------|---------|-------------|
| `actions.<event>` | `HookAction[]` | absent | Ordered list of actions to execute when `<event>` fires |
| `matchers.<event>` | `string` | absent | Optional filter for that event's actions (a regex over `tool_name` for tool events and `notification_type` for notifications, or a Darkmatter condition) |

An event with no `actions` entry runs no actions. A file that still uses the old `version` + `providers.<provider>.events` shape is treated as the old format: Claudine renames it to `config.json.bak` and treats the configuration as missing, so setup runs again.

Every dispatched event is also appended to a daily JSONL log under `~/.claudine/logs/` while the top-level `logging` setting is `true` (the default); this is independent of the actions bound to the event.

When a matcher is set, the event's actions run only if the matcher matches. For example, `"before_tool": "^Bash$"` restricts the `before_tool` actions to Bash tool invocations.

A matcher can also be a Darkmatter condition over the same paths as [`when`](#available-paths), such as `"before_tool": "hook.tool_name == 'Bash' && git.branch == 'main'"`. Claudine tries the string as a condition first; a bare word such as `Bash`, or a string that is not a valid condition, is compiled as a regex.

## Action Types

Every action is a JSON object with a `"type"` discriminator. Claudine supports six action types; an unknown `type`, or an unknown field on an action, fails to load.

### `sound_effect`

Play an embedded sound from the Playa library. Claudine durably publishes the
job to Playa's private per-user spool and returns without waiting for playback.
The global scheduler serializes it with lifecycle, speech, and other Playa jobs.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `effect` | `string` | (required) | Effect name from playa's 88 embedded effects |
| `volume` | `f32` | `1.0` | Playback volume (0.0 to 1.0) |
| `speed` | `f32` | `1.0` | Playback speed multiplier |

```json
{ "type": "sound_effect", "effect": "confirmation", "volume": 0.8, "speed": 1.2 }
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
| `voice` | `string?` | `null` | Voice override for this action |
| `gender` | `male` \| `female` | `null` | Gender preference for voice selection |

```json
{ "type": "speak", "message": "Tool {{tool_name}} completed on {{git.branch}}" }
```

The provider and default voice come from the [`tts` setting](#text-to-speech-settings-tts);
`speak` actions run only when `tts` is `true` or an object.

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

### `bash`

Run an external command without inspecting its output. The command is spawned directly with an argument list, not through a shell, so shell syntax such as pipes or `&&` is not interpreted. Claudine waits up to 3 seconds for it; a non-zero exit, a spawn failure, or a timeout is logged as a warning and the rest of the binding continues. A `bash` action cannot influence agent behavior; use [`call`](#call) for that.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `command` | `string` | (required) | An absolute path to an existing executable, or a command name found on `PATH`. A `.js`/`.ts`/`.mjs`/`.mts` script runs through its shebang interpreter, else `bun`, else `node` (`.js`/`.mjs` only) |
| `params` | `string` | `""` | Arguments as one string, split into separate arguments with shell-style quoting after interpolation |

Both fields support template interpolation. Quote a placeholder whose value may contain spaces (`--message '{{tool_name}}'`); otherwise the value splits into several arguments.

```json
{ "type": "bash", "command": "notify-send", "params": "Claudine '{{event}} fired'" }
```

### `message`

Send a message to the configured messaging destination (Slack, Discord, Signal, WhatsApp, or webhooks). Delivery runs in the background and does not block the pipeline, but the CLI waits for it before exiting, for up to 10 seconds (less under `claudine handle`'s deadline). A send still running then is reported with a warning that its delivery is unknown, and the exit code does not change (see [Messaging](messaging.md#the-cli-drains-before-every-ordinary-exit)). Empty messages after template interpolation are silently skipped.

Requires messaging configuration under the top-level `messenger` key (see [Messaging Configuration](#messaging-configuration) below).

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
| `hook` / `hook.*` | The whole normalized event payload / one field of it (see [The `hook` root](#the-hook-root)) |
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
  "when": "!is_empty(ctx.today)"
}
```

### The `hook` root

`hook` names the event payload explicitly. Bare `hook` is the whole normalized
payload (its event fields, `extra`, and the `os`, `hardware`, `git`, and
`project` groups), and `hook.<path>` is one field of it. The unprefixed paths
above read the same fields, so `tool_name` and `hook.tool_name` agree.

```json
{ "type": "speak", "message": "{{ hook.tool_name }} finished", "when": "hook.tool_name == 'Bash'" }
```

- A missing `hook.<path>` is absent (`null`); it never falls back to an
  unprefixed field or another root.
- `hook` does not contain the environment: `hook.env.HOME` is absent. Read
  environment variables through `env.*`.
- `hook.extra.doc` reaches an `extra` field named `doc`.
- `hook.<path>` reads that one field straight from the event and copies only
  its value, so `hook.tool_name` stays cheap beside a megabyte of
  `tool_response`. Only bare `hook` builds the whole payload object. Every
  evaluation reads the event being dispatched; nothing is kept from an
  earlier event.
- `hook` exists only in hook actions. A hook expression has no frontmatter, so
  Darkmatter's document root `fm` does not apply here.

#### Migrating from the `doc` root

The payload root was previously named `doc`. It was renamed with no alias, so
`doc` gets no special treatment in a hook expression:

| Written as | Before | Now |
| --- | --- | --- |
| `when: "doc.tool_name == 'Bash'"` | compares the tool name | `doc.tool_name` is absent, so the action never runs |
| `{{ doc.tool_name }}` in a template | the tool name | left in the output as literal text |
| `{{ hook.tool_name }}` | left as literal text | the tool name |
| `{{ hook }}` | left as literal text | the whole payload as JSON |

Replace `doc.` with `hook.` (or drop the prefix) in every `when`, `matcher`,
and template string. Claudine does not rewrite configuration automatically.

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
| `{{hook}}` / `{{hook.<path>}}` | The whole payload as JSON / one field of it (see [The `hook` root](#the-hook-root)) | all events |

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

## Text-to-speech settings (`tts`)

The top-level `tts` key in `~/.claudine/config.json` (JSON5 accepted) chooses
how Claudine speaks. It is read in three places: hook [`speak`](#speak)
actions, the voice that composition lifecycle `say`/`speak` plays with, and
[pre-warm](flow-control/lifecycle.md#speech-is-prepared-ahead-of-time-pre-warm),
which synthesizes lifecycle phrases into the audio cache before their events
fire. It is a user-scope setting only; it is never a frontmatter key.

`tts` is either a boolean or an object:

```json5
{ tts: true }                                                      // hook speech on, default provider
{ tts: false }                                                     // hook speech off (the default)
{ tts: { provider: "kokoro", voice: "af_heart" } }                 // hook speech on, this provider and voice
{ tts: { provider: "kokoro", voice: "af_heart", prewarm: false } } // same, without pre-warm
{ tts: { prewarm: false } }                                        // default provider, without pre-warm
```

Every key of the object is optional:

| Key | Type | Default | Meaning |
| --- | --- | --- | --- |
| `provider` | string | the default provider | the TTS provider (`say`, `kokoro`, `espeak`, `elevenlabs`, …); absent selects the default provider, as `true` does |
| `voice` | string or `{ male, female }` | absent | a voice ID, or one voice per gender |
| `gender` | `male` \| `female` | `female` | which voice of a `{ male, female }` pair is used |
| `prewarm` | boolean | `true` | `false` turns pre-warm off |

What each shape does:

| Value | Hook `speak` actions | Lifecycle `say` voice | Pre-warm |
| --- | --- | --- | --- |
| `tts` absent | off | auto-detected | on |
| `true` | on, auto-detected provider | auto-detected | on |
| `false` | off | auto-detected | on |
| object with `provider`, no `prewarm` | on, with the object's provider and voice | the object's provider and voice | on |
| object with `provider` and `prewarm: false` | as the same object without `prewarm` | as the same object without `prewarm` | off |
| object without `provider` (for example `{ prewarm: false }`) | on, with the default provider, as `true` | as `true` | per its `prewarm` (on when absent) |

Lifecycle `say` plays under every shape: `false` silences hook speech only, so
it leaves lifecycle speech and its pre-warming on. `prewarm` exists only in the
object form, so turning pre-warm off always writes an object, and an object
also turns hook `speak` actions on. A user with `tts: false` who wants
pre-warm off therefore gets hook speech with the default provider as well.

The value is validated strictly when `config.json` loads:

- an unknown key in the object (for example `enabled` or `pre_warm`) is
  rejected;
- an explicit `null` for `tts`, `provider`, or `prewarm` is rejected, not read
  as absent;
- a wrong type (`tts: "say"`, `provider: 1`, `prewarm: "no"`) is rejected;
- `{}` loads as an object with every key at its default;
- a key written twice keeps its last value, as every key in `config.json` does.

The configuration editor's TTS tab keeps `prewarm` when you change the
object's provider, voice, or gender, and when it saves. Switching the tab's
toggle to a boolean replaces the object, which restores pre-warm's default of
on.

## Messaging Configuration

The `message` action requires a destination configured under the top-level `messenger` key in your user config (`~/.claudine/config.json`). A repo config (`.claudine/config.json`) can only choose which named configuration is active, through `active_messenger`.

### Named Config Pattern

Multiple provider configurations are stored under a `configurations` map keyed by user-chosen names. An `active_config` field selects which named configuration is currently in use; it must name a key in `configurations`. Omitting `active_config` disables messaging.

Scope resolution: a repo-scope `active_messenger` string replaces the user-scope `active_config` for that repo. A repo-scope `"active_messenger": null` disables messaging for that repo. When the repo config omits `active_messenger`, the user-scope `active_config` applies.

### Provider Configs

Secrets are read from the environment variables named by the `*_env` fields (e.g., `bot_token_env`); each field has a default that follows the `messenger` CLI conventions. Only the webhook routes also accept an inline secret, `webhook_url`, which takes precedence over `webhook_url_env`.

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

This is useful when the config file is shared or backed up and the webhook secret should not be stored in it.

### Desktop Notifications

Desktop notifications are **zero-config** and are not managed through `claudine config`. They are triggered via the `notify` field in composition lifecycle frontmatter only. See [Lifecycle Notifications](flow-control/lifecycle.md).

### Example Messenger Block

```json
{
  "messenger": {
    "active_config": "work-slack",
    "configurations": {
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
```

## Full Example

A complete set of event bindings with multiple actions:

```json
{
  "actions": {
    "before_tool": [
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
    "turn_complete": [
      { "type": "sound_effect", "effect": "confirmation" },
      { "type": "speak", "message": "Turn complete on {{git.branch}}" },
      { "type": "message", "message": "Turn complete on **{{git.branch}}**" }
    ],
    "tool_error": [
      { "type": "sound_effect", "effect": "error-1", "volume": 0.6 },
      { "type": "speak", "message": "{{tool_name}} failed: {{error}}" }
    ]
  },
  "matchers": {
    "before_tool": "^Bash$"
  }
}
```

This configuration runs a security check that can block each Bash tool invocation, then reports it. Turn completions get a sound, a TTS announcement, and a message. Tool errors get an alert sound and a spoken error summary. Every event is also written to the JSONL event log, because `logging` defaults to `true`.
