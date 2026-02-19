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
    YamlParse(#[from] serde_yaml::Error),

    /// Configuration file not found at expected path.
    #[error("config not found: {0}")]
    ConfigNotFound(PathBuf),

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
}

/// Convenience type alias for Claudine results.
pub type Result<T> = std::result::Result<T, ClaudineError>;
