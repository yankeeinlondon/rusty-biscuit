---
schema: ""
schema_type: ""
data_format: ndjson
docs: https://opencode.ai/docs/cli/
last_updated: 2026-04-06
---

# OpenCode CLI Structured Output in Non-Interactive Sessions

## Summary

As of 2026-04-06, OpenCode's non-interactive structured output is the `opencode run --format json` mode documented at <https://opencode.ai/docs/cli/> and implemented in [`packages/opencode/src/cli/cmd/run.ts`](https://github.com/anomalyco/opencode/blob/dev/packages/opencode/src/cli/cmd/run.ts). It writes one JSON object per line to stdout, so the wire format is newline-delimited JSON rather than a single JSON document.

The stream is useful, but lightly specified. OpenCode does publish formal schemas for the underlying session/message/event model through an official OpenAPI 3.1.1 document and generated TypeScript types, but it does **not** publish a formal schema for the exact `opencode run --format json` envelope itself. The current CLI implementation emits a narrow subset of session activity as custom event records: `tool_use`, `step_start`, `step_finish`, `text`, `reasoning`, and `error`. That is enough for automation, but it is notably less expressive than the raw bus event system available to plugins.

For automation, the practical split is:

- Use `step_finish` to collect token and cost data.
- Use `tool_use` to inspect completed or failed tool calls.
- Use plugin hooks or the SDK event stream, not `opencode run --format json`, when you need model identity, permission prompts, user questions, or raw session lifecycle events.

The biggest gaps today are the lack of a formal CLI stream schema, the lack of a dedicated session-complete event, and the fact that important human-in-the-loop signals are available to hooks but not surfaced in the CLI JSON stream.

## Schema

### Bottom line

There does not appear to be a provider-published formal schema for the **exact** output of `opencode run --format json`.

What OpenCode **does** publish officially is:

| Artifact | Schema language | URL | What it covers |
| --- | --- | --- | --- |
| Server API spec | OpenAPI 3.1.1 | <https://raw.githubusercontent.com/anomalyco/opencode/dev/packages/sdk/openapi.json> | Official schema for server routes, raw bus events, messages, parts, permission requests, question requests, and related types |
| Generated SDK types | TypeScript | <https://github.com/anomalyco/opencode/blob/dev/packages/sdk/js/src/v2/gen/types.gen.ts> | Generated client types such as `Part`, `ToolPart`, `StepFinishPart`, `EventSessionError`, `PermissionRequest`, and `QuestionRequest` |
| Internal source-of-truth validators | TypeScript + Zod | <https://github.com/anomalyco/opencode/blob/dev/packages/opencode/src/session/message-v2.ts> | The runtime Zod definitions from which many message/part shapes are derived |

Those official schemas are extremely useful because the CLI JSON stream embeds official `part` and `error` objects inside its envelope. The problem is that the **envelope** is only described by implementation code in `run.ts`, not by OpenAPI, JSON Schema, or a published TypeScript type.

### Best available formal schema

The best official formal schema I found is OpenCode's OpenAPI 3.1.1 document:

- URL: <https://raw.githubusercontent.com/anomalyco/opencode/dev/packages/sdk/openapi.json>
- Schema language: `open-api`
- Scope: underlying OpenCode session/message/event model, **not** the exact `opencode run --format json` line format

Important relevant components inside that spec include:

- `Part`
- `ToolPart`
- `StepStartPart`
- `StepFinishPart`
- `SnapshotPart`
- `Event.message.part.updated`
- `Event.session.error`
- `Event.permission.asked`
- `Event.question.asked`

### Informal schema for the CLI NDJSON stream

The following TypeScript-style shape is an **inference** from the current CLI implementation, not an official provider schema:

```ts
type RunJsonEvent =
  | {
      type: "tool_use"
      timestamp: number
      sessionID: string
      part: ToolPart
    }
  | {
      type: "step_start"
      timestamp: number
      sessionID: string
      part: StepStartPart
    }
  | {
      type: "step_finish"
      timestamp: number
      sessionID: string
      part: StepFinishPart
    }
  | {
      type: "text"
      timestamp: number
      sessionID: string
      part: TextPart
    }
  | {
      type: "reasoning"
      timestamp: number
      sessionID: string
      part: ReasoningPart
    }
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

Important implementation details from the current source:

- `timestamp` is produced with `Date.now()`, so it is an epoch-millisecond number.
- `tool_use` is emitted only when a tool part reaches `completed` or `error`.
- `reasoning` is emitted only when `--thinking` is enabled and the reasoning part is complete.
- There is no `session.complete` or equivalent terminal JSON event in the current source.

### Places checked for a formal schema

I looked in all of the following places before concluding that the exact CLI envelope is undocumented:

- Official CLI docs: <https://opencode.ai/docs/cli/>
- Official SDK docs: <https://opencode.ai/docs/sdk/>
- Official server docs: <https://opencode.ai/docs/server/>
- Official plugins docs: <https://opencode.ai/docs/plugins/>
- Official tools docs: <https://opencode.ai/docs/tools/>
- Official repo OpenAPI spec: <https://raw.githubusercontent.com/anomalyco/opencode/dev/packages/sdk/openapi.json>
- Official generated TypeScript types: <https://github.com/anomalyco/opencode/blob/dev/packages/sdk/js/src/v2/gen/types.gen.ts>
- Internal runtime schema source: <https://github.com/anomalyco/opencode/blob/dev/packages/opencode/src/session/message-v2.ts>
- Current CLI implementation: <https://github.com/anomalyco/opencode/blob/dev/packages/opencode/src/cli/cmd/run.ts>
- Community harness docs that consume this stream, including Cub and Harness

The result is clear: OpenCode formally specifies the **embedded data model**, but not the exact `run --format json` NDJSON wrapper.

## Documentation

### Official documentation

| Topic | URL | Notes |
| --- | --- | --- |
| CLI non-interactive mode | <https://opencode.ai/docs/cli/> | Documents `opencode run` and the `--format` flag |
| SDK structured output | <https://opencode.ai/docs/sdk/> | Documents `format: { type: "json_schema", schema, retryCount }` for `session.prompt()` |
| Plugin events and hooks | <https://opencode.ai/docs/plugins/> | Documents raw event names such as `session.error`, `permission.asked`, and `question.asked` plus `tool.execute.before/after` |
| Built-in tools | <https://opencode.ai/docs/tools/> | Documents most public built-ins and their permission behavior |
| Server routes | <https://opencode.ai/docs/server/> | Useful because the published SDK/server schemas are closer to the raw internal model than the CLI docs are |
| OpenAPI spec | <https://raw.githubusercontent.com/anomalyco/opencode/dev/packages/sdk/openapi.json> | Best formal source for event/message/part shapes |
| Generated TypeScript types | <https://github.com/anomalyco/opencode/blob/dev/packages/sdk/js/src/v2/gen/types.gen.ts> | Best readable source for payload shapes without reading raw OpenAPI |

### Third-party documentation and articles

Long-form blog coverage of OpenCode's JSON stream is still sparse. The most useful third-party writeups I found were documentation from tools that integrate OpenCode as a subprocess:

| Source | URL | Why it matters |
| --- | --- | --- |
| Cub harness docs | <https://docs.cub.tools/docs/guide/harnesses/opencode/> | Explains how another tool consumes `opencode run --format json`, especially `step_finish` for token accounting |
| Harness docs | <https://www.harness.lol/docs> | Describes wrapping OpenCode's native stream into a provider-agnostic NDJSON format |
| Cupcake OpenCode reference | <https://cupcake.eqtylab.io/reference/harnesses/opencode/> | Documents how OpenCode plugin hooks are mapped into a policy harness |

These are valuable integration references, but they should be treated as secondary sources. They occasionally simplify or overgeneralize current OpenCode behavior.

## CLI

### Available output formats for non-interactive `run`

For `opencode run`, the current CLI enumerates exactly two output formats:

| Format | Meaning |
| --- | --- |
| `default` | Human-oriented formatted output |
| `json` | Machine-oriented NDJSON stream of custom event objects |

The CLI syntax is:

```bash
opencode run --format default "your prompt"
opencode run --format json "your prompt"
```

The current source defines that flag in `packages/opencode/src/cli/cmd/run.ts` with:

- `choices: ["default", "json"]`
- `default: "default"`

### What `json` actually means

Despite the docs calling this "raw JSON events", the implementation is not the full raw bus event stream. The CLI subscribes to the internal event bus, selects a handful of signals, then re-emits them as its own NDJSON records.

Current emitted event types:

- `tool_use`
- `step_start`
- `step_finish`
- `text`
- `reasoning`
- `error`

### Side effects and behavior changes when `--format json` is used

| Behavior | Effect |
| --- | --- |
| Stdout becomes NDJSON | The primary result channel is machine-readable line output, not prose |
| Human status output moves to stderr | Share URLs, warnings, and formatted UI messages still go to stderr because `UI.println()` writes there |
| No dedicated completion event | Callers must infer completion from process exit and the last received events |
| Tool start visibility is incomplete | The current formatter does not emit a generic "tool started" event for most tools |
| Reasoning is opt-in | `reasoning` records only appear when `--thinking` is supplied |
| Permission prompts are not exposed as JSON | `permission.asked` is handled internally and not emitted as structured stdout |

### Related but different `--format` flags

OpenCode also uses `--format json` on some non-streaming commands such as session/model listing, but those return regular JSON documents or tables rather than the NDJSON event stream used by `opencode run`.

## Tools

### Built-in tools currently available out of the box

The official tools page documents these public built-ins:

- `bash`
- `edit`
- `write`
- `read`
- `grep`
- `glob`
- `list`
- `lsp` (experimental)
- `apply_patch`
- `skill`
- `todowrite`
- `webfetch`
- `websearch`
- `question`

The current source-level registry also includes:

- `task`
- `codesearch`
- `batch` (experimental)
- `plan_exit` (experimental CLI plan mode)
- `invalid` (internal fallback, not a user-facing tool)

This is a good example of why the source is the stronger reference than the public tools page for integration work.

### What the CLI JSON stream exposes for tool calls

Current `opencode run --format json` behavior is asymmetric:

| Phase | CLI JSON visibility | Hook visibility |
| --- | --- | --- |
| Before a tool runs | No general structured stdout event today | `tool.execute.before` and `event` hook both see it |
| While a tool is running | Usually no structured stdout event; `task` gets special pretty-print handling only in default mode | `tool.execute.before`, internal part updates, and plugin `event` hook are richer |
| After success | Yes, as `tool_use` with `part.type = "tool"` and `part.state.status = "completed"` | `tool.execute.after` and `event` hook both see it |
| After failure | Yes, as `tool_use` with `part.state.status = "error"` | `tool.execute.after` may still observe the post-call state; raw `event` hook is the safer universal source |

The important payload sits inside `part.state`:

- `input`: the tool arguments the model used
- `title`: the tool title shown by OpenCode
- `output`: textual tool result
- `metadata`: tool-specific structured metadata
- `time`: start/end timestamps
- `attachments`: optional attached files or images

### Tool metadata examples

#### `bash`

`tool_use.part.state.metadata` includes:

- `output`: preview of command output
- `exit`: exit code
- `description`: the tool description shown to the agent

#### `read`

`tool_use.part.state.metadata` includes:

- `preview`: preview of the loaded content
- `truncated`: whether the read was truncated
- `loaded`: referenced system-reminder file paths that were injected

#### `write`

`tool_use.part.state.metadata` includes:

- `diagnostics`: LSP diagnostics after the write
- `filepath`: absolute file path
- `exists`: whether the file already existed

#### `edit`

`tool_use.part.state.metadata` includes:

- `diagnostics`
- `diff`
- `filediff` with before/after content and addition/deletion counts

#### `task`

`tool_use.part.state.metadata` includes:

- `sessionId`: child session ID for the subagent run
- `model`: `{ providerID, modelID }` used by the child agent

#### `step_finish`

Strictly speaking this is not a tool call, but it is the most important structured "after" record for accounting. It includes:

- `cost`
- `tokens.input`
- `tokens.output`
- `tokens.reasoning`
- `tokens.cache.read`
- `tokens.cache.write`
- `reason`

### Example NDJSON shapes

These are representative examples reconstructed from the current source and official part schemas.

#### Completed tool call

```json
{
  "type": "tool_use",
  "timestamp": 1775490000000,
  "sessionID": "ses_123",
  "part": {
    "type": "tool",
    "tool": "read",
    "callID": "call_1",
    "state": {
      "status": "completed",
      "input": {
        "filePath": "/repo/src/lib.rs",
        "offset": 1,
        "limit": 200
      },
      "title": "src/lib.rs",
      "output": "<path>/repo/src/lib.rs</path> ...",
      "metadata": {
        "preview": "pub fn example() { ... }",
        "truncated": false,
        "loaded": []
      },
      "time": {
        "start": 1775490000100,
        "end": 1775490000325
      }
    }
  }
}
```

#### Step accounting event

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

#### Failed tool call caused by permissions

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

## Use Cases

### Plan Cap Approaching

| Question | Answer |
| --- | --- |
| CLI JSON event type | No dedicated event type |
| Best fallback | Possibly `error` or raw `session.status` retry messages, but only if the provider surfaces a useful string |
| How to distinguish | Only by provider-specific message text; there is no normalized "cap approaching" field |
| Remaining budget extractable? | No structured field today |
| Reset window extractable? | No structured field today |
| Hook exposure | No dedicated hook event either; the best hook fallback is raw `event` subscriptions for `session.error` or `session.status` |
| Stream vs hook parity | Hooks are slightly richer because they preserve the raw event type and payload; the CLI JSON formatter does not surface retry/status events at all |

Assessment: OpenCode does not currently formalize "plan cap approaching" as a first-class signal. If a provider emits a warning in plain text, callers must pattern-match on vendor-specific strings.

### Plan Capped

| Question | Answer |
| --- | --- |
| CLI JSON event type | Usually `error` |
| Best raw event | `session.error` with `APIError` or `ProviderAuthError`, depending provider behavior |
| How to distinguish | Inspect `error.name`, then inspect `error.data.message` and, when present, `error.data.responseBody` |
| Remaining budget extractable? | Not in a normalized field |
| Reset window extractable? | Not in a normalized field |
| Hook exposure | Yes, via `event` hook on `session.error` |
| Stream vs hook parity | Similar payload, but the hook sees the raw `session.error` object instead of the CLI envelope |

Provider-specific notes:

- OpenAI-style `insufficient_quota` is normalized by OpenCode to an `APIError` message that says quota was exceeded and billing should be checked.
- OpenAI-style `usage_not_included` is normalized to an upgrade message for Codex access.

That means "capped" is detectable only as a generic provider error, not as a standardized plan-window object.

### No Funds

| Question | Answer |
| --- | --- |
| CLI JSON event type | `error` |
| Best raw event | `session.error` with `APIError` |
| How to distinguish | Check `error.name === "APIError"` and inspect the provider-specific message or response body |
| Normalized fields? | Only partially; OpenCode normalizes some OpenAI-style quota errors into friendlier text |
| Hook exposure | Yes, via `event` hook on `session.error` |
| Stream vs hook parity | Very close; the hook gets the original raw event, the CLI wraps it |

Assessment: "no funds" is not a dedicated OpenCode event. It is a subclass of provider error handling.

### Auth

| Question | Answer |
| --- | --- |
| CLI JSON event type | `error` when auth fails |
| Best raw event | `session.error` with `ProviderAuthError` |
| How to distinguish | `error.name === "ProviderAuthError"` |
| Can auth kind be detected? | Not from the current `opencode run --format json` stream |
| Hook exposure | Yes, via `event` hook on `session.error` |
| Stream vs hook parity | Same fundamental signal, but neither includes "API key vs OAuth vs subscription" as a structured field |

Important nuance: older or abandoned community proposals exposed fields like `apiKeySource`, but the current `run` implementation does not.

### Permissions: Can't Read File

| Question | Answer |
| --- | --- |
| CLI JSON event type | Usually `tool_use` with `part.tool === "read"` and `part.state.status === "error"` |
| Full path available? | Yes, via `part.state.input.filePath` for the attempted read |
| Reason available? | Sometimes. Explicit deny usually becomes a generic permission-denied error string. Ask-mode rejection becomes a rejection/corrected message |
| How to distinguish | Check `tool === "read"`, inspect `state.error`, and inspect `state.input.filePath` |
| Hook exposure | Yes. `event` hook sees `permission.asked` for ask-mode reads; `tool.execute.before` sees raw args even earlier |
| Stream vs hook parity | Hooks are richer. CLI JSON only shows the post-failure tool record; hooks can expose the permission request itself |

Important source-level detail:

- The read tool asks for permission `read`.
- For reads, the raw permission request uses `patterns: [absolute_file_path]`.
- Read permission metadata is empty, so the file path comes primarily from `patterns` or `tool.state.input.filePath`.

### Permissions: Can't Write File

| Question | Answer |
| --- | --- |
| CLI JSON event type | Usually `tool_use` with `tool` equal to `write`, `edit`, or `apply_patch`, and `state.status === "error"` |
| Full path available? | Usually yes |
| Reason available? | Sometimes; explicit deny becomes a generic permission-denied error string |
| How to distinguish | Inspect `tool`, `state.input`, and `state.error` |
| Hook exposure | Yes. `permission.asked` plus `tool.execute.before/after` are both relevant |
| Stream vs hook parity | Hooks are significantly richer for write-like tools because metadata includes diffs and per-file details |

Important tool differences:

- `write`: `state.input.filePath` gives the absolute path; permission metadata includes `filepath` and `diff`.
- `edit`: `state.input.filePath` gives the absolute path; permission metadata includes `filepath` and `diff`.
- `apply_patch`: the path may be embedded in `patchText`, but permission metadata includes relative paths, total diff, and per-file details.

### Tokens Consumed

| Question | Answer |
| --- | --- |
| CLI JSON event type | `step_finish` |
| Session total available directly? | No dedicated final total event in the current run formatter |
| Granular data available? | Yes, per model step |
| Cost basis available? | Yes, `part.cost` |
| Hook exposure | Yes, via `message.part.updated` for `step-finish`, and `message.updated` for assistant-turn totals |
| Stream vs hook parity | Hooks are richer because `message.updated` exposes final assistant-turn `cost` and `tokens` in addition to per-step accounting |

The best current automation strategy is to sum every `step_finish.part.tokens` and `step_finish.part.cost` record you observe.

### Model Used

| Question | Answer |
| --- | --- |
| CLI JSON event type | No dedicated event in the current `run --format json` stream |
| Can model be detected reliably from CLI JSON? | Not generally |
| Hook exposure | Yes, via `message.updated` |
| Raw hook fields | Assistant messages expose `providerID` and `modelID`; user messages expose `model.providerID` and `model.modelID` |
| Stream vs hook parity | Hooks are much better; the CLI formatter drops model identity entirely |

The default human formatter prints `agent · modelID` to stderr when the assistant starts, but that is not structured stdout data.

### Human in the Loop

| Question | Answer |
| --- | --- |
| Can the CLI JSON stream detect prompts/questions? | Not today |
| Can the CLI JSON stream detect permission prompts? | Not as structured stdout; `run` handles them internally and writes warnings to stderr |
| Hook exposure | Yes, strongly |
| Relevant hook events | `question.asked`, `question.replied`, `question.rejected`, `permission.asked`, `permission.replied` |
| Stream vs hook parity | Not close. Hooks expose full request payloads; CLI JSON exposes none of them |

Current non-interactive behavior nuances:

- `opencode run` creates sessions with `question`, `plan_enter`, and `plan_exit` denied.
- That suppresses built-in primary-agent question/plan approval flows in non-interactive mode.
- Built-in subagents also inherit `question: deny` by default from the agent defaults.
- Custom agents or plugins can still create human-in-the-loop scenarios that the CLI JSON stream will not surface cleanly.

For subagents specifically:

- A `task` tool result can tell you that a child session existed and which model it used.
- It does **not** expose live child-session `question.asked` or `permission.asked` events on stdout.
- The plugin event bus is the only reliable structured source for those.

### Injecting into Subagent Prompt

| Question | Answer |
| --- | --- |
| First-class CLI support? | No |
| Task-tool prompt field? | Yes, but it is controlled by the parent model when it calls `task` |
| Caller-side runtime append field? | No dedicated flag or JSON-stream mechanism |
| Hook workaround? | Partially, via system/message transformation hooks |
| Stream vs hook parity | The CLI stream gives no help here; hooks are the only structured interception point |

What is possible today:

- The parent agent can include any text it wants in the `task` tool's `prompt`.
- Agent definitions in `.opencode/agents/*.md` can inject persistent subagent instructions.
- Global/project instructions and plugin hooks such as `experimental.chat.system.transform` and `chat.message` can modify session context more broadly.

What is **not** available today:

- A dedicated non-interactive CLI flag like "append this extra string to every subagent prompt".
- A structured child-prompt injection field surfaced by `opencode run --format json`.

## Gotchas

### No formal schema for the exact CLI NDJSON envelope

The official OpenAPI and TypeScript types are good, but they stop at the underlying message/event model. If your parser depends on the exact `run --format json` envelope, you are coding against implementation behavior in `run.ts`.

### "Raw JSON events" is slightly misleading

The CLI docs say `json` means raw JSON events. In practice, `opencode run --format json` emits a **filtered and reformatted** stream, not the full raw event bus. Critical signals like `message.updated`, `session.status`, `permission.asked`, and `question.asked` are omitted from stdout.

### No session-complete event

Community requests such as <https://github.com/anomalyco/opencode/issues/17221> explicitly ask for a terminal event containing the session ID, but the current formatter still does not emit one. Callers have to infer completion from process exit and the last observed events.

### Tool start visibility is incomplete

Open PR <https://github.com/anomalyco/opencode/pull/18249> exists specifically because current JSON mode does not provide good generic "tool started" visibility. If you need live progress bars or "tool is running" UX, the current stdout stream is weaker than the hook layer.

### Docs and SDK examples have drifted

Issue <https://github.com/anomalyco/opencode/issues/14875> reports that the SDK docs showed structured output under `structured_output`, while the current model stores it under `structured`. That kind of drift matters when you are generating parsers or adapters automatically.

### Structured output and reasoning-model interactions have been brittle

Issue <https://github.com/anomalyco/opencode/issues/15226> documents a real failure mode where `toolChoice: "required"` for structured output collided with thinking-enabled models. Open PR <https://github.com/anomalyco/opencode/pull/18450> exists to move toward native provider JSON-schema support.

### Third-party harness docs can be outdated

Some third-party documentation is useful but lags the source. For example:

- Cub's OpenCode harness page assumes `opencode run` behaves like an auto-approve autonomous harness.
- Cupcake's OpenCode reference simplifies plugin behavior into a smaller event model than OpenCode actually exposes.

Treat those docs as integration notes, not as the source of truth.

## Timeline

The dates below focus on structured output in non-interactive or machine-consumable workflows.

| Date | Event | Why it matters |
| --- | --- | --- |
| 2025-06-29 | PR #533 proposed an earlier `run --print` mode with `json` and `stream-json` outputs: <https://github.com/anomalyco/opencode/pull/533> | Useful historical context: OpenCode experimented with richer machine-readable non-interactive output before the current `--format json` shape |
| 2025-10-31 | PR #3638 updated docs/source alignment for `opencode run --format json`: <https://github.com/anomalyco/opencode/pull/3638> | This is the earliest clear upstream evidence I found that the current `run --format json` mode was established and documented as "raw JSON events" |
| 2025-12-16 | Feature request #5639 opened for SDK structured outputs: <https://github.com/anomalyco/opencode/issues/5639> | Marks explicit demand for schema-constrained structured output beyond plain JSON streaming |
| 2026-02-12 | PR #8161 merged and shipped in release `v1.1.60`: <https://github.com/anomalyco/opencode/pull/8161>, <https://github.com/anomalyco/opencode/releases/tag/v1.1.60> | Official introduction of Claude Agent SDK-style structured outputs in the OpenCode SDK |
| 2026-02-12 | Issue #13342 reported docs drift immediately after the structured-output merge: <https://github.com/anomalyco/opencode/issues/13342> | Shows that consumers had to track naming/details closely because documentation changed quickly |
| 2026-02-24 | Issue #14875 reported the docs still used the wrong structured-output field name: <https://github.com/anomalyco/opencode/issues/14875> | Confirms that even after rollout, the documentation for structured output remained easy to misread |
| 2026-02-26 | Issue #15226 documented a structured-output failure with thinking-enabled models: <https://github.com/anomalyco/opencode/issues/15226> | Highlights a real integration gotcha for machine-reliant callers |
| 2026-03-19 | PR #18249 proposed emitting running `tool_use` events in JSON mode: <https://github.com/anomalyco/opencode/pull/18249> | Important for observability: it shows current JSON mode is still seen as incomplete by downstream integrators |
| 2026-03-20 | PR #18450 proposed moving structured output to native `Output.object()` support: <https://github.com/anomalyco/opencode/pull/18450> | Signals likely future change in how structured output is implemented and possibly how failures look |
| 2026-04-06 | Release `v1.3.16` fixed output token totals when reasoning tokens are separated: <https://github.com/anomalyco/opencode/releases/tag/v1.3.16> | Important for budget/accounting consumers who sum token usage from structured output |

