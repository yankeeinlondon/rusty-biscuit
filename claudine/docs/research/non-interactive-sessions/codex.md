---
schema: https://raw.githubusercontent.com/openai/codex/main/codex-rs/app-server-protocol/schema/json/codex_app_server_protocol.v2.schemas.json
schema_type: json-schema
data_format: JSONL
docs: https://developers.openai.com/codex/noninteractive
created: 2026-04-06
last_updated: 2026-04-06
---

# OpenAI Codex CLI: Non-Interactive Structured Output

## Summary

`codex exec --json` is the current non-interactive structured output surface for Codex CLI. It writes newline-delimited JSON to `stdout`, with each line tagged by a top-level `type` such as `thread.started`, `turn.started`, `item.started`, `item.updated`, `item.completed`, `turn.completed`, `turn.failed`, or `error`.

The most accurate source of truth for the exact `exec --json` event shapes is the upstream Rust source at [`codex-rs/exec/src/exec_events.rs`](https://github.com/openai/codex/blob/main/codex-rs/exec/src/exec_events.rs). OpenAI also publishes a broader, formal JSON Schema bundle for the Codex App Server protocol, but that bundle is not a one-to-one schema for `codex exec --json`; `exec` is a simplified projection of the richer app-server event model.

For Claudine, the practical implications are:

- The stream is good for session boundaries, tool lifecycle, token counts, MCP failures, and final-message capture.
- The stream does not currently expose model name, auth mode, cost basis, or ChatGPT plan-cap percentages in a stable documented way.
- The exact `exec` item union differs from the richer app-server `ThreadItem` union. Claudine should not parse `exec --json` as if it were raw app-server v2 JSON-RPC.
- Human-in-the-loop signals exist in the broader Codex protocol (`requestUserInput`, approval requests, hooks), but `exec --json` mostly surfaces those as indirect failures or not at all.

## Schema

### Best Exact Schema for `codex exec --json`

The best exact schema OpenAI currently ships for the `exec --json` stream is the Rust enum/struct source:

- Exact event source:
  - <https://github.com/openai/codex/blob/main/codex-rs/exec/src/exec_events.rs>
- Closely related mapper:
  - <https://github.com/openai/codex/blob/main/codex-rs/exec/src/event_processor_with_jsonl_output.rs>

Schema language / format:

- Rust enums and structs, serialized with Serde
- Generated TypeScript in the official SDK mirrors this surface, but is not always perfectly current

As of April 6, 2026, the exact top-level `exec` union in `exec_events.rs` is:

- `thread.started`
- `turn.started`
- `turn.completed`
- `turn.failed`
- `item.started`
- `item.updated`
- `item.completed`
- `error`

As of the same date, the exact `exec` item union in `exec_events.rs` is:

- `agent_message`
- `reasoning`
- `command_execution`
- `file_change`
- `mcp_tool_call`
- `collab_tool_call`
- `web_search`
- `todo_list`
- `error`

Important caveat:

- The official non-interactive docs list `item.*` generically and show only a subset of item types.
- The official app-server schemas cover many more item types, including approvals, hook prompts, dynamic tools, and richer thread state.
- The official TypeScript SDK files for `exec` currently lag the Rust source in a few places. For example, `sdk/typescript/src/events.ts` includes `item.updated`, but `sdk/typescript/src/items.ts` does not currently include `collab_tool_call`, and its status enums are narrower than the Rust source.

Relevant official SDK files:

- <https://github.com/openai/codex/blob/main/sdk/typescript/src/events.ts>
- <https://github.com/openai/codex/blob/main/sdk/typescript/src/items.ts>

### Best Formal Provider Schema

The best formal schema OpenAI publishes is the Codex App Server JSON Schema bundle:

- Raw JSON Schema bundle:
  - <https://raw.githubusercontent.com/openai/codex/main/codex-rs/app-server-protocol/schema/json/codex_app_server_protocol.v2.schemas.json>
- Schema directory:
  - <https://github.com/openai/codex/tree/main/codex-rs/app-server-protocol/schema/json>

Schema language / format:

- JSON Schema Draft 7

Why it matters:

- It is provider-authored.
- It includes formal request/response/notification schemas.
- It covers approvals, `tool/requestUserInput`, auth/account events, rate-limit events, and v2 thread items.

Why it is not enough on its own for Claudine:

- It formalizes the bidirectional app-server JSON-RPC protocol.
- `codex exec --json` is not raw app-server JSON-RPC; it is a flattened JSONL event stream produced by the `exec` layer.

OpenAI’s own App Server blog explicitly says you can generate the JSON Schema bundle with:

- `codex app-server generate-json-schema`

Source:

- <https://openai.com/index/unlocking-the-codex-harness/>

### Broader Search Results

I looked for a dedicated formal schema specifically for `codex exec --json` in:

- Official docs:
  - <https://developers.openai.com/codex/noninteractive>
  - <https://developers.openai.com/codex/cli/reference>
  - <https://developers.openai.com/codex/app-server>
- Official repo:
  - `codex-rs/exec/`
  - `codex-rs/app-server-protocol/schema/`
  - `sdk/typescript/src/`
- npm / SDK surface:
  - `@openai/codex`
  - official TypeScript SDK sources in the repo
- PyPI / broader ecosystem:
  - broad search for standalone `codex exec --json` schemas
- Other respected codebases:
  - Vercel AI SDK
  - LangChain JS / TypeScript

Result:

- I did not find a provider-published standalone JSON Schema or OpenAPI document dedicated only to `codex exec --json`.
- I did not find a Vercel AI SDK or LangChain JS formalization of the `exec --json` stream.
- I did find third-party typed wrappers, such as `codex-codes`, but they are not canonical:
  - <https://docs.rs/codex-codes/latest/codex_codes/>

## Documentation

### Official Documentation

- Non-interactive mode:
  - <https://developers.openai.com/codex/noninteractive>
- CLI reference:
  - <https://developers.openai.com/codex/cli/reference>
- CLI features:
  - <https://developers.openai.com/codex/cli/features>
- App Server:
  - <https://developers.openai.com/codex/app-server>
- Hooks:
  - <https://developers.openai.com/codex/hooks>
- Subagents:
  - <https://developers.openai.com/codex/subagents>
- Changelog:
  - <https://developers.openai.com/codex/changelog>
- Repo:
  - <https://github.com/openai/codex>

### Official Articles

- Unlocking the Codex harness: how we built the App Server
  - <https://openai.com/index/unlocking-the-codex-harness/>
  - Best high-level explanation of the App Server protocol, JSON-RPC-over-JSONL transport, approvals, and generated schemas.
- Introducing GPT-5-Codex
  - <https://developers.openai.com/codex/changelog>
  - Relevant because it anchors the period where the CLI/app-server architecture and related structured surfaces became more explicit in public release notes.
- Introducing the Codex app
  - Linked from the Codex changelog and relevant for current plan/rate-limit context across app, CLI, IDE, and cloud.

### Community / Ecosystem References

- `codex-codes` Rust crate docs
  - <https://docs.rs/codex-codes/latest/codex_codes/>
  - Useful because it explicitly parses both exec-format JSONL and the app-server JSON-RPC protocol.
- OpenAI Developer Community, Codex CLI category
  - <https://community.openai.com/c/codex/codex-cli/39>
  - Useful for real-world gotchas, rate-limit confusion, and auth behavior reports.
- OpenAI Developer Community thread on API credits vs ChatGPT subscription limits in Codex surfaces
  - <https://community.openai.com/t/introducing-the-codex-ide-extension/1354930>
- SmartScope article on a concrete Codex CLI failure mode
  - <https://smartscope.blog/en/generative-ai/chatgpt/codex-cli-reconnecting-issue-2025/>

## CLI

### Output Formats Available in Practice

For non-interactive Codex runs, the meaningful output modes are:

| Mode | How to enable | What you get |
| --- | --- | --- |
| Default text mode | `codex exec "prompt"` | Progress on `stderr`, final assistant message on `stdout` |
| JSONL event stream | `codex exec --json "prompt"` | Newline-delimited JSON events on `stdout` instead of formatted text |
| Final-message file | `codex exec -o out.txt "prompt"` | Same as default mode, plus final assistant message written to a file |
| JSONL + final-message file | `codex exec --json -o out.txt "prompt"` | JSONL stream on `stdout`, plus final assistant message written separately |
| Schema-constrained final output | `codex exec --output-schema schema.json "prompt"` | Final assistant message is requested to conform to a JSON Schema |
| JSONL + schema-constrained final output | `codex exec --json --output-schema schema.json -o out.json "prompt"` | Live JSONL telemetry plus a schema-constrained final artifact |

### Syntax

Current help and docs support:

```bash
codex exec "prompt"
codex exec --json "prompt"
codex exec --output-schema ./schema.json -o ./result.json "prompt"
codex exec resume --last "follow-up prompt"
codex exec resume <SESSION_ID> "follow-up prompt"
```

Current official docs and local `codex-cli 0.115.0` agree on these relevant flags:

- `--json`
- `--output-schema <path>`
- `--output-last-message <path>` / `-o <path>`
- `--ephemeral`
- `resume`

The reference docs still list `--experimental-json` as an alias of `--json`:

- <https://developers.openai.com/codex/cli/reference>

The local CLI help on this machine (`codex-cli 0.115.0`) currently shows `--json` but not `--experimental-json` in the short help text.

### Output-Mode Side Effects

- `--json` changes `stdout` from a single final answer into a JSONL event stream.
- In default text mode, progress goes to `stderr`; in JSON mode, progress is represented as events.
- `-o` is additive, not redirective. The final assistant message is still otherwise emitted through the normal output path.
- `--output-schema` constrains only the final assistant response shape, not the intermediate event stream.
- `--ephemeral` disables session persistence, which also makes `resume` impossible for that run.

### Enumerated Event Types in the Exact `exec` Stream

From the current upstream `exec_events.rs`, the top-level event types are:

- `thread.started`
- `turn.started`
- `turn.completed`
- `turn.failed`
- `item.started`
- `item.updated`
- `item.completed`
- `error`

Notable implication:

- `item.updated` is real and currently used for `todo_list` updates, even though most high-level docs only describe `item.*` generically.

## Gotchas

### 1. The Exact `exec` Schema and the App Server Schema Are Not the Same Thing

This is the biggest integration trap.

- `codex exec --json` uses the `exec_events.rs` surface.
- The app-server JSON Schema bundle covers a richer JSON-RPC protocol.
- Many fields and item types exist in app-server that never appear in `exec --json`.

For Claudine, this means:

- Parse `exec --json` with an `exec`-specific parser.
- Treat app-server schemas as broader protocol context, not as a direct drop-in validator for the CLI stream.

### 2. The Official TypeScript SDK Lags the Rust Source in a Few Places

As of April 6, 2026:

- `sdk/typescript/src/events.ts` includes `item.updated`.
- `sdk/typescript/src/items.ts` does not currently include `collab_tool_call`.
- `items.ts` also narrows some status enums compared with the Rust source.

So:

- For exactness, prefer `codex-rs/exec/src/exec_events.rs`.
- Treat SDK TS types as useful, but not always exhaustive.

### 3. `file_change` Declines Are Collapsed to `failed` in the `exec` Projection

The broader protocol supports `declined` for file changes, but the `exec` projection currently maps declined file changes to `failed`.

Implication:

- In `exec --json`, you cannot always distinguish "write was declined by policy/approval" from "write failed for another reason" by item status alone.

Evidence:

- Upstream `event_processor_with_json_output` tests in the Codex repo.

### 4. Some Rich Signals Are Missing from `exec --json`

Still missing as stable documented `exec` fields:

- active model name
- auth mode
- ChatGPT rate-limit percentages
- reset timestamps
- pricing / cost basis

Relevant public evidence:

- <https://github.com/openai/codex/issues/14728>
- <https://github.com/openai/codex/issues/14736>

### 5. Reasoning Output Can Disappear Under API-Key Auth

Open issue:

- <https://github.com/openai/codex/issues/10746>

Implication:

- Absence of `reasoning` items is not proof that the model produced no reasoning.
- Do not build core flow control around `reasoning` items being present.

### 6. `--output-schema` Has Real Edge Cases

Known problems include:

- tool/MCP runs causing schema-constrained output to degrade:
  - <https://github.com/openai/codex/issues/15451>
- internal schema shape bug:
  - <https://github.com/openai/codex/issues/16552>
- no `--output-schema` on `exec resume`:
  - <https://github.com/openai/codex/issues/14343>

Implication:

- `--output-schema` is useful, but not yet strong enough to treat as infallible in all automation paths.

### 7. MCP Tool Calls in `exec` Can Still Hit Hidden Human-Input Paths

Open issue:

- <https://github.com/openai/codex/issues/16685>

Observed behavior:

- an MCP call starts
- it quickly completes with failure
- the error message is `user cancelled MCP tool call`
- logs indicate `request_user_input is not supported in exec mode`

Implication:

- Non-interactive runs can still encounter code paths that were designed for user elicitation.

### 8. Hook Parity Is Incomplete for Exec Sessions

Open issue:

- <https://github.com/openai/codex/issues/16246>

Implication:

- If Claudine relies on hooks for lifecycle pairing, it should expect gaps, especially around some long-running tool executions.

### 9. Process Exit Code Is Not a Reliable Command-Failure Signal

Open issue:

- <https://github.com/openai/codex/issues/15536>

Implication:

- Parse the JSONL stream for failed `command_execution` items.
- Do not rely only on the outer `codex exec` process exit code.

## Timeline

The dates below focus on structured output and adjacent protocol maturity.

| Date | Event | Why it matters |
| --- | --- | --- |
| 2025-07-24 | Issue [#1673](https://github.com/openai/codex/issues/1673) opened requesting a JSON Schema for `--json` output | Confirms `codex exec --json` was already public by this date and that a dedicated published schema still did not exist |
| 2025-10-05 | Issue [#4776](https://github.com/openai/codex/issues/4776) filed about JSON output docs being out of date | Shows the structured-output contract was changing fast enough that docs drift became visible |
| 2025-11-16 | Issue [#6717](https://github.com/openai/codex/issues/6717) filed for `exec resume --last` rejecting a prompt when `--json` is set | Important if Claudine wants resumable JSON-mode runs; the issue was later closed on January 27, 2026 |
| 2026-03-10 | Codex CLI `0.113.0` released | Changelog says `exec` was wired to the new in-process app-server path, which explains why the `exec` surface increasingly shadows app-server concepts |
| 2026-03-11 | Codex CLI `0.114.0` released | Changelog notes the hooks engine start and groundwork for generated v2 schema types, both relevant to structured integrations |
| 2026-03-15 | Issues [#14728](https://github.com/openai/codex/issues/14728) and [#14736](https://github.com/openai/codex/issues/14736) opened | Confirms that even after the Rust/app-server transition, `exec --json` still lacked rate-limit and model metadata |
| 2026-04-02 to 2026-04-03 | Issues [#16552](https://github.com/openai/codex/issues/16552) and [#16685](https://github.com/openai/codex/issues/16685) opened | Shows active regressions affecting schema-constrained automation and non-interactive MCP flows |

Contextual release references:

- Codex CLI `0.113.0` changelog entry:
  - <https://developers.openai.com/codex/changelog>
- Codex CLI `0.114.0` changelog entry:
  - <https://developers.openai.com/codex/changelog>

## Tools

The exact `exec` item model currently exposes these built-in runtime surfaces:

| Item type | Lifecycle in `exec --json` | Key metadata available |
| --- | --- | --- |
| `command_execution` | `item.started` then `item.completed` | `command`, `aggregated_output`, `exit_code`, `status` |
| `file_change` | completion only in the `exec` projection | `changes[].path`, `changes[].kind`, `status` |
| `mcp_tool_call` | `item.started` then `item.completed` | `server`, `tool`, `arguments`, `result`, `error`, `status` |
| `collab_tool_call` | `item.started` then `item.completed` | `tool`, `sender_thread_id`, `receiver_thread_ids`, `prompt`, `agents_states`, `status` |
| `web_search` | item lifecycle events | `query`, and in Rust source `action` |
| `todo_list` | `item.started`, `item.updated`, `item.completed` | checklist items with `completed` booleans |
| `agent_message` | completion-focused in practice | `text` |
| `reasoning` | completion-focused in practice | summary text |
| `error` item | completion only | non-fatal warning/error message |

### Before / After Visibility

#### `command_execution`

Before:

```json
{"type":"item.started","item":{"id":"item_1","type":"command_execution","command":"bash -lc ls","aggregated_output":"","status":"in_progress"}}
```

After:

```json
{"type":"item.completed","item":{"id":"item_1","type":"command_execution","command":"bash -lc ls","aggregated_output":"Cargo.toml\nsrc\n","exit_code":0,"status":"completed"}}
```

Useful to Claudine:

- exact command string
- combined output
- exit code
- machine-readable success / failure / decline state

#### `file_change`

In `exec --json`, file changes are effectively summarized after the patch attempt:

```json
{"type":"item.completed","item":{"id":"item_4","type":"file_change","changes":[{"path":"src/lib.rs","kind":"update"}],"status":"completed"}}
```

Useful to Claudine:

- changed paths
- add/delete/update classification
- high-level success/failure state

Limitation:

- the `exec` projection does not preserve the richer approval/request lifecycle the app-server has for file changes

#### `mcp_tool_call`

Before:

```json
{"type":"item.started","item":{"id":"item_7","type":"mcp_tool_call","server":"memory","tool":"create_entities","arguments":{"entities":[{"name":"A"}]},"status":"in_progress"}}
```

After success:

```json
{"type":"item.completed","item":{"id":"item_7","type":"mcp_tool_call","server":"memory","tool":"create_entities","arguments":{"entities":[{"name":"A"}]},"result":{"content":[{"type":"text","text":"Created 1 entity"}],"structured_content":null},"status":"completed"}}
```

After failure:

```json
{"type":"item.completed","item":{"id":"item_7","type":"mcp_tool_call","server":"minimal","tool":"count","arguments":{},"error":{"message":"user cancelled MCP tool call"},"status":"failed"}}
```

Useful to Claudine:

- target server/tool
- structured arguments
- structured result when present
- direct failure reason when present

#### `todo_list`

Started:

```json
{"type":"item.started","item":{"id":"item_0","type":"todo_list","items":[{"text":"inspect repo","completed":false},{"text":"write summary","completed":false}]}}
```

Updated:

```json
{"type":"item.updated","item":{"id":"item_0","type":"todo_list","items":[{"text":"inspect repo","completed":true},{"text":"write summary","completed":false}]}}
```

Completed:

```json
{"type":"item.completed","item":{"id":"item_0","type":"todo_list","items":[{"text":"inspect repo","completed":true},{"text":"write summary","completed":false}]}}
```

Useful to Claudine:

- progress UI
- plan-to-status mapping
- low-latency progress without needing natural-language parsing

## Use Cases

### Plan Cap Approaching

- `exec --json` event type:
  - No dedicated documented event.
- Exact `exec` schema support:
  - No field in `ThreadEvent`, `Usage`, or `ThreadItem` reports "approaching limit".
- Distinguishing signal:
  - Not reliably possible from `exec --json` alone.
- Amount remaining:
  - Not exposed in `exec --json`.
- Reset window:
  - Not exposed in `exec --json`.
- Better alternative outside `exec`:
  - The app-server exposes `account/rateLimits/read` and `account/rateLimits/updated`, including `usedPercent`, `windowDurationMins`, and `resetsAt`, but that is a different surface:
    - <https://developers.openai.com/codex/app-server>
- Hook exposure:
  - No. Hooks do not include a rate-limit or usage-cap event.

### Plan Capped

- `exec --json` event type:
  - No dedicated cap-specific event.
  - In practice, the best available signal is likely a top-level `error` or `turn.failed`, but the `exec` stream only guarantees a message string.
- Distinguishing signal:
  - In `exec --json`, only by text matching on the error message.
  - In the broader app-server protocol, failed turns can carry structured `codexErrorInfo`, including `UsageLimitExceeded`.
- Amount remaining:
  - Not exposed in `exec --json`.
- Reset window:
  - Not exposed in `exec --json`.
- Hook exposure:
  - No.

### No Funds

- `exec --json` event type:
  - No dedicated "no funds" event.
  - Likely surfaces as `error` or `turn.failed.error.message`.
- Distinguishing signal:
  - In `exec --json`, only by message text such as quota / billing language.
  - For API-key flows generally, upstream API-style errors often use `insufficient_quota`, but Codex does not currently document a stable `exec` enum for this.
- Practical distinction from plan cap:
  - ChatGPT plan-cap problems and API-credit problems are different failure classes, but `exec --json` does not expose a stable machine-readable discriminator for either.
- Hook exposure:
  - No.

### Auth

- Can the stream reveal auth kind?
  - Not in `codex exec --json`.
- Out-of-band CLI support:
  - `codex login status` prints active authentication mode.
  - CLI docs explicitly document browser login and `--with-api-key`.
- Broader protocol support:
  - The app-server auth/account API exposes `authMode` with:
    - `apikey`
    - `chatgpt`
    - `chatgptAuthTokens`
  - Source:
    - <https://developers.openai.com/codex/app-server>
- Hook exposure:
  - No hook event exposes auth mode directly.

### Permissions: Can't Read File

- Dedicated `exec --json` event?
  - No dedicated "read denied" event.
- Best available `exec` signals:
  - `command_execution` with `status: "declined"` if the command/tool was denied before running
  - `command_execution` with `status: "failed"` plus `aggregated_output` containing an OS-level permission error if the command actually ran and the OS denied access
- How to identify the path:
  - Best effort from the shell command string itself
  - In the broader app-server approval path, `item/commandExecution/requestApproval` can include `commandActions` with parsed paths
- Is a reason exposed?
  - Not as a stable top-level `exec` field
  - Often only embedded in text output or approval/hook feedback
- Distinguishing from similar events:
  - `declined` suggests policy/approval denial before execution
  - `failed` suggests the command ran and failed
- Hook exposure:
  - Partial
  - `PreToolUse` hooks can deny Bash before execution and provide a reason, but that is hook input/output, not the same as an `exec --json` event

### Permissions: Can't Write File

- Dedicated `exec --json` event?
  - No dedicated "write denied" event.
- Best available `exec` signals:
  - `file_change` completion with `status: "failed"`
  - `command_execution` failure or decline if the write was attempted via shell
- How to identify the path:
  - For `file_change`, from `changes[].path`
  - For shell-based writes, only from the command string or error text
- Is a reason exposed?
  - Not as a stable dedicated field in the `exec` projection
- Distinguishing from similar events:
  - Weak in `exec --json`
  - A known sharp edge is that file-change `declined` becomes `failed` in the `exec` projection
- Hook exposure:
  - Partial
  - `PreToolUse` can block shell commands with a reason
  - Hooks are not a complete substitute for a stable write-denied event in the `exec` stream

### Tokens Consumed

- Session / turn event:
  - `turn.completed`
- Fields exposed:
  - `usage.input_tokens`
  - `usage.cached_input_tokens`
  - `usage.output_tokens`
- Granularity:
  - Per completed turn
  - No documented session-total aggregate in `exec --json`
- Cost basis:
  - Not exposed
- Hook exposure:
  - No dedicated hook event for token usage

### Model Used

- `exec --json` event?
  - No documented model field in current `exec` JSONL.
- Current public evidence:
  - Open issue requesting model name in the stream:
    - <https://github.com/openai/codex/issues/14736>
- Do model-identifying events always fire?
  - Not in `exec --json`, because there is currently no dedicated event for it.
- Nomenclature:
  - Outside the stream, Codex docs use full model slugs such as `gpt-5.4`, `gpt-5-codex`, and `gpt-5.3-codex-spark`.
- Hook exposure:
  - Yes, indirectly
  - Hook input includes `model` on every hook invocation:
    - <https://developers.openai.com/codex/hooks>
- Stream vs hook parity:
  - Not identical
  - Hooks have model information today; `exec --json` does not

### Human in the Loop

- Can non-interactive runs detect user-question / permission attempts?
  - Not reliably through `codex exec --json` alone.
- What the broader Codex protocol supports:
  - command approval requests
  - file change approval requests
  - `tool/requestUserInput` with a structured list of 1-3 questions and options
- Exact question schema exists here:
  - <https://raw.githubusercontent.com/openai/codex/main/codex-rs/app-server-protocol/schema/json/ToolRequestUserInputParams.json>
- What `exec --json` does in practice:
  - It usually does not surface those approval/request-user-input requests directly.
  - It may instead surface downstream effects such as:
    - failed MCP tool calls
    - `error` / `turn.failed`
    - text like `user cancelled MCP tool call`
- Subagent case:
  - The same limitation applies. The `exec` stream can show `collab_tool_call`, but not a dedicated user-input request event from inside a subagent turn.
- Hook exposure:
  - Partial
  - Hooks can intercept prompt submission, tool use, and stop/continuation, but there is no dedicated hook that mirrors `tool/requestUserInput`

### Injecting into Subagent Prompt

- Is there a direct CLI flag for "append this to every subagent prompt" in `exec` mode?
  - No documented dedicated flag.
- What is exposed structurally?
  - `collab_tool_call` items include a `prompt` field in the exact `exec` schema
  - the broader app-server `collabAgentToolCall` item also includes `prompt`, `model`, and `reasoningEffort`
- Practical ways to influence subagent prompts:
  - custom agent definitions with `developer_instructions`
  - repo instructions such as `AGENTS.md`
  - system skills / config-driven agent definitions
- Official subagent docs:
  - <https://developers.openai.com/codex/subagents>
- Hook exposure:
  - No dedicated hook specifically for subagent-prompt injection
- Stream vs hook parity:
  - The stream can show the prompt that was used for a collab tool call
  - Hooks do not provide an equivalent dedicated subagent-prompt event
