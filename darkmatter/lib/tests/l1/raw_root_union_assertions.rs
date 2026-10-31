//! A raw JSON Schema whose root declares assertions beside `anyOf` keeps them
//! through Darkmatter's effective schema: the root union is judged arm by
//! arm, and each arm still carries the root's `required`. A root
//! `unevaluatedProperties` still sees every accepting arm's fields, so each
//! arm's verdict agrees with the complete schema's.

use std::path::{Path, PathBuf};

use darkmatter::markdown::Markdown;
use darkmatter::markdown::schemas::{DarkmatterSchemas, EffectiveSchema, ValidationReport};
use serde_json::json;
use simplified_schema::ValidationProblemCode;

use crate::request_support::context_at;

struct Fixture {
    _dir: tempfile::TempDir,
    root: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().expect("temp dir");
        let root = biscuit_file::canonicalize_simplified(dir.path()).expect("canonical temp dir");
        let schema = json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object",
            "required": ["name"],
            "anyOf": [
                {"properties": {"kind": {"const": "a"}}},
                {"properties": {"kind": {"const": "b"}}}
            ]
        });
        std::fs::write(root.join("schema.json"), serde_json::to_string_pretty(&schema).expect("schema JSON"))
            .expect("write schema");
        Self { _dir: dir, root }
    }

    fn document(&self, frontmatter: &str) -> Markdown {
        let path = self.root.join("doc.md");
        std::fs::write(&path, format!("---\n$schema: ./schema.json\n{frontmatter}---\nBody\n")).expect("write");
        Markdown::try_from(path.as_path()).expect("document loads")
    }

    fn schemas(&self) -> DarkmatterSchemas {
        DarkmatterSchemas::new(context_at(Path::new(&self.root)))
    }

    fn effective(&self) -> EffectiveSchema {
        self.schemas()
            .effective_for(&self.document("kind: a\n"))
            .expect("schema resolves")
            .expect("the document declares a schema")
    }
}

#[track_caller]
fn assert_missing_name(report: &ValidationReport) {
    assert!(!report.valid, "accepted without `name`: {report:?}");
    assert!(
        report
            .problems
            .iter()
            .any(|problem| problem.code == ValidationProblemCode::MissingRequired && problem.message.contains("name")),
        "expected a missing-required `name` problem, got {:?}",
        report.problems
    );
}

#[test]
fn effective_schema_validation_keeps_root_required_beside_any_of() {
    let fixture = Fixture::new();

    assert_missing_name(&fixture.effective().validate(&json!({"kind": "a"})));
    assert_missing_name(&fixture.schemas().validate(&fixture.document("kind: a\n")).expect("the schema loads"));
    assert!(fixture.schemas().validate(&fixture.document("kind: a\nname: Ada\n")).expect("the schema loads").valid);
}

#[test]
fn effective_schema_raw_validation_keeps_root_required_beside_any_of() {
    let fixture = Fixture::new();
    let effective = fixture.effective();

    assert_missing_name(&effective.validate_raw(&json!({"kind": "a"})));
    assert!(effective.validate_raw(&json!({"kind": "b", "name": "Ada"})).valid);
}

/// A canonical temp root holding `schema` as `schema.json` (declaring the
/// Draft 2020-12 dialect when `schema` does not) and `doc.md` declaring it
/// with `frontmatter`.
fn raw_schema_document(schema: &serde_json::Value, frontmatter: &str) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("temp dir");
    let root = biscuit_file::canonicalize_simplified(dir.path()).expect("canonical temp dir");
    let mut declared = schema.clone();
    declared
        .as_object_mut()
        .expect("schema object")
        .entry("$schema")
        .or_insert_with(|| json!(simplified_schema::DRAFT_2020_12_DIALECT));
    std::fs::write(root.join("schema.json"), serde_json::to_string_pretty(&declared).expect("schema JSON"))
        .expect("write schema");
    std::fs::write(root.join("doc.md"), format!("---\n$schema: ./schema.json\n{frontmatter}---\nBody\n"))
        .expect("write document");
    (dir, root)
}

/// The eager-`file` fragment a raw schema declares.
fn eager_file() -> serde_json::Value {
    json!({"type": "string", "format": "darkmatter-file"})
}

/// The normalized form of `instance` under `doc.md`'s effective schema.
fn normalized(root: &Path, instance: &serde_json::Value) -> serde_json::Value {
    let path = root.join("doc.md");
    let markdown = Markdown::try_from(path.as_path())
        .expect("document loads")
        .with_source(darkmatter::markdown::compose::ComposeSource::File(path.clone()));
    let effective = DarkmatterSchemas::new(context_at(root))
        .effective_for(&markdown)
        .expect("schema resolves")
        .expect("the document declares a schema");
    let report = effective.validate(instance);
    assert!(report.valid, "validation accepts {instance}: {:?}", report.problems);
    effective.normalize_frontmatter(instance, &std::collections::HashSet::new()).value
}

/// An eager `file` declared on the root's own `properties` beside `anyOf` is
/// normalized exactly as it is without the union, and so is one an arm
/// declares.
#[test]
fn eager_file_declared_beside_root_any_of_is_normalized() {
    let with_union = json!({
        "type": "object",
        "properties": {"doc": eager_file()},
        "anyOf": [
            {"properties": {"kind": {"const": "a"}, "spec": eager_file()}, "required": ["kind"]},
            {"properties": {"kind": {"const": "b"}}, "required": ["kind"]}
        ]
    });
    let without_union = json!({"type": "object", "properties": {"doc": eager_file()}});
    let instance = json!({"kind": "a", "doc": "./a.md", "spec": "./b.md"});

    for (label, schema) in [("with `anyOf`", &with_union), ("without `anyOf`", &without_union)] {
        let (_dir, root) = raw_schema_document(schema, "kind: a\n");
        std::fs::write(root.join("a.md"), "# A\n").expect("write a.md");
        std::fs::write(root.join("b.md"), "# B\n").expect("write b.md");

        let normalized = normalized(&root, &instance);

        assert_eq!(normalized["doc"], json!("a.md"), "{label}: root-declared `doc`");
        if label == "with `anyOf`" {
            assert_eq!(normalized["spec"], json!("b.md"), "{label}: arm-declared `spec`");
        }
    }
}

/// A caller's file value under a root union resolves from the caller's
/// origin when the arm only applies after a conversion declared on the
/// root's own `properties` beside `anyOf` (the caller's `count` arrives as
/// the text `"3"`).
#[test]
fn caller_file_resolves_from_its_origin_when_root_properties_convert_beside_any_of() {
    use darkmatter::markdown::compose::{CallerInputRecord, CallerInputRecords, ComposeOperation, ComposeOptions};

    let schema = json!({
        "type": "object",
        "properties": {"count": {"type": "integer"}},
        "required": ["count"],
        "anyOf": [
            {"properties": {"kind": {"const": "a"}, "doc": eager_file()}, "required": ["kind", "doc"]},
            {"properties": {"kind": {"const": "b"}}, "required": ["kind"]}
        ]
    });
    let (_dir, root) = raw_schema_document(&schema, "kind: a\n");
    let launch = root.join("launch");
    std::fs::create_dir_all(&launch).expect("create launch");
    let notes = launch.join("notes.md");
    std::fs::write(&notes, "# Notes\n").expect("write notes");
    let records: CallerInputRecords = [("count", json!("3")), ("doc", json!("notes.md"))]
        .into_iter()
        .map(|(key, raw)| (key.to_string(), CallerInputRecord::new(raw, context_at(&launch))))
        .collect();
    let path = root.join("doc.md");
    let options = ComposeOptions::new()
        .with_source_file(&path)
        .with_set_overrides(json!({"count": "3", "doc": "notes.md"}))
        .with_caller_input_records(records)
        .only(&[ComposeOperation::FrontmatterInterpolation, ComposeOperation::Interpolation]);

    let (composed, _) = Markdown::try_from(path.as_path())
        .expect("document loads")
        .compose_with(&crate::request_support::request(options))
        .expect("composes");

    let frontmatter = composed.frontmatter().as_map();
    assert_eq!(frontmatter["count"], json!(3));
    assert_eq!(
        frontmatter["doc"],
        json!(PathBuf::from_iter(notes.components()).to_string_lossy()),
        "the caller's `doc` resolves from its own origin"
    );
}

/// `md clean`'s schema-proven quoting sees a string property the root
/// declares beside `anyOf`: the authored number `1.20` under it is quoted,
/// exactly as it is without the union.
#[test]
fn clean_quotes_a_number_under_a_string_declared_beside_root_any_of() {
    use biscuit_file::{YamlCertainty, YamlDiagnosticCode, apply_edit_set};
    use darkmatter::markdown::schemas::analyze_frontmatter;

    let with_union = json!({
        "type": "object",
        "properties": {"title": {"type": "string"}},
        "anyOf": [
            {"properties": {"kind": {"const": "a"}}, "required": ["kind"]},
            {"properties": {"kind": {"const": "b"}}, "required": ["kind"]}
        ]
    });
    let without_union = json!({"type": "object", "properties": {"title": {"type": "string"}}});
    let body = "$schema: ./schema.json\nkind: a\ntitle: 1.20\n";

    for (label, schema) in [("with `anyOf`", &with_union), ("without `anyOf`", &without_union)] {
        let (_dir, root) = raw_schema_document(schema, "kind: a\n");
        let path = root.join("doc.md");
        let effective = DarkmatterSchemas::new(context_at(&root))
            .effective_for(&Markdown::try_from(path.as_path()).expect("document loads"))
            .expect("schema resolves")
            .expect("the document declares a schema");

        let analysis = analyze_frontmatter(&effective, body);

        let diagnostics = analysis.diagnostics();
        assert_eq!(diagnostics.len(), 1, "{label}: {diagnostics:#?}");
        assert_eq!(diagnostics[0].code, YamlDiagnosticCode::SchemaTypeMismatch, "{label}");
        assert_eq!(diagnostics[0].classification, YamlCertainty::Deterministic, "{label}");
        let repaired = apply_edit_set(body, &diagnostics[0].repairs).source;
        assert!(repaired.contains("title: \"1.20\"") || repaired.contains("title: '1.20'"), "{label}: {repaired}");
    }
}

/// Either alternative names the record, both may, and nothing else is
/// allowed; `extra` adds root `properties`.
fn alternatives_with_unevaluated_properties(extra: serde_json::Value) -> serde_json::Value {
    let mut schema = json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "type": "object",
        "anyOf": [
            {"properties": {"name": {"type": "string"}}, "required": ["name"]},
            {"properties": {"age": {"type": "number"}}, "required": ["age"]}
        ],
        "unevaluatedProperties": false
    });
    if !extra.is_null() {
        schema["properties"] = extra;
    }
    schema
}

fn effective_for(root: &Path) -> EffectiveSchema {
    let path = root.join("doc.md");
    DarkmatterSchemas::new(context_at(root))
        .effective_for(&Markdown::try_from(path.as_path()).expect("document loads"))
        .expect("schema resolves")
        .expect("the document declares a schema")
}

#[test]
fn effective_schema_validation_accepts_fields_of_two_accepting_arms_beside_unevaluated_properties() {
    let (_dir, root) = raw_schema_document(&alternatives_with_unevaluated_properties(json!(null)), "name: Ada\n");
    let effective = effective_for(&root);

    let report = effective.validate(&json!({"name": "Ada", "age": 36}));
    assert!(report.valid, "{:?}", report.problems);
    let report = effective.validate(&json!({"name": "Ada", "extra": true}));
    assert!(!report.valid, "an undeclared field is still rejected");
}

#[test]
fn effective_schema_raw_validation_accepts_fields_of_two_accepting_arms_beside_unevaluated_properties() {
    let (_dir, root) = raw_schema_document(&alternatives_with_unevaluated_properties(json!(null)), "name: Ada\n");
    let effective = effective_for(&root);

    let report = effective.validate_raw(&json!({"name": "Ada", "age": 36}));
    assert!(report.valid, "{:?}", report.problems);
}

#[test]
fn document_with_fields_of_two_accepting_arms_beside_unevaluated_properties_is_valid() {
    let (_dir, root) =
        raw_schema_document(&alternatives_with_unevaluated_properties(json!(null)), "name: Ada\nage: 36\n");
    let path = root.join("doc.md");

    let report = DarkmatterSchemas::new(context_at(&root))
        .validate(&Markdown::try_from(path.as_path()).expect("document loads"))
        .expect("the schema loads");

    assert!(report.valid, "{:?}", report.problems);
}

#[test]
fn eager_file_beside_a_union_restricting_unevaluated_properties_is_normalized() {
    let schema = alternatives_with_unevaluated_properties(json!({"doc": eager_file()}));
    let (_dir, root) = raw_schema_document(&schema, "name: Ada\n");
    std::fs::write(root.join("a.md"), "# A\n").expect("write a.md");

    let normalized = normalized(&root, &json!({"name": "Ada", "age": 36, "doc": "./a.md"}));

    assert_eq!(normalized["doc"], json!("a.md"));
}

/// A caller's file value beside a root union restricting unevaluated fields
/// composes from the caller's origin, and a caller's `count` text converts
/// against the root's own `properties`.
#[test]
fn caller_file_beside_a_union_restricting_unevaluated_properties_composes_from_its_origin() {
    use darkmatter::markdown::compose::{CallerInputRecord, CallerInputRecords, ComposeOperation, ComposeOptions};

    let schema = alternatives_with_unevaluated_properties(json!({"doc": eager_file(), "count": {"type": "integer"}}));
    let (_dir, root) = raw_schema_document(&schema, "name: Ada\nage: 36\n");
    let launch = root.join("launch");
    std::fs::create_dir_all(&launch).expect("create launch");
    let notes = launch.join("a.md");
    std::fs::write(&notes, "# A\n").expect("write a.md");
    let records: CallerInputRecords = [("count", json!("3")), ("doc", json!("a.md"))]
        .into_iter()
        .map(|(key, raw)| (key.to_string(), CallerInputRecord::new(raw, context_at(&launch))))
        .collect();
    let path = root.join("doc.md");
    let options = ComposeOptions::new()
        .with_source_file(&path)
        .with_set_overrides(json!({"count": "3", "doc": "a.md"}))
        .with_caller_input_records(records)
        .only(&[ComposeOperation::FrontmatterInterpolation, ComposeOperation::Interpolation]);

    let (composed, _) = Markdown::try_from(path.as_path())
        .expect("document loads")
        .compose_with(&crate::request_support::request(options))
        .expect("composes");

    let frontmatter = composed.frontmatter().as_map();
    assert_eq!((&frontmatter["name"], &frontmatter["age"]), (&json!("Ada"), &json!(36)));
    assert_eq!(frontmatter["count"], json!(3));
    assert_eq!(
        frontmatter["doc"],
        json!(PathBuf::from_iter(notes.components()).to_string_lossy()),
        "the caller's `doc` resolves from its own origin"
    );
}
