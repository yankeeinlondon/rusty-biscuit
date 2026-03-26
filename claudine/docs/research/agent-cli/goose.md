---
homepage: https://block.github.io/goose/
docs: https://block.github.io/goose/docs/getting-started/installation
cli_docs: https://block.github.io/goose/docs/guides/goose-cli-commands/
---

# Goose CLI

Goose is an open-source, local AI agent by Block (parent of Square, CashApp, Tidal) that automates
engineering tasks. It supports 30+ LLM providers, MCP server extensions, and is available as both a
desktop application and CLI. Installed via Homebrew (`brew install block-goose-cli`), shell script,
or platform-specific downloads.

## Model Specification

### CLI Override

The `goose run` command accepts `--provider` and `--model` flags to override the configured
provider and model for a single invocation:

```bash
goose run --provider anthropic --model claude-sonnet-4-0 -t "refactor auth module"
```

The `goose session` command does **not** accept `--provider` or `--model` flags. Interactive
sessions always use the configured provider/model.

### Configuration Methods

1. **`goose configure`** -- interactive wizard that writes to `config.yaml`
2. **Environment variables** -- `GOOSE_PROVIDER` and `GOOSE_MODEL` override `config.yaml`
3. **Config file** -- directly edit `~/.config/goose/config.yaml`

Priority order: environment variables > config file > defaults.

### Additional Model Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `GOOSE_PROVIDER` | LLM provider name (e.g., `anthropic`, `openai`, `ollama`) | Required |
| `GOOSE_MODEL` | Model name from provider | Required |
| `GOOSE_TEMPERATURE` | Response randomness (0.0--1.0) | Model-specific |
| `GOOSE_MAX_TOKENS` | Maximum response token count | Model-specific |

### Lead/Worker Multi-Model Pattern

Goose supports a dual-model workflow where a stronger "lead" model handles initial planning turns
before handing off to a cheaper "worker" model:

| Variable | Description | Default |
|----------|-------------|---------|
| `GOOSE_LEAD_MODEL` | High-capability model for planning | -- |
| `GOOSE_LEAD_PROVIDER` | Provider for lead model | Falls back to `GOOSE_PROVIDER` |
| `GOOSE_LEAD_TURNS` | Turns using lead before switching | 3 |
| `GOOSE_LEAD_FAILURE_THRESHOLD` | Consecutive failures triggering fallback | 2 |
| `GOOSE_LEAD_FALLBACK_TURNS` | Turns in fallback mode | 2 |

### Planner Model

| Variable | Description | Default |
|----------|-------------|---------|
| `GOOSE_PLANNER_PROVIDER` | Provider for planning mode | Falls back to main provider |
| `GOOSE_PLANNER_MODEL` | Model for planning mode | Falls back to main model |

### Default Model

When configured through `goose configure`, the documentation recommends Claude Sonnet 4.5 as the
default. There is no hardcoded default -- the user must configure a provider and model before first
use.


## Non-interactive Engagement

Goose supports non-interactive execution through the `goose run` subcommand. Three input methods
are available:

### Method 1: Inline Text (`-t`)

Pass instructions directly as a string:

```bash
goose run -t "list all TODO comments in the codebase"
```

**Benefits**: Simple, scriptable, good for one-liners.
**Limitations**: Awkward for multi-line or complex instructions.

### Method 2: Instruction File (`-i`)

Point to a file containing instructions:

```bash
goose run -i instructions.md
```

**Benefits**: Supports complex multi-line instructions, version-controllable.
**Limitations**: Requires a file on disk.

### Method 3: Stdin Pipe (`-i -`)

Pipe instructions via stdin:

```bash
echo "What is 2+2?" | goose run -i -

cat <<EOF | goose run -i -
Analyze the repository structure.
List the top 5 largest files.
EOF
```

**Benefits**: Composable with other shell commands, supports heredocs.
**Limitations**: Must use `-i -` explicitly; bare piping without the flag is not supported.

### Non-interactive Control Flags

| Flag | Description |
|------|-------------|
| `--no-session` | Discard session data (no persistence) |
| `--max-turns <N>` | Limit autonomous turns |
| `--max-tool-repetitions <N>` | Prevent infinite tool loops |
| `-q, --quiet` | Suppress non-response output |
| `--output-format <fmt>` | `text`, `json`, or `stream-json` |
| `--debug` | Show complete tool responses |

### Recipes

Recipes are reusable task definitions (YAML files) that bundle instructions, extensions, and
parameters. They can be executed non-interactively:

```bash
goose run --recipe my-recipe.yaml --params KEY=value
```

Recipes support sub-recipes (`--sub-recipe`) and can be validated with `goose recipe validate`.

### Staying Interactive After Run

Use `-s, --interactive` to drop into an interactive session after the non-interactive instructions
complete:

```bash
goose run -t "set up the project" -s
```


## Subscription versus Per Call API

Goose itself is free and open-source. There is no subscription pricing for the tool. All costs come
from the underlying LLM provider's API pricing, which is exclusively per-call (token-based).

To start a non-interactive session, use `goose run` with one of the input methods above. The LLM
API key is configured via provider-specific environment variables (e.g., `ANTHROPIC_API_KEY`,
`OPENAI_API_KEY`, `GOOGLE_API_KEY`) or through `goose configure`.


## System Prompt

Goose provides multiple layers for supplementing its system prompt. None of these **replace** the
built-in system prompt; they are appended to it.

### `.goosehints` File

Place a `.goosehints` file in your project root. Its contents are appended to the system prompt when
the developer extension is enabled. This behaves like a "README for AI" -- project-specific
conventions, structure, and instructions.

The recognized filename can be customized via `CONTEXT_FILE_NAMES` (JSON array, default:
`[".goosehints"]`).

### `--system` Flag (goose run)

The `goose run --system <TEXT>` flag provides additional system instructions for that run. This
supplements (does not replace) the built-in system prompt.

### `GOOSE_MOIM_MESSAGE_TEXT` / `GOOSE_MOIM_MESSAGE_FILE`

Persistent working memory that is injected into every session:

- `GOOSE_MOIM_MESSAGE_TEXT` -- inline text content
- `GOOSE_MOIM_MESSAGE_FILE` -- path to a file (max 64 KB)

### Recipe Instructions

Recipes include an `instructions` field that acts as a system prompt supplement for that recipe's
execution.


## Permissions

### Default Mode

The default permission mode is **smart_approve** (per the `GOOSE_MODE` environment variable
documentation; the permissions guide notes "auto" as the desktop default -- behavior may vary by
version).

### Four Permission Modes

| Mode | Behavior |
|------|----------|
| `auto` | Fully autonomous -- file modifications, extensions, deletions without approval |
| `smart_approve` | Auto-approves low-risk actions; flags higher-risk actions for review |
| `approve` | Requires manual confirmation for all tool/extension use |
| `chat` | Chat only -- no file modifications or extension use |

### Configuration Methods

1. **In-session slash command**: `/mode auto`, `/mode smart_approve`, `/mode approve`, `/mode chat`
2. **`goose configure`**: Select "goose settings" then "goose mode"
3. **Environment variable**: `GOOSE_MODE=auto`
4. **Config file**: Set `GOOSE_MODE` in `~/.config/goose/config.yaml`

### "Yolo" Mode

Goose does not have a flag named "yolo". The equivalent is **auto mode** (`GOOSE_MODE=auto` or
`/mode auto`), which bypasses all approval prompts and allows Goose to execute tools freely.

### Permission Files

- `~/.config/goose/permission.yaml` -- tool permission levels
- `~/.config/goose/permissions/tool_permissions.json` -- runtime permission decisions


## Thinking Level

Goose has limited, provider-specific thinking level support. There is no universal `--thinking` CLI
flag.

### Gemini 3 Models

Set `GEMINI3_THINKING_LEVEL` to `"low"` or `"high"` (default: `"low"`).

### Codex Models (when using Codex as a provider)

Set `CODEX_REASONING_EFFORT` to `"low"`, `"medium"`, `"high"`, or `"xhigh"`.

### Other Providers

Thinking/reasoning level is not configurable from Goose for other providers (Anthropic, OpenAI
direct, etc.). The model's default behavior is used.


## Logging

### Log Locations

| Log Type | Location (macOS/Linux) |
|----------|------------------------|
| CLI logs | `~/.local/state/goose/logs/cli/` |
| Server logs | `~/.local/state/goose/logs/server/` |
| LLM request/response logs | `~/.local/state/goose/logs/llm_request.*.jsonl` |
| Command history | `~/.config/goose/history.txt` |
| Session database | `~/.local/share/goose/sessions/sessions.db` |
| Desktop app log (macOS) | `~/Library/Application Support/Goose/logs/main.log` |

### Log Organization

- CLI and server logs use **date-based subdirectories** (e.g., `cli/2025-11-13/`)
- Subdirectories older than **two weeks** are automatically deleted
- LLM request logs rotate through 10 files: `llm_request.0.jsonl` through `llm_request.9.jsonl`

### Session Database

Since v1.10.0, sessions are stored in an SQLite database containing:

- Session metadata (ID, name, working directory, timestamps)
- Conversation messages (user commands, assistant responses)
- Tool calls and results (arguments, responses, success/failure)

Session IDs follow the format `YYYYMMDD_<COUNT>` (e.g., `20250310_2`).

### Security Logs

When prompt injection detection is enabled (`SECURITY_PROMPT_ENABLED=true`), security findings are
logged with IDs in the format `SEC-{uuid}`.

### Accessing Log Info

```bash
goose info           # Show version, config location, session storage, logs
goose info -v        # Verbose: include environment variables and extensions
```

### Observability

Goose supports OpenTelemetry export (`OTEL_EXPORTER_OTLP_ENDPOINT`) and Langfuse integration
(`LANGFUSE_PUBLIC_KEY`, `LANGFUSE_SECRET_KEY`, `LANGFUSE_URL`) for external observability.


## CLI Options

### Subcommands

| Subcommand | Description |
|------------|-------------|
| `session` | Start or resume an interactive chat session |
| `session list` | List all saved sessions |
| `session remove` | Delete saved sessions |
| `session export` | Export sessions (markdown, json, yaml) |
| `session diagnostics` | Generate troubleshooting bundle |
| `run` | Execute commands from text, file, stdin, or recipe |
| `configure` | Configure providers, extensions, and settings |
| `info` | Show version, config location, session storage, logs |
| `version` | Display installed version |
| `update` | Update to a newer version |
| `completion` | Generate shell completion scripts (bash, elvish, fish, powershell, zsh) |
| `bench` | Evaluate system configuration across practical tasks |
| `recipe` | Validate and manage recipe files |
| `recipe deeplink` | Generate shareable recipe link |
| `recipe list` | List available recipes |
| `recipe open` | Open recipe in Goose desktop |
| `recipe validate` | Validate a recipe file |
| `schedule` | Automate recipes with cron scheduling |
| `schedule add` | Create a scheduled job |
| `schedule list` | View all scheduled jobs |
| `schedule remove` | Delete a scheduled job |
| `schedule sessions` | List sessions from a scheduled recipe |
| `schedule run-now` | Execute a scheduled recipe immediately |
| `project` / `p` | Start working on last or new project |
| `projects` / `ps` | Choose a project to work on |
| `web` | Start web-based Goose interface |
| `mcp <name>` | Run an enabled MCP server by name |
| `acp` | Run Goose as an ACP agent server over stdio |
| `help` | Display help menu |

### Global Switches

| Flag | Description |
|------|-------------|
| `--help` | Display help |
| `--version` | Display version |

### `session` Switches

| Flag | Description |
|------|-------------|
| `--session-id <id>` | Specify session by ID |
| `-n, --name <name>` | Give session a name |
| `-r, --resume` | Resume previous session |
| `--fork` | Fork session with copied history (requires `--resume`) |
| `--history` | Show previous messages when resuming |
| `--container <id>` | Run extensions in Docker container |
| `--debug` | Output complete tool responses |
| `--max-tool-repetitions <N>` | Prevent infinite tool loops |
| `--max-turns <N>` | Maximum turns without user input |
| `--with-extension <cmd>` | Add stdio extension |
| `--with-streamable-http-extension <url>` | Add remote HTTP extension |
| `--with-builtin <id>` | Enable built-in extension |

### `run` Switches

| Flag | Description |
|------|-------------|
| `-t, --text <TEXT>` | Input text directly |
| `-i, --instructions <FILE>` | Path to instruction file (use `-` for stdin) |
| `--system <TEXT>` | Additional system instructions |
| `--recipe <FILE>` | Load a recipe file |
| `--params <KEY=VALUE>` | Recipe parameters (repeatable) |
| `--sub-recipe <RECIPE>` | Include sub-recipes |
| `-s, --interactive` | Stay in interactive mode after run |
| `-n, --name <name>` | Name for the run session |
| `-r, --resume` | Resume from previous run |
| `--no-session` | Run without storing session |
| `--container <id>` | Run extensions in Docker container |
| `--debug` | Output complete tool responses |
| `--max-tool-repetitions <N>` | Prevent infinite tool loops |
| `--max-turns <N>` | Maximum turns allowed |
| `-q, --quiet` | Suppress non-response output |
| `--output-format <fmt>` | Output format: `text`, `json`, `stream-json` |
| `--provider <name>` | Override LLM provider |
| `--model <name>` | Override LLM model |
| `--explain` | Show recipe title and description |
| `--render-recipe` | Print recipe instead of running |
| `--with-extension <cmd>` | Add stdio extension |
| `--with-streamable-http-extension <url>` | Add remote HTTP extension |
| `--with-builtin <id>` | Enable built-in extension |

### `update` Switches

| Flag | Description |
|------|-------------|
| `-c, --canary` | Update to development/canary version |
| `-r, --reconfigure` | Reset configuration during update |

### `info` Switches

| Flag | Description |
|------|-------------|
| `-v, --verbose` | Show detailed config including env vars and extensions |

### `web` Switches

| Flag | Description |
|------|-------------|
| `-p, --port <PORT>` | Server port (default: 3000) |
| `--host <HOST>` | Bind address (default: 127.0.0.1) |
| `--open` | Auto-open browser on start |
| `--auth-token <TOKEN>` | Require authentication token |

### `schedule` Switches

| Flag | Description |
|------|-------------|
| `--schedule-id <NAME>` | Unique job identifier |
| `--cron <EXPR>` | Cron expression for timing |
| `--recipe-source <PATH>` | Recipe file path |
| `-l, --limit <N>` | Max sessions to display |

### `session list` Switches

| Flag | Description |
|------|-------------|
| `-f, --format <fmt>` | Output as `text` or `json` |
| `--ascending` | Sort oldest first |
| `-w, --working_dir <path>` | Filter by directory |
| `-l, --limit <N>` | Limit result count |

### `session remove` Switches

| Flag | Description |
|------|-------------|
| `--session-id <id>` | Remove specific session by ID |
| `-n, --name <name>` | Remove by name |
| `-r, --regex <pattern>` | Remove sessions matching regex pattern |

### `session export` Switches

| Flag | Description |
|------|-------------|
| `--session-id <id>` | Export specific session |
| `-n, --name <name>` | Export by name |
| `-o, --output <file>` | Save to file |
| `--format <fmt>` | Choose `markdown`, `json`, or `yaml` |


## Sources

- [Goose Homepage](https://block.github.io/goose/)
- [Goose GitHub Repository](https://github.com/block/goose)
- [CLI Commands Guide](https://block.github.io/goose/docs/guides/goose-cli-commands/)
- [Running Tasks (Non-interactive)](https://block.github.io/goose/docs/guides/running-tasks/)
- [Permission Modes](https://block.github.io/goose/docs/guides/goose-permissions/)
- [Configure LLM Providers](https://block.github.io/goose/docs/getting-started/providers/)
- [CLI Provider Configuration](https://block.github.io/goose/docs/guides/cli-providers/)
- [Environment Variables Reference](https://block.github.io/goose/docs/guides/environment-variables/)
- [Logging System](https://block.github.io/goose/docs/guides/logs/)
- [Configuration Files](https://block.github.io/goose/docs/guides/config-files)
- [Session Management](https://block.github.io/goose/docs/guides/sessions/session-management/)
- [Homebrew Formula](https://formulae.brew.sh/formula/block-goose-cli)
