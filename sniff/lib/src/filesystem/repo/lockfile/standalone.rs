//! Repository-level observations of standalone Poetry, PDM, and Composer
//! lockfiles (ruling R2).
//!
//! No workspace authority covers these tools, so they are reported beside the
//! layers rather than on one. Their lockfiles record dependencies, not
//! workspace members, so they are metadata-only: never opened or parsed.

use std::collections::BTreeSet;
use std::path::Path;

use serde::{Deserialize, Serialize};
use tracing::debug;

use super::super::detection::ManifestStore;
use super::membership::manifest_member;
use super::{LockfileObservation, LockfilePresence, LockfileReason};

/// A lockfile found at the repository root or a discovered package root.
///
/// Serializes flat: `root`, `tool`, and then the same five fields as a
/// layer's `lockfile` object. `extra` and `missing` are always `[]`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StandaloneLockfileObservation {
    /// Repository-relative, `/`-separated root the file was found at; `""`
    /// for the repository root.
    pub root: String,
    /// The tool that writes the lockfile.
    pub tool: StandaloneLockfileTool,
    /// What the lockfile's metadata shows; `paths` is relative to `root`.
    #[serde(flatten)]
    pub observation: LockfileObservation,
}

/// A tool whose lockfile is observed at the repository level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StandaloneLockfileTool {
    /// `composer.lock`
    Composer,
    /// `pdm.lock`
    Pdm,
    /// `poetry.lock`
    Poetry,
}

impl StandaloneLockfileTool {
    const ALL: [Self; 3] = [Self::Composer, Self::Pdm, Self::Poetry];

    fn lockfile(self) -> &'static str {
        match self {
            Self::Composer => "composer.lock",
            Self::Pdm => "pdm.lock",
            Self::Poetry => "poetry.lock",
        }
    }
}

/// Observe the standalone lockfiles at `repo_root` and at each already
/// discovered package root.
///
/// Each root costs one cached metadata probe per tool and no walk. An absent
/// file produces no entry. A present file is `not_requested` when the
/// request declines corroboration, otherwise `unverifiable` with
/// `no_membership_data`; a failed probe is `unreadable` with
/// `metadata_failed`. Entries are unique by `(root, tool)` and sorted.
pub(crate) fn observe_standalone_lockfiles<'a>(
    repo_root: &Path,
    package_roots: impl IntoIterator<Item = &'a Path>,
    wants: bool,
    store: &ManifestStore,
) -> Vec<StandaloneLockfileObservation> {
    let mut roots = BTreeSet::from([String::new()]);
    for package_root in package_roots {
        match manifest_member(repo_root, package_root) {
            Ok(relative) => {
                roots.insert(relative);
            }
            Err(reason) => {
                debug!(root = %package_root.display(), ?reason, "package root has no repository-relative spelling");
            }
        }
    }

    let mut observations = Vec::new();
    for root in roots {
        let directory = if root.is_empty() {
            repo_root.to_path_buf()
        } else {
            repo_root.join(&root)
        };
        for tool in StandaloneLockfileTool::ALL {
            let lockfile = tool.lockfile();
            let path = directory.join(lockfile);
            let observation = match store.lockfile_presence(&path) {
                LockfilePresence::Absent => continue,
                LockfilePresence::Failed(kind) => {
                    debug!(path = %path.display(), ?kind, "lockfile metadata probe failed");
                    LockfileObservation::unreadable(Vec::new(), LockfileReason::MetadataFailed)
                }
                LockfilePresence::Present if wants => LockfileObservation::unverifiable(
                    vec![lockfile.to_owned()],
                    LockfileReason::NoMembershipData,
                ),
                LockfilePresence::Present => {
                    LockfileObservation::not_requested(vec![lockfile.to_owned()])
                }
            };
            observations.push(StandaloneLockfileObservation {
                root: root.clone(),
                tool,
                observation,
            });
        }
    }
    observations
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::super::test_seam;
    use super::*;
    use crate::performance::counters;
    use crate::performance::testing;

    fn touch(root: &Path, relative: &str) {
        let path = root.join(relative);
        fs::create_dir_all(path.parent().expect("parent")).expect("create parent");
        fs::write(path, "").expect("write file");
    }

    fn entry(
        root: &str,
        tool: StandaloneLockfileTool,
        observation: LockfileObservation,
    ) -> StandaloneLockfileObservation {
        StandaloneLockfileObservation {
            root: root.to_owned(),
            tool,
            observation,
        }
    }

    #[test]
    fn present_files_at_the_root_and_package_roots_are_reported_sorted() {
        let dir = TempDir::new().expect("tempdir");
        let root = dir.path();
        for file in [
            "poetry.lock",
            "composer.lock",
            "services/api/pdm.lock",
            "services/api/poetry.lock",
            "services/web/package.json",
            // Never probed: not the root and not a package root.
            "vendor/lib/composer.lock",
            "services/api/.venv/poetry.lock",
        ] {
            touch(root, file);
        }
        let packages = [root.join("services/api"), root.join("services/web"), root.join("services/api")];
        let store = ManifestStore::default();

        let (observations, counts) = testing::measure(|| {
            observe_standalone_lockfiles(root, packages.iter().map(|path| path.as_path()), true, &store)
        });

        let unverifiable = |lockfile: &str| {
            LockfileObservation::unverifiable(
                vec![lockfile.to_owned()],
                LockfileReason::NoMembershipData,
            )
        };
        assert_eq!(
            observations,
            [
                entry("", StandaloneLockfileTool::Composer, unverifiable("composer.lock")),
                entry("", StandaloneLockfileTool::Poetry, unverifiable("poetry.lock")),
                entry("services/api", StandaloneLockfileTool::Pdm, unverifiable("pdm.lock")),
                entry("services/api", StandaloneLockfileTool::Poetry, unverifiable("poetry.lock")),
            ]
        );
        // Three roots (the duplicate package root collapses), three tools each.
        assert_eq!(counts.get(counters::REPO_LOCKFILE_PROBES), 9);
        assert_eq!(counts.get(counters::REPO_LOCKFILE_READS), 0);
        assert_eq!(counts.get(counters::FS_READ_DIRS), 0);
    }

    #[test]
    fn a_declined_request_reports_not_requested_and_absence_reports_nothing() {
        let dir = TempDir::new().expect("tempdir");
        touch(dir.path(), "pdm.lock");
        let store = ManifestStore::default();

        let observations = observe_standalone_lockfiles(dir.path(), [], false, &store);

        assert_eq!(
            observations,
            [entry(
                "",
                StandaloneLockfileTool::Pdm,
                LockfileObservation::not_requested(vec!["pdm.lock".to_owned()])
            )]
        );

        let empty = TempDir::new().expect("tempdir");
        assert!(observe_standalone_lockfiles(empty.path(), [], true, &ManifestStore::default()).is_empty());
    }

    #[test]
    fn a_metadata_failure_is_unreadable() {
        let dir = TempDir::new().expect("tempdir");
        let _failure = test_seam::fail_metadata(
            &dir.path().join("poetry.lock"),
            std::io::ErrorKind::PermissionDenied,
        );

        let observations =
            observe_standalone_lockfiles(dir.path(), [], false, &ManifestStore::default());

        assert_eq!(
            observations,
            [entry(
                "",
                StandaloneLockfileTool::Poetry,
                LockfileObservation::unreadable(Vec::new(), LockfileReason::MetadataFailed)
            )]
        );
    }

    #[test]
    fn the_entry_serializes_flat_with_every_field() {
        let observation = entry(
            "services/api",
            StandaloneLockfileTool::Poetry,
            LockfileObservation::not_requested(vec!["poetry.lock".to_owned()]),
        );
        let json = serde_json::to_value(&observation).expect("serializes");
        assert_eq!(
            json,
            serde_json::json!({
                "root": "services/api",
                "tool": "poetry",
                "status": "not_requested",
                "paths": ["poetry.lock"],
                "reason": "request_disabled",
                "extra": [],
                "missing": [],
            })
        );
        let mut round_tripped = observation.clone();
        for _ in 0..2 {
            let text = serde_json::to_string(&round_tripped).expect("serializes");
            round_tripped = serde_json::from_str(&text).expect("deserializes");
        }
        assert_eq!(round_tripped, observation);
    }
}
