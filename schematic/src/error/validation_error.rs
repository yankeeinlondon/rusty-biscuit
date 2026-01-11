//! Response validation and parsing errors.

use thiserror::Error;

/// Errors during response parsing and validation.
///
/// These errors occur when the API response cannot be parsed or fails
/// schema validation. Each variant corresponds to a specific format.
#[derive(Debug, Error)]
pub enum ValidationError {
    /// JSON parsing failed.
    #[error("JSON parse error: {0}")]
    JsonParse(#[from] serde_json::Error),

    /// YAML parsing failed.
    #[error("YAML parse error: {0}")]
    YamlParse(#[from] serde_yaml::Error),

    /// XML parsing failed.
    #[error("XML parse error: {0}")]
    XmlParse(#[from] quick_xml::DeError),

    /// XSD schema validation failed.
    #[error("XSD validation failed: {0}")]
    XsdValidation(String),

    /// Response content type doesn't match expected format.
    #[error("Unexpected content type: expected {expected}, got {actual}")]
    ContentTypeMismatch {
        /// The expected content type.
        expected: String,
        /// The actual content type received.
        actual: String,
    },

    /// Empty response body when content was expected.
    #[error("Empty response body")]
    EmptyBody,
}

impl ValidationError {
    /// Returns `true` if this is a format mismatch error.
    pub fn is_format_mismatch(&self) -> bool {
        matches!(self, Self::ContentTypeMismatch { .. })
    }

    /// Returns `true` if this is a parsing error.
    pub fn is_parse_error(&self) -> bool {
        matches!(
            self,
            Self::JsonParse(_) | Self::YamlParse(_) | Self::XmlParse(_)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_content_type_mismatch() {
        let err = ValidationError::ContentTypeMismatch {
            expected: "application/json".to_string(),
            actual: "text/html".to_string(),
        };
        assert!(err.is_format_mismatch());
        assert!(!err.is_parse_error());
    }

    #[test]
    fn test_json_parse_is_parse_error() {
        let json_err = serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err();
        let err = ValidationError::JsonParse(json_err);
        assert!(err.is_parse_error());
        assert!(!err.is_format_mismatch());
    }

    #[test]
    fn test_xsd_validation_display() {
        let err = ValidationError::XsdValidation("element 'foo' not expected".to_string());
        let display = err.to_string();
        assert!(display.contains("XSD validation failed"));
        assert!(display.contains("element 'foo' not expected"));
    }
}
