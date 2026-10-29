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

#[test]
fn a_suffix_survives_clamping() {
    let long = "x".repeat(CONCISE_MESSAGE_MAX_CHARS * 2);
    let message = concise_message_with_suffix(&long, " (attempt 2)");

    assert_eq!(message.chars().count(), CONCISE_MESSAGE_MAX_CHARS);
    assert!(message.ends_with("… (attempt 2)"), "{message}");
}

#[test]
fn concise_message_is_idempotent() {
    let once = concise_message("\u{1b}[31mred\u{1b}[0m\nline");

    assert_eq!(once, "red line");
    assert_eq!(concise_message(&once), once);
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
