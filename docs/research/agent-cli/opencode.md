---
homepage: https://opencode.ai
docs: https://opencode.ai/docs/
cli_docs: https://opencode.ai/docs/cli/
repo: https://github.com/anomalyco/opencode
---

# OpenCode CLI

OpenCode is an open-source AI coding agent available as a terminal TUI, desktop app, IDE extension, or web interface. It supports 75+ LLM providers via Models.dev and can run local models through Ollama, LM Studio, llama.cpp, and more. Binary name: `opencode`. Configuration lives in `opencode.json` / `opencode.jsonc` (JSON/JSONC format) at global (`~/.config/opencode/`) or project level.

Repository: https://github.com/anomalyco/opencode

> **Note:** As of April 2026, OpenCode has 151k+ GitHub stars, 850+ contributors, and 6.5M+ monthly developers. Current version: v1.14.28.

---

## Model Specification

**CLI flag:** `-m` / `--model <provider/model>`

```bash
opencode -m anthropic/claude-sonnet-4-5 "refactor the auth module"
opencode run -m openai/gpt-5 "summarize the repo"
```

**Config file (`opencode.json`):**

```json
{
  "$schema": "https://opencode.ai/config.json",
  "model": "anthropic/claude-sonnet-4-5",
  "small_model": "anthropic/claude-haiku-4-5"
}
```

**Default model:** No default — must be configured via `/connect` or config. The `small_model` option configures a cheaper model for lightweight tasks (title generation, etc.); falls back to the main model if unset.

**Model format:** `provider/model-id` (e.g., `anthropic/claude-sonnet-4-5`, `opencode/gpt-5.1-codex`, `ollama/llama2`).

**Interactive switching:** Use the `/models` slash command during an active TUI session to change the model mid-session.

**Local models:** Configure via `provider` section with `@ai-sdk/openai-compatible` for any OpenAI-compatible endpoint (Ollama, LM Studio, llama.cpp).

**Provider options:**

```json
{
  "provider": {
    "anthropic": {
      "options": {
        "timeout": 600000,
        "chunkTimeout": 30000,
        "setCacheKey": true
      }
    }
  }
}
```

- `timeout` — Request timeout in ms (default: 300000). Set to `false` to disable.
- `chunkTimeout` — Timeout between streamed chunks in ms.
- `setCacheKey` — Ensure a cache key is always set for designated provider.

---

## Non-interactive Engagement

Non-interactive mode is supported through the `opencode run` subcommand.

### 1. `opencode run` with inline prompt

```bash
opencode run "summarize the repository structure"
opencode run "generate release notes" | tee release-notes.md
```

### 2. Attach to a running server

```bash
# Start a headless server in one terminal
opencode serve

# In another terminal, run commands that attach to it
opencode run --attach http://localhost:4096 "Explain async/await in JavaScript"
```

### 3. JSON output (`--format json`)

Produces machine-readable JSON events suitable for programmatic consumption.

```bash
opencode run --format json "summarize the repo" | jq
```

### 4. File attachments (`--file` / `-f`)

Attach files to the prompt for context.

```bash
opencode run -f schema.sql -f README.md "explain the database design"
```

### 5. Session continuation

Continue the last session or a specific session non-interactively:

```bash
opencode run -c "fix the race conditions you found"
opencode run -s <SESSION_ID> "apply the suggested changes"
opencode run --fork -c "try a different approach"
```

### 6. Share after run (`--share`)

```bash
opencode run --share "explain the architecture"
```

### 7. Skip permissions (`--dangerously-skip-permissions`)

Auto-approve permissions that are not explicitly denied. Dangerous — only for sandboxed environments.

```bash
opencode run --dangerously-skip-permissions "run the full test suite"
```

---

## Subscription versus Per Call API

OpenCode supports multiple billing models depending on provider:

**OpenCode Zen:** Curated models tested by the OpenCode team. Sign up at opencode.ai/auth, generate an API key, and connect via `/connect`.

**OpenCode Go:** Low-cost subscription plan for popular open coding models.

**ChatGPT Plus/Pro:** Log in with OpenAI credentials via `/connect` to use your ChatGPT subscription.

**GitHub Copilot:** Log in with GitHub to use Copilot models via device auth flow.

**GitLab Duo:** Use GitLab Duo Agent Platform with Premium/Ultimate subscription (OAuth or PAT).

**Per-call API keys:** Configure any provider's API key via `/connect`, environment variables, or `opencode.json`. Billed at standard per-token rates.

**Local models:** Free — Ollama, LM Studio, llama.cpp, etc.

---

## System Prompt / Instructions

OpenCode uses a layered instruction system built from multiple sources:

### AGENTS.md (primary project instructions)

OpenCode discovers and concatenates `AGENTS.md` files walking from the project root. Initialize via `/init`.

- **Global scope:** `AGENTS.md` in project root
- Initialize with `/init` — analyzes project and creates appropriate `AGENTS.md`

### instructions (config)

Additional instruction files and glob patterns:

```json
{
  "instructions": ["CONTRIBUTING.md", "docs/guidelines.md", ".cursor/rules/*.md"]
}
```

### Rules

Place rules in `.opencode/rules/` or configure via `opencode.json`. Rules provide additional context and constraints.

### Agent Skills

SKILL.md files in `.opencode/skills/`, `.claude/skills/`, or `~/.config/opencode/skills/`. Loaded on-demand via the `skill` tool.

### Config precedence

1. Remote config (`.well-known/opencode`) — organizational defaults
2. Global config (`~/.config/opencode/opencode.json`) — user preferences
3. Custom config (`OPENCODE_CONFIG` env var)
4. Project config (`opencode.json` in project)
5. `.opencode` directories — agents, commands, plugins
6. Inline config (`OPENCODE_CONFIG_CONTENT` env var)
7. Managed config files (system-level, admin-controlled)
8. macOS managed preferences (`.mobileconfig` via MDM) — highest priority

---

## Permissions

OpenCode uses a permission system to control what agents can do. By default, **all operations are allowed** without approval.

### Permission actions

Each permission key can be set to:

| Value | Behavior |
|---|---|
| `"allow"` | Allow without approval |
| `"ask"` | Prompt for approval before running |
| `"deny"` | Disable the tool entirely |

### Permission keys

| Key | Tools gated |
|---|---|
| `read` | `read` |
| `edit` | `write`, `edit`, `apply_patch` |
| `glob` | `glob` |
| `grep` | `grep` |
| `list` | `list` |
| `bash` | `bash` |
| `task` | `task` |
| `external_directory` | Any tool that reads/writes outside the project worktree |
| `todowrite` | `todowrite`, `todoread` |
| `webfetch` | `webfetch` |
| `websearch` | `websearch` |
| `codesearch` | `codesearch` |
| `lsp` | `lsp` |
| `skill` | `skill` |
| `question` | `question` |
| `doom_loop` | Recovery prompts when agent appears stuck |

Fine-grained bash permissions with glob patterns:

```json
{
  "permission": {
    "bash": {
      "*": "ask",
      "git status *": "allow",
      "git push": "ask"
    }
  }
}
```

Per-agent overrides:

```json
{
  "agent": {
    "build": {
      "permission": {
        "edit": "ask"
      }
    }
  }
}
```

---

## Agents

OpenCode includes built-in agents and supports custom agents.

### Built-in agents

| Agent | Mode | Description |
|---|---|---|
| **build** | primary | Default agent with all tools enabled. Full development work. |
| **plan** | primary | Read-only agent for analysis. Edit and bash set to `ask` by default. Switch with **Tab** key. |
| **general** | subagent | General-purpose agent for complex searches and multi-step tasks. Full tool access. Invoke with `@general`. |
| **explore** | subagent | Fast, read-only agent for codebase exploration. Cannot modify files. |
| **compaction** | primary (hidden) | System agent for context compaction. |
| **title** | primary (hidden) | System agent for session title generation. |
| **summary** | primary (hidden) | System agent for session summaries. |

### Custom agents

Define via JSON in `opencode.json` or markdown files in `~/.config/opencode/agents/` or `.opencode/agents/`.

Create interactively:

```bash
opencode agent create
```

Agent options: `description` (required), `mode`, `model`, `prompt`, `temperature`, `top_p`, `steps` (max agentic iterations), `permission`, `hidden`, `color`, `disable`.

### Default agent

```json
{
  "default_agent": "plan"
}
```

---

## ACP Support

OpenCode supports the Agent Client Protocol (ACP) for use in compatible editors (Zed, JetBrains, Neovim).

```bash
opencode acp
```

This starts an ACP server communicating via stdin/stdout using nd-JSON. Compatible with Zed, JetBrains IDEs, Avante.nvim, CodeCompanion.nvim, and other ACP-compatible editors.

---

## Logging

OpenCode stores data in:

- **Config:** `~/.config/opencode/` (global), `opencode.json` (project)
- **Auth:** `~/.local/share/opencode/auth.json`
- **TUI config:** `tui.json` (alongside `opencode.json`)

---

## CLI Switch Summary

### Subcommands

| Subcommand | Description |
|---|---|
| *(default)* | Launch the interactive TUI |
| `run` | Run non-interactively with a prompt |
| `serve` | Start a headless HTTP API server |
| `web` | Start a headless server with web UI |
| `acp` | Start an ACP server (stdin/stdout nd-JSON) |
| `attach` | Attach TUI to a running backend |
| `agent` | Manage agents |
| `agent create` | Create a new custom agent |
| `agent list` | List all available agents |
| `auth` | Manage credentials and login |
| `auth login` | Configure API keys for providers |
| `auth list` / `auth ls` | List authenticated providers |
| `auth logout` | Clear provider credentials |
| `github` | Manage the GitHub agent |
| `github install` | Install GitHub Actions workflow |
| `github run` | Run the GitHub agent (CI/CD) |
| `mcp` | Manage MCP servers |
| `mcp add` | Add an MCP server |
| `mcp list` / `mcp ls` | List MCP servers and status |
| `mcp auth` | Authenticate with OAuth MCP server |
| `mcp auth list` / `mcp auth ls` | List OAuth-capable servers |
| `mcp logout` | Remove MCP OAuth credentials |
| `mcp debug` | Debug OAuth connection issues |
| `models` | List available models from providers |
| `session` | Manage sessions |
| `session list` | List all sessions |
| `stats` | Show token usage and cost statistics |
| `export` | Export session data as JSON |
| `import` | Import session from JSON file or share URL |
| `uninstall` | Uninstall OpenCode and remove files |
| `upgrade` | Update to latest or specific version |

### TUI (default command)

```bash
opencode [project]
```

#### `--continue` / `-c`

Continue the last session.

- **Default:** Starts a new session
- **Example:**

```bash
opencode -c
```

#### `--session` / `-s` `<SESSION_ID>`

Continue a specific session by ID.

- **Default:** None
- **Example:**

```bash
opencode -s abc123-def456
```

#### `--fork`

Fork the session when continuing (use with `--continue` or `--session`). Creates a new session from the existing one.

- **Default:** Off (continues in-place)
- **Example:**

```bash
opencode -c --fork
opencode -s abc123 --fork
```

#### `--prompt` `<text>`

Provide an initial prompt to the session.

- **Default:** None (launches empty TUI)
- **Example:**

```bash
opencode --prompt "Explain the auth module"
```

#### `--model` / `-m` `<provider/model>`

Override the model for this session.

- **Default:** Value from `opencode.json` config
- **Example:**

```bash
opencode -m anthropic/claude-sonnet-4-5
opencode -m opencode/gpt-5.1-codex
```

#### `--agent` `<name>`

Specify which agent to use.

- **Default:** `build`
- **Example:**

```bash
opencode --agent plan
opencode --agent code-reviewer
```

#### `--port` `<number>`

Port for the local server to listen on.

- **Default:** Random port
- **Example:**

```bash
opencode --port 4096
```

#### `--hostname` `<address>`

Hostname for the local server to listen on.

- **Default:** localhost
- **Example:**

```bash
opencode --hostname 0.0.0.0 --port 4096
```

### `opencode run`

Run non-interactively with a prompt.

```bash
opencode run [message..]
```

#### `--command`

The command to run; use message for args.

- **Default:** None
- **Example:**

```bash
opencode run --command "fix lint errors"
```

#### `--continue` / `-c`

Continue the last session.

- **Default:** New session
- **Example:**

```bash
opencode run -c "continue fixing the tests"
```

#### `--session` / `-s` `<SESSION_ID>`

Continue a specific session.

- **Default:** None
- **Example:**

```bash
opencode run -s abc123 "apply the suggested changes"
```

#### `--fork`

Fork the session when continuing.

- **Default:** Off
- **Example:**

```bash
opencode run --fork -c "try a different approach"
```

#### `--share`

Share the session after completion.

- **Default:** Off
- **Example:**

```bash
opencode run --share "explain the architecture"
```

#### `--model` / `-m` `<provider/model>`

Model to use.

- **Default:** Value from config
- **Example:**

```bash
opencode run -m openai/gpt-5 "summarize the repo"
```

#### `--agent` `<name>`

Agent to use for this run.

- **Default:** `build`
- **Example:**

```bash
opencode run --agent plan "analyze the codebase"
```

#### `--file` / `-f` `<path>`

File(s) to attach to the message. Repeatable.

- **Default:** No files
- **Example:**

```bash
opencode run -f schema.sql -f README.md "explain the database design"
```

#### `--format` `<default|json>`

Output format. Use `json` for machine-readable output.

- **Default:** `default` (formatted text)
- **Example:**

```bash
opencode run --format json "summarize" | jq
```

#### `--title` `<text>`

Title for the session. Uses truncated prompt if no value provided.

- **Default:** Auto-generated from prompt
- **Example:**

```bash
opencode run --title "Auth refactor" "refactor the auth module"
```

#### `--attach` `<url>`

Attach to a running `opencode serve` instance to avoid MCP cold boot.

- **Default:** None (standalone)
- **Example:**

```bash
opencode run --attach http://localhost:4096 "fix the bug"
```

#### `--port` `<number>`

Port for the local server (defaults to random).

- **Default:** Random
- **Example:**

```bash
opencode run --port 4096 "explain closures"
```

#### `--dangerously-skip-permissions`

Auto-approve permissions not explicitly denied. Dangerous.

- **Default:** Off
- **Example:**

```bash
opencode run --dangerously-skip-permissions "run full test suite"
```

#### Positional: `message`

The prompt to send.

```bash
opencode run "Explain how closures work in JavaScript"
```

### `opencode serve`

Start a headless HTTP API server.

```bash
opencode serve
```

#### `--port` `<number>`

Port to listen on.

- **Default:** 4096
- **Example:**

```bash
opencode serve --port 4096
```

#### `--hostname` `<address>`

Hostname to listen on.

- **Default:** localhost
- **Example:**

```bash
opencode serve --hostname 0.0.0.0
```

#### `--mdns`

Enable mDNS service discovery for other devices on the network.

- **Default:** Off
- **Example:**

```bash
opencode serve --mdns
```

#### `--cors` `<origin>`

Additional browser origin(s) to allow CORS. Repeatable.

- **Default:** None
- **Example:**

```bash
opencode serve --cors http://localhost:5173
```

### `opencode web`

Start a headless server with a web browser interface.

```bash
opencode web
```

Same flags as `serve` (`--port`, `--hostname`, `--mdns`, `--cors`).

### `opencode acp`

Start an ACP server communicating via stdin/stdout using nd-JSON.

```bash
opencode acp
```

#### `--cwd` `<path>`

Working directory.

- **Default:** Current directory
- **Example:**

```bash
opencode acp --cwd ~/projects/my-app
```

#### `--port` `<number>`

Port to listen on (for HTTP mode instead of stdio).

- **Default:** stdio mode
- **Example:**

```bash
opencode acp --port 4096
```

#### `--hostname` `<address>`

Hostname to listen on.

- **Default:** localhost
- **Example:**

```bash
opencode acp --hostname 0.0.0.0
```

### `opencode attach`

Attach a terminal TUI to an already running backend.

```bash
opencode attach [url]
```

#### `--dir` `<path>`

Working directory to start the TUI in.

- **Default:** Current directory
- **Example:**

```bash
opencode attach --dir ~/projects/my-app http://10.20.30.40:4096
```

#### `--session` / `-s` `<SESSION_ID>`

Session ID to continue.

- **Default:** None
- **Example:**

```bash
opencode attach -s abc123 http://localhost:4096
```

### `opencode agent create`

Create a new agent with custom configuration.

```bash
opencode agent create
```

#### `--path` `<dir>`

Directory to write the agent file to.

- **Default:** Global or `.opencode/agent` based on prompt

#### `--description` `<text>`

What the agent should do.

#### `--mode` `<all|primary|subagent>`

Agent mode.

- **Default:** Interactive prompt

#### `--permissions` / `--tools` `<comma-separated>`

Permissions to allow. Available: `bash`, `read`, `edit`, `glob`, `grep`, `webfetch`, `task`, `todowrite`, `websearch`, `codesearch`, `lsp`, `skill`.

- **Default:** All permissions (interactive prompt)

#### `--model` / `-m` `<provider/model>`

Model to use.

- **Default:** Interactive prompt

Passing all of `--path`, `--description`, `--mode`, and `--permissions` runs the command non-interactively.

```bash
opencode agent create --path .opencode/agents/review.md \
  --description "Reviews code for quality" \
  --mode subagent \
  --permissions read,glob,grep
```

### `opencode auth login`

Configure API keys for LLM providers.

```bash
opencode auth login
```

No flags. Interactive prompt to select provider and enter API key. Stored in `~/.local/share/opencode/auth.json`.

### `opencode auth list` / `opencode auth ls`

List all authenticated providers.

```bash
opencode auth list
```

### `opencode auth logout`

Clear provider credentials.

```bash
opencode auth logout
```

### `opencode github install`

Install the GitHub agent in your repository. Sets up GitHub Actions workflow.

```bash
opencode github install
```

### `opencode github run`

Run the GitHub agent (typically in CI/CD).

```bash
opencode github run
```

#### `--event` `<type>`

GitHub mock event to run the agent for.

#### `--token` `<token>`

GitHub personal access token.

### `opencode mcp add`

Add an MCP server to configuration.

```bash
opencode mcp add
```

Interactive — guides through adding local or remote MCP servers.

### `opencode mcp list` / `opencode mcp ls`

List configured MCP servers and connection status.

```bash
opencode mcp list
```

### `opencode mcp auth [name]`

Authenticate with an OAuth-enabled MCP server. If no name given, prompts to select from available servers.

```bash
opencode mcp auth jira
opencode mcp auth list   # list OAuth-capable servers
```

### `opencode mcp logout [name]`

Remove OAuth credentials for an MCP server.

```bash
opencode mcp logout jira
```

### `opencode mcp debug <name>`

Debug OAuth connection issues for an MCP server.

```bash
opencode mcp debug jira
```

### `opencode models`

List all available models from configured providers.

```bash
opencode models [provider]
```

#### `--refresh`

Refresh the models cache from models.dev.

- **Default:** Uses cached model list
- **Example:**

```bash
opencode models --refresh
opencode models anthropic
```

#### `--verbose`

Include metadata like costs in the output.

- **Default:** Compact output
- **Example:**

```bash
opencode models --verbose
```

### `opencode session list`

List all sessions.

```bash
opencode session list
```

#### `--max-count` / `-n` `<N>`

Limit to N most recent sessions.

- **Default:** All sessions
- **Example:**

```bash
opencode session list -n 10
```

#### `--format` `<table|json>`

Output format.

- **Default:** `table`
- **Example:**

```bash
opencode session list --format json
```

### `opencode stats`

Show token usage and cost statistics.

```bash
opencode stats
```

#### `--days` `<N>`

Show stats for the last N days.

- **Default:** All time
- **Example:**

```bash
opencode stats --days 7
```

#### `--tools` `<N>`

Number of tools to show.

- **Default:** All tools
- **Example:**

```bash
opencode stats --tools 10
```

#### `--models` `[N]`

Show model usage breakdown. Pass a number for top N.

- **Default:** Hidden
- **Example:**

```bash
opencode stats --models 5
```

#### `--project` `<path>`

Filter by project. Empty string for current project.

- **Default:** All projects
- **Example:**

```bash
opencode stats --project ""
```

### `opencode export`

Export session data as JSON.

```bash
opencode export [sessionID]
```

If no session ID provided, prompts to select from available sessions.

### `opencode import`

Import session data from a JSON file or share URL.

```bash
opencode import <file>
opencode import https://opncd.ai/s/abc123
```

### `opencode uninstall`

Uninstall OpenCode and remove all related files.

```bash
opencode uninstall
```

#### `--keep-config` / `-c`

Keep configuration files.

- **Default:** Removes everything
- **Example:**

```bash
opencode uninstall --keep-config
```

#### `--keep-data` / `-d`

Keep session data and snapshots.

- **Default:** Removes everything
- **Example:**

```bash
opencode uninstall --keep-data
```

#### `--dry-run`

Show what would be removed without removing.

- **Default:** Off
- **Example:**

```bash
opencode uninstall --dry-run
```

#### `--force` / `-f`

Skip confirmation prompts.

- **Default:** Prompts before removing
- **Example:**

```bash
opencode uninstall --force
```

### `opencode upgrade`

Update to latest or specific version.

```bash
opencode upgrade [target]
```

#### `--method` / `-m` `<method>`

Installation method that was used: `curl`, `npm`, `pnpm`, `bun`, `brew`.

- **Default:** Auto-detected
- **Example:**

```bash
opencode upgrade
opencode upgrade v0.1.48
opencode upgrade -m brew
```

### Global switches

Available on all commands.

#### `--help` / `-h`

Display help text and exit.

#### `--version` / `-v`

Print version number and exit.

#### `--print-logs`

Print logs to stderr.

- **Default:** Off

#### `--log-level` `<DEBUG|INFO|WARN|ERROR>`

Set the log level.

- **Default:** INFO
- **Example:**

```bash
opencode --log-level DEBUG run "debug the issue"
```

---

## Environment Variables

### Standard

| Variable | Type | Description |
|---|---|---|
| `OPENCODE_AUTO_SHARE` | boolean | Automatically share sessions |
| `OPENCODE_GIT_BASH_PATH` | string | Path to Git Bash executable (Windows) |
| `OPENCODE_CONFIG` | string | Path to config file |
| `OPENCODE_TUI_CONFIG` | string | Path to TUI config file |
| `OPENCODE_CONFIG_DIR` | string | Path to config directory |
| `OPENCODE_CONFIG_CONTENT` | string | Inline JSON config content |
| `OPENCODE_DISABLE_AUTOUPDATE` | boolean | Disable automatic update checks |
| `OPENCODE_DISABLE_PRUNE` | boolean | Disable pruning of old data |
| `OPENCODE_DISABLE_TERMINAL_TITLE` | boolean | Disable automatic terminal title updates |
| `OPENCODE_PERMISSION` | string | Inlined JSON permissions config |
| `OPENCODE_DISABLE_DEFAULT_PLUGINS` | boolean | Disable default plugins |
| `OPENCODE_DISABLE_LSP_DOWNLOAD` | boolean | Disable automatic LSP server downloads |
| `OPENCODE_ENABLE_EXPERIMENTAL_MODELS` | boolean | Enable experimental models |
| `OPENCODE_DISABLE_AUTOCOMPACT` | boolean | Disable automatic context compaction |
| `OPENCODE_DISABLE_CLAUDE_CODE` | boolean | Disable reading from `.claude` (prompt + skills) |
| `OPENCODE_DISABLE_CLAUDE_CODE_PROMPT` | boolean | Disable reading `~/.claude/CLAUDE.md` |
| `OPENCODE_DISABLE_CLAUDE_CODE_SKILLS` | boolean | Disable loading `.claude/skills` |
| `OPENCODE_DISABLE_MODELS_FETCH` | boolean | Disable fetching models from remote sources |
| `OPENCODE_DISABLE_MOUSE` | boolean | Disable mouse capture in the TUI |
| `OPENCODE_FAKE_VCS` | string | Fake VCS provider for testing |
| `OPENCODE_CLIENT` | string | Client identifier (default: `cli`) |
| `OPENCODE_ENABLE_EXA` | boolean | Enable Exa web search tools |
| `OPENCODE_SERVER_PASSWORD` | string | Enable basic auth for `serve`/`web` |
| `OPENCODE_SERVER_USERNAME` | string | Override basic auth username (default: `opencode`) |
| `OPENCODE_MODELS_URL` | string | Custom URL for fetching models configuration |

### Experimental

| Variable | Type | Description |
|---|---|---|
| `OPENCODE_EXPERIMENTAL` | boolean | Enable all experimental features |
| `OPENCODE_EXPERIMENTAL_ICON_DISCOVERY` | boolean | Enable icon discovery |
| `OPENCODE_EXPERIMENTAL_DISABLE_COPY_ON_SELECT` | boolean | Disable copy on select in TUI |
| `OPENCODE_EXPERIMENTAL_BASH_DEFAULT_TIMEOUT_MS` | number | Default timeout for bash commands in ms |
| `OPENCODE_EXPERIMENTAL_OUTPUT_TOKEN_MAX` | number | Max output tokens for LLM responses |
| `OPENCODE_EXPERIMENTAL_FILEWATCHER` | boolean | Enable file watcher for entire dir |
| `OPENCODE_EXPERIMENTAL_OXFMT` | boolean | Enable oxfmt formatter |
| `OPENCODE_EXPERIMENTAL_LSP_TOOL` | boolean | Enable experimental LSP tool |
| `OPENCODE_EXPERIMENTAL_DISABLE_FILEWATCHER` | boolean | Disable file watcher |
| `OPENCODE_EXPERIMENTAL_EXA` | boolean | Enable experimental Exa features |
| `OPENCODE_EXPERIMENTAL_LSP_TY` | boolean | Enable TY LSP for Python files |
| `OPENCODE_EXPERIMENTAL_MARKDOWN` | boolean | Enable experimental markdown features |
| `OPENCODE_EXPERIMENTAL_PLAN_MODE` | boolean | Enable plan mode |
| `OPENCODE_EXPERIMENTAL_DISABLE_FILEWATCHER` | boolean | Disable file watcher |

---

## Sources

- [OpenCode Homepage](https://opencode.ai)
- [OpenCode GitHub Repository](https://github.com/anomalyco/opencode)
- [OpenCode CLI Reference](https://opencode.ai/docs/cli/)
- [OpenCode Config Reference](https://opencode.ai/docs/config/)
- [OpenCode Providers](https://opencode.ai/docs/providers/)
- [OpenCode Agents](https://opencode.ai/docs/agents/)
- [OpenCode ACP Support](https://opencode.ai/docs/acp/)
- [OpenCode Agent Skills](https://opencode.ai/docs/skills/)
- [OpenCode Models](https://opencode.ai/docs/models/)
- [OpenCode Permissions](https://opencode.ai/docs/permissions/)
- [OpenCode MCP Servers](https://opencode.ai/docs/mcp-servers/)
- [OpenCode Server](https://opencode.ai/docs/server/)
- [OpenCode GitHub Integration](https://opencode.ai/docs/github/)
- [OpenCode Enterprise](https://opencode.ai/docs/enterprise/)
