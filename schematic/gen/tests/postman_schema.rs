//! Postman v2.1.0 collection JSON Schema validation harness.
//!
//! This integration test file vendors and exposes a thin wrapper around
//! [`jsonschema`] for validating Postman collection JSON against the
//! upstream v2.1.0 schema. It is the validator used by the schematic
//! generator's Postman-related tests.
//!
//! ## Notes
//!
//! - The vendored schema lives at
//!   `tests/fixtures/postman/v2.1.0-collection.schema.json` and is treated
//!   as a frozen artifact; see the README in that directory for refresh
//!   guidance.
//! - The Postman schema declares JSON Schema draft-04 (`$schema`); the
//!   `jsonschema` crate auto-selects the matching draft when
//!   [`jsonschema::validator_for`] inspects the `$schema` keyword.
//! - The compiled validator is cached in a [`OnceLock`] so the (relatively
//!   expensive) compilation runs once per test binary regardless of how
//!   many tests call [`validate_postman_json`].

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
/// schema.
///
/// On success returns `Ok(())`. On failure returns `Err(messages)` where
/// each entry is a human-readable error string of the form
/// `"<json-pointer>: <reason>"`, suitable for direct use in test failure
/// output.
///
/// ## Examples
///
/// ```ignore
/// // Inside an integration test in this file's sibling tests:
/// let collection = serde_json::json!({
///     "info": {
///         "name": "x",
///         "schema": "https://schema.getpostman.com/json/collection/v2.1.0/collection.json"
///     },
///     "item": []
/// });
/// validate_postman_json(&collection).expect("minimal collection should validate");
/// ```
#[allow(dead_code)] // Phases 1, 2, 3, and 5 will exercise this helper.
pub(crate) fn validate_postman_json(value: &Value) -> Result<(), Vec<String>> {
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

/// Confirms the vendored schema itself parses and compiles as a valid
/// JSON Schema. If this fails, the schema file has been corrupted or the
/// `jsonschema` crate's draft-04 support has regressed.
#[test]
fn vendored_schema_compiles_as_json_schema() {
    let schema: Value = serde_json::from_slice(POSTMAN_SCHEMA_BYTES).expect("schema is valid JSON");
    assert_eq!(
        schema.get("$schema").and_then(Value::as_str),
        Some("http://json-schema.org/draft-04/schema#"),
        "vendored Postman schema is expected to declare draft-04",
    );
    let validator = jsonschema::validator_for(&schema)
        .expect("vendored Postman schema must compile as a JSON Schema");
    // Touch the validator so the compiler doesn't drop the binding early.
    let _ = validator.is_valid(&Value::Null);
}

/// Confirms a hand-crafted minimal valid collection passes validation.
/// This catches both validator bootstrapping bugs and any future drift
/// in our [`validate_postman_json`] helper signature.
#[test]
fn minimal_collection_validates_ok() {
    let collection = serde_json::json!({
        "info": {
            "name": "x",
            "schema": "https://schema.getpostman.com/json/collection/v2.1.0/collection.json"
        },
        "item": []
    });
    validate_postman_json(&collection).expect("minimal collection must validate");
}

/// Confirms the regenerated EMQX grouped collection (Phase 3) validates
/// against the Postman v2.1.0 schema. EMQX is the canonical mixed-auth
/// regression target: it groups `EmqxBasic` and `EmqxBearer` into one
/// collection, so per-request auth must be set, collection auth must be
/// `null`, and duplicate request names must be disambiguated.
#[test]
fn emqx_grouped_collection_validates_against_postman_schema() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("schematic/gen has parent")
        .join("postman")
        .join("emqx.postman_collection.json");
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("read {}: {}", path.display(), e));
    let value: Value = serde_json::from_slice(&bytes)
        .unwrap_or_else(|e| panic!("parse {}: {}", path.display(), e));
    if let Err(errors) = validate_postman_json(&value) {
        panic!(
            "{} failed Postman v2.1.0 schema validation:\n{}",
            path.display(),
            errors.join("\n"),
        );
    }
}

/// Sanity check the negative path: a clearly invalid document (missing
/// the required `info` object) must produce errors. Without this, a
/// silently-broken validator could mask future regressions.
#[test]
fn invalid_collection_reports_errors() {
    // `info` is required; an empty object should fail validation.
    let bogus = serde_json::json!({});
    let result = validate_postman_json(&bogus);
    let errors = result.expect_err("empty object must not validate as a Postman collection");
    assert!(
        !errors.is_empty(),
        "expected at least one validation error for empty object"
    );
}
