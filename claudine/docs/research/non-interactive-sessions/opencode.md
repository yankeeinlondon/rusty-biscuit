---
schema: https://raw.githubusercontent.com/anomalyco/opencode/dev/packages/sdk/openapi.json
schema_type: open-api
data_format: NDJSON
docs: https://opencode.ai/docs/cli/
last_updated: 2026-04-10
---

# OpenCode Non-Interactive Structured Output

## Summary

As of 2026-04-10, OpenCode's machine-readable non-interactive output is `opencode run --format json`. It emits newline-delimited JSON records to stdout. Each record is a small custom envelope created by the CLI implementation in `packages/opencode/src/cli/cmd/run.ts`, not the raw internal event bus and not a single JSON document.

For Claudine, this stream is useful but incomplete. It is strong for:

- completed text output
- completed or failed tool calls
- step-level token and cost accounting
- terminal provider/session errors

It is weak for:

- parent-session model identity
- permission prompts and user questions as structured stdout events
- a terminal `session.complete` style event
- subagent live activity beyond the final `task` tool result

OpenCode does publish formal schemas for the underlying session/message/event model through its official OpenAPI 3.1.1 spec, generated TypeScript SDK types, and runtime Zod validators. However, it does not publish a formal schema for the exact NDJSON envelope produced by `opencode run --format json`. The best provider-authored formal schema is therefore the OpenAPI spec for the underlying model, not the CLI wrapper itself.

The practical recommendation for Claudine is:

- use `opencode run --format json` when you need a simple subprocess-friendly NDJSON stream
- use OpenCode hooks or the server/SDK event stream when you need human-in-the-loop visibility, raw session lifecycle events, or richer observability

## Schema

### Bottom Line

There is no provider-published JSON Schema, OpenAPI component, or standalone TypeScript type for the exact line format produced by `opencode run --format json`.

What OpenCode does publish officially is a formal schema for the underlying data model that the CLI wraps.

### Best Formal Schema Available

| Artifact | Schema language | URL | Scope |
| --- | --- | --- | --- |
| OpenCode server spec | OpenAPI 3.1.1 | <https://raw.githubusercontent.com/anomalyco/opencode/dev/packages/sdk/openapi.json> | Formal schema for server routes plus session, message, part, question, permission, and error types |
| Generated SDK types | TypeScript | <https://github.com/anomalyco/opencode/blob/dev/packages/sdk/js/src/v2/gen/types.gen.ts> | Generated readable types such as `Part`, `ToolPart`, `StepFinishPart`, `PermissionRequest`, `QuestionRequest`, and `EventSessionError` |
| Runtime validators | TypeScript + Zod | <https://github.com/anomalyco/opencode/blob/dev/packages/opencode/src/session/message-v2.ts> | Runtime source of truth for `Part`, `AssistantMessage`, `ToolState`, `QuestionRequest`, and related data |

The frontmatter `schema` field points to the official OpenAPI document because it is the strongest formal provider-authored schema currently available, even though it does not define the CLI NDJSON envelope exactly.

### Inferred CLI Envelope

The following shape is an inference from the current `run.ts` implementation, not an official provider schema:

```ts
type RunJsonEvent =
  | { type: "tool_use"; timestamp: number; sessionID: string; part: ToolPart }
  | { type: "step_start"; timestamp: number; sessionID: string; part: StepStartPart }
  | { type: "step_finish"; timestamp: number; sessionID: string; part: StepFinishPart }
  | { type: "text"; timestamp: number; sessionID: string; part: TextPart }
  | { type: "reasoning"; timestamp: number; sessionID: string; part: ReasoningPart }
  | {
      type: "error"
      timestamp: number
      sessionID: string
      error:
        | ProviderAuthError
        | UnknownError
        | MessageOutputLengthError
        | MessageAbortedError
        | StructuredOutputError
        | ContextOverflowError
        | ApiError
    }
```

Important implementation details:

- `timestamp` is emitted with `Date.now()`, so it is epoch milliseconds.
- `tool_use` is emitted only after a tool reaches `completed` or `error`.
- `reasoning` is only emitted when `--thinking` is enabled and the reasoning part has finished.
- completion is inferred from process exit and the session reaching `idle`; there is no dedicated completion record on stdout.

### Places Checked

I looked in the following places before concluding that the exact CLI envelope is undocumented:

- OpenCode CLI docs: <https://opencode.ai/docs/cli/>
- OpenCode SDK docs: <https://opencode.ai/docs/sdk/>
- OpenCode server docs: <https://opencode.ai/docs/server/>
- OpenCode plugins docs: <https://opencode.ai/docs/plugins/>
- OpenCode tools docs: <https://opencode.ai/docs/tools/>
- OpenCode official OpenAPI spec: <https://raw.githubusercontent.com/anomalyco/opencode/dev/packages/sdk/openapi.json>
- OpenCode generated TypeScript types: <https://github.com/anomalyco/opencode/blob/dev/packages/sdk/js/src/v2/gen/types.gen.ts>
- OpenCode runtime schema source: <https://github.com/anomalyco/opencode/blob/dev/packages/opencode/src/session/message-v2.ts>
- OpenCode CLI source: <https://github.com/anomalyco/opencode/blob/dev/packages/opencode/src/cli/cmd/run.ts>
- Vercel AI SDK codebase: no formalization found for OpenCode's CLI stream
- LangChain TypeScript codebase: no formalization found for OpenCode's CLI stream

## Documentation

### Official Documentation

| Topic | URL | Notes |
| --- | --- | --- |
| CLI | <https://opencode.ai/docs/cli/> | Documents `opencode run` and `--format default|json` |
| SDK | <https://opencode.ai/docs/sdk/> | Documents SDK structured output via `format: { type: "json_schema", schema }` |
| Server | <https://opencode.ai/docs/server/> | Documents `/event` SSE, `/global/event`, `/doc`, and session/message APIs |
| Plugins | <https://opencode.ai/docs/plugins/> | Documents hook APIs and the raw event taxonomy |
| Tools | <https://opencode.ai/docs/tools/> | Documents most public built-in tools and permission behavior |
| Permissions | <https://opencode.ai/docs/permissions/> | Useful for understanding ask/deny behavior |
| Agents | <https://opencode.ai/docs/agents/> | Useful for subagent definitions and prompt injection via agent files |
| OpenAPI spec | <https://raw.githubusercontent.com/anomalyco/opencode/dev/packages/sdk/openapi.json> | Best formal schema source |
| Generated SDK types | <https://github.com/anomalyco/opencode/blob/dev/packages/sdk/js/src/v2/gen/types.gen.ts> | Best readable type-level source |

### Secondary Documentation

Specific coverage of the `run --format json` stream is still sparse. The most useful non-provider references I found are integration docs written by tools that consume OpenCode as a subprocess:

| Source | URL | Why it matters |
| --- | --- | --- |
| Cub OpenCode harness docs | <https://docs.cub.tools/docs/guide/harnesses/opencode/> | Shows how another tool consumes `step_finish` events for token tracking |
| Harness docs | <https://www.harness.lol/docs> | Describes translating native agent streams, including OpenCode, into a unified NDJSON format |
| Cupcake OpenCode harness docs | <https://cupcake.eqtylab.io/reference/harnesses/opencode/> | Shows another policy-oriented wrapper around OpenCode hooks/events |

### Documentation State

The docs are useful, but they drift:

- the CLI docs describe `json` as "raw JSON events", but the implementation actually emits a filtered custom wrapper
- the SDK docs still show `result.data.info.structured_output`, while the generated types and issue reports show the actual field is `structured`

That drift is important for Claudine because it means code generators and adapters should treat the OpenAPI spec and source as more authoritative than prose examples.

## CLI

### Available Output Formats

For `opencode run`, the current CLI exposes exactly two output formats:

| Format | Meaning | Data returned |
| --- | --- | --- |
| `default` | Human-oriented formatted output | Prose output on stdout plus status/warnings on stderr |
| `json` | Machine-oriented NDJSON | One JSON object per line with `type`, `timestamp`, `sessionID`, and payload |

The relevant syntax is:

```bash
opencode run --format default "Explain async/await in JavaScript"
opencode run --format json "Explain async/await in JavaScript"
```

Other commands also use `--format json`, but those are different output contracts. For example, `opencode session list --format json` returns a normal JSON document, not the NDJSON event stream used by `run`.

### JSON Event Types Currently Emitted

The current `run.ts` implementation emits only these structured stdout event types:

- `tool_use`
- `step_start`
- `step_finish`
- `text`
- `reasoning`
- `error`

### Important Behavior Changes When Switching to `--format json`

| Behavior change | Effect |
| --- | --- |
| Stdout becomes NDJSON | The primary result channel is machine-readable lines rather than final prose |
| The stream is filtered | You do not get the full raw event bus, only the custom subset listed above |
| No terminal completion event | Callers must infer completion from process exit or session idleness |
| Reasoning is opt-in | `reasoning` is only emitted when `--thinking` is enabled |
| Permission handling stays out of stdout | permission prompts are auto-rejected by default, or auto-approved with `--dangerously-skip-permissions`, but no structured permission event is emitted to stdout |
| Top-level questions are disabled | `run` creates the session with `question`, `plan_enter`, and `plan_exit` denied |

### Non-Interactive Permission Side Effects

`opencode run` hardcodes these deny rules when it creates a fresh session:

- `question: deny`
- `plan_enter: deny`
- `plan_exit: deny`

Permission prompts are handled separately:

- if `--dangerously-skip-permissions` is passed, parent-session permission requests are auto-approved once
- otherwise parent-session permission requests are auto-rejected and a warning is printed to stderr

That distinction matters because `question` and plan-approval style interaction are denied up front, while regular tool permissions can still be asked for internally and then auto-handled.

## Gotchas

### No Formal Schema For The Exact CLI Stream

The OpenAPI spec and generated TypeScript types are official and useful, but they stop at the underlying message/event model. If Claudine parses the exact stdout envelope from `opencode run --format json`, it is coding against implementation behavior in `run.ts`.

### "Raw JSON Events" Is Slightly Misleading

The CLI docs say `json` means raw JSON events. In practice, stdout is a filtered wrapper over the bus. Events such as `message.updated`, `session.status`, `permission.asked`, and `question.asked` exist in the official model and hook system but are not forwarded to stdout.

### No `session.complete` Event

Every emitted line includes `sessionID`, but there is still no dedicated terminal record for "this session is now done". Issue `#17221` exists specifically because downstream users want a completion record or a clean final session ID signal for chaining non-interactive runs.

### Tool Start Visibility Is Still Incomplete

Today `tool_use` is emitted after tool completion or error. Open PR `#18249` exists because integrators want `tool_use` events while a tool is still running. Until that lands, JSON mode is weaker for live progress than the hook layer.

### Official Structured-Output Docs Still Drift

Two concrete examples:

- issue `#13342` reported that the docs showed the wrong request field name for SDK structured output
- issue `#14875` reported that docs still referenced `structured_output` while the actual field is `structured`

If Claudine builds adapters automatically, it should prefer the generated types and current source over prose examples.

### Structured Output Implementation Is In Flux

Issue `#15226` documents a real failure mode where `format: { type: "json_schema" }` collided with thinking-enabled models because the server forced `toolChoice: "required"`. Open PR `#18450` proposes moving to native `Output.object()` support. This means SDK structured-output behavior may keep changing even if `run --format json` stays stable.

### Third-Party Harness Docs Can Lag Upstream

Useful integration docs exist, but some are already stale relative to current upstream behavior. For example, Cub's OpenCode page still says:

- OpenCode `run` auto-approves all operations
- runtime model selection is not available via CLI

Current upstream source contradicts both points:

- `run` auto-rejects parent-session permissions unless `--dangerously-skip-permissions` is used
- `run` does support `--model provider/model`

### Social Coverage Is Thin

I did not find strong blog or social-media coverage specifically about the NDJSON shape. Most actionable developer discussion lives in GitHub issues, PRs, and harness documentation rather than in polished articles.

## Timeline

| Date | Event | Why it matters |
| --- | --- | --- |
| 2025-06-29 | PR `#533` proposed an earlier `run --print` mode with `json` and `stream-json` outputs: <https://github.com/anomalyco/opencode/pull/533> | Early upstream evidence that machine-readable non-interactive output was being actively designed |
| 2025-10-31 | PR `#3638` fixed docs/flag alignment for `run --format json`: <https://github.com/anomalyco/opencode/pull/3638> | Earliest clear upstream evidence I found that the current `run --format json` contract was established and documented |
| 2025-12-16 | Issue `#5639` requested structured outputs in the OpenCode SDK: <https://github.com/anomalyco/opencode/issues/5639> | Marks explicit user demand for schema-constrained structured results beyond the CLI NDJSON stream |
| 2026-02-12 | PR `#8161` merged and release `v1.1.60` shipped SDK structured outputs: <https://github.com/anomalyco/opencode/pull/8161>, <https://github.com/anomalyco/opencode/releases/tag/v1.1.60> | Official introduction of Claude Agent SDK-style structured outputs in the SDK layer |
| 2026-02-12 | Issue `#13342` reported docs drift immediately after the structured-output rollout: <https://github.com/anomalyco/opencode/issues/13342> | Shows that integrators had to track naming and request-field changes closely from day one |
| 2026-02-24 | Issue `#14875` reported that docs still used the wrong result field name: <https://github.com/anomalyco/opencode/issues/14875> | Confirms that even after rollout, the documented shape was easy to misread |
| 2026-02-26 | Issue `#15226` documented a structured-output failure with thinking-enabled models: <https://github.com/anomalyco/opencode/issues/15226> | Important operational gotcha for anyone using JSON-schema output in automation |
| 2026-03-12 | Issue `#17221` requested a final session ID / completion signal for `opencode run`: <https://github.com/anomalyco/opencode/issues/17221> | Confirms that the current CLI stream still lacks a clean terminal completion event |
| 2026-03-19 | PR `#18249` proposed emitting running `tool_use` events in JSON mode: <https://github.com/anomalyco/opencode/pull/18249> | Good evidence that current JSON mode is still seen as insufficient for live progress UX |
| 2026-03-20 | PR `#18450` proposed switching structured output to native `Output.object()` support: <https://github.com/anomalyco/opencode/pull/18450> | Signals likely future change in the server/SDK structured-output implementation |
| 2026-04-06 | Release `v1.3.16` fixed output token totals when reasoning tokens are separated: <https://github.com/anomalyco/opencode/releases/tag/v1.3.16> | Important for Claudine if it aggregates token usage from structured records |

## Tools

### Built-In Tools

The public tools docs and the current source do not line up perfectly. Combining the official docs, the tool source files, and `packages/opencode/src/tool/registry.ts`, the built-in tool surface looks like this:

| Tool | Official docs page | Current default registry | Notes |
| --- | --- | --- | --- |
| `bash` | Yes | Yes | Execute shell commands |
| `read` | Yes | Yes | Read files and directories |
| `list` | Yes | No | Publicly documented and implemented in `ls.ts`, but not present in the current default registry list I verified |
| `glob` | Yes | Yes | Pattern-based file search |
| `grep` | Yes | Yes | Regex content search |
| `edit` | Yes | Yes | String-based file edits |
| `write` | Yes | Yes | Create or overwrite files |
| `apply_patch` | Yes | Yes | Patch-based edits |
| `skill` | Yes | Yes | Load a `SKILL.md` into context |
| `todowrite` | Yes | Yes | Update session todos |
| `webfetch` | Yes | Yes | Fetch a specific URL |
| `websearch` | Yes | Yes | Web search, subject to provider/env availability |
| `question` | Yes | Yes | Ask the user questions; enabled conditionally |
| `task` | No | Yes | Spawn or resume a subagent task |
| `codesearch` | No | Yes | Code search tool present in the current registry |
| `lsp` | Yes | Yes | Experimental |
| `plan_exit` | No public page | Yes | Experimental CLI plan-mode tool |
| `invalid` | No | Yes | Internal fallback, not a user-facing tool |

Two caveats:

- `list` is a docs/source mismatch worth watching. It has a public docs page and a source implementation, but `read` also now supports directories, and the verified default registry snapshot does not include `list`.
- `question` is present in the registry, but `opencode run` creates sessions with `question` denied.
- `lsp` and `plan_exit` are gated by experimental flags.

### What The JSON Stream Exposes

For tools, `run --format json` currently emits only post-processed `tool_use` records for completed or failed tools.

| Phase | Stdout JSON | Hook layer | Notes |
| --- | --- | --- | --- |
| Before tool execution | No generic event | `tool.execute.before` | Hooks are the only first-class "before" view |
| While running | Usually no | `message.part.updated`, possibly `tool.execute.before/after` bracketing | PR `#18249` exists to improve this |
| After success | `tool_use` with `part.state.status = "completed"` | `tool.execute.after` plus raw `event` | Stdout is adequate here |
| After failure | `tool_use` with `part.state.status = "error"` | `tool.execute.after`, `permission.asked`, `session.error`, raw `event` | Hooks are richer for why the failure happened |

The important structured payload is inside `part.state`:

- `input`: tool arguments
- `output`: textual tool result for completed tools
- `error`: error string for failed tools
- `title`: human-readable title
- `metadata`: tool-specific structured metadata
- `time.start` and `time.end`: timestamps for duration measurement
- `attachments`: optional attached files or images for completed tools

### Non-Tool Operational Records

`step_start` and `step_finish` are not tool calls, but they are critical to Claudine:

- `step_finish.part.tokens` is the best stdout source for usage
- `step_finish.part.cost` is the best stdout source for spend
- there is no dedicated final aggregate event, so totals should be computed by summing observed `step_finish` records

### Examples

#### Completed File Read

```json
{
  "type": "tool_use",
  "timestamp": 1775490000000,
  "sessionID": "ses_123",
  "part": {
    "type": "tool",
    "tool": "read",
    "state": {
      "status": "completed",
      "input": {
        "filePath": "/repo/src/lib.rs",
        "offset": 1,
        "limit": 200
      },
      "metadata": {
        "preview": "pub fn example() { ... }",
        "truncated": false,
        "loaded": []
      }
    }
  }
}
```

#### Step-Level Accounting

```json
{
  "type": "step_finish",
  "timestamp": 1775490001000,
  "sessionID": "ses_123",
  "part": {
    "type": "step-finish",
    "reason": "tool-calls",
    "cost": 0.00123,
    "tokens": {
      "input": 1024,
      "output": 220,
      "reasoning": 0,
      "cache": {
        "read": 0,
        "write": 0
      }
    }
  }
}
```

#### Failed Write Due To Permissions

```json
{
  "type": "tool_use",
  "timestamp": 1775490002000,
  "sessionID": "ses_123",
  "part": {
    "type": "tool",
    "tool": "write",
    "state": {
      "status": "error",
      "input": {
        "filePath": "/repo/.env",
        "content": "SECRET=..."
      },
      "error": "Tool execution failed: The user has specified a rule which prevents you from using this specific tool call."
    }
  }
}
```

#### Completed Subagent Task

`task` is especially important because it is the only built-in tool that surfaces some child-session metadata in the parent stream. The tool implementation attaches:

- `metadata.sessionId`
- `metadata.model.providerID`
- `metadata.model.modelID`

That is enough to learn that a subagent session existed, but not enough to observe the child session live.

## Use Cases

### Plan Cap Approaching

- `run --format json` does not expose a dedicated "plan cap approaching" event.
- The closest raw structured signal is `session.status` with `status.type = "retry"` and fields such as `attempt`, `message`, and `next`, but `run.ts` does not forward `session.status` to stdout JSON.
- In practice, any "approaching cap" detection would have to pattern-match provider-specific text in a retry message or error message.
- The remaining allowance is not exposed as a percentage, token count, or dollar amount in a normalized field.
- The reset window is not exposed as a billing-cap reset time. `session.status.retry.next` is just a retry timestamp, not a plan-window reset timestamp.
- Hook exposure: yes, partially, through the `event` hook on `session.status` and `session.error`.
- Stream vs hook parity: not identical. The hook layer is richer because stdout omits `session.status` entirely.

### Plan Capped

- The best stdout signal today is usually an `error` record.
- That `error` record wraps the raw `session.error` payload, typically an `APIError` or `ProviderAuthError`.
- Distinguish it by inspecting `error.name`, `error.data.message`, and, when present, `error.data.responseBody` or `error.data.metadata`.
- Remaining allowance is not exposed in a normalized field.
- Reset-window timing is not exposed in a normalized field.
- Hook exposure: yes, through the `event` hook on `session.error`.
- Stream vs hook parity: close but not identical. The hook receives the raw `session.error` event; stdout receives the CLI envelope.

### No Funds

- There is no dedicated "no funds" event type.
- The likely stdout signal is `error` wrapping a provider-specific `APIError`.
- Distinguish it using `error.name === "APIError"` plus provider-specific message text or response body.
- Hook exposure: yes, through `session.error`.
- Stream vs hook parity: close but not identical for the same reason as above.

### Auth

- Authentication failures surface as `error` records when they are terminal.
- The strongest raw type is `ProviderAuthError`, which includes `providerID` and a message.
- `run --format json` does not reveal what kind of auth the user used, such as API key versus subscription versus OAuth-backed console login.
- Hook exposure: yes, through `session.error`.
- Stream vs hook parity: the signal is similar, but neither the stream nor the hook exposes the auth method kind as a first-class field.

### Permissions: Can't Read File

- The normal stdout signal is `tool_use` where `part.tool === "read"` and `part.state.status === "error"`.
- The attempted file path is available in `part.state.input.filePath`.
- A reason may be available in `part.state.error`, but it is usually just a tool-level failure string, not a rich permission object.
- The best way to distinguish it from other read failures is the combination of tool name, status, attempted path, and error string.
- Hook exposure: yes.
- Relevant hook/event signals: `permission.asked`, `permission.replied`, `tool.execute.before`, `tool.execute.after`, and the general `event` hook.
- Stream vs hook parity: not identical. Hooks are richer because they can expose the permission request before denial; stdout only shows the final failed tool result, and only for the parent session.

### Permissions: Can't Write File

- The normal stdout signal is `tool_use` with `part.tool` equal to `write`, `edit`, or `apply_patch`, plus `part.state.status === "error"`.
- For `write` and `edit`, the path is usually available in `part.state.input.filePath`.
- For `apply_patch`, the path may be embedded inside `patchText`; hook-side permission metadata is a better source of per-file detail.
- A reason may be available via `part.state.error`, but the richer cause chain normally lives in the permission request or hook context, not stdout.
- Hook exposure: yes.
- Relevant hook/event signals: `permission.asked`, `permission.replied`, `tool.execute.before`, `tool.execute.after`, and raw bus events.
- Stream vs hook parity: not identical. Hooks expose more path and diff detail than stdout does.

### Tokens Consumed

- The best stdout event is `step_finish`.
- `step_finish.part.tokens` gives step-level input, output, reasoning, and cache token counts.
- `step_finish.part.cost` gives step-level cost.
- There is no dedicated final session-total event in the `run --format json` stream.
- The recommended strategy is to sum all observed `step_finish` records for the session.
- Hook exposure: yes.
- Relevant hook/event signals: `message.part.updated` for `step-finish` parts, and `message.updated` for final assistant-message totals.
- Stream vs hook parity: not identical. Hooks are richer because `message.updated` carries final assistant-level `tokens` and `cost`, which stdout omits.

### Model Used

- There is no dedicated parent-session model event in stdout JSON.
- In `default` mode, OpenCode prints `> agent · modelID` to stderr when the first assistant message arrives, but that is not structured stdout data.
- The one reliable model signal inside stdout JSON is the completed `task` tool result, which can include child-session model metadata.
- Hook exposure: yes.
- Relevant hook/event signals: `message.updated` for assistant messages, which include `providerID` and `modelID`, and user messages, which include `model.providerID` and `model.modelID`.
- Stream vs hook parity: not identical. Hooks are clearly better than stdout for model identification.

### Human In The Loop

- Top-level `opencode run` explicitly creates sessions with `question`, `plan_enter`, and `plan_exit` denied.
- Permission prompts can still happen internally, but `run --format json` does not emit structured permission events on stdout.
- For the parent session, permission prompts are handled side-effectfully: auto-approved once with `--dangerously-skip-permissions`, otherwise auto-rejected with a stderr warning.
- `question.asked` is part of the raw event model, but `run.ts` does not forward it to stdout JSON.
- Subagent attempts to ask questions or request permissions are also not surfaced cleanly on stdout.
- Hook exposure: yes.
- Relevant hook/event signals: `question.asked`, `question.replied`, `question.rejected`, `permission.asked`, and `permission.replied`.
- Stream vs hook parity: not close. The hook/event layer is the only reliable structured source for HITL attempts.

### Injecting Into Subagent Prompt

- There is no dedicated `opencode run` flag or stdout event for "append this context to every subagent prompt".
- The parent model can place extra text into the `task` tool's `prompt` argument, but that is model-driven, not caller-controlled by a dedicated runtime flag.
- Persistent subagent prompt injection is supported through agent definitions in `opencode.json` or `.opencode/agents/*.md`.
- Broader context rewriting is possible through plugins and hook-based prompt transformation, not through the `run --format json` stream.
- Hook exposure: this is not an event-driven signal, so there is no equivalent hook event to compare.
- Stream vs hook parity: not applicable. This capability exists through configuration and hooks, not through the stdout NDJSON stream.
