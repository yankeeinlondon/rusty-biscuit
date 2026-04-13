---
schema: https://github.com/MoonshotAI/kimi-cli/blob/main/src/kimi_cli/wire/types.py
schema_type: python-pydantic
data_format: jsonl
docs: https://moonshotai.github.io/kimi-cli/en/customization/wire-mode.html
created: 2026-04-06
last_updated: 2026-04-06
---

## Summary

Kimi Code CLI currently exposes two different machine-readable outputs for non-interactive execution, and they are not equivalent.


`kimi --wire` is the richer protocol and the better fit for Claudine. It uses JSON-RPC 2.0 over line-delimited JSON and carries explicit event and request objects for approvals, human questions, hooks, token usage updates, plan displays, subagent activity, and tool execution state. If Claudine needs to understand progress, surface decisions, or respond to the agent mid-run, Wire is the protocol to target.

Moonshot does not currently publish a standalone JSON Schema, OpenAPI spec, or AsyncAPI spec for this protocol. The best formal schema I found is the official provider source itself: the Pydantic model definitions in `src/kimi_cli/wire/types.py`, together with the JSON-RPC envelope definitions in `src/kimi_cli/wire/jsonrpc.py`. The official docs also include TypeScript-style interface definitions, but during this research window they lagged the shipping source in a few places. In particular, the public Wire docs still described protocol `1.4`, while the shipping source exposed protocol `1.8`. For protocol-sensitive work, the repository source is the more authoritative schema reference.

The practical implication for Claudine is straightforward: use `--wire` when you need structured orchestration, human-in-the-loop detection, hooks, or subagent visibility; keep `--print --output-format stream-json` only as a lighter fallback for simple one-shot automation.

## Schema

The best available formalization is the official Kimi Code CLI source code:

- Primary schema source: `https://github.com/MoonshotAI/kimi-cli/blob/main/src/kimi_cli/wire/types.py`
- Transport wrapper: `https://github.com/MoonshotAI/kimi-cli/blob/main/src/kimi_cli/wire/jsonrpc.py`
- Protocol version constant: `https://github.com/MoonshotAI/kimi-cli/blob/main/src/kimi_cli/wire/protocol.py`
- Human-readable schema docs: `https://moonshotai.github.io/kimi-cli/en/customization/wire-mode.html`

### What exists today

For Wire mode, the runtime schema is expressed as Python Pydantic models and tagged unions. The transport is JSON-RPC 2.0, with each request, response, or notification serialized as a single JSON line on stdin/stdout.

At a high level, the wire envelope looks like this:

```json
{
  "jsonrpc": "2.0",
  "method": "event",
  "params": {
    "event": {
      "type": "StatusUpdate",
      "payload": {
        "token_usage": 1234
      }
    }
  }
}
```

The schema is split conceptually into:

- JSON-RPC envelope types such as `initialize`, `prompt`, `steer`, `replay`, `set_plan_mode`, `cancel`, `event`, and `request`
- Event unions such as `TurnBegin`, `StepBegin`, `StatusUpdate`, `Notification`, `PlanDisplay`, `SubagentEvent`, `ToolCall`, `ToolResult`, and `ApprovalResponse`
- Request unions such as `ApprovalRequest`, `QuestionRequest`, `ToolCallRequest`, and `HookRequest`
- Response payloads such as `QuestionResponse` and `HookResponse`

For print mode, the official docs describe the output more informally as a `Message` stream in JSONL, but the actual implementation also emits non-message objects such as notifications and plan displays in `stream-json` mode. That behavior is best understood from source:

- `https://github.com/MoonshotAI/kimi-cli/blob/main/src/kimi_cli/ui/print/visualize.py`
- `https://github.com/MoonshotAI/kimi-cli/blob/main/src/kimi_cli/ui/print/__init__.py`

### What does not exist

I did not find any official:

- JSON Schema file
- OpenAPI document for print or wire mode
- AsyncAPI document
- Versioned standalone `.d.ts` package for the wire protocol

I also looked for third-party formalizations in places where one might reasonably exist, including the official docs, the `MoonshotAI/kimi-cli` repository, `MoonshotAI/kimi-agent-rs`, Vercel AI SDK references, LangChain TypeScript references, Smithery CLI-agent integration docs, and DeepWiki mirrors. I did not find a respected third-party package that formalizes Kimi’s non-interactive structured output beyond restating the official protocol.

### Best schema choice for Claudine

If Claudine needs a source of truth today, the safest choice is:

- Treat `src/kimi_cli/wire/types.py` as the canonical schema
- Treat `src/kimi_cli/wire/jsonrpc.py` as the canonical transport definition
- Treat the docs page as explanatory rather than authoritative

That recommendation is based on one practical detail: during this research pass, the public Wire documentation still described protocol `1.4`, while the source and changelog had moved forward to `1.8`.

## Documentation

### Official documentation

The most important official documentation URLs are:

- Wire mode overview and TypeScript-style protocol docs: `https://moonshotai.github.io/kimi-cli/en/customization/wire-mode.html`
- Print mode docs, including `stream-json` examples: `https://moonshotai.github.io/kimi-cli/en/customization/print-mode.html`
- Main command reference for `--print`, `--wire`, `--input-format`, `--output-format`, `--quiet`, and `--final-message-only`: `https://moonshotai.github.io/kimi-cli/en/reference/kimi-command.html`
- FAQ entries covering JSONL input quirks and print/wire troubleshooting: `https://moonshotai.github.io/kimi-cli/en/faq.html`
- Release notes and changelog: `https://moonshotai.github.io/kimi-cli/en/release-notes/changelog.html`
- Data locations, including `wire.jsonl`: `https://moonshotai.github.io/kimi-cli/en/configuration/data-locations.html`
- Agent and subagent docs: `https://moonshotai.github.io/kimi-cli/en/customization/agents.html`
- Providers and models: `https://moonshotai.github.io/kimi-cli/en/configuration/providers.html`
- Project repository: `https://github.com/MoonshotAI/kimi-cli`
- Rust Wire implementation: `https://github.com/MoonshotAI/kimi-agent-rs`

### Secondary references and ecosystem explainers

I did not find a large ecosystem of independent write-ups specifically focused on Kimi’s structured output format. The most useful secondary references I found were:

- DeepWiki repository overview: `https://deepwiki.com/MoonshotAI/kimi-cli/1-overview`
- DeepWiki tool-system page: `https://deepwiki.com/MoonshotAI/kimi-cli/6-tool-system`
- DeepWiki advanced-features page: `https://deepwiki.com/MoonshotAI/kimi-cli/7-advanced-features`
- Smithery CLI-agent integration docs: `https://smithery.ai/docs/build/cli-agents`
- Smithers CLI-agent integrations page: `https://smithers.sh/integrations/cli-agents`
- Context7 mirror of Kimi CLI docs/reference material: `https://context7.com/moonshotai/kimi-cli`

These secondary references are useful for orientation, but they should not override the official source or official docs when Claudine needs protocol-level certainty.

### Documentation quality assessment

The official documentation is good enough to understand the intent of both print mode and wire mode, but it still leaves important gaps for implementers:

- Print mode examples are simpler than the real emitted stream
- The wire docs are more formal, but the runtime source is still more complete
- Edge cases like permission errors, auth distinctions, quota exhaustion, or subagent forwarding behavior are clearer in tests and source than in the docs

## CLI

Kimi exposes structured output in two ways:

- Print mode with `--output-format stream-json`
- Wire mode with `--wire`

Only the first is part of the `--output-format` enumeration.

### Enumerated output formats

The official `--output-format` values are:

- `text`
  - Human-readable text output
  - Best for terminal use and simple scripting
  - Loses structure unless you parse the prose yourself
- `stream-json`
  - JSONL output, one JSON object per line
  - Intended for programmatic processing
  - Emits assistant/tool-message style objects and a few non-message objects such as notifications and plan displays

Related input formats:

- `text`
- `stream-json`

In `--input-format stream-json`, stdin is JSONL and only user-role messages are consumed as prompts.

### CLI syntax

Print mode, text output:

```bash
kimi --print -p "Summarize this repo"
```

Print mode, structured JSONL output:

```bash
kimi --print -p "Summarize this repo" --output-format stream-json
```

Bidirectional JSONL in and JSONL out:

```bash
echo '{"role":"user","content":"Hello"}' | \
  kimi --print --input-format stream-json --output-format stream-json
```

Final assistant message only:

```bash
kimi --print -p "Generate a commit message" --final-message-only
```

Shortcut for final text only:

```bash
kimi --quiet -p "Generate a commit message"
```

Wire mode:

```bash
kimi --wire
```

Auto-approval, which matters for unattended runs:

```bash
kimi --print -p "Run the task" --output-format stream-json --yolo
```

### Behavioral side effects

- `--print` implicitly enables non-interactive execution and auto-approval behavior
- `--quiet` is equivalent to `--print --output-format text --final-message-only`
- `--final-message-only` intentionally discards intermediate structure, so it is not appropriate when Claudine needs tool or progress visibility
- `--wire` changes the interaction model entirely: it becomes a JSON-RPC server over stdin/stdout rather than a simple prompt-to-text pipeline
- In non-interactive or `--yolo` flows, Kimi injects instructions telling the model that the user cannot answer questions during execution
- In `--wire`, some features require capability negotiation during `initialize`, especially structured question handling and plan-mode support

### Output mode comparison

| Mode | Transport | Direction | Best use |
| --- | --- | --- | --- |
| `--print --output-format text` | plain text | one-way | humans and simple scripts |
| `--print --output-format stream-json` | JSONL | mostly one-way | lightweight automation |
| `--wire` | JSON-RPC 2.0 over JSONL | bidirectional | full integration, orchestration, and Claudine-style supervision |

## Gotchas

- `stream-json` and `--wire` are both structured, but they are not interchangeable. `stream-json` is much poorer for orchestration.
- `--final-message-only` and `--quiet` are intentionally lossy. They are useful for wrappers that only want the final answer, but they suppress the intermediate state Claudine usually cares about.
- The official docs are not always perfectly synchronized with the shipping source. During this pass, the Wire docs still described protocol `1.4` while the runtime source exposed `1.8`. For protocol-sensitive work, prefer the repository’s runtime models and changelog.
- There is no dedicated structured event for quota nearing exhaustion, quota exhausted, or account balance depleted. Those conditions may require out-of-band usage queries or generic provider-error interpretation.
- There is no dedicated structured event for file permission denial. You normally infer it from the preceding `ToolCall` and the following `ToolResult` error payload or tool-error text.
- There is no reliable stream event that names the active model/provider for each turn. Claudine will need external configuration or session metadata if that matters.
- Question handling in wire mode depends on capability negotiation. If the client does not declare question support, Kimi can hide `AskUserQuestion` from the model entirely.
- In `--yolo` mode, `AskUserQuestion` does not become a true interactive pause. It is auto-dismissed and returns a synthetic answer payload telling the agent to decide on its own.
- Subagent traffic is only partially wrapped. Some subagent events appear inside `SubagentEvent`, but blocking requests such as approvals and questions are forwarded to the parent stream directly.
- Notifications can interleave with message streaming. If Claudine preserves ordering, it should treat the stream as an event log rather than as a simple assistant transcript.

## Timeline

The dates below are from Kimi CLI’s published changelog and the repository source history as visible on 2026-04-06. The main focus here is the evolution of structured output.

- 2025-11-03, `0.46`
  - Wire over stdio was introduced experimentally, establishing the JSON-RPC based structured protocol surface.
- 2025-11-12, `0.54`
  - `stream-json` received an important correctness fix so the last assistant message would not be dropped.
- 2025-11-28, `0.59`
  - Wire mode was substantially reworked; `TurnBegin` was added and wire JSONL recording was improved.
- 2025-12-19, `0.66`
  - `StatusUpdate` gained `token_usage` and `message_id`, making structured token accounting more useful.
- 2025-12-31, `0.70`
  - `--final-message-only` and `--quiet` were added, explicitly creating more lossy print-mode variants.
- 2026-01-20, `0.80`
  - `initialize` and external tool call support were added to wire mode; the protocol started to behave more like a proper integration surface.
- 2026-02-03, `1.6`
  - `TurnEnd` was added, making turn boundaries explicit for integrators.
- 2026-02-06, `1.9.0`
  - `replay` was added, enabling replay-oriented consumers over wire.
- 2026-02-26, `1.14.0`
  - `QuestionRequest` and `QuestionResponse` were introduced, along with capability negotiation for structured human questions.
- 2026-02-27, `1.16.0`
  - `AskUserQuestion` was automatically hidden when the client did not advertise question support.
- 2026-03-11, `1.20.0`
  - `set_plan_mode` support was added.
- 2026-03-23, `1.25.0`
  - Subagent-related wire metadata became richer, including `SubagentEvent` metadata and more approval source metadata.
- 2026-03-28, `1.27.0`
  - Wire protocol `1.7` added `PlanDisplay`, `HookTriggered`, `HookResolved`, `HookRequest`, and `HookResponse`.
- 2026-03-30, `1.28.0`
  - Wire protocol `1.8` added another display-level schema change (`DiffDisplayBlock.is_summary`), showing the protocol is still actively evolving.

Contextual milestones that help explain the ecosystem, but are not themselves structured-output milestones:

- 2026-02
  - `kimi-agent-rs` began shipping as a Rust implementation specifically targeting Wire mode.
- 2026-03
  - Web UI documentation described a browser UI built on top of the same wire-style protocol layer.

## Tools

The default built-in tools exposed by the stock Kimi agent configuration are:

- `Agent`
- `AskUserQuestion`
- `SetTodoList`
- `Shell`
- `TaskList`
- `TaskOutput`
- `TaskStop`
- `ReadFile`
- `ReadMediaFile`
- `Glob`
- `Grep`
- `WriteFile`
- `StrReplaceFile`
- `SearchWeb`
- `FetchURL`
- `ExitPlanMode`
- `EnterPlanMode`

The default subagent types are:

- `coder`
- `explore`
- `plan`

### Tool visibility in structured output

Wire mode gives the clearest tool lifecycle:

- Before or during execution
  - `ToolCall` or `ToolCallPart`
  - `ApprovalRequest` if the tool needs confirmation
  - `HookTriggered` or `HookRequest` if hooks are configured
- After execution
  - `ToolResult`
  - `ApprovalResponse`
  - `HookResolved`
  - Possible `Notification`

Print `stream-json` is much thinner:

- Assistant messages may include `tool_calls`
- Tool results are flattened into tool-role messages
- Notifications and plan displays may appear as separate JSON objects
- There is no full request/approval/hook lifecycle

### What the JSON stream tells you by tool class

| Tool class | Typical examples | What Wire exposes |
| --- | --- | --- |
| File read tools | `ReadFile`, `ReadMediaFile`, `Glob`, `Grep` | tool name, arguments, result payload, and failures via `ToolResult` |
| File write tools | `WriteFile`, `StrReplaceFile` | tool call, optional approval, result or error, hook traffic if configured |
| Shell execution | `Shell` | command arguments, optional approval, structured result, error text, notifications |
| Remote fetch/search | `SearchWeb`, `FetchURL` | call arguments and returned content or error |
| Planning tools | `EnterPlanMode`, `ExitPlanMode`, `SetTodoList` | status changes, plan displays, and sometimes notifications |
| Human tools | `AskUserQuestion` | `QuestionRequest` and `QuestionResponse` in wire mode, or auto-dismiss behavior in yolo mode |
| Subagent tools | `Agent`, `TaskList`, `TaskOutput`, `TaskStop` | `SubagentEvent` plus forwarded approvals/questions/tool requests from the child |

### Examples

Tool call followed by tool result in Wire mode:

```json
{"jsonrpc":"2.0","method":"event","params":{"event":{"type":"ToolCall","payload":{"id":"tc_1","function":{"name":"ReadFile","arguments":"{\"path\":\"/tmp/a.txt\"}"}}}}}
{"jsonrpc":"2.0","method":"event","params":{"event":{"type":"ToolResult","payload":{"tool_call_id":"tc_1","return_value":{"content":"hello"}}}}}
```

Approval-gated tool:

```json
{"jsonrpc":"2.0","method":"request","params":{"request":{"type":"ApprovalRequest","payload":{"id":"apr_1","tool_call_id":"tc_2","action":"execute","description":"Run shell command"}}}}
{"jsonrpc":"2.0","id":"apr_1","result":{"request_id":"apr_1","response":"approve"}}
```

Question in a client that supports it:

```json
{"jsonrpc":"2.0","method":"request","params":{"request":{"type":"QuestionRequest","payload":{"id":"q_1","tool_call_id":"tc_3","questions":[{"header":"Mode","question":"How should I proceed?"}]}}}}
{"jsonrpc":"2.0","id":"q_1","result":{"request_id":"q_1","answers":{"Mode":"Fast"}}}
```

Print mode `stream-json` assistant message with tool call:

```json
{"role":"assistant","content":"Let me inspect the repository.","tool_calls":[{"type":"function","id":"tc_1","function":{"name":"Shell","arguments":"{\"command\":\"ls\"}"}}]}
```

Print mode `stream-json` tool result:

```json
{"role":"tool","tool_call_id":"tc_1","content":"Cargo.toml\nREADME.md\nsrc\n"}
```

## Use Cases

### Plan Cap Approaching

- Event types:
  - I did not find a dedicated print-mode or wire-mode event for “you are close to your plan cap.”
- How to distinguish it:
  - Today, you generally cannot distinguish this from the structured execution stream because it is not modeled as a first-class event.
- Remaining quota:
  - Not from the normal stream. The nearest related mechanism is the separate `/usage` flow, which queries usage data out of band.
- Reset window:
  - Not exposed in the normal structured stream. Some usage responses may contain limit/reset hints, but they are not part of the standard `--print` or `--wire` event stream.
- Hook exposure:
  - I did not find a hook event that mirrors an “approaching cap” signal.

### Plan Capped

- Event types:
  - I did not find a dedicated structured “plan capped” event in print mode or wire mode.
- How to distinguish it:
  - In practice this would likely surface as a provider or account error, not as a typed quota event.
- Remaining quota:
  - Not from the stream.
- Reset window:
  - Not from the stream.
- Hook exposure:
  - No matching hook event found.

### No Funds

- Event types:
  - I did not find a dedicated “no funds” event type.
- How to distinguish it:
  - This would most likely surface as a generic provider-side error message or a failed request rather than as a typed structured event.
- Hook exposure:
  - No specific hook event found for “no funds.”

### Auth

- Event types:
  - Wire mode defines an `AUTH_EXPIRED` JSON-RPC error code for one specific auth failure case.
- How to distinguish it:
  - `AUTH_EXPIRED` is emitted when an OAuth-backed session receives a 401-like expiration condition.
  - Other auth failures may remain generic provider errors such as `CHAT_PROVIDER_ERROR`.
- Auth kind detection:
  - Partial only.
  - If you receive `AUTH_EXPIRED`, that strongly suggests an OAuth-style logged-in session.
  - If you receive a generic provider error, you cannot reliably infer whether the user was using an API key, an OAuth session, or another auth path.
- Hook exposure:
  - I did not find a hook that exposes auth kind or an auth-expired event with equivalent structure.

### Permissions: Can't Read File

- Event types:
  - No dedicated permission-denied event exists.
  - The normal pattern is `ToolCall` for `ReadFile` followed by a failed `ToolResult`.
- How to identify the file path:
  - The full path is usually available in the preceding `ToolCall` arguments.
  - The returned tool error text may also embed the path.
- Reason for block:
  - The reason is not normalized into a dedicated permission enum.
  - You usually get generic tool-error text, often derived from the underlying OS error.
- How to distinguish it:
  - Match `ReadFile` or `ReadMediaFile` in the tool call, then inspect the failed `ToolResult`.
- Hook exposure:
  - Yes, approximately.
  - `PostToolUseFailure` hooks can expose tool name, tool input, and the resulting error text.
  - That is useful, but it is still a generic failed-tool signal rather than a dedicated “read permission denied” event.

### Permissions: Can't Write File

- Event types:
  - No dedicated permission-denied event exists.
  - The normal pattern is `ToolCall` for `WriteFile` or `StrReplaceFile`, optional `ApprovalRequest`, then a failed `ToolResult`.
- How to identify the file path:
  - Usually from the tool arguments in the preceding `ToolCall`.
- Reason for block:
  - Often only from the tool-error text.
  - Some errors are policy-like rather than OS-level, for example a rejected relative path where an absolute path is required.
- How to distinguish it:
  - Match a write/edit tool in the call, then inspect the error-bearing `ToolResult`.
- Hook exposure:
  - Yes, approximately via `PostToolUseFailure`.
  - As with reads, the hook payload is useful but not a dedicated write-permission schema.

### Tokens Consumed

- Event types:
  - `StatusUpdate` is the main structured event carrying token information in wire mode.
- Granularity:
  - It appears to be incremental or per-step progress metadata rather than a single authoritative end-of-session total.
- Cost basis:
  - I did not find pricing or currency metadata in the normal wire event schema.
- Print mode:
  - `stream-json` does not expose raw `StatusUpdate` events in the same way.
- Hook exposure:
  - I did not find a documented hook event that mirrors `StatusUpdate.token_usage` with equivalent detail.

### Model Used

- Event types:
  - I did not find a normal print-mode or wire-mode event that explicitly names the active model for the turn.
- Reliability:
  - Because the stream does not carry model identity directly, Claudine should not rely on the stream alone if model attribution matters.
- Naming conventions:
  - Model names exist elsewhere in configuration and runtime state, but I did not find a guaranteed per-turn stream event containing provider plus model name.
- Hook exposure:
  - I did not find a hook with an equivalent structured “model used” payload.

### Human in the Loop

- Detecting user questions or permission asks:
  - Yes.
  - Wire mode exposes `ApprovalRequest` for approval-gated operations.
  - Wire mode also exposes `QuestionRequest` when the client declares support for structured questions.
- Structured question data:
  - Yes.
  - `QuestionRequest` includes a structured list of questions with fields such as `question`, `header`, `options`, `multi_select`, `body`, `other_label`, and `other_description`.
- Non-interactive behavior:
  - In `--yolo` or similar non-interactive execution, Kimi injects instructions that the user cannot answer questions during execution.
  - In that mode, `AskUserQuestion` is effectively auto-dismissed and returns a synthetic result instructing the agent to make its own decision.
- Detecting the same behavior in subagents:
  - Yes.
  - Subagents can surface approvals and questions too.
  - Some child activity is wrapped in `SubagentEvent`, but blocking requests such as approvals and questions are forwarded to the parent stream directly rather than only being nested.
- Hook exposure:
  - Not as an equivalent hook pair.
  - Human-interaction requests are first-class wire requests, not merely hook events.

### Injecting into Subagent Prompt

- Can additional context be injected:
  - Yes, but not through a special “subagent prompt injection” wire API that I found.
  - The normal path is to supply prompt text through the `Agent` tool call or agent configuration.
- Non-interactive warning injection:
  - For the specific use case you described, Kimi already does this itself in yolo or non-interactive flows.
  - The runtime injects guidance telling the agent, including subagents, that the user cannot answer questions during execution.
- Distinguishing what is automatic versus caller-controlled:
  - Automatic:
    - non-interactive and yolo safety text
  - Caller-controlled:
    - the `prompt` argument you pass when spawning a subagent
    - agent-spec configuration
- Hook exposure:
  - No dedicated hook for “prompt was injected into subagent.”
  - The nearest related hooks are `SubagentStart` and `SubagentStop`, which expose lifecycle rather than prompt content.

## Notes on Hook Parity

Kimi’s hook system and Kimi’s wire stream overlap, but they are not identical surfaces.

- Hook lifecycle events I found include `PreToolUse`, `PostToolUse`, `PostToolUseFailure`, `UserPromptSubmit`, `Stop`, `StopFailure`, `SessionStart`, `SessionEnd`, `SubagentStart`, `SubagentStop`, `PreCompact`, `PostCompact`, and `Notification`.
- Wire-specific hook transport objects include `HookTriggered`, `HookResolved`, `HookRequest`, and `HookResponse`.
- In general:
  - tool failures can often be observed both in the normal stream and in hook callbacks
  - human interaction requests are wire-native requests, not hook-native events
  - quota, cap, no-funds, and model-used signals are not well covered by either surface
