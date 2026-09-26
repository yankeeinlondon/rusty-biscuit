//! Fork-origin records: the local branch each `wt create` branch was forked
//! from, keyed by the new branch's name.
//!
//! Persisted per repository beside the comparison cache as
//! `<repo hash>.fork-origins.json` (see [`crate::cache::repo_cache_file`]).
//! `wt create` writes a record only when it creates a new branch; reusing an
//! existing branch records nothing. Records for deleted branches are removed
//! with [`ForkOriginStore::prune`]; a record whose *parent* was deleted is
//! kept, because the listing reports that parent as deleted.

use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::cache::{atomic_write, repo_cache_file};
use crate::error::WorktreeError;

pub const FORK_ORIGIN_FORMAT_VERSION: u32 = 1;

/// Where a branch was forked from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForkOrigin {
    /// The local branch the new branch started from.
    pub base_branch: String,
    /// The base branch's tip when the fork was made.
    pub base_sha: String,
    /// Seconds since the Unix epoch.
    pub created_at: u64,
}

/// Every fork-origin record of one repository.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ForkOriginStore {
    records: BTreeMap<String, ForkOrigin>,
}

#[derive(Serialize, Deserialize)]
struct StoreFile {
    format_version: u32,
    records: BTreeMap<String, ForkOrigin>,
}

#[allow(missing_docs)]
impl ForkOriginStore {
    /// Loads the store at `path`. A missing, unreadable, or other-version
    /// file loads as empty, so a stale format never blocks `wt`.
    pub fn load_from(path: &Path) -> Self {
        let Ok(bytes) = fs::read(path) else {
            return Self::default();
        };
        match serde_json::from_slice::<StoreFile>(&bytes) {
            Ok(file) if file.format_version == FORK_ORIGIN_FORMAT_VERSION => Self {
                records: file.records,
            },
            _ => Self::default(),
        }
    }

    pub fn save_atomic(&self, path: &Path) -> Result<(), WorktreeError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let file = StoreFile {
            format_version: FORK_ORIGIN_FORMAT_VERSION,
            records: self.records.clone(),
        };
        atomic_write(path, &serde_json::to_vec_pretty(&file)?)
    }

    pub fn get(&self, branch: &str) -> Option<&ForkOrigin> {
        self.records.get(branch)
    }

    pub fn insert(&mut self, branch: impl Into<String>, origin: ForkOrigin) {
        self.records.insert(branch.into(), origin);
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// Removes the records of branches not in `live_branches` and returns how
    /// many were removed.
    pub fn prune(&mut self, live_branches: &HashSet<String>) -> usize {
        let before = self.records.len();
        self.records
            .retain(|branch, _| live_branches.contains(branch));
        before - self.records.len()
    }
}

/// The store file for the repository whose main worktree is `repo_root`.
pub fn fork_origin_path(repo_root: &Path) -> Result<PathBuf, WorktreeError> {
    repo_cache_file(repo_root, "fork-origins.json")
}

/// Adds or replaces the record for `branch` in the store at `path`.
///
/// Read-modify-write without a lock: two concurrent `wt create` runs can lose
/// one record (last rename wins), which only flattens that branch in the
/// listing's tree.
pub fn record(path: &Path, branch: &str, origin: ForkOrigin) -> Result<(), WorktreeError> {
    let mut store = ForkOriginStore::load_from(path);
    store.insert(branch, origin);
    store.save_atomic(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn origin(base: &str) -> ForkOrigin {
        ForkOrigin {
            base_branch: base.to_string(),
            base_sha: "1111111111111111111111111111111111111111".to_string(),
            created_at: 1_790_000_000,
        }
    }

    #[test]
    fn repeated_round_trip_preserves_records() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("nested").join("repo.fork-origins.json");

        record(&path, "feat/dark-fixes", origin("feat/theme")).expect("first record");
        record(&path, "fix/x", origin("main")).expect("second record");
        let first = ForkOriginStore::load_from(&path);
        first.save_atomic(&path).expect("re-save");
        let second = ForkOriginStore::load_from(&path);

        assert_eq!(first, second);
        assert_eq!(second.len(), 2);
        assert_eq!(second.get("feat/dark-fixes"), Some(&origin("feat/theme")));
        assert_eq!(second.get("fix/x"), Some(&origin("main")));
    }

    #[test]
    fn record_replaces_an_existing_entry() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("store.json");
        record(&path, "fix/x", origin("main")).unwrap();
        record(&path, "fix/x", origin("feat/theme")).unwrap();

        let store = ForkOriginStore::load_from(&path);
        assert_eq!(store.len(), 1);
        assert_eq!(store.get("fix/x").unwrap().base_branch, "feat/theme");
    }

    #[test]
    fn missing_corrupt_and_other_version_files_load_empty() {
        let dir = tempfile::tempdir().expect("temp dir");
        assert!(ForkOriginStore::load_from(&dir.path().join("missing.json")).is_empty());

        let corrupt = dir.path().join("corrupt.json");
        fs::write(&corrupt, b"{not json").unwrap();
        assert!(ForkOriginStore::load_from(&corrupt).is_empty());

        let other = dir.path().join("other.json");
        let file = StoreFile {
            format_version: FORK_ORIGIN_FORMAT_VERSION + 1,
            records: BTreeMap::from([("fix/x".to_string(), origin("main"))]),
        };
        fs::write(&other, serde_json::to_vec(&file).unwrap()).unwrap();
        assert!(ForkOriginStore::load_from(&other).is_empty());
    }

    #[test]
    fn prune_drops_deleted_branches_and_keeps_orphaned_children() {
        let mut store = ForkOriginStore::default();
        store.insert("feat/theme", origin("main"));
        store.insert("feat/dark-fixes", origin("feat/theme"));
        store.insert("spike/parser", origin("experiments"));

        let live: HashSet<String> = ["feat/dark-fixes", "spike/parser", "main"]
            .into_iter()
            .map(String::from)
            .collect();
        assert_eq!(store.prune(&live), 1);

        assert!(store.get("feat/theme").is_none());
        // The parent `experiments` is gone, but the child's record stays so the
        // listing can say its parent was deleted.
        assert_eq!(store.get("spike/parser"), Some(&origin("experiments")));
        assert_eq!(store.get("feat/dark-fixes"), Some(&origin("feat/theme")));
    }

    #[test]
    fn store_file_sits_beside_the_comparison_cache() {
        let dir = tempfile::tempdir().expect("temp dir");
        let cache = crate::cache::cache_path(dir.path()).unwrap();
        let store = fork_origin_path(dir.path()).unwrap();

        assert_eq!(cache.parent(), store.parent());
        let cache_stem = cache.file_stem().unwrap().to_string_lossy().into_owned();
        assert_eq!(
            store.file_name().unwrap().to_string_lossy(),
            format!("{cache_stem}.fork-origins.json")
        );
    }
}
