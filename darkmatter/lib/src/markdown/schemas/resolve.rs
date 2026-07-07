//! `$schema` resolution and baseline merging.
//!
//! This module turns a raw `$schema` frontmatter value into the fully-formed
//! Draft 2020-12 JSON Schema that the validator runs against. Three input
//! shapes are recognised:
//!
//! - **Inline mapping** — interpreted as a [`SimplifiedSchema`] directly.
//! - **Inline sequence** — interpreted as a root-level SimplifiedSchema union.
//!   String items are file references (resolved here); mapping items are
//!   inline arms.
//! - **String scalar** — interpreted as a `biscuit-file` reference to a YAML
//!   or JSON file. The referenced file is disambiguated using the rule in
//!   the spec: YAML files whose root holds a `$schema:` mapping are
//!   SimplifiedSchemas; everything else is raw JSON Schema. JSON files are
//!   always raw JSON Schema.
//!
//! After resolution, a baseline schema (if configured) is merged with the
//! document schema using property-keyed deep merge — the document side wins
//! on conflict.
//!
//! Remote (`http://` / `https://`) `$schema` values are rejected up front
//! with [`SchemaError::RemoteUnsupported`].

use std::{fs, path::Path};

use biscuit_file::FileReference;
use serde_json::{Map, Value};
use serde_yaml_ng::Value as YamlValue;

use super::{
    SchemaArm, SchemaOrigin, SchemaShape, SimplifiedSchema,
    errors::SchemaError,
    simplified::{parse_yaml_schema, to_json_schema},
};

/// The product of resolving a document's `$schema`.
///
/// `simplified` is `None` when the effective schema came from a raw JSON
/// Schema file (or from a root union arm referencing one); `json_schema`
/// is always populated.
#[derive(Debug, Clone)]
pub struct ResolvedSchema {
    pub simplified: Option<SimplifiedSchema>,
    pub json_schema: Value,
    /// Where the schema came from — an inline `$schema` mapping/sequence
    /// (`Document`) or a referenced schema file (`ReferencedFile`, carrying
    /// the resolved path). Preserved so diagnostics can point
    /// `relatedInformation` at the schema source (R-5 Priority 2).
    pub origin: SchemaOrigin,
}

/// Resolves a frontmatter `$schema` value into a JSON Schema.
///
/// `base_dir` is the directory the document lives in — file references in
/// `$schema` resolve relative to it (as opposed to `file` *property* values,
/// which resolve from the CWD).
///
/// ## Errors
///
/// Returns:
/// - [`SchemaError::RemoteUnsupported`] for `http://` / `https://` values.
/// - [`SchemaError::Unresolved`] when a file reference fails to resolve.
/// - [`SchemaError::Io`] when reading the referenced file fails.
/// - [`SchemaError::AmbiguousReferenced`] when the referenced file is neither
///   a valid SimplifiedSchema nor a JSON Schema.
/// - [`SchemaError::FrontmatterShape`] for unsupported `$schema` shapes.
/// - [`SchemaError::Grammar`] / [`SchemaError::Convert`] propagated from the
///   parser and converter.
pub fn resolve_schema(value: &Value, base_dir: &Path) -> Result<ResolvedSchema, SchemaError> {
    let yaml = json_to_yaml(value);
    resolve_yaml_schema(&yaml, base_dir)
}

/// Same as [`resolve_schema`] but takes a YAML value directly. Used by the
/// resolver itself (referenced YAML files) and by tests.
pub fn resolve_yaml_schema(
    value: &YamlValue,
    base_dir: &Path,
) -> Result<ResolvedSchema, SchemaError> {
    match value {
        YamlValue::String(reference) => resolve_reference(reference, base_dir),
        YamlValue::Mapping(_) => {
            let schema = parse_yaml_schema(value)?;
            let json = to_json_schema(&schema)?;
            Ok(ResolvedSchema {
                simplified: Some(schema),
                json_schema: json,
                origin: SchemaOrigin::document(),
            })
        }
        YamlValue::Sequence(items) => resolve_root_union(items, base_dir),
        other => Err(SchemaError::FrontmatterShape {
            message: format!(
                "$schema must be a mapping, sequence, or string; got {}",
                describe_yaml(other)
            ),
        }),
    }
}

fn resolve_root_union(items: &[YamlValue], base_dir: &Path) -> Result<ResolvedSchema, SchemaError> {
    if items.is_empty() {
        return Err(SchemaError::FrontmatterShape {
            message: "$schema root union must have at least one arm".into(),
        });
    }
    let mut any_of: Vec<Value> = Vec::with_capacity(items.len());
    let mut all_simplified_arms: Option<Vec<SchemaArm>> = Some(Vec::with_capacity(items.len()));
    for item in items {
        match item {
            YamlValue::Mapping(_) => {
                let arm_schema = parse_yaml_schema(item)?;
                let arm_json = to_json_schema(&arm_schema)?;
                if let SimplifiedSchema::Single(shape) = &arm_schema
                    && let Some(arms) = all_simplified_arms.as_mut()
                {
                    arms.push(SchemaArm::Inline(shape.clone()));
                } else {
                    all_simplified_arms = None;
                }
                any_of.push(strip_schema_uri(arm_json));
            }
            YamlValue::String(reference) => {
                let resolved = resolve_reference(reference, base_dir)?;
                // If this arm itself came from a SimplifiedSchema file, preserve
                // it in the simplified projection only when it is a single
                // shape (root unions of unions are not modelled in v1).
                match (&resolved.simplified, all_simplified_arms.as_mut()) {
                    (Some(SimplifiedSchema::Single(shape)), Some(arms)) => {
                        arms.push(SchemaArm::Inline(shape.clone()));
                    }
                    _ => {
                        all_simplified_arms = None;
                    }
                }
                any_of.push(strip_schema_uri(resolved.json_schema));
            }
            other => {
                return Err(SchemaError::FrontmatterShape {
                    message: format!(
                        "root union arms must be a mapping or a file reference, got {}",
                        describe_yaml(other)
                    ),
                });
            }
        }
    }
    let mut root = Map::new();
    root.insert(
        "$schema".into(),
        Value::String(super::simplified::DRAFT_2020_12.into()),
    );
    root.insert("anyOf".into(), Value::Array(any_of));

    Ok(ResolvedSchema {
        simplified: all_simplified_arms.map(SimplifiedSchema::Union),
        json_schema: Value::Object(root),
        origin: SchemaOrigin::document(),
    })
}

fn resolve_reference(reference: &str, base_dir: &Path) -> Result<ResolvedSchema, SchemaError> {
    let trimmed = reference.trim();
    if let Some(rest) = trimmed.strip_prefix("http://") {
        let _ = rest;
        return Err(SchemaError::RemoteUnsupported {
            reference: reference.into(),
        });
    }
    if let Some(rest) = trimmed.strip_prefix("https://") {
        let _ = rest;
        return Err(SchemaError::RemoteUnsupported {
            reference: reference.into(),
        });
    }

    let file_ref = FileReference::new(reference).map_err(|source| SchemaError::Unresolved {
        reference: reference.to_string(),
        source,
    })?;
    let path = file_ref
        .resolve_from(base_dir)
        .map_err(|source| SchemaError::Unresolved {
            reference: reference.to_string(),
            source,
        })?
        .ok_or_else(|| SchemaError::Unresolved {
            reference: reference.to_string(),
            source: biscuit_file::FileReferenceError::InvalidSyntax(format!(
                "no file matched `{reference}`"
            )),
        })?;

    let mut resolved = load_schema_from_path(&path)?;
    // The schema was loaded from a file — record the resolved path so
    // diagnostics can point `relatedInformation` at the referenced source.
    resolved.origin = SchemaOrigin::referenced_file(path);
    Ok(resolved)
}

fn load_schema_from_path(path: &Path) -> Result<ResolvedSchema, SchemaError> {
    let bytes = fs::read(path).map_err(|source| SchemaError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(str::to_ascii_lowercase);

    match extension.as_deref() {
        Some("json") => parse_raw_json_schema(path, &bytes),
        _ => parse_yaml_referenced_file(path, &bytes),
    }
}

fn parse_yaml_referenced_file(path: &Path, bytes: &[u8]) -> Result<ResolvedSchema, SchemaError> {
    let text = std::str::from_utf8(bytes).map_err(|_| SchemaError::AmbiguousReferenced {
        path: path.to_path_buf(),
    })?;
    let value: YamlValue =
        serde_yaml_ng::from_str(text).map_err(|_| SchemaError::AmbiguousReferenced {
            path: path.to_path_buf(),
        })?;

    // SimplifiedSchema disambiguation: root has `$schema:` that is a mapping
    // or a sequence (i.e. an authored SimplifiedSchema). Otherwise treat as
    // raw JSON Schema.
    if let YamlValue::Mapping(map) = &value
        && let Some(schema_value) = map.get(YamlValue::String("$schema".into()))
        && matches!(schema_value, YamlValue::Mapping(_) | YamlValue::Sequence(_))
    {
        let parsed = parse_yaml_schema(schema_value)?;
        let json = to_json_schema(&parsed)?;
        return Ok(ResolvedSchema {
            simplified: Some(parsed),
            json_schema: json,
            // `resolve_reference` overwrites this with the file path.
            origin: SchemaOrigin::document(),
        });
    }

    // Treat the file's contents as a raw JSON Schema serialised in YAML.
    let json: Value =
        serde_yaml_ng::from_str(text).map_err(|_| SchemaError::AmbiguousReferenced {
            path: path.to_path_buf(),
        })?;
    if !json.is_object() {
        return Err(SchemaError::AmbiguousReferenced {
            path: path.to_path_buf(),
        });
    }
    Ok(ResolvedSchema {
        simplified: None,
        json_schema: json,
        origin: SchemaOrigin::document(),
    })
}

fn parse_raw_json_schema(path: &Path, bytes: &[u8]) -> Result<ResolvedSchema, SchemaError> {
    let value: Value =
        serde_json::from_slice(bytes).map_err(|_| SchemaError::AmbiguousReferenced {
            path: path.to_path_buf(),
        })?;
    if !value.is_object() {
        return Err(SchemaError::AmbiguousReferenced {
            path: path.to_path_buf(),
        });
    }
    Ok(ResolvedSchema {
        simplified: None,
        json_schema: value,
        origin: SchemaOrigin::document(),
    })
}

/// Merges a baseline JSON Schema into a document JSON Schema using the
/// property-keyed deep merge documented in the spec.
///
/// - When both schemas declare the same property, the **document** side wins
///   entirely (its type + constraints replace the baseline's).
/// - `required` arrays are unioned, then de-duplicated, but only document
///   properties retain their baseline-required status if they appear in the
///   baseline's `required` array (since the document's own `required` may
///   restate them).
/// - Properties present only in one side are preserved verbatim.
///
/// ## Errors
///
/// Returns [`SchemaError::Baseline`] when the baseline is not a simple
/// object schema (rooted at `"type": "object"` with only `properties` /
/// `required`).
pub fn merge_baseline(baseline: &Value, document: Value) -> Result<Value, SchemaError> {
    let baseline_obj = baseline.as_object().ok_or_else(|| SchemaError::Baseline {
        message: "baseline must be an object schema".into(),
        source: None,
    })?;
    validate_simple_object_schema(baseline_obj)?;

    // Root unions: merge into each arm independently.
    if let Some(arms) = document.get("anyOf").and_then(Value::as_array) {
        let mut new_arms = Vec::with_capacity(arms.len());
        for arm in arms {
            new_arms.push(merge_baseline(baseline, arm.clone())?);
        }
        let mut merged = document.as_object().cloned().unwrap_or_else(Map::new);
        merged.insert("anyOf".into(), Value::Array(new_arms));
        return Ok(Value::Object(merged));
    }

    let mut document_obj = match document {
        Value::Object(m) => m,
        _ => {
            return Err(SchemaError::Baseline {
                message: "document schema must be an object".into(),
                source: None,
            });
        }
    };

    let baseline_props: Map<String, Value> = baseline_obj
        .get("properties")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let baseline_required: Vec<String> = baseline_obj
        .get("required")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    let mut doc_props = document_obj
        .remove("properties")
        .and_then(|v| match v {
            Value::Object(m) => Some(m),
            _ => None,
        })
        .unwrap_or_default();
    let mut doc_required: Vec<String> = document_obj
        .remove("required")
        .and_then(|v| match v {
            Value::Array(a) => Some(a),
            _ => None,
        })
        .unwrap_or_default()
        .into_iter()
        .filter_map(|v| v.as_str().map(String::from))
        .collect();

    for (key, baseline_schema) in &baseline_props {
        if !doc_props.contains_key(key) {
            doc_props.insert(key.clone(), baseline_schema.clone());
            if baseline_required.contains(key) && !doc_required.contains(key) {
                doc_required.push(key.clone());
            }
        }
    }

    document_obj.insert("properties".into(), Value::Object(doc_props));
    if !doc_required.is_empty() {
        // Stable order: keep the existing document order, then append any
        // baseline-only required props.
        let mut seen = std::collections::HashSet::new();
        let mut ordered: Vec<Value> = Vec::with_capacity(doc_required.len());
        for name in doc_required {
            if seen.insert(name.clone()) {
                ordered.push(Value::String(name));
            }
        }
        document_obj.insert("required".into(), Value::Array(ordered));
    } else {
        document_obj.remove("required");
    }

    // Ensure required fields like "$schema" and "type" survive — copy from
    // baseline where the document didn't supply them.
    if !document_obj.contains_key("type") && baseline_obj.get("type").is_some() {
        document_obj.insert("type".into(), baseline_obj["type"].clone());
    }
    if !document_obj.contains_key("$schema") && baseline_obj.get("$schema").is_some() {
        document_obj.insert("$schema".into(), baseline_obj["$schema"].clone());
    }
    if !document_obj.contains_key("additionalProperties")
        && baseline_obj.get("additionalProperties").is_some()
    {
        document_obj.insert(
            "additionalProperties".into(),
            baseline_obj["additionalProperties"].clone(),
        );
    }

    Ok(Value::Object(document_obj))
}

/// Validates that a baseline JSON Schema is a simple object schema, per the
/// spec's restriction.
fn validate_simple_object_schema(schema: &Map<String, Value>) -> Result<(), SchemaError> {
    // `additionalProperties: true` is permitted (SimplifiedSchema-generated
    // baselines emit it); `false` and schema-shaped values are not.
    const ALLOWED_KEYS: &[&str] = &[
        "$schema",
        "type",
        "properties",
        "required",
        "additionalProperties",
        "description",
        "title",
    ];

    if let Some(ty) = schema.get("type") {
        if ty != "object" {
            return Err(SchemaError::Baseline {
                message: format!("baseline root must be type=object, got {ty}"),
                source: None,
            });
        }
    } else {
        return Err(SchemaError::Baseline {
            message: "baseline root must declare type=object".into(),
            source: None,
        });
    }

    for key in schema.keys() {
        if !ALLOWED_KEYS.contains(&key.as_str()) {
            return Err(SchemaError::Baseline {
                message: format!(
                    "baseline uses unsupported JSON Schema construct `{key}`; only \
                     simple object schemas (properties + required) are allowed"
                ),
                source: None,
            });
        }
    }

    if let Some(ap) = schema.get("additionalProperties") {
        match ap {
            Value::Bool(true) => {}
            _ => {
                return Err(SchemaError::Baseline {
                    message: "baseline uses unsupported JSON Schema construct \
                              `additionalProperties: false`; only simple object \
                              schemas (properties + required) are allowed"
                        .into(),
                    source: None,
                });
            }
        }
    }

    Ok(())
}

// ── Helpers ───────────────────────────────────────────────────────────────

fn describe_yaml(value: &YamlValue) -> &'static str {
    match value {
        YamlValue::Null => "null",
        YamlValue::Bool(_) => "boolean",
        YamlValue::Number(_) => "number",
        YamlValue::String(_) => "string",
        YamlValue::Sequence(_) => "sequence",
        YamlValue::Mapping(_) => "mapping",
        YamlValue::Tagged(_) => "tagged-value",
    }
}

/// Converts a `serde_json::Value` (the type frontmatter uses internally)
/// into a `serde_yaml_ng::Value` so the YAML-shape parser can consume it
/// without a second deserialisation round-trip.
fn json_to_yaml(value: &Value) -> YamlValue {
    match value {
        Value::Null => YamlValue::Null,
        Value::Bool(b) => YamlValue::Bool(*b),
        Value::Number(n) => {
            // serde_yaml_ng accepts the same numeric forms as serde_json.
            if let Some(i) = n.as_i64() {
                YamlValue::Number(serde_yaml_ng::Number::from(i))
            } else if let Some(u) = n.as_u64() {
                YamlValue::Number(serde_yaml_ng::Number::from(u))
            } else if let Some(f) = n.as_f64() {
                YamlValue::Number(serde_yaml_ng::Number::from(f))
            } else {
                YamlValue::Null
            }
        }
        Value::String(s) => YamlValue::String(s.clone()),
        Value::Array(items) => YamlValue::Sequence(items.iter().map(json_to_yaml).collect()),
        Value::Object(map) => {
            let mut out = serde_yaml_ng::Mapping::new();
            for (k, v) in map {
                out.insert(YamlValue::String(k.clone()), json_to_yaml(v));
            }
            YamlValue::Mapping(out)
        }
    }
}

fn strip_schema_uri(value: Value) -> Value {
    if let Value::Object(mut map) = value {
        map.remove("$schema");
        Value::Object(map)
    } else {
        value
    }
}

/// Resolves a SimplifiedSchema (typed) under the same disambiguation rules
/// used for `$schema` references. Useful when callers already have a
/// SimplifiedSchema in hand (e.g. the library-level `with_baseline`) and
/// only need to convert + merge.
pub fn shape_to_schema(shape: &SchemaShape) -> Result<Value, SchemaError> {
    to_json_schema(&SimplifiedSchema::Single(shape.clone()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn yaml_value(input: &str) -> YamlValue {
        serde_yaml_ng::from_str(input).expect("yaml parse")
    }

    #[test]
    fn resolves_inline_mapping() {
        let v = yaml_value("title: 'string(required)'");
        let resolved = resolve_yaml_schema(&v, Path::new(".")).unwrap();
        assert!(resolved.simplified.is_some());
        assert_eq!(resolved.json_schema["type"], "object");
        let required = resolved.json_schema["required"].as_array().unwrap();
        assert_eq!(required[0], "title");
    }

    #[test]
    fn resolves_inline_root_union() {
        let v = yaml_value("- title: 'string(required)'\n- name: string");
        let resolved = resolve_yaml_schema(&v, Path::new(".")).unwrap();
        assert!(resolved.json_schema["anyOf"].is_array());
    }

    #[test]
    fn remote_https_is_rejected() {
        let v = yaml_value("'https://example.com/schema.json'");
        let err = resolve_yaml_schema(&v, Path::new(".")).unwrap_err();
        assert!(matches!(err, SchemaError::RemoteUnsupported { .. }));
    }

    #[test]
    fn remote_http_is_rejected() {
        let v = yaml_value("'http://example.com/schema.json'");
        let err = resolve_yaml_schema(&v, Path::new(".")).unwrap_err();
        assert!(matches!(err, SchemaError::RemoteUnsupported { .. }));
    }

    #[test]
    fn unsupported_shape_is_rejected() {
        let v = yaml_value("42");
        let err = resolve_yaml_schema(&v, Path::new(".")).unwrap_err();
        assert!(matches!(err, SchemaError::FrontmatterShape { .. }));
    }

    fn write_schema_file(dir: &Path, name: &str, body: &str) -> std::path::PathBuf {
        let path = dir.join(name);
        std::fs::write(&path, body).unwrap();
        path
    }

    #[test]
    fn resolves_simplified_yaml_file_reference() {
        let dir = tempfile::tempdir().unwrap();
        write_schema_file(
            dir.path(),
            "schema.yaml",
            "$schema:\n  title: 'string(required)'\n",
        );
        let v = yaml_value("./schema.yaml");
        let resolved = resolve_yaml_schema(&v, dir.path()).unwrap();
        assert!(resolved.simplified.is_some());
        let required = resolved.json_schema["required"].as_array().unwrap();
        assert_eq!(required[0], "title");
    }

    #[test]
    fn resolves_raw_json_schema_file_reference() {
        let dir = tempfile::tempdir().unwrap();
        write_schema_file(
            dir.path(),
            "schema.json",
            r#"{"type":"object","properties":{"x":{"type":"number"}}}"#,
        );
        let v = yaml_value("./schema.json");
        let resolved = resolve_yaml_schema(&v, dir.path()).unwrap();
        assert!(resolved.simplified.is_none());
        assert_eq!(resolved.json_schema["properties"]["x"]["type"], "number");
    }

    #[test]
    fn yaml_without_schema_key_is_treated_as_json_schema() {
        let dir = tempfile::tempdir().unwrap();
        write_schema_file(
            dir.path(),
            "no-schema.yaml",
            "title: hello\nproperties:\n  x:\n    type: number\n",
        );
        let v = yaml_value("./no-schema.yaml");
        let resolved = resolve_yaml_schema(&v, dir.path()).unwrap();
        assert!(resolved.simplified.is_none());
    }

    #[test]
    fn non_object_yaml_is_ambiguous() {
        let dir = tempfile::tempdir().unwrap();
        write_schema_file(dir.path(), "bad.yaml", "- one\n- two\n");
        let v = yaml_value("./bad.yaml");
        let err = resolve_yaml_schema(&v, dir.path()).unwrap_err();
        assert!(matches!(err, SchemaError::AmbiguousReferenced { .. }));
    }

    #[test]
    fn missing_file_errors_with_unresolved() {
        let dir = tempfile::tempdir().unwrap();
        let v = yaml_value("./nope.yaml");
        let err = resolve_yaml_schema(&v, dir.path()).unwrap_err();
        assert!(matches!(err, SchemaError::Unresolved { .. }));
    }

    #[test]
    fn baseline_merge_adds_baseline_only_props() {
        let baseline = json!({
            "type": "object",
            "properties": {
                "owner": { "type": "string" }
            },
            "required": ["owner"]
        });
        let doc = json!({
            "type": "object",
            "properties": {
                "title": { "type": "string" }
            },
            "required": ["title"]
        });
        let merged = merge_baseline(&baseline, doc).unwrap();
        let props = merged["properties"].as_object().unwrap();
        assert!(props.contains_key("owner"));
        assert!(props.contains_key("title"));
        let required = merged["required"].as_array().unwrap();
        let names: Vec<&str> = required.iter().map(|v| v.as_str().unwrap()).collect();
        assert!(names.contains(&"owner"));
        assert!(names.contains(&"title"));
    }

    #[test]
    fn baseline_merge_document_wins_on_conflict() {
        let baseline = json!({
            "type": "object",
            "properties": {
                "title": { "type": "number" }
            }
        });
        let doc = json!({
            "type": "object",
            "properties": {
                "title": { "type": "string" }
            }
        });
        let merged = merge_baseline(&baseline, doc).unwrap();
        assert_eq!(merged["properties"]["title"]["type"], "string");
    }

    #[test]
    fn baseline_must_be_simple_object_schema() {
        let baseline = json!({
            "type": "object",
            "patternProperties": { "^x": {} }
        });
        let doc = json!({"type":"object","properties":{}});
        let err = merge_baseline(&baseline, doc).unwrap_err();
        assert!(matches!(err, SchemaError::Baseline { .. }));
    }

    #[test]
    fn baseline_rejects_additional_properties_false() {
        let baseline = json!({
            "type": "object",
            "properties": { "owner": { "type": "string" } },
            "additionalProperties": false,
        });
        let doc = json!({"type":"object","properties":{}});
        let err = merge_baseline(&baseline, doc).unwrap_err();
        match err {
            SchemaError::Baseline { message, .. } => {
                assert!(
                    message.contains("additionalProperties"),
                    "expected message to mention additionalProperties, got: {message}"
                );
            }
            other => panic!("expected SchemaError::Baseline, got {other:?}"),
        }
    }

    #[test]
    fn baseline_rejects_additional_properties_schema() {
        let baseline = json!({
            "type": "object",
            "properties": { "owner": { "type": "string" } },
            "additionalProperties": { "type": "string" },
        });
        let doc = json!({"type":"object","properties":{}});
        let err = merge_baseline(&baseline, doc).unwrap_err();
        assert!(matches!(err, SchemaError::Baseline { .. }));
    }

    #[test]
    fn baseline_permits_additional_properties_true() {
        // SimplifiedSchema-generated baselines emit `additionalProperties:
        // true`; permitting that keeps the generated path working.
        let baseline = json!({
            "type": "object",
            "properties": { "owner": { "type": "string" } },
            "additionalProperties": true,
        });
        let doc = json!({"type":"object","properties":{}});
        merge_baseline(&baseline, doc).unwrap();
    }

    #[test]
    fn baseline_merge_into_root_union_applies_per_arm() {
        let baseline = json!({
            "type": "object",
            "properties": {
                "owner": { "type": "string" }
            }
        });
        let doc = json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "anyOf": [
                {"type":"object","properties":{"title":{"type":"string"}}},
                {"type":"object","properties":{"name":{"type":"string"}}}
            ]
        });
        let merged = merge_baseline(&baseline, doc).unwrap();
        let arms = merged["anyOf"].as_array().unwrap();
        for arm in arms {
            assert!(arm["properties"].get("owner").is_some());
        }
    }

    // ── `generated` baseline merge audit (Phase 2) ──────────────────────────

    /// A baseline `generated; required` property must NOT get force-added to
    /// the merged `required` list. The convert layer suppresses the
    /// `required` entry for generated properties, so by the time the baseline
    /// reaches `merge_baseline` its `required` array already excludes the
    /// generated property — the merge has nothing to copy over. This test
    /// proves that contract end-to-end (SimplifiedSchema → JSON Schema →
    /// merge_baseline) and guards against a future regression that re-introduces
    /// generated properties into the baseline `required` list.
    #[test]
    fn baseline_generated_required_property_is_not_force_added_to_required() {
        // Build the baseline via the real convert path so the audit reflects
        // the actual baseline shape a library caller hands to merge_baseline.
        let baseline_yaml = "ctx_today: 'string(generated; required)'";
        let baseline_value: YamlValue = serde_yaml_ng::from_str(baseline_yaml).unwrap();
        let baseline_schema = parse_yaml_schema(&baseline_value).unwrap();
        let baseline_json = to_json_schema(&baseline_schema).unwrap();

        // Sanity: the generated property is absent from the baseline's own
        // `required` array (this is the convert-level suppression).
        assert!(
            baseline_json.get("required").is_none(),
            "baseline must not carry a static `required` entry for a generated property: {baseline_json:?}"
        );

        // A document schema that does not declare `ctx_today`.
        let doc = json!({
            "type": "object",
            "properties": {
                "title": { "type": "string" }
            }
        });
        let merged = merge_baseline(&baseline_json, doc).unwrap();

        // The generated property is copied across (it is baseline-only), but
        // the merged `required` list must NOT contain it.
        assert!(
            merged["properties"].get("ctx_today").is_some(),
            "generated baseline property should be merged into properties: {merged:?}"
        );
        let required = merged.get("required").and_then(|r| r.as_array());
        match required {
            None => { /* no required array at all — also fine */ }
            Some(arr) => {
                let names: Vec<&str> = arr.iter().filter_map(|v| v.as_str()).collect();
                assert!(
                    !names.contains(&"ctx_today"),
                    "generated baseline property must not be force-added to required: {merged:?}"
                );
            }
        }
    }

    /// End-to-end: a baseline that models `ctx` as a nested object with a
    /// `generated; required` inner `today` property merges cleanly, and an
    /// authored document omitting `ctx` entirely validates. This mirrors the
    /// motivating `darkmatter.yaml` baseline shape.
    #[test]
    fn baseline_nested_ctx_generated_validates_when_ctx_absent() {
        use crate::markdown::schemas::validate::build_validator;
        let baseline_yaml = "ctx:\n  today: \"date(generated; required)\"";
        let baseline_value: YamlValue = serde_yaml_ng::from_str(baseline_yaml).unwrap();
        let baseline_schema = parse_yaml_schema(&baseline_value).unwrap();
        let baseline_json = to_json_schema(&baseline_schema).unwrap();

        let doc = json!({"type": "object", "properties": {}});
        let merged = merge_baseline(&baseline_json, doc).unwrap();
        let validator = build_validator(&merged, None, None).unwrap();

        // Authored document omits `ctx` entirely — validates.
        assert!(
            validator.is_valid(&json!({ "title": "Hello" })),
            "authored doc omitting generated ctx must validate: merged={merged:?}"
        );
        // Host supplies a wrongly-typed `ctx.today` — fails type validation.
        assert!(
            !validator.is_valid(&json!({ "ctx": { "today": 42 } })),
            "wrongly-typed ctx.today must fail: merged={merged:?}"
        );
        // Host supplies a correctly-typed `ctx.today` — validates.
        assert!(
            validator.is_valid(&json!({ "ctx": { "today": "2026-07-04" } })),
            "correctly-typed ctx.today must validate: merged={merged:?}"
        );
    }
}
