use super::canonical::MappingFidelity;

/// Structured explanation of a policy query result.
#[derive(Debug, Clone)]
pub struct PolicyExplanation {
    /// Short human-readable summary of the result.
    pub summary: String,
    /// Machine-friendly reasons contributing to the result.
    pub reasons: Vec<ExplanationReason>,
}

/// A single reason contributing to a policy explanation.
#[derive(Debug, Clone)]
pub struct ExplanationReason {
    /// Identifier of the source that produced this reason.
    pub source_id: String,
    /// Provider-native reference (e.g., config key or rule name).
    pub native_reference: Option<String>,
    /// Human-readable description of this reason.
    pub message: String,
    /// Fidelity of the mapping that produced this reason.
    pub fidelity: MappingFidelity,
}

impl PolicyExplanation {
    /// Creates an explanation with no matched rules.
    pub fn no_match(summary: impl Into<String>) -> Self {
        Self {
            summary: summary.into(),
            reasons: Vec::new(),
        }
    }

    /// Creates an explanation from a summary and reasons.
    pub fn new(summary: impl Into<String>, reasons: Vec<ExplanationReason>) -> Self {
        Self {
            summary: summary.into(),
            reasons,
        }
    }
}
