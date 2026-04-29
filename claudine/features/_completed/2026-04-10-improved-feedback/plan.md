# Plan: Improved Feedback

## Summary

Implement the `current-spec.md` slice by fixing parser correctness first for Claude, Codex, Goose, and Qwen, then tightening the existing wrapper reporting path without introducing a new UI contract. The work should stay inside the current `StreamParser` -> `StreamChunk` / `StreamEventSink` -> wrapper summary pipeline.

This plan intentionally does **not** take on:

- Kimi Wire-mode migration
- Roo stream support
- hook/stream fusion
- pricing-table cost estimation
- a new checklist, plan-panel, or nested-tree UI abstraction

## Design Constraints

1. Prefer provider-authored stream contracts already captured in `claudine/docs/research/non-interactive-sessions/`.
2. Keep `StreamExecutionSummary` compact. Add only fields that are cross-provider enough to justify normalization.
3. Reuse existing surfaces:
   - `StreamChunk::Text`
   - `StreamChunk::Thinking`
   - `StreamEventSink::{on_before_tool,on_after_tool,on_permission_request,on_subagent_start,on_subagent_stop,on_warning,on_turn_error,on_session_start}`
4. Preserve provider-specific details for reporting under provider-summary metadata instead of adding one-off top-level summary fields.

## Files In Scope

### Core stream layer

- `claudine/lib/src/stream/mod.rs`
- `claudine/lib/src/stream/parser.rs`
- `claudine/lib/src/stream/summary.rs`
- `claudine/lib/src/stream/reporting.rs`
- `claudine/lib/src/stream/claude.rs`
- `claudine/lib/src/stream/codex.rs`
- `claudine/lib/src/stream/qwen.rs`
- `claudine/lib/src/stream/goose.rs` (new)

### Wrapper/reporting layer

- `claudine/cli/src/commands/wrap/mod.rs`

No wrapper-profile changes should be required unless Goose structured mode wiring turns out to have an undiscovered gap. The current profile layer already requests structured output for Goose and the other targeted providers.

## Phase 1: Shared Plumbing And Summary Boundaries

### 1.1 Add Goose to the parser registry

Update `claudine/lib/src/stream/mod.rs` to:

- declare `pub mod goose;`
- route `Provider::Goose` through `create_parser()`
- return the real protocol from `stream_protocol_for(Provider::Goose)`
- expand the parser/protocol tests so Goose is no longer treated as unsupported

### 1.2 Add one provider-metadata bucket instead of more ad hoc fields

The current summary model has `raw_summary` for final-result payloads, but no clean place for session-start metadata such as Claude verbose `init` details. Add a single optional provider-metadata field to `StreamExecutionSummary`, for example:

```rust
pub provider_metadata: Option<serde_json::Value>
```

Use it only for extra provider/session metadata that should survive into reporting, not for normalized fields like `model` or `session_id`.

This change requires matching updates in:

- `claudine/lib/src/stream/summary.rs`
- `claudine/lib/src/stream/reporting.rs`
- serde/round-trip tests

### 1.3 Extend `RateLimitInfo` just enough for modern contracts

The current `RateLimitInfo` only models `is_throttled`, `retry_after_ms`, and `message`. Claude’s modern shape needs a stable home for fields like status and reset time. Extend it conservatively with optional generic fields such as:

- `status: Option<String>`
- `resets_at: Option<String>`

Keep the legacy fields so older providers or older payloads still fit.

### 1.4 Keep the sink contract unchanged in this slice

Do **not** add checklist-specific or provider-specific sink methods yet. Use:

- `StreamChunk::Thinking` for low-risk live progress/reporting text such as Codex `todo_list`, Qwen partial thinking, or Goose notifications
- existing sink callbacks for tool, subagent, permission, warning, and error lifecycles

That keeps the implementation aligned with the current spec’s “existing wrapper surfaces first” requirement.

## Phase 2: Claude Contract Fixes

**Primary file:** `claudine/lib/src/stream/claude.rs`

### 2.1 Parse `system` by subtype instead of treating all `system` messages as init

Refactor the current `"init" | "system"` match into a small subtype dispatcher:

- `system/init` -> session start metadata
- `system/api_retry` -> classified warning or turn error handling
- `system/files_persisted` -> execution-side notification handling
- any other supported subtype -> preserve metadata without corrupting session-start behavior

Only `init` should call `on_session_start`.

### 2.2 Preserve verbose init metadata

Capture fields like:

- `apiKeySource`
- `claude_code_version`
- `permissionMode`

Store them in `provider_metadata`, while still normalizing `session_id` and `model` into the existing top-level fields.

### 2.3 Update rate-limit parsing to the nested shape

Teach `handle_rate_limit()` to accept:

- `rate_limit_info.status`
- `rate_limit_info.resetsAt`
- legacy `is_throttled` / `retry_after_ms` when present

The parser should populate the normalized `RateLimitInfo` and emit a useful warning message for hard throttles or approaching-limit signals.

### 2.4 Classify `api_retry` errors cleanly

Map the modern Claude error enum into:

- actionable warnings for retry-like conditions
- `on_turn_error` for terminal failures such as billing/auth failures

The important outcome is that billing failures, auth failures, and generic execution errors are no longer collapsed together.

### 2.5 Surface `files_persisted`

Do not drop the event. Preserve the payload in metadata and emit a lightweight execution notification through the existing pipeline, ideally as a `Thinking`-style progress line unless the existing sink hooks prove to be a better fit.

### 2.6 Claude tests

Extend `claudine/lib/src/stream/claude.rs` tests to cover:

- `system/init` with verbose metadata retention
- `system/api_retry` billing/auth/rate-limit classification
- nested `rate_limit_info`
- `files_persisted`
- continued `thinking_delta` support

## Phase 3: Codex Schema Refresh

**Primary file:** `claudine/lib/src/stream/codex.rs`

### 3.1 Replace stale item-name assumptions

Update the item-type handling to match the current `exec_events.rs` projection:

- `command_execution`
- `file_change`
- `mcp_tool_call`
- `collab_tool_call`
- `web_search`
- `todo_list`
- `reasoning`
- `error`

Remove dependence on stale names such as `command_exec` and `patch_apply` except possibly as backwards-compatible aliases if they are already covered by tests.

### 3.2 Handle `item.updated`

Add explicit handling for `item.updated`, especially for:

- `todo_list`
- `reasoning`
- any live status-bearing items whose latest state should replace the earlier snapshot

For this slice, the most pragmatic output path is:

- `todo_list` -> render compact checklist text through `StreamChunk::Thinking`
- `reasoning` -> render as `StreamChunk::Thinking`

This keeps live observability without inventing a new renderer contract.

### 3.3 Map `collab_tool_call` to subagent lifecycle

Use:

- `item.started` -> `on_subagent_start`
- `item.completed` -> `on_subagent_stop`

Continue to use the ordinary tool callbacks for ordinary tool items.

### 3.4 Treat `turn.failed` as a first-class error path

Make `turn.failed` populate `is_error`, `error_kind`, `error_message`, and `provider_status`, then emit `on_turn_error`.

### 3.5 Keep token math conservative

When parsing usage snapshots:

- preserve `cached_input_tokens` in `cache_read`
- avoid inventing totals beyond what Codex provides
- prefer provider-supplied `total_tokens` when present, otherwise compute only the obvious `input + output`

### 3.6 Codex tests

Expand `claudine/lib/src/stream/codex.rs` tests to cover:

- `command_execution`
- `file_change`
- `mcp_tool_call`
- `collab_tool_call`
- `web_search`
- `todo_list`
- `reasoning`
- `item.updated`
- `turn.failed`

Also add assertions that subagent lifecycle callbacks fire for `collab_tool_call`.

## Phase 4: Add A Real Goose Parser

**Primary file:** `claudine/lib/src/stream/goose.rs` (new)

### 4.1 Implement `GooseStreamParser`

Parse the current top-level Goose envelope:

- `message`
- `notification`
- `error`
- `complete`

The parser should not assume Claude-compatible shapes anywhere.

### 4.2 Handle flattened notifications correctly

Goose notifications are flattened at the top level. Parse fields like:

- `notification_type`
- `message`
- `progress`
- `total`
- `extension_id`

Recommended behavior in this slice:

- generic progress/log notifications -> `StreamChunk::Thinking`
- `subagent_tool_request` -> `on_subagent_start`
- `tasks_complete` -> `on_subagent_stop`

### 4.3 Parse nested message content items

Inside `message.content[]`, support at least:

- text content -> assistant text / `StreamChunk::Text`
- `toolRequest` -> `on_before_tool`
- `toolResponse` -> `on_after_tool`
- `thinking` / system-like notification content -> `StreamChunk::Thinking` where appropriate

Handle the Goose camelCase nested keys exactly as documented in the research notes.

### 4.4 Capture final totals from `complete`

Map `complete.total_tokens` into `token_usage.total`, preserve the full payload in `raw_summary`, and keep the final provider status if Goose exposes one.

### 4.5 Goose tests

Create inline tests in `claudine/lib/src/stream/goose.rs` covering:

- each top-level event type
- flattened notifications
- nested tool request/response extraction
- subagent start/stop notifications
- `complete.total_tokens`

Also update `stream/mod.rs` tests so Goose is treated as a supported structured provider.

## Phase 5: Qwen Partial-Stream Support

**Primary file:** `claudine/lib/src/stream/qwen.rs`

### 5.1 Parse `stream_event`

Add a dedicated handler for `type: "stream_event"` and support at least:

- `message_start`
- `content_block_start`
- `content_block_delta`
- `content_block_stop`
- `message_stop`
- `tool_progress`

### 5.2 Support partial thinking and text without regressing whole-message handling

Keep the current full-message path for ordinary assistant messages, but layer in partial handling so:

- text deltas append to assistant output
- thinking deltas emit `StreamChunk::Thinking`
- tool-progress events emit compact `Thinking`-style progress text

### 5.3 Preserve final permission denials

Ensure `result.permission_denials` survives into reporting by preserving it in `raw_summary`. If the parser currently rebuilds or trims the result payload in any path, stop dropping this field.

### 5.4 Avoid depending on buffered-only stats

Do not wire the stream parser to `result.stats` fields that exist only in buffered JSON mode. Keep the stream parser aligned to what `stream-json` actually guarantees.

### 5.5 Qwen tests

Extend `claudine/lib/src/stream/qwen.rs` tests for:

- `stream_event` dispatch
- `content_block_delta` text and thinking
- `tool_progress`
- final `permission_denials`

## Phase 6: Wrapper Reporting Polish

**Primary file:** `claudine/cli/src/commands/wrap/mod.rs`

This phase stays deliberately small. It should consume the richer parser output rather than inventing a new renderer.

### 6.1 Improve tool-result formatting

Update `format_tool_result_line()` and, if needed, `format_tool_progress_line()` so file-change and command-execution style items include:

- stable status
- compact result context
- errors without dumping huge payloads

This is especially important once Codex and Goose start feeding richer tool metadata through the existing callbacks.

### 6.2 Improve verbose summary details

Update `format_verbose_summary_details_prose()` to surface:

- provider session ID when present
- model identity
- stop reason
- selected provider metadata only when it materially helps debugging

Do not dump raw JSON into the normal summary line. Keep the default view compact.

### 6.3 Ensure provider-summary logging keeps the new metadata

Update `claudine/lib/src/stream/reporting.rs` so synthetic summary events include:

- `provider_metadata` / session metadata
- expanded rate-limit fields
- existing `raw_summary`

This is the main mechanism for preserving Claude init metadata, Qwen permission denials, and Goose/Codex provider extras for later reporting.

### 6.4 Add wrap-layer tests where formatting changed

If `wrap/mod.rs` already has helper tests for summary/tool formatting, extend them. If not, add focused unit tests for:

- compact tool-result formatting
- verbose summary details with provider metadata present

## Phase 7: Contract-Focused Regression Coverage

The parser layer is the drift point, so the regression suite should be explicit and provider-shaped.

### 7.1 Provider-specific fixture strategy

Keep the first implementation simple:

- inline JSON lines in parser tests are acceptable for the initial lock-in
- if the test payloads become too large, move them to per-provider fixture files later

The important part is to lock exact event names and field shapes, not the storage format of the tests.

### 7.2 Minimum regression matrix

- Claude:
  - `system/init`
  - `system/api_retry`
  - nested `rate_limit_info`
  - `files_persisted`
- Codex:
  - `command_execution`
  - `file_change`
  - `collab_tool_call`
  - `todo_list`
  - `item.updated`
  - `turn.failed`
- Goose:
  - `message`
  - flattened `notification`
  - `error`
  - `complete`
- Qwen:
  - `stream_event`
  - `content_block_delta`
  - `tool_progress`
  - `permission_denials`

### 7.3 Verification commands

Run focused tests first:

```bash
cargo test -p claudine stream::claude
cargo test -p claudine stream::codex
cargo test -p claudine stream::goose
cargo test -p claudine stream::qwen
cargo test -p claudine stream::reporting
```

Then run the broader crate test pass if the focused suites are green:

```bash
cargo test -p claudine
```

If the full crate pass is too slow while iterating, keep the parser-specific runs as the developer loop and use the full pass as the final gate.

## Recommended Delivery Order

1. Shared summary/model plumbing in `stream/mod.rs`, `summary.rs`, and `reporting.rs`
2. Claude parser fixes
3. Codex parser refresh
4. Goose parser implementation and registry wiring
5. Qwen partial-stream support
6. Wrapper/reporting polish
7. Final regression pass

This keeps the work biased toward confirmed correctness problems first, then closes the most obvious provider-enablement gap, then tightens presentation once the underlying signals are trustworthy.

## Key Risks And Mitigations

- **Risk:** Overfitting to old or mixed provider schemas.
  - **Mitigation:** Use only the current local research docs already curated under `claudine/docs/research/non-interactive-sessions/` and align tests directly to those shapes.

- **Risk:** Expanding the normalized summary too aggressively.
  - **Mitigation:** Add only one provider-metadata bucket plus the minimal `RateLimitInfo` expansion; keep provider-specific details inside provider-summary logging.

- **Risk:** Introducing a new UI contract while chasing provider-specific features.
  - **Mitigation:** Reuse `Thinking` chunks and existing sink callbacks in this slice; defer checklist/tree/status-bar abstractions.

- **Risk:** Goose notification handling becomes too ad hoc.
  - **Mitigation:** Restrict live output to generic progress text plus the two explicit subagent notification types already called out in the spec.
