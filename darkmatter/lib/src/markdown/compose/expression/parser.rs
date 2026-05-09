//! Parser for interpolation expressions.
//!
//! This module provides a recursive descent parser that converts tokens
//! from the lexer into an AST. The parser implements the following grammar:
//!
//! ```text
//! expression     = ternary
//! ternary        = ternary_branch ("?" ternary ":" ternary)?
//! ternary_branch = fallback                       (interpolation mode)
//!                | logical_or                     (condition mode)
//! logical_or     = logical_and ("||" logical_and)*
//! fallback       = logical_and ("||" logical_and)*
//! logical_and    = comparison ("&&" comparison)*  (condition mode only)
//! comparison     = additive (comp_op additive)?
//! additive       = multiplicative (("+" | "-") multiplicative)*
//! multiplicative = unary (("*" | "/" | "%") unary)*
//! unary          = "!" unary | "-" unary | postfix
//! postfix        = primary ( "[" expression "]" | "." IDENT )*
//! primary        = literal | variable | function_call | "(" expression ")"
//! function_call  = variable "(" args? ")"
//! args           = expression ("," expression)*
//! literal        = STRING | NUMBER | BOOL
//! ```
//!
//! ## Operator Precedence
//!
//! Precedence from highest to lowest:
//! 1. **Primary / postfix access** - literals, variables, function calls, `foo[0]`, `foo.bar`, `(expr)`
//! 2. **Unary** - `!x`, `-x`
//! 3. **Multiplicative** - `*`, `/`, `%`
//! 4. **Additive** - `+`, `-`
//! 5. **Comparison** - `==`, `!=`, `>`, `>=`, `<`, `<=`
//! 6. **Logical AND** - `&&` (condition mode)
//! 7. **Logical OR / Fallback** - `||`
//! 8. **Ternary** - `? :` (right-associative)
//!
//! ## Examples
//!
//! ```
//! use darkmatter::markdown::compose::expression::{parse, Expr};
//!
//! // Simple variable
//! let expr = parse("foo").unwrap();
//! assert!(matches!(expr, Expr::Variable(name) if name == "foo"));
//!
//! // Fallback
//! let expr = parse(r#"foo || "default""#).unwrap();
//! assert!(matches!(expr, Expr::Fallback { .. }));
//!
//! // Ternary
//! let expr = parse(r#"x ? "yes" : "no""#).unwrap();
//! assert!(matches!(expr, Expr::Ternary { .. }));
//!
//! // Nested ternary (right-associative)
//! let expr = parse(r#"a ? b ? c : d : e"#).unwrap();
//! assert!(matches!(expr, Expr::Ternary { .. }));
//! ```

use super::{
    Lexer, LexerError, ParseMode, Token,
    ast::{BinaryOp, Expr},
};
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
    mode: ParseMode,
}

impl<'a> Parser<'a> {
    /// Creates a new parser for the given expression in interpolation mode.
    ///
    /// ## Errors
    ///
    /// Returns an error if the lexer fails on the first token.
    pub fn new(input: &'a str) -> Result<Self, ParseError> {
        Self::with_mode(input, ParseMode::Interpolation)
    }

    /// Creates a new parser for the given expression with a specific parse mode.
    ///
    /// Condition mode enables `&&` and `||` as logical operators following the
    /// condition-mode precedence ladder. Interpolation mode keeps the default
    /// interpolation grammar.
    ///
    /// ## Errors
    ///
    /// Returns an error if the lexer fails on the first token.
    pub fn with_mode(input: &'a str, mode: ParseMode) -> Result<Self, ParseError> {
        let mut lexer = Lexer::with_mode(input, mode);
        let current = lexer.next_token()?;
        Ok(Self {
            lexer,
            current,
            position: 0,
            mode,
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

    /// Parses a ternary expression.
    ///
    /// In interpolation mode: `fallback ("?" ternary ":" ternary)?`
    ///
    /// In condition mode: `logical_or ("?" ternary ":" ternary)?`
    ///
    /// Branches are parsed recursively so nested ternaries are supported
    /// without extra parentheses: `a ? b ? c : d : e`.
    fn parse_ternary(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_ternary_branch()?;

        if matches!(self.current, Token::Question) {
            self.advance()?; // consume ?
            let then_branch = self.parse_ternary()?;

            self.expect(&Token::Colon, "':'")?;
            let else_branch = self.parse_ternary()?;

            expr = Expr::Ternary {
                condition: Box::new(expr),
                then_branch: Box::new(then_branch),
                else_branch: Box::new(else_branch),
            };
        }

        Ok(expr)
    }

    /// Parses a ternary branch — the level immediately below ternary.
    ///
    /// This is `logical_or` in condition mode and `fallback` in interpolation mode.
    fn parse_ternary_branch(&mut self) -> Result<Expr, ParseError> {
        match self.mode {
            ParseMode::Condition => self.parse_logical_or(),
            ParseMode::Interpolation => self.parse_fallback(),
        }
    }

    /// Parses a logical OR expression (condition mode only):
    /// `logical_and ("||" logical_and)*`.
    ///
    /// Infix `a || b` is lowered into the existing function-call AST as
    /// `Or(a, b)` so downstream evaluation and AST consumers do not need to
    /// learn a new variant.
    fn parse_logical_or(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_logical_and()?;

        while matches!(self.current, Token::OrOr) {
            self.advance()?; // consume ||
            let rhs = self.parse_logical_and()?;
            expr = Expr::FunctionCall {
                name: "Or".to_string(),
                args: vec![expr, rhs],
            };
        }

        Ok(expr)
    }

    /// Parses a logical AND expression (condition mode only):
    /// `comparison ("&&" comparison)*`.
    ///
    /// Infix `a && b` is lowered into the existing function-call AST as
    /// `And(a, b)`.
    fn parse_logical_and(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_comparison()?;

        while matches!(self.current, Token::AndAnd) {
            self.advance()?; // consume &&
            let rhs = self.parse_comparison()?;
            expr = Expr::FunctionCall {
                name: "And".to_string(),
                args: vec![expr, rhs],
            };
        }

        Ok(expr)
    }

    /// Parses a fallback expression: `logical_and ("||" logical_and)*`.
    ///
    /// `logical_and` is shared with condition mode so the comparison ladder
    /// behaves identically across both parse modes.
    fn parse_fallback(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_logical_and()?;

        while matches!(self.current, Token::Pipe) {
            self.advance()?; // consume ||
            let fallback = self.parse_logical_and()?;
            expr = Expr::Fallback {
                primary: Box::new(expr),
                fallback: Box::new(fallback),
            };
        }

        Ok(expr)
    }

    /// Parses a comparison expression: `additive (comp_op additive)?`.
    fn parse_comparison(&mut self) -> Result<Expr, ParseError> {
        let left = self.parse_additive()?;

        if let Token::CompOp(op) = self.current {
            self.advance()?; // consume operator
            let right = self.parse_additive()?;
            return Ok(Expr::Comparison {
                left: Box::new(left),
                op,
                right: Box::new(right),
            });
        }

        Ok(left)
    }

    /// Parses an additive expression: `multiplicative (("+" | "-") multiplicative)*`.
    ///
    /// Left-associative — `a - b - c` parses as `(a - b) - c`.
    fn parse_additive(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_multiplicative()?;

        loop {
            let op = match self.current {
                Token::Plus => BinaryOp::Add,
                Token::Minus => BinaryOp::Sub,
                _ => break,
            };
            self.advance()?; // consume operator
            let rhs = self.parse_multiplicative()?;
            expr = Expr::Binary {
                op,
                left: Box::new(expr),
                right: Box::new(rhs),
            };
        }

        Ok(expr)
    }

    /// Parses a multiplicative expression: `unary (("*" | "/" | "%") unary)*`.
    ///
    /// Left-associative — `a / b / c` parses as `(a / b) / c`.
    fn parse_multiplicative(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_unary()?;

        loop {
            let op = match self.current {
                Token::Star => BinaryOp::Mul,
                Token::Slash => BinaryOp::Div,
                Token::Percent => BinaryOp::Mod,
                _ => break,
            };
            self.advance()?; // consume operator
            let rhs = self.parse_unary()?;
            expr = Expr::Binary {
                op,
                left: Box::new(expr),
                right: Box::new(rhs),
            };
        }

        Ok(expr)
    }

    /// Parses unary expressions: `"!" unary | "-" unary | postfix`.
    fn parse_unary(&mut self) -> Result<Expr, ParseError> {
        if matches!(self.current, Token::Bang) {
            self.advance()?;
            let expr = self.parse_unary()?;
            return Ok(Expr::UnaryNot(Box::new(expr)));
        }
        if matches!(self.current, Token::Minus) {
            self.advance()?;
            let expr = self.parse_unary()?;
            return Ok(Expr::UnaryMinus(Box::new(expr)));
        }
        self.parse_postfix()
    }

    /// Parses postfix access chains: `primary ( "[" expression "]" | "." IDENT )*`.
    ///
    /// Bracket access supports any expression as the index (numbers, negative
    /// numbers via unary minus, string literals, or computed indexes). Postfix
    /// dot is only emitted by the lexer when the dot cannot be folded into a
    /// `Variable` token, so it appears after `]`, `)`, or function-call return.
    fn parse_postfix(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_primary()?;

        loop {
            match &self.current {
                Token::LBracket => {
                    self.advance()?; // consume [
                    let index = self.parse_expression()?;
                    self.expect(&Token::RBracket, "']'")?;
                    expr = Expr::Index {
                        base: Box::new(expr),
                        index: Box::new(index),
                    };
                }
                Token::Dot => {
                    self.advance()?; // consume .
                    match self.current.clone() {
                        Token::Variable(name) => {
                            self.advance()?;
                            expr = Expr::MemberAccess {
                                base: Box::new(expr),
                                name,
                            };
                        }
                        Token::NumberLiteral(_) => {
                            return Err(ParseError::new(
                                "Numeric dot access is not supported (use bracket indexing for arrays)",
                                self.position,
                            ));
                        }
                        other => {
                            return Err(ParseError::unexpected(
                                "identifier after '.'",
                                &other,
                                self.position,
                            ));
                        }
                    }
                }
                _ => break,
            }
        }

        Ok(expr)
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
            Token::BoolLiteral(b) => {
                let value = *b;
                self.advance()?;
                Ok(Expr::BoolLiteral(value))
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
                Ok(Expr::Paren(Box::new(expr)))
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
/// use darkmatter::markdown::compose::expression::{parse, Expr};
///
/// let expr = parse("foo || \"default\"").unwrap();
/// assert!(matches!(expr, Expr::Fallback { .. }));
/// ```
///
/// ## Errors
///
/// Returns an error for invalid syntax.
pub fn parse(input: &str) -> Result<Expr, ParseError> {
    Parser::new(input)?.parse()
}

/// Parses an expression string using condition-mode grammar.
///
/// In condition mode, `&&` and `||` are recognized as infix logical operators
/// and lowered into `And(...)` / `Or(...)` function-call nodes. Use this
/// entrypoint for every `when="..."` expression so interpolation parsing
/// elsewhere is unaffected.
///
/// ## Examples
///
/// ```
/// use darkmatter::markdown::compose::expression::{parse_condition, Expr};
///
/// let expr = parse_condition("a && b").unwrap();
/// match expr {
///     Expr::FunctionCall { name, args } => {
///         assert_eq!(name, "And");
///         assert_eq!(args.len(), 2);
///     }
///     _ => panic!("expected And(...) function call"),
/// }
/// ```
///
/// ## Errors
///
/// Returns an error for invalid syntax.
pub fn parse_condition(input: &str) -> Result<Expr, ParseError> {
    Parser::with_mode(input, ParseMode::Condition)?.parse()
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
            let expr = parse("3.15").unwrap();
            match expr {
                Expr::NumberLiteral(n) => assert!((n - 3.15).abs() < f64::EPSILON),
                _ => panic!("Expected NumberLiteral"),
            }
        }

        #[test]
        fn parses_negative_number() {
            // Negative numbers are now represented as UnaryMinus over a positive literal.
            let expr = parse("-42").unwrap();
            match expr {
                Expr::UnaryMinus(inner) => {
                    assert!(matches!(*inner, Expr::NumberLiteral(n) if n == 42.0));
                }
                other => panic!("Expected UnaryMinus, got {other:?}"),
            }
        }

        #[test]
        fn parses_true_literal() {
            let expr = parse("true").unwrap();
            assert!(matches!(expr, Expr::BoolLiteral(b) if b));
        }

        #[test]
        fn parses_false_literal() {
            let expr = parse("false").unwrap();
            assert!(matches!(expr, Expr::BoolLiteral(b) if !b));
        }

        #[test]
        fn parses_ternary_with_bool_literals() {
            let expr = parse("enabled ? true : false").unwrap();
            match expr {
                Expr::Ternary {
                    condition,
                    then_branch,
                    else_branch,
                } => {
                    assert!(matches!(*condition, Expr::Variable(ref n) if n == "enabled"));
                    assert!(matches!(*then_branch, Expr::BoolLiteral(true)));
                    assert!(matches!(*else_branch, Expr::BoolLiteral(false)));
                }
                _ => panic!("Expected Ternary"),
            }
        }
    }

    mod fallback_expressions {
        use super::*;

        #[test]
        fn parses_fallback() {
            let expr = parse(r#"foo || "default""#).unwrap();
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
            let expr = parse("foo || bar").unwrap();
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
            // foo || bar || baz parses as ((foo || bar) || baz)
            let expr = parse("foo || bar || baz").unwrap();
            match expr {
                Expr::Fallback { primary, fallback } => {
                    // The fallback is baz
                    assert!(matches!(*fallback, Expr::Variable(ref n) if n == "baz"));
                    // The primary is (foo || bar)
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
            let expr = parse(r#"env.FAVORITE_COLOR || "unknown""#).unwrap();
            match expr {
                Expr::Fallback { primary, fallback } => {
                    assert!(matches!(*primary, Expr::Variable(ref n) if n == "env.FAVORITE_COLOR"));
                    assert!(matches!(*fallback, Expr::StringLiteral(ref s) if s == "unknown"));
                }
                _ => panic!("Expected Fallback"),
            }
        }

        #[test]
        fn parses_double_pipe_as_fallback() {
            // || is the fallback operator in interpolation mode
            let expr = parse(r#"plan || "plan.md""#).unwrap();
            match expr {
                Expr::Fallback { primary, fallback } => {
                    assert!(matches!(*primary, Expr::Variable(ref n) if n == "plan"));
                    assert!(matches!(*fallback, Expr::StringLiteral(ref s) if s == "plan.md"));
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
            // x ? (a || b) : (c || d)
            let expr = parse(r#"x ? a || b : c || d"#).unwrap();
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

        #[test]
        fn parses_nested_ternary_in_true_branch() {
            // a ? b ? c : d : e parses as a ? (b ? c : d) : e
            let expr = parse("a ? b ? c : d : e").unwrap();
            match expr {
                Expr::Ternary {
                    condition,
                    then_branch,
                    else_branch,
                } => {
                    assert!(matches!(*condition, Expr::Variable(ref n) if n == "a"));
                    assert!(matches!(*else_branch, Expr::Variable(ref n) if n == "e"));
                    // then_branch should itself be a ternary
                    match *then_branch {
                        Expr::Ternary {
                            condition: inner_cond,
                            then_branch: inner_then,
                            else_branch: inner_else,
                        } => {
                            assert!(matches!(*inner_cond, Expr::Variable(ref n) if n == "b"));
                            assert!(matches!(*inner_then, Expr::Variable(ref n) if n == "c"));
                            assert!(matches!(*inner_else, Expr::Variable(ref n) if n == "d"));
                        }
                        _ => panic!("Expected nested Ternary in then_branch"),
                    }
                }
                _ => panic!("Expected Ternary"),
            }
        }

        #[test]
        fn parses_nested_ternary_in_false_branch() {
            // a ? b : c ? d : e parses as a ? b : (c ? d : e)
            let expr = parse("a ? b : c ? d : e").unwrap();
            match expr {
                Expr::Ternary {
                    condition,
                    then_branch,
                    else_branch,
                } => {
                    assert!(matches!(*condition, Expr::Variable(ref n) if n == "a"));
                    assert!(matches!(*then_branch, Expr::Variable(ref n) if n == "b"));
                    // else_branch should itself be a ternary
                    match *else_branch {
                        Expr::Ternary {
                            condition: inner_cond,
                            then_branch: inner_then,
                            else_branch: inner_else,
                        } => {
                            assert!(matches!(*inner_cond, Expr::Variable(ref n) if n == "c"));
                            assert!(matches!(*inner_then, Expr::Variable(ref n) if n == "d"));
                            assert!(matches!(*inner_else, Expr::Variable(ref n) if n == "e"));
                        }
                        _ => panic!("Expected nested Ternary in else_branch"),
                    }
                }
                _ => panic!("Expected Ternary"),
            }
        }

        #[test]
        fn parses_parenthesized_nested_ternary() {
            // a ? (b ? c : d) : e
            let expr = parse("a ? (b ? c : d) : e").unwrap();
            match expr {
                Expr::Ternary {
                    condition,
                    then_branch,
                    else_branch,
                } => {
                    assert!(matches!(*condition, Expr::Variable(ref n) if n == "a"));
                    assert!(matches!(*else_branch, Expr::Variable(ref n) if n == "e"));
                    match *then_branch {
                        Expr::Paren(inner) => match *inner {
                            Expr::Ternary {
                                condition: inner_cond,
                                then_branch: inner_then,
                                else_branch: inner_else,
                            } => {
                                assert!(matches!(*inner_cond, Expr::Variable(ref n) if n == "b"));
                                assert!(matches!(*inner_then, Expr::Variable(ref n) if n == "c"));
                                assert!(matches!(*inner_else, Expr::Variable(ref n) if n == "d"));
                            }
                            _ => panic!("Expected nested Ternary inside Paren"),
                        },
                        _ => panic!("Expected Paren wrapping nested Ternary in then_branch"),
                    }
                }
                _ => panic!("Expected Ternary"),
            }
        }

        #[test]
        fn parses_deeply_nested_ternary() {
            // a ? b ? c ? d : e : f : g
            let expr = parse("a ? b ? c ? d : e : f : g").unwrap();
            match expr {
                Expr::Ternary {
                    condition,
                    then_branch,
                    else_branch,
                } => {
                    assert!(matches!(*condition, Expr::Variable(ref n) if n == "a"));
                    assert!(matches!(*else_branch, Expr::Variable(ref n) if n == "g"));
                    match *then_branch {
                        Expr::Ternary {
                            condition: inner_cond,
                            then_branch: inner_then,
                            else_branch: inner_else,
                        } => {
                            assert!(matches!(*inner_cond, Expr::Variable(ref n) if n == "b"));
                            assert!(matches!(*inner_else, Expr::Variable(ref n) if n == "f"));
                            match *inner_then {
                                Expr::Ternary {
                                    condition: deepest_cond,
                                    then_branch: deepest_then,
                                    else_branch: deepest_else,
                                } => {
                                    assert!(matches!(*deepest_cond, Expr::Variable(ref n) if n == "c"));
                                    assert!(matches!(*deepest_then, Expr::Variable(ref n) if n == "d"));
                                    assert!(matches!(*deepest_else, Expr::Variable(ref n) if n == "e"));
                                }
                                _ => panic!("Expected deepest Ternary"),
                            }
                        }
                        _ => panic!("Expected nested Ternary in then_branch"),
                    }
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
            // Test: foo || bar ? "truthy" : "falsy"
            // This should parse as: (foo || bar) ? "truthy" : "falsy"
            // But actually per precedence: foo || (bar ? "truthy" : "falsy")
            // Wait, ternary has lowest precedence, so:
            // ternary parses fallback first, which consumes foo || bar
            // then sees ?, so the condition is (foo || bar)
            let expr = parse(r#"foo || bar ? "truthy" : "falsy""#).unwrap();
            match expr {
                Expr::Ternary {
                    condition,
                    then_branch,
                    else_branch,
                } => {
                    // Condition should be foo || bar
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
            // length(items || defaults)
            let expr = parse("length(items || defaults)").unwrap();
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
            assert!(matches!(expr, Expr::Paren(inner) if matches!(*inner, Expr::Variable(ref name) if name == "foo")));
        }

        #[test]
        fn parses_nested_parentheses() {
            let expr = parse("((foo))").unwrap();
            match expr {
                Expr::Paren(outer) => match *outer {
                    Expr::Paren(inner) => {
                        assert!(matches!(*inner, Expr::Variable(ref name) if name == "foo"));
                    }
                    _ => panic!("Expected inner Paren"),
                },
                _ => panic!("Expected Paren"),
            }
        }

        #[test]
        fn parses_parenthesized_fallback() {
            let expr = parse("(foo || bar)").unwrap();
            assert!(matches!(expr, Expr::Paren(inner) if matches!(*inner, Expr::Fallback { .. })));
        }

        #[test]
        fn parses_parentheses_in_ternary() {
            // (a || b) ? c : d
            let expr = parse("(a || b) ? c : d").unwrap();
            match expr {
                Expr::Ternary { condition, .. } => {
                    assert!(matches!(*condition, Expr::Paren(inner) if matches!(*inner, Expr::Fallback { .. })));
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
        fn error_bare_pipe_in_interpolation() {
            let result = parse(r#"foo | "default""#);
            assert!(
                result.is_err(),
                "bare '|' should be rejected in interpolation mode"
            );
            let err = result.unwrap_err();
            assert!(
                err.message.contains("Unexpected '|'"),
                "error should mention '|', got: {}",
                err.message
            );
        }

        #[test]
        fn error_bare_pipe_in_condition() {
            let result = parse_condition("a | b");
            assert!(
                result.is_err(),
                "bare '|' should be rejected in condition mode"
            );
            let err = result.unwrap_err();
            assert!(
                err.message.contains("Unexpected '|'"),
                "error should mention '|', got: {}",
                err.message
            );
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
        fn error_paren_around_bare_colon_in_ternary() {
            // a ? (b : c) — colon inside parentheses without matching ?
            let result = parse("a ? (b : c)");
            assert!(result.is_err(), "colon inside parens without ? should fail");
        }

        #[test]
        fn error_unmatched_paren_in_ternary() {
            // a ? b) : c
            let result = parse("a ? b) : c");
            assert!(result.is_err(), "unmatched ')' should fail");
        }

        #[test]
        fn error_unbalanced_paren_in_nested_ternary() {
            // a ? (b ? c : d
            let result = parse("a ? (b ? c : d");
            assert!(result.is_err(), "missing closing ')' should fail");
            let err = result.unwrap_err();
            assert!(err.message.contains("')'"));
        }

        #[test]
        fn error_trailing_pipe() {
            let result = parse("foo ||");
            assert!(result.is_err());
        }

        #[test]
        fn error_leading_pipe() {
            let result = parse("|| foo");
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

        #[test]
        fn error_invalid_groupings_from_spec() {
            // a group can not encapsulate both the true and false path of the top level comparison
            assert!(parse("a ? ( b ? 'tt' : 'tf' : c ? 'ft' : 'ff' )").is_err());
            
            // unbalanced: missing closing
            assert!(parse("a ? ( b ? 'tt' : 'tf' : c ? 'ft' : 'ff'").is_err());
            
            // unbalanced: missing opening (parenthesis must wrap a complete pattern)
            assert!(parse("a ? b ? 'tt' : 'tf' : c ? ('ft' : 'ff')").is_err());
            
            // parenthesis must wrap a complete pattern
            assert!(parse("a ? b ? ( 'tt' : 'tf' ) : c ? ( 'ft' : 'ff' )").is_err());
            
            // parenthesis encapsulates part of top level but not full
            assert!(parse("a ? b ( ? 'tt' : 'tf' ) : c ( ? 'ft' : 'ff' )").is_err());
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
        fn roundtrip_bool_literal() {
            let expr = parse("true").unwrap();
            assert_eq!(expr.to_string(), "true");
            let expr = parse("false").unwrap();
            assert_eq!(expr.to_string(), "false");
        }

        #[test]
        fn roundtrip_parenthesized_variable() {
            let expr = parse("(foo)").unwrap();
            assert_eq!(expr.to_string(), "(foo)");
        }

        #[test]
        fn roundtrip_parenthesized_fallback() {
            let expr = parse(r#"(foo || "default")"#).unwrap();
            assert_eq!(expr.to_string(), "(foo || \"default\")");
        }

        #[test]
        fn roundtrip_fallback() {
            let expr = parse(r#"foo || "default""#).unwrap();
            assert_eq!(expr.to_string(), "foo || \"default\"");
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

    mod arithmetic {
        use super::*;
        use crate::markdown::compose::expression::ast::BinaryOp;

        fn extract_binary(expr: &Expr) -> (BinaryOp, &Expr, &Expr) {
            match expr {
                Expr::Binary { op, left, right } => (*op, left.as_ref(), right.as_ref()),
                other => panic!("expected Binary, got {other:?}"),
            }
        }

        #[test]
        fn parses_addition() {
            let (op, left, right) = {
                let expr = parse("a + b").unwrap();
                let (op, l, r) = extract_binary(&expr);
                (op, l.clone(), r.clone())
            };
            assert_eq!(op, BinaryOp::Add);
            assert!(matches!(left, Expr::Variable(ref n) if n == "a"));
            assert!(matches!(right, Expr::Variable(ref n) if n == "b"));
        }

        #[test]
        fn parses_all_arithmetic_ops() {
            for (src, expected) in [
                ("a + b", BinaryOp::Add),
                ("a - b", BinaryOp::Sub),
                ("a * b", BinaryOp::Mul),
                ("a / b", BinaryOp::Div),
                ("a % b", BinaryOp::Mod),
            ] {
                let expr = parse(src).unwrap();
                let (op, _, _) = extract_binary(&expr);
                assert_eq!(op, expected, "for source {src}");
            }
        }

        #[test]
        fn subtraction_is_left_associative() {
            // a - b - c parses as (a - b) - c
            let expr = parse("a - b - c").unwrap();
            let (outer_op, outer_left, outer_right) = extract_binary(&expr);
            assert_eq!(outer_op, BinaryOp::Sub);
            assert!(matches!(outer_right, Expr::Variable(n) if n == "c"));
            let (inner_op, inner_left, inner_right) = extract_binary(outer_left);
            assert_eq!(inner_op, BinaryOp::Sub);
            assert!(matches!(inner_left, Expr::Variable(n) if n == "a"));
            assert!(matches!(inner_right, Expr::Variable(n) if n == "b"));
        }

        #[test]
        fn division_is_left_associative() {
            // a / b / c parses as (a / b) / c
            let expr = parse("a / b / c").unwrap();
            let (outer_op, outer_left, outer_right) = extract_binary(&expr);
            assert_eq!(outer_op, BinaryOp::Div);
            assert!(matches!(outer_right, Expr::Variable(n) if n == "c"));
            let (inner_op, _, _) = extract_binary(outer_left);
            assert_eq!(inner_op, BinaryOp::Div);
        }

        #[test]
        fn fallback_is_left_associative() {
            // a || b || c parses as (a || b) || c
            let expr = parse("a || b || c").unwrap();
            match expr {
                Expr::Fallback { primary, fallback } => {
                    assert!(matches!(*fallback, Expr::Variable(ref n) if n == "c"));
                    assert!(matches!(*primary, Expr::Fallback { .. }));
                }
                other => panic!("expected Fallback, got {other:?}"),
            }
        }

        #[test]
        fn multiplicative_binds_tighter_than_additive() {
            // a + b * c parses as a + (b * c)
            let expr = parse("a + b * c").unwrap();
            let (top_op, top_left, top_right) = extract_binary(&expr);
            assert_eq!(top_op, BinaryOp::Add);
            assert!(matches!(top_left, Expr::Variable(n) if n == "a"));
            let (inner_op, _, _) = extract_binary(top_right);
            assert_eq!(inner_op, BinaryOp::Mul);
        }

        #[test]
        fn additive_binds_tighter_than_comparison() {
            // a + b <= c parses as (a + b) <= c
            let expr = parse("a + b <= c").unwrap();
            match expr {
                Expr::Comparison { left, op, right } => {
                    assert_eq!(op, ComparisonOp::LessThanOrEqual);
                    let (inner_op, _, _) = extract_binary(left.as_ref());
                    assert_eq!(inner_op, BinaryOp::Add);
                    assert!(matches!(*right, Expr::Variable(ref n) if n == "c"));
                }
                other => panic!("expected Comparison, got {other:?}"),
            }
        }

        #[test]
        fn comparison_binds_tighter_than_fallback() {
            // a + b * c <= d || e parses as ((a + b * c) <= d) || e
            let expr = parse("a + b * c <= d || e").unwrap();
            match expr {
                Expr::Fallback { primary, fallback } => {
                    assert!(matches!(*fallback, Expr::Variable(ref n) if n == "e"));
                    assert!(matches!(*primary, Expr::Comparison { .. }));
                }
                other => panic!("expected Fallback, got {other:?}"),
            }
        }

        #[test]
        fn unary_minus_on_variable() {
            let expr = parse("-a").unwrap();
            match expr {
                Expr::UnaryMinus(inner) => {
                    assert!(matches!(*inner, Expr::Variable(ref n) if n == "a"));
                }
                other => panic!("expected UnaryMinus, got {other:?}"),
            }
        }

        #[test]
        fn unary_minus_in_subtraction_no_space() {
            // 5-3 is left=5, op=Sub, right=3
            let expr = parse("5-3").unwrap();
            let (op, left, right) = extract_binary(&expr);
            assert_eq!(op, BinaryOp::Sub);
            assert!(matches!(left, Expr::NumberLiteral(n) if *n == 5.0));
            assert!(matches!(right, Expr::NumberLiteral(n) if *n == 3.0));
        }

        #[test]
        fn unary_minus_after_operator() {
            // 5 + -3 is 5 + (-3)
            let expr = parse("5 + -3").unwrap();
            let (op, _, right) = extract_binary(&expr);
            assert_eq!(op, BinaryOp::Add);
            assert!(matches!(right, Expr::UnaryMinus(_)));
        }
    }

    mod bracket_access {
        use super::*;

        #[test]
        fn parses_simple_index() {
            let expr = parse("items[0]").unwrap();
            match expr {
                Expr::Index { base, index } => {
                    assert!(matches!(*base, Expr::Variable(ref n) if n == "items"));
                    assert!(matches!(*index, Expr::NumberLiteral(n) if n == 0.0));
                }
                other => panic!("expected Index, got {other:?}"),
            }
        }

        #[test]
        fn parses_negative_index() {
            let expr = parse("items[-1]").unwrap();
            match expr {
                Expr::Index { base, index } => {
                    assert!(matches!(*base, Expr::Variable(ref n) if n == "items"));
                    match *index {
                        Expr::UnaryMinus(inner) => {
                            assert!(matches!(*inner, Expr::NumberLiteral(n) if n == 1.0));
                        }
                        other => panic!("expected UnaryMinus index, got {other:?}"),
                    }
                }
                other => panic!("expected Index, got {other:?}"),
            }
        }

        #[test]
        fn parses_string_key() {
            let expr = parse(r#"config["key"]"#).unwrap();
            match expr {
                Expr::Index { base, index } => {
                    assert!(matches!(*base, Expr::Variable(ref n) if n == "config"));
                    assert!(matches!(*index, Expr::StringLiteral(ref s) if s == "key"));
                }
                other => panic!("expected Index, got {other:?}"),
            }
        }

        #[test]
        fn parses_chained_bracket_access() {
            // config["key"][0]
            let expr = parse(r#"config["key"][0]"#).unwrap();
            match expr {
                Expr::Index { base, index } => {
                    assert!(matches!(*index, Expr::NumberLiteral(n) if n == 0.0));
                    assert!(matches!(*base, Expr::Index { .. }));
                }
                other => panic!("expected outer Index, got {other:?}"),
            }
        }

        #[test]
        fn parses_index_then_member_access() {
            // items[-1].name
            let expr = parse("items[-1].name").unwrap();
            match expr {
                Expr::MemberAccess { base, name } => {
                    assert_eq!(name, "name");
                    assert!(matches!(*base, Expr::Index { .. }));
                }
                other => panic!("expected MemberAccess, got {other:?}"),
            }
        }

        #[test]
        fn member_access_after_paren() {
            let expr = parse("(foo).bar").unwrap();
            match expr {
                Expr::MemberAccess { base, name } => {
                    assert_eq!(name, "bar");
                    assert!(matches!(*base, Expr::Paren(_)));
                }
                other => panic!("expected MemberAccess, got {other:?}"),
            }
        }

        #[test]
        fn dotted_variable_then_bracket() {
            // foo.bar[0] — dotted path is a single Variable, followed by Index.
            let expr = parse("foo.bar[0]").unwrap();
            match expr {
                Expr::Index { base, index } => {
                    assert!(matches!(*base, Expr::Variable(ref n) if n == "foo.bar"));
                    assert!(matches!(*index, Expr::NumberLiteral(n) if n == 0.0));
                }
                other => panic!("expected Index, got {other:?}"),
            }
        }

        #[test]
        fn rejects_numeric_dot_access() {
            let result = parse("foo.0");
            assert!(result.is_err(), "foo.0 should be rejected");
            let err = result.unwrap_err();
            assert!(
                err.message.contains("Numeric dot access"),
                "expected numeric-dot-access error, got: {}",
                err.message
            );
        }
    }

    mod ternary_associativity {
        use super::*;

        #[test]
        fn ternary_is_right_associative_for_else_branch() {
            // a ? b : c ? d : e parses as a ? b : (c ? d : e)
            let expr = parse("a ? b : c ? d : e").unwrap();
            match expr {
                Expr::Ternary {
                    condition,
                    then_branch,
                    else_branch,
                } => {
                    assert!(matches!(*condition, Expr::Variable(ref n) if n == "a"));
                    assert!(matches!(*then_branch, Expr::Variable(ref n) if n == "b"));
                    assert!(matches!(*else_branch, Expr::Ternary { .. }));
                }
                other => panic!("expected Ternary, got {other:?}"),
            }
        }
    }

    mod condition_mode_logic {
        use super::*;

        fn extract_call(expr: &Expr) -> (&str, &[Expr]) {
            match expr {
                Expr::FunctionCall { name, args } => (name.as_str(), args.as_slice()),
                other => panic!("expected FunctionCall, got {other:?}"),
            }
        }

        #[test]
        fn condition_parses_infix_and() {
            let expr = parse_condition("a && b").unwrap();
            let (name, args) = extract_call(&expr);
            assert_eq!(name, "And");
            assert_eq!(args.len(), 2);
            assert!(matches!(&args[0], Expr::Variable(n) if n == "a"));
            assert!(matches!(&args[1], Expr::Variable(n) if n == "b"));
        }

        #[test]
        fn condition_parses_infix_or() {
            let expr = parse_condition("a || b").unwrap();
            let (name, args) = extract_call(&expr);
            assert_eq!(name, "Or");
            assert_eq!(args.len(), 2);
            assert!(matches!(&args[0], Expr::Variable(n) if n == "a"));
            assert!(matches!(&args[1], Expr::Variable(n) if n == "b"));
        }

        #[test]
        fn condition_and_binds_tighter_than_or_left() {
            // a && b || c parses as (a && b) || c
            let expr = parse_condition("a && b || c").unwrap();
            let (outer, args) = extract_call(&expr);
            assert_eq!(outer, "Or");
            assert_eq!(args.len(), 2);
            let (inner, inner_args) = extract_call(&args[0]);
            assert_eq!(inner, "And");
            assert!(matches!(&inner_args[0], Expr::Variable(n) if n == "a"));
            assert!(matches!(&inner_args[1], Expr::Variable(n) if n == "b"));
            assert!(matches!(&args[1], Expr::Variable(n) if n == "c"));
        }

        #[test]
        fn condition_and_binds_tighter_than_or_right() {
            // a || b && c parses as a || (b && c)
            let expr = parse_condition("a || b && c").unwrap();
            let (outer, args) = extract_call(&expr);
            assert_eq!(outer, "Or");
            assert_eq!(args.len(), 2);
            assert!(matches!(&args[0], Expr::Variable(n) if n == "a"));
            let (inner, inner_args) = extract_call(&args[1]);
            assert_eq!(inner, "And");
            assert!(matches!(&inner_args[0], Expr::Variable(n) if n == "b"));
            assert!(matches!(&inner_args[1], Expr::Variable(n) if n == "c"));
        }

        #[test]
        fn condition_parenthesized_or_then_and() {
            // (a || b) && c parses as And(Paren(Or(a, b)), c)
            let expr = parse_condition("(a || b) && c").unwrap();
            let (outer, args) = extract_call(&expr);
            assert_eq!(outer, "And");
            assert_eq!(args.len(), 2);
            let paren = match &args[0] {
                Expr::Paren(inner) => inner,
                other => panic!("expected Paren, got {other:?}"),
            };
            let (inner, inner_args) = extract_call(paren);
            assert_eq!(inner, "Or");
            assert!(matches!(&inner_args[0], Expr::Variable(n) if n == "a"));
            assert!(matches!(&inner_args[1], Expr::Variable(n) if n == "b"));
            assert!(matches!(&args[1], Expr::Variable(n) if n == "c"));
        }

        #[test]
        fn condition_or_inside_or() {
            // a || (b || c) — chained OR
            let expr = parse_condition("a || (b || c)").unwrap();
            let (outer, args) = extract_call(&expr);
            assert_eq!(outer, "Or");
            assert_eq!(args.len(), 2);
            assert!(matches!(&args[0], Expr::Variable(n) if n == "a"));
            let paren = match &args[1] {
                Expr::Paren(inner) => inner,
                other => panic!("expected Paren, got {other:?}"),
            };
            let (inner, inner_args) = extract_call(paren);
            assert_eq!(inner, "Or");
            assert!(matches!(&inner_args[0], Expr::Variable(n) if n == "b"));
            assert!(matches!(&inner_args[1], Expr::Variable(n) if n == "c"));
        }

        #[test]
        fn interpolation_double_pipe_still_fallback() {
            let expr = parse(r#"plan || "plan.md""#).unwrap();
            match expr {
                Expr::Fallback { primary, fallback } => {
                    assert!(matches!(*primary, Expr::Variable(ref n) if n == "plan"));
                    assert!(matches!(*fallback, Expr::StringLiteral(ref s) if s == "plan.md"));
                }
                _ => panic!("Expected Fallback"),
            }
        }

        #[test]
        fn interpolation_rejects_infix_and() {
            let result = parse("a && b");
            assert!(result.is_err(), "&& must be rejected in interpolation mode");
        }

        #[test]
        fn condition_chained_and_left_associative() {
            // a && b && c parses as And(And(a, b), c)
            let expr = parse_condition("a && b && c").unwrap();
            let (outer, args) = extract_call(&expr);
            assert_eq!(outer, "And");
            assert_eq!(args.len(), 2);
            let (inner, inner_args) = extract_call(&args[0]);
            assert_eq!(inner, "And");
            assert!(matches!(&inner_args[0], Expr::Variable(n) if n == "a"));
            assert!(matches!(&inner_args[1], Expr::Variable(n) if n == "b"));
            assert!(matches!(&args[1], Expr::Variable(n) if n == "c"));
        }

        #[test]
        fn condition_chained_or_left_associative() {
            // a || b || c parses as Or(Or(a, b), c)
            let expr = parse_condition("a || b || c").unwrap();
            let (outer, args) = extract_call(&expr);
            assert_eq!(outer, "Or");
            assert_eq!(args.len(), 2);
            let (inner, _) = extract_call(&args[0]);
            assert_eq!(inner, "Or");
        }
    }
}
