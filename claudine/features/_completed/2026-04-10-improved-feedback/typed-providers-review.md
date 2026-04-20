# Review: Strongly Typed Provider Protocol Models

Implementation review of the technical design for strongly typed provider protocol models.

## Overview

The implementation of strongly typed provider protocol models is **substantially complete** and follows the proposed architecture. All six target parsers (`claude`, `codex`, `gemini`, `opencode`, `qwen`, `kimi`) have been migrated to use typed protocol models defined in `stream/protocol/`.

## Functionality Gaps

- **Missing `delta` field in `GeminiMessage`**: The `GeminiMessage` struct in `protocol/gemini.rs` is missing the `delta: Option<bool>` field specified in the technical design. While not strictly used by the parser today (streaming is handled by the arrival of the event itself), it should be included for completeness.
- **`CodexItem` Tagged Enum Deviation**: The technical design proposed a tagged enum for `CodexItem` variants (`AgentMessage`, `ToolUse`, `PermissionRequest`, etc.). The implementation instead uses a single flattened struct `CodexItem` with an optional `kind` field.
    - **Impact**: This forces the parser to continue using manual `as_array()` and `.get("text")` calls on the `content` field for agent messages, rather than having them deserialized into a `Vec<CodexContentPart>` as designed.
- **Incomplete `CodexItem` Variants**: The `CodexItem` implementation is missing specific struct representations for `PermissionRequest`, `ApprovalRequest`, and `UserInputRequest` variants mentioned in the design. They are currently handled via string matching on the `kind` field in the parser.

## Implementation Quality

- **Ergonomics**: The addition of `resolve()` and `take_input()`/`take_output()` helpers in the protocol modules (especially for `OpenCode` and `Kimi`) is excellent. These helpers consolidate the messiness of multiple field aliases (`input`/`parameters`/`args`) and nested `part` objects, making the parser code significantly cleaner.
- **Tolerance**: Every struct correctly employs `#[serde(default)]` and `Option<T>` for all fields, ensuring that the parsers remain tolerant of provider format evolution.
- **Backward Compatibility**: The use of a `Err(_)` fallback in the match arms (which re-parses as `Value` for tracing) successfully preserves the "silent skip" behavior for unknown event types, ensuring no regressions for future provider events.

## Test Coverage

- **Strong Unit Testing**: Every protocol module (`stream/protocol/*.rs`) includes a thorough suite of unit tests verifying deserialization of all key event variants from real-world JSON samples.
- **Regression Guard**: Existing parser tests (e.g., `claude.rs`, `opencode.rs`) were successfully migrated to the new handlers and continue to pass, confirming that state accumulation logic remains correct.
- **Tool Contract Verification**: All providers use the shared `assert_tool_event_contract` helper, ensuring that tool ID/name/input/output extraction is verified consistently across the entire suite.

## Performance & Ergonomics Suggestions

### 1. Optimize `feed_line` Deserialization
Current implementation:
```rust
let raw: Value = serde_json::from_str(line)?;
match serde_json::from_value::<ProviderEvent>(raw.clone()) { ... }
```
This involves an extra clone and a second pass over the data.
**Suggestion**: If performance becomes a bottleneck for high-volume streams, consider parsing directly to the typed event first:
```rust
if let Ok(event) = serde_json::from_str::<ProviderEvent>(line) {
    // handle typed event
} else if let Ok(raw) = serde_json::from_str::<Value>(line) {
    // handle fallback/tracing
}
```
*Note: The current approach is safer for retaining the raw `Value` required by some `handle_result` methods, so this is a trade-off.*

### 2. Standardize Helper Naming
The protocol modules use slightly different names for similar helpers:
- `ClaudeToolUse::resolved_id()`
- `OpenCodeToolFields::tool_id()`
- `GeminiToolUse::resolved_tool_name()`
- `QwenTool::resolved_tool_id()`
**Suggestion**: Standardize on `resolved_tool_id()` and `resolved_tool_name()` across all providers to make the parser handlers more idiomatic.

### 3. Fully Type `AgentMessage` Content
For `Gemini`, `Qwen`, and `Codex`, the `content` field in messages is still `Option<Value>`.
**Suggestion**: Define a `ContentPart` struct (as was done in `protocol/claude.rs`) and use `Option<Vec<ContentPart>>` or a dedicated enum to eliminate the remaining `.get("text")` calls in the `handle_message` methods.

## Final Assessment

The implementation is **Ready for Production** with the minor exception of adding the missing `delta` field to Gemini for completeness. The deviation in `CodexItem` is a valid ergonomic choice for a messy protocol, though it leaves some minor "Value-walking" in the parser.
