//! Side-effect engine: a callable catalog of *mutating* operations.
//!
//! Unlike the read-only expression engine, these operations change external
//! state. The engine is deliberately **not** wired into the compose pipeline —
//! composing a document never invokes a side effect. Only an external
//! orchestrator (e.g. Claudine's lifecycle stack) drives it.

pub mod catalog;
mod error;
mod fs_write;
mod verbs;

pub use catalog::{
    effect_descriptors, EffectDescriptor, EffectSafety, EFFECT_DESCRIPTORS,
};
pub use error::EffectError;

use std::path::{Path, PathBuf};

/// Process-wide effect instrumentation, compiled only under the
/// `effects-instrumentation` feature.
///
/// The counters let a downstream test suite assert that a documentation-only
/// code path (e.g. Claudine's `context --side-effects` report) constructs no
/// [`EffectEngine`] and attempts no network access. Production builds do not
/// enable the feature, so they carry no counter state and the atomics never
/// appear on the engine-build or `http_post` hot paths.
#[cfg(feature = "effects-instrumentation")]
mod instrumentation {
    use std::sync::atomic::{AtomicU64, Ordering};

    static ENGINE_BUILD_COUNT: AtomicU64 = AtomicU64::new(0);
    static NETWORK_ATTEMPT_COUNT: AtomicU64 = AtomicU64::new(0);

    pub(crate) fn record_engine_build() {
        ENGINE_BUILD_COUNT.fetch_add(1, Ordering::Relaxed);
    }

    pub(crate) fn record_network_attempt() {
        NETWORK_ATTEMPT_COUNT.fetch_add(1, Ordering::Relaxed);
    }

    /// Returns how many [`super::EffectEngine`] instances have been built this
    /// process. Compare a before/after delta around a code path to assert it
    /// constructed no engine.
    pub fn engine_build_count() -> u64 {
        ENGINE_BUILD_COUNT.load(Ordering::Relaxed)
    }

    /// Returns how many network attempts [`super::EffectEngine::http_post`] has
    /// made this process (including allowlist-refused attempts).
    pub fn network_attempt_count() -> u64 {
        NETWORK_ATTEMPT_COUNT.load(Ordering::Relaxed)
    }
}

#[cfg(feature = "effects-instrumentation")]
pub use instrumentation::{engine_build_count, network_attempt_count};

#[cfg(feature = "effects-instrumentation")]
pub(crate) use instrumentation::record_network_attempt;

/// The mutating side-effect engine. Construct via [`EffectEngine::builder`].
#[derive(Clone, Debug)]
pub struct EffectEngine {
    mutation_root: PathBuf,
    allowed_hosts: Vec<String>,
    auto_rehash: bool,
}

impl EffectEngine {
    pub fn builder() -> EffectEngineBuilder {
        EffectEngineBuilder::default()
    }
    pub fn mutation_root(&self) -> &Path {
        &self.mutation_root
    }
    pub fn allowed_hosts(&self) -> &[String] {
        &self.allowed_hosts
    }
    pub fn auto_rehash(&self) -> bool {
        self.auto_rehash
    }
}

/// Builder for [`EffectEngine`].
#[derive(Debug)]
pub struct EffectEngineBuilder {
    mutation_root: PathBuf,
    allowed_hosts: Vec<String>,
    auto_rehash: bool,
}

impl Default for EffectEngineBuilder {
    fn default() -> Self {
        Self {
            mutation_root: PathBuf::from("."),
            // Deny-all by default: a host must be explicitly allow-listed.
            allowed_hosts: Vec::new(),
            auto_rehash: true,
        }
    }
}

impl EffectEngineBuilder {
    pub fn mutation_root(mut self, root: impl Into<PathBuf>) -> Self {
        self.mutation_root = root.into();
        self
    }
    pub fn allowed_hosts<I, S>(mut self, hosts: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.allowed_hosts = hosts.into_iter().map(Into::into).collect();
        self
    }
    pub fn auto_rehash(mut self, on: bool) -> Self {
        self.auto_rehash = on;
        self
    }
    pub fn build(self) -> EffectEngine {
        #[cfg(feature = "effects-instrumentation")]
        instrumentation::record_engine_build();
        EffectEngine {
            mutation_root: self.mutation_root,
            allowed_hosts: self.allowed_hosts,
            auto_rehash: self.auto_rehash,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_sets_defaults() {
        let dir = tempfile::TempDir::new().unwrap();
        let engine = EffectEngine::builder().mutation_root(dir.path()).build();
        assert!(engine.auto_rehash());
        assert!(engine.allowed_hosts().is_empty());
    }
}
