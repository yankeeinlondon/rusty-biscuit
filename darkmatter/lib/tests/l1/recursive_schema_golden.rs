//! The golden corpus of schema lowering: frozen inputs keep their lowered
//! output byte for byte unless resolver code intentionally changes the
//! representation.
//!
//! Acyclic output is not promised byte-stable across such a change. A cyclic
//! named type, and a structured named type used at two or more authored
//! sites, is stored once in `$defs` and referenced with a local `$ref` at
//! each use, so a change in what the resolver shares moves goldens without
//! changing what they accept. The behavior tests prove that semantic
//! preservation; these goldens only pin the bytes.
//!
//! `tests/fixtures/recursive_schema_golden/manifest.yaml` lists every entry:
//! a frozen input under `inputs/` and how it is resolved. Each entry's lowered
//! JSON Schema, serialized with `serde_json::to_string_pretty` plus a newline,
//! must equal `golden/<id>.json` byte for byte, or, when its compact
//! serialization exceeds [`FULL_GOLDEN_LIMIT`], the length and BLAKE3 digest
//! of that compact text in `golden/<id>.digest`. Inputs are copies, so
//! only resolver code can move a golden; the compiled-in baseline and reserved
//! `@darkmatter:` partials are the exception and are read live.
//!
//! `DARKMATTER_BLESS_RECURSIVE_GOLDEN=1` rewrites the goldens instead of
//! comparing. That is legitimate only for an intentional representation
//! change, recorded with its cause, after the behavior tests pass.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};

use biscuit_file::{FileResolutionContext, canonicalize_simplified};
use darkmatter::markdown::{
    Markdown,
    compose::{RequestSnapshot, build_resolution_context},
    schemas::{
        DarkmatterSchemas, EmbeddedSchemaBundle, darkmatter_base_json_schema,
        parse_trigger_envelope_from_str,
        resolve::{ResolvedSchema, resolve_schema_with_roots, resolve_yaml_schema, resolve_yaml_schema_with_roots},
    },
};
use serde::Deserialize;
use serde_json::Value;
use serde_yaml_ng::Value as YamlValue;

/// The largest compact serialization stored in full (pretty-printed); a
/// larger one is stored as the length and BLAKE3 digest of its compact text.
/// Digesting the compact form keeps key order in the comparison while
/// sparing a debug build from pretty-printing and hashing tens of megabytes.
const FULL_GOLDEN_LIMIT: usize = 128 * 1024;

const BLESS_VARIABLE: &str = "DARKMATTER_BLESS_RECURSIVE_GOLDEN";

/// Replaces the fixture directory in a lowered schema, so goldens do not
/// depend on where the checkout lives.
const FIXTURES_PLACEHOLDER: &str = "<FIXTURES>";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    entries: Vec<Entry>,
    imports_only: Vec<ImportOnly>,
    skipped: Vec<Skipped>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    id: String,
    kind: Kind,
    /// Relative to `inputs/`; absent only for [`Kind::EmbeddedBaseline`].
    input: Option<String>,
    /// The repository path `input` was copied from, `authored`, or (for the
    /// embedded baseline) a description of the compiled-in source.
    source: String,
    /// The top-level key holding an inline root or root union.
    key: Option<String>,
    /// Why an authored input exists and what it mirrors.
    note: Option<String>,
    baseline: Option<Baseline>,
    /// Embedded trigger bundle files, relative to `inputs/`.
    #[serde(default)]
    bundle: Vec<String>,
    /// A test of its own for an entry too slow to share one; see
    /// [`GROUP_TESTS`].
    group: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ImportOnly {
    /// Relative to `inputs/`.
    path: String,
    /// The entry whose resolution reads it.
    by: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Skipped {
    source: String,
    reason: String,
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "kebab-case")]
enum Kind {
    SchemaFile,
    PureSchemaDocument,
    InlineRoot,
    RootUnion,
    TriggerPayload,
    EmbeddedBaseline,
    Effective,
}

const ALL_KINDS: [Kind; 7] = [
    Kind::SchemaFile,
    Kind::PureSchemaDocument,
    Kind::InlineRoot,
    Kind::RootUnion,
    Kind::TriggerPayload,
    Kind::EmbeddedBaseline,
    Kind::Effective,
];

/// The test that checks `kind`'s goldens. Exhaustive, so a new kind cannot
/// be added without a test.
fn kind_test(kind: Kind) -> &'static str {
    match kind {
        Kind::SchemaFile => "schema_file_entries_match_their_goldens",
        Kind::PureSchemaDocument => "pure_schema_document_entries_match_their_goldens",
        Kind::InlineRoot => "inline_root_entries_match_their_goldens",
        Kind::RootUnion => "root_union_entries_match_their_goldens",
        Kind::TriggerPayload => "trigger_payload_entries_match_their_goldens",
        Kind::EmbeddedBaseline => "embedded_baseline_entries_match_their_goldens",
        Kind::Effective => "effective_entries_match_their_goldens",
    }
}

/// `(kind, group, test)`: the entries of a manifest `group`, each of which
/// lowers to megabytes and would push a shared test past the L1 time budget,
/// get a test of their own.
const GROUP_TESTS: &[(Kind, &str, &str)] = &[
    (Kind::SchemaFile, "claudine-sequence-types", "schema_file_claudine_sequence_types_matches_its_golden"),
    (Kind::SchemaFile, "claudine-lifecycle", "schema_file_claudine_lifecycle_matches_its_golden"),
    (Kind::TriggerPayload, "claudine-claudine", "trigger_payload_claudine_claudine_matches_its_golden"),
    (Kind::TriggerPayload, "claudine-sequence", "trigger_payload_claudine_sequence_matches_its_golden"),
    (Kind::Effective, "claudine-inline-compose", "effective_claudine_inline_compose_matches_its_golden"),
    (Kind::Effective, "claudine-document-schema", "effective_claudine_document_schema_matches_its_golden"),
    (Kind::Effective, "claudine-sequence", "effective_claudine_sequence_matches_its_golden"),
];

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq, Hash)]
#[serde(rename_all = "kebab-case")]
enum Baseline {
    Darkmatter,
    None,
}

/// Names inside the fixture directory, kept out of path literals so CI's
/// test-input index attributes the corpus to each test's own
/// fixture-directory literal rather than to a helper (which would widen the
/// cell to the whole test binary).
const MANIFEST_FILE: &str = concat!("manifest", ".yaml");
const INPUTS_DIR: &str = concat!("in", "puts");
const GOLDEN_DIR: &str = concat!("gold", "en");

fn load_manifest(fixtures: &Path) -> Manifest {
    let path = fixtures.join(MANIFEST_FILE);
    let text = fs::read_to_string(&path).unwrap_or_else(|error| panic!("{} is unreadable: {error}", path.display()));
    serde_yaml_ng::from_str(&text).unwrap_or_else(|error| panic!("{} is malformed: {error}", path.display()))
}

/// `path` relative to `base`, `/`-separated; `None` outside `base`.
fn relative(path: &Path, base: &Path) -> Option<String> {
    let rest = path.strip_prefix(base).ok()?;
    Some(
        rest.components()
            .map(|component| component.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/"),
    )
}

fn files_below(dir: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(current) = pending.pop() {
        let entries =
            fs::read_dir(&current).unwrap_or_else(|error| panic!("{} is unreadable: {error}", current.display()));
        for entry in entries {
            let path = entry.expect("directory entry").path();
            if path.is_dir() {
                pending.push(path);
            } else {
                found.push(path);
            }
        }
    }
    found.sort();
    found
}

/// The YAML value of `key` in a YAML file, or in a Markdown file's
/// frontmatter.
fn declared_value(path: &Path, key: &str) -> YamlValue {
    let text = fs::read_to_string(path).unwrap_or_else(|error| panic!("{} is unreadable: {error}", path.display()));
    let yaml = if path.extension().is_some_and(|extension| extension == "md") {
        text.strip_prefix("---\n")
            .and_then(|rest| rest.find("\n---").map(|end| &rest[..=end]))
            .unwrap_or_else(|| panic!("{} has no frontmatter", path.display()))
            .to_string()
    } else {
        text
    };
    let document: YamlValue =
        serde_yaml_ng::from_str(&yaml).unwrap_or_else(|error| panic!("{} is not YAML: {error}", path.display()));
    document
        .as_mapping()
        .and_then(|map| map.get(YamlValue::String(key.into())))
        .cloned()
        .unwrap_or_else(|| panic!("{} declares no `{key}`", path.display()))
}

/// One lowered entry: the JSON Schema and the files its resolution read.
struct Lowered {
    json_schema: Value,
    dependencies: Vec<PathBuf>,
}

impl From<ResolvedSchema> for Lowered {
    fn from(resolved: ResolvedSchema) -> Self {
        let dependencies = resolved
            .referenced_files
            .into_iter()
            .chain(resolved.imports)
            .chain(resolved.examples)
            .collect();
        Self {
            json_schema: resolved.json_schema,
            dependencies,
        }
    }
}

/// Builds and reuses the effective-schema configurations one test needs.
struct EffectiveApis {
    scratch: PathBuf,
    built: HashMap<(Baseline, Vec<String>), DarkmatterSchemas>,
}

impl EffectiveApis {
    fn new(scratch: &Path) -> Self {
        Self {
            scratch: scratch.to_path_buf(),
            built: HashMap::new(),
        }
    }

    /// The Darkmatter API with `baseline` and an embedded trigger bundle of
    /// `bundle`, discovering triggers in an empty folder with an empty home
    /// so the bundle is the only trigger source.
    fn api(&mut self, inputs: &Path, baseline: Baseline, bundle: &[String]) -> Result<&DarkmatterSchemas, String> {
        let key = (baseline, bundle.to_vec());
        if !self.built.contains_key(&key) {
            let root = self.scratch.join(format!("effective-{}", self.built.len()));
            fs::create_dir_all(root.join("home")).map_err(|error| error.to_string())?;
            let snapshot = RequestSnapshot::new(&root)
                .with_home(Some(root.join("home")))
                .with_env(HashMap::new());
            let context = build_resolution_context(&snapshot).map_err(|error| format!("{error:?}"))?;
            let mut api = DarkmatterSchemas::new(context);
            if baseline == Baseline::Darkmatter {
                api = api.with_darkmatter_baseline_json_schema().map_err(|error| error.to_string())?;
            }
            api = api.with_trigger_discovery().map_err(|error| error.to_string())?;
            if !bundle.is_empty() {
                api = api
                    .with_embedded_trigger_bundle(leaked_bundle(inputs, bundle)?)
                    .map_err(|error| error.to_string())?;
            }
            self.built.insert(key.clone(), api);
        }
        Ok(&self.built[&key])
    }
}

/// An embedded bundle of the frozen `files`, leaked because a bundle holds
/// `'static` text. A test process builds at most a few.
fn leaked_bundle(inputs: &Path, files: &[String]) -> Result<EmbeddedSchemaBundle, String> {
    let mut documents = Vec::new();
    for file in files {
        let path = inputs.join(file);
        let text = fs::read_to_string(&path).map_err(|error| format!("{}: {error}", path.display()))?;
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| format!("{file} has no UTF-8 file name"))?;
        documents.push((&*Box::leak(name.to_string().into_boxed_str()), &*Box::leak(text.into_boxed_str())));
    }
    Ok(EmbeddedSchemaBundle::new("golden", Box::leak(documents.into_boxed_slice())))
}

/// Resolves `entry` the way its kind says.
fn lower(
    entry: &Entry,
    inputs: &Path,
    context: &FileResolutionContext,
    apis: &mut EffectiveApis,
) -> Result<Lowered, String> {
    if entry.kind == Kind::EmbeddedBaseline {
        return Ok(Lowered {
            json_schema: darkmatter_base_json_schema(),
            dependencies: Vec::new(),
        });
    }
    let input = inputs.join(entry.input.as_deref().ok_or("the entry names no input")?);
    let dir = input.parent().ok_or("the input has no directory")?;
    let file_name = input
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or("the input has no UTF-8 file name")?;
    let key = entry.key.as_deref().unwrap_or("$schema");
    match entry.kind {
        Kind::SchemaFile | Kind::PureSchemaDocument => {
            resolve_schema_with_roots(&serde_json::json!(format!("./{file_name}")), dir, &[], context)
                .map(Lowered::from)
                .map_err(|error| error.to_string())
        }
        Kind::InlineRoot | Kind::RootUnion => {
            let value = declared_value(&input, key);
            let expected = if entry.kind == Kind::InlineRoot {
                value.is_mapping()
            } else {
                value.is_sequence()
            };
            if !expected {
                return Err(format!("`{key}` is not a {:?} value", entry.kind));
            }
            resolve_yaml_schema(&value, dir, context)
                .map(Lowered::from)
                .map_err(|error| error.to_string())
        }
        Kind::TriggerPayload => {
            let text = fs::read_to_string(&input).map_err(|error| error.to_string())?;
            let envelope = parse_trigger_envelope_from_str(&text)
                .map_err(|error| error.to_string())?
                .ok_or("not a `kind: schema-trigger` envelope")?;
            let payload = envelope.payload.ok_or("the trigger has no `$schema` payload")?;
            resolve_yaml_schema_with_roots(&payload, dir, &[dir.to_path_buf()], context)
                .map(Lowered::from)
                .map_err(|error| error.to_string())
        }
        Kind::Effective => {
            let baseline = entry.baseline.ok_or("an effective entry names its `baseline`")?;
            let api = apis.api(inputs, baseline, &entry.bundle)?;
            let markdown = Markdown::try_from(input.as_path()).map_err(|error| error.to_string())?;
            let effective = api
                .effective_for(&markdown)
                .map_err(|error| error.to_string())?
                .ok_or("no schema applies to the document")?;
            Ok(Lowered {
                json_schema: (*effective.json_schema).clone(),
                dependencies: effective.dependencies().to_vec(),
            })
        }
        Kind::EmbeddedBaseline => unreachable!("handled above"),
    }
}

/// The spellings of `dir` a lowered schema could carry.
fn path_spellings(dir: &Path) -> Vec<String> {
    let mut spellings: Vec<String> = [Some(dir.to_path_buf()), canonicalize_simplified(dir).ok()]
        .into_iter()
        .flatten()
        .map(|path| path.to_string_lossy().into_owned())
        .collect();
    spellings.sort();
    spellings.dedup();
    // Longest first, so a spelling that extends another is replaced whole.
    spellings.sort_by_key(|spelling| std::cmp::Reverse(spelling.len()));
    spellings
}

/// Replaces every spelling of the fixture directory in `value`'s strings and
/// keys with [`FIXTURES_PLACEHOLDER`], normalizing that string's separators
/// to `/`. Returns the JSON pointer of each rewritten location.
fn normalize_paths(value: &mut Value, spellings: &[String], path: &mut Vec<String>, rewritten: &mut Vec<String>) {
    let rewrite = |text: &str| -> Option<String> {
        let spelling = spellings.iter().find(|spelling| text.contains(spelling.as_str()))?;
        Some(text.replace(spelling.as_str(), FIXTURES_PLACEHOLDER).replace('\\', "/"))
    };
    let pointer = |path: &[String]| format!("/{}", path.join("/"));
    match value {
        Value::String(text) => {
            if let Some(normalized) = rewrite(text) {
                *text = normalized;
                rewritten.push(pointer(path));
            }
        }
        Value::Array(items) => {
            for (index, item) in items.iter_mut().enumerate() {
                path.push(index.to_string());
                normalize_paths(item, spellings, path, rewritten);
                path.pop();
            }
        }
        Value::Object(map) => {
            if map.keys().any(|key| rewrite(key).is_some()) {
                // Rebuild in order so a renamed key keeps its position.
                let old = std::mem::take(map);
                for (key, child) in old {
                    let key = match rewrite(&key) {
                        Some(normalized) => {
                            rewritten.push(format!("{} (key)", pointer(path)));
                            normalized
                        }
                        None => key,
                    };
                    map.insert(key, child);
                }
            }
            for (key, child) in map.iter_mut() {
                path.push(key.clone());
                normalize_paths(child, spellings, path, rewritten);
                path.pop();
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) => {}
    }
}

/// The serialized golden text of `value`.
fn serialize(value: &Value) -> String {
    let mut text = serde_json::to_string_pretty(value).expect("a JSON value serializes");
    text.push('\n');
    text
}

fn digest_text(compact: &str) -> String {
    format!("bytes: {}\nblake3: {}\n", compact.len(), biscuit_hash::blake3_hash(compact))
}

fn bless_requested() -> bool {
    std::env::var(BLESS_VARIABLE).is_ok_and(|value| value == "1")
}

/// Compares `value` (whose compact serialization is `compact`) with `id`'s
/// golden, or writes the golden when blessing.
fn check_golden(golden: &Path, id: &str, value: &Value, compact: &str) -> Result<(), String> {
    let full = golden.join(format!("{id}.json"));
    let digest = golden.join(format!("{id}.digest"));
    let (path, expected_text, stale) = if compact.len() <= FULL_GOLDEN_LIMIT {
        (&full, serialize(value), &digest)
    } else {
        (&digest, digest_text(compact), &full)
    };
    if bless_requested() {
        fs::create_dir_all(golden).map_err(|error| error.to_string())?;
        fs::write(path, &expected_text).map_err(|error| format!("{}: {error}", path.display()))?;
        if stale.exists() {
            fs::remove_file(stale).map_err(|error| format!("{}: {error}", stale.display()))?;
        }
        return Ok(());
    }
    let Ok(stored) = fs::read_to_string(path) else {
        return Err(format!(
            "{id}: no golden at {}; run with {BLESS_VARIABLE}=1 only if this lowering is intended",
            path.display()
        ));
    };
    if stored == expected_text {
        return Ok(());
    }
    let serialized = serialize(value);
    let actual = std::env::temp_dir().join(format!("recursive_schema_golden-{id}.actual.json"));
    let saved = match fs::write(&actual, &serialized) {
        Ok(()) => format!("actual output written to {}", actual.display()),
        Err(error) => format!("actual output could not be saved: {error}"),
    };
    let detail = if path == &full {
        let line = stored
            .lines()
            .zip(serialized.lines())
            .position(|(stored, actual)| stored != actual)
            .unwrap_or_else(|| stored.lines().count().min(serialized.lines().count()));
        format!(
            "first difference at line {}: golden {:?}, actual {:?}",
            line + 1,
            stored.lines().nth(line).unwrap_or("<end>"),
            serialized.lines().nth(line).unwrap_or("<end>"),
        )
    } else {
        format!("digest golden {stored:?}, actual {expected_text:?}")
    };
    Err(format!("{id}: lowering changed; {detail}; {saved}"))
}

/// Whether `serialized` names any of `spellings`, raw or JSON-escaped.
fn names_any(serialized: &str, spellings: &[String]) -> bool {
    spellings.iter().any(|spelling| {
        serialized.contains(spelling.as_str()) || serialized.contains(spelling.replace('\\', "\\\\").as_str())
    })
}

/// Lowers every entry of `kind` in `group` (`None`: entries naming no group)
/// and compares it with its golden. Every dependency must be a frozen input
/// the manifest accounts for.
fn check_group(fixtures: &Path, kind: Kind, group: Option<&str>) {
    let manifest = load_manifest(fixtures);
    let inputs = fixtures.join(INPUTS_DIR);
    let golden = fixtures.join(GOLDEN_DIR);
    let canonical_inputs = canonicalize_simplified(&inputs).expect("inputs/ exists");
    let accounted: BTreeSet<&str> = manifest
        .entries
        .iter()
        .filter_map(|entry| entry.input.as_deref())
        .chain(manifest.entries.iter().flat_map(|entry| entry.bundle.iter().map(String::as_str)))
        .collect();
    let spellings = path_spellings(fixtures);
    let scratch = tempfile::tempdir().expect("scratch dir");
    let scratch_spellings = path_spellings(scratch.path());
    let context = FileResolutionContext::new(scratch.path());
    let mut apis = EffectiveApis::new(scratch.path());

    let mut failures = Vec::new();
    let mut checked = 0usize;
    let selected = manifest
        .entries
        .iter()
        .filter(|entry| entry.kind == kind && entry.group.as_deref() == group);
    for entry in selected {
        checked += 1;
        let mut lowered = match lower(entry, &inputs, &context, &mut apis) {
            Ok(lowered) => lowered,
            Err(error) => {
                failures.push(format!("{}: does not resolve: {error}", entry.id));
                continue;
            }
        };
        let mut read = BTreeSet::new();
        for dependency in &lowered.dependencies {
            let canonical = canonicalize_simplified(dependency).unwrap_or_else(|_| dependency.clone());
            match relative(&canonical, &canonical_inputs) {
                Some(path) => {
                    read.insert(path);
                }
                None => failures.push(format!(
                    "{}: read {} outside the frozen inputs",
                    entry.id,
                    dependency.display()
                )),
            }
        }
        for path in &read {
            let declared = accounted.contains(path.as_str())
                || manifest.imports_only.iter().any(|import| import.path == *path);
            if !declared {
                failures.push(format!(
                    "{}: reads `{path}`, which is neither an entry input nor an `imports_only` path",
                    entry.id
                ));
            }
        }
        for import in manifest.imports_only.iter().filter(|import| import.by == entry.id) {
            if !read.contains(&import.path) {
                failures.push(format!("{}: never reads its `imports_only` path `{}`", entry.id, import.path));
            }
        }

        let mut compact = serde_json::to_string(&lowered.json_schema).expect("a JSON value serializes");
        // Walking a multi-megabyte schema is slow in a debug build, so only a
        // schema whose text names the fixture directory is walked.
        if names_any(&compact, &spellings) {
            let mut rewritten = Vec::new();
            normalize_paths(&mut lowered.json_schema, &spellings, &mut Vec::new(), &mut rewritten);
            eprintln!("{}: normalized the fixture path at {rewritten:?}", entry.id);
            compact = serde_json::to_string(&lowered.json_schema).expect("a JSON value serializes");
        }
        if names_any(&compact, &spellings) || names_any(&compact, &scratch_spellings) {
            failures.push(format!("{}: the lowered schema still names a local path", entry.id));
        }
        if let Err(error) = check_golden(&golden, &entry.id, &lowered.json_schema, &compact) {
            failures.push(error);
        }
    }
    assert!(checked > 0, "the manifest has no {kind:?} entry in group {group:?}");
    assert!(
        failures.is_empty(),
        "{} of {checked} {kind:?} entries in group {group:?} failed:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn schema_file_entries_match_their_goldens() {
    let fixtures = biscuit_test_harness::manifest_dir!().join("tests/fixtures/recursive_schema_golden");
    check_group(&fixtures, Kind::SchemaFile, None);
}

#[test]
fn schema_file_claudine_sequence_types_matches_its_golden() {
    let fixtures = biscuit_test_harness::manifest_dir!().join("tests/fixtures/recursive_schema_golden");
    check_group(&fixtures, Kind::SchemaFile, Some("claudine-sequence-types"));
}

#[test]
fn schema_file_claudine_lifecycle_matches_its_golden() {
    let fixtures = biscuit_test_harness::manifest_dir!().join("tests/fixtures/recursive_schema_golden");
    check_group(&fixtures, Kind::SchemaFile, Some("claudine-lifecycle"));
}

#[test]
fn pure_schema_document_entries_match_their_goldens() {
    let fixtures = biscuit_test_harness::manifest_dir!().join("tests/fixtures/recursive_schema_golden");
    check_group(&fixtures, Kind::PureSchemaDocument, None);
}

#[test]
fn inline_root_entries_match_their_goldens() {
    let fixtures = biscuit_test_harness::manifest_dir!().join("tests/fixtures/recursive_schema_golden");
    check_group(&fixtures, Kind::InlineRoot, None);
}

#[test]
fn root_union_entries_match_their_goldens() {
    let fixtures = biscuit_test_harness::manifest_dir!().join("tests/fixtures/recursive_schema_golden");
    check_group(&fixtures, Kind::RootUnion, None);
}

#[test]
fn trigger_payload_entries_match_their_goldens() {
    let fixtures = biscuit_test_harness::manifest_dir!().join("tests/fixtures/recursive_schema_golden");
    check_group(&fixtures, Kind::TriggerPayload, None);
}

#[test]
fn trigger_payload_claudine_claudine_matches_its_golden() {
    let fixtures = biscuit_test_harness::manifest_dir!().join("tests/fixtures/recursive_schema_golden");
    check_group(&fixtures, Kind::TriggerPayload, Some("claudine-claudine"));
}

#[test]
fn trigger_payload_claudine_sequence_matches_its_golden() {
    let fixtures = biscuit_test_harness::manifest_dir!().join("tests/fixtures/recursive_schema_golden");
    check_group(&fixtures, Kind::TriggerPayload, Some("claudine-sequence"));
}

#[test]
fn embedded_baseline_entries_match_their_goldens() {
    let fixtures = biscuit_test_harness::manifest_dir!().join("tests/fixtures/recursive_schema_golden");
    check_group(&fixtures, Kind::EmbeddedBaseline, None);
}

#[test]
fn effective_entries_match_their_goldens() {
    let fixtures = biscuit_test_harness::manifest_dir!().join("tests/fixtures/recursive_schema_golden");
    check_group(&fixtures, Kind::Effective, None);
}

#[test]
fn effective_claudine_inline_compose_matches_its_golden() {
    let fixtures = biscuit_test_harness::manifest_dir!().join("tests/fixtures/recursive_schema_golden");
    check_group(&fixtures, Kind::Effective, Some("claudine-inline-compose"));
}

#[test]
fn effective_claudine_document_schema_matches_its_golden() {
    let fixtures = biscuit_test_harness::manifest_dir!().join("tests/fixtures/recursive_schema_golden");
    check_group(&fixtures, Kind::Effective, Some("claudine-document-schema"));
}

#[test]
fn effective_claudine_sequence_matches_its_golden() {
    let fixtures = biscuit_test_harness::manifest_dir!().join("tests/fixtures/recursive_schema_golden");
    check_group(&fixtures, Kind::Effective, Some("claudine-sequence"));
}

/// Every kind and every manifest group has entries and a test of the expected
/// name in this file, so no entry goes unchecked.
#[test]
fn every_manifest_kind_and_group_has_a_test() {
    let fixtures = biscuit_test_harness::manifest_dir!().join("tests/fixtures/recursive_schema_golden");
    let manifest = load_manifest(&fixtures);
    let this_file = include_str!("recursive_schema_golden.rs");
    let has_test = |test: &str| this_file.contains(&format!("#[test]\nfn {test}()"));
    for kind in ALL_KINDS {
        assert!(
            manifest.entries.iter().any(|entry| entry.kind == kind && entry.group.is_none()),
            "the manifest has no ungrouped {kind:?} entry, so {} checks nothing",
            kind_test(kind)
        );
        assert!(has_test(kind_test(kind)), "{kind:?} has no test named {}", kind_test(kind));
    }
    for (kind, group, test) in GROUP_TESTS {
        assert!(
            manifest.entries.iter().any(|entry| entry.kind == *kind && entry.group.as_deref() == Some(*group)),
            "the manifest has no {kind:?} entry in group `{group}`, so {test} checks nothing"
        );
        assert!(has_test(test), "group `{group}` has no test named {test}");
    }
    for entry in &manifest.entries {
        if let Some(group) = entry.group.as_deref() {
            assert!(
                GROUP_TESTS.iter().any(|(kind, listed, _)| *kind == entry.kind && *listed == group),
                "{}: group `{group}` of a {:?} entry has no test in GROUP_TESTS",
                entry.id,
                entry.kind
            );
        }
    }
}

/// The manifest and the fixture tree agree: ids are unique and filename-safe,
/// every input exists, every file under `inputs/` is an entry input, a bundle
/// file, or an `imports_only` path, and every golden belongs to an entry.
#[test]
fn manifest_accounts_for_every_fixture_file() {
    let fixtures = biscuit_test_harness::manifest_dir!().join("tests/fixtures/recursive_schema_golden");
    let manifest = load_manifest(&fixtures);
    let inputs = fixtures.join(INPUTS_DIR);
    let mut problems = Vec::new();

    let mut ids = BTreeSet::new();
    for entry in &manifest.entries {
        let id = entry.id.as_str();
        if !ids.insert(id) {
            problems.push(format!("duplicate id `{id}`"));
        }
        if id.is_empty() || !id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') {
            problems.push(format!("`{id}` is not a lowercase, digit, and hyphen file name"));
        }
        match (entry.kind, entry.input.as_deref()) {
            (Kind::EmbeddedBaseline, None) => {}
            (Kind::EmbeddedBaseline, Some(_)) => problems.push(format!("{id}: the embedded baseline has no input")),
            (_, None) => problems.push(format!("{id}: names no input")),
            (_, Some(input)) => {
                if !inputs.join(input).is_file() {
                    problems.push(format!("{id}: input `{input}` does not exist"));
                }
                let authored = entry.source == "authored";
                if authored != input.starts_with("authored/") {
                    problems.push(format!("{id}: only `authored/` inputs are authored"));
                }
                if !authored && entry.source != input {
                    problems.push(format!("{id}: a frozen copy keeps its source path (`{}`)", entry.source));
                }
            }
        }
        if entry.source == "authored" && entry.kind != Kind::Effective && entry.note.is_none() {
            problems.push(format!("{id}: an authored input needs a `note` naming what it mirrors"));
        }
        if entry.key.is_some() && !matches!(entry.kind, Kind::InlineRoot | Kind::RootUnion) {
            problems.push(format!("{id}: only an inline root or root union names a `key`"));
        }
        if (entry.baseline.is_some() || !entry.bundle.is_empty()) && entry.kind != Kind::Effective {
            problems.push(format!("{id}: only an effective entry names a `baseline` or `bundle`"));
        }
        for file in &entry.bundle {
            if !inputs.join(file).is_file() {
                problems.push(format!("{id}: bundle file `{file}` does not exist"));
            }
        }
    }

    let entry_inputs: BTreeSet<&str> = manifest.entries.iter().filter_map(|entry| entry.input.as_deref()).collect();
    let bundle_files: BTreeSet<&str> =
        manifest.entries.iter().flat_map(|entry| entry.bundle.iter().map(String::as_str)).collect();
    let mut imports = BTreeSet::new();
    for import in &manifest.imports_only {
        if !imports.insert(import.path.as_str()) {
            problems.push(format!("`imports_only` lists `{}` twice", import.path));
        }
        if !ids.contains(import.by.as_str()) {
            problems.push(format!("`imports_only` `{}` names no entry `{}`", import.path, import.by));
        }
        if entry_inputs.contains(import.path.as_str()) || bundle_files.contains(import.path.as_str()) {
            problems.push(format!("`imports_only` `{}` is already an entry input", import.path));
        }
        if !inputs.join(&import.path).is_file() {
            problems.push(format!("`imports_only` `{}` does not exist", import.path));
        }
    }
    for path in files_below(&inputs) {
        let relative = relative(&path, &inputs).expect("walked below inputs/");
        let named = entry_inputs.contains(relative.as_str())
            || bundle_files.contains(relative.as_str())
            || imports.contains(relative.as_str());
        if !named {
            problems.push(format!("inputs/{relative} is not named by the manifest"));
        }
    }

    let golden = fixtures.join(GOLDEN_DIR);
    for path in files_below(&golden) {
        let name = relative(&path, &golden).expect("walked below golden/");
        let owner = name
            .strip_suffix(".json")
            .or_else(|| name.strip_suffix(".digest"))
            .filter(|id| ids.contains(id));
        if owner.is_none() {
            problems.push(format!("golden/{name} belongs to no entry"));
        }
    }
    if !bless_requested() {
        for id in &ids {
            let stored = [".json", ".digest"]
                .iter()
                .filter(|suffix| golden.join(format!("{id}{suffix}")).is_file())
                .count();
            if stored != 1 {
                problems.push(format!("{id} has {stored} goldens; expected exactly one"));
            }
        }
    }
    for skipped in &manifest.skipped {
        if skipped.source.trim().is_empty() || skipped.reason.trim().is_empty() {
            problems.push(format!("a skipped input needs a source and a reason: {skipped:?}"));
        }
    }
    assert!(problems.is_empty(), "the golden manifest is inconsistent:\n{}", problems.join("\n"));
}

/// Resolving the same frozen input twice gives the same bytes, so a golden
/// mismatch is a lowering change rather than nondeterminism.
#[test]
fn repeated_resolution_is_byte_identical() {
    let fixtures = biscuit_test_harness::manifest_dir!().join("tests/fixtures/recursive_schema_golden");
    let manifest = load_manifest(&fixtures);
    let inputs = fixtures.join(INPUTS_DIR);
    let scratch = tempfile::tempdir().expect("scratch dir");
    let context = FileResolutionContext::new(scratch.path());
    let mut apis = EffectiveApis::new(scratch.path());
    let sample = [
        "content-policy-schemas-content-policy",
        "darkmatter-schemas-partials-fm",
        "validate-root-union-fileref",
        "authored-local-names-doc",
        "effective-baseline-schema-file",
    ];
    let mut outputs: BTreeMap<&str, Vec<String>> = BTreeMap::new();
    for _ in 0..2 {
        for id in sample {
            let entry = manifest
                .entries
                .iter()
                .find(|entry| entry.id == id)
                .unwrap_or_else(|| panic!("the manifest has no `{id}`"));
            let lowered = lower(entry, &inputs, &context, &mut apis).unwrap_or_else(|error| panic!("{id}: {error}"));
            outputs.entry(id).or_default().push(serialize(&lowered.json_schema));
        }
    }
    for (id, runs) in outputs {
        assert!(runs[0] == runs[1], "{id} lowered to different bytes on a second resolution");
    }
}
