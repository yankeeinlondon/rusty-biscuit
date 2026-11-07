//! The balanced, parse-gated `{{ … }}` scan (spec R11): quoted strings and
//! single braces no longer end an expression early, while every span that
//! parses under the older first-`}}` rule stays as it was.

use simplified_schema::expression::scan::{ExpressionLocation, find_all_plain, scan, scan_plain};

fn expressions(content: &str) -> Vec<String> {
    find_all_plain(content)
        .into_iter()
        .map(|found| found.expression)
        .collect()
}

#[test]
fn a_nested_type_string_keeps_the_whole_call() {
    for content in [
        r#"{{ load_data(f, "{ a: { b: string }}") }}"#,
        r#"{{ load_data(f, "{a: {b: string}}") }}"#,
    ] {
        let found = find_all_plain(content);
        assert_eq!(found.len(), 1, "for {content:?}");
        assert_eq!((found[0].start, found[0].end), (0, content.len()), "for {content:?}");
        assert_eq!(found[0].expression, content[2..content.len() - 2].trim());
    }
}

#[test]
fn a_nested_object_literal_closes_after_its_braces() {
    assert_eq!(expressions("{{ {a: {b: 1}} }}"), vec!["{a: {b: 1}}"]);
}

#[test]
fn closing_and_opening_braces_inside_strings_are_text() {
    for (content, expected) in [
        (r#"{{ "}}" }}"#, r#""}}""#),
        ("{{ '{{' }}", "'{{'"),
        (r#"{{ "a\"}}" }}"#, r#""a\"}}""#),
        (r"{{ 'it\'s }}' }}", r"'it\'s }}'"),
    ] {
        assert_eq!(expressions(content), vec![expected], "for {content:?}");
    }
}

#[test]
fn an_escaped_backslash_ends_the_string() {
    assert_eq!(expressions(r#"{{ "a\\" }} b"#), vec![r#""a\\""#]);
}

#[test]
fn a_string_may_span_lines() {
    let content = "{{ f(\"multi\nline }}\") }}";
    assert_eq!(expressions(content), vec!["f(\"multi\nline }}\")"]);
    let crlf = "{{ f(\"multi\r\nline }}\") }}";
    assert_eq!(find_all_plain(crlf)[0].end, crlf.len());
}

#[test]
fn prose_apostrophes_keep_the_older_spans() {
    assert_eq!(
        expressions("{{ don't }} it's fine {{ x }} and more }}"),
        vec!["don't", "x"]
    );
    assert_eq!(
        expressions(r#"{{ "open }} then {{ y }} and " more }}"#),
        vec![r#""open"#, "y"]
    );
    assert_eq!(
        expressions("{{ don't }} and later 'quoted' text {{ x }}"),
        vec!["don't", "x"]
    );
    assert_eq!(
        expressions("Ken's notes: {{ name }} isn't here, {{ x }}'s value"),
        vec!["name", "x"]
    );
}

#[test]
fn a_quoted_opener_in_prose_leaves_later_expressions_found() {
    let content = "Check with `contains(\"{{\")` first.\n\nThen {{ title }} renders,\nand {{ b * 2 }} and {{ c + 1 }}.\nIt's \"done\" here.\n";
    assert_eq!(expressions(content), vec!["title", "b * 2", "c + 1"]);
}

#[test]
fn an_unbalanced_quote_without_a_closer_scans_as_before() {
    assert!(expressions(r#"{{ "open and no close"#).is_empty());
    assert_eq!(expressions(r#"{{ "open }} rest"#), vec![r#""open"#]);
    // No `}}` after the quote: the older scan resumes inside the opener.
    assert_eq!(expressions(r#"{{ "open {{ x }}"#), vec!["x"]);
}

#[test]
fn unbalanced_and_nested_openers_scan_as_before() {
    assert_eq!(expressions("{{ a { }} b"), vec!["a {"]);
    assert_eq!(expressions("{{ a {{ b }} c }}"), vec!["a {{ b }} c"]);
    assert_eq!(expressions("{{{ x }}"), vec!["{ x"]);
}

#[test]
fn literals_and_tokens_are_unchanged() {
    for (content, literal) in [
        ("{{{VAR}}}", "VAR"),
        (r#"{{{ "}}}" }}}"#, r#" ""#),
        ("{{{a: 1}}}", "a: 1"),
    ] {
        let result = scan_plain(content);
        assert!(result.expressions.is_empty(), "for {content:?}");
        assert_eq!(result.literals[0].content, literal, "for {content:?}");
    }
    let result = scan_plain(r#"{{!data:v1:YQ}} {{ "}}" }}"#);
    assert_eq!((result.tokens[0].start, result.tokens[0].end), (0, 15));
    assert_eq!(result.expressions[0].expression, r#""}}""#);
}

#[test]
fn escaped_and_excluded_openers_stay_inactive() {
    assert!(expressions(r#"\{{ "}}" }}"#).is_empty());
    assert_eq!(expressions(r#"\\{{ "}}" }}"#), vec![r#""}}""#]);
    let content = r#"{{ "}}" }} {{ "}}" }}"#;
    let found = scan(content, &[(0, 10)]).expressions;
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].start, 11);
}

#[test]
fn a_changed_span_ends_after_the_real_closer_and_scanning_continues() {
    let content = r#"a {{ f("}}") }} b {{ z }}"#;
    assert_eq!(
        find_all_plain(content),
        vec![
            ExpressionLocation {
                start: 2,
                end: 15,
                expression: r#"f("}}")"#.to_string(),
            },
            ExpressionLocation {
                start: 18,
                end: 25,
                expression: "z".to_string(),
            },
        ]
    );
}

#[test]
fn multibyte_text_keeps_byte_offsets() {
    let content = "é {{ f(\"ü}}\\é\") }} ñ";
    let found = find_all_plain(content);
    assert_eq!(found.len(), 1);
    assert_eq!(&content[found[0].start..found[0].end], "{{ f(\"ü}}\\é\") }}");
}
