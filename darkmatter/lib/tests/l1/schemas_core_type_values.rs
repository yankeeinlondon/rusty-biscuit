//! Value semantics of the core schema types (acceptance criteria 5, 7, 13,
//! and 14 of `2026-10-16-core-schema-types`): what `alphanumeric`, `shell`,
//! `unknown`, and `null` accept through Darkmatter's validator, how coercion
//! treats them, which defaults and suggestions load, and that the exported
//! JSON Schema agrees with the typed model.

use darkmatter::markdown::Markdown;
use darkmatter::markdown::schemas::coerce::coerce_frontmatter;
use darkmatter::markdown::schemas::simplified::grammar::parse_type_expr;
use darkmatter::markdown::schemas::{
    AlphanumericOption, AlphanumericOptions, DarkmatterSchemas, EffectiveSchema, SchemaError,
    parse_yaml_schema, to_json_schema,
};
use serde_json::{Value, json};

fn effective(schema: &str) -> Result<EffectiveSchema, SchemaError> {
    load(schema)
}

fn load(schema: &str) -> Result<EffectiveSchema, SchemaError> {
    let source: Markdown = format!("---\n$schema:\n{schema}---\nbody\n").into();
    DarkmatterSchemas::new(crate::request_support::cwd_context())
        .effective_for(&source)
        .map(|schema| schema.expect("effective schema"))
}

/// The load error of one property `p` declared as `definition`.
#[track_caller]
fn load_error(definition: &str) -> String {
    match load(&format!("  p: \"{definition}\"\n")) {
        Ok(_) => panic!("{definition} loads, but must fail"),
        Err(error) => error.to_string(),
    }
}

/// The effective schema of one property `p` declared as `definition`.
fn property(definition: &str) -> EffectiveSchema {
    effective(&format!("  p: \"{definition}\"\n")).unwrap_or_else(|error| panic!("{definition}: {error}"))
}

fn accepts(schema: &EffectiveSchema, value: Value) -> bool {
    schema.validate(&json!({ "p": value })).valid
}

fn json_schema(body: &str) -> Value {
    let yaml: serde_yaml_ng::Value = serde_yaml_ng::from_str(body).expect("schema YAML");
    to_json_schema(&parse_yaml_schema(&yaml).expect("schema parses")).expect("schema lowers")
}

fn compose_value(document: &str, key: &str) -> Result<Value, String> {
    let markdown: Markdown = document.into();
    let options = darkmatter::markdown::compose::ComposeOptions::new();
    markdown
        .compose_with(&crate::request_support::request(options))
        .map(|(composed, _)| composed.frontmatter().as_map().get(key).cloned().unwrap_or(Value::Null))
        .map_err(|error| error.to_string())
}

// ── alphanumeric ─────────────────────────────────────────────────────────

#[test]
fn each_alphanumeric_option_has_a_positive_and_a_negative_value() {
    for (options, accepted, rejected) in [
        ("", "abc1", "a-b"),
        ("allow-dashes", "a-b", "a_b"),
        ("allow-underscores", "a_b", "a-b"),
        ("allow-slashes", "a/b", "a\\b"),
        ("allow-periods", "a.b", "a/b"),
        ("allow-whitespace", "a b", "a\tb"),
        ("allow-periods, block-double-periods", "a.b", "a..b"),
        ("allow-dashes, block-leading-dash", "a-", "-a"),
        ("allow-slashes, block-leading-slash", "a/", "/a"),
        ("leading-alpha", "abc", "1abc"),
    ] {
        let definition = if options.is_empty() {
            "alphanumeric(required)".to_string()
        } else {
            format!("alphanumeric({options}; required)")
        };
        let schema = property(&definition);
        assert!(accepts(&schema, json!(accepted)), "{definition} accepts {accepted:?}");
        assert!(!accepts(&schema, json!(rejected)), "{definition} rejects {rejected:?}");
    }
}

#[test]
fn alphanumeric_edges_and_overlapping_constraints() {
    let base = property("alphanumeric(required)");
    assert!(accepts(&base, json!("")), "the empty string without constraints");
    for rejected in ["é", "abc\n", "\nabc", "a\tb", "a\u{a0}b", "ａ", "a b"] {
        assert!(!accepts(&base, json!(rejected)), "{rejected:?}");
    }
    let spaced = property("alphanumeric(allow-whitespace; required)");
    assert!(accepts(&spaced, json!(" ")));
    for rejected in ["a\u{3000}b", "a\nb", "a\u{a0}b"] {
        assert!(!accepts(&spaced, json!(rejected)), "only U+0020: {rejected:?}");
    }
    for definition in ["alphanumeric(min(1); required)", "alphanumeric(not-empty; required)"] {
        assert!(!accepts(&property(definition), json!("")), "{definition}");
    }

    let overlapping =
        property("alphanumeric(allow-periods, block-double-periods; pattern('^a'); min(1); max(5); required)");
    assert!(accepts(&overlapping, json!("a.b")));
    for rejected in ["", "b.a", "a..b", "a.b.c.d", "a.b\n"] {
        assert!(!accepts(&overlapping, json!(rejected)), "every pattern applies: {rejected:?}");
    }
}

#[test]
fn optional_alphanumeric_accepts_null_and_required_rejects_it() {
    assert!(accepts(&property("alphanumeric"), Value::Null));
    assert!(property("alphanumeric").validate(&json!({})).valid);
    assert!(!accepts(&property("alphanumeric(required)"), Value::Null));
}

#[test]
fn exported_schema_and_typed_options_agree() {
    let option_sets = [
        AlphanumericOptions::base(),
        AlphanumericOptions::base().with(AlphanumericOption::AllowWhitespace),
        AlphanumericOptions::base()
            .with(AlphanumericOption::AllowPeriods)
            .with(AlphanumericOption::BlockDoublePeriods),
        AlphanumericOptions::base()
            .with(AlphanumericOption::AllowSlashes)
            .with(AlphanumericOption::AllowPeriods)
            .with(AlphanumericOption::BlockDoublePeriods)
            .with(AlphanumericOption::BlockLeadingSlash),
        AlphanumericOptions::base()
            .with(AlphanumericOption::LeadingAlpha)
            .with(AlphanumericOption::AllowDashes)
            .with(AlphanumericOption::AllowUnderscores),
    ];
    let values = [
        "", "a", "abc1", "a.b", "a..b", ".", "..", ".a", "a/b", "/a", "./a", "a/../b", "a b", " ",
        "a\tb", "a\n", "\na", "-a", "a-", "_a", "1a", "é", "a\u{3000}b",
    ];
    for options in option_sets {
        let keywords: Vec<_> = options.keywords().collect();
        let definition = if keywords.is_empty() {
            "alphanumeric(required)".to_string()
        } else {
            format!("alphanumeric({}; required)", keywords.join(", "))
        };
        let exported = json_schema(&format!("p: \"{definition}\"\n"));
        assert_eq!(exported["properties"]["p"]["x-darkmatter-type"], json!("alphanumeric"));
        assert_eq!(exported["properties"]["p"]["x-darkmatter-options"], json!(keywords));
        // A plain validator with no Darkmatter registrations reads the export.
        let plain = jsonschema::validator_for(&exported).expect("the exported regex compiles");
        let schema = property(&definition);
        for value in values {
            let typed = options.accepts(value);
            assert_eq!(plain.is_valid(&json!({ "p": value })), typed, "{definition}: exported {value:?}");
            assert_eq!(accepts(&schema, json!(value)), typed, "{definition}: Darkmatter {value:?}");
        }
    }
}

#[test]
fn exported_alphanumeric_patterns_preserve_interacting_restrictions() {
    use AlphanumericOption::*;

    let allowed = [AllowDashes, AllowUnderscores, AllowSlashes, AllowPeriods, AllowWhitespace];
    let restrictions = [BlockDoublePeriods, BlockLeadingDash, BlockLeadingSlash, LeadingAlpha];
    let alphabet = ['a', '1', '-', '/', '.', ' ', '\n', 'é'];
    let mut samples = vec![String::new()];
    let mut previous = vec![String::new()];
    for _ in 0..3 {
        let next: Vec<String> = previous
            .iter()
            .flat_map(|prefix| alphabet.map(|ch| format!("{prefix}{ch}")))
            .collect();
        samples.extend(next.iter().cloned());
        previous = next;
    }

    for mask in 0..(1 << restrictions.len()) {
        let mut options: AlphanumericOptions = allowed.into_iter().collect();
        for (index, restriction) in restrictions.into_iter().enumerate() {
            if mask & (1 << index) != 0 {
                options = options.with(restriction);
            }
        }
        let keywords = options.canonical().keywords().collect::<Vec<_>>().join(", ");
        let exported = json_schema(&format!("p: 'alphanumeric({keywords}; required)'\n"));
        let validator = jsonschema::validator_for(&exported).expect("exported pattern compiles");
        for sample in &samples {
            let expected = sample.chars().all(|ch| ch.is_ascii_alphanumeric() || "-_/ .".contains(ch))
                && (!options.contains(BlockDoublePeriods) || !sample.contains(".."))
                && sample.chars().next().is_none_or(|first| {
                    (!options.contains(LeadingAlpha) || first.is_ascii_alphabetic())
                        && (!options.contains(BlockLeadingDash) || first != '-')
                        && (!options.contains(BlockLeadingSlash) || first != '/')
                });
            assert_eq!(options.accepts(sample), expected, "typed {keywords}: {sample:?}");
            assert_eq!(
                validator.is_valid(&json!({ "p": sample })),
                expected,
                "exported {keywords}: {sample:?}"
            );
        }
    }
}

#[test]
fn alphanumeric_never_coerces_numbers_or_booleans() {
    let schema = json_schema(
        "scalar: alphanumeric\nitems: alphanumeric[]\nflag: alphanumeric\n\
         either: [alphanumeric, string]\nchoice: [alphanumeric, number]\n",
    );
    let instance = json!({ "scalar": 3, "items": ["a", 12], "flag": true, "either": 3, "choice": 7 });
    let coerced = coerce_frontmatter(&schema, &instance).value;
    assert_eq!(coerced["scalar"], json!(3));
    assert_eq!(coerced["items"], json!(["a", 12]));
    assert_eq!(coerced["flag"], json!(true));
    // Other arms keep their own conversions: only `string` turns `3` into
    // text, so it is the one reachable option.
    assert_eq!(coerced["either"], json!("3"));
    assert_eq!(coerced["choice"], json!(7));

    let schema = property("alphanumeric(required)");
    for value in [json!(3), json!(true), json!(["a"]), json!({ "a": "b" })] {
        assert!(!accepts(&schema, value.clone()), "{value} is a type mismatch");
    }
    let error = compose_value("---\n$schema:\n  p: alphanumeric\np: 3\n---\nbody\n", "p")
        .expect_err("composition does not coerce `3` to text");
    assert!(error.contains("did not satisfy the schema"), "{error}");
}

#[test]
fn alphanumeric_defaults_and_suggestions_must_be_valid_values() {
    for definition in [
        "alphanumeric(allow-dashes; default(a-b))",
        "alphanumeric(default(''))",
        "alphanumeric(min(2); suggest(ab, abc); default(abc))",
        "alphanumeric(allow-dashes)[](default([]))",
    ] {
        effective(&format!("  p: \"{definition}\"\n")).unwrap_or_else(|error| panic!("{definition}: {error}"));
    }
    for (definition, value) in [
        ("alphanumeric(default(a-b))", "a-b"),
        ("alphanumeric(min(3); default(ab))", "ab"),
        ("alphanumeric(default(12))", "12"),
        ("alphanumeric(default(true))", "true"),
        ("alphanumeric(suggest(ab, a-b))", "a-b"),
        ("alphanumeric(pattern('^a'); suggest(ab, ba))", "ba"),
        ("alphanumeric(leading-alpha; suggest(1a))", "1a"),
    ] {
        let error = load_error(definition);
        assert!(error.contains(value) && error.contains("alphanumeric"), "{definition}: {error}");
    }
    // Other types' suggestions stay advisory.
    effective("  p: \"string(min(3); suggest(ab))\"\n").expect("string suggestions are only linted");
}

// ── shell ────────────────────────────────────────────────────────────────

#[test]
fn shell_accepts_any_non_blank_command_without_parsing_it() {
    let schema = property("shell(required)");
    for accepted in ["echo ok", "ls | wc -l; echo done", "x", "$(echo hi)", "{{ command }}", "unterminated '"] {
        assert!(accepts(&schema, json!(accepted)), "{accepted:?}");
    }
    for rejected in ["", " ", "\t\n", "\u{3000}", "\u{a0}"] {
        assert!(!accepts(&schema, json!(rejected)), "blank {rejected:?}");
    }
    for mismatch in [json!({ "a": 1 }), json!(["echo"]), Value::Null] {
        assert!(!accepts(&schema, mismatch.clone()), "{mismatch}");
    }
    assert!(accepts(&property("shell"), Value::Null), "optional shell accepts null");

    let exported = json_schema("p: shell\n");
    assert_eq!(exported["properties"]["p"]["anyOf"][1], json!({ "type": "string", "format": "darkmatter-shell" }));
}

#[test]
fn shell_scalars_coerce_to_text_like_expression() {
    for (raw, text) in [("true", "true"), ("3", "3"), ("false", "false")] {
        let value = compose_value(&format!("---\n$schema:\n  cmd: shell\ncmd: {raw}\n---\nbody\n"), "cmd")
            .unwrap_or_else(|error| panic!("{raw}: {error}"));
        assert_eq!(value, json!(text), "{raw}");
    }
    let coerced = coerce_frontmatter(&json_schema("cmds: shell[]\n"), &json!({ "cmds": [true, 2, "ls"] })).value;
    assert_eq!(coerced["cmds"], json!(["true", "2", "ls"]));
}

#[test]
fn shell_constraints_and_defaults_mirror_expression() {
    for definition in [
        "shell(required; eager)",
        "shell(generated)",
        "shell(default('echo hi'))",
        "shell(default(true))",
        "shell(default(3))",
        "shell[](min(1); required)",
    ] {
        effective(&format!("  p: \"{definition}\"\n")).unwrap_or_else(|error| panic!("{definition}: {error}"));
    }
    for (definition, expected) in [
        ("shell(min(1))", "not valid on `shell`"),
        ("shell(not-empty)", "not valid on `shell`"),
        ("shell(pattern('^a'))", "not valid on `shell`"),
        ("shell(suggest(ls))", "shell"),
        ("shell(default(''))", "non-whitespace"),
        ("shell(default('  '))", "non-whitespace"),
    ] {
        let error = load_error(definition);
        assert!(error.contains(expected), "{definition}: {error}");
    }
}

// ── unknown and null ─────────────────────────────────────────────────────

#[test]
fn unknown_values_keep_their_native_types() {
    let schema = json_schema("a: unknown\nb: any\nc: unknown(required)\nd: unknown[]\n");
    let instance = json!({
        "a": 3,
        "b": true,
        "c": { "nested": [1, null, "x", { "deep": null }] },
        "d": [1, "1", true, null, []],
    });
    let outcome = coerce_frontmatter(&schema, &instance);
    assert!(!outcome.changed, "no string coercion: {}", outcome.value);
    assert_eq!(outcome.value, instance);
    assert_eq!(
        compose_value("---\n$schema:\n  p: unknown\np:\n  n: 1\n  z: null\n---\nbody\n", "p").expect("composes"),
        json!({ "n": 1, "z": null })
    );
}

#[test]
fn null_is_never_coerced() {
    let schema = json_schema("p: \"null\"\n");
    for value in [json!("null"), json!(""), json!(0), json!(false)] {
        let outcome = coerce_frontmatter(&schema, &json!({ "p": value.clone() }));
        assert!(!outcome.changed, "{value}");
        assert!(!accepts(&property("null"), value.clone()), "{value}");
    }
}

#[test]
fn subtype_relation_is_reachable_through_the_public_schema_api() {
    use darkmatter::markdown::schemas::is_subtype;
    let atom = |text: &str| parse_type_expr("p", text).expect("parses");
    let rule = atom("alphanumeric(allow-slashes)");
    assert!(is_subtype(&atom("alphanumeric(allow-slashes)"), &rule));
    assert!(is_subtype(&atom("alphanumeric(min(2))"), &rule));
    assert!(!is_subtype(&atom("string"), &rule));
    assert!(!is_subtype(&atom("email"), &rule));
    assert!(is_subtype(&atom("alphanumeric(block-leading-dash)"), &atom("alphanumeric")));
    assert!(is_subtype(&atom("alphanumeric"), &atom("alphanumeric(block-leading-dash)")));
}

/// The shared structural relation accepts a narrowed type where its family,
/// enum, or union admits it (`2026-10-16-type-narrowing` acceptance 5).
#[test]
fn narrowed_literals_and_unions_satisfy_their_parameter_types_structurally() {
    use darkmatter::markdown::schemas::{PropertyDef, SimplifiedSchema, is_definition_subtype, is_subtype};
    let atom = |text: &str| parse_type_expr("p", text).expect("parses");
    let declared = |body: &str| -> PropertyDef {
        let yaml: serde_yaml_ng::Value = serde_yaml_ng::from_str(body).expect("schema YAML");
        match parse_yaml_schema(&yaml).expect("schema parses") {
            SimplifiedSchema::Single(shape) => shape.properties["p"].clone(),
            SimplifiedSchema::Union(_) => panic!("a single shape"),
        }
    };

    assert!(is_subtype(&atom("literal('release')"), &atom("string")));
    assert!(is_subtype(&atom("literal(4)"), &atom("number")));
    assert!(is_subtype(&atom("literal(true)"), &atom("boolean")));
    let literals = declared("p: [\"literal('a')\", \"literal('b')\"]\n");
    assert!(is_definition_subtype(&literals, &PropertyDef::Single(atom("enum(a, b, c)"))));

    // Controls: the relation stays structural and bounded.
    assert!(!is_subtype(&atom("literal(4)"), &atom("enum('4')")));
    assert!(!is_definition_subtype(&declared("p: [\"literal('a')\", \"literal('z')\"]\n"), &PropertyDef::Single(atom("enum(a, b, c)"))));
    let scalars = declared("p: [string, number]\n");
    assert!(is_definition_subtype(&scalars, &declared("p: [boolean, number, string]\n")));
    assert!(!is_definition_subtype(&declared("p: [boolean, number, string]\n"), &scalars));
}
