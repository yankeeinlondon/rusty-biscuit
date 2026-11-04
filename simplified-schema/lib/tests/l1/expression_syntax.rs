//! The passive expression parser through the crate's public API: the entry
//! points the schema checks call (`parse` for expression-typed
//! values and expression defaults, `parse_spanned` for function-example calls
//! and their expected literals) accept any well-formed call, whether or not a
//! host defines the function, and reject malformed text with a byte position.

use simplified_schema::expression::{
    Expr, SpannedExpr, SpannedExprKind, Token, lex_spanned, parse, parse_spanned,
};

/// The name and argument count of a top-level call, or `None` when `expr` is
/// not one.
fn call_of(expr: &SpannedExpr) -> Option<(&str, usize)> {
    match &expr.kind {
        SpannedExprKind::FunctionCall { name, args } => Some((name.as_str(), args.len())),
        _ => None,
    }
}

#[test]
fn an_unresolvable_call_is_an_expression_typed_value_and_default() {
    // The expression-typed value check and the expression-default check both
    // ask `parse`. No host defines `unknown_fn`; syntax is all that
    // is checked.
    let expr = parse("unknown_fn(1)").expect("a well-formed call parses");
    assert_eq!(
        expr,
        Expr::FunctionCall { name: "unknown_fn".into(), args: vec![Expr::NumberLiteral(1.0)] }
    );
    // Nested unknown calls, unknown variables, and both logical operators.
    assert!(parse("unknown_fn(other_fn(x), y.z) && !missing || w").is_ok());
}

#[test]
fn an_unresolvable_call_is_a_function_example_call() {
    // The function-example check asks `parse_spanned` and requires a call.
    let source = "unknown_fn(1)";
    let call = parse_spanned(source).expect("a well-formed call parses");
    assert_eq!(call_of(&call), Some(("unknown_fn", 1)));
    assert_eq!(call.span, 0..source.len());
    // The spanned tree erases to exactly what `parse` produces.
    assert_eq!(call.erase(), parse(source).unwrap());
}

#[test]
fn an_example_expected_literal_parses_to_literal_nodes() {
    // The expected side of `example(call => result)` is parsed, then folded
    // into JSON by the schema grammar; here, only its parse is the library's.
    let expected = parse_spanned("{ok: [1, 2.5], none: null}").expect("literal parses");
    let SpannedExprKind::ObjectLiteral(entries) = &expected.kind else {
        panic!("object literal, got {expected:?}");
    };
    let keys: Vec<&str> = entries.iter().map(|(key, _)| key.value.as_str()).collect();
    assert_eq!(keys, ["ok", "none"]);
    assert!(matches!(&entries[0].1.kind, SpannedExprKind::ArrayLiteral(items) if items.len() == 2));
    // `null` is an identifier to the parser; the grammar gives it meaning.
    assert!(matches!(&entries[1].1.kind, SpannedExprKind::Variable(name) if name == "null"));
}

#[test]
fn malformed_text_is_rejected_with_a_byte_position() {
    for (source, position) in [("unknown_fn(", 11), ("((", 2), ("unknown_fn(1,)", 13), ("1 +", 3)] {
        let erased = parse(source).expect_err(source);
        let spanned = parse_spanned(source).expect_err(source);
        assert_eq!((erased.position, spanned.position), (position, position), "{source}: {erased}");
        assert!(erased.to_string().ends_with(&format!("at position {position}")));
    }
    // A lexer failure surfaces through the parser at the offending byte.
    let unterminated = parse("unknown_fn(\"open)").expect_err("unterminated string");
    assert_eq!(unterminated.position, 11);
}

#[test]
fn or_keeps_its_own_node_and_and_lowers_to_a_call() {
    // One grammar: `||` is the `LogicalOr` node wherever it is parsed, and
    // `&&` lowers to `and(a, b)`.
    let call_name = |expr: Expr| match expr {
        Expr::FunctionCall { name, .. } => Some(name),
        _ => None,
    };
    assert_eq!(
        parse("a || b").unwrap(),
        Expr::LogicalOr { left: Box::new(Expr::Variable("a".into())), right: Box::new(Expr::Variable("b".into())) }
    );
    assert_eq!(call_name(parse("a && b").unwrap()).as_deref(), Some("and"));
    let tokens: Vec<Token> = lex_spanned("a || b").unwrap().into_iter().map(|token| token.value).collect();
    assert_eq!(tokens, [Token::Variable("a".into()), Token::Pipe, Token::Variable("b".into()), Token::Eof]);
}

#[test]
fn spans_are_byte_offsets_into_crlf_text_with_windows_paths() {
    // Criterion 8: offsets index the exact supplied text. CRLF is whitespace
    // (two bytes), and a Windows path in a string literal is opaque data.
    let source = "unknown_fn(\r\n  'C:\\\\Users\\\\é\\\\a.md',\r\n  2)";
    let call = parse_spanned(source).expect("CRLF-separated call parses");
    let SpannedExprKind::FunctionCall { args, .. } = &call.kind else { panic!("call") };
    assert_eq!(&source[args[0].span.clone()], "'C:\\\\Users\\\\é\\\\a.md'");
    assert!(matches!(&args[0].kind, SpannedExprKind::StringLiteral(text) if text == "C:\\Users\\é\\a.md"));
    assert_eq!(&source[args[1].span.clone()], "2");
    assert_eq!(args[1].span.start, source.rfind('2').unwrap());
    assert_eq!(call.span, 0..source.len());
}

#[cfg(feature = "work-counters")]
#[test]
fn each_parse_is_counted_once_whatever_the_outcome() {
    use simplified_schema::work_counters::{Work, snapshot};

    let before = snapshot();
    parse("unknown_fn(1)").unwrap();
    parse_spanned("unknown_fn(").unwrap_err();
    parse("a || b").unwrap();
    assert_eq!(snapshot().since(&before).get(Work::ExpressionParse), 3);
}
