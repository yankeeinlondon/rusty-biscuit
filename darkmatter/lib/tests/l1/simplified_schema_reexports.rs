//! Darkmatter's paths to the foundations that moved into `simplified-schema`
//! name the library's own types and functions (one implementation), and
//! `ExpressionFinder` hands its Markdown code regions to the library's scan,
//! and the expression syntax (tokens, parser, AST) is the library's.
//! A path that drifted to a lookalike type fails to compile this file.

use darkmatter::markdown::compose::expression::{
    self as expression, ExpressionFinder, ExpressionScanResult, LiteralTokenLocation,
};
use darkmatter::markdown::literal_token::{self, TokenError};
use darkmatter::markdown::numeric::{self, NumericAdmissionError};

#[test]
fn the_darkmatter_paths_are_the_library_types() {
    let spanned: simplified_schema::Spanned<&str> = darkmatter::Spanned::new("v", 1..2);
    let back: darkmatter::markdown::span::Spanned<&str> = spanned;
    assert_eq!(back.span, 1..2);

    let error: simplified_schema::numeric::NumericAdmissionError = NumericAdmissionError::Inexact;
    assert_eq!(numeric::admit_integer((1 << 53) + 1), Err(error));

    let token_error: simplified_schema::pending::TokenError = TokenError::Embedded;
    assert_eq!(literal_token::decode_leaf("x {{!data:v1:YQ}}"), Some(Err(token_error)));

    let scan: simplified_schema::expression::scan::ExpressionScanResult = ExpressionFinder::scan_plain("{{!data:v1:}}");
    let scan: ExpressionScanResult = scan;
    let location: simplified_schema::expression::scan::LiteralTokenLocation = LiteralTokenLocation { start: 0, end: 13 };
    assert_eq!(scan.tokens, vec![location]);
}

#[test]
fn the_finder_skips_code_blocks_and_otherwise_matches_the_plain_scan() {
    let content = "Hi {{ a }}\r\n\r\n```\r\n{{ b }}\r\n```\r\n\r\n`{{ c }}` {{!data:v1:YQ}}\r\n";
    let finder = ExpressionFinder::new(content).scan();
    let plain = simplified_schema::expression::scan::scan_plain(content);

    let names = |result: &ExpressionScanResult| -> Vec<String> {
        result.expressions.iter().map(|found| found.expression.clone()).collect()
    };
    assert_eq!(names(&plain), vec!["a", "b", "c"]);
    // The fenced block is excluded; the inline code span is still scanned.
    assert_eq!(names(&finder), vec!["a", "c"]);
    assert_eq!(finder.tokens, plain.tokens);
    for found in &finder.expressions {
        assert!(content[found.start..found.end].starts_with("{{"), "{found:?}");
    }
}

#[test]
fn the_darkmatter_expression_syntax_paths_are_the_library_parser() {
    let parsed: simplified_schema::expression::Expr = expression::parse("unknown_fn(1)").unwrap();
    assert_eq!(parsed, simplified_schema::expression::parse("unknown_fn(1)").unwrap());
    let spanned: simplified_schema::expression::SpannedExpr = expression::ast::SpannedExpr::clone(
        &expression::parser::parse_spanned("a || b").unwrap(),
    );
    assert_eq!(spanned.erase(), expression::parse("a || b").unwrap());
    let error: simplified_schema::expression::ParseError = expression::parse_condition("((").unwrap_err();
    assert_eq!(error.position, 2);
    let token: simplified_schema::expression::Token = expression::lexer::Token::Pipe;
    let tokens = expression::Lexer::new("a || b").tokenize_all().unwrap();
    assert_eq!(tokens[1], token);
    let mode: simplified_schema::expression::ParseMode = expression::ParseMode::Condition;
    let lexed: Vec<simplified_schema::Spanned<simplified_schema::expression::Token>> = expression::lex_spanned("a || b", mode).unwrap();
    assert_eq!(lexed[1].value, simplified_schema::expression::Token::OrOr);
}
