//! Frontmatter conversion through the one conversion engine
//! (`2026-10-28-schema-enhancements`, criteria 28–34, 47, 59): each case runs
//! through the public validation API and through library composition of a
//! fixture document, and asserts the composed frontmatter.

use std::path::{Path, PathBuf};

use darkmatter::markdown::Markdown;
use darkmatter::markdown::coercion::{Shape, accepts_exactly, bind};
use darkmatter::markdown::compose::ComposeOptions;
use darkmatter::markdown::schemas::{DarkmatterSchemas, ValidationReport};
use darkmatter::markdown::MarkdownError;
use serde_json::{Value, json};

use crate::request_support::{context_at, request};

struct Document {
    _dir: tempfile::TempDir,
    path: PathBuf,
}

impl Document {
    /// Writes `frontmatter` (YAML lines without fences) as `doc.md`.
    fn new(frontmatter: &str) -> Self {
        let dir = tempfile::tempdir().expect("temp dir");
        let root = biscuit_file::canonicalize_simplified(dir.path()).expect("canonical temp dir");
        let path = root.join("doc.md");
        std::fs::write(&path, format!("---\n{frontmatter}---\nBody\n")).expect("write document");
        Self { _dir: dir, path }
    }

    fn root(&self) -> &Path {
        self.path.parent().expect("document directory")
    }

    fn markdown(&self) -> Markdown {
        Markdown::try_from(self.path.as_path()).expect("document loads")
    }

    fn validate(&self) -> ValidationReport {
        DarkmatterSchemas::new(context_at(self.root()))
            .validate(&self.markdown())
            .expect("the schema loads")
    }

    /// The composed frontmatter, or the composition error's text with every
    /// problem message.
    fn compose(&self) -> Result<Value, String> {
        compose_markdown(self.markdown(), &self.path)
    }
}

fn compose_markdown(markdown: Markdown, path: &Path) -> Result<Value, String> {
    match markdown.compose_with(&request(ComposeOptions::new().with_source_file(path))) {
        Ok((composed, _)) => Ok(serde_json::to_value(composed.frontmatter().as_map()).expect("frontmatter JSON")),
        Err(MarkdownError::SchemaValidationFailed { problems, summary, .. }) => Err(format!(
            "{summary}: {}",
            problems.iter().map(|problem| format!("{} {}", problem.path, problem.message)).collect::<Vec<_>>().join("; ")
        )),
        Err(other) => Err(other.to_string()),
    }
}

/// Validates and composes `frontmatter`, returning the composed value of
/// `key` after asserting validation passed.
fn composed(frontmatter: &str, key: &str) -> Value {
    let document = Document::new(frontmatter);
    let report = document.validate();
    assert!(report.valid, "{frontmatter}: {:?}", report.problems);
    let composed = document.compose().unwrap_or_else(|error| panic!("{frontmatter}: {error}"));
    composed[key].clone()
}

/// Validates and composes `frontmatter`, asserting both fail and the
/// composition error names `reason`.
fn rejected(frontmatter: &str, reason: &str) {
    let document = Document::new(frontmatter);
    let report = document.validate();
    assert!(!report.valid, "{frontmatter}: expected a validation failure");
    let messages: Vec<&str> = report.problems.iter().map(|problem| problem.message.as_str()).collect();
    assert!(messages.iter().any(|message| message.contains(reason)), "{frontmatter}: {messages:?}");
    let error = document.compose().expect_err("composition must fail");
    assert!(error.contains(reason), "{frontmatter}: {error}");
}

#[test]
fn number_version_composes_as_text() {
    assert_eq!(composed("$schema:\n  version: string\nversion: 1.2\n", "version"), json!("1.2"));
}

#[test]
fn yes_and_on_become_true() {
    for word in ["\"yes\"", "\"On\""] {
        assert_eq!(composed(&format!("$schema:\n  draft: boolean\ndraft: {word}\n"), "draft"), json!(true));
    }
}

#[test]
fn plus_and_underscore_numbers_convert() {
    assert_eq!(composed("$schema:\n  n: number\nn: \"+4\"\n", "n"), json!(4));
    assert_eq!(composed("$schema:\n  n: number\nn: \"1_000\"\n", "n"), json!(1000));
}

#[test]
fn unsafe_integers_fail_validation() {
    rejected("$schema:\n  n: number\nn: \"99999999999999999999\"\n", "cannot be represented exactly");
    // A native YAML integer no `f64` holds is a located load error.
    let error = Markdown::try_from_content("---\n$schema:\n  n: number\nn: 9007199254740993\n---\nBody\n".to_string())
        .expect_err("an inexact native integer must not load");
    assert!(error.to_string().contains("cannot be represented exactly"), "{error}");
}

#[test]
fn root_union_result_is_order_independent() {
    let arms = ["  - kind: literal(a)\n    x: number\n", "  - kind: literal(a)\n    x: boolean\n"];
    for order in [[0, 1], [1, 0]] {
        let schema = format!("$schema:\n{}{}", arms[order[0]], arms[order[1]]);
        let frontmatter = format!("{schema}kind: a\nx: \"1\"\n");
        assert_eq!(composed(&frontmatter, "x"), json!(1), "{schema}");
    }
}

#[test]
fn nested_object_count_converts() {
    let frontmatter = "$schema:\n  meta: \"{ inner: { count: number } }\"\nmeta:\n  inner:\n    count: \"3\"\n";
    assert_eq!(composed(frontmatter, "meta"), json!({ "inner": { "count": 3 } }));
}

#[test]
fn json_property_serializes_mapping() {
    let frontmatter = "$schema:\n  config: json\nconfig:\n  port: 8080\n  name: api\n";
    assert_eq!(composed(frontmatter, "config"), json!("{\"port\":8080,\"name\":\"api\"}"));
}

#[test]
fn json_or_yaml_keeps_written_key_order() {
    for schema in ["json | yaml", "yaml | json"] {
        let frontmatter = format!("$schema:\n  config: \"{schema}\"\nconfig:\n  port: 8080\n  debug: true\n");
        assert_eq!(composed(&frontmatter, "config"), json!("{\"port\":8080,\"debug\":true}"), "{schema}");
        let text = format!("$schema:\n  config: \"{schema}\"\nconfig: \"port: 8080\"\n");
        assert_eq!(composed(&text, "config"), json!("port: 8080"), "{schema}");
    }
}

#[test]
fn null_is_not_serialized() {
    assert_eq!(composed("$schema:\n  config: json\nconfig: null\n", "config"), Value::Null);
    rejected("$schema:\n  config: json(required)\nconfig: null\n", "null");
}

#[test]
fn json_text_with_duplicate_keys_is_rejected() {
    rejected("$schema:\n  config: json\nconfig: '{\"a\": 1, \"a\": 2}'\n", "darkmatter-json");
}

#[test]
fn enum_number_composes_as_text() {
    assert_eq!(composed("$schema:\n  n: \"enum(1, 2)\"\nn: 2\n", "n"), json!("2"));
}

#[test]
fn pattern_key_dictionary_converts() {
    let frontmatter = "$schema:\n  scores: \"{ <string>: number }\"\nscores:\n  a: \"3\"\n  b: 4\n";
    assert_eq!(composed(frontmatter, "scores"), json!({ "a": 3, "b": 4 }));
}

#[test]
fn nested_union_numberlike_and_patterns() {
    let frontmatter = concat!(
        "$schema:\n",
        "  meta: \"{ pick: number | boolean, zip: numberlike, flags: { <starting::f_>: boolean } }\"\n",
        "meta:\n  pick: \"1\"\n  zip: \"02134\"\n  flags:\n    f_a: \"yes\"\n    f_b: \"0\"\n",
    );
    assert_eq!(
        composed(frontmatter, "meta"),
        json!({ "pick": 1, "zip": "02134", "flags": { "f_a": true, "f_b": false } })
    );
}

/// The compiled `numberlike`/`boolish` formats and the conversion engine
/// accept exactly the same text.
#[test]
fn numberlike_boolish_formats_agree_with_engine_on_probe_list() {
    let document = Document::new("$schema:\n  n: numberlike\n  b: boolish\n");
    let effective = DarkmatterSchemas::new(context_at(document.root()))
        .effective_for(&document.markdown())
        .expect("the schema loads")
        .expect("a schema applies");
    let properties = &effective.json_schema["properties"];
    let typed = |name: &str| {
        properties[name]["anyOf"]
            .as_array()
            .and_then(|arms| arms.iter().find(|arm| arm.get("format").is_some()))
            .cloned()
            .unwrap_or_else(|| properties[name].clone())
    };
    assert_eq!(typed("n")["format"], json!("darkmatter-numberlike"));
    assert_eq!(typed("b")["format"], json!("darkmatter-boolish"));

    let probes = [
        "+4", "4", " 4 ", "-3.5", "1e3", ".5", "1_000", "02134", "yes", "On", "no", "OFF", "true", "FALSE", "1", "0",
        "pear", "", "1,000", "0x10", "NaN", "inf", "99999999999999999999", "9007199254740993", "9007199254740992",
        "1e400", "1e-400", "maybe", " true ",
    ];
    let mut disagreements = Vec::new();
    for (property, shape) in [("n", Shape::Number(darkmatter::markdown::coercion::NumberShape {
        as_sent: true,
        ..Default::default()
    })), ("b", Shape::Boolean { as_sent: true })] {
        for probe in probes {
            let value = json!(probe);
            let validator_accepts = effective.validate(&json!({ property: probe })).valid;
            let engine_accepts = accepts_exactly(&shape, &value);
            let kept = bind(&shape, &value).is_ok_and(|bound| bound.is_kept());
            if validator_accepts != engine_accepts || engine_accepts != kept {
                disagreements.push(format!(
                    "{property}={probe:?}: validator {validator_accepts}, engine {engine_accepts}, kept {kept}"
                ));
            }
        }
    }
    assert!(disagreements.is_empty(), "{}", disagreements.join("\n"));
}

#[test]
fn preservation_types_compose_unchanged() {
    let frontmatter = "$schema:\n  zip: numberlike\n  flag: boolish\nzip: \"02134\"\nflag: \"On\"\n";
    assert_eq!(composed(frontmatter, "zip"), json!("02134"));
    assert_eq!(composed(frontmatter, "flag"), json!("On"));
    rejected("$schema:\n  zip: numberlike\nzip: pear\n", "darkmatter-numberlike");
}

#[test]
fn list_union_converts_element_by_element() {
    let frontmatter = "$schema:\n  v: \"(boolean | number)[]\"\nv: [\"2\", \"yes\"]\n";
    assert_eq!(composed(frontmatter, "v"), json!([2, true]));
}

#[test]
fn whole_number_admission_every_spelling() {
    for spelling in ["9007199254740993.0", "9.007199254740993e15", "-9007199254740993"] {
        rejected(&format!("$schema:\n  n: number\nn: \"{spelling}\"\n"), "cannot be represented exactly");
    }
    assert_eq!(composed("$schema:\n  n: number\nn: \"-9007199254740992\"\n", "n"), json!(-9007199254740992_i64));
    let error = Markdown::try_from_content("---\n$schema:\n  n: number\nn: 9007199254740993.0\n---\nBody\n".to_string())
        .expect_err("an inexact native number must not load");
    assert!(error.to_string().contains("cannot be represented exactly"), "{error}");
}

/// A failed value commits nothing: the composition error leaves the source
/// file as written, and validation reports only the value that fails.
#[test]
fn a_failed_value_commits_nothing() {
    let frontmatter = "$schema:\n  n: number\n  flag: boolean\nn: \"3\"\nflag: maybe\n";
    let document = Document::new(frontmatter);
    let before = std::fs::read_to_string(&document.path).expect("read");
    let report = document.validate();
    let paths: Vec<&str> = report.problems.iter().map(|problem| problem.path.as_str()).collect();
    assert_eq!(paths, ["/flag"], "{:?}", report.problems);
    assert!(document.compose().is_err());
    assert_eq!(std::fs::read_to_string(&document.path).expect("read"), before);
}

/// Composition writes converted values back, and the written document reads,
/// writes, and reads again with the same converted values.
#[test]
fn converted_values_survive_a_read_write_read_round_trip() {
    let frontmatter = concat!(
        "$schema:\n  n: number\n  flag: boolean\n  zip: numberlike\n  config: json\n  version: string\n",
        "n: \"1_000\"\nflag: \"On\"\nzip: \"02134\"\nconfig:\n  b: 1\n  a: [true]\nversion: 1.2\n",
    );
    let expected = json!({
        "n": 1000,
        "flag": true,
        "zip": "02134",
        "config": "{\"b\":1,\"a\":[true]}",
        "version": "1.2",
    });
    let document = Document::new(frontmatter);
    let markdown = document.markdown();
    let (composed, _) = markdown
        .compose_with(&request(ComposeOptions::new().with_source_file(&document.path)))
        .expect("composition succeeds");
    let mut current = composed;
    for round in 0..2 {
        let path = document.root().join(format!("round-{round}.md"));
        std::fs::write(&path, current.as_string()).expect("write");
        let reread = Markdown::try_from(path.as_path()).expect("read");
        let values = serde_json::to_value(reread.frontmatter().as_map()).expect("frontmatter JSON");
        for (key, value) in expected.as_object().expect("object") {
            assert_eq!(&values[key], value, "round {round}: {key}");
        }
        let (again, _) = reread
            .compose_with(&request(ComposeOptions::new().with_source_file(&path)))
            .expect("recomposition succeeds");
        current = again;
    }
}

/// `number(integer)` means a whole number below 2^64 in magnitude, in the
/// compiled schema as in the engine; raw JSON Schema `integer` has no ceiling.
#[test]
fn integer_ceiling_applies_to_frontmatter() {
    assert_eq!(composed("$schema:\n  n: \"number(integer)\"\nn: \"18446744073709549568\"\n", "n"), json!(18446744073709549568u64));
    rejected("$schema:\n  n: \"number(integer)\"\nn: \"1e20\"\n", "2^64");
    rejected("$schema:\n  n: \"number(integer)\"\nn: 1.0e20\n", "2^64");
}

/// Each top-level property is its own value: a missing required sibling is
/// the validator's verdict and never blocks a present property's conversion.
#[test]
fn a_missing_required_sibling_does_not_block_conversion() {
    let schema = darkmatter::markdown::schemas::simplified::to_json_schema(
        &darkmatter::markdown::schemas::simplified::parse_yaml_schema(
            &serde_yaml_ng::from_str("enabled: boolean\ncount: \"number(required)\"\n").expect("schema YAML"),
        )
        .expect("schema parses"),
    )
    .expect("schema lowers");
    let outcome = darkmatter::markdown::schemas::coerce::coerce_frontmatter(&schema, &json!({ "enabled": "true" }));
    assert_eq!(outcome.value, json!({ "enabled": true }));
    rejected("$schema:\n  enabled: boolean\n  count: \"number(required)\"\nenabled: \"true\"\n", "count");
}
