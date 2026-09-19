//! File-backed persistent cache store.
//!
//! Stores cache artifacts in a workspace-local directory structure:
//! ```text
//! .darkmatter/cache/v{STORE_LAYOUT_VERSION}/
//!   manifests/{class}/{ab}/{cd}/{hex}.json
//!   blobs/{ext}/{ab}/{cd}/{hex}.{ext}
//! ```
//!
//! Writes are atomic (tempfile + rename) to prevent corruption from crashes;
//! concurrent processes rely on that rename, not on lock files.
//!
//! Configuring a store mutates nothing: directories are created only by the
//! first write that needs them, so a run that never writes an artifact leaves
//! the cache root exactly as it found it (missing roots stay missing).

use super::manifest::STORE_LAYOUT_VERSION;
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
    /// Records `root` without touching the filesystem.
    pub fn at(root: PathBuf) -> Self {
        Self { root }
    }

    /// Resolves the store root `<cache_root>/.darkmatter/cache/v{STORE_LAYOUT_VERSION}[/<namespace>]`.
    ///
    /// There is deliberately no platform-cache fallback: persistence happens
    /// only under an explicitly configured cache root.
    pub fn resolve_cache_root(cache_root: &Path, namespace: Option<&str>) -> PathBuf {
        let versioned = cache_root
            .join(".darkmatter")
            .join("cache")
            .join(format!("v{}", STORE_LAYOUT_VERSION));

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

    /// Removes a manifest; a missing manifest is success.
    pub fn remove_manifest(&self, class: ArtifactClass, key: u64) -> io::Result<()> {
        remove_if_present(&self.manifest_path(class, key))
    }

    /// Removes a blob; a missing blob is success.
    pub fn remove_blob(&self, hash: u64, ext: &str) -> io::Result<()> {
        remove_if_present(&self.blob_path(hash, ext))
    }

    // ── Path helpers ───────────────────────────────────────────────

    pub(super) fn manifest_path(&self, class: ArtifactClass, key: u64) -> PathBuf {
        let hex = format!("{:016x}", key);
        let (ab, cd) = (&hex[..2], &hex[2..4]);
        let class_dir = match class {
            ArtifactClass::RemoteUrl => "remote",
        };
        self.root
            .join("manifests")
            .join(class_dir)
            .join(ab)
            .join(cd)
            .join(format!("{}.json", hex))
    }

    pub(super) fn blob_path(&self, hash: u64, ext: &str) -> PathBuf {
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
    /// only the temp file is left behind (harmless). This is the store's only
    /// directory-creation point, so it is where an unusable root first fails.
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

fn remove_if_present(path: &Path) -> io::Result<()> {
    match fs::remove_file(path) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        result => result,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::compose::cache::manifest::{CACHE_VERSION, RemoteUrlManifest};
    use std::time::SystemTime;

    fn remote_manifest(source_id_hash: u64, body_blob_hash: u64) -> RemoteUrlManifest {
        RemoteUrlManifest {
            cache_version: CACHE_VERSION,
            redacted_url: "https://example.com/doc.md".to_string(),
            source_id_hash,
            status: 200,
            etag: None,
            last_modified: None,
            cache_control: None,
            fetched_at: SystemTime::UNIX_EPOCH,
            expires_at: None,
            content_hash: body_blob_hash,
            body_blob_hash,
            size_bytes: 0,
        }
    }

    fn test_store() -> (tempfile::TempDir, FileStore) {
        let dir = tempfile::tempdir().unwrap();
        let store = FileStore::at(dir.path().join("cache/v1"));
        (dir, store)
    }

    #[test]
    fn at_and_reads_leave_a_missing_root_missing() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("cache/v1");
        let store = FileStore::at(root.clone());

        assert!(store.read_blob(1, "md").unwrap().is_none());
        let manifest: Option<RemoteUrlManifest> =
            store.read_manifest(ArtifactClass::RemoteUrl, 1).unwrap();
        assert!(manifest.is_none());
        assert!(!store.manifest_path(ArtifactClass::RemoteUrl, 1).exists());

        assert!(!dir.path().join("cache").exists(), "construction or reads created the root");
    }

    #[test]
    fn first_blob_write_creates_only_the_blob_fanout() {
        let (dir, store) = test_store();
        let root = dir.path().join("cache/v1");
        let key: u64 = 0x0011223344556677;

        store.write_blob(key, "md", b"body").unwrap();

        assert!(root.join("blobs/md/00/11/0011223344556677.md").is_file());
        assert!(!root.join("manifests").exists(), "a blob write must not create manifests/");
        assert_eq!(store.read_blob(key, "md").unwrap().as_deref(), Some(b"body".as_slice()));
    }

    #[test]
    fn remove_deletes_written_files_and_tolerates_missing_ones() {
        let (dir, store) = test_store();
        let root = dir.path().join("cache/v1");
        let manifest = serde_json::json!({ "k": 1 });
        store
            .write_artifact(ArtifactClass::RemoteUrl, 7, &manifest, b"body", 9, "remote")
            .unwrap();

        store.remove_manifest(ArtifactClass::RemoteUrl, 7).unwrap();
        store.remove_blob(9, "remote").unwrap();
        assert!(!store.manifest_path(ArtifactClass::RemoteUrl, 7).exists());
        assert!(store.read_blob(9, "remote").unwrap().is_none());

        // Removing again, or from a root that never existed, is success.
        store.remove_manifest(ArtifactClass::RemoteUrl, 7).unwrap();
        store.remove_blob(9, "remote").unwrap();
        let missing = FileStore::at(dir.path().join("missing"));
        missing.remove_manifest(ArtifactClass::RemoteUrl, 7).unwrap();
        missing.remove_blob(9, "remote").unwrap();
        assert!(!dir.path().join("missing").exists());
        assert!(root.join("manifests").is_dir(), "removal must not prune directories");
    }

    #[test]
    fn write_under_a_file_root_fails_without_touching_the_file() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("not-a-directory");
        fs::write(&file, "plain").unwrap();
        let store = FileStore::at(file.join("cache/v1"));

        assert!(store.write_blob(1, "md", b"body").is_err());
        assert_eq!(fs::read_to_string(&file).unwrap(), "plain");
    }

    #[test]
    fn write_read_manifest_roundtrip() {
        let (_dir, store) = test_store();

        let manifest = remote_manifest(12345, 0);

        store
            .write_manifest(ArtifactClass::RemoteUrl, 12345, &manifest)
            .unwrap();

        let loaded: Option<RemoteUrlManifest> = store
            .read_manifest(ArtifactClass::RemoteUrl, 12345)
            .unwrap();

        let loaded = loaded.expect("manifest should exist");
        assert_eq!(loaded.source_id_hash, 12345);
        assert_eq!(loaded.redacted_url, "https://example.com/doc.md");
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
        let result: Option<RemoteUrlManifest> = store
            .read_manifest(ArtifactClass::RemoteUrl, 99999)
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
    fn write_artifact_creates_both_manifest_and_blob() {
        let (_dir, store) = test_store();

        let manifest = remote_manifest(44444, 77777);

        let blob = b"remote body here";
        store
            .write_artifact(
                ArtifactClass::RemoteUrl,
                55555,
                &manifest,
                blob,
                77777,
                "md",
            )
            .unwrap();

        // Both should be readable
        let loaded_manifest: Option<RemoteUrlManifest> = store
            .read_manifest(ArtifactClass::RemoteUrl, 55555)
            .unwrap();
        assert!(loaded_manifest.is_some());
        assert_eq!(loaded_manifest.unwrap().body_blob_hash, 77777);

        let loaded_blob = store.read_blob(77777, "md").unwrap();
        assert_eq!(loaded_blob.as_deref(), Some(blob.as_slice()));
    }

    #[test]
    fn fanout_directory_structure() {
        let (_dir, store) = test_store();

        // Key 0x0011223344556677 should create ab=00, cd=11 directories
        let key: u64 = 0x0011223344556677;
        let manifest = remote_manifest(key, 0);
        store
            .write_manifest(ArtifactClass::RemoteUrl, key, &manifest)
            .unwrap();

        // Verify the fanout path exists
        let expected = store
            .root
            .join("manifests/remote/00/11/0011223344556677.json");
        assert!(
            expected.exists(),
            "Expected fanout path: {}",
            expected.display()
        );
    }

    #[test]
    fn resolve_cache_root_with_workspace() {
        let root = FileStore::resolve_cache_root(Path::new("/project"), None);
        assert_eq!(root, PathBuf::from("/project/.darkmatter/cache/v1"));
    }

    #[test]
    fn resolve_cache_root_with_namespace() {
        let root = FileStore::resolve_cache_root(Path::new("/project"), Some("test-ns"));
        assert_eq!(root, PathBuf::from("/project/.darkmatter/cache/v1/test-ns"));
    }
}
