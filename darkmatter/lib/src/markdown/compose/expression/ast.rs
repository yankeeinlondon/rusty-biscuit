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
use crate::markdown::span::{SourceSpan, Spanned};
use std::fmt;

/// Arithmetic binary operators supported by the expression evaluator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    /// Addition `+`; mixed numbers and numeric strings add, while strings
    /// concatenate otherwise.
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

    /// Immutable array literal: `[value, computed]`.
    ArrayLiteral(Vec<Expr>),

    /// Immutable object literal: `{ key: value, "quoted-key": computed }`.
    ObjectLiteral(Vec<(String, Expr)>),

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
            Expr::ArrayLiteral(items) => {
                write!(f, "[")?;
                for (index, item) in items.iter().enumerate() {
                    if index > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{item}")?;
                }
                write!(f, "]")
            }
            Expr::ObjectLiteral(entries) => {
                write!(f, "{{")?;
                for (index, (key, value)) in entries.iter().enumerate() {
                    if index > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "\"{key}\": {value}")?;
                }
                write!(f, "}}")
            }
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

/// A span-carrying expression node — the primary product of the recursive
/// descent parser.
///
/// [`SpannedExpr`] mirrors [`Expr`] node-for-node, adding a [`SourceSpan`]
/// (byte-offset range into the expression source) at every level so DMLS can
/// map a cursor position or a sub-expression back to its exact source text.
/// The compose engine consumes the span-erased [`Expr`] via [`SpannedExpr::erase`];
/// the two are produced by one grammar, so any [`Expr`] the compose pipeline
/// sees is exactly `parse_spanned(src).erase()`.
#[derive(Debug, Clone, PartialEq)]
pub struct SpannedExpr {
    /// The node payload.
    pub kind: SpannedExprKind,
    /// Byte span of the source text this node was parsed from.
    pub span: SourceSpan,
}

/// The payload of a [`SpannedExpr`], mirroring [`Expr`] with spanned children.
#[derive(Debug, Clone, PartialEq)]
pub enum SpannedExprKind {
    /// Variable reference: `foo`, `user.name`, `ctx.today`, `env.HOME`.
    Variable(String),
    /// String literal: `"hello"` or `'hello'`.
    StringLiteral(String),
    /// Number literal: `42`, `3.14` (always non-negative; see [`SpannedExprKind::UnaryMinus`]).
    NumberLiteral(f64),
    /// Boolean literal: `true` or `false`.
    BoolLiteral(bool),
    /// Immutable array literal. Each child retains its complete source span.
    ArrayLiteral(Vec<SpannedExpr>),
    /// Immutable object literal. Keys and values retain their source spans.
    ObjectLiteral(Vec<(Spanned<String>, SpannedExpr)>),
    /// Unary not expression: `!expr`.
    UnaryNot(Box<SpannedExpr>),
    /// Unary minus expression: `-expr`.
    UnaryMinus(Box<SpannedExpr>),
    /// Parenthesized expression: `(expr)`.
    Paren(Box<SpannedExpr>),
    /// Arithmetic binary expression: `left op right`.
    Binary {
        /// Arithmetic operator.
        op: BinaryOp,
        /// Left operand.
        left: Box<SpannedExpr>,
        /// Right operand.
        right: Box<SpannedExpr>,
    },
    /// Bracket access: `base[index]`.
    Index {
        /// The collection to index into.
        base: Box<SpannedExpr>,
        /// The index expression.
        index: Box<SpannedExpr>,
    },
    /// Postfix member access: `base.name`.
    MemberAccess {
        /// The base expression to read from.
        base: Box<SpannedExpr>,
        /// The member name (may contain `.` for nested access).
        name: String,
    },
    /// Fallback expression: `expr || fallback`.
    Fallback {
        /// The expression to evaluate first.
        primary: Box<SpannedExpr>,
        /// The fallback if primary is falsy.
        fallback: Box<SpannedExpr>,
    },
    /// Ternary expression: `condition ? then_branch : else_branch`.
    Ternary {
        /// The condition to test.
        condition: Box<SpannedExpr>,
        /// Expression if condition is truthy.
        then_branch: Box<SpannedExpr>,
        /// Expression if condition is falsy.
        else_branch: Box<SpannedExpr>,
    },
    /// Comparison expression: `left op right`.
    Comparison {
        /// Left operand.
        left: Box<SpannedExpr>,
        /// Comparison operator.
        op: ComparisonOp,
        /// Right operand.
        right: Box<SpannedExpr>,
    },
    /// Function call: `name(args...)`.
    ///
    /// Condition-mode infix `&&` / `||` lower into `and(...)` / `or(...)`
    /// function calls here, identically to [`Expr`], so span erasure is exact.
    FunctionCall {
        /// Function name (e.g., `length`, `number`, `and`, `or`).
        name: String,
        /// Arguments to the function.
        args: Vec<SpannedExpr>,
    },
}

impl SpannedExpr {
    /// Creates a spanned expression node.
    pub fn new(kind: SpannedExprKind, span: SourceSpan) -> Self {
        Self { kind, span }
    }

    /// Lowers this spanned node into the span-erased [`Expr`] the compose
    /// evaluator consumes.
    ///
    /// This is the span-erasure half of the single-grammar contract: the
    /// compose pipeline is provably unchanged because every [`Expr`] it
    /// evaluates is `parse_spanned(src).erase()`.
    pub fn erase(&self) -> Expr {
        match &self.kind {
            SpannedExprKind::Variable(name) => Expr::Variable(name.clone()),
            SpannedExprKind::StringLiteral(s) => Expr::StringLiteral(s.clone()),
            SpannedExprKind::NumberLiteral(n) => Expr::NumberLiteral(*n),
            SpannedExprKind::BoolLiteral(b) => Expr::BoolLiteral(*b),
            SpannedExprKind::ArrayLiteral(items) => {
                Expr::ArrayLiteral(items.iter().map(SpannedExpr::erase).collect())
            }
            SpannedExprKind::ObjectLiteral(entries) => Expr::ObjectLiteral(
                entries
                    .iter()
                    .map(|(key, value)| (key.value.clone(), value.erase()))
                    .collect(),
            ),
            SpannedExprKind::UnaryNot(inner) => Expr::UnaryNot(Box::new(inner.erase())),
            SpannedExprKind::UnaryMinus(inner) => Expr::UnaryMinus(Box::new(inner.erase())),
            SpannedExprKind::Paren(inner) => Expr::Paren(Box::new(inner.erase())),
            SpannedExprKind::Binary { op, left, right } => Expr::Binary {
                op: *op,
                left: Box::new(left.erase()),
                right: Box::new(right.erase()),
            },
            SpannedExprKind::Index { base, index } => Expr::Index {
                base: Box::new(base.erase()),
                index: Box::new(index.erase()),
            },
            SpannedExprKind::MemberAccess { base, name } => Expr::MemberAccess {
                base: Box::new(base.erase()),
                name: name.clone(),
            },
            SpannedExprKind::Fallback { primary, fallback } => Expr::Fallback {
                primary: Box::new(primary.erase()),
                fallback: Box::new(fallback.erase()),
            },
            SpannedExprKind::Ternary {
                condition,
                then_branch,
                else_branch,
            } => Expr::Ternary {
                condition: Box::new(condition.erase()),
                then_branch: Box::new(then_branch.erase()),
                else_branch: Box::new(else_branch.erase()),
            },
            SpannedExprKind::Comparison { left, op, right } => Expr::Comparison {
                left: Box::new(left.erase()),
                op: *op,
                right: Box::new(right.erase()),
            },
            SpannedExprKind::FunctionCall { name, args } => Expr::FunctionCall {
                name: name.clone(),
                args: args.iter().map(SpannedExpr::erase).collect(),
            },
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
