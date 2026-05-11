# OpenCode Log Stream Integration — Functional Specification

Capture, parse, and surface structured diagnostic information from OpenCode's `--print-logs --log-level ERROR` stderr stream so Claudine can react to rate limits, malformed assets, and fatal errors in real time instead of hanging silently.

## Problem Statement

When Claudine wraps OpenCode in non-interactive mode (`opencode run`), the JSON event stream on stdout is the sole information channel. Two critical failure modes are invisible on that channel:

1. **Usage caps / rate limits** — OpenCode retries internally and then hangs when `AI_RetryError` exhausts `maxRetries`. The stdout stream goes silent. Claudine's non-interactive wrapper waits indefinitely.
2. **Malformed skills, commands, or agents** — Invalid frontmatter or missing files produce `failed to load skill` / `failed to load command` warnings on stderr, but the stdout stream continues normally. Callers never learn that assets were silently skipped.

OpenCode's `--print-logs --log-level ERROR` flag routes these diagnostics to stderr. Claudine currently discards stderr. This spec defines what to build so Claudine consumes, parses, classifies, and acts on that stderr log stream.

## Goals

- Parse OpenCode's structured log lines from stderr in real time, concurrently with the stdout JSON stream parser.
- Classify parsed log records into typed events that integrate with the existing `SemanticEvent` and `StreamExecutionSummary` models.
- Surface rate-limit events as `SemanticEvent::Warning` / `SemanticEvent::Error` so the existing 9-section renderer, badge system, and non-interactive exit path can display them.
- Extract reset-at times from rate-limit payloads so Claudine can report *when* the limit resets instead of hanging.
- Surface malformed-asset warnings as `SemanticEvent::Warning` so callers know that skills, commands, or agents were skipped.
- Preserve unrecognized log lines for diagnostic purposes without breaking the parser.
- Apply the same `#[serde(default)]` resilience strategy used in the existing `protocol/opencode.rs` typed structs so new upstream tags never break deserialization.

## Non-Goals

- Parsing log levels other than ERROR (future scope: WARN and INFO for richer diagnostics).
- Reconstructing per-service latencies from the `+Nms` delta field (it is process-global, not per-service).
- Maintaining a whitelist of `service=` tag values (the upstream inventory grows with every release).
- Building a persistence layer or analytics store for log records (the existing JSONL reporting pipeline handles this).

## Log Format Reference

The canonical format (from `packages/opencode/src/util/log.ts`, stable for 9+ months):

```text
<LEVEL><sp+> <ISO-SECONDS> +<delta>ms <key=value...> <message>\n
```

| Component | Shape | Notes |
|-----------|-------|-------|
| `LEVEL` | `DEBUG`, `INFO`, `WARN`, `ERROR` | Padded: `DEBUG`/`ERROR` get 1 trailing space; `INFO`/`WARN` get 2. Use `\s+`. |
| `ISO-SECONDS` | `2026-04-15T21:28:30` | UTC, second-resolution, no `Z` suffix. Parse as `chrono::NaiveDateTime` → `Utc`. |
| `+Nms` | `+123ms` | Milliseconds since the previous log line from any logger in the process. Not per-service. |
| `key=value` pairs | `service=llm providerID=z.ai error={...}` | Values: bare string, `JSON.stringify(obj)` inlined bare, or `formatError()` chain (unescaped, contains spaces). Key `error` is terminal — consume to EOL. |
| `message` | `stream error` | Final trailing text after all key=value pairs. |

Lines that do not match the header regex are **not log lines** — they are Bun uncaught-exception stacks or ANSI-colored error blocks and should be passed through as raw text.

See [research.md](./research.md) section "Logger Source of Truth" for the full specification.

## New Types

### `OpenCodeLogRecord`

A single parsed log line, defined in a new module `claudine/lib/src/stream/logs/opencode.rs`.

```rust
pub struct OpenCodeLogRecord {
    pub level: LogLevel,
    pub timestamp: chrono::NaiveDateTime,
    pub delta_ms: u64,
    pub tags: BTreeMap<String, String>,
    pub message: String,
    pub raw: String,
}

pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}
```

Design notes:

- `tags` is an open `BTreeMap<String, String>`, not a fixed struct, because the tag inventory grows with every OpenCode release.
- `raw` preserves the original line for diagnostics and JSONL reporting.
- `tags` values for JSON objects are stored as their serialized string form (the parser extracts them as raw strings from the line).

### `LogClassification`

Categorization of a parsed record into a Claudine-actionable event:

```rust
pub enum LogClassification {
    RateLimit {
        status_code: u16,
        reset_at: Option<String>,
        provider_error: String,
    },
    MalformedAsset {
        asset_type: AssetType,
        path: Option<String>,
        error: String,
    },
    ApiFailure {
        status_code: Option<u16>,
        error_name: String,
        message: String,
    },
    AuthFailure {
        message: String,
    },
    UncaughtError {
        raw_text: String,
    },
    Unclassified,
}

pub enum AssetType {
    Skill,
    Command,
    Agent,
    Config,
    Unknown,
}
```

## Parsing Strategy

### Header Regex

```rust
static HEADER_RE: &str = r"^(DEBUG|INFO|WARN|ERROR)\s+(\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2})\s+\+(\d+)ms\s+(.*)$";
```

Lines that do not match are passed through as raw text (potential stack traces or ANSI blocks).

### Body Walk

After extracting the header, walk the remaining body left-to-right:

1. Scan for `ident=` boundaries where `ident` matches `[a-zA-Z_][a-zA-Z0-9_.\-]*`.
2. For each boundary:
   - If value starts with `{` or `[`: use `serde_json` to find the matching end of the JSON literal, store the raw substring as the tag value.
   - If key is `error`: consume the rest of the line as the value (it contains unescaped spaces from `formatError()`).
   - Otherwise: read until the next whitespace followed by another `ident=` boundary or end of line.
3. The remaining text after the last parsed value is the `message`.

### Classification Rules

| Condition | Classification |
|-----------|----------------|
| `service=llm` + any of `AI_RetryError`, `statusCode:429`, `maxRetriesExceeded` | `RateLimit` |
| `service=llm` + `AI_APICallError` + no rate-limit signals | `ApiFailure` |
| `service=llm` or `service=provider` + `AuthenticationError` or `fetch failed` | `AuthFailure` |
| `service=config` or `service=skill-discovery` + message starts with `failed to load` | `MalformedAsset` |
| No header match on a stderr line containing `Error:` or ANSI-colored error prefix | `UncaughtError` |
| Everything else | `Unclassified` |

Rate-limit detection must **not** depend on English strings like `"Usage limit reached"` — these are upstream-provider-specific. Match on structured fields (`statusCode`, error class names) instead.

## Integration Points

### 1. New Module: `stream::logs`

```text
claudine/lib/src/stream/
├── logs/
│   ├── mod.rs          # pub mod opencode;
│   └── opencode.rs     # OpenCodeLogRecord, LogClassification, parser
├── protocol/
│   └── opencode.rs     # existing — unchanged
└── ...
```

This mirrors the existing `stream/protocol/opencode.rs` pattern. The `logs` module handles the stderr channel; `protocol` handles the stdout channel.

### 2. `SemanticEvent` Emission

A new `LogSemanticBridge` struct wraps a `SemanticEventSink` and ingests parsed log records, converting classified records into `SemanticEvent`s:

```rust
impl LogSemanticBridge {
    pub fn ingest_log_record(&mut self, record: OpenCodeLogRecord) {
        match classify(&record) {
            LogClassification::RateLimit { reset_at, .. } => {
                self.sink.on_semantic_event(SemanticEvent::Warning {
                    message: format_rate_limit_message(reset_at),
                    extra: /* structured fields */,
                });
            }
            LogClassification::MalformedAsset { asset_type, path, error } => {
                self.sink.on_semantic_event(SemanticEvent::Warning {
                    message: format!("{}: {error}", asset_type_label(asset_type)),
                    extra: /* structured fields */,
                });
            }
            LogClassification::AuthFailure { message } => {
                self.sink.on_semantic_event(SemanticEvent::Error {
                    message,
                    terminal: false,
                    extra: /* ... */,
                });
            }
            LogClassification::ApiFailure { .. } => {
                self.sink.on_semantic_event(SemanticEvent::Error {
                    message: /* ... */,
                    terminal: true,
                    extra: /* ... */,
                });
            }
            LogClassification::UncaughtError { raw_text } => {
                self.sink.on_semantic_event(SemanticEvent::Error {
                    message: raw_text,
                    terminal: true,
                    extra: /* ... */,
                });
            }
            LogClassification::Unclassified => {
                // Emit as Info so nothing is silently dropped
                self.sink.on_semantic_event(SemanticEvent::Info {
                    message: record.raw,
                    extra: /* ... */,
                });
            }
        }
    }
}
```

This ensures log-derived events flow through the same rendering pipeline as stdout-derived events.

### 3. `StreamExecutionSummary` Enrichment

Add an optional `stderr_diagnostics` field to `StreamExecutionSummary`:

```rust
pub struct StderrDiagnostics {
    pub log_records_parsed: u32,
    pub rate_limit_events: u32,
    pub malformed_asset_events: u32,
    pub uncaught_errors: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_limit_reset_at: Option<String>,
}
```

This populates the existing `stderr_text` field with the raw stderr output (already present on the summary) and adds structured counters for the log-derived diagnostics. The badge system (`badges::derive_badges`) can then produce badges from these counters even when the stdout stream did not emit an error event.

### 4. Badge Derivation Enhancement

Extend `badges::derive_badges` to also check `stderr_diagnostics`:

- If `rate_limit_events > 0`: emit a `BadgeCategory::RateLimit` badge with the `reset_at` time in the message.
- If `malformed_asset_events > 0`: emit a `BadgeCategory::Config` badge (new category) summarizing the count of skipped assets.
- If `uncaught_errors > 0`: emit a `BadgeCategory::Quota` or `BadgeCategory::Auth` badge depending on whether the uncaught error text matches known auth/billing patterns.

Add `Config` to `BadgeCategory`:

```rust
pub enum BadgeCategory {
    // ... existing variants ...
    Config,
}
```

### 5. Process Invocation Changes

When spawning OpenCode for a non-interactive session, Claudine must:

1. Add `--print-logs --log-level ERROR` to the OpenCode command args (when the provider is OpenCode).
2. Pipe stderr (`Stdio::piped()`) instead of discarding it.
3. Spawn a concurrent task that reads stderr line-by-line, parses each line through the log parser, and feeds classified records to the `LogSemanticBridge`.

This concurrent stderr reader runs alongside the existing stdout line reader. Both terminate when the child process exits.

### 6. Non-Interactive Exit on Rate Limit

When a `RateLimit` classification is emitted **before** any stdout `SemanticEvent` has been received, Claudine's non-interactive wrapper should:

1. Emit a `SemanticEvent::Error` with `terminal: true` and a message containing the reset time.
2. Set `StreamExecutionSummary.is_error = true` and `error_kind = "usage_limit_reached"`.
3. Terminate the wrapper with a non-zero exit code.

This replaces the current behavior of hanging silently until the usage limit resets (potentially hours later).

## CLI Changes

### Agent Definition Update

Update `claudine/lib/src/agents/opencode.rs` to reflect the new logging capabilities:

```rust
logging: LoggingCapabilities {
    session_locations: vec![],
    log_locations: vec![],
    debug_controls: vec!["--print-logs", "--log-level ERROR"],
    telemetry_controls: vec![],
},
```

## Test Plan

### Unit Tests

| Test | Description |
|------|-------------|
| Header regex acceptance | Valid lines with all four levels parse correctly; invalid lines return `None`. |
| Header regex rejects non-log lines | Stack traces, ANSI blocks, and empty lines return `None`. |
| Body walk: bare values | `service=llm providerID=z.ai` extracts two tags. |
| Body walk: JSON value | `service=llm error={"name":"AI_RetryError","statusCode":429}` extracts the JSON object as a raw string. |
| Body walk: error terminal key | `error=something with spaces and: colons` consumes the entire remainder. |
| Body walk: message extraction | Trailing text after the last tag value is the message. |
| Classification: rate limit | `service=llm` + `AI_RetryError` + `statusCode:429` → `RateLimit`. |
| Classification: rate limit from code 1308 | `service=llm` + `"code":"1308"` → `RateLimit`. |
| Classification: API failure | `service=llm` + `AI_APICallError` + no rate-limit signals → `ApiFailure`. |
| Classification: malformed skill | `service=skill-discovery` + `failed to load skill` → `MalformedAsset(AssetType::Skill)`. |
| Classification: malformed command | `service=config` + `failed to load command` → `MalformedAsset(AssetType::Command)`. |
| Classification: auth failure | `service=llm` + `AuthenticationError` → `AuthFailure`. |
| Classification: uncaught error | Non-header line with `Error:` prefix → `UncaughtError`. |
| Classification: unclassified | Unknown service/message → `Unclassified`. |
| Round-trip: full record to SemanticEvent | A classified `RateLimit` record produces a `SemanticEvent::Warning` with correct `extra` fields. |
| Round-trip: badge derivation | Summary with `stderr_diagnostics.rate_limit_events > 0` yields a `RateLimit` badge. |
| Resilience: unknown tags | Log line with unknown `service=newthing` parses without error; tags preserved. |
| Resilience: missing tags | Log line with only `LEVEL +Nms message` parses; tags map is empty. |

### Integration Tests

| Test | Description |
|------|-------------|
| Concurrent stderr parsing | Feed a stdout NDJSON fixture and a stderr log fixture concurrently; verify both channels produce `SemanticEvent`s. |
| Rate limit before first event | Stderr emits rate-limit log before any stdout event; verify `SemanticEvent::Error(terminal: true)` is emitted. |
| Malformed asset warning | Stderr emits `failed to load skill` warning; verify `SemanticEvent::Warning` with `MalformedAsset` extra. |
| Summary enrichment | After a session with both stdout and stderr, verify `StreamExecutionSummary.stderr_diagnostics` is populated with correct counters. |

### Fixture Files

Create test fixtures under `claudine/lib/tests/fixtures/logs/`:

- `opencode-rate-limit.txt` — Real stderr output capturing a z.ai 1308 usage-limit error.
- `opencode-malformed-skill.txt` — Stderr output with `failed to load skill` lines.
- `opencode-mixed.txt` — A mix of valid log lines, stack traces, and empty lines.

## File Changes Summary

| File | Action | Description |
|------|--------|-------------|
| `claudine/lib/src/stream/logs/mod.rs` | **Create** | Module root with `pub mod opencode;` |
| `claudine/lib/src/stream/logs/opencode.rs` | **Create** | `OpenCodeLogRecord`, `LogClassification`, `AssetType`, `LogLevel`, parser, classifier |
| `claudine/lib/src/stream/mod.rs` | **Edit** | Add `pub mod logs;` |
| `claudine/lib/src/stream/summary.rs` | **Edit** | Add `StderrDiagnostics` struct and `stderr_diagnostics` field to `StreamExecutionSummary` |
| `claudine/lib/src/stream/badges.rs` | **Edit** | Add `Config` to `BadgeCategory`; extend `derive_badges` to check `stderr_diagnostics` |
| `claudine/lib/src/agents/opencode.rs` | **Edit** | Update `LoggingCapabilities.debug_controls` |
| Process invocation site | **Edit** | Add `--print-logs --log-level ERROR` args; pipe stderr; spawn concurrent log parser task |

## Implementation Order

1. **`stream::logs::opencode`** — Parser, record type, classifier (pure functions, fully unit-testable).
2. **`StderrDiagnostics` on `StreamExecutionSummary`** — Add the struct and field.
3. **`LogSemanticBridge`** — Wire classifier output to `SemanticEvent` emission.
4. **Badge derivation** — Extend `derive_badges` for `stderr_diagnostics`.
5. **Process invocation** — Add CLI flags and concurrent stderr reader.
6. **Non-interactive exit path** — Early termination on pre-stream rate-limit events.
7. **Integration tests and fixtures.**

## Open Questions

- Should `--print-logs --log-level ERROR` be enabled by default for all OpenCode sessions, or only when a `--verbose` / `--debug` flag is set on the Claudine wrapper? **Recommendation: enable by default** — the ERROR-level stream is low-volume and the rate-limit signal is critical.
- Should the `reset_at` string be parsed into a `chrono::DateTime<Utc>` for structured consumers, or left as a string? **Recommendation: parse to `DateTime<Utc>`** — structured consumers (JSONL reporting, programmatic API) benefit from a typed timestamp; renderers can format it for display.
- Should the log parser be enabled for providers other than OpenCode? **Recommendation: OpenCode-only initially** — the log format is OpenCode-specific. Other providers emit different stderr patterns. A generic stderr-capture facility is a separate feature.
