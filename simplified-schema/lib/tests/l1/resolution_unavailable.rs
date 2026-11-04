//! Without the `file` feature the crate resolves no file reference: every
//! check that needs one fails with a typed "resolution unavailable" cause,
//! never a silent pass. With the feature the same inputs resolve (the
//! control cases).

use std::path::Path;

use biscuit_file::{FileReferenceError, FileResolutionContext};
use serde_json::json;
use simplified_schema::embedded::ShippedSources;
use simplified_schema::resolve::resolve_yaml_schema;
use simplified_schema::validate::build_validator_in_context;
use simplified_schema::{SchemaError, parse_yaml_schema, to_json_schema};

/// A folder holding `types.yaml` (a schema file) and `note.md`.
fn fixture() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("types.yaml"), "$schema:\n  title: string\n").unwrap();
    std::fs::write(dir.path().join("note.md"), "note").unwrap();
    dir
}

#[test]
fn a_schema_file_reference_is_resolved_only_with_the_file_feature() {
    let dir = fixture();
    let context = FileResolutionContext::new(dir.path());
    let reference: serde_yaml_ng::Value = serde_yaml_ng::from_str("./types.yaml").unwrap();
    let result = resolve_yaml_schema(&reference, dir.path(), &context, ShippedSources::NONE);
    if cfg!(feature = "file") {
        let resolved = result.expect("the referenced schema resolves");
        assert!(resolved.json_schema["properties"]["title"].is_object(), "{}", resolved.json_schema);
    } else {
        match result {
            Err(SchemaError::Unresolved { reference, source: FileReferenceError::ResolutionUnavailable { .. } }) => {
                assert_eq!(reference, "./types.yaml");
            }
            other => panic!("expected an unavailable resolution, got {other:?}"),
        }
    }
}

#[test]
fn an_eager_file_value_fails_without_the_file_feature() {
    let dir = fixture();
    let context = FileResolutionContext::new(dir.path());
    let schema: serde_yaml_ng::Value = serde_yaml_ng::from_str("doc: file(eager)\n").unwrap();
    let json_schema = to_json_schema(&parse_yaml_schema(&schema).unwrap()).unwrap();
    let validator = build_validator_in_context(&json_schema, Some(Path::new(dir.path())), None, &context).unwrap();
    let accepted = validator.is_valid(&json!({ "doc": "./note.md" }));
    assert_eq!(accepted, cfg!(feature = "file"), "an existing file is accepted only when it can be resolved");
    assert!(!validator.is_valid(&json!({ "doc": "./missing.md" })), "a missing file is never accepted");
}
