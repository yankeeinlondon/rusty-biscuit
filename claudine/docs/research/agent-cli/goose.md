---
homepage: https://goose-docs.ai/
docs: https://goose-docs.ai/docs/guides/goose-cli-commands/
cli_docs: https://goose-docs.ai/docs/guides/goose-cli-commands/
repo: https://github.com/aaif-goose/goose
schema:
schema_type:
data_format: json / ndjson
latest_version: v1.32.0
last_updated: 2026-04-27
---

# Goose CLI Structured Output in Non-Interactive Sessions

## Summary

Goose currently exposes two machine-readable output modes for `goose run`: batch `json` and streaming `stream-json`. The batch mode returns one JSON object after the run finishes. The streaming mode emits one JSON object per line as the run progresses, so it is functionally NDJSON even though the CLI flag is named `stream-json`.

Goose has moved from `block/goose` to [aaif-goose/goose](https://github.com/aaif-goose/goose) under the Agentic AI Foundation (AAIF) at the Linux Foundation. Documentation moved from `block.github.io/goose/` to [goose-docs.ai](https://goose-docs.ai/). The latest version is v1.32.0 (2026-04-23).

The important Claudine implication is that Goose does **not** publish a standalone JSON Schema or OpenAPI document for this wire format. The closest provider-owned formal specification is the Rust source: `StreamEvent` in `crates/goose-cli/src/session/mod.rs`, plus the reusable `Message`, `MessageContent`, `ActionRequiredData`, and `SystemNotificationContent` types in `crates/goose/src/conversation/message.rs`.

The current source-backed stream envelope is narrower than some older docs and examples imply:

- Current top-level stream events are only `message`, `notification`, `error`, and `complete`.
- I did **not** find a current `model_change` event in Goose's mainline source.
- Top-level stream event names use `snake_case`, but nested `message.content[]` item types and fields use `camelCase`.
- `notification` events flatten their payload into the top level of the event object; they are not nested under `log` or `progress`.
- Many high-value conditions Claudine cares about are exposed only as nested `message.content[]` variants or flattened log strings, not as dedicated top-level event types.

For Claudine, the most robust integration strategy is:

- Treat `stream-json` as the primary live integration surface.
- Parse the outer event envelope first.
- For `message` events, parse the nested Goose `Message` schema exactly.
- Use `complete.total_tokens` as the only stable built-in session token total.
- Expect several important cases, especially permissions, auth failures, and subagent activity, to require inference from tool calls, tool responses, or notification log strings.

## CLI Switch Summary

Source: `crates/goose-cli/src/cli.rs` in [aaif-goose/goose](https://github.com/aaif-goose/goose). Based on v1.32.0.

### `goose run`

Primary non-interactive / headless execution command. Combines all shared argument groups.

| Flag | Short | Type | Default | Description |
|---|---|---|---|---|
| `--text` | `-t` | `String` | — | Input text to provide directly |
| `--instructions` | `-i` | `String` | — | Path to instruction file. Use `-` for stdin |
| `--recipe` | | `String` | — | Recipe name or full path to recipe file |
| `--system` | | `String` | — | Additional system prompt |
| `--params` | | `KEY=VALUE` (repeatable) | — | Dynamic parameters for recipe |
| `--sub-recipe` | | `String` (repeatable) | — | Sub-recipe name or file path |
| `--explain` | | `bool` | `false` | Show recipe title, description, and parameters |
| `--render-recipe` | | `bool` | `false` | Print rendered recipe instead of running it |
| `--name` | `-n` | `String` | — | Name for the session |
| `--session-id` | (alias `--id`) | `String` | — | Session ID (e.g. `20250921_143022`) |
| `--path` | | `PathBuf` | — | Legacy: path for session storage |
| `--interactive` | `-s` | `bool` | `false` | Continue in interactive mode after processing input |
| `--no-session` | | `bool` | `false` | Run without storing a session file |
| `--resume` | `-r` | `bool` | `false` | Resume from a previous run |
| `--debug` | | `bool` | `false` | Enable debug output (full content, no truncation) |
| `--max-tool-repetitions` | | `u32` | — | Max consecutive identical tool calls |
| `--max-turns` | | `u32` | `1000` | Max turns without user input |
| `--container` | | `String` | — | Docker container ID for extension isolation |
| `--with-extension` | | `String` (repeatable) | — | Add stdio extensions |
| `--with-streamable-http-extension` | | `String` (repeatable) | — | Add Streamable HTTP extensions |
| `--with-builtin` | | `String` (comma-delimited) | — | Add builtin extensions by name |
| `--no-profile` | | `bool` | `false` | Don't load default extensions |
| `--quiet` | `-q` | `bool` | `false` | Suppress non-response output |
| `--output-format` | | `String` | `text` | Output format: `text`, `json`, `stream-json` |
| `--provider` | | `String` | — | Override GOOSE_PROVIDER (e.g. `openai`, `anthropic`) |
| `--model` | | `String` | — | Override GOOSE_MODEL (e.g. `gpt-4o`) |

Examples:

```bash
goose run -t "summarize this repo" --output-format json
goose run -t "summarize this repo" --output-format stream-json
goose run --recipe my-recipe.yaml --params env=production --params region=us-west-2
goose run --provider anthropic --model claude-4-sonnet -t "initial prompt"
goose run -i instructions.txt --no-session --output-format stream-json
goose run -t "fix the tests" --max-turns 10 --interactive
goose run --with-extension "npx -y @modelcontextprotocol/server-memory" -t "remember my coding style"
goose run --with-builtin developer,computercontroller -t "analyze the repo"
```

### `goose session` (alias: `goose s`)

Start or resume interactive sessions.

| Flag | Short | Type | Default | Description |
|---|---|---|---|---|
| `--name` | `-n` | `String` | — | Name for the session |
| `--session-id` | | `String` | — | Session ID |
| `--path` | | `PathBuf` | — | Legacy: path for session |
| `--resume` | `-r` | `bool` | `false` | Resume a previous session |
| `--fork` | | `bool` | `false` | Fork session (requires `--resume`) |
| `--history` | | `bool` | `false` | Show previous messages on resume |
| `--debug` | | `bool` | `false` | Enable debug output |
| `--max-tool-repetitions` | | `u32` | — | Max consecutive identical tool calls |
| `--max-turns` | | `u32` | `1000` | Max turns without user input |
| `--container` | | `String` | — | Docker container ID |
| `--with-extension` | | `String` (repeatable) | — | Add stdio extensions |
| `--with-streamable-http-extension` | | `String` (repeatable) | — | Add Streamable HTTP extensions |
| `--with-builtin` | | `String` | — | Add builtin extensions |
| `--no-profile` | | `bool` | `false` | Don't load default extensions |

Examples:

```bash
goose session -n my-project
goose session --resume -n my-project
goose session --resume --fork --history
goose session --with-extension "npx -y @modelcontextprotocol/server-memory"
goose session --with-builtin developer --debug --max-turns 25
```

### `goose session list`

| Flag | Short | Type | Default | Description |
|---|---|---|---|---|
| `--format` | `-f` | `String` | `text` | Output format: `text`, `json` |
| `--ascending` | | `bool` | `false` | Sort oldest first |
| `--working_dir` | `-w` (alias `-p`) | `PathBuf` | — | Filter by working directory |
| `--limit` | `-l` | `usize` | — | Limit number of results |

### `goose session remove`

| Flag | Short | Type | Default | Description |
|---|---|---|---|---|
| `--session-id` | | `String` | — | Remove by session ID |
| `--name` | `-n` | `String` | — | Remove by name |
| `--regex` | `-r` | `String` | — | Remove sessions matching regex |
| `--path` | | `PathBuf` | — | Remove by path (legacy) |

### `goose session export`

| Flag | Short | Type | Default | Description |
|---|---|---|---|---|
| `--session-id` | | `String` | — | Export by session ID |
| `--name` | `-n` | `String` | — | Export by name |
| `--path` | | `PathBuf` | — | Export by path (legacy) |
| `--output` | `-o` | `PathBuf` | stdout | Output file path |
| `--format` | | `String` | `markdown` | Output format: `markdown`, `json`, `yaml` |

### `goose session diagnostics`

| Flag | Short | Type | Default | Description |
|---|---|---|---|---|
| `--session-id` | | `String` | — | Diagnostics by session ID |
| `--name` | `-n` | `String` | — | Diagnostics by name |
| `--path` | | `PathBuf` | — | Diagnostics by path (legacy) |
| `--output` | `-o` | `PathBuf` | `diagnostics_{id}.zip` | Output path for diagnostics zip |

### `goose configure`

Interactive configuration. No flags.

### `goose info`

| Flag | Short | Type | Default | Description |
|---|---|---|---|---|
| `--verbose` | `-v` | `bool` | `false` | Show config.yaml, env vars, enabled extensions |
| `--check` | | `bool` | `false` | Test provider connection and show status |

### `goose update`

| Flag | Short | Type | Default | Description |
|---|---|---|---|---|
| `--canary` | `-c` | `bool` | `false` | Update to canary version |
| `--reconfigure` | | `bool` | `false` | Force re-configure during update |

### `goose completion <SHELL>`

| Arg | Type | Description |
|---|---|---|
| `<SHELL>` | enum: `bash`, `zsh`, `fish`, `powershell`, `elvish` | Shell to generate completions for |
| `--bin-name` | `String` (default: `goose`) | Custom binary name |

### `goose recipe`

| Subcommand | Flags |
|---|---|
| `validate <RECIPE>` | positional recipe name/path |
| `deeplink <RECIPE>` | `-p, --param KEY=VALUE` (repeatable) |
| `open <RECIPE>` | `-p, --param KEY=VALUE` (repeatable) |
| `list` | `--format text\|json` (default `text`), `-v, --verbose` |

### `goose schedule` (alias: `goose sched`)

| Subcommand | Flags |
|---|---|
| `add` | `--schedule-id` (required), `--cron` (required), `--recipe-source` (required) |
| `list` | no flags |
| `remove` | `--schedule-id` (required) |
| `sessions` | `--schedule-id` (required), `-l, --limit` |
| `run-now` | `--schedule-id` (required) |
| `services-status` | no flags (deprecated) |
| `services-stop` | no flags (deprecated) |
| `cron-help` | no flags |

### `goose serve`

| Flag | Type | Default | Description |
|---|---|---|---|
| `--host` | `String` | `127.0.0.1` | Host to bind |
| `--port` | `u16` | `3284` | Port to bind |
| `--with-builtin` | `String` (repeatable) | — | Add builtin extensions |

### `goose acp`

Run goose as an ACP agent server over stdio.

| Flag | Type | Default | Description |
|---|---|---|---|
| `--with-builtin` | `String` | — | Add builtin extensions |

### `goose gateway` (alias: `goose gw`)

| Subcommand | Flags |
|---|---|
| `status` | no flags |
| `start <TYPE>` | `--bot-token` (required) |
| `stop <TYPE>` | positional gateway type |
| `pair <TYPE>` | positional gateway type |

### `goose mcp <SERVER>`

Run a bundled MCP server. Positional arg: server name (e.g. `auto-visualiser`, `computer-controller`, `memory`, `tutorial`).

### `goose project` (alias `goose p`)

Open last project. No flags.

### `goose projects` (alias `goose ps`)

List recent projects. No flags.

### `goose term`

| Subcommand | Flags |
|---|---|
| `init <SHELL>` | `-n, --name`, `--default` |
| `run <PROMPT...>` | positional prompt words |
| `info` | no flags |
| `log <COMMAND>` | positional command (hidden) |

### `goose local-models` (alias `goose lm`, feature-gated: `local-inference`)

| Subcommand | Flags |
|---|---|
| `search <QUERY>` | `-l, --limit` (default `10`) |
| `download <SPEC>` | positional `user/repo:quantization` |
| `list` | no flags |
| `delete <ID>` | positional model ID |

### `goose bench`

Evaluate system configuration across practical tasks. See [benchmarking tutorial](https://goose-docs.ai/docs/tutorials/benchmarking).

### Interactive Session Slash Commands

These are in-session commands (not subcommands):

| Command | Description |
|---|---|
| `/help`, `/?` | Display help menu |
| `/exit`, `/quit` | Exit session |
| `/builtin <names>` | Add builtin extensions (comma-separated) |
| `/extension <command>` | Add stdio extension |
| `/clear` | Clear chat history |
| `/compact` | Summarize conversation to reduce context |
| `/mode <name>` | Set goose mode (`auto`, `approve`, `chat`, `smart_approve`) |
| `/plan [text]` | Enter plan mode |
| `/endplan` | Exit plan mode |
| `/prompts [--extension <name>]` | List available prompts |
| `/prompt <n> [--info] [key=value...]` | Get prompt info or execute prompt |
| `/recipe [filepath]` | Generate recipe from conversation |
| `/skills` | List available skills |
| `/load-skill <names>` | Load skills by name |
| `/edit [prefill]` | Open external editor for prompt |
| `/r` | Toggle full tool output display |
| `/t` | Cycle theme (light/dark/ansi) |
| `/t <name>` | Set theme directly |

## Schema

### What Goose publishes today

I did **not** find a standalone published schema artifact for Goose's non-interactive structured output:

- No JSON Schema
- No OpenAPI document for `goose run --output-format json`
- No OpenAPI / AsyncAPI / JSON Schema for `goose run --output-format stream-json`
- No published TypeScript type definition for the CLI wire format

I checked:

- Official docs: [CLI Commands](https://goose-docs.ai/docs/guides/goose-cli-commands/), [Running Tasks](https://goose-docs.ai/docs/guides/running-tasks/), [Using goose in Headless Mode for Automation](https://goose-docs.ai/docs/tutorials/headless-goose/), [MCP Elicitation](https://goose-docs.ai/docs/guides/mcp-elicitation/), [Customizing Prompt Templates](https://goose-docs.ai/docs/guides/prompt-templates/)
- Current Goose source on GitHub
- Release notes on GitHub
- Broader searches for third-party formalizations, including searches targeted at Vercel AI SDK and LangChain repositories

I did not find a respected third-party project that formalizes Goose's CLI output schema either.

### Closest thing to a formal schema

The closest provider-owned formal definitions are Rust `serde` types in Goose's source:

- Top-level streaming envelope: [`StreamEvent`](https://raw.githubusercontent.com/aaif-goose/goose/main/crates/goose-cli/src/session/mod.rs)
- Batch output object: same file, `JsonOutput` and `JsonMetadata`
- Shared message schema: [`Message`, `MessageMetadata`, `MessageContent`, `ActionRequiredData`, `SystemNotificationContent`, `SystemNotificationType`](https://raw.githubusercontent.com/aaif-goose/goose/main/crates/goose/src/conversation/message.rs)
- Tool request / response serialization rules: [`tool_result_serde.rs`](https://raw.githubusercontent.com/aaif-goose/goose/main/crates/goose/src/conversation/tool_result_serde.rs)
- Developer shell tool output schema: [`ShellOutput`](https://raw.githubusercontent.com/aaif-goose/goose/main/crates/goose/src/agents/platform_extensions/developer/shell.rs)

Schema language used by those source definitions:

- Rust structs/enums with `serde` serialization attributes
- Some reusable subtypes also derive `ToSchema` / `JsonSchema`, but Goose does not publish a generated schema document for the CLI wire format itself

### Current stream schema

Top-level `stream-json` events are currently defined as:

```json
{"type":"message","message":{...}}
{"type":"notification","extension_id":"...","message":"..."}
{"type":"notification","extension_id":"...","progress":0.5,"total":1.0,"message":"..."}
{"type":"error","error":"..."}
{"type":"complete","total_tokens":1234}
```

Important details from current source:

- Outer `type` values are `snake_case`
- `notification` uses `#[serde(flatten)]`, so log/progress payloads are flattened into the event object
- `complete.total_tokens` is optional

### Current batch JSON schema

Batch `json` mode currently returns one object:

```json
{
  "messages": [
    {
      "id": null,
      "role": "assistant",
      "created": 1743999999,
      "content": [],
      "metadata": {
        "userVisible": true,
        "agentVisible": true
      }
    }
  ],
  "metadata": {
    "total_tokens": 1234,
    "status": "completed"
  }
}
```

### Nested `Message` / `MessageContent` gotcha

The nested message schema uses `camelCase`, not `snake_case`.

Examples:

- `type: "toolRequest"`
- `type: "toolResponse"`
- `type: "actionRequired"`
- `type: "systemNotification"`
- `actionType: "toolConfirmation"`
- `actionType: "elicitation"`
- `notificationType: "creditsExhausted"`
- `toolCall`, not `tool_call`
- `toolResult`, not `tool_result`
- `userVisible` / `agentVisible`, not `user_visible` / `agent_visible`

That casing split between the outer envelope and inner message schema is easy to get wrong.

### Tool request / response serialization

`toolCall` and `toolResult` are serialized with explicit status wrappers:

```json
{
  "type": "toolRequest",
  "id": "call_123",
  "toolCall": {
    "status": "success",
    "value": {
      "name": "shell",
      "arguments": {
        "command": "pwd"
      }
    }
  }
}
```

```json
{
  "type": "toolResponse",
  "id": "call_123",
  "toolResult": {
    "status": "success",
    "value": {
      "content": [
        {
          "type": "text",
          "text": "..."
        }
      ]
    }
  }
}
```

Or, on error:

```json
{
  "type": "toolResponse",
  "id": "call_123",
  "toolResult": {
    "status": "error",
    "error": "Failed to write /path/file.txt: Permission denied (os error 13)"
  }
}
```

## Documentation

### Official documentation

The most relevant official docs for non-interactive structured output are:

- [CLI Commands](https://goose-docs.ai/docs/guides/goose-cli-commands/)
  - Documents `goose run`, `goose session`, `goose recipe`, `goose schedule`, `goose serve`, `goose acp`, `goose mcp`, `goose configure`, and more
  - Documents `--output-format text|json|stream-json`
  - Explicitly says `json` is for results after completion and `stream-json` is for events as they occur
- [Run Tasks](https://goose-docs.ai/docs/guides/running-tasks/)
  - Main official non-interactive task execution guide
- [Using goose in Headless Mode for Automation](https://goose-docs.ai/docs/tutorials/headless-goose/)
  - Best official explanation of non-interactive behavior, limitations, and automation gotchas
- [MCP Elicitation](https://goose-docs.ai/docs/guides/mcp-elicitation/)
  - Important for understanding structured human-in-the-loop requests
- [goose Permission Modes](https://goose-docs.ai/docs/guides/goose-permissions/)
  - Four modes: Auto (default), Approve, Smart Approve, Chat
  - Important for understanding when tool confirmation requests can happen
- [Customizing Prompt Templates](https://goose-docs.ai/docs/guides/prompt-templates/)
  - Important for subagent prompt injection and customization
- [ACP Providers](https://goose-docs.ai/docs/guides/acp-providers/)
  - ACP adapters for Claude Code, Codex, Amp, and Pi as providers
- [Subagents](https://goose-docs.ai/docs/guides/subagents/)
  - Internal and external subagent lifecycle, configuration, and security constraints

Official built-in extension docs that matter when interpreting tool events:

- [Developer Extension](https://goose-docs.ai/docs/mcp/developer-mcp/)
- [Computer Controller Extension](https://goose-docs.ai/docs/mcp/computer-controller-mcp/)
- [Memory Extension](https://goose-docs.ai/docs/mcp/memory-mcp/)
- [Tutorial Extension](https://goose-docs.ai/docs/mcp/tutorial-mcp/)
- [Auto Visualiser Extension](https://goose-docs.ai/docs/mcp/autovisualiser-mcp/)

### Release notes and changelog-style docs

These are important because Goose's current stream behavior is clarified more by source and releases than by one dedicated schema doc:

- [v1.25.0 release](https://github.com/aaif-goose/goose/releases/tag/v1.25.0)
- [v1.26.0 release](https://github.com/aaif-goose/goose/releases/tag/v1.26.0)
- [v1.27.0 release](https://github.com/aaif-goose/goose/releases/tag/v1.27.0)
- [v1.29.0 release](https://github.com/aaif-goose/goose/releases/tag/v1.29.0)
- [v1.30.0 release](https://github.com/aaif-goose/goose/releases/tag/v1.30.0)
- [v1.31.0 release](https://github.com/aaif-goose/goose/releases/tag/v1.31.0)
- [v1.32.0 release](https://github.com/aaif-goose/goose/releases/tag/v1.32.0)
- [v1.17.0 release](https://github.com/aaif-goose/goose/releases/tag/v1.17.0)
- [v1.0.30 release](https://github.com/aaif-goose/goose/releases/tag/v1.0.30)

### Articles and tutorials worth reading

These are the highest-signal public writeups I found that help explain or operationalize Goose's structured output surface:

- [Using goose in Headless Mode for Automation](https://goose-docs.ai/docs/tutorials/headless-goose/)
  - Best official prose guide for non-interactive usage
- [goose v1.25.0 is here](https://github.com/aaif-goose/goose/releases/tag/v1.25.0)
  - Explicitly mentions `stream-json` in provider integration work
- [goose v1.27.0 is here](https://github.com/aaif-goose/goose/releases/tag/v1.27.0)
  - Important because it adds a structured output schema for the Developer extension's `shell` tool
- [goose v1.32.0](https://github.com/aaif-goose/goose/releases/tag/v1.32.0)
  - ACP providers, skills system, terminal integration, gateway, local-models, auto-compaction, and many bug fixes

### Documentation quality assessment

Goose's documentation is good at explaining:

- How to invoke structured output
- What output modes exist
- How headless mode behaves operationally
- How elicitation and permissions behave conceptually

It is weaker on:

- Publishing a canonical machine-readable wire schema
- Explaining exact serialized field names
- Explaining the mixed `snake_case` / `camelCase` split
- Clarifying what is flattened vs nested in `notification` events
- Keeping examples aligned with the latest source in every case

## CLI

### Available output formats

Goose currently accepts:

| CLI value | Meaning | Best use |
|---|---|---|
| `text` | Human-oriented terminal rendering | Manual use |
| `json` | One final JSON document after completion | Batch automation where progress is not needed |
| `stream-json` | One JSON object per line as the run executes | Live orchestration, telemetry, wrappers, Claudine |

### Syntax

The switch is:

```bash
goose run --output-format <text|json|stream-json> ...
```

Examples:

```bash
goose run -t "summarize this repo" --output-format json
```

```bash
goose run -t "summarize this repo" --output-format stream-json
```

Structured output can be combined with the usual non-interactive input methods:

- `-t, --text`
- `-i, --instructions <FILE>`
- `-i -` for stdin
- `--recipe ...`
- `--system ...`
- `--params KEY=VALUE`
- `--sub-recipe ...`
- `--provider ...` and `--model ...` to override provider/model
- `--with-extension`, `--with-streamable-http-extension`, `--with-builtin` to add extensions
- `--quiet` to suppress non-response output
- `--interactive` to drop into a session after processing

### Behavioral differences by format

`text`

- Renders human-friendly markdown and tool output
- Not a stable machine contract

`json`

- Suppresses incremental progress rendering
- Emits a single final JSON object
- Includes the full stored conversation messages plus `metadata.total_tokens`

`stream-json`

- Emits line-delimited JSON events during execution
- Emits `error` as JSON rather than plain stderr text
- Ends with a `complete` event
- Is the only mode suitable for real-time wrappers

### Known side effects of switching formats

- `stream-json` does **not** give you one valid JSON document. It gives you one JSON object per line.
- `json` gives you the final conversation state, not a normalized event history.
- `notification` payloads are flattened in `stream-json`.
- Rich MCP/subagent payloads are sometimes reduced to formatted strings before they hit the stream.
- Current stream output does not include provider/model identity.

## Gotchas

### 1. There is no published canonical JSON Schema

If Claudine wants strict validation, it must derive it from Goose source or maintain its own adapter schema.

Sources:

- [CLI Commands](https://goose-docs.ai/docs/guides/goose-cli-commands/)
- [`session/mod.rs`](https://raw.githubusercontent.com/aaif-goose/goose/main/crates/goose-cli/src/session/mod.rs)

### 2. Outer events are `snake_case`; nested messages are `camelCase`

This is a real wire-format sharp edge:

- Outer event: `{"type":"complete"}`
- Nested content: `{"type":"toolRequest"}`
- Nested action discriminator: `actionType`
- Nested system notification discriminator: `notificationType`

Sources:

- [`session/mod.rs`](https://raw.githubusercontent.com/aaif-goose/goose/main/crates/goose-cli/src/session/mod.rs)
- [`message.rs`](https://raw.githubusercontent.com/aaif-goose/goose/main/crates/goose/src/conversation/message.rs)

### 3. `notification` events flatten their payload

Current source emits:

```json
{"type":"notification","extension_id":"developer","message":"..."}
```

and

```json
{"type":"notification","extension_id":"developer","progress":0.5,"total":1.0,"message":"..."}
```

not:

```json
{"type":"notification","log":{"message":"..."}}
```

This matters if Claudine already has code written against older examples.

Source:

- [`session/mod.rs`](https://raw.githubusercontent.com/aaif-goose/goose/main/crates/goose-cli/src/session/mod.rs)

### 4. Current source does not expose `model_change`

Older Goose examples and some secondary research mention a `model_change` event. I did **not** find it in the current `StreamEvent` enum on mainline Goose as of 2026-04-27.

Claudine should therefore treat model/provider detection as out-of-band unless pinned to an older Goose build that still emitted such an event.

Source:

- [`session/mod.rs`](https://raw.githubusercontent.com/aaif-goose/goose/main/crates/goose-cli/src/session/mod.rs)

### 5. Headless mode cannot answer questions

Official headless docs are clear: non-interactive execution cannot provide clarification or approval during the run. That means `actionRequired` content is a blocker or failure mode for automation, not just informational metadata.

Sources:

- [Using goose in Headless Mode for Automation](https://goose-docs.ai/docs/tutorials/headless-goose/)
- [MCP Elicitation](https://goose-docs.ai/docs/guides/mcp-elicitation/)
- [goose Permission Modes](https://goose-docs.ai/docs/guides/goose-permissions/)

### 6. Subagent and MCP notifications are sometimes downgraded to strings

Current source formats several MCP logging notifications into strings before printing them to `stream-json`. Subagent tool requests get a formatted log line, not the original structured payload. Task execution notifications can also be flattened into prose-like log messages.

This is the single biggest information-loss problem in Goose's live stream for Claudine.

Source:

- [`handle_mcp_notification` and `format_logging_notification`](https://raw.githubusercontent.com/aaif-goose/goose/main/crates/goose-cli/src/session/mod.rs)

### 7. Official tool docs lag the current Developer extension surface

The current Developer extension source exposes `write`, `edit`, `shell`, and `tree`. Some official docs still describe an older tool surface such as `text_editor`, `analyze`, `screen_capture`, and `image_processor`.

For Claudine's parser, source should be treated as more authoritative than prose docs when they disagree.

Sources:

- [Developer Extension docs](https://goose-docs.ai/docs/mcp/developer-mcp/)
- [`developer/mod.rs`](https://raw.githubusercontent.com/aaif-goose/goose/main/crates/goose/src/agents/platform_extensions/developer/mod.rs)

## Timeline

This timeline focuses on structured output and adjacent features that materially affect what Claudine can observe.

| Date | Version | Event | Why it matters |
|---|---|---|---|
| 2025-06-27 | [v1.0.30](https://github.com/aaif-goose/goose/releases/tag/v1.0.30) | Subagents shipped | Subagent activity later becomes something stream consumers want to observe |
| 2025-12-18 | [v1.17.0](https://github.com/aaif-goose/goose/releases/tag/v1.17.0) | MCP Elicitation support shipped | Introduces structured user-input requests that appear as `actionRequired` content |
| 2026-02-18 | [v1.25.0](https://github.com/aaif-goose/goose/releases/tag/v1.25.0) | Release notes explicitly mention `stream-json` in provider integration work, including Gemini CLI and isolated `session_id` handling | Confirms `stream-json` is an important internal and external integration surface by this point |
| 2026-02-26 | [v1.26.0](https://github.com/aaif-goose/goose/releases/tag/v1.26.0) | Release notes mention low-balance detection, stream subagent tool call docs, and stream-json event work | Important for Claudine's caps/funds/subagent monitoring story |
| 2026-03-05 | [v1.27.0](https://github.com/aaif-goose/goose/releases/tag/v1.27.0) | Developer `shell` tool gains structured `{stdout, stderr}` output schema | Significant improvement in tool-response structure |
| 2026-04-07 | — | Goose moves from Block to AAIF at the Linux Foundation | Repo moves to `aaif-goose/goose`, docs move to `goose-docs.ai` |
| 2026-04-23 | [v1.32.0](https://github.com/aaif-goose/goose/releases/tag/v1.32.0) | ACP providers (Claude Code, Codex, Amp, Pi), skills system, `/skills` command, auto-compaction, gateway, local-models, terminal integration, `/edit` command, Novita AI provider, Kimi Code provider with OAuth | Major expansion of CLI surface and provider ecosystem |

Timeline note:

- I did **not** find a release note that cleanly identifies the original introduction date of `--output-format json` or `--output-format stream-json`.
- The earliest explicit release-note mentions I found were in the February 2026 release stream above.

## Tools

### Built-in extensions shipped with Goose

Current bundled built-ins from Goose's desktop extension manifest are:

| Extension | Bundled | Enabled by default | Current tool surface |
|---|---|---:|---|
| `developer` | yes | yes | `write`, `edit`, `shell`, `tree` |
| `computercontroller` | yes | no | `web_scrape`, `automation_script`, `computer_control`, `xlsx_tool`, `docx_tool`, `pdf_tool`, `cache` |
| `autovisualiser` | yes | no | `render_sankey`, `render_radar`, `render_donut`, `render_treemap`, `render_chord`, `render_map`, `render_mermaid`, `show_chart` |
| `memory` | yes | no | `remember_memory`, `retrieve_memories`, `remove_memory_category`, `remove_specific_memory` |
| `tutorial` | yes | no | `load_tutorial` |

Sources:

- [`bundled-extensions.json`](https://raw.githubusercontent.com/aaif-goose/goose/main/ui/desktop/src/components/settings/extensions/bundled-extensions.json)
- [`developer/mod.rs`](https://raw.githubusercontent.com/aaif-goose/goose/main/crates/goose/src/agents/platform_extensions/developer/mod.rs)
- [`computercontroller/mod.rs`](https://raw.githubusercontent.com/aaif-goose/goose/main/crates/goose-mcp/src/computercontroller/mod.rs)
- [`autovisualiser/mod.rs`](https://raw.githubusercontent.com/aaif-goose/goose/main/crates/goose-mcp/src/autovisualiser/mod.rs)
- [`memory/mod.rs`](https://raw.githubusercontent.com/aaif-goose/goose/main/crates/goose-mcp/src/memory/mod.rs)
- [`tutorial/mod.rs`](https://raw.githubusercontent.com/aaif-goose/goose/main/crates/goose-mcp/src/tutorial/mod.rs)

### What the stream exposes before and after tool calls

For normal tool usage, Goose exposes tool activity inside `message` events:

1. Assistant emits a `message` whose `content[]` contains one or more `toolRequest` items
2. Later, Goose emits another `message` whose `content[]` contains matching `toolResponse` items
3. The `id` field ties the request and response together

This means the stream usually exposes:

- Tool name
- Tool arguments
- Tool success vs error
- Tool output, usually inside `toolResult.value.content[]`
- Any provider metadata attached to the tool message

### What the stream exposes around MCP notifications

In addition to `toolRequest` / `toolResponse`, Goose emits `notification` events for MCP logs and progress.

For those, the stream can expose:

- `extension_id`
- `message` for log-style notifications
- `progress`, `total`, and optional `message` for progress notifications

But the current CLI stream does **not** preserve every original MCP payload in full fidelity. Several notification types are converted into strings first.

### Example: Developer `shell`

Possible request:

```json
{
  "type": "message",
  "message": {
    "role": "assistant",
    "content": [
      {
        "type": "toolRequest",
        "id": "call_1",
        "toolCall": {
          "status": "success",
          "value": {
            "name": "shell",
            "arguments": {
              "command": "cargo test -p claudine"
            }
          }
        }
      }
    ]
  }
}
```

Possible response:

```json
{
  "type": "message",
  "message": {
    "role": "user",
    "content": [
      {
        "type": "toolResponse",
        "id": "call_1",
        "toolResult": {
          "status": "success",
          "value": {
            "content": [
              {
                "type": "text",
                "text": "{\"stdout\":\"...\",\"stderr\":\"\",\"exit_code\":0}"
              }
            ]
          }
        }
      }
    ]
  }
}
```

Important note:

- The `shell` tool now has a structured output schema in source, but the message stream still carries that result inside Goose's generic tool-response container.
- Consumers usually still need to parse the returned content payload.

### Example: Memory tool

Request:

```json
{
  "type": "toolRequest",
  "id": "call_mem_1",
  "toolCall": {
    "status": "success",
    "value": {
      "name": "remember_memory",
      "arguments": {
        "category": "coding_style",
        "data": "Prefer ripgrep over grep",
        "tags": ["cli", "search"],
        "is_global": true
      }
    }
  }
}
```

Response:

```json
{
  "type": "toolResponse",
  "id": "call_mem_1",
  "toolResult": {
    "status": "success",
    "value": {
      "content": [
        {
          "type": "text",
          "text": "Memory stored successfully"
        }
      ]
    }
  }
}
```

### Example: Progress notification

```json
{
  "type": "notification",
  "extension_id": "computercontroller",
  "progress": 0.5,
  "total": 1.0,
  "message": "Processing files..."
}
```

### Claudine-specific tool observations

- Tool correlation should key on `toolRequest.id` and `toolResponse.id`
- `notification.extension_id` is useful, but often not enough by itself
- For subagents, a lot of the most useful metadata is currently reduced to log strings
- The best live observability still comes from combining:
  - top-level `notification`
  - nested `toolRequest`
  - nested `toolResponse`
  - final `complete.total_tokens`

## Use Cases

### Plan Cap Approaching

I did **not** find a Goose-native dedicated structured event for "you are approaching your plan cap".

What I found:

- No top-level `stream-json` event dedicated to plan-cap warnings
- No dedicated `MessageContent` variant for near-cap usage warnings
- Official rate-limit / automation docs do not document a structured "approaching cap" signal

Implications:

- If a provider surfaces a near-cap warning, Goose may expose it only as:
  - assistant text inside a `message`
  - a flattened `notification.message`
  - or a generic `error.error`
- I found no stable structured field for:
  - percentage remaining
  - token amount remaining
  - reset time / reset window

Distinguishing signal:

- None that is Goose-normalized today
- Claudine would need provider-specific text matching

Hook parity:

- No
- Goose's only true hook is `GOOSE_STATUS_HOOK` with `thinking` / `waiting`; it does not expose cap metadata

### Plan Capped

Goose does not appear to normalize generic provider plan caps into a dedicated top-level stream event.

However, Goose **does** have a normalized `creditsExhausted` system notification, which is closer to "billing exhausted / no funds" than to "subscription plan window capped".

What I found:

- No dedicated "plan capped" outer event
- No dedicated "plan capped" `MessageContent` discriminator
- One dedicated system notification for exhausted credits:
  - `type: "systemNotification"`
  - `notificationType: "creditsExhausted"`
  - optional `data.top_up_url`

What Claudine should do:

- Treat `creditsExhausted` as a strong "cannot continue for billing reasons" signal
- Treat actual provider hard-cap / quota-window messages as text-pattern inference unless Goose adds a dedicated normalized event later

Remaining amount / reset window:

- Not exposed

Hook parity:

- No

### No Funds

This is the strongest current structured detection story Goose has for billing exhaustion.

Primary signal:

- `message` event
- nested `message.content[]`
- item with:
  - `type: "systemNotification"`
  - `notificationType: "creditsExhausted"`
  - `msg`
  - optional `data.top_up_url`

How to distinguish it:

- It is stronger than a generic error because Goose gives it its own typed system notification
- The optional `top_up_url` is especially useful operationally

What you can extract:

- Yes: a user-facing message
- Sometimes: a top-up URL
- No: remaining balance
- No: reset window

Hook parity:

- No

Sources:

- [`message.rs`](https://raw.githubusercontent.com/aaif-goose/goose/main/crates/goose/src/conversation/message.rs)
- [`output.rs`](https://raw.githubusercontent.com/aaif-goose/goose/main/crates/goose-cli/src/session/output.rs)
- [v1.26.0 release](https://github.com/aaif-goose/goose/releases/tag/v1.26.0) mentions low-balance detection work

### Auth

I did **not** find a structured event that tells Claudine which auth mode the user is using, such as:

- API key
- subscription
- OAuth
- other device/browser flow

What Goose exposes today:

- Auth failures can surface as generic `error` events or message text
- Some provider setup flows support auto-detection of provider from API key during onboarding, but that is setup behavior, not run-stream metadata
- OAuth cleanup and provider auth improvements show up in release notes, not in the run stream

What Claudine can reliably know from the stream:

- Usually only that something auth-related failed, if the error text says so
- Not the auth kind

Hook parity:

- No

### Permissions: Can't Read File

This is weaker than write detection.

Important current-source caveat:

- The current built-in Developer extension does **not** expose a first-class `read` tool
- The current source tool surface is `write`, `edit`, `shell`, `tree`

So read failures usually happen indirectly via:

- `shell` commands like `cat`, `sed`, `rg`
- `tree`
- extension-specific tools such as `pdf_tool`, `docx_tool`, or `xlsx_tool`

Detection story:

- There is no Goose-native dedicated "read permission denied" event
- Claudine must infer it from:
  - the tool being called
  - the tool arguments
  - and the resulting error text

Can the full path be identified?

- Sometimes
- Usually from:
  - tool arguments
  - echoed path in the error text

Is a reason available?

- Sometimes
- Usually as plain text, for example an OS-level permission denied string

How to distinguish it:

- There is no single normalized event type
- You distinguish it by matching a read-like tool call with a tool-response error message

Hook parity:

- No

### Permissions: Can't Write File

This is more detectable than read failure because current-source Developer tools include explicit write/edit tools.

Primary signals:

1. `message.content[]` item with `type: "toolRequest"` where:
   - tool name is `write` or `edit`
   - arguments include the target path
2. Matching later `toolResponse` with the same `id`
3. `toolResult.status == "error"` and error text indicating failure

Current-source write/edit error strings include the path:

- `Failed to write <path>: <reason>`
- `Failed to read <path>: <reason>` for edit pre-read failures

Can the full path be identified?

- Usually yes
- Best source is tool arguments
- Fallback is the echoed path in the error string

Is a reason available?

- Yes, but as plain text, not a structured enum

How to distinguish it:

- Tool name is `write` / `edit`
- Error text contains a write/read failure with the path and OS/library reason

Hook parity:

- No

Sources:

- [`developer/mod.rs`](https://raw.githubusercontent.com/aaif-goose/goose/main/crates/goose/src/agents/platform_extensions/developer/mod.rs)
- [`edit.rs`](https://raw.githubusercontent.com/aaif-goose/goose/main/crates/goose/src/agents/platform_extensions/developer/edit.rs)

### Tokens Consumed

Goose exposes only an overall session token total in the structured non-interactive output.

Signals:

- `stream-json`: final `complete.total_tokens`
- `json`: final `metadata.total_tokens`

What Goose does **not** expose in the run output:

- per-turn token counts
- per-message token counts
- per-tool token counts
- price / currency / cost basis

Hook parity:

- No

Sources:

- [`session/mod.rs`](https://raw.githubusercontent.com/aaif-goose/goose/main/crates/goose-cli/src/session/mod.rs)

### Model Used

I did **not** find a current stable structured stream field for provider/model identity.

Important current-source finding:

- The current `StreamEvent` enum does not include `model_change`
- Batch `json` output also does not include provider/model metadata

That means:

- Claudine cannot rely on the stream alone to know which model Goose used
- Model/provider usually needs to be inferred from:
  - command-line context
  - environment/config
  - or separate Goose logs/config files

Do such events always fire?

- I found no current-source event to rely on

Nomenclature:

- Not available from the run stream today

Hook parity:

- No

### Human in the Loop

#### Main session

Yes. Goose exposes two structured main-session human-in-the-loop cases inside `message.content[]`.

1. Tool confirmation

```json
{
  "type": "actionRequired",
  "data": {
    "actionType": "toolConfirmation",
    "id": "...",
    "toolName": "...",
    "arguments": {},
    "prompt": "..."
  }
}
```

2. Elicitation

```json
{
  "type": "actionRequired",
  "data": {
    "actionType": "elicitation",
    "id": "...",
    "message": "...",
    "requestedSchema": {}
  }
}
```

And when a response is provided, Goose can emit:

```json
{
  "type": "actionRequired",
  "data": {
    "actionType": "elicitationResponse",
    "id": "...",
    "userData": {}
  }
}
```

This is a strong integration point for Claudine because:

- the question/prompt is structured
- the requested schema is structured
- tool confirmation includes the tool name and arguments

#### Subagents

I did **not** find equivalent reliable structured forwarding for subagent human-in-the-loop prompts into the parent stream.

Current source strongly suggests:

- parent stream receives subagent-related `notification` events
- several notification types are flattened into log strings
- raw child `actionRequired` payloads are not preserved as first-class stream events in the parent

So:

- Main session: yes, structured
- Subagent: not reliably exposed in the parent stream as structured data

Hook parity:

- No
- These are stream/message constructs, not status-hook events

Sources:

- [`message.rs`](https://raw.githubusercontent.com/aaif-goose/goose/main/crates/goose/src/conversation/message.rs)
- [`session/mod.rs`](https://raw.githubusercontent.com/aaif-goose/goose/main/crates/goose-cli/src/session/mod.rs)
- [MCP Elicitation](https://goose-docs.ai/docs/guides/mcp-elicitation/)
- [Headless Mode](https://goose-docs.ai/docs/tutorials/headless-goose/)

### Injecting into Subagent Prompt

Yes, but indirectly rather than through one dedicated "subagent prompt injection" CLI flag.

What Goose gives you:

- Global / user-overridden prompt templates, including:
  - [`subagent_system.md`](https://github.com/aaif-goose/goose/blob/main/crates/goose/src/prompts/subagent_system.md)
- Additional run-scoped system instructions via `goose run --system`
- Project / user hints via `.goosehints` and `AGENTS.md`
- Recipe and sub-recipe instructions

What this means for Claudine:

- If you want all subagents to inherit "you are running headless, do not ask for user input", the strongest native Goose mechanism is customizing `subagent_system.md`
- If you want it only for one run, `--system` and repo hints help, but they are broader and less targeted than a dedicated subagent-only switch

What I did **not** find:

- A dedicated `goose run` flag whose sole purpose is "prepend this exact string to every subagent prompt"

Operational recommendation for Claudine:

- Use prompt-template customization for persistent policy
- Use `--system` or repo hints for per-run policy
- Still detect `actionRequired` in case a tool or extension asks anyway

Hook parity:

- Not applicable
- This is a configuration / prompting capability, not a runtime event hook

Sources:

- [Customizing Prompt Templates](https://goose-docs.ai/docs/guides/prompt-templates/)
- [Using goose in Headless Mode for Automation](https://goose-docs.ai/docs/tutorials/headless-goose/)
- [Using Subagents](https://goose-docs.ai/docs/tutorials/subagents/)
- [v1.27.0 release](https://github.com/aaif-goose/goose/releases/tag/v1.27.0) mentions restored subagent system-prompt behavior
