//! Error type returned by `darkmatter::style` parsers.

use thiserror::Error;

use super::warning::{StyleSpan, StyleWarning};

/// Errors that can be returned by `darkmatter::style` parsers.
///
/// ## Notes
///
/// `Strict` is produced by [`into_strict`](super::into_strict) when an
/// otherwise successful parse carries `UnknownKey` or `Deprecated` warnings.
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

impl StyleParseError {
    /// The source span this error is anchored at, when known (R-5 Priority 5).
    ///
    /// Populated only for [`StyleParseError::Strict`], whose first schema
    /// warning carries a span when the parse ran through
    /// [`from_frontmatter`](super::parse::from_frontmatter) with the raw
    /// frontmatter text available. The value-level typed variants
    /// (`Structure`, `InvalidLength`, `InvalidPercent`, `InvalidColor`) carry a
    /// dotted `path` but no span in v1 and return `None`.
    pub fn source_span(&self) -> Option<&StyleSpan> {
        match self {
            StyleParseError::Strict { warnings } => {
                warnings.iter().find_map(|w| w.source_span.as_ref())
            }
            _ => None,
        }
    }
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

    #[test]
    fn source_span_returns_strict_warning_span() {
        use super::super::warning::StyleWarningKind;
        let mut warning = StyleWarning::new("style.page.lft-margin", StyleWarningKind::UnknownKey);
        warning.source_span = Some(StyleSpan {
            line: 3,
            column: 5,
            length: 11,
        });
        let err = StyleParseError::Strict {
            warnings: vec![warning],
        };
        assert_eq!(
            err.source_span(),
            Some(&StyleSpan {
                line: 3,
                column: 5,
                length: 11
            })
        );
    }

    #[test]
    fn source_span_is_none_for_typed_variant() {
        let err = StyleParseError::InvalidPercent {
            path: "style.page.left-margin".to_string(),
            value: 200.0,
        };
        assert_eq!(err.source_span(), None);
    }
}
