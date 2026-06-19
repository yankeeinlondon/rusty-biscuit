use std::fmt;

use serde::{Deserialize, Serialize};

/// Categories for lint and semantic diagnostics.
///
/// Categories are stable identifiers used for policy configuration,
/// grouping related rules, and CI policy decisions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DiagnosticCategory {
    /// Syntax errors and parse failures.
    Correctness,
    /// Patterns that are likely bugs or problematic.
    Suspicious,
    /// Code style and convention issues.
    Style,
    /// Performance-related concerns.
    Performance,
    /// Extra-strict checks, often noisy.
    Pedantic,
    /// Checks that limit what APIs can be used.
    Restriction,
    /// Experimental or low-confidence checks.
    Experimental,
}

impl fmt::Display for DiagnosticCategory {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Self::Correctness => "correctness",
            Self::Suspicious => "suspicious",
            Self::Style => "style",
            Self::Performance => "performance",
            Self::Pedantic => "pedantic",
            Self::Restriction => "restriction",
            Self::Experimental => "experimental",
        };
        formatter.write_str(label)
    }
}

impl std::str::FromStr for DiagnosticCategory {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "correctness" => Ok(Self::Correctness),
            "suspicious" => Ok(Self::Suspicious),
            "style" => Ok(Self::Style),
            "performance" => Ok(Self::Performance),
            "pedantic" => Ok(Self::Pedantic),
            "restriction" => Ok(Self::Restriction),
            "experimental" => Ok(Self::Experimental),
            _ => Err(format!("unknown diagnostic category: {s}")),
        }
    }
}

/// Confidence levels for diagnostics.
///
/// Indicates how reliable the analyzer believes a finding is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DiagnosticConfidence {
    /// High confidence: very low false-positive rate.
    High,
    /// Medium confidence: may have occasional false positives.
    Medium,
    /// Low confidence: likely to have false positives without project context.
    Low,
    /// Experimental: not yet proven, may have systematic false positives.
    Experimental,
}

impl fmt::Display for DiagnosticConfidence {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Self::High => "high",
            Self::Medium => "medium",
            Self::Low => "low",
            Self::Experimental => "experimental",
        };
        formatter.write_str(label)
    }
}

impl std::str::FromStr for DiagnosticConfidence {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "high" => Ok(Self::High),
            "medium" => Ok(Self::Medium),
            "low" => Ok(Self::Low),
            "experimental" => Ok(Self::Experimental),
            _ => Err(format!("unknown diagnostic confidence: {s}")),
        }
    }
}

/// Source of a diagnostic.
///
/// Identifies which analyzer subsystem produced the finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DiagnosticSource {
    /// Pattern-based query from a `.scm` file.
    TreeSitterQuery,
    /// Semantic analysis (undefined symbols, unused symbols, etc.).
    SemanticAnalysis,
    /// Parse error from the tree-sitter parser.
    SyntaxParser,
    /// External tool adapter (e.g., Oxlint, Ruff).
    ExternalTool,
}

impl fmt::Display for DiagnosticSource {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Self::TreeSitterQuery => "tree-sitter-query",
            Self::SemanticAnalysis => "semantic-analysis",
            Self::SyntaxParser => "syntax-parser",
            Self::ExternalTool => "external-tool",
        };
        formatter.write_str(label)
    }
}

/// Metadata attached to every diagnostic for policy and display.
///
/// This struct provides the contract that allows consumers (humans, JSON
/// consumers, CI policy, and future adapters) to understand rule identity,
/// category, confidence, source, and effective severity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticMetadata {
    /// The category this diagnostic belongs to.
    pub category: DiagnosticCategory,
    /// Confidence level in this finding.
    pub confidence: DiagnosticConfidence,
    /// Which subsystem produced this diagnostic.
    pub source: DiagnosticSource,
    /// The default severity defined by the rule.
    pub default_severity: crate::shared::DiagnosticSeverity,
    /// The effective severity after policy application.
    pub effective_severity: crate::shared::DiagnosticSeverity,
    /// Whether this rule is enabled by default.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub is_enabled_by_default: bool,
    /// Whether this rule requires experimental semantics to be enabled.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub requires_experimental_semantics: bool,
}

impl Default for DiagnosticMetadata {
    fn default() -> Self {
        use crate::shared::DiagnosticSeverity;
        Self {
            category: DiagnosticCategory::Suspicious,
            confidence: DiagnosticConfidence::Medium,
            source: DiagnosticSource::TreeSitterQuery,
            default_severity: DiagnosticSeverity::Warning,
            effective_severity: DiagnosticSeverity::Warning,
            is_enabled_by_default: true,
            requires_experimental_semantics: false,
        }
    }
}
