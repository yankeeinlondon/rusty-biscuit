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

    /// The repository is not a monorepo (required for package scoping).
    #[error("Not a monorepo: {0}")]
    NotAMonorepo(PathBuf),

    /// The specified package name was not found in the monorepo.
    #[error("package '{name}' not found. Valid packages: {valid}")]
    UnknownPackage { name: String, valid: String },

    /// The specified package area was not found in the monorepo.
    #[error("package area '{area}' not found. Valid areas: {valid}")]
    UnknownPackageArea { area: String, valid: String },

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

    // ─────────────────────────────────────────────────────────────────────────
    // Remote provider errors (requires `remote` feature)
    // ─────────────────────────────────────────────────────────────────────────
    /// Failed to initialize a remote provider client.
    #[error("failed to initialize {provider} remote provider: {message}")]
    RemoteInit {
        /// Provider name (e.g., "GitHub", "GitLab").
        provider: String,
        /// Initialization error message.
        message: String,
    },

    /// Remote API request failed.
    #[error("{provider} API error (HTTP {status}): {message}")]
    RemoteApi {
        /// Provider name (e.g., "GitHub", "GitLab").
        provider: String,
        /// HTTP status code.
        status: u16,
        /// Error message from the API or description.
        message: String,
    },

    /// Unsupported hosting provider for remote queries.
    ///
    /// The URL points to a git hosting provider that is not yet supported
    /// for remote inspection (e.g., SourceHut, self-hosted servers).
    #[error("unsupported hosting provider for remote inspection: {url}")]
    UnsupportedProvider {
        /// The remote URL that could not be resolved to a supported provider.
        url: String,
    },

    /// Authentication credentials not configured for provider.
    #[error("missing credentials for {provider}: set the {env_var} environment variable")]
    MissingCredentials {
        /// Provider name (e.g., "GitHub", "GitLab").
        provider: String,
        /// Environment variable name that should be set.
        env_var: String,
    },

    /// No provider could resolve an `owner/repo` shorthand.
    ///
    /// This is returned by `GitRemote::from_shorthand` when all tried providers
    /// returned 404 (not found). The message lists which providers were attempted.
    #[error("repository '{owner}/{repo}' not found on any provider (tried: {providers_tried})")]
    ShorthandNotFound {
        /// Repository owner.
        owner: String,
        /// Repository name.
        repo: String,
        /// Comma-separated list of providers that were tried.
        providers_tried: String,
    },

    /// Authentication credentials were provided but rejected by the provider.
    #[error("{provider} rejected credentials: {message}")]
    InvalidCredentials {
        /// Provider name (e.g., "GitHub", "GitLab").
        provider: String,
        /// Human-readable reason for rejection.
        message: String,
    },

    /// The specified commit hash is not an ancestor of HEAD.
    #[error("commit {hash} is not reachable from HEAD")]
    HashNotReachable { hash: String },

    /// Invalid period specifier for recent-commits queries.
    #[error(
        "invalid period specifier: '{0}'. Expected duration (e.g., 3d, 1w), date (YYYY-MM-DD), hash, 'today', or 'yesterday'."
    )]
    InvalidPeriod(String),

    /// Rate limited by the hosting provider API.
    #[error("rate limited by {provider} API{}", retry_after.map(|s| format!(", retry after {}s", s)).unwrap_or_default())]
    RateLimited {
        /// Provider name (e.g., "GitHub", "GitLab").
        provider: String,
        /// Seconds until rate limit resets (if provided by the API).
        retry_after: Option<u64>,
    },
}

#[derive(Debug, thiserror::Error)]
pub enum SniffInstallationError {
    #[error("Error installing {pkg} on host using the install command: {cmd}")]
    InstallationError { pkg: String, cmd: String },

    #[error("The package manager {manager} failed to install {pkg}: {msg}")]
    PackageManagerFailed {
        pkg: String,
        manager: String,
        msg: String,
    },

    #[error("The package {pkg} is not installable on {os}!")]
    NotInstallableOnOs { pkg: String, os: String },

    #[error(
        "The package {pkg} requires a package manager ({manager}) which is NOT installed on this computer!"
    )]
    MissingPackageManager { pkg: String, manager: String },
}

/// Convenience Result type for Sniff operations.
pub type Result<T> = std::result::Result<T, SniffError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shorthand_not_found_display() {
        let err = SniffError::ShorthandNotFound {
            owner: "user".to_string(),
            repo: "repo".to_string(),
            providers_tried: "GitHub, GitLab".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("user/repo"));
        assert!(msg.contains("GitHub, GitLab"));
    }

    #[test]
    fn test_invalid_credentials_display() {
        let err = SniffError::InvalidCredentials {
            provider: "GitHub".to_string(),
            message: "token expired".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("GitHub"));
        assert!(msg.contains("token expired"));
    }

    #[test]
    fn test_rate_limited_display_with_retry() {
        let err = SniffError::RateLimited {
            provider: "GitHub".to_string(),
            retry_after: Some(60),
        };
        let msg = err.to_string();
        assert!(msg.contains("rate limited"));
        assert!(msg.contains("60s"));
    }

    #[test]
    fn test_rate_limited_display_without_retry() {
        let err = SniffError::RateLimited {
            provider: "GitHub".to_string(),
            retry_after: None,
        };
        let msg = err.to_string();
        assert!(msg.contains("rate limited"));
    }

    #[test]
    fn test_installation_error_display() {
        let err = SniffInstallationError::InstallationError {
            pkg: "vim".to_string(),
            cmd: "brew install vim".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("vim"));
        assert!(msg.contains("brew install vim"));
    }

    #[test]
    fn test_not_installable_on_os_display() {
        let err = SniffInstallationError::NotInstallableOnOs {
            pkg: "winget".to_string(),
            os: "macos".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("winget"));
        assert!(msg.contains("macos"));
    }

    #[test]
    fn test_missing_package_manager_display() {
        let err = SniffInstallationError::MissingPackageManager {
            pkg: "ripgrep".to_string(),
            manager: "brew".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("ripgrep"));
        assert!(msg.contains("brew"));
    }
}
