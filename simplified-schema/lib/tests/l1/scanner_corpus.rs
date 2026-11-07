//! Old-versus-new scanner invariants over fixed inputs (plan rule 25): every
//! span the balanced scan changes parses and its older span did not, and
//! every older span that parses is still found unchanged. A span the older
//! scan never reached, because an earlier span used to swallow its opener,
//! must be either a parsing span or the older rule's span at that opener.
//!
//! The broad comparison was run once, by the 2026-11-05 scanner spike, over
//! 6,909 repository files holding 10,399 expressions; 6 files changed, all
//! improvements. This test does not assert that count. It reads only the
//! frozen fixture and the shipped schema files named below, each through an
//! `include_str!` inside the test so CI's test-input index sees it.

use std::collections::HashSet;

use simplified_schema::expression::parse;
use simplified_schema::expression::scan::{ExpressionLocation, scan_plain};

/// The first-`}}` scanner as it stood before the balanced rule, copied
/// verbatim from `scan.rs` with nothing excluded and only expressions kept.
fn legacy_scan(content: &str) -> Vec<ExpressionLocation> {
    let mut expressions = Vec::new();
    let mut pos = 0;
    let bytes = content.as_bytes();
    let len = bytes.len();

    while pos < len.saturating_sub(3) {
        if bytes[pos] == b'{' && bytes[pos + 1] == b'{' && is_escaped_opener(bytes, pos) {
            while pos < len && bytes[pos] == b'{' {
                pos += 1;
            }
            continue;
        }

        if bytes[pos] == b'{'
            && pos + 1 < len
            && bytes[pos + 1] == b'{'
            && pos + 2 < len
            && bytes[pos + 2] == b'{'
            && (pos + 3 >= len || bytes[pos + 3] != b'{')
        {
            let start = pos;
            let mut close_pos = pos + 3;
            let mut found_close = false;
            while close_pos + 2 < len {
                if bytes[close_pos] == b'}' && bytes[close_pos + 1] == b'}' && bytes[close_pos + 2] == b'}' {
                    pos = close_pos + 3;
                    found_close = true;
                    break;
                }
                close_pos += 1;
            }

            if !found_close {
                let (expr, next_pos) = scan_legacy_expression(content, start);
                if let Some(expr) = expr {
                    expressions.push(expr);
                }
                pos = next_pos;
            }
            continue;
        }

        if bytes[pos] == b'{' && pos + 1 < len && bytes[pos + 1] == b'{' {
            let start = pos;
            if content[start + 2..].starts_with("!data:") {
                let body = start + 2 + "!data:".len();
                pos = content[body..].find("}}").map_or(len, |close| body + close + 2);
                continue;
            }
            let (expr, next_pos) = scan_legacy_expression(content, start);
            if let Some(expr) = expr {
                expressions.push(expr);
            }
            pos = next_pos;
            continue;
        }

        pos += 1;
    }
    expressions
}

fn scan_legacy_expression(content: &str, start: usize) -> (Option<ExpressionLocation>, usize) {
    let bytes = content.as_bytes();
    let len = bytes.len();
    let mut pos = start + 2;
    let mut depth = 1;

    while pos + 1 < len {
        if bytes[pos] == b'{' && bytes[pos + 1] == b'{' {
            depth += 1;
            pos += 2;
        } else if bytes[pos] == b'}' && bytes[pos + 1] == b'}' {
            depth -= 1;
            if depth == 0 {
                let end = pos + 2;
                let expr_start = start + 2;
                let expr_end = pos;
                if expr_start < expr_end {
                    let expression = content[expr_start..expr_end].trim();
                    if !expression.is_empty() {
                        return (
                            Some(ExpressionLocation {
                                start,
                                end,
                                expression: expression.to_string(),
                            }),
                            end,
                        );
                    }
                }
                return (None, end);
            }
            pos += 2;
        } else {
            pos += 1;
        }
    }

    (None, start + 2)
}

fn is_escaped_opener(bytes: &[u8], pos: usize) -> bool {
    let run = bytes[..pos].iter().rev().take_while(|&&b| b == b'\\').count();
    run % 2 == 1
}

/// Asserts the invariants for one input and returns its changed spans.
fn changed_spans(name: &str, text: &str) -> Vec<ExpressionLocation> {
    let old = legacy_scan(text);
    let new = scan_plain(text).expressions;
    let old_set: HashSet<_> = old.iter().map(|found| (found.start, found.end)).collect();
    let new_set: HashSet<_> = new.iter().map(|found| (found.start, found.end)).collect();

    for kept in old.iter().filter(|found| parse(&found.expression).is_ok()) {
        assert!(
            new.contains(kept),
            "{name}: older span {kept:?} parses but the new scan changed or lost it"
        );
    }
    let changed: Vec<_> = new
        .into_iter()
        .filter(|found| !old_set.contains(&(found.start, found.end)))
        .collect();
    for span in &changed {
        // A fixed span can end before an opener the older scan swallowed;
        // that opener then gets the older rule's own span, which is not a
        // change of rule.
        if scan_legacy_expression(text, span.start).0.as_ref() == Some(span) {
            continue;
        }
        assert!(
            parse(&span.expression).is_ok(),
            "{name}: changed span {span:?} does not parse"
        );
        if let Some(before) = old.iter().find(|found| found.start == span.start) {
            assert!(
                parse(&before.expression).is_err(),
                "{name}: changed span {span:?} replaced a parsing span {before:?}"
            );
        }
    }
    for lost in old.iter().filter(|found| !new_set.contains(&(found.start, found.end))) {
        assert!(
            parse(&lost.expression).is_err(),
            "{name}: older span {lost:?} parses but is gone"
        );
    }
    changed
}

#[test]
fn scanner_corpus_changes_only_spans_that_did_not_parse() {
    let fixture = include_str!("../fixtures/scanner/regressions.md");
    let changed: Vec<String> = changed_spans("regressions.md", fixture)
        .into_iter()
        .map(|found| found.expression)
        .collect();
    for expected in [
        r#""{{""#,
        r#"load_data(f, "{ a: { b: string }}")"#,
        r#"load_data(f, "{a: {b: string}}")"#,
        "{a: {b: 1}}",
        r#""}}""#,
        "'{{'",
        r#""a\"}}""#,
        r"'it\'s }}'",
    ] {
        assert!(
            changed.iter().any(|found| found == expected),
            "{expected} not among {changed:?}"
        );
    }
    let found: Vec<String> = scan_plain(fixture)
        .expressions
        .into_iter()
        .map(|found| found.expression)
        .collect();
    for kept in ["don't", "x", r#""open"#, "y", "name", "title", "b * 2", "c + 1"] {
        assert!(
            found.iter().any(|expression| expression == kept),
            "{kept} not among {found:?}"
        );
    }

    for (name, text) in [
        (
            "darkmatter/schemas/partials/lifecycle.yaml",
            include_str!("../../../../darkmatter/schemas/partials/lifecycle.yaml"),
        ),
        (
            "darkmatter/schemas/partials/functions.yaml",
            include_str!("../../../../darkmatter/schemas/partials/functions.yaml"),
        ),
        (
            "darkmatter/schemas/darkmatter.yaml",
            include_str!("../../../../darkmatter/schemas/darkmatter.yaml"),
        ),
        (
            "simplified-schema/schemas/schema.yaml",
            include_str!("../../../schemas/schema.yaml"),
        ),
        (
            "simplified-schema/schemas/yaml-schema.yaml",
            include_str!("../../../schemas/yaml-schema.yaml"),
        ),
    ] {
        changed_spans(name, text);
    }
}
