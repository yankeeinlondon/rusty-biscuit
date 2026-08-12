use std::path::PathBuf;
use std::sync::Arc;

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
    ///
    /// The underlying backend error (git2 or gix) is boxed as a source so the
    /// concrete per-operation type is preserved in the cause chain without
    /// enumerating every backend error enum here. The `operation` tag records
    /// which conceptual operation failed (e.g. "discover", "status", "diff").
    #[error("Git error during {operation}: {source}")]
    Git {
        /// The conceptual operation that failed.
        operation: &'static str,
        /// The underlying backend error.
        #[source]
        source: Box<dyn std::error::Error + Send + Sync + 'static>,
    },

    /// A request reused a previously failed observation.
    ///
    /// The original typed error remains available through the source chain.
    /// Wrapping it in an [`Arc`] lets request-scoped owners retain one failure
    /// and project it to multiple strict consumers without retrying the work.
    #[error("{source}")]
    RetainedObservation {
        /// The original observation failure.
        #[source]
        source: Arc<SniffError>,
    },

    /// A committed-tip merge would require repository-defined executable behavior.
    #[error(
        "unsupported merge configuration '{setting}' applies to '{}'",
        path.display()
    )]
    UnsupportedMergeConfiguration {
        /// The Git setting that would require a command or unsupported normalization.
        setting: String,
        /// Repository-relative path to which the setting applies.
        path: PathBuf,
    },

    /// The specified path is not a git repository.
    #[error("Not a git repository: {0}")]
    NotARepository(PathBuf),

    /// The repository is not a monorepo (required for package scoping).
    #[error("Not a monorepo: {0}")]
    NotAMonorepo(PathBuf),

    /// The specified package name was not found in the monorepo.
    #[error("package '{name}' not found. Valid packages: {valid}")]
    UnknownPackage { name: String, valid: String },

    /// The specified package name matches more than one catalog entry.
    ///
    /// Returned by scope-override resolvers when `--package` is used and the
    /// name resolves to more than one discovered package. Areas are unique
    /// by name, so ambiguity is reported only for packages.
    #[error("package '{name}' is ambiguous: matches {count} entries ({matches})")]
    AmbiguousPackage {
        name: String,
        count: usize,
        matches: String,
    },

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

    /// An OS call made while discovering the current user's stable identity
    /// failed.
    ///
    /// The `operation` tag names the OS API that failed (e.g.
    /// "OpenProcessToken", "GetTokenInformation") and the underlying error is
    /// boxed as a source so its concrete type survives in the cause chain.
    /// Callers must not soften this into a username or a placeholder: a
    /// process that cannot learn its own principal must not create per-user
    /// private state.
    #[error("user identity error during {operation}: {source}")]
    UserIdentity {
        /// The OS operation that failed.
        operation: &'static str,
        /// The underlying OS error.
        #[source]
        source: Box<dyn std::error::Error + Send + Sync + 'static>,
    },

    /// The OS succeeded but returned something that is not a usable account
    /// identifier (e.g. a malformed SID or an undersized token buffer).
    #[error("invalid user identity from {operation}: {message}")]
    InvalidUserIdentity {
        /// The OS operation that produced the unusable value.
        operation: &'static str,
        /// What failed validation.
        message: String,
    },

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

    /// A requested configured remote does not exist.
    #[error("remote `{name}` is not configured")]
    RemoteNotConfigured {
        /// Exact case-sensitive remote name requested by the caller.
        name: String,
    },

    /// A configured remote has no usable fetch URL.
    #[error("remote `{name}` has no usable URL")]
    RemoteUrlMissing {
        /// Remote whose URL could not be resolved.
        name: String,
    },

    /// The remote host was rejected before network access.
    #[error("remote host `{host}` is not allowed by network policy")]
    RemotePolicyDenied {
        /// Exact host rejected by the shared fetch policy.
        host: String,
    },

    /// The requested observation cannot be performed for this transport/provider.
    #[error("remote capability `{capability}` is unsupported for {target}")]
    UnsupportedRemoteCapability {
        /// Operation the caller requested.
        capability: &'static str,
        /// Transport or provider that cannot supply it.
        target: String,
    },

    /// A provider family was identified, but its server version cannot supply
    /// the requested endpoint contract.
    #[error(
        "{provider} API flavor {flavor} at server version {version} does not support {capability}; {requirement}"
    )]
    UnsupportedServerVersion {
        /// Provider family selected for the query.
        provider: String,
        /// Concrete API flavor detected from the server response.
        flavor: String,
        /// Server-reported version, retained verbatim for diagnosis.
        version: String,
        /// Operation that cannot be performed.
        capability: &'static str,
        /// Minimum version or endpoint requirement that would satisfy it.
        requirement: &'static str,
    },

    /// The remote endpoint could not be reached or decoded.
    #[error("remote endpoint `{url}` is unreachable: {message}")]
    RemoteUnreachable {
        /// Sanitized endpoint URL without credentials.
        url: String,
        /// Focused transport or response error.
        message: String,
    },

    /// The remote authenticated the request but denied the operation.
    #[error("{provider} denied remote operation: {message}")]
    RemoteForbidden {
        /// Provider or transport that returned the denial.
        provider: String,
        /// Focused denial detail.
        message: String,
    },

    /// A provider query is malformed before any request is issued.
    #[error("invalid remote query field `{field}`: {message}")]
    InvalidRemoteQuery {
        /// Query field that failed validation.
        field: &'static str,
        /// Focused validation detail.
        message: String,
    },

    /// A valid canonical filter cannot be honored exactly by the selected adapter.
    #[error("remote filter `{field}` is unsupported by {provider}")]
    UnsupportedRemoteFilter {
        /// Canonical query field that is unavailable.
        field: &'static str,
        /// Provider and API flavor selected for the query.
        provider: String,
    },

    /// A bounded-traversal cap was reached before the provider exhausted the
    /// result set, so no complete result domain exists.
    ///
    /// Exact canonical filters and ordering are emulated locally over the whole
    /// domain. A partial domain would silently answer the wrong question, so the
    /// query fails instead of returning truncated or empty items.
    #[error(
        "remote query against {provider} reached the bounded `{bound}` limit of {limit} before the provider was exhausted; narrow the query so a complete result domain can be traversed"
    )]
    IncompleteRemoteDomain {
        /// Provider and API flavor selected for the query.
        provider: String,
        /// Traversal bound that stopped the walk.
        bound: &'static str,
        /// Configured maximum for that bound.
        limit: usize,
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

    /// The install command was killed at its deadline rather than exiting.
    ///
    /// Distinct from [`SniffInstallationError::PackageManagerFailed`]: the
    /// package manager never reached a verdict, so the host may hold a
    /// partial install. On Unix termination is best-effort — a descendant
    /// that detached with `setsid()` outlives the kill and may still be
    /// modifying the host. Callers that surface this to a user must say so.
    #[error(
        "The package manager {manager} did not finish installing {pkg} within {timeout_secs}s and was terminated; a detached installer process may still be modifying this host"
    )]
    InstallationTimedOut {
        pkg: String,
        manager: String,
        timeout_secs: u64,
    },

    #[error("The package {pkg} is not installable on {os}!")]
    NotInstallableOnOs { pkg: String, os: String },

    #[error(
        "The package {pkg} requires a package manager ({manager}) which is NOT installed on this computer!"
    )]
    MissingPackageManager { pkg: String, manager: String },

    /// No runnable installation method exists for this program on this host.
    ///
    /// The embedded `detail` is human-readable and already aware of the rejection
    /// reasons for every evaluated method. Callers that want the full plan should
    /// call `install_plan()` directly rather than relying on `install()`.
    #[error("No viable installation method for {pkg}: {detail}")]
    NoViableMethod { pkg: String, detail: String },

    /// A remote-bash installation was selected but execution has not been
    /// authorized by the caller.
    #[error("Installing {pkg} via remote bash requires explicit consent (url: {url})")]
    RemoteBashConsentRequired { pkg: String, url: String },
}

impl SniffError {
    /// Construct a [`SniffError::Git`] tagging the failing operation and boxing
    /// the backend error as its source.
    pub(crate) fn git(
        operation: &'static str,
        source: impl Into<Box<dyn std::error::Error + Send + Sync + 'static>>,
    ) -> Self {
        SniffError::Git {
            operation,
            source: source.into(),
        }
    }

    /// Construct a [`SniffError::UserIdentity`] tagging the failing OS
    /// operation and boxing its error as the source.
    ///
    /// Only the Windows identity backend can fail this way today — `geteuid`
    /// is infallible — so non-Windows builds see this only from tests.
    #[cfg_attr(not(windows), allow(dead_code))]
    pub(crate) fn user_identity(
        operation: &'static str,
        source: impl Into<Box<dyn std::error::Error + Send + Sync + 'static>>,
    ) -> Self {
        SniffError::UserIdentity {
            operation,
            source: source.into(),
        }
    }
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

    #[test]
    fn test_no_viable_method_display() {
        let err = SniffInstallationError::NoViableMethod {
            pkg: "vim".to_string(),
            detail: "no runnable installation method".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("vim"));
        assert!(msg.contains("no runnable installation method"));
    }

    #[test]
    fn test_remote_bash_consent_required_display() {
        let err = SniffInstallationError::RemoteBashConsentRequired {
            pkg: "rustup".to_string(),
            url: "https://sh.rustup.rs".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("rustup"));
        assert!(msg.contains("https://sh.rustup.rs"));
        assert!(msg.to_lowercase().contains("consent"));
    }
}
