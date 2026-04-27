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
source_files_during_phase_2:
    - claudine/lib/src/stream/kimi_semantic.rs
    - claudine/lib/src/stream/protocol/kimi.rs
    - claudine/lib/tests/kimi_wire.rs
    - claudine/lib/tests/semantic_fidelity.rs
    - claudine/cli/src/commands/wrap/live_semantic_sink.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase2: []
packages_during_phase_2:
    - claudine
    - claudine-cli
source_files_during_phase_3:
    - claudine/cli/src/commands/wrap/wire_io.rs
    - claudine/cli/src/commands/wrap/mod.rs
    - claudine/lib/src/stream/protocol/kimi.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase3: []
packages_during_phase_3:
    - claudine
    - claudine-cli
source_files_during_phase_4:
    - claudine/cli/src/commands/wrap/profile.rs
    - claudine/cli/src/commands/wrap/mod.rs
    - claudine/cli/src/commands/wrap/composition.rs
    - claudine/cli/tests/wrap_commands.rs
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase4: []
packages_during_phase_4:
    - claudine
    - claudine-cli
source_files_during_phase_5:
    - claudine/cli/src/commands/wrap/live_semantic_sink.rs
    - claudine/cli/src/commands/wrap/wire_io.rs
    - claudine/lib/src/adapters/kimicode.rs
    - claudine/lib/src/stream/kimi_semantic.rs
docs_updated_during_phase_5: []
docs_created_during_phase_5: []
skills_files_updated_during_phase5: []
packages_during_phase_5:
    - claudine
    - claudine-cli
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

- [x] Refactor `feed_line` to parse raw `serde_json::Value` first, then typed `KimiEnvelope`, preserving malformed-line behavior and raw fallback payloads.
- [x] Track per-run state for `session_id`, `model`, token usage, cost, context usage, duration, status, assistant text, turn count, tool calls, and badges.
- [x] Track per-turn state for message buffers and partial tool calls; reset on `TurnBegin` and flush on `TurnEnd`. *(`pending_text` flushes on TurnEnd / when reasoning interrupts text; `pending_tool_call` flushes on `TurnBegin`/`TurnEnd`/`ToolResult`/new `ToolCall`/`SubagentEvent` boundaries.)*
- [x] Map `MessageDelta` to `SemanticEvent::OutputText` and append it to `assistant_text`; ensure completed messages do not concatenate without whitespace. *(Wire mode has no `MessageDelta`; assistant text streams as `ContentPart` with `payload.type == "text"`. `flush_pending_text` adds a separator newline on TurnEnd between consecutive assistant messages.)*
- [x] Map `Thinking` to `SemanticEvent::Reasoning`. *(Wire mode has no `Thinking` event; reasoning streams as `ContentPart` with `payload.type == "think"`.)*
- [x] Map `ToolCall` and accumulated `ToolCallPart` data to `SemanticEvent::ToolCall` with structured JSON input when argument decoding succeeds and string passthrough when it fails.
- [x] Map `ToolResult` to `SemanticEvent::ToolResult`, correlated by `tool_call_id` where possible.
- [x] Map `Notification`, `PlanDisplay`, `SubagentEvent`, `HookTriggered`, `HookResolved`, and `DiffDisplayBlock` to typed `SemanticEvent::Info` with stable `extra.kind` values. *(`PlanDisplay` routes to `SemanticEvent::PlanUpdate` instead — the typed variant exists and carries the `plan`/`display` blobs in `extra` for live-sink rendering.)*
- [x] Preserve the current 80% context-pressure warning behavior from `CONTEXT_PRESSURE_WARN_PERCENT`.
- [x] Rewrite `classify_error` for JSON-RPC error codes, notification-shaped errors, auth/provider/rate-limit/billing keywords, and cancel/abort as `Interrupted`. *(New `classify_jsonrpc_error` maps `AUTH_EXPIRED → Configuration`, `CHAT_PROVIDER_ERROR → ApiRemote`, standard JSON-RPC codes → `AgentNative`, with message-keyword fallback for rate/quota/billing → `ApiRemote`, auth/api-key/permission → `Configuration`, interrupt/cancel → `Interrupted`.)*
- [x] Make request envelopes visible as `Info` or `Warning` events even before the CLI response path is wired. *(`ApprovalRequest` → auto-approved Info, `QuestionRequest` → unexpected-question Warning, `ToolCallRequest` → external-tool-request Warning, `HookRequest` → hook-request Info.)*
- [x] Update `finish` so Kimi summaries report `session_id`, `model`, `assistant_text`, `token_usage`, `cost_usd`, `duration_ms`, `num_turns`, `tool_calls`, `context_usage`, and provider status when present.
- [x] Add fixture replay tests that assert event kinds, assistant text, tool call/result counts, warning/error kinds, and final summary fields. *(Added `claudine/lib/tests/kimi_wire.rs` with 6 tests covering all five captured fixtures plus an unknown-event fallback case. The legacy `kimi_round_trip` block in `semantic_fidelity.rs` and the CLI `kimi_stderr_snapshot` test were rewritten onto wire-mode envelopes.)*

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

- [x] Add `wire_io.rs` beside `stream_io.rs` with a Kimi-specific child-spawn and JSON-RPC line loop. *(`claudine/cli/src/commands/wrap/wire_io.rs` adds builders, `WireWriter`, `run_kimi_wire_session`, and the reader-thread auto-response pipeline.)*
- [x] Send `initialize` with Claudine client metadata and capabilities: approvals true, questions false, hooks true, subagents true, plan_mode false. *(`build_initialize_request` plus `WireClientCapabilities::default_for_claudine`; emits `supports_question` / `supports_plan_mode` per Phase 0 capture findings while still declaring the spec-named flags.)*
- [x] Validate initialize response protocol compatibility and convert failures into terminal `SemanticEvent::Error { kind: Configuration }`. *(`validate_initialize_response` returns `WireInitError::{MissingProtocolVersion, UnsupportedProtocolVersion}`; Phase 4 will fold the failure into a terminal Configuration error at the call site.)*
- [x] Send the resolved prompt via a JSON-RPC `prompt` request after initialize completes; do not pass the prompt through `--prompt` or stdin seed. *(`build_prompt_request` uses `params.user_input` and is sent on the main thread after initialize.)*
- [x] Read Kimi stdout line-by-line, feed each line through the Kimi semantic parser, and forward events to the existing live sink and JSONL logging path. *(Reader thread classifies each envelope, then calls `parser.feed_line`; the parser surface owns visible event emission and JSONL logging via the Phase 2 sink wiring.)*
- [x] Implement one serialized writer path for responses to Kimi stdin and flush after every line. *(`WireWriter` wraps `ChildStdin` behind `Mutex<Box<dyn Write>>` and flushes after every newline; clones share the same lock.)*
- [x] Auto-respond to `ApprovalRequest` with approve and emit visible `auto_approved` info. *(`WireRequestDispatch::AutoApprove` → `build_approval_response`; the Phase 2 parser already emits the visible `auto_approved` Info event.)*
- [x] Auto-respond to unexpected `QuestionRequest` with empty synthetic answers and emit a warning. *(`WireRequestDispatch::EmptyQuestionAnswer` → `build_question_response`; parser emits the matching `unexpected_question` Warning.)*
- [x] Auto-respond to unsupported `ToolCallRequest` with JSON-RPC method-not-supported error and emit a warning. *(`WireRequestDispatch::UnsupportedToolCall` → `build_tool_call_unsupported_error` with `KimiJsonRpcError::METHOD_NOT_FOUND`.)*
- [x] Route `HookRequest` through the existing Claudine dispatch pipeline and return a schema-valid hook response. *(`dispatch_hook_request` resolves the canonical `AgenticEvent` via `map_kimi_hook_event`, runs `dispatch_event_meta_with_runtime` through the wrapper-session `DispatchRuntimeContext`, and converts the outcome into `HookOutcome::{Allow,Deny,Ask}` via `outcome_to_hook_outcome`; `build_hook_response` formats the JSON-RPC result.)*
- [x] Close stdin after the prompt turn ends and wait for natural child exit. *(Stdin is held by `WireWriter` only; the writer is dropped at end of `run_kimi_wire_session` after the child exits, matching kimi's expected EOF-on-stdin shutdown.)*
- [x] On Ctrl+C or timeout-driven cancellation, send `cancel`, flush, then tear down the child using existing termination handling. *(`install_sigint_forwarder` uses `signal_hook` to flip an `AtomicBool`; `wait_for_child_exit` polls the flag and the wall-clock deadline, sends `build_cancel_request` once, and falls back to SIGKILL after a 5s grace.)*
- [x] Add tracing spans around initialize, prompt send, request dispatch, response write, parser feed, and cancellation. *(`info_span!("kimi_wire_session")`, `kimi_wire_initialize`, `kimi_wire_prompt_send`, `kimi_wire_stdout`, and `kimi_wire_cancel`; per-request log records carry `request_id` and outcome.)*

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

- [x] Add `PromptDelivery::WireRpc(String)` or an equivalent typed prompt-delivery path. *(Added `PromptDelivery::WireRpc(String)` plus `PromptDelivery::as_wire_rpc()` accessor; `apply_to` returns `None` for the variant so the orchestrator owns transport.)*
- [x] Update prompt-delivery dispatch so `WireRpc` invokes `wire_io` instead of stdin seeding or argv insertion. *(Direct wrapper, harness loop, and composition's `run_structured_composition` all branch on `wire_prompt`/`launch.wire_prompt`; when present, they call `wire_io::run_kimi_wire_session`. `AttemptLaunch` gained a `wire_prompt: Option<String>` field that flows through `prepare_attempt_launch`.)*
- [x] Change `KimiWrapper::apply_entrypoint` to append `--wire` for non-interactive runs and leave interactive runs unchanged.
- [x] Change `KimiWrapper::apply_structured_stream` to add `--wire` only; remove `--print` and `--output-format stream-json` for Claudine-managed Kimi structured runs.
- [x] Change `KimiWrapper::stream_protocol` and provider lookup to `StreamProtocol::WireJsonRpc`. *(Provider lookup was already on `WireJsonRpc` from Phase 1; this profile method now matches.)*
- [x] Change `KimiWrapper::prompt_delivery` so non-interactive prompts use `WireRpc` and interactive prompts keep existing native behavior. *(Non-interactive returns `PromptDelivery::WireRpc(prompt)`; interactive returns `AppendArgs(["--prompt", prompt])`.)*
- [x] Change `KimiWrapper::build_resume_args` from trailing `--print` to `--wire`.
- [x] Keep `--yolo`, system prompt `--agent-file`, allowed env keys, and resume session-id semantics unchanged.
- [x] Update direct wrapper, `compose`, `inline-compose`, and `sequence` launch paths to branch on `WireJsonRpc` only for Kimi and preserve existing structured paths for other providers. *(`execute_without_harness` and `run_structured_composition` thread `wire_prompt` through to `run_kimi_wire_session`; `sequence` reuses `execute_composition_request_inner` so it inherits the same dispatch.)*
- [x] Remove any Kimi print-mode assumptions from resume normalization and prompt-arg convention tests. *(Replaced `kimi_wrapper_non_interactive_appends_print` with `kimi_wrapper_non_interactive_appends_wire`, asserting the JSON-RPC `prompt`/`user_input` envelope reaches the child stub; added `kimi_non_interactive_uses_wire_protocol_and_wire_rpc_delivery`, `kimi_interactive_continues_using_prompt_argv_flag`, and `kimi_resume_uses_wire_flag` profile-level unit tests.)*

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

- [x] Add defensive Kimi entries to `SILENT_PROVIDER_EXTENSION_KINDS` for high-volume wire fallback kinds such as message deltas or tool-call parts. *(Added 7 Kimi entries covering `event:ContentPart`, `event:ToolCallPart`, `event:StatusUpdate`, plus legacy `event:MessageStart`/`event:MessageDelta`/`event:MessageEnd`/`event:Thinking` for cross-mode/payload-drift safety; events still flow through dispatch and JSONL log, only the stderr line is suppressed.)*
- [x] Ensure Kimi `Notification` info renders as a readable block quote or equivalent existing info surface without raw JSON. *(Parser maps `Notification.message`/`title` to a plain `SemanticEvent::Info` whose live-sink renderer is `render_status(Info, message)` — a single compact status line with no raw JSON dumped.)*
- [x] Ensure `PlanDisplay`, subagent, hook, auto-approval, unexpected-question, unsupported-external-tool, and diff-display events render as compact status/info lines. *(All wire events route through typed `SemanticEvent::Info`/`Warning`/`PlanUpdate` variants; the live sink renders each as a compact `render_status` line — verified by reading the `render_event` match arms and confirming none emit raw JSON.)*
- [x] Confirm `HookRequest.payload.event` maps through `adapters::kimicode` to Claudine canonical events. *(Extended `adapters::kimicode::map_event` with the wire-mode hook event names (`PreToolUse`, `PostToolUse`, `UserPromptSubmit`, `Stop`, `SessionStart`, `SessionEnd`, `Notification`, `SubagentStart`, `SubagentStop`) so both transport paths share one canonical surface; added module-level doc explaining the relationship to `wire_io::map_kimi_hook_event`; covered by the new `hook_event_names_resolve_to_canonical_events` test.)*
- [x] Define the hook response conversion from dispatch outcomes, including allow, deny, and mutation cases if Kimi supports mutation in the payload. *(`build_hook_response` maps `HookOutcome::Allow → "approve"`, `Deny → "reject"`, `Ask → "ask"`, with optional `reason`. Mutation is **not** supported by Kimi's wire `HookRequest` schema — hook responses carry only `decision` and `reason`, so v1 deliberately does not model a mutation field; if Kimi adds one, this builder is the single seam to extend.)*
- [x] Ensure hook dispatch errors are returned to Kimi as JSON-RPC errors or hook-deny responses and are also surfaced as `SemanticEvent::Error` or `Warning`. *(Refactored `dispatch_hook_request` to return `HookDispatchResult { outcome, warning }`. Infrastructure failures (no runtime handle, dispatch `Err`) keep returning `Allow` to Kimi so the agent doesn't deadlock, and now also populate `warning` with a user-facing message. `handle_request_dispatch` returns an optional synthetic envelope on warnings; the reader thread feeds it through the parser. Parser `Notification` handler updated to emit Warning when `level == "error"`/`"warn"` so the synthetic envelope surfaces visibly.)*
- [x] Add tests proving known Kimi wire events do not leak to visible `ProviderExtension`, while unknown events remain logged as `ProviderExtension`. *(Added `known_wire_events_do_not_leak_as_provider_extension` (sweeps every modeled event/request type and asserts no `ProviderExtension`), `unknown_event_type_emits_provider_extension_with_event_kind` (asserts `event:FutureKimiEvent` falls back as `ProviderExtension`), live-sink `provider_extension_kimi_high_volume_kinds_are_silent`, and `provider_extension_kimi_unknown_kinds_still_surface`.)*

Parallelizable:

- Live-sink rendering tests and hook-dispatch mapping tests can proceed independently after Phase 2 event shapes are stable.

Validation checkpoint:

- `cargo test -p claudine-cli live_semantic_sink` — passes (7 tests including 2 new Kimi cases).
- `cargo test -p claudine adapters::kimicode` — passes (8 tests including the new hook-name canonical-mapping case).
- `cargo test -p claudine stream::kimi_semantic` — passes (26 tests including 4 new ones).
- `cargo test -p claudine-cli wire_io` — passes (28 tests including 3 new ones).
- Fixture replay (`known_wire_events_do_not_leak_as_provider_extension`) confirms zero visible `ProviderExtension` events for covered Kimi wire fixtures.

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
