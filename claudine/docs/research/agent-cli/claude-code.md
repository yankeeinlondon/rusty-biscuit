# Claude Code

---
homepage: https://claude.ai/code
docs: https://code.claude.com/docs/en
cli_docs: https://code.claude.com/docs/en/cli-usage
---

Claude Code is Anthropic's official agentic CLI for Claude. It starts an
interactive REPL by default and supports non-interactive output via `-p`/`--print`.

## Model Specification

### CLI Parameter

Use `--model <alias|name>` at launch:

```sh
claude --model opus
claude --model claude-sonnet-4-6
claude --model sonnet[1m]          # 1M context window variant
```

### Model Aliases

| Alias | Resolves To | Notes |
|-------|-------------|-------|
| `default` | Tier-dependent (see below) | Recommended setting |
| `sonnet` | Latest Sonnet (currently Sonnet 4.6) | Daily coding tasks |
| `opus` | Latest Opus (currently Opus 4.6) | Complex reasoning |
| `haiku` | Latest Haiku | Fast, simple tasks |
| `sonnet[1m]` | Sonnet with 1M context window | Long sessions / large codebases |
| `opusplan` | Opus in plan mode, Sonnet in execution | Hybrid reasoning + efficiency |

### Default Model Behavior

The default model depends on the user's subscription tier:

| User Type | Default Model |
|-----------|---------------|
| Max, Team Premium | Opus 4.6 |
| Pro, Team Standard | Sonnet 4.6 |
| Pay-as-you-go (API) | Sonnet 4.5 |

Claude Code may automatically fall back to Sonnet if the user hits a usage
threshold with Opus.

### Setting Priority (highest to lowest)

1. `/model` command during a session
2. `--model` flag at startup
3. `ANTHROPIC_MODEL` environment variable
4. `model` field in settings files

### Model-Related Environment Variables

| Variable | Purpose |
|----------|---------|
| `ANTHROPIC_MODEL` | Override the default model |
| `ANTHROPIC_DEFAULT_OPUS_MODEL` | Pin the model that `opus` resolves to |
| `ANTHROPIC_DEFAULT_SONNET_MODEL` | Pin the model that `sonnet` resolves to |
| `ANTHROPIC_DEFAULT_HAIKU_MODEL` | Pin the model that `haiku` resolves to |
| `CLAUDE_CODE_SUBAGENT_MODEL` | Model used for subagent tasks |

### Restricting Model Selection

Administrators can set `availableModels` in managed or project settings to
restrict which models users can select:

```json
{
  "availableModels": ["sonnet", "haiku"],
  "model": "sonnet"
}
```

The `default` option always remains available regardless of `availableModels`.

## Non-interactive Engagement

Non-interactive mode is fully supported. There are several approaches:

### 1. Print Mode (`-p` / `--print`)

The primary non-interactive mechanism. Claude processes the prompt and prints
the response to stdout, then exits.

```sh
claude -p "explain this function"
```

Benefits: clean stdout output, supports piping, supports `--output-format`,
`--max-turns`, `--max-budget-usd`, `--json-schema`.

### 2. Piped Input

Pipe content via stdin for Claude to process:

```sh
cat logs.txt | claude -p "summarize errors"
git diff | claude -p "review this diff"
```

### 3. Continue in Print Mode (`-c -p`)

Continue the most recent conversation non-interactively:

```sh
claude -c -p "now check for type errors"
```

### 4. Resume in Print Mode (`-r -p`)

Resume a specific session by ID or name non-interactively:

```sh
claude -r "auth-refactor" -p "finish the PR"
```

### 5. Structured Output

Use `--output-format` to get machine-parseable output:

```sh
claude -p "list functions" --output-format json
claude -p "stream analysis" --output-format stream-json
```

Use `--json-schema` for validated structured output:

```sh
claude -p --json-schema '{"type":"object","properties":{"name":{"type":"string"}},"required":["name"]}' "extract the name"
```

### 6. Streaming Input (`--input-format stream-json`)

For real-time bidirectional communication:

```sh
claude -p --input-format stream-json --output-format stream-json
```

### Key Print-Mode-Only Flags

These flags only work with `-p`:

- `--output-format` (text, json, stream-json)
- `--input-format` (text, stream-json)
- `--max-turns`
- `--max-budget-usd`
- `--json-schema`
- `--no-session-persistence`
- `--fallback-model`
- `--system-prompt-file`
- `--append-system-prompt-file`
- `--include-partial-messages`

## Subscription versus Per Call API

Claude Code supports two billing models:

1. **Subscription (Claude Pro / Max / Team / Enterprise)**: Users log in with
   their claude.ai account. Usage is included in the subscription. Run
   `claude auth login` and authenticate via browser OAuth.

2. **Per-call API (Console / Pay-as-you-go)**: Users authenticate with a
   Console account or API key. Billed by token consumption. Set
   `ANTHROPIC_API_KEY` or log in with Console credentials via
   `claude auth login`.

Both modes use the same `-p` flag for non-interactive operation. The billing
model is determined by the authentication method, not by any CLI flag.

Additional billing backends are available via environment variables:

- `CLAUDE_CODE_USE_BEDROCK=true` for AWS Bedrock
- `CLAUDE_CODE_USE_VERTEX=true` for Google Vertex AI
- `CLAUDE_CODE_USE_FOUNDRY=true` for Microsoft Foundry

## System Prompt

### Replacing the System Prompt

```sh
# Inline replacement (interactive + print modes)
claude --system-prompt "You are a Python expert"

# File-based replacement (print mode only)
claude -p --system-prompt-file ./prompts/reviewer.txt "review this PR"
```

This fully replaces the default Claude Code system prompt. Use only when you
need complete control; default capabilities (tool use instructions, etc.) are
removed.

### Appending to the System Prompt

```sh
# Inline append (interactive + print modes)
claude --append-system-prompt "Always use TypeScript"

# File-based append (print mode only)
claude -p --append-system-prompt-file ./rules.txt "review this PR"
```

This preserves the default system prompt and adds custom instructions at the
end. Recommended for most use cases.

### Mutual Exclusivity

`--system-prompt` and `--system-prompt-file` are mutually exclusive. The
`--append-*` variants can be combined with either replacement flag.

### CLAUDE.md Memory Files

In addition to CLI flags, Claude Code loads memory files into context:

| Scope | File |
|-------|------|
| User (global) | `~/.claude/CLAUDE.md` |
| Project (shared) | `CLAUDE.md` or `.claude/CLAUDE.md` |
| Project (local) | `.claude/CLAUDE.local.md` |

These are always loaded and supplement the system prompt.

## Permissions

### Default Setup

Claude Code uses a tiered permission system:

| Tool Type | Example | Approval Required | "Don't ask again" Scope |
|-----------|---------|-------------------|------------------------|
| Read-only | File reads, Grep | No | N/A |
| Bash commands | Shell execution | Yes | Permanent per project+command |
| File modification | Edit/write files | Yes | Until session end |

### Permission Configuration Files

| Scope | Location |
|-------|----------|
| Managed | `/Library/Application Support/ClaudeCode/managed-settings.json` (macOS) |
| User | `~/.claude/settings.json` |
| Project | `.claude/settings.json` |
| Local | `.claude/settings.local.json` |

Precedence: Managed > CLI args > Local > Project > User.

### Permission Modes

Set via `--permission-mode <mode>` or `defaultMode` in settings:

| Mode | Description |
|------|-------------|
| `default` | Prompts for permission on first use of each tool |
| `acceptEdits` | Auto-accepts file edit permissions for the session |
| `plan` | Read-only: Claude can analyze but not modify files or run commands |
| `dontAsk` | Auto-denies tools unless pre-approved in permissions config |
| `bypassPermissions` | Skips all permission prompts (dangerous; sandboxed environments only) |

### "Yolo" Mode

Yes. Two switches provide full permission bypass:

- `--dangerously-skip-permissions` -- Immediately bypasses all permission checks.
- `--allow-dangerously-skip-permissions` -- Enables bypass as an option without
  activating it immediately. Can be composed with `--permission-mode`.

Administrators can disable bypass mode:

```json
{
  "permissions": {
    "disableBypassPermissionsMode": "disable"
  }
}
```

### Tool Allow/Deny via CLI

```sh
# Allow specific tools without prompting
claude --allowedTools "Bash(git log *)" "Bash(git diff *)" "Read"

# Deny specific tools entirely
claude --disallowedTools "Bash(curl *)" "WebFetch"

# Restrict available built-in tools
claude --tools "Bash,Edit,Read"
claude --tools ""          # disable all tools
claude --tools "default"   # all tools (default)
```

## Thinking Level

### Effort Level (Adaptive Reasoning)

Effort controls Opus 4.6's adaptive reasoning depth. Three levels are available:

| Level | Behavior |
|-------|----------|
| `low` | Faster, cheaper; minimal reasoning for straightforward tasks |
| `medium` | Balanced reasoning |
| `high` | Deep reasoning for complex problems (default) |

**Setting effort:**

- **CLI flag**: `--effort low|medium|high`
- **Environment variable**: `CLAUDE_CODE_EFFORT_LEVEL=low|medium|high`
- **Settings file**: `"effortLevel": "low|medium|high"`
- **In-session**: `/model` then use left/right arrow keys to adjust the slider

### Extended Thinking

Extended thinking is enabled by default with a budget of 31,999 tokens.

| Setting | Purpose |
|---------|---------|
| `alwaysThinkingEnabled` | Enable/disable extended thinking by default (`true`/`false`) |
| `MAX_THINKING_TOKENS` | Token budget for thinking (e.g., `10000`; set `0` to disable) |

Toggle in-session with `Option+T` (macOS) or `Alt+T` (Windows/Linux).

## Logging

### Session Files

Sessions are stored as newline-delimited JSON (JSONL) files:

```
~/.claude/projects/<encoded-directory>/<session-uuid>.jsonl
```

Each file contains the full conversation history for a session. Sessions are
automatically cleaned up after the period configured by `cleanupPeriodDays`
(default: 30 days).

### Debug Logging

Enable with `--debug` (optional category filter) or `--debug-file <path>`:

```sh
claude --debug                     # all debug categories to stderr
claude --debug "api,hooks"         # filter to specific categories
claude --debug "!statsig,!file"    # exclude specific categories
claude --debug-file /tmp/debug.log # write to file (implicitly enables debug)
```

### Verbose Mode

```sh
claude --verbose
```

Shows full turn-by-turn output including tool usage details. Can also be toggled
in-session with `Ctrl+O`.

### Configuration File Locations

| File | Purpose |
|------|---------|
| `~/.claude/settings.json` | User settings |
| `.claude/settings.json` | Project settings (committed) |
| `.claude/settings.local.json` | Local project settings (gitignored) |
| `~/.claude.json` | Global state (theme, OAuth, MCP servers) |
| `.mcp.json` | Project MCP servers (committed) |

### Telemetry Environment Variables

| Variable | Purpose |
|----------|---------|
| `CLAUDE_CODE_ENABLE_TELEMETRY=1` | Enable OpenTelemetry export |
| `DISABLE_TELEMETRY=1` | Opt out of Statsig telemetry |
| `DISABLE_ERROR_REPORTING=1` | Opt out of Sentry error reporting |

## CLI Options

### Subcommands

| Subcommand | Description |
|------------|-------------|
| `auth` | Manage authentication (login, logout, status) |
| `doctor` | Check installation health (auto-updater, settings, MCP, search) |
| `install [target]` | Install native build; target = `stable`, `latest`, or version number |
| `mcp` | Configure and manage MCP servers (add, remove, list, get, serve) |
| `plugin` | Manage plugins (install, uninstall, list, enable, disable, update, validate, marketplace) |
| `setup-token` | Set up a long-lived authentication token (requires Claude subscription) |
| `update` / `upgrade` | Check for updates and install if available |

### Switches

| Switch | Description |
|--------|-------------|
| `--add-dir <dirs...>` | Additional directories to allow tool access to |
| `--agent <agent>` | Agent for the current session (overrides `agent` setting) |
| `--agents <json>` | JSON object defining custom subagents |
| `--allow-dangerously-skip-permissions` | Enable permission bypass as an option without immediately activating |
| `--allowedTools <tools...>` | Tools that execute without prompting (also `--allowed-tools`) |
| `--append-system-prompt <prompt>` | Append text to the default system prompt |
| `--append-system-prompt-file <file>` | Append file contents to the default system prompt (print mode only) |
| `--betas <betas...>` | Beta headers for API requests (API key users only) |
| `--chrome` | Enable Chrome browser integration |
| `-c`, `--continue` | Continue the most recent conversation in the current directory |
| `--dangerously-skip-permissions` | Bypass all permission checks |
| `-d`, `--debug [filter]` | Enable debug mode with optional category filtering |
| `--debug-file <path>` | Write debug logs to a file (implicitly enables debug) |
| `--disable-slash-commands` | Disable all skills and slash commands |
| `--disallowedTools <tools...>` | Tools removed from model context (also `--disallowed-tools`) |
| `--effort <level>` | Effort level: `low`, `medium`, `high` |
| `--fallback-model <model>` | Fallback model when default is overloaded (print mode only) |
| `--file <specs...>` | File resources to download at startup (`file_id:relative_path`) |
| `--fork-session` | Create a new session ID when resuming (use with `--resume` or `--continue`) |
| `--from-pr [value]` | Resume session linked to a PR by number/URL, or open picker |
| `-h`, `--help` | Display help |
| `--ide` | Auto-connect to IDE on startup if exactly one valid IDE is available |
| `--include-partial-messages` | Include partial streaming events (requires `--print` + `--output-format stream-json`) |
| `--init` | Run initialization hooks and start interactive mode |
| `--init-only` | Run initialization hooks and exit |
| `--input-format <format>` | Input format for print mode: `text` (default), `stream-json` |
| `--json-schema <schema>` | JSON Schema for validated structured output (print mode only) |
| `--maintenance` | Run maintenance hooks and exit |
| `--max-budget-usd <amount>` | Maximum dollar spend on API calls (print mode only) |
| `--max-turns <n>` | Limit agentic turns (print mode only); exits with error at limit |
| `--mcp-config <configs...>` | Load MCP servers from JSON files or strings |
| `--mcp-debug` | Deprecated; use `--debug` instead |
| `--model <model>` | Model for the session (alias like `sonnet` or full name) |
| `--no-chrome` | Disable Chrome browser integration |
| `--no-session-persistence` | Disable session persistence (print mode only) |
| `--output-format <format>` | Output format for print mode: `text`, `json`, `stream-json` |
| `--permission-mode <mode>` | Permission mode: `default`, `acceptEdits`, `plan`, `dontAsk`, `bypassPermissions` |
| `--permission-prompt-tool <tool>` | MCP tool to handle permission prompts in non-interactive mode |
| `--plugin-dir <paths...>` | Load plugins from directories for this session |
| `-p`, `--print` | Print response and exit (non-interactive mode) |
| `--remote` | Create a new web session on claude.ai with the provided task description |
| `--replay-user-messages` | Re-emit user messages from stdin on stdout (requires stream-json I/O) |
| `-r`, `--resume [value]` | Resume a session by ID/name, or open interactive picker |
| `--session-id <uuid>` | Use a specific session ID (must be valid UUID) |
| `--setting-sources <sources>` | Comma-separated setting sources to load: `user`, `project`, `local` |
| `--settings <file-or-json>` | Path to settings JSON file or inline JSON string |
| `--strict-mcp-config` | Only use MCP servers from `--mcp-config`, ignore all other configs |
| `--system-prompt <prompt>` | Replace the entire system prompt |
| `--system-prompt-file <file>` | Replace system prompt with file contents (print mode only) |
| `--teammate-mode <mode>` | Agent team display: `auto`, `in-process`, `tmux` |
| `--teleport` | Resume a web session in local terminal |
| `--tools <tools...>` | Restrict available built-in tools (`""` = none, `"default"` = all, or names) |
| `--verbose` | Enable verbose output (full turn-by-turn details) |
| `-v`, `--version` | Output the version number |

## Sources

- [Claude Code home page](https://claude.ai/code)
- [CLI reference](https://code.claude.com/docs/en/cli-usage)
- [Settings](https://code.claude.com/docs/en/settings)
- [Permissions](https://code.claude.com/docs/en/permissions)
- [Model configuration](https://code.claude.com/docs/en/model-config)
- [Interactive mode](https://code.claude.com/docs/en/interactive-mode)
- [Costs](https://code.claude.com/docs/en/costs)
- [Authentication](https://code.claude.com/docs/en/authentication)
- [Troubleshooting](https://code.claude.com/docs/en/troubleshooting)
