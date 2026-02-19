---
homepage: https://qwen.ai/
docs: https://qwenlm.github.io/qwen-code-docs/
cli_docs: https://qwenlm.github.io/qwen-code-docs/en/users/overview
---

# Qwen Code CLI

Qwen Code is an open-source AI agent from Alibaba that runs in the terminal. It is forked from Gemini CLI and adapted with customized prompts and function-calling protocols optimized for Qwen3-Coder. Installed via npm (`npm i -g @qwen-code/qwen-code@latest`) or Homebrew (`brew install qwen-code`). Requires Node.js 20+. Current version: 0.9.0.

## Model Specification

**CLI flag:** `-m, --model <model-name>`

```bash
qwen -m qwen3-coder-plus
```

**Default model resolution** (highest to lowest priority):

1. `--model` CLI flag
2. `OPENAI_MODEL` environment variable
3. `model.name` in `~/.qwen/settings.json`
4. The model associated with the active auth type (Qwen OAuth defaults to `qwen3-coder-plus`)

**Interactive switching:** Use the `/model` command inside a session to switch between all configured models.

**Configuring additional models** in `~/.qwen/settings.json`:

```json
{
  "modelProviders": {
    "openai": [
      {
        "id": "qwen3-coder-plus",
        "name": "Qwen3-Coder-Plus",
        "baseUrl": "https://dashscope.aliyuncs.com/compatible-mode/v1",
        "envKey": "DASHSCOPE_API_KEY"
      }
    ]
  },
  "model": {
    "name": "qwen3-coder-plus"
  }
}
```

Models from any OpenAI-compatible, Anthropic, Gemini, or Vertex AI provider can be added under the corresponding key in `modelProviders`. The `--auth-type` flag selects which provider protocol to use: `openai`, `anthropic`, `qwen-oauth`, `gemini`, or `vertex-ai`.

## Non-interactive Engagement

Non-interactive (headless) mode is fully supported. There are several ways to run Qwen Code without the interactive TUI.

### Positional prompt (preferred)

```bash
qwen "Explain the architecture of this project"
```

The positional form runs the prompt as a one-shot task and exits. This is the current recommended approach.

### `-p, --prompt` flag (deprecated)

```bash
qwen -p "Explain the architecture of this project"
```

Functionally identical to the positional form. Marked deprecated in favor of positional usage.

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

### Output formats for automation

| Flag | Format | Use case |
|------|--------|----------|
| `-o text` | Plain text (default) | Human-readable output |
| `-o json` | Buffered JSON array | Programmatic processing |
| `-o stream-json` | Line-delimited JSON | Real-time streaming |

Use `--include-partial-messages` with `stream-json` to receive incremental assistant tokens.

### Session resumption in headless mode

```bash
qwen --continue "What was the next step?"
qwen --resume <sessionId> "Continue from here"
```

### Turn limits

`--max-session-turns <N>` caps the number of agent turns, useful for CI budgets.

### Limitations

- Qwen OAuth cannot authenticate in headless/CI environments (no browser). Use API-KEY auth (`--auth-type openai`) with environment variables instead.
- The `-p` flag is deprecated; prefer positional prompts.

## Subscription versus Per Call API

**Qwen OAuth (free tier):** 1,000 requests/day, 60 requests/minute. No credit card required. Authenticate via `/auth` in interactive mode or browser flow on first launch.

**Alibaba Cloud Bailian Coding Plan (subscription):** Fixed monthly fee with higher quotas. Requires an active subscription from Alibaba Cloud ModelStudio. Uses a dedicated API key prefixed `sk-sp-`. Configure with:

```bash
export DASHSCOPE_API_KEY="sk-sp-xxxxxxxxx"
qwen --auth-type openai -m qwen3-coder-plus
```

**Third-party per-call API:** Use any OpenAI-compatible, Anthropic, or Gemini provider with their own per-token pricing. Set the appropriate environment variables (`OPENAI_API_KEY`, `ANTHROPIC_API_KEY`, `GEMINI_API_KEY`) and `--auth-type` flag.

For headless/CI non-interactive usage, both the Bailian subscription key and third-party API keys work. Qwen OAuth does not work in headless environments.

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

**Tool-level control** in `settings.json`:

- `tools.allowed` -- tools that bypass confirmation prompts
- `tools.exclude` -- tools to deny (not a security mechanism, uses string matching)
- `tools.core` -- allowlist of permitted tools
- `--allowed-tools` and `--exclude-tools` CLI flags

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
- Qwen3-Coder (the flagship 480B MoE model) operates in non-thinking mode only and does not produce `<think>` blocks. Thinking mode applies to other Qwen3 models like `qwen3.5-plus`.
- Within an interactive session, you can toggle thinking per-turn using `/think` and `/no_think` prefixes in your prompt.

## Logging

**Session history:** Project-scoped JSONL files stored at `~/.qwen/projects/<sanitized-cwd>/chats/`. Controlled by `--chat-recording` (disable to prevent session persistence; `--continue` and `--resume` will not work).

**OpenAI API logging:** Records request/response pairs as JSON files for debugging.

- Enable: `--openai-logging` flag or `model.enableOpenAILogging` in settings
- Directory: `--openai-logging-dir <path>` or `model.openAILoggingDir` in settings (default: `logs/openai` relative to cwd)

**Debug mode:** `-d, --debug` enables verbose debug output to stderr.

**Telemetry:** Configurable via `telemetry.*` settings in `settings.json`. Supports local and GCP targets with OTLP export. Privacy opt-out: `privacy.usageStatisticsEnabled: false`.

## CLI Options

### Subcommands

| Subcommand | Description |
|------------|-------------|
| `qwen [query..]` | Launch interactive session (default). Positional text runs as one-shot prompt. |
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
| `-p, --prompt <string>` | Prompt (deprecated; use positional args) |
| `-i, --prompt-interactive <string>` | Execute prompt then continue interactively |
| `-y, --yolo` | Auto-approve all tool calls (YOLO mode) |
| `--approval-mode <mode>` | Set approval mode: `plan`, `default`, `auto-edit`, `yolo` |
| `-s, --sandbox` | Run in sandbox |
| `--sandbox-image <string>` | Custom sandbox container image (deprecated) |
| `-c, --continue` | Resume the most recent session for current project |
| `-r, --resume [id]` | Resume a specific session by ID (no ID shows picker) |
| `--max-session-turns <n>` | Maximum number of session turns |
| `-o, --output-format <fmt>` | Output format: `text`, `json`, `stream-json` |
| `--input-format <fmt>` | Input format: `text`, `stream-json` |
| `--include-partial-messages` | Include partial messages in stream-json output |
| `-d, --debug` | Enable debug mode |
| `-a, --all-files` | Include all files in context (deprecated) |
| `-e, --extensions <list>` | Extensions to use (default: all) |
| `-l, --list-extensions` | List available extensions and exit |
| `--allowed-tools <list>` | Tools to allow without confirmation |
| `--exclude-tools <list>` | Tools to exclude |
| `--core-tools <list>` | Core tool paths |
| `--allowed-mcp-server-names <list>` | Allowed MCP server names |
| `--include-directories <list>` | Additional workspace directories |
| `--auth-type <type>` | Auth type: `openai`, `anthropic`, `qwen-oauth`, `gemini`, `vertex-ai` |
| `--openai-api-key <string>` | OpenAI API key |
| `--openai-base-url <string>` | Custom OpenAI-compatible base URL |
| `--openai-logging` | Enable OpenAI API call logging |
| `--openai-logging-dir <path>` | Directory for OpenAI API logs |
| `--tavily-api-key <string>` | Tavily API key for web search |
| `--google-api-key <string>` | Google Custom Search API key |
| `--google-search-engine-id <string>` | Google Custom Search Engine ID |
| `--web-search-default <provider>` | Default web search provider: `dashscope`, `tavily`, `google` |
| `--chat-recording` | Enable/disable chat recording to disk |
| `--checkpointing` | Enable file edit checkpointing (deprecated) |
| `--acp` | Start agent in ACP mode |
| `--experimental-skills` | Enable experimental Skills feature |
| `--experimental-lsp` | Enable experimental LSP support |
| `--channel <string>` | Channel identifier: `VSCode`, `ACP`, `SDK`, `CI` |
| `--screen-reader` | Enable screen reader accessibility mode |
| `--vlm-switch-mode <mode>` | VLM behavior on image input: `once`, `session`, `persist` |
| `--proxy <string>` | HTTP proxy (deprecated; use settings.json) |
| `--telemetry` | Enable telemetry (deprecated; use settings.json) |
| `--telemetry-target <target>` | Telemetry target: `local`, `gcp` (deprecated) |
| `--telemetry-otlp-endpoint <url>` | OTLP endpoint for telemetry (deprecated) |
| `--telemetry-otlp-protocol <proto>` | OTLP protocol: `grpc`, `http` (deprecated) |
| `--telemetry-log-prompts` | Log prompts in telemetry (deprecated) |
| `--telemetry-outfile <path>` | Redirect telemetry output to file (deprecated) |
| `-v, --version` | Show version number |
| `-h, --help` | Show help |

## Sources

- [QwenLM/qwen-code GitHub repository](https://github.com/QwenLM/qwen-code)
- [Qwen Code documentation](https://qwenlm.github.io/qwen-code-docs/)
- [Qwen Code settings reference](https://qwenlm.github.io/qwen-code-docs/en/users/configuration/settings/)
- [Qwen Code authentication](https://qwenlm.github.io/qwen-code-docs/en/users/configuration/auth/)
- [Qwen Code approval mode](https://qwenlm.github.io/qwen-code-docs/en/users/features/approval-mode/)
- [Qwen Code headless mode](https://qwenlm.github.io/qwen-code-docs/en/users/features/headless/)
- [Qwen3-Coder announcement](https://qwenlm.github.io/blog/qwen3-coder/)
- [Qwen3-Coder model card](https://huggingface.co/Qwen/Qwen3-Coder-480B-A35B-Instruct)
