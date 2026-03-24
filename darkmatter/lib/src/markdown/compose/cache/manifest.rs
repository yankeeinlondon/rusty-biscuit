//! Manifest types for persistent cache artifacts.
//!
//! Each cached artifact has a JSON manifest describing its provenance,
//! hashes, and dependencies. Manifests enable Merkle-style invalidation:
//! a composed document is valid only when its closure hash matches the
//! current state of all transitive dependencies.

use super::types::{DependencyRef, SourceKind};
use serde::{Deserialize, Serialize};
use std::time::SystemTime;

/// Current cache format version. Bump when on-disk layout changes.
pub const CACHE_VERSION: u16 = 1;

/// Manifest for a raw document snapshot (parsed markdown before compose).
///
/// Created during `load_markdown()` when persistent caching is enabled.
/// Provides the foundation for validating composed document manifests.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentSnapshotManifest {
    /// Cache format version for forward compatibility.
    pub cache_version: u16,
    /// Whether the source is local or remote.
    pub source_kind: SourceKind,
    /// Canonical source identifier (absolute path or URL).
    pub canonical_source: String,
    /// xxHash of the canonical source string.
    pub source_id_hash: u64,
    /// xxHash of the raw file bytes.
    pub raw_bytes_hash: u64,
    /// xxHash of the frontmatter (canonical JSON with sorted keys).
    pub frontmatter_hash: u64,
    /// Semantic hash of the body (whitespace-normalized).
    pub body_semantic_hash: u64,
    /// Template hash of the body (more aggressive normalization).
    pub body_template_hash: u64,
    /// Last modification time of the source file.
    pub modified_at: SystemTime,
    /// Size of the source file in bytes.
    pub size_bytes: u64,
}

/// Manifest for a composed document core artifact.
///
/// The composed core is the result of recursive compose on a child document,
/// before any parent-specific transforms (exclude, releveling, wrappers).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComposedDocumentManifest {
    /// Cache format version for forward compatibility.
    pub cache_version: u16,
    /// Combined cache key for this entry.
    pub entry_key: u64,
    /// Hash of the composed document's source identifier.
    pub source_id_hash: u64,
    /// Body semantic hash from the source document snapshot used to compute this entry.
    pub source_body_semantic_hash: u64,
    /// Hash of this document's own content + options (excluding deps).
    pub self_hash: u64,
    /// Merkle-style hash: `hash(self_hash, dep1.closure_hash, dep2.closure_hash, ...)`.
    pub closure_hash: u64,
    /// Number of direct dependencies.
    pub dependency_count: usize,
    /// References to direct dependencies with their closure hashes.
    pub dependencies: Vec<DependencyRef>,
    /// xxHash of the composed content blob.
    pub payload_blob_hash: u64,
    /// xxHash of serialized warnings (for report reconstruction).
    pub warnings_hash: u64,
    /// When this artifact was created.
    pub created_at: SystemTime,
    /// When this artifact was last read from cache.
    pub last_accessed_at: SystemTime,
    /// Optional expiration time (for remote/LLM content in future phases).
    pub expires_at: Option<SystemTime>,
}

impl DocumentSnapshotManifest {
    /// Checks if the snapshot is still valid against the current file state.
    ///
    /// Compares modification time and size as a fast staleness check
    /// before more expensive content hashing.
    pub fn is_fresh(&self, current_modified: SystemTime, current_size: u64) -> bool {
        self.modified_at == current_modified && self.size_bytes == current_size
    }
}

/// Manifest for an individual operation result (code transclusion, TOC linking).
///
/// Caches the core output of a single operation before parent-specific
/// transforms (wrappers like quotation/disclosure) are applied.
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationResultManifest {
    /// Cache format version for forward compatibility.
    pub cache_version: u16,
    /// Combined cache key for this entry.
    pub entry_key: u64,
    /// Operation kind identifier (e.g., "code", "toc-linking").
    pub op_kind: String,
    /// Hash of the operation's own content + variant parameters.
    pub self_hash: u64,
    /// Closure hash for the operation result after validating its source inputs.
    pub closure_hash: u64,
    /// xxHash of the result content blob.
    pub payload_blob_hash: u64,
    /// Canonical source identifier (absolute path or URL).
    pub canonical_source: String,
    /// xxHash of the source file identifier.
    pub source_id_hash: u64,
    /// xxHash of the source content used to produce this result.
    pub source_content_hash: u64,
    /// When this artifact was created.
    pub created_at: SystemTime,
    /// When this artifact was last read from cache.
    pub last_accessed_at: SystemTime,
    /// Optional expiration time for time-bound operation artifacts.
    pub expires_at: Option<SystemTime>,
}

#[allow(dead_code)]
impl OperationResultManifest {
    /// Updates the last-accessed timestamp to now.
    pub fn touch(&mut self) {
        self.last_accessed_at = SystemTime::now();
    }

    /// Returns true if the artifact has expired.
    pub fn is_expired(&self) -> bool {
        self.expires_at
            .map(|exp| SystemTime::now() > exp)
            .unwrap_or(false)
    }
}

impl ComposedDocumentManifest {
    /// Updates the last-accessed timestamp to now.
    pub fn touch(&mut self) {
        self.last_accessed_at = SystemTime::now();
    }

    /// Returns true if the artifact has expired.
    pub fn is_expired(&self) -> bool {
        self.expires_at
            .map(|exp| SystemTime::now() > exp)
            .unwrap_or(false)
    }
}
