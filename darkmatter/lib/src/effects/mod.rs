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
    effect_descriptors, effect_verbs, EffectDescriptor, EffectSafety, EffectVerb,
    EFFECT_DESCRIPTORS, EFFECT_VERBS,
};
pub use error::EffectError;

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

/// Counts every [`EffectEngine`] constructed in this process. Used as test
/// instrumentation to prove that documentation-only paths (e.g. Claudine's
/// `context --side-effects` report) never instantiate an effect engine.
static ENGINE_BUILD_COUNT: AtomicU64 = AtomicU64::new(0);

/// Counts every network attempt made by [`EffectEngine::http_post`], including
/// attempts the allowlist refuses before any socket is opened. Used as test
/// instrumentation to prove metadata-only paths attempt no network.
static NETWORK_ATTEMPT_COUNT: AtomicU64 = AtomicU64::new(0);

pub(crate) fn record_network_attempt() {
    NETWORK_ATTEMPT_COUNT.fetch_add(1, Ordering::Relaxed);
}

/// Returns how many [`EffectEngine`] instances have been built this process.
///
/// Test instrumentation; compare a before/after delta around a code path to
/// assert it constructed no engine.
#[doc(hidden)]
pub fn engine_build_count() -> u64 {
    ENGINE_BUILD_COUNT.load(Ordering::Relaxed)
}

/// Returns how many network attempts [`EffectEngine::http_post`] has made this
/// process (including allowlist-refused attempts).
///
/// Test instrumentation; compare a before/after delta around a code path to
/// assert it attempted no network.
#[doc(hidden)]
pub fn network_attempt_count() -> u64 {
    NETWORK_ATTEMPT_COUNT.load(Ordering::Relaxed)
}

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
        ENGINE_BUILD_COUNT.fetch_add(1, Ordering::Relaxed);
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
