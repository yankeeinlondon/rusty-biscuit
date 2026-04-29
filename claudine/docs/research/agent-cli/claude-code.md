---
homepage: https://claude.ai/code
docs: https://code.claude.com/docs/en
cli_docs: https://code.claude.com/docs/en/cli-reference
---
# Claude Code


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
| `default` | Tier-dependent (see below) | Recommended setting; clears any override |
| `best` | Equivalent to `opus` | Most capable available model |
| `sonnet` | Latest Sonnet (currently Sonnet 4.6) | Daily coding tasks |
| `opus` | Latest Opus (currently Opus 4.7 on Anthropic API, 4.6 on Bedrock/Vertex/Foundry) | Complex reasoning |
| `haiku` | Latest Haiku | Fast, simple tasks |
| `sonnet[1m]` | Sonnet with 1M context window | Long sessions / large codebases |
| `opus[1m]` | Opus with 1M context window | Long sessions / large codebases |
| `opusplan` | Opus in plan mode, Sonnet in execution | Hybrid reasoning + efficiency |

### Default Model Behavior

The default model depends on the user's subscription tier:

| User Type | Default Model |
|-----------|---------------|
| Max, Team Premium | Opus 4.7 |
| Pro, Team Standard, Enterprise | Sonnet 4.6 |
| Bedrock, Vertex, Foundry | Sonnet 4.5 |
| Pay-as-you-go (API) | Sonnet 4.6 (changing to Opus 4.7 on April 23, 2026) |

Claude Code may automatically fall back to Sonnet if the user hits a usage
threshold with Opus.

### Setting Priority (highest to lowest)

1. `/model` command during a session (persists to settings)
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
| `ANTHROPIC_CUSTOM_MODEL_OPTION` | Add a custom entry to the `/model` picker |

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

### Extended Context

Opus 4.7, Opus 4.6, and Sonnet 4.6 support a 1 million token context window.
On Max, Team, and Enterprise plans, Opus is automatically upgraded to 1M
context. Pro and pay-as-you-go users can access 1M context (may require extra
usage). Disable with `CLAUDE_CODE_DISABLE_1M_CONTEXT=1`.

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

### 7. Bare Mode (`--bare`)

Minimal mode that skips auto-discovery of hooks, skills, plugins, MCP servers,
auto memory, and CLAUDE.md. Recommended for CI and scripts where you need
deterministic results. Claude has access to Bash, file read, and file edit tools.

```sh
claude --bare -p "Summarize this file" --allowedTools "Read"
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
- `--include-partial-messages`

## Subscription versus Per Call API

Claude Code supports two billing models:

1. **Subscription (Claude Pro / Max / Team / Enterprise)**: Users log in with
   their claude.ai account. Usage is included in the subscription. Run
   `claude auth login` and authenticate via browser OAuth.

2. **Per-call API (Console / Pay-as-you-go)**: Users authenticate with a
   Console account or API key. Billed by token consumption. Set
   `ANTHROPIC_API_KEY` or log in with Console credentials via
   `claude auth login --console`.

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

# File-based replacement
claude --system-prompt-file ./prompts/reviewer.txt
```

This fully replaces the default Claude Code system prompt. Use only when you
need complete control; default capabilities (tool use instructions, etc.) are
removed.

### Appending to the System Prompt

```sh
# Inline append (interactive + print modes)
claude --append-system-prompt "Always use TypeScript"

# File-based append
claude --append-system-prompt-file ./rules.txt
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

Set via `--permission-mode <mode>` or `defaultMode` in settings. Cycle modes
in-session with `Shift+Tab`.

| Mode | Description |
|------|-------------|
| `default` | Prompts for permission on first use of each tool |
| `acceptEdits` | Auto-accepts file edits and common filesystem commands (`mkdir`, `touch`, `mv`, `cp`) |
| `plan` | Read-only: Claude can analyze but not modify files or run commands |
| `auto` | Classifier-reviewed auto-execution with background safety checks (requires Max/Team/Enterprise/API, v2.1.83+) |
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

Effort controls the model's adaptive reasoning depth. Available levels depend
on the model:

| Model | Levels |
|-------|--------|
| Opus 4.7 | `low`, `medium`, `high`, `xhigh`, `max` |
| Opus 4.6, Sonnet 4.6 | `low`, `medium`, `high`, `max` |

| Level | Behavior |
|-------|----------|
| `low` | Faster, cheaper; minimal reasoning for straightforward tasks |
| `medium` | Balanced reasoning |
| `high` | Deep reasoning for complex problems |
| `xhigh` | Best results for most coding/agentic tasks (default on Opus 4.7) |
| `max` | Deepest reasoning with no token spend constraint; session-scoped only |

**Setting effort:**

- **CLI flag**: `--effort low|medium|high|xhigh|max`
- **Environment variable**: `CLAUDE_CODE_EFFORT_LEVEL=<level>`
- **Settings file**: `"effortLevel": "<level>"`
- **In-session**: `/effort` (interactive slider), `/effort auto` to reset to model default
- **In `/model`**: use left/right arrow keys to adjust the effort slider

`low`, `medium`, `high`, and `xhigh` persist across sessions. `max` is
session-scoped only (except when set via environment variable).

### Extended Thinking

Extended thinking is enabled by default. On Opus 4.7, adaptive reasoning is
always used. On Opus 4.6 and Sonnet 4.6, you can set
`CLAUDE_CODE_DISABLE_ADAPTIVE_THINKING=1` to revert to fixed thinking budgets.

| Setting | Purpose |
|---------|---------|
| `alwaysThinkingEnabled` | Enable/disable extended thinking by default (`true`/`false`) |
| `MAX_THINKING_TOKENS` | Token budget for thinking (e.g., `10000`; set `0` to disable; not applicable to Opus 4.7) |

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

## CLI Switch Summary

Every CLI switch/parameter supported by Claude Code, with description, default
value, and examples.

### Session Control

| Switch | Short | Description | Default | Example |
|--------|-------|-------------|---------|---------|
| `--print` | `-p` | Print response and exit (non-interactive mode) | (interactive) | `claude -p "explain this function"` |
| `--continue` | `-c` | Load the most recent conversation in the current directory | (new session) | `claude -c` |
| `--resume` | `-r` | Resume a session by ID or name, or open interactive picker | (new session) | `claude -r "auth-refactor"` |
| `--name` | `-n` | Set a display name for the session (shown in `/resume` and terminal title) | auto-generated | `claude -n "my-feature-work"` |
| `--session-id` | | Use a specific session ID (must be a valid UUID) | auto-generated UUID | `claude --session-id "550e8400-e29b-41d4-a716-446655440000"` |
| `--fork-session` | | When resuming, create a new session ID instead of reusing the original | (reuses original) | `claude --resume abc123 --fork-session` |
| `--from-pr` | | Resume sessions linked to a specific PR (number or URL) | (none) | `claude --from-pr 123` |
| `--no-session-persistence` | | Disable session persistence so sessions are not saved to disk (print mode only) | sessions saved | `claude -p --no-session-persistence "query"` |
| `--teleport` | | Resume a web session in your local terminal | (none) | `claude --teleport` |
| `--remote` | | Create a new web session on claude.ai with the provided task description | (none) | `claude --remote "Fix the login bug"` |

### Model and Effort

| Switch | Description | Default | Example |
|--------|-------------|---------|---------|
| `--model` | Set the model for the session (alias or full name) | tier-dependent | `claude --model opus` |
| `--effort` | Set effort level (`low`, `medium`, `high`, `xhigh`, `max`; available levels depend on model) | `xhigh` on Opus 4.7, `high` on others | `claude --effort high` |
| `--fallback-model` | Enable automatic fallback to specified model when default is overloaded (print mode only) | (none) | `claude -p --fallback-model sonnet "query"` |
| `--betas` | Beta headers to include in API requests (API key users only) | (none) | `claude --betas interleaved-thinking` |

### System Prompt

| Switch | Description | Default | Example |
|--------|-------------|---------|---------|
| `--system-prompt` | Replace the entire system prompt with custom text | Claude Code built-in prompt | `claude --system-prompt "You are a Python expert"` |
| `--system-prompt-file` | Load system prompt from a file, replacing the default prompt | Claude Code built-in prompt | `claude --system-prompt-file ./custom-prompt.txt` |
| `--append-system-prompt` | Append custom text to the end of the default system prompt | (none) | `claude --append-system-prompt "Always use TypeScript"` |
| `--append-system-prompt-file` | Load additional system prompt text from a file and append to the default prompt | (none) | `claude --append-system-prompt-file ./extra-rules.txt` |

### Permissions

| Switch | Description | Default | Example |
|--------|-------------|---------|---------|
| `--permission-mode` | Begin in a specified permission mode (`default`, `acceptEdits`, `plan`, `auto`, `dontAsk`, `bypassPermissions`) | `default` | `claude --permission-mode plan` |
| `--dangerously-skip-permissions` | Skip all permission prompts. Equivalent to `--permission-mode bypassPermissions` | (prompts enabled) | `claude --dangerously-skip-permissions` |
| `--allow-dangerously-skip-permissions` | Add `bypassPermissions` to the Shift+Tab mode cycle without starting in it | (bypass not in cycle) | `claude --permission-mode plan --allow-dangerously-skip-permissions` |
| `--allowedTools` | Tools that execute without prompting. Supports pattern matching (`Bash(git log *)`) | (all tools prompt) | `claude --allowedTools "Bash(git log *)" "Read"` |
| `--disallowedTools` | Tools removed from the model's context entirely | (none) | `claude --disallowedTools "Bash(curl *)" "Edit"` |
| `--tools` | Restrict which built-in tools Claude can use (`""` = none, `"default"` = all) | `"default"` | `claude --tools "Bash,Edit,Read"` |
| `--permission-prompt-tool` | MCP tool to handle permission prompts in non-interactive mode | (none) | `claude -p --permission-prompt-tool mcp_auth_tool "query"` |

### Output and Format

| Switch | Description | Default | Example |
|--------|-------------|---------|---------|
| `--output-format` | Output format for print mode (`text`, `json`, `stream-json`) | `text` | `claude -p "query" --output-format json` |
| `--input-format` | Input format for print mode (`text`, `stream-json`) | `text` | `claude -p --input-format stream-json` |
| `--json-schema` | JSON Schema for validated structured output (print mode only) | (none) | `claude -p --json-schema '{"type":"object"}' "query"` |
| `--include-partial-messages` | Include partial streaming events in output (requires `-p` + `--output-format stream-json`) | (disabled) | `claude -p --output-format stream-json --include-partial-messages "query"` |
| `--include-hook-events` | Include all hook lifecycle events in the output stream (requires `--output-format stream-json`) | (disabled) | `claude -p --output-format stream-json --include-hook-events "query"` |
| `--replay-user-messages` | Re-emit user messages from stdin on stdout (requires `--input-format stream-json`) | (disabled) | `claude -p --input-format stream-json --output-format stream-json --replay-user-messages` |

### Budget and Limits

| Switch | Description | Default | Example |
|--------|-------------|---------|---------|
| `--max-budget-usd` | Maximum dollar amount to spend on API calls before stopping (print mode only) | (no limit) | `claude -p --max-budget-usd 5.00 "query"` |
| `--max-turns` | Limit the number of agentic turns; exits with error at limit (print mode only) | (no limit) | `claude -p --max-turns 3 "query"` |

### Directories and Worktree

| Switch | Short | Description | Default | Example |
|--------|-------|-------------|---------|---------|
| `--add-dir` | | Add additional working directories for Claude to read and edit files | current directory only | `claude --add-dir ../apps ../lib` |
| `--worktree` | `-w` | Start Claude in an isolated git worktree at `<repo>/.claude/worktrees/<name>` | (none) | `claude -w feature-auth` |
| `--tmux` | | Create a tmux session for the worktree (requires `--worktree`). Use `--tmux=classic` for traditional tmux | (none) | `claude -w feature-auth --tmux` |

### MCP and Plugins

| Switch | Description | Default | Example |
|--------|-------------|---------|---------|
| `--mcp-config` | Load MCP servers from JSON files or strings (space-separated) | (none) | `claude --mcp-config ./mcp.json` |
| `--strict-mcp-config` | Only use MCP servers from `--mcp-config`, ignoring all other MCP configurations | (all configs loaded) | `claude --strict-mcp-config --mcp-config ./mcp.json` |
| `--plugin-dir` | Load plugins from a directory for this session only. Repeat for multiple dirs | (none) | `claude --plugin-dir ./my-plugins` |

### Agents and Skills

| Switch | Description | Default | Example |
|--------|-------------|---------|---------|
| `--agent` | Specify an agent for the current session (overrides the `agent` setting) | (none) | `claude --agent my-custom-agent` |
| `--agents` | Define custom subagents dynamically via JSON | (none) | `claude --agents '{"reviewer":{"description":"Reviews code","prompt":"You are a code reviewer"}}'` |
| `--disable-slash-commands` | Disable all skills and commands for this session | (enabled) | `claude --disable-slash-commands` |
| `--teammate-mode` | Set how agent team teammates display (`auto`, `in-process`, `tmux`) | `auto` | `claude --teammate-mode in-process` |

### Settings and Configuration

| Switch | Description | Default | Example |
|--------|-------------|---------|---------|
| `--settings` | Path to a settings JSON file or inline JSON string | (none) | `claude --settings ./settings.json` |
| `--setting-sources` | Comma-separated list of setting sources to load (`user`, `project`, `local`) | all sources | `claude --setting-sources user,project` |
| `--exclude-dynamic-system-prompt-sections` | Move per-machine sections (working directory, env info, git status) from system prompt to first user message. Improves prompt-cache reuse | (sections in system prompt) | `claude -p --exclude-dynamic-system-prompt-sections "query"` |

### Initialization and Hooks

| Switch | Description | Default | Example |
|--------|-------------|---------|---------|
| `--init` | Run initialization hooks and start interactive mode | (none) | `claude --init` |
| `--init-only` | Run initialization hooks and exit (no interactive session) | (none) | `claude --init-only` |
| `--maintenance` | Run maintenance hooks and start interactive mode | (none) | `claude --maintenance` |
| `--bare` | Minimal mode: skip auto-discovery of hooks, skills, plugins, MCP servers, auto memory, and CLAUDE.md | (all auto-discovered) | `claude --bare -p "query"` |

### IDE and Browser Integration

| Switch | Description | Default | Example |
|--------|-------------|---------|---------|
| `--ide` | Auto-connect to IDE on startup if exactly one valid IDE is available | (none) | `claude --ide` |
| `--chrome` | Enable Chrome browser integration for web automation and testing | (disabled) | `claude --chrome` |
| `--no-chrome` | Disable Chrome browser integration for this session | (auto-detected) | `claude --no-chrome` |

### Remote Control

| Switch | Short | Description | Default | Example |
|--------|-------|-------------|---------|---------|
| `--remote-control` | `--rc` | Start interactive session with Remote Control enabled so you can control it from claude.ai or the Claude app | (disabled) | `claude --remote-control "My Project"` |
| `--remote-control-session-name-prefix` | | Prefix for auto-generated Remote Control session names | machine hostname | `claude remote-control --remote-control-session-name-prefix dev-box` |

### Channels

| Switch | Description | Default | Example |
|--------|-------------|---------|---------|
| `--channels` | MCP servers whose channel notifications Claude should listen for. Space-separated `plugin:<name>@<marketplace>` entries | (none) | `claude --channels plugin:my-notifier@my-marketplace` |
| `--dangerously-load-development-channels` | Enable channels not on the approved allowlist, for local development | (none) | `claude --dangerously-load-development-channels server:webhook` |

### Debugging and Logging

| Switch | Short | Description | Default | Example |
|--------|-------|-------------|---------|---------|
| `--debug` | `-d` | Enable debug mode with optional category filtering (e.g. `"api,hooks"` or `"!statsig,!file"`) | (disabled) | `claude --debug "api,mcp"` |
| `--debug-file` | | Write debug logs to a specific file path. Implicitly enables debug mode | (disabled) | `claude --debug-file /tmp/claude-debug.log` |
| `--verbose` | | Enable verbose logging, shows full turn-by-turn output | (disabled) | `claude --verbose` |
| `--version` | `-v` | Output the version number | (n/a) | `claude -v` |
| `--help` | `-h` | Display help | (n/a) | `claude --help` |

## Subcommands

| Subcommand | Description | Example |
|------------|-------------|---------|
| `claude` | Start interactive session | `claude` |
| `claude "query"` | Start interactive session with initial prompt | `claude "explain this project"` |
| `claude update` | Update to latest version | `claude update` |
| `claude install [version]` | Install or reinstall the native binary. Accepts version like `2.1.118`, `stable`, or `latest` | `claude install stable` |
| `claude auth login` | Sign in. `--email` pre-fills email, `--sso` forces SSO, `--console` uses Anthropic Console | `claude auth login --console` |
| `claude auth logout` | Log out from your Anthropic account | `claude auth logout` |
| `claude auth status` | Show authentication status as JSON. `--text` for human-readable. Exit 0 if logged in, 1 if not | `claude auth status` |
| `claude agents` | List all configured subagents, grouped by source | `claude agents` |
| `claude auto-mode defaults` | Print built-in auto mode classifier rules as JSON. `claude auto-mode config` for effective config with settings | `claude auto-mode defaults > rules.json` |
| `claude mcp` | Configure MCP servers (add, remove, list, get, serve) | `claude mcp add my-server` |
| `claude plugin` | Manage plugins (install, uninstall, list, enable, disable, update, validate, marketplace). Alias: `claude plugins` | `claude plugin install code-review@claude-plugins-official` |
| `claude remote-control` | Start a Remote Control server. Runs in server mode (no local interactive session) | `claude remote-control --name "My Project"` |
| `claude setup-token` | Generate a long-lived OAuth token for CI and scripts. Prints token without saving | `claude setup-token` |

If you mistype a subcommand, Claude Code suggests the closest match and exits
without starting a session (e.g., `claude udpate` prints `Did you mean claude update?`).

## Sources

- [Claude Code home page](https://claude.ai/code)
- [CLI reference](https://code.claude.com/docs/en/cli-reference)
- [Settings](https://code.claude.com/docs/en/settings)
- [Permissions](https://code.claude.com/docs/en/permissions)
- [Permission modes](https://code.claude.com/docs/en/permission-modes)
- [Model configuration](https://code.claude.com/docs/en/model-config)
- [Interactive mode](https://code.claude.com/docs/en/interactive-mode)
- [Headless / Agent SDK](https://code.claude.com/docs/en/headless)
- [Costs](https://code.claude.com/docs/en/costs)
- [Authentication](https://code.claude.com/docs/en/authentication)
- [Troubleshooting](https://code.claude.com/docs/en/troubleshooting)
