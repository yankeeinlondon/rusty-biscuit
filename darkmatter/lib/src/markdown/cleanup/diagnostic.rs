//! The clean diagnostic model shared by `md clean` and DMLS formatting.
//!
//! Frontmatter findings reuse biscuit-file's YAML vocabulary
//! ([`YamlDiagnosticCode`], [`YamlCertainty`], [`YamlRepair`]); findings that
//! are about Markdown cleanup itself get Darkmatter-owned codes here, so no
//! Markdown-specific code is added to biscuit-file's YAML enum.

use biscuit_file::{SourceSpan, YamlCertainty, YamlDiagnostic, YamlDiagnosticCode, YamlRepair};
use serde::{Serialize, Serializer};

use super::body::Fallback;

/// Which stage of a clean run produced a diagnostic.
///
/// This identifies the diagnostic's owner, not its coordinate space: every
/// [`CleanDiagnostic`] span is in authored document coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CleanStage {
    /// Schema-free YAML analysis from `biscuit-file`.
    Syntax,
    /// Schema-aware analysis from Darkmatter's `SimplifiedSchema` engine.
    Schema,
    /// Validation of the document's `style.markdown` formatting rules.
    Rules,
    /// The final fidelity check of the cleaned body.
    Fidelity,
}

/// A clean diagnostic's stable code.
///
/// Serializes as its dotted string: the `yaml.*`/`schema.*` spellings come
/// from biscuit-file unchanged, so the version-1 JSON contract keeps them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CleanCode {
    /// A frontmatter finding from biscuit-file or the schema tier.
    Yaml(YamlDiagnosticCode),
    /// The cleaned body was not equivalent to the input, so the original was
    /// kept.
    FidelityFallback,
    /// The document's `style.markdown` object is unusable and was ignored.
    InvalidRules,
}

impl CleanCode {
    /// The stable dotted spelling.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            CleanCode::Yaml(code) => code.as_str(),
            CleanCode::FidelityFallback => "clean.fidelity-fallback",
            CleanCode::InvalidRules => "clean.invalid-rules",
        }
    }
}

impl Serialize for CleanCode {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

/// Why cleanup declined and returned the original document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum FallbackReason {
    /// A malformed or unpaired directive region was left untouched.
    MalformedRegion,
    /// The candidate could not be produced.
    SerializationFailure,
    /// The candidate's structure or meaning differed from the input.
    SemanticMismatch,
}

/// One finding from a clean run, in authored document coordinates.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CleanDiagnostic {
    /// The stage that owns this finding.
    pub stage: CleanStage,
    /// The stable code.
    pub code: CleanCode,
    /// Byte span in the document exactly as the caller supplied it.
    #[serde(with = "biscuit_file::span_serde")]
    pub span: SourceSpan,
    /// Certainty class; only [`YamlCertainty::Deterministic`] findings are
    /// ever auto-applied.
    pub classification: YamlCertainty,
    /// Human-readable explanation.
    pub message: String,
    /// Candidate repairs, in the same document coordinates as `span`.
    pub repairs: Vec<YamlRepair>,
    /// Set only for [`CleanCode::FidelityFallback`]: why cleanup declined
    /// and the source offset of the first affected input block, so a caller
    /// can restate the location in its own coordinates (an LSP client's
    /// position encoding, for example).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fallback: Option<Fallback>,
}

impl CleanDiagnostic {
    /// Wraps a YAML finding whose spans are relative to the frontmatter YAML,
    /// shifting them by `yaml_offset` into document coordinates.
    #[must_use]
    pub fn from_yaml(stage: CleanStage, diagnostic: YamlDiagnostic, yaml_offset: usize) -> Self {
        Self {
            stage,
            code: CleanCode::Yaml(diagnostic.code),
            span: shift(&diagnostic.span, yaml_offset),
            classification: diagnostic.classification,
            message: diagnostic.message,
            repairs: diagnostic
                .repairs
                .into_iter()
                .map(|repair| shift_repair(repair, yaml_offset))
                .collect(),
            fallback: None,
        }
    }

    /// A report-only `clean.invalid-rules` finding at `span`.
    #[must_use]
    pub fn invalid_rules(span: SourceSpan, message: impl Into<String>) -> Self {
        Self {
            stage: CleanStage::Rules,
            code: CleanCode::InvalidRules,
            span,
            classification: YamlCertainty::DeterministicFindNonDeterministicSolution,
            message: message.into(),
            repairs: Vec::new(),
            fallback: None,
        }
    }
}

impl CleanDiagnostic {
    /// A report-only `clean.fidelity-fallback` finding: cleanup kept the
    /// original, so nothing is repaired.
    #[must_use]
    pub fn fidelity_fallback(span: SourceSpan, message: impl Into<String>, fallback: Fallback) -> Self {
        Self {
            stage: CleanStage::Fidelity,
            code: CleanCode::FidelityFallback,
            span,
            classification: YamlCertainty::NonDeterministicFind,
            message: message.into(),
            repairs: Vec::new(),
            fallback: Some(fallback),
        }
    }
}

pub(crate) fn shift(span: &SourceSpan, offset: usize) -> SourceSpan {
    span.start + offset..span.end + offset
}

pub(crate) fn shift_repair(mut repair: YamlRepair, offset: usize) -> YamlRepair {
    repair.span = shift(&repair.span, offset);
    repair
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_keep_their_dotted_spellings() {
        assert_eq!(
            serde_json::to_value(CleanCode::Yaml(YamlDiagnosticCode::DuplicateKey)).unwrap(),
            serde_json::json!("yaml.duplicate-key")
        );
        assert_eq!(CleanCode::FidelityFallback.as_str(), "clean.fidelity-fallback");
        assert_eq!(CleanCode::InvalidRules.as_str(), "clean.invalid-rules");
    }

    #[test]
    fn yaml_findings_shift_into_document_coordinates() {
        let diagnostic = YamlDiagnostic {
            code: YamlDiagnosticCode::TrailingWhitespace,
            classification: YamlCertainty::Deterministic,
            message: "m".into(),
            span: 2..4,
            repairs: vec![YamlRepair {
                span: 2..4,
                replacement: String::new(),
                explanation: "e".into(),
            }],
        };
        let shifted = CleanDiagnostic::from_yaml(CleanStage::Syntax, diagnostic, 4);
        assert_eq!(shifted.span, 6..8);
        assert_eq!(shifted.repairs[0].span, 6..8);
        assert_eq!(shifted.fallback, None);
    }
}
