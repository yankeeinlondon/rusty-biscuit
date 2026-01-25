use std::path::PathBuf;

/// Error types for the Sniff library.
///
/// This enum encompasses all possible errors that can occur during
/// system information gathering, git repository analysis, and language detection.
#[derive(Debug, thiserror::Error)]
pub enum SniffError {
    /// IO error occurred during file operations.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Git operation failed.
    #[error("Git error: {0}")]
    Git(#[from] git2::Error),

    /// The specified path is not a git repository.
    #[error("Not a git repository: {0}")]
    NotARepository(PathBuf),

    /// Error gathering system information.
    ///
    /// The `domain` field indicates which system area failed
    /// (e.g., "hardware", "network", "filesystem").
    #[error("System info error in {domain}: {message}")]
    SystemInfo {
        domain: &'static str,
        message: String,
    },

    /// Language detection failed for the given reason.
    #[error("Language detection failed: {0}")]
    LanguageDetection(String),

}

#[derive(Debug, thiserror::Error)]
pub enum SniffInstallationError {

    #[error("Error installing {pkg} on host using the install command: {cmd}")]
    InstallationError {
        pkg: String,
        cmd: String
    },

    #[error("The package manager {manager} failed to install {pkg}: {msg}")]
    PackageManagerFailed {
        pkg: String,
        manager: String,
        msg: String
    },

    #[error("The package {pkg} is not installable on {os}!")]
    NotInstallableOnOs {
        pkg: String,
        os: String
    },

    #[error("The package {pkg} requires a package manager ({manager}) which is NOT installed on this computer!")]
    MissingPackageManager {
        pkg: String,
        manager: String,
    }
}

/// Convenience Result type for Sniff operations.
pub type Result<T> = std::result::Result<T, SniffError>;
