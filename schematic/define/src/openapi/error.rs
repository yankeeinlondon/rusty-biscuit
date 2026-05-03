//! Error types for OpenAPI operations.
//!
//! This module provides the [`OpenApiError`] enum for representing errors
//! that can occur when parsing, validating, or processing OpenAPI specifications.
//!
//! ## Error Variants
//!
//! - [`Parse`](OpenApiError::Parse) - Syntax errors in YAML/JSON documents
//! - [`Reference`](OpenApiError::Reference) - Unresolved `$ref` references
//! - [`UnsupportedFeature`](OpenApiError::UnsupportedFeature) - OpenAPI features not supported by this crate
//! - [`Validation`](OpenApiError::Validation) - Semantic validation errors
//! - [`Io`](OpenApiError::Io) - File system errors
//!
//! ## Examples
//!
//! ```
//! use schematic_define::openapi::OpenApiError;
//!
//! let error = OpenApiError::Parse {
//!     message: "Unexpected end of input".to_string(),
//!     location: Some("line 42".to_string()),
//! };
//!
//! assert!(error.to_string().contains("Parse error"));
//! ```

use std::path::PathBuf;
use thiserror::Error;

/// Errors that can occur when working with OpenAPI specifications.
///
/// This error type covers the full lifecycle of OpenAPI processing:
/// parsing, reference resolution, validation, and I/O operations.
///
/// ## Examples
///
/// Create a parse error:
///
/// ```
/// use schematic_define::openapi::OpenApiError;
///
/// let error = OpenApiError::Parse {
///     message: "Invalid JSON".to_string(),
///     location: None,
/// };
/// ```
///
/// Create a validation error:
///
/// ```
/// use schematic_define::openapi::OpenApiError;
///
/// let error = OpenApiError::Validation {
///     path: "/paths/~1users/get/responses".to_string(),
///     message: "Missing 200 response".to_string(),
/// };
/// ```
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum OpenApiError {
    /// Failed to parse YAML or JSON document.
    ///
    /// This variant is used for syntax errors in the OpenAPI specification file,
    /// including malformed YAML, invalid JSON, or structure that doesn't match
    /// the expected OpenAPI schema.
    #[error("Parse error: {message}{}", location.as_ref().map(|s| format!(" at {s}")).unwrap_or_default())]
    Parse {
        /// Description of the parse error.
        message: String,
        /// Optional source location (line number, column, etc.).
        location: Option<String>,
    },

    /// Unresolved `$ref` reference.
    ///
    /// This variant is used when a `$ref` pointer in the specification
    /// refers to a schema, parameter, or other component that doesn't exist.
    #[error("Reference error at '{ref_path}': {message}")]
    Reference {
        /// The JSON pointer path of the unresolved reference (e.g., `#/components/schemas/User`).
        ref_path: String,
        /// Description of why the reference couldn't be resolved.
        message: String,
    },

    /// One or more schema references could not be resolved in `components.schemas`.
    ///
    /// This variant is used by the export validator when a `$ref` under
    /// `paths` or `components` points to a schema name that is missing from
    /// the generated OpenAPI document.
    #[error("Unresolved schema references: {refs:?}")]
    UnresolvedRefs {
        /// List of missing schema names (e.g., `["User", "CreateUserBody"]`).
        refs: Vec<String>,
    },

    /// Unsupported OpenAPI feature.
    ///
    /// This variant is used when the specification uses features that this
    /// crate doesn't support (e.g., callbacks, links, webhooks).
    #[error("Unsupported feature: {feature}{}", context.as_ref().map(|c| format!(" (in {c})")).unwrap_or_default())]
    UnsupportedFeature {
        /// Name of the unsupported feature.
        feature: String,
        /// Optional context describing where the feature was encountered.
        context: Option<String>,
    },

    /// Semantic validation error.
    ///
    /// This variant is used for errors where the document is syntactically
    /// valid but doesn't meet semantic requirements (e.g., missing required
    /// fields, invalid combinations of options).
    #[error("Validation error at '{path}': {message}")]
    Validation {
        /// JSON pointer path to the problematic element.
        path: String,
        /// Description of the validation failure.
        message: String,
    },

    /// File system I/O error.
    ///
    /// This variant is used when reading an OpenAPI specification file fails
    /// due to file system issues (file not found, permission denied, etc.).
    #[error("I/O error reading '{path}': {message}")]
    Io {
        /// Path to the file that couldn't be read.
        path: PathBuf,
        /// Description of the I/O error.
        message: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_error_display() {
        let error = OpenApiError::Parse {
            message: "Unexpected token".to_string(),
            location: None,
        };
        assert_eq!(error.to_string(), "Parse error: Unexpected token");
    }

    #[test]
    fn parse_error_with_location() {
        let error = OpenApiError::Parse {
            message: "Unexpected token".to_string(),
            location: Some("line 10, column 5".to_string()),
        };
        assert_eq!(
            error.to_string(),
            "Parse error: Unexpected token at line 10, column 5"
        );
    }

    #[test]
    fn reference_error_display() {
        let error = OpenApiError::Reference {
            ref_path: "#/components/schemas/User".to_string(),
            message: "Schema not found".to_string(),
        };
        assert_eq!(
            error.to_string(),
            "Reference error at '#/components/schemas/User': Schema not found"
        );
    }

    #[test]
    fn unsupported_feature_display() {
        let error = OpenApiError::UnsupportedFeature {
            feature: "callbacks".to_string(),
            context: None,
        };
        assert_eq!(error.to_string(), "Unsupported feature: callbacks");
    }

    #[test]
    fn unsupported_feature_with_context() {
        let error = OpenApiError::UnsupportedFeature {
            feature: "callbacks".to_string(),
            context: Some("POST /webhooks".to_string()),
        };
        assert_eq!(
            error.to_string(),
            "Unsupported feature: callbacks (in POST /webhooks)"
        );
    }

    #[test]
    fn validation_error_display() {
        let error = OpenApiError::Validation {
            path: "/paths/~1users".to_string(),
            message: "Missing operationId".to_string(),
        };
        assert_eq!(
            error.to_string(),
            "Validation error at '/paths/~1users': Missing operationId"
        );
    }

    #[test]
    fn io_error_display() {
        let error = OpenApiError::Io {
            path: PathBuf::from("/api/spec.yaml"),
            message: "Permission denied".to_string(),
        };
        assert_eq!(
            error.to_string(),
            "I/O error reading '/api/spec.yaml': Permission denied"
        );
    }
}
