//! Raw JSON Schema conversion (`2026-10-28-schema-enhancements`, criteria 33a
//! and 55): conversion follows every keyword the translator models, the
//! complete Draft 2020-12 schema still decides what is admitted, and a failed
//! conversion leaves the input as written.

use std::path::PathBuf;

use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::ComposeOptions;
use darkmatter::markdown::schemas::coerce::coerce_frontmatter;
use darkmatter::markdown::schemas::{DarkmatterSchemas, ValidationReport};
use serde_json::{Value, json};

use crate::request_support::{context_at, request};

/// A raw schema written as `schema.json` beside documents that reference it.
struct RawSchema {
    _dir: tempfile::TempDir,
    root: PathBuf,
}

impl RawSchema {
    fn new(schema: &Value) -> Self {
        let dir = tempfile::tempdir().expect("temp dir");
        let root = biscuit_file::canonicalize_simplified(dir.path()).expect("canonical temp dir");
        std::fs::write(root.join("schema.json"), serde_json::to_string_pretty(schema).expect("schema JSON"))
            .expect("write schema");
        Self { _dir: dir, root }
    }

    /// Writes `doc.md` referencing the schema with `frontmatter` (YAML lines).
    fn document(&self, frontmatter: &str) -> PathBuf {
        let path = self.root.join("doc.md");
        std::fs::write(&path, format!("---\n$schema: ./schema.json\n{frontmatter}---\nBody\n")).expect("write");
        path
    }

    fn validate(&self, frontmatter: &str) -> ValidationReport {
        let path = self.document(frontmatter);
        let markdown = Markdown::try_from(path.as_path()).expect("document loads");
        DarkmatterSchemas::new(context_at(&self.root)).validate(&markdown).expect("the schema loads")
    }

    /// The composed frontmatter (without `$schema`), or `None` when
    /// composition rejects the document; the source file is asserted
    /// unchanged either way.
    fn compose(&self, frontmatter: &str) -> Option<Value> {
        let path = self.document(frontmatter);
        let before = std::fs::read_to_string(&path).expect("read");
        let markdown = Markdown::try_from(path.as_path()).expect("document loads");
        let outcome = markdown.compose_with(&request(ComposeOptions::new().with_source_file(&path)));
        assert_eq!(std::fs::read_to_string(&path).expect("read"), before, "the source file is never rewritten");
        outcome.ok().map(|(composed, _)| {
            let mut value = serde_json::to_value(composed.frontmatter().as_map()).expect("frontmatter JSON");
            value.as_object_mut().expect("mapping").remove("$schema");
            value
        })
    }

    /// Validation and composition agree: `Some(converted)` when both accept.
    fn outcome(&self, frontmatter: &str) -> Option<Value> {
        let report = self.validate(frontmatter);
        let composed = self.compose(frontmatter);
        assert_eq!(report.valid, composed.is_some(), "{frontmatter}: {:?}", report.problems);
        composed
    }
}

fn root(properties: Value) -> Value {
    json!({ "type": "object", "properties": properties })
}

#[test]
fn ref_defs_type_array_oneof_convert_referenced_number() {
    let schema = RawSchema::new(&json!({
        "$defs": { "count": { "type": "number" } },
        "type": "object",
        "properties": {
            "referenced": { "$ref": "#/$defs/count" },
            "typed": { "type": ["number", "null"] },
            "either": { "oneOf": [{ "$ref": "#/$defs/count" }, { "type": ["boolean", "null"] }] }
        }
    }));
    assert_eq!(
        schema.outcome("referenced: \"3\"\ntyped: \"3\"\neither: \"3\"\n"),
        Some(json!({ "referenced": 3, "typed": 3, "either": 3 }))
    );
}

#[test]
fn oneof_overlap() {
    let schema = RawSchema::new(&root(json!({ "n": { "oneOf": [{ "type": "number" }, { "type": "integer" }] } })));
    // `3` matches both alternatives, so exactly-one fails.
    assert_eq!(schema.outcome("n: \"3\"\n"), None);
    assert_eq!(schema.outcome("n: \"2.5\"\n"), Some(json!({ "n": 2.5 })));
}

#[test]
fn ref_siblings() {
    let schema = RawSchema::new(&json!({
        "$defs": { "count": { "type": "number" } },
        "type": "object",
        "properties": { "n": { "$ref": "#/$defs/count", "not": { "const": 4 }, "description": "not four" } }
    }));
    assert_eq!(schema.outcome("n: \"3\"\n"), Some(json!({ "n": 3 })));
    assert_eq!(schema.outcome("n: \"4\"\n"), None);
}

#[test]
fn referenced_number_with_sibling_constraint() {
    let schema = RawSchema::new(&json!({
        "$defs": { "count": { "type": "number" } },
        "type": "object",
        "properties": { "n": { "$ref": "#/$defs/count", "maximum": 5 } }
    }));
    assert_eq!(schema.outcome("n: \"3\"\n"), Some(json!({ "n": 3 })));
    assert_eq!(schema.outcome("n: \"7\"\n"), None);
}

#[test]
fn overlapping_literal_and_pattern_keys() {
    let schema = RawSchema::new(&json!({
        "type": "object",
        "properties": { "n_count": { "type": "number" } },
        "patternProperties": { "^n_": { "type": "integer" } }
    }));
    assert_eq!(schema.outcome("n_count: \"3\"\nn_other: \"4\"\n"), Some(json!({ "n_count": 3, "n_other": 4 })));
    // The declared property alone would take `2.5`; the matching pattern
    // still applies.
    assert_eq!(schema.outcome("n_count: \"2.5\"\n"), None);
}

#[test]
fn typed_additional_keys() {
    let schema = RawSchema::new(&json!({
        "type": "object",
        "properties": { "name": { "type": "string" }, "$schema": { "type": "string" } },
        "additionalProperties": { "type": "number" }
    }));
    assert_eq!(schema.outcome("name: 5\nextra: \"3\"\n"), Some(json!({ "name": "5", "extra": 3 })));
    assert_eq!(schema.outcome("name: x\nextra: pear\n"), None);
}

#[test]
fn unmodeled_multiple_of() {
    let schema = RawSchema::new(&root(json!({ "n": { "type": "number", "multipleOf": 2 } })));
    assert_eq!(schema.outcome("n: \"3\"\n"), None);
    assert_eq!(schema.outcome("n: \"4\"\n"), Some(json!({ "n": 4 })));
    // Unmodeled keywords constrain the converted value too.
    let schema = RawSchema::new(&root(json!({
        "n": { "type": "number", "not": { "const": 4 }, "if": { "minimum": 10 }, "then": { "multipleOf": 5 } }
    })));
    assert_eq!(schema.outcome("n: \"4\"\n"), None);
    assert_eq!(schema.outcome("n: \"12\"\n"), None);
    assert_eq!(schema.outcome("n: \"15\"\n"), Some(json!({ "n": 15 })));
}

#[test]
fn cyclic_local_reference() {
    let schema = RawSchema::new(&json!({
        "$defs": {
            "node": {
                "type": "object",
                "properties": {
                    "value": { "type": "number" },
                    "children": { "type": "array", "items": { "$ref": "#/$defs/node" } }
                }
            }
        },
        "type": "object",
        "properties": { "tree": { "$ref": "#/$defs/node" } }
    }));
    assert_eq!(
        schema.outcome("tree:\n  value: \"1\"\n  children:\n    - value: \"2\"\n      children: []\n"),
        Some(json!({ "tree": { "value": 1, "children": [{ "value": 2, "children": [] }] } }))
    );
    // Below the expansion bound the value is validation-only: never
    // converted, never unconstrained.
    let mut deep_number = String::from("tree:\n");
    let mut deep_text = String::from("tree:\n");
    let mut indent = String::from("  ");
    for _ in 0..8 {
        deep_number.push_str(&format!("{indent}value: 1\n{indent}children:\n{indent}  -\n"));
        deep_text.push_str(&format!("{indent}value: 1\n{indent}children:\n{indent}  -\n"));
        indent.push_str("    ");
    }
    deep_number.push_str(&format!("{indent}value: 2\n"));
    deep_text.push_str(&format!("{indent}value: \"2\"\n"));
    assert!(schema.outcome(&deep_number).is_some());
    assert_eq!(schema.outcome(&deep_text), None);
}

#[test]
fn failure_leaves_input_unchanged() {
    let schema_value = root(json!({ "n": { "type": "number" }, "flag": { "type": "boolean" } }));
    let schema = RawSchema::new(&schema_value);
    let instance = json!({ "n": "3", "flag": "maybe" });
    let original = instance.clone();
    // The failing property stays as written, its sibling converts on its
    // own, and the caller's value is never mutated.
    let outcome = coerce_frontmatter(&schema_value, &instance);
    assert_eq!(outcome.value, json!({ "n": 3, "flag": "maybe" }));
    assert_eq!(instance, original);
    // Validation reports only the value that fails, and composition leaves
    // the source file as written (asserted by `compose`).
    let report = schema.validate("n: \"3\"\nflag: maybe\n");
    let paths: Vec<&str> = report.problems.iter().map(|problem| problem.path.as_str()).collect();
    assert_eq!(paths, ["/flag"], "{:?}", report.problems);
    assert_eq!(schema.outcome("n: \"3\"\nflag: maybe\n"), None);
}

#[test]
fn alternative_and_pattern_order_is_irrelevant() {
    let alternatives = [json!({ "type": "number", "minimum": 5 }), json!({ "type": "boolean" })];
    let patterns = [("^a", json!({ "type": "integer" })), ("b$", json!({ "type": "number", "maximum": 10 }))];
    let mut results = Vec::new();
    for reversed in [false, true] {
        let mut options = alternatives.to_vec();
        let mut keyed = patterns.to_vec();
        if reversed {
            options.reverse();
            keyed.reverse();
        }
        let pattern_properties: serde_json::Map<String, Value> =
            keyed.into_iter().map(|(pattern, schema)| (pattern.to_string(), schema)).collect();
        let schema = RawSchema::new(&json!({
            "type": "object",
            "properties": { "pick": { "anyOf": options } },
            "patternProperties": pattern_properties
        }));
        results.push((
            schema.outcome("pick: \"1\"\nab: \"3\"\n"),
            schema.outcome("pick: \"7\"\n"),
            schema.outcome("ab: \"2.5\"\n"),
            schema.outcome("ab: \"12\"\n"),
        ));
    }
    assert_eq!(
        results[0],
        (Some(json!({ "pick": true, "ab": 3 })), Some(json!({ "pick": 7 })), None, None)
    );
    assert_eq!(results[0], results[1]);
}

/// `$id` changes the base a nested reference resolves against, and `$anchor`
/// names a node within its own resource: neither is read as a root pointer.
#[test]
fn id_and_anchor_resolve_through_their_resource() {
    let schema = RawSchema::new(&json!({
        "$id": "https://example.com/schemas/root.json",
        "$defs": {
            "count": { "type": "boolean" },
            "inner": {
                "$id": "inner.json",
                "$defs": {
                    "count": { "type": "number" },
                    "word": { "$anchor": "word", "type": "string" }
                },
                "type": "object",
                "properties": {
                    "n": { "$ref": "#/$defs/count" },
                    "w": { "$ref": "#word" }
                }
            }
        },
        "type": "object",
        "properties": {
            "child": { "$ref": "inner.json" },
            "flag": { "$ref": "#/$defs/count" }
        }
    }));
    assert_eq!(
        schema.outcome("child:\n  n: \"1\"\n  w: 5\nflag: \"1\"\n"),
        Some(json!({ "child": { "n": 1, "w": "5" }, "flag": true }))
    );
}

#[test]
fn yaml_raw_schema_converts_like_json() {
    let dir = tempfile::tempdir().expect("temp dir");
    let root_dir = biscuit_file::canonicalize_simplified(dir.path()).expect("canonical temp dir");
    std::fs::write(
        root_dir.join("schema.yaml"),
        "$defs:\n  count:\n    type: number\ntype: object\nproperties:\n  n:\n    $ref: '#/$defs/count'\n",
    )
    .expect("write schema");
    let path = root_dir.join("doc.md");
    std::fs::write(&path, "---\n$schema: ./schema.yaml\nn: \"3\"\n---\nBody\n").expect("write");
    let (composed, _) = Markdown::try_from(path.as_path())
        .expect("document loads")
        .compose_with(&request(ComposeOptions::new().with_source_file(&path)))
        .expect("composition succeeds");
    assert_eq!(composed.frontmatter().get::<Value>("n").expect("reads"), Some(json!(3)));
}
