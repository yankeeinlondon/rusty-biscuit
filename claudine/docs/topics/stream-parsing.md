# Stream Parsing

When **Claudine** is running in non-interactive mode it will ask the Agent to respond in a streaming format. Each Agent has slightly different structure and semantics but we standardize the way we want to interact with this streaming information with the [`StreamParser`](claudine/lib/src/stream/parser.rs) trait.

Each provider's wire format is modelled by a strongly typed, serde-derived event enum in [`stream/protocol/`](claudine/lib/src/stream/protocol/). The parser's `feed_line` walks a two-pass pipeline: parse the raw NDJSON line into a `serde_json::Value` first (which preserves the pre-existing malformed-line handling and keeps a raw copy available for `raw_summary` construction), then attempt typed deserialization into a provider-specific `*Event` enum. Unknown event types fall through to a silent skip that matches the legacy behavior, so provider format drift never turns into a hard failure. See the "Typed Protocol Models" section below for the module layout, the patterns each provider uses, and the edge cases that shaped them.


## Research

We have done deep research on the JSON streaming data that each of CLI Agents reports:

- [claude](claudine/docs/research/non-interactive-sessions/claude.md)
- [codex](claudine/docs/research/non-interactive-sessions/codex.md)
- [gemini](claudine/docs/research/non-interactive-sessions/gemini.md)
- [kimi](claudine/docs/research/non-interactive-sessions/kimi.md)
- [opencode](claudine/docs/research/non-interactive-sessions/opencode.md)
- [qwen](claudine/docs/research/non-interactive-sessions/qwen.md)
- [goose](claudine/docs/research/non-interactive-sessions/goose.md)
- [roo code](claudine/docs/research/non-interactive-sessions/roo-code.md)

## Streaming Schemas

Based on this research we've been able to establish the following schemas for the various providers:

### Claude

- **Flag:** `claude -p "<prompt>" --verbose --output-format stream-json`
- **Format:** NDJSON, one self-contained JSON object per line
- **Discriminator:** top-level `type`
- **Schema source:** TypeScript `SDKMessage` union in the `@anthropic-ai/claude-agent-sdk` npm package (`sdk.d.ts`). No standalone JSON Schema or OpenAPI spec is published.
- **Top-level `type` values** (21-member `SDKMessage` union): `assistant`, `user`, `user_message_replay`, `result`, `system`, `stream_event`, `compact_boundary`, `status`, `local_command_output`, `hook_started`, `hook_progress`, `hook_response`, `tool_progress`, `auth_status`, `task_notification`, `task_started`, `task_progress`, `files_persisted`, `tool_use_summary`, `rate_limit_event`, `prompt_suggestion`
- **`system.subtype` values:** `init`, `api_retry`, `compact_boundary`, `status`, `hook_started`, `hook_progress`, `hook_response`, `task_notification`, `task_started`, `task_progress`, `files_persisted`, `local_command_output`
- **`result.subtype` values:** `success`, `error_max_turns`, `error_during_execution`, `error_max_budget_usd`, `error_max_structured_output_retries`
- **Tool calls:** `tool_use` → `tool_result` pairs correlated by `tool_use.id` ↔ `tool_result.tool_use_id`; `is_error` marks failures. MCP tools use `mcp__<server>__<tool>` naming.
- **Key fields:** `init.model`, `init.apiKeySource` (requires `--verbose`), `result.usage.{input_tokens,output_tokens,cache_read_input_tokens,cache_creation_input_tokens}`, `result.total_cost_usd`, `result.modelUsage`, `rate_limit_event.rate_limit_info.{status,resetsAt,rateLimitType,overageStatus}`, `system/api_retry.error` enum (`billing_error`, `rate_limit`, `authentication_failed`, `server_error`, `invalid_request`, `max_output_tokens`, `unknown`).
- **Notes:** hook lifecycle events (`hook_started`/`hook_progress`/`hook_response`) require `--include-hook-events`; token-delta events (`stream_event`) require `--include-partial-messages`; `rate_limit_event` is subscription-only (API-key users instead see `billing_error` in `system/api_retry`). Subagent-internal events do not appear in the parent stream.

### Codex CLI

- **Command:** `codex exec --json "<prompt>"`
- **Format:** JSONL (newline-delimited JSON), one event per line on stdout
- **Discriminator:** top-level `type`
- **Schema source:** Rust Serde enums/structs in `codex-rs/exec/src/exec_events.rs` (authoritative); OpenAI also ships a broader JSON Schema Draft 7 bundle for the Codex App Server at `codex-rs/app-server-protocol/schema/json/codex_app_server_protocol.v2.schemas.json`, but that is a richer JSON-RPC protocol and is **not** 1:1 with the flattened `exec --json` stream.
- **Top-level event types:** `thread.started`, `turn.started`, `turn.completed`, `turn.failed`, `item.started`, `item.updated`, `item.completed`, `error`
- **`item.type` union:** `agent_message`, `reasoning`, `command_execution`, `file_change`, `mcp_tool_call`, `collab_tool_call`, `web_search`, `todo_list`, `error`
- **Tool lifecycle:** `item.started` → (optional `item.updated` for `todo_list`) → `item.completed`, correlated by `item.id`. `command_execution` carries `command`, `aggregated_output`, `exit_code`, `status`. `mcp_tool_call` carries `server`, `tool`, `arguments`, `result`, `error`, `status`. `file_change` carries `changes[].{path,kind}` and `status`.
- **Token usage:** `turn.completed.usage.{input_tokens, cached_input_tokens, output_tokens}` (per-turn only; no session aggregate).
- **Notes:** the stream does **not** expose model name, auth mode, cost basis, or ChatGPT rate-limit percentages as stable documented fields. `file_change` declines collapse to `status: "failed"` in the exec projection. Process exit code is not a reliable failure signal — parse `command_execution` items instead. Official TypeScript SDK (`sdk/typescript/src/{events,items}.ts`) lags the Rust source.

### Gemini CLI

- **Flag:** `gemini --output-format stream-json -p "<prompt>"` (or `--output-format json` for a single buffered object)
- **Format:** NDJSON for `stream-json`; single JSON object for `json`
- **Discriminator:** top-level `type`
- **Schema source:** TypeScript interfaces/enums in `packages/core/src/output/types.ts` and `packages/core/src/output/stream-json-formatter.ts`; the headless runtime lives in `packages/cli/src/nonInteractiveCliAgentSession.ts`. No JSON Schema published.
- **`stream-json` event types:** `init`, `message`, `tool_use`, `tool_result`, `error`, `result`
- **Key fields:**
    - `init`: `timestamp`, `session_id`, `model`
    - `message`: `timestamp`, `role`, `content`, optional `delta` (repeated deltas stream incremental assistant text)
    - `tool_use`: `timestamp`, `tool_name`, `tool_id`, `parameters`
    - `tool_result`: `timestamp`, `tool_id`, `status`, optional `output`, optional `error.{type,message}` (**tool_name is not repeated — join on `tool_id`**)
    - `error`: `timestamp`, `severity`, `message`
    - `result`: `timestamp`, `status`, optional `error`, optional `stats`
- **`result.stats` fields:** `total_tokens`, `input_tokens`, `output_tokens`, `cached`, `input`, `duration_ms`, `tool_calls`, `models` (per-model breakdown, added March 2026)
- **`json` mode shape:** `{ session_id?, response?, stats?, error? }`
- **Notes:** the headless formatter explicitly **drops** internal events `initialize`, `session_update`, `agent_start`, `tool_update`, `elicitation_request`, `elicitation_response`, `usage`, `custom`. `init.model` may be an alias like `auto`/`pro`/`flash`; `result.stats.models` is stronger evidence of the actual backend used. Cancelled tool calls can still report `status: "success"`. Persistent config `output.format` only accepts `text`/`json`; `stream-json` requires the CLI flag.

### Kimi Code

- **Flags:** `kimi --print --output-format stream-json` (thin, one-way) or `kimi --wire` (rich JSON-RPC 2.0, bidirectional)
- **Format:**
    - `--print --output-format stream-json`: JSONL assistant/tool-role messages plus occasional non-message objects (notifications, plan displays)
    - `--wire`: JSON-RPC 2.0 over stdin/stdout, one complete envelope per line
- **Schema source:** provider-authored Python Pydantic models in `src/kimi_cli/wire/types.py` (canonical), JSON-RPC envelope in `src/kimi_cli/wire/jsonrpc.py`, protocol version constant in `src/kimi_cli/wire/protocol.py`. No JSON Schema, OpenAPI, or AsyncAPI spec. Docs lag source (docs describe `1.4`; source ships `1.8` as of April 2026).
- **Wire envelope:** `{ "jsonrpc": "2.0", "method": "event"|"request"|<rpc>, "params": { "event"|"request": { "type": <string>, "payload": <object> } } }`
- **Wire method surface:** `initialize`, `prompt`, `steer`, `replay`, `set_plan_mode`, `cancel`, `event`, `request`
- **Event union (`method: "event"`):** `TurnBegin`, `TurnEnd`, `StepBegin`, `StatusUpdate`, `Notification`, `PlanDisplay`, `SubagentEvent`, `ToolCall`, `ToolCallPart`, `ToolResult`, `ApprovalResponse`, `HookTriggered`, `HookResolved`
- **Request union (`method: "request"`):** `ApprovalRequest`, `QuestionRequest` (only when client advertises support), `ToolCallRequest`, `HookRequest`
- **Response payloads:** `QuestionResponse`, `HookResponse`
- **Print-mode `stream-json`** is much thinner: assistant messages include embedded `tool_calls`; tool outputs arrive as tool-role messages flattened into the stream; no full request/approval/hook lifecycle.
- **Notes:** `StatusUpdate.token_usage` is the primary token channel in wire mode. There is no dedicated event for model identity, quota, or no-funds. `--yolo` auto-dismisses `AskUserQuestion` and injects non-interactive guidance into agent prompts. Wire mode gates structured question handling through `initialize` capability negotiation.

### Opencode CLI

- **Command:** `opencode run --format json "<prompt>"`
- **Format:** NDJSON, one event per line on stdout
- **Discriminator:** top-level `type`
- **Envelope:** `{ type, timestamp (epoch ms), sessionID, part? | error? }`
- **Schema source:** the CLI envelope is defined only in implementation (`packages/opencode/src/cli/cmd/run.ts`); the nested `part` payloads are formally schema'd by the OpenCode OpenAPI 3.1.1 spec (`packages/sdk/openapi.json`), generated TypeScript types (`packages/sdk/js/src/v2/gen/types.gen.ts`), and runtime Zod validators (`packages/opencode/src/session/message-v2.ts`). No published schema covers the exact CLI envelope.
- **Top-level event types:** `tool_use`, `step_start`, `step_finish`, `text`, `reasoning`, `error`
- **`part` payload types (per `type`):** `ToolPart`, `StepStartPart`, `StepFinishPart`, `TextPart`, `ReasoningPart`
- **`error` payload** wraps one of: `ProviderAuthError`, `UnknownError`, `MessageOutputLengthError`, `MessageAbortedError`, `StructuredOutputError`, `ContextOverflowError`, `ApiError`
- **Tool part structure:** `part.state.{status,input,output?,error?,title?,metadata?,time.start,time.end,attachments?}`; `status` is `"completed"` or `"error"`.
- **Token / cost accounting:** `step_finish.part.tokens.{input,output,reasoning,cache.{read,write}}` and `step_finish.part.cost` (step-level; sum across steps for session totals)
- **Notes:** `tool_use` is emitted **only** after tool completion or failure (no pre-execution event; PR `#18249` proposes this). `reasoning` requires `--thinking`. There is no terminal `session.complete` record — infer completion from process exit. `run` hardcodes `question`, `plan_enter`, `plan_exit` to `deny`; parent-session permission prompts auto-reject unless `--dangerously-skip-permissions` is set. Subagent live activity is not exposed beyond the final `task` tool result (which carries `metadata.sessionId`, `metadata.model.providerID`, `metadata.model.modelID`).

### Qwen CLI

- **Flag:** `qwen "<prompt>" --output-format stream-json [--include-partial-messages]` (or `--output-format json` for a buffered array)
- **Format:** NDJSON for `stream-json`; single JSON array for `json`
- **Discriminator:** top-level `type`
- **Schema source:** TypeScript unions in `packages/sdk-typescript/src/types/protocol.ts` (externalized formal definition) and `packages/cli/src/nonInteractive/types.ts` (closest to shipping behavior). No JSON Schema, OpenAPI, or AsyncAPI published.
- **Top-level unions:**
    - `CLIMessage`: `user`, `assistant`, `system`, `result`, `stream_event`
    - `ControlMessage` (bidirectional control plane): `control_request`, `control_response`, `control_cancel_request`
- **Assistant content blocks:** `text`, `thinking`, `tool_use`, `tool_result`
- **Stream-event sub-types** (only when `--include-partial-messages` is set): `message_start`, `content_block_start`, `content_block_delta`, `content_block_stop`, `message_stop`, `tool_progress` (MCP progress only)
- **`system` message:** current implementation emits subtype `init` (docs still say `session_start`) with `model` field
- **`result` message:** `subtype`, `duration_*`, `usage`, `permission_denials[]`, optional `stats` (**only populated in buffered `json` mode, not `stream-json`**)
- **Permission denials:** `result.permission_denials[]` carries `{tool_name, tool_use_id, tool_input}`; the `blocked_path` field exists in the control-plane types but the CLI currently sends `null`.
- **Control plane:** `control_request` subtypes include `can_use_tool` for structured permission approvals — only available when driven through bidirectional `--input-format stream-json --output-format stream-json` or the `@qwen-code/sdk` host.
- **Notes:** no stable typed event for auth method, quota, or no-funds. The `modelUsage` type field exists but is not populated by the current result builder. Subagent messages link back via `parent_tool_use_id`.

### Goose

- **Flag:** `goose run --output-format stream-json ...` (or `--output-format json` for one final object)
- **Format:** NDJSON for `stream-json` (despite the flag name); single JSON object for `json`
- **Discriminator:** top-level `type` (`snake_case`) **but nested `message.content[].type` is `camelCase`** — this is a real parser footgun.
- **Schema source:** outer envelope is defined only in Rust Serde types in `crates/goose-cli/src/session/mod.rs` (`StreamEvent`, `NotificationData`, `JsonOutput`, `JsonMetadata`); nested message payloads (`Message`, `MessageContent`, `ActionRequiredData`, `SystemNotificationContent`) are formally schema'd via `ui/desktop/openapi.json` (OpenAPI 3) and the generated `ui/desktop/src/api/types.gen.ts`. No complete CLI schema published.
- **Top-level `stream-json` event types:** `message`, `notification`, `error`, `complete`
- **`notification` shape:** `{ type: "notification", extension_id, message, progress?, total? }` — the payload is **flattened** via `#[serde(flatten)]`, not nested under a `log` key.
- **`complete` shape:** `{ type: "complete", total_tokens? }` — this is the only stable session token total.
- **Nested `MessageContent.type` values:** `text`, `image`, `toolRequest`, `toolResponse`, `toolConfirmationRequest`, `actionRequired`, `frontendToolRequest`, `thinking`, `redactedThinking`, `systemNotification`
- **`ActionRequiredData.actionType` values:** `toolConfirmation`, `elicitation`, `elicitationResponse`
- **`SystemNotificationType` enum:** `thinkingMessage`, `inlineMessage`, `creditsExhausted` (the sole credit-related structured signal; HTTP 402 maps to `ProviderError::CreditsExhausted` and then to a `systemNotification`)
- **Batch `json` shape:** `{ messages: Message[], metadata: { total_tokens, status } }`
- **Notes:** current `StreamEvent` enum has **no** `model_change` variant; authentication failures surface as `{type: "error", error: "<string>"}`. Some MCP and subagent notifications are reduced to formatted strings before stream emission. Non-interactive human-in-the-loop is still sharp-edged: tool confirmations and elicitation requests are currently intercepted before stream emission.

### Roo Code

- **Flag:** `roo --output-format stream-json --yolo` (or `--json` for a single completion-time array)
- **Format:** NDJSON
- **Discriminator:** top-level `type`
- **Envelope:** `{ type, timestamp (ISO-8601), data }`
- **Schema source:** TypeScript interfaces in the `@roo-code/types` package (`schemas/cli/json-events.ts`); JSON Schema versions are exported for cross-language validation.
- **Event types:** `init`, `thinking`, `tool_use`, `tool_result`, `cost`, `final_result`, `model_switch`, `plan_cap_approaching`, `plan_capped`, `insufficient_funds`, `auth_info`, `permission_denied`, `fatal_error`
- **Key event payloads:**
    - `init.data`: `model`, `provider`
    - `tool_use.data`: `name`, `input`, `tool_use_id`
    - `tool_result.data`: `tool_use_id`, `content`, `isError`
    - `cost.data`: `input_tokens`, `output_tokens`, `cache_creation_input_tokens`, `total_cost_usd` (per-turn)
    - `final_result.data`: aggregated status and totals
    - `auth_info.data`: `method` (`api_key`|`oauth`|`session_token`), `provider` (never secrets)
    - `permission_denied.data`: `operation` (`read`|`write`), `path`, `reason` (e.g. `EACCES`, `system_whitelist_blocked`)
    - `plan_cap_approaching.data`: `remaining_tokens`, `percent_used`, `reset_timestamp` (fires at 80% usage)
    - `plan_capped.data`: `reset_timestamp`, `upgrade_url`
    - `insufficient_funds.data`: `provider_name`, `current_balance`, `required_minimum`
- **Notes:** `-y` / `--yolo` / `--permission-mode acceptAll` is required for non-interactive runs to prevent the agent from hanging on approval prompts. In `--json` mode, ANSI styling is disabled; in `stream-json`, stdout is reserved for JSON and logging is redirected to stderr. `ask_followup_question` tool use is the detection point for HITL attempts; subagent HITL events nest within the subagent's event context.

## Typed Protocol Models

The 6 currently-supported providers (Claude, Codex, Gemini, OpenCode, Qwen, Kimi) have a matching typed event model at [`claudine/lib/src/stream/protocol/<provider>.rs`](claudine/lib/src/stream/protocol/). Each module exports a tagged enum `*Event` that covers every event type the corresponding parser dispatches on, plus one struct per variant payload. Shared design rules across all six modules:

- **`#[serde(tag = "type")]`** — each top-level enum is internally tagged on the `type` field, so each variant's struct receives the remaining fields.
- **Every field is optional.** Structs use `#[serde(default)]` on every field and are `#[derive(Debug, Default, Deserialize)]`. There is no `#[serde(deny_unknown_fields)]` anywhere in `protocol/`, so provider format evolution (new fields, new subtypes) never breaks deserialization.
- **No unknown-variant fallback.** When a provider emits an event whose `type` string isn't listed in the enum, `serde_json::from_value::<*Event>` returns `Err(_)`. The parser's `feed_line` treats that `Err(_)` as a silent skip, matching the legacy `_ => Ok(None)` arm.
- **Helper methods carry alias resolution.** Instead of exposing every field alias to handlers, each struct provides `resolved_*` / `take_*` helpers (e.g. `resolved_tool_name()`, `take_input()`) that walk all accepted aliases in a single place. Handlers call one method and are oblivious to the underlying aliasing.

### Module layout

```
claudine/lib/src/stream/protocol/
├── mod.rs       — module root + `ProtocolError` (documentation-only today)
├── claude.rs    — ClaudeEvent + 14 structs
├── codex.rs     — CodexEvent + 8 structs, dotted type names
├── gemini.rs    — GeminiEvent + 9 structs
├── opencode.rs  — OpenCodeEvent + 12 structs, `#[serde(flatten)]` for dual-location fields
├── qwen.rs      — QwenEvent + 9 structs, subtype-dispatched system events
└── kimi.rs      — KimiEvent + 10 structs, three-way content fallback
```

Every protocol module has its own `#[cfg(test)] mod tests` block covering each event variant, the major field aliases, and the `unknown_event_type_fails_typed` contract. Those tests deserialize raw JSON strings to guarantee the serde derives line up with the wire format — they are the safety net for provider format drift.

### Two-pass `feed_line` dispatch

Every parser now uses the same shape:

```rust
fn feed_line(&mut self, line: &str) -> Result<Option<StreamChunk>, StreamParseError> {
    // 1. line trimming + empty-line shortcut (unchanged)

    // 2. Parse as `Value` first — preserves malformed-line handling
    let raw: Value = serde_json::from_str(line).map_err(...)?;

    // 3. Extract event_type for tracing (the only residual `.get("type")`
    //    call in the parsers)
    let event_type = raw.get("type").and_then(|t| t.as_str()).unwrap_or("");
    super::trace_parser_event(Provider::<X>, event_type, self.line_num);

    // 4. Typed dispatch
    match serde_json::from_value::<<X>Event>(raw.clone()) {
        Ok(<X>Event::<Variant>(payload)) => { self.handle_<variant>(payload); Ok(None) }
        ...
        Err(_) => Ok(None),  // unknown or schema-mismatched — silently skipped
    }
}
```

Handler methods take the typed struct by value (e.g. `fn handle_result(&mut self, result: ClaudeResult, raw: Value)`) so there is no lingering `fn handle_*(&mut self, obj: &Value)` in any parser. The only handlers that still accept a raw `Value` are the two that construct a `raw_summary` for the execution summary (Claude's `handle_result`, Codex's `handle_turn_completed`, Gemini's `handle_result`, Qwen's `handle_result`) — those take both the typed event and the `raw` clone so the compact "large-arrays stripped" summary can be built from the original JSON.

### Per-provider idioms worth knowing

**Claude** — `ClaudeEvent` has separate `Init` and `System` variants that both wrap `ClaudeInit`, so `init` and `system` events funnel into the same handler via `Ok(ClaudeEvent::Init(init) | ClaudeEvent::System(init))`. `ContentBlockStart` is its own variant and the parser checks `content_block.kind == "tool_use"` before forwarding via `ClaudeContentBlock::into_tool_use()`. `ClaudeResult::effective_cost_usd()` picks `total_cost_usd` over the legacy `cost_usd`.

**Codex** — Dotted event names work cleanly with `#[serde(rename = "thread.created")]` on each variant. `turn.started` uses an empty `CodexTurnStarted {}` struct because internally-tagged unit variants in serde have quirky behavior around extra fields; empty structs silently accept any residual fields. `CodexItem` is a single flat struct covering every item subtype (agent_message, tool_use, tool_call, permission_request, etc.), with associated functions `is_tool_item_kind()` and `is_permission_item_kind()` that replace the old string-matching helpers. The `item.started` → `item.completed` merge is now a typed operation via `CodexItem::merge_started()` instead of a `Value` map-overlay. Codex keeps `StreamParseError::Fatal` (not `MalformedLine`) for malformed JSON. `handle_error` additionally suppresses duplicate sink emissions: Codex commonly emits `turn.failed` plus a top-level `error` carrying the same resolved `kind`/`message` for a single failure (rate-limit hits, auth failures). Summary state is always refreshed with the latest values, but the sink only sees one `SemanticEvent::Error` per distinct (kind, message) pair to avoid rendering identical "Agent Error" blocks twice on the live stderr surface.

**Gemini** — `GeminiMessage.content` stays `Option<Value>` because Gemini emits content as either a plain string or an array of `{text: ...}` parts; the handler branches on `content_val.as_str()` vs `content_val.as_array()` after typed deserialization. The severity-based warning/error branching in `handle_error` stays in the handler, since it affects sink dispatch rather than field extraction.

**OpenCode** — The most complex parser. Tool fields can appear at the top level of an event OR nested inside a `part` object, and `OpenCodeTool` captures both via `#[serde(flatten)] top: OpenCodeToolFields` plus a separate `part: Option<OpenCodeToolFields>`. A single `OpenCodeTool::resolve()` method collapses both locations into a `ResolvedOpenCodeTool` (top-level fields win; `part` fills gaps), replacing the 7 legacy `opencode_*` helper functions. `OpenCodeText::resolved_text()` walks `part.text` → top-level `text` → top-level `content` in one place. `OpenCodeStepStart` uses `#[serde(rename = "sessionID")]` for the camelCase session ID.

**Qwen** — The `system` event is dispatched only when `subtype == "session_start"` via `QwenSystem::is_session_start()` + `into_init()`. `QwenEvent::Assistant` (where the event type is literally `"assistant"`) skips the `role == "assistant"` filter that `QwenEvent::Message` / `QwenEvent::AssistantMessage` require, matching the legacy parser's dual-check logic. `QwenTool::take_input()` accepts all five aliases: `input`, `parameters`, `arguments`, `args`, `params` — the legacy type audit caught `args`/`params` as missing from the design doc, and the protocol module fixes that.

**Kimi** — `KimiContent::resolved_text()` implements the three-way fallback (`content` array → top-level `text` → `content` as string) in one method. `KimiStatusUpdate::resolved_context()` returns a `KimiContextUsage` struct whose `computed_percent()` method falls back to computing `used/total * 100.0` when the provider doesn't pre-supply `percent` — the 80% warning logic stays in the parser, not the type, because it's a sink-dispatch concern. Tool input aliases include `args`/`params` for parity with Qwen.

### Why `raw.clone()` is acceptable

The `feed_line` pattern parses the line twice: once into `Value`, once into the typed enum via `from_value`. That clone is unavoidable today because we need both the malformed-line error path and the raw `Value` for result summaries. The cost is negligible — result/summary events are rare, and the clone is a single `serde_json::Value` (not a full string reparse). Cleaning this up would require either collapsing the raw `Value` needs into the typed structs (via `#[serde(flatten)] extra: serde_json::Map<String, Value>`) or introducing a second code path that parses only the typed enum and loses the raw fallback.

### What still uses `.get()`

A `rg '\.get\('` across the parser files turns up only legitimate uses after the migration:

1. **`raw.get("type")` in each `feed_line`** — event-type extraction for the tracing call, invoked before typed dispatch.
2. **`part.get("text")` in Gemini, Qwen, Codex content walkers** — iterating a `Vec<Value>` whose entries are shape-diverse content parts (text, image, tool_use, etc.) that the typed layer intentionally leaves as `Value`.
3. **`self.tool_uses.get(id)` / `.remove(id)`** — `HashMap<String, ...>` lookups, not `Value` extraction.
4. **Test assertions like `raw.get("tools").is_none()`** — verifying the `raw_summary` compaction pass stripped the large arrays.

No handler method accepts `&Value` for field extraction anymore.
