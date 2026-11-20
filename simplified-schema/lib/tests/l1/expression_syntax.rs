//! The passive expression parser through the crate's public API: the entry
//! points the schema checks call (`parse` for expression-typed
//! values and expression defaults, `parse_spanned` for function-example calls
//! and their expected literals) accept any well-formed call, whether or not a
//! host defines the function, and reject malformed text with a byte position.

use simplified_schema::expression::scan::scan_plain;
use simplified_schema::expression::{
    Expr, ObjectEntry, SpannedExpr, SpannedExprKind, SpannedObjectEntry, Token, lex_spanned, parse, parse_spanned,
};
use simplified_schema::simplified::parse_property_definition;

fn parse_type_definition(
    property: &str,
    text: &str,
) -> Result<simplified_schema::simplified::PropertyDef, simplified_schema::SchemaError> {
    parse_property_definition(property, &serde_yaml_ng::Value::String(text.to_string()))
}

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
    let keys: Vec<&str> = entries
        .iter()
        .map(|entry| match entry {
            SpannedObjectEntry::Field { key, .. } => key.value.as_str(),
            SpannedObjectEntry::Spread { .. } => panic!("no spread written"),
        })
        .collect();
    assert_eq!(keys, ["ok", "none"]);
    assert!(matches!(&entries[0].value().kind, SpannedExprKind::ArrayLiteral(items) if items.len() == 2));
    // `null` is an identifier to the parser; the grammar gives it meaning.
    assert!(matches!(&entries[1].value().kind, SpannedExprKind::Variable(name) if name == "null"));
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

#[test]
fn object_spread_keeps_entry_order_and_operand_spans() {
    let source = "{ ...defaults, color: 'blue', ...overrides, id }";
    let spanned = parse_spanned(source).expect("spread parses");
    let SpannedExprKind::ObjectLiteral(entries) = &spanned.kind else {
        panic!("object literal, got {spanned:?}");
    };
    let shape: Vec<(&str, &str)> = entries
        .iter()
        .map(|entry| match entry {
            SpannedObjectEntry::Spread { operand } => ("spread", &source[operand.span.clone()]),
            SpannedObjectEntry::Field { key, .. } => ("field", &source[key.span.clone()]),
        })
        .collect();
    assert_eq!(shape, [("spread", "defaults"), ("field", "color"), ("spread", "overrides"), ("field", "id")]);
    // Erasure keeps the order and distinguishes a shorthand field from a spread.
    let Expr::ObjectLiteral(erased) = parse(source).unwrap() else { panic!("object literal") };
    assert_eq!(erased[0], ObjectEntry::Spread { operand: Expr::Variable("defaults".into()) });
    assert_eq!(erased[3], ObjectEntry::Field { key: "id".into(), value: Expr::Variable("id".into()) });
    assert_eq!(spanned.erase(), Expr::ObjectLiteral(erased));
    // `...` is one token with a three-byte span.
    let tokens = lex_spanned("{...a}").unwrap();
    assert_eq!((&tokens[1].value, tokens[1].span.clone()), (&Token::Ellipsis, 1..4));
}

#[test]
fn object_spread_rejects_written_duplicates_and_array_spread() {
    // A written key repeated across a spread is still a duplicate; the
    // error points at the second occurrence.
    let duplicate = parse("{ a: 1, ...rest, a: 2 }").unwrap_err();
    assert!(duplicate.message.contains("duplicate object key \"a\""), "{duplicate}");
    assert_eq!(duplicate.position, 17);
    let shorthand = parse("{ a, ...rest, a }").unwrap_err();
    assert_eq!(shorthand.position, 14, "{shorthand}");
    let array = parse("[1, ...rest]").unwrap_err();
    assert!(array.message.contains("array spread is not supported"), "{array}");
    assert_eq!(array.position, 4);
}

#[test]
fn a_spread_with_nested_objects_closes_at_the_interpolation_closer() {
    // `}}}}`: the inner object and the literal close before the `}}` delimiter.
    let text = "A {{ {...base, nested: {k: 1}}}} B {{ {...a}}} C";
    let found = scan_plain(text);
    let expressions: Vec<&str> = found.expressions.iter().map(|e| e.expression.as_str()).collect();
    assert_eq!(expressions, ["{...base, nested: {k: 1}}", "{...a}"]);
    assert_eq!(&text[found.expressions[0].end..], " B {{ {...a}}} C");
}

#[test]
fn a_catalog_example_result_rejects_spread_at_any_depth() {
    let control = parse_type_definition(
        "f",
        "function(parameters(x: unknown); returns(unknown); example(f(1) => {a: {b: 1}}))",
    );
    assert!(control.is_ok(), "{control:?}");
    for result in ["{...base}", "{a: 1, ...base}", "{a: {...base}}", "[{...base}]"] {
        let definition = format!("function(parameters(x: unknown); returns(unknown); example(f(1) => {result}))");
        let error = parse_type_definition("f", &definition).expect_err(result).to_string();
        assert!(error.contains("an example result must be a literal"), "{result}: {error}");
        assert!(error.contains("`...base` is not a literal"), "{result}: {error}");
    }
}
