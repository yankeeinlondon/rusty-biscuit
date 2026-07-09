//! `&'static` row types for the generated signal-detection tables.
//!
//! `claudine-gen` compiles the declarative detection records in
//! `docs/research/signals/` into `lib/src/signals/generated.rs`, whose
//! tables are built from these rows. Only runtime-relevant matching data is
//! carried here — evidence paths, `distinguish` prose, and other
//! documentation fields stay in the research docs, which `signals check`
//! reads directly.
//!
//! ## Notes
//!
//! - `match_path` and [`ExtractionSpec::path`] use the restricted JSONPath
//!   subset: dot segments plus numeric bracket indices only (no wildcards,
//!   filters, or recursive descent).
//! - `value`/`values` hold match terms as strings; regex patterns are
//!   validated at generate time, not at match time.
//! - Rows with `mode:` [`DetectionMode::Bespoke`] have a hand-written emitter
//!   in the behavior half rather than the generic matcher.

use crate::signal::{DetectionMode, MatchOp, SignalKind, SignalSource};
use crate::vocab::{Unit, Zone};

/// All detection records for one provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProviderSignalTable {
    pub slug: &'static str,
    pub records: &'static [DetectionRecord],
}

/// One compiled detection record — the runtime half of a
/// `records[]` row in a signals research doc.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DetectionRecord {
    pub id: &'static str,
    pub kind: SignalKind,
    pub source: SignalSource,
    pub mode: DetectionMode,
    /// Orders first-match-wins evaluation within one provider×source group.
    pub priority: u16,
    pub match_path: Option<&'static str>,
    pub op: Option<MatchOp>,
    pub value: Option<&'static str>,
    pub values: &'static [&'static str],
    pub since: Option<&'static str>,
    pub until: Option<&'static str>,
    pub extractions: &'static [ExtractionSpec],
}

/// How a detection record reads one field's raw value from the payload.
///
/// The source is provider-neutral at the type level: each provider's research
/// declares the strategy that fits its wire format. `Path` is the common case;
/// `Literal` pins a research-declared constant (e.g. a decomposed cap axis when
/// the provider bundles several into one token); `Regex` / `StartStopTokens`
/// carve a value out of a string field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtractStrategy {
    /// Dot/bracket path into the JSON payload.
    Path(&'static str),
    /// First capture group of `pattern` applied to the string at `path`.
    Regex {
        path: &'static str,
        pattern: &'static str,
    },
    /// Substring of the string at `path` between `start` and `stop` (exclusive).
    StartStopTokens {
        path: &'static str,
        start: &'static str,
        stop: &'static str,
    },
    /// A constant value declared by research, independent of the payload.
    Literal(&'static str),
}

/// One payload extraction site for a detection record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExtractionSpec {
    /// Normalized field name in the signal's typed payload.
    pub field: &'static str,
    /// How the raw value is read from the payload.
    pub source: ExtractStrategy,
    pub unit: Option<Unit>,
    pub zone: Option<Zone>,
}
