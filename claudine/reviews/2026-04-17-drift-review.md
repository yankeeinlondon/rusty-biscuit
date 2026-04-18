# Non-Interactive Sessions Drift Review (2026-04-17)

This review evaluates the current implementation of non-interactive sessions in claudine, identifies divergences from the documentation, and proposes refactors to improve consistency and reduce code duplication.

## 1. Documentation Divergence

The implementation in `claudine/lib/src/stream/` generally follows the spec in `claudine/docs/topics/non-interactive-sessions.md`, with the following notable divergences:

| Area | Documentation Claim | Implementation Status | Recommendation |
|------|----------------------|-----------------------|----------------|
| **Qwen Reasoning** | "emits `<think>…</think>` blocks inline... rather than as a separate event" | **Missing**. `qwen_semantic.rs` does not yet attempt to parse or extract `<think>` tags from `assistant_message` content. | Implement a regex-based extractor in `qwen_semantic.rs` to split `<think>` blocks into `SemanticEvent::Reasoning` events. |
| **Gemini Reasoning** | "thinkingConfig.includeThoughts controls model-side emission but no dedicated event... observed" | **Confirmed**. `gemini_semantic.rs` lacks reasoning support. | Monitor Gemini `stream-json` updates. If thinking blocks appear as a new message kind or field, add them to `handle_message`. |
| **Error Classification** | "Provider Error Classification" table lists specific mappings. | **Partially DRY**. Classification is implemented per-parser (`classify_error` helper) leading to slight logic drift. | Centralize error classification in `claudine/lib/src/stream/mod.rs` using a shared mapping table. |

## 2. DRY Opportunities & Architectural Improvements

The current semantic parser implementations share significant structural patterns that should be consolidated.

### A. Parser State Consolidation
Every `*SemanticStreamParser` maintains nearly identical state (line_num, session_id, model, token_usage, cost, tool_calls, etc.).
- **Recommendation:** Introduce a `ParserState` struct or a `BaseParser` trait that provides default implementations for `base_extra`, `trace_parser_event`, and malformed JSON warnings.

### B. Error Classification Centralization
Claude, OpenCode, and Qwen each have a local `classify_error` function.
- **Recommendation:** Move this logic to `claudine/lib/src/stream/semantic.rs` as `SemanticErrorKind::classify(provider, kind, message)`. This ensures that a "rate limit" string is classified as `ApiRemote` consistently across all providers.

### C. Tool Use / Result Tracking
Most parsers maintain a `tool_uses: HashMap<String, (Option<String>, Option<Value>)>` to correlate results with calls.
- **Recommendation:** Move this correlation logic into the `BaseParser` or a specialized `ToolTracker` helper.

### D. Malformed JSON Handling
The logic for `super::trace_malformed_line` and emitting a `SemanticEvent::Warning` is duplicated in every `feed_line` implementation.
- **Recommendation:** Move this into the `SemanticStreamParser` trait as a provided method or a shared helper in `parser.rs`.

## 3. Reporting & Visibility Improvements

- **Duration Fidelity**: Claude reports `duration_api_ms` (time spent in the provider's API) vs `duration_ms` (wall-clock time). Other providers only report one or the other. We should encourage parsers to differentiate these where the underlying protocol allows.
- **OpenCode TUI Suppression**: The list of `opencode_default_tui_noise_prefixes` in `profile.rs` should be periodically reviewed as OpenCode evolves its terminal formatter.
- **Gemini Echo Filtering**: The silent dropping of `role: "user"` messages in `gemini_semantic.rs` is a great example of "high-signal" filtering that should be documented as a pattern for other providers that "echo" the prompt.

## 4. Summary of Recommendations

1.  **Fix Qwen Reasoning**: Add inline `<think>` block extraction to `qwen_semantic.rs`.
2.  **Refactor for DRY**: Create a shared state/base for semantic parsers to eliminate the 5x duplication of metadata tracking and tracing.
3.  **Unified Error Mapping**: Centralize `SemanticErrorKind` resolution.
4.  **Audit Gemini Thinking**: Verify if recent Gemini models have introduced a structured field for thoughts in `stream-json`.
