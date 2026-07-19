//! Shared YAML diagnostic and repair vocabulary.
//!
//! These types are the one uniform diagnostic shape produced by both the
//! schema-agnostic analyzer in this crate and schema-aware layers in
//! downstream crates (e.g. Darkmatter). Field names, enum spellings, and
//! diagnostic codes are the v1-stable JSON contract: additive changes are
//! permitted, existing spellings never change meaning.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::span::SourceSpan;

/// Certainty classification of a YAML diagnostic.
///
/// Mirrors the research certainty tiers; only
/// [`YamlCertainty::Deterministic`] findings are eligible for automatic
/// repair. Both report-only tiers must never reach edit application.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum YamlCertainty {
    /// The error is identified with certainty and the fix is provable.
    Deterministic,
    /// The problem is certain, but the fix is a guess. Report only.
    DeterministicFindNonDeterministicSolution,
    /// A suspected smell that cannot be proven. Suggest only.
    NonDeterministicFind,
}

impl YamlCertainty {
    /// Returns `true` when findings of this class may be auto-applied.
    ///
    /// This is the single auto-apply filter keyed on classification: every
    /// repair-application path gates on it, so report-only classifications
    /// can never reach edit application even when a diagnostic carries
    /// candidate repairs.
    #[must_use]
    pub fn is_auto_apply_eligible(self) -> bool {
        matches!(self, Self::Deterministic)
    }
}

/// Stable diagnostic code for a YAML finding.
///
/// Codes serialize as dotted lowercase strings. The `yaml.*` family is
/// emitted by this crate's schema-agnostic analyzer; the `schema.*` family is
/// emitted by schema-aware layers in downstream crates onto the same
/// [`YamlDiagnostic`] shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum YamlDiagnosticCode {
    /// YAML syntax parse failure.
    #[serde(rename = "yaml.parse")]
    Parse,
    /// A UTF-8 byte-order mark is present at stream start.
    #[serde(rename = "yaml.bom")]
    Bom,
    /// CRLF or lone CR line endings are present.
    #[serde(rename = "yaml.line-ending")]
    LineEnding,
    /// Trailing whitespace outside scalar content.
    #[serde(rename = "yaml.trailing-whitespace")]
    TrailingWhitespace,
    /// Missing or superfluous final newline.
    #[serde(rename = "yaml.final-newline")]
    FinalNewline,
    /// Parse-equivalent whitespace around flow delimiters, mapping colons,
    /// or sequence markers.
    #[serde(rename = "yaml.whitespace")]
    Whitespace,
    /// A plain scalar begins with a reserved YAML indicator character.
    #[serde(rename = "yaml.reserved-indicator")]
    ReservedIndicator,
    /// A mapping key is defined more than once.
    #[serde(rename = "yaml.duplicate-key")]
    DuplicateKey,
    /// An alias references an anchor that is never declared.
    #[serde(rename = "yaml.anchor-undeclared")]
    AnchorUndeclared,
    /// An alias references an anchor declared later in the document.
    #[serde(rename = "yaml.anchor-forward")]
    AnchorForward,
    /// An anchor name is declared more than once.
    #[serde(rename = "yaml.anchor-duplicate")]
    AnchorDuplicate,
    /// A declared anchor is never referenced by an alias.
    #[serde(rename = "yaml.anchor-unused")]
    AnchorUnused,
    /// An alias closely matches but does not equal a declared anchor.
    #[serde(rename = "yaml.anchor-misspelled")]
    AnchorMisspelled,
    /// The stream contains more than one YAML document.
    #[serde(rename = "yaml.multi-document")]
    MultiDocument,
    /// A plain scalar that parses to a surprising type or value.
    #[serde(rename = "yaml.ambiguous-scalar")]
    AmbiguousScalar,
    /// A key with an empty value where content was likely intended.
    #[serde(rename = "yaml.suspicious-empty-value")]
    SuspiciousEmptyValue,
    /// A block scalar used where it interacts badly with content.
    #[serde(rename = "yaml.block-scalar-smell")]
    BlockScalarSmell,
    /// Content that looks truncated by an unintended comment.
    #[serde(rename = "yaml.comment-truncation")]
    CommentTruncation,
    /// Inconsistent scalar style or indentation across the document.
    #[serde(rename = "yaml.style-inconsistency")]
    StyleInconsistency,
    /// A key closely resembling another key or a likely misplaced key.
    #[serde(rename = "yaml.similar-key")]
    SimilarKey,
    /// A schema-aware layer found a value of the wrong type; emitted by
    /// downstream schema-aware crates, never by this crate's analyzer.
    #[serde(rename = "schema.type-mismatch")]
    SchemaTypeMismatch,
    /// A schema-aware layer suggests a key correction; report-only.
    #[serde(rename = "schema.key-correction")]
    SchemaKeyCorrection,
    /// A schema-aware layer suggests a shape or type repair; report-only.
    #[serde(rename = "schema.shape-repair")]
    SchemaShapeRepair,
}

impl YamlDiagnosticCode {
    /// All diagnostic codes, in declaration order.
    pub const ALL: [Self; 23] = [
        Self::Parse,
        Self::Bom,
        Self::LineEnding,
        Self::TrailingWhitespace,
        Self::FinalNewline,
        Self::Whitespace,
        Self::ReservedIndicator,
        Self::DuplicateKey,
        Self::AnchorUndeclared,
        Self::AnchorForward,
        Self::AnchorDuplicate,
        Self::AnchorUnused,
        Self::AnchorMisspelled,
        Self::MultiDocument,
        Self::AmbiguousScalar,
        Self::SuspiciousEmptyValue,
        Self::BlockScalarSmell,
        Self::CommentTruncation,
        Self::StyleInconsistency,
        Self::SimilarKey,
        Self::SchemaTypeMismatch,
        Self::SchemaKeyCorrection,
        Self::SchemaShapeRepair,
    ];

    /// Returns the stable dotted string spelling of this code.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Parse => "yaml.parse",
            Self::Bom => "yaml.bom",
            Self::LineEnding => "yaml.line-ending",
            Self::TrailingWhitespace => "yaml.trailing-whitespace",
            Self::FinalNewline => "yaml.final-newline",
            Self::Whitespace => "yaml.whitespace",
            Self::ReservedIndicator => "yaml.reserved-indicator",
            Self::DuplicateKey => "yaml.duplicate-key",
            Self::AnchorUndeclared => "yaml.anchor-undeclared",
            Self::AnchorForward => "yaml.anchor-forward",
            Self::AnchorDuplicate => "yaml.anchor-duplicate",
            Self::AnchorUnused => "yaml.anchor-unused",
            Self::AnchorMisspelled => "yaml.anchor-misspelled",
            Self::MultiDocument => "yaml.multi-document",
            Self::AmbiguousScalar => "yaml.ambiguous-scalar",
            Self::SuspiciousEmptyValue => "yaml.suspicious-empty-value",
            Self::BlockScalarSmell => "yaml.block-scalar-smell",
            Self::CommentTruncation => "yaml.comment-truncation",
            Self::StyleInconsistency => "yaml.style-inconsistency",
            Self::SimilarKey => "yaml.similar-key",
            Self::SchemaTypeMismatch => "schema.type-mismatch",
            Self::SchemaKeyCorrection => "schema.key-correction",
            Self::SchemaShapeRepair => "schema.shape-repair",
        }
    }
}

impl fmt::Display for YamlDiagnosticCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for YamlDiagnosticCode {
    type Err = String;

    fn from_str(spelling: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .iter()
            .find(|code| code.as_str() == spelling)
            .copied()
            .ok_or_else(|| format!("unknown YAML diagnostic code: {spelling}"))
    }
}

/// A candidate source repair: replace `span` with `replacement`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct YamlRepair {
    /// Byte span of the source text the repair patches.
    #[serde(with = "crate::span::span_serde")]
    pub span: SourceSpan,
    /// Replacement text for the span.
    pub replacement: String,
    /// Why the repair is offered.
    pub explanation: String,
}

/// A single YAML diagnostic with zero or more candidate repairs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct YamlDiagnostic {
    /// Stable diagnostic code.
    pub code: YamlDiagnosticCode,
    /// Byte span of the offending source region.
    #[serde(with = "crate::span::span_serde")]
    pub span: SourceSpan,
    /// Certainty classification; gates auto-apply eligibility.
    pub classification: YamlCertainty,
    /// Human-readable explanation of the finding.
    pub message: String,
    /// Candidate repairs, empty for report-only findings without candidates.
    #[serde(default)]
    pub repairs: Vec<YamlRepair>,
}
