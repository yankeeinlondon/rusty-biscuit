//! SHA-pair cache for worktree branch status.
//!
//! Cache entries are keyed by the deterministic pair of default-branch tip SHA
//! and worktree branch tip SHA. Tip changes self-invalidate by producing a new
//! key, while stale entries are tolerated opportunistically because they are
//! unreachable until the same pair appears again. Working-tree dirtiness is not
//! cached; it remains a live `git status` walk for each listing.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::WorktreeError;
use crate::git::{git_command, repo_info};

pub const CACHE_FORMAT_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CacheKey {
    pub default_tip_sha: String,
    pub branch_tip_sha: String,
    pub version: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CacheValue {
    pub ahead: usize,
    pub behind: usize,
    pub is_clean: bool,
}

#[derive(Debug, Default)]
pub struct Cache {
    entries: HashMap<CacheKey, CacheValue>,
}

#[derive(Serialize, Deserialize)]
struct CacheFile {
    format_version: u32,
    entries: Vec<CacheRecord>,
}

#[derive(Serialize, Deserialize)]
struct CacheRecord {
    key: CacheKey,
    value: CacheValue,
}

impl Cache {
    pub fn load_or_default() -> Self {
        let repo_root = main_worktree_path().or_else(|| repo_info().ok().map(|info| info.root));
        let Some(repo_root) = repo_root else {
            return Self::default();
        };
        let Ok(path) = cache_path(&repo_root) else {
            return Self::default();
        };
        Self::load_or_default_from(&path)
    }

    pub fn load_or_default_from(path: &Path) -> Self {
        let Ok(bytes) = fs::read(path) else {
            return Self::default();
        };
        let Ok(file) = serde_json::from_slice::<CacheFile>(&bytes) else {
            return Self::default();
        };
        if file.format_version != CACHE_FORMAT_VERSION {
            return Self::default();
        }

        let entries = file
            .entries
            .into_iter()
            .filter(|record| record.key.version == CACHE_FORMAT_VERSION)
            .map(|record| (record.key, record.value))
            .collect();
        Self { entries }
    }

    pub fn get(&self, key: &CacheKey) -> Option<&CacheValue> {
        self.entries.get(key)
    }

    pub fn put(&mut self, key: CacheKey, value: CacheValue) {
        self.entries.insert(key, value);
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn save_atomic(&self, path: &Path) -> Result<(), WorktreeError> {
        let file = CacheFile {
            format_version: CACHE_FORMAT_VERSION,
            entries: self
                .entries
                .iter()
                .map(|(key, value)| CacheRecord {
                    key: key.clone(),
                    value: *value,
                })
                .collect(),
        };
        let bytes = serde_json::to_vec_pretty(&file)?;
        atomic_write(path, &bytes)
    }
}

fn main_worktree_path() -> Option<PathBuf> {
    let output = git_command(&["worktree", "list", "--porcelain"]).ok()?;
    output
        .lines()
        .find_map(|line| line.strip_prefix("worktree "))
        .map(PathBuf::from)
}

pub fn cache_path(repo_root: &Path) -> Result<PathBuf, WorktreeError> {
    let cache_dir = dirs::cache_dir().ok_or_else(|| {
        WorktreeError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "user cache directory unavailable",
        ))
    })?;
    let canonical = fs::canonicalize(repo_root)?;
    let hash = biscuit_hash::xx_hash(&canonical.to_string_lossy());
    Ok(cache_dir.join("worktree").join(format!("{hash:016x}.json")))
}

/// Atomically replace `path` with `bytes`.
///
/// The replacement uses write-temp-then-rename, so concurrent writers have
/// last-rename-wins semantics without torn reads.
pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), WorktreeError> {
    let parent = path.parent().ok_or_else(|| {
        WorktreeError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "cache path has no parent",
        ))
    })?;
    let file_name = path.file_name().ok_or_else(|| {
        WorktreeError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "cache path has no file name",
        ))
    })?;
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let temp = parent.join(format!(
        ".{}.{}.{:?}.{nonce}.tmp",
        file_name.to_string_lossy(),
        std::process::id(),
        std::thread::current().id()
    ));
    fs::write(&temp, bytes)?;
    fs::rename(temp, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::os::unix::fs as unix_fs;
    use std::sync::Arc;

    use super::*;

    fn sample_key(version: u32) -> CacheKey {
        CacheKey {
            default_tip_sha: "1111111111111111111111111111111111111111".to_string(),
            branch_tip_sha: "2222222222222222222222222222222222222222".to_string(),
            version,
        }
    }

    fn sample_value() -> CacheValue {
        CacheValue {
            ahead: 2,
            behind: 1,
            is_clean: true,
        }
    }

    #[test]
    fn cache_round_trip_atomic() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let path = dir.path().join("cache.json");
        let key = sample_key(CACHE_FORMAT_VERSION);
        let value = sample_value();
        let mut cache = Cache::default();
        cache.put(key.clone(), value);

        cache.save_atomic(&path).expect("save cache");
        let loaded = Cache::load_or_default_from(&path);

        assert_eq!(loaded.get(&key), Some(&value));
    }

    #[test]
    fn cache_corrupt_json_returns_empty() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let path = dir.path().join("cache.json");
        fs::write(&path, b"not json").expect("write corrupt cache");

        assert!(Cache::load_or_default_from(&path).is_empty());
        assert_eq!(fs::read(&path).expect("cache file remains"), b"not json");
    }

    #[test]
    fn cache_missing_file_returns_empty() {
        let dir = tempfile::tempdir().expect("create temp dir");
        assert!(Cache::load_or_default_from(&dir.path().join("missing.json")).is_empty());
    }

    #[test]
    fn cache_wrong_version_returns_empty() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let path = dir.path().join("cache.json");
        let file = CacheFile {
            format_version: CACHE_FORMAT_VERSION + 1,
            entries: vec![CacheRecord {
                key: sample_key(CACHE_FORMAT_VERSION + 1),
                value: sample_value(),
            }],
        };
        let bytes = serde_json::to_vec(&file).expect("serialize cache");
        fs::write(&path, &bytes).expect("write wrong-version cache");

        assert!(Cache::load_or_default_from(&path).is_empty());
        assert_eq!(fs::read(&path).expect("cache file remains"), bytes);
    }

    #[test]
    fn cache_path_uses_canonical_repo_root() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let repo = dir.path().join("repo");
        fs::create_dir(&repo).expect("create repo dir");
        let link = dir.path().join("repo-link");
        unix_fs::symlink(&repo, &link).expect("create symlink");

        assert_eq!(
            cache_path(&repo).expect("cache path for repo"),
            cache_path(&link).expect("cache path for symlink")
        );
    }

    #[test]
    fn atomic_write_concurrent_writers_last_rename_wins() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let path = Arc::new(dir.path().join("cache.json"));
        let first = {
            let path = Arc::clone(&path);
            std::thread::spawn(move || atomic_write(&path, b"first").expect("first write"))
        };
        let second = {
            let path = Arc::clone(&path);
            std::thread::spawn(move || atomic_write(&path, b"second").expect("second write"))
        };

        first.join().expect("first thread");
        second.join().expect("second thread");

        let bytes = fs::read(&*path).expect("read final file");
        assert!(
            bytes == b"first" || bytes == b"second",
            "expected one complete writer, got {bytes:?}"
        );
    }
}
