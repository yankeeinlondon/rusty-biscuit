//! Snapshot coverage for `SimplifiedSchema → JSON Schema` conversion.
//!
//! Each test mirrors a row of the mapping table in
//! `darkmatter/features/2026-05-11-schemas/spec.md`. Snapshots live under
//! `tests/snapshots/schemas_convert_snapshots__*.snap` and are reviewed via
//! `cargo insta`.

use darkmatter::markdown::schemas::{
    PropertyAtom, PropertyDef, SchemaShape, SimplifiedSchema, parse_yaml_schema,
    simplified::{grammar::parse_type_expr, to_json_schema},
};
use insta::assert_snapshot;

/// Convert a YAML document string into a pretty-printed JSON Schema string
/// suitable for snapshot diffing.
fn convert_yaml(yaml: &str) -> String {
    let parsed: serde_yaml_ng::Value = serde_yaml_ng::from_str(yaml).expect("yaml parse");
    let schema = parse_yaml_schema(&parsed).expect("schema parse");
    let value = to_json_schema(&schema).expect("convert");
    serde_json::to_string_pretty(&value).expect("serialize")
}

/// Convert a single type expression string into its JSON Schema by wrapping
/// it in a one-property schema named `field`.
fn convert_atom(type_expr: &str) -> String {
    let atom: PropertyAtom = parse_type_expr("field", type_expr).expect("parse atom");
    let mut shape = SchemaShape::new();
    shape
        .properties
        .insert("field".to_string(), PropertyDef::Single(atom));
    let schema = SimplifiedSchema::Single(shape);
    let value = to_json_schema(&schema).expect("convert");
    serde_json::to_string_pretty(&value).expect("serialize")
}

// ── Scalar atoms ─────────────────────────────────────────────────────────

#[test]
fn snapshot_atom_string() {
    assert_snapshot!(convert_atom("string"));
}

#[test]
fn snapshot_atom_string_min_max() {
    assert_snapshot!(convert_atom("string(min(5); max(80))"));
}

#[test]
fn snapshot_atom_string_not_empty() {
    assert_snapshot!(convert_atom("string(not-empty)"));
}

#[test]
fn snapshot_atom_string_pattern() {
    assert_snapshot!(convert_atom("string(pattern(^[a-z]+$))"));
}

#[test]
fn snapshot_atom_date() {
    assert_snapshot!(convert_atom("date"));
}

#[test]
fn snapshot_atom_datetime() {
    assert_snapshot!(convert_atom("datetime"));
}

#[test]
fn snapshot_atom_time() {
    assert_snapshot!(convert_atom("time"));
}

#[test]
fn snapshot_atom_number() {
    assert_snapshot!(convert_atom("number"));
}

#[test]
fn snapshot_atom_number_integer_with_min() {
    assert_snapshot!(convert_atom("number(integer; min(0))"));
}

#[test]
fn snapshot_atom_numberlike() {
    assert_snapshot!(convert_atom("numberlike"));
}

#[test]
fn snapshot_atom_boolean() {
    assert_snapshot!(convert_atom("boolean"));
}

#[test]
fn snapshot_atom_boolish() {
    assert_snapshot!(convert_atom("boolish"));
}

#[test]
fn snapshot_atom_object() {
    assert_snapshot!(convert_atom("object"));
}

#[test]
fn snapshot_atom_file_bare() {
    assert_snapshot!(convert_atom("file"));
}

#[test]
fn snapshot_atom_file_match() {
    assert_snapshot!(convert_atom("file(match('*.md', '!_*.md'))"));
}

#[test]
fn snapshot_atom_file_array_min_items() {
    assert_snapshot!(convert_atom("file(match('*.png'))[](min(1))"));
}

#[test]
fn snapshot_atom_enum_members() {
    assert_snapshot!(convert_atom("enum(red, green, blue)"));
}

#[test]
fn snapshot_atom_url_bare() {
    assert_snapshot!(convert_atom("url"));
}

#[test]
fn snapshot_atom_url_scheme() {
    assert_snapshot!(convert_atom("url(scheme(https))"));
}

#[test]
fn snapshot_atom_email() {
    assert_snapshot!(convert_atom("email"));
}

#[test]
fn snapshot_atom_any() {
    assert_snapshot!(convert_atom("any"));
}

// ── Array wrappings ──────────────────────────────────────────────────────

#[test]
fn snapshot_array_string_with_pattern_and_array_constraints() {
    assert_snapshot!(convert_atom(
        "string(pattern(^[a-z]+$))[](min(1); max(5); unique)"
    ));
}

// ── Defaults & required ──────────────────────────────────────────────────

#[test]
fn snapshot_default_and_required_atoms() {
    let yaml = r#"
slug:   "string(not-empty; required)"
rating: "number(min(0); max(5); default(3))"
"#;
    assert_snapshot!(convert_yaml(yaml));
}

#[test]
fn snapshot_enum_with_default_and_required() {
    let yaml = r#"
status: "enum(draft, published, archived; default(draft); required)"
"#;
    assert_snapshot!(convert_yaml(yaml));
}

// ── Descriptions ─────────────────────────────────────────────────────────

#[test]
fn snapshot_atom_with_description() {
    assert_snapshot!(convert_atom("string(required) -> The author's full name"));
}

#[test]
fn snapshot_array_with_description() {
    assert_snapshot!(convert_atom("string[] -> A list of tags"));
}

// ── Property-level unions ────────────────────────────────────────────────

#[test]
fn snapshot_property_union_basic() {
    let yaml = r#"
foo:
  - string
  - number
"#;
    assert_snapshot!(convert_yaml(yaml));
}

#[test]
fn snapshot_property_union_hoists_required() {
    let yaml = r#"
title:
  - "string(required; not-empty)"
  - "number"
"#;
    assert_snapshot!(convert_yaml(yaml));
}

#[test]
fn snapshot_property_union_hoists_matching_default() {
    let yaml = r#"
flag:
  - "boolean(default(true))"
  - "boolish(default(true))"
"#;
    assert_snapshot!(convert_yaml(yaml));
}

// ── Root-level unions ────────────────────────────────────────────────────

#[test]
fn snapshot_root_union_inline_only() {
    let yaml = r#"
- title: "string(required)"
  body:  string
- name:  "string(required)"
"#;
    assert_snapshot!(convert_yaml(yaml));
}

// ── End-to-end document ──────────────────────────────────────────────────

#[test]
fn snapshot_full_document_schema() {
    let yaml = r#"
title:  "string(required; not-empty)"
slug:   "string(pattern(^[a-z0-9-]+$); required)"
tags:   "string[](min(1); max(5); unique)"
author:
  - "string(not-empty)"
  - email
rating: "number(min(0); max(5); default(3))"
status: "enum(draft, published, archived; default(draft))"
homepage: "url(scheme(https))"
hero:   "file(match('*.png', '*.jpg'))"
"#;
    assert_snapshot!(convert_yaml(yaml));
}

// ── Inline object literals (Phase 2) ─────────────────────────────────────

#[test]
fn snapshot_inline_object_bare() {
    let yaml = r#"
config: "{ host: string(required), port: number(default(8080)) }"
"#;
    assert_snapshot!(convert_yaml(yaml));
}

#[test]
fn snapshot_inline_object_array() {
    let yaml = r#"
entries: "{ foo: string(required), bar: string }[]"
"#;
    assert_snapshot!(convert_yaml(yaml));
}

#[test]
fn snapshot_inline_object_required_value() {
    let yaml = r#"
config_required: "{ host: string }(required)"
"#;
    assert_snapshot!(convert_yaml(yaml));
}

#[test]
fn snapshot_inline_object_constrained_array() {
    let yaml = r#"
replicas: "{ host: string }[](min(1); required)"
"#;
    assert_snapshot!(convert_yaml(yaml));
}

#[test]
fn snapshot_inline_object_nested() {
    let yaml = r#"
database: "{
    primary: { host: string(required), port: number(default(5432)) },
    replicas: { host: string, port: number }[]
}"
"#;
    assert_snapshot!(convert_yaml(yaml));
}

#[test]
fn snapshot_inline_object_multi_line() {
    let yaml = r#"
endpoints: "{
    url: url(scheme(https); required),
    method: enum(GET, POST, PUT, DELETE; required),
    timeout: number(default(30))
}[]"
"#;
    assert_snapshot!(convert_yaml(yaml));
}

#[test]
fn snapshot_inline_object_as_union_arm() {
    let yaml = r#"
payload:
  - "{ type: enum(foo; required), foo_id: string(required) }"
  - "{ type: enum(bar; required), bar_count: number(required) }"
"#;
    assert_snapshot!(convert_yaml(yaml));
}

#[test]
fn snapshot_inline_object_mixed_with_string_fallback() {
    let yaml = r#"
metadata:
  - "{ key: string(required), value: string(required) }[]"
  - "string"
"#;
    assert_snapshot!(convert_yaml(yaml));
}

#[test]
fn snapshot_inline_object_empty() {
    let yaml = r#"
empty: "{}"
"#;
    assert_snapshot!(convert_yaml(yaml));
}
