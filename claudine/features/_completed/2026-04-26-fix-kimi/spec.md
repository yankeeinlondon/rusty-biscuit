# Fix Kimi Wrapper — Switch to Wire Mode

_Last updated: 2026-04-26_

## Problem

The `claudine kimi` wrapper, and by extension every `compose` / `inline-compose` / `sequence` invocation that resolves to Kimi, currently produces no visible assistant output, no token telemetry, no tool-call rendering, and no error classification. From the user's perspective `claudine @prompts/greet.md --kimi` looks like Kimi returned nothing — but Kimi did respond. Claudine's structured-stream pipeline silently dropped the entire turn.

The root cause is a schema mismatch between what Claudine's Kimi parser expects and what Kimi actually emits in print mode:

- **What the parser expects** ([`claudine/lib/src/stream/protocol/kimi.rs`](../../../lib/src/stream/protocol/kimi.rs:9-36)): a `#[serde(tag = "type")]` tagged enum with variants `init`, `assistant`, `message`, `content`, `tool_use`, `tool_result`, `StatusUpdate`, `error`. The semantic parser ([`claudine/lib/src/stream/kimi_semantic.rs`](../../../lib/src/stream/kimi_semantic.rs:277-281)) reads `raw_kind` from `raw.get("type")`.
- **What Kimi emits** in `--print --output-format stream-json`: OpenAI Chat Completions–shaped messages with `role` as the discriminator and no top-level `type` field. Example:

  ```json
  {"role":"assistant",
   "content":[{"type":"think","think":"…","encrypted":null},
              {"type":"text","text":"Hi Bob! …"}],
   "tool_calls":[{"type":"function","id":"tool_…","function":{"name":"Shell","arguments":"{\"command\":\"ls -la\"}"}}]}
  {"role":"tool",
   "content":[{"type":"text","text":"…"}],
   "tool_call_id":"tool_…"}
  ```

Because every line lacks the `type` field the parser keys off, **every** Kimi event fails typed deserialization, falls into the `ProviderExtension` fallback with `raw_kind = ""`, and disappears into the JSONL log. The live sink's last-resort summarizer then renders `kimi/ · assistant` because the `role` string is the first non-empty top-level scalar, which is exactly the symptom the user reports.

A second consequence is that the existing Kimi parser's unit tests in [`kimi_semantic.rs:436-490`](../../../lib/src/stream/kimi_semantic.rs:436-490) and [`kimi.rs:249-376`](../../../lib/src/stream/protocol/kimi.rs:249-376) are written against the wrong fixtures — they pass against synthetic `{"type":"…"}` payloads that Kimi has never actually emitted.

## Why Wire Mode

Print mode (`--print --output-format stream-json`) is intentionally lossy. The Kimi research doc at [`claudine/docs/research/non-interactive-sessions/kimi.md`](../../../docs/research/non-interactive-sessions/kimi.md) is explicit: print mode is _"a lighter fallback for simple one-shot automation"_ and _"`stream-json` is much poorer for orchestration"_. In particular print mode does not expose:

- session init (no `init` event with session id, model, capability list)
- `TurnBegin` / `TurnEnd` boundaries
- `StatusUpdate` token usage / message ids
- `ApprovalRequest` / `QuestionRequest` (request/response handshake)
- `HookTriggered` / `HookResolved` / `HookRequest` / `HookResponse`
- `SubagentEvent` lifecycle metadata
- `PlanDisplay` and plan-mode transitions
- structured `AUTH_EXPIRED` errors
- a structured `Notification` channel

Wire mode (`--wire`, JSON-RPC 2.0 over line-delimited JSON) exposes all of the above. It is the protocol Kimi's own [`kimi-agent-rs`](https://github.com/MoonshotAI/kimi-agent-rs) crate targets, and it is the surface the rest of Claudine's parsers are conceptually calibrated to (Codex, OpenCode, Claude all expose comparable richness).

Switching the Kimi wrapper from `--print --output-format stream-json` to `--wire` therefore both **fixes the silent-drop bug** and **brings Kimi up to feature parity** with the other wrappers on token/cost telemetry, tool lifecycle, error classification, and HITL detection.

## Goals

1. `claudine kimi`, `claudine kimi -p "<prompt>"`, and any composition command resolving to Kimi must produce live structured output with assistant text, thinking blocks, tool calls, tool results, and a final summary line — matching the visual contract of `live_semantic_sink` for the other providers.
2. The end-of-run [`StreamExecutionSummary`](../../../lib/src/stream/summary.rs) for Kimi must populate `session_id`, `model`, `assistant_text`, `token_usage`, `cost_usd` (when available), `duration_ms`, `num_turns`, `tool_calls`, `context_usage`, and badges where the wire stream carries them.
3. Structured error classification for Kimi must distinguish `Configuration` (auth, capability negotiation, agent-file errors), `ApiRemote` (provider error, rate limit, billing), `AgentNative` (unclassified Kimi runtime errors), and `Interrupted` (cancel / abort) — feeding [`SemanticErrorKind`](../../../lib/src/stream/semantic.rs).
4. Approval and question requests must be auto-resolved in non-interactive runs (Claudine's existing `--yolo` policy applies), but must **also** surface as visible events in the live sink so the user can see what was auto-approved.
5. Existing wrapper-level features (resume, system-prompt injection, MCP injection-when-supported, hook registration, JSONL replay) continue to work unchanged from the user's perspective.

## Non-Goals

- This feature does **not** add interactive HITL to Kimi runs. `--yolo`/non-interactive semantics remain the only supported approval policy.
- This feature does **not** wire MCP runtime injection into the Kimi wrapper. Kimi remains in the `claudine mcp export kimi --apply` cohort. (MCP injection for Kimi is a follow-up.)
- This feature does **not** add a Kimi-specific UI for plan-mode transitions beyond rendering `PlanDisplay` events as structured info lines.
- This feature does **not** introduce a JSON-RPC client abstraction for re-use across providers. The wire transport is implemented Kimi-locally; a generic re-usable JSON-RPC client is out of scope.

## Wire Protocol Overview

Reference: [`https://moonshotai.github.io/kimi-cli/en/customization/wire-mode.html`](https://moonshotai.github.io/kimi-cli/en/customization/wire-mode.html) and the canonical Pydantic models at [`https://github.com/MoonshotAI/kimi-cli/blob/main/src/kimi_cli/wire/types.py`](https://github.com/MoonshotAI/kimi-cli/blob/main/src/kimi_cli/wire/types.py).

### Transport

- One JSON object per line on stdin and stdout (LSP-style "JSON Lines", **not** LSP `Content-Length` framed).
- Every object is a JSON-RPC 2.0 envelope: `{"jsonrpc": "2.0", …}`.
- Three envelope kinds are in scope:
  - **Request** (server → client) — `{ "jsonrpc": "2.0", "id": "<rpc-id>", "method": "request", "params": { "request": { "type": "<RequestType>", "payload": {…} } } }`. Requires a response.
  - **Notification** (server → client) — `{ "jsonrpc": "2.0", "method": "event", "params": { "event": { "type": "<EventType>", "payload": {…} } } }`. No response required.
  - **Response** (client → server) — `{ "jsonrpc": "2.0", "id": "<rpc-id>", "result": {…} }` or `{ "jsonrpc": "2.0", "id": "<rpc-id>", "error": { "code": <int>, "message": "<str>" } }`.
- Client-issued requests in scope:
  - `initialize` — capability negotiation, sent once at startup.
  - `prompt` — submit a user prompt.
  - `cancel` — abort the current turn.
  - (Optional, post-MVP) `replay`, `steer`, `set_plan_mode` — defer until a clear use case.
- Protocol version observed in the field is `1.8` (see research doc); Claudine must declare its supported version range during `initialize` and degrade gracefully when the server reports a higher version.

### Initialize Handshake

Claudine's `initialize` request must declare its capability surface. At minimum:

```jsonc
{
  "jsonrpc": "2.0",
  "id": "init-1",
  "method": "initialize",
  "params": {
    "protocol_version": "1.8",
    "client": { "name": "claudine", "version": "<crate version>" },
    "capabilities": {
      "approvals": true,
      "questions": false,
      "hooks": true,
      "subagents": true,
      "plan_mode": false
    }
  }
}
```

Capabilities Claudine declares in v1:

- `approvals: true` — auto-approve via `--yolo`-equivalent response.
- `questions: false` — Kimi will hide `AskUserQuestion` from the model, mirroring its non-interactive behavior. (Setting `questions: true` would force Claudine to synthesize answers, which is out of scope for this feature.)
- `hooks: true` — Claudine routes hook traffic through its existing dispatch pipeline.
- `subagents: true` — Claudine surfaces `SubagentEvent` notifications as structured info lines.
- `plan_mode: false` — Claudine does not currently drive `set_plan_mode`; if the server emits a `PlanDisplay` it is rendered as info but no transition is requested.

If `initialize` fails (e.g. AUTH_EXPIRED, protocol mismatch, or the binary does not advertise wire mode at all), the wrapper must surface a `SemanticEvent::Error { kind: Configuration, terminal: true }` and exit non-zero with a Prose-styled stderr message that points the user at `kimi auth login` / a version upgrade.

### Event Catalog (notifications, server → client)

The parser must handle these `event.type` values. Any unknown `event.type` falls through to `ProviderExtension` exactly as the existing protocol modules do.

| Event type        | Maps to `SemanticEvent`                                | Notes                                                                                  |
|-------------------|--------------------------------------------------------|----------------------------------------------------------------------------------------|
| `TurnBegin`       | (no direct semantic event; updates `num_turns` and resets per-turn buffers) | Boundary for live-sink "thinking" coalescing. |
| `TurnEnd`         | (no direct semantic event; flushes per-turn buffers)   | Triggers `flush_if_idle` style flush in the live renderer.                              |
| `StatusUpdate`    | `SemanticEvent::Warning` only when `>= 80%` context pressure (mirroring current behavior); otherwise updates summary state | Carries `token_usage`, `message_id`, `context_usage`. |
| `Notification`    | `SemanticEvent::Info` with prose body                  | Already a typed Kimi shape. Render with `▌ ` border in the live sink to match Reasoning. |
| `PlanDisplay`     | `SemanticEvent::Info` with `kind: "plan_display"` extra | First-class info; do not drop to ProviderExtension.                                    |
| `MessageStart`    | (begins assistant text accumulation)                   | Resets the active assistant-text buffer; does not emit.                                |
| `MessageDelta`    | `SemanticEvent::OutputText { text }`                   | Streamed token chunks; appended to `assistant_text`.                                   |
| `MessageEnd`      | (closes assistant text buffer; ensures trailing newline) | No emission unless trailer needs to be flushed.                                        |
| `Thinking`        | `SemanticEvent::Reasoning { text }`                    | Rendered as `BlockQuote` per the 2026-04-16 reasoning contract.                         |
| `ToolCall`        | `SemanticEvent::ToolCall { name, id, input, … }`       | `payload.function.arguments` is a JSON-encoded **string**; the parser must deserialize it before forwarding so the live sink's `ToolCallDisplay` formatter sees structured input. |
| `ToolCallPart`    | (accumulates partial tool call args)                   | Buffer until the matching `ToolCall` finalizes (mirrors OpenCode/Codex incremental tool patterns). |
| `ToolResult`      | `SemanticEvent::ToolResult { name, id, status, output, … }` | Correlated by `tool_call_id`. Status comes from `payload.error` presence. |
| `SubagentEvent`   | `SemanticEvent::Info { kind: "subagent", … }`          | Surface name, action, and forwarded tool name when present. Forwarded approvals/questions arrive on the parent stream as separate request envelopes (see below). |
| `HookTriggered`   | `SemanticEvent::Info { kind: "hook_triggered", … }`    | Hook name, lifecycle event.                                                            |
| `HookResolved`    | `SemanticEvent::Info { kind: "hook_resolved", … }`     | Hook outcome.                                                                          |
| `DiffDisplayBlock` | `SemanticEvent::Info { kind: "diff_display", is_summary }` | Carries the 2026-03-30 `is_summary` field; live sink renders short summaries inline and large diffs as a single status line. |

Field naming is canonical PascalCase per the wire protocol; aliases observed in the field are resolved by helper methods on the typed structs (mirroring the existing `protocol/` module pattern).

### Request Catalog (server → client requests requiring a response)

All requests are answered automatically by Claudine in v1. Each request also produces a visible `SemanticEvent` so the user can see what was auto-resolved.

| Request type        | Auto-response                                | Visible event                                                          |
|---------------------|-----------------------------------------------|------------------------------------------------------------------------|
| `ApprovalRequest`   | `{ "request_id": "<id>", "response": "approve" }` | `SemanticEvent::Info { kind: "auto_approved", … }` carrying `tool_call_id`, `action`, `description`. |
| `QuestionRequest`   | (should not arrive — Claudine declares `questions: false`. If it does arrive, respond with a synthetic `request_id` + empty answers and emit a `Warning`.) | `SemanticEvent::Warning { … }` |
| `ToolCallRequest`   | (external tool calls — out of scope in v1; respond with `error: { code: -32601, message: "external tools not supported" }` and emit a `Warning`.) | `SemanticEvent::Warning { … }` |
| `HookRequest`       | Forwarded to Claudine's existing dispatch pipeline (`crate::dispatch`); response built from the dispatch outcome. | `SemanticEvent::Info { kind: "hook_request", … }` |

### Error Classification

`classify_error` must be rewritten against the wire-mode error envelope. Two error sources matter:

1. **JSON-RPC envelope errors** — `error: { code, message }` returned in response to a client request. Classification by code:
   - `AUTH_EXPIRED` → `SemanticErrorKind::Configuration`.
   - `CHAT_PROVIDER_ERROR` → `SemanticErrorKind::ApiRemote`.
   - `INTERNAL_ERROR` (`-32603`) → `SemanticErrorKind::AgentNative`.
   - Method-not-found (`-32601`), invalid params (`-32602`) → `SemanticErrorKind::Configuration`.
   - Anything else → `SemanticErrorKind::AgentNative`.
2. **Notification-shaped errors** (`event.type == "Error"`, when present) — keyword classification on the message body, similar to today's `classify_error` but operating on the new payload shape.

Cancel / abort signals (`event.type == "Cancelled"` or any error caused by Claudine sending a `cancel` request) map to `SemanticErrorKind::Interrupted`.

## Wrapper Changes

### Profile

[`claudine/cli/src/commands/wrap/profile.rs`](../../../cli/src/commands/wrap/profile.rs) — `KimiWrapper`:

- `apply_entrypoint` — for non-interactive runs, append `--wire` instead of `--print`. The interactive path remains unchanged (no `--wire`, no `--print`).
- `apply_structured_stream` — replace the current `push_stream_json_flags(args, &["--print"])` call with a new path that adds `--wire` (no `--output-format stream-json`, no `--print`).
- `prompt_delivery` — must change from "stdin or `--prompt`" to a wire-mode-specific delivery. Wire mode does not accept `--prompt`; the prompt is delivered via a JSON-RPC `prompt` request after `initialize` completes. New variant `PromptDelivery::WireRpc(String)` (or equivalent) on the existing [`PromptDelivery`](../../../cli/src/commands/wrap/profile.rs) enum, dispatched in [`exec.rs`](../../../cli/src/commands/wrap/exec.rs) by a new wire-aware launcher.
- `apply_yolo` — `--yolo` continues to be appended to the kimi binary args (as today). The wire-mode `ApprovalRequest` auto-approval logic in the parser is the runtime enforcement; the `--yolo` flag remains as belt-and-braces and matches user expectations from `claudine kimi --yolo`.
- `supports_structured_stream` and `stream_protocol` continue to return `true` and `Some(StreamProtocol::WireJsonRpc)` respectively. A new `StreamProtocol::WireJsonRpc` variant is added (or the existing `StreamJson` variant is extended with a sub-flavour) so downstream code can branch on the transport when needed.
- `apply_system_prompt`, `build_resume_args`, `allowed_env_keys`, `prompt_arg_conventions` are unchanged.

### Launch / IO loop

A new module — `claudine/cli/src/commands/wrap/wire_io.rs` (peer of [`stream_io.rs`](../../../cli/src/commands/wrap/stream_io.rs)) — owns the wire-mode IO loop:

1. Spawn `kimi … --wire …` with stdin and stdout connected to Claudine.
2. Send `initialize` request with the capability set above; await response and validate protocol version.
3. Send `prompt` request with the resolved user prompt body. (System prompt is delivered through the existing `--agent-file` mechanism in `apply_system_prompt`; it does **not** go through the wire `prompt` request.)
4. Run a bidirectional select loop:
   - Read lines from kimi stdout, dispatch to the parser, forward parsed `SemanticEvent`s to the existing live sink.
   - For each `request` envelope received from kimi, build the appropriate auto-response (per the Request Catalog above) and write it back on kimi stdin. The response must be flushed; wire mode is line-buffered and unflushed bytes will hang the child.
5. On `TurnEnd` for the prompt's turn, send no further requests; await the child's natural exit. (The wire docs do not require an explicit shutdown request, but Claudine should close stdin to signal end-of-input cleanly.)
6. On Claudine-initiated cancel (Ctrl+C, deadline timeout), send a `cancel` JSON-RPC request before tearing down the child, then close stdin.

The existing tracing spans (`handle_stdin_read`, etc.) extend to wire-mode by introducing a parallel `wire_request_dispatch` span around the IO loop. The 5-second hook handler deadline does **not** apply to the wrapper's main loop.

### Resume

`build_resume_args` already returns `["kimi", "--resume", session_id, "--print"]`. For wire mode, change the trailing `--print` to `--wire`. The session id space is shared between print and wire mode, so resume across modes is safe.

## Parser Changes

[`claudine/lib/src/stream/protocol/kimi.rs`](../../../lib/src/stream/protocol/kimi.rs) is rewritten end-to-end. The new module exposes:

- `KimiEnvelope` — top-level enum tagged on the JSON-RPC envelope shape (`{ "method": "event", … }` vs `{ "method": "request", … }` vs `{ "id": "…", "result": … }` vs `{ "id": "…", "error": … }`). This enum is the single seam between the line reader and downstream dispatch.
- `KimiEvent` — typed enum over `event.type` with per-variant payload structs (`KimiTurnBegin`, `KimiTurnEnd`, `KimiStatusUpdate`, `KimiNotification`, `KimiPlanDisplay`, `KimiMessageStart`, `KimiMessageDelta`, `KimiMessageEnd`, `KimiThinking`, `KimiToolCall`, `KimiToolCallPart`, `KimiToolResult`, `KimiSubagentEvent`, `KimiHookTriggered`, `KimiHookResolved`, `KimiDiffDisplayBlock`, `KimiCancelled`, `KimiError`).
- `KimiRequest` — typed enum over `request.type` (`KimiApprovalRequest`, `KimiQuestionRequest`, `KimiToolCallRequest`, `KimiHookRequest`).
- All structs derive `Deserialize` with `#[serde(default)]` on every field. No `#[serde(deny_unknown_fields)]` anywhere.
- Helper methods on payload structs to resolve aliases (`resolved_tool_id`, `resolved_message`, `take_arguments`, `parse_arguments_string`, etc.). `KimiToolCall::parse_arguments_string` is required because wire-mode `payload.function.arguments` is a JSON-encoded string per the OpenAI tool-call convention.
- `#[cfg(test)] mod tests` per the existing pattern, with fixtures captured from a real Kimi wire-mode run (see "Test Fixtures" below). The unit tests must cover every variant, every alias resolver, and the `unknown_event_type_fails_typed` contract for both `KimiEvent` and `KimiRequest`.

[`claudine/lib/src/stream/kimi_semantic.rs`](../../../lib/src/stream/kimi_semantic.rs) is rewritten to consume `KimiEnvelope`:

- `KimiSemanticStreamParser` gains an outbound channel/closure for sending JSON-RPC responses back to the child (so request envelopes can be auto-answered without the parser owning the kimi stdin handle directly). The wire IO loop wires this up.
- The parser tracks per-turn state (`current_turn_index`, `pending_message_buffer`, `pending_tool_calls: HashMap<id, KimiToolCall>`) and resets at `TurnBegin` / flushes at `TurnEnd`.
- `feed_line` becomes a two-pass dispatch: parse to `serde_json::Value` for the malformed-line path and `raw_summary` extraction, then attempt typed deserialization into `KimiEnvelope`.
- `finish` returns a `StreamExecutionSummary` whose `provider_status` reflects the last observed `TurnEnd.status` (when present) or is derived from the child exit code, and whose context/usage fields come from the latest `StatusUpdate`.

The 80% context-pressure warning behavior is preserved verbatim; `CONTEXT_PRESSURE_WARN_PERCENT` stays at `80.0`.

## Live Sink Changes

[`claudine/cli/src/commands/wrap/live_semantic_sink.rs`](../../../cli/src/commands/wrap/live_semantic_sink.rs) requires no structural changes — it already routes `OutputText`, `Reasoning`, `ToolCall`, `ToolResult`, `Info`, `Warning`, and `Error` events through the 9-section model. Two small adjustments:

1. The `kimi/` provider prefix needs new entries in `SILENT_PROVIDER_EXTENSION_KINDS` for high-volume wire-mode events that should bypass stderr but remain in the JSONL log (candidates: any `MessageDelta` that arrives as a ProviderExtension fallback, `tool_call_part` partials). These are added defensively as drift-protection; they should be empty in practice once the parser is fully covering the stream.
2. The `Notification` event uses the same `BlockQuote` rendering Claude's reasoning blocks use, with the `▌ ` border. This matches the 2026-04-16 reasoning rendering contract.

## Hook Dispatch

Wire-mode `HookRequest` envelopes route into the existing [`crate::dispatch`](../../../lib/src/dispatch) pipeline. The mapping:

- `HookRequest.payload.event` → canonical Claudine event name (canonical mapping table lives at [`crate::adapters::kimicode`](../../../lib/src/adapters/kimicode.rs)).
- `HookRequest.payload.context` → the dispatch payload.
- The dispatch outcome (allow / deny / mutate) feeds back into a `HookResponse` JSON-RPC reply.

The existing `claudine/lib/src/adapters/kimicode.rs` already canonicalizes Kimi hook event names; the wire-mode integration reuses that adapter. No changes required there beyond confirming the mapping covers wire-mode-only events (`HookTriggered` is a notification, not a hook event in the dispatch sense — it's informational).

## Test Fixtures

Real-world wire-mode samples must be captured before the parser is finalized. Capture procedure:

1. Run `kimi --wire` in a controlled environment with a deterministic prompt (`"Hi how are you? My name is Bob."`).
2. Drive `initialize` / `prompt` / pump events to completion via a thin scratch script.
3. Save the stdout transcript verbatim to `claudine/lib/src/stream/protocol/fixtures/kimi/wire-greet.jsonl`.
4. Repeat with a tool-using prompt (`"Run ls and tell me what you see"`) to capture `ToolCall` / `ToolResult` / `ApprovalRequest`. Save to `wire-tool-shell.jsonl`.
5. Repeat with a subagent-spawning prompt to capture `SubagentEvent` and forwarded approvals. Save to `wire-subagent.jsonl`.
6. Repeat with an authentication-expired account to capture `AUTH_EXPIRED`. Save to `wire-auth-expired.jsonl`.
7. Repeat with a deliberately cancelled run to capture `Cancelled`. Save to `wire-cancelled.jsonl`.

Integration tests in [`claudine/lib/tests/`](../../../lib/tests/) replay each fixture line-by-line through `KimiSemanticStreamParser` and assert on the resulting `Vec<SemanticEvent>` and final `StreamExecutionSummary`. These tests are the contract the parser must satisfy and are the safety net against future Kimi protocol drift.

## Migration & Compatibility

- The wrapper's external CLI surface is unchanged: `claudine kimi`, `claudine kimi -p "…"`, `--yolo`, `--resume <id>` all continue to work.
- JSONL log shape changes: events that previously surfaced as `provider_extension` with empty `raw_kind` now surface as proper typed events. Any downstream consumer reading historical Kimi JSONL must tolerate the schema change. The `claudine logs` reporter is consumer-agnostic and requires no migration.
- The `provider_extension` Kimi escape hatch remains for any wire event variant Kimi adds in future protocol versions.
- Print-mode (`--print --output-format stream-json`) support is **removed** from the wrapper. If a user needs print-mode output, they can invoke `kimi --print --output-format stream-json` directly without Claudine. Removing the print-mode branch keeps the wrapper code single-shaped and avoids the temptation to dual-maintain two parsers.

## Risks

- **Protocol drift.** The wire protocol has moved from `1.4` to `1.8` in the past few months. The parser's `#[serde(default)]` strategy and the `ProviderExtension` fallback contain drift to additive shape changes; breaking shape changes still require an update. Mitigation: the protocol-version check during `initialize` fails loudly, and the integration-test fixtures are pinned per-version with the protocol version in the filename.
- **Tool argument decoding.** Wire-mode tool arguments are JSON-encoded strings inside `payload.function.arguments`. Parse failures must fall through to a string passthrough rather than dropping the call entirely. The `KimiToolCall::parse_arguments_string` helper handles this; tests must cover the malformed-arguments case.
- **Race between request response and next event.** Some wire-mode events (notably `ToolResult`) may arrive before Claudine has flushed its response to a preceding `ApprovalRequest`. The IO loop must use a single writer task (or a mutex) to serialize writes to kimi stdin so responses are well-ordered.
- **Cancel propagation.** Sending `cancel` requires the IO writer to be alive at the moment the user presses Ctrl+C. Claudine's existing signal-handling for wrappers (`exec.rs`) must be updated to invoke the wire IO loop's cancel hook rather than killing the child outright.
- **Capability negotiation strictness.** If a future kimi build hard-rejects an `initialize` whose declared capabilities omit something the server now requires, the wrapper degrades to a Configuration error. Mitigation: `initialize` failure surfaces a clear remediation message ("upgrade `kimi` or downgrade `claudine`").

## Success Criteria

- `claudine @prompts/greet.md --kimi` produces visible assistant text in the live sink, a populated `assistant_text` in the summary, and a non-zero `num_turns`. The reproducer from the bug report passes.
- `claudine kimi -p "Run ls -la"` shows a `→ Shell(ls -la)` tool-call line, a `← Shell(success)` result line, and assistant text — matching the visual contract of the other wrappers.
- The end-of-run summary shows token usage when `StatusUpdate` carried it, cost when present, duration, num turns, tool count, and context window usage.
- An auth-expired Kimi session emits `Configuration Error` styling on stderr (orange `▌ ` block) and exits non-zero with a remediation hint.
- Hook events fired by kimi reach Claudine's dispatch pipeline and trigger the user's configured actions exactly as Claude / Codex / OpenCode do.
- The integration tests under `claudine/lib/tests/` against the captured wire-mode fixtures all pass.
- The legacy `--print --output-format stream-json` Kimi tests are deleted; the new wire-mode tests in `kimi_semantic.rs` and `protocol/kimi.rs` cover every event variant with realistic fixtures.
- `cargo test -p claudine-lib`, `cargo test -p claudine-cli`, and `just test` (root) pass.
