//! AST nodes for interpolation expressions.
//!
//! This module defines the abstract syntax tree (AST) used to represent
//! parsed interpolation expressions. The AST is produced by the parser
//! and consumed by the evaluator.
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
//! Simple variable:
//! ```text
//! {{ foo }} -> Variable("foo")
//! ```
//!
//! Fallback:
//! ```text
//! {{ foo | "default" }} -> Fallback { primary: Variable("foo"), fallback: StringLiteral("default") }
//! ```
//!
//! Ternary with comparison:
//! ```text
//! {{ x == y ? "yes" : "no" }} -> Ternary {
//!     condition: Comparison { left: Variable("x"), op: Equal, right: Variable("y") },
//!     then_branch: StringLiteral("yes"),
//!     else_branch: StringLiteral("no"),
//! }
//! ```

use super::ComparisonOp;
use std::fmt;

/// An expression node in the AST.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// Variable reference: `foo`, `user.name`, `ctx.today`, `env.HOME`.
    ///
    /// Dotted paths are kept as single strings for simpler lookup.
    Variable(String),

    /// String literal: `"hello"` or `'hello'`.
    StringLiteral(String),

    /// Number literal: `42`, `3.14`, `-1`.
    NumberLiteral(f64),

    /// Unary not expression: `!expr`.
    UnaryNot(Box<Expr>),

    /// Fallback expression: `expr | fallback`.
    ///
    /// Returns the primary if truthy, otherwise the fallback.
    Fallback {
        /// The expression to evaluate first.
        primary: Box<Expr>,
        /// The fallback if primary is falsy.
        fallback: Box<Expr>,
    },

    /// Ternary expression: `condition ? then_branch : else_branch`.
    Ternary {
        /// The condition to test.
        condition: Box<Expr>,
        /// Expression if condition is truthy.
        then_branch: Box<Expr>,
        /// Expression if condition is falsy.
        else_branch: Box<Expr>,
    },

    /// Comparison expression: `left op right`.
    ///
    /// Evaluates to a boolean for use in ternary conditions.
    Comparison {
        /// Left operand.
        left: Box<Expr>,
        /// Comparison operator.
        op: ComparisonOp,
        /// Right operand.
        right: Box<Expr>,
    },

    /// Function call: `name(args...)`.
    FunctionCall {
        /// Function name (e.g., `length`, `number`).
        name: String,
        /// Arguments to the function.
        args: Vec<Expr>,
    },
}

impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Expr::Variable(name) => write!(f, "{}", name),
            Expr::StringLiteral(s) => write!(f, "\"{}\"", s),
            Expr::NumberLiteral(n) => {
                if n.fract() == 0.0 {
                    write!(f, "{}", *n as i64)
                } else {
                    write!(f, "{}", n)
                }
            }
            Expr::UnaryNot(expr) => write!(f, "!{}", expr),
            Expr::Fallback { primary, fallback } => {
                write!(f, "{} | {}", primary, fallback)
            }
            Expr::Ternary {
                condition,
                then_branch,
                else_branch,
            } => {
                write!(f, "{} ? {} : {}", condition, then_branch, else_branch)
            }
            Expr::Comparison { left, op, right } => {
                write!(f, "{} {} {}", left, op, right)
            }
            Expr::FunctionCall { name, args } => {
                write!(f, "{}(", name)?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", arg)?;
                }
                write!(f, ")")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_variable() {
        let expr = Expr::Variable("user.name".to_string());
        assert_eq!(expr.to_string(), "user.name");
    }

    #[test]
    fn display_string_literal() {
        let expr = Expr::StringLiteral("hello".to_string());
        assert_eq!(expr.to_string(), "\"hello\"");
    }

    #[test]
    fn display_number_literal_integer() {
        let expr = Expr::NumberLiteral(42.0);
        assert_eq!(expr.to_string(), "42");
    }

    #[test]
    fn display_number_literal_float() {
        let expr = Expr::NumberLiteral(3.15);
        assert_eq!(expr.to_string(), "3.15");
    }

    #[test]
    fn display_fallback() {
        let expr = Expr::Fallback {
            primary: Box::new(Expr::Variable("foo".to_string())),
            fallback: Box::new(Expr::StringLiteral("default".to_string())),
        };
        assert_eq!(expr.to_string(), "foo | \"default\"");
    }

    #[test]
    fn display_unary_not() {
        let expr = Expr::UnaryNot(Box::new(Expr::Variable("enabled".to_string())));
        assert_eq!(expr.to_string(), "!enabled");
    }

    #[test]
    fn display_ternary() {
        let expr = Expr::Ternary {
            condition: Box::new(Expr::Variable("x".to_string())),
            then_branch: Box::new(Expr::StringLiteral("yes".to_string())),
            else_branch: Box::new(Expr::StringLiteral("no".to_string())),
        };
        assert_eq!(expr.to_string(), "x ? \"yes\" : \"no\"");
    }

    #[test]
    fn display_comparison() {
        let expr = Expr::Comparison {
            left: Box::new(Expr::Variable("count".to_string())),
            op: ComparisonOp::GreaterThan,
            right: Box::new(Expr::NumberLiteral(0.0)),
        };
        assert_eq!(expr.to_string(), "count > 0");
    }

    #[test]
    fn display_function_call_no_args() {
        let expr = Expr::FunctionCall {
            name: "now".to_string(),
            args: vec![],
        };
        assert_eq!(expr.to_string(), "now()");
    }

    #[test]
    fn display_function_call_one_arg() {
        let expr = Expr::FunctionCall {
            name: "length".to_string(),
            args: vec![Expr::Variable("items".to_string())],
        };
        assert_eq!(expr.to_string(), "length(items)");
    }

    #[test]
    fn display_function_call_multiple_args() {
        let expr = Expr::FunctionCall {
            name: "number".to_string(),
            args: vec![
                Expr::Variable("value".to_string()),
                Expr::NumberLiteral(0.0),
            ],
        };
        assert_eq!(expr.to_string(), "number(value, 0)");
    }

    #[test]
    fn display_complex_nested() {
        // count > 0 ? "items" : "empty"
        let expr = Expr::Ternary {
            condition: Box::new(Expr::Comparison {
                left: Box::new(Expr::Variable("count".to_string())),
                op: ComparisonOp::GreaterThan,
                right: Box::new(Expr::NumberLiteral(0.0)),
            }),
            then_branch: Box::new(Expr::StringLiteral("items".to_string())),
            else_branch: Box::new(Expr::StringLiteral("empty".to_string())),
        };
        assert_eq!(expr.to_string(), "count > 0 ? \"items\" : \"empty\"");
    }
}
