//! Envelope data: message hygiene, the opaque source excerpt, and the
//! serialized shape.

use serde_json::json;

use super::*;

#[derive(Debug, PartialEq)]
struct Lines(&'static str);

impl SourceAppendix for Lines {
    fn render_appendix(&self, _term: &Terminal) -> String {
        format!("\n\n{}", self.0)
    }
}

fn coded() -> DiagnosticSnapshot {
    DiagnosticSnapshot {
        schema_version: DIAGNOSTIC_SNAPSHOT_SCHEMA_VERSION,
        category: "composition".to_string(),
        code: "composition.cancelled".to_string(),
        disposition: "terminal".to_string(),
        origin: "runtime".to_string(),
        severity: "error".to_string(),
        detail: json!({}),
        message: "cancelled".to_string(),
        cause: None,
        frontmatter_excerpt: Some(SourceExcerpt::new(Lines("a: 1"))),
    }
}

/// `(input, expected)` pairs covering every hygiene rule: escapes, every line
/// boundary, stray controls, empty and blank input, and interior spacing.
const HYGIENE: &[(&str, &str)] = &[
    ("\u{1b}[1;31mred\u{1b}[0m text", "red text"),
    ("see \u{1b}]8;;https://example.com\u{7}the docs\u{1b}]8;;\u{7} now", "see the docs now"),
    ("first\nsecond", "first second"),
    ("first\r\nsecond", "first second"),
    ("first\tsecond", "first second"),
    ("bell\u{7}here", "bell here"),
    ("line\u{2028}separator", "line separator"),
    ("paragraph\u{2029}separator", "paragraph separator"),
    ("next\u{85}line", "next line"),
    ("", ""),
    (" \n\t\r\n  ", ""),
    ("interior  double  spaces", "interior  double  spaces"),
    ("  padded\n  lines  \n", "padded lines"),
];

#[test]
fn hygiene_table() {
    for (input, expected) in HYGIENE {
        assert_eq!(concise_message(input), *expected, "input {input:?}");
    }
}

#[test]
fn concise_message_is_idempotent() {
    let once = concise_message("\u{1b}[31mred\u{1b}[0m\nline");

    assert_eq!(once, "red line");
    assert_eq!(concise_message(&once), once);
    for (input, _) in HYGIENE {
        let once = concise_message(input);
        assert_eq!(concise_message(&once), once, "input {input:?}");
    }
}

#[test]
fn a_long_base_and_suffix_stay_intact() {
    let long = "x".repeat(600);

    let message = concise_message_with_suffix(&long, " (attempt 2)");

    assert_eq!(message, format!("{long} (attempt 2)"));
    assert!(!message.contains('…'), "{message}");
}

#[test]
fn lengths_around_the_old_limit_are_unchanged() {
    let mixed: String = "path/段落/🦀 ".chars().cycle().take(2_000).collect::<String>().trim_end().to_string();
    let mut inputs: Vec<String> = [239, 240, 241].iter().map(|len| "a".repeat(*len)).collect();
    inputs.push(mixed);

    for input in &inputs {
        let message = concise_message(input);
        assert_eq!(message, *input);
        assert_eq!(message.chars().count(), input.chars().count());
    }
}

#[test]
fn the_suffix_is_normalized_too() {
    let message = concise_message_with_suffix("base", " (attempt \u{1b}[31m2\u{1b}[0m\n)");

    assert_eq!(message, "base (attempt 2 )");
}

#[test]
fn excerpts_compare_by_value_and_render_through_the_host() {
    let term = Terminal::new_optimistic(80);
    let excerpt = SourceExcerpt::new(Lines("a: 1"));

    assert_eq!(excerpt, SourceExcerpt::new(Lines("a: 1")));
    assert_ne!(excerpt, SourceExcerpt::new(Lines("b: 2")));
    assert_eq!(excerpt.render_appendix(&term), "\n\na: 1");
    assert_eq!(excerpt.downcast_ref::<Lines>(), Some(&Lines("a: 1")));
}

#[test]
fn the_excerpt_never_serializes() {
    let value = serde_json::to_value(coded()).unwrap();

    assert!(value.get("frontmatter_excerpt").is_none());
    assert!(value.get("cause").is_none());
    let restored: DiagnosticSnapshot = serde_json::from_value(value).unwrap();
    assert_eq!(restored.frontmatter_excerpt, None);
}
