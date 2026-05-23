//! Error type returned by `darkmatter::style` parsers.

use thiserror::Error;

use super::warning::StyleWarning;

/// Errors that can be returned by `darkmatter::style` parsers.
///
/// ## Notes
///
/// `Strict` is produced by `into_strict` (added in a later sub-spec phase) when
/// an otherwise successful parse carries `UnknownKey` or `Deprecated` warnings.
#[derive(Debug, Error)]
pub enum StyleParseError {
    /// A YAML node had the wrong shape at the given path.
    #[error("Invalid YAML structure at `{path}`: expected {expected}, got {actual}")]
    Structure {
        path: String,
        expected: &'static str,
        actual: String,
    },

    /// A length value could not be parsed.
    #[error("Invalid length `{raw}` at `{path}`: {reason}")]
    InvalidLength {
        path: String,
        raw: String,
        reason: &'static str,
    },

    /// A percent value was out of `0.0..=100.0`.
    #[error("Invalid percent `{value}` at `{path}`: must be in 0.0..=100.0")]
    InvalidPercent { path: String, value: f32 },

    /// A color value could not be parsed.
    #[error("Invalid color `{raw}` at `{path}`: {reason}")]
    InvalidColor {
        path: String,
        raw: String,
        reason: &'static str,
    },

    /// Strict mode: schema-validation warnings (`UnknownKey` or `Deprecated`)
    /// were promoted to an error.
    #[error("Strict mode: {} schema warning(s)", warnings.len())]
    Strict { warnings: Vec<StyleWarning> },

    /// Pass-through for serde failures.
    #[error("serde error: {0}")]
    Serde(#[from] serde_json::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn structure_message_contains_path_and_types() {
        let err = StyleParseError::Structure {
            path: "style.page.left-margin".to_string(),
            expected: "string",
            actual: "number".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("style.page.left-margin"));
        assert!(msg.contains("string"));
        assert!(msg.contains("number"));
    }

    #[test]
    fn invalid_length_message_contains_raw_and_reason() {
        let err = StyleParseError::InvalidLength {
            path: "style.page.left-margin".to_string(),
            raw: "2px".to_string(),
            reason: "unsupported unit `px`; allowed: ch, %",
        };
        let msg = err.to_string();
        assert!(msg.contains("2px"));
        assert!(msg.contains("unsupported unit"));
    }
}
