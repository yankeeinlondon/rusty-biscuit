use std::collections::BTreeMap;
use std::path::PathBuf;

use super::native::ProviderCliOverrides;

/// Discovery context for policy resolution.
///
/// Provides the ambient state the engine needs to discover and resolve
/// provider policies without reading directly from the process environment.
#[derive(Debug, Clone)]
pub struct PolicyContext {
    /// Current working directory. Always required.
    pub cwd: PathBuf,
    /// Repository root, if known. `None` when outside a repository.
    pub repo_root: Option<PathBuf>,
    /// User home directory. Overridable for deterministic tests.
    pub home_dir: Option<PathBuf>,
    /// System root directory. Overridable for deterministic tests.
    pub system_root: Option<PathBuf>,
    /// Captured environment variables relevant to provider config discovery.
    ///
    /// Owned for deterministic tests. Backends should prefer these over
    /// reading `std::env` directly.
    pub env: BTreeMap<String, String>,
    /// Project trust state for providers with trust gates.
    pub trust: ProjectTrustContext,
}

impl PolicyContext {
    /// Creates a minimal context with just a working directory.
    pub fn new(cwd: PathBuf) -> Self {
        Self {
            cwd,
            repo_root: None,
            home_dir: dirs::home_dir(),
            system_root: None,
            env: BTreeMap::new(),
            trust: ProjectTrustContext::default(),
        }
    }

    /// Sets the repository root.
    pub fn with_repo_root(mut self, repo_root: PathBuf) -> Self {
        self.repo_root = Some(repo_root);
        self
    }

    /// Sets the home directory.
    pub fn with_home_dir(mut self, home_dir: PathBuf) -> Self {
        self.home_dir = Some(home_dir);
        self
    }

    /// Sets the trust context.
    pub fn with_trust(mut self, trust: ProjectTrustContext) -> Self {
        self.trust = trust;
        self
    }
}

/// Trust state for the current project.
#[derive(Debug, Clone)]
pub struct ProjectTrustContext {
    /// Whether the project is trusted. `None` when unknown.
    pub is_trusted: Option<bool>,
    /// How trust state was determined.
    pub source: TrustSource,
}

impl Default for ProjectTrustContext {
    fn default() -> Self {
        Self {
            is_trusted: None,
            source: TrustSource::Unknown,
        }
    }
}

/// How project trust state was determined.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrustSource {
    /// Trust state was read from provider config.
    ProviderConfig,
    /// Trust state was explicitly supplied by the caller.
    ExplicitInput,
    /// Trust state is not known.
    Unknown,
}

/// CLI/runtime override input for effective policy resolution.
#[derive(Debug)]
pub enum CliPolicyInput<'a> {
    /// No CLI overrides.
    None,
    /// Raw argv to be parsed by the provider backend.
    Argv(&'a [String]),
    /// Pre-parsed overrides from a backend.
    Parsed(&'a ProviderCliOverrides),
}
