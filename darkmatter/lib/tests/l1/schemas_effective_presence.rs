//! One presence calculation across every surface: ordinary validation,
//! launch/completion phase validation, trigger matching, and composition
//! agree that `required` asks only whether the key was written. A
//! null-admitting type accepts a written `null` but never cancels
//! `required`, and a phase projection that relaxes presence keeps the
//! declared value type.

use std::path::Path;

use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::{ComposeContext, ComposeOptions};
use simplified_schema::triggers::{PathSubject, parse_trigger_envelope_from_str};
use darkmatter::markdown::schemas::{DarkmatterSchemas, EffectiveSchema};
use simplified_schema::SchemaPhase;
use serde_json::{Value, json};
use tempfile::TempDir;

fn effective(schema: &str) -> EffectiveSchema {
    let source: Markdown = format!("---\n$schema:\n{schema}---\nbody\n").into();
    DarkmatterSchemas::new(crate::request_support::cwd_context())
        .effective_for(&source)
        .expect("schema resolves")
        .expect("effective schema")
}

/// Validity of `instance` under ordinary validation, launch, and completion.
fn surfaces(schema: &EffectiveSchema, instance: &Value) -> [bool; 3] {
    let phase = |phase| {
        schema
            .validate_for_phase(instance, phase)
            .expect("phase schema builds")
            .valid
    };
    [schema.validate(instance).valid, phase(SchemaPhase::Launch), phase(SchemaPhase::Completion)]
}

const OPTIONAL_EVERYWHERE: [bool; 3] = [true, true, true];

#[test]
fn null_admitting_required_values_need_their_key_at_the_phases_that_demand_it() {
    // `[ordinary, launch, completion]` for an omitted key: launch demands only
    // an eager required property, completion every required one, and
    // ordinary validation leaves a `generated` one to its host.
    for (declaration, omitted) in [
        ("'null(required)'", [false, true, false]),
        ("'null(eager; required)'", [false, false, false]),
        ("'null(generated; required)'", [true, true, false]),
        ("'unknown(required)'", [false, true, false]),
        ("'any(eager; required)'", [false, false, false]),
        ("'unknown(generated; required)'", [true, true, false]),
        ("['string(required; min(3))', 'null']", [false, true, false]),
        ("['string(eager; required)', unknown]", [false, false, false]),
    ] {
        let schema = effective(&format!("  p: {declaration}\n"));
        assert_eq!(surfaces(&schema, &json!({})), omitted, "{declaration}: omitted");
        assert_eq!(surfaces(&schema, &json!({ "p": null })), OPTIONAL_EVERYWHERE, "{declaration}: null");
    }

    // The concrete arm still constrains a non-null value.
    let union = effective("  p: ['string(required; min(3))', 'null']\n");
    assert_eq!(surfaces(&union, &json!({ "p": "ab" })), [false; 3]);
    assert_eq!(surfaces(&union, &json!({ "p": "abc" })), OPTIONAL_EVERYWHERE);
}

#[test]
fn a_phase_that_relaxes_presence_keeps_the_declared_value_type() {
    // Launch leaves a deferred required number unlisted, but a supplied
    // `null` is still not a number, at the top level and inside an object.
    let schema = effective("  n: 'number(required)'\n  o: \"{ m: number(required) }\"\n");
    assert_eq!(surfaces(&schema, &json!({ "n": 1 })), OPTIONAL_EVERYWHERE);
    assert_eq!(surfaces(&schema, &json!({})), [false, true, false], "omitted");
    assert_eq!(surfaces(&schema, &json!({ "n": null })), [false; 3], "a null number");
    assert_eq!(surfaces(&schema, &json!({ "n": 1, "o": {} })), [false, true, false], "an omitted child");
    assert_eq!(surfaces(&schema, &json!({ "n": 1, "o": { "m": null } })), [false; 3], "a null child");
    // A generated required string keeps its type whatever the phase.
    let generated = effective("  g: 'string(generated; required)'\n");
    assert_eq!(surfaces(&generated, &json!({ "g": null })), [false; 3]);
}

#[test]
fn inherited_and_postfixed_required_survives_a_null_arm() {
    let schema = effective(
        "  person: \"{ name: string(required) }(required)\"\n  code: 'string(required; min(3))'\n  \
         choice: ['string(required)', 'null']\n  maybe_person: [person, null]\n  \
         maybe_code: [code, null]\n  postfixed: choice(eager)\n",
    );
    let base = json!({
        "person": { "name": "Ada" }, "code": "abc", "choice": null, "maybe_person": null, "maybe_code": null,
        "postfixed": null,
    });
    assert_eq!(surfaces(&schema, &base), OPTIONAL_EVERYWHERE);
    for (property, omitted) in
        [("maybe_person", [false, true, false]), ("maybe_code", [false, true, false]), ("postfixed", [false; 3])]
    {
        let mut without = base.clone();
        without.as_object_mut().expect("an object").remove(property);
        assert_eq!(surfaces(&schema, &without), omitted, "{property}: omitted");
    }
    let mut short = base.clone();
    short["maybe_code"] = json!("ab");
    assert!(!schema.validate(&short).valid, "the named arm keeps `min(3)`");
}

#[test]
fn nullable_items_and_children_leave_their_container_required() {
    for (declaration, valid) in [
        ("'null[](required)'", json!([null])),
        ("'unknown[](required)'", json!([1, "x", null, { "a": [] }])),
        ("'unknown[](eager; required)'", json!([])),
        ("'{ c: null }(required)'", json!({})),
    ] {
        let schema = effective(&format!("  p: {declaration}\n"));
        let eager = declaration.contains("eager");
        assert_eq!(surfaces(&schema, &json!({ "p": valid })), OPTIONAL_EVERYWHERE, "{declaration}");
        assert_eq!(surfaces(&schema, &json!({})), [false, !eager, false], "{declaration}: omitted");
        assert!(!schema.validate(&json!({ "p": null })).valid, "{declaration}: null");
    }
}

#[test]
fn ordinary_concrete_types_keep_their_presence_and_timing() {
    let schema = effective("  p: 'string(required)'\n");
    assert_eq!(surfaces(&schema, &json!({})), [false, true, false]);
    let schema = effective("  p: 'string(eager; required)'\n");
    assert_eq!(surfaces(&schema, &json!({})), [false; 3]);
    let schema = effective("  p: 'string(generated; required)'\n");
    assert_eq!(surfaces(&schema, &json!({})), [true, true, false]);
    let schema = effective("  p: string\n");
    assert_eq!(surfaces(&schema, &json!({})), OPTIONAL_EVERYWHERE);
}

#[test]
fn raw_json_schema_keeps_authored_required_for_nullable_properties() {
    let dir = TempDir::new().expect("tempdir");
    std::fs::write(
        dir.path().join("raw.json"),
        r#"{"$schema":"https://json-schema.org/draft/2020-12/schema","type":"object",
            "required":["anything","nothing"],
            "properties":{"anything":{},"nothing":{"type":"null"}}}"#,
    )
    .expect("write raw schema");
    let document = dir.path().join("document.md");
    std::fs::write(&document, "---\n$schema: ./raw.json\n---\nbody\n").expect("write document");
    let source = Markdown::try_from(document.as_path()).expect("read document");
    let schema = DarkmatterSchemas::new(crate::request_support::context_at(dir.path()))
        .effective_for(&source)
        .expect("schema resolves")
        .expect("effective schema");
    assert!(schema.simplified.is_none());
    assert_eq!(surfaces(&schema, &json!({ "anything": null })), [false; 3], "`nothing` omitted");
    assert_eq!(surfaces(&schema, &json!({ "anything": 1, "nothing": null })), OPTIONAL_EVERYWHERE);
}

#[test]
fn trigger_conditions_use_the_same_presence() {
    let arms = |condition: &str| {
        parse_trigger_envelope_from_str(&format!(
            "kind: schema-trigger\nmatch:\n  - all:\n      - kind: 'literal(t; required)'\n      - p: {condition}\n"
        ))
        .unwrap_or_else(|error| panic!("{condition}: {error}"))
        .expect("trigger envelope")
        .conditions
    };
    let selects = |condition: &str, frontmatter: Value| arms(condition).matches(&frontmatter, &PathSubject::detached());

    for condition in ["'unknown(required)'", "'null(required)'"] {
        assert!(!selects(condition, json!({ "kind": "t" })), "{condition}: omitted");
        assert!(selects(condition, json!({ "kind": "t", "p": null })), "{condition}: null");
        // Alone, the condition is a gate (the key must be written).
        let alone = parse_trigger_envelope_from_str(&format!(
            "kind: schema-trigger\nmatch:\n  - p: {condition}\n"
        ));
        assert!(alone.is_ok(), "{condition} alone is a gate");
    }
    // Without the marker the condition is a guard, vacuous alone.
    let vacuous = parse_trigger_envelope_from_str("kind: schema-trigger\nmatch:\n  - p: unknown\n");
    assert!(vacuous.is_err(), "an unmarked `unknown` alone is vacuous");
    assert!(!selects("'null(required)'", json!({ "kind": "t", "p": 1 })));

    let list = "'unknown[](required)'";
    assert!(selects(list, json!({ "kind": "t", "p": [] })));
    assert!(selects(list, json!({ "kind": "t", "p": [1, "x", null] })));
    for value in [Value::Null, json!("oops"), json!({ "a": 1 })] {
        assert!(!selects(list, json!({ "kind": "t", "p": value.clone() })), "{value}");
    }
    assert!(!selects(list, json!({ "kind": "t" })), "absent");
    assert!(
        parse_trigger_envelope_from_str(&format!("kind: schema-trigger\nmatch:\n  - p: {list}\n")).is_ok(),
        "a required list is a gate"
    );
}

fn compose(dir: &Path, content: &str) -> Result<Markdown, String> {
    let path = dir.join("doc.md");
    std::fs::write(&path, content).expect("write document");
    let options = ComposeOptions::new_with_context(ComposeContext::capture_for_content(dir, ""))
        .with_source_file(path.clone());
    Markdown::try_from(path.as_path())
        .expect("read document")
        .compose_with(&crate::request_support::request(options))
        .map(|(composed, _)| composed)
        .map_err(|error| error.to_string())
}

#[test]
fn composition_requires_written_keys_for_null_admitting_required_properties() {
    let dir = TempDir::new().expect("tempdir");
    let schema = "---\n$schema:\n  n: 'null(required)'\n  u: 'unknown(eager; required)'\n  \
                  d: ['string(required)', 'null']\n";
    let composed = compose(
        dir.path(),
        &format!("{schema}n: null\nu: null\nd: null\n---\n{{{{ is_null(n) }}}} {{{{ is_null(u) }}}} {{{{ is_null(d) }}}}\n"),
    )
    .expect("written nulls compose");
    let frontmatter = composed.frontmatter().as_map();
    for name in ["n", "u", "d"] {
        assert_eq!(frontmatter.get(name), Some(&Value::Null), "`{name}` stays a written null");
    }
    assert_eq!(composed.content().trim_end(), "true true true");
    let error = compose(dir.path(), &format!("{schema}n: null\nd: null\n---\nbody\n"))
        .expect_err("an omitted required key fails");
    assert!(error.contains("did not satisfy the schema"), "{error}");

    let list = "---\n$schema:\n  items: 'null[](required)'\n";
    compose(dir.path(), &format!("{list}items: [null]\n---\nbody\n")).expect("a list of nulls composes");
    let error = compose(dir.path(), &format!("{list}---\nbody\n"))
        .expect_err("a required list of nulls is still required");
    assert!(error.contains("did not satisfy the schema"), "{error}");
}
