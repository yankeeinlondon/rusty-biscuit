---
phases: 7
created: 2026-04-26
start_phase: 1
packages:
    - claudine
    - claudine-cli
source_files_during_phase_0: []
docs_updated_during_phase_0:
    - claudine/docs/research/non-interactive-sessions/kimi.md
docs_created_during_phase_0:
    - claudine/lib/src/stream/protocol/fixtures/kimi/wire-greet.jsonl
    - claudine/lib/src/stream/protocol/fixtures/kimi/wire-tool-shell.jsonl
    - claudine/lib/src/stream/protocol/fixtures/kimi/wire-subagent.jsonl
    - claudine/lib/src/stream/protocol/fixtures/kimi/wire-cancelled.jsonl
    - claudine/lib/src/stream/protocol/fixtures/kimi/wire-auth-expired.jsonl
    - claudine/features/2026-04-26-fix-kimi/phase-0/wire-driver.py
skills_files_updated_during_phase_0: []
packages_during_phase_0:
    - claudine
source_files_during_phase_1:
    - claudine/lib/src/stream/protocol/kimi.rs
    - claudine/lib/src/stream/mod.rs
    - claudine/lib/src/stream/reporting.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase1: []
packages_during_phase_1:
    - claudine
---

# Fix Kimi Wrapper Execution Plan

Source document:

- `claudine/features/_unscheduled/fix-kimi/spec.md`

Validated current seams:

- `claudine/cli/src/commands/wrap/profile.rs` still makes `KimiWrapper` append `--print`, return `StreamProtocol::StreamJson`, and deliver prompts through stdin or `--prompt`.
- `claudine/lib/src/stream/mod.rs` still maps `Provider::KimiCode` to `StreamProtocol::StreamJson`.
- `claudine/lib/src/stream/protocol/kimi.rs` is still a `#[serde(tag = "type")]` stream-json model.
- `claudine/lib/src/stream/kimi_semantic.rs` still reads top-level `type`, classifies legacy error shapes, and falls through to `ProviderExtension` for untyped lines.
- `claudine/cli/src/commands/wrap/live_semantic_sink.rs` already renders `OutputText`, `Reasoning`, `ToolCall`, `ToolResult`, `Info`, `Warning`, and typed `Error`; changes there should stay narrow.

## Phase Index

| Phase | Outcome | Depends on |
| --- | --- | --- |
| 0 | Wire protocol assumptions and fixture capture path are locked | none |
| 1 | Kimi wire JSON-RPC protocol types compile and cover captured envelope shapes | 0 |
| 2 | Kimi semantic parser consumes wire envelopes and emits complete semantic events and summaries | 1 |
| 3 | CLI wire IO loop can initialize Kimi, send prompts, answer requests, and stream events | 1, 2 |
| 4 | Kimi wrapper and composition paths route non-interactive runs through wire mode | 3 |
| 5 | Hook requests, live-sink drift handling, and error display polish are complete | 2, 3 |
| 6 | End-to-end regression, docs, and drift-maintenance updates are complete | 4, 5 |

## Phase 0: Lock Protocol Evidence

Outcome: implementation starts from real Kimi wire-mode traffic and explicit current-code constraints.

Files:

- `claudine/docs/research/non-interactive-sessions/kimi.md`
- `claudine/lib/src/stream/protocol/fixtures/kimi/*.jsonl`
- `claudine/features/_unscheduled/fix-kimi/plan.md`

Steps:

- [x] Confirm installed `kimi` supports `--wire` and note the observed protocol version, binary version, and auth state. *(`kimi 1.38.0`, protocol `1.9` — note: not `1.8` as the spec assumed; OAuth credentials present at `~/.kimi/credentials/kimi-code.json`.)*
- [x] Write or use a scratch driver that sends `initialize` and `prompt` JSON-RPC lines to `kimi --wire`, flushes each write, and records stdout verbatim. *(`claudine/features/2026-04-26-fix-kimi/phase-0/wire-driver.py`.)*
- [x] Capture `wire-greet.jsonl` with the deterministic prompt `"Hi how are you? My name is Bob."`.
- [x] Capture `wire-tool-shell.jsonl` with a shell/tool prompt that produces `ToolCall`, `ApprovalRequest`, and `ToolResult`. *(Captured complete tool lifecycle: `ToolCall`, streamed `ToolCallPart` deltas, `ApprovalRequest` JSON-RPC request, sent approve response, `ApprovalResponse` event echo, `ToolResult`.)*
- [x] Capture `wire-subagent.jsonl` only if the local Kimi build and account can produce `SubagentEvent`; otherwise create a clearly marked synthetic fixture from the wire schema and track the missing real capture in the plan notes. *(Captured live with the `explore` subagent; nested `SubagentEvent` envelopes carry `TurnBegin`, `StepBegin`, `ContentPart`, `ToolCall`, `ToolCallPart`, `ToolResult`, `StatusUpdate`, `TurnEnd`. Parent-stream `ApprovalRequest`s arrive un-nested for tool calls inside the subagent — confirms the spec's "forwarded approvals" note.)*
- [x] Capture `wire-auth-expired.jsonl` and `wire-cancelled.jsonl` if feasible without damaging the user's active auth state; otherwise create minimal schema-valid synthetic fixtures and record why real capture was skipped. *(`wire-cancelled.jsonl` captured live; `wire-auth-expired.jsonl` is synthetic — real capture skipped to avoid invalidating the user's active OAuth session. Synthetic shape mirrors `kimi_cli/wire/jsonrpc.py::ErrorCodes::AUTH_EXPIRED = -32004` returned as a `prompt` request error response.)*
- [x] Update the Kimi research doc only if the capture contradicts or materially refines the existing wire-mode documentation. *(Material refinements: protocol bumped to `1.9`; `ClientCapabilities` only carries `supports_question` and `supports_plan_mode` — none of `approvals`/`hooks`/`subagents`/`plan_mode`; the `prompt` request takes `params.user_input`, not `params.prompt`; the actual Event union has no `MessageStart`/`MessageDelta`/`MessageEnd`/`Thinking`/`Cancelled`/top-level `Error`/`DiffDisplayBlock` events — instead reasoning and assistant text stream as `ContentPart` events with inner `payload.type: "think"|"text"`, diffs ride inside `ApprovalRequest.display`, and cancellation surfaces as `result.status == "cancelled"` on the prompt response. Phase 1 protocol modeling and Phase 2 semantic mapping must be re-aligned to these shapes.)*

### Phase 0 capture-derived findings (must inform Phase 1+)

- Protocol version is `1.9`, not `1.8`. Initialize must declare `1.9`.
- `ClientCapabilities` has only `supports_question: bool` and `supports_plan_mode: bool`. Drop `approvals`, `hooks`, `subagents`, `plan_mode` from the spec's capability set; they are silently ignored by Pydantic but should not be modeled.
- `prompt` request param shape is `{user_input: str | list[ContentPart]}`. Sending `{prompt: ...}` is rejected with `-32602`.
- Final `prompt` response `result.status` is one of `Statuses` = `finished` / `cancelled` / `max_steps_reached` / `steered` (per `kimi_cli/wire/jsonrpc.py`). This is the canonical end-of-turn signal — there is no `TurnEnd.status` field in the captures (`TurnEnd.payload == {}`).
- The Event union contains: `TurnBegin`, `SteerInput`, `TurnEnd`, `StepBegin`, `StepInterrupted`, `CompactionBegin`, `CompactionEnd`, `MCPLoadingBegin`, `MCPLoadingEnd`, `StatusUpdate`, `Notification`, `PlanDisplay`, `ContentPart`, `ToolCall`, `ToolCallPart`, `ToolResult`, `ApprovalResponse`, `SubagentEvent`, `BtwBegin`, `BtwEnd`. **No** `MessageStart`/`MessageDelta`/`MessageEnd`/`Thinking`/`HookTriggered`/`HookResolved` (HookTriggered/HookResolved exist in the type module but were not observed in our captures and are server-side only without `hooks` subscriptions in initialize)/`DiffDisplayBlock`/`Cancelled`/top-level `Error`. Phase 1 protocol module and Phase 2 semantic mapping must be re-aligned: assistant reasoning and assistant text are streamed as `ContentPart` with discriminator `payload.type` ∈ {`think`, `text`, `image_url`, `audio_url`, `video_url`}, and diffs ride inside `ApprovalRequest.display`.
- `ToolCall` arrives once (often with empty `function.arguments`); subsequent `ToolCallPart` events stream `arguments_part` string deltas that must be concatenated and JSON-parsed for the `KimiToolCall::parse_arguments_string` helper required by Phase 1.
- `ApprovalResponse` is *also* a notification event (kimi echoes the resolution back on the stream after the client replies), distinct from the JSON-RPC response the client sent.
- Cancellation pipeline: client sends `cancel` request → kimi replies `{result: {}}` → kimi emits `TurnEnd` event → kimi returns the originating `prompt` request with `{result: {status: "cancelled"}}`. There is no `Cancelled` event.
- Auth-expired error code is `-32004` and surfaces as a JSON-RPC error on the `prompt` request, not a notification.

Parallelizable:

- Fixture capture for greet/tool/cancel can run independently once the scratch driver exists.
- Research-doc verification can run in parallel with fixture capture.

Validation checkpoint:

- `kimi --wire` accepts JSON-RPC line input and produces JSON-RPC line output.
- `jq -c . claudine/lib/src/stream/protocol/fixtures/kimi/*.jsonl >/dev/null`
- Fixture lines include at least one `method: "event"` notification and one request/response path, or the plan notes explain the unavailable shape.

## Phase 1: Wire Protocol Models

Outcome: `protocol/kimi.rs` models JSON-RPC envelopes and typed Kimi wire event/request payloads without denying unknown fields.

Files:

- `claudine/lib/src/stream/protocol/kimi.rs`
- `claudine/lib/src/stream/protocol/mod.rs`
- `claudine/lib/src/stream/mod.rs`
- `claudine/lib/src/stream/protocol/fixtures/kimi/*.jsonl`

Steps:

- [x] Replace the legacy `KimiEvent` stream-json enum with `KimiEnvelope`, `KimiEvent`, `KimiRequest`, response, and error-envelope types for Kimi wire mode. *(Added new wire types `KimiEnvelope`, `KimiWireEvent`, `KimiWireRequest`, `KimiJsonRpcError`, `KimiNotificationParams`, `KimiRequestParams`, `KimiInitializeResult`, `KimiPromptResult` alongside the legacy stream-json types so the in-flight `kimi_semantic.rs` parser keeps compiling. Phase 2 will swap the parser onto the new types and drop the legacy enum.)*
- [x] Model envelope dispatch for `method: "event"`, `method: "request"`, successful responses, and error responses while preserving raw `serde_json::Value` for fallbacks and summaries. *(`KimiEnvelope::classify(value)` walks the four shapes via an internal `KimiRawEnvelope`; raw `Value` is preserved on `Notification.payload`, `Request.params.payload`, `SuccessResponse.result`, and `ErrorResponse.error.data`.)*
- [x] Add typed event payloads for `TurnBegin`, `TurnEnd`, `StatusUpdate`, `Notification`, `PlanDisplay`, `MessageStart`, `MessageDelta`, `MessageEnd`, `Thinking`, `ToolCall`, `ToolCallPart`, `ToolResult`, `SubagentEvent`, `HookTriggered`, `HookResolved`, `DiffDisplayBlock`, `Cancelled`, and `Error`. *(Aligned with the Phase 0 capture findings: `MessageStart`/`MessageDelta`/`MessageEnd`/`Thinking`/`Cancelled`/top-level `Error` are absent from the wire union — assistant text and reasoning stream as `ContentPart`, cancellation surfaces as `prompt` response `result.status == "cancelled"`. Added `KimiContentPart`, `KimiToolCall`, `KimiToolCallPart`, `KimiToolResult`, `KimiSubagentEvent`, `KimiApprovalResponseEvent`, `KimiTurnBegin`/`KimiTurnEnd`/`KimiStepBegin`/`KimiStepInterrupted`/`KimiSteerInput`/`KimiCompactionBegin`/`KimiCompactionEnd`/`KimiMcpLoadingBegin`/`KimiMcpLoadingEnd`/`KimiWireStatusUpdate`/`KimiWireNotification`/`KimiPlanDisplay`/`KimiBtwBegin`/`KimiBtwEnd`/`KimiHookTriggered`/`KimiHookResolved`/`KimiDiffDisplayBlock`.)*
- [x] Add typed request payloads for `ApprovalRequest`, `QuestionRequest`, `ToolCallRequest`, and `HookRequest`. *(`KimiApprovalRequest`, `KimiQuestionRequest`, `KimiToolCallRequest`, `KimiHookRequest` dispatched through `KimiWireRequest`.)*
- [x] Put `#[serde(default)]` on every payload field and avoid `#[serde(deny_unknown_fields)]`. *(Verified across all wire payload structs.)*
- [x] Add helper methods for aliases and derived data: resolved ids, messages, status, context percent, function names, raw argument strings, and parsed tool arguments. *(Added `KimiTurnBegin::user_input_text`, `KimiContentPart::is_thinking`/`is_text`/`resolved_text`, `KimiWireStatusUpdate::computed_context_percent`, `KimiWireTokenUsage::total_input`/`cache_read_input`, `KimiToolCall::resolved_tool_id`/`resolved_tool_name`/`take_arguments_string`/`parse_arguments_string`, `KimiToolResult::resolved_tool_id`/`is_error`/`take_output`/`derived_status`, `KimiApprovalRequest::shell_command`, `KimiSubagentEvent::nested_event`, plus `KimiJsonRpcError` code constants and `KimiPromptResult` status constants.)*
- [x] Add `StreamProtocol::WireJsonRpc` and update Kimi protocol lookup functions to return it. *(Added the variant in `claudine/lib/src/stream/mod.rs`, updated `stream_protocol_for(KimiCode)` to return `WireJsonRpc`, extended `claudine/lib/src/stream/reporting.rs::summary_to_event_meta` match arm with `"wire-json-rpc"` string, and added the round-trip case in the `stream_protocol_serde_round_trip` test plus a `stream_protocol_for_supported_providers` assertion for KimiCode.)*
- [x] Delete or rewrite legacy Kimi protocol tests that assert synthetic top-level `type` fixtures. *(Kept the legacy stream-json tests intact for Phase 1 because the in-flight `kimi_semantic.rs` parser still references the legacy types; Phase 2 will rewrite the parser and remove the legacy types and tests together.)*
- [x] Add protocol unit tests for each captured fixture shape, alias helper, malformed tool-argument fallback, unknown event type failure, and unknown request type failure. *(Added 35 new wire-mode tests covering envelope classification for all four shapes, typed event/request decoding, every helper method, the `parse_arguments_string` empty/valid/malformed cases, unknown-event/request-type failures, and full fixture replays — `wire-greet.jsonl`, `wire-tool-shell.jsonl`, `wire-subagent.jsonl`, `wire-cancelled.jsonl`, `wire-auth-expired.jsonl`. All 50 tests in `stream::protocol::kimi::tests` pass.)*

Parallelizable:

- Event payload modeling and request payload modeling can proceed in parallel after `KimiEnvelope` is defined.
- Helper-method tests can be written in parallel with fixture-deserialization tests.

Validation checkpoint:

- `cargo test -p claudine stream::protocol::kimi`
- `cargo check -p claudine`

## Phase 2: Semantic Parser Rewrite

Outcome: `KimiSemanticStreamParser` turns wire envelopes into first-class `SemanticEvent`s and a populated `StreamExecutionSummary`.

Files:

- `claudine/lib/src/stream/kimi_semantic.rs`
- `claudine/lib/src/stream/semantic.rs`
- `claudine/lib/src/stream/summary.rs`
- `claudine/lib/src/stream/token_usage.rs`
- `claudine/lib/tests/semantic_fidelity.rs`
- `claudine/lib/tests/kimi_wire.rs` or equivalent integration test

Steps:

- [ ] Refactor `feed_line` to parse raw `serde_json::Value` first, then typed `KimiEnvelope`, preserving malformed-line behavior and raw fallback payloads.
- [ ] Track per-run state for `session_id`, `model`, token usage, cost, context usage, duration, status, assistant text, turn count, tool calls, and badges.
- [ ] Track per-turn state for message buffers and partial tool calls; reset on `TurnBegin` and flush on `TurnEnd`.
- [ ] Map `MessageDelta` to `SemanticEvent::OutputText` and append it to `assistant_text`; ensure completed messages do not concatenate without whitespace.
- [ ] Map `Thinking` to `SemanticEvent::Reasoning`.
- [ ] Map `ToolCall` and accumulated `ToolCallPart` data to `SemanticEvent::ToolCall` with structured JSON input when argument decoding succeeds and string passthrough when it fails.
- [ ] Map `ToolResult` to `SemanticEvent::ToolResult`, correlated by `tool_call_id` where possible.
- [ ] Map `Notification`, `PlanDisplay`, `SubagentEvent`, `HookTriggered`, `HookResolved`, and `DiffDisplayBlock` to typed `SemanticEvent::Info` with stable `extra.kind` values.
- [ ] Preserve the current 80% context-pressure warning behavior from `CONTEXT_PRESSURE_WARN_PERCENT`.
- [ ] Rewrite `classify_error` for JSON-RPC error codes, notification-shaped errors, auth/provider/rate-limit/billing keywords, and cancel/abort as `Interrupted`.
- [ ] Make request envelopes visible as `Info` or `Warning` events even before the CLI response path is wired.
- [ ] Update `finish` so Kimi summaries report `session_id`, `model`, `assistant_text`, `token_usage`, `cost_usd`, `duration_ms`, `num_turns`, `tool_calls`, `context_usage`, and provider status when present.
- [ ] Add fixture replay tests that assert event kinds, assistant text, tool call/result counts, warning/error kinds, and final summary fields.

Parallelizable:

- Summary aggregation and event mapping can proceed in parallel once the typed envelope dispatch exists.
- Error-classification tests can be developed independently of tool-call accumulation.

Validation checkpoint:

- `cargo test -p claudine stream::kimi_semantic`
- `cargo test -p claudine --test semantic_fidelity`
- `cargo test -p claudine --test kimi_wire`

## Phase 3: Wire IO Loop

Outcome: the CLI has a Kimi-local JSON-RPC line transport that owns initialize, prompt delivery, auto-responses, cancellation, and event forwarding.

Files:

- `claudine/cli/src/commands/wrap/wire_io.rs`
- `claudine/cli/src/commands/wrap/mod.rs`
- `claudine/cli/src/commands/wrap/exec.rs`
- `claudine/cli/src/commands/wrap/stream_io.rs`
- `claudine/cli/src/commands/wrap/live_semantic_sink.rs`

Steps:

- [ ] Add `wire_io.rs` beside `stream_io.rs` with a Kimi-specific child-spawn and JSON-RPC line loop.
- [ ] Send `initialize` with Claudine client metadata and capabilities: approvals true, questions false, hooks true, subagents true, plan_mode false.
- [ ] Validate initialize response protocol compatibility and convert failures into terminal `SemanticEvent::Error { kind: Configuration }`.
- [ ] Send the resolved prompt via a JSON-RPC `prompt` request after initialize completes; do not pass the prompt through `--prompt` or stdin seed.
- [ ] Read Kimi stdout line-by-line, feed each line through the Kimi semantic parser, and forward events to the existing live sink and JSONL logging path.
- [ ] Implement one serialized writer path for responses to Kimi stdin and flush after every line.
- [ ] Auto-respond to `ApprovalRequest` with approve and emit visible `auto_approved` info.
- [ ] Auto-respond to unexpected `QuestionRequest` with empty synthetic answers and emit a warning.
- [ ] Auto-respond to unsupported `ToolCallRequest` with JSON-RPC method-not-supported error and emit a warning.
- [ ] Route `HookRequest` through the existing Claudine dispatch pipeline and return a schema-valid hook response.
- [ ] Close stdin after the prompt turn ends and wait for natural child exit.
- [ ] On Ctrl+C or timeout-driven cancellation, send `cancel`, flush, then tear down the child using existing termination handling.
- [ ] Add tracing spans around initialize, prompt send, request dispatch, response write, parser feed, and cancellation.

Parallelizable:

- Initialize/prompt handshake and request auto-response builders can be implemented in parallel after the wire module skeleton exists.
- Cancellation wiring can proceed in parallel with hook-request dispatch once the writer path is available.

Validation checkpoint:

- Unit tests for JSON-RPC request builders and response builders.
- `cargo test -p claudine-cli wire_io`
- Manual smoke with a fake child or fixture-backed harness that verifies writes are newline-delimited and flushed.

## Phase 4: Wrapper Routing

Outcome: every non-interactive Kimi run launched by wrappers or composition uses wire mode with no legacy print-mode branch.

Files:

- `claudine/cli/src/commands/wrap/profile.rs`
- `claudine/cli/src/commands/wrap/mod.rs`
- `claudine/cli/src/commands/wrap/composition.rs`
- `claudine/cli/src/commands/wrap/sequence.rs`
- `claudine/cli/src/commands/wrap/exec.rs`
- `claudine/cli/tests/wrap_commands.rs`

Steps:

- [ ] Add `PromptDelivery::WireRpc(String)` or an equivalent typed prompt-delivery path.
- [ ] Update prompt-delivery dispatch so `WireRpc` invokes `wire_io` instead of stdin seeding or argv insertion.
- [ ] Change `KimiWrapper::apply_entrypoint` to append `--wire` for non-interactive runs and leave interactive runs unchanged.
- [ ] Change `KimiWrapper::apply_structured_stream` to add `--wire` only; remove `--print` and `--output-format stream-json` for Claudine-managed Kimi structured runs.
- [ ] Change `KimiWrapper::stream_protocol` and provider lookup to `StreamProtocol::WireJsonRpc`.
- [ ] Change `KimiWrapper::prompt_delivery` so non-interactive prompts use `WireRpc` and interactive prompts keep existing native behavior.
- [ ] Change `KimiWrapper::build_resume_args` from trailing `--print` to `--wire`.
- [ ] Keep `--yolo`, system prompt `--agent-file`, allowed env keys, and resume session-id semantics unchanged.
- [ ] Update direct wrapper, `compose`, `inline-compose`, and `sequence` launch paths to branch on `WireJsonRpc` only for Kimi and preserve existing structured paths for other providers.
- [ ] Remove any Kimi print-mode assumptions from resume normalization and prompt-arg convention tests.

Parallelizable:

- Profile changes and CLI dispatch changes can proceed in parallel once `PromptDelivery::WireRpc` is defined.
- Composition/sequence routing tests can be written in parallel with direct wrapper tests.

Validation checkpoint:

- `cargo test -p claudine-cli --test wrap_commands kimi`
- `cargo test -p claudine-cli wrap`
- Manual command construction checks show `claudine kimi -p "hi"` and Kimi-resolved composition include `--wire` and do not include `--print`.

## Phase 5: Live Sink, Hook Dispatch, And Error Polish

Outcome: wire-only Kimi events are visible, noisy drift events stay out of stderr, and hook/request outcomes are understandable to users.

Files:

- `claudine/cli/src/commands/wrap/live_semantic_sink.rs`
- `claudine/lib/src/adapters/kimicode.rs`
- `claudine/lib/src/dispatch/**`
- `claudine/lib/src/stream/kimi_semantic.rs`
- `claudine/cli/tests/wrap_commands.rs`

Steps:

- [ ] Add defensive Kimi entries to `SILENT_PROVIDER_EXTENSION_KINDS` for high-volume wire fallback kinds such as message deltas or tool-call parts.
- [ ] Ensure Kimi `Notification` info renders as a readable block quote or equivalent existing info surface without raw JSON.
- [ ] Ensure `PlanDisplay`, subagent, hook, auto-approval, unexpected-question, unsupported-external-tool, and diff-display events render as compact status/info lines.
- [ ] Confirm `HookRequest.payload.event` maps through `adapters::kimicode` to Claudine canonical events.
- [ ] Define the hook response conversion from dispatch outcomes, including allow, deny, and mutation cases if Kimi supports mutation in the payload.
- [ ] Ensure hook dispatch errors are returned to Kimi as JSON-RPC errors or hook-deny responses and are also surfaced as `SemanticEvent::Error` or `Warning`.
- [ ] Add tests proving known Kimi wire events do not leak to visible `ProviderExtension`, while unknown events remain logged as `ProviderExtension`.

Parallelizable:

- Live-sink rendering tests and hook-dispatch mapping tests can proceed independently after Phase 2 event shapes are stable.

Validation checkpoint:

- `cargo test -p claudine-cli live_semantic_sink`
- `cargo test -p claudine canonical_dispatch kimi`
- Fixture replay confirms zero visible `ProviderExtension` events for covered Kimi wire fixtures.

## Phase 6: End-To-End Validation And Drift Maintenance

Outcome: the feature is proven through unit tests, fixture replay, CLI smoke tests, and documentation updates.

Files:

- `claudine/docs/research/non-interactive-sessions/kimi.md`
- `claudine/docs/topics/composition.md`
- `claudine/cli/README.md`
- `.claude/skills/claudine/SKILL.md`
- `claudine/features/_unscheduled/fix-kimi/plan.md`

Steps:

- [ ] Run direct Kimi smoke tests: `claudine kimi -p "Hi how are you? My name is Bob."` and verify assistant text, reasoning if present, final summary, session id, model, and token telemetry.
- [ ] Run tool smoke tests with `claudine kimi --yolo -p "Run ls and tell me what you see"` and verify visible auto-approval, tool call, tool result, assistant text, and summary tool count.
- [ ] Run composition smoke tests for `compose`, `inline-compose`, and `sequence` where resolution selects Kimi; verify the same live-sink contract and closure behavior.
- [ ] Run resume smoke with a Kimi session id and verify generated args use `--wire`.
- [ ] Run cancellation smoke and verify Claudine sends `cancel`, exits cleanly, and reports `SemanticErrorKind::Interrupted`.
- [ ] Run auth/configuration failure smoke when safely possible and verify `SemanticErrorKind::Configuration` with actionable stderr.
- [ ] Update public docs if the user-visible Kimi behavior, JSONL event shape, or non-interactive support table changes.
- [ ] Update `.claude/skills/claudine/SKILL.md` because this changes wrapper architecture and Kimi support behavior.
- [ ] Record any fixture capture gaps or protocol-version caveats in this plan before moving the feature out of `_unscheduled`.

Parallelizable:

- Documentation updates can proceed while the final test matrix runs, once the wrapper behavior is stable.
- Direct wrapper and composition smoke tests can run independently if they use separate temporary output files and sessions.

Validation checkpoint:

- `cargo test -p claudine stream::protocol::kimi`
- `cargo test -p claudine stream::kimi_semantic`
- `cargo test -p claudine --test semantic_fidelity`
- `cargo test -p claudine --test kimi_wire`
- `cargo test -p claudine-cli --test wrap_commands kimi`
- `cargo test -p claudine-cli live_semantic_sink`
- `cargo check -p claudine -p claudine-cli`
- Manual smoke matrix passes for direct wrapper, composition, tool call, resume, cancellation, and failure classification.
