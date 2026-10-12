//! The passive shipped-artifact corpus of
//! `2026-10-13-named-type-presence-and-local-names`.
//!
//! Every shipped schema artifact is walked recursively and classified: an
//! artifact is expected to load through the ordinary `$schema: ./file` path
//! unless [`EXCEPTIONS`] names it with a reason. An exception that no longer
//! matches its file (the file disappeared, gained content, or became
//! loadable) fails, so the list cannot rot. Inline `$schema` blocks in
//! shipped Markdown are parsed passively too, and those carrying named-type
//! references are resolved.
//!
//! Passivity is observed, not assumed: the effect-engine and network-attempt
//! counters (`effects-instrumentation`) must not move; a parse is repeated
//! with the artifact's path moved into an empty directory and must give the
//! same result, so no neighboring file was consulted; and every input's bytes
//! and modification time are unchanged afterwards.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use biscuit_file::FileResolutionContext;
use darkmatter::markdown::schemas::{
    SchemaDeclaration, SchemaError, StandaloneSchemaDocument, parse_property_definition,
    parse_schema_declaration, parse_standalone_schema_document, parse_trigger_envelope_from_str,
    resolve::{ResolvedSchema, resolve_schema_with_roots, resolve_yaml_schema},
};
use serde_yaml_ng::Value as YamlValue;

fn repo_root() -> PathBuf {
    biscuit_test_harness::manifest_dir!().join("../..")
}

/// How a shipped schema artifact is expected to behave. Unlisted artifacts are
/// [`Class::Loadable`].
#[derive(Clone, Copy, Debug, PartialEq)]
enum Class {
    /// Parses as a standalone schema and resolves through `$schema: ./file`.
    Loadable,
    /// An empty file reserving a name.
    Placeholder,
    /// A YAML artifact that is not a schema document; any `$schema` it carries
    /// describes the artifact itself and must parse and resolve inline.
    NotASchema,
    /// Header keys beyond `kind`/`types` make the standalone loader reject the
    /// file, but every definition it carries parses passively.
    EnvelopeRejected,
    /// A definition does not parse; `error` is part of the expected message.
    Draft { error: &'static str },
}

struct Exception {
    /// The walked directory, repository-relative.
    dir: &'static str,
    /// The file, relative to `dir`.
    file: &'static str,
    class: Class,
    #[allow(dead_code, reason = "the reason is documentation for the reader")]
    reason: &'static str,
}

const fn exception(
    dir: &'static str,
    file: &'static str,
    class: Class,
    reason: &'static str,
) -> Exception {
    Exception {
        dir,
        file,
        class,
        reason,
    }
}

/// Every shipped schema artifact that does not load through `$schema: ./file`.
const EXCEPTIONS: &[Exception] = &[
    // darkmatter/docs/schemas
    exception(
        "darkmatter/docs/schemas",
        "schema-definition.yaml",
        Class::Placeholder,
        "reserved meta-schema name that `err.yaml` declares as its `$schema`",
    ),
    exception(
        "darkmatter/docs/schemas",
        "expression-functions.yaml",
        Class::NotASchema,
        "`kind: expression-function-catalog`; its `$schema` describes the catalog",
    ),
    exception(
        "darkmatter/docs/schemas",
        "err.yaml",
        Class::EnvelopeRejected,
        "tagged envelope carries `$schema`, `name`, and `description` header keys",
    ),
    // darkmatter/schemas: authored with a `description`/`name` header the
    // standalone loader does not accept; no reader loads this tree today.
    exception(
        "darkmatter/schemas",
        "darkmatter.yaml",
        Class::EnvelopeRejected,
        "`description` and `$schema` beside `kind`",
    ),
    exception(
        "darkmatter/schemas",
        "partials/context-variables.yaml",
        Class::EnvelopeRejected,
        "`description` header key",
    ),
    exception(
        "darkmatter/schemas",
        "partials/doc.yaml",
        Class::EnvelopeRejected,
        "`description` header key",
    ),
    exception(
        "darkmatter/schemas",
        "partials/error.yaml",
        Class::EnvelopeRejected,
        "`name` and `description` header keys",
    ),
    exception(
        "darkmatter/schemas",
        "partials/tracking.yaml",
        Class::EnvelopeRejected,
        "`description` header key",
    ),
    exception(
        "darkmatter/schemas",
        "partials/style.yaml",
        Class::Draft {
            error: "`suggest` requires at least one candidate",
        },
        "`style.color` declares `suggest()` with no candidates",
    ),
    // repo-root schemas/
    exception(
        "schemas",
        "memory.yaml",
        Class::NotASchema,
        "stub `kind: schema-trigger` with an empty `match:`; the trigger parser claims only `kind: trigger-schema`",
    ),
    // claudine/schemas
    exception(
        "claudine/schemas",
        "claudine.yaml",
        Class::Placeholder,
        "reserved for Claudine's global-variable schema",
    ),
    exception(
        "claudine/schemas",
        "partials/lifecycle.yaml",
        Class::EnvelopeRejected,
        "`description` header key",
    ),
    // claudine/docs/schemas: design drafts, not loaded by any reader.
    exception(
        "claudine/docs/schemas",
        "claudine.yaml",
        Class::Placeholder,
        "draft placeholder",
    ),
    exception(
        "claudine/docs/schemas",
        "compose.yaml",
        Class::Placeholder,
        "draft placeholder",
    ),
    exception(
        "claudine/docs/schemas",
        "inline-compose.yaml",
        Class::Placeholder,
        "draft placeholder",
    ),
    exception(
        "claudine/docs/schemas",
        "partials/sequence.yaml",
        Class::Placeholder,
        "draft placeholder",
    ),
    exception(
        "claudine/docs/schemas",
        "partials/communication.yaml",
        Class::EnvelopeRejected,
        "draft with a `description` header key",
    ),
    exception(
        "claudine/docs/schemas",
        "partials/loop.yaml",
        Class::EnvelopeRejected,
        "draft spelling its catalog `typed:`",
    ),
    exception(
        "claudine/docs/schemas",
        "action.yaml",
        Class::Draft {
            error: "found `Ident(\"action\")`",
        },
        "prose inside a `$description` expression",
    ),
    exception(
        "claudine/docs/schemas",
        "partials/flow-control.yaml",
        Class::Draft {
            error: "found `Ident(\"file\")`",
        },
        "draft function-signature syntax",
    ),
    exception(
        "claudine/docs/schemas",
        "partials/stack.yaml",
        Class::Draft {
            error: "expected a type name",
        },
        "spread syntax `[ ...stack_item[] ]`; also names the missing `shell@./shell.yaml`",
    ),
    exception(
        "claudine/docs/schemas",
        "partials/lifecycle.yaml",
        Class::Draft { error: "got null" },
        "event names without definitions",
    ),
    exception(
        "claudine/docs/schemas",
        "partials/side-effect.yaml",
        Class::Draft {
            error: "unexpected character `:`",
        },
        "draft call syntax",
    ),
    exception(
        "claudine/docs/schemas",
        "partials/mutations.yaml",
        Class::Draft {
            error: "unknown constraint `parameters(...)`",
        },
        "draft `parameters(...)` constraint",
    ),
    exception(
        "claudine/docs/schemas",
        "partials/functions.yaml",
        Class::Draft {
            error: "unknown constraint `parameters(...)`",
        },
        "draft `parameters(...)` constraint; local `number`/`date` collide with built-ins",
    ),
];

struct Artifact {
    dir: &'static str,
    /// Relative to `dir`, `/`-separated.
    file: String,
    path: PathBuf,
    source: String,
    class: Class,
}

impl Artifact {
    fn label(&self) -> String {
        format!("{}/{}", self.dir, self.file)
    }
}

/// Files below `dir` with extension `ext`, recursively and sorted. Build output,
/// dependencies, and fixture directories (`tests/fixtures`, a spike's `fixtures`)
/// are not shipped and are skipped.
fn files_below(dir: &Path, ext: &str) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(current) = pending.pop() {
        let entries = fs::read_dir(&current)
            .unwrap_or_else(|error| panic!("{} is unreadable: {error}", current.display()));
        for entry in entries {
            let path = entry.expect("directory entry").path();
            let name = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or_default();
            if path.is_dir() {
                if !matches!(name, "fixtures" | "target" | "node_modules" | ".git") {
                    pending.push(path);
                }
            } else if path.extension().is_some_and(|extension| extension == ext) {
                found.push(path);
            }
        }
    }
    found.sort();
    found
}

fn relative(path: &Path, base: &Path) -> String {
    path.strip_prefix(base)
        .expect("walked below its base")
        .components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

/// Classifies every `.yaml` artifact below each group and fails when an
/// exception no longer names an existing file.
fn collect_artifacts(groups: &[(&'static str, PathBuf)]) -> Vec<Artifact> {
    let mut artifacts = Vec::new();
    for (dir, path) in groups {
        for file in files_below(path, "yaml") {
            let relative = relative(&file, path);
            let class = EXCEPTIONS
                .iter()
                .find(|exception| exception.dir == *dir && exception.file == relative)
                .map_or(Class::Loadable, |exception| exception.class);
            let source = fs::read_to_string(&file)
                .unwrap_or_else(|error| panic!("{} is unreadable: {error}", file.display()));
            artifacts.push(Artifact {
                dir,
                file: relative,
                path: file,
                source,
                class,
            });
        }
    }
    for exception in EXCEPTIONS {
        assert!(
            artifacts
                .iter()
                .any(|artifact| artifact.dir == exception.dir && artifact.file == exception.file),
            "allow-listed `{}/{}` no longer exists; remove its exception",
            exception.dir,
            exception.file,
        );
    }
    artifacts
}

type Fingerprint = Vec<(PathBuf, Vec<u8>, Option<SystemTime>)>;

fn fingerprint<'a>(paths: impl IntoIterator<Item = &'a Path>) -> Fingerprint {
    paths
        .into_iter()
        .map(|path| {
            let bytes = fs::read(path).expect("fingerprinted input is readable");
            let modified = fs::metadata(path)
                .and_then(|metadata| metadata.modified())
                .ok();
            (path.to_path_buf(), bytes, modified)
        })
        .collect()
}

#[cfg(feature = "effects-instrumentation")]
fn effect_counters() -> (u64, u64) {
    (
        darkmatter::effects::engine_build_count(),
        darkmatter::effects::network_attempt_count(),
    )
}

#[cfg(not(feature = "effects-instrumentation"))]
fn effect_counters() -> (u64, u64) {
    (0, 0)
}

fn yaml_key<'a>(map: &'a YamlValue, key: &str) -> Option<&'a YamlValue> {
    map.as_mapping()?.get(YamlValue::String(key.into()))
}

/// Passively parses an artifact's `$schema` and `types` bodies: the whole body
/// through the declaration parser, and every non-`$` entry of a mapping (or of
/// each mapping arm of a root union) as a property definition. Returns the
/// number of definitions parsed, or every failure message.
fn parse_bodies(document: &YamlValue) -> Result<usize, Vec<String>> {
    let mut definitions = 0usize;
    let mut failures = Vec::new();
    for key in ["$schema", "types"] {
        let Some(body) = yaml_key(document, key) else {
            continue;
        };
        if let Err(error) = parse_schema_declaration(body) {
            failures.push(format!("{key}: {error}"));
        }
        let mappings: Vec<&serde_yaml_ng::Mapping> = match body {
            YamlValue::Mapping(map) => vec![map],
            YamlValue::Sequence(arms) => arms.iter().filter_map(YamlValue::as_mapping).collect(),
            _ => Vec::new(),
        };
        for map in mappings {
            for (name, value) in map {
                let Some(name) = name.as_str() else { continue };
                if name.starts_with('$') {
                    continue;
                }
                match parse_property_definition(name, value) {
                    Ok(_) => definitions += 1,
                    Err(error) => failures.push(format!("{key}.{name}: {error}")),
                }
            }
        }
    }
    if failures.is_empty() {
        Ok(definitions)
    } else {
        Err(failures)
    }
}

/// The standalone parse of `source`, reduced to what must not depend on the
/// path: the parsed product, or the error message.
fn standalone_outcome(
    source: &str,
    path: &Path,
) -> Result<Option<StandaloneSchemaDocument>, String> {
    match parse_standalone_schema_document(source, path) {
        Ok(document) => Ok(document.map(|mut document| {
            document.path = PathBuf::new();
            document
        })),
        Err(SchemaError::SchemaDocument { message, .. }) => Err(message),
        Err(other) => Err(other.to_string()),
    }
}

fn is_named_type_error(error: &SchemaError) -> bool {
    match error {
        SchemaError::UnknownNamedType { .. }
        | SchemaError::ConstraintConflict { .. }
        | SchemaError::UnionReference { .. } => true,
        SchemaError::ReferencedSchema { source, .. } => is_named_type_error(source),
        SchemaError::Aggregate { errors } => errors.iter().any(is_named_type_error),
        _ => false,
    }
}

fn assert_lowered(resolved: &ResolvedSchema, label: &str) {
    assert!(
        resolved.json_schema.is_object(),
        "{label} must lower to a JSON Schema object: {}",
        resolved.json_schema
    );
}

/// Every shipped schema artifact parses passively, or fails exactly as its
/// exception says. Parsing builds no effect engine, attempts no network
/// access, gives the same result from an empty directory, and mutates nothing.
#[test]
fn shipped_schema_artifacts_parse_passively() {
    let root = repo_root();
    let groups = [
        (
            "darkmatter/docs/schemas",
            root.join("darkmatter/docs/schemas"),
        ),
        ("darkmatter/schemas", root.join("darkmatter/schemas")),
        (
            "content-policy/schemas",
            root.join("content-policy/schemas"),
        ),
        ("schemas", root.join("schemas")),
        ("claudine/schemas", root.join("claudine/schemas")),
        (".claudine/schemas", root.join(".claudine/schemas")),
        (
            "darkmatter/example-docs/schemas",
            root.join("darkmatter/example-docs/schemas"),
        ),
        ("claudine/docs/schemas", root.join("claudine/docs/schemas")),
    ];
    let artifacts = collect_artifacts(&groups);
    // A single top-level directory is too coarse for CI's test-input index, so
    // the repository-root schemas are also named file by file.
    for file in [
        "schemas/feature-review.yaml",
        "schemas/suggestion-review.yaml",
        "schemas/memory.yaml",
    ] {
        assert!(root.join(file).is_file(), "{file} is part of the corpus");
    }
    let before = fingerprint(artifacts.iter().map(|artifact| artifact.path.as_path()));
    let counters = effect_counters();
    let scratch = tempfile::tempdir().expect("scratch dir");
    let phantom_dir = scratch.path().join("empty");

    let mut definitions = 0usize;
    let mut loadable = 0usize;
    for artifact in &artifacts {
        let label = artifact.label();
        let document: YamlValue = serde_yaml_ng::from_str(&artifact.source)
            .unwrap_or_else(|error| panic!("{label} is not YAML: {error}"));
        let outcome = standalone_outcome(&artifact.source, &artifact.path);
        let phantom = phantom_dir.join(&artifact.file);
        assert_eq!(
            standalone_outcome(&artifact.source, &phantom),
            outcome,
            "{label}: the passive parse must not depend on files near the artifact",
        );
        let bodies = parse_bodies(&document);
        match artifact.class {
            Class::Loadable => {
                assert!(
                    matches!(outcome, Ok(Some(_))),
                    "{label} must parse as a standalone schema; classify it in EXCEPTIONS if it is not one: {outcome:?}",
                );
                definitions += bodies.unwrap_or_else(|failures| panic!("{label}: {failures:#?}"));
                loadable += 1;
            }
            Class::Placeholder => {
                assert!(
                    artifact.source.trim().is_empty(),
                    "{label} is no longer an empty placeholder; reclassify it",
                );
                assert_eq!(outcome, Ok(None), "{label}");
            }
            Class::NotASchema => {
                assert_eq!(
                    outcome,
                    Ok(None),
                    "{label} is now claimed as a schema; reclassify it"
                );
                assert!(
                    matches!(parse_trigger_envelope_from_str(&artifact.source), Ok(None)),
                    "{label} is now claimed as a trigger; reclassify it",
                );
                definitions += bodies.unwrap_or_else(|failures| panic!("{label}: {failures:#?}"));
            }
            Class::EnvelopeRejected => {
                let message =
                    outcome.expect_err(&format!("{label} now loads; remove its exception"));
                assert!(message.contains("unsupported keys"), "{label}: {message}");
                definitions += bodies.unwrap_or_else(|failures| panic!("{label}: {failures:#?}"));
            }
            Class::Draft { error } => {
                let failures = bodies.expect_err(&format!("{label} now parses; reclassify it"));
                assert!(
                    failures.iter().any(|failure| failure.contains(error)),
                    "{label}: expected a failure containing `{error}`, got {failures:#?}",
                );
            }
        }
    }

    assert_eq!(
        effect_counters(),
        counters,
        "passive parsing must build no effect engine and attempt no network access"
    );
    assert!(!phantom_dir.exists(), "passive parsing must create nothing");
    assert!(
        before == fingerprint(artifacts.iter().map(|artifact| artifact.path.as_path())),
        "passive parsing must not modify any artifact",
    );
    assert!(
        artifacts.len() >= 30,
        "the shipped artifact corpus shrank to {} files",
        artifacts.len()
    );
    assert!(
        loadable >= 10,
        "only {loadable} shipped artifacts are loadable"
    );
    assert!(
        definitions >= 120,
        "the shipped corpus shrank to {definitions} definitions"
    );
}

/// Every loadable shipped schema resolves through the ordinary
/// `$schema: ./file` path, with no unknown named type or constraint conflict,
/// and lowers to JSON Schema. Rejected envelopes still resolve their bodies
/// inline without a named-type error.
#[test]
fn loadable_shipped_schemas_resolve_and_lower() {
    let root = repo_root();
    let groups = [
        (
            "darkmatter/docs/schemas",
            root.join("darkmatter/docs/schemas"),
        ),
        ("darkmatter/schemas", root.join("darkmatter/schemas")),
        (
            "content-policy/schemas",
            root.join("content-policy/schemas"),
        ),
        ("schemas", root.join("schemas")),
        ("claudine/schemas", root.join("claudine/schemas")),
        (".claudine/schemas", root.join(".claudine/schemas")),
        (
            "darkmatter/example-docs/schemas",
            root.join("darkmatter/example-docs/schemas"),
        ),
        ("claudine/docs/schemas", root.join("claudine/docs/schemas")),
    ];
    let artifacts = collect_artifacts(&groups);
    for file in [
        "schemas/feature-review.yaml",
        "schemas/suggestion-review.yaml",
        "schemas/memory.yaml",
    ] {
        assert!(root.join(file).is_file(), "{file} is part of the corpus");
    }
    // Anchored in an empty directory: references resolve from each artifact's
    // own directory, never from the process directory.
    let scratch = tempfile::tempdir().expect("scratch dir");
    let context = FileResolutionContext::new(scratch.path());

    let mut resolved_files = 0usize;
    let mut inline_bodies = 0usize;
    for artifact in &artifacts {
        let label = artifact.label();
        let dir = artifact.path.parent().expect("artifact directory");
        let reference = serde_json::json!(format!(
            "./{}",
            artifact
                .path
                .file_name()
                .and_then(|name| name.to_str())
                .expect("UTF-8 file name")
        ));
        let document: YamlValue = serde_yaml_ng::from_str(&artifact.source).expect("YAML");
        match artifact.class {
            Class::Loadable => {
                let resolved = resolve_schema_with_roots(&reference, dir, &[], &context)
                    .unwrap_or_else(|error| panic!("{label} must resolve: {error}"));
                assert!(
                    resolved.simplified.is_some(),
                    "{label} must resolve as a SimplifiedSchema"
                );
                assert_lowered(&resolved, &label);
                resolved_files += 1;
            }
            Class::EnvelopeRejected | Class::NotASchema => {
                if artifact.class == Class::EnvelopeRejected {
                    let error = resolve_schema_with_roots(&reference, dir, &[], &context)
                        .expect_err(&format!("{label} now loads; remove its exception"));
                    assert!(
                        matches!(error, SchemaError::SchemaDocument { .. }),
                        "{label}: {error}"
                    );
                }
                for key in ["$schema", "types"] {
                    let Some(body) = yaml_key(&document, key) else {
                        continue;
                    };
                    match resolve_yaml_schema(body, dir, &context) {
                        Ok(resolved) => assert_lowered(&resolved, &format!("{label} `{key}`")),
                        Err(error) => assert!(
                            !is_named_type_error(&error),
                            "{label} `{key}` resolved inline must not fail on a named type: {error}",
                        ),
                    }
                    inline_bodies += 1;
                }
            }
            Class::Placeholder | Class::Draft { .. } => {}
        }
    }
    assert!(
        resolved_files >= 10,
        "only {resolved_files} shipped schemas resolved"
    );
    assert!(
        inline_bodies >= 10,
        "only {inline_bodies} rejected-envelope bodies resolved inline"
    );
}

struct InlineSchema {
    label: String,
    path: PathBuf,
    declaration: YamlValue,
}

/// The `$schema` value of every Markdown file below `dirs` whose frontmatter
/// declares one. Files without the `$schema` needle are never YAML-decoded.
fn inline_schemas(root: &Path, dirs: &[PathBuf]) -> Vec<InlineSchema> {
    let mut found = Vec::new();
    for dir in dirs {
        for path in files_below(dir, "md") {
            let Ok(text) = fs::read_to_string(&path) else {
                continue;
            };
            if !text.contains("$schema") {
                continue;
            }
            let Some(frontmatter) = text
                .strip_prefix("---\n")
                .and_then(|rest| rest.find("\n---").map(|end| &rest[..end]))
            else {
                continue;
            };
            let Ok(frontmatter) = serde_yaml_ng::from_str::<YamlValue>(frontmatter) else {
                continue;
            };
            let Some(declaration) = yaml_key(&frontmatter, "$schema") else {
                continue;
            };
            found.push(InlineSchema {
                label: relative(&path, root),
                declaration: declaration.clone(),
                path,
            });
        }
    }
    found
}

fn has_named_reference(declaration: &SchemaDeclaration) -> bool {
    matches!(declaration, SchemaDeclaration::Schema(_))
        && format!("{declaration:?}").contains("Imported {")
}

/// Every inline `$schema` in shipped documentation, and in the darkmatter,
/// claudine, and messenger planning trees (where every inline named-type
/// reference lives), parses passively. The internal `prompts/` folders and
/// fixture directories are outside the scope.
#[test]
fn shipped_markdown_inline_schemas_parse_passively() {
    let root = repo_root();
    let dirs = [
        root.join("darkmatter/docs"),
        root.join("darkmatter/example-docs"),
        root.join("claudine/docs"),
        root.join("content-policy/docs"),
        root.join("biscuit-terminal/docs"),
        root.join("biscuit-file/docs"),
        root.join("sniff/docs"),
        root.join("homelab/docs"),
        root.join("docs/topics"),
        root.join("docs/cicd"),
        root.join(".claude/skills"),
        root.join("darkmatter/features"),
        root.join("darkmatter/fixes"),
        root.join("claudine/features"),
        root.join("claudine/fixes"),
        root.join("messenger/features"),
    ];
    // A spike probe recording that an unquoted parenthesized pattern does not
    // parse.
    let expected_failures = [(
        "messenger/features/2026-09-17-research-metadata-pipeline/spikes/schema-pilots/probes/p12_pattern_parens.md",
        "expected `,` or `)` in argument list",
    )];
    let schemas = inline_schemas(&root, &dirs);
    let before = fingerprint(schemas.iter().map(|schema| schema.path.as_path()));
    let counters = effect_counters();

    let mut named = Vec::new();
    let mut failed = Vec::new();
    for schema in &schemas {
        match parse_schema_declaration(&schema.declaration) {
            Ok(declaration) => {
                if has_named_reference(&declaration) {
                    named.push(schema.label.as_str());
                }
            }
            Err(error) => {
                let expected = expected_failures
                    .iter()
                    .find(|(label, _)| *label == schema.label)
                    .unwrap_or_else(|| {
                        panic!("{} does not parse passively: {error}", schema.label)
                    });
                assert!(
                    error.to_string().contains(expected.1),
                    "{}: {error}",
                    schema.label
                );
                failed.push(schema.label.as_str());
            }
        }
    }

    assert_eq!(
        effect_counters(),
        counters,
        "passive parsing must build no effect engine and attempt no network access"
    );
    assert!(
        before == fingerprint(schemas.iter().map(|schema| schema.path.as_path())),
        "passive parsing must not modify any document",
    );
    let expected_failed: Vec<&str> = expected_failures.iter().map(|(label, _)| *label).collect();
    assert_eq!(
        failed, expected_failed,
        "an expected failure now parses; remove it"
    );
    for label in [
        "darkmatter/docs/research/parsing/_fleet_future.md",
        "messenger/features/2026-09-17-research-metadata-pipeline/spikes/schema-pilots/probes/p14_multiline_import.md",
        "messenger/features/2026-09-17-research-metadata-pipeline/spikes/schema-pilots/probes/p15_one_line.md",
        "messenger/features/2026-09-17-research-metadata-pipeline/spikes/schema-pilots/probes/p15_single_quoted.md",
    ] {
        assert!(
            named.contains(&label),
            "{label} must parse with a named-type reference; found {named:#?}"
        );
    }
    assert!(
        schemas.len() >= 500,
        "the inline-schema corpus shrank to {}",
        schemas.len()
    );
}

/// Every shipped inline `$schema` carrying a named-type reference resolves
/// without a constraint conflict. The only unknown name is
/// `_fleet_future.md`'s bare `crate`, which that document defines under
/// `$types:` rather than in its `$schema`.
#[test]
fn shipped_markdown_named_type_references_resolve() {
    let root = repo_root();
    let dirs = [
        root.join("darkmatter/docs"),
        root.join("darkmatter/example-docs"),
        root.join("claudine/docs"),
        root.join("content-policy/docs"),
        root.join("biscuit-terminal/docs"),
        root.join("biscuit-file/docs"),
        root.join("sniff/docs"),
        root.join("homelab/docs"),
        root.join("docs/topics"),
        root.join("docs/cicd"),
        root.join(".claude/skills"),
        root.join("darkmatter/features"),
        root.join("darkmatter/fixes"),
        root.join("claudine/features"),
        root.join("claudine/fixes"),
        root.join("messenger/features"),
    ];
    let fleet_future = "darkmatter/docs/research/parsing/_fleet_future.md";
    let scratch = tempfile::tempdir().expect("scratch dir");
    let context = FileResolutionContext::new(scratch.path());

    let mut resolved = 0usize;
    for schema in inline_schemas(&root, &dirs) {
        let Ok(declaration) = parse_schema_declaration(&schema.declaration) else {
            continue;
        };
        if !has_named_reference(&declaration) {
            continue;
        }
        let dir = schema.path.parent().expect("document directory");
        let outcome = resolve_yaml_schema(&schema.declaration, dir, &context);
        if schema.label == fleet_future {
            match outcome {
                Err(SchemaError::UnknownNamedType { name, .. }) => assert_eq!(name, "crate"),
                other => panic!("{fleet_future} must report the unknown `crate`: {other:?}"),
            }
        } else {
            match outcome {
                Ok(resolved) => assert_lowered(&resolved, &schema.label),
                Err(error) => assert!(
                    !is_named_type_error(&error),
                    "{} must not fail on a named type: {error}",
                    schema.label
                ),
            }
        }
        resolved += 1;
    }
    assert!(
        resolved >= 4,
        "only {resolved} inline schemas carry named-type references"
    );
}

/// Research contracts are the largest shipped users of named types, and they
/// live outside any `schemas/` directory as `_schema.yaml`, `_types.yaml`, and
/// `*.schema.yaml`. Each must parse passively, resolve through the ordinary
/// `$schema: ./file` path with no named-type error, and lower to JSON Schema.
#[test]
fn shipped_research_contracts_resolve_and_lower() {
    let root = repo_root();
    let dirs = [
        root.join("claudine/docs/research"),
        root.join("messenger/docs"),
    ];
    let contracts: Vec<PathBuf> = dirs
        .iter()
        .flat_map(|dir| files_below(dir, "yaml"))
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| {
                    matches!(name, "_schema.yaml" | "_types.yaml") || name.ends_with(".schema.yaml")
                })
        })
        .collect();
    let before = fingerprint(contracts.iter().map(PathBuf::as_path));
    let counters = effect_counters();
    let scratch = tempfile::tempdir().expect("scratch dir");
    let context = FileResolutionContext::new(scratch.path());

    let mut definitions = 0usize;
    let mut named_references = 0usize;
    for path in &contracts {
        let label = relative(path, &root);
        let source = fs::read_to_string(path).expect("contract is readable");
        let document: YamlValue = serde_yaml_ng::from_str(&source)
            .unwrap_or_else(|error| panic!("{label} is not YAML: {error}"));
        definitions += parse_bodies(&document)
            .unwrap_or_else(|failures| panic!("{label} does not parse passively: {failures:#?}"));
        named_references += source.matches("@this").count() + source.matches("@./").count()
            + source.matches("@../").count();

        let reference = serde_json::json!(format!(
            "./{}",
            path.file_name().and_then(|name| name.to_str()).expect("UTF-8 file name")
        ));
        let dir = path.parent().expect("contract directory");
        let resolved = resolve_schema_with_roots(&reference, dir, &[], &context)
            .unwrap_or_else(|error| panic!("{label} must resolve: {error}"));
        assert!(
            resolved.simplified.is_some(),
            "{label} must resolve as a SimplifiedSchema"
        );
        assert_lowered(&resolved, &label);
    }

    assert!(
        contracts.len() >= 30,
        "the research contract corpus shrank to {} files",
        contracts.len()
    );
    assert!(
        definitions >= 550,
        "the research contract corpus shrank to {definitions} definitions"
    );
    assert!(
        named_references >= 330,
        "the research contracts carry only {named_references} named-type references"
    );
    assert_eq!(effect_counters(), counters, "resolving a contract ran an effect");
    assert_eq!(
        fingerprint(contracts.iter().map(PathBuf::as_path)),
        before,
        "resolving a contract mutated it"
    );
}
