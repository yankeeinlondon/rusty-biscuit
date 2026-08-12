//! Shared vocabulary enums for detection records and research facts.
//!
//! Canonical home per `design/signal-detection.md` — research sidecar
//! schemas mirror these member lists, and a generator-side test keeps the
//! copies from drifting.

use serde::Serialize;
use strum::{EnumIter, IntoStaticStr, VariantNames};

/// Unit of a measured or extracted quantity at a detection site.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, EnumIter, IntoStaticStr, VariantNames)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum Unit {
    UnixSeconds,
    UnixMillis,
    UnixNanos,
    Iso8601,
    DurationSecs,
    DurationMillis,
    Percent,
    Tokens,
    Requests,
    Usd,
}

/// Timezone semantics of a time-valued extraction site.
///
/// `Unspecified` is allowed but surfaced: it flows into `known_gaps`
/// reporting rather than silently passing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, EnumIter, IntoStaticStr, VariantNames)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum Zone {
    Utc,
    Local,
    EmbeddedOffset,
    Unspecified,
}

/// How a research fact was established.
///
/// Declared in ascending strength so the derived `Ord` ranks
/// `SourceCode > Observed > Documented > Inferred`.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Serialize,
    EnumIter,
    IntoStaticStr,
    VariantNames,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum Confidence {
    Inferred,
    Documented,
    Observed,
    SourceCode,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn confidence_ranks_source_code_highest() {
        assert!(Confidence::SourceCode > Confidence::Observed);
        assert!(Confidence::Observed > Confidence::Documented);
        assert!(Confidence::Documented > Confidence::Inferred);
    }
}
