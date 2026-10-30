//! Public resolution evidence for a named import (`name@./file.yaml`) whose
//! target file cannot supply named types. The standalone schema classifier
//! is the only authority on the target's YAML, so each failure is the
//! classifier's error, located at the imported file.

use std::path::{Path, PathBuf};

use biscuit_file::FileResolutionContext;
use darkmatter::markdown::schemas::SchemaError;
use darkmatter::markdown::schemas::resolve::{ResolvedSchema, resolve_yaml_schema};
use serde_yaml_ng::Value as YamlValue;

/// Writes `types.yaml` with `body` and resolves an inline schema importing
/// `name` from it, the way a document in `dir` would.
fn import_from(dir: &Path, body: &str) -> (PathBuf, Result<ResolvedSchema, SchemaError>) {
    let types = dir.join("types.yaml");
    std::fs::write(&types, body).expect("write imported file");
    let schema: YamlValue = serde_yaml_ng::from_str("value: name@./types.yaml\n").expect("fixture YAML parses");
    (types.clone(), resolve_yaml_schema(&schema, dir, &FileResolutionContext::new(dir)))
}

/// The `SchemaDocument` error an import must fail with, after checking it
/// names the imported file.
fn schema_document_message(label: &str, types: &Path, result: Result<ResolvedSchema, SchemaError>) -> String {
    match result {
        Err(SchemaError::SchemaDocument { path, message }) => {
            assert_eq!(
                biscuit_file::canonicalize_simplified(&path).unwrap_or(path),
                biscuit_file::canonicalize_simplified(types).expect("imported file canonicalizes"),
                "{label}: the error must name the imported file"
            );
            message
        }
        other => panic!("{label}: expected SchemaDocument, got {:?}", other.map(|resolved| resolved.json_schema)),
    }
}

#[test]
fn an_unparseable_import_target_is_a_located_yaml_error() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (types, result) = import_from(dir.path(), "kind: schema\ntypes: [\n");

    let message = schema_document_message("unclosed flow sequence", &types, result);

    assert!(message.starts_with("not valid YAML"), "{message}");
}

/// The classifier's admission wording, not serde's raw parse message: the
/// strict loader reports this file the same way wherever it is read.
#[test]
fn an_inadmissible_number_in_an_import_target_reports_the_admission_rule() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (types, result) =
        import_from(dir.path(), "kind: schema\ntypes:\n  name: string\n  big: 99999999999999999999999\n");

    let message = schema_document_message("oversized integer", &types, result);

    assert!(!message.starts_with("not valid YAML"), "{message}");
    assert!(message.contains("99999999999999999999999"), "{message}");
    assert!(message.contains("quote it to keep it as text"), "{message}");
}

#[test]
fn valid_yaml_that_is_not_a_schema_is_an_ambiguous_import() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (types, result) = import_from(dir.path(), "title: notes\nname: string\n");

    match result {
        Err(SchemaError::AmbiguousReferenced { path }) => assert_eq!(
            biscuit_file::canonicalize_simplified(&path).unwrap_or(path),
            biscuit_file::canonicalize_simplified(&types).expect("imported file canonicalizes")
        ),
        other => panic!("expected AmbiguousReferenced, got {:?}", other.map(|resolved| resolved.json_schema)),
    }
}

/// Recognized envelopes that cannot supply named types, and the document
/// shapes the import loader rejects, each from a one-edit fixture.
#[test]
fn an_import_target_that_cannot_supply_named_types_is_a_located_schema_document_error() {
    for (label, body, expected) in [
        (
            "unsupported envelope key",
            "kind: schema\ntypes:\n  name: string\nextra: 1\n",
            "tagged schema documents support only `kind`, `types`, `$schema`, and `categories`",
        ),
        ("implicit null types", "kind: schema\ntypes:\n", "tagged schema document `types` must be a mapping"),
        (
            "root union",
            "$schema:\n  - name: string\n  - other: string\n",
            "root-union schema documents cannot supply named imports",
        ),
        (
            "whole-file reference",
            "$schema: ./elsewhere.yaml\n",
            "whole-file reference schema documents cannot supply named imports",
        ),
        ("duplicate key", "kind: schema\ntypes:\n  name: string\n  name: number\n", "not valid YAML"),
        ("trailing content", "kind: schema\ntypes:\n  name: string\n]\n", "not valid YAML"),
        ("second document", "kind: schema\ntypes:\n  name: string\n---\nkind: schema\ntypes: {}\n", "not valid YAML"),
    ] {
        let dir = tempfile::tempdir().expect("tempdir");
        let (types, result) = import_from(dir.path(), body);

        let message = schema_document_message(label, &types, result);

        assert!(message.starts_with(expected), "{label}: {message}");
    }
}

#[test]
fn a_well_formed_import_target_supplies_its_named_type() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (_, result) = import_from(dir.path(), "kind: schema\ntypes:\n  name: string\n");

    let resolved = result.expect("a tagged schema supplies its named types");

    assert_eq!(
        resolved.json_schema["properties"]["value"],
        serde_json::json!({ "anyOf": [{ "type": "null" }, { "type": "string" }] })
    );
}

/// Undecodable bytes never reach the classifier; the import fails naming the
/// file, as a target that is not a schema does.
#[test]
fn an_import_target_that_is_not_utf8_is_an_ambiguous_import() {
    let dir = tempfile::tempdir().expect("tempdir");
    let types = dir.path().join("types.yaml");
    std::fs::write(&types, b"kind: schema\ntypes:\n  name: \xff\n").expect("write imported file");
    let schema: YamlValue = serde_yaml_ng::from_str("value: name@./types.yaml\n").expect("fixture YAML parses");

    match resolve_yaml_schema(&schema, dir.path(), &FileResolutionContext::new(dir.path())) {
        Err(SchemaError::AmbiguousReferenced { path }) => assert_eq!(
            biscuit_file::canonicalize_simplified(&path).unwrap_or(path),
            biscuit_file::canonicalize_simplified(&types).expect("imported file canonicalizes")
        ),
        other => panic!("expected AmbiguousReferenced, got {:?}", other.map(|resolved| resolved.json_schema)),
    }
}

/// Other valid spellings of the control: JSON-compatible YAML and the pure
/// `$schema` envelope supply the same named type.
#[test]
fn json_compatible_and_pure_envelope_import_targets_supply_their_named_type() {
    for (label, body) in [
        ("JSON-compatible YAML", r#"{"kind": "schema", "types": {"name": "string"}}"#),
        ("pure envelope", "$schema:\n  name: string\n"),
    ] {
        let dir = tempfile::tempdir().expect("tempdir");
        let (_, result) = import_from(dir.path(), body);

        let resolved = result.unwrap_or_else(|error| panic!("{label}: {error}"));

        assert_eq!(
            resolved.json_schema["properties"]["value"],
            serde_json::json!({ "anyOf": [{ "type": "null" }, { "type": "string" }] }),
            "{label}"
        );
    }
}

/// An invalid entry beside the imported one fails the whole file, in either
/// envelope: no entry is silently dropped to let the valid one through.
#[test]
fn an_invalid_sibling_entry_fails_an_import_of_a_valid_one() {
    for (label, body) in [
        ("tagged", "kind: schema\ntypes:\n  name: string\n  bad: 5\n"),
        ("pure", "$schema:\n  name: string\n  bad: 5\n"),
    ] {
        let dir = tempfile::tempdir().expect("tempdir");
        let (types, result) = import_from(dir.path(), body);

        let message = schema_document_message(label, &types, result);

        assert!(message.contains("bad"), "{label}: {message}");
    }
}

/// An empty `types` mapping is a valid table that declares nothing, so the
/// import names the missing type rather than treating the file as invalid.
#[test]
fn an_empty_types_mapping_reports_the_imported_name_as_unknown() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (_, result) = import_from(dir.path(), "kind: schema\ntypes: {}\n");

    match result {
        Err(SchemaError::UnknownNamedType { name, .. }) => assert_eq!(name, "name"),
        other => panic!("expected UnknownNamedType, got {:?}", other.map(|resolved| resolved.json_schema)),
    }
}
