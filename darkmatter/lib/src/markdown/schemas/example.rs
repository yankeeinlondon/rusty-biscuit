//! `example(...)` artifact resolution and validation (Feature A —
//! `darkmatter/features/2026-07-08-schema-plus`).
//!
//! An `example(<file>, …)` constraint attaches one or more example YAML files
//! to a schema property. The examples are **documentation** — non-validating
//! for the annotated document — but each referenced file is itself validated at
//! schema-load time so a schema never ships a broken example (O-A2, fail-loud).
//!
//! Validation has three layers:
//!
//! 1. The **artifact envelope** ([`EXAMPLE_ENVELOPE_YAML`]): every example is a
//!    `{ kind, invocation, returns, description }` document. `invocation` is a
//!    union of a plain expression string or a `{ frontmatter: yaml }` block, so
//!    a native mapping under `frontmatter` is accepted via `yaml` coercion
//!    (Feature D).
//! 2. The **inherited `parameters` shape**: when an example carries a
//!    `parameters` block it must be an array of single-key maps
//!    (`parameter[]` = `{ "<string>": any }` with exact one-key arity, O-A4).
//! 3. The **annotated target type**: the example's demonstrated `returns` value
//!    validates against the property the `example(...)` is attached to (its
//!    compiled JSON Schema minus the `x-darkmatter-example` annotation). This is
//!    the target-specific layer the spec calls for — an `example()` on a
//!    `number` property may not carry a string `returns`
//!    ([`validate_returns_against_target`]).
//!
//! Layers 1–2 are byte-cacheable (target-independent, keyed on the example
//! file's content hash); layer 3 is a **separate gate** applied per use site so
//! two properties referencing the same example file with different target types
//! do not share a target verdict through the byte cache.
//!
//! **Deferred (O-A1):** validating an example's `parameters` against a *typed
//! expression-function signature* (param names, arity, per-param types) is
//! gated on the function catalog being typed, which lives in the downstream
//! single-sourcing feature. Until then layer 2 keeps only the generic
//! `parameter[]` structural check, and layer 3 covers the `returns` value
//! against the annotated property (the one target type available today).
//!
//! Resolved example objects are emitted into the compiled JSON Schema under the
//! `x-darkmatter-example` extension (see [`super::simplified::convert`]) so
//! downstream consumers (`md schema about`, DMLS hover) read them without
//! re-reading disk. A process-wide content-hash cache keeps warm loads from
//! revalidating unchanged example files (O-A2).

use std::{
    collections::HashMap,
    sync::{LazyLock, Mutex, OnceLock},
};

use serde_json::Value;

use super::{
    coerce::{coerce_frontmatter, coerce_value_against_schema},
    errors::SchemaError,
    simplified::parse_yaml_schema,
    simplified::to_json_schema,
    validate::{ValidatorCache, build_validator},
};

/// Process-wide cache for `example(...)` returns-target validators (F28).
///
/// A property carrying multiple `example(...)` references — or the same schema
/// resolved more than once — otherwise rebuilds the identical target validator
/// per reference per resolution. The returns-target is compiled with no
/// file-reference anchors (`build_validator(target, None, None)` — the check is
/// structural/type-level, never filesystem existence), so a cache keyed on the
/// target JSON alone returns a byte-identical validator on a hit. Kept separate
/// from the effective-schema and coercion caches so these small target schemas
/// do not churn their LRU budgets.
static RETURNS_TARGET_VALIDATOR_CACHE: LazyLock<ValidatorCache> =
    LazyLock::new(ValidatorCache::new);

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

/// Validates an example artifact's demonstrated `returns` value against the
/// annotated property's target schema — the property the `example(...)` is
/// attached to. This is the spec's target-specific layer: an `example()` on a
/// `number` property may not carry a string `returns`.
///
/// `target` is the property's compiled JSON Schema with the
/// `x-darkmatter-example` annotation removed; its sibling keys describe the
/// property's value type. The validator is built with no `base_dir` / fallback
/// dir so the check stays structural/type-level and never asserts filesystem
/// existence for a `format: darkmatter-file` target.
///
/// Kept separate from [`validate_example_object`] (and outside the byte-hash
/// cache) so the same example file can be checked against different targets at
/// different use sites without one target's verdict leaking through the cache.
/// A no-op when the example carries no `returns` key.
///
/// ## Errors
///
/// Returns [`SchemaError::InvalidExample`] when the target schema cannot build
/// a validator, or when the example's `returns` value does not match it.
pub fn validate_returns_against_target(
    reference: &str,
    object: &Value,
    target: &Value,
) -> Result<(), SchemaError> {
    let Some(returns) = object.get("returns") else {
        return Ok(());
    };
    // Coerce a transient copy against the target before validating, mirroring the
    // envelope path in `validate_example_object`. This lets a `yaml`/`json`
    // content-format target accept a native mapping/sequence `returns` (serialized
    // to its string form) rather than rejecting it against `type: string`
    // (Feature D). The caller's `object` is never mutated.
    let coerced = coerce_value_against_schema(target, returns);
    let validator = RETURNS_TARGET_VALIDATOR_CACHE
        .validator_for(target, None)
        .map_err(|source| SchemaError::InvalidExample {
            reference: reference.to_string(),
            message: format!("annotated target schema failed to build a validator: {source}"),
        })?;
    if let Some(message) = first_error(&validator, &coerced) {
        return Err(SchemaError::InvalidExample {
            reference: reference.to_string(),
            message: format!(
                "example `returns` does not match the annotated target type: {message}"
            ),
        });
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
    fn returns_matching_target_type_validates() {
        // A string `returns` against a string-typed target passes — this guards
        // the acceptance corpus (`date` → string, `returns: "2024-12-25"`).
        let example = json!({
            "kind": "example",
            "invocation": "{{ ctx.today }}",
            "returns": "2024-12-25",
            "description": "d"
        });
        let target = json!({ "type": "string", "format": "date" });
        assert!(
            validate_returns_against_target("./x.yaml", &example, &target).is_ok(),
            "string `returns` must satisfy a string/date target"
        );
    }

    #[test]
    fn returns_wrong_target_type_is_rejected() {
        // The core gap: a well-formed envelope whose `returns` is the wrong type
        // for the annotated property (string `returns` on a `number` target)
        // must be rejected once the target is threaded in.
        let example = json!({
            "kind": "example",
            "invocation": "count()",
            "returns": "not-a-number",
            "description": "d"
        });
        let target = json!({ "type": "number" });
        let err = validate_returns_against_target("./x.yaml", &example, &target)
            .expect_err("string returns must not satisfy a number target");
        match err {
            SchemaError::InvalidExample { message, .. } => assert!(
                message.contains("annotated target type"),
                "message must name the target-type mismatch: {message}"
            ),
            other => panic!("expected InvalidExample, got {other:?}"),
        }
    }

    #[test]
    fn returns_native_mapping_validates_against_yaml_target() {
        // A `yaml` property's target is `type: string, format: darkmatter-yaml`.
        // A native mapping `returns` is serialized to a YAML string before the
        // `type: string` check, matching the envelope path's `frontmatter: yaml`
        // acceptance (Feature D) rather than being rejected as a non-string.
        let example = json!({
            "kind": "example",
            "invocation": "load()",
            "returns": { "title": "Foo", "tags": ["a", "b"] },
            "description": "d"
        });
        let target = json!({ "type": "string", "format": "darkmatter-yaml" });
        assert!(
            validate_returns_against_target("./x.yaml", &example, &target).is_ok(),
            "native mapping `returns` must coerce to a yaml string and pass"
        );
    }

    #[test]
    fn returns_native_sequence_validates_against_yaml_target() {
        let example = json!({
            "kind": "example",
            "invocation": "list()",
            "returns": [1, 2, 3],
            "description": "d"
        });
        let target = json!({ "type": "string", "format": "darkmatter-yaml" });
        assert!(
            validate_returns_against_target("./x.yaml", &example, &target).is_ok(),
            "native sequence `returns` must coerce to a yaml string and pass"
        );
    }

    #[test]
    fn returns_native_value_validates_against_json_target() {
        let target = json!({ "type": "string", "format": "darkmatter-json" });
        let mapping = json!({
            "kind": "example",
            "invocation": "load()",
            "returns": { "title": "Foo" },
            "description": "d"
        });
        assert!(validate_returns_against_target("./x.yaml", &mapping, &target).is_ok());
        let sequence = json!({
            "kind": "example",
            "invocation": "list()",
            "returns": [1, 2, 3],
            "description": "d"
        });
        assert!(validate_returns_against_target("./x.yaml", &sequence, &target).is_ok());
    }

    #[test]
    fn returns_already_string_passes_against_content_format_target() {
        // A `returns` already serialized as a string flows straight to the
        // `format` validator — coercion is a no-op (idempotence, no double-encode).
        let example = json!({
            "kind": "example",
            "invocation": "load()",
            "returns": "title: Foo",
            "description": "d"
        });
        let yaml_target = json!({ "type": "string", "format": "darkmatter-yaml" });
        assert!(validate_returns_against_target("./x.yaml", &example, &yaml_target).is_ok());
        let json_example = json!({
            "kind": "example",
            "invocation": "load()",
            "returns": "{\"title\":\"Foo\"}",
            "description": "d"
        });
        let json_target = json!({ "type": "string", "format": "darkmatter-json" });
        assert!(validate_returns_against_target("./x.yaml", &json_example, &json_target).is_ok());
    }

    #[test]
    fn returns_coercion_does_not_mutate_caller_object() {
        // The transient coercion works on a copy: even when a native mapping
        // `returns` IS coerced for the check, the caller's object is untouched.
        let example = json!({
            "kind": "example",
            "invocation": "load()",
            "returns": { "title": "Foo" },
            "description": "d"
        });
        let before = example.clone();
        let target = json!({ "type": "string", "format": "darkmatter-yaml" });
        validate_returns_against_target("./x.yaml", &example, &target).expect("valid");
        assert_eq!(example, before, "caller object must not be mutated by coercion");
    }

    #[test]
    fn returns_absent_skips_target_check() {
        // No `returns` key ⇒ nothing to validate against the target (no-op).
        let example = json!({ "kind": "example", "invocation": "x", "description": "d" });
        let target = json!({ "type": "number" });
        assert!(validate_returns_against_target("./x.yaml", &example, &target).is_ok());
    }

    #[test]
    fn content_hash_cache_reuses_validated_bytes() {
        let bytes = b"kind: example\ninvocation: \"x\"\nreturns: y\ndescription: d\n";
        let first = validate_example_bytes("./x.yaml", bytes).expect("valid example");
        let second = validate_example_bytes("./x.yaml", bytes).expect("cached example");
        assert_eq!(first, second);
    }
}
