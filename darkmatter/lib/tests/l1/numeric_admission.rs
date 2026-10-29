//! Darkmatter's one numeric model through its public surfaces: every number
//! is a finite `f64`, admitted from its written spelling where it enters
//! (frontmatter, raw schemas, expression literals, numeric text), computed
//! without integer casts, and rendered without a decimal suffix on a whole
//! value.

use darkmatter::markdown::{
    Markdown, MarkdownError,
    compose::ComposeOptions,
    schemas::{DarkmatterSchemas, SchemaError},
};
use serde_json::json;

const INEXACT: &str = "cannot be represented exactly";

fn compose(content: &str) -> Result<Markdown, MarkdownError> {
    let markdown = Markdown::try_from_content(content)?;
    markdown
        .compose_with(&crate::request_support::request(ComposeOptions::new()))
        .map(|(composed, _)| composed)
}

fn composed_body(content: &str) -> String {
    compose(content)
        .unwrap_or_else(|error| panic!("{content}: {error}"))
        .content()
        .trim_end()
        .to_string()
}

fn compose_error(content: &str) -> String {
    match compose(content) {
        Ok(composed) => panic!("{content} composed: {}", composed.content()),
        Err(error) => error.to_string(),
    }
}

/// The exact decimal digits of `f64::MAX`, a whole number an `f64` holds; the
/// shorter `1e308` names a whole number no `f64` holds.
fn largest_finite_spelling() -> String {
    format!("{:.0}", f64::MAX)
}

// ── Expression literals ───────────────────────────────────────────────────

#[test]
fn an_inexact_expression_literal_is_rejected_at_the_literal() {
    let error = compose_error("Value: {{ 9007199254740993 }}\n");
    assert!(error.contains(INEXACT), "{error}");
    assert!(error.contains("9007199254740993"), "{error}");

    let error = darkmatter::markdown::compose::expression::parse("n + 9007199254740993").unwrap_err();
    assert!(error.to_string().contains(INEXACT), "{error}");
    assert_eq!(error.position, 4, "{error}");
}

#[test]
fn an_exact_expression_literal_evaluates_exactly() {
    assert_eq!(composed_body("{{ 9007199254740992 }}|{{ -9007199254740992 }}\n"), "9007199254740992|-9007199254740992");
}

// ── Frontmatter numbers ───────────────────────────────────────────────────

#[test]
fn native_frontmatter_numbers_no_f64_holds_fail_to_load_with_location() {
    for (value, reason) in [
        ("9007199254740993", INEXACT),
        ("-9007199254740993", INEXACT),
        ("9007199254740993.0", INEXACT),
        ("9.007199254740993e15", INEXACT),
        ("99999999999999999999", INEXACT),
        ("1e308", INEXACT),
        ("1e-400", INEXACT),
        (".inf", "not finite"),
        ("-.inf", "not finite"),
        (".nan", "not finite"),
    ] {
        let content = format!("---\ntitle: Numbers\nn: {value}\n---\nBody\n");
        let error = Markdown::try_from_content(content).expect_err(value);
        let MarkdownError::FrontmatterParse { source, .. } = &error else {
            panic!("{value}: unexpected error {error:?}");
        };
        let message = source.to_string();
        assert!(message.starts_with(&format!("n: number {value} ")), "{message}");
        assert!(message.contains(reason), "{message}");
        let location = source.location().expect("a located error");
        assert_eq!((location.line(), location.column()), (2, 4), "{value}");
    }
}

#[test]
fn admissible_frontmatter_numbers_load_exactly() {
    let markdown = Markdown::try_from_content(
        "---\nmax: 9007199254740992\nmin: -9007199254740992\ntenth: 0.1\nthousand: 1e3\nhalf: .5\nquoted: \"9007199254740993\"\n---\nBody\n",
    )
    .unwrap();
    let map = markdown.frontmatter().as_map();
    assert_eq!(map["max"], json!(9_007_199_254_740_992_u64));
    assert_eq!(map["min"], json!(-9_007_199_254_740_992_i64));
    assert_eq!(map["tenth"], json!(0.1));
    assert_eq!(map["thousand"].as_f64(), Some(1000.0));
    assert_eq!(map["half"], json!(0.5));
    assert_eq!(map["quoted"], json!("9007199254740993"));
    assert_eq!(
        map.keys().map(String::as_str).collect::<Vec<_>>(),
        ["max", "min", "tenth", "thousand", "half", "quoted"]
    );
}

#[test]
fn quoted_inexact_text_against_a_number_fails_validation_with_the_reason() {
    let content = "---\n$schema:\n  n: number(required)\nn: \"9007199254740993\"\n---\nBody\n";
    let report = DarkmatterSchemas::new(crate::request_support::cwd_context())
        .validate(&Markdown::from(content))
        .expect("validation runs");
    assert!(!report.valid);
    let messages: Vec<_> = report.problems.iter().map(|problem| problem.message.clone()).collect();
    assert!(messages.iter().any(|message| message.contains(INEXACT)), "{messages:?}");

    match compose(content) {
        Err(MarkdownError::SchemaValidationFailed { .. }) => {}
        other => panic!("expected a schema validation failure, got {other:?}"),
    }
    assert_eq!(
        composed_body("---\n$schema:\n  n: number(required)\nn: \"9007199254740992\"\n---\n{{ n - 1 }}\n"),
        "9007199254740991"
    );
}

// ── Arithmetic and presentation ───────────────────────────────────────────

#[test]
fn non_finite_results_are_errors() {
    let big = largest_finite_spelling();
    let error = compose_error(&format!("---\nbig: {big}\n---\n{{{{ big * 10 }}}}\n"));
    assert!(error.contains("not finite"), "{error}");
    assert!(compose_error("{{ 1 / 0 }}\n").contains("Division by zero"));
    assert!(compose_error("{{ 1 % 0 }}\n").contains("Remainder by zero"));
    assert!(compose_error("{{ 0 / 0 }}\n").contains("Division by zero"));
    let squares = "9007199254740992 * 9007199254740992 * 9007199254740992 * 9007199254740992 * 9007199254740992 \
                   * 9007199254740992 * 9007199254740992 * 9007199254740992 * 9007199254740992 * 9007199254740992 \
                   * 9007199254740992 * 9007199254740992 * 9007199254740992 * 9007199254740992 * 9007199254740992 \
                   * 9007199254740992 * 9007199254740992 * 9007199254740992 * 9007199254740992 * 9007199254740992";
    assert!(compose_error(&format!("{{{{ {squares} }}}}\n")).contains("not finite"));
}

#[test]
fn the_largest_finite_number_loads_and_renders_every_digit() {
    let big = largest_finite_spelling();
    assert_eq!(composed_body(&format!("---\nbig: {big}\n---\n{{{{ big }}}}\n")), big);
}

#[test]
fn equality_and_ordering_agree_on_represented_values() {
    assert_eq!(
        composed_body("---\nf: 2.0\n---\n{{ 2 == 2.0 }}|{{ 2 < 2.5 }}|{{ 2.5 > 2 }}|{{ f == 2 }}|{{ f < 2.5 }}|{{ 2 != 2.0 }}\n"),
        "true|true|true|true|true|false"
    );
}

#[test]
fn whole_numbers_never_pass_through_an_integer_cast() {
    assert_eq!(
        composed_body("{{ 9007199254740992 + 0 }}|{{ 4.0 }}|{{ 2.5 }}|{{ 4.5 - 0.5 }}|{{ 10 / 4 }}\n"),
        "9007199254740992|4|2.5|4|2.5"
    );
    assert_eq!(composed_body("---\nf: 4.0\n---\n{{ f }}|{{ f * 1 }}\n"), "4|4");
}

#[test]
fn numeric_text_converts_through_the_canonical_parser() {
    // Arithmetic on numeric text keeps converting; inexact text is an error,
    // not a rounded number, and non-numeric text still concatenates.
    assert_eq!(composed_body("{{ \"4\" - 1 }}|{{ \" 4 \" * 2 }}|{{ \"1_000\" + 1 }}|{{ \"a\" + 1 }}\n"), "3|8|1001|a1");
    let error = compose_error("{{ \"9007199254740993\" + 1 }}\n");
    assert!(error.contains(INEXACT), "{error}");
}

#[test]
fn an_out_of_range_index_is_none_rather_than_saturated() {
    assert_eq!(
        composed_body("---\nitems: [a, b]\n---\n[{{ items[1] }}|{{ items[-1] }}|{{ items[9007199254740992] }}|{{ items[-9007199254740992] }}|{{ items[1.5] }}]\n"),
        "[b|b|||]"
    );
}

// ── Raw schemas ───────────────────────────────────────────────────────────

fn validate_against(dir: &std::path::Path, schema_file: &str, frontmatter: &str) -> Result<bool, SchemaError> {
    let markdown = Markdown::from(format!("---\n$schema: ./{schema_file}\n{frontmatter}\n---\nBody\n"))
        .with_source(darkmatter::markdown::compose::ComposeSource::File(dir.join("document.md")));
    DarkmatterSchemas::new(biscuit_file::FileResolutionContext::new(dir))
        .validate(&markdown)
        .map(|report| report.valid)
}

#[test]
fn raw_schema_numbers_are_admitted_at_load() {
    let workspace = tempfile::tempdir().expect("workspace");
    let dir = workspace.path();
    for (index, (extension, schema, reason)) in [
        ("json", r#"{"type": "object", "properties": {"n": {"maximum": 9007199254740993.0}}}"#, INEXACT),
        ("json", r#"{"type": "object", "properties": {"n": {"minimum": 1e-400}}}"#, "a non-zero value would become 0"),
        ("json", r#"{"type": "object", "properties": {"n": {"enum": [1, 9007199254740993]}}}"#, INEXACT),
        ("yaml", "type: object\nproperties:\n  n:\n    maximum: .inf\n", "not finite"),
    ]
    .into_iter()
    .enumerate()
    {
        let name = format!("schema-{index}.{extension}");
        std::fs::write(dir.join(&name), schema).unwrap();
        let error = validate_against(dir, &name, "n: 1").expect_err(schema);
        assert!(matches!(error, SchemaError::SchemaDocument { .. }), "{error:?}");
        assert!(error.to_string().contains(reason), "{error}");
        assert!(error.to_string().contains("/properties/n/") || error.to_string().contains("properties.n."), "{error}");
    }
}

#[test]
fn ordinary_raw_schema_bounds_still_load_and_validate() {
    let workspace = tempfile::tempdir().expect("workspace");
    let dir = workspace.path();
    std::fs::write(
        dir.join("bounds.json"),
        r#"{"type": "object", "properties": {"n": {"minimum": 0.5, "maximum": 9007199254740992, "multipleOf": 0.5}}}"#,
    )
    .unwrap();
    std::fs::write(dir.join("bounds.yaml"), "type: object\nproperties:\n  n:\n    maximum: 1e3\n").unwrap();
    assert!(validate_against(dir, "bounds.json", "n: 2.5").unwrap());
    assert!(!validate_against(dir, "bounds.json", "n: 0.25").unwrap());
    assert!(validate_against(dir, "bounds.yaml", "n: 1000").unwrap());
    assert!(!validate_against(dir, "bounds.yaml", "n: 1001").unwrap());
}
