//! Top-level YAML analysis result.
//!
//! [`YamlAnalysis`] is the product of the source-first analyzer: it retains
//! the authored source, the structured parse outcome (so clean input is
//! parsed exactly once), and every diagnostic in stable source order. It is
//! produced for both parseable and unparseable input — construction never
//! fails on invalid YAML.

use serde_yaml_ng::Value;

use super::diagnostic::YamlDiagnostic;
use super::edit_set::{self, EditSetOutcome};
use crate::yaml::location::YamlLocation;

/// Structured outcome of parsing the analyzed source.
///
/// Retaining the parsed value (or the projected failure) lets callers and
/// candidate generators reuse the single parse the analyzer already
/// performed instead of reparsing.
#[derive(Debug, Clone)]
pub enum YamlParseOutcome {
    /// The source parsed successfully; the value is retained.
    Parsed(Value),
    /// The source failed to parse; the projected error is retained.
    Failed(YamlParseFailure),
}

/// Projected parse failure: the rendered message plus the structured
/// source location when the parser reported one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct YamlParseFailure {
    /// The parser's rendered error message.
    pub message: String,
    /// Structured byte/line/column location, when available.
    pub location: Option<YamlLocation>,
}

/// Source-first YAML analysis result.
///
/// Diagnostics are stored in stable source order. Candidate repairs are only
/// ever attached to the diagnostic that justifies them; documents without
/// diagnostics carry no candidates.
#[derive(Debug, Clone)]
pub struct YamlAnalysis {
    source: String,
    outcome: YamlParseOutcome,
    diagnostics: Vec<YamlDiagnostic>,
}

impl YamlAnalysis {
    /// Creates an analysis from its parts.
    #[must_use]
    pub fn new(
        source: impl Into<String>,
        outcome: YamlParseOutcome,
        diagnostics: Vec<YamlDiagnostic>,
    ) -> Self {
        Self {
            source: source.into(),
            outcome,
            diagnostics,
        }
    }

    /// The authored source as analyzed.
    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
    }

    /// The structured parse outcome.
    #[must_use]
    pub fn outcome(&self) -> &YamlParseOutcome {
        &self.outcome
    }

    /// Returns `true` when the source parsed successfully.
    #[must_use]
    pub fn is_parseable(&self) -> bool {
        matches!(self.outcome, YamlParseOutcome::Parsed(_))
    }

    /// The parsed value, when the source is parseable.
    #[must_use]
    pub fn parsed_value(&self) -> Option<&Value> {
        match &self.outcome {
            YamlParseOutcome::Parsed(value) => Some(value),
            YamlParseOutcome::Failed(_) => None,
        }
    }

    /// The projected parse failure, when the source is unparseable.
    #[must_use]
    pub fn parse_failure(&self) -> Option<&YamlParseFailure> {
        match &self.outcome {
            YamlParseOutcome::Parsed(_) => None,
            YamlParseOutcome::Failed(failure) => Some(failure),
        }
    }

    /// All diagnostics in stable source order.
    #[must_use]
    pub fn diagnostics(&self) -> &[YamlDiagnostic] {
        &self.diagnostics
    }

    /// Returns `true` when the analysis produced no diagnostics.
    #[must_use]
    pub fn is_clean(&self) -> bool {
        self.diagnostics.is_empty()
    }

    /// Every candidate repair attached to [`YamlAnalysis::diagnostics`], in
    /// stable source order.
    ///
    /// This is the repairs view of the same scan that produced the
    /// diagnostics — holding one `YamlAnalysis` and calling both accessors
    /// costs a single analysis pass.
    ///
    /// Candidates are only ever attached once they have satisfied their
    /// safety proof. Presence here does not imply auto-apply eligibility,
    /// which is gated per diagnostic by
    /// [`crate::yaml::YamlCertainty::is_auto_apply_eligible`]; use
    /// [`YamlAnalysis::apply`] to patch only the eligible subset.
    pub fn repairs(&self) -> impl Iterator<Item = &super::diagnostic::YamlRepair> {
        self.diagnostics
            .iter()
            .flat_map(|diagnostic| diagnostic.repairs.iter())
    }

    /// Applies every auto-apply-eligible candidate repair.
    ///
    /// Repairs are gathered from diagnostics whose classification passes
    /// [`crate::yaml::YamlCertainty::is_auto_apply_eligible`], in diagnostic
    /// order, and applied through the shared edit-set utility. Report-only
    /// diagnostics never contribute edits, even when they carry candidates.
    ///
    /// Returns the patched source plus an audit record of applied and
    /// rejected candidates. When no repair applies, the returned source is
    /// byte-identical to [`YamlAnalysis::source`].
    #[must_use]
    pub fn apply(&self) -> EditSetOutcome {
        let repairs: Vec<super::diagnostic::YamlRepair> = self
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.classification.is_auto_apply_eligible())
            .flat_map(|diagnostic| diagnostic.repairs.iter().cloned())
            .collect();
        edit_set::apply_edit_set(&self.source, &repairs)
    }
}
