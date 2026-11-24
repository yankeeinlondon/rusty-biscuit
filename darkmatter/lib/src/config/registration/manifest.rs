//! The `darkmatter-host` manifest envelope: its shipped schema, the
//! validation a manifest passes before any member is read, and (for format
//! 2) the normalized member specification a verified snapshot reads.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use biscuit_file::{FileReference, FileReferenceKind, FileResolutionContext, PathIdentity};
use serde::Deserialize;
use serde_json::Value;
use simplified_schema::yaml_documents::{LoadedYaml, load_yaml_document, locating_problem};
use simplified_schema::{SchemaError, SimplifiedSchema, ValidationFailure, ValidationProblem};

use super::bundle::{BundleFunctionPair, BundleMember};
use super::{HOST_REGISTRATION_CODE, Refusal};
use crate::config::load::line_column;

/// The virtual path of the shipped manifest schema.
pub const HOST_MANIFEST_SCHEMA_PATH: &str = crate::markdown::schemas::embedded::HOST_MANIFEST_SCHEMA_PATH;

/// The manifest schema's text, as shipped: the format 2 contract.
pub const HOST_MANIFEST_SCHEMA_SOURCE: &str = include_str!("../../../../schemas/darkmatter-host.yaml");

/// The named type a whole manifest satisfies.
pub const HOST_MANIFEST_TYPE: &str = "darkmatter-host";

/// The manifest format the shipped contract describes.
pub const HOST_MANIFEST_FORMAT_VERSION: u64 = 2;

/// The format 1 contract, which only the legacy profile loader
/// ([`read`]) still validates against. It is removed together with legacy
/// loading; nothing serves it as a shipped schema.
const LEGACY_MANIFEST_SCHEMA_SOURCE: &str = r#"kind: schema
description: Darkmatter's host registration manifest.
types:
    member-reference: "string(not-empty) -> A local file reference, resolved from the manifest's directory (`./schemas/globals.yaml`)."
    function-pair:
        catalog: "member-reference(required) -> The host function catalog: the signatures."
        facts: "member-reference(required) -> The facts artifact generated from the host's registrations for that catalog."
        $constraints: { closed: true }
    darkmatter-host:
        kind: "literal(darkmatter-host; required) -> The manifest envelope."
        format_version: "literal(1; required) -> The manifest format; this reader understands 1."
        host: "string(not-empty; required) -> The host's identifier; its lifecycle artifact must record the same host."
        version: "string(not-empty; required) -> The host release that wrote the manifest (not the format)."
        identify: "member-reference(required) -> The identifying schema trigger: a document it matches belongs to this host."
        triggers: "member-reference[](required) -> Schema triggers applied, in this order, only to documents the host identifies."
        globals: "member-reference(required) -> The host's global catalog."
        lifecycle: "member-reference(required) -> The host's lifecycle artifact for that catalog."
        functions: "function-pair[](required) -> Host function catalogs, each with its facts artifact."
        $constraints: { closed: true }
"#;

/// The format 1 fields format 2 removed; any of them marks a manifest
/// written for the old reader.
const LEGACY_FIELDS: [&str; 3] = ["identify", "triggers", "globals"];

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

/// The prepared format 1 contract; see [`contract`].
fn legacy_contract() -> &'static SimplifiedSchema {
    static CONTRACT: OnceLock<SimplifiedSchema> = OnceLock::new();
    CONTRACT.get_or_init(|| prepare(LEGACY_MANIFEST_SCHEMA_SOURCE))
}

fn prepare(source: &str) -> SimplifiedSchema {
    SimplifiedSchema::try_from(source)
        .and_then(|schema| schema.select(HOST_MANIFEST_TYPE))
        .unwrap_or_else(|error| panic!("the `{HOST_MANIFEST_TYPE}` type prepares: {error}"))
}

/// A format 1 manifest the legacy contract accepted. Every field is required by the schema,
/// so none is defaulted here.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Manifest {
    #[serde(rename = "kind")]
    _kind: String,
    #[serde(rename = "format_version")]
    _format_version: u64,
    pub host: String,
    pub version: String,
    pub identify: String,
    pub triggers: Vec<String>,
    pub globals: String,
    pub lifecycle: String,
    pub functions: Vec<FunctionPair>,
}

/// One `functions` item.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct FunctionPair {
    pub catalog: String,
    pub facts: String,
}

/// A manifest as read: its text, its positions, and its accepted fields.
pub(super) struct ReadManifest {
    pub text: String,
    pub loaded: LoadedYaml,
    pub manifest: Manifest,
}

impl ReadManifest {
    /// The 1-based line and column of the value at `segments`.
    pub fn locate(&self, segments: &[&str]) -> Option<(usize, usize)> {
        locate(&self.loaded, &self.text, segments)
    }

    /// The manifest as the schema judged it: its meaning, without comments
    /// or layout.
    pub fn meaning(&self) -> &Value {
        &self.loaded.json
    }
}

fn locate(loaded: &LoadedYaml, text: &str, segments: &[&str]) -> Option<(usize, usize)> {
    let segments: Vec<String> = segments.iter().map(|segment| (*segment).to_string()).collect();
    loaded.byte_range(&locating_problem(&segments)).map(|range| line_column(text, range.start))
}

/// Validates the format 1 manifest `bytes` read from `path`: one
/// admissible YAML mapping that the closed legacy contract accepts.
pub(super) fn read(path: &Path, bytes: Vec<u8>) -> Result<ReadManifest, Refusal> {
    let (text, loaded) = load_mapping(path, bytes)?;
    judge(legacy_contract(), path, &text, &loaded)?;
    // The contract admitted the shape; this typed read refuses only what a
    // JSON Schema `const` cannot (a `format_version` written `1.0`).
    let manifest: Manifest = serde_json::from_value(loaded.json.clone()).map_err(|error| {
        refusal(path, locate(&loaded, &text, &["format_version"]), format!("the manifest is not a `{HOST_MANIFEST_TYPE}` manifest: {error}"))
    })?;
    Ok(ReadManifest { text, loaded, manifest })
}

fn refusal(path: &Path, at: Option<(usize, usize)>, message: String) -> Refusal {
    Refusal { code: HOST_REGISTRATION_CODE, file: path.to_path_buf(), at, member: None, message }
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
        return Err(refusal(path, Some((1, 1)), format!("the manifest {what}; a `{HOST_MANIFEST_TYPE}` manifest is a mapping")));
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
fn problem_location(loaded: &LoadedYaml, text: &str, problem: &ValidationProblem) -> Option<(usize, usize)> {
    let mut segments: Vec<&str> = problem.instance_path.segments().iter().map(String::as_str).collect();
    if let Some(key) = unknown_key(problem) {
        segments.push(key);
    }
    locate(loaded, text, &segments).or(Some((1, 1)))
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

// ── format 2 ────────────────────────────────────────────────────────────────

/// A format 2 manifest the shipped contract accepted. Every field is
/// required by the schema, so none is defaulted here.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ManifestV2 {
    #[serde(rename = "kind")]
    _kind: String,
    #[serde(rename = "format_version")]
    _format_version: u64,
    pub host: String,
    pub version: String,
    pub files: Vec<String>,
    pub lifecycle: String,
    pub functions: Vec<FunctionPair>,
    pub digests: BTreeMap<String, String>,
}

/// Why [`read_v2`] refused a manifest.
pub(super) enum ManifestRefusal {
    /// The manifest was written for the format 1 reader.
    Format1(Refusal),
    /// Anything else: not YAML, not a mapping, or not the closed contract.
    Invalid(Refusal),
}

/// A format 2 manifest as read: its text, its positions, and its accepted
/// fields.
pub(super) struct ReadManifestV2 {
    pub text: String,
    pub loaded: LoadedYaml,
    pub manifest: ManifestV2,
}

/// Validates the format 2 manifest `bytes` read from `path`. A format 1
/// manifest is recognized before the strict contract judges it, so its
/// author is told to republish rather than shown every removed field.
pub(super) fn read_v2(path: &Path, bytes: Vec<u8>) -> Result<ReadManifestV2, ManifestRefusal> {
    let (text, loaded) = load_mapping(path, bytes).map_err(ManifestRefusal::Invalid)?;
    if is_format_1(&loaded.json) {
        return Err(ManifestRefusal::Format1(format_1_refusal(path, &text, &loaded)));
    }
    judge(contract(), path, &text, &loaded).map_err(ManifestRefusal::Invalid)?;
    // The contract admitted the shape; this typed read refuses only what a
    // JSON Schema `const` cannot (a `format_version` written `2.0`).
    let manifest: ManifestV2 = serde_json::from_value(loaded.json.clone()).map_err(|error| {
        ManifestRefusal::Invalid(refusal(
            path,
            locate(&loaded, &text, &["format_version"]),
            format!("the manifest is not a `{HOST_MANIFEST_TYPE}` manifest: {error}"),
        ))
    })?;
    Ok(ReadManifestV2 { text, loaded, manifest })
}

/// A `format_version` of 1, or a removed field beside any version but 2
/// (a format 2 manifest with a stray `identify` is a malformed format 2
/// manifest, judged by the contract).
fn is_format_1(json: &Value) -> bool {
    let version = json.get("format_version").and_then(Value::as_f64);
    version == Some(1.0) || (version != Some(2.0) && LEGACY_FIELDS.iter().any(|field| json.get(field).is_some()))
}

fn format_1_refusal(path: &Path, text: &str, loaded: &LoadedYaml) -> Refusal {
    let at = std::iter::once("format_version")
        .chain(LEGACY_FIELDS)
        .find_map(|field| loaded.json.get(field).and_then(|_| locate(loaded, text, &[field])))
        .or(Some((1, 1)));
    let repair = match loaded.json.get("host").and_then(Value::as_str).filter(|host| !host.is_empty()) {
        Some(host) => format!("run `{host} setup`, or update `{host}`, to publish a format 2 manifest"),
        None => "rerun the host program's setup, or update it, to publish a format 2 manifest".to_string(),
    };
    refusal(
        path,
        at,
        format!(
            "this is a format 1 `{HOST_MANIFEST_TYPE}` manifest, which this Darkmatter no longer reads; format 2 lists \
             every file of the host's bundle with its digest: {repair}"
        ),
    )
}

impl ReadManifestV2 {
    /// The 1-based line and column of the value at `segments`.
    pub fn locate(&self, segments: &[&str]) -> Option<(usize, usize)> {
        locate(&self.loaded, &self.text, segments)
    }

    /// Every member entry in authored order (each `files` item, `lifecycle`,
    /// then each function's `catalog` and `facts`), resolved, checked, and
    /// matched with its digest.
    ///
    /// The bundle root is the manifest's directory. A member must be written
    /// `./…` or `../…` with forward slashes, its names drawn from
    /// [`portable_name`]'s set, and it must stay inside the root after `.`
    /// and `..` are applied. Two entries naming one file, or names that
    /// differ only in ASCII case, are refused: a bundle is copied between
    /// file systems that disagree on both. Reads nothing.
    pub fn members(&self, manifest: &Path, context: &FileResolutionContext) -> Result<MemberSpec, Refusal> {
        let anchor = super::anchored_at(context, manifest);
        let root = PathIdentity::new(manifest.parent().unwrap_or(manifest));
        let mut entries: Vec<(Vec<String>, &str)> = Vec::new();
        for (index, file) in self.manifest.files.iter().enumerate() {
            entries.push((vec!["files".to_string(), index.to_string()], file));
        }
        entries.push((vec!["lifecycle".to_string()], &self.manifest.lifecycle));
        for (index, pair) in self.manifest.functions.iter().enumerate() {
            entries.push((vec!["functions".to_string(), index.to_string(), "catalog".to_string()], &pair.catalog));
            entries.push((vec!["functions".to_string(), index.to_string(), "facts".to_string()], &pair.facts));
        }

        let mut members: Vec<BundleMember> = Vec::with_capacity(entries.len());
        let mut folded: BTreeMap<String, (String, String)> = BTreeMap::new();
        for (segments, authored) in entries {
            let segment_refs: Vec<&str> = segments.iter().map(String::as_str).collect();
            let field = super::field(&segment_refs);
            let at = self.locate(&segment_refs);
            let refuse = |message: String| Refusal {
                code: HOST_REGISTRATION_CODE,
                file: manifest.to_path_buf(),
                at,
                member: Some(field.clone()),
                message,
            };
            let (path, location) = normalize_member(authored, &anchor, &root).map_err(&refuse)?;
            if let Some(earlier) = members.iter().find(|member| member.path == path) {
                return Err(refuse(format!(
                    "`{authored}` names `{path}`, which `{}` (`{}`) already names; each bundle file is listed once",
                    earlier.field, earlier.reference
                )));
            }
            let names: Vec<&str> = path.split('/').collect();
            for depth in 1..=names.len() {
                let prefix = names[..depth].join("/");
                let (spelled, by) =
                    folded.entry(prefix.to_ascii_lowercase()).or_insert_with(|| (prefix.clone(), field.clone()));
                if *spelled != prefix {
                    return Err(refuse(format!(
                        "`{prefix}` differs only in letter case from `{spelled}` (named by `{by}`); a bundle must \
                         load the same on file systems that ignore case"
                    )));
                }
            }
            let Some(digest) = self.manifest.digests.get(authored) else {
                return Err(refuse(format!("`{authored}` has no entry in `digests`; every member lists its digest")));
            };
            members.push(BundleMember {
                field: field.clone(),
                reference: authored.to_string(),
                path,
                location,
                line: at.map(|(line, _)| line),
                column: at.map(|(_, column)| column),
                digest: digest.clone(),
            });
        }
        if let Some(stray) =
            self.manifest.digests.keys().find(|key| !members.iter().any(|member| member.reference == **key))
        {
            return Err(Refusal {
                code: HOST_REGISTRATION_CODE,
                file: manifest.to_path_buf(),
                at: self.locate(&["digests", stray]),
                member: Some(format!("digests.{stray}")),
                message: format!("`digests` lists `{stray}`, which no member entry names; list each member's digest only"),
            });
        }

        let functions = self.manifest.functions.len();
        let mut members = members.into_iter();
        let files: Vec<BundleMember> = members.by_ref().take(self.manifest.files.len()).collect();
        let lifecycle = members.next().ok_or_else(|| refusal(manifest, None, "the manifest has no `lifecycle`".to_string()))?;
        let mut pairs = Vec::with_capacity(functions);
        while let (Some(catalog), Some(facts)) = (members.next(), members.next()) {
            pairs.push(BundleFunctionPair { catalog, facts });
        }
        Ok(MemberSpec { files, lifecycle, functions: pairs })
    }
}

/// A format 2 manifest's members, checked and matched with their digests.
pub(super) struct MemberSpec {
    pub files: Vec<BundleMember>,
    pub lifecycle: BundleMember,
    pub functions: Vec<BundleFunctionPair>,
}

/// The bundle-relative identity of the member `authored` and where it is
/// looked for, or why it is not a member.
fn normalize_member(
    authored: &str,
    anchor: &FileResolutionContext,
    root: &PathIdentity,
) -> Result<(String, PathBuf), String> {
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
    let resolution = reference.resolve_detailed(anchor);
    let Some(location) = resolution.probed_paths().last() else {
        let why = resolution.error().map_or_else(|| "nothing was probed".to_string(), ToString::to_string);
        return Err(format!("`{authored}` cannot be resolved: {why}"));
    };
    let route = PathIdentity::new(&location).relative_from(root).filter(|route| route.parent_hops() == 0);
    let Some(route) = route.filter(|route| !route.forward().is_empty()) else {
        return Err(format!("`{authored}` leaves the bundle: members sit in the manifest's directory or below it"));
    };
    let path = route.forward().iter().map(|name| name.to_string_lossy()).collect::<Vec<_>>().join("/");
    Ok((path, location))
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
        legacy_contract();
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
