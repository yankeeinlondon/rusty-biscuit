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
}
