---
schema: https://github.com/QwenLM/qwen-code/blob/main/packages/sdk-typescript/src/types/protocol.ts
schema_type: typescript
data_format: json-array + ndjson
docs: https://qwenlm.github.io/qwen-code-docs/en/users/features/headless/
created: 2026-04-10
last_updated: 2026-04-10
---

## Summary

Qwen CLI currently has two provider-supported structured output modes for non-interactive runs: `--output-format json` and `--output-format stream-json`. `json` is a buffered JSON array emitted at the end of the run. `stream-json` is a line-delimited stream of JSON objects that can carry both completed messages and, when `--include-partial-messages` is enabled, lower-level streaming events such as `message_start`, `content_block_delta`, and `tool_progress`.

For Claudine, `stream-json` is the more useful transport, but it is not a complete orchestration protocol by itself. It is strong for model identity, tool calls, tool results, subagent linkage, and aggregate token usage. It is weak for auth detection, quota state, permission denials with full path context, and human-in-the-loop questions unless Qwen is being driven through its bidirectional `stream-json` control plane or the official SDK.

Qwen does not publish a standalone JSON Schema, OpenAPI document, or AsyncAPI document for this exact non-interactive output. The best formal schema available today is the provider's own TypeScript protocol definition in [`packages/sdk-typescript/src/types/protocol.ts`](https://github.com/QwenLM/qwen-code/blob/main/packages/sdk-typescript/src/types/protocol.ts), mirrored closely by [`packages/cli/src/nonInteractive/types.ts`](https://github.com/QwenLM/qwen-code/blob/main/packages/cli/src/nonInteractive/types.ts). The official docs are good enough to learn the feature, but the source and tests are still the more authoritative references for edge cases.

The practical recommendation for Claudine is:

- Use `--output-format stream-json` for ordinary machine-readable supervision.
- Add `--include-partial-messages` when you need streaming deltas or MCP progress events.
- Use bidirectional `--input-format stream-json --output-format stream-json` or the [`@qwen-code/sdk` docs](https://qwenlm.github.io/qwen-code-docs/en/developers/sdk-typescript/) when you need structured permission handling or question/answer loops.

## Schema

### Best formal schema currently available

Qwen's best current schema source is provider-authored TypeScript, not JSON Schema:

| Artifact | Schema language | URL | Notes |
| --- | --- | --- | --- |
| Headless protocol types | TypeScript | [packages/sdk-typescript/src/types/protocol.ts](https://github.com/QwenLM/qwen-code/blob/main/packages/sdk-typescript/src/types/protocol.ts) | Best externalized formal definition for message, stream-event, result, and control-plane shapes |
| CLI-local protocol types | TypeScript | [packages/cli/src/nonInteractive/types.ts](https://github.com/QwenLM/qwen-code/blob/main/packages/cli/src/nonInteractive/types.ts) | Closest to the shipping CLI implementation |
| SDK reference | Docs + TypeScript concepts | [TypeScript SDK docs](https://qwenlm.github.io/qwen-code-docs/en/developers/sdk-typescript/) | Documents message categories and control callbacks, but not the entire union in one formal schema |

### What the schema looks like

At the top level, Qwen models the non-interactive protocol as two adjacent unions: one for ordinary output messages, and one for control-plane messages used by SDK-style integrations:

```ts
type CLIMessage =
  | { type: "user"; ... }
  | { type: "assistant"; ... }
  | { type: "system"; ... }
  | { type: "result"; ... }
  | { type: "stream_event"; ... };

type ControlMessage =
  | { type: "control_request"; ... }
  | { type: "control_response"; ... }
  | { type: "control_cancel_request"; ... };
```

The streaming event layer currently includes:

```ts
type CLIStreamEvent["event"]["type"] =
  | "message_start"
  | "content_block_start"
  | "content_block_delta"
  | "content_block_stop"
  | "message_stop"
  | "tool_progress";
```

Important payload families:

- Assistant content blocks can be `text`, `thinking`, `tool_use`, or `tool_result`.
- Final `result` envelopes carry `subtype`, duration fields, `usage`, `permission_denials`, and optionally `stats`.
- Control requests currently include `can_use_tool`, and the SDK protocol also models `blocked_path`, even though the current CLI implementation sends `null` there.

### What does not exist

I did not find an official:

- JSON Schema for the non-interactive stream
- OpenAPI document for the CLI's `json` or `stream-json` output
- AsyncAPI document for the stream
- third-party schema package from a major integration project that formalizes the Qwen CLI stream better than Qwen's own source

### Places checked

Before concluding that no standalone machine-readable schema exists for the exact CLI output, I checked:

- [Headless Mode](https://qwenlm.github.io/qwen-code-docs/en/users/features/headless/)
- [Settings](https://qwenlm.github.io/qwen-code-docs/en/users/configuration/settings/)
- [Authentication](https://qwenlm.github.io/qwen-code-docs/en/users/configuration/auth/)
- [TypeScript SDK docs](https://qwenlm.github.io/qwen-code-docs/en/developers/sdk-typescript/)
- the Qwen source files above
- the published `@qwen-code/sdk` package metadata via UNPKG
- code search in Vercel AI SDK and LangChain repositories for Qwen CLI stream/schema references, which returned no Qwen-specific formalization

## Documentation

### Official documentation

| Topic | URL | Why it matters |
| --- | --- | --- |
| Headless mode | [qwen-code-docs: Headless Mode](https://qwenlm.github.io/qwen-code-docs/en/users/features/headless/) | Primary user-facing docs for `json`, `stream-json`, `--include-partial-messages`, and system-prompt flags |
| Settings and CLI flags | [qwen-code-docs: Settings](https://qwenlm.github.io/qwen-code-docs/en/users/configuration/settings/) | Enumerates `outputFormat`, `inputFormat`, and related CLI/config behavior |
| Authentication | [qwen-code-docs: Authentication](https://qwenlm.github.io/qwen-code-docs/en/users/configuration/auth/) | Required for understanding headless auth limitations and quota context |
| TypeScript SDK | [qwen-code-docs: TypeScript SDK](https://qwenlm.github.io/qwen-code-docs/en/developers/sdk-typescript/) | Best public explanation of message categories and control callbacks |
| Tools overview | [qwen-code-docs: Tools Introduction](https://qwenlm.github.io/qwen-code-docs/en/developers/tools/introduction/) | Useful orientation for the built-in tool surface |
| Hooks docs in repo | [docs/users/features/hooks.md](https://github.com/QwenLM/qwen-code/blob/main/docs/users/features/hooks.md) | Important for understanding hook parity versus stream parity |
| Weekly update, March 13 2026 | [Qwen Code Weekly: Automated Workflows...](https://qwenlm.github.io/qwen-code-docs/en/blog/weekly-update-2026-03-13/) | Introduced hooks and `ask_user_question` |
| Weekly update, March 20 2026 | [Qwen Code Weekly: Token Limit Doubled...](https://qwenlm.github.io/qwen-code-docs/en/blog/weekly-update-2026-03-20/) | Gives context for token-usage visibility, though mainly in interactive UI |

### Secondary documentation and ecosystem references

Independent coverage of Qwen's structured output is still fairly sparse. The most useful external writeups I found were:

| Source | URL | Why it matters |
| --- | --- | --- |
| Kong Docs | [Route Qwen Code CLI traffic through AI Gateway](https://developer.konghq.com/how-to/use-qwen-code-with-ai-gateway/) | Shows Qwen being used programmatically and captures its API traffic in practice |
| DataCamp | [Qwen Code CLI: A Guide With Examples](https://www.datacamp.com/tutorial/qwen-code) | Broad adoption-focused tutorial; useful for understanding how developers are actually using the CLI |
| TongLife | [Qwen Code v0.3.0 release article](https://www.tonglife.net/post-6006.html) | One of the clearer secondary references calling out `stream-json` as a notable new capability |

### Historical issue references worth reading

These are not documentation, but they are highly relevant to the structured-output story:

- [Issue #795: request for `--output-format json/stream-json`](https://github.com/QwenLM/qwen-code/issues/795)
- [Issue #873: `--output-format` not recognized on older CLI versions](https://github.com/QwenLM/qwen-code/issues/873)

## CLI

### Available output formats

Qwen currently exposes three enumerated output formats:

| Format | Transport shape | What you get |
| --- | --- | --- |
| `text` | plain text | Human-readable prose on stdout |
| `json` | one JSON array | Buffered session transcript with `system`, `assistant`, `user`, and final `result` objects |
| `stream-json` | NDJSON / JSONL | One JSON object per line, optionally including low-level stream events |

Related input formats:

| Input format | Meaning |
| --- | --- |
| `text` | Normal prompt text |
| `stream-json` | Bidirectional structured protocol intended for SDK-style integration |

### CLI syntax

Basic buffered JSON:

```bash
qwen "Explain this repository" --output-format json
```

Streaming NDJSON:

```bash
qwen "Explain this repository" \
  --output-format stream-json \
  --include-partial-messages
```

Bidirectional structured mode:

```bash
cat request.jsonl | \
  qwen --input-format stream-json --output-format stream-json
```

Main-session prompt control:

```bash
qwen "Review this patch" \
  --output-format stream-json \
  --append-system-prompt "You are running headlessly. Do not ask the user questions."
```

### Side effects when switching formats

- `json` buffers everything until the run ends. It is easier to ingest if you only need the final session transcript.
- `json` is the only mode that currently emits the richer final `result.stats` object.
- `stream-json` emits each envelope as it happens. With `--include-partial-messages`, it can also emit `stream_event` objects for incremental text/tool blocks.
- `tool_progress` is only emitted in `stream-json` mode, only when partial messages are enabled, and only for MCP progress events.
- `--input-format stream-json` is not just a parser toggle. It switches Qwen into a control-capable protocol where stdin is reserved for structured messages.
- In practice, plain one-shot `json` or `stream-json` is not enough for full human-in-the-loop operation. Structured approvals and questions depend on the control plane or SDK host.

## Gotchas

- The official Headless Mode docs still show a first system message subtype of `session_start`, but the current implementation emits a `system` message with subtype `init`.
- The Headless Mode docs still include `jq '.response'` style examples, but the current `json` mode returns an array of message objects rather than a single top-level `response` field.
- `result.stats` is not symmetric across formats. It appears in buffered `json`, not in `stream-json`.
- `blocked_path` exists in the control-plane types, but the current CLI implementation sends `null`, so you cannot rely on it to identify the denied file path.
- Plain structured output still lacks dedicated typed events for auth method, quota-near-limit, quota-exhausted, or no-funds conditions. Those cases currently collapse into generic error/result text.
- `--output-format` is version-gated. [Issue #873](https://github.com/QwenLM/qwen-code/issues/873) shows an older `0.0.14` CLI rejecting the flag entirely.
- Qwen OAuth is explicitly a poor fit for headless CI or SSH use because it depends on browser login. For unattended runs, the official docs recommend Alibaba Cloud Coding Plan or API-key-based auth instead.
- The hook system is now real and useful, but some older summaries on the web still describe hooks as not yet shipped. Use the March 2026 weekly updates and current source, not older snapshots.

## Timeline

| Date | Artifact | Event | Why it matters |
| --- | --- | --- | --- |
| 2025-10-11 | [Issue #795](https://github.com/QwenLM/qwen-code/issues/795) | Community requests `--output-format json/stream-json` for programmatic integration | Earliest clear public record that structured headless output was being pushed as an integration requirement |
| 2025-11-21 | [commit `9e5387f15908c580b0ee9495b9f198e38299c899`](https://github.com/QwenLM/qwen-code/commit/9e5387f15908c580b0ee9495b9f198e38299c899) | Qwen adds `stream-json` as `input-format` and `output-format` | This is the real introduction point for the modern structured protocol |
| 2025-11-28 | [roadmap entry for Headless Mode](https://github.com/QwenLM/qwen-code/blob/main/docs/developers/roadmap.md) | `V0.3.0` lists Headless Mode as shipped | Useful release anchor for when structured non-interactive mode became productized |
| 2026-02-08 | [commit `5ebbceea65557beab917c18fc7c203d0555b3a31`](https://github.com/QwenLM/qwen-code/commit/5ebbceea65557beab917c18fc7c203d0555b3a31) | Adds `tool_progress` support for MCP progress updates | First meaningful expansion of stream richness beyond simple message/tool envelopes |
| 2026-03-13 | [weekly update](https://qwenlm.github.io/qwen-code-docs/en/blog/weekly-update-2026-03-13/) | Hooks system and `ask_user_question` ship in `v0.12.0` | Important because structured headless integration now has a hook/control counterpart |
| 2026-03-18 | [commit `eea92fc8dbc7f4a36026d088b1f96994d1695d47`](https://github.com/QwenLM/qwen-code/commit/eea92fc8dbc7f4a36026d088b1f96994d1695d47) and [commit `79083ffd50d84e7b7399f79692b82cb0159cc0b6`](https://github.com/QwenLM/qwen-code/commit/79083ffd50d84e7b7399f79692b82cb0159cc0b6) | Stream-event pairing and content-block handling are tightened | Signals that the protocol was still maturing and that consumers should expect some drift between versions |
| 2026-03-20 | [weekly update](https://qwenlm.github.io/qwen-code-docs/en/blog/weekly-update-2026-03-20/) | Real-time token usage display and session-export metadata improve | Not a headless schema change, but useful context for how token/usage observability was evolving around the same time |

## Tools

The source of truth for Qwen's first-party built-in tools is [`packages/core/src/tools/tool-names.ts`](https://github.com/QwenLM/qwen-code/blob/main/packages/core/src/tools/tool-names.ts) plus their registration in [`packages/core/src/config/config.ts`](https://github.com/QwenLM/qwen-code/blob/main/packages/core/src/config/config.ts). The public tools docs are helpful, but they are not the most exhaustive inventory.

### Built-in tool set

| Tool | Purpose | Notes |
| --- | --- | --- |
| `agent` | Spawn a subagent / task agent | Child activity is linked back with `parent_tool_use_id` |
| `skill` | Load and use skills | First-party tool for curated skill content |
| `list_directory` | Directory listing | Read-oriented |
| `read_file` | Read file contents | Read-oriented |
| `grep_search` | Search file contents | Read-oriented |
| `glob` | Find files by pattern | Read-oriented |
| `edit` | Patch existing files | Write-oriented |
| `write_file` | Write or create a file | Write-oriented |
| `run_shell_command` | Execute shell commands | Write/risk depends on command |
| `save_memory` | Persist memory | First-party long-term memory tool |
| `todo_write` | Create/update todo list | Planning/state-tracking tool |
| `ask_user_question` | Ask the user a structured question | Requires user participation path |
| `exit_plan_mode` | Exit plan mode after presenting a plan | Only relevant when planning workflow is active |
| `web_fetch` | Fetch a URL | First-party web retrieval |
| `web_search` | Search the web | Availability may depend on configuration/provider support |
| `lsp` | Language-server assistance | Experimental and feature-gated in practice |
| `cron_create` | Create scheduled task | First-party scheduler tool |
| `cron_list` | List scheduled tasks | First-party scheduler tool |
| `cron_delete` | Delete scheduled task | First-party scheduler tool |

### What the JSON stream shows before and after tool calls

For most first-party tools, the structured output follows the same pattern:

| Phase | JSON / stream shape | What is exposed |
| --- | --- | --- |
| Before execution | `assistant` message with a `tool_use` content block | tool name, tool-use id, raw tool input |
| During execution | usually nothing extra for core built-ins | no dedicated progress object for most core tools |
| Progress updates | `stream_event` with `tool_progress` | only for MCP progress events, only in `stream-json` with partials |
| After execution | `user` message with a `tool_result` content block | tool-use id, success/error content, and tool result payload |
| Final session summary | final `result` message | aggregate `permission_denials`, `usage`, durations, and maybe `stats` |

### Example: ordinary file read

```json
{"type":"assistant","message":{"role":"assistant","content":[{"type":"tool_use","id":"toolu_1","name":"read_file","input":{"path":"/repo/src/lib.rs"}}],"stop_reason":"tool_use","usage":{"input_tokens":123,"output_tokens":18}}}
{"type":"user","message":{"role":"user","content":[{"type":"tool_result","tool_use_id":"toolu_1","content":"pub fn main() { ... }","is_error":false}]}}
```

### Example: partial stream around a tool call

```json
{"type":"stream_event","event":{"type":"message_start","message":{"role":"assistant"}}}
{"type":"stream_event","event":{"type":"content_block_start","index":0,"content_block":{"type":"tool_use","id":"toolu_2","name":"write_file"}}}
{"type":"stream_event","event":{"type":"content_block_delta","index":0,"delta":{"type":"input_json_delta","partial_json":"{\"path\":\"/tmp/out.txt\""}}}
{"type":"stream_event","event":{"type":"content_block_stop","index":0}}
{"type":"stream_event","event":{"type":"message_stop"}}
```

### Example: denied write operation

```json
{"type":"assistant","message":{"role":"assistant","content":[{"type":"tool_use","id":"toolu_3","name":"write_file","input":{"path":"/protected/file.txt","content":"hi"}}],"stop_reason":"tool_use"}}
{"type":"result","subtype":"error_during_execution","permission_denials":[{"tool_name":"write_file","tool_use_id":"toolu_3","tool_input":{"path":"/protected/file.txt","content":"hi"}}]}
```

### Important tool-observability limits

- Core built-in tools do not currently emit a rich "started / still running / finished" lifecycle beyond `tool_use` followed by `tool_result`.
- Only MCP progress currently maps to `tool_progress`.
- For denied calls, the final `permission_denials` array is useful but incomplete. It does not carry a typed denial reason or a separate `blocked_path`.
- `ask_user_question` is special. In plain non-interactive mode it often degrades into a failed tool result; in ACP/SDK-driven runs it can surface as structured confirmation/question data.
- Subagent work is visible, but it is not a separate top-level stream family. Child messages are normal messages associated with the parent `agent` tool call via `parent_tool_use_id`.

## Use Cases

### Plan Cap Approaching

- **Event type**: I did not find a dedicated structured event or hook for "your quota is almost exhausted."
- **How to distinguish it**: There is no stable typed signal in the non-interactive stream. Interactive UI code contains token-limit warnings, but that is not the same as a plan or billing cap event in headless output.
- **How much is left**: Not exposed.
- **Reset window**: Not exposed.
- **Hook parity**: No corresponding hook event was found.

### Plan Capped

- **Event type**: There is no dedicated `quota_exceeded` or `plan_capped` message type. In practice this collapses into a generic failure, usually a final `result` with `subtype: "error_during_execution"` and quota-related text.
- **How to distinguish it**: For Qwen OAuth specifically, the most reliable signatures are provider-side `429`, code `insufficient_quota`, message text containing `free allocated quota exceeded`, or the normalized error text `Qwen OAuth quota exceeded: Your free daily quota has been reached.`
- **How much is left**: Not exposed.
- **Reset window**: Not exposed in the stream. The official auth docs only document the general limits for Qwen OAuth: `60 requests/minute` and `1,000 requests/day`.
- **Hook parity**: No dedicated hook event. The current notification hook types are `permission_prompt`, `idle_prompt`, and `auth_success`, with `elicitation_dialog` defined but not documented as implemented.

### No Funds

- **Event type**: No dedicated structured event was found.
- **How to distinguish it**: Expect provider-specific billing text inside a generic execution failure rather than a Qwen-specific typed envelope. This is especially true when using third-party OpenAI-compatible endpoints.
- **How much is left**: Not exposed.
- **Reset window**: Not exposed.
- **Hook parity**: No dedicated hook event was found.

### Auth

- **Can the stream identify auth kind?** Plain `json` and `stream-json` do not expose a typed `auth_type` field.
- **What can be inferred anyway?** Very little. `system` messages carry the model id, not the auth method. Even `openai`-protocol auth is ambiguous because it may represent Alibaba Coding Plan, DashScope, OpenRouter, or another compatible endpoint.
- **Best available structured signal**: The hook system is better than the stream here. The `Notification` hook with `notification_type: "auth_success"` receives a message like `Successfully authenticated with <authMethod>`, which lets you infer the auth protocol.
- **Hook parity**: Yes, but it is not identical. The stream has no direct auth event; the hook provides only a free-form success message, not a first-class `auth_type` field.

### Permissions: Can't Read File

- **Event type**: Plain headless output has no dedicated "read denied" event.
- **How to distinguish it**: Use the attempted `assistant` `tool_use` block plus the final `result.permission_denials[]`. Read-oriented denials normally involve `read_file`, `list_directory`, `glob`, or `grep_search`.
- **How to identify the full path**: There is no dedicated path field in the final result. You can often recover it from `tool_input.path` or equivalent fields inside `permission_denials[].tool_input`.
- **Why it was blocked**: There is no typed denial reason in the final structured output. If a reason is surfaced, it is usually generic permission text.
- **Hook parity**: Yes. `PreToolUse` and `PermissionRequest` hooks expose `tool_name` and `tool_input` before execution, which is better than the stream for policy enforcement. However, this still does not give you a populated `blocked_path` field today.

### Permissions: Can't Write File

- **Event type**: Plain headless output again has no dedicated "write denied" event.
- **How to distinguish it**: Look for `edit` or `write_file` in the attempted `tool_use`, then inspect `result.permission_denials[]` or the failure text. `run_shell_command` may also be relevant when a shell action would modify the filesystem.
- **How to identify the full path**: As with read denials, path information must usually be recovered from `tool_input`.
- **Why it was blocked**: No typed reason field is exposed in the final stream. The control-plane type has `blocked_path`, but current implementation sends `null`.
- **Hook parity**: Yes. `PermissionRequest` is particularly useful here because write tools are the main path where confirmation/approval logic appears. The hook payload is more actionable than the plain output stream.

### Tokens Consumed

- **Overall session usage**: The final `result.usage` object is the main overall token summary.
- **More granular usage**: Completed `assistant` messages also carry `message.usage`, which lets you attribute usage to individual assistant turns. Buffered `json` mode additionally emits `result.stats`, which includes richer per-model and per-tool telemetry.
- **Cost basis**: No explicit money or currency field is exposed in the headless protocol.
- **Caveat**: The TypeScript types define `modelUsage`, but the current result builder does not populate it.
- **Hook parity**: No dedicated hook event carries the same usage payloads today.

### Model Used

- **Event type**: The most reliable field is the opening `system` message. In the current implementation its subtype is `init`, and it includes `model`.
- **Does it always fire?** Yes for the session start path observed in current code and tests. Assistant messages also include `message.model` once a model has actually produced a response.
- **Naming format**: The value is the raw configured model id, such as `qwen3-coder-plus` or another provider-defined model string. Provider identity is not emitted as a separate typed field.
- **Hook parity**: Yes. `SessionStart` hook input includes a typed `model` field and a `source` field. That is not identical to the stream, but it is a useful parallel signal.

### Human in the Loop

- **Permissions in non-interactive mode**: Yes, but only in the bidirectional control path. With `--input-format stream-json --output-format stream-json` or SDK control, Qwen can emit `control_request` messages with subtype `can_use_tool`.
- **What the permission request includes**: Structured fields include `tool_name`, `tool_input`, and a typed request/response cycle. The schema also includes `blocked_path`, but current implementation sends `null`.
- **Asking the user questions**: Plain one-shot headless mode does not emit a dedicated structured question event. Without ACP-style support, `ask_user_question` degrades into a failed tool result that says user questions cannot be asked in non-interactive mode.
- **Subagents**: Yes in the structured control path. The repo tests show question payloads propagating from subagents, and subagent messages are linked using `parent_tool_use_id`.
- **Hook parity**: Partial. `PermissionRequest` exists and is useful. `Notification(elicitation_dialog)` is typed in the hook system, but the published hooks docs say it is not currently implemented, so I would not depend on it as the primary question signal.

### Injecting into Subagent Prompt

- **Dedicated CLI support**: I did not find a dedicated `--subagent-system-prompt` style flag for headless mode.
- **What is explicitly supported**: `SubagentStart` hooks can return `hookSpecificOutput.additionalContext`, which is the cleanest first-party way to inject additional structured context into a subagent run.
- **What probably works indirectly**: Main-session `--system-prompt` and `--append-system-prompt` likely influence subagents indirectly because they shape the overall session prompt context, but that is an inference rather than a dedicated contract.
- **Alternative mechanism**: Agent definitions themselves have a `systemPrompt`, which is a stronger way to bake in non-interactive behavior for a known subagent.
- **Hook parity**: This use case is primarily hook-based. The stream lets you observe subagent prompt traffic, but it does not provide a dedicated injection channel by itself.
