//! Public lowering and validation evidence for conjunctive constraints:
//! repeated `pattern`/`not-empty`, merged bounds, and grouped `match(...)`.
//!
//! Repeats are authored directly on one atom (the grammar allows them), so
//! these tests hold independently of named-type merging in the resolver.

use std::path::PathBuf;

use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::ComposeSource;
use darkmatter::markdown::schemas::simplified::{Constraint, PropertyDef, SimplifiedSchema, parse_yaml_schema};
use darkmatter::markdown::schemas::{DarkmatterSchemas, SchemaError, SchemaPhase, ValidationReport, to_json_schema};
use serde_json::{Value, json};

use crate::request_support::context_at;

/// A temp directory holding real files, canonicalized so the document path
/// and the resolution context agree on every OS.
struct Fixture {
    _dir: tempfile::TempDir,
    root: PathBuf,
}

impl Fixture {
    fn new(files: &[&str]) -> Self {
        let dir = tempfile::tempdir().expect("temp dir");
        let root = biscuit_file::canonicalize_simplified(dir.path()).expect("canonical temp dir");
        for file in files {
            std::fs::write(root.join(file), "x\n").expect("write fixture file");
        }
        Self { _dir: dir, root }
    }

    fn validate(&self, document: &str) -> ValidationReport {
        self.try_validate(document, None).expect("schema resolves")
    }

    fn try_validate(&self, document: &str, phase: Option<SchemaPhase>) -> Result<ValidationReport, SchemaError> {
        let path = self.root.join("doc.md");
        std::fs::write(&path, document).expect("write document");
        let markdown = Markdown::from(document).with_source(ComposeSource::File(path));
        let api = DarkmatterSchemas::new(context_at(&self.root));
        match phase {
            Some(phase) => api.validate_for_phase(&markdown, phase),
            None => api.validate(&markdown),
        }
    }
}

fn document(schema: &str, frontmatter: &str) -> String {
    format!("---\n$schema:\n{schema}{frontmatter}---\nBody\n")
}

fn lower(yaml: &str) -> Result<Value, SchemaError> {
    let value: serde_yaml_ng::Value = serde_yaml_ng::from_str(yaml).expect("yaml");
    to_json_schema(&parse_yaml_schema(&value).expect("schema parses"))
}

fn convert_error(yaml: &str) -> String {
    match lower(yaml) {
        Err(error @ SchemaError::Convert { .. }) => error.to_string(),
        other => panic!("`{yaml}` must be a conversion error, got {other:?}"),
    }
}

fn accepts(fixture: &Fixture, schema: &str, frontmatter: &str) -> bool {
    let report = fixture.validate(&document(schema, frontmatter));
    report.valid
}

// ── patterns ──────────────────────────────────────────────────────────────

#[test]
fn a_value_must_satisfy_every_pattern() {
    let fixture = Fixture::new(&[]);
    let schema = "  code: \"string(pattern(^a); pattern(z$))\"\n";
    assert!(accepts(&fixture, schema, "code: abz\n"));
    assert!(!accepts(&fixture, schema, "code: abc\n"), "satisfies only the first pattern");
    assert!(!accepts(&fixture, schema, "code: xyz\n"), "satisfies only the second pattern");
}

#[test]
fn not_empty_and_a_pattern_both_apply() {
    let fixture = Fixture::new(&[]);
    let schema = "  code: \"string(not-empty; pattern('^[ a-z]*$'))\"\n";
    assert!(accepts(&fixture, schema, "code: ab\n"));
    assert!(!accepts(&fixture, schema, "code: '   '\n"), "matches the pattern but is blank");
    assert!(!accepts(&fixture, schema, "code: AB\n"), "not blank but outside the pattern");
}

#[test]
fn several_patterns_lower_to_all_of_beside_the_base_keywords() {
    let schema = lower(
        "code: \"string(required; not-empty; pattern(^[a-z]+$); max(9); suggest(ab); example(./a.md)) -> A code\"\n",
    )
    .unwrap();
    assert_eq!(
        schema["properties"]["code"],
        json!({
            "type": "string",
            "maxLength": 9,
            "description": "A code",
            "x-darkmatter-suggest": ["ab"],
            "x-darkmatter-example": ["./a.md"],
            "allOf": [{ "pattern": r"\S" }, { "pattern": "^[a-z]+$" }]
        })
    );
}

/// AC 10: a schema with no repeated constraint lowers as it always has.
#[test]
fn schemas_without_repeats_keep_their_lowered_output() {
    let schema = lower(
        "title: \"string(required; not-empty; max(80))\"\n\
         slug: \"string(pattern(^[a-z-]+$))\"\n\
         count: \"number(min(0); max(10); integer; default(3))\"\n\
         tags: \"string[](min(1); unique)\"\n\
         site: \"url(scheme(https))\"\n\
         meta: \"{ a: string }(min-keys(1))\"\n",
    )
    .unwrap();
    assert_eq!(
        schema,
        json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object",
            "additionalProperties": true,
            "required": ["title"],
            "properties": {
                "title": { "type": "string", "pattern": r"\S", "maxLength": 80 },
                "slug": { "anyOf": [{ "type": "null" }, { "type": "string", "pattern": "^[a-z-]+$" }] },
                "count": {
                    "anyOf": [{ "type": "null" }, { "type": "integer", "minimum": 0, "maximum": 10, "x-darkmatter-integer": true }],
                    "default": 3
                },
                "tags": {
                    "anyOf": [
                        { "type": "null" },
                        { "type": "array", "items": { "type": "string" }, "minItems": 1, "uniqueItems": true }
                    ]
                },
                "site": {
                    "anyOf": [
                        { "type": "null" },
                        { "type": "string", "format": "uri", "x-darkmatter-url-scheme": ["https"] }
                    ]
                },
                "meta": {
                    "anyOf": [
                        { "type": "null" },
                        {
                            "type": "object",
                            "additionalProperties": false,
                            "properties": { "a": { "anyOf": [{ "type": "null" }, { "type": "string" }] } },
                            "minProperties": 1
                        }
                    ]
                }
            }
        })
    );
}

// ── bounds, schemes, defaults ─────────────────────────────────────────────

#[test]
fn repeated_bounds_keep_the_stricter_value_in_validation() {
    let fixture = Fixture::new(&[]);
    let text = "  name: \"string(min(3); min(5); max(8); max(6))\"\n";
    assert!(!accepts(&fixture, text, "name: abcd\n"), "the larger minimum wins");
    assert!(accepts(&fixture, text, "name: abcdef\n"));
    assert!(!accepts(&fixture, text, "name: abcdefg\n"), "the smaller maximum wins");

    let number = "  n: \"number(min(1); min(2); max(9); max(4))\"\n";
    assert!(!accepts(&fixture, number, "n: 1\n"));
    assert!(accepts(&fixture, number, "n: 4\n"));
    assert!(!accepts(&fixture, number, "n: 5\n"));

    let items = "  tags: \"string[](min(1); min(2); max(3))\"\n";
    assert!(!accepts(&fixture, items, "tags: [a]\n"));
    assert!(accepts(&fixture, items, "tags: [a, b]\n"));

    let keys = "  meta: \"{ <string>: any }(min-keys(1); min-keys(2); max-keys(3); max-keys(2))\"\n";
    assert!(!accepts(&fixture, keys, "meta: {a: 1}\n"));
    assert!(accepts(&fixture, keys, "meta: {a: 1, b: 2}\n"));
    assert!(!accepts(&fixture, keys, "meta: {a: 1, b: 2, c: 3}\n"));
}

#[test]
fn block_and_postfix_key_bounds_merge_as_one_scope() {
    let value: serde_yaml_ng::Value =
        serde_yaml_ng::from_str("meta:\n  \"<string>\": any\n  $constraints:\n    min-keys: 1\n    max-keys: 3\n")
            .unwrap();
    let SimplifiedSchema::Single(mut shape) = parse_yaml_schema(&value).unwrap() else {
        panic!("single schema");
    };
    let Some(PropertyDef::Single(atom)) = shape.properties.get_mut("meta") else {
        panic!("single property");
    };
    // The postfix form on the same object, as a constrained reuse produces.
    atom.constraints.extend([Constraint::MinKeys(2), Constraint::MaxKeys(4)]);
    let schema = to_json_schema(&SimplifiedSchema::Single(shape.clone())).unwrap();
    let object = &schema["properties"]["meta"]["anyOf"][1];
    assert_eq!((&object["minProperties"], &object["maxProperties"]), (&json!(2), &json!(3)));

    let Some(PropertyDef::Single(atom)) = shape.properties.get_mut("meta") else {
        unreachable!()
    };
    atom.constraints.push(Constraint::MaxKeys(1));
    let error = to_json_schema(&SimplifiedSchema::Single(shape)).unwrap_err();
    assert!(error.to_string().contains("`min-keys(2)` is greater than `max-keys(1)`"), "{error}");
}

#[test]
fn equal_bounds_are_valid_and_contradictory_bounds_are_errors() {
    let fixture = Fixture::new(&[]);
    let schema = "  code: \"string(min(3); max(3))\"\n";
    assert!(accepts(&fixture, schema, "code: abc\n"));
    assert!(!accepts(&fixture, schema, "code: abcd\n"));

    for (yaml, names) in [
        ("p: \"string(min(10); max(5))\"\n", ["`min(10)`", "`max(5)`"]),
        ("p: \"number(min(2); max(1))\"\n", ["`min(2)`", "`max(1)`"]),
        ("p: \"string[](min(3); max(2))\"\n", ["`min(3)`", "`max(2)`"]),
        ("p: \"{ <string>: any }(min-keys(2); max-keys(1))\"\n", ["`min-keys(2)`", "`max-keys(1)`"]),
    ] {
        let message = convert_error(yaml);
        assert!(message.contains("p:") || message.contains("`p`"), "names the property: {message}");
        for name in names {
            assert!(message.contains(name), "{yaml}: {message}");
        }
    }
}

#[test]
fn repeated_schemes_intersect() {
    let fixture = Fixture::new(&[]);
    let schema = "  site: \"url(scheme(http, https); scheme(https))\"\n";
    assert!(accepts(&fixture, schema, "site: https://example.com\n"));
    assert!(!accepts(&fixture, schema, "site: http://example.com\n"));
    let message = convert_error("site: \"url(scheme(http); scheme(https))\"\n");
    assert!(message.contains("share no scheme"), "{message}");
}

#[test]
fn repeated_defaults_compare_after_number_normalization() {
    let schema = lower("n: \"number(default(3); default(3.0))\"\n").unwrap();
    assert_eq!(schema["properties"]["n"]["default"], json!(3));
    let message = convert_error("v: \"any(default('3'); default(3))\"\n");
    assert!(message.contains("conflicting `default(...)` values"), "{message}");
}

// ── `$constraints` presence flags and invalid keys ───────────────────────

#[test]
fn block_required_makes_the_object_property_required_like_the_postfix() {
    let fixture = Fixture::new(&[]);
    let block = "  meta:\n    a: string\n    $constraints:\n      required: true\n";
    let postfix = "  meta: \"{ a: string }(required)\"\n";
    for schema in [block, postfix] {
        assert!(!accepts(&fixture, schema, ""), "absent: {schema}");
        assert!(accepts(&fixture, schema, "meta: {a: x}\n"), "present: {schema}");
        assert!(!accepts(&fixture, schema, "meta: {b: x}\n"), "an undeclared key fails: {schema}");
        let completion = fixture
            .try_validate(&document(schema, ""), Some(SchemaPhase::Completion))
            .expect("phase validation");
        assert!(!completion.valid, "effective validation enforces presence: {schema}");
        let launch = fixture
            .try_validate(&document(schema, ""), Some(SchemaPhase::Launch))
            .expect("phase validation");
        assert!(launch.valid, "launch defers a non-eager presence: {schema}");
    }
}

#[test]
fn block_generated_defers_presence_in_authored_validation_like_the_postfix() {
    let fixture = Fixture::new(&[]);
    let block = "  meta:\n    a: string\n    $constraints:\n      required: true\n      generated: true\n";
    let postfix = "  meta: \"{ a: string }(required; generated)\"\n";
    assert_eq!(
        lower("meta:\n  a: string\n  $constraints:\n    required: true\n    generated: true\n").unwrap(),
        lower("meta: \"{ a: string }(required; generated)\"\n").unwrap(),
        "both spellings lower identically"
    );
    for schema in [block, postfix] {
        assert!(accepts(&fixture, schema, ""), "authored: absent is permitted: {schema}");
        assert!(!accepts(&fixture, schema, "meta: {b: x}\n"), "a supplied invalid value fails: {schema}");
        let completion = fixture
            .try_validate(&document(schema, ""), Some(SchemaPhase::Completion))
            .expect("phase validation");
        assert!(!completion.valid, "effective validation enforces presence: {schema}");
    }
}

#[test]
fn block_keys_with_no_object_meaning_are_errors() {
    for key in ["not-empty: true", "unique: true", "integer: true", "min-items: 1", "max-items: 1"] {
        let name = key.split(':').next().unwrap();
        let message = convert_error(&format!("meta:\n  a: string\n  $constraints:\n    {key}\n"));
        assert!(
            message.contains(&format!("`$constraints` key `{name}` is not valid on an inline object")),
            "{key}: {message}"
        );
    }
    for key in ["required", "generated", "not-empty"] {
        let message = convert_error(&format!("a: string\n$constraints:\n  {key}: true\n"));
        assert!(
            message.contains(&format!("`$constraints` key `{key}` is not valid on the root object")),
            "{key}: {message}"
        );
    }
}

// ── grouped `match(...)` ──────────────────────────────────────────────────

const GROUPED_UNION: &str = "  - doc: \"file(match('*.md'); match('guide*'))\"\n  - doc: file\n    other: string(required)\n";

#[test]
fn root_union_selection_requires_every_match_group() {
    let fixture = Fixture::new(&["guide-intro.md", "notes.md", "guide.txt"]);
    assert!(accepts(&fixture, GROUPED_UNION, "doc: guide-intro.md\n"), "both groups admit it");
    for (value, why) in [("notes.md", "only the first group"), ("guide.txt", "only the second group")] {
        let report = fixture.validate(&document(GROUPED_UNION, &format!("doc: {value}\n")));
        assert!(!report.valid, "{value} satisfies {why}");
        assert!(
            report.problems.iter().any(|problem| problem.message.contains("other")),
            "{value}: the grouped arm is ruled out, so the other arm's requirement is reported: {:?}",
            report.problems
        );
    }
    // Control: the other arm still applies when its own requirement is met.
    assert!(accepts(&fixture, GROUPED_UNION, "doc: notes.md\nother: x\n"));
}

#[test]
fn an_exclusion_applies_only_inside_its_own_group() {
    let fixture = Fixture::new(&["guide-a.md", "guide-b.md", "notes.md"]);
    let schema = "  - doc: \"file(match('*.md', '!notes*'); match('guide*', '!guide-b*'))\"\n  - doc: file\n    other: string(required)\n";
    assert!(accepts(&fixture, schema, "doc: guide-a.md\n"), "neither exclusion names it");
    assert!(!accepts(&fixture, schema, "doc: guide-b.md\n"), "the second group excludes it");
    assert!(!accepts(&fixture, schema, "doc: notes.md\n"), "the first group excludes it");
}

#[test]
fn match_groups_stay_passive_in_a_single_schema() {
    let fixture = Fixture::new(&["guide-intro.md", "notes.md", "guide.txt"]);
    let schema = "  doc: \"file(match('*.md'); match('guide*'))\"\n";
    for value in ["guide-intro.md", "notes.md", "guide.txt"] {
        assert!(accepts(&fixture, schema, &format!("doc: {value}\n")), "{value}");
    }
}

#[test]
fn several_groups_emit_nested_arrays_and_one_group_a_flat_array() {
    let schema = lower(
        "- doc: \"file(match('*.md'); match('guide*'))\"\n\
         - doc: \"file(match('*.md'))\"\n",
    )
    .unwrap();
    let keyword = |arm: usize| -> Value {
        let doc = &schema["anyOf"][arm]["properties"]["doc"];
        let file = doc["anyOf"]
            .as_array()
            .and_then(|arms| arms.iter().find(|arm| arm.get("format").is_some()))
            .expect("file fragment");
        file["x-darkmatter-match"].clone()
    };
    assert_eq!(keyword(0), json!([["*.md"], ["guide*"]]));
    assert_eq!(keyword(1), json!(["*.md"]));
}
