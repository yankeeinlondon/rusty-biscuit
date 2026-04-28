---
homepage: https://qwen.ai/
docs: https://qwenlm.github.io/qwen-code-docs/
cli_docs: https://qwenlm.github.io/qwen-code-docs/en/users/overview
repo: https://github.com/QwenLM/qwen-code
npm: https://www.npmjs.com/package/@qwen-code/qwen-code
last_researched: 2026-04-27
---

# Qwen Code CLI

Qwen Code is an open-source AI agent from Alibaba that runs in the terminal. It is forked from Gemini CLI and adapted with customized prompts and function-calling protocols optimized for the Qwen3-Coder model family. Installed via npm (`npm i -g @qwen-code/qwen-code@latest`) or Homebrew (`brew install qwen-code`). Requires Node.js 20+.

**Notable:** Qwen OAuth (free tier) was **discontinued on April 15, 2026**. Users must now authenticate via Alibaba Cloud Coding Plan (subscription), API key (OpenAI/Anthropic/Gemini-compatible), or local models (Ollama/vLLM).

## Model Specification

**CLI flag:** `-m, --model <model-name>`

```bash
qwen -m qwen3.6-plus
```

**Default model resolution** (highest to lowest priority):

1. `--model` CLI flag
2. `OPENAI_MODEL` environment variable
3. `model.name` in `~/.qwen/settings.json`
4. The model associated with the active auth type

**Interactive switching:** Use the `/model` command inside a session to switch between all configured models. Use `/model --fast <model>` to set a lighter model for prompt suggestions.

**Configuring additional models** in `~/.qwen/settings.json`:

```json
{
  "modelProviders": {
    "openai": [
      {
        "id": "qwen3.6-plus",
        "name": "Qwen3.6-Plus",
        "baseUrl": "https://dashscope.aliyuncs.com/compatible-mode/v1",
        "envKey": "DASHSCOPE_API_KEY"
      }
    ]
  },
  "model": {
    "name": "qwen3.6-plus"
  }
}
```

Models from any OpenAI-compatible, Anthropic, Gemini, or Vertex AI provider can be added under the corresponding key in `modelProviders`. The `--auth-type` flag selects which provider protocol to use: `openai`, `anthropic`, `gemini`, or `vertex-ai` (`qwen-oauth` is no longer available).

## Non-interactive Engagement

Non-interactive (headless) mode is fully supported. There are several ways to run Qwen Code without the interactive TUI.

### Positional prompt (preferred)

```bash
qwen "Explain the architecture of this project"
```

The positional form runs the prompt as a one-shot task and exits. This is the current recommended approach.

### `-p, --prompt` flag

```bash
qwen -p "Explain the architecture of this project"
```

Functionally identical to the positional form.

### `-i, --prompt-interactive` flag

```bash
qwen -i "Refactor the auth module"
```

Executes the prompt, then drops into the interactive TUI so you can continue the conversation.

### Piping stdin

```bash
cat src/main.rs | qwen "Review this code for bugs"
echo "Explain Docker" | qwen
git diff --staged | qwen "Write a commit message"
```

Stdin content is prepended to the prompt. When using `--input-format stream-json`, stdin is reserved for the JSON protocol instead.

### System prompt overrides

```bash
qwen -p "Review this patch" --system-prompt "You are a terse release reviewer."
qwen -p "Review this patch" --append-system-prompt "Focus on concrete findings."
```

- `--system-prompt` replaces the built-in main-session prompt (context files like `QWEN.md` are still appended).
- `--append-system-prompt` adds extra instructions after the built-in prompt and loaded context.

### Output formats for automation

| Flag | Format | Use case |
|------|--------|----------|
| `-o text` | Plain text (default) | Human-readable output |
| `-o json` | Buffered JSON array | Programmatic processing |
| `-o stream-json` | Line-delimited JSON | Real-time streaming |

Use `--include-partial-messages` with `stream-json` to receive incremental assistant tokens.

### Session resumption in headless mode

```bash
qwen --continue -p "What was the next step?"
qwen --resume <sessionId> -p "Continue from here"
```

### Turn limits

`--max-session-turns <N>` caps the number of agent turns, useful for CI budgets.

### Limitations

- Qwen OAuth cannot authenticate in headless/CI environments. Use API-KEY auth (`--auth-type openai`) with environment variables instead.
- Headless mode does not support `--prompt-interactive` with stdin piping.

## Subscription versus Per Call API

**Alibaba Cloud Bailian Coding Plan (subscription):** Fixed monthly fee with higher quotas. Requires an active subscription from Alibaba Cloud ModelStudio. Uses a dedicated API key prefixed `sk-sp-`. Configure with:

```bash
export BAILIAN_CODING_PLAN_API_KEY="sk-sp-xxxxxxxxx"
qwen --auth-type openai -m qwen3.6-plus
```

Coding Plan endpoint: `https://coding.dashscope.aliyuncs.com/v1`. Available models include `qwen3.6-plus`, `qwen3.5-plus`, `glm-4.7`, `kimi-k2.5`.

**Third-party per-call API:** Use any OpenAI-compatible, Anthropic, or Gemini provider with their own per-token pricing. Set the appropriate environment variables (`OPENAI_API_KEY`, `ANTHROPIC_API_KEY`, `GEMINI_API_KEY`) and `--auth-type` flag.

**Local models (Ollama/vLLM):** No API key needed. Configure local model endpoint in `settings.json` with `baseUrl` pointing to `http://localhost:11434/v1` (Ollama) or `http://localhost:8000/v1` (vLLM).

For headless/CI non-interactive usage, Coding Plan keys, third-party API keys, and local models all work. The discontinued Qwen OAuth does not work in headless environments.

## System Prompt

Qwen Code uses hierarchical context files (defaulting to `QWEN.md`, configurable via `context.fileName` in settings) to supplement the system prompt.

**File locations searched** (all found files are concatenated):

1. `~/.qwen/QWEN.md` -- global user-level instructions
2. `QWEN.md` files in the current directory and every parent directory up to the project root (`.git` boundary) or home directory

**Behavior:** Context files are _supplements_ to the built-in system prompt, not full replacements. The built-in prompt remains; your instructions are appended.

**Modularizing:** Import other markdown files within a context file using `@path/to/file.md` syntax.

**Managing context at runtime:**

- `/memory show` -- display the combined context currently loaded
- `/memory refresh` -- force re-scan and reload of all context files
- `/memory add <text>` -- add a line to the context for this session

## Permissions

**Default mode:** `default` -- requires manual approval for both file edits and shell commands.

**Four approval modes** (set via `--approval-mode` flag or `Shift+Tab` cycling in TUI):

| Mode | File edits | Shell commands | Risk level |
|------|-----------|---------------|------------|
| `plan` | Read-only | Not executed | Lowest |
| `default` | Manual approval | Manual approval | Low |
| `auto-edit` | Auto-approved | Manual approval | Medium |
| `yolo` | Auto-approved | Auto-approved | Highest |

**YOLO mode** is available via:

- `--yolo` / `-y` CLI flag
- `--approval-mode yolo`
- `Shift+Tab` cycling in interactive mode
- `permissions.defaultMode: "yolo"` in `.qwen/settings.json` (project-level)

**Permissions system** (in `settings.json`):

- `permissions.allow` -- rules for auto-approved tool calls
- `permissions.ask` -- rules for tool calls that always require confirmation
- `permissions.deny` -- rules for blocked tool calls (highest priority)

Decision priority (highest first): `deny` > `ask` > `allow` > default/interactive mode.

Rules use the format `"ToolName"` or `"ToolName(specifier)"`. Meta-categories: `Read` covers `read_file`, `grep_search`, `glob`, `list_directory`; `Edit` covers `edit`, `write_file`.

**Sandbox mode:** `--sandbox` / `-s` runs the agent in a sandboxed environment. `--sandbox-image` specifies a custom container image.

**MCP server trust:** `qwen mcp add --trust <name> <command>` bypasses tool confirmation for that server.

## Thinking Level

There is no dedicated CLI flag for thinking level. Thinking is configured through `settings.json`:

```json
{
  "model": {
    "generationConfig": {
      "extra_body": {
        "enable_thinking": true,
        "thinking_budget": 4096
      }
    }
  }
}
```

**Notes:**

- `enable_thinking` enables the model's chain-of-thought reasoning.
- `thinking_budget` sets the maximum token budget for the reasoning phase. Values below 1024 are not recommended.
- Qwen3-Coder (the flagship 480B MoE model) operates in non-thinking mode only. Thinking mode applies to other Qwen3 models like `qwen3.5-plus`.
- Within an interactive session, you can toggle thinking per-turn using `/think` and `/no_think` prefixes in your prompt.

## Logging

**Session history:** Project-scoped JSONL files stored at `~/.qwen/projects/<sanitized-cwd>/chats/`. Controlled by `--chat-recording` (disable to prevent session persistence; `--continue` and `--resume` will not work).

**OpenAI API logging:** Records request/response pairs as JSON files for debugging.

- Enable: `--openai-logging` flag or `model.enableOpenAILogging` in settings
- Directory: `--openai-logging-dir <path>` or `model.openAILoggingDir` in settings (default: `logs/openai` relative to cwd)

**Debug mode:** `-d, --debug` enables verbose debug output to stderr.

**Telemetry:** Configurable via `telemetry.*` settings in `settings.json`. Supports local and GCP targets with OTLP export. Privacy opt-out: `privacy.usageStatisticsEnabled: false`.

## Features

### Slash commands (interactive)

| Command | Description |
|---------|-------------|
| `/help` | Display available commands |
| `/clear` | Clear conversation history (`Ctrl+L`) |
| `/compress` | Compress history to save tokens |
| `/stats` | Show current session statistics |
| `/context` | Show context window usage breakdown |
| `/model` | Switch model in current session |
| `/model --fast <model>` | Set lighter model for prompt suggestions |
| `/auth` | Change authentication method |
| `/approval-mode <mode>` | Set approval mode |
| `/plan [task]` | Enter plan mode (or `/plan exit`) |
| `/mcp` | List configured MCP servers and tools |
| `/tools` | Display available tool list |
| `/skills` | List and run available skills |
| `/extensions` | List active extensions |
| `/review` | Code review with 5 parallel agents |
| `/loop <interval> <prompt>` | Run a prompt on a recurring schedule |
| `/btw <question>` | Quick side question without affecting main conversation |
| `/copy` | Copy last output to clipboard |
| `/memory` | Manage instruction context |
| `/bug` | Submit a bug report |
| `/quit` or `/exit` | Exit Qwen Code |

### Custom commands

Markdown files in `~/.qwen/commands/` (global) or `.qwen/commands/` (project). Subdirectories create colon-separated names (e.g., `.qwen/commands/git/commit.md` becomes `/git:commit`). Support `{{args}}` parameter injection, `!{shell command}` dynamic content, and `@{file}` content injection.

### IDE integration

- **VS Code:** [Qwen Code Companion extension](https://marketplace.visualstudio.com/items?itemName=qwenlm.qwen-code-vscode-ide-companion) (sidebar integration)
- **Zed:** Native LSP/agent integration
- **JetBrains:** Editor support for IntelliJ/PyCharm/etc.

### Channels (messaging platform access)

Qwen Code can be accessed through Telegram, WeChat, and DingTalk channels for conversational coding assistance outside the terminal.

### SubAgents

Built-in sub-agent system for parallel task execution and context sharing between agent forks.

### Skills

Agent Skills (GA since Feb 2026) provide specialized workflows. Skills live in `.qwen/skills/` directories, each containing a `SKILL.md`. Enable with `--experimental-skills` (now GA, flag may still be accepted).

### Hooks

Hooks allow running custom commands before/after tool executions. Configured in `settings.json`.

## CLI Options

### Subcommands

| Subcommand | Description |
|------------|-------------|
| `qwen [query..]` | Launch interactive session (default). Positional text runs as one-shot prompt. |
| `qwen auth` | Interactive authentication setup |
| `qwen auth qwen-oauth` | Authenticate with Qwen OAuth (deprecated) |
| `qwen auth coding-plan` | Authenticate with Alibaba Cloud Coding Plan |
| `qwen auth coding-plan --region china --key sk-sp-...` | Non-interactive Coding Plan setup |
| `qwen auth status` | Show current authentication status |
| `qwen mcp` | Manage MCP servers |
| `qwen mcp add <name> <commandOrUrl> [args..]` | Add an MCP server (stdio, sse, or http transport) |
| `qwen mcp remove <name>` | Remove an MCP server |
| `qwen mcp list` | List configured MCP servers |
| `qwen extensions install <source>` | Install extension from git URL, local path, or marketplace |
| `qwen extensions uninstall <name>` | Uninstall an extension |
| `qwen extensions list` | List installed extensions |
| `qwen extensions update [name] [--all]` | Update one or all extensions |
| `qwen extensions disable [--scope] <name>` | Disable an extension |
| `qwen extensions enable [--scope] <name>` | Enable an extension |
| `qwen extensions link <path>` | Link extension from local path (live updates) |
| `qwen extensions new <path> [template]` | Scaffold a new extension |
| `qwen extensions settings <command>` | Manage extension settings |

### Switches

| Flag | Description |
|------|-------------|
| `-m, --model <string>` | Model to use for the session |
| `-p, --prompt <string>` | Run in headless mode with given prompt |
| `-i, --prompt-interactive <string>` | Execute prompt then continue interactively |
| `-y, --yolo` | Auto-approve all tool calls (YOLO mode) |
| `--approval-mode <mode>` | Set approval mode: `plan`, `default`, `auto-edit`, `yolo` |
| `-s, --sandbox` | Run in sandbox |
| `--sandbox-image <string>` | Custom sandbox container image |
| `-c, --continue` | Resume the most recent session for current project |
| `-r, --resume [id]` | Resume a specific session by ID (no ID shows picker) |
| `--max-session-turns <n>` | Maximum number of session turns |
| `-o, --output-format <fmt>` | Output format: `text`, `json`, `stream-json` |
| `--input-format <fmt>` | Input format: `text`, `stream-json` |
| `--include-partial-messages` | Include partial messages in stream-json output |
| `--system-prompt <string>` | Override the built-in main session system prompt |
| `--append-system-prompt <string>` | Append extra instructions to the system prompt |
| `-d, --debug` | Enable debug mode |
| `-a, --all-files` | Include all files in context |
| `-e, --extensions <list>` | Extensions to use (default: all). Use `-e none` to disable all. |
| `-l, --list-extensions` | List available extensions and exit |
| `--allowed-tools <list>` | Tools to allow without confirmation |
| `--exclude-tools <list>` | Tools to exclude |
| `--core-tools <list>` | Core tool paths |
| `--allowed-mcp-server-names <list>` | Allowed MCP server names |
| `--include-directories <list>` | Additional workspace directories (max 5) |
| `--auth-type <type>` | Auth type: `openai`, `anthropic`, `gemini`, `vertex-ai` |
| `--openai-api-key <string>` | OpenAI API key |
| `--openai-base-url <string>` | Custom OpenAI-compatible base URL |
| `--openai-logging` | Enable OpenAI API call logging |
| `--openai-logging-dir <path>` | Directory for OpenAI API logs |
| `--tavily-api-key <string>` | Tavily API key for web search |
| `--google-api-key <string>` | Google Custom Search API key |
| `--google-search-engine-id <string>` | Google Custom Search Engine ID |
| `--web-search-default <provider>` | Default web search provider: `dashscope`, `tavily`, `google` |
| `--chat-recording` | Enable/disable chat recording to disk |
| `--checkpointing` | Enable file edit checkpointing |
| `--acp` | Start agent in ACP (Agent Client Protocol) mode |
| `--experimental-lsp` | Enable experimental LSP support |
| `--experimental-skills` | Enable Skills feature |
| `--channel <string>` | Channel identifier: `VSCode`, `ACP`, `SDK`, `CI` |
| `--screen-reader` | Enable screen reader accessibility mode |
| `--vlm-switch-mode <mode>` | VLM behavior on image input: `once`, `session`, `persist` |
| `--proxy <string>` | HTTP proxy |
| `--telemetry` | Enable telemetry |
| `--telemetry-target <target>` | Telemetry target: `local`, `gcp` |
| `--telemetry-otlp-endpoint <url>` | OTLP endpoint for telemetry |
| `--telemetry-otlp-protocol <proto>` | OTLP protocol: `grpc`, `http` |
| `--telemetry-log-prompts` | Log prompts in telemetry |
| `--telemetry-outfile <path>` | Redirect telemetry output to file |
| `--show-memory-usage` | Display current memory usage |
| `-v, --version` | Show version number |
| `-h, --help` | Show help |

## CLI Switch Summary

### `-m, --model <string>`

Specifies the model to use for the session.

**Default:** Determined by auth type; typically `qwen3-coder-plus` or `qwen3.6-plus` for Alibaba Cloud.

```bash
qwen -m qwen3.6-plus
qwen -m claude-sonnet-4-20250514
qwen -m gemini-2.5-pro
```

### `-p, --prompt <string>`

Run in headless (non-interactive) mode. The agent processes the prompt and exits without launching the TUI. Ideal for scripts, CI/CD, and one-shot automation.

**Default:** Not set (interactive mode launches).

```bash
qwen -p "Explain the architecture of this project"
qwen -p "Find bugs in the auth module" --output-format json
```

### `-i, --prompt-interactive <string>`

Execute the prompt, then drop into the interactive TUI to continue the conversation. Cannot be combined with stdin piping.

**Default:** Not set.

```bash
qwen -i "Refactor the auth module"
```

### `--system-prompt <string>`

Override the built-in main session system prompt for this run only. Loaded context files (`QWEN.md`, etc.) are still appended after the override. Can be combined with `--append-system-prompt`.

**Default:** Not set (uses built-in system prompt).

```bash
qwen -p "Review this patch" --system-prompt "You are a terse release reviewer. Report only blocking issues."
```

### `--append-system-prompt <string>`

Append extra instructions to the main session system prompt. Applied after the built-in prompt and loaded context files. Can be combined with `--system-prompt`.

**Default:** Not set.

```bash
qwen -p "Review this patch" --append-system-prompt "Be terse and focus on concrete findings."
qwen -p "Summarize" --system-prompt "You are a migration planner." --append-system-prompt "Return exactly three bullets."
```

### `-y, --yolo`

Enable YOLO mode, which automatically approves all tool calls including file edits and shell commands. Equivalent to `--approval-mode yolo`. Cannot be combined with `--approval-mode`.

**Default:** Not set (default approval mode).

```bash
qwen -y -p "Run the test suite, fix failures, and commit"
```

### `--approval-mode <mode>`

Set the approval mode for tool usage. Cannot be used together with `--yolo`.

**Default:** `default`

**Possible values:**
- `plan` -- read-only analysis, no file modifications or command execution
- `default` -- require approval for file edits and shell commands
- `auto-edit` -- auto-approve file edits, require approval for shell commands
- `yolo` -- auto-approve all tool calls

```bash
qwen --approval-mode plan -p "Analyze this codebase"
qwen --approval-mode auto-edit -p "Refactor the utils module"
```

### `-s, --sandbox`

Run the agent in a sandboxed environment. On macOS, uses Seatbelt (`sandbox-exec`). On Linux, uses Docker/Podman containers.

**Default:** Not set (no sandbox).

```bash
qwen -s -p "Analyze this untrusted code"
```

### `--sandbox-image <string>`

Specify a custom container image for the sandbox. Only applies when `--sandbox` is enabled and running on a container-based sandbox (Linux).

**Default:** Uses the built-in default container image.

```bash
qwen -s --sandbox-image "python:3.12-slim" -p "Run this Python script"
```

### `-c, --continue`

Resume the most recent session for the current project. Restores conversation history, tool outputs, and chat-compression checkpoints before sending a new prompt. Requires chat recording to be enabled.

**Default:** Not set (starts a new session).

```bash
qwen --continue -p "What was the next step?"
qwen -c
```

### `-r, --resume [id]`

Resume a specific session by ID. If no ID is provided, shows an interactive session picker.

**Default:** Not set.

```bash
qwen --resume 123e4567-e89b-12d3-a456-426614174000 -p "Continue the refactor"
qwen -r
```

### `--max-session-turns <n>`

Cap the number of agent turns (user/model/tool interaction cycles). Useful for controlling costs in CI/CD pipelines. Set to `-1` for unlimited.

**Default:** `-1` (unlimited).

```bash
qwen --max-session-turns 5 -p "Fix the lint errors"
```

### `-o, --output-format <fmt>`

Set the output format for headless mode.

**Default:** `text`

**Possible values:**
- `text` -- human-readable plain text
- `json` -- buffered JSON array emitted at end of execution (machine-readable)
- `stream-json` -- line-delimited JSON emitted as events occur (real-time streaming)

```bash
qwen -p "What is Kubernetes?" --output-format json
qwen -p "Write code" --output-format stream-json --include-partial-messages
```

### `--input-format <fmt>`

Set the input format consumed from standard input.

**Default:** `text`

**Possible values:**
- `text` -- standard text input from stdin or command-line arguments
- `stream-json` -- JSON message protocol via stdin for bidirectional SDK communication. Requires `--output-format stream-json`.

```bash
qwen --input-format stream-json --output-format stream-json
```

### `--include-partial-messages`

Include partial assistant messages in `stream-json` output. Emits stream events (`message_start`, `content_block_delta`, etc.) for real-time UI updates. Requires `--output-format stream-json`.

**Default:** Not set (partial messages excluded).

```bash
qwen -p "Write a Python script" --output-format stream-json --include-partial-messages
```

### `-d, --debug`

Enable debug mode for verbose output to stderr. Useful for troubleshooting API requests, tool calls, and internal behavior.

**Default:** Not set.

```bash
qwen -d -p "Explain this code"
```

### `-a, --all-files`

Recursively include all files within the current directory as context for the prompt. Use sparingly on large projects as it consumes significant tokens.

**Default:** Not set.

```bash
qwen -a -p "What does this project do?"
```

### `-e, --extensions <list>`

Specify which extensions to load for the session. Can be specified multiple times. Use `-e none` to disable all extensions.

**Default:** All available extensions are loaded.

```bash
qwen -e my-extension -e my-other-extension
qwen -e none
```

### `-l, --list-extensions`

List all available extensions and exit. Useful for discovering what extensions are installed.

**Default:** Not set.

```bash
qwen -l
```

### `--allowed-tools <list>`

A comma-separated list of tool names that bypass the confirmation dialog. Supports the same rule syntax as `permissions.allow`.

**Default:** Not set.

```bash
qwen --allowed-tools "Bash(git status),Read"
qwen -p "Run tests" --allowed-tools "Shell(npm run *)"
```

### `--exclude-tools <list>`

A list of tool names to exclude from the session. Tools listed here will not be available to the model.

**Default:** Not set.

```bash
qwen --exclude-tools "write_file,web_fetch"
```

### `--core-tools <list>`

Restrict available built-in tools to an allowlist. All tools not in the list are disabled.

**Default:** Not set (all core tools available).

```bash
qwen --core-tools "read_file,edit,run_shell_command"
```

### `--allowed-mcp-server-names <list>`

Restrict which MCP servers are connected to by name. Overrides `mcp.allowed` and `mcp.excluded` settings.

**Default:** Not set (all configured servers are connected).

```bash
qwen --allowed-mcp-server-names "puppeteer,filesystem"
```

### `--include-directories <list>`

Include additional directories in the workspace context. Supports absolute paths, relative paths, and `~` expansion. Maximum of 5 directories.

**Default:** Not set.

```bash
qwen --include-directories /path/to/project1,/path/to/project2
qwen --include-directories ../shared-lib --include-directories ~/common-utils
```

### `--auth-type <type>`

Select the authentication/provider protocol. This determines how API requests are formatted and which credentials are used.

**Default:** Determined by `security.auth.selectedType` in settings.

**Possible values:**
- `openai` -- OpenAI-compatible API (Alibaba Cloud DashScope, OpenRouter, etc.)
- `anthropic` -- Anthropic Claude models
- `gemini` -- Google Gemini models
- `vertex-ai` -- Google Vertex AI

```bash
qwen --auth-type openai
qwen --auth-type anthropic
```

### `--openai-api-key <string>`

Set the OpenAI-compatible API key directly on the command line. Overrides environment variables and settings.

**Default:** Not set (uses `OPENAI_API_KEY` env var or `envKey` from settings).

```bash
qwen --openai-api-key "sk-xxxxxxxxx" -m gpt-4o
```

### `--openai-base-url <string>`

Set a custom base URL for the OpenAI-compatible API endpoint. Overrides the provider's default URL.

**Default:** Not set (uses provider default, e.g., `https://api.openai.com/v1`).

```bash
qwen --openai-base-url "https://dashscope.aliyuncs.com/compatible-mode/v1" --openai-api-key "sk-xxx"
```

### `--openai-logging`

Enable logging of OpenAI API calls (requests and responses) to JSON files for debugging. Overrides `model.enableOpenAILogging` in settings.

**Default:** Not set.

```bash
qwen --openai-logging --openai-logging-dir ~/qwen-logs
```

### `--openai-logging-dir <path>`

Set the directory for OpenAI API log files. Supports absolute paths, relative paths, and `~` expansion.

**Default:** `logs/openai` relative to current working directory.

```bash
qwen --openai-logging-dir "~/qwen-logs" --openai-logging
qwen --openai-logging-dir "./api-logs"
```

### `--tavily-api-key <string>`

Set the Tavily API key for web search functionality. Enables the `web_search` tool with Tavily as the provider.

**Default:** Not set (uses `advanced.tavilyApiKey` from settings or DashScope for Qwen OAuth users).

```bash
qwen --tavily-api-key "tvly-xxxxxxxxx"
```

### `--google-api-key <string>`

Set the Google Custom Search API key for web search functionality.

**Default:** Not set.

```bash
qwen --google-api-key "AIza-xxxxxxxxx"
```

### `--google-search-engine-id <string>`

Set the Google Custom Search Engine ID for web search functionality.

**Default:** Not set.

```bash
qwen --google-search-engine-id "xxxxxxxxx"
```

### `--web-search-default <provider>`

Set the default web search provider.

**Default:** Determined by auth type (DashScope for Qwen users).

**Possible values:** `dashscope`, `tavily`, `google`

```bash
qwen --web-search-default tavily --tavily-api-key "tvly-xxx"
```

### `--chat-recording`

Enable or disable chat recording to disk. When disabled, sessions are not persisted and `--continue`/`--resume` will not work.

**Default:** Enabled.

```bash
qwen --chat-recording=false -p "Quick question"
```

### `--checkpointing`

Enable file edit checkpointing, allowing `/restore` to revert files to their state before a tool execution.

**Default:** Determined by `general.checkpointing.enabled` in settings.

```bash
qwen --checkpointing
```

### `--acp`

Start the agent in ACP (Agent Client Protocol) mode. Used for IDE/editor integrations (Zed, VS Code sidebar). Replaces the deprecated `--experimental-acp` flag.

**Default:** Not set.

```bash
qwen --acp
```

### `--experimental-lsp`

Enable experimental LSP (Language Server Protocol) support for code intelligence features (go-to-definition, find references, diagnostics). Requires language servers to be installed separately.

**Default:** Not set.

```bash
qwen --experimental-lsp
```

### `--experimental-skills`

Enable the Skills feature. Skills provide specialized workflows loaded from `.qwen/skills/` directories. (Note: Skills are now GA as of Feb 2026; this flag may still be accepted but is no longer required.)

**Default:** Not set.

```bash
qwen --experimental-skills
```

### `--channel <string>`

Set the channel identifier. Used to tag the session's origin for analytics and behavior customization.

**Default:** Not set.

**Possible values:** `VSCode`, `ACP`, `SDK`, `CI`

```bash
qwen --channel CI -p "Run the test suite"
```

### `--screen-reader`

Enable screen reader accessibility mode. Adjusts the TUI for better compatibility with screen reading software.

**Default:** Not set.

```bash
qwen --screen-reader
```

### `--vlm-switch-mode <mode>`

Control the vision-language model behavior when image input is detected. Determines whether the agent switches to a vision-capable model and for how long.

**Default:** Determined by settings.

**Possible values:**
- `once` -- switch for the current turn only
- `session` -- switch for the remainder of the session
- `persist` -- switch permanently until explicitly changed

```bash
qwen --vlm-switch-mode session
```

### `--proxy <string>`

Set an HTTP proxy for API requests.

**Default:** Not set.

```bash
qwen --proxy "http://localhost:7890"
```

### `--telemetry`

Enable telemetry collection. Overrides `telemetry.enabled` in settings.

**Default:** Not set.

```bash
qwen --telemetry
```

### `--telemetry-target <target>`

Set the telemetry destination.

**Default:** Not set.

**Possible values:** `local`, `gcp`

```bash
qwen --telemetry --telemetry-target local
```

### `--telemetry-otlp-endpoint <url>`

Set the OTLP endpoint for telemetry export.

**Default:** Not set.

```bash
qwen --telemetry --telemetry-target local --telemetry-otlp-endpoint "http://localhost:4317"
```

### `--telemetry-otlp-protocol <proto>`

Set the OTLP protocol for telemetry export.

**Default:** `grpc`

**Possible values:** `grpc`, `http`

```bash
qwen --telemetry --telemetry-otlp-protocol http
```

### `--telemetry-log-prompts`

Enable logging of user prompts in telemetry data.

**Default:** Not set.

```bash
qwen --telemetry --telemetry-log-prompts
```

### `--telemetry-outfile <path>`

Redirect telemetry output to a file. Used when `telemetry-target` is `local`.

**Default:** Not set.

```bash
qwen --telemetry --telemetry-target local --telemetry-outfile ./telemetry.log
```

### `--show-memory-usage`

Display the current memory usage of the Qwen Code process.

**Default:** Not set.

```bash
qwen --show-memory-usage
```

### `-v, --version`

Display the version number and exit.

```bash
qwen -v
qwen --version
```

### `-h, --help`

Display help information about CLI arguments and exit.

```bash
qwen -h
qwen --help
```

## Sources

- [QwenLM/qwen-code GitHub repository](https://github.com/QwenLM/qwen-code)
- [Qwen Code documentation](https://qwenlm.github.io/qwen-code-docs/)
- [Qwen Code settings reference](https://qwenlm.github.io/qwen-code-docs/en/users/configuration/settings/)
- [Qwen Code authentication](https://qwenlm.github.io/qwen-code-docs/en/users/configuration/auth/)
- [Qwen Code approval mode](https://qwenlm.github.io/qwen-code-docs/en/users/features/approval-mode/)
- [Qwen Code headless mode](https://qwenlm.github.io/qwen-code-docs/en/users/features/headless/)
- [Qwen Code commands](https://qwenlm.github.io/qwen-code-docs/en/users/features/commands/)
- [Qwen Code model providers](https://qwenlm.github.io/qwen-code-docs/en/users/configuration/model-providers/)
- [Qwen3-Coder announcement](https://qwenlm.github.io/blog/qwen3-coder/)
- [Qwen3-Coder model card](https://huggingface.co/Qwen/Qwen3-Coder-480B-A35B-Instruct)
