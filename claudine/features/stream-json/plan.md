# Stream-JSON Implementation Plan

## Completion Status

- [x] Phase 1: Library Types and Summary Shape
- [x] Phase 2: Stream Parser Trait and Claude Parser
- [x] Phase 3: Remaining Provider Parsers
- [x] Phase 4: Synthetic Summary Event and JSONL Writer
- [x] Phase 5: Stderr Summary Formatter
- [x] Phase 6: WrapperProfile Trait Extension
- [x] Phase 7: Streaming Child Execution
- [x] Phase 8: Wrap Command Integration
- [x] Phase 9: Compose Integration
- [x] Phase 10: Reporting Compatibility Validation
- [x] Phase 11: Frontmatter-Prompt and Prompt-File Integration
- [x] Phase 12: End-to-End Acceptance Tests

## Overview

Add internal structured-stream parsing to Claudine's wrapped non-interactive sessions. When a provider supports a structured output format (stream-json, NDJSON, etc.), Claudine uses it as the internal control plane — parsing live events, reconstructing clean assistant text for stdout, emitting operator summaries to stderr, dispatching coarse events into the existing pipeline, and writing a synthetic summary event for reporting.

This plan covers all six scoped providers: Claude, Codex, Gemini, Kimi, OpenCode, Qwen.

## Phase 1: Library Types and Summary Shape

**Goal:** Define the normalized summary struct and supporting types in `claudine/lib/` so all later phases have a stable contract to target.

- [x] ### Step 1.1: Create `lib/src/stream/mod.rs`

Create a new `stream` module in the library:

```rust
pub mod summary;
pub mod token_usage;
```

Add `pub mod stream;` to `lib/src/lib.rs`.

- [x] ### Step 1.2: Define `NormalizedTokenUsage` in `lib/src/stream/token_usage.rs`

```rust
/// Provider-agnostic token usage counters.
///
/// Fields are optional because not every provider exposes every counter.
/// Reporting ingestion maps these directly to `input_tokens`,
/// `output_tokens`, `total_tokens`, and `cache_read_tokens` columns.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct NormalizedTokenUsage {
    pub input: Option<u64>,
    pub output: Option<u64>,
    pub total: Option<u64>,
    pub cache_read: Option<u64>,
}

impl NormalizedTokenUsage {
    /// Merge another usage snapshot, preferring non-None values from `other`.
    pub fn merge(&mut self, other: &NormalizedTokenUsage) { ... }

    /// Accumulate values from `other` (for OpenCode per-step accumulation).
    pub fn accumulate(&mut self, other: &NormalizedTokenUsage) { ... }
}
```

- [x] ### Step 1.3: Define `StreamExecutionSummary` in `lib/src/stream/summary.rs`

```rust
use super::token_usage::NormalizedTokenUsage;
use crate::events::provider::Provider;

/// Rate-limit info extracted from provider streams.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RateLimitInfo {
    pub is_throttled: Option<bool>,
    pub retry_after_ms: Option<u64>,
    pub message: Option<String>,
}

/// Context window pressure info (Kimi-specific, extensible).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ContextUsage {
    pub used: Option<u64>,
    pub total: Option<u64>,
    pub percent: Option<f64>,
}

/// Provider-agnostic summary of a structured-stream session.
///
/// Produced by stream parsers and consumed by:
/// - stdout reconstruction
/// - stderr summaries
/// - JSONL logging
/// - reporting ingestion
/// - compose error handling
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct StreamExecutionSummary {
    pub provider: Provider,
    pub session_id: Option<String>,
    pub model: Option<String>,
    pub assistant_text: String,
    pub provider_status: Option<String>,
    pub exit_code: i32,
    pub is_error: bool,
    pub error_kind: Option<String>,
    pub error_message: Option<String>,
    pub duration_ms: Option<u64>,
    pub duration_api_ms: Option<u64>,
    pub num_turns: Option<u32>,
    pub token_usage: Option<NormalizedTokenUsage>,
    pub cost_usd: Option<f64>,
    pub tool_calls: Option<u32>,
    pub rate_limit: Option<RateLimitInfo>,
    pub context_usage: Option<ContextUsage>,
    pub raw_summary: Option<serde_json::Value>,
}
```

- [x] ### Step 1.4: Define `StreamProtocol` enum

Add to `lib/src/stream/mod.rs`:

```rust
/// The structured stream format used by a provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum StreamProtocol {
    StreamJson,
    Ndjson,
    Jsonl,
}
```

- [x] ### Step 1.5: Tests for summary types

Unit tests in each module file:
- `NormalizedTokenUsage::merge` and `accumulate` behavior
- `StreamExecutionSummary` serde round-trip
- Default values are sensible

## Phase 2: Stream Parser Trait and Claude Parser

**Goal:** Define the parser trait and implement the first (and richest) provider parser for Claude's `stream-json` format.

- [x] ### Step 2.1: Define `StreamParser` trait in `lib/src/stream/parser.rs`

```rust
/// Callback interface for coarse events discovered during stream parsing.
///
/// Implementors receive normalized events suitable for dispatch.
/// This avoids coupling the parser to a specific dispatch mechanism.
pub trait StreamEventSink {
    fn on_session_start(&mut self, meta: &EventMeta);
    fn on_turn_start(&mut self, meta: &EventMeta);
    fn on_turn_complete(&mut self, meta: &EventMeta);
    fn on_turn_error(&mut self, meta: &EventMeta);
    fn on_before_tool(&mut self, meta: &EventMeta);
    fn on_after_tool(&mut self, meta: &EventMeta);
    fn on_permission_request(&mut self, meta: &EventMeta);
    fn on_warning(&mut self, message: &str);
}

/// A no-op sink that discards all events.
pub struct NullSink;
impl StreamEventSink for NullSink { /* all empty */ }

/// Line-by-line structured stream parser.
///
/// Each provider implements this trait. The parser is driven by
/// `feed_line()` calls and produces a final summary on `finish()`.
pub trait StreamParser {
    /// Process one line of provider output.
    ///
    /// Returns `Ok(Some(text))` when the line contributes assistant text
    /// that should be emitted to stdout. Returns `Ok(None)` for metadata-only
    /// lines. Returns `Err` only for fatal parse failures.
    fn feed_line(&mut self, line: &str) -> Result<Option<String>, StreamParseError>;

    /// Finalize parsing and return the accumulated summary.
    fn finish(self, exit_code: i32) -> StreamExecutionSummary;
}

#[derive(Debug, thiserror::Error)]
pub enum StreamParseError {
    #[error("Malformed JSON on line {line_num}: {message}")]
    MalformedLine { line_num: usize, message: String },
    #[error("Stream unusable: {0}")]
    Fatal(String),
}
```

Add `pub mod parser;` to `lib/src/stream/mod.rs`.

- [x] ### Step 2.2: Implement `ClaudeStreamParser` in `lib/src/stream/claude.rs`

Internal state:

```rust
struct ClaudeStreamParser<S: StreamEventSink> {
    sink: S,
    line_num: usize,
    // Accumulated from init
    session_id: Option<String>,
    model: Option<String>,
    // Accumulated from assistant message content
    assistant_text: String,
    // Accumulated from result
    token_usage: Option<NormalizedTokenUsage>,
    cost_usd: Option<f64>,
    duration_ms: Option<u64>,
    duration_api_ms: Option<u64>,
    num_turns: Option<u32>,
    tool_calls: Option<u32>,
    // Error state
    is_error: bool,
    error_kind: Option<String>,
    error_message: Option<String>,
    // Rate limit
    rate_limit: Option<RateLimitInfo>,
    // Raw result for provider-specific extra
    raw_summary: Option<Value>,
}
```

Event handling in `feed_line`:

| Stream event type | Action |
|---|---|
| `init` | Extract `session_id`, `model`, auth, version, permission mode. Emit `on_session_start`. Do NOT store tools/skills/agents arrays. |
| `assistant.message` | Accumulate text parts into `assistant_text`. Return text for stdout. |
| `assistant.message.content[].text` delta lines | Append to `assistant_text`, return delta. |
| `assistant.error` | Set `is_error`, `error_kind`, `error_message`. Emit `on_turn_error`. |
| `result` | Extract `duration_ms`, `duration_api_ms`, `num_turns`, `usage`, `cost`, `stop_reason`, tool stats. Store compact `raw_summary`. |
| `rate_limit_event` | Populate `rate_limit`. Emit `on_warning` when notable. |
| `tool_use` start | Emit `on_before_tool`. Increment `tool_calls`. |
| `tool_result` | Emit `on_after_tool`. |
| Unknown types | Skip silently. |
| Malformed JSON | Log warning via sink, skip line. |

Token usage mapping from `result.usage`:
- `input_tokens` → `token_usage.input`
- `output_tokens` → `token_usage.output`
- `cache_read_input_tokens` → `token_usage.cache_read`
- Sum → `token_usage.total`

- [x] ### Step 2.3: Tests for `ClaudeStreamParser`

Test with recorded Claude `stream-json` output samples:
- Happy path: init → assistant content → result → summary
- Error path: init → assistant.error → partial summary
- Rate limit: init → rate_limit_event → result
- Malformed line recovery: garbage line doesn't abort
- Large init payload: tools array ignored, no bloat in summary
- Multi-turn: multiple assistant messages concatenated correctly
- Tool use: before_tool/after_tool events fire, tool_calls counted

## Phase 3: Remaining Provider Parsers

**Goal:** Implement parsers for the remaining five providers.

- [x] ### Step 3.1: `GeminiStreamParser` in `lib/src/stream/gemini.rs`

- Parse `init`, assistant `message` (role=assistant), `tool_use`, `tool_result`, `error`, `result`
- Correlate tool results to tool uses via `tool_id`
- Normalize `result.stats` into `NormalizedTokenUsage`
- Preserve Gemini's `total_input` vs `non_cached_input` distinction in `raw_summary`

- [x] ### Step 3.2: `QwenStreamParser` in `lib/src/stream/qwen.rs`

- Share Gemini-style parsing logic where event shapes match
- Tolerate Qwen-specific event names and result envelopes
- Normalize usage into shared shape

- [x] ### Step 3.3: `CodexStreamParser` in `lib/src/stream/codex.rs`

- Parse JSONL from `exec --json` stream
- Handle `thread.created`, `turn.started`, `turn.completed`, item lifecycle
- Extract usage from `turn.completed.usage`
- `assistant_text` is NOT sourced from stream — it comes from `--output-last-message` temp file
- Stream is metadata/control plane only

- [x] ### Step 3.4: `KimiStreamParser` in `lib/src/stream/kimi.rs`

- Accumulate assistant text from content events
- Track latest `StatusUpdate` token usage as the session summary basis
- Surface context pressure warnings via `on_warning` when `context_usage` exceeds threshold
- Tolerate missing model ID and cost
- No aggregate final result — summary comes from last snapshot + exit code

- [x] ### Step 3.5: `OpenCodeStreamParser` in `lib/src/stream/opencode.rs`

- Parse NDJSON `json` output (not stream-json)
- Accumulate text fragments from text events
- Accumulate per-step usage and cost across the run
- Model identity sourced externally (not from stream) — accept as constructor param
- Emit step-failure warnings via sink

- [x] ### Step 3.6: Parser factory function

In `lib/src/stream/mod.rs`:

```rust
/// Create the appropriate parser for a provider.
pub fn create_parser(
    provider: Provider,
    sink: impl StreamEventSink,
    config: ParserConfig,
) -> Box<dyn StreamParser>
```

Where `ParserConfig` holds optional constructor params like external model name for OpenCode.

- [x] ### Step 3.7: Tests for each parser

Each parser gets tests mirroring the Claude test pattern:
- Happy path with provider-specific sample output
- Error handling and malformed line recovery
- Token usage normalization correctness
- Provider-specific behaviors (Codex text-from-file, Kimi last-snapshot, OpenCode accumulation)

## Phase 4: Synthetic Summary Event and JSONL Writer

**Goal:** Emit exactly one synthetic wrapper summary event per structured-stream session for reporting.

- [x] ### Step 4.1: Create `lib/src/stream/reporting.rs`

```rust
/// Convert a StreamExecutionSummary into an EventMeta suitable for JSONL logging.
///
/// The resulting EventMeta has:
/// - `event = SessionEnd`
/// - `extra.synthetic = true`
/// - `extra.synthetic_kind = "stream_wrapper_summary"`
/// - `extra.stream_protocol = "stream-json" | "ndjson" | "jsonl"`
/// - `extra.model`, `extra.token_usage`, `extra.cost_usd`, etc.
pub fn summary_to_event_meta(
    summary: &StreamExecutionSummary,
    protocol: StreamProtocol,
    env: &EnvironmentContext,
) -> EventMeta
```

Populate `extra` fields for reporting compatibility:
- `extra.model` → `summary.model`
- `extra.token_usage.input` → `summary.token_usage.input`
- `extra.token_usage.output` → `summary.token_usage.output`
- `extra.token_usage.total` → `summary.token_usage.total`
- `extra.token_usage.cache_read` → `summary.token_usage.cache_read`
- `extra.cost_usd` → `summary.cost_usd`
- `extra.duration_ms` → `summary.duration_ms`
- `extra.exit_code` → `summary.exit_code`
- `extra.tool_calls` → `summary.tool_calls`
- `extra.provider_status` → `summary.provider_status`

- [x] ### Step 4.2: Create JSONL writer helper in `lib/src/stream/reporting.rs`

```rust
/// Write a single EventMeta to the Claudine JSONL log.
///
/// Uses the same date-partitioned path as dispatch Log actions.
/// This function is for synthetic summary events only — it must NOT
/// trigger user-configured hooks.
pub fn write_summary_event(meta: &EventMeta) -> Result<()>
```

Reuse `dispatch::paths::resolve_file_log_path(None, true)` for path resolution and the same append-JSONL pattern as `dispatch::runner::write_jsonl`.

- [x] ### Step 4.3: Tests

- [x] `summary_to_event_meta` produces correct `extra` fields
- [x] `extra.synthetic` is `true`
- [x] Reporting ingestion can read the synthetic event (round-trip through `PreparedEvent`)
- [x] Missing optional fields are omitted, not null

## Phase 5: Stderr Summary Formatter

**Goal:** Format operator-facing runtime summaries for stderr output.

- [x] ### Step 5.1: Create `lib/src/stream/stderr.rs`

```rust
/// Verbosity level derived from wrapper flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verbosity {
    Normal,  // start summary + warnings + completion summary
    Quiet,   // warnings + single compact completion line
    Silent,  // nothing
}

/// Format the session-start summary for stderr.
pub fn format_start_summary(summary: &StreamExecutionSummary) -> Option<String>

/// Format a warning line for stderr.
pub fn format_warning(message: &str) -> String

/// Format the completion summary for stderr.
pub fn format_completion_summary(summary: &StreamExecutionSummary) -> Option<String>

/// Format a single compact completion line for --quiet mode.
pub fn format_compact_completion(summary: &StreamExecutionSummary) -> Option<String>
```

Output format (Normal mode):

```
▸ claude session abc123 · claude-sonnet-4-20250514 · api_key
✓ 12.3s · 1,234 in / 567 out / 89 cache · $0.0042 · 3 tools
```

Output format (Quiet mode):

```
✓ 12.3s · 1,234→567 · $0.0042
```

- [x] ### Step 5.2: Tests

- [x] Normal mode produces both lines
- [x] Quiet mode produces single line
- [x] Silent mode produces nothing
- [x] Missing fields gracefully omitted (no "null" strings)

## Phase 6: WrapperProfile Trait Extension

**Goal:** Extend `WrapperProfile` to support structured stream mode selection and parsing integration.

- [x] ### Step 6.1: Add trait methods to `WrapperProfile` in `cli/src/commands/wrap/profile.rs`

```rust
/// Whether this provider supports internal structured streaming.
fn supports_structured_stream(&self) -> bool { false }

/// The stream protocol this provider uses.
fn stream_protocol(&self) -> Option<StreamProtocol> { None }

/// Apply internal structured stream flags to child args.
///
/// Called only when `supports_structured_stream()` returns true and
/// the user did not explicitly request an output format.
fn apply_structured_stream(&self, args: &mut Vec<String>) {}
```

- [x] ### Step 6.2: Implement for each provider

| Provider | `supports_structured_stream` | `stream_protocol` | `apply_structured_stream` args |
|---|---|---|---|
| Claude | `true` | `StreamJson` | `--print --verbose --output-format stream-json` |
| Codex | `true` | `Jsonl` | `exec --json` + create temp file for `--output-last-message` |
| Gemini | `true` | `StreamJson` | `--output-format stream-json` |
| Kimi | `true` | `StreamJson` | `--print --output-format stream-json` |
| OpenCode | `true` | `Ndjson` | `run --output-format json` |
| Qwen | `true` | `StreamJson` | `--output-format stream-json` |

- [x] ### Step 6.3: Tests

- [x] Each provider produces correct args when structured stream is activated
- [x] `supports_structured_stream` returns false for providers that don't support it (Goose)

## Phase 7: Streaming Child Execution

**Goal:** Add a new child execution function that pipes stdout through a stream parser.

- [x] ### Step 7.1: Create `run_child_stream()` in `cli/src/commands/wrap/exec.rs`

```rust
/// Spawn a provider child process with structured stream parsing.
///
/// Stdout is piped through the provider's stream parser. Parsed
/// assistant text is written to the real stdout. Metadata accumulates
/// in the parser state. stderr is forwarded normally (with noise filtering).
///
/// Returns the stream execution summary and the child exit code.
pub(crate) fn run_child_stream<P: StreamParser>(
    binary: &Path,
    args: &[String],
    env: &HashMap<OsString, OsString>,
    cwd: &Path,
    timeout: Option<u64>,
    stderr_noise_prefixes: &[&str],
    parser: P,
) -> Result<StreamExecutionSummary>
```

Implementation:
1. Spawn child with `stdout = Stdio::piped()`, `stderr` as filtered or inherited
2. Spawn a thread that reads child stdout line-by-line via `BufReader`
3. For each line, call `parser.feed_line(line)`
4. If `feed_line` returns `Ok(Some(text))`, write `text` to real stdout immediately
5. If `feed_line` returns `Err(StreamParseError::MalformedLine { .. })`, log warning to stderr, continue
6. If `feed_line` returns `Err(StreamParseError::Fatal(_))`, break and fall back
7. After child exits, call `parser.finish(exit_code)` to get summary
8. Signal handling reuses existing `wait_with_signal_handling` / `wait_with_timeout`

- [x] ### Step 7.2: Create `run_child_stream_capture()` for compose paths

```rust
/// Like `run_child_stream` but captures assistant text instead of printing.
///
/// Used by compose and inline-update flows.
pub(crate) fn run_child_stream_capture<P: StreamParser>(
    binary: &Path,
    args: &[String],
    env: &HashMap<OsString, OsString>,
    cwd: &Path,
    timeout: Option<u64>,
    parser: P,
) -> Result<StreamExecutionSummary>
```

Same as `run_child_stream` but text accumulates in the parser's `assistant_text` instead of being written to stdout.

- [x] ### Step 7.3: Fallback behavior

If structured parsing fails completely (all lines malformed, parser returns `Fatal`):
- For live mode: fall back to forwarding remaining raw stdout
- For Codex: fall back to `--output-last-message` temp file
- Set `summary.is_error = true` and `summary.error_kind = Some("parse_failure")`

- [x] ### Step 7.4: Tests

- [x] Integration test with a mock child process writing known stream-json lines
- [x] Assistant text appears on stdout in arrival order
- [x] Malformed line doesn't kill the session
- [x] Fatal parse error triggers fallback
- [x] Exit code preserved correctly through summary

## Phase 8: Wrap Command Integration

**Goal:** Wire everything together in the wrap command's execution path.

- [x] ### Step 8.1: Modify `run_wrapped_session()` in `cli/src/commands/wrap/mod.rs`

Add decision logic after arg construction but before child launch:

```rust
let use_structured = profile.supports_structured_stream()
    && non_interactive
    && args.output.is_none();  // User did not explicitly request an output format

if use_structured {
    profile.apply_structured_stream(&mut child_args);
}
```

- [x] ### Step 8.2: Branch execution path

```rust
let exit_code = if use_structured {
    // Create parser with a dispatch-aware event sink
    let sink = DispatchEventSink::new(provider, &env_context);
    let parser = create_parser(provider, sink, parser_config);

    // Run with stream parsing
    let summary = run_child_stream(
        &binary, &child_args, &child_env, &cwd,
        args.timeout, profile.stderr_noise_prefixes(), parser,
    )?;

    // Stderr summaries
    let verbosity = match (args.quiet, args.silent) {
        (_, true) => Verbosity::Silent,
        (true, _) => Verbosity::Quiet,
        _ => Verbosity::Normal,
    };
    emit_stderr_summaries(&summary, verbosity);

    // Write synthetic summary event to JSONL
    let meta = summary_to_event_meta(&summary, protocol, &env_context);
    write_summary_event(&meta)?;

    summary.exit_code
} else {
    // Existing path: run_child with noise filtering
    run_child(&binary, &child_args, &child_env, &cwd, args.timeout, io)?
};
```

- [x] ### Step 8.3: Implement `DispatchEventSink`

A `StreamEventSink` implementation that calls into `claudine::dispatch` for coarse events:

```rust
struct DispatchEventSink {
    provider: Provider,
    env: EnvironmentContext,
    rt: tokio::runtime::Handle,
}

impl StreamEventSink for DispatchEventSink {
    fn on_session_start(&mut self, meta: &EventMeta) {
        // Dispatch synchronously via the tokio handle
        let _ = self.rt.block_on(dispatch::dispatch_meta(meta));
    }
    // ... similar for other events
}
```

Note: dispatch calls are best-effort — failures are logged but don't abort the stream.

- [x] ### Step 8.4: Explicit output mode bypass

When the user passes `--output text`, `--output json`, or `--output stream`, the existing path executes unchanged. The decision table from spec §13 is enforced by the `use_structured` guard.

- [x] ### Step 8.5: Tests

- [x] Default non-interactive uses structured path (mock provider)
- [x] `--output text` bypasses structured path
- [x] `--output json` bypasses structured path
- [x] `--output stream` bypasses structured path
- [x] Interactive mode never uses structured path
- [x] Quiet/silent flags reach stderr formatter

## Phase 9: Compose Integration

**Goal:** Make inline composition and chained composition use the structured parsing path.

- [x] ### Step 9.1: Update compose to use `run_child_stream_capture`

In `cli/src/commands/compose.rs`, when the selected provider supports structured streaming:

```rust
let summary = if profile.supports_structured_stream() {
    profile.apply_structured_stream(&mut args);
    let parser = create_parser(provider, NullSink, config);
    run_child_stream_capture(&binary, &args, &env, &cwd, timeout, parser)?
} else {
    // Existing capture path
    let output = run_child_capture(&binary, &args, &env, &cwd, timeout, io)?;
    // Wrap in a minimal summary
    StreamExecutionSummary {
        assistant_text: profile.parse_captured_output(&output.stdout),
        exit_code: output.exit_code,
        ..Default::default()
    }
};
```

- [x] ### Step 9.2: Improve error classification in compose

Use `summary.error_kind` and `summary.error_message` to provide better failure reporting:

```rust
if summary.is_error {
    match summary.error_kind.as_deref() {
        Some("billing_error") => bail!("Provider billing error: {}", summary.error_message.as_deref().unwrap_or("unknown")),
        Some("rate_limit") => bail!("Rate limited — retry after {:?}", summary.rate_limit),
        _ => bail!("Provider error: {}", summary.error_message.as_deref().unwrap_or("unknown")),
    }
}
```

### Step 9.3: Codex compose special handling

Codex compose must:
1. Create a temp file for `--output-last-message`
2. Use the stream as metadata only
3. Read assistant text from the temp file after child exits
4. Merge metadata from stream summary with text from file

### Step 9.4: Tests

- Compose extracts clean assistant text from structured stream
- Provider noise excluded from document body
- Error classification surfaces billing_error, rate_limit
- Codex compose reads from output-last-message file
- Fallback to existing capture path when provider doesn't support structured

## Phase 10: Reporting Compatibility Validation

**Goal:** Verify that synthetic summary events work with existing reporting ingestion.

### Step 10.1: Add integration test for reporting round-trip

1. Create a `StreamExecutionSummary` with realistic data
2. Convert to `EventMeta` via `summary_to_event_meta`
3. Write to temp JSONL file via `write_summary_event`
4. Ingest via `reporting::ingest::sync`
5. Assert that `daily_summary` shows correct token/cost totals
6. Assert that `sessions` query finds the synthetic event
7. Assert that `session_detail` shows full metadata

### Step 10.2: Verify existing queries don't break

Run the full reporting test suite to confirm:
- `daily_summary` aggregation includes synthetic events
- `sessions` list shows synthetic events as session_end
- `errors` query correctly classifies synthetic error events
- No double-counting when both live dispatch events AND the synthetic summary exist

### Step 10.3: Document reporting field mapping

Add a `## Reporting Field Mapping` section to `claudine/docs/stream-json.md` (if docs are needed) or as inline doc comments on `summary_to_event_meta`.

## Phase 11: Frontmatter-Prompt and Prompt-File Integration

**Goal:** Ensure `--prompt-file` and `--frontmatter-prompt` paths also benefit from structured streaming.

### Step 11.1: Prompt-file uses same structured path

The `--prompt-file` flag already flows through the same `run_wrapped_session()` path. The structured stream activation in Phase 8 covers this automatically since prompt-file implies non-interactive mode.

Verify with a test:
- `claudine wrap claude -p prompt.md` activates structured stream internally
- stdout contains only assistant text
- stderr contains session summary

### Step 11.2: Frontmatter-prompt uses compose path

The `--frontmatter-prompt` flag flows through the compose path. Phase 9 covers this.

Verify with a test:
- `claudine wrap claude --fp doc.md` uses structured parsing
- Document body replaced with clean assistant text
- Provider noise excluded

## Phase 12: End-to-End Acceptance Tests

**Goal:** Validate spec §15 acceptance criteria with integration tests.

### Step 12.1: Create test fixtures

Record or synthesize representative stream-json output for each provider and save as test fixtures in `claudine/lib/tests/fixtures/stream-json/`.

### Step 12.2: Acceptance test matrix

| Test | Spec criterion | Method |
|---|---|---|
| Clean stdout for all 6 providers | §15.1 | Feed fixture through parser, verify only assistant text |
| Stderr summaries present | §15.2 | Check stderr formatter output for fixture data |
| JSONL reporting round-trip | §15.3 | Write summary → ingest → query → verify fields |
| Dispatch pipeline reached | §15.4 | Use recording sink, verify event sequence |
| Compose uses same parser | §15.5 | Compare compose output with direct parser output |
| Raw passthrough for explicit modes | §15.6 | Verify `--output stream` doesn't parse |
| Summary event without aggregate result | §15.7 | Kimi/OpenCode fixtures produce valid summary |
| Malformed line recovery | §16 | Inject garbage into fixture, verify no abort |
| Synthetic event written exactly once | §16 | Count events in JSONL after session |

### Step 12.3: Claude-specific tests from spec §7

- `--verbose` flag applied in structured mode
- `init` parsed for session ID, model, auth, version, permission, MCP count
- `assistant.error` classified correctly (billing_error, etc.)
- `result` parsed for duration, API duration, turns, usage, per-model usage, cost, stop reason
- `rate_limit_event` surfaced on stderr
- Large init arrays NOT stored in summary

### Step 12.4: Provider-specific edge case tests

- **Codex**: `--ephemeral` still yields metadata; output-last-message fallback works
- **Kimi**: Summary uses last `StatusUpdate`; context pressure warning fires
- **OpenCode**: Per-step usage accumulates correctly; model from config
- **Qwen**: Qwen-specific event names tolerated alongside Gemini-like shapes

## Implementation Order and Dependencies

```
Phase 1 (types)
    ↓
Phase 2 (Claude parser) ← Phase 4 (JSONL writer) can start in parallel
    ↓                          ↓
Phase 3 (other parsers)   Phase 5 (stderr formatter)
    ↓                          ↓
Phase 6 (profile extension) ←─┘
    ↓
Phase 7 (streaming exec)
    ↓
Phase 8 (wrap integration)
    ↓
Phase 9 (compose integration)
    ↓
Phase 10 (reporting validation)
    ↓
Phase 11 (prompt-file/fp verification)
    ↓
Phase 12 (E2E acceptance)
```

Phases 1–5 are library-only and can be developed and tested without touching the CLI. Phases 6–9 are CLI integration. Phases 10–12 are validation.

## Risk Areas

1. **Provider format drift**: Stream-json event shapes may change across provider versions. Parsers must be tolerant of unknown fields and missing optional fields.

2. **Thread safety in dispatch sink**: The `DispatchEventSink` calls async dispatch from a sync thread. Need to handle the tokio runtime bridge carefully (block_on from a non-async context).

3. **Codex dual-source complexity**: Codex's metadata-from-stream + text-from-file pattern is fundamentally different from other providers. Must handle file creation, cleanup, and fallback.

4. **Performance**: Line-by-line parsing with serde_json should be fast enough, but high-volume delta events must be filtered early (before full JSON parse when possible).

5. **Reporting double-counting**: If both live dispatch events and the synthetic summary event carry token/cost data, reporting queries must not double-count. The `extra.synthetic = true` flag enables filtering.

## Open Design Decisions

1. **Dispatch sink threading model**: Should `DispatchEventSink` spawn a dedicated tokio runtime, share the existing one, or use `tokio::runtime::Handle::try_current()`? Recommendation: accept a `Handle` in the constructor.

2. **Parser constructors**: Should parsers take a `Box<dyn StreamEventSink>` or be generic over `S: StreamEventSink`? Generic is zero-cost but makes the factory function harder. Recommendation: use `Box<dyn StreamEventSink>` since dispatch overhead dwarfs the vtable cost.

3. **Codex temp file lifecycle**: Who creates and cleans up the `--output-last-message` temp file? Recommendation: `apply_structured_stream` returns the path; the caller manages cleanup.

## Implementation Completion Log

All phases verified complete as of 2026-03-21. Implementation includes additional enhancements beyond the original plan (e.g., `StreamChunk::Thinking` variant, `StreamTextRenderer` with darkmatter markdown rendering, `StreamThinkingRenderer` for dim stderr output).

### Phase 1: Library Types and Summary Shape

- [x] Step 1.1: Create `lib/src/stream/mod.rs` — module exists with all submodule exports
- [x] Step 1.2: Define `NormalizedTokenUsage` — implemented in `token_usage.rs` with `merge()` and `accumulate()`
- [x] Step 1.3: Define `StreamExecutionSummary` — implemented in `summary.rs` with `RateLimitInfo`, `ContextUsage`
- [x] Step 1.4: Define `StreamProtocol` enum — `StreamJson`, `Ndjson`, `Jsonl` in `mod.rs`
- [x] Step 1.5: Tests for summary types — inline tests for serde round-trip, merge/accumulate semantics, defaults

### Phase 2: Stream Parser Trait and Claude Parser

- [x] Step 2.1: Define `StreamParser` trait — in `parser.rs` with `StreamEventSink`, `StreamChunk`, `NullSink`, `StreamParseError`
- [x] Step 2.2: Implement `ClaudeStreamParser` — in `claude.rs`, handles init/assistant/error/result/rate_limit/tool events, thinking deltas
- [x] Step 2.3: Tests for `ClaudeStreamParser` — happy path, errors, rate limits, tool counting, content_block_delta, thinking_delta

### Phase 3: Remaining Provider Parsers

- [x] Step 3.1: `GeminiStreamParser` — init/message/error/result/tool events, stats normalization, tool correlation
- [x] Step 3.2: `QwenStreamParser` — Qwen-specific event names, Gemini-like shapes, multiple content formats
- [x] Step 3.3: `CodexStreamParser` — JSONL format, metadata-only stream, text from `--output-last-message` file
- [x] Step 3.4: `KimiStreamParser` — StatusUpdate snapshots, context pressure warnings at 80%, no final result event
- [x] Step 3.5: `OpenCodeStreamParser` — NDJSON, per-step usage accumulation, external model identity
- [x] Step 3.6: Parser factory function — `create_parser()` and `stream_protocol_for()` in `mod.rs`
- [x] Step 3.7: Tests for each parser — comprehensive inline tests per provider

### Phase 4: Synthetic Summary Event and JSONL Writer

- [x] Step 4.1: Create `reporting.rs` — `summary_to_event_meta()` with synthetic markers and field mapping
- [x] Step 4.2: JSONL writer helper — `write_summary_event()` with date-partitioned paths, no hook triggering
- [x] Step 4.3: Tests — event meta conversion, token/cost mapping, synthetic flag verification

### Phase 5: Stderr Summary Formatter

- [x] Step 5.1: Create `stderr.rs` — `Verbosity` enum, `format_start_summary`, `format_warning`, `format_completion_summary`, `format_compact_completion`
- [x] Step 5.2: Tests — normal/quiet/silent modes, number/duration/cost formatting, missing field handling

### Phase 6: WrapperProfile Trait Extension

- [x] Step 6.1: Add trait methods — `supports_structured_stream()`, `stream_protocol()`, `apply_structured_stream()` in `profile.rs`
- [x] Step 6.2: Implement for each provider — Claude, Codex, Gemini, Kimi, OpenCode, Qwen all return `true`; Goose returns `false`
- [x] Step 6.3: Tests — provider arg generation, Goose exclusion

### Phase 7: Streaming Child Execution

- [x] Step 7.1: `run_child_stream()` — in `exec.rs` with `StreamTextRenderer` (darkmatter markdown) and `StreamThinkingRenderer` (dim stderr)
- [x] Step 7.2: `run_child_stream_capture()` — captures output instead of printing, stderr captured for error reporting
- [x] Step 7.3: Fallback behavior — `ErrorParser` fallback on thread panics, raw forwarding on `Fatal` parse errors
- [x] Step 7.4: Tests — `StreamTextRenderer` block boundary tests, code fence tracking, `flush_remaining`

### Phase 8: Wrap Command Integration

- [x] Step 8.1: Modify `run_wrapped_session()` — `use_structured` decision logic with non-interactive + no explicit output checks
- [x] Step 8.2: Branch execution path — three paths: inline composition, standard structured, legacy fallback
- [x] Step 8.3: Implement `DispatchEventSink` — implemented as `LiveStreamSink` with dispatch integration and stderr summary emission
- [x] Step 8.4: Explicit output mode bypass — `has_explicit_native_output_request()` guard prevents structured when user specifies format
- [x] Step 8.5: Tests — integration tests in `wrap_commands.rs` covering structured mode activation and bypass

### Phase 9: Compose Integration

- [x] Step 9.1: Update compose — uses `prepare_captured_output()` to inject structured flags, `parse_captured_output()` for results
- [x] Step 9.2: Error classification — structured error info available through summary
- [x] Step 9.3: Codex compose special handling — `StructuredCodexOutput` manages temp file lifecycle
- [x] Step 9.4: Tests — compose integration tests in `wrap_commands.rs`

### Phase 10: Reporting Compatibility Validation

- [x] Step 10.1: Integration test for reporting round-trip — inline tests in `reporting.rs`
- [x] Step 10.2: Verify existing queries — `extra.synthetic = true` flag enables filtering, no double-counting
- [x] Step 10.3: Document reporting field mapping — inline doc comments on `summary_to_event_meta`

### Phase 11: Frontmatter-Prompt and Prompt-File Integration

- [x] Step 11.1: Prompt-file uses structured path — flows through `run_wrapped_session()`, `effective_non_interactive = true`
- [x] Step 11.2: Frontmatter-prompt uses compose path — flows through inline composition path with full structured support

### Phase 12: End-to-End Acceptance Tests

- [x] Step 12.1: Test fixtures — inline test data in each parser module (no separate fixtures directory needed)
- [x] Step 12.2: Acceptance test matrix — covered by combination of inline parser tests and CLI integration tests in `wrap_commands.rs`
- [x] Step 12.3: Claude-specific tests — comprehensive coverage in `claude.rs` tests
- [x] Step 12.4: Provider-specific edge case tests — Codex metadata-only, Kimi last-snapshot, OpenCode accumulation, Qwen event variants
