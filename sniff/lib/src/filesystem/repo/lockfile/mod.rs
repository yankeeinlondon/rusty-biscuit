//! Per-layer lockfile corroboration.
//!
//! The manifest stays the membership authority. [`observe_layer_lockfile`]
//! selects the layer authority's lockfile from the [`sources`] table, probes
//! and reads it through the request's [`ManifestStore`], and reports what the
//! file proves about the manifest-derived member set. Reading a lockfile never
//! adds or removes packages. The contract (statuses, reasons, precedence) is
//! the `2026-09-26-lockfile-corroboration` spec and its owner rulings R1–R11.

mod bun;
pub(crate) mod cargo;
mod fallback;
mod membership;
mod npm;
mod pnpm;
mod rush;
pub(crate) mod sources;
mod standalone;
mod uv;
mod yarn;

use std::path::Path;

use serde::{Deserialize, Serialize};
use tracing::debug;

use super::detection::ManifestStore;
use super::seed::PackageSeed;
use super::standard::MonorepoLayer;
use crate::request::RepoRequest;

use sources::Source;

pub(crate) use standalone::observe_standalone_lockfiles;
pub use standalone::{StandaloneLockfileObservation, StandaloneLockfileTool};

/// What a layer's lockfile says about the layer's manifest-derived members.
///
/// Every field is always serialized. Paths are `/`-separated and relative to
/// the layer root, which is excluded from both member sets.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LockfileObservation {
    /// The outcome of corroboration.
    pub status: LockfileStatus,
    /// Sorted, unique lockfile paths known to exist; `[]` when none is.
    pub paths: Vec<String>,
    /// Why the status was reached; `null` for `match`, `mismatch`, and
    /// `absent`.
    pub reason: Option<LockfileReason>,
    /// Sorted member paths only the lockfile records; `[]` unless `mismatch`.
    pub extra: Vec<String>,
    /// Sorted member paths only the manifest declares; `[]` unless `mismatch`
    /// or `members_missing`.
    pub missing: Vec<String>,
}

/// The outcome of one layer's lockfile corroboration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LockfileStatus {
    /// A complete lockfile member set equals the manifest-derived set.
    Match,
    /// A complete lockfile member set differs from the manifest-derived set.
    Mismatch,
    /// Cargo only: every manifest member has a matching source-less
    /// `Cargo.lock` entry. Set equality is not claimed.
    MembersPresent,
    /// Cargo only: at least one manifest member has no matching source-less
    /// `Cargo.lock` entry.
    MembersMissing,
    /// The file or configuration cannot establish a complete member set.
    Unverifiable,
    /// A metadata probe, read, or supported-format parse failed.
    Unreadable,
    /// Every applicable candidate was probed and is missing.
    Absent,
    /// The authority has no applicable lockfile source.
    NotApplicable,
    /// A candidate exists, but the request disabled corroboration.
    NotRequested,
}

/// Stable, machine-readable reason behind a [`LockfileStatus`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LockfileReason {
    /// With [`LockfileStatus::NotRequested`].
    RequestDisabled,
    /// With [`LockfileStatus::NotApplicable`]: the authority writes no
    /// membership lockfile.
    NoLockfileSource,
    /// With [`LockfileStatus::NotApplicable`]: no known membership authority.
    UnknownStandard,
    /// With [`LockfileStatus::Unverifiable`].
    UnsupportedVersion,
    /// With [`LockfileStatus::Unverifiable`].
    UnsupportedLayout,
    /// With [`LockfileStatus::Unverifiable`]: fallback formats and binary
    /// lockfiles record no workspace membership.
    NoMembershipData,
    /// With [`LockfileStatus::Unverifiable`]: a member name maps to no local
    /// package path, or to more than one.
    AmbiguousMembership,
    /// With [`LockfileStatus::Unverifiable`]: the manifest-derived member set
    /// is not known to be complete.
    IncompleteManifestDiscovery,
    /// With [`LockfileStatus::Unverifiable`]: an absolute or unrepresentable
    /// member path.
    InvalidMemberPath,
    /// With [`LockfileStatus::Unreadable`].
    MetadataFailed,
    /// With [`LockfileStatus::Unreadable`], including a directory where a file
    /// is expected and a file that vanished between probe and read.
    ReadFailed,
    /// With [`LockfileStatus::Unreadable`]: invalid syntax or invalid required
    /// membership fields.
    ParseFailed,
    /// With [`LockfileStatus::MembersPresent`] and
    /// [`LockfileStatus::MembersMissing`], so neither reads as equality.
    SubsetOnly,
}

impl LockfileObservation {
    /// An observation for an authority with no applicable lockfile source.
    pub fn not_applicable(reason: LockfileReason) -> Self {
        Self::new(LockfileStatus::NotApplicable, Vec::new(), Some(reason))
    }

    fn new(status: LockfileStatus, paths: Vec<String>, reason: Option<LockfileReason>) -> Self {
        Self {
            status,
            paths,
            reason,
            extra: Vec::new(),
            missing: Vec::new(),
        }
    }

    fn absent() -> Self {
        Self::new(LockfileStatus::Absent, Vec::new(), None)
    }

    fn not_requested(paths: Vec<String>) -> Self {
        Self::new(
            LockfileStatus::NotRequested,
            paths,
            Some(LockfileReason::RequestDisabled),
        )
    }

    fn unverifiable(paths: Vec<String>, reason: LockfileReason) -> Self {
        Self::new(LockfileStatus::Unverifiable, paths, Some(reason))
    }

    fn unreadable(paths: Vec<String>, reason: LockfileReason) -> Self {
        Self::new(LockfileStatus::Unreadable, paths, Some(reason))
    }
}

/// Whether one lockfile path exists, as a cached metadata probe saw it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LockfilePresence {
    /// Metadata succeeded. A directory is present, so its read fails later.
    Present,
    Absent,
    Failed(std::io::ErrorKind),
}

impl LockfilePresence {
    pub(crate) fn is_present(self) -> bool {
        self == Self::Present
    }
}

/// What a parser recovered from one lockfile, cached per request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ParsedLockfile {
    /// Every workspace member path the lockfile records, as written and
    /// relative to the lockfile's directory. The root may be included.
    Members(Vec<String>),
    /// `Cargo.lock` entries without `source`, as `(name, version)`.
    CargoPackages(Vec<(String, String)>),
    /// A member name maps to no local package path, or to more than one.
    AmbiguousMembership,
    /// A recognized version or signature outside the accepted matrix.
    UnsupportedVersion,
}

/// A parser's answer: `Err` carries parse-failure detail for debug logs only.
pub(crate) type Outcome = std::result::Result<ParsedLockfile, String>;

/// Why a cached lockfile outcome has no parsed value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum LockfileFailure {
    Absent,
    Metadata(std::io::ErrorKind),
    Read(std::io::ErrorKind),
    Parse(String),
}

impl LockfileFailure {
    fn observation(&self, paths: Vec<String>) -> LockfileObservation {
        match self {
            Self::Absent => LockfileObservation::absent(),
            Self::Metadata(_) => {
                LockfileObservation::unreadable(Vec::new(), LockfileReason::MetadataFailed)
            }
            Self::Read(_) => LockfileObservation::unreadable(paths, LockfileReason::ReadFailed),
            Self::Parse(_) => LockfileObservation::unreadable(paths, LockfileReason::ParseFailed),
        }
    }
}

/// Probe one lockfile path's metadata, following symlinks.
///
/// A missing path and a missing parent directory are both absence, as they
/// were for `Path::exists`; any other error is a failure the caller must not
/// read as absence.
pub(crate) fn probe_presence(path: &Path) -> LockfilePresence {
    #[cfg(test)]
    if let Some(kind) = test_seam::injected_failure(path) {
        return LockfilePresence::Failed(kind);
    }
    match std::fs::metadata(path) {
        Ok(_) => LockfilePresence::Present,
        Err(error)
            if matches!(
                error.kind(),
                std::io::ErrorKind::NotFound | std::io::ErrorKind::NotADirectory
            ) =>
        {
            LockfilePresence::Absent
        }
        Err(error) => LockfilePresence::Failed(error.kind()),
    }
}

/// Observe `layer`'s lockfile under `request`.
///
/// `owned` is the seed list of the detector outcome that built the layer, or
/// `None` when no outcome matched, which makes the manifest-side set
/// incomplete. Presence is always probed; contents are read only when the
/// request wants lockfile provenance. The steps follow the spec's precedence:
/// no source, metadata failure, absence, disabled request, metadata-only
/// fallback, then parse.
pub(crate) fn observe_layer_lockfile(
    layer: &MonorepoLayer,
    owned: Option<&[PackageSeed]>,
    request: &RepoRequest,
    store: &ManifestStore,
) -> LockfileObservation {
    let wants = request.wants_lockfile_provenance();
    let root = layer.root.as_path();
    let candidates = match sources::source(layer.authority) {
        Source::NotApplicable(reason) => return LockfileObservation::not_applicable(reason),
        Source::Fallback(fallback) => return fallback::observe(fallback, root, wants, store),
        Source::Configured => return rush::observe(layer, owned, wants, store),
        Source::Candidates(candidates) => candidates,
    };

    let mut selected = None;
    for &(name, format) in candidates {
        let path = root.join(name);
        match store.lockfile_presence(&path) {
            LockfilePresence::Absent => {}
            LockfilePresence::Failed(kind) => {
                debug!(path = %path.display(), ?kind, "lockfile metadata probe failed");
                return LockfileObservation::unreadable(
                    Vec::new(),
                    LockfileReason::MetadataFailed,
                );
            }
            // A lower-priority candidate is never consulted once one exists.
            LockfilePresence::Present => {
                selected = Some((name, format));
                break;
            }
        }
    }
    let Some((name, format)) = selected else {
        return LockfileObservation::absent();
    };
    let paths = vec![name.to_owned()];
    if !wants {
        return LockfileObservation::not_requested(paths);
    }
    if !format.records_membership() {
        return LockfileObservation::unverifiable(paths, LockfileReason::NoMembershipData);
    }

    let path = root.join(name);
    let parsed = match store.lockfile(&path, format) {
        Ok(parsed) => parsed,
        Err(failure) => {
            debug!(path = %path.display(), ?failure, "lockfile could not be corroborated");
            return failure.observation(paths);
        }
    };
    classify(&parsed, &[], layer, owned, store, paths)
}

/// Turn a parsed lockfile into the layer's observation. `base` is the
/// directory, relative to the layer root, that recorded member paths resolve
/// against.
fn classify(
    parsed: &ParsedLockfile,
    base: &[&str],
    layer: &MonorepoLayer,
    owned: Option<&[PackageSeed]>,
    store: &ManifestStore,
    paths: Vec<String>,
) -> LockfileObservation {
    match parsed {
        ParsedLockfile::UnsupportedVersion => {
            LockfileObservation::unverifiable(paths, LockfileReason::UnsupportedVersion)
        }
        ParsedLockfile::AmbiguousMembership => {
            LockfileObservation::unverifiable(paths, LockfileReason::AmbiguousMembership)
        }
        ParsedLockfile::CargoPackages(sourceless) => {
            cargo::compare(sourceless, layer, owned, store, paths)
        }
        ParsedLockfile::Members(recorded) => compare_members(recorded, base, layer, owned, paths),
    }
}

/// Compare a lockfile's recorded member paths with the layer's manifest
/// members. `base` is the directory, relative to the layer root, that
/// `recorded` paths resolve against.
fn compare_members(
    recorded: &[String],
    base: &[&str],
    layer: &MonorepoLayer,
    owned: Option<&[PackageSeed]>,
    paths: Vec<String>,
) -> LockfileObservation {
    let Some(owned) = owned else {
        return LockfileObservation::unverifiable(
            paths,
            LockfileReason::IncompleteManifestDiscovery,
        );
    };
    let manifest = match membership::manifest_member_set(&layer.root, owned) {
        Ok(set) => set,
        Err(reason) => return LockfileObservation::unverifiable(paths, reason),
    };
    let locked = match membership::recorded_member_set(recorded, base) {
        Ok(set) => set,
        Err(reason) => return LockfileObservation::unverifiable(paths, reason),
    };
    membership::compare(&manifest, &locked, paths)
}

/// A per-thread `#[cfg(test)]` seam that makes chosen lockfile metadata probes
/// fail (ruling R9): Unix mode bits cannot inject a metadata failure portably.
#[cfg(test)]
pub(crate) mod test_seam {
    use std::cell::RefCell;
    use std::path::{Path, PathBuf};

    thread_local! {
        static FAILURES: RefCell<Vec<(PathBuf, std::io::ErrorKind)>> =
            const { RefCell::new(Vec::new()) };
    }

    /// Make every probe of `path` on this thread fail with `kind` until the
    /// guard drops.
    pub(crate) fn fail_metadata(path: &Path, kind: std::io::ErrorKind) -> Guard {
        FAILURES.with(|failures| failures.borrow_mut().push((path.to_path_buf(), kind)));
        Guard
    }

    pub(super) fn injected_failure(path: &Path) -> Option<std::io::ErrorKind> {
        FAILURES.with(|failures| {
            failures
                .borrow()
                .iter()
                .find(|(failed, _)| failed == path)
                .map(|(_, kind)| *kind)
        })
    }

    pub(crate) struct Guard;

    impl Drop for Guard {
        fn drop(&mut self) {
            FAILURES.with(|failures| failures.borrow_mut().clear());
        }
    }
}

#[cfg(test)]
mod tests;
