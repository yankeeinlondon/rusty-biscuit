---
homepage: https://github.com/google-gemini/gemini-cli
docs: https://geminicli.com/docs/
cli_docs: https://geminicli.com/docs/cli/cli-reference
latest_stable: v0.39.1
last_reviewed: 2026-04-27
---

# Gemini CLI

Google's open-source AI agent that brings Gemini models directly into the terminal. Installed
via npm (`@google/gemini-cli`), Homebrew, or MacPorts. Provides interactive REPL,
headless/non-interactive mode, MCP server management, extensions, agent skills, hooks,
subagents, git worktrees, session checkpointing, and plan mode.

Configuration lives in `~/.gemini/settings.json` (user-level) and `.gemini/settings.json`
(workspace-level). Four configuration layers exist with increasing precedence: system defaults,
user settings, project settings, system overrides. Environment variables and CLI arguments
override all settings files. Project context is provided via `GEMINI.md` files at global
(`~/.gemini/GEMINI.md`), workspace, and just-in-time discovery levels.

Current stable release: **v0.39.1** (April 24, 2026). Written in TypeScript (98%). Licensed
under Apache 2.0 with 103k+ GitHub stars.

## Model Specification

### CLI Flag

Use `-m` / `--model` to specify a model:

```bash
gemini -m gemini-3-pro-preview "explain this codebase"
gemini -m flash "quick summary"
gemini -m auto-gemini-3 "refactor this module"
```

### Model Aliases

| Alias             | Resolves To                          | Purpose                              |
| ----------------- | ------------------------------------ | ------------------------------------ |
| `auto`            | `gemini-3-pro-preview` (or 3.1)     | Default; uses preview if enabled     |
| `auto-gemini-3`   | `gemini-3-pro-preview` (or 3.1)     | Auto-select from Gemini 3 family     |
| `auto-gemini-2.5` | `gemini-2.5-pro`                     | Auto-select from Gemini 2.5 family   |
| `pro`             | `gemini-3-pro-preview` (or 3.1)     | Complex reasoning tasks              |
| `flash`           | `gemini-3-flash-preview`             | Fast, balanced for most tasks        |
| `flash-lite`      | `gemini-2.5-flash-lite` (or 3.1)    | Fastest for simple tasks             |

### Concrete Model Names

| Model                          | Family    | Tier       | Thinking | Multimodal Tools |
| ------------------------------ | --------- | ---------- | -------- | ---------------- |
| `gemini-3.1-pro-preview`       | gemini-3  | pro        | yes      | yes              |
| `gemini-3.1-flash-lite-preview`| gemini-3  | flash-lite | no       | yes              |
| `gemini-3-pro-preview`         | gemini-3  | pro        | yes      | yes              |
| `gemini-3-flash-preview`       | gemini-3  | flash      | no       | yes              |
| `gemini-2.5-pro`               | gemini-2.5| pro        | no       | no               |
| `gemini-2.5-flash`             | gemini-2.5| flash      | no       | no               |
| `gemini-2.5-flash-lite`        | gemini-2.5| flash-lite | no       | no               |
| `gemma-4-31b-it`               | gemma-4   | custom     | yes      | no               |
| `gemma-4-26b-a4b-it`           | gemma-4   | custom     | yes      | no               |

### Model Selection Precedence

1. `--model` CLI flag (highest priority)
2. `GEMINI_MODEL` environment variable
3. `model.name` in `settings.json`
4. Default: `auto`

### Auto Routing and Model Chains

When set to `auto`, the CLI automatically selects the best model based on task complexity.
Model resolution uses context-aware rules: if preview access is disabled, Gemini 3 models
fall back to Gemini 2.5 equivalents. If `useGemini3_1` is enabled, 3.1 variants are
preferred.

Availability policy chains define fallback behavior. The default chain is:
`gemini-3-pro-preview` -> `gemini-3-flash-preview` (preview chain), or
`gemini-2.5-pro` -> `gemini-2.5-flash` (default chain). The `lite` chain:
`gemini-2.5-flash-lite` -> `gemini-2.5-flash` -> `gemini-2.5-pro`.

A classifier model (`gemini-2.5-flash-lite`) determines routing between pro and flash tiers
based on task complexity.

### Custom Aliases and Overrides

Advanced model configuration via `modelConfigs` in `settings.json`:

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

Functionally identical to the positional argument. When both `-p` and stdin are provided,
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

## CLI Switch Summary

### `--model` / `-m` (string, default: `auto`)

Specifies the Gemini model to use. Accepts model aliases (`auto`, `pro`, `flash`,
`flash-lite`, `auto-gemini-3`, `auto-gemini-2.5`) or concrete model names
(`gemini-3-pro-preview`, `gemini-2.5-flash`, etc.).

```bash
gemini -m flash "quick summary"
gemini -m gemini-3-pro-preview "complex reasoning task"
gemini -m auto-gemini-2.5 "use 2.5 models only"
```

### `--prompt` / `-p` (string, no default)

Provides a prompt for non-interactive execution. When combined with stdin, the prompt text
is appended to the stdin content. Use positional arguments instead when possible (they are
the preferred syntax).

```bash
gemini -p "Explain the architecture of this codebase"
echo "README content" | gemini -p "summarize this"
```

### `--prompt-interactive` / `-i` (string, no default)

Executes the given prompt and then continues in interactive mode. Useful for bootstrapping
a session with an initial task.

```bash
gemini -i "Analyze the test coverage in this project"
```

### `--output-format` / `-o` (string, default: `text`)

Controls the format of non-interactive output. Choices: `text`, `json`, `stream-json`.

```bash
gemini -p "List all TODO comments" --output-format json
gemini -p "Run tests and deploy" --output-format stream-json
gemini -o json "explain the API"
```

### `--approval-mode` (string, default: `default`)

Sets the approval mode for tool execution. Choices:

- `default` -- Prompts for approval on write/shell operations
- `auto_edit` -- Auto-approves file edit tools; prompts for shell commands
- `yolo` -- Auto-approves ALL tool executions without confirmation
- `plan` -- Read-only mode; blocks all write operations

```bash
gemini --approval-mode yolo "refactor everything"
gemini --approval-mode plan "analyze the codebase"
gemini --approval-mode auto_edit "fix the lint errors"
```

### `--yolo` / `-y` (boolean, default: `false`)

**Deprecated.** Auto-approves all actions. Use `--approval-mode yolo` instead.

```bash
gemini -y "deploy to production"
```

### `--sandbox` / `-s` (boolean, default: `false`)

Runs commands in a sandboxed environment for safer execution. On macOS, uses the built-in
Seatbelt (`sandbox-exec`). On all platforms, supports Docker/Podman container isolation.
Can also be enabled via `GEMINI_SANDBOX` environment variable or `tools.sandbox` in settings.

```bash
gemini --sandbox "run the test suite"
gemini -s "analyze this untrusted code"
```

### `--skip-trust` (boolean, default: `false`)

Trusts the current workspace for this session, skipping the folder trust check. Only
relevant when folder trust is enabled via `security.folderTrust.enabled: true` in settings.

```bash
gemini --skip-trust "run tasks in this project"
```

### `--worktree` / `-w` (string, no default)

Starts Gemini CLI in a new git worktree. If no name is provided, one is generated
automatically. Requires `experimental.worktrees: true` in settings.

```bash
gemini --worktree feature-branch
gemini -w "fix-auth-bug"
```

### `--debug` / `-d` (boolean, default: `false`)

Runs in debug mode with verbose logging. Opens a debug console accessible with F12.

```bash
gemini --debug
gemini -d "trace this error"
```

### `--version` / `-v` (no type)

Shows the CLI version number and exits.

```bash
gemini --version
gemini -v
```

### `--help` / `-h` (no type)

Shows help information and exits.

```bash
gemini --help
gemini -h
```

### `--resume` / `-r` (string, no default)

Resumes a previous session. Use `"latest"` for the most recent session or a specific session
index number. A new prompt can be appended as a positional argument.

```bash
gemini --resume "latest"
gemini -r "latest" "Check for type errors"
gemini -r 5 "continue the refactor"
```

### `--list-sessions` (boolean, no default)

Lists all available sessions for the current project and exits.

```bash
gemini --list-sessions
```

### `--delete-session` (string, no default)

Deletes a session by index number. Use `--list-sessions` to see available sessions.

```bash
gemini --delete-session 3
```

### `--extensions` / `-e` (array, no default)

Limits the session to specific extensions. When omitted, all installed extensions are
enabled. Accepts comma-separated values or multiple flags.

```bash
gemini --extensions my-ext,another-ext
gemini -e linter -e formatter
```

### `--list-extensions` / `-l` (boolean, no default)

Lists all available extensions and exits.

```bash
gemini --list-extensions
gemini -l
```

### `--allowed-mcp-server-names` (array, no default)

Restricts which MCP servers can connect during the session. Accepts comma-separated values
or multiple flags. Useful for limiting tool access in automated workflows.

```bash
gemini --allowed-mcp-server-names github,slack
gemini --allowed-mcp-server-names github --allowed-mcp-server-names database
```

### `--allowed-tools` (array, no default)

**Deprecated.** Use the Policy Engine instead. Specifies tools that are allowed to run
without confirmation. Accepts comma-separated values or multiple flags.

```bash
gemini --allowed-tools read_file,write_file
```

### `--include-directories` (array, no default)

Adds additional directories to the workspace beyond the current working directory. Accepts
comma-separated paths or multiple flags.

```bash
gemini --include-directories ../lib,../docs
gemini --include-directories ../shared --include-directories ../config
```

### `--screen-reader` (boolean, no default)

Enables screen reader mode for accessibility. Renders output in plain text to be more
compatible with assistive technology.

```bash
gemini --screen-reader
```

### `--experimental-acp` (boolean, no default)

Starts the CLI in ACP (Agent Communication Protocol) mode. Experimental feature for IDE
integration and agent-to-agent communication.

```bash
gemini --experimental-acp
```

### `--experimental-zed-integration` (boolean, no default)

Runs in Zed editor integration mode. Experimental feature.

```bash
gemini --experimental-zed-integration
```

## Subscription versus Per Call API

Gemini CLI supports multiple authentication methods, each with different pricing and quotas.

### Free Tiers

| Auth Method                   | Limits                                    | Model Access    |
| ----------------------------- | ----------------------------------------- | --------------- |
| Google Login (OAuth)          | 1,000 requests/day, 60 requests/min       | Full family     |
| Gemini API Key (unpaid)       | 250 requests/day, 10 requests/min         | Flash only      |
| Vertex AI Express Mode        | 90-day trial before billing required       | Varies          |

### Paid Options

| Auth Method                         | Pricing Model      | Limits / Notes                              |
| ----------------------------------- | ------------------ | ------------------------------------------- |
| Google AI Pro                       | Fixed subscription | 1,500 requests/day                          |
| Google AI Ultra                     | Fixed subscription | 2,000 requests/day                          |
| Workspace AI Ultra                  | Fixed subscription | 2,000 requests/day                          |
| Gemini Code Assist Standard         | Per-seat license   | 1,500 requests/day, 120/min                |
| Gemini Code Assist Enterprise       | Per-seat license   | 2,000 requests/day, 120/min                |
| Vertex AI (regular)                 | Pay-per-token      | Dynamic shared quota                        |
| Gemini API Key (paid tier)          | Pay-per-token      | Per-token pricing varies by model           |

The documentation warns that pay-per-token pricing "can be more expensive for many small
calls with few tokens," making subscription plans preferable for frequent use.

How to start non-interactive sessions under each pricing model is determined entirely by
the authentication method configured (Google Login, API key, or Vertex AI), not by any
CLI flag. The non-interactive invocation is the same regardless of pricing tier.

### Authentication Methods

**Sign in with Google (recommended):** Launch `gemini`, select "Sign in with Google" in the
browser flow. Credentials are cached locally. Requires a web browser on the local machine.
Organization/Workspace accounts require setting `GOOGLE_CLOUD_PROJECT`.

**Gemini API Key:** Set `GEMINI_API_KEY` environment variable. Get key from
[Google AI Studio](https://aistudio.google.com/apikey). Best for headless/CI environments.

**Vertex AI:** Three sub-methods available:
1. Application Default Credentials via `gcloud auth application-default login`
2. Service account JSON key via `GOOGLE_APPLICATION_CREDENTIALS`
3. Google Cloud API key via `GOOGLE_API_KEY`

Requires `GOOGLE_CLOUD_PROJECT` and `GOOGLE_CLOUD_LOCATION` environment variables.

Environment variables can be persisted in shell config files or in `.gemini/.env` files
(searched from current directory upward, then `~/.gemini/.env`).

## System Prompt

Gemini CLI distinguishes between two types of instructional files:

### GEMINI.md (Strategy / Context)

GEMINI.md files provide project-specific context, persona, goals, and coding standards.
They are loaded in a hierarchy:

1. **Global**: `~/.gemini/GEMINI.md`
2. **Workspace/Parent directories**: `.gemini/GEMINI.md` files discovered up the tree
3. **Just-in-Time**: Automatically discovered when tools access files/directories

GEMINI.md is a **supplement** to the system prompt -- it does not replace it.

Manage context via the `/memory` slash command (`show`, `refresh`, `add`, `inbox`). The
default filename can be changed in `settings.json` via `context.fileName`. Large files
can be modularized using `@file.md` import syntax.

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

Gemini CLI supports "yolo" mode. Use `--approval-mode yolo` (preferred) or the deprecated
`-y` / `--yolo` flag. YOLO mode can be disabled organization-wide via
`security.disableYoloMode` in settings.

### Policy Engine

Fine-grained permission control through TOML policy files:

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
auto-accept). Use `--skip-trust` to bypass for a single session.

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
        "match": { "model": "gemini-3-pro-preview" },
        "modelConfig": {
          "generateContentConfig": {
            "thinkingConfig": {
              "thinkingLevel": "HIGH"
            }
          }
        }
      }
    ]
  }
}
```

The `thinkingConfig` object supports:
- `thinkingBudget` (number): Token budget allocated for extended reasoning (Gemini 2.5 models)
- `thinkingLevel` (string): Named level for Gemini 3 models (e.g., `"HIGH"`)
- `includeThoughts` (boolean): Whether to include thinking in output

Thinking display can be toggled in the UI via the `ui.inlineThinkingMode` setting (`"off"`
or `"full"`, default: `"off"`).

Gemini CLI does not expose named thinking levels as CLI flags. Instead, it uses the
`modelConfigs` system with model-specific configuration that is passed to the provider.

## Features

### Extensions

Extensions add tools and capabilities. Managed via the `gemini extensions` subcommand.
Extensions can be installed from Git URLs or local paths, linked for development, and
auto-updated. Each extension can have its own `.env` file and `gemini-extension.json`
configuration.

### Agent Skills

Skills provide specialized knowledge for specific tasks. Managed via `gemini skills`
subcommand. Skills are installed from Git repos, local paths, or files. They can be
enabled/disabled individually or in bulk. Auto-memory (experimental) extracts skills
during sessions.

### Hooks

Hooks allow running custom commands before or after tool execution. Configured in
`settings.json` or via the `gemini hooks` subcommand. Supports migrating hooks from
Claude Code.

### Subagents

Built-in subagents handle specialized tasks (codebase investigation, file editing, etc.).
Remote subagents are supported for distributed workflows. Subagent memory is managed via
AbortSignal-based cleanup.

### Plan Mode

Plan mode provides read-only safety during planning phases. Models automatically switch
between Pro (planning) and Flash (implementation) when `general.plan.modelRouting` is
enabled. Plans are stored as artifacts with user confirmation required for skill
activation.

### Checkpointing

Session checkpointing enables recovery from interruptions. Enabled via
`general.checkpointing.enabled: true` in settings. Sessions stored in
`~/.gemini/tmp/<project_hash>/chats/`.

### Git Worktrees

Experimental feature for running Gemini CLI in isolated git worktrees. Requires
`experimental.worktrees: true` in settings and the `--worktree` CLI flag.

### IDE Integration

VS Code companion support. ACP (Agent Communication Protocol) mode enables IDE agent
integration via `--experimental-acp`. Zed editor integration via
`--experimental-zed-integration`.

### Notifications

Experimental terminal notifications for action-required prompts and session completion.
Configured via `general.enableNotifications` and `general.notificationMethod` (supports
`osc9`, `osc777`, `bell`, and `auto` detection).

### Rewind

Allows reverting conversation state to a previous turn. Useful for undoing tool
executions or exploring alternative approaches.

### Model Steering

Experimental feature for fine-grained control over model behavior during plan mode.
Enables switching between reasoning-intensive and execution-focused model profiles.

## Logging

### Debug Mode

Launch with `-d` / `--debug` to enable verbose logging and open a debug console (F12).

### Telemetry (OpenTelemetry)

Gemini CLI uses OpenTelemetry for structured observability. Configuration in
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

Sessions are stored in `~/.gemini/tmp/<project_hash>/chats/`. Automatic session cleanup
is configurable via `general.sessionRetention` (max age, max count, min retention).

## CLI Options

### Subcommands

| Subcommand                    | Alias        | Description                               |
| ----------------------------- | ------------ | ----------------------------------------- |
| `gemini [query..]`            | --           | Launch Gemini CLI (default command)        |
| `gemini update`               | --           | Update to latest version                   |
| `gemini mcp`                  | --           | Manage MCP servers                         |
| `gemini mcp add`              | --           | Add a stdio or HTTP MCP server             |
| `gemini mcp remove`           | --           | Remove an MCP server                       |
| `gemini mcp list`             | --           | List configured MCP servers                |
| `gemini mcp enable`           | --           | Enable an MCP server                       |
| `gemini mcp disable`          | --           | Disable an MCP server                      |
| `gemini extensions`           | `extension`  | Manage extensions                          |
| `gemini extensions install`   | --           | Install from Git URL or local path         |
| `gemini extensions uninstall` | --           | Uninstall one or more extensions           |
| `gemini extensions list`      | --           | List installed extensions                  |
| `gemini extensions update`    | --           | Update specific or all extensions          |
| `gemini extensions enable`    | --           | Enable an extension                        |
| `gemini extensions disable`   | --           | Disable an extension                       |
| `gemini extensions link`      | --           | Link local extension for development       |
| `gemini extensions new`       | --           | Create new extension from template         |
| `gemini extensions validate`  | --           | Validate extension structure               |
| `gemini extensions config`    | --           | Configure extension settings               |
| `gemini skills`               | `skill`      | Manage agent skills                        |
| `gemini skills list`          | --           | List discovered agent skills               |
| `gemini skills install`       | --           | Install skill from Git, path, or file      |
| `gemini skills link`          | --           | Link local skills via symlink              |
| `gemini skills uninstall`     | --           | Uninstall an agent skill                   |
| `gemini skills enable`        | --           | Enable an agent skill                      |
| `gemini skills disable`       | --           | Disable an agent skill                     |
| `gemini hooks`                | `hook`       | Manage hooks                               |
| `gemini hooks migrate`        | --           | Migrate hooks from Claude Code             |

### Interactive Commands

Available within the REPL session:

| Command             | Description                              |
| ------------------- | ---------------------------------------- |
| `/skills reload`    | Reload discovered skills from disk       |
| `/agents reload`    | Reload the agent registry                |
| `/commands reload`  | Reload custom slash commands             |
| `/memory reload`    | Reload context files (GEMINI.md)         |
| `/memory inbox`     | Review and patch extracted skills        |
| `/mcp reload`       | Restart and reload MCP servers           |
| `/extensions reload`| Reload all active extensions             |
| `/stats model`      | Show token usage and quota limits        |
| `/help`             | Show help for all commands               |
| `/quit`             | Exit the interactive session             |

## Sources

- [Gemini CLI GitHub Repository](https://github.com/google-gemini/gemini-cli)
- [Gemini CLI Documentation](https://geminicli.com/docs/)
- [CLI Cheatsheet](https://geminicli.com/docs/cli/cli-reference/)
- [Configuration Reference](https://geminicli.com/docs/reference/configuration/)
- [Model Selection](https://geminicli.com/docs/cli/model/)
- [Model Routing](https://geminicli.com/docs/cli/model-routing/)
- [Headless Mode](https://geminicli.com/docs/cli/headless/)
- [System Prompt](https://geminicli.com/docs/cli/system-prompt/)
- [GEMINI.md Context Files](https://geminicli.com/docs/cli/gemini-md/)
- [Settings](https://geminicli.com/docs/cli/settings/)
- [Generation Settings](https://geminicli.com/docs/cli/generation-settings/)
- [Sandbox](https://geminicli.com/docs/cli/sandbox/)
- [Trusted Folders](https://geminicli.com/docs/cli/trusted-folders/)
- [Policy Engine](https://geminicli.com/docs/reference/policy-engine/)
- [Hooks Reference](https://geminicli.com/docs/hooks/reference/)
- [Telemetry](https://geminicli.com/docs/cli/telemetry/)
- [Quota and Pricing](https://geminicli.com/docs/resources/quota-and-pricing/)
- [Authentication](https://geminicli.com/docs/get-started/authentication/)
- [Agent Skills](https://geminicli.com/docs/cli/skills/)
- [Extensions](https://geminicli.com/docs/extensions/)
- [Plan Mode](https://geminicli.com/docs/cli/plan-mode/)
- [Subagents](https://geminicli.com/docs/core/subagents/)
- [Checkpointing](https://geminicli.com/docs/cli/checkpointing/)
- [Git Worktrees](https://geminicli.com/docs/cli/git-worktrees/)
- [Notifications](https://geminicli.com/docs/cli/notifications/)
- [Changelog v0.39.0](https://geminicli.com/docs/changelogs/latest/)
