//! A format 3 host registration as one verified snapshot: the manifest and
//! the exact bytes of every member it lists or derives, each matched against
//! its manifest digest before anything parses it.
//!
//! [`load_snapshot`] reads the manifest, checks its format version and the
//! closed `darkmatter-host` contract ([`manifest`](super::manifest)),
//! resolves and checks every member path, then reads each listed member once
//! and compares its BLAKE3 digest. Each verified `files` item whose `kind` is
//! `expression-schema` then derives its facts companion (the catalog's name
//! with its last extension replaced by `.facts.json`); the `digests` keys
//! must be exactly the listed members and those companions, and each
//! companion is read and verified the same way. The verified bytes are the
//! only copy later preparation reads, so a file replaced after verification
//! cannot reach a profile. An installed bundle, one compiled into a host
//! ([`VerifiedSnapshot::from_embedded`]), and one a caller already read
//! ([`verify_read`]) all go through this one verification.
//!
//! A publisher replaces members first and the manifest last. A reader that
//! meets a missing member or a digest mismatch may have read between those
//! writes, so it reads the manifest once more: when its bytes changed, the
//! whole verification runs again over the new manifest alone; when they did
//! not, the bundle is refused. A snapshot therefore never holds members of
//! two releases.
//!
//! User-facing guide: `darkmatter/docs/topics/host-registration.md`.

use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use biscuit_file::FileResolutionContext;
use serde_json::json;

use super::manifest::{self, ManifestRefusal, MemberRoots, MemberSpec, ReadManifest};
use super::{HOST_REGISTRATION_CODE, HostDependency, Refusal, RegistrationDiagnostic, RegistrationRelated};
use crate::config::load::{Registration, SourcePosition};

/// One member of a manifest, resolved and checked: an entry it lists, or an
/// expression catalog's derived facts companion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BundleMember {
    /// The manifest field that lists it (`files[2]`, `lifecycle`); for a
    /// facts companion, its catalog's `files` entry.
    pub field: String,
    /// The reference as written (for a companion, as derived from its
    /// catalog's), which is also its `digests` key.
    pub reference: String,
    /// Its identity: the slash-separated path from the bundle root
    /// (`schemas/composition.yaml`), the same wherever the bundle is
    /// installed.
    pub path: String,
    /// Where it is read from.
    pub location: PathBuf,
    /// Where the entry's value is written in the manifest, when known.
    pub at: Option<SourcePosition>,
    /// The manifest's digest for it, which its bytes matched in a
    /// [`VerifiedSnapshot`] (an [`AdmittedManifest`] reads no bytes).
    pub digest: String,
}

/// An expression catalog of `files` and the facts companion it derives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BundleCompanion {
    /// The catalog's `files` member.
    pub catalog: BundleMember,
    /// Its derived `.facts.json` sibling.
    pub facts: BundleMember,
}

/// A format 3 registration whose every member matched its digest. Holds the
/// only bytes later preparation may read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedSnapshot {
    host: String,
    host_at: Option<SourcePosition>,
    version: String,
    manifest: PathBuf,
    manifest_hash: String,
    revision: String,
    bundle_root: PathBuf,
    files: Vec<BundleMember>,
    lifecycle: BundleMember,
    companions: Vec<BundleCompanion>,
    bytes: BTreeMap<String, Arc<[u8]>>,
    dependencies: Vec<HostDependency>,
    candidates: Vec<PathBuf>,
    /// The bundle name of a snapshot verified from memory
    /// ([`Self::from_embedded`]); `None` for an installed one.
    embedded: Option<String>,
}

#[allow(missing_docs)]
impl VerifiedSnapshot {
    /// The snapshot of a bundle compiled into a host program: the format 3
    /// manifest `manifest` (file name `manifest_name`) and every member's
    /// bytes in `members` (each catalog's facts companion included), keyed by
    /// bundle-relative path, verified by the same steps [`load_snapshot`]
    /// applies to an installed one. Nothing is read from, or probed on, the
    /// disk.
    ///
    /// The bundle is rooted at the virtual directory `<name>`, the root of
    /// [`BundleSources::embedded`](simplified_schema::resolve::bundle::BundleSources::embedded):
    /// the manifest and every [`BundleMember::location`] lie under it and
    /// name no host file. [`Self::dependencies`] and [`Self::candidates`]
    /// are empty, since nothing on disk can change the snapshot.
    ///
    /// ## Errors
    ///
    /// The first reason the bundle cannot be used, as for an installed
    /// bundle: an invalid manifest or one of another format, a member or
    /// companion that `members` lacks, or one whose bytes do not match its
    /// digest. A `name` that is not one plain path segment is refused too.
    pub fn from_embedded(
        name: &str,
        manifest_name: &str,
        manifest: &[u8],
        members: &BTreeMap<String, Arc<[u8]>>,
    ) -> Result<Self, Box<SnapshotRefusal>> {
        let root = PathBuf::from(format!("<{name}>"));
        let path = root.join(manifest_name);
        if !simplified_schema::embedded::is_plain_segment(name) || name.contains(['<', '>']) {
            let message = format!("the embedded bundle name `{name}` is not one plain path segment");
            let refusal = Refusal { code: HOST_REGISTRATION_CODE, file: path, at: None, member: None, message, related: Vec::new()  };
            return Err(Reads::default().refuse(refusal, None));
        }
        let mut reads = Reads::default();
        let mut source = |member: &BundleMember, _: &mut Reads| {
            let Some(bytes) = members.get(&member.path) else {
                return Err(member_failure(&path, member, format!("`{}` is missing from the embedded bundle", member.reference), false));
            };
            let actual = content_hash(bytes);
            if actual != member.digest {
                return Err(member_failure(
                    &path,
                    member,
                    format!(
                        "`{}` does not match its digest (the manifest lists `{}`, the embedded bytes hash to `{actual}`)",
                        member.reference, member.digest
                    ),
                    false,
                ));
            }
            Ok(Arc::clone(bytes))
        };
        let mut snapshot = attempt(&path, &MemberRoots::Lexical, manifest, &mut reads, &mut source)
            .map_err(|failure| Reads::default().refuse(*failure.refusal, failure.unsupported))?;
        // Nothing on disk can change an embedded snapshot.
        snapshot.embedded = Some(name.to_string());
        snapshot.bundle_root = root;
        Ok(snapshot)
    }

    /// The bundle name of a snapshot built by [`Self::from_embedded`];
    /// `None` for an installed bundle.
    #[must_use]
    pub fn embedded_bundle(&self) -> Option<&str> {
        self.embedded.as_deref()
    }

    #[must_use]
    pub fn host(&self) -> &str {
        &self.host
    }

    /// Where the manifest's `host` value is written.
    #[must_use]
    pub const fn host_position(&self) -> Option<&SourcePosition> {
        self.host_at.as_ref()
    }

    /// The host release that wrote the manifest, not the format.
    #[must_use]
    pub fn version(&self) -> &str {
        &self.version
    }

    #[must_use]
    pub fn manifest(&self) -> &Path {
        &self.manifest
    }

    /// The BLAKE3 digest of the manifest bytes the snapshot was verified
    /// against: what a watcher compares to notice a republished manifest.
    /// Unlike [`Self::revision`], it changes with layout and comments.
    #[must_use]
    pub fn manifest_hash(&self) -> &str {
        &self.manifest_hash
    }

    /// The content revision: a canonical JSON digest of the host, version,
    /// format, the sorted file paths, the lifecycle path, each catalog's
    /// derived companion sorted by catalog path, and every member's digest
    /// (companions included), all by bundle-relative path. The order of `files`, the manifest's layout,
    /// and where the bundle is installed do not change it; any member byte
    /// does.
    #[must_use]
    pub fn revision(&self) -> &str {
        &self.revision
    }

    /// The manifest's directory, which every member path is relative to:
    /// the virtual `<name>` of an embedded bundle.
    #[must_use]
    pub fn bundle_root(&self) -> &Path {
        &self.bundle_root
    }

    /// The `files` members, in authored order.
    #[must_use]
    pub fn files(&self) -> &[BundleMember] {
        &self.files
    }

    /// The `files` members' paths, sorted: the order nothing may depend on
    /// the manifest for.
    #[must_use]
    pub fn sorted_files(&self) -> Vec<&str> {
        let mut paths: Vec<&str> = self.files.iter().map(|member| member.path.as_str()).collect();
        paths.sort_unstable();
        paths
    }

    #[must_use]
    pub const fn lifecycle(&self) -> &BundleMember {
        &self.lifecycle
    }

    /// Each `files` expression catalog with its derived facts companion, in
    /// `files` order.
    #[must_use]
    pub fn companions(&self) -> &[BundleCompanion] {
        &self.companions
    }

    /// Every member: each `files` item, `lifecycle`, then each companion.
    pub fn members(&self) -> impl Iterator<Item = &BundleMember> {
        members(&self.files, &self.lifecycle, self.companions.iter().map(|pair| &pair.facts))
    }

    /// Every member's verified bytes, by bundle-relative path.
    #[must_use]
    pub const fn bytes(&self) -> &BTreeMap<String, Arc<[u8]>> {
        &self.bytes
    }

    /// The verified bytes of the member at bundle-relative `path`.
    #[must_use]
    pub fn member_bytes(&self, path: &str) -> Option<&Arc<[u8]>> {
        self.bytes.get(path)
    }

    /// Every file read, once per path with the hash of its latest read, in
    /// first-read order; a load that retried keeps the first attempt's
    /// paths too.
    #[must_use]
    pub fn dependencies(&self) -> &[HostDependency] {
        &self.dependencies
    }

    /// Every location the load looked at, present or not (the manifest and
    /// each member of every attempt): what a watcher observes.
    #[must_use]
    pub fn candidates(&self) -> &[PathBuf] {
        &self.candidates
    }
}

/// Why no snapshot was published. Always a
/// [`Severity::Warning`](crate::markdown::compose::expression::Severity::Warning) with
/// [`HOST_REGISTRATION_CODE`], located where the problem is: the manifest
/// for its own shape and for a member that is missing, misnamed, or does
/// not match its digest (at that member's entry).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotRefusal {
    #[allow(missing_docs)]
    pub code: &'static str,
    /// What happened, naming the member when one failed.
    pub message: String,
    /// The file the problem is in.
    pub file: PathBuf,
    /// 1-based line in [`Self::file`], when known.
    pub line: Option<usize>,
    /// 1-based character column, when [`Self::line`] is known.
    pub column: Option<usize>,
    /// The line's text before [`Self::column`] (see
    /// [`SourcePosition::line_prefix`]), when [`Self::line`] is known.
    pub line_prefix: Option<String>,
    /// Earlier declarations involved in a conflict.
    pub related: Vec<RegistrationRelated>,
    /// The manifest field of the failing entry (`files[1]`, `lifecycle`,
    /// `digests../schemas/a.yaml`); `None` when the manifest itself failed.
    pub member: Option<String>,
    /// The failing member's display label, `name (bundle path)`, when it is
    /// a trigger or types-only library declaring a valid `name`.
    pub label: Option<String>,
    /// The manifest declares a format this reader does not load (1, 2, or a
    /// later one): the host must update and republish it, and nothing else
    /// about it was judged.
    pub unsupported_format: Option<i128>,
    /// As [`VerifiedSnapshot::dependencies`], for every attempt.
    pub dependencies: Vec<HostDependency>,
    /// As [`VerifiedSnapshot::candidates`], for every attempt: a missing or
    /// mismatched member's creation or repair is seen here.
    pub candidates: Vec<PathBuf>,
}

impl SnapshotRefusal {
    /// A refusal at `refusal`'s file and position.
    pub(super) fn of(refusal: Refusal, unsupported_format: Option<i128>, dependencies: Vec<HostDependency>, candidates: Vec<PathBuf>) -> Self {
        let (line, column, line_prefix) = match refusal.at {
            Some(at) => (Some(at.line), Some(at.column), Some(at.line_prefix)),
            None => (None, None, None),
        };
        Self {
            code: refusal.code,
            message: refusal.message,
            file: refusal.file,
            line,
            column,
            line_prefix,
            related: refusal.related,
            member: refusal.member,
            label: None,
            unsupported_format,
            dependencies,
            candidates,
        }
    }

    /// Where the refusal is in [`Self::file`], when known.
    pub(super) fn position(&self) -> Option<SourcePosition> {
        Some(SourcePosition { line: self.line?, column: self.column?, line_prefix: self.line_prefix.clone()? })
    }

    /// The refusal as the warning of the `register` entry that named the
    /// manifest.
    #[must_use]
    pub fn diagnostic(&self, entry: &Registration) -> RegistrationDiagnostic {
        Refusal {
            code: self.code,
            file: self.file.clone(),
            at: self.position(),
            member: self.member.clone(),
            message: self.message.clone(),
            related: self.related.clone(),
        }
        .into_diagnostic(entry)
    }
}

/// A manifest admitted by the same envelope, digest-map, and member path
/// checks [`load_snapshot`] applies, with no member read: its digests are
/// what the manifest lists, not what the files hold.
///
/// This is what a publisher may trust about the manifest it is replacing
/// (which members it published) when some members no longer match it, as
/// after an interrupted publish. Without reading a catalog, the companions
/// it knows are the `digests` keys that are the companion spelling of a
/// `files` item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmittedManifest {
    format_version: u64,
    host: String,
    version: String,
    files: Vec<BundleMember>,
    lifecycle: BundleMember,
    extra: Vec<BundleMember>,
}

#[allow(missing_docs)]
impl AdmittedManifest {
    pub fn host(&self) -> &str {
        &self.host
    }

    pub fn version(&self) -> &str {
        &self.version
    }

    /// The manifest's format: 3, or 2 for a previous manifest
    /// [`admit_manifest`] admitted for cleanup.
    pub const fn format_version(&self) -> u64 {
        self.format_version
    }

    /// Every member in authored order: each `files` item, `lifecycle`, then
    /// each companion `digests` lists (format 3) or each function pair's
    /// `catalog` and `facts` (format 2).
    pub fn members(&self) -> impl Iterator<Item = &BundleMember> {
        members(&self.files, &self.lifecycle, self.extra.iter())
    }
}

/// Admits the manifest `bytes` read from `manifest`, format 3 or (for a
/// publisher retiring what it published before) a well-formed format 2,
/// without reading or probing any member. Nothing it admits is loadable: a
/// format 2 manifest only names the files it listed.
///
/// Member paths are judged lexically from the manifest's directory: the
/// spelling, portable-name, duplicate, letter-case, and digest-map rules of
/// [`load_snapshot`], with `.` and `..` applied to the spelling. A path that
/// climbs out of the directory and back in is refused here even where the
/// installed reader would accept it, so this admission never accepts a
/// manifest that reader refuses; [`admit_installed_manifest`] admits as
/// that reader does.
///
/// ## Errors
///
/// The first reason the manifest is not a valid format 3 (or format 2)
/// manifest, located as [`load_snapshot`] locates it;
/// [`SnapshotRefusal::unsupported_format`] for any other format.
pub fn admit_manifest(manifest: &Path, bytes: &[u8]) -> Result<AdmittedManifest, Box<SnapshotRefusal>> {
    admit_with(manifest, bytes, manifest::read_for_cleanup, &MemberRoots::Lexical)
}

/// [`admit_manifest`] with each member resolved as [`load_snapshot`] and
/// [`verify_read`] resolve it: from a context derived from `context` and
/// anchored at the manifest, confined to the manifest's directory. A
/// spelling that leaves the directory and comes back (`../host/x.yaml` for a
/// bundle in `host/`) is admitted under its in-bundle
/// [`BundleMember::path`]; one that ends outside it is refused.
///
/// Reads no member and checks no link: a caller that reads the members
/// itself (`claudine doctor`) gets the paths and locations the loader
/// would check, then reads them its own way.
///
/// Only a format 3 manifest is admitted: an installed manifest of any other
/// format is what the loader refuses.
///
/// ## Errors
///
/// As [`admit_manifest`], with the member path rules of [`load_snapshot`].
pub fn admit_installed_manifest(
    manifest: &Path,
    bytes: &[u8],
    context: &FileResolutionContext,
) -> Result<AdmittedManifest, Box<SnapshotRefusal>> {
    admit_with(manifest, bytes, manifest::read, &MemberRoots::Installed(context))
}

fn admit_with(
    manifest: &Path,
    bytes: &[u8],
    read: fn(&Path, Vec<u8>) -> Result<ReadManifest, ManifestRefusal>,
    roots: &MemberRoots<'_>,
) -> Result<AdmittedManifest, Box<SnapshotRefusal>> {
    let refuse = |refusal: Refusal, unsupported: Option<i128>| Reads::default().refuse(refusal, unsupported);
    let read = read(manifest, bytes.to_vec()).map_err(|refusal| match refusal {
        ManifestRefusal::Unsupported { version, refusal } => refuse(refusal, Some(version)),
        ManifestRefusal::Invalid(refusal) => refuse(refusal, None),
    })?;
    let MemberSpec { files, lifecycle, previous } = read.members(manifest, roots).map_err(|refusal| refuse(refusal, None))?;
    let mut extra = previous;
    if read.manifest.format_version == manifest::HOST_MANIFEST_FORMAT_VERSION {
        extra = read.lexical_companions(manifest, roots, &files).map_err(|refusal| refuse(refusal, None))?;
    }
    let all: Vec<&BundleMember> = members(&files, &lifecycle, extra.iter()).collect();
    read.check_digests(manifest, &all, &[]).map_err(|refusal| refuse(refusal, None))?;
    let ReadManifest { manifest: accepted, .. } = read;
    Ok(AdmittedManifest {
        format_version: accepted.format_version,
        host: accepted.host,
        version: accepted.version,
        files,
        lifecycle,
        extra,
    })
}

/// The verified snapshot of the format 3 manifest at `manifest`, reading
/// from disk.
///
/// `context` is the request's captured context; members resolve in a
/// context derived from it and anchored at the manifest.
///
/// ## Errors
///
/// The first reason the bundle cannot be used, after at most one retry
/// over a republished manifest.
pub fn load_snapshot(manifest: &Path, context: &FileResolutionContext) -> Result<VerifiedSnapshot, Box<SnapshotRefusal>> {
    load_snapshot_with(manifest, context, &|path: &Path| std::fs::read(path))
}

/// [`load_snapshot`] reading every file's bytes through `read`, so a caller
/// (a test) controls what each read returns. Path checks (symlinks,
/// junctions, presence) still consult the file system.
///
/// ## Errors
///
/// As [`load_snapshot`].
pub fn load_snapshot_with(
    manifest: &Path,
    context: &FileResolutionContext,
    read: &dyn Fn(&Path) -> io::Result<Vec<u8>>,
) -> Result<VerifiedSnapshot, Box<SnapshotRefusal>> {
    let mut reads = Reads::default();
    let first = reads.read_manifest(manifest, read)?;
    let root = manifest.parent().unwrap_or(manifest).to_path_buf();
    let mut source = |member: &BundleMember, reads: &mut Reads| verify(manifest, &root, member, read, reads);
    let roots = MemberRoots::Installed(context);
    let failure = match attempt(manifest, &roots, &first, &mut reads, &mut source) {
        Ok(snapshot) => return Ok(reads.finish(snapshot)),
        Err(failure) => failure,
    };
    if !failure.retryable {
        return Err(reads.refuse(*failure.refusal, failure.unsupported));
    }
    // Only a changed manifest earns the retry: the same bytes describe the
    // same release, whose members did not match.
    let second = match reads.read_manifest(manifest, read) {
        Ok(second) if second != first => second,
        _ => return Err(reads.refuse(*failure.refusal, failure.unsupported)),
    };
    match attempt(manifest, &roots, &second, &mut reads, &mut source) {
        Ok(snapshot) => Ok(reads.finish(snapshot)),
        Err(failure) => Err(reads.refuse(*failure.refusal, failure.unsupported)),
    }
}

/// The verified snapshot of the format 3 manifest `bytes` at `manifest`
/// whose members' bytes (companions included) the caller has already read, keyed by bundle path
/// ([`BundleMember::path`]): the manifest's admission, member resolution,
/// and digest checks of [`load_snapshot`], with nothing read from or probed
/// on the disk and no retry. Member locations, and so every later
/// diagnostic, name the installed files.
///
/// The caller vouches for how the bytes were read: [`load_snapshot`]'s own
/// refusal of a member reached through a symbolic link or junction is not
/// applied, since nothing is probed. A diagnostic that has already read a
/// bundle without following links (`claudine doctor`) judges it this way,
/// then prepares it with [`load_verified`](super::load_verified), as
/// [`load_registrations`](super::load_registrations) does.
///
/// ## Errors
///
/// The first reason the bundle cannot be used, located as
/// [`load_snapshot`] locates it: a member `members` lacks is missing from
/// the bundle, and one whose bytes differ from its digest is a mismatch.
pub fn verify_read(
    manifest: &Path,
    bytes: &[u8],
    context: &FileResolutionContext,
    members: &BTreeMap<String, Arc<[u8]>>,
) -> Result<VerifiedSnapshot, Box<SnapshotRefusal>> {
    let mut reads = Reads::default();
    reads.probe(manifest);
    reads.record(manifest, bytes);
    let mut source = |member: &BundleMember, reads: &mut Reads| {
        let Some(found) = members.get(&member.path) else {
            return Err(member_failure(manifest, member, format!("`{}` is missing from the bundle", member.reference), true));
        };
        matches_digest(manifest, member, found, reads)?;
        Ok(Arc::clone(found))
    };
    match attempt(manifest, &MemberRoots::Installed(context), bytes, &mut reads, &mut source) {
        Ok(snapshot) => Ok(reads.finish(snapshot)),
        Err(failure) => Err(reads.refuse(*failure.refusal, failure.unsupported)),
    }
}

/// What one verification attempt refused.
struct Failure {
    // Boxed: a located refusal keeps its line's text, and the error stays small.
    refusal: Box<Refusal>,
    /// A missing member or a digest mismatch: a publish may be in progress.
    retryable: bool,
    /// The manifest's unsupported format version.
    unsupported: Option<i128>,
}

impl Failure {
    fn fatal(refusal: Refusal) -> Self {
        Self { refusal: Box::new(refusal), retryable: false, unsupported: None }
    }
}

/// Every read and probe across attempts.
#[derive(Default)]
struct Reads {
    dependencies: Vec<HostDependency>,
    candidates: Vec<PathBuf>,
}

impl Reads {
    fn probe(&mut self, path: &Path) {
        if !self.candidates.iter().any(|seen| seen == path) {
            self.candidates.push(path.to_path_buf());
        }
    }

    fn record(&mut self, path: &Path, bytes: &[u8]) -> String {
        let hash = content_hash(bytes);
        match self.dependencies.iter_mut().find(|dependency| dependency.path == path) {
            Some(dependency) => dependency.content_hash.clone_from(&hash),
            None => self.dependencies.push(HostDependency { path: path.to_path_buf(), content_hash: hash.clone() }),
        }
        hash
    }

    fn read_manifest(
        &mut self,
        path: &Path,
        read: &dyn Fn(&Path) -> io::Result<Vec<u8>>,
    ) -> Result<Vec<u8>, Box<SnapshotRefusal>> {
        self.probe(path);
        match read(path) {
            Ok(bytes) => {
                self.record(path, &bytes);
                Ok(bytes)
            }
            Err(error) => {
                let refusal = Refusal {
                    code: HOST_REGISTRATION_CODE,
                    file: path.to_path_buf(),
                    at: None,
                    member: None,
                    message: format!("the manifest cannot be read: {error}"),
                    related: Vec::new(),
                };
                Err(self.refuse_ref(refusal, None))
            }
        }
    }

    fn refuse_ref(&self, refusal: Refusal, unsupported: Option<i128>) -> Box<SnapshotRefusal> {
        Box::new(SnapshotRefusal::of(refusal, unsupported, self.dependencies.clone(), self.candidates.clone()))
    }

    fn refuse(self, refusal: Refusal, unsupported: Option<i128>) -> Box<SnapshotRefusal> {
        self.refuse_ref(refusal, unsupported)
    }

    fn finish(self, mut snapshot: VerifiedSnapshot) -> VerifiedSnapshot {
        snapshot.dependencies = self.dependencies;
        snapshot.candidates = self.candidates;
        snapshot
    }
}

/// Where one attempt takes a member's bytes from, once they matched its
/// digest.
type MemberSource<'a> = dyn FnMut(&BundleMember, &mut Reads) -> Result<Arc<[u8]>, Failure> + 'a;

/// One verification over the manifest `bytes`: every member taken fresh from
/// `source`, nothing kept from an earlier attempt.
///
/// The listed members are verified first. Each verified `files` item whose
/// `kind` is `expression-schema` then derives its facts companion, whose
/// location is probed (so a watcher sees it appear) before the `digests`
/// keys are checked to be exactly the members and companions and the
/// companion's own bytes are verified.
fn attempt(
    path: &Path,
    roots: &MemberRoots<'_>,
    bytes: &[u8],
    reads: &mut Reads,
    source: &mut MemberSource<'_>,
) -> Result<VerifiedSnapshot, Failure> {
    let manifest_hash = content_hash(bytes);
    let manifest = manifest::read(path, bytes.to_vec()).map_err(|refusal| match refusal {
        ManifestRefusal::Unsupported { version, refusal } => {
            Failure { refusal: Box::new(refusal), retryable: false, unsupported: Some(version) }
        }
        ManifestRefusal::Invalid(refusal) => Failure::fatal(refusal),
    })?;
    let MemberSpec { files, lifecycle, .. } = manifest.members(path, roots).map_err(Failure::fatal)?;
    for member in members(&files, &lifecycle, std::iter::empty()) {
        reads.probe(&member.location);
    }
    // Every companion `digests` names is watched before any read can fail,
    // so a watcher sees it whichever member is missing.
    for companion in manifest.lexical_companions(path, roots, &files).unwrap_or_default() {
        reads.probe(&companion.location);
    }
    let mut table: BTreeMap<String, Arc<[u8]>> = BTreeMap::new();
    for member in members(&files, &lifecycle, std::iter::empty()) {
        let bytes = source(member, reads)?;
        table.insert(member.path.clone(), bytes);
    }

    let mut companions: Vec<BundleCompanion> = Vec::new();
    let mut unclassified: Vec<&BundleMember> = Vec::new();
    for file in &files {
        match classify(&table[&file.path]) {
            Classified::Catalog => {
                let known: Vec<&BundleMember> =
                    members(&files, &lifecycle, companions.iter().map(|pair| &pair.facts)).collect();
                let facts = manifest.companion(path, roots, file, &known).map_err(Failure::fatal)?;
                companions.push(BundleCompanion { catalog: file.clone(), facts });
            }
            Classified::Other => {}
            Classified::Unknown => unclassified.push(file),
        }
    }
    for pair in &companions {
        reads.probe(&pair.facts.location);
    }
    let all: Vec<&BundleMember> = members(&files, &lifecycle, companions.iter().map(|pair| &pair.facts)).collect();
    manifest.check_digests(path, &all, &unclassified).map_err(Failure::fatal)?;
    for pair in &companions {
        let bytes = source(&pair.facts, reads)?;
        table.insert(pair.facts.path.clone(), bytes);
    }

    let revision = revision(&manifest, &files, &lifecycle, &companions);
    let host_at = manifest.locate(&["host"]);
    let root = path.parent().unwrap_or(path).to_path_buf();
    let ReadManifest { manifest: accepted, .. } = manifest;
    Ok(VerifiedSnapshot {
        host: accepted.host,
        host_at,
        version: accepted.version,
        manifest: path.to_path_buf(),
        manifest_hash,
        revision,
        bundle_root: root,
        files,
        lifecycle,
        companions,
        bytes: table,
        dependencies: Vec::new(),
        candidates: Vec::new(),
        embedded: None,
    })
}

/// What a verified `files` item is, as far as companions go.
enum Classified {
    /// `kind: expression-schema`: it derives a facts companion.
    Catalog,
    /// Any other kind.
    Other,
    /// Not one YAML mapping with a string `kind`: the member is refused when
    /// it is prepared, so whether it would derive a companion is not judged.
    Unknown,
}

/// Reads only `bytes`' top-level `kind`, leniently: the member's full
/// admission happens when it is prepared.
fn classify(bytes: &[u8]) -> Classified {
    let Ok(text) = std::str::from_utf8(bytes) else {
        return Classified::Unknown;
    };
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let Ok(value) = serde_yaml_ng::from_str::<serde_yaml_ng::Value>(text) else {
        return Classified::Unknown;
    };
    match value.as_mapping().and_then(|map| map.get("kind")).and_then(serde_yaml_ng::Value::as_str) {
        Some("expression-schema") => Classified::Catalog,
        Some(_) => Classified::Other,
        None => Classified::Unknown,
    }
}

/// The lowercase hexadecimal BLAKE3 digest of `bytes`: what `digests`
/// records and [`HostDependency::content_hash`] holds.
fn content_hash(bytes: &[u8]) -> String {
    use std::fmt::Write;
    biscuit_hash::blake3_hash_bytes(bytes).iter().fold(String::with_capacity(64), |mut hex, byte| {
        let _ = write!(hex, "{byte:02x}");
        hex
    })
}

fn members<'a>(
    files: &'a [BundleMember],
    lifecycle: &'a BundleMember,
    extra: impl Iterator<Item = &'a BundleMember>,
) -> impl Iterator<Item = &'a BundleMember> {
    files.iter().chain(std::iter::once(lifecycle)).chain(extra)
}

/// The bytes of `member`, once they matched its digest.
fn verify(
    manifest: &Path,
    root: &Path,
    member: &BundleMember,
    read: &dyn Fn(&Path) -> io::Result<Vec<u8>>,
    reads: &mut Reads,
) -> Result<Arc<[u8]>, Failure> {
    let refuse = |message: String, retryable: bool| member_failure(manifest, member, message, retryable);
    let missing = || refuse(format!("`{}` is missing from the bundle", member.reference), true);
    match linked_component(root, &member.path) {
        Ok(None) => {}
        Ok(Some(link)) => {
            return Err(refuse(
                format!(
                    "`{}` passes through `{link}`, a symbolic link or junction; bundle members are plain files below \
                     the manifest's directory",
                    member.reference
                ),
                false,
            ));
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Err(missing()),
        Err(error) => return Err(refuse(format!("`{}` cannot be inspected: {error}", member.reference), false)),
    }
    let bytes = match read(&member.location) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Err(missing()),
        Err(error) => return Err(refuse(format!("`{}` cannot be read: {error}", member.reference), false)),
    };
    matches_digest(manifest, member, &bytes, reads)?;
    Ok(Arc::from(bytes))
}

/// A refusal at `member`'s manifest entry.
fn member_failure(manifest: &Path, member: &BundleMember, message: String, retryable: bool) -> Failure {
    Failure {
        refusal: Box::new(Refusal {
            code: HOST_REGISTRATION_CODE,
            file: manifest.to_path_buf(),
            at: member.at.clone(),
            member: Some(member.field.clone()),
            message,
            related: Vec::new(),
        }),
        retryable,
        unsupported: None,
    }
}

/// Records `bytes` as read from `member`'s location and refuses them unless
/// they match its digest.
fn matches_digest(manifest: &Path, member: &BundleMember, bytes: &[u8], reads: &mut Reads) -> Result<(), Failure> {
    let actual = reads.record(&member.location, bytes);
    if actual == member.digest {
        return Ok(());
    }
    Err(member_failure(
        manifest,
        member,
        format!(
            "`{}` does not match its digest (the manifest lists `{}`, the file hashes to `{actual}`); the host may be \
             mid-publish, or its bundle was edited",
            member.reference, member.digest
        ),
        true,
    ))
}

/// The first name of `path` below `root` that is a symbolic link, or on
/// Windows any reparse point (a junction included), never following one.
fn linked_component(root: &Path, path: &str) -> io::Result<Option<String>> {
    let mut at = root.to_path_buf();
    let mut spelled = String::new();
    for name in path.split('/') {
        at.push(name);
        if !spelled.is_empty() {
            spelled.push('/');
        }
        spelled.push_str(name);
        let metadata = std::fs::symlink_metadata(&at)?;
        if metadata.file_type().is_symlink() || is_reparse_point(&metadata) {
            return Ok(Some(spelled));
        }
    }
    Ok(None)
}

#[cfg(windows)]
fn is_reparse_point(metadata: &std::fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
    metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

#[cfg(not(windows))]
const fn is_reparse_point(_: &std::fs::Metadata) -> bool {
    false
}

/// See [`VerifiedSnapshot::revision`].
fn revision(manifest: &ReadManifest, files: &[BundleMember], lifecycle: &BundleMember, companions: &[BundleCompanion]) -> String {
    let mut sorted_files: Vec<&str> = files.iter().map(|member| member.path.as_str()).collect();
    sorted_files.sort_unstable();
    let mut pairs: Vec<(&str, &str)> =
        companions.iter().map(|pair| (pair.catalog.path.as_str(), pair.facts.path.as_str())).collect();
    pairs.sort_unstable();
    let digests: serde_json::Map<String, serde_json::Value> = members(files, lifecycle, companions.iter().map(|pair| &pair.facts))
        .map(|member| (member.path.clone(), json!(member.digest)))
        .collect();
    biscuit_hash::blake3_hash_json(&json!({
        "host": manifest.manifest.host,
        "version": manifest.manifest.version,
        "format": manifest::HOST_MANIFEST_FORMAT_VERSION,
        "files": sorted_files,
        "lifecycle": lifecycle.path,
        "companions": pairs.iter().map(|(catalog, facts)| json!({ "catalog": catalog, "facts": facts })).collect::<Vec<_>>(),
        "digests": digests,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_snapshot_is_shareable_across_threads() {
        fn shareable<T: Send + Sync>() {}
        shareable::<VerifiedSnapshot>();
        shareable::<SnapshotRefusal>();
    }
}
