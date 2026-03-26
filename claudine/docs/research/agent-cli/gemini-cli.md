---
homepage: https://github.com/google-gemini/gemini-cli
docs: https://geminicli.com/docs/
cli_docs: https://geminicli.com/docs/cli/cli-reference
---

# Gemini CLI

Google's open-source AI agent that brings Gemini models directly into the terminal. Installed
via npm (`@google/gemini-cli`), it provides interactive REPL, headless/non-interactive
mode, MCP server management, extensions, agent skills, hooks, and session management.

Configuration lives in `~/.gemini/settings.json` (user-level) and `.gemini/settings.json`
(workspace-level). Project context is provided via `GEMINI.md` files at global
(`~/.gemini/GEMINI.md`), workspace, and just-in-time discovery levels.

## Model Specification

### CLI Flag

Use `-m` / `--model` to specify a model:

```bash
gemini -m gemini-2.5-pro "explain this codebase"
gemini -m flash "quick summary"
```

### Model Aliases

| Alias        | Resolves To                                        | Purpose                          |
| ------------ | -------------------------------------------------- | -------------------------------- |
| `auto`       | `gemini-2.5-pro` or `gemini-3-pro-preview`         | Default; uses preview if enabled |
| `pro`        | `gemini-2.5-pro` or `gemini-3-pro-preview`         | Complex reasoning tasks          |
| `flash`      | `gemini-2.5-flash`                                 | Fast, balanced for most tasks    |
| `flash-lite` | `gemini-2.5-flash-lite`                            | Fastest for simple tasks         |

### Model Selection Precedence

1. `--model` CLI flag (highest priority)
2. `GEMINI_MODEL` environment variable
3. `model.name` in `settings.json`
4. Default: `auto`

### Auto Routing

When set to `auto`, the CLI automatically selects the best model based on task complexity.
Gemini 3 models are available when enabled via `/settings` > Preview Features.

### Custom Aliases and Overrides

Advanced model configuration is available via `modelConfigs` in `settings.json`:

```json
{
  "modelConfigs": {
    "customAliases": {
      "precise-mode": {
        "extends": "chat-base",
        "modelConfig": {
          "generateContentConfig": {
            "temperature": 0.0,
            "topP": 1.0
          }
        }
      }
    }
  }
}
```

### Limitations

- The `/model` command and `--model` flag do NOT override the model used by sub-agents.
  Other models may appear in usage reports despite manual selection.
- Model routing fallback (e.g. quota exhaustion) may silently switch internal utility calls
  through the chain: `flash-lite` -> `flash` -> `pro`.

## Non-interactive Engagement

Non-interactive (headless) mode is supported and activates automatically when the CLI runs
in a non-TTY environment or when a query is provided as a positional argument.

### Method 1: Positional Argument

```bash
gemini "explain this project"
```

Runs the prompt, outputs the response, and exits. This is the **recommended** approach.

### Method 2: `-p` / `--prompt` Flag

```bash
gemini -p "explain this project"
```

Functionally identical to the positional argument. The documentation marks `-p` as
**deprecated** in favor of positional arguments. When both `-p` and stdin are provided,
the `-p` text is appended to stdin content.

### Method 3: Stdin Piping

```bash
cat logs.txt | gemini "summarize these logs"
cat error.log | gemini
```

Pipe content to the CLI. Combine with a positional argument for additional instructions.

### Method 4: `-i` / `--prompt-interactive`

```bash
gemini -i "What is the purpose of this project?"
```

Executes the prompt and then **continues** in interactive mode rather than exiting. Useful
for an initial task that leads into a conversation.

### Output Formats

Control non-interactive output with `--output-format` / `-o`:

| Format        | Description                                              |
| ------------- | -------------------------------------------------------- |
| `text`        | Plain text response (default)                            |
| `json`        | Single JSON object with `response`, `stats`, and `error` |
| `stream-json` | Newline-delimited JSON events (JSONL)                    |

Stream-JSON event types: `init`, `message`, `tool_use`, `tool_result`, `error`, `result`.

### Exit Codes

| Code | Meaning                            |
| ---- | ---------------------------------- |
| 0    | Success                            |
| 1    | General error or API failure       |
| 42   | Input error (invalid prompt/args)  |
| 53   | Turn limit exceeded                |

## Subscription versus Per Call API

Gemini CLI supports multiple authentication methods, each with different pricing:

### Free Tiers

| Auth Method                   | Limits                                    | Model Access   |
| ----------------------------- | ----------------------------------------- | -------------- |
| Google Login (OAuth)          | 1,000 requests/day, 60 requests/min       | Full model family |
| Gemini API Key (unpaid)       | 250 requests/day, 10 requests/min         | Flash only     |
| Vertex AI Express Mode        | 90-day trial before billing required       | Varies         |

### Paid Options

| Auth Method                        | Pricing Model        | Notes                                            |
| ---------------------------------- | -------------------- | ------------------------------------------------ |
| Google AI Pro/Ultra                | Fixed subscription   | Recommended for individual developers            |
| Google Cloud Code Assist Standard  | Per-seat license     | 1,500 requests/day, 120/min                      |
| Google Cloud Code Assist Enterprise| Per-seat license     | 2,000 requests/day, 120/min                      |
| Vertex AI (regular)                | Pay-per-token        | Dynamic shared quota                             |
| Gemini API Key (paid tier)         | Pay-per-token        | Per-token pricing varies by model                |

The documentation warns that pay-per-token pricing "can be more expensive for many small
calls with few tokens," making subscription plans preferable for frequent use.

How to start non-interactive sessions under each pricing model is determined entirely by
the authentication method configured (Google Login, API key, or Vertex AI), not by any
CLI flag. The non-interactive invocation is the same regardless of pricing tier.

## System Prompt

Gemini CLI distinguishes between two types of instructional files:

### GEMINI.md (Strategy / Context)

GEMINI.md files provide project-specific context, persona, goals, and coding standards.
They are loaded in a hierarchy:

1. **Global**: `~/.gemini/GEMINI.md`
2. **Workspace/Parent directories**: `.gemini/GEMINI.md` files discovered up the tree
3. **Just-in-Time**: Automatically discovered when tools access files/directories

GEMINI.md is a **supplement** to the system prompt -- it does not replace it.

Manage context via the `/memory` slash command (`show`, `refresh`, `add`). The default
filename can be changed in `settings.json` via `context.fileName`. Large files can be
modularized using `@file.md` import syntax.

### SYSTEM.md (Full Replacement)

The system prompt can be **completely overwritten** using the `GEMINI_SYSTEM_MD`
environment variable:

| Value             | Behavior                                         |
| ----------------- | ------------------------------------------------ |
| `true` or `1`     | Load `./.gemini/system.md` (project default)     |
| File path         | Load custom system prompt from the given path    |
| `false` or `0`    | Disable custom override; use built-in defaults   |

This is a **full replacement** -- the built-in instructions will not apply unless you
include them yourself. A UI indicator (`|~-~|`) signals when a custom system prompt is
active.

Custom system prompt files support variable substitution:
- `${AgentSkills}` -- injects available agent skills
- `${SubAgents}` -- available sub-agents
- `${AvailableTools}` -- enabled tool names
- `${toolName_ToolName}` -- specific tool names

To export the default system prompt for reference:

```bash
GEMINI_WRITE_SYSTEM_MD=1 gemini
```

## Permissions

### Approval Modes

Gemini CLI uses an approval mode system to control tool execution:

| Mode        | Behavior                                                 |
| ----------- | -------------------------------------------------------- |
| `default`   | Prompts for approval on write/shell operations           |
| `auto_edit` | Auto-approves file edit tools; prompts for others        |
| `yolo`      | Auto-approves ALL tool executions                        |
| `plan`      | Read-only mode; blocks all write operations              |

Set via CLI flag, settings, or interactive toggle:

```bash
gemini --approval-mode yolo
gemini --approval-mode plan
gemini -y                      # shorthand for yolo (deprecated flag)
```

In interactive mode, cycle through modes with `Shift+Tab` or the `/plan` command.

### YOLO Mode

Yes, Gemini CLI supports "yolo" mode. Use `--approval-mode yolo` (preferred) or the
deprecated `-y` / `--yolo` flag. YOLO mode can be disabled organization-wide via
`security.disableYoloMode` in settings.

### Policy Engine

Fine-grained permission control is available through TOML policy files:

| Tier    | Location                                                                  |
| ------- | ------------------------------------------------------------------------- |
| User    | `~/.gemini/policies/*.toml`                                               |
| Admin   | `/Library/Application Support/GeminiCli/policies` (macOS)                 |
| Admin   | `/etc/gemini-cli/policies` (Linux)                                        |
| Admin   | `C:\ProgramData\gemini-cli\policies` (Windows)                            |

Policy rules specify `toolName`, `decision` (`allow`, `deny`, `ask_user`), `priority`
(0-999), and optional `matcher` patterns (`commandPrefix`, `commandRegex`, `argsPattern`).
Rules can be scoped to specific approval `modes`.

Admin policies always override user policies. Admin directories require strict filesystem
permissions (root-owned on Linux/macOS) or they are ignored.

### Trusted Folders

Disabled by default. When enabled via `security.folderTrust.enabled: true` in settings,
the CLI prompts to trust new folders before loading workspace config. Untrusted folders
have restricted capabilities (no MCP servers, no workspace settings, no extensions, no
auto-accept).

### Sandbox Mode

Enable sandboxed execution with `-s` / `--sandbox`, the `GEMINI_SANDBOX` environment
variable, or `tools.sandbox` in settings:

| Platform | Method           | Description                                      |
| -------- | ---------------- | ------------------------------------------------ |
| macOS    | Seatbelt         | Lightweight built-in isolation via `sandbox-exec` |
| All      | Docker/Podman    | Container-based complete process isolation        |

macOS Seatbelt profiles (set via `SEATBELT_PROFILE`): `permissive-open`,
`permissive-proxied`, `restrictive-open`, `restrictive-proxied`, `strict-open`,
`strict-proxied`.

## Thinking Level

There is no CLI flag to directly set thinking level. Thinking budget is configured
through `settings.json` using the generation settings system:

```json
{
  "modelConfigs": {
    "overrides": [
      {
        "match": { "model": "gemini-2.5-pro" },
        "modelConfig": {
          "generateContentConfig": {
            "thinkingConfig": {
              "thinkingBudget": 4096
            }
          }
        }
      }
    ]
  }
}
```

The `thinkingConfig` object supports:
- `thinkingBudget` (number): Token budget allocated for extended reasoning
- `includeThoughts` (boolean): Whether to include thinking in output

Thinking display can be toggled in the UI via the `inlineThinking` setting (`on` or `off`,
default: `off`).

Gemini CLI does not expose named thinking levels (like "low", "medium", "high"). Instead,
it uses a numeric token budget that is passed directly to the model provider.

## Logging

### Debug Mode

Launch with `-d` / `--debug` to enable verbose logging and open a debug console (F12).

### Telemetry (OpenTelemetry)

Gemini CLI uses OpenTelemetry for structured observability. Configuration is in
`settings.json` or via environment variables:

| Setting      | Environment Variable             | Default   |
| ------------ | -------------------------------- | --------- |
| `enabled`    | `GEMINI_TELEMETRY_ENABLED`       | `false`   |
| `target`     | `GEMINI_TELEMETRY_TARGET`        | `"local"` |
| `outfile`    | `GEMINI_TELEMETRY_OUTFILE`       | --        |
| `logPrompts` | `GEMINI_TELEMETRY_LOG_PROMPTS`   | `true`    |

Enable local file-based telemetry:

```json
{
  "telemetry": {
    "enabled": true,
    "target": "local",
    "outfile": ".gemini/telemetry.log"
  }
}
```

When using a local collector, logs are saved to `~/.gemini/tmp/<projectHash>/otel/collector.log`.

Telemetry covers: sessions, approval mode changes, tool execution, file operations, API
requests/responses, model routing decisions, chat compression, and UI interactions. All
events include `session.id`, `installation.id`, and `active_approval_mode`.

### Session Storage

Sessions are stored in `~/.gemini/tmp/<project_hash>/chats/`.

## CLI Options

### Subcommands

| Subcommand                    | Alias        | Description                               |
| ----------------------------- | ------------ | ----------------------------------------- |
| `gemini [query..]`            | --           | Launch Gemini CLI (default command)        |
| `gemini mcp`                  | --           | Manage MCP servers                        |
| `gemini mcp add`              | --           | Add a stdio or HTTP MCP server            |
| `gemini mcp remove`           | --           | Remove an MCP server                      |
| `gemini mcp list`             | --           | List configured MCP servers               |
| `gemini mcp enable`           | --           | Enable an MCP server                      |
| `gemini mcp disable`          | --           | Disable an MCP server                     |
| `gemini extensions`           | `extension`  | Manage extensions                         |
| `gemini extensions install`   | --           | Install from Git URL or local path        |
| `gemini extensions uninstall` | --           | Uninstall one or more extensions          |
| `gemini extensions list`      | --           | List installed extensions                 |
| `gemini extensions update`    | --           | Update specific or all extensions         |
| `gemini extensions enable`    | --           | Enable an extension                       |
| `gemini extensions disable`   | --           | Disable an extension                      |
| `gemini extensions link`      | --           | Link local extension for development      |
| `gemini extensions new`       | --           | Create new extension from template        |
| `gemini extensions validate`  | --           | Validate extension structure              |
| `gemini extensions config`    | --           | Configure extension settings              |
| `gemini skills`               | `skill`      | Manage agent skills                       |
| `gemini skills list`          | --           | List discovered agent skills              |
| `gemini skills install`       | --           | Install skill from Git, path, or file     |
| `gemini skills uninstall`     | --           | Uninstall an agent skill                  |
| `gemini skills enable`        | --           | Enable an agent skill                     |
| `gemini skills disable`       | --           | Disable an agent skill                    |
| `gemini hooks`                | `hook`       | Manage hooks                              |
| `gemini hooks migrate`        | --           | Migrate hooks from Claude Code            |

### Switches

| Switch                        | Short | Type    | Default   | Description                                                     |
| ----------------------------- | ----- | ------- | --------- | --------------------------------------------------------------- |
| `--debug`                     | `-d`  | boolean | `false`   | Run in debug mode (open debug console with F12)                 |
| `--model`                     | `-m`  | string  | `auto`    | Model name or alias (`auto`, `pro`, `flash`, `flash-lite`)      |
| `--prompt`                    | `-p`  | string  | --        | Non-interactive mode with given prompt (deprecated; use positional args) |
| `--prompt-interactive`        | `-i`  | string  | --        | Execute prompt and continue in interactive mode                 |
| `--sandbox`                   | `-s`  | boolean | `false`   | Run in sandboxed environment                                    |
| `--yolo`                      | `-y`  | boolean | `false`   | Auto-approve all actions (deprecated; use `--approval-mode yolo`) |
| `--approval-mode`             | --    | string  | `default` | Approval mode: `default`, `auto_edit`, `yolo`, `plan`           |
| `--experimental-acp`          | --    | boolean | --        | Start in ACP (Agent Communication Protocol) mode                |
| `--allowed-mcp-server-names`  | --    | array   | --        | Restrict which MCP servers can connect                          |
| `--allowed-tools`             | --    | array   | --        | Tools allowed without confirmation (deprecated; use Policy Engine) |
| `--extensions`                | `-e`  | array   | --        | Limit to specific extensions; omit to use all                   |
| `--list-extensions`           | `-l`  | boolean | --        | List all available extensions and exit                          |
| `--resume`                    | `-r`  | string  | --        | Resume session (`"latest"` or index number)                     |
| `--list-sessions`             | --    | boolean | --        | List available sessions for current project and exit            |
| `--delete-session`            | --    | string  | --        | Delete session by index number                                  |
| `--include-directories`       | --    | array   | --        | Additional workspace directories (comma-separated)              |
| `--screen-reader`             | --    | boolean | --        | Enable screen reader mode for accessibility                     |
| `--output-format`             | `-o`  | string  | `text`    | Output format: `text`, `json`, `stream-json`                    |
| `--raw-output`                | --    | boolean | --        | Disable sanitization of model output (allows ANSI escapes)      |
| `--accept-raw-output-risk`    | --    | boolean | --        | Suppress the security warning when using `--raw-output`         |
| `--version`                   | `-v`  | boolean | --        | Show version number                                             |
| `--help`                      | `-h`  | boolean | --        | Show help                                                       |

## Sources

- [Gemini CLI GitHub Repository](https://github.com/google-gemini/gemini-cli)
- [Gemini CLI Documentation](https://geminicli.com/docs/)
- [CLI Reference](https://geminicli.com/docs/cli/cli-reference)
- [Model Selection](https://github.com/google-gemini/gemini-cli/blob/main/docs/cli/model.md)
- [Model Routing](https://github.com/google-gemini/gemini-cli/blob/main/docs/cli/model-routing.md)
- [Headless Mode](https://github.com/google-gemini/gemini-cli/blob/main/docs/cli/headless.md)
- [System Prompt](https://github.com/google-gemini/gemini-cli/blob/main/docs/cli/system-prompt.md)
- [GEMINI.md Context Files](https://github.com/google-gemini/gemini-cli/blob/main/docs/cli/gemini-md.md)
- [Settings](https://github.com/google-gemini/gemini-cli/blob/main/docs/cli/settings.md)
- [Generation Settings](https://github.com/google-gemini/gemini-cli/blob/main/docs/cli/generation-settings.md)
- [Sandbox](https://github.com/google-gemini/gemini-cli/blob/main/docs/cli/sandbox.md)
- [Trusted Folders](https://github.com/google-gemini/gemini-cli/blob/main/docs/cli/trusted-folders.md)
- [Policy Engine](https://github.com/google-gemini/gemini-cli/blob/main/docs/core/policy-engine.md)
- [Hooks Reference](https://github.com/google-gemini/gemini-cli/blob/main/docs/hooks/reference.md)
- [Telemetry](https://github.com/google-gemini/gemini-cli/blob/main/docs/cli/telemetry.md)
- [Quota and Pricing](https://github.com/google-gemini/gemini-cli/blob/main/docs/quota-and-pricing.md)
- [Session Management](https://github.com/google-gemini/gemini-cli/blob/main/docs/cli/session-management.md)
- [Slash Commands](https://github.com/google-gemini/gemini-cli/blob/main/docs/cli/commands.md)
