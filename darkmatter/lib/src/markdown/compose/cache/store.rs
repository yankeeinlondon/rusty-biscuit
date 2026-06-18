//! File-backed persistent cache store.
//!
//! Stores cache artifacts in a workspace-local directory structure:
//! ```text
//! .darkmatter/cache/v1/
//!   manifests/{class}/{ab}/{cd}/{hex}.json
//!   blobs/{ext}/{ab}/{cd}/{hex}.{ext}
//! ```
//!
//! Writes are atomic (tempfile + rename) to prevent corruption from crashes.
//! Cross-process safety uses filesystem-level lock files.

use super::manifest::CACHE_VERSION;
use super::types::ArtifactClass;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// File-backed persistent cache store.
///
/// All operations are synchronous and safe for concurrent access
/// from multiple threads (via the run-local cache layer) and
/// multiple processes (via filesystem atomicity).
#[derive(Debug, Clone)]
pub(crate) struct FileStore {
    /// Root directory for all cache data (e.g., `.darkmatter/cache/v1/`).
    root: PathBuf,
}

impl FileStore {
    /// Creates a new FileStore, creating the directory structure if needed.
    pub fn new(root: PathBuf) -> io::Result<Self> {
        fs::create_dir_all(root.join("manifests"))?;
        fs::create_dir_all(root.join("blobs"))?;
        Ok(Self { root })
    }

    /// Resolves the cache root for a given compose source.
    ///
    /// Prefers `<workspace>/.darkmatter/cache/v1/`. Falls back to
    /// the platform cache directory if no workspace is detected.
    pub fn resolve_cache_root(workspace_root: Option<&Path>, namespace: Option<&str>) -> PathBuf {
        let base = if let Some(root) = workspace_root {
            root.join(".darkmatter").join("cache")
        } else {
            dirs::cache_dir()
                .unwrap_or_else(|| PathBuf::from("/tmp"))
                .join("darkmatter")
                .join("cache")
        };

        let versioned = base.join(format!("v{}", CACHE_VERSION));

        match namespace {
            Some(ns) => versioned.join(ns),
            None => versioned,
        }
    }

    // ── Manifest operations ────────────────────────────────────────

    /// Reads a manifest from the store.
    pub fn read_manifest<T: DeserializeOwned>(
        &self,
        class: ArtifactClass,
        key: u64,
    ) -> io::Result<Option<T>> {
        let path = self.manifest_path(class, key);
        match fs::read_to_string(&path) {
            Ok(content) => {
                let manifest: T = serde_json::from_str(&content).map_err(|e| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("corrupt manifest at {}: {}", path.display(), e),
                    )
                })?;
                Ok(Some(manifest))
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Writes a manifest to the store atomically.
    pub fn write_manifest<T: Serialize>(
        &self,
        class: ArtifactClass,
        key: u64,
        manifest: &T,
    ) -> io::Result<()> {
        let path = self.manifest_path(class, key);
        let content = serde_json::to_string_pretty(manifest).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("failed to serialize manifest: {}", e),
            )
        })?;
        self.atomic_write(&path, content.as_bytes())
    }

    // ── Blob operations ────────────────────────────────────────────

    /// Reads a blob from the store.
    pub fn read_blob(&self, hash: u64, ext: &str) -> io::Result<Option<Vec<u8>>> {
        let path = self.blob_path(hash, ext);
        match fs::read(&path) {
            Ok(data) => Ok(Some(data)),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Writes a blob to the store atomically.
    pub fn write_blob(&self, hash: u64, ext: &str, data: &[u8]) -> io::Result<()> {
        let path = self.blob_path(hash, ext);
        self.atomic_write(&path, data)
    }

    /// Writes a full artifact (manifest + blob) atomically.
    pub fn write_artifact<T: Serialize>(
        &self,
        class: ArtifactClass,
        key: u64,
        manifest: &T,
        blob: &[u8],
        blob_hash: u64,
        blob_ext: &str,
    ) -> io::Result<()> {
        self.write_blob(blob_hash, blob_ext, blob)?;
        self.write_manifest(class, key, manifest)?;
        Ok(())
    }

    /// Checks whether a manifest exists without reading it.
    pub fn has_manifest(&self, class: ArtifactClass, key: u64) -> bool {
        self.manifest_path(class, key).exists()
    }

    // ── Path helpers ───────────────────────────────────────────────

    fn manifest_path(&self, class: ArtifactClass, key: u64) -> PathBuf {
        let hex = format!("{:016x}", key);
        let (ab, cd) = (&hex[..2], &hex[2..4]);
        let class_dir = match class {
            ArtifactClass::DocumentSnapshot => "snapshot",
            ArtifactClass::ComposeDocumentCore => "composed",
            ArtifactClass::OperationResult => "operation",
            ArtifactClass::RemoteUrl => "remote",
        };
        self.root
            .join("manifests")
            .join(class_dir)
            .join(ab)
            .join(cd)
            .join(format!("{}.json", hex))
    }

    fn blob_path(&self, hash: u64, ext: &str) -> PathBuf {
        let hex = format!("{:016x}", hash);
        let (ab, cd) = (&hex[..2], &hex[2..4]);
        self.root
            .join("blobs")
            .join(ext)
            .join(ab)
            .join(cd)
            .join(format!("{}.{}", hex, ext))
    }

    /// Atomic write: write to a tempfile in the same directory, then rename.
    ///
    /// This ensures readers never see a partially-written file. On crash,
    /// only the temp file is left behind (harmless).
    fn atomic_write(&self, target: &Path, data: &[u8]) -> io::Result<()> {
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }

        // Use tempfile in the same directory for same-filesystem rename
        let parent = target.parent().unwrap_or(Path::new("."));
        let temp = tempfile::NamedTempFile::new_in(parent)?;
        fs::write(temp.path(), data)?;

        // Persist atomically via rename
        temp.persist(target).map_err(|e| {
            io::Error::other(format!(
                "failed to persist cache file {}: {}",
                target.display(),
                e
            ))
        })?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::compose::cache::manifest::{
        ComposedDocumentManifest, DocumentSnapshotManifest,
    };
    use crate::markdown::compose::cache::types::SourceKind;
    use std::time::SystemTime;

    fn test_store() -> (tempfile::TempDir, FileStore) {
        let dir = tempfile::tempdir().unwrap();
        let store = FileStore::new(dir.path().join("cache/v1")).unwrap();
        (dir, store)
    }

    #[test]
    fn write_read_manifest_roundtrip() {
        let (_dir, store) = test_store();

        let manifest = DocumentSnapshotManifest {
            cache_version: CACHE_VERSION,
            source_kind: SourceKind::LocalFile,
            canonical_source: "/test/doc.md".to_string(),
            source_id_hash: 12345,
            raw_bytes_hash: 67890,
            frontmatter_hash: 11111,
            body_semantic_hash: 22222,
            body_template_hash: 33333,
            modified_at: SystemTime::UNIX_EPOCH,
            size_bytes: 1024,
        };

        store
            .write_manifest(ArtifactClass::DocumentSnapshot, 12345, &manifest)
            .unwrap();

        let loaded: Option<DocumentSnapshotManifest> = store
            .read_manifest(ArtifactClass::DocumentSnapshot, 12345)
            .unwrap();

        let loaded = loaded.expect("manifest should exist");
        assert_eq!(loaded.source_id_hash, 12345);
        assert_eq!(loaded.raw_bytes_hash, 67890);
        assert_eq!(loaded.canonical_source, "/test/doc.md");
    }

    #[test]
    fn write_read_blob_roundtrip() {
        let (_dir, store) = test_store();

        let data = b"# Hello World\n\nThis is cached content.";
        store.write_blob(99999, "md", data).unwrap();

        let loaded = store.read_blob(99999, "md").unwrap();
        assert_eq!(loaded.as_deref(), Some(data.as_slice()));
    }

    #[test]
    fn read_missing_manifest_returns_none() {
        let (_dir, store) = test_store();
        let result: Option<DocumentSnapshotManifest> = store
            .read_manifest(ArtifactClass::DocumentSnapshot, 99999)
            .unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn read_missing_blob_returns_none() {
        let (_dir, store) = test_store();
        let result = store.read_blob(99999, "md").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn has_manifest_check() {
        let (_dir, store) = test_store();

        assert!(!store.has_manifest(ArtifactClass::DocumentSnapshot, 12345));

        let manifest = DocumentSnapshotManifest {
            cache_version: CACHE_VERSION,
            source_kind: SourceKind::LocalFile,
            canonical_source: "/test.md".to_string(),
            source_id_hash: 12345,
            raw_bytes_hash: 0,
            frontmatter_hash: 0,
            body_semantic_hash: 0,
            body_template_hash: 0,
            modified_at: SystemTime::UNIX_EPOCH,
            size_bytes: 0,
        };
        store
            .write_manifest(ArtifactClass::DocumentSnapshot, 12345, &manifest)
            .unwrap();

        assert!(store.has_manifest(ArtifactClass::DocumentSnapshot, 12345));
    }

    #[test]
    fn write_artifact_creates_both_manifest_and_blob() {
        let (_dir, store) = test_store();

        let manifest = ComposedDocumentManifest {
            cache_version: CACHE_VERSION,
            entry_key: 55555,
            source_id_hash: 44444,
            source_body_semantic_hash: 33333,
            self_hash: 100,
            closure_hash: 200,
            dependency_count: 0,
            dependencies: vec![],
            payload_blob_hash: 77777,
            warnings_hash: 0,
            created_at: SystemTime::now(),
            last_accessed_at: SystemTime::now(),
            expires_at: None,
        };

        let blob = b"composed content here";
        store
            .write_artifact(
                ArtifactClass::ComposeDocumentCore,
                55555,
                &manifest,
                blob,
                77777,
                "md",
            )
            .unwrap();

        // Both should be readable
        let loaded_manifest: Option<ComposedDocumentManifest> = store
            .read_manifest(ArtifactClass::ComposeDocumentCore, 55555)
            .unwrap();
        assert!(loaded_manifest.is_some());
        assert_eq!(loaded_manifest.unwrap().closure_hash, 200);

        let loaded_blob = store.read_blob(77777, "md").unwrap();
        assert_eq!(loaded_blob.as_deref(), Some(blob.as_slice()));
    }

    #[test]
    fn fanout_directory_structure() {
        let (_dir, store) = test_store();

        // Key 0x0011223344556677 should create ab=00, cd=11 directories
        let key: u64 = 0x0011223344556677;
        let manifest = DocumentSnapshotManifest {
            cache_version: CACHE_VERSION,
            source_kind: SourceKind::LocalFile,
            canonical_source: "/test.md".to_string(),
            source_id_hash: key,
            raw_bytes_hash: 0,
            frontmatter_hash: 0,
            body_semantic_hash: 0,
            body_template_hash: 0,
            modified_at: SystemTime::UNIX_EPOCH,
            size_bytes: 0,
        };
        store
            .write_manifest(ArtifactClass::DocumentSnapshot, key, &manifest)
            .unwrap();

        // Verify the fanout path exists
        let expected = store
            .root
            .join("manifests/snapshot/00/11/0011223344556677.json");
        assert!(
            expected.exists(),
            "Expected fanout path: {}",
            expected.display()
        );
    }

    #[test]
    fn resolve_cache_root_with_workspace() {
        let root = FileStore::resolve_cache_root(Some(Path::new("/project")), None);
        assert_eq!(root, PathBuf::from("/project/.darkmatter/cache/v1"));
    }

    #[test]
    fn resolve_cache_root_with_namespace() {
        let root = FileStore::resolve_cache_root(Some(Path::new("/project")), Some("test-ns"));
        assert_eq!(root, PathBuf::from("/project/.darkmatter/cache/v1/test-ns"));
    }

    #[test]
    fn resolve_cache_root_without_workspace() {
        let root = FileStore::resolve_cache_root(None, None);
        // Should use platform cache dir, not workspace
        assert!(!root.starts_with(".darkmatter"));
        assert!(root.to_string_lossy().contains("darkmatter"));
    }
}
