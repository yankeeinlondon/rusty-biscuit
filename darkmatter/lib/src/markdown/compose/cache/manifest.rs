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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::compose::cache::types::ArtifactClass;
    use std::time::Duration;

    #[test]
    fn document_snapshot_is_fresh_when_unchanged() {
        let modified_time = SystemTime::now();
        let size = 1024;

        let manifest = DocumentSnapshotManifest {
            cache_version: CACHE_VERSION,
            source_kind: SourceKind::LocalFile,
            canonical_source: "/path/to/file.md".to_string(),
            source_id_hash: 12345,
            raw_bytes_hash: 67890,
            frontmatter_hash: 11111,
            body_semantic_hash: 22222,
            body_template_hash: 33333,
            modified_at: modified_time,
            size_bytes: size,
        };

        assert!(manifest.is_fresh(modified_time, size));
    }

    #[test]
    fn document_snapshot_stale_when_modified_time_changes() {
        let original_time = SystemTime::now();
        let later_time = original_time + Duration::from_secs(60);
        let size = 1024;

        let manifest = DocumentSnapshotManifest {
            cache_version: CACHE_VERSION,
            source_kind: SourceKind::LocalFile,
            canonical_source: "/path/to/file.md".to_string(),
            source_id_hash: 12345,
            raw_bytes_hash: 67890,
            frontmatter_hash: 11111,
            body_semantic_hash: 22222,
            body_template_hash: 33333,
            modified_at: original_time,
            size_bytes: size,
        };

        assert!(!manifest.is_fresh(later_time, size));
    }

    #[test]
    fn document_snapshot_stale_when_size_changes() {
        let modified_time = SystemTime::now();
        let original_size = 1024;
        let new_size = 2048;

        let manifest = DocumentSnapshotManifest {
            cache_version: CACHE_VERSION,
            source_kind: SourceKind::LocalFile,
            canonical_source: "/path/to/file.md".to_string(),
            source_id_hash: 12345,
            raw_bytes_hash: 67890,
            frontmatter_hash: 11111,
            body_semantic_hash: 22222,
            body_template_hash: 33333,
            modified_at: modified_time,
            size_bytes: original_size,
        };

        assert!(!manifest.is_fresh(modified_time, new_size));
    }

    #[test]
    fn document_snapshot_stale_when_both_change() {
        let original_time = SystemTime::now();
        let later_time = original_time + Duration::from_secs(60);
        let original_size = 1024;
        let new_size = 2048;

        let manifest = DocumentSnapshotManifest {
            cache_version: CACHE_VERSION,
            source_kind: SourceKind::LocalFile,
            canonical_source: "/path/to/file.md".to_string(),
            source_id_hash: 12345,
            raw_bytes_hash: 67890,
            frontmatter_hash: 11111,
            body_semantic_hash: 22222,
            body_template_hash: 33333,
            modified_at: original_time,
            size_bytes: original_size,
        };

        assert!(!manifest.is_fresh(later_time, new_size));
    }

    #[test]
    fn composed_document_touch_updates_timestamp() {
        let initial_time = SystemTime::now() - Duration::from_secs(3600);

        let mut manifest = ComposedDocumentManifest {
            cache_version: CACHE_VERSION,
            entry_key: 12345,
            source_id_hash: 67890,
            source_body_semantic_hash: 11111,
            self_hash: 22222,
            closure_hash: 33333,
            dependency_count: 2,
            dependencies: vec![],
            payload_blob_hash: 44444,
            warnings_hash: 55555,
            created_at: initial_time,
            last_accessed_at: initial_time,
            expires_at: None,
        };

        let before_touch = manifest.last_accessed_at;
        manifest.touch();
        let after_touch = manifest.last_accessed_at;

        assert!(after_touch > before_touch);
    }

    #[test]
    fn composed_document_not_expired_without_expiration() {
        let manifest = ComposedDocumentManifest {
            cache_version: CACHE_VERSION,
            entry_key: 12345,
            source_id_hash: 67890,
            source_body_semantic_hash: 11111,
            self_hash: 22222,
            closure_hash: 33333,
            dependency_count: 0,
            dependencies: vec![],
            payload_blob_hash: 44444,
            warnings_hash: 55555,
            created_at: SystemTime::now(),
            last_accessed_at: SystemTime::now(),
            expires_at: None,
        };

        assert!(!manifest.is_expired());
    }

    #[test]
    fn composed_document_not_expired_when_future_expiration() {
        let future_time = SystemTime::now() + Duration::from_secs(3600);

        let manifest = ComposedDocumentManifest {
            cache_version: CACHE_VERSION,
            entry_key: 12345,
            source_id_hash: 67890,
            source_body_semantic_hash: 11111,
            self_hash: 22222,
            closure_hash: 33333,
            dependency_count: 0,
            dependencies: vec![],
            payload_blob_hash: 44444,
            warnings_hash: 55555,
            created_at: SystemTime::now(),
            last_accessed_at: SystemTime::now(),
            expires_at: Some(future_time),
        };

        assert!(!manifest.is_expired());
    }

    #[test]
    fn composed_document_expired_when_past_expiration() {
        let past_time = SystemTime::now() - Duration::from_secs(3600);

        let manifest = ComposedDocumentManifest {
            cache_version: CACHE_VERSION,
            entry_key: 12345,
            source_id_hash: 67890,
            source_body_semantic_hash: 11111,
            self_hash: 22222,
            closure_hash: 33333,
            dependency_count: 0,
            dependencies: vec![],
            payload_blob_hash: 44444,
            warnings_hash: 55555,
            created_at: past_time,
            last_accessed_at: past_time,
            expires_at: Some(past_time),
        };

        assert!(manifest.is_expired());
    }

    #[test]
    fn operation_result_touch_updates_timestamp() {
        let initial_time = SystemTime::now() - Duration::from_secs(3600);

        let mut manifest = OperationResultManifest {
            cache_version: CACHE_VERSION,
            entry_key: 12345,
            op_kind: "code".to_string(),
            self_hash: 22222,
            closure_hash: 33333,
            payload_blob_hash: 44444,
            canonical_source: "/path/to/source.rs".to_string(),
            source_id_hash: 67890,
            source_content_hash: 11111,
            created_at: initial_time,
            last_accessed_at: initial_time,
            expires_at: None,
        };

        let before_touch = manifest.last_accessed_at;
        manifest.touch();
        let after_touch = manifest.last_accessed_at;

        assert!(after_touch > before_touch);
    }

    #[test]
    fn operation_result_not_expired_without_expiration() {
        let manifest = OperationResultManifest {
            cache_version: CACHE_VERSION,
            entry_key: 12345,
            op_kind: "toc-linking".to_string(),
            self_hash: 22222,
            closure_hash: 33333,
            payload_blob_hash: 44444,
            canonical_source: "/path/to/source.md".to_string(),
            source_id_hash: 67890,
            source_content_hash: 11111,
            created_at: SystemTime::now(),
            last_accessed_at: SystemTime::now(),
            expires_at: None,
        };

        assert!(!manifest.is_expired());
    }

    #[test]
    fn operation_result_not_expired_when_future_expiration() {
        let future_time = SystemTime::now() + Duration::from_secs(3600);

        let manifest = OperationResultManifest {
            cache_version: CACHE_VERSION,
            entry_key: 12345,
            op_kind: "code".to_string(),
            self_hash: 22222,
            closure_hash: 33333,
            payload_blob_hash: 44444,
            canonical_source: "/path/to/source.rs".to_string(),
            source_id_hash: 67890,
            source_content_hash: 11111,
            created_at: SystemTime::now(),
            last_accessed_at: SystemTime::now(),
            expires_at: Some(future_time),
        };

        assert!(!manifest.is_expired());
    }

    #[test]
    fn operation_result_expired_when_past_expiration() {
        let past_time = SystemTime::now() - Duration::from_secs(3600);

        let manifest = OperationResultManifest {
            cache_version: CACHE_VERSION,
            entry_key: 12345,
            op_kind: "code".to_string(),
            self_hash: 22222,
            closure_hash: 33333,
            payload_blob_hash: 44444,
            canonical_source: "/path/to/source.rs".to_string(),
            source_id_hash: 67890,
            source_content_hash: 11111,
            created_at: past_time,
            last_accessed_at: past_time,
            expires_at: Some(past_time),
        };

        assert!(manifest.is_expired());
    }

    #[test]
    fn different_source_kinds_supported() {
        let modified_time = SystemTime::now();

        let local_manifest = DocumentSnapshotManifest {
            cache_version: CACHE_VERSION,
            source_kind: SourceKind::LocalFile,
            canonical_source: "/path/to/file.md".to_string(),
            source_id_hash: 12345,
            raw_bytes_hash: 67890,
            frontmatter_hash: 11111,
            body_semantic_hash: 22222,
            body_template_hash: 33333,
            modified_at: modified_time,
            size_bytes: 1024,
        };

        let remote_manifest = DocumentSnapshotManifest {
            cache_version: CACHE_VERSION,
            source_kind: SourceKind::RemoteUrl,
            canonical_source: "https://example.com/doc.md".to_string(),
            source_id_hash: 54321,
            raw_bytes_hash: 98765,
            frontmatter_hash: 44444,
            body_semantic_hash: 55555,
            body_template_hash: 66666,
            modified_at: modified_time,
            size_bytes: 2048,
        };

        assert!(local_manifest.is_fresh(modified_time, 1024));
        assert!(remote_manifest.is_fresh(modified_time, 2048));
    }

    #[test]
    fn composed_document_with_dependencies() {
        let dep1 = DependencyRef {
            artifact_class: ArtifactClass::DocumentSnapshot,
            entry_key: 111,
            source_id_hash: 222,
            closure_hash: 333,
        };

        let dep2 = DependencyRef {
            artifact_class: ArtifactClass::ComposeDocumentCore,
            entry_key: 444,
            source_id_hash: 555,
            closure_hash: 666,
        };

        let manifest = ComposedDocumentManifest {
            cache_version: CACHE_VERSION,
            entry_key: 12345,
            source_id_hash: 67890,
            source_body_semantic_hash: 11111,
            self_hash: 22222,
            closure_hash: 33333,
            dependency_count: 2,
            dependencies: vec![dep1, dep2],
            payload_blob_hash: 44444,
            warnings_hash: 55555,
            created_at: SystemTime::now(),
            last_accessed_at: SystemTime::now(),
            expires_at: None,
        };

        assert_eq!(manifest.dependency_count, 2);
        assert_eq!(manifest.dependencies.len(), 2);
        assert!(!manifest.is_expired());
    }
}
