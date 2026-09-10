//! Pure parsing and classification for OpenCode stderr logs.
//!
//! OpenCode emits structured log records on stderr when launched with
//! `--print-logs --log-level ERROR`. Each record has a fixed header
//! (`LEVEL TIMESTAMP +DELTAms ...`) followed by a free-form body of
//! `key=value` tags and an optional trailing message. Inline JSON is
//! permitted as a value, and the special keys `error=` and `err=` are
//! terminal-to-end-of-line.
//!
//! The parser is deliberately small and resilient: unknown tags are
//! preserved in an open-ended [`BTreeMap`](std::collections::BTreeMap),
//! missing tags are accepted,
//! and any line that does not match the header falls through to
//! [`ParsedOpenCodeStderrLine::RawText`]. Classification is a pure
//! function over the parsed record plus a small raw-text fallback for
//! fatal exceptions.

pub mod bridge;
pub mod classify;
pub mod events;
pub mod state;

// Re-export the original public API surface so existing callers don't break.
pub use classify::{classify, classify_raw, merge_rate_limit};
pub use events::{
    AssetType, LogClassification, LogLevel, OpenCodeLogRecord, ParsedOpenCodeStderrLine, parse_line,
};
pub use bridge::{
    EarlyTermination, OpenCodeLogBridge, StalledGenerationContext, StalledGenerationProgress,
    StderrIngestOutcome, StuckSubagentInfo,
};
pub use state::{SharedStderrState, merge_stderr_state_into_summary};
