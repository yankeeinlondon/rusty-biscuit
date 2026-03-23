//! Core types for the compose pipeline cache.

use serde::{Deserialize, Serialize};

/// Controls whether caching is active and in what mode.
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

/// Controls staleness tolerance for cached artifacts.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum CacheFreshnessMode {
    /// Only accept entries whose closure hash matches current state.
    #[default]
    Strict,
    /// Prefer fresh, but serve stale on revalidation failure.
    Fallback,
    /// Serve any cached entry without revalidation.
    Optimistic,
    /// Serve cached entry unconditionally, skip all checks.
    Forced,
}

/// Classification of cached artifacts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactClass {
    /// Raw document snapshot (parsed markdown before compose).
    DocumentSnapshot,
    /// Composed document core (after recursive compose, before parent transforms).
    ComposeDocumentCore,
    /// Individual operation result (code transclusion, TOC linking).
    OperationResult,
}

/// Classification of a document's source origin.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceKind {
    /// Local filesystem path.
    LocalFile,
    /// Remote URL (HTTP/HTTPS).
    RemoteUrl,
}

/// Reference to a dependency in a Merkle-style closure hash chain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DependencyRef {
    /// Hash of the dependency's source identifier.
    pub source_id_hash: u64,
    /// Closure hash of the dependency (includes its own transitive deps).
    pub closure_hash: u64,
}

/// Accumulated cache statistics for a single compose run.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CacheStats {
    // ── Run-local (Phase 1) ────────────────────────────────────
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

    // ── Persistent (Phase 2) ───────────────────────────────────
    /// Number of persistent cache hits.
    pub persistent_hits: usize,
    /// Number of entries written to persistent cache.
    pub persistent_writes: usize,
    /// Number of closure hash revalidation checks performed.
    pub revalidations: usize,
    /// Number of stale entries served (Fallback/Optimistic modes).
    pub stale_hits: usize,
}

impl CacheStats {
    /// Merges another stats instance into this one.
    pub fn merge(&mut self, other: &CacheStats) {
        self.hits += other.hits;
        self.misses += other.misses;
        self.writes += other.writes;
        self.inflight_waits += other.inflight_waits;
        self.errors += other.errors;
        self.persistent_hits += other.persistent_hits;
        self.persistent_writes += other.persistent_writes;
        self.revalidations += other.revalidations;
        self.stale_hits += other.stale_hits;
    }

    /// Returns true if any cache activity occurred.
    pub fn has_activity(&self) -> bool {
        self.hits > 0
            || self.misses > 0
            || self.writes > 0
            || self.inflight_waits > 0
            || self.persistent_hits > 0
            || self.persistent_writes > 0
    }
}
