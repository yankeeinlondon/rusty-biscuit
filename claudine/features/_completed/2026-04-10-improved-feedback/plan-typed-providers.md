# Implementation Plan: Strongly Typed Provider Protocol Models

## A. Executive Summary

Replace ~134 manual `serde_json::Value` field extraction sites across six stream parsers with serde-derived typed structs and enums in a new `stream/protocol/` module. Each parser's `feed_line()` method will deserialize into a tagged provider event enum instead of manual `.get().and_then()` chains. Handler signatures change from `fn handle_xxx(&mut self, obj: &Value)` to `fn handle_xxx(&mut self, event: XxxStruct)` with direct field access. A `serde_json::from_str::<Value>` fallback arm preserves the current behavior of silently skipping unknown event types. The migration is phased by provider pair (Claude+Codex, Gemini+OpenCode, Qwen+Kimi, Cleanup), with all existing tests passing unmodified at each phase boundary.

## B. Current State Audit

### B.1 Claude Parser (`stream/claude.rs`)

**File size**: 843 lines | **Parser struct**: `ClaudeStreamParser<S>` (19 fields)

**`feed_line()` structure** (lines 306–374):
1. Increment `line_num`, trim, skip empty
2. Parse as `serde_json::Value` (line 313)
3. Extract `event_type` via `obj.get("type").and_then(|t| t.as_str())` (line 323)
4. Dispatch on string match arms
5. Malformed JSON → `StreamParseError::MalformedLine`

**Handler methods and signatures**:

| Method | Lines | Signature |
|--------|-------|-----------|
| `tool_use_payload` | 70–72 | `fn(obj: &Value) -> &Value` |
| `handle_init` | 74–96 | `fn(&mut self, obj: &Value)` |
| `handle_assistant_message` | 98–120 | `fn(&mut self, obj: &Value) -> Option<StreamChunk>` |
| `handle_content_block_delta` | 122–137 | `fn(&mut self, obj: &Value) -> Option<StreamChunk>` |
| `handle_error` | 139–154 | `fn(&mut self, obj: &Value)` |
| `handle_result` | 156–213 | `fn(&mut self, obj: &Value)` |
| `handle_rate_limit` | 215–228 | `fn(&mut self, obj: &Value)` |
| `handle_tool_use` | 230–269 | `fn(&mut self, obj: &Value)` |
| `handle_tool_result` | 271–302 | `fn(&mut self, obj: &Value)` |

**`.get()` / `.and_then()` extraction sites**: ~45 (design doc estimated ~25)

**Special handling**:
- Dual assistant message format: `obj.get("message").and_then(|m| m.get("content")).or_else(|| obj.get("content"))` (lines 103–106)
- Tool use payload unwrapping: `obj.get("content_block").unwrap_or(obj)` (line 71) — `content_block_start` events with `type: "tool_use"` are treated as tool use events
- Dual cost field: `obj.get("total_cost_usd").or_else(|| obj.get("cost_usd"))` (lines 175–177)
- `content_block_delta` handles `text_delta`, `thinking_delta`, and `input_json_delta` subtypes (lines 125–136)
- Raw summary strips large arrays (`tools`, `skills`, `agents`, `mcp_servers`) before storing (lines 199–206)

**Test count**: 18 tests (design doc said 15)

| # | Test name | Key scenario |
|---|-----------|--------------|
| 1 | `happy_path_init_assistant_result` | Init → assistant → result with usage |
| 2 | `error_path_assistant_error` | `type: "error"` with billing_error |
| 3 | `rate_limit_event` | Rate limit with throttled + retry_after_ms |
| 4 | `malformed_line_recovery` | Bad JSON followed by valid line |
| 5 | `multi_turn_concatenation` | Two assistant messages concatenated |
| 6 | `tool_use_events_counted_and_dispatched` | Tool counting + sink dispatch |
| 7 | `tool_use_events_preserve_input_and_result_metadata` | Tool contract (id, name, input, response) |
| 8 | `content_block_tool_use_preserves_nested_input` | `content_block_start` with nested tool_use |
| 9 | `large_init_arrays_not_stored_in_summary` | Result strips tools/skills/agents/mcp_servers |
| 10 | `content_block_delta` | Streaming text deltas |
| 11 | `empty_lines_skipped` | Empty/whitespace lines |
| 12 | `unknown_event_types_skipped` | Unknown type silently skipped |
| 13 | `thinking_delta_emits_thinking_chunk` | `StreamChunk::Thinking` variant |
| 14 | `thinking_and_text_deltas_interleaved` | Thinking NOT accumulated into assistant_text |
| 15 | `finish_with_nonzero_exit_code` | Exit code propagation |
| 16 | `tool_calls_none_when_zero` | `tool_calls` is `None` not `Some(0)` |
| 17 | `total_cost_usd_field_name` | `total_cost_usd` vs `cost_usd` fallback |
| 18 | `assistant_message_nested_under_message_key` | `message.content` nested format |

### B.2 Codex Parser (`stream/codex.rs`)

**File size**: 497 lines | **Parser struct**: `CodexStreamParser<S>` (16 fields)

**`feed_line()` structure** (lines 304–366):
- Same pattern as Claude but uses `StreamParseError::Fatal` instead of `MalformedLine` for bad JSON (line 315) — **important difference**

**Handler methods and signatures**:

| Method | Lines | Signature |
|--------|-------|-----------|
| `session_meta` | 59–70 | `fn(&self) -> EventMeta` |
| `handle_thread_started` | 72–86 | `fn(&mut self, obj: &Value)` |
| `handle_turn_started` | 88–92 | `fn(&mut self)` |
| `handle_turn_completed` | 94–143 | `fn(&mut self, obj: &Value)` |
| `handle_error` | 145–169 | `fn(&mut self, obj: &Value)` |
| `handle_agent_message_item` | 177–192 | `fn(&mut self, item: &Value)` |
| `tool_meta_from_item` | 194–223 | `fn(&self, item: &Value) -> EventMeta` |
| `is_tool_item_type` | 225–237 | `fn(item_type: &str) -> bool` |
| `handle_item_started` | 239–268 | `fn(&mut self, obj: &Value)` |
| `handle_item_completed` | 270–300 | `fn(&mut self, obj: &Value) -> Option<StreamChunk>` |

**`.get()` extraction sites**: ~35

**Special handling**:
- Dual thread ID: `obj.get("thread_id").or_else(|| obj.get("id"))` (lines 73–77)
- Dual cache_read: `usage.get("cached_input_tokens").or_else(|| usage.get("cache_read_input_tokens"))` (lines 99–102)
- Error has three sources: `error_type` / `error.type` / `message` (lines 147–157)
- Agent message text accumulated but NOT emitted to stdout (line 177 doc comment)
- `item.completed` merges started+completed item data (lines 280–294)
- `is_tool_item_type` recognizes 8 tool item types (lines 226–237)
- Permission/approval/user_input_request items trigger `on_permission_request` (lines 245–252)

**Test count**: 4

| # | Test name | Key scenario |
|---|-----------|--------------|
| 1 | `happy_path_metadata_only` | Thread lifecycle + usage + agent message accumulation |
| 2 | `stream_accumulates_text_without_emitting` | Text stored but `feed_line` returns `None` |
| 3 | `error_handling` | `error_type` + `error_message` extraction |
| 4 | `tool_counting` | `item.tool_use` / `item.tool_result` pairs |

### B.3 Gemini Parser (`stream/gemini.rs`)

**File size**: 521 lines | **Parser struct**: `GeminiStreamParser<S>` (16 fields)

**`feed_line()` structure** (lines 270–314):
- Standard pattern, uses `MalformedLine` error (not Fatal)

**Handler methods and signatures**:

| Method | Lines | Signature |
|--------|-------|-----------|
| `handle_init` | 56–78 | `fn(&mut self, obj: &Value)` |
| `handle_message` | 80–109 | `fn(&mut self, obj: &Value) -> Option<StreamChunk>` |
| `handle_result` | 111–162 | `fn(&mut self, obj: &Value)` |
| `handle_error` | 164–192 | `fn(&mut self, obj: &Value)` |
| `handle_tool_use` | 194–232 | `fn(&mut self, obj: &Value)` |
| `handle_tool_result` | 234–266 | `fn(&mut self, obj: &Value)` |

**`.get()` extraction sites**: ~35

**Special handling**:
- Content as string OR array: `content_val.as_str()` then fallback to `content_val.as_array()` (lines 90–102)
- Error severity-based branching: `severity == "warning"` → `on_warning` only, not `is_error` (lines 183–188)
- Result can carry errors: `status == "error"` → extract `error.type` + `error.message` (lines 116–125)
- Stats override: `stats.tool_calls` overrides event-level count (lines 135–137)
- `cached` field maps to `cache_read` (line 142)

**Test count**: 8 (design doc said 7)

| # | Test name | Key scenario |
|---|-----------|--------------|
| 1 | `happy_path_real_gemini_format` | Real NDJSON with timestamps, stats |
| 2 | `user_messages_ignored` | `role != "assistant"` filtered |
| 3 | `error_event` | Non-fatal warning severity |
| 4 | `result_error_status` | Result with status=error |
| 5 | `tool_counting_from_events` | tool_use/tool_result pair |
| 6 | `tool_result_correlates_back_to_tool_use` | Full tool contract |
| 7 | `tool_count_from_stats_overrides_events` | stats.tool_calls authoritative |
| 8 | `content_as_array_fallback` | Array format content |

### B.4 OpenCode Parser (`stream/opencode.rs`)

**File size**: 643 lines | **Parser struct**: `OpenCodeStreamParser<S>` (16 fields)

**`feed_line()` structure** (lines 271–369):
- Standard pattern. Has free helper functions for tool field extraction.

**Handler methods and free functions**:

| Method | Lines | Signature |
|--------|-------|-----------|
| `handle_init` | 58–83 | `fn(&mut self, obj: &Value)` |
| `handle_text` | 85–99 | `fn(&mut self, obj: &Value) -> Option<StreamChunk>` |
| `handle_step_start` | 101–127 | `fn(&mut self, obj: &Value)` |
| `handle_step_finish` | 129–165 | `fn(&mut self, obj: &Value)` |
| `handle_step_complete` | 167–198 | `fn(&mut self, obj: &Value)` |
| `handle_error` | 200–219 | `fn(&mut self, obj: &Value)` |
| `opencode_tool_name` | 222–237 | `fn(obj: &Value) -> Option<&str>` |
| `opencode_value` | 239–244 | `fn<'a>(obj: &'a Value, keys: &[&str]) -> Option<&'a Value>` |
| `opencode_tool_id` | 246–250 | `fn(obj: &Value) -> Option<String>` |
| `opencode_tool_input` | 252–254 | `fn(obj: &Value) -> Option<Value>` |
| `opencode_tool_output` | 256–258 | `fn(obj: &Value) -> Option<Value>` |
| `opencode_tool_status` | 260–264 | `fn(obj: &Value) -> Option<String>` |
| `opencode_tool_error` | 266–268 | `fn(obj: &Value) -> Option<Value>` |

**`.get()` extraction sites**: ~50 (most complex parser)

**Special handling**:
- Nested `part` object: `opencode_value` checks both top-level and `obj.get("part")` for every field (lines 239–244)
- Tool name has 8 source paths: `name`/`tool_name`/`toolName`/`tool` at top-level and inside `part` (lines 222–237)
- Step start captures session ID from `sessionID` (camelCase) on first step (lines 107–110) — acts as init event
- Usage accumulation (not merge): `NormalizedTokenUsage::accumulate` for per-step totals (lines 144, 178)
- Cache read from nested `cache.read` (lines 139–143)
- Cost accumulation across steps (line 149)

**Test count**: 9 (design doc said 8)

| # | Test name | Key scenario |
|---|-----------|--------------|
| 1 | `accumulates_usage_across_steps` | Two step_complete events, cumulative |
| 2 | `model_from_constructor` | External model parameter |
| 3 | `model_overridden_by_stream` | Init overrides constructor model |
| 4 | `no_usage_when_no_steps` | No token_usage when zero steps |
| 5 | `real_opencode_ndjson_format` | Real NDJSON with nested part, cache.read |
| 6 | `step_failure_warning` | Error with error_message |
| 7 | `tool_name_extraction_supports_nested_part_fields` | Part-nested tool names |
| 8 | `tool_events_preserve_opencode_parameters_and_results` | Full tool contract |
| 9 | `step_boundaries_do_not_emit_high_level_turn_lifecycle_events` | Sink dispatch counts |

### B.5 Qwen Parser (`stream/qwen.rs`)

**File size**: 489 lines | **Parser struct**: `QwenStreamParser<S>` (16 fields)

**`feed_line()` structure** (lines 206–304):
- Standard pattern. Also extracts `subtype` (line 224).

**Handler methods**:

| Method | Lines | Signature |
|--------|-------|-----------|
| `handle_init` | 55–77 | `fn(&mut self, obj: &Value)` |
| `handle_message` | 79–111 | `fn(&mut self, obj: &Value) -> Option<StreamChunk>` |
| `handle_result` | 113–154 | `fn(&mut self, obj: &Value)` |
| `handle_error` | 156–178 | `fn(&mut self, obj: &Value)` |
| `tool_id` | 180–186 | `fn(obj: &Value) -> Option<String>` |
| `tool_input` | 188–195 | `fn(obj: &Value) -> Option<Value>` |
| `tool_output` | 197–202 | `fn(obj: &Value) -> Option<Value>` |

**`.get()` extraction sites**: ~30

**Special handling**:
- `subtype == "session_start"` check for system events (line 232)
- Message accepts `role == "assistant"` OR `event_type == "assistant"` (line 82)
- Content as array OR string (lines 87–108)
- Result usage from three fields: `stats` / `usage` / `token_usage` (lines 125–128)
- Tool input from 5 aliases: `input`/`parameters`/`arguments`/`args`/`params` (lines 188–195) — **design doc missed `args` and `params`**

**Test count**: 5 (design doc said 6)

| # | Test name | Key scenario |
|---|-----------|--------------|
| 1 | `happy_path` | Init → message → result |
| 2 | `qwen_specific_event_names` | `assistant_message`, `tool_call`, `tool_response`, `summary` |
| 3 | `qwen_hook_design_session_and_assistant_events_are_supported` | `system` with `subtype=session_start`, `type=assistant` |
| 4 | `content_as_string` | String content |
| 5 | `tool_events_preserve_parameters_and_results` | Tool contract |

### B.6 Kimi Parser (`stream/kimi.rs`)

**File size**: 520 lines | **Parser struct**: `KimiStreamParser<S>` (16 fields)

**`feed_line()` structure** (lines 237–330):
- Standard pattern.

**Handler methods**:

| Method | Lines | Signature |
|--------|-------|-----------|
| `handle_init` | 59–81 | `fn(&mut self, obj: &Value)` |
| `handle_content` | 83–119 | `fn(&mut self, obj: &Value) -> Option<StreamChunk>` |
| `handle_status_update` | 121–191 | `fn(&mut self, obj: &Value)` |
| `handle_error` | 193–209 | `fn(&mut self, obj: &Value)` |
| `tool_id` | 211–217 | `fn(obj: &Value) -> Option<String>` |
| `tool_input` | 219–226 | `fn(obj: &Value) -> Option<Value>` |
| `tool_output` | 228–233 | `fn(obj: &Value) -> Option<Value>` |

**`.get()` extraction sites**: ~35

**Special handling**:
- Three-way content fallback: array → `text` field → `content` as string (lines 85–116)
- Context pressure calculation with computed percent fallback (lines 159–165)
- Context pressure warning at >= 80% (line 173, const at line 11)
- Status update is "last snapshot wins" not accumulation (lines 123–139)
- Usage from `usage` or `token_usage` (line 123)
- Context from `context_usage` or `context` (line 156)
- Tool input from 5 aliases including `args`/`params` (lines 219–226) — **design doc missed these**

**Test count**: 5 (design doc said 6)

| # | Test name | Key scenario |
|---|-----------|--------------|
| 1 | `summary_from_last_status_update` | Two StatusUpdate events, second overwrites |
| 2 | `context_pressure_warning` | 110000/128000 = 85.9% triggers warning |
| 3 | `no_warning_below_threshold` | 50000/128000 = 39% no warning |
| 4 | `missing_model_and_cost_tolerated` | Graceful handling of missing fields |
| 5 | `tool_events_preserve_parameters_and_results` | Tool contract |

## C. File-by-File Implementation Steps

### C.1 `stream/protocol/mod.rs` — CREATE

**Purpose**: Module root with `ProtocolError` enum and re-exports.

**Contents**:
```rust
pub mod claude;
pub mod codex;
pub mod gemini;
pub mod opencode;
pub mod qwen;
pub mod kimi;

#[derive(Debug, thiserror::Error)]
pub enum ProtocolError {
    #[error("Unknown event type: {0}")]
    UnknownEventType(String),
    #[error("Deserialization failed for {event_type}: {reason}")]
    DeserializationFailed { event_type: String, reason: String },
}
```

**Note**: `ProtocolError` is for documentation/future use. During the initial migration, deserialization errors are handled inline in the `Err(_)` fallback arm of `feed_line()` without constructing `ProtocolError`.

### C.2 `stream/protocol/claude.rs` — CREATE

**Purpose**: Typed event enum and structs for Claude Code `stream-json` events.

**Key structs** (adapted from design, with corrections):
- `ClaudeEvent` — tagged enum with `#[serde(tag = "type")]`
- `ClaudeInit` — session_id, model, tools
- `ClaudeAssistant` — message (optional), content (optional)
- `ClaudeMessage` — content array, role
- `ClaudeContentBlock` — tagged: Text, ToolUse, Unknown (with `#[serde(other)]`)
- `ClaudeContentBlockStart` — content_block, index
- `ClaudeContentBlockDelta` — delta
- `ClaudeDelta` — tagged: TextDelta, ThinkingDelta, InputJsonDelta
- `ClaudeErrorEvent` — error struct
- `ClaudeError` — kind (renamed from "type"), message
- `ClaudeResult` — all result fields
- `ClaudeUsage` — input_tokens, output_tokens, cache_read_input_tokens
- `ClaudeRateLimit` — is_throttled, retry_after_ms, message
- `ClaudeToolUse` — id, name, tool_name, input, content_block
- `ClaudeToolResult` — tool_use_id, id, content, output, result

**Discrepancies from design doc that require adjustments**:

1. The design's `ClaudeContentBlockStart` needs `content_block: Option<ClaudeToolUseContent>` but the actual code at line 352–357 also checks `content_block.type == "tool_use"` to decide whether to dispatch as tool_use. The `feed_line()` match arm must retain this conditional check after deserialization.

2. The design names the tool_use content struct `ClaudeToolUseContent` but the parser's `tool_use_payload` helper (line 70–72) unwraps `content_block` before passing to the handler. The typed version must handle this in the match arm, not in the handler.

3. The `ClaudeResult` struct needs `#[serde(default)]` on ALL fields since Claude omits many fields in practice.

### C.3 `stream/protocol/codex.rs` — CREATE

**Purpose**: Typed events for Codex CLI JSONL events.

**Key structs**:
- `CodexEvent` — tagged enum with dotted event names (`thread.created`, etc.)
- `CodexThreadMeta` — thread_id, id
- `CodexEmpty` — empty struct for turn.started
- `CodexTurnCompleted` — usage, duration_ms, cost_usd, status, stop_reason
- `CodexUsage` — input_tokens, output_tokens, cached_input_tokens, cache_read_input_tokens, total_tokens
- `CodexErrorEnvelope` — error_type, error_message, message, error (nested)
- `CodexErrorDetail` — kind (renamed from "type"), message
- `CodexItemEnvelope` — item
- `CodexItem` — tagged enum with `#[serde(other)]` for Unknown
- `CodexToolItemFields` — id, name, tool_name, input, arguments, parameters, output, result, content
- `CodexPermissionItem` — id, name
- `CodexContentPart` — text
- `CodexToolItem` — wraps fields with `#[serde(flatten)]`
- `CodexAgentMessage` — text, content
- `CodexReasoning` — text, summary

**Discrepancies from design doc**:

1. The design's `CodexUsage` has both `cached_input_tokens` and `cache_read_input_tokens` — correct, the actual code falls back between them (codex.rs line 100–101).

2. The design's `CodexEvent` doesn't include `Reasoning` variant — the actual code at line 361 falls to `_` wildcard. Adding it to the enum is optional (will be caught by fallback).

3. The `item.completed` handler merges started and completed item data (lines 280–294). This merge logic must stay in the parser; the typed structs only replace field extraction, not the merge pattern.

### C.4 `stream/protocol/gemini.rs` — CREATE

**Key structs**: `GeminiEvent`, `GeminiInit`, `GeminiMessage`, `GeminiErrorEvent`, `GeminiResult`, `GeminiResultError`, `GeminiStats`, `GeminiToolUse`, `GeminiToolResult`

**Discrepancies**:

1. `GeminiMessage.content` is `Option<serde_json::Value>` in the design — this is correct because Gemini can emit content as either a string or an array. The parser handler must branch on `content_val.as_str()` vs `content_val.as_array()` after typed deserialization.

2. `GeminiStats` has both `cached` and `input` fields. The parser maps `cached` → `cache_read` and `input` is not currently used. Both should be in the typed struct for completeness.

### C.5 `stream/protocol/opencode.rs` — CREATE

**Key structs**: `OpenCodeEvent`, `OpenCodeInit`, `OpenCodeStepStart`, `OpenCodeText`, `OpenCodeTextPart`, `OpenCodeStepFinish`, `OpenCodeStepFinishPart`, `OpenCodeTokens`, `OpenCodeCache`, `OpenCodeStepComplete`, `OpenCodeUsage`, `OpenCodeError`, `OpenCodeErrorDetail`, `OpenCodeToolEvent`, `OpenCodeToolPart`, `OpenCodeToolResult`

**Discrepancies**:

1. The design's `OpenCodeStepStart` has `session_id_alt` with `#[serde(rename = "sessionID")]`. The actual code uses `sessionID` (camelCase) as the primary source on first step_start (line 109). The `session_id` snake_case field is also accepted. This is correct in the design.

2. The design's tool name aliases (`tool_name_camel` with `#[serde(rename = "toolName")]`) match the actual code's `opencode_tool_name` function.

3. The actual code's `opencode_value` helper (lines 239–244) checks both top-level and `part`-nested keys. The typed struct approach can capture the `part` nesting via the `OpenCodeToolEvent.part` field, but the handler must still check both `event.name`/`event.tool_name` and `event.part.name`/`event.part.tool_name`. This logic stays in the parser handler.

### C.6 `stream/protocol/qwen.rs` — CREATE

**Key structs**: `QwenEvent`, `QwenInit`, `QwenSystem`, `QwenMessage`, `QwenErrorEvent`, `QwenErrorDetail`, `QwenResult`, `QwenUsage`, `QwenToolUse`, `QwenToolResult`

**Discrepancies**:

1. **Missing `args` and `params` aliases** in design's `QwenToolUse` and `QwenToolResult`. The actual code has `tool_input` checking 5 keys: `input`/`parameters`/`arguments`/`args`/`params` (qwen.rs line 188–195). The typed struct needs additional fields:
   ```rust
   #[serde(default)]
   pub args: Option<serde_json::Value>,
   #[serde(default)]
   pub params: Option<serde_json::Value>,
   ```
   And the handler must check all five when extracting tool input.

2. **Missing `subtype` handling** in design. The actual code at line 232 checks `subtype == "session_start"` for `system` events. The `QwenEvent` enum already maps `System` to `QwenSystem`, and `QwenSystem` has `subtype: Option<String>`. The parser's match arm must check `subtype` after deserialization to decide whether to call `handle_init`.

3. **Message `content` as string**: The design uses `Option<serde_json::Value>` which correctly handles both string and array forms.

### C.7 `stream/protocol/kimi.rs` — CREATE

**Key structs**: `KimiEvent`, `KimiInit`, `KimiContent`, `KimiTextPart`, `KimiStatusUpdate`, `KimiUsage`, `KimiContextUsage`, `KimiErrorEvent`, `KimiErrorDetail`, `KimiToolUse`, `KimiToolResult`

**Discrepancies**:

1. **Missing `args` and `params` aliases** in design's `KimiToolUse` and `KimiToolResult` — same issue as Qwen. Actual code checks `input`/`parameters`/`arguments`/`args`/`params` (kimi.rs lines 219–226).

2. **Missing `content` as string in `KimiContent`** — the actual code has a three-way fallback: array → `text` field → `content` as string (lines 85–116). The typed struct has `content: Option<Vec<KimiTextPart>>` and `text: Option<String>`. The handler must also try `content` as a string after the array check.

3. **Context percent computation**: The actual code computes percent from `used/total` if not provided (lines 159–165). This computation stays in the parser handler, not in the typed struct.

### C.8 `stream/mod.rs` — EDIT

**Change**: Add `pub mod protocol;` after the existing module declarations (after line 7).

### C.9 `stream/claude.rs` — EDIT

**Changes**:
1. Add `use super::protocol::claude::ClaudeEvent;` to imports
2. Rewrite `feed_line()` match block (lines 326–374) to:
   - First try `serde_json::from_str::<ClaudeEvent>(line)`
   - On success, match on typed variants
   - On failure, fall back to `Value` parsing for unknown events
3. Change handler signatures from `fn handle_xxx(&mut self, obj: &Value)` to accept typed structs
4. Replace `.get()` chains with direct field access
5. Retain `serde_json::Value` import for: raw_summary construction, EventMeta extra values, tool_input/tool_response as Value

**Specific handler changes**:

| Handler | Before | After |
|---------|--------|-------|
| `handle_init` | Extract 2 fields via `.get()` | Direct `init.session_id`, `init.model` |
| `handle_assistant_message` | 5 `.get()` chains | Direct field access on `ClaudeAssistant` |
| `handle_content_block_delta` | 4 `.get()` chains | Match on `ClaudeDelta` variants |
| `handle_error` | 4 `.get()` chains | Direct `err.error.kind`, `err.error.message` |
| `handle_result` | 12 `.get()` chains | Direct field access on `ClaudeResult` |
| `handle_rate_limit` | 3 `.get()` chains | Direct field access on `ClaudeRateLimit` |
| `handle_tool_use` | 8 `.get()` chains | Direct field access on `ClaudeToolUse` |
| `handle_tool_result` | 5 `.get()` chains | Direct field access on `ClaudeToolResult` |

**Key: content_block_start conditional tool_use dispatch** — Lines 352–357 match `"tool_use" | "content_block_start"` with a condition on `content_block.type == "tool_use"`. In the typed version:
```rust
Ok(ClaudeEvent::ContentBlockStart(cbs)) => {
    if cbs.content_block.as_ref().map(|cb| cb.id.is_some()).unwrap_or(false) {
        self.handle_tool_use_from_content_block(cbs);
    }
    Ok(None)
}
Ok(ClaudeEvent::ToolUse(tu)) => {
    self.handle_tool_use(tu);
    Ok(None)
}
```

### C.10 `stream/codex.rs` — EDIT

**Changes**: Same pattern as Claude. Add `use super::protocol::codex::CodexEvent;`, rewrite `feed_line()`, update handlers.

**Special: Fatal error type** — Codex uses `StreamParseError::Fatal` for malformed JSON (line 315). This must be preserved; the typed deserialization error path should also use `Fatal`.

**Special: item merging** — The `handle_item_completed` merge logic (lines 280–294) works with `Value` clones. After typed deserialization, this can remain as-is since the `item.completed` event carries its own `CodexItem` and the `tool_items` HashMap stores `CodexToolItemFields` (or we keep `Value` in the HashMap for simplicity during Phase 1).

### C.11 `stream/gemini.rs` — EDIT

**Changes**: Same pattern. Add protocol import, rewrite `feed_line()`, update handlers.

**Special: severity-based error handling** — `handle_error` branches on `severity == "warning"` (lines 183–188). After typed deserialization:
```rust
fn handle_error(&mut self, event: GeminiErrorEvent) {
    self.error_kind = event.severity;
    self.error_message = event.message;
    if self.error_kind.as_deref() == Some("warning") {
        if let Some(message) = &self.error_message {
            self.sink.on_warning(message);
        }
        return;
    }
    self.is_error = true;
    self.sink.on_turn_error(&meta);
}
```

**Special: tool_calls override from stats** — The `handle_result` code at lines 135–137 overrides `tool_calls` from stats. This logic stays in the handler.

### C.12 `stream/opencode.rs` — EDIT

**Changes**: Same pattern. The free functions (`opencode_tool_name`, etc.) can be removed or kept as helpers that work with typed structs.

**Highest risk**: The nested `part` object pattern is unique to OpenCode. The typed `OpenCodeToolEvent` struct has both top-level fields and `part: Option<OpenCodeToolPart>`. The handler must check both:
```rust
let name = event.name
    .or_else(|| event.tool_name)
    .or_else(|| event.tool_name_camel)
    .or_else(|| event.tool)
    .or_else(|| event.part.as_ref().and_then(|p| p.name.clone()))
    .or_else(|| event.part.as_ref().and_then(|p| p.tool_name.clone()))
    // ... etc
```

This is still cleaner than the current 14+ `.get()` calls in `opencode_tool_name`, but the handler retains multi-source field resolution.

### C.13 `stream/qwen.rs` — EDIT

**Changes**: Same pattern. Key addition: handle `subtype == "session_start"` after deserialization:
```rust
Ok(QwenEvent::System(sys)) => {
    if sys.subtype.as_deref() == Some("session_start") {
        self.handle_init(QwenInit { session_id: sys.session_id, model: sys.model });
    }
    Ok(None)
}
```

### C.14 `stream/kimi.rs` — EDIT

**Changes**: Same pattern. Key: `handle_content` three-way fallback stays in handler, `handle_status_update` context pressure computation stays in handler.

## D. Per-Phase Execution Checklist

### Phase 1: Claude + Codex

#### Step 1.1: Create `stream/protocol/mod.rs`

- **Action**: CREATE
- **Contents**: Module declarations + `ProtocolError` enum
- **Verify**: `cargo check -p claudine` compiles (protocol submodules are empty)

#### Step 1.2: Edit `stream/mod.rs`

- **Action**: Add `pub mod protocol;` after line 7
- **Verify**: `cargo check -p claudine`

#### Step 1.3: Create `stream/protocol/claude.rs`

- **Action**: CREATE with all Claude typed structs
- **Verify**: `cargo check -p claudine`
- **Test command**: `cargo test -p claudine --lib stream::protocol::claude` (no tests yet, should pass with no output)

#### Step 1.4: Add protocol deserialization tests for Claude

- **Action**: Add `#[cfg(test)] mod tests` block to `stream/protocol/claude.rs`
- **Tests to add**:
  1. `claude_init_deserializes` — basic init event
  2. `claude_assistant_deserializes_with_message_key` — nested `message.content` format
  3. `claude_assistant_deserializes_flat_content` — top-level `content` array
  4. `claude_content_block_delta_text` — text_delta variant
  5. `claude_content_block_delta_thinking` — thinking_delta variant
  6. `claude_error_event_deserializes` — error with type + message
  7. `claude_result_deserializes` — full result with usage
  8. `claude_result_total_cost_usd` — both cost field names
  9. `claude_rate_limit_deserializes` — rate limit event
  10. `claude_tool_use_deserializes` — tool use with id/name/input
  11. `claude_tool_result_deserializes` — tool result with content
  12. `claude_unknown_event_type_falls_through` — unknown type returns Err
  13. `claude_init_tolerates_unknown_fields` — extra fields don't break
  14. `claude_content_block_start_tool_use` — content_block_start with tool_use type
- **Test command**: `cargo test -p claudine --lib stream::protocol::claude`
- **Expected**: All 14 new tests pass

#### Step 1.5: Migrate `stream/claude.rs` feed_line()

- **Action**: EDIT `stream/claude.rs`
- **Specific changes**:
  1. Add `use super::protocol::claude::{ClaudeEvent, ...};` import
  2. Rewrite `feed_line()` (lines 313–374): try typed deserialization first, fall back to `Value`
  3. Change `handle_init(&mut self, obj: &Value)` → `handle_init(&mut self, init: ClaudeInit)`
  4. Change `handle_assistant_message(&mut self, obj: &Value)` → `handle_assistant_message(&mut self, assistant: ClaudeAssistant)`
  5. Change `handle_content_block_delta(&mut self, obj: &Value)` → `handle_content_block_delta(&mut self, delta: ClaudeContentBlockDelta)`
  6. Change `handle_error(&mut self, obj: &Value)` → `handle_error(&mut self, err: ClaudeErrorEvent)`
  7. Change `handle_result(&mut self, obj: &Value)` → `handle_result(&mut self, result: ClaudeResult)`
  8. Change `handle_rate_limit(&mut self, obj: &Value)` → `handle_rate_limit(&mut self, rl: ClaudeRateLimit)`
  9. Change `handle_tool_use(&mut self, obj: &Value)` → `handle_tool_use(&mut self, tu: ClaudeToolUse)`
  10. Change `handle_tool_result(&mut self, obj: &Value)` → `handle_tool_result(&mut self, tr: ClaudeToolResult)`
- **Test command**: `cargo test -p claudine --lib stream::claude`
- **Expected**: All 18 existing tests pass unmodified
- **Rollback**: Revert `stream/claude.rs` to git HEAD

#### Step 1.6: Create `stream/protocol/codex.rs`

- **Action**: CREATE with all Codex typed structs
- **Test command**: `cargo check -p claudine`

#### Step 1.7: Add protocol deserialization tests for Codex

- **Action**: Add `#[cfg(test)] mod tests` to `stream/protocol/codex.rs`
- **Tests to add**:
  1. `codex_thread_started_deserializes`
  2. `codex_turn_completed_with_usage`
  3. `codex_error_deserializes`
  4. `codex_item_started_agent_message`
  5. `codex_item_started_tool_use`
  6. `codex_item_completed_agent_message`
  7. `codex_unknown_event_type_falls_through`
  8. `codex_tool_use_deserializes`
  9. `codex_tool_result_deserializes`
- **Test command**: `cargo test -p claudine --lib stream::protocol::codex`

#### Step 1.8: Migrate `stream/codex.rs` feed_line()

- **Action**: EDIT `stream/codex.rs`
- **Specific changes**:
  1. Add protocol import
  2. Rewrite `feed_line()` (lines 318–366): typed deserialization + Value fallback
  3. Update handler signatures to accept typed structs
  4. Keep `StreamParseError::Fatal` for malformed JSON (not MalformedLine)
  5. Keep `item.completed` merge logic (may need to temporarily store `Value` for merging)
- **Test command**: `cargo test -p claudine --lib stream::codex`
- **Expected**: All 4 existing tests pass unmodified
- **Rollback**: Revert `stream/codex.rs` to git HEAD

#### Phase 1 Verification

```bash
cargo test -p claudine --lib stream::claude
cargo test -p claudine --lib stream::codex
cargo test -p claudine --lib stream::protocol
cargo clippy -p claudine -- -D warnings
cargo fmt -p claudine --check
```

### Phase 2: Gemini + OpenCode

#### Step 2.1: Create `stream/protocol/gemini.rs`

- **Action**: CREATE with all Gemini typed structs
- **Verify**: `cargo check -p claudine`

#### Step 2.2: Add protocol deserialization tests for Gemini

- **Tests to add**: 10–12 tests covering all variants and edge cases (string content, array content, error severity, result errors, stats override)
- **Test command**: `cargo test -p claudine --lib stream::protocol::gemini`

#### Step 2.3: Migrate `stream/gemini.rs`

- **Action**: EDIT — rewrite `feed_line()`, update 6 handler signatures
- **Test command**: `cargo test -p claudine --lib stream::gemini`
- **Expected**: All 8 existing tests pass
- **Rollback**: Revert to git HEAD

#### Step 2.4: Create `stream/protocol/opencode.rs`

- **Action**: CREATE with all OpenCode typed structs
- **Verify**: `cargo check -p claudine`

#### Step 2.5: Add protocol deserialization tests for OpenCode

- **Tests to add**: 12–15 tests covering nested `part`, camelCase aliases, cache nesting, step lifecycle
- **Test command**: `cargo test -p claudine --lib stream::protocol::opencode`

#### Step 2.6: Migrate `stream/opencode.rs`

- **Action**: EDIT — rewrite `feed_line()`, update 6 handler signatures, update/remove 7 free functions
- **Risk**: Highest complexity due to nested `part` pattern
- **Test command**: `cargo test -p claudine --lib stream::opencode`
- **Expected**: All 9 existing tests pass
- **Rollback**: Revert to git HEAD

#### Phase 2 Verification

```bash
cargo test -p claudine --lib stream
cargo clippy -p claudine -- -D warnings
cargo fmt -p claudine --check
```

### Phase 3: Qwen + Kimi

#### Step 3.1: Create `stream/protocol/qwen.rs`

- **Action**: CREATE with corrected structs (include `args`/`params` aliases)
- **Verify**: `cargo check -p claudine`

#### Step 3.2: Add protocol deserialization tests for Qwen

- **Tests to add**: 8–10 tests
- **Test command**: `cargo test -p claudine --lib stream::protocol::qwen`

#### Step 3.3: Migrate `stream/qwen.rs`

- **Action**: EDIT — rewrite `feed_line()`, update handlers. Handle `subtype == "session_start"`.
- **Test command**: `cargo test -p claudine --lib stream::qwen`
- **Expected**: All 5 existing tests pass
- **Rollback**: Revert to git HEAD

#### Step 3.4: Create `stream/protocol/kimi.rs`

- **Action**: CREATE with corrected structs (include `args`/`params` aliases)
- **Verify**: `cargo check -p claudine`

#### Step 3.5: Add protocol deserialization tests for Kimi

- **Tests to add**: 8–10 tests
- **Test command**: `cargo test -p claudine --lib stream::protocol::kimi`

#### Step 3.6: Migrate `stream/kimi.rs`

- **Action**: EDIT — rewrite `feed_line()`, update handlers. Keep context pressure computation in handler.
- **Test command**: `cargo test -p claudine --lib stream::kimi`
- **Expected**: All 5 existing tests pass
- **Rollback**: Revert to git HEAD

#### Phase 3 Verification

```bash
cargo test -p claudine --lib stream
cargo clippy -p claudine -- -D warnings
cargo fmt -p claudine --check
```

### Phase 4: Cleanup

#### Step 4.1: Remove unnecessary `use serde_json::Value` imports

- **Action**: EDIT all 6 parser files — remove `Value` import if no longer needed in handlers
- **Note**: Most parsers still need `Value` for tool_input/tool_response (stored as `Value`) and raw_summary construction. Only remove where truly unused.

#### Step 4.2: Audit remaining Value extraction sites

- **Action**: Grep for `.get(` across all 6 parser files — should find zero in handler methods (only in feed_line fallback arm and raw_summary construction)
- **Command**: `rg '\.get\(' claudine/lib/src/stream/claude.rs claudine/lib/src/stream/codex.rs claudine/lib/src/stream/gemini.rs claudine/lib/src/stream/opencode.rs claudine/lib/src/stream/qwen.rs claudine/lib/src/stream/kimi.rs`

#### Step 4.3: Update `stream/mod.rs` re-exports if needed

- **Action**: Evaluate whether `protocol` types need to be re-exported for downstream consumers
- **Likely answer**: No — protocol types are internal to the stream module

#### Step 4.4: Final verification

```bash
cargo test -p claudine
cargo clippy -p claudine -- -D warnings
cargo fmt -p claudine --check
```

## E. Risk Matrix

### Risk 1: Serde tag conflicts with dotted event names (Codex)
- **Likelihood**: Medium
- **Impact**: High (blocks Codex migration)
- **Details**: Codex uses `thread.created`, `turn.started`, etc. as type tags. `#[serde(tag = "type")]` with `#[serde(rename = "thread.created")]` should work but needs verification.
- **Mitigation**: Test serde tagged enum with dotted names in isolation before migration. If it fails, use `#[serde(untagged)]` with manual dispatch or keep the string-based match for Codex.
- **Parser risk**: Codex (Medium)

### Risk 2: Claude `content_block_start` conditional dispatch
- **Likelihood**: Low
- **Impact**: Medium
- **Details**: The `content_block_start` event is dispatched as tool_use only when `content_block.type == "tool_use"` (claude.rs:352–357). With typed deserialization, this event maps to `ClaudeEvent::ContentBlockStart` and the handler must check the content_block type.
- **Mitigation**: The `ClaudeContentBlockStart` struct already has `content_block: Option<ClaudeToolUseContent>`. Check `content_block` presence in the match arm.
- **Parser risk**: Claude (Low)

### Risk 3: OpenCode nested `part` field resolution
- **Likelihood**: Medium
- **Impact**: High (most complex parser)
- **Details**: OpenCode tools can have fields at top-level OR nested under `part`. The typed structs capture both, but the handler still needs multi-source resolution logic.
- **Mitigation**: The typed structs flatten this: `OpenCodeToolEvent` has both `name: Option<String>` and `part: Option<OpenCodeToolPart>`. The handler resolves priority. This is still cleaner than the current 14-call `opencode_tool_name` function.
- **Parser risk**: OpenCode (High)

### Risk 4: Provider format drift during migration
- **Likelihood**: Low
- **Impact**: Low
- **Details**: If a provider changes their JSON format while we're mid-migration, the typed structs would fail to deserialize but the fallback arm handles this gracefully.
- **Mitigation**: The `Err(_)` fallback arm re-parses as `Value` and traces the event type. Zero behavior change for unknown events.
- **Parser risk**: All (Low)

### Risk 5: Missing field aliases causing silent data loss
- **Likelihood**: Medium
- **Impact**: Medium
- **Details**: If a parser uses a field alias (e.g., `args`/`params` for tool input) that isn't in the typed struct, that data is silently lost.
- **Mitigation**: The audit in Section B identified all aliases. Qwen and Kimi tool helpers check 5 keys each. The typed structs must include all aliases with `#[serde(default)]`.
- **Parser risk**: Qwen, Kimi (Medium)

### Riskiest Parsers (Ranked)

1. **OpenCode** — Nested `part` objects, camelCase aliases, accumulation semantics, 50+ extraction sites
2. **Codex** — Dotted event names, item merge logic, Fatal vs MalformedLine error type
3. **Kimi** — Three-way content fallback, context pressure computation, missing aliases in design
4. **Qwen** — `subtype` dispatch, dual message format, missing aliases in design
5. **Gemini** — Severity-based error branching, stats tool_calls override, dual content format
6. **Claude** — Most tests, but cleanest event structure. `content_block_start` conditional is the only complexity.

## F. Acceptance Criteria

### F.1 All existing tests pass unmodified

```bash
cargo test -p claudine --lib stream::claude    # 18 tests
cargo test -p claudine --lib stream::codex     # 4 tests
cargo test -p claudine --lib stream::gemini    # 8 tests
cargo test -p claudine --lib stream::opencode  # 9 tests
cargo test -p claudine --lib stream::qwen      # 5 tests
cargo test -p claudine --lib stream::kimi      # 5 tests
```

Total: 49 existing tests, all must pass without any test file modifications.

### F.2 New typed deserialization tests pass

- Each protocol module has 8–15 tests covering all event variants
- Unknown event type tests confirm graceful degradation
- Field evolution tests confirm tolerance for extra fields

### F.3 No `serde_json::Value` in handler methods

After Phase 4, running:
```bash
rg 'fn handle_.*&Value' claudine/lib/src/stream/*.rs
```
Should return zero results. All handlers accept typed structs.

Exception: `tool_input`/`tool_response` fields are legitimately `serde_json::Value` in the typed structs (they carry arbitrary JSON payloads). Handler code may reference `Value` for constructing `EventMeta.extra` entries, which is acceptable.

### F.4 `cargo clippy` and `cargo fmt` clean

```bash
cargo clippy -p claudine -- -D warnings
cargo fmt -p claudine --check
```

### F.5 No `#[serde(deny_unknown_fields)]` anywhere in protocol/

All typed structs use `#[serde(default)]` on every field to tolerate provider format evolution.

## G. Estimated Effort

| Phase | Scope | New LOC (protocol) | Parser edits | Tests | Effort | Confidence |
|-------|-------|-------------------:|-------------:|------:|--------|------------|
| 1 | Claude + Codex | ~350 | ~200 | ~25 new | 3–4h | 85% |
| 2 | Gemini + OpenCode | ~350 | ~250 | ~25 new | 4–5h | 75% |
| 3 | Qwen + Kimi | ~250 | ~180 | ~18 new | 2–3h | 80% |
| 4 | Cleanup | 0 | ~50 | 0 | 1h | 95% |
| **Total** | | **~950** | **~680** | **~68 new** | **10–13h** | **80%** |

**Confidence caveats**:
- Phase 2 (OpenCode) has the widest uncertainty due to nested `part` complexity
- Phase 1 (Claude) is highest confidence — simplest event structure, most tests
- Each phase includes a built-in rollback point before proceeding to the next
