//! `expression_region_at`: the editor's view of whether an offset sits in a
//! complete or still-open `{{ … }}`, under the scanner's lexical rules.

use simplified_schema::expression::scan::{ExpressionRegion, expression_region_at};

fn region_at_end(text: &str) -> Option<ExpressionRegion> {
    expression_region_at(text, text.len(), &[])
}

fn open(start: usize, string_start: Option<usize>, brace_depth: usize) -> Option<ExpressionRegion> {
    Some(ExpressionRegion {
        start,
        end: None,
        string_start,
        brace_depth,
    })
}

#[test]
fn a_cursor_after_a_nested_type_string_is_in_an_open_region() {
    assert_eq!(
        region_at_end(r#"{{ load_data(f, "{a: {b: string}}") "#),
        open(0, None, 0)
    );
    assert_eq!(
        region_at_end(r#"{{ load_data(f, "{ a: { b: string }}") "#),
        open(0, None, 0)
    );
}

#[test]
fn a_quoted_opener_never_moves_the_region_start() {
    let text = "{{ f('{{', ";
    assert_eq!(region_at_end(text), open(0, None, 0));
    // Inside `'{{`, the quote at byte 5 holds the cursor.
    assert_eq!(expression_region_at(text, 7, &[]), open(0, Some(5), 0));
}

#[test]
fn a_cursor_after_a_complete_expression_is_in_the_next_opener() {
    assert_eq!(region_at_end("{{ a }} {{ b"), open(8, None, 0));
    assert_eq!(expression_region_at("{{ a }} {{ b", 7, &[]), None);
}

#[test]
fn a_cursor_in_a_string_of_a_complete_expression_reports_its_end() {
    let text = r#"{{ f("}}") }}"#;
    assert_eq!(
        expression_region_at(text, 7, &[]),
        Some(ExpressionRegion {
            start: 0,
            end: Some(text.len()),
            string_start: Some(5),
            brace_depth: 0,
        })
    );
}

#[test]
fn open_single_braces_count_toward_the_depth() {
    assert_eq!(region_at_end("{{ {a: {b: "), open(0, None, 2));
    assert_eq!(region_at_end("{{ {a: {b: 1}, c: "), open(0, None, 1));
}

#[test]
fn prose_and_delimiters_are_outside_every_region() {
    assert_eq!(region_at_end("plain prose, no opener"), None);
    assert_eq!(expression_region_at("x {{ a }} y", 0, &[]), None);
    // Between the braces of the opener and of the closer.
    assert_eq!(expression_region_at("x {{ a }} y", 3, &[]), None);
    assert_eq!(expression_region_at("x {{ a }} y", 8, &[]), None);
    assert_eq!(expression_region_at("x", 5, &[]), None);
}

#[test]
fn the_cursor_right_after_the_opener_and_before_the_closer_is_inside() {
    let text = "{{ a }}";
    let complete = Some(ExpressionRegion {
        start: 0,
        end: Some(7),
        string_start: None,
        brace_depth: 0,
    });
    assert_eq!(expression_region_at(text, 2, &[]), complete);
    assert_eq!(expression_region_at(text, 5, &[]), complete);
    assert_eq!(region_at_end("{{"), open(0, None, 0));
}

#[test]
fn excluded_escaped_literal_and_token_openers_hold_no_region() {
    assert_eq!(expression_region_at("`{{ x` y", 5, &[(0, 6)]), None);
    assert_eq!(expression_region_at("a {{{ x }}} b", 6, &[]), None);
    assert_eq!(region_at_end(r"\{{ x"), None);
    assert_eq!(expression_region_at("{{!data:v1:YQ}} b", 9, &[]), None);
    // A region after a literal or token is still found.
    assert_eq!(region_at_end("{{{ x }}} {{ y"), open(10, None, 0));
    assert_eq!(region_at_end("{{!data:v1:YQ}} {{ y"), open(16, None, 0));
}

#[test]
fn a_new_opener_outside_strings_starts_a_new_region() {
    let text = "{{ don't }} it's fine {{ x";
    assert_eq!(region_at_end(text), open(22, None, 0));
}

#[test]
fn offsets_are_bytes_across_crlf_and_multibyte_text() {
    let text = "é: ü\r\n{{ f(\"ñ";
    let quote = text.find('"').unwrap();
    assert_eq!(region_at_end(text), open(text.find("{{").unwrap(), Some(quote), 0));
    // An offset between a backslash and a multibyte character stays in the string.
    let escaped = "{{ \"\\é";
    assert_eq!(expression_region_at(escaped, 5, &[]), open(0, Some(3), 0));
}
