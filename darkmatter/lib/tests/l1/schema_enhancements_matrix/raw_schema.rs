//! Reader (c): raw JSON Schema documents, in both supported formats — a
//! `.json` file and a YAML file that is not a SimplifiedSchema envelope —
//! through the public validation result.
//!
//! Each row names one keyword at one location in a control fixture. The
//! control proves the keyword is load-bearing: removing it flips the sample
//! instance's validity or leaves a reference unresolved (a load error). The
//! invalid shapes are each rejected when the validator is built
//! (`SchemaError::BuildValidator`), through the Draft 2020-12 meta-schema. Edits are made on the decoded value and written
//! back with the same serializer that produced the fixture, so each cell
//! differs from the control by exactly one value.
//!
//! Absent here, because a pending ruling or a later task decides them:
//! duplicate keys in `.json` (R3; last-wins today), `enum: []` and
//! `prefixItems: []` (R7), conversion controls (Wave 3 Conversion core and
//! Wave 4 Schema adapter), and lossless numeric tokens (Wave 2 Numeric).

use std::path::Path;

use serde_json::Value;

use darkmatter::markdown::schemas::SchemaError;

use super::validate_document;

#[derive(Clone, Copy)]
enum Fixture {
    /// `md schema detect --format json` output.
    Detected,
    /// Hand-written keyword coverage.
    Keywords,
}

/// One replacement of the keyword's value.
#[derive(Clone, Copy, Debug)]
enum Shape {
    Null,
    /// The whole value, as JSON text.
    Whole(&'static str),
    /// Adds one bad element (JSON text) beside the existing good ones: pushed
    /// onto an array, or inserted as `matrix_bad` into an object.
    OneBad(&'static str),
    /// Replaces every element of the array or object with the bad value.
    EveryBad(&'static str),
}

use Shape::*;

/// What removing the keyword does to the row's instance.
#[derive(Clone, Copy)]
enum Removal {
    /// The schema still loads and the instance's validity flips.
    Flips,
    /// A reference the keyword satisfied is left unresolved: load error.
    Unresolved,
    /// No observable change today (see the row's comment).
    NoEffect,
}

use Removal::*;

struct Row {
    keyword: &'static str,
    fixture: Fixture,
    /// JSON Pointer to the keyword.
    pointer: &'static str,
    /// Frontmatter YAML for the sample document.
    instance: &'static str,
    valid_with: bool,
    removal: Removal,
    /// Shapes rejected when the validator is built.
    rejects: &'static [Shape],
    /// Shapes the meta-schema admits: the schema loads.
    loads: &'static [Shape],
}

/// A single subschema value.
const SCHEMA: &[Shape] = &[Null, Whole("5"), Whole("\"x\""), Whole("[]")];
/// A map of name to subschema.
const SCHEMA_MAP: &[Shape] = &[Null, Whole("5"), Whole("[]"), OneBad("5"), EveryBad("5")];
/// A non-empty array of subschemas.
const SCHEMA_ARRAY: &[Shape] = &[Null, Whole("5"), Whole("{}"), OneBad("5"), EveryBad("5")];
/// A non-negative integer.
const COUNT: &[Shape] = &[Null, Whole("\"2\""), Whole("-1"), Whole("[2]"), Whole("[]"), Whole("\"\"")];
/// Any number.
const NUMBER: &[Shape] = &[Null, Whole("\"5\""), Whole("[5]"), Whole("{}"), Whole("\"\"")];
/// A string (reference, identifier, anchor, regex, or format name).
const STRING: &[Shape] = &[Null, Whole("5"), Whole("[\"x\"]"), Whole("{}")];
/// An anchor name, which must also be non-empty.
const ANCHOR: &[Shape] = &[Null, Whole("5"), Whole("[\"x\"]"), Whole("\"\"")];

const fn row(
    keyword: &'static str,
    fixture: Fixture,
    pointer: &'static str,
    instance: &'static str,
    valid_with: bool,
    removal: Removal,
    rejects: &'static [Shape],
) -> Row {
    Row { keyword, fixture, pointer, instance, valid_with, removal, rejects, loads: &[] }
}

use Fixture::*;

const ROWS: &[Row] = &[
    // Keywords `md schema detect` emits, on its output.
    row("$schema", Detected, "/$schema", "count: word", false, NoEffect, STRING),
    row("properties", Detected, "/properties", "count: word", false, Flips, SCHEMA_MAP),
    row("anyOf", Detected, "/properties/count/anyOf", "count: word", false, Flips, SCHEMA_ARRAY),
    row(
        "type",
        Detected,
        "/properties/count/anyOf/1/type",
        "count: word",
        false,
        Flips,
        &[
            Null,
            Whole("5"),
            Whole("{}"),
            Whole("[]"),
            Whole("\"bogus\""),
            Whole("[\"integer\", \"bogus\"]"),
            Whole("[\"bogus\"]"),
        ],
    ),
    row("items", Detected, "/properties/scores/anyOf/1/items", "scores: [word]", false, Flips, SCHEMA),
    // Hand-written keyword coverage.
    row("$defs", Keywords, "/$defs", "ref_defs: 0", false, Unresolved, SCHEMA_MAP),
    row("definitions", Keywords, "/definitions", "ref_definitions: Word1", false, Unresolved, SCHEMA_MAP),
    row("$ref", Keywords, "/properties/ref_defs/$ref", "ref_defs: 0", false, Flips, STRING),
    row("$anchor", Keywords, "/$defs/flagged/$anchor", "anchored: 5", false, Unresolved, ANCHOR),
    row("$id", Keywords, "/$defs/identified/$id", "identified: 5", false, Unresolved, STRING),
    row("$dynamicRef", Keywords, "/properties/dynamic/$dynamicRef", "dynamic: word", false, Flips, STRING),
    row("$dynamicAnchor", Keywords, "/$defs/dynamic/$dynamicAnchor", "dynamic: word", false, Unresolved, ANCHOR),
    // `$vocabulary` only takes effect in a meta-schema, which no loader can
    // reach without a retriever: removal changes nothing a caller sees.
    row(
        "$vocabulary",
        Keywords,
        "/$vocabulary",
        "ref_defs: 1",
        true,
        NoEffect,
        &[Null, Whole("5"), Whole("[]"), EveryBad("5")],
    ),
    row(
        "patternProperties",
        Keywords,
        "/properties/pattern_props/patternProperties",
        "pattern_props: {n_a: word}",
        false,
        Flips,
        SCHEMA_MAP,
    ),
    row(
        "additionalProperties",
        Keywords,
        "/properties/additional/additionalProperties",
        "additional: {x: word}",
        false,
        Flips,
        SCHEMA,
    ),
    row("oneOf", Keywords, "/properties/one_of/oneOf", "one_of: 5", false, Flips, SCHEMA_ARRAY),
    row("prefixItems", Keywords, "/properties/prefix/prefixItems", "prefix: [word]", false, Flips, SCHEMA_ARRAY),
    row(
        "required",
        Keywords,
        "/properties/required_names/required",
        "required_names: {}",
        false,
        Flips,
        &[Null, Whole("5"), Whole("\"a\""), OneBad("5"), EveryBad("5")],
    ),
    Row {
        loads: &[OneBad("5"), EveryBad("{}")],
        ..row(
            "enum",
            Keywords,
            "/properties/enumerated/enum",
            "enumerated: c",
            false,
            Flips,
            &[Null, Whole("5"), Whole("{}"), Whole("\"a\"")],
        )
    },
    Row {
        loads: &[Null, Whole("5"), Whole("[1, {}]")],
        ..row("const", Keywords, "/properties/constant/const", "constant: b", false, Flips, &[])
    },
    // `default` is an annotation for a raw schema; no stage applies it here.
    Row {
        loads: &[Null, Whole("5"), Whole("{}")],
        ..row("default", Keywords, "/properties/defaulted/default", "defaulted: word", true, NoEffect, &[])
    },
    row("minimum", Keywords, "/properties/minimum/minimum", "minimum: 3", false, Flips, NUMBER),
    row("maximum", Keywords, "/properties/maximum/maximum", "maximum: 7", false, Flips, NUMBER),
    row(
        "exclusiveMinimum",
        Keywords,
        "/properties/exclusive_minimum/exclusiveMinimum",
        "exclusive_minimum: 5",
        false,
        Flips,
        NUMBER,
    ),
    row(
        "exclusiveMaximum",
        Keywords,
        "/properties/exclusive_maximum/exclusiveMaximum",
        "exclusive_maximum: 5",
        false,
        Flips,
        NUMBER,
    ),
    row(
        "multipleOf",
        Keywords,
        "/properties/multiple_of/multipleOf",
        "multiple_of: 3",
        false,
        Flips,
        &[Null, Whole("\"5\""), Whole("[5]"), Whole("{}"), Whole("\"\""), Whole("0")],
    ),
    row("minLength", Keywords, "/properties/min_length/minLength", "min_length: ab", false, Flips, COUNT),
    row("maxLength", Keywords, "/properties/max_length/maxLength", "max_length: ab", false, Flips, COUNT),
    row("minItems", Keywords, "/properties/min_items/minItems", "min_items: [1]", false, Flips, COUNT),
    row("maxItems", Keywords, "/properties/max_items/maxItems", "max_items: [1, 2]", false, Flips, COUNT),
    row(
        "minProperties",
        Keywords,
        "/properties/min_properties/minProperties",
        "min_properties: {}",
        false,
        Flips,
        COUNT,
    ),
    row(
        "maxProperties",
        Keywords,
        "/properties/max_properties/maxProperties",
        "max_properties: {a: 1, b: 2}",
        false,
        Flips,
        COUNT,
    ),
    row(
        "pattern",
        Keywords,
        "/properties/patterned/pattern",
        "patterned: A1",
        false,
        Flips,
        &[Null, Whole("5"), Whole("[\"x\"]"), Whole("{}"), Whole("\"[\"")],
    ),
    row("format", Keywords, "/properties/formatted/format", "formatted: not-a-date", false, Flips, STRING),
    row(
        "uniqueItems",
        Keywords,
        "/properties/unique/uniqueItems",
        "unique: [1, 1]",
        false,
        Flips,
        &[Null, Whole("\"true\""), Whole("[true]"), Whole("\"\""), Whole("1")],
    ),
    row("allOf", Keywords, "/properties/all_of/allOf", "all_of: word", false, Flips, SCHEMA_ARRAY),
    row("not", Keywords, "/properties/negated/not", "negated: 5", false, Flips, SCHEMA),
    row("if", Keywords, "/properties/conditional/if", "conditional: 3", false, Flips, SCHEMA),
    row("then", Keywords, "/properties/conditional/then", "conditional: 3", false, Flips, SCHEMA),
    row("else", Keywords, "/properties/conditional/else", "conditional: ab", false, Flips, SCHEMA),
    row(
        "dependentRequired",
        Keywords,
        "/properties/dependent_required/dependentRequired",
        "dependent_required: {a: 1}",
        false,
        Flips,
        &[Null, Whole("5"), Whole("[]"), OneBad("5"), EveryBad("5")],
    ),
    row(
        "dependentSchemas",
        Keywords,
        "/properties/dependent_schemas/dependentSchemas",
        "dependent_schemas: {a: 1}",
        false,
        Flips,
        SCHEMA_MAP,
    ),
    row(
        "propertyNames",
        Keywords,
        "/properties/property_names/propertyNames",
        "property_names: {ab: 1}",
        false,
        Flips,
        SCHEMA,
    ),
    row("contains", Keywords, "/properties/contained/contains", "contained: [word]", false, Flips, SCHEMA),
    row("minContains", Keywords, "/properties/contained/minContains", "contained: [1]", false, Flips, COUNT),
    row(
        "maxContains",
        Keywords,
        "/properties/contained/maxContains",
        "contained: [1, 2, 3, 4]",
        false,
        Flips,
        COUNT,
    ),
    row(
        "unevaluatedItems",
        Keywords,
        "/properties/unevaluated_items/unevaluatedItems",
        "unevaluated_items: [1, 2]",
        false,
        Flips,
        SCHEMA,
    ),
    row(
        "unevaluatedProperties",
        Keywords,
        "/properties/unevaluated_properties/unevaluatedProperties",
        "unevaluated_properties: {a: 1, b: 1}",
        false,
        Flips,
        SCHEMA,
    ),
];

#[derive(Clone, Copy)]
enum Format {
    Json,
    Yaml,
}

impl Format {
    fn extension(self) -> &'static str {
        match self {
            Format::Json => "json",
            Format::Yaml => "yaml",
        }
    }

    fn write(self, value: &Value) -> String {
        match self {
            Format::Json => serde_json::to_string_pretty(value).expect("JSON") + "\n",
            Format::Yaml => serde_yaml_ng::to_string(value).expect("YAML"),
        }
    }

    fn read(self, text: &str) -> Value {
        match self {
            Format::Json => serde_json::from_str(text).expect("fixture JSON"),
            Format::Yaml => serde_yaml_ng::from_str(text).expect("fixture YAML"),
        }
    }
}

/// Splits a pointer into its parent and final token.
fn parent_and_key(pointer: &str) -> (&str, String) {
    let at = pointer.rfind('/').expect("pointer");
    (&pointer[..at], pointer[at + 1..].replace("~1", "/").replace("~0", "~"))
}

fn without(control: &Value, pointer: &str) -> Value {
    let mut value = control.clone();
    let (parent, key) = parent_and_key(pointer);
    let removed = value
        .pointer_mut(parent)
        .and_then(Value::as_object_mut)
        .and_then(|object| object.remove(&key));
    assert!(removed.is_some(), "{pointer} not found in the control");
    value
}

fn reshaped(control: &Value, pointer: &str, shape: Shape) -> Value {
    let mut value = control.clone();
    let target = value.pointer_mut(pointer).unwrap_or_else(|| panic!("{pointer} not found"));
    let parse = |text: &str| serde_json::from_str::<Value>(text).expect("shape JSON");
    match shape {
        Null => *target = Value::Null,
        Whole(text) => *target = parse(text),
        OneBad(text) => match target {
            Value::Array(items) => items.push(parse(text)),
            Value::Object(entries) => {
                entries.insert("matrix_bad".into(), parse(text));
            }
            other => panic!("{pointer}: OneBad needs a collection, found {other}"),
        },
        EveryBad(text) => match target {
            Value::Array(items) => items.iter_mut().for_each(|item| *item = parse(text)),
            Value::Object(entries) => entries.values_mut().for_each(|item| *item = parse(text)),
            other => panic!("{pointer}: EveryBad needs a collection, found {other}"),
        },
    }
    value
}

/// Writes one cell and validates the row's instance against it.
fn run_cell(dir: &Path, format: Format, index: &mut usize, text: &str, instance: &str) -> Result<bool, SchemaError> {
    *index += 1;
    let file_name = format!("cell-{index}.{}", format.extension());
    std::fs::write(dir.join(&file_name), text).expect("write cell schema");
    validate_document(dir, &format!("$schema: ./{file_name}\n{instance}")).map(|report| report.valid)
}

/// Walks every row against the two controls of one format, then the
/// whole-document cells, which are the same for every keyword.
fn walk(format: Format, detected: &str, keywords: &str, whole_document: &[(&str, String)]) {
    let workspace = tempfile::tempdir().expect("matrix workspace");
    let dir = workspace.path();
    let mut index = 0;
    let mut failures = Vec::new();
    let detected_value = format.read(detected);
    let keywords_value = format.read(keywords);
    for (name, text, value) in [("detected", detected, &detected_value), ("keywords", keywords, &keywords_value)] {
        assert_eq!(format.write(value), text, "{name}: the control must be the serializer's own output");
    }

    for row in ROWS {
        let (control_text, control) = match row.fixture {
            Detected => (detected, &detected_value),
            Keywords => (keywords, &keywords_value),
        };
        let mut check = |label: String, text: String, expected: Result<Option<bool>, ()>| {
            let result = run_cell(dir, format, &mut index, &text, row.instance);
            let ok = match (&expected, &result) {
                (Ok(Some(valid)), Ok(actual)) => valid == actual,
                (Ok(None), Ok(_)) | (Err(()), Err(SchemaError::BuildValidator { .. })) => true,
                _ => false,
            };
            if !ok {
                let wanted = match expected {
                    Ok(Some(valid)) => format!("loads, valid={valid}"),
                    Ok(None) => "loads".to_string(),
                    Err(()) => "a BuildValidator load error".to_string(),
                };
                failures.push(format!(
                    "{} {label}: expected {wanted}, got {result:?}\n--- schema ---\n{text}",
                    row.keyword
                ));
            }
        };
        check("control".into(), control_text.to_string(), Ok(Some(row.valid_with)));
        let absent = format.write(&without(control, row.pointer));
        let removal = match row.removal {
            Flips => Ok(Some(!row.valid_with)),
            Unresolved => Err(()),
            NoEffect => Ok(Some(row.valid_with)),
        };
        check("absent".into(), absent, removal);
        for shape in row.rejects {
            check(format!("{shape:?}"), format.write(&reshaped(control, row.pointer, *shape)), Err(()));
        }
        for shape in row.loads {
            check(format!("{shape:?}"), format.write(&reshaped(control, row.pointer, *shape)), Ok(None));
        }
    }

    for (label, text) in whole_document {
        if let Ok(valid) = run_cell(dir, format, &mut index, text, "count: 1") {
            failures.push(format!("{label}: expected a load error, got valid={valid}\n--- schema ---\n{text}"));
        }
    }
    assert!(failures.is_empty(), "{} cell(s) failed:\n\n{}", failures.len(), failures.join("\n\n"));
}

#[test]
fn raw_json_schema_keyword_matrix() {
    let detected = include_str!("../../fixtures/schema_enhancements_matrix/detected.schema.json");
    let keywords = include_str!("../../fixtures/schema_enhancements_matrix/keywords.schema.json");
    // The error class is `AmbiguousReferenced` with no detail today; the Wave
    // 4 Schema adapter owns it, so only the failure is asserted.
    let whole_document = [
        ("trailing garbage", format!("{detected}]")),
        ("second value", format!("{detected}{{}}")),
    ];
    walk(Format::Json, detected, keywords, &whole_document);
}

#[test]
fn raw_yaml_schema_keyword_matrix() {
    let detected = include_str!("../../fixtures/schema_enhancements_matrix/detected.schema.yaml");
    let keywords = include_str!("../../fixtures/schema_enhancements_matrix/keywords.schema.yaml");
    // As for JSON, the reported class is `AmbiguousReferenced` today and the
    // Wave 4 Schema adapter owns it.
    let whole_document = [
        ("trailing garbage", format!("{detected}]\n")),
        ("second document", format!("{detected}---\ntype: object\n")),
        ("duplicate root key", detected.replacen("\ntype: object\n", "\ntype: object\ntype: object\n", 1)),
    ];
    assert_ne!(whole_document[2].1, detected, "the duplicate edit must change the control");
    walk(Format::Yaml, detected, keywords, &whole_document);
}
