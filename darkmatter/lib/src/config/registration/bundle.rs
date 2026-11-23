//! A format 2 host registration as one verified snapshot: the manifest and
//! the exact bytes of every member it lists, each matched against its
//! manifest digest before anything parses it.
//!
//! [`load_snapshot`] reads the manifest, checks it against the closed
//! `darkmatter-host` contract ([`manifest`](super::manifest)), resolves and
//! checks every member path, then reads each member once and compares its
//! BLAKE3 digest. The verified bytes are the only copy later preparation
//! reads, so a file replaced after verification cannot reach a profile.
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

use super::manifest::{self, ManifestRefusal, MemberSpec, ReadManifestV2};
use super::{HOST_REGISTRATION_CODE, HostDependency, Refusal, RegistrationDiagnostic};
use crate::config::load::{Registration, SourcePosition};

/// One member entry of a format 2 manifest, resolved and checked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BundleMember {
    /// The manifest field that lists it (`files[2]`, `lifecycle`,
    /// `functions[0].facts`).
    pub field: String,
    /// The reference as written, which is also its `digests` key.
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

/// One `functions` item, resolved and checked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BundleFunctionPair {
    #[allow(missing_docs)]
    pub catalog: BundleMember,
    #[allow(missing_docs)]
    pub facts: BundleMember,
}

/// A format 2 registration whose every member matched its digest. Holds the
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
    functions: Vec<BundleFunctionPair>,
    bytes: BTreeMap<String, Arc<[u8]>>,
    dependencies: Vec<HostDependency>,
    candidates: Vec<PathBuf>,
    /// The bundle name of a snapshot verified from memory
    /// ([`Self::from_embedded`]); `None` for an installed one.
    embedded: Option<String>,
}

#[allow(missing_docs)]
impl VerifiedSnapshot {
    /// The snapshot of a bundle compiled into a host program: the format 2
    /// manifest `manifest` (file name `manifest_name`) and every member's
    /// bytes in `members`, keyed by bundle-relative path, each checked
    /// against the manifest's digest exactly as [`load_snapshot`] checks an
    /// installed one. Nothing is read from, or probed on, the disk.
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
    /// bundle: an invalid or format 1 manifest, a member the manifest lists
    /// that `members` lacks, or one whose bytes do not match its digest. A
    /// `name` that is not one plain path segment is refused too.
    pub fn from_embedded(
        name: &str,
        manifest_name: &str,
        manifest: &[u8],
        members: &BTreeMap<String, Arc<[u8]>>,
    ) -> Result<Self, Box<SnapshotRefusal>> {
        let root = PathBuf::from(format!("<{name}>"));
        let path = root.join(manifest_name);
        let refuse = |refusal: Refusal, legacy: bool| Reads::default().refuse(refusal, legacy);
        if !simplified_schema::embedded::is_plain_segment(name) || name.contains(['<', '>']) {
            let message = format!("the embedded bundle name `{name}` is not one plain path segment");
            return Err(refuse(Refusal { code: HOST_REGISTRATION_CODE, file: path, at: None, member: None, message }, false));
        }
        let read = manifest::read_v2(&path, manifest.to_vec()).map_err(|refusal| match refusal {
            ManifestRefusal::Format1(refusal) => refuse(refusal, true),
            ManifestRefusal::Invalid(refusal) => refuse(refusal, false),
        })?;
        let MemberSpec { files, lifecycle, functions } = read.embedded_members(&path).map_err(|refusal| refuse(refusal, false))?;
        let mut table: BTreeMap<String, Arc<[u8]>> = BTreeMap::new();
        for member in self::members(&files, &lifecycle, &functions) {
            let at = member.at.clone();
            let refuse_member = |message: String| {
                refuse(
                    Refusal { code: HOST_REGISTRATION_CODE, file: path.clone(), at: at.clone(), member: Some(member.field.clone()), message },
                    false,
                )
            };
            let bytes = members
                .get(&member.path)
                .ok_or_else(|| refuse_member(format!("`{}` is missing from the embedded bundle", member.reference)))?;
            let actual = content_hash(bytes);
            if actual != member.digest {
                return Err(refuse_member(format!(
                    "`{}` does not match its digest (the manifest lists `{}`, the embedded bytes hash to `{actual}`)",
                    member.reference, member.digest
                )));
            }
            table.insert(member.path.clone(), Arc::clone(bytes));
        }
        let revision = revision(&read, &files, &lifecycle, &functions);
        let host_at = read.locate(&["host"]);
        let ReadManifestV2 { manifest: accepted, .. } = read;
        Ok(Self {
            host: accepted.host,
            host_at,
            version: accepted.version,
            manifest: path,
            manifest_hash: content_hash(manifest),
            revision,
            bundle_root: root,
            files,
            lifecycle,
            functions,
            bytes: table,
            dependencies: Vec::new(),
            candidates: Vec::new(),
            embedded: Some(name.to_string()),
        })
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
    /// format, the sorted file paths, the lifecycle path, the function pairs
    /// sorted by catalog then facts path, and every member's digest, all by
    /// bundle-relative path. The order of `files`, the manifest's layout,
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

    /// The `functions` pairs, in authored order.
    #[must_use]
    pub fn functions(&self) -> &[BundleFunctionPair] {
        &self.functions
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
    /// The manifest field of the failing entry (`files[1]`, `lifecycle`,
    /// `digests../schemas/a.yaml`); `None` when the manifest itself failed.
    pub member: Option<String>,
    /// The manifest is format 1: the host must republish it, and nothing
    /// else about it was judged.
    pub legacy_format: bool,
    /// As [`VerifiedSnapshot::dependencies`], for every attempt.
    pub dependencies: Vec<HostDependency>,
    /// As [`VerifiedSnapshot::candidates`], for every attempt: a missing or
    /// mismatched member's creation or repair is seen here.
    pub candidates: Vec<PathBuf>,
}

impl SnapshotRefusal {
    /// A refusal at `refusal`'s file and position.
    pub(super) fn of(refusal: Refusal, legacy_format: bool, dependencies: Vec<HostDependency>, candidates: Vec<PathBuf>) -> Self {
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
            member: refusal.member,
            legacy_format,
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
        }
        .into_diagnostic(entry)
    }
}

/// A format 2 manifest admitted by the same envelope, digest-map, and member
/// path checks [`load_snapshot`] applies, with no member read: its digests
/// are what the manifest lists, not what the files hold.
///
/// This is what a publisher may trust about the manifest it is replacing
/// (which members it published) when some members no longer match it, as
/// after an interrupted publish.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmittedManifest {
    host: String,
    version: String,
    files: Vec<BundleMember>,
    lifecycle: BundleMember,
    functions: Vec<BundleFunctionPair>,
}

#[allow(missing_docs)]
impl AdmittedManifest {
    pub fn host(&self) -> &str {
        &self.host
    }

    pub fn version(&self) -> &str {
        &self.version
    }

    /// Every member entry in authored order: each `files` item, `lifecycle`,
    /// then each function's `catalog` and `facts`.
    pub fn members(&self) -> impl Iterator<Item = &BundleMember> {
        members(&self.files, &self.lifecycle, &self.functions)
    }
}

/// Admits the format 2 manifest `bytes` read from `manifest` without reading
/// or probing any member.
///
/// Member paths are judged lexically from the manifest's directory: the
/// spelling, portable-name, duplicate, letter-case, and digest-map rules of
/// [`load_snapshot`], with `.` and `..` applied to the spelling. A path that
/// climbs out of the directory and back in is refused here even where the
/// installed reader would accept it, so this admission never accepts a
/// manifest that reader refuses.
///
/// ## Errors
///
/// The first reason the manifest is not a valid format 2 manifest, located
/// as [`load_snapshot`] locates it; [`SnapshotRefusal::legacy_format`] for a
/// format 1 manifest.
pub fn admit_manifest(manifest: &Path, bytes: &[u8]) -> Result<AdmittedManifest, Box<SnapshotRefusal>> {
    let refuse = |refusal: Refusal, legacy: bool| Reads::default().refuse(refusal, legacy);
    let read = manifest::read_v2(manifest, bytes.to_vec()).map_err(|refusal| match refusal {
        ManifestRefusal::Format1(refusal) => refuse(refusal, true),
        ManifestRefusal::Invalid(refusal) => refuse(refusal, false),
    })?;
    let MemberSpec { files, lifecycle, functions } =
        read.embedded_members(manifest).map_err(|refusal| refuse(refusal, false))?;
    let ReadManifestV2 { manifest: accepted, .. } = read;
    Ok(AdmittedManifest { host: accepted.host, version: accepted.version, files, lifecycle, functions })
}

/// The verified snapshot of the format 2 manifest at `manifest`, reading
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
    let failure = match attempt(manifest, context, &first, read, &mut reads) {
        Ok(snapshot) => return Ok(reads.finish(snapshot)),
        Err(failure) => failure,
    };
    if !failure.retryable {
        return Err(reads.refuse(*failure.refusal, failure.legacy));
    }
    // Only a changed manifest earns the retry: the same bytes describe the
    // same release, whose members did not match.
    let second = match reads.read_manifest(manifest, read) {
        Ok(second) if second != first => second,
        _ => return Err(reads.refuse(*failure.refusal, failure.legacy)),
    };
    match attempt(manifest, context, &second, read, &mut reads) {
        Ok(snapshot) => Ok(reads.finish(snapshot)),
        Err(failure) => Err(reads.refuse(*failure.refusal, failure.legacy)),
    }
}

/// What one verification attempt refused.
struct Failure {
    // Boxed: a located refusal keeps its line's text, and the error stays small.
    refusal: Box<Refusal>,
    /// A missing member or a digest mismatch: a publish may be in progress.
    retryable: bool,
    legacy: bool,
}

impl Failure {
    fn fatal(refusal: Refusal) -> Self {
        Self { refusal: Box::new(refusal), retryable: false, legacy: false }
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
                };
                Err(self.refuse_ref(refusal, false))
            }
        }
    }

    fn refuse_ref(&self, refusal: Refusal, legacy_format: bool) -> Box<SnapshotRefusal> {
        Box::new(SnapshotRefusal::of(refusal, legacy_format, self.dependencies.clone(), self.candidates.clone()))
    }

    fn refuse(self, refusal: Refusal, legacy_format: bool) -> Box<SnapshotRefusal> {
        self.refuse_ref(refusal, legacy_format)
    }

    fn finish(self, mut snapshot: VerifiedSnapshot) -> VerifiedSnapshot {
        snapshot.dependencies = self.dependencies;
        snapshot.candidates = self.candidates;
        snapshot
    }
}

/// One verification over the manifest `bytes`: every member read fresh,
/// nothing kept from an earlier attempt.
fn attempt(
    path: &Path,
    context: &FileResolutionContext,
    bytes: &[u8],
    read: &dyn Fn(&Path) -> io::Result<Vec<u8>>,
    reads: &mut Reads,
) -> Result<VerifiedSnapshot, Failure> {
    let manifest_hash = content_hash(bytes);
    let manifest = manifest::read_v2(path, bytes.to_vec()).map_err(|refusal| match refusal {
        ManifestRefusal::Format1(refusal) => Failure { refusal: Box::new(refusal), retryable: false, legacy: true },
        ManifestRefusal::Invalid(refusal) => Failure::fatal(refusal),
    })?;
    let MemberSpec { files, lifecycle, functions } = manifest.members(path, context).map_err(Failure::fatal)?;
    for member in members(&files, &lifecycle, &functions) {
        reads.probe(&member.location);
    }
    let root = path.parent().unwrap_or(path).to_path_buf();
    let mut table: BTreeMap<String, Arc<[u8]>> = BTreeMap::new();
    for member in members(&files, &lifecycle, &functions) {
        let bytes = verify(path, &root, member, read, reads)?;
        table.insert(member.path.clone(), bytes);
    }
    let revision = revision(&manifest, &files, &lifecycle, &functions);
    let host_at = manifest.locate(&["host"]);
    let ReadManifestV2 { manifest: accepted, .. } = manifest;
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
        functions,
        bytes: table,
        dependencies: Vec::new(),
        candidates: Vec::new(),
        embedded: None,
    })
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
    functions: &'a [BundleFunctionPair],
) -> impl Iterator<Item = &'a BundleMember> {
    files
        .iter()
        .chain(std::iter::once(lifecycle))
        .chain(functions.iter().flat_map(|pair| [&pair.catalog, &pair.facts]))
}

/// The bytes of `member`, once they matched its digest.
fn verify(
    manifest: &Path,
    root: &Path,
    member: &BundleMember,
    read: &dyn Fn(&Path) -> io::Result<Vec<u8>>,
    reads: &mut Reads,
) -> Result<Arc<[u8]>, Failure> {
    let refuse = |message: String, retryable: bool| Failure {
        refusal: Box::new(Refusal {
            code: HOST_REGISTRATION_CODE,
            file: manifest.to_path_buf(),
            at: member.at.clone(),
            member: Some(member.field.clone()),
            message,
        }),
        retryable,
        legacy: false,
    };
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
    let actual = reads.record(&member.location, &bytes);
    if actual != member.digest {
        return Err(refuse(
            format!(
                "`{}` does not match its digest (the manifest lists `{}`, the file hashes to `{actual}`); the host may \
                 be mid-publish, or its bundle was edited",
                member.reference, member.digest
            ),
            true,
        ));
    }
    Ok(Arc::from(bytes))
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
fn revision(
    manifest: &ReadManifestV2,
    files: &[BundleMember],
    lifecycle: &BundleMember,
    functions: &[BundleFunctionPair],
) -> String {
    let mut sorted_files: Vec<&str> = files.iter().map(|member| member.path.as_str()).collect();
    sorted_files.sort_unstable();
    let mut pairs: Vec<(&str, &str)> =
        functions.iter().map(|pair| (pair.catalog.path.as_str(), pair.facts.path.as_str())).collect();
    pairs.sort_unstable();
    let digests: serde_json::Map<String, serde_json::Value> = members(files, lifecycle, functions)
        .map(|member| (member.path.clone(), json!(member.digest)))
        .collect();
    biscuit_hash::blake3_hash_json(&json!({
        "host": manifest.manifest.host,
        "version": manifest.manifest.version,
        "format": manifest::HOST_MANIFEST_FORMAT_VERSION,
        "files": sorted_files,
        "lifecycle": lifecycle.path,
        "functions": pairs.iter().map(|(catalog, facts)| json!({ "catalog": catalog, "facts": facts })).collect::<Vec<_>>(),
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
