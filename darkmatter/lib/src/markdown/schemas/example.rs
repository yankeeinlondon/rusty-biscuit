//! `example(...)` artifact resolution and validation (Feature A —
//! `darkmatter/features/2026-07-08-schema-plus`).
//!
//! An `example(<file>, …)` constraint attaches one or more example YAML files
//! to a schema property. The examples are **documentation** — non-validating
//! for the annotated document — but each referenced file is itself validated at
//! schema-load time so a schema never ships a broken example (O-A2, fail-loud).
//!
//! Validation has two layers:
//!
//! 1. The **artifact envelope** ([`EXAMPLE_ENVELOPE_YAML`]): every example is a
//!    `{ kind, invocation, returns, description }` document. `invocation` is a
//!    union of a plain expression string or a `{ frontmatter: yaml }` block, so
//!    a native mapping under `frontmatter` is accepted via `yaml` coercion
//!    (Feature D).
//! 2. The **inherited `parameters` shape**: when an example carries a
//!    `parameters` block it must be an array of single-key maps
//!    (`parameter[]` = `{ "<string>": any }` with exact one-key arity, O-A4).
//!
//! Resolved example objects are emitted into the compiled JSON Schema under the
//! `x-darkmatter-example` extension (see [`super::simplified::convert`]) so
//! downstream consumers (`md schema about`, DMLS hover) read them without
//! re-reading disk. A process-wide content-hash cache keeps warm loads from
//! revalidating unchanged example files (O-A2).

use std::{
    collections::HashMap,
    sync::{Mutex, OnceLock},
};

use serde_json::Value;

use super::{
    coerce::coerce_frontmatter, errors::SchemaError, simplified::parse_yaml_schema,
    simplified::to_json_schema, validate::build_validator,
};

/// The example-artifact envelope, authored in the SimplifiedSchema grammar. The
/// on-disk `example.yaml` fixture under the schema-plus feature directory is the
/// same schema; `example_envelope_matches_fixture` guards against drift.
pub const EXAMPLE_ENVELOPE_YAML: &str = "\
kind: enum(example; required)
invocation:
    - string(required)
    - { frontmatter: yaml }
returns: any
description: string
";

/// Returns the compiled example-artifact envelope JSON Schema (cached).
fn example_envelope_json() -> &'static Value {
    static ENVELOPE: OnceLock<Value> = OnceLock::new();
    ENVELOPE.get_or_init(|| {
        let value: serde_yaml_ng::Value =
            serde_yaml_ng::from_str(EXAMPLE_ENVELOPE_YAML).expect("example envelope yaml must parse");
        let schema = parse_yaml_schema(&value).expect("example envelope must parse");
        to_json_schema(&schema).expect("example envelope must convert")
    })
}

/// Returns the inherited `parameters` shape: `parameter[]`, i.e. an array of
/// single-key maps (`{ "<string>": any }` with exact one-key arity — O-A4).
///
/// Hand-built rather than derived from a SimplifiedSchema import so the built-in
/// carries no cross-file dependency; the shape mirrors what
/// `parameter[]@types.yaml` expands to (a `<string>` catch-all lowers to
/// `additionalProperties: {}`, plus `minProperties`/`maxProperties` 1).
fn parameters_target_json() -> &'static Value {
    static SCHEMA: OnceLock<Value> = OnceLock::new();
    SCHEMA.get_or_init(|| {
        serde_json::json!({
            "type": "array",
            "items": {
                "type": "object",
                "additionalProperties": {},
                "minProperties": 1,
                "maxProperties": 1
            }
        })
    })
}

/// Content-hash cache of validated example artifacts. Keyed by the xxHash of the
/// file's raw bytes so an unchanged file is parsed and validated once per
/// process (O-A2 warm-load reuse). The stored value is the resolved example
/// object emitted into `x-darkmatter-example`.
fn example_cache() -> &'static Mutex<HashMap<u64, Value>> {
    static CACHE: OnceLock<Mutex<HashMap<u64, Value>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Validates one example artifact's raw file bytes against the envelope (and its
/// `parameters` shape) and returns the resolved example object to embed under
/// `x-darkmatter-example`.
///
/// `reference` is the authored `example(...)` reference, used only for error
/// messages. Results are memoized by content hash, so an unchanged file is
/// validated once per process.
///
/// ## Errors
///
/// Returns [`SchemaError::InvalidExample`] when the bytes are not UTF-8, are not
/// valid YAML, do not validate against the envelope, or carry a `parameters`
/// block that is not an array of single-key maps.
pub fn validate_example_bytes(reference: &str, bytes: &[u8]) -> Result<Value, SchemaError> {
    let hash = biscuit_hash::xx_hash_bytes(bytes);
    if let Some(cached) = example_cache().lock().expect("example cache poisoned").get(&hash) {
        return Ok(cached.clone());
    }

    let text = std::str::from_utf8(bytes).map_err(|_| SchemaError::InvalidExample {
        reference: reference.to_string(),
        message: "example file is not valid UTF-8".into(),
    })?;
    let yaml: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(text).map_err(|source| SchemaError::InvalidExample {
            reference: reference.to_string(),
            message: format!("example file is not valid YAML: {source}"),
        })?;
    let object = serde_json::to_value(&yaml).map_err(|source| SchemaError::InvalidExample {
        reference: reference.to_string(),
        message: format!("example file could not be normalized to JSON: {source}"),
    })?;

    validate_example_object(reference, &object)?;

    example_cache()
        .lock()
        .expect("example cache poisoned")
        .insert(hash, object.clone());
    Ok(object)
}

/// Validates an already-parsed example object against the envelope and its
/// inherited `parameters` shape. Split out so the acceptance-corpus fixture
/// tests can validate through the same path a schema load uses.
///
/// ## Errors
///
/// Returns [`SchemaError::InvalidExample`] on any envelope or `parameters`
/// failure.
pub fn validate_example_object(reference: &str, object: &Value) -> Result<(), SchemaError> {
    let envelope = example_envelope_json();
    // Coerce a working copy so a native `{ frontmatter: { … } }` mapping is
    // serialized to a YAML string for the `yaml` content-format arm (Feature D).
    // The caller's object is never mutated.
    let coerced = coerce_frontmatter(envelope, object);
    let validator = build_validator(envelope, None, None).map_err(|source| {
        SchemaError::InvalidExample {
            reference: reference.to_string(),
            message: format!("example envelope failed to build a validator: {source}"),
        }
    })?;
    if let Some(message) = first_error(&validator, &coerced.value) {
        return Err(SchemaError::InvalidExample {
            reference: reference.to_string(),
            message: format!("does not match the example envelope: {message}"),
        });
    }

    // Layer 2: an example's optional `parameters` block validates against the
    // inherited `parameter[]` shape (single-key maps, O-A4).
    if let Some(parameters) = object.get("parameters") {
        let target = parameters_target_json();
        let validator = build_validator(target, None, None).map_err(|source| {
            SchemaError::InvalidExample {
                reference: reference.to_string(),
                message: format!("parameters target failed to build a validator: {source}"),
            }
        })?;
        if let Some(message) = first_error(&validator, parameters) {
            return Err(SchemaError::InvalidExample {
                reference: reference.to_string(),
                message: format!(
                    "`parameters` must be an array of single-key maps: {message}"
                ),
            });
        }
    }
    Ok(())
}

/// Returns the first validation error message, or `None` when the value is
/// valid.
fn first_error(validator: &jsonschema::Validator, value: &Value) -> Option<String> {
    validator.iter_errors(value).next().map(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn envelope_accepts_string_invocation() {
        let example = json!({
            "kind": "example",
            "invocation": "{{ ctx.today }}",
            "returns": "2024-12-25",
            "description": "The date."
        });
        assert!(validate_example_object("./x.yaml", &example).is_ok());
    }

    #[test]
    fn envelope_accepts_native_frontmatter_via_coercion() {
        // The `{ frontmatter: yaml }` union arm accepts a native mapping under
        // `frontmatter` because coercion serializes it to a YAML string first.
        let example = json!({
            "kind": "example",
            "invocation": { "frontmatter": { "list": [1, 2, 3], "output": "{{ x }}" } },
            "returns": "- 1\n- 2\n- 3",
            "description": "native frontmatter"
        });
        assert!(
            validate_example_object("./x.yaml", &example).is_ok(),
            "native frontmatter mapping must validate via yaml coercion"
        );
    }

    #[test]
    fn envelope_rejects_missing_kind() {
        let example = json!({ "invocation": "x", "description": "d" });
        assert!(validate_example_object("./x.yaml", &example).is_err());
    }

    #[test]
    fn envelope_rejects_wrong_kind_enum() {
        let example = json!({ "kind": "sample", "invocation": "x" });
        assert!(validate_example_object("./x.yaml", &example).is_err());
    }

    #[test]
    fn parameters_single_key_maps_validate() {
        let example = json!({
            "kind": "example",
            "parameters": [ { "list": [1, 2, 3] } ],
            "invocation": "as_unordered_list(list)",
            "returns": "- 1",
            "description": "d"
        });
        assert!(validate_example_object("./x.yaml", &example).is_ok());
    }

    #[test]
    fn parameters_multi_key_map_is_rejected() {
        // A two-key parameter map violates the exact one-key arity (O-A4).
        let example = json!({
            "kind": "example",
            "parameters": [ { "a": 1, "b": 2 } ],
            "invocation": "f(a, b)",
            "returns": "x",
            "description": "d"
        });
        let err = validate_example_object("./x.yaml", &example).unwrap_err();
        assert!(
            matches!(err, SchemaError::InvalidExample { .. }),
            "multi-key parameter map must be an InvalidExample: {err:?}"
        );
    }

    #[test]
    fn content_hash_cache_reuses_validated_bytes() {
        let bytes = b"kind: example\ninvocation: \"x\"\nreturns: y\ndescription: d\n";
        let first = validate_example_bytes("./x.yaml", bytes).expect("valid example");
        let second = validate_example_bytes("./x.yaml", bytes).expect("cached example");
        assert_eq!(first, second);
    }
}
