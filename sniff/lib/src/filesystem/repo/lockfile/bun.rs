//! Bun text `bun.lock` parser.
//!
//! Phase 2 stub: every document is reported as an unsupported version, so a
//! layer never reads as `match` or `mismatch` before the real parser lands.

use super::{Outcome, ParsedLockfile};

pub(super) fn parse(_content: &str) -> Outcome {
    Ok(ParsedLockfile::UnsupportedVersion)
}
