//! Core types for the compose pipeline cache.

/// Controls the run-local, in-memory compose cache.
///
/// Governs reuse within a single compose invocation only. Nothing this mode
/// selects is written to disk; the remote transport cache under a configured
/// cache root has its own controls (`RemoteReadConfig`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum CacheAccessMode {
    /// Caching completely disabled — every request computes fresh.
    Off,
    /// Read from cache but never write new entries.
    ReadOnly,
    /// Full caching: read hits, write misses.
    #[default]
    ReadWrite,
    /// Ignore existing entries and recompute, then write fresh results.
    Refresh,
}

/// Classification of persisted artifacts; selects the store's
/// `manifests/{class}` directory.
///
/// Fetched remote bodies are the only persisted class: until a `ContentPolicy`
/// exists no semantic result (composed document, operation result, document
/// snapshot) may be persisted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArtifactClass {
    /// Remote URL artifact (fetched HTTP/HTTPS response body).
    RemoteUrl,
}

/// Accumulated run-local cache statistics for a single compose run.
///
/// Remote transport-cache activity is reported separately by
/// `RemoteFetchStats`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CacheStats {
    /// Number of run-local cache hits (result already available).
    pub hits: usize,
    /// Number of run-local cache misses (result had to be computed).
    pub misses: usize,
    /// Number of entries written to run-local cache.
    pub writes: usize,
    /// Number of times a thread waited on an in-flight computation.
    pub inflight_waits: usize,
    /// Number of cache-related errors (e.g., timeout fallback).
    pub errors: usize,
}

impl CacheStats {
    /// Merges another stats instance into this one.
    pub fn merge(&mut self, other: &CacheStats) {
        self.hits += other.hits;
        self.misses += other.misses;
        self.writes += other.writes;
        self.inflight_waits += other.inflight_waits;
        self.errors += other.errors;
    }

    /// Returns true if any cache activity occurred.
    pub fn has_activity(&self) -> bool {
        self.hits > 0 || self.misses > 0 || self.writes > 0 || self.inflight_waits > 0
    }
}
