use biscuit_file::YamlParseError;
use std::path::PathBuf;

/// All errors that can occur within the Claudine library.
#[derive(Debug, thiserror::Error)]
pub enum ClaudineError {
    /// I/O error during file or directory operations.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Failed to parse JSON content.
    #[error("JSON parse error: {0}")]
    JsonParse(#[from] serde_json::Error),

    /// Failed to parse TOML content.
    #[error("TOML parse error: {0}")]
    TomlParse(#[from] toml_edit::TomlError),

    /// Failed to parse YAML content (e.g., SKILL.md frontmatter).
    #[error("YAML parse error: {0}")]
    YamlParse(#[from] YamlParseError),

    /// Configuration file not found at expected path.
    #[error("config not found: {0}")]
    ConfigNotFound(PathBuf),

    /// Configuration file failed semantic validation.
    #[error("config validation error: {0}")]
    ConfigValidation(String),

    /// Requested provider is not available or not detected.
    #[error("provider not available: {0}")]
    ProviderNotAvailable(String),

    /// Error during template interpolation.
    #[error("template error: {0}")]
    TemplateError(String),

    /// Error during skill/command linking.
    #[error("linking error: {0}")]
    LinkingError(String),

    /// HTTP request failed (e.g., log server POST).
    #[error("HTTP error: {0}")]
    HttpError(#[from] reqwest::Error),

    /// File lock could not be acquired within timeout.
    #[error("lock timeout on {path}")]
    LockError {
        /// Path that could not be locked.
        path: PathBuf,
    },

    /// SQLite access failed while working with the reporting index.
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    /// Date or timestamp parsing failed.
    #[error("date/time parse error: {0}")]
    ChronoParse(#[from] chrono::ParseError),

    /// Regex compilation failed.
    #[error("invalid regex pattern: {0}")]
    RegexError(#[from] regex::Error),

    /// URL parsing failed.
    #[error("invalid URL: {0}")]
    UrlError(#[from] url::ParseError),

    /// Provider adapter parse/format error.
    #[error("adapter error: {0}")]
    Adapter(#[from] crate::adapters::AdapterError),

    /// Provider does not support automatic config creation.
    #[error("config creation not supported for provider: {provider}")]
    ConfigCreationNotSupported {
        /// Provider name.
        provider: String,
    },

    /// Protect rule pattern failed to parse as regex.
    #[error("protect rule parse error for pattern `{pattern}`: {source}")]
    ProtectRuleParse {
        /// Rule regex pattern.
        pattern: String,
        /// Regex parser error.
        source: regex::Error,
    },

    /// Protect policy is semantically invalid.
    #[error("protect policy invalid: {0}")]
    ProtectInvalidPolicy(String),

    /// Failed mapping a protect outcome to provider-native enforcement.
    #[error("protect enforcement mapping error: {0}")]
    ProtectEnforcementMapping(String),

    /// Required Claudine reporting path could not be determined.
    #[error("reporting path unavailable: {0}")]
    ReportingPathUnavailable(String),

    /// Reporting date range is invalid.
    #[error("invalid reporting date range: {from} > {to}")]
    InvalidReportingDateRange {
        /// Inclusive range start.
        from: String,
        /// Inclusive range end.
        to: String,
    },
}

/// Convenience type alias for Claudine results.
pub type Result<T> = std::result::Result<T, ClaudineError>;
