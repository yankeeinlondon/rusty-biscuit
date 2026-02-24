//! Core error types for the biscuit-file library.

use thiserror::Error;

/// Top-level error type that unifies all format-specific errors.
///
/// This error type provides a common interface for handling errors from
/// TOML, YAML, and PDF operations. Each variant wraps the format-specific
/// error type for detailed diagnostics.
#[derive(Debug, Error)]
pub enum BiscuitFileError {
    /// Error during TOML operations.
    #[cfg(feature = "toml")]
    #[error("TOML error: {0}")]
    Toml(#[from] crate::toml::TomlError),

    /// Error during YAML operations.
    #[cfg(feature = "yaml")]
    #[error("YAML error: {0}")]
    Yaml(#[from] crate::yaml::YamlError),

    /// Error during JSON5 operations.
    #[cfg(feature = "json5")]
    #[error("JSON5 error: {0}")]
    Json5(#[from] crate::json5::Json5Error),

    /// Error during PDF operations.
    #[cfg(any(feature = "extract", feature = "lopdf", feature = "pdfium"))]
    #[error("PDF error: {0}")]
    Pdf(#[from] crate::pdf::PdfError),

    /// IO error not associated with a specific format.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// File type could not be detected.
    #[error("Unknown file type: {0}")]
    UnknownFileType(String),

    /// Feature required for this operation is not enabled.
    #[error("Feature not enabled: {0}")]
    FeatureNotEnabled(String),
}
