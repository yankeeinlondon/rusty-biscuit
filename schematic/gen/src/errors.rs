//! Error types for the schematic generator.

use thiserror::Error;

/// Errors that can occur during code generation.
#[derive(Debug, Error)]
pub enum GeneratorError {
    /// Failed to parse API definition
    #[error("Failed to parse API definition: {0}")]
    ParseError(String),

    /// Failed to generate code
    #[error("Code generation failed: {0}")]
    CodeGenError(String),

    /// Failed to write output file
    #[error("Failed to write output file '{path}': {source}")]
    WriteError {
        path: String,
        #[source]
        source: std::io::Error,
    },

    /// Output directory does not exist
    #[error("Output directory does not exist: {0}")]
    OutputDirNotFound(String),

    /// Invalid configuration
    #[error("Invalid configuration: {0}")]
    ConfigError(String),

    /// Naming collision between body type and generated wrapper struct.
    ///
    /// This occurs when an endpoint's body type name matches the generated
    /// request struct name (e.g., body type "CreateUserRequest" conflicts
    /// with endpoint "CreateUser" + suffix "Request").
    #[error(
        "Naming collision for endpoint '{endpoint_id}': body type '{body_type}' conflicts with generated request struct name. Suggestion: rename the body type to '{suggestion}'"
    )]
    NamingCollision {
        /// The endpoint ID that has the collision.
        endpoint_id: String,
        /// The body type name that conflicts.
        body_type: String,
        /// Suggested alternative name for the body type.
        suggestion: String,
    },

    /// Invalid request suffix configuration.
    ///
    /// The request suffix must be alphanumeric (letters and numbers only)
    /// to ensure valid Rust identifier generation.
    #[error("Invalid request suffix '{suffix}': {reason}")]
    InvalidRequestSuffix {
        /// The invalid suffix value.
        suffix: String,
        /// Explanation of why the suffix is invalid.
        reason: String,
    },
}
