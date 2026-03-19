//! Parser for interpolation expressions.
//!
//! This module provides a recursive descent parser that converts tokens
//! from the lexer into an AST. The parser implements the following grammar:
//!
//! ```text
//! expression     = ternary
//! ternary        = fallback ("?" fallback ":" fallback)?
//! fallback       = comparison ("|" comparison)*
//! comparison     = unary (comp_op unary)?
//! unary          = "!" unary | primary
//! primary        = literal | variable | function_call | "(" expression ")"
//! function_call  = variable "(" args? ")"
//! args           = expression ("," expression)*
//! literal        = STRING | NUMBER
//! ```
//!
//! ## Operator Precedence
//!
//! Precedence from highest to lowest:
//! 1. **Function calls** - `length(x)`, `number(x, 0)`
//! 2. **Unary NOT** - `!x`
//! 3. **Comparison** - `==`, `!=`, `>`, `>=`, `<`
//! 4. **Fallback** - `|`
//! 5. **Ternary** - `? :`
//!
//! ## Examples
//!
//! ```
//! use darkmatter::markdown::compose::interpolation::{parse, Expr};
//!
//! // Simple variable
//! let expr = parse("foo").unwrap();
//! assert!(matches!(expr, Expr::Variable(name) if name == "foo"));
//!
//! // Fallback
//! let expr = parse(r#"foo | "default""#).unwrap();
//! assert!(matches!(expr, Expr::Fallback { .. }));
//!
//! // Ternary
//! let expr = parse(r#"x ? "yes" : "no""#).unwrap();
//! assert!(matches!(expr, Expr::Ternary { .. }));
//! ```

use super::{Lexer, LexerError, Token, ast::Expr};
use std::fmt;

#[cfg(test)]
use super::ComparisonOp;

/// Error that can occur during parsing.
#[derive(Debug, Clone, PartialEq)]
pub struct ParseError {
    /// Human-readable error message.
    pub message: String,

    /// Approximate byte position in the input where the error occurred.
    pub position: usize,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} at position {}", self.message, self.position)
    }
}

impl std::error::Error for ParseError {}

impl ParseError {
    /// Creates a new parse error.
    fn new(message: impl Into<String>, position: usize) -> Self {
        Self {
            message: message.into(),
            position,
        }
    }

    /// Creates a parse error for an unexpected token.
    fn unexpected(expected: &str, found: &Token, position: usize) -> Self {
        Self::new(
            format!("Expected {}, found '{}'", expected, found),
            position,
        )
    }
}

impl From<LexerError> for ParseError {
    fn from(err: LexerError) -> Self {
        ParseError {
            message: err.message,
            position: err.position,
        }
    }
}

/// Parser for interpolation expressions.
///
/// Converts a stream of tokens into an AST using recursive descent parsing.
pub struct Parser<'a> {
    lexer: Lexer<'a>,
    current: Token,
    /// Approximate position for error reporting.
    position: usize,
}

impl<'a> Parser<'a> {
    /// Creates a new parser for the given expression.
    ///
    /// ## Errors
    ///
    /// Returns an error if the lexer fails on the first token.
    pub fn new(input: &'a str) -> Result<Self, ParseError> {
        let mut lexer = Lexer::new(input);
        let current = lexer.next_token()?;
        Ok(Self {
            lexer,
            current,
            position: 0,
        })
    }

    /// Parses the expression and returns the AST.
    ///
    /// ## Errors
    ///
    /// Returns an error for invalid syntax.
    pub fn parse(&mut self) -> Result<Expr, ParseError> {
        let expr = self.parse_expression()?;

        // Ensure we consumed all input
        if !matches!(self.current, Token::Eof) {
            return Err(ParseError::unexpected(
                "end of expression",
                &self.current,
                self.position,
            ));
        }

        Ok(expr)
    }

    /// Advances to the next token.
    fn advance(&mut self) -> Result<Token, ParseError> {
        let prev = std::mem::replace(&mut self.current, Token::Eof);
        self.current = self.lexer.next_token()?;
        self.position += 1;
        Ok(prev)
    }

    /// Checks if the current token matches the expected token.
    fn check(&self, expected: &Token) -> bool {
        std::mem::discriminant(&self.current) == std::mem::discriminant(expected)
    }

    /// Consumes the current token if it matches, otherwise returns an error.
    fn expect(&mut self, expected: &Token, description: &str) -> Result<Token, ParseError> {
        if self.check(expected) {
            self.advance()
        } else {
            Err(ParseError::unexpected(
                description,
                &self.current,
                self.position,
            ))
        }
    }

    /// Parses the top-level expression (ternary has lowest precedence).
    fn parse_expression(&mut self) -> Result<Expr, ParseError> {
        self.parse_ternary()
    }

    /// Parses a ternary expression: `fallback ("?" fallback ":" fallback)?`
    fn parse_ternary(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_fallback()?;

        if matches!(self.current, Token::Question) {
            self.advance()?; // consume ?
            let then_branch = self.parse_fallback()?;

            self.expect(&Token::Colon, "':'")?;
            let else_branch = self.parse_fallback()?;

            expr = Expr::Ternary {
                condition: Box::new(expr),
                then_branch: Box::new(then_branch),
                else_branch: Box::new(else_branch),
            };
        }

        Ok(expr)
    }

    /// Parses a fallback expression: `comparison ("|" comparison)*`
    fn parse_fallback(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_comparison()?;

        while matches!(self.current, Token::Pipe) {
            self.advance()?; // consume |
            let fallback = self.parse_comparison()?;
            expr = Expr::Fallback {
                primary: Box::new(expr),
                fallback: Box::new(fallback),
            };
        }

        Ok(expr)
    }

    /// Parses a comparison expression: `unary (comp_op unary)?`
    fn parse_comparison(&mut self) -> Result<Expr, ParseError> {
        let left = self.parse_unary()?;

        if let Token::CompOp(op) = self.current {
            self.advance()?; // consume operator
            let right = self.parse_unary()?;
            return Ok(Expr::Comparison {
                left: Box::new(left),
                op,
                right: Box::new(right),
            });
        }

        Ok(left)
    }

    /// Parses unary expressions: `"!" unary | primary`.
    fn parse_unary(&mut self) -> Result<Expr, ParseError> {
        if matches!(self.current, Token::Bang) {
            self.advance()?;
            let expr = self.parse_unary()?;
            return Ok(Expr::UnaryNot(Box::new(expr)));
        }
        self.parse_primary()
    }

    /// Parses a primary expression: literal, variable, function call, or parenthesized expression.
    fn parse_primary(&mut self) -> Result<Expr, ParseError> {
        match &self.current {
            Token::StringLiteral(s) => {
                let value = s.clone();
                self.advance()?;
                Ok(Expr::StringLiteral(value))
            }
            Token::NumberLiteral(n) => {
                let value = *n;
                self.advance()?;
                Ok(Expr::NumberLiteral(value))
            }
            Token::Variable(name) => {
                let name = name.clone();
                self.advance()?;

                // Check for function call
                if matches!(self.current, Token::LParen) {
                    self.parse_function_call(name)
                } else {
                    Ok(Expr::Variable(name))
                }
            }
            Token::LParen => {
                self.advance()?; // consume (
                let expr = self.parse_expression()?;
                self.expect(&Token::RParen, "')'")?;
                Ok(expr)
            }
            _ => Err(ParseError::unexpected(
                "expression",
                &self.current,
                self.position,
            )),
        }
    }

    /// Parses a function call: `name "(" args? ")"`
    ///
    /// The function name has already been consumed.
    fn parse_function_call(&mut self, name: String) -> Result<Expr, ParseError> {
        self.advance()?; // consume (

        let mut args = Vec::new();

        // Handle empty args
        if !matches!(self.current, Token::RParen) {
            args.push(self.parse_expression()?);

            while matches!(self.current, Token::Comma) {
                self.advance()?; // consume ,
                args.push(self.parse_expression()?);
            }
        }

        self.expect(&Token::RParen, "')'")?;
        Ok(Expr::FunctionCall { name, args })
    }
}

/// Convenience function to parse an expression string.
///
/// ## Examples
///
/// ```
/// use darkmatter::markdown::compose::interpolation::{parse, Expr};
///
/// let expr = parse("foo | \"default\"").unwrap();
/// assert!(matches!(expr, Expr::Fallback { .. }));
/// ```
///
/// ## Errors
///
/// Returns an error for invalid syntax.
pub fn parse(input: &str) -> Result<Expr, ParseError> {
    Parser::new(input)?.parse()
}

#[cfg(test)]
mod tests {
    use super::*;

    mod simple_expressions {
        use super::*;

        #[test]
        fn parses_simple_variable() {
            let expr = parse("foo").unwrap();
            assert!(matches!(expr, Expr::Variable(name) if name == "foo"));
        }

        #[test]
        fn parses_dotted_variable() {
            let expr = parse("user.name").unwrap();
            assert!(matches!(expr, Expr::Variable(name) if name == "user.name"));
        }

        #[test]
        fn parses_context_variable() {
            let expr = parse("ctx.today").unwrap();
            assert!(matches!(expr, Expr::Variable(name) if name == "ctx.today"));
        }

        #[test]
        fn parses_env_variable() {
            let expr = parse("env.HOME").unwrap();
            assert!(matches!(expr, Expr::Variable(name) if name == "env.HOME"));
        }

        #[test]
        fn parses_unary_not() {
            let expr = parse("!missing").unwrap();
            assert!(matches!(expr, Expr::UnaryNot(_)));
        }

        #[test]
        fn parses_string_literal() {
            let expr = parse(r#""hello world""#).unwrap();
            assert!(matches!(expr, Expr::StringLiteral(s) if s == "hello world"));
        }

        #[test]
        fn parses_single_quoted_string() {
            let expr = parse("'hello'").unwrap();
            assert!(matches!(expr, Expr::StringLiteral(s) if s == "hello"));
        }

        #[test]
        fn parses_integer() {
            let expr = parse("42").unwrap();
            assert!(matches!(expr, Expr::NumberLiteral(n) if n == 42.0));
        }

        #[test]
        fn parses_float() {
            let expr = parse("3.14").unwrap();
            match expr {
                Expr::NumberLiteral(n) => assert!((n - 3.14).abs() < f64::EPSILON),
                _ => panic!("Expected NumberLiteral"),
            }
        }

        #[test]
        fn parses_negative_number() {
            let expr = parse("-42").unwrap();
            assert!(matches!(expr, Expr::NumberLiteral(n) if n == -42.0));
        }
    }

    mod fallback_expressions {
        use super::*;

        #[test]
        fn parses_fallback() {
            let expr = parse(r#"foo | "default""#).unwrap();
            match expr {
                Expr::Fallback { primary, fallback } => {
                    assert!(matches!(*primary, Expr::Variable(ref n) if n == "foo"));
                    assert!(matches!(*fallback, Expr::StringLiteral(ref s) if s == "default"));
                }
                _ => panic!("Expected Fallback"),
            }
        }

        #[test]
        fn parses_fallback_with_variable() {
            let expr = parse("foo | bar").unwrap();
            match expr {
                Expr::Fallback { primary, fallback } => {
                    assert!(matches!(*primary, Expr::Variable(ref n) if n == "foo"));
                    assert!(matches!(*fallback, Expr::Variable(ref n) if n == "bar"));
                }
                _ => panic!("Expected Fallback"),
            }
        }

        #[test]
        fn parses_chained_fallback() {
            // foo | bar | baz parses as ((foo | bar) | baz)
            let expr = parse("foo | bar | baz").unwrap();
            match expr {
                Expr::Fallback { primary, fallback } => {
                    // The fallback is baz
                    assert!(matches!(*fallback, Expr::Variable(ref n) if n == "baz"));
                    // The primary is (foo | bar)
                    match *primary {
                        Expr::Fallback {
                            primary: p2,
                            fallback: f2,
                        } => {
                            assert!(matches!(*p2, Expr::Variable(ref n) if n == "foo"));
                            assert!(matches!(*f2, Expr::Variable(ref n) if n == "bar"));
                        }
                        _ => panic!("Expected nested Fallback"),
                    }
                }
                _ => panic!("Expected Fallback"),
            }
        }

        #[test]
        fn parses_env_fallback() {
            let expr = parse(r#"env.FAVORITE_COLOR | "unknown""#).unwrap();
            match expr {
                Expr::Fallback { primary, fallback } => {
                    assert!(matches!(*primary, Expr::Variable(ref n) if n == "env.FAVORITE_COLOR"));
                    assert!(matches!(*fallback, Expr::StringLiteral(ref s) if s == "unknown"));
                }
                _ => panic!("Expected Fallback"),
            }
        }
    }

    mod ternary_expressions {
        use super::*;

        #[test]
        fn parses_ternary() {
            let expr = parse(r#"x ? "yes" : "no""#).unwrap();
            assert!(matches!(expr, Expr::Ternary { .. }));
        }

        #[test]
        fn parses_ternary_details() {
            let expr = parse(r#"active ? "on" : "off""#).unwrap();
            match expr {
                Expr::Ternary {
                    condition,
                    then_branch,
                    else_branch,
                } => {
                    assert!(matches!(*condition, Expr::Variable(ref n) if n == "active"));
                    assert!(matches!(*then_branch, Expr::StringLiteral(ref s) if s == "on"));
                    assert!(matches!(*else_branch, Expr::StringLiteral(ref s) if s == "off"));
                }
                _ => panic!("Expected Ternary"),
            }
        }

        #[test]
        fn parses_ternary_with_variables() {
            let expr = parse("x ? y : z").unwrap();
            match expr {
                Expr::Ternary {
                    condition,
                    then_branch,
                    else_branch,
                } => {
                    assert!(matches!(*condition, Expr::Variable(ref n) if n == "x"));
                    assert!(matches!(*then_branch, Expr::Variable(ref n) if n == "y"));
                    assert!(matches!(*else_branch, Expr::Variable(ref n) if n == "z"));
                }
                _ => panic!("Expected Ternary"),
            }
        }

        #[test]
        fn parses_ternary_with_fallback_in_branches() {
            // x ? (a | b) : (c | d)
            let expr = parse(r#"x ? a | b : c | d"#).unwrap();
            match expr {
                Expr::Ternary {
                    condition,
                    then_branch,
                    else_branch,
                } => {
                    assert!(matches!(*condition, Expr::Variable(ref n) if n == "x"));
                    assert!(matches!(*then_branch, Expr::Fallback { .. }));
                    assert!(matches!(*else_branch, Expr::Fallback { .. }));
                }
                _ => panic!("Expected Ternary"),
            }
        }
    }

    mod comparison_expressions {
        use super::*;

        #[test]
        fn parses_equality() {
            let expr = parse("a == b").unwrap();
            match expr {
                Expr::Comparison { left, op, right } => {
                    assert!(matches!(*left, Expr::Variable(ref n) if n == "a"));
                    assert_eq!(op, ComparisonOp::Equal);
                    assert!(matches!(*right, Expr::Variable(ref n) if n == "b"));
                }
                _ => panic!("Expected Comparison"),
            }
        }

        #[test]
        fn parses_inequality() {
            let expr = parse("a != b").unwrap();
            match expr {
                Expr::Comparison { op, .. } => {
                    assert_eq!(op, ComparisonOp::NotEqual);
                }
                _ => panic!("Expected Comparison"),
            }
        }

        #[test]
        fn parses_greater_than() {
            let expr = parse("count > 0").unwrap();
            match expr {
                Expr::Comparison { left, op, right } => {
                    assert!(matches!(*left, Expr::Variable(ref n) if n == "count"));
                    assert_eq!(op, ComparisonOp::GreaterThan);
                    assert!(matches!(*right, Expr::NumberLiteral(n) if n == 0.0));
                }
                _ => panic!("Expected Comparison"),
            }
        }

        #[test]
        fn parses_greater_than_or_equal() {
            let expr = parse("x >= y").unwrap();
            match expr {
                Expr::Comparison { op, .. } => {
                    assert_eq!(op, ComparisonOp::GreaterThanOrEqual);
                }
                _ => panic!("Expected Comparison"),
            }
        }

        #[test]
        fn parses_less_than() {
            let expr = parse("a < b").unwrap();
            match expr {
                Expr::Comparison { op, .. } => {
                    assert_eq!(op, ComparisonOp::LessThan);
                }
                _ => panic!("Expected Comparison"),
            }
        }

        #[test]
        fn parses_comparison_with_string() {
            let expr = parse(r#"color == "red""#).unwrap();
            match expr {
                Expr::Comparison { left, op, right } => {
                    assert!(matches!(*left, Expr::Variable(ref n) if n == "color"));
                    assert_eq!(op, ComparisonOp::Equal);
                    assert!(matches!(*right, Expr::StringLiteral(ref s) if s == "red"));
                }
                _ => panic!("Expected Comparison"),
            }
        }
    }

    mod combined_expressions {
        use super::*;

        #[test]
        fn parses_comparison_ternary() {
            let expr = parse(r#"a == b ? "equal" : "different""#).unwrap();
            match expr {
                Expr::Ternary {
                    condition,
                    then_branch,
                    else_branch,
                } => {
                    assert!(matches!(*condition, Expr::Comparison { .. }));
                    assert!(matches!(*then_branch, Expr::StringLiteral(ref s) if s == "equal"));
                    assert!(matches!(*else_branch, Expr::StringLiteral(ref s) if s == "different"));
                }
                _ => panic!("Expected Ternary"),
            }
        }

        #[test]
        fn parses_count_ternary() {
            let expr = parse(r#"count > 0 ? "has items" : "empty""#).unwrap();
            match expr {
                Expr::Ternary {
                    condition,
                    then_branch,
                    else_branch,
                } => {
                    match *condition {
                        Expr::Comparison { left, op, right } => {
                            assert!(matches!(*left, Expr::Variable(ref n) if n == "count"));
                            assert_eq!(op, ComparisonOp::GreaterThan);
                            assert!(matches!(*right, Expr::NumberLiteral(n) if n == 0.0));
                        }
                        _ => panic!("Expected Comparison in condition"),
                    }
                    assert!(matches!(*then_branch, Expr::StringLiteral(ref s) if s == "has items"));
                    assert!(matches!(*else_branch, Expr::StringLiteral(ref s) if s == "empty"));
                }
                _ => panic!("Expected Ternary"),
            }
        }

        #[test]
        fn parses_complex_nested() {
            // Test: foo | bar ? "truthy" : "falsy"
            // This should parse as: (foo | bar) ? "truthy" : "falsy"
            // But actually per precedence: foo | (bar ? "truthy" : "falsy")
            // Wait, ternary has lowest precedence, so:
            // ternary parses fallback first, which consumes foo | bar
            // then sees ?, so the condition is (foo | bar)
            let expr = parse(r#"foo | bar ? "truthy" : "falsy""#).unwrap();
            match expr {
                Expr::Ternary {
                    condition,
                    then_branch,
                    else_branch,
                } => {
                    // Condition should be foo | bar
                    assert!(matches!(*condition, Expr::Fallback { .. }));
                    assert!(matches!(*then_branch, Expr::StringLiteral(ref s) if s == "truthy"));
                    assert!(matches!(*else_branch, Expr::StringLiteral(ref s) if s == "falsy"));
                }
                _ => panic!("Expected Ternary"),
            }
        }
    }

    mod function_calls {
        use super::*;

        #[test]
        fn parses_function_call_no_args() {
            let expr = parse("now()").unwrap();
            match expr {
                Expr::FunctionCall { name, args } => {
                    assert_eq!(name, "now");
                    assert!(args.is_empty());
                }
                _ => panic!("Expected FunctionCall"),
            }
        }

        #[test]
        fn parses_function_call_one_arg() {
            let expr = parse("length(items)").unwrap();
            match expr {
                Expr::FunctionCall { name, args } => {
                    assert_eq!(name, "length");
                    assert_eq!(args.len(), 1);
                    assert!(matches!(&args[0], Expr::Variable(n) if n == "items"));
                }
                _ => panic!("Expected FunctionCall"),
            }
        }

        #[test]
        fn parses_function_call_multiple_args() {
            let expr = parse("number(value, 0)").unwrap();
            match expr {
                Expr::FunctionCall { name, args } => {
                    assert_eq!(name, "number");
                    assert_eq!(args.len(), 2);
                    assert!(matches!(&args[0], Expr::Variable(n) if n == "value"));
                    assert!(matches!(&args[1], Expr::NumberLiteral(n) if *n == 0.0));
                }
                _ => panic!("Expected FunctionCall"),
            }
        }

        #[test]
        fn parses_function_with_string_arg() {
            let expr = parse(r#"format(date, "YYYY-MM-DD")"#).unwrap();
            match expr {
                Expr::FunctionCall { name, args } => {
                    assert_eq!(name, "format");
                    assert_eq!(args.len(), 2);
                    assert!(matches!(&args[0], Expr::Variable(n) if n == "date"));
                    assert!(matches!(&args[1], Expr::StringLiteral(s) if s == "YYYY-MM-DD"));
                }
                _ => panic!("Expected FunctionCall"),
            }
        }

        #[test]
        fn parses_nested_function_call() {
            let expr = parse("upper(lower(text))").unwrap();
            match expr {
                Expr::FunctionCall { name, args } => {
                    assert_eq!(name, "upper");
                    assert_eq!(args.len(), 1);
                    match &args[0] {
                        Expr::FunctionCall { name: inner, args } => {
                            assert_eq!(inner, "lower");
                            assert_eq!(args.len(), 1);
                        }
                        _ => panic!("Expected nested FunctionCall"),
                    }
                }
                _ => panic!("Expected FunctionCall"),
            }
        }

        #[test]
        fn parses_function_with_expression_arg() {
            // length(items | defaults)
            let expr = parse("length(items | defaults)").unwrap();
            match expr {
                Expr::FunctionCall { name, args } => {
                    assert_eq!(name, "length");
                    assert_eq!(args.len(), 1);
                    assert!(matches!(&args[0], Expr::Fallback { .. }));
                }
                _ => panic!("Expected FunctionCall"),
            }
        }
    }

    mod parentheses {
        use super::*;

        #[test]
        fn parses_parenthesized_expression() {
            let expr = parse("(foo)").unwrap();
            assert!(matches!(expr, Expr::Variable(name) if name == "foo"));
        }

        #[test]
        fn parses_nested_parentheses() {
            let expr = parse("((foo))").unwrap();
            assert!(matches!(expr, Expr::Variable(name) if name == "foo"));
        }

        #[test]
        fn parses_parenthesized_fallback() {
            let expr = parse("(foo | bar)").unwrap();
            assert!(matches!(expr, Expr::Fallback { .. }));
        }

        #[test]
        fn parses_parentheses_in_ternary() {
            // (a | b) ? c : d
            let expr = parse("(a | b) ? c : d").unwrap();
            match expr {
                Expr::Ternary { condition, .. } => {
                    assert!(matches!(*condition, Expr::Fallback { .. }));
                }
                _ => panic!("Expected Ternary"),
            }
        }
    }

    mod error_cases {
        use super::*;

        #[test]
        fn error_empty_input() {
            let result = parse("");
            assert!(result.is_err());
        }

        #[test]
        fn error_unclosed_paren() {
            let result = parse("(foo");
            assert!(result.is_err());
            let err = result.unwrap_err();
            assert!(err.message.contains("')'"));
        }

        #[test]
        fn error_extra_paren() {
            let result = parse("foo)");
            assert!(result.is_err());
        }

        #[test]
        fn error_missing_ternary_colon() {
            let result = parse(r#"x ? "yes""#);
            assert!(result.is_err());
            let err = result.unwrap_err();
            assert!(err.message.contains("':'"));
        }

        #[test]
        fn error_missing_ternary_else() {
            let result = parse("x ? y :");
            assert!(result.is_err());
        }

        #[test]
        fn error_trailing_pipe() {
            let result = parse("foo |");
            assert!(result.is_err());
        }

        #[test]
        fn error_leading_pipe() {
            let result = parse("| foo");
            assert!(result.is_err());
        }

        #[test]
        fn error_unclosed_function() {
            let result = parse("length(foo");
            assert!(result.is_err());
            let err = result.unwrap_err();
            assert!(err.message.contains("')'"));
        }

        #[test]
        fn error_lexer_propagation() {
            // Test that lexer errors are properly converted
            let result = parse(r#""unclosed"#);
            assert!(result.is_err());
            let err = result.unwrap_err();
            assert!(err.message.contains("Unterminated"));
        }

        #[test]
        fn error_invalid_token() {
            let result = parse("@invalid");
            assert!(result.is_err());
        }

        #[test]
        fn error_trailing_content() {
            let result = parse("foo bar");
            assert!(result.is_err());
            let err = result.unwrap_err();
            assert!(err.message.contains("end of expression"));
        }
    }

    mod parse_error_display {
        use super::*;

        #[test]
        fn error_display() {
            let err = ParseError::new("Test error", 5);
            assert_eq!(err.to_string(), "Test error at position 5");
        }

        #[test]
        fn error_unexpected() {
            let err = ParseError::unexpected("expression", &Token::Pipe, 3);
            assert_eq!(
                err.to_string(),
                "Expected expression, found '|' at position 3"
            );
        }
    }

    mod roundtrip {
        use super::*;

        #[test]
        fn roundtrip_simple_variable() {
            let input = "foo";
            let expr = parse(input).unwrap();
            assert_eq!(expr.to_string(), "foo");
        }

        #[test]
        fn roundtrip_fallback() {
            let expr = parse(r#"foo | "default""#).unwrap();
            assert_eq!(expr.to_string(), "foo | \"default\"");
        }

        #[test]
        fn roundtrip_ternary() {
            let expr = parse(r#"x ? "yes" : "no""#).unwrap();
            assert_eq!(expr.to_string(), "x ? \"yes\" : \"no\"");
        }

        #[test]
        fn roundtrip_comparison() {
            let expr = parse("count > 0").unwrap();
            assert_eq!(expr.to_string(), "count > 0");
        }

        #[test]
        fn roundtrip_function() {
            let expr = parse("length(items)").unwrap();
            assert_eq!(expr.to_string(), "length(items)");
        }
    }
}
