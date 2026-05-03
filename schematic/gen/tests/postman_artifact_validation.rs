//! Schema validation for every committed Postman artifact.
//!
//! Walks `schematic/postman/*.postman_collection.json` and validates each
//! file against the vendored Postman v2.1.0 collection JSON Schema. This
//! is the broad-coverage half of review-2 Finding 5; complementary
//! single-purpose golden fixtures live in `postman_golden.rs`.
//!
//! On failure the test reports the offending file name plus every JSON
//! pointer / message returned by [`jsonschema`], so triage can jump
//! directly to the broken node.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use jsonschema::Validator;
use serde_json::Value;

/// Vendored Postman Collection v2.1.0 JSON Schema, embedded at compile time.
const POSTMAN_SCHEMA_BYTES: &[u8] =
    include_bytes!("fixtures/postman/v2.1.0-collection.schema.json");

/// Lazily-compiled validator for the Postman v2.1.0 collection schema.
fn postman_validator() -> &'static Validator {
    static VALIDATOR: OnceLock<Validator> = OnceLock::new();
    VALIDATOR.get_or_init(|| {
        let schema: Value = serde_json::from_slice(POSTMAN_SCHEMA_BYTES)
            .expect("vendored Postman schema must be valid JSON");
        jsonschema::validator_for(&schema)
            .expect("vendored Postman schema must compile as a JSON Schema")
    })
}

/// Validates a JSON value against the vendored Postman v2.1.0 collection
/// schema, returning every failure as a `"<json-pointer>: <reason>"` string.
fn validate_postman_json(value: &Value) -> Result<(), Vec<String>> {
    let validator = postman_validator();
    let errors: Vec<String> = validator
        .iter_errors(value)
        .map(|err| format!("{}: {}", err.instance_path(), err))
        .collect();
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Returns the absolute path to the `schematic/postman/` directory
/// holding committed artifacts.
fn postman_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("CARGO_MANIFEST_DIR has a parent")
        .join("postman")
}

/// Validates every committed `*.postman_collection.json` against the
/// Postman v2.1.0 schema. Aggregates failures so a single CI run reports
/// all broken artifacts at once instead of stopping at the first.
#[test]
fn every_committed_postman_artifact_validates_against_schema() {
    let dir = postman_dir();
    assert!(
        dir.is_dir(),
        "expected postman artifact directory at {}",
        dir.display(),
    );

    let mut checked_files = 0usize;
    let mut failures: Vec<String> = Vec::new();

    for entry in std::fs::read_dir(&dir).expect("read postman dir") {
        let entry = entry.expect("read postman dir entry");
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        let name = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or_default();
        if !name.ends_with(".postman_collection.json") {
            continue;
        }

        checked_files += 1;
        let bytes = std::fs::read(&path)
            .unwrap_or_else(|e| panic!("read {}: {}", path.display(), e));
        let value: Value = serde_json::from_slice(&bytes)
            .unwrap_or_else(|e| panic!("parse {}: {}", path.display(), e));

        if let Err(errors) = validate_postman_json(&value) {
            for err in errors {
                failures.push(format!("{}: {}", name, err));
            }
        }
    }

    assert!(
        checked_files > 0,
        "no Postman collection JSON files found under {}",
        dir.display(),
    );

    assert!(
        failures.is_empty(),
        "{} committed Postman artifact(s) failed v2.1.0 schema validation \
         ({} files checked):\n  - {}",
        failures.len(),
        checked_files,
        failures.join("\n  - "),
    );
}
