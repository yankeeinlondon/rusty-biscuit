---
homepage: https://www.kimi.com/code
docs: https://moonshotai.github.io/kimi-cli/en/
cli_docs: https://moonshotai.github.io/kimi-cli/en/reference/kimi-command
---

# Kimi Code CLI

Kimi Code CLI is Moonshot AI's open-source (Apache 2.0) terminal-based AI coding agent. It is written in Python and installed via `uv`. The CLI supports interactive shell mode, non-interactive print mode, a Wire protocol for custom UIs, and an ACP server mode for IDE integration.

As of v1.7.0 the CLI supports six LLM providers (Kimi, OpenAI Legacy, OpenAI Responses, Anthropic, Gemini, Vertex AI) and can be authenticated either via OAuth browser login or API key.

## Model Specification

**CLI flag:** `--model <NAME>` or `-m <NAME>`

The model name must reference a model already defined in the config file (`~/.kimi/config.toml`). It does not accept arbitrary model identifiers -- the model must exist under the `[models.<name>]` table in config.

**Default model:** Set via the `default_model` key in `config.toml`. When omitted, the CLI uses whatever model was configured during initial `/login` setup.

**Environment override:** `KIMI_MODEL_NAME` overrides the model identifier sent in API calls. Additional environment variables allow overriding model properties without touching config:

| Variable | Purpose |
|----------|---------|
| `KIMI_MODEL_NAME` | Override model identifier |
| `KIMI_MODEL_MAX_CONTEXT_SIZE` | Override token limit |
| `KIMI_MODEL_CAPABILITIES` | Comma-separated: `thinking`, `always_thinking`, `image_in`, `video_in` |
| `KIMI_MODEL_TEMPERATURE` | Override temperature |
| `KIMI_MODEL_TOP_P` | Override top-p sampling |
| `KIMI_MODEL_MAX_TOKENS` | Override max tokens per response |

**Interactive model switching:** Use the `/model` slash command inside a session to change model or toggle thinking mode without restarting.

**Model capabilities:** Each model definition can declare capabilities via an array: `thinking` (toggleable deep reasoning), `always_thinking` (cannot be disabled), `image_in` (clipboard image support via Ctrl-V), `video_in` (video content support).

## Non-interactive Engagement

Kimi Code CLI supports non-interactive usage through several mechanisms:

### 1. Print Mode (`--print`)

The primary non-interactive mode. The CLI processes the prompt, executes tools, prints output, and exits.

```bash
# Via -p flag
kimi --print -p "Explain the architecture of this project"

# Via stdin
echo "List all TODO comments" | kimi --print
```

**Key behaviors:**
- Implicitly enables `--yolo` (auto-approves all tool calls)
- Outputs to stdout in plain text by default
- Exits after the response completes

**Output control:**
- `--output-format text` (default) -- plain text output
- `--output-format stream-json` -- JSONL output, one JSON object per line
- `--final-message-only` -- suppress intermediate tool call output, print only the final assistant message

### 2. Quiet Mode (`--quiet`)

A convenience shortcut equivalent to `--print --output-format text --final-message-only`. Useful when only the final answer matters:

```bash
kimi --quiet -p "What version of Rust is this project using?"
```

### 3. Streaming JSON I/O (`--input-format stream-json`)

For programmatic integration, combine with `--print` to accept JSONL input on stdin and produce JSONL output. The CLI reads user messages continuously until stdin closes:

```bash
echo '{"role":"user","content":"Hello"}' | kimi --print --input-format stream-json --output-format stream-json
```

### 4. Wire Mode (`--wire`)

An experimental JSON-RPC 2.0 bidirectional protocol over stdin/stdout (Wire protocol v1.3). Wire mode exposes the internal message-passing layer that the Shell UI and IDE integrations use, enabling:

- Custom UI development (web, desktop, mobile)
- Application integration (embed the agent into other software)
- Automated testing of agent behavior

Wire mode supports streaming content, external tool registration via an `initialize` handshake, token usage tracking, session replay (`replay` message in Wire 1.3+), and cancellation.

If only simple non-interactive I/O is needed, print mode is simpler.

### 5. ACP Server Mode (`--acp` or `kimi acp`)

Runs as an Agent Client Protocol server for IDE integration. The `--acp` top-level flag is deprecated in favor of the `kimi acp` subcommand.

### 6. Prompt Flag Without Print (`-p` / `--prompt`)

Passing `-p "prompt"` without `--print` starts an interactive session with the prompt pre-submitted. The CLI remains interactive after the first response.

## Subscription versus Per Call API

Kimi Code CLI supports two billing models depending on the provider:

**Kimi Code Subscription (OAuth login):**
- Authenticate via `kimi login` or the `/login` slash command, which opens a browser for OAuth
- Quota is included as part of a Kimi Membership (no additional fees beyond the subscription)
- Quotas operate on a rolling 7-day cycle; unused quota does not carry over
- Non-interactive sessions work with OAuth credentials; run `kimi login` once before using `--print` or `--wire`

**Moonshot AI Open Platform (API key):**
- Pay-per-token pricing via API key from `platform.moonshot.ai`
- Set via `KIMI_API_KEY` environment variable or configured in `config.toml`
- Standard token-based billing (input and output tokens priced separately)

**Third-party providers (OpenAI, Anthropic, Gemini, Vertex AI):**
- Each provider uses its own API key and billing
- Set via `OPENAI_API_KEY`, or configured in the provider section of `config.toml`
- Billed per the provider's standard pricing

There is no CLI flag to switch between subscription and API billing -- the distinction is determined by which provider is configured and how authentication was performed.

## System Prompt

The system prompt is controlled through the **agent specification** system rather than a simple CLI flag.

**Custom agent file (`--agent-file <PATH>`):**
An agent spec is a YAML file that defines `system_prompt_path` (path to a prompt template file), along with tools, subagents, and other behavior. Create a custom agent file to supply a fully custom system prompt:

```yaml
extend: default
name: my-agent
system_prompt_path: ./my-prompt.md
```

The `extend: default` directive inherits all default tools and behavior while replacing only the system prompt.

**System prompt template variables:**
- `${KIMI_NOW}` -- current timestamp (ISO format)
- `${KIMI_WORK_DIR}` -- working directory path
- `${KIMI_WORK_DIR_LS}` -- directory listing
- `${KIMI_AGENTS_MD}` -- contents of the project's `AGENTS.md` file
- `${KIMI_SKILLS}` -- loaded skills inventory

**Project-level context (`AGENTS.md`):**
The `/init` slash command generates an `AGENTS.md` file in the project root. Its contents are injected into the system prompt via the `${KIMI_AGENTS_MD}` variable, providing project-specific conventions to the agent.

**Built-in agents:**
- `default` -- general-purpose agent with standard tool set
- `okabe` -- experimental agent with additional tools (e.g., `SendDMail`)

Select via `--agent default` or `--agent okabe`.

There is no standalone `--system-prompt` flag. Modifying the system prompt requires creating an agent spec file.

## Permissions

**Default behavior:** The CLI requests user approval before modifying files, running shell commands, or invoking MCP tools. Users can Allow, Allow for session, or Reject each operation.

**YOLO mode (`--yolo`, `--yes`, `-y`):**
Auto-approves all operations without prompting. This is Kimi's equivalent of a "yolo" mode.

```bash
kimi --yolo -p "Fix all clippy warnings"
```

Can also be enabled persistently via `default_yolo = true` in `config.toml`, or toggled mid-session with the `/yolo` slash command.

**Print mode note:** `--print` implicitly enables `--yolo`.

**Working directory restrictions:** File operations are restricted to the working directory. Absolute paths are required for files outside the working directory.

**Config file:** `~/.kimi/config.toml` with `default_yolo` option. There is no granular per-tool permission configuration beyond the allow/reject approval flow and the YOLO toggle.

## Thinking Level

Thinking mode enables extended reasoning for complex problems.

**CLI flags:**
- `--thinking` -- enable thinking mode
- `--no-thinking` -- disable thinking mode

**Config default:** `default_thinking = true` or `false` in `config.toml` (defaults to `false`).

**Interactive toggle:** Use the `/model` slash command mid-session to toggle thinking on or off.

**Model capability dependency:** Thinking mode only works with models that declare the `thinking` or `always_thinking` capability. Models with `always_thinking` cannot have thinking disabled. There are no granular thinking "levels" (e.g., low/medium/high) -- it is a binary on/off toggle.

## Logging

**Log file location:** `~/.kimi/logs/kimi.log`

**Log levels:**
- Default: INFO level
- `--debug` flag: TRACE level (verbose diagnostic output)
- `--verbose` flag: prints verbose runtime information to the terminal

**Session data:**
- `~/.kimi/sessions/<dir-hash>/<session-id>/context.jsonl` -- message history (JSON Lines)
- `~/.kimi/sessions/<dir-hash>/<session-id>/wire.jsonl` -- Wire event logs for session replay

Sessions are indexed by MD5 hash of the working directory path.

**Other data locations:**
- `~/.kimi/config.toml` -- main configuration
- `~/.kimi/kimi.json` -- runtime metadata (working directories, session IDs)
- `~/.kimi/mcp.json` -- MCP server configuration
- `~/.kimi/credentials/` -- OAuth tokens (file permissions 600)
- `~/.kimi/user-history/` -- shell mode command history

The base directory (`~/.kimi/`) can be relocated via the `KIMI_SHARE_DIR` environment variable.

## CLI Options

### Subcommands

| Subcommand | Description |
|------------|-------------|
| `kimi login` | Log in to Kimi account via OAuth browser flow. `--json` emits events as JSON lines. |
| `kimi logout` | Log out from Kimi account. `--json` emits events as JSON lines. |
| `kimi info` | Show version, agent spec version, wire protocol version, and Python version. |
| `kimi acp` | Run as ACP (Agent Client Protocol) server for IDE integration. |
| `kimi mcp add` | Add an MCP server (stdio or HTTP transport). |
| `kimi mcp remove` | Remove an MCP server by name. |
| `kimi mcp list` | List all configured MCP servers. |
| `kimi mcp auth` | Authorize with an OAuth-enabled MCP server. |
| `kimi mcp reset-auth` | Clear cached OAuth tokens for an MCP server. |
| `kimi mcp test` | Test connection to an MCP server and list available tools. |
| `kimi term` | Run Toad TUI backed by Kimi Code CLI ACP server. |
| `kimi web` | Run the web interface (default port 5494). |

### Switches

| Switch | Short | Description |
|--------|-------|-------------|
| `--version` | `-V` | Show version and exit. |
| `--help` | `-h` | Show help message and exit. |
| `--verbose` | | Print verbose runtime information. |
| `--debug` | | Log debug (TRACE level) information to `~/.kimi/logs/kimi.log`. |
| `--work-dir <DIR>` | `-w` | Set working directory (default: current directory). |
| `--session <ID>` | `-S` | Resume a specific session by ID. |
| `--continue` | `-C` | Continue the most recent session for the working directory. |
| `--config <TOML/JSON>` | | Load config from an inline TOML or JSON string. |
| `--config-file <FILE>` | | Load config from a file (default: `~/.kimi/config.toml`). |
| `--model <NAME>` | `-m` | Select LLM model (must be defined in config). |
| `--thinking` | | Enable thinking mode. |
| `--no-thinking` | | Disable thinking mode. |
| `--yolo` / `--yes` / `--auto-approve` | `-y` | Auto-approve all actions. |
| `--prompt <TEXT>` / `--command <TEXT>` | `-p` / `-c` | Pass a user prompt to the agent. |
| `--print` | | Run in print mode (non-interactive, implies `--yolo`). |
| `--quiet` | | Shortcut for `--print --output-format text --final-message-only`. |
| `--input-format <FMT>` | | Input format: `text` (default) or `stream-json`. Requires `--print`. |
| `--output-format <FMT>` | | Output format: `text` (default) or `stream-json`. Requires `--print`. |
| `--final-message-only` | | Print only the final assistant message (print mode). |
| `--wire` | | Run as Wire server (experimental, JSON-RPC 2.0). |
| `--acp` | | Run as ACP server (deprecated; use `kimi acp`). |
| `--agent <NAME>` | | Built-in agent spec: `default` or `okabe`. |
| `--agent-file <FILE>` | | Custom agent specification YAML file. |
| `--mcp-config-file <FILE>` | | MCP config file to load (repeatable). |
| `--mcp-config <JSON>` | | MCP config JSON string to load (repeatable). |
| `--skills-dir <DIR>` | | Path to skills directory (overrides auto-discovery). |
| `--max-steps-per-turn <N>` | | Max steps in one turn (default: from config, typically 100). |
| `--max-retries-per-step <N>` | | Max retries in one step (default: from config, typically 3). |
| `--max-ralph-iterations <N>` | | Extra iterations after first turn in Ralph mode (-1 = unlimited). |

### Web Subcommand Switches

| Switch | Short | Description |
|--------|-------|-------------|
| `--host <ADDR>` | `-h` | Bind to specific IP address. |
| `--network` | `-n` | Enable network access (bind to `0.0.0.0`). |
| `--port <INT>` | `-p` | Port to bind to (default: 5494). |
| `--reload` | | Enable auto-reload for development. |
| `--open` / `--no-open` | | Open browser automatically (default: open). |
| `--auth-token <TOKEN>` | | Bearer token for API authentication. |
| `--allowed-origins <LIST>` | | Comma-separated allowed Origin values. |
| `--dangerously-omit-auth` | | Disable auth checks (dangerous on public networks). |
| `--restrict-sensitive-apis` / `--no-restrict-sensitive-apis` | | Toggle sensitive API restrictions. |
| `--lan-only` / `--public` | | Restrict to local network (default) or allow public access. |

## Sources

- [Kimi Code CLI GitHub](https://github.com/MoonshotAI/kimi-cli)
- [Kimi Code CLI Documentation](https://moonshotai.github.io/kimi-cli/en/)
- [Kimi Code CLI LLM-friendly docs](https://moonshotai.github.io/kimi-cli/llms.txt)
- [Kimi Code Reference: kimi command](https://moonshotai.github.io/kimi-cli/en/reference/kimi-command)
- [Kimi Code: Providers and Models](https://moonshotai.github.io/kimi-cli/en/configuration/providers)
- [Kimi Code: Config Files](https://moonshotai.github.io/kimi-cli/en/configuration/config-files)
- [Kimi Code: Environment Variables](https://moonshotai.github.io/kimi-cli/en/configuration/env-vars)
- [Kimi Code: Data Locations](https://moonshotai.github.io/kimi-cli/en/configuration/data-locations)
- [Kimi Code: Print Mode](https://moonshotai.github.io/kimi-cli/en/customization/print-mode)
- [Kimi Code: Wire Mode](https://moonshotai.github.io/kimi-cli/en/customization/wire-mode)
- [Kimi Code: Agents and Subagents](https://moonshotai.github.io/kimi-cli/en/customization/agents)
- [Kimi Code: Agent Skills](https://moonshotai.github.io/kimi-cli/en/customization/skills)
- [Kimi Code Membership Benefits](https://www.kimi.com/code/docs/en/)
- [Kimi Code Benefit Description](https://www.kimi.com/code/docs/en/benefits.html)
- [Moonshot AI Open Platform](https://platform.moonshot.ai/)
