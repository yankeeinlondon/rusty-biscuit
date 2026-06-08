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
