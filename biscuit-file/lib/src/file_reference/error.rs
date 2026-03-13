use std::path::PathBuf;

/// Errors that can occur when parsing or resolving a file reference.
///
/// ## Examples
///
/// ```rust
/// use biscuit_file::FileReference;
///
/// // Invalid syntax produces an error
/// let err = FileReference::new("{{}}").unwrap_err();
/// assert!(err.to_string().contains("invalid"));
/// ```
#[derive(Debug, thiserror::Error)]
pub enum FileReferenceError {
    #[error("file reference syntax is invalid: {0}")]
    InvalidSyntax(String),

    #[error("environment variable `{name}` is not set")]
    MissingEnvironmentVariable { name: String },

    #[error("could not determine the current working directory: {0}")]
    CurrentDirectory(#[source] std::io::Error),

    #[error("could not inspect git repository state: {0}")]
    Git(String),

    #[error("could not inspect Cargo workspace state: {0}")]
    Workspace(String),

    #[error("vault reference used without any configured vault roots")]
    VaultNotConfigured,

    #[error("could not produce a relative path from `{from}` to `{to}`")]
    RelativePath { from: PathBuf, to: PathBuf },

    #[error("filesystem error while resolving `{path}`: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}
