# Kimi Code CLI

> Moonshot AI's open-source terminal-based AI coding agent.

- **Repository**: <https://github.com/MoonshotAI/kimi-cli>
- **Documentation**: <https://moonshotai.github.io/kimi-cli/en/>
- **Latest Release**: v1.39.0 (2026-04-24)
- **License**: Apache-2.0
- **Language**: Python (77.5%), TypeScript (21.5%)
- **Python Support**: 3.12–3.14 (3.13 recommended)
- **Package**: `kimi-cli` on PyPI, installed via `uv`

## Overview

Kimi Code CLI is an AI-powered terminal agent that assists with software development tasks and terminal operations. It can read and edit code, execute shell commands, search and fetch web pages, and autonomously plan and adjust actions during execution.

Three usage modes:

- **Interactive CLI (`kimi`)**: Chat with AI in the terminal using natural language or execute shell commands directly via built-in shell mode (`Ctrl-X` to toggle).
- **Browser UI (`kimi web`)**: Open a graphical interface in the local browser with session management, file references, code highlighting.
- **Agent integration (`kimi acp`)**: Run as an ACP service and integrate with IDEs (JetBrains, Zed, VS Code) via the [Agent Client Protocol](https://agentclientprotocol.com/).

## Installation

```bash
# Linux / macOS
curl -LsSf https://code.kimi.com/install.sh | bash

# Windows (PowerShell)
Invoke-RestMethod https://code.kimi.com/install.ps1 | Invoke-Expression

# Alternative (if uv already installed)
uv tool install --python 3.13 kimi-cli

# Verify
kimi --version

# Upgrade
uv tool upgrade kimi-cli --no-cache

# Uninstall
uv tool uninstall kimi-cli
```

## First Run

```bash
cd your-project
kimi
```

On first launch, run `/login` to configure API source. Recommended: **Kimi Code** (browser OAuth). Other platforms require an API key.

Generate project context with:

```
/init
```

This creates an `AGENTS.md` file for the project.

## Provider Types

Kimi Code CLI supports multiple LLM backends:

| Type | Description |
|------|-------------|
| `kimi` | Kimi API (Moonshot) |
| `openai_legacy` | OpenAI Chat Completions API (and compatible services) |
| `openai_responses` | OpenAI Responses API |
| `anthropic` | Anthropic Claude API |
| `gemini` | Google Gemini API |
| `vertexai` | Google Vertex AI |

All providers support `custom_headers` for attaching custom HTTP headers.

## Model Capabilities

Models declare capabilities in config:

| Capability | Description |
|-----------|-------------|
| `thinking` | Supports thinking mode (deep reasoning), toggleable |
| `always_thinking` | Always uses thinking mode (cannot be disabled) |
| `image_in` | Supports image input (`Ctrl-V` to paste) |
| `video_in` | Supports video input |

## Configuration

### Config File

Default location: `~/.kimi/config.toml` (also supports JSON format).

Specify custom config:

```bash
kimi --config-file /path/to/config.toml
kimi --config '{"default_model": "kimi-for-coding", ...}'
```

### Config Structure

```toml
default_model = "kimi-for-coding"
default_thinking = false
default_yolo = false
default_plan_mode = false
default_editor = ""
theme = "dark"
show_thinking_stream = true
merge_all_available_skills = true

[providers.kimi-for-coding]
type = "kimi"
base_url = "https://api.kimi.com/coding/v1"
api_key = "sk-xxx"

[models.kimi-for-coding]
provider = "kimi-for-coding"
model = "kimi-for-coding"
max_context_size = 262144

[loop_control]
max_steps_per_turn = 500
max_retries_per_step = 3
max_ralph_iterations = 0
reserved_context_size = 50000
compaction_trigger_ratio = 0.85

[background]
max_running_tasks = 4
keep_alive_on_exit = false
agent_task_timeout_s = 900

[services.moonshot_search]
base_url = "https://api.kimi.com/coding/v1/search"
api_key = "sk-xxx"

[services.moonshot_fetch]
base_url = "https://api.kimi.com/coding/v1/fetch"
api_key = "sk-xxx"

[mcp.client]
tool_call_timeout_ms = 60000
```

### Environment Variables

| Variable | Description |
|----------|-------------|
| `KIMI_BASE_URL` | Override provider `base_url` |
| `KIMI_API_KEY` | Override provider `api_key` |
| `KIMI_MODEL_NAME` | Override model identifier |
| `KIMI_MODEL_MAX_CONTEXT_SIZE` | Override max context size (tokens) |
| `KIMI_MODEL_CAPABILITIES` | Override capabilities (comma-separated: `thinking,image_in`) |
| `KIMI_MODEL_TEMPERATURE` | Generation temperature |
| `KIMI_MODEL_TOP_P` | Generation top_p |
| `KIMI_MODEL_MAX_TOKENS` | Max tokens per response |
| `KIMI_MODEL_THINKING_KEEP` | Moonshot preserved thinking (e.g., `all`) |
| `OPENAI_BASE_URL` | Override OpenAI provider base URL |
| `OPENAI_API_KEY` | Override OpenAI provider API key |
| `KIMI_SHARE_DIR` | Custom data directory (default: `~/.kimi`) |
| `KIMI_CLI_NO_AUTO_UPDATE` | Disable update features (`1`, `true`, `t`, `yes`, `y`) |
| `KIMI_CLI_PASTE_CHAR_THRESHOLD` | Character threshold for paste folding (default: `1000`) |
| `KIMI_CLI_PASTE_LINE_THRESHOLD` | Line threshold for paste folding (default: `15`) |

### Data Locations

All runtime data stored under `~/.kimi/` (or `KIMI_SHARE_DIR`):

| Path | Description |
|------|-------------|
| `config.toml` | Main configuration file |
| `mcp.json` | MCP server configuration |
| `logs/kimi.log` | Debug log output |
| `sessions/` | Session data |
| `plans/` | Plan mode files |

## Key Features

### Shell Command Mode

Press `Ctrl-X` to toggle between agent mode and shell mode. In shell mode, run shell commands directly without leaving the CLI.

### VS Code Extension

Available via the [Kimi Code VS Code Extension](https://marketplace.visualstudio.com/items?itemName=moonshot-ai.kimi-code).

### ACP Integration

Kimi Code CLI supports Agent Client Protocol out of the box:

```bash
kimi acp
```

Configure in Zed or JetBrains:

```json
{
  "agent_servers": {
    "Kimi Code CLI": {
      "type": "custom",
      "command": "kimi",
      "args": ["acp"],
      "env": {}
    }
  }
}
```

### Zsh Integration

```bash
git clone https://github.com/MoonshotAI/zsh-kimi-cli.git \
  ${ZSH_CUSTOM:-~/.oh-my-zsh/custom}/plugins/kimi-cli
```

Add `kimi-cli` to plugins in `~/.zshrc`, then use `Ctrl-X` to switch modes.

### MCP Support

```bash
# Add streamable HTTP server
kimi mcp add --transport http context7 https://mcp.context7.com/mcp --header "CONTEXT7_API_KEY: ctx7sk-your-key"

# Add streamable HTTP server with OAuth
kimi mcp add --transport http --auth oauth linear https://mcp.linear.app/mcp

# Add stdio server
kimi mcp add --transport stdio chrome-devtools -- npx chrome-devtools-mcp@latest

# List, remove, authorize, test
kimi mcp list
kimi mcp remove chrome-devtools
kimi mcp auth linear
kimi mcp test context7
kimi mcp reset-auth linear
```

### Agent Skills

Skills are discovered from:

- Project-level: `.kimi/skills/`, `.claude/skills/`, `.codex/skills/`, `.agents/skills/` (walks up to nearest `.git` ancestor)
- User-level: `~/.kimi/skills/`, `~/.claude/skills/`, `~/.codex/skills/`
- Extra: via `extra_skill_dirs` config or `--skills-dir` flag

Both `<name>/SKILL.md` subdirectory and single-file `<name>.md` layouts are supported.

Priority order: Project > User > Extra > Built-in.

### Hooks System (Beta)

Configure lifecycle hooks in `config.toml`:

```toml
[[hooks]]
event = "PreToolUse"
matcher = "Shell"
command = ".kimi/hooks/safety-check.sh"
timeout = 10
```

### Plan Mode

Start in plan mode to explore codebase (read-only tools only) and write an implementation plan:

```bash
kimi --plan
```

Or toggle at runtime with `/plan`.

### Subagent System

Three built-in subagent types: `coder`, `explore`, `plan`. Each maintains its own context history and can run in foreground or background.

### Background Tasks

Run long-running commands as background tasks:

- `Shell` tool with `run_in_background=true`
- Managed via `TaskList`, `TaskOutput`, `TaskStop` tools
- View via `/task` interactive browser

## CLI Switch Summary

### Basic Information

| Switch | Short | Description | Default |
|--------|-------|-------------|---------|
| `--version` | `-V` | Show version number and exit | — |
| `--help` | `-h` | Show help message and exit | — |
| `--verbose` | | Output detailed runtime information | off |
| `--debug` | | Log debug info to `~/.kimi/logs/kimi.log` | off |

Example:

```bash
kimi --version
kimi -V
kimi --debug
```

### Agent Configuration

| Switch | Description | Default |
|--------|-------------|---------|
| `--agent NAME` | Use built-in agent (`default`, `okabe`) | `default` |
| `--agent-file PATH` | Use custom agent file | — |

`--agent` and `--agent-file` are mutually exclusive.

Example:

```bash
kimi --agent okabe
kimi --agent-file ./my-agent.toml
```

### Configuration Files

| Switch | Description | Default |
|--------|-------------|---------|
| `--config STRING` | Load TOML/JSON config string | — |
| `--config-file PATH` | Load config file | `~/.kimi/config.toml` |

`--config` and `--config-file` are mutually exclusive.

Example:

```bash
kimi --config-file ./custom-config.toml
kimi --config '{"default_model": "kimi-for-coding"}'
```

### Model Selection

| Switch | Short | Description | Default |
|--------|-------|-------------|---------|
| `--model NAME` | `-m` | Specify LLM model (overrides config) | config's `default_model` |

Example:

```bash
kimi -m kimi-k2-thinking-turbo
kimi --model gpt-4.1
```

### Working Directory

| Switch | Short | Description | Default |
|--------|-------|-------------|---------|
| `--work-dir PATH` | `-w` | Working directory root for file operations | current directory |
| `--add-dir PATH` | | Add additional directory to workspace scope (repeatable) | — |

Example:

```bash
kimi -w /path/to/project
kimi --add-dir /path/to/shared-lib --add-dir /path/to/other-project
```

### Session Management

| Switch | Short | Description | Default |
|--------|-------|-------------|---------|
| `--continue` | `-C` | Continue previous session in current working directory | — |
| `--session [ID]` | `-S` | Resume session. With ID: resume that session. Without ID: interactive picker (shell only) | — |
| `--resume [ID]` | `-r` | Alias for `--session` | — |

`--continue` and `--session`/`--resume` are mutually exclusive.

Example:

```bash
kimi -C
kimi -S abc123
kimi -r
kimi --session
```

### Input and Commands

| Switch | Short | Description | Default |
|--------|-------|-------------|---------|
| `--prompt TEXT` | `-p` | Pass user prompt (non-interactive, exits after processing) | — |
| `--command TEXT` | `-c` | Alias for `--prompt` | — |

Example:

```bash
kimi -p "List all TypeScript files in this project"
kimi --command "Explain the main entry point"
```

### Loop Control

| Switch | Description | Default |
|--------|-------------|---------|
| `--max-steps-per-turn N` | Max steps per turn (overrides config `loop_control.max_steps_per_turn`) | `500` |
| `--max-retries-per-step N` | Max retries per step (overrides config `loop_control.max_retries_per_step`) | `3` |
| `--max-ralph-iterations N` | Ralph Loop iterations; `0` disables, `-1` unlimited | `0` |

Ralph Loop automatically re-feeds the same prompt to iterate on a big task until the agent outputs `<choice>STOP</choice>` or the iteration limit is reached.

Example:

```bash
kimi --max-steps-per-turn 1000
kimi --max-ralph-iterations -1
```

### UI Modes

| Switch | Description | Default |
|--------|-------------|---------|
| `--print` | Print mode (non-interactive); implicitly enables `--yolo` | — |
| `--quiet` | Shortcut for `--print --output-format text --final-message-only` | — |
| `--acp` | ACP server mode (deprecated; use `kimi acp` instead) | — |
| `--wire` | Wire server mode (experimental) | — |

These four options are mutually exclusive. Default is interactive shell mode.

Example:

```bash
kimi --print -p "Fix the failing tests"
kimi --quiet -p "What does this project do?"
```

### Print Mode Options

Only effective with `--print`:

| Switch | Description | Default |
|--------|-------------|---------|
| `--input-format FORMAT` | Input format: `text` or `stream-json` | `text` |
| `--output-format FORMAT` | Output format: `text` or `stream-json` | `text` |
| `--final-message-only` | Only output the final assistant message | off |

`stream-json` uses JSONL (one JSON object per line) for programmatic integration.

Example:

```bash
kimi --print --output-format stream-json -p "Explain main()"
kimi --print --final-message-only -p "Summarize the codebase"
```

### MCP Configuration

| Switch | Description | Default |
|--------|-------------|---------|
| `--mcp-config-file PATH` | Load MCP config file (repeatable) | `~/.kimi/mcp.json` if exists |
| `--mcp-config JSON` | Load MCP config JSON string (repeatable) | — |

Example:

```bash
kimi --mcp-config-file ./mcp-servers.json
kimi --mcp-config '{"mcpServers":{"context7":{"url":"https://mcp.context7.com/mcp"}}}'
```

### Approval Control

| Switch | Short | Description | Default |
|--------|-------|-------------|---------|
| `--yolo` | `-y` | Auto-approve all operations | off |
| `--yes` | | Alias for `--yolo` | off |
| `--auto-approve` | | Alias for `--yolo` | off |

In YOLO mode, all file modifications and shell commands are automatically executed without confirmation.

Example:

```bash
kimi -y
kimi --yolo -p "Refactor all imports"
```

### Plan Mode

| Switch | Description | Default |
|--------|-------------|---------|
| `--plan` | Start new session in plan mode (read-only tools only) | off |

Can also be set via `default_plan_mode = true` in config.

Example:

```bash
kimi --plan
kimi --plan -p "Design a REST API for user management"
```

### Thinking Mode

| Switch | Description | Default |
|--------|-------------|---------|
| `--thinking` | Enable thinking mode | last session's setting |
| `--no-thinking` | Disable thinking mode | last session's setting |

Thinking mode requires model support (declared via `thinking` or `always_thinking` capability).

Example:

```bash
kimi --thinking
kimi --no-thinking
```

### Skills Configuration

| Switch | Description | Default |
|--------|-------------|---------|
| `--skills-dir PATH` | Append additional skills directories (repeatable) | auto-discovered |

When specified, replaces default user/project skill discovery.

Example:

```bash
kimi --skills-dir ./custom-skills
kimi --skills-dir ./team-skills --skills-dir ./personal-skills
```

## Subcommands

| Subcommand | Description |
|-----------|-------------|
| `kimi login` | Log in to Kimi account (opens browser for OAuth) |
| `kimi logout` | Log out (clears OAuth credentials) |
| `kimi info` | Display version and protocol info |
| `kimi acp` | Start multi-session ACP server |
| `kimi mcp` | Manage MCP server configuration |
| `kimi plugin` | Manage plugins (Beta) |
| `kimi term` | Launch Toad terminal UI |
| `kimi export` | Export session as ZIP file |
| `kimi vis` | Launch Agent Tracing Visualizer (Technical Preview) |
| `kimi web` | Start Web UI server |

### `kimi info`

```bash
kimi info [--json]
```

Output includes: `kimi_cli_version`, `agent_spec_versions`, `wire_protocol_version`, `python_version`.

### `kimi acp`

```bash
kimi acp
```

Starts ACP server for IDE integration. Requires prior login via `/login` or `kimi login`.

### `kimi mcp`

```bash
kimi mcp add [OPTIONS] NAME [TARGET_OR_COMMAND...]
kimi mcp list
kimi mcp remove NAME
kimi mcp auth NAME
kimi mcp reset-auth NAME
kimi mcp test NAME
```

`add` options:

| Option | Short | Description |
|--------|-------|-------------|
| `--transport TYPE` | `-t` | `stdio` (default) or `http` |
| `--env KEY=VALUE` | `-e` | Environment variable (stdio only, repeatable) |
| `--header KEY:VALUE` | `-H` | HTTP header (http only, repeatable) |
| `--auth TYPE` | `-a` | Auth type (e.g., `oauth`, http only) |

### `kimi web`

```bash
kimi web [OPTIONS]
```

| Option | Short | Description | Default |
|--------|-------|-------------|---------|
| `--host TEXT` | `-h` | Host to bind | `127.0.0.1` |
| `--network` | `-n` | Bind to `0.0.0.0` with LAN IP display | off |
| `--port INTEGER` | `-p` | Port number | `5494` |
| `--reload` | | Enable auto-reload (dev mode) | off |
| `--open` / `--no-open` | | Auto-open browser | enabled |

### `kimi vis`

```bash
kimi vis [OPTIONS]
```

| Option | Short | Description | Default |
|--------|-------|-------------|---------|
| `--host TEXT` | `-h` | Host to bind | `127.0.0.1` |
| `--network` | `-n` | Bind to `0.0.0.0` with LAN IP display | off |
| `--port INTEGER` | `-p` | Port number | `5495` |
| `--open` / `--no-open` | | Auto-open browser | enabled |
| `--reload` | | Enable auto-reload (dev mode) | off |

### `kimi export`

```bash
kimi export [<session_id>] [-o <output_path>] [--yes]
```

| Argument/Option | Short | Description |
|----------------|-------|-------------|
| `<session_id>` | | Session ID to export (defaults to previous session for CWD) |
| `--output PATH` | `-o` | Output ZIP path (defaults to `session-<id>.zip`) |
| `--yes` | `-y` | Skip confirmation prompt |

## Slash Commands

| Command | Alias | Description |
|---------|-------|-------------|
| `/help` | `/h`, `/?` | Display help, keyboard shortcuts, and loaded skills |
| `/version` | | Display version |
| `/changelog` | `/release-notes` | Display recent changelog |
| `/feedback` | | Submit feedback |
| `/login` | `/setup` | Log in / configure API platform |
| `/logout` | | Log out from current platform |
| `/model` | | Switch model and thinking mode interactively |
| `/editor` | | Set external editor (e.g., `/editor vim`) |
| `/theme` | | Switch color theme (`dark` or `light`) |
| `/reload` | | Reload configuration file |
| `/debug` | | Display debug info (messages, tokens, checkpoints) |
| `/usage` | `/status` | Display API usage and quota (Kimi Code platform only) |
| `/mcp` | | Display connected MCP servers and tools |
| `/hooks` | | Display configured hooks |
| `/new` | | Create new session |
| `/sessions` | `/resume` | List and switch sessions (`Ctrl-A` toggles scope) |
| `/title [TEXT]` | `/rename` | View or set session title (max 200 chars) |
| `/undo` | | Roll back to a previous turn (fork + re-edit) |
| `/fork` | | Fork current session |
| `/export [PATH]` | | Export session to Markdown |
| `/import <PATH or ID>` | | Import context from file or session |
| `/clear` | `/reset` | Clear session context |
| `/compact [INSTRUCTIONS]` | | Manually compact context |
| `/skill:<name> [TEXT]` | | Load a skill as prompt |
| `/flow:<name>` | | Execute a flow skill |
| `/add-dir [PATH]` | | Add directory to workspace scope |
| `/btw <QUESTION>` | | Quick side question without interrupting conversation |
| `/init` | | Generate `AGENTS.md` from project analysis |
| `/plan` | | Toggle plan mode (`on`/`off`/`view`/`clear`) |
| `/task` | | Open interactive task browser for background tasks |
| `/yolo` | | Toggle YOLO (auto-approve) mode |
| `/web` | | Switch to Web UI |
| `/vis` | | Switch to Agent Tracing Visualizer |

## Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `Ctrl-X` | Toggle between agent and shell mode |
| `Ctrl-O` | Open external editor for input |
| `Ctrl-V` | Paste image (when model supports `image_in`) |
| `Ctrl-C` | Cancel / interrupt current operation |
| `Ctrl-D` | Exit |
| `Ctrl-S` | Inject message into running turn (steer) |
| `Enter` | Queue message for delivery after current turn (during streaming) |

## Recent Release Highlights

### v1.39.0 (2026-04-24)

- Skill scope priority fix: project skills now correctly override user/built-in skills
- Single-file `<name>.md` skills support alongside `<name>/SKILL.md` directories
- `extra_skill_dirs` config field for custom skill directories
- DeepSeek V4 thinking-mode fix (OpenAI-compatible backends)
- `KIMI_MODEL_THINKING_KEEP` env var for Moonshot Preserved Thinking
- `merge_all_available_skills` default changed to `true`

### v1.37.0 (2026-04-20)

- Print mode waits for background tasks before exiting
- Auto-refresh managed model list at startup for OAuth users
- Provider-supplied `display_name` shown across UI

### v1.36.0 (2026-04-17)

- Claude Opus 4.7 adaptive thinking fix
- Default `max_steps_per_turn` increased from 100 to 500

### v1.31.0 (2026-04-10)

- `/btw` side question command
- Queue and steer dual-channel input during streaming
- `/undo` and `/fork` session forking commands
- `--plan` flag and `default_plan_mode` config
- `--session`/`--resume` flag for session resumption
- Hierarchical `AGENTS.md` loading

### v1.28.0 (2026-03-30)

- Hooks system (Beta) with 13 lifecycle events
- Dark/light theme support (`/theme`)
- Agent Tracing Visualizer (`kimi vis`) improvements

### v1.25.0 (2026-03-23)

- Plugin system (Skills + Tools)
- `Agent` tool for subagent delegation (`coder`, `explore`, `plan`)
- Interactive approval request panel with reject-with-feedback
- Background task system

## ACP Integration Details

Kimi Code CLI implements the [Agent Client Protocol](https://agentclientprotocol.com/) (ACP) for IDE integration:

- Wire protocol version: 1.7+
- Agent spec versions: 1
- Start ACP server: `kimi acp`
- Authentication: Checks login status; returns `AUTH_REQUIRED` (code `-32000`) if not logged in
- Supported IDEs: Zed, JetBrains, VS Code (via extension)

## Wire Protocol

The Wire protocol is Kimi Code CLI's internal event streaming protocol for real-time communication between the agent and clients (Web UI, ACP, Vis). Key event types include:

- `TurnBegin` / `TurnEnd`
- `ToolCallRequest` / `ToolResult`
- `ApprovalRequest` / `ApprovalResponse`
- `SubagentEvent`
- `HookTriggered` / `HookResolved`
- `PlanDisplay`
- `SteerInput`
- `BtwBegin` / `BtwEnd`
- `StatusUpdate` (with token counts, cache hit rate)
- `MCPLoadingBegin` / `MCPLoadingEnd`
- `QuestionRequest`
- `Notification`

## Tools Available to Agent

The agent has access to built-in tools:

| Tool | Description |
|------|-------------|
| `ReadFile` | Read file contents (supports negative offsets for tail mode) |
| `WriteFile` | Write or create files |
| `StrReplaceFile` | Surgical string replacement in files |
| `Glob` | Find files by glob pattern |
| `Grep` | Search file contents (ripgrep-based) |
| `Shell` | Execute shell commands (supports background mode) |
| `Agent` | Spawn subagent instances (coder, explore, plan) |
| `SearchWeb` | Web search (requires Moonshot search service) |
| `FetchURL` | Fetch web page content (requires Moonshot fetch service or falls back to local) |
| `AskUserQuestion` | Ask user for input |
| `EnterPlanMode` / `ExitPlanMode` | Toggle plan mode |
| `SetTodoList` | Manage task lists |
| `TaskList` / `TaskOutput` / `TaskStop` | Manage background tasks |
| MCP tools | Dynamically loaded from configured MCP servers |
| Plugin tools | Dynamically loaded from installed plugins |
