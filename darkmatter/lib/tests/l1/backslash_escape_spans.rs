//! A backslash before `{{` opts body text out of expression scanning, and
//! compose preserves every authored backslash.

use darkmatter::markdown::Markdown;

fn compose_body(body: &str) -> (String, Vec<String>) {
    let source = format!("---\nx: value\n---\n{body}\n");
    let markdown: Markdown = source.into();
    let (composed, report) = markdown.compose().expect("compose should succeed");
    let warnings = report.warnings.iter().map(|w| w.message.clone()).collect();
    (composed.content().trim_end().to_string(), warnings)
}

#[test]
fn escaped_opener_survives_compose_with_backslash_intact() {
    let (output, warnings) = compose_body(r"Write \{{ x }} to template.");
    assert_eq!(output, r"Write \{{ x }} to template.");
    assert!(warnings.is_empty(), "{warnings:?}");
}

#[test]
fn fully_escaped_braces_survive_compose() {
    let (output, warnings) = compose_body(r"Write \{\{ x }} to template.");
    assert_eq!(output, r"Write \{\{ x }} to template.");
    assert!(warnings.is_empty(), "{warnings:?}");
}

#[test]
fn unescaped_span_still_interpolates() {
    let (output, warnings) = compose_body("Write {{ x }} to template.");
    assert_eq!(output, "Write value to template.");
    assert!(warnings.is_empty(), "{warnings:?}");
}

#[test]
fn backslash_parity_decides_interpolation_and_keeps_every_backslash() {
    let (even, _) = compose_body(r"even \\{{ x }} end");
    assert_eq!(even, r"even \\value end");

    let (odd, _) = compose_body(r"odd \\\{{ x }} end");
    assert_eq!(odd, r"odd \\\{{ x }} end");
}

#[test]
fn unrelated_backslashes_are_untouched() {
    let (output, _) = compose_body(r"C:\path\to\file and \* star and {{ x }}");
    assert_eq!(output, r"C:\path\to\file and \* star and value");
}

#[test]
fn escaped_foreign_template_examples_produce_no_expression_warnings() {
    // The deprecated single-pipe form from `claudine/docs/topics/unified-events.md`
    // is not Darkmatter syntax, so unescaped it warns.
    let example = r#"{{env.VAR | "default"}}"#;
    let (_, unescaped_warnings) = compose_body(&format!("Old form: {example}"));
    assert!(
        unescaped_warnings.iter().any(|w| w.contains("failed to parse")),
        "control case should warn: {unescaped_warnings:?}"
    );

    let escaped = format!("Old form: \\{example} and `\\{example}`");
    let (output, warnings) = compose_body(&escaped);
    assert_eq!(output, escaped);
    assert!(warnings.is_empty(), "{warnings:?}");
}
