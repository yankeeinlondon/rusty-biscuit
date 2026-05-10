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
//! 4. **Fallback** - `||`
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
//! {{ foo || "default" }} -> Fallback { primary: Variable("foo"), fallback: StringLiteral("default") }
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

/// Arithmetic binary operators supported by the expression evaluator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    /// Addition `+` (also string concatenation when either operand is a string).
    Add,
    /// Subtraction `-`.
    Sub,
    /// Multiplication `*`.
    Mul,
    /// Division `/`.
    Div,
    /// Remainder `%` with C-style sign behavior (sign follows the dividend).
    Mod,
}

impl fmt::Display for BinaryOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BinaryOp::Add => write!(f, "+"),
            BinaryOp::Sub => write!(f, "-"),
            BinaryOp::Mul => write!(f, "*"),
            BinaryOp::Div => write!(f, "/"),
            BinaryOp::Mod => write!(f, "%"),
        }
    }
}

/// An expression node in the AST.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// Variable reference: `foo`, `user.name`, `ctx.today`, `env.HOME`.
    ///
    /// Dotted paths are kept as single strings for simpler lookup.
    Variable(String),

    /// String literal: `"hello"` or `'hello'`.
    StringLiteral(String),

    /// Number literal: `42`, `3.14`. Negative literals are represented as
    /// [`Expr::UnaryMinus`] over a non-negative `NumberLiteral`.
    NumberLiteral(f64),

    /// Boolean literal: `true` or `false`.
    BoolLiteral(bool),

    /// Unary not expression: `!expr`.
    UnaryNot(Box<Expr>),

    /// Unary minus expression: `-expr`.
    UnaryMinus(Box<Expr>),

    /// Parenthesized expression: `(expr)`.
    ///
    /// Preserves user-provided grouping for display and debugging.
    Paren(Box<Expr>),

    /// Arithmetic binary expression: `left op right`.
    Binary {
        /// Arithmetic operator.
        op: BinaryOp,
        /// Left operand.
        left: Box<Expr>,
        /// Right operand.
        right: Box<Expr>,
    },

    /// Bracket access: `base[index]`.
    ///
    /// `index` may be any expression. At evaluation time integer-valued
    /// numbers index into arrays (negative indexes count from the end) and
    /// string keys index into objects.
    Index {
        /// The collection to index into.
        base: Box<Expr>,
        /// The index expression.
        index: Box<Expr>,
    },

    /// Postfix member access: `base.name`.
    ///
    /// Used when the dotted segment cannot be folded into a [`Expr::Variable`]
    /// — for example after a bracket access, function call, or parenthesized
    /// expression. `name` may itself be a dotted path because the lexer folds
    /// pure identifier paths together.
    MemberAccess {
        /// The base expression to read from.
        base: Box<Expr>,
        /// The member name (may contain `.` for nested access).
        name: String,
    },

    /// Fallback expression: `expr || fallback`.
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
            Expr::BoolLiteral(b) => write!(f, "{}", b),
            Expr::UnaryNot(expr) => write!(f, "!{}", expr),
            Expr::UnaryMinus(expr) => write!(f, "-{}", expr),
            Expr::Paren(expr) => write!(f, "({})", expr),
            Expr::Binary { op, left, right } => write!(f, "{} {} {}", left, op, right),
            Expr::Index { base, index } => write!(f, "{}[{}]", base, index),
            Expr::MemberAccess { base, name } => write!(f, "{}.{}", base, name),
            Expr::Fallback { primary, fallback } => {
                write!(f, "{} || {}", primary, fallback)
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
    fn display_bool_literal_true() {
        let expr = Expr::BoolLiteral(true);
        assert_eq!(expr.to_string(), "true");
    }

    #[test]
    fn display_bool_literal_false() {
        let expr = Expr::BoolLiteral(false);
        assert_eq!(expr.to_string(), "false");
    }

    #[test]
    fn display_paren() {
        let expr = Expr::Paren(Box::new(Expr::Variable("foo".to_string())));
        assert_eq!(expr.to_string(), "(foo)");
    }

    #[test]
    fn display_fallback() {
        let expr = Expr::Fallback {
            primary: Box::new(Expr::Variable("foo".to_string())),
            fallback: Box::new(Expr::StringLiteral("default".to_string())),
        };
        assert_eq!(expr.to_string(), "foo || \"default\"");
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
    fn display_unary_minus() {
        let expr = Expr::UnaryMinus(Box::new(Expr::NumberLiteral(42.0)));
        assert_eq!(expr.to_string(), "-42");
    }

    #[test]
    fn display_binary_arithmetic() {
        let expr = Expr::Binary {
            op: BinaryOp::Add,
            left: Box::new(Expr::Variable("a".to_string())),
            right: Box::new(Expr::Variable("b".to_string())),
        };
        assert_eq!(expr.to_string(), "a + b");
    }

    #[test]
    fn display_index() {
        let expr = Expr::Index {
            base: Box::new(Expr::Variable("items".to_string())),
            index: Box::new(Expr::NumberLiteral(0.0)),
        };
        assert_eq!(expr.to_string(), "items[0]");
    }

    #[test]
    fn display_index_negative() {
        let expr = Expr::Index {
            base: Box::new(Expr::Variable("items".to_string())),
            index: Box::new(Expr::UnaryMinus(Box::new(Expr::NumberLiteral(1.0)))),
        };
        assert_eq!(expr.to_string(), "items[-1]");
    }

    #[test]
    fn display_member_access() {
        let expr = Expr::MemberAccess {
            base: Box::new(Expr::Index {
                base: Box::new(Expr::Variable("items".to_string())),
                index: Box::new(Expr::NumberLiteral(0.0)),
            }),
            name: "name".to_string(),
        };
        assert_eq!(expr.to_string(), "items[0].name");
    }

    #[test]
    fn display_binary_op_kinds() {
        assert_eq!(BinaryOp::Add.to_string(), "+");
        assert_eq!(BinaryOp::Sub.to_string(), "-");
        assert_eq!(BinaryOp::Mul.to_string(), "*");
        assert_eq!(BinaryOp::Div.to_string(), "/");
        assert_eq!(BinaryOp::Mod.to_string(), "%");
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
