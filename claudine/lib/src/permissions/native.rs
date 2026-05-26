use std::any::Any;
use std::fmt;
use std::path::PathBuf;

use crate::provider::Provider;
/// A discovered policy source from which native config can be loaded.
#[derive(Debug, Clone)]
pub struct PolicySource {
    /// Unique identifier for this source within a resolution.
    pub id: String,
    /// Kind of source.
    pub kind: PolicySourceKind,
    /// Filesystem path, if this source is file-based.
    pub path: Option<PathBuf>,
    /// Precedence rank (lower numbers take priority).
    pub precedence: u16,
    /// Whether this source can be written to for mutation planning.
    pub writable: bool,
}

/// Classification of a policy source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum PolicySourceKind {
    /// User-scope configuration file.
    UserConfig,
    /// Repository-scope configuration file.
    RepoConfig,
    /// Local override file.
    LocalOverride,
    /// Managed or enterprise configuration.
    ManagedConfig,
    /// System-wide configuration.
    SystemConfig,
    /// Profile-specific configuration.
    ProfileConfig,
    /// Standalone rule file.
    RuleFile,
    /// CLI flag overrides (ephemeral).
    CliOverride,
    /// Environment variable overrides (ephemeral).
    EnvironmentOverride,
    /// Derived or computed policy (not directly from a file).
    Derived,
}

/// A single loaded native policy layer from one source.
///
/// The payload is provider-specific and opaque to the engine. Backends
/// create and consume layers using [`payload`](Self::payload) to downcast.
pub struct NativePolicyLayer {
    /// The source this layer was loaded from.
    pub source: PolicySource,
    inner: Box<dyn Any + Send + Sync>,
}

impl NativePolicyLayer {
    /// Creates a new layer with a typed payload.
    pub fn new<T: Send + Sync + 'static>(source: PolicySource, payload: T) -> Self {
        Self {
            source,
            inner: Box::new(payload),
        }
    }

    pub fn payload<T: 'static>(&self) -> Option<&T> {
        self.inner.downcast_ref()
    }
}

impl fmt::Debug for NativePolicyLayer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NativePolicyLayer")
            .field("source", &self.source)
            .finish_non_exhaustive()
    }
}

/// Composed native policy after merging all layers for a provider.
///
/// The payload is provider-specific and opaque to the engine.
pub struct NativeEffectivePolicy {
    /// Provider this policy belongs to.
    pub provider: Provider,
    /// Sources that contributed to this policy, in precedence order.
    pub sources: Vec<PolicySource>,
    inner: Box<dyn Any + Send + Sync>,
}

impl NativeEffectivePolicy {
    /// Creates a new effective policy with a typed payload.
    pub fn new<T: Send + Sync + 'static>(
        provider: Provider,
        sources: Vec<PolicySource>,
        payload: T,
    ) -> Self {
        Self {
            provider,
            sources,
            inner: Box::new(payload),
        }
    }

    pub fn payload<T: 'static>(&self) -> Option<&T> {
        self.inner.downcast_ref()
    }
}

impl fmt::Debug for NativeEffectivePolicy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NativeEffectivePolicy")
            .field("provider", &self.provider)
            .field("sources", &self.sources)
            .finish_non_exhaustive()
    }
}

/// Parsed CLI overrides for a provider.
///
/// The payload is provider-specific and opaque to the engine.
pub struct ProviderCliOverrides {
    /// Provider these overrides apply to.
    pub provider: Provider,
    inner: Box<dyn Any + Send + Sync>,
}

impl ProviderCliOverrides {
    /// Creates new CLI overrides with a typed payload.
    pub fn new<T: Send + Sync + 'static>(provider: Provider, payload: T) -> Self {
        Self {
            provider,
            inner: Box::new(payload),
        }
    }

    /// Creates empty CLI overrides (no overrides).
    pub fn none(provider: Provider) -> Self {
        Self {
            provider,
            inner: Box::new(()),
        }
    }

    pub fn payload<T: 'static>(&self) -> Option<&T> {
        self.inner.downcast_ref()
    }
}

impl fmt::Debug for ProviderCliOverrides {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProviderCliOverrides")
            .field("provider", &self.provider)
            .finish_non_exhaustive()
    }
}
