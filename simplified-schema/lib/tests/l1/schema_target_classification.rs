//! What a document's `$schema` may target, observed through
//! `resolve_schema_with_roots`: a YAML document with a top-level `$schema`
//! (informal, or `kind: schema`), or a JSON object declaring exactly the
//! Draft 2020-12 dialect. Every other shape is a typed error naming the file.
//!
//! Each cell is one edit of a control fixture ([`INFORMAL`], [`TAGGED`],
//! [`SCHEMA_JSON`]), so every edit is known to be the load-bearing one.

#![cfg(feature = "file")]

use std::fs;
use std::path::{Path, PathBuf};

use biscuit_file::{FileResolutionContext, canonicalize_simplified};
use serde_json::json;
use simplified_schema::embedded::ShippedSources;
use simplified_schema::resolve::{ResolvedSchema, resolve_schema_with_roots};
use simplified_schema::{NotSchemaReason, SchemaError, parse_standalone_schema_document};

/// An informal envelope with every key it may carry.
const INFORMAL: &str = "description: Feature reviews\n$schema:\n  title: string(required)\n  id: Id\ntypes:\n  Id: string\n";
/// A tagged envelope with every key it may carry.
const TAGGED: &str = "kind: schema\ndescription: Reviews\ncategories: [Core]\n$schema:\n  title: string\ntypes:\n  Id: string(category(Core))\n";
/// A Draft 2020-12 JSON Schema.
const SCHEMA_JSON: &str = r#"{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "properties": { "title": { "type": "string" } },
  "required": ["title"]
}
"#;
const DIALECT: &str = "https://json-schema.org/draft/2020-12/schema";

struct Fixture {
    _tmp: tempfile::TempDir,
    dir: PathBuf,
    context: FileResolutionContext,
}

impl Fixture {
    fn new() -> Self {
        let tmp = tempfile::tempdir().unwrap();
        let dir = canonicalize_simplified(tmp.path()).unwrap();
        let context = FileResolutionContext::new(&dir);
        Self { _tmp: tmp, dir, context }
    }

    fn write(&self, name: &str, text: &str) -> PathBuf {
        let path = self.dir.join(name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&path, text).unwrap();
        path
    }

    /// Resolves `$schema: {reference}` for a document in the fixture folder.
    fn resolve(&self, reference: &str) -> Result<ResolvedSchema, SchemaError> {
        self.resolve_with_roots(reference, &[])
    }

    fn resolve_with_roots(&self, reference: &str, roots: &[PathBuf]) -> Result<ResolvedSchema, SchemaError> {
        resolve_schema_with_roots(&json!(reference), &self.dir, roots, &self.context, ShippedSources::NONE)
    }

    /// Writes `text` as `name` and resolves `$schema: ./{name}`.
    fn target(&self, name: &str, text: &str) -> (PathBuf, Result<ResolvedSchema, SchemaError>) {
        let path = self.write(name, text);
        (path, self.resolve(&format!("./{name}")))
    }
}

/// The first `NotASchemaDocument` in `error`'s source chain, so an error a
/// delegation or root union wraps is still found with the file it names.
fn not_schema(error: &SchemaError) -> Option<(PathBuf, NotSchemaReason)> {
    let mut cause: &dyn std::error::Error = error;
    loop {
        if let Some(SchemaError::NotASchemaDocument { path, reason }) = cause.downcast_ref::<SchemaError>() {
            return Some((path.clone(), *reason));
        }
        cause = cause.source()?;
    }
}

#[track_caller]
fn assert_not_schema(result: Result<ResolvedSchema, SchemaError>, path: &Path, reason: NotSchemaReason) {
    let error = result.expect_err("the target must be rejected");
    assert_eq!(not_schema(&error), Some((path.to_path_buf(), reason)), "{error}");
}

/// Asserts a `SchemaDocument` naming `path` whose message contains each of
/// `fragments`; returns the message.
#[track_caller]
fn assert_document_error(result: Result<ResolvedSchema, SchemaError>, path: &Path, fragments: &[&str]) -> String {
    match result {
        Err(SchemaError::SchemaDocument { path: actual, message, .. }) => {
            assert_eq!(actual, path, "{message}");
            for fragment in fragments {
                assert!(message.contains(fragment), "expected `{fragment}` in: {message}");
            }
            message
        }
        other => panic!("expected a SchemaDocument naming {}, got {other:?}", path.display()),
    }
}

// ── YAML: controls and the loader's document shapes ───────────────────────

#[test]
fn yaml_controls_resolve_as_simplified_schemas() {
    let fixture = Fixture::new();
    for (name, text) in [("informal.yaml", INFORMAL), ("tagged.yaml", TAGGED)] {
        let (_, result) = fixture.target(name, text);
        let resolved = result.unwrap_or_else(|error| panic!("{name}: {error}"));
        assert!(resolved.simplified.is_some(), "{name}");
        assert!(resolved.json_schema["properties"].get("title").is_some(), "{name}: {}", resolved.json_schema);
    }
}

#[test]
fn yaml_document_shapes_are_located_document_errors() {
    let fixture = Fixture::new();
    for (label, text, fragment) in [
        ("duplicate top-level key", format!("{INFORMAL}description: again\n"), "duplicate"),
        ("trailing content", format!("{INFORMAL}]\n"), "not valid YAML"),
        ("second document", format!("{INFORMAL}---\n$schema:\n  x: string\n"), "not valid YAML"),
    ] {
        let (path, result) = fixture.target(&format!("{}.yaml", label.replace(' ', "-")), &text);
        assert_document_error(result, &path, &[fragment]);
    }
}

#[test]
fn a_bare_schema_key_is_a_null_payload_error() {
    let fixture = Fixture::new();
    let (path, result) = fixture.target("bare.yaml", "description: Reviews\n$schema:\n");
    assert_document_error(result, &path, &[]);
}

// ── YAML: the classification table ────────────────────────────────────────

#[test]
fn a_non_mapping_yaml_target_is_not_a_mapping() {
    let fixture = Fixture::new();
    for (name, text) in [("list.yaml", "- a\n- b\n"), ("scalar.yaml", "42\n")] {
        let (path, result) = fixture.target(name, text);
        assert_not_schema(result, &path, NotSchemaReason::NotAMapping);
    }
}

#[test]
fn a_mapping_without_schema_is_not_a_schema_document() {
    let fixture = Fixture::new();
    for (name, text) in [
        ("plain.yaml", "title: string(required)\n"),
        ("raw-no-dialect.yaml", "type: object\nproperties:\n  x:\n    type: number\n"),
        ("trigger.yaml", "kind: schema-trigger\nmatch: path(**/*.md)\n"),
        ("described.yaml", "description: Reviews\ntypes:\n  Id: string\n"),
    ] {
        let (path, result) = fixture.target(name, text);
        assert_not_schema(result, &path, NotSchemaReason::NoSchemaProperty);
    }
}

#[test]
fn raw_json_schema_in_yaml_names_its_extra_keys_and_the_json_migration() {
    let fixture = Fixture::new();
    let (path, result) = fixture.target("raw.yaml", &format!("$schema: {DIALECT}\ntype: object\nproperties: {{}}\n"));
    assert_document_error(
        result,
        &path,
        &[
            "unsupported top-level key(s): type, properties",
            "raw JSON Schema in YAML is not a `$schema` target",
            DIALECT,
        ],
    );
}

#[test]
fn an_informal_mapping_with_an_unknown_key_is_a_document_error() {
    let fixture = Fixture::new();
    let (path, result) = fixture.target("extra.yaml", &format!("notes: draft\n{INFORMAL}"));
    let message = assert_document_error(result, &path, &["unsupported top-level key(s): notes"]);
    assert!(!message.contains("raw JSON Schema"), "{message}");
    // Another `kind` never claims the informal envelope.
    let (path, result) = fixture.target("other-kind.yaml", "kind: config\n$schema:\n  title: string\n");
    assert_document_error(result, &path, &["unsupported top-level key(s): kind"]);
}

#[test]
fn tagged_envelope_rules() {
    let fixture = Fixture::new();
    // `$schema` without `types` behaves like an informal document.
    let (_, result) = fixture.target("schema-only.yaml", "kind: schema\ndescription: Reviews\n$schema:\n  title: string\n");
    assert!(result.expect("tagged `$schema`-only").simplified.is_some());
    // Neither payload key is an envelope error.
    let (path, result) = fixture.target("neither.yaml", "kind: schema\ndescription: Reviews\n");
    assert_document_error(result, &path, &["needs a `$schema` or `types` property"]);
    // An unknown key is an envelope error.
    let (path, result) = fixture.target("unknown.yaml", &format!("notes: draft\n{TAGGED}"));
    assert_document_error(result, &path, &["found unsupported keys: notes"]);
}

#[test]
fn a_types_only_document_is_refused_as_schema_but_still_imports() {
    let fixture = Fixture::new();
    let (path, result) = fixture.target("types-only.yaml", "kind: schema\ntypes:\n  Id: string(min(2))\n");
    let error = result.expect_err("a types-only document is not a `$schema` target");
    assert_eq!(not_schema(&error), Some((path.clone(), NotSchemaReason::TypesOnly)));
    assert!(error.to_string().contains("use a named-type import (`Name@file.yaml`)"), "{error}");

    let consumer = fixture.write("consumer.yaml", "$schema:\n  id: Id@./types-only.yaml\n");
    let resolved = fixture.resolve("./consumer.yaml").expect("the import still resolves");
    assert_eq!(resolved.imports.len(), 1, "{:?}", resolved.imports);
    assert!(resolved.json_schema["properties"]["id"].to_string().contains("\"minLength\":2"), "{}", resolved.json_schema);
    assert!(consumer.is_file());
}

#[test]
fn delegation_and_root_union_arms_classify_each_hop() {
    let fixture = Fixture::new();
    let types_only = fixture.write("types-only.yaml", "kind: schema\ntypes:\n  Id: string\n");
    let no_dialect = fixture.write("raw.json", r#"{"type": "object"}"#);
    fixture.write("delegate.yaml", "$schema: ./types-only.yaml\n");
    assert_not_schema(fixture.resolve("./delegate.yaml"), &types_only, NotSchemaReason::TypesOnly);

    fixture.write("valid.yaml", "$schema:\n  title: string\n");
    fixture.write("union.yaml", "$schema:\n  - ./valid.yaml\n  - ./raw.json\n");
    assert_not_schema(fixture.resolve("./union.yaml"), &no_dialect, NotSchemaReason::JsonMissingDialect);
}

#[test]
fn a_malformed_file_in_the_first_root_is_reported_not_skipped() {
    let fixture = Fixture::new();
    let first = fixture.write("first/shared.yaml", "kind: schema\ntypes:\n  Id: string\n");
    fixture.write("second/shared.yaml", "$schema:\n  title: string\n");
    let roots = [fixture.dir.join("first"), fixture.dir.join("second")];
    assert_not_schema(fixture.resolve_with_roots("shared.yaml", &roots), &first, NotSchemaReason::TypesOnly);
    // The later root's file is valid on its own.
    assert!(fixture.resolve_with_roots("shared.yaml", &roots[1..]).is_ok());
}

// ── YAML: field shapes ────────────────────────────────────────────────────

#[test]
fn description_field_shapes() {
    let fixture = Fixture::new();
    let without = INFORMAL.replace("description: Feature reviews\n", "");
    let empty = INFORMAL.replace("Feature reviews", "''");
    for (label, text, expected) in [
        ("absent", without.as_str(), None),
        ("empty", empty.as_str(), Some("")),
        ("control", INFORMAL, Some("Feature reviews")),
    ] {
        let document = parse_standalone_schema_document(text, "s.yaml").unwrap().expect(label);
        assert_eq!(document.description.as_deref(), expected, "{label}");
        let (_, result) = fixture.target(&format!("description-{label}.yaml"), text);
        assert!(result.is_ok(), "{label}: {result:?}");
    }
    for (label, value) in [("null", "null"), ("implicit-null", ""), ("number", "5"), ("sequence", "[a]"), ("mapping", "{a: b}")] {
        let text = INFORMAL.replace("description: Feature reviews", &format!("description: {value}"));
        let (path, result) = fixture.target(&format!("description-{label}.yaml"), &text);
        assert_document_error(result, &path, &["`description` must be a string"]);
        // The tagged envelope shares the validator.
        let tagged = TAGGED.replace("description: Reviews", &format!("description: {value}"));
        let (path, result) = fixture.target(&format!("tagged-description-{label}.yaml"), &tagged);
        assert_document_error(result, &path, &["`description` must be a string"]);
    }
}

#[test]
fn schema_field_shapes_in_yaml() {
    let fixture = Fixture::new();
    for (label, value) in [("null", "null"), ("number", "5"), ("boolean", "true"), ("empty", "''")] {
        let text = format!("description: Reviews\n$schema: {value}\n");
        let (path, result) = fixture.target(&format!("schema-{label}.yaml"), &text);
        assert_document_error(result, &path, &[]);
        let tagged = format!("kind: schema\n$schema: {value}\ntypes:\n  Id: string\n");
        let (path, result) = fixture.target(&format!("tagged-schema-{label}.yaml"), &tagged);
        assert_document_error(result, &path, &[]);
    }
}

#[test]
fn types_field_shapes() {
    let fixture = Fixture::new();
    let (_, result) = fixture.target("types-empty.yaml", "$schema:\n  title: string\ntypes: {}\n");
    assert!(result.is_ok(), "{result:?}");
    for (label, value) in [("null", "null"), ("scalar", "word"), ("sequence", "[a]")] {
        let (path, result) = fixture.target(&format!("types-{label}.yaml"), &format!("$schema:\n  title: string\ntypes: {value}\n"));
        assert_document_error(result, &path, &["`types` must be a mapping"]);
        let tagged = format!("kind: schema\n$schema:\n  title: string\ntypes: {value}\n");
        let (path, result) = fixture.target(&format!("tagged-types-{label}.yaml"), &tagged);
        assert_document_error(result, &path, &["`types` must be a mapping"]);
    }
}

#[test]
fn categories_field_shapes() {
    let fixture = Fixture::new();
    let without = TAGGED.replace("categories: [Core]\n", "").replace("(category(Core))", "");
    let empty = TAGGED.replace("[Core]", "[]").replace("(category(Core))", "");
    for (label, text) in [("absent", without.as_str()), ("empty", empty.as_str())] {
        let (_, result) = fixture.target(&format!("categories-{label}.yaml"), text);
        assert!(result.is_ok(), "{label}: {result:?}");
    }
    for (label, value) in [("null", "null"), ("scalar", "Core")] {
        let text = TAGGED.replace("[Core]", value);
        let (path, result) = fixture.target(&format!("categories-{label}.yaml"), &text);
        assert_document_error(result, &path, &["`categories` must be a list"]);
    }
}

// ── JSON: control, document shapes, and the dialect ───────────────────────

#[test]
fn json_control_resolves_as_raw_json_schema() {
    let fixture = Fixture::new();
    for name in ["schema.json", "upper.JSON"] {
        let (_, result) = fixture.target(name, SCHEMA_JSON);
        let resolved = result.unwrap_or_else(|error| panic!("{name}: {error}"));
        assert!(resolved.simplified.is_none(), "{name}");
        assert_eq!(resolved.json_schema["required"], json!(["title"]), "{name}");
    }
}

#[test]
fn json_document_shapes_are_located_document_errors() {
    let fixture = Fixture::new();
    for (label, text, fragment) in [
        ("duplicate-key", SCHEMA_JSON.replace("\"type\": \"object\",", "\"type\": \"object\", \"type\": \"object\","), "duplicate"),
        ("trailing-content", format!("{SCHEMA_JSON}{{}}\n"), "trailing"),
        ("json5-comment", format!("// note\n{SCHEMA_JSON}"), "invalid JSON"),
        ("json5-trailing-comma", SCHEMA_JSON.replace("[\"title\"]", "[\"title\",]"), "invalid JSON"),
    ] {
        let (path, result) = fixture.target(&format!("{label}.json"), &text);
        assert_document_error(result, &path, &[fragment]);
    }
}

#[test]
fn json_dialect_and_root_shapes() {
    let fixture = Fixture::new();
    let declared = format!("\"$schema\": \"{DIALECT}\",");
    for (label, replacement) in [("absent", ""), ("null", "\"$schema\": null,"), ("number", "\"$schema\": 7,")] {
        let (path, result) = fixture.target(&format!("dialect-{label}.json"), &SCHEMA_JSON.replace(&declared, replacement));
        assert_not_schema(result, &path, NotSchemaReason::JsonMissingDialect);
    }
    for (label, dialect) in [
        ("empty", ""),
        ("trailing-hash", "https://json-schema.org/draft/2020-12/schema#"),
        ("draft-07", "http://json-schema.org/draft-07/schema#"),
    ] {
        let text = SCHEMA_JSON.replace(DIALECT, dialect);
        let (path, result) = fixture.target(&format!("dialect-{label}.json"), &text);
        match result {
            Err(SchemaError::UnsupportedJsonDialect { path: actual, declared }) => {
                assert_eq!(actual, path);
                assert_eq!(declared, dialect);
            }
            other => panic!("{label}: expected UnsupportedJsonDialect, got {other:?}"),
        }
    }
    for (label, text) in [("array", "[]\n"), ("scalar", "42\n")] {
        let (path, result) = fixture.target(&format!("root-{label}.json"), text);
        let error = result.expect_err(label);
        assert_eq!(not_schema(&error), Some((path, NotSchemaReason::NotAMapping)));
        assert!(error.to_string().contains(DIALECT), "{label}: the hint names the dialect: {error}");
    }
}

#[test]
fn a_declared_schema_failing_the_meta_schema_is_invalid_json_schema() {
    let fixture = Fixture::new();
    let (path, result) = fixture.target("invalid.json", &SCHEMA_JSON.replace("\"type\": \"object\"", "\"type\": 5"));
    match result {
        Err(SchemaError::InvalidJsonSchema { path: actual, message }) => {
            assert_eq!(actual, path);
            assert!(message.contains("/type"), "{message}");
        }
        other => panic!("expected InvalidJsonSchema, got {other:?}"),
    }
}
