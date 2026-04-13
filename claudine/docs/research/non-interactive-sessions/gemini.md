---
schema: https://raw.githubusercontent.com/google-gemini/gemini-cli/main/packages/core/src/output/types.ts
schema_type: typescript
data_format: JSON and JSONL
docs: https://geminicli.com/docs/cli/headless/
created: 2026-04-06
last_updated: 2026-04-06
---

# Gemini CLI: Non-Interactive Structured Output

## Summary

Gemini CLI currently has two structured output modes for non-interactive runs:
`--output-format json` and `--output-format stream-json`.

`json` returns one final JSON object. `stream-json` returns newline-delimited
JSON events in real time. The most exact provider-authored schema for both
surfaces is the TypeScript source in the Gemini CLI repository, especially
[`packages/core/src/output/types.ts`](https://github.com/google-gemini/gemini-cli/blob/main/packages/core/src/output/types.ts)
and
[`packages/core/src/output/stream-json-formatter.ts`](https://github.com/google-gemini/gemini-cli/blob/main/packages/core/src/output/stream-json-formatter.ts).

The practical state of the surface as of 2026-04-06 is:

- `stream-json` is the richer machine-consumable surface for Claudine.
- Gemini CLI still does not publish a dedicated JSON Schema or OpenAPI spec for
  the headless output stream.
- The headless stream intentionally drops several internal agent events,
  including `usage`, `tool_update`, `session_update`,
  `elicitation_request`, and `elicitation_response`.
- The stream is strong for session IDs, message chunks, tool lifecycle,
  aggregate token usage, and final success or failure.
- The stream is weak for auth detection, plan-cap telemetry, cost basis, and
  human-in-the-loop prompts in headless mode.

## Schema

### Best Exact Schema

The best exact schema for Gemini CLI's non-interactive structured output is the
provider-authored TypeScript in the main repository:

- Output type definitions:
  - <https://raw.githubusercontent.com/google-gemini/gemini-cli/main/packages/core/src/output/types.ts>
- Streaming formatter:
  - <https://raw.githubusercontent.com/google-gemini/gemini-cli/main/packages/core/src/output/stream-json-formatter.ts>
- Final JSON formatter:
  - <https://raw.githubusercontent.com/google-gemini/gemini-cli/main/packages/core/src/output/json-formatter.ts>
- Headless runtime that emits the events:
  - <https://github.com/google-gemini/gemini-cli/blob/main/packages/cli/src/nonInteractiveCliAgentSession.ts>

Schema language:

- TypeScript enums and interfaces

### Structured Output Shapes

Gemini CLI exposes two provider-defined structured shapes in headless mode.

#### `--output-format json`

The single JSON object currently has these top-level fields:

| Field | Type | Notes |
| --- | --- | --- |
| `session_id` | string, optional | Added to JSON output in December 2025. |
| `response` | string, optional | Final assistant text. |
| `stats` | object, optional | Session metrics object. |
| `error` | object, optional | Error object with `type`, `message`, and optional `code`. |

Important note:

- The official headless docs summarize `response`, `stats`, and `error`, but
  the current TypeScript source also includes `session_id`.

#### `--output-format stream-json`

The streaming JSONL union currently contains these event types:

| Event type | Purpose | Key fields |
| --- | --- | --- |
| `init` | Session metadata | `timestamp`, `session_id`, `model` |
| `message` | User or assistant content | `timestamp`, `role`, `content`, optional `delta` |
| `tool_use` | Tool request | `timestamp`, `tool_name`, `tool_id`, `parameters` |
| `tool_result` | Tool completion | `timestamp`, `tool_id`, `status`, optional `output`, optional `error` |
| `error` | Non-fatal warning or runtime error | `timestamp`, `severity`, `message` |
| `result` | Final outcome | `timestamp`, `status`, optional `error`, optional `stats` |

The final `result.stats` object currently includes:

| Field | Type | Notes |
| --- | --- | --- |
| `total_tokens` | number | Aggregate across all models used in the run |
| `input_tokens` | number | Prompt-side total before cache subtraction |
| `output_tokens` | number | Output tokens |
| `cached` | number | Cached prompt tokens |
| `input` | number | Non-cached prompt tokens |
| `duration_ms` | number | Wall-clock session duration |
| `tool_calls` | number | Total completed tool calls |
| `models` | object | Per-model token breakdown, added in March 2026 |

### Formal Schema Availability

I did not find a provider-published JSON Schema, OpenAPI document, or other
machine-readable formal schema dedicated to Gemini CLI headless output.

Places checked:

- Official docs:
  - <https://geminicli.com/docs/cli/headless/>
  - <https://geminicli.com/docs/cli/tutorials/automation/>
  - <https://geminicli.com/docs/reference/configuration/>
- Official repository:
  - `packages/core/src/output/`
  - `packages/cli/src/nonInteractiveCliAgentSession.ts`
  - `schemas/`
- Hosted schemas directory in the official repo:
  - <https://github.com/google-gemini/gemini-cli/tree/main/schemas>
- Broader ecosystem checks:
  - Vercel AI SDK
  - LangChain JS / TypeScript
  - General web searches for standalone Gemini CLI stream schemas

Result:

- The repository's `schemas/` directory currently contains
  `settings.schema.json`, but nothing for headless output.
- I did not find a respected third-party project that has become the de facto
  formal schema for Gemini CLI `json` or `stream-json`.

## Documentation

### Official Documentation

- Headless mode reference:
  - <https://geminicli.com/docs/cli/headless/>
- Automation tutorial:
  - <https://geminicli.com/docs/cli/tutorials/automation/>
- CLI reference:
  - <https://geminicli.com/docs/cli/cli-reference/>
- Configuration reference:
  - <https://geminicli.com/docs/reference/configuration/>
- Tools reference:
  - <https://geminicli.com/docs/reference/tools/>
- Hooks overview:
  - <https://geminicli.com/docs/hooks/>
- Hooks reference:
  - <https://geminicli.com/docs/hooks/reference/>
- Session management:
  - <https://geminicli.com/docs/cli/session-management/>
- Subagents:
  - <https://geminicli.com/docs/core/subagents/>
- Quota and pricing:
  - <https://geminicli.com/docs/resources/quota-and-pricing/>

### Official and Semi-Official Articles / Leverage References

- GitHub Action for CI and automation:
  - <https://github.com/google-github-actions/run-gemini-cli>
  - Useful because it shows how Google expects Gemini CLI to be used in
    unattended workflows.
- Changelog index:
  - <https://geminicli.com/docs/changelogs/>
  - Useful for structured-output timeline changes.

### Community / Ecosystem References

There are not many good third-party articles focused specifically on the
headless JSON schema. Most of the useful ecosystem writing focuses on how to
leverage Gemini CLI in scripts rather than documenting the event grammar.

Useful secondary references:

- DEV Community overview that explicitly mentions `--output-format json` for
  scripting:
  - <https://dev.to/deployhq/getting-started-with-google-gemini-cli-open-source-ai-agent-for-your-terminal-25e1>
- Medium getting-started article:
  - <https://medium.com/@pieczynski.pawel/gemini-cli-quest-log-ddd9c619d2eb>
- The New Stack coverage of Gemini CLI GitHub Actions:
  - <https://thenewstack.io/googles-gemini-cli-agent-comes-to-github/>

## CLI

### Output Formats

Gemini CLI currently exposes these non-interactive output formats:

| Format | CLI value | What you get |
| --- | --- | --- |
| Plain text | `text` | Human-readable final response on stdout |
| Final JSON object | `json` | One JSON object with final response, session stats, optional error, and current `session_id` |
| Streaming JSONL | `stream-json` | Real-time NDJSON/JSONL event stream with init, messages, tool lifecycle, warnings, and final result |

### Syntax

Common headless invocations:

```bash
gemini -p "Summarize this repository"
gemini --output-format json -p "Return a JSON object from @package.json"
gemini --output-format stream-json -p "Explain this codebase"
cat diff.txt | gemini --output-format stream-json -p "Review this diff"
gemini --resume a1b2c3d4-e5f6-7890-abcd-ef1234567890 --output-format stream-json -p "Continue"
```

### Output-Mode Side Effects

- `text` prints human-readable output only.
- `json` delays machine-readable output until the run completes.
- `stream-json` changes stdout into a JSONL event stream; consumers must parse
  one JSON object per line.
- `stream-json` is the only mode that exposes tool lifecycle and incremental
  assistant output.
- In `stream-json`, assistant text arrives as repeated `message` events with
  `delta: true`.
- In `json`, the final `response` is aggregated text only. Intermediate tool
  steps are not preserved.
- `--resume` works with structured output modes, and `session_id` is part of
  both the JSON object and the stream `init` event.

### Persistent Config Gotcha

There is a notable split between CLI flags and settings:

- The command-line flag `--output-format` accepts `text`, `json`, and
  `stream-json`.
- The documented `output.format` setting currently only exposes `text` and
  `json`.

Practical implication:

- If Claudine wants `stream-json`, it should pass the CLI flag explicitly
  instead of relying on persistent settings.

## Gotchas

### 1. There Is No Dedicated JSON Schema for the Headless Output

The current source of truth is TypeScript, not JSON Schema or OpenAPI. This
means Claudine should treat the provider's TS source as canonical and expect the
shape to evolve with releases.

### 2. `stream-json` Is a Projection of Internal Agent Events, Not the Whole Event Bus

The internal agent protocol has more event types than the headless stream.
`nonInteractiveCliAgentSession.ts` explicitly ignores:

- `initialize`
- `session_update`
- `agent_start`
- `tool_update`
- `elicitation_request`
- `elicitation_response`
- `usage`
- `custom`

Implication:

- Claudine cannot rely on `stream-json` for every internal lifecycle detail.

### 3. `init.model` Is Not Always the Best Evidence of the Actual Backend Model

The `init` event uses `config.getModel()`. In practice this can be:

- an alias such as `auto`, `pro`, or `flash`
- a configured model name

The more concrete evidence of what was actually used is often in
`result.stats.models`, which records per-model token usage and can show routing
or fallback behavior.

### 4. Cancelled Tool Calls Can Still Look Like Success

Gemini CLI currently preserves a legacy behavior where cancelled tool calls can
still surface as `tool_result.status: "success"` in `stream-json`.

Implication:

- Treat `tool_result.status` as necessary but not always sufficient.
- Correlate with `output` and surrounding error messages.

### 5. Human-Input Paths Are Flattened or Suppressed in Headless Mode

`ask_user` is an interactive tool, but policy handling in non-interactive mode
treats `ask_user` as `deny`. Internal `elicitation_request` events also do not
appear in the stream.

Implication:

- Claudine should assume that many human-in-the-loop situations appear only as
  denials, fatal errors, or missing progress, not as first-class question
  objects in `stream-json`.

### 6. The Headless Surface Has Changed Quickly

Important changes landed across:

- September 2025: initial JSON mode
- October 2025: `stream-json`
- December 2025: `session_id` in JSON output
- March 2026: per-model token stats in stream results

Implication:

- Version pinning matters for automation.
- Claudine should record the Gemini CLI version alongside parsed sessions.

## Timeline

| Date | Event | Why it matters |
| --- | --- | --- |
| 2025-09-11 | Commit [`514767c88`](https://github.com/google-gemini/gemini-cli/commit/514767c88b27f1e2c4e072fd87bd9a4022a8014a) added structured JSON output | First provider-authored JSON output implementation |
| 2025-09-19 | Release `v0.0.77` | First tagged release containing JSON output |
| 2025-10-15 | Commit [`47f693173`](https://github.com/google-gemini/gemini-cli/commit/47f693173ab7aab35376c656f12695b9cde31c51) added `--output-format stream-json` | Introduced the real-time JSONL stream |
| 2025-10-29 | Release `v0.11.0` | First stable tagged release containing `stream-json` |
| 2025-12-04 | Commit [`8b0a8f47c`](https://github.com/google-gemini/gemini-cli/commit/8b0a8f47c1b2324db51d566ca93f600be3d9f419) added `session_id` to JSON output | Important for resume and cross-system correlation |
| 2025-12-16 | Release `v0.21.0` | First stable tagged release with JSON `session_id` |
| 2026-02-09 | Changelog added broader quota visibility in `/stats` | Important context because cap and quota telemetry still mostly lives outside the headless stream |
| 2026-03-10 | Commit [`4da0366ee`](https://github.com/google-gemini/gemini-cli/commit/4da0366eed481a4e81c3d6eb6ee5aec061e77c8a) added per-model token usage to stream results | Made `result.stats.models` useful for real model attribution |
| 2026-03-17 | Release `v0.34.0` | First stable tagged release with per-model stream stats |
| 2026-04-01 | Release `v0.36.0` | Current stable release reported by npm as of 2026-04-06 |

## Tools

### Built-In Core Tools

Gemini CLI's built-in core tools are documented here:

- <https://geminicli.com/docs/reference/tools/>

The built-in tools that matter most for headless structured output are:

| Tool name | Category | Stream visibility |
| --- | --- | --- |
| `run_shell_command` | Execute | `tool_use.parameters.command`, then `tool_result` |
| `glob` | Search | `tool_use.parameters`, then `tool_result` |
| `grep_search` | Search | `tool_use.parameters`, then `tool_result` |
| `list_directory` | Read | `tool_use.parameters.dir_path`, then `tool_result` |
| `read_file` | Read | `tool_use.parameters.file_path`, then `tool_result` |
| `read_many_files` | Read | `tool_use.parameters`, then `tool_result` |
| `replace` | Edit | `tool_use.parameters.file_path`, then `tool_result` |
| `write_file` | Edit | `tool_use.parameters.file_path`, then `tool_result` |
| `ask_user` | Communicate | Usually absent in headless runs because non-interactive policy treats it as deny |
| `write_todos` | Other | Visible as normal tool lifecycle if used |
| `activate_skill` | Other | Visible as normal tool lifecycle if used |
| `get_internal_docs` | Think | Visible as normal tool lifecycle if used |
| `enter_plan_mode` | Plan | Visible if Plan Mode is entered during headless execution |
| `exit_plan_mode` | Plan | Visible if Plan Mode is exited during headless execution |
| `google_web_search` | Search | Visible as normal tool lifecycle |
| `web_fetch` | Fetch | Visible as normal tool lifecycle |

Built-in subagents are a separate category:

- <https://geminicli.com/docs/core/subagents/>

They are exposed to the main agent as tools of the same name, but the
non-interactive JSON stream does not expand their internal activity into a
separate nested schema. Claudine should expect to see the outer subagent
invocation, not the full subagent event bus.

### Before / After Metadata

Before a tool runs, `stream-json` gives:

- `tool_name`
- `tool_id`
- raw `parameters`

After a tool finishes, `stream-json` gives:

- the same `tool_id` for correlation
- `status`
- optional `output`
- optional `error.type`
- optional `error.message`

Important limitation:

- `tool_result` does not repeat `tool_name`. Claudine must join on `tool_id`.

### Examples

#### Read File Success

Before:

```json
{"type":"tool_use","timestamp":"2026-04-06T00:00:00.000Z","tool_name":"read_file","tool_id":"tool-1","parameters":{"file_path":"/repo/src/lib.rs"}}
```

After:

```json
{"type":"tool_result","timestamp":"2026-04-06T00:00:01.000Z","tool_id":"tool-1","status":"success","output":"Read 180 lines from /repo/src/lib.rs"}
```

#### Write Blocked by Workspace Boundary

Before:

```json
{"type":"tool_use","timestamp":"2026-04-06T00:00:00.000Z","tool_name":"write_file","tool_id":"tool-2","parameters":{"file_path":"/etc/passwd","content":"..."}} 
```

After:

```json
{"type":"tool_result","timestamp":"2026-04-06T00:00:01.000Z","tool_id":"tool-2","status":"error","error":{"type":"path_not_in_workspace","message":"The path is outside the workspace root."}}
```

#### Shell Command

Before:

```json
{"type":"tool_use","timestamp":"2026-04-06T00:00:00.000Z","tool_name":"run_shell_command","tool_id":"tool-3","parameters":{"command":"cargo test -p my-crate","description":"Run focused tests"}} 
```

After:

```json
{"type":"tool_result","timestamp":"2026-04-06T00:00:03.000Z","tool_id":"tool-3","status":"success","output":"running 12 tests\n..."}
```

## Use Cases

### Plan Cap Approaching

- Event type:
  - No dedicated `json` or `stream-json` event.
- Distinguishing signal:
  - Not available as a stable structured field in headless output.
- Remaining amount:
  - Not exposed.
- Reset window:
  - Not exposed.
- Hook exposure:
  - No dedicated hook. The public `Notification` hook only documents
    `ToolPermission`, not quota or plan-cap warnings.

### Plan Capped

- Event type:
  - Usually a final `result` event with `status: "error"`.
- Distinguishing signal:
  - Best-effort only. Inference comes from `result.error.type` and
    `result.error.message`.
  - When quota errors are classified upstream, the type may be
    `TerminalQuotaError` or `RetryableQuotaError`.
- Remaining amount:
  - Not exposed.
- Reset window:
  - Not exposed as a dedicated field. It may only appear as human-readable text
    in the message when a retry delay is known.
- Hook exposure:
  - No dedicated cap hook. `SessionEnd` can fire afterward, but it does not
    carry quota metadata.

### No Funds

- Event type:
  - Usually a final `result` event with `status: "error"`.
- Distinguishing signal:
  - The internal quota classifier distinguishes
    `INSUFFICIENT_G1_CREDITS_BALANCE`, but that reason is not emitted as a
    first-class field in the headless stream.
  - In practice, detection is message-text based, not schema-stable.
- Hook exposure:
  - No dedicated hook. Interactive overage and empty-wallet flows are UI-level
    features, not headless stream events.

### Auth

- Selected auth type in structured output:
  - Not exposed in `json` or `stream-json`.
- What is available instead:
  - Local configuration has `security.auth.selectedType`, but that is a config
    file setting, not a runtime stream field.
  - Failures may surface as `FatalAuthenticationError` or generic API errors.
- Reliable auth-kind detection:
  - Not from the structured output alone.
- Hook exposure:
  - No dedicated auth hook. Hook payloads do not include selected auth type.

### Permissions: Can't Read File

- Event type:
  - `tool_use` followed by `tool_result.status: "error"`.
- Relevant tools:
  - `read_file`, `read_many_files`, `list_directory`, `glob`, `grep_search`
- How to identify the path:
  - Read it from the earlier `tool_use.parameters`, such as `file_path`,
    `dir_path`, or `path`.
- Reason visibility:
  - `tool_result.error.type` and `tool_result.error.message`.
  - Common values include `permission_denied` and `path_not_in_workspace`.
- Distinguishing from similar failures:
  - Use the combination of `tool_name` plus `error.type`.
- Hook exposure:
  - Yes. `BeforeTool` and `AfterTool` expose `tool_name`, full `tool_input`,
    and post-tool `tool_response`.
  - Difference from stream:
    - Hooks do not use `tool_id`.
    - Hooks give richer pre/post interception semantics than the flat stream.

### Permissions: Can't Write File

- Event type:
  - `tool_use` followed by `tool_result.status: "error"`.
- Relevant tools:
  - `write_file`, `replace`
- How to identify the path:
  - `tool_use.parameters.file_path`
- Reason visibility:
  - `tool_result.error.type` and `tool_result.error.message`
  - Common values include `permission_denied`, `path_not_in_workspace`, and
    `file_write_failure`
- Distinguishing from similar failures:
  - Use the write-tool name plus the error type.
- Hook exposure:
  - Yes, through `BeforeTool` and `AfterTool`.
  - Difference from stream:
    - Hooks can deny or rewrite the call before execution.
    - Stream output only reports what was attempted and what came back.

### Tokens Consumed

- Session-level usage event:
  - `result.stats`
- Granularity:
  - Session-level only in `stream-json`
  - More granular internal `usage` events exist in the agent protocol, but the
    headless formatter explicitly ignores them.
- Cost basis:
  - No price or USD field is exposed.
- Hook exposure:
  - No dedicated usage hook. `SessionEnd` does not receive token counters.

### Model Used

- Structured events:
  - `init.model`
  - `result.stats.models`
- Do these always fire:
  - `init` is emitted very early in normal `stream-json` runs.
  - In catastrophic startup failures before the headless session initializes,
    you cannot assume the stream will start cleanly.
- Naming form:
  - `init.model` is whatever the CLI config says to use, which may be an alias
    like `auto`, `pro`, or `flash`.
  - `result.stats.models` is stronger evidence of the concrete backend models
    that actually consumed tokens.
- Provider visibility:
  - The provider is implied to be Gemini / Google, but no separate provider
    field is emitted.
- Hook exposure:
  - Yes, indirectly. `BeforeModel` receives `llm_request.model`, which is richer
    for per-call inspection than the flat stream.

### Human in the Loop

- Can we detect prompts for user input or permissions in a headless stream:
  - Not reliably.
- Why:
  - Internal `elicitation_request` and `elicitation_response` events are
    explicitly ignored by the headless formatter.
  - Non-interactive policy handling treats `ask_user` as `deny`.
  - Slash commands that require confirmation abort instead of surfacing a
    structured question object.
- Subagent case:
  - Not reliably from `stream-json`.
  - Subagents have internal activity types, but the headless output does not
    expose a dedicated nested subagent prompt protocol.
- Hook exposure:
  - Partial only.
  - `Notification` can observe documented tool-permission alerts in interactive
    mode, but it is not a substitute for a structured headless question stream.

### Injecting into Subagent Prompt

- Supported:
  - Yes, through subagent definition files in `.gemini/agents/*.md` or
    `~/.gemini/agents/*.md`.
  - The body of the Markdown file becomes the subagent's system prompt.
- Also true by default:
  - Gemini CLI already appends a standard non-interactive rule block to
    subagent prompts, including an instruction that they cannot ask the user
    for input or clarification.
- Not found:
  - I did not find a dedicated `stream-json` field or a dedicated public hook
    specifically for mutating all subagent prompts globally at runtime.
- Hook exposure:
  - No dedicated hook event for "subagent prompt injected".
  - The closest adjacent surface is `BeforeAgent` additional context, but the
    public docs do not document it as a universal subagent-prompt override.
