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
//! - Rows with `mode:` [`DetectionMode::Bespoke`] are catalog documentation:
//!   their emission comes from bespoke code in the behavior half, not the
//!   generic matcher.

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

/// One payload extraction site for a detection record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExtractionSpec {
    /// Normalized field name in the signal's typed payload.
    pub field: &'static str,
    pub path: &'static str,
    pub unit: Option<Unit>,
    pub zone: Option<Zone>,
}
