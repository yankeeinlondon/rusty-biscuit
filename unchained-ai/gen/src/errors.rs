use thiserror::Error;

#[derive(Debug, Error)]
pub enum GeneratorError {
    #[error("Failed to fetch models from {provider}: {reason}")]
    FetchFailed { provider: String, reason: String },

    #[error("Failed to write output file {path}: {reason}")]
    WriteFailed { path: String, reason: String },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
