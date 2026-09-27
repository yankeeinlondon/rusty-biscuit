//! Metadata-only sources: dependency locks that exist beside a workspace but
//! do not enumerate its members. They are probed, never opened.

use std::path::Path;

use tracing::debug;

use super::super::detection::{ManifestStore, probe_exists};
use super::{LockfileObservation, LockfilePresence, LockfileReason};
use crate::performance;
use crate::performance::counters;

/// A fallback lockfile source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FallbackSource {
    /// Go `go.work.sum`: checksums only.
    GoWorkSum,
    /// Gradle's root `gradle.lockfile` and the legacy root
    /// `gradle/dependency-locks/*.lockfile` group.
    Gradle,
    /// Bazel's `MODULE.bazel.lock`: external module resolution. It applies
    /// only where `MODULE.bazel` enables Bzlmod.
    BazelModule,
}

/// Gradle's pre-7 lock directory, relative to the root. Only its direct
/// `*.lockfile` children are reported; subproject locks are never searched.
const GRADLE_LEGACY_LOCK_DIR: &str = "gradle/dependency-locks";

impl FallbackSource {
    /// Root-relative single-file candidates, every one of which is reported
    /// when present.
    fn candidates(self) -> &'static [&'static str] {
        match self {
            Self::GoWorkSum => &["go.work.sum"],
            Self::Gradle => &["gradle.lockfile"],
            Self::BazelModule => &["MODULE.bazel.lock"],
        }
    }
}

/// Observe a fallback source from metadata alone.
///
/// A present file is `not_requested` when the request declines
/// corroboration, otherwise `unverifiable` with `no_membership_data`; its
/// contents are never inspected either way. A Bazel root without
/// `MODULE.bazel` has no lockfile source (`not_applicable`).
pub(super) fn observe(
    source: FallbackSource,
    root: &Path,
    wants: bool,
    store: &ManifestStore,
) -> LockfileObservation {
    if source == FallbackSource::BazelModule && !probe_exists(&root.join("MODULE.bazel")) {
        return LockfileObservation::not_applicable(LockfileReason::NoLockfileSource);
    }
    let mut paths = Vec::new();
    for &candidate in source.candidates() {
        let path = root.join(candidate);
        match store.lockfile_presence(&path) {
            LockfilePresence::Absent => {}
            LockfilePresence::Present => paths.push(candidate.to_owned()),
            LockfilePresence::Failed(kind) => return metadata_failed(&path, kind),
        }
    }
    if source == FallbackSource::Gradle {
        let dir = root.join(GRADLE_LEGACY_LOCK_DIR);
        match store.lockfile_presence(&dir) {
            LockfilePresence::Absent => {}
            LockfilePresence::Failed(kind) => return metadata_failed(&dir, kind),
            LockfilePresence::Present => match legacy_gradle_lockfiles(&dir) {
                Ok(names) => paths.extend(
                    names
                        .into_iter()
                        .map(|name| format!("{GRADLE_LEGACY_LOCK_DIR}/{name}")),
                ),
                Err(kind) => return metadata_failed(&dir, kind),
            },
        }
    }
    if paths.is_empty() {
        return LockfileObservation::absent();
    }
    paths.sort();
    if wants {
        LockfileObservation::unverifiable(paths, LockfileReason::NoMembershipData)
    } else {
        LockfileObservation::not_requested(paths)
    }
}

fn metadata_failed(path: &Path, kind: std::io::ErrorKind) -> LockfileObservation {
    debug!(path = %path.display(), ?kind, "lockfile metadata probe failed");
    LockfileObservation::unreadable(Vec::new(), LockfileReason::MetadataFailed)
}

/// The names of the `*.lockfile` files directly inside Gradle's legacy lock
/// directory, from one directory listing. A directory entry named
/// `*.lockfile` is not a lockfile. A path that is a file rather than a
/// directory holds no lockfiles.
fn legacy_gradle_lockfiles(dir: &Path) -> Result<Vec<String>, std::io::ErrorKind> {
    performance::increment_counter(counters::FS_READ_DIRS, 1);
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        // The error kind for listing a file differs by OS, so ask directly.
        Err(error) => {
            performance::increment_counter(counters::FS_METADATA_PROBES, 1);
            return match std::fs::metadata(dir) {
                Ok(metadata) if !metadata.is_dir() => Ok(Vec::new()),
                _ => Err(error.kind()),
            };
        }
    };
    let mut names = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|error| error.kind())?;
        let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        if !name.ends_with(".lockfile") {
            continue;
        }
        let file_type = entry.file_type().map_err(|error| error.kind())?;
        // A symlink is reported when it resolves to a file.
        let is_file = file_type.is_file()
            || (file_type.is_symlink() && {
                performance::increment_counter(counters::FS_METADATA_PROBES, 1);
                std::fs::metadata(entry.path()).is_ok_and(|metadata| metadata.is_file())
            });
        if is_file {
            names.push(name);
        }
    }
    Ok(names)
}
