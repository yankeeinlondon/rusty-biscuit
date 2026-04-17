//! Structured log parsing for provider stderr streams.
//!
//! OpenCode is the first (and currently only) provider that emits
//! structured log records on stderr when launched with `--print-logs
//! --log-level ERROR`. This module holds pure parsing and classification
//! helpers so the wrapper layer can consume stderr log lines without
//! pulling provider-specific logic out of the CLI.

pub mod opencode;
