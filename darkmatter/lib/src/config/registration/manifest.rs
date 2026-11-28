//! The `darkmatter-host` manifest envelope: its shipped schema, the
//! validation a manifest passes before any member is read, and (for format
//! 3) the normalized member specification a verified snapshot reads,
//! including the facts companion each expression catalog derives.
//!
//! The format 2 decoder is kept for one reader only: a publisher deciding
//! which files its previous, format 2 manifest lets it retire
//! ([`super::bundle::admit_manifest`]). No format 2 manifest is ever loaded
//! as a profile.

// A refusal is a located message plus its related locations, built once
// per refused member and never on a hot path; boxing every `Err` would only
// add noise at each construction site.
#![allow(clippy::result_large_err)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use biscuit_file::{FileReference, FileReferenceKind, FileResolutionContext, PathIdentity};
use serde::Deserialize;
use simplified_schema::yaml_documents::{LoadedYaml, load_yaml_document, locating_problem};
use simplified_schema::{SchemaError, SimplifiedSchema, ValidationFailure, ValidationProblem};

use super::bundle::BundleMember;
use super::{HOST_REGISTRATION_CODE, Refusal};
use crate::config::load::{SourcePosition, line_column};

/// The virtual path of the shipped manifest schema.
pub const HOST_MANIFEST_SCHEMA_PATH: &str = crate::markdown::schemas::embedded::HOST_MANIFEST_SCHEMA_PATH;

/// The manifest schema's text, as shipped: the format 3 contract.
pub const HOST_MANIFEST_SCHEMA_SOURCE: &str = include_str!("../../../../schemas/darkmatter-host.yaml");

/// The named type a whole manifest satisfies.
pub const HOST_MANIFEST_TYPE: &str = "darkmatter-host";

/// The manifest format the shipped contract describes.
pub const HOST_MANIFEST_FORMAT_VERSION: u64 = 3;

/// The format a publisher's previous manifest may still have, read only to
/// learn which files it published ([`read_for_cleanup`]).
const PREVIOUS_FORMAT_VERSION: u64 = 2;

/// The format 1 fields later formats removed; any of them, beside no
/// `format_version`, marks a manifest written for the format 1 reader.
const LEGACY_FIELDS: [&str; 3] = ["identify", "triggers", "globals"];

/// The extensions an expression catalog's file name may end in; the last
/// one is replaced to name its facts companion.
const CATALOG_EXTENSIONS: [&str; 2] = ["yaml", "yml"];

/// The prepared contract of a whole manifest.
///
/// ## Panics
///
/// When the shipped schema does not prepare, which the shipped artifact and
/// the registration tests rule out.
fn contract() -> &'static SimplifiedSchema {
    static CONTRACT: OnceLock<SimplifiedSchema> = OnceLock::new();
    CONTRACT.get_or_init(|| prepare(HOST_MANIFEST_SCHEMA_SOURCE))
}

fn prepare(source: &str) -> SimplifiedSchema {
    SimplifiedSchema::try_from(source)
        .and_then(|schema| schema.select(HOST_MANIFEST_TYPE))
        .unwrap_or_else(|error| panic!("the `{HOST_MANIFEST_TYPE}` type prepares: {error}"))
}

fn locate(loaded: &LoadedYaml, text: &str, segments: &[&str]) -> Option<SourcePosition> {
    let segments: Vec<String> = segments.iter().map(|segment| (*segment).to_string()).collect();
    loaded.byte_range(&locating_problem(&segments)).map(|range| line_column(text, range.start))
}

fn refusal(path: &Path, at: Option<SourcePosition>, message: String) -> Refusal {
    Refusal { code: HOST_REGISTRATION_CODE, file: path.to_path_buf(), at, member: None, message, related: Vec::new()  }
}

/// The manifest's text and its one YAML mapping.
fn load_mapping(path: &Path, bytes: Vec<u8>) -> Result<(String, LoadedYaml), Refusal> {
    let text = String::from_utf8(bytes).map_err(|error| {
        let at = error.utf8_error().valid_up_to();
        let text = String::from_utf8_lossy(error.as_bytes()).into_owned();
        refusal(path, Some(line_column(&text, at)), "the manifest is not valid UTF-8".to_string())
    })?;
    let loaded = load_yaml_document(&text, path).map_err(|error| match error {
        SchemaError::SchemaDocument { message, span, .. } => refusal(
            path,
            span.map(|span| line_column(&text, span.start)),
            format!("the manifest is not valid YAML: {message}"),
        ),
        other => refusal(path, None, format!("the manifest is not valid YAML: {other}")),
    })?;
    if !loaded.json.is_object() {
        let what = if loaded.json.is_null() { "is empty" } else { "is not a mapping" };
        return Err(refusal(path, Some(SourcePosition::START), format!("the manifest {what}; a `{HOST_MANIFEST_TYPE}` manifest is a mapping")));
    }
    Ok((text, loaded))
}

/// Every problem `contract` finds in the manifest, located at the first.
fn judge(contract: &SimplifiedSchema, path: &Path, text: &str, loaded: &LoadedYaml) -> Result<(), Refusal> {
    let Err(failure) = contract.validate_strict(&loaded.json) else {
        return Ok(());
    };
    let problems = match &failure {
        ValidationFailure::InvalidData { problems } | ValidationFailure::UnavailableCheck { problems } => {
            problems.as_slice()
        }
        _ => &[],
    };
    let Some(first) = problems.first() else {
        return Err(refusal(path, None, format!("the manifest schema cannot judge it: {failure:?}")));
    };
    let at = problem_location(loaded, text, first);
    let message = problems.iter().map(describe).collect::<Vec<_>>().join("; ");
    Err(refusal(path, at, message))
}

/// Where `problem` is: the offending key for an unknown one, else its value
/// (a missing field's parent).
fn problem_location(loaded: &LoadedYaml, text: &str, problem: &ValidationProblem) -> Option<SourcePosition> {
    let segments: Vec<String> = problem.instance_path.segments().to_vec();
    let mut locating = locating_problem(&segments);
    // The key's own token: a value written as a block starts on a later line.
    locating.offending_property = unknown_key(problem).map(str::to_string);
    loaded.byte_range(&locating).map(|range| line_column(text, range.start)).or(Some(SourcePosition::START))
}

fn unknown_key(problem: &ValidationProblem) -> Option<&str> {
    problem
        .offending_properties
        .first()
        .or(problem.offending_property.as_ref())
        .map(String::as_str)
}

/// One problem, worded for a manifest author.
fn describe(problem: &ValidationProblem) -> String {
    let at = simplified_schema::yaml_documents::pointer_string(problem.instance_path.segments());
    if let Some(key) = unknown_key(problem) {
        let parent = if at.is_empty() { String::new() } else { format!(" at `{at}`") };
        return format!("`{key}` is not a manifest field{parent}");
    }
    if let Some(missing) = &problem.property {
        let parent = if at.is_empty() { String::new() } else { format!(" at `{at}`") };
        return format!("`{missing}` is required{parent}");
    }
    if at.is_empty() { problem.message.clone() } else { format!("`{at}`: {}", problem.message) }
}

// ── format 3 ────────────────────────────────────────────────────────────────

/// A format 3 manifest the shipped contract accepted. Every field is
/// required by the schema, so none is defaulted here.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ManifestV3 {
    #[serde(rename = "kind")]
    _kind: String,
    #[serde(rename = "format_version")]
    _format_version: u64,
    host: String,
    version: String,
    files: Vec<String>,
    lifecycle: String,
    digests: BTreeMap<String, String>,
}

/// A format 2 manifest, decoded only for publication cleanup: its typed
/// shape, closed at every level, with nothing defaulted.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ManifestV2 {
    kind: String,
    format_version: u64,
    host: String,
    version: String,
    files: Vec<String>,
    lifecycle: String,
    functions: Vec<FunctionPairV2>,
    digests: BTreeMap<String, String>,
}

/// One format 2 `functions` item.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FunctionPairV2 {
    catalog: String,
    facts: String,
}

/// An accepted manifest's fields, whatever its format.
#[derive(Debug)]
pub(super) struct Manifest {
    pub format_version: u64,
    pub host: String,
    pub version: String,
    pub files: Vec<String>,
    pub lifecycle: String,
    /// A format 2 manifest's `functions` pairs (catalog, facts); empty for
    /// format 3, whose catalogs are `files` and whose companions are derived.
    pub previous_pairs: Vec<(String, String)>,
    pub digests: BTreeMap<String, String>,
}

/// Why [`read`] refused a manifest.
pub(super) enum ManifestRefusal {
    /// The manifest declares a format this reader does not load (located at
    /// its `format_version`, or at a format 1 field when it has none).
    Unsupported { version: i128, refusal: Refusal },
    /// Anything else: not YAML, not a mapping, or not the closed contract.
    Invalid(Refusal),
}

/// A manifest as read: its text, its positions, and its accepted fields.
pub(super) struct ReadManifest {
    pub text: String,
    pub loaded: LoadedYaml,
    pub manifest: Manifest,
}

/// Validates the format 3 manifest `bytes` read from `path`.
///
/// The integer `format_version` is checked before the closed contract, so a
/// manifest of any other format (1, 2, or a later one) is refused with one
/// instruction to update and republish rather than with every field the
/// formats disagree on.
pub(super) fn read(path: &Path, bytes: Vec<u8>) -> Result<ReadManifest, ManifestRefusal> {
    let (text, loaded) = load_mapping(path, bytes).map_err(ManifestRefusal::Invalid)?;
    if let Some((version, at)) = declared_version(&loaded, &text)
        && version != i128::from(HOST_MANIFEST_FORMAT_VERSION)
    {
        return Err(ManifestRefusal::Unsupported { version, refusal: unsupported_refusal(path, version, at) });
    }
    judge(contract(), path, &text, &loaded).map_err(ManifestRefusal::Invalid)?;
    // The contract admitted the shape; this typed read refuses only what a
    // JSON Schema `const` cannot (a `format_version` written `3.0`).
    let accepted: ManifestV3 = serde_json::from_value(loaded.json.clone()).map_err(|error| {
        ManifestRefusal::Invalid(refusal(
            path,
            locate(&loaded, &text, &["format_version"]),
            format!("the manifest is not a `{HOST_MANIFEST_TYPE}` manifest: {error}"),
        ))
    })?;
    let manifest = Manifest {
        format_version: HOST_MANIFEST_FORMAT_VERSION,
        host: accepted.host,
        version: accepted.version,
        files: accepted.files,
        lifecycle: accepted.lifecycle,
        previous_pairs: Vec::new(),
        digests: accepted.digests,
    };
    Ok(ReadManifest { text, loaded, manifest })
}

/// [`read`], also accepting a well-formed format 2 manifest: what a
/// publisher may trust about the manifest it is replacing. A malformed
/// format 2 manifest is refused, so it confers no cleanup authority.
pub(super) fn read_for_cleanup(path: &Path, bytes: Vec<u8>) -> Result<ReadManifest, ManifestRefusal> {
    let (text, loaded) = load_mapping(path, bytes.clone()).map_err(ManifestRefusal::Invalid)?;
    match declared_version(&loaded, &text) {
        Some((version, _)) if version == i128::from(PREVIOUS_FORMAT_VERSION) => {}
        _ => return read(path, bytes),
    }
    let invalid = |message: String| ManifestRefusal::Invalid(refusal(path, Some(SourcePosition::START), message));
    let previous: ManifestV2 = serde_json::from_value(loaded.json.clone())
        .map_err(|error| invalid(format!("the format 2 manifest is malformed: {error}")))?;
    let well_formed = previous.kind == HOST_MANIFEST_TYPE
        && previous.format_version == PREVIOUS_FORMAT_VERSION
        && !previous.host.is_empty()
        && !previous.version.is_empty()
        && previous.digests.values().all(|digest| is_digest(digest));
    if !well_formed {
        return Err(invalid("the format 2 manifest is malformed".to_string()));
    }
    let manifest = Manifest {
        format_version: PREVIOUS_FORMAT_VERSION,
        host: previous.host,
        version: previous.version,
        files: previous.files,
        lifecycle: previous.lifecycle,
        previous_pairs: previous.functions.into_iter().map(|pair| (pair.catalog, pair.facts)).collect(),
        digests: previous.digests,
    };
    Ok(ReadManifest { text, loaded, manifest })
}

fn is_digest(text: &str) -> bool {
    text.len() == 64 && text.bytes().all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
}

/// The manifest's integer `format_version` and where it is written; a
/// manifest without one that writes a format 1 field is format 1, located
/// at that field. `None` when the version is absent or not an integer,
/// which the contract judges.
fn declared_version(loaded: &LoadedYaml, text: &str) -> Option<(i128, Option<SourcePosition>)> {
    match loaded.json.get("format_version") {
        Some(value) => {
            let version = value.as_u64().map(i128::from).or_else(|| value.as_i64().map(i128::from))?;
            Some((version, locate(loaded, text, &["format_version"])))
        }
        None => {
            let field = LEGACY_FIELDS.iter().find(|field| loaded.json.get(**field).is_some())?;
            Some((1, locate(loaded, text, &[field])))
        }
    }
}

fn unsupported_refusal(path: &Path, version: i128, at: Option<SourcePosition>) -> Refusal {
    refusal(
        path,
        at.or(Some(SourcePosition::START)),
        format!(
            "unsupported host registration format version {version}; update Darkmatter and the host tool, then \
             republish the registration"
        ),
    )
}

/// The facts companion spelling of the expression catalog written
/// `authored`: its last extension, `.yaml` or `.yml`, replaced by
/// `.facts.json` ([`facts_artifact_path`]), every earlier dot and the
/// directory kept (`./schemas/a.b.yaml` → `./schemas/a.b.facts.json`).
/// `None` for any other extension.
///
/// [`facts_artifact_path`]: crate::markdown::compose::expression::host::facts_artifact_path
#[must_use]
pub fn companion_spelling(authored: &str) -> Option<String> {
    let name = authored.rsplit('/').next().unwrap_or(authored);
    let (stem, extension) = name.rsplit_once('.')?;
    if stem.is_empty() || !CATALOG_EXTENSIONS.contains(&extension) {
        return None;
    }
    let derived = crate::markdown::compose::expression::host::facts_artifact_path(Path::new(authored));
    derived.to_str().map(str::to_string)
}

impl ReadManifest {
    /// The 1-based line and column of the value at `segments`.
    pub fn locate(&self, segments: &[&str]) -> Option<SourcePosition> {
        locate(&self.loaded, &self.text, segments)
    }

    /// The manifest's explicit member entries in authored order (each
    /// `files` item, `lifecycle`, then a format 2 manifest's function
    /// pairs), resolved, checked, and matched with their digests. Stray
    /// digests are judged once the companions are known ([`Self::check_digests`]).
    ///
    /// The bundle root is the manifest's directory. A member must be written
    /// `./…` or `../…` with forward slashes, its names drawn from
    /// [`portable_name`]'s set, and it must stay inside the root after `.`
    /// and `..` are applied. Two entries naming one file, or names that
    /// differ only in ASCII case, are refused: a bundle is copied between
    /// file systems that disagree on both. Reads nothing.
    pub fn members(&self, manifest: &Path, roots: &MemberRoots<'_>) -> Result<MemberSpec, Refusal> {
        let mut entries: Vec<(Vec<String>, &str)> = Vec::new();
        for (index, file) in self.manifest.files.iter().enumerate() {
            entries.push((vec!["files".to_string(), index.to_string()], file));
        }
        entries.push((vec!["lifecycle".to_string()], &self.manifest.lifecycle));
        for (index, (catalog, facts)) in self.manifest.previous_pairs.iter().enumerate() {
            entries.push((vec!["functions".to_string(), index.to_string(), "catalog".to_string()], catalog));
            entries.push((vec!["functions".to_string(), index.to_string(), "facts".to_string()], facts));
        }

        let mut members: Vec<BundleMember> = Vec::with_capacity(entries.len());
        for (segments, authored) in entries {
            let segment_refs: Vec<&str> = segments.iter().map(String::as_str).collect();
            let field = super::field(&segment_refs);
            let at = self.locate(&segment_refs);
            let refuse = member_refusal(manifest, &field, at.as_ref());
            let (path, location) = roots.normalize(manifest, authored).map_err(&refuse)?;
            let Some(digest) = self.manifest.digests.get(authored) else {
                return Err(refuse(format!("`{authored}` has no entry in `digests`; every member lists its digest")));
            };
            let member = BundleMember { field: field.clone(), reference: authored.to_string(), path, location, at: at.clone(), digest: digest.clone() };
            check_identity(manifest, &members, &member)?;
            members.push(member);
        }

        let mut members = members.into_iter();
        let files: Vec<BundleMember> = members.by_ref().take(self.manifest.files.len()).collect();
        let lifecycle = members.next().ok_or_else(|| refusal(manifest, None, "the manifest has no `lifecycle`".to_string()))?;
        Ok(MemberSpec { files, lifecycle, previous: members.collect() })
    }

    /// The facts companion the expression catalog `catalog` (a `files`
    /// member) derives, checked against every member already known
    /// (`known`): its spelling ([`companion_spelling`]), its `digests` entry,
    /// and its identity. Refused at the catalog's `files` entry, naming the
    /// expected companion.
    pub fn companion(
        &self,
        manifest: &Path,
        roots: &MemberRoots<'_>,
        catalog: &BundleMember,
        known: &[&BundleMember],
    ) -> Result<BundleMember, Refusal> {
        let refuse = member_refusal(manifest, &catalog.field, catalog.at.as_ref());
        let Some(spelling) = companion_spelling(&catalog.reference) else {
            return Err(refuse(format!(
                "`{}` is an expression catalog, whose file name must end in `.yaml` or `.yml` so its facts \
                 companion can be named",
                catalog.reference
            )));
        };
        let Some(digest) = self.manifest.digests.get(&spelling) else {
            return Err(refuse(format!(
                "`{}` is an expression catalog, so `digests` must list its facts companion `{spelling}`",
                catalog.reference
            )));
        };
        let (path, location) = roots.normalize(manifest, &spelling).map_err(&refuse)?;
        if let Some(listed) = known.iter().find(|member| member.path == path) {
            let message = if listed.field.starts_with("files[") && listed.reference != catalog.reference {
                format!(
                    "`{}` derives the facts companion `{spelling}`, which `{}` (`{}`) also names; a companion is \
                     derived from its catalog and never listed in `files`, and no two catalogs may share one",
                    catalog.reference, listed.field, listed.reference
                )
            } else {
                format!("`{}` derives the facts companion `{spelling}`, which `{}` already names", catalog.reference, listed.field)
            };
            return Err(refuse(message));
        }
        let member = BundleMember { field: catalog.field.clone(), reference: spelling, path, location, at: catalog.at.clone(), digest: digest.clone() };
        check_identity(manifest, known, &member)?;
        Ok(member)
    }

    /// Refuses a `digests` key that names no member in `members`: every key
    /// is a member's reference as written or a derived companion's spelling.
    /// A key that is the companion spelling of a `files` item whose kind is
    /// unknown (`unclassified`) is tolerated: that item is refused itself.
    pub fn check_digests(&self, manifest: &Path, members: &[&BundleMember], unclassified: &[&BundleMember]) -> Result<(), Refusal> {
        let possible = |key: &str| unclassified.iter().any(|member| companion_spelling(&member.reference).as_deref() == Some(key));
        let Some(stray) = self
            .manifest
            .digests
            .keys()
            .find(|key| !members.iter().any(|member| member.reference == **key) && !possible(key))
        else {
            return Ok(());
        };
        Err(Refusal {
            code: HOST_REGISTRATION_CODE,
            file: manifest.to_path_buf(),
            at: self.locate(&["digests", stray]),
            member: Some(format!("digests.{stray}")),
            message: format!(
                "`digests` lists `{stray}`, which no member entry names and no expression catalog derives; list each \
                 member's digest, and each catalog companion's, only"
            ),
            related: Vec::new(),
        })
    }

    /// The companions a lexical admission ([`super::bundle::admit_manifest`])
    /// can know without reading any member: each `digests` key that is the
    /// companion spelling of a `files` item.
    pub fn lexical_companions(&self, manifest: &Path, roots: &MemberRoots<'_>, files: &[BundleMember]) -> Result<Vec<BundleMember>, Refusal> {
        let mut companions = Vec::new();
        for file in files {
            if let Some(spelling) = companion_spelling(&file.reference)
                && let Some(digest) = self.manifest.digests.get(&spelling)
            {
                let refuse = member_refusal(manifest, &file.field, file.at.as_ref());
                let (path, location) = roots.normalize(manifest, &spelling).map_err(&refuse)?;
                companions.push(BundleMember { field: file.field.clone(), reference: spelling, path, location, at: file.at.clone(), digest: digest.clone() });
            }
        }
        Ok(companions)
    }
}

/// A refusal at a member's manifest entry.
fn member_refusal<'a>(manifest: &'a Path, field: &'a str, at: Option<&'a SourcePosition>) -> impl Fn(String) -> Refusal + 'a {
    move |message| Refusal {
        code: HOST_REGISTRATION_CODE,
        file: manifest.to_path_buf(),
        at: at.cloned(),
        member: Some(field.to_string()),
        message,
        related: Vec::new(),
    }
}

/// Refuses `member` when it names the file an earlier member names, or a
/// path that differs from an earlier one only in letter case.
fn check_identity(manifest: &Path, earlier: &[impl std::borrow::Borrow<BundleMember>], member: &BundleMember) -> Result<(), Refusal> {
    let refuse = member_refusal(manifest, &member.field, member.at.as_ref());
    if let Some(earlier) = earlier.iter().map(std::borrow::Borrow::borrow).find(|earlier| earlier.path == member.path) {
        return Err(refuse(format!(
            "`{}` names `{}`, which `{}` (`{}`) already names; each bundle file is listed once",
            member.reference, member.path, earlier.field, earlier.reference
        )));
    }
    let names: Vec<&str> = member.path.split('/').collect();
    for depth in 1..=names.len() {
        let prefix = names[..depth].join("/");
        let folded = prefix.to_ascii_lowercase();
        for other in earlier.iter().map(std::borrow::Borrow::borrow) {
            let other_names: Vec<&str> = other.path.split('/').collect();
            if other_names.len() < depth {
                continue;
            }
            let spelled = other_names[..depth].join("/");
            if spelled.to_ascii_lowercase() == folded && spelled != prefix {
                return Err(refuse(format!(
                    "`{prefix}` differs only in letter case from `{spelled}` (named by `{}`); a bundle must load the \
                     same on file systems that ignore case",
                    other.field
                )));
            }
        }
    }
    Ok(())
}

/// Where a manifest's members are: an installed bundle, resolved from the
/// manifest's directory in a request context, or a bundle verified from
/// memory, whose members are their spellings under a virtual root.
pub(super) enum MemberRoots<'a> {
    /// Members resolve in a context derived from this one, anchored at the
    /// manifest.
    Installed(&'a FileResolutionContext),
    /// Members are named lexically; nothing is probed.
    Lexical,
}

impl MemberRoots<'_> {
    fn normalize(&self, manifest: &Path, authored: &str) -> Result<(String, PathBuf), String> {
        match self {
            Self::Installed(context) => {
                let anchor = super::anchored_at(context, manifest);
                let root = PathIdentity::new(manifest.parent().unwrap_or(manifest));
                normalize_member(authored, &anchor, &root)
            }
            Self::Lexical => lexical_member(authored, manifest.parent().unwrap_or(manifest)),
        }
    }
}

/// A manifest's explicit members, checked and matched with their digests.
pub(super) struct MemberSpec {
    pub files: Vec<BundleMember>,
    pub lifecycle: BundleMember,
    /// A format 2 manifest's function pair members, catalog then facts.
    pub previous: Vec<BundleMember>,
}

/// The bundle-relative identity of the member `authored` and where it is
/// looked for, or why it is not a member.
fn normalize_member(
    authored: &str,
    anchor: &FileResolutionContext,
    root: &PathIdentity,
) -> Result<(String, PathBuf), String> {
    let reference = check_member_spelling(authored)?;
    let resolution = reference.resolve_detailed(anchor);
    let Some(location) = resolution.probed_paths().last() else {
        let why = resolution.error().map_or_else(|| "nothing was probed".to_string(), ToString::to_string);
        return Err(format!("`{authored}` cannot be resolved: {why}"));
    };
    let route = PathIdentity::new(&location).relative_from(root).filter(|route| route.parent_hops() == 0);
    let Some(route) = route.filter(|route| !route.forward().is_empty()) else {
        return Err(leaves_bundle(authored));
    };
    let path = route.forward().iter().map(|name| name.to_string_lossy()).collect::<Vec<_>>().join("/");
    Ok((path, location))
}

/// [`normalize_member`] for an embedded bundle rooted at the virtual
/// directory `root`: `.` and `..` are applied to the spelling itself.
fn lexical_member(authored: &str, root: &Path) -> Result<(String, PathBuf), String> {
    check_member_spelling(authored)?;
    let mut names: Vec<&str> = Vec::new();
    for name in authored.split('/') {
        match name {
            "." => {}
            ".." => {
                names.pop().ok_or_else(|| leaves_bundle(authored))?;
            }
            name => names.push(name),
        }
    }
    if names.is_empty() {
        return Err(leaves_bundle(authored));
    }
    let location = names.iter().fold(root.to_path_buf(), |path, name| path.join(name));
    Ok((names.join("/"), location))
}

fn leaves_bundle(authored: &str) -> String {
    format!("`{authored}` leaves the bundle: members sit in the manifest's directory or below it")
}

/// The member spelling `authored` as a reference, once it is local, written
/// `./…` or `../…` with forward slashes, and made of portable names.
fn check_member_spelling(authored: &str) -> Result<FileReference, String> {
    let reference =
        FileReference::new(authored).map_err(|error| format!("`{authored}` is not a file reference: {error}"))?;
    if reference.class().kind == FileReferenceKind::Url {
        return Err(format!("`{authored}` is remote; registration reads only local files and fetches nothing"));
    }
    if authored.contains('\\') {
        return Err(format!("`{authored}` uses `\\`; write a member with forward slashes"));
    }
    let rest = authored
        .strip_prefix("./")
        .or_else(|| authored.strip_prefix("../").map(|_| authored))
        .ok_or_else(|| format!("`{authored}` must start with `./` or `../`: a member is written from the manifest's directory"))?;
    let names: Vec<&str> = rest.split('/').collect();
    for (index, name) in names.iter().enumerate() {
        let last = index + 1 == names.len();
        match *name {
            "" => return Err(format!("`{authored}` has an empty path segment")),
            "." | ".." if !last => {}
            "." | ".." => return Err(format!("`{authored}` names a directory, not a file")),
            name => portable_name(name).map_err(|why| format!("`{authored}`: `{name}` {why}"))?,
        }
    }
    Ok(reference)
}

/// Whether `name` is a file or directory name every supported file system
/// stores and reads back unchanged: ASCII letters, digits, `.`, `_`, and
/// `-`; not ending in `.` (Windows drops it); not a Windows device name
/// (`CON`, `PRN`, `AUX`, `NUL`, `COM1`–`COM9`, `LPT1`–`LPT9`, with any
/// extension, in any case).
fn portable_name(name: &str) -> Result<(), &'static str> {
    if !name.bytes().all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-')) {
        return Err("uses a character outside `A-Z`, `a-z`, `0-9`, `.`, `_`, and `-`");
    }
    if name.ends_with('.') {
        return Err("ends with `.`, which Windows drops");
    }
    let stem = name.split('.').next().unwrap_or(name).to_ascii_uppercase();
    let device = matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || ((stem.starts_with("COM") || stem.starts_with("LPT"))
            && stem.len() == 4
            && matches!(stem.as_bytes()[3], b'1'..=b'9'));
    if device {
        return Err("is a Windows device name");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_shipped_document_is_the_text_the_namespace_serves() {
        let shipped = crate::markdown::schemas::embedded::SHIPPED_SOURCES
            .partial("host")
            .expect("`darkmatter:host` names a shipped document");
        assert_eq!((shipped.path, shipped.text), (HOST_MANIFEST_SCHEMA_PATH, HOST_MANIFEST_SCHEMA_SOURCE));
        contract();
    }

    #[test]
    fn a_companion_replaces_only_the_last_yaml_extension() {
        for (catalog, companion) in [
            ("./schemas/project.yaml", Some("./schemas/project.facts.json")),
            ("./schemas/project.yml", Some("./schemas/project.facts.json")),
            ("./a.b.c.yaml", Some("./a.b.c.facts.json")),
            ("../host/x.yaml", Some("../host/x.facts.json")),
            ("./schemas/project.json", None),
            ("./schemas/project", None),
            ("./schemas/.yaml", None),
            ("./schemas/project.YAML", None),
        ] {
            assert_eq!(companion_spelling(catalog).as_deref(), companion, "{catalog}");
        }
    }

    #[test]
    fn portable_names_refuse_what_one_supported_file_system_would_change() {
        for name in ["a.yaml", ".hidden", "my_type-2.yaml", "console.yaml", "com10.yaml", "lpt0.txt"] {
            assert_eq!(portable_name(name), Ok(()), "{name}");
        }
        for name in ["CON", "con.yaml", "Aux.json", "nul.tar.gz", "COM1", "lpt9.md", "a.", "a b", "a\u{e9}", "a:b"] {
            assert!(portable_name(name).is_err(), "{name}");
        }
    }
}
