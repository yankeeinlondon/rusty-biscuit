//! Interpolation expression processing for the compose pipeline.
//!
//! This module provides the interpolation system for expanding `{{variable}}`
//! expressions in markdown content. The system is built in layers:
//!
//! - **Lexer** - Tokenizes interpolation expressions
//! - **Parser** - Builds an AST from tokens
//! - **Evaluator** - Evaluates expressions against state (Phase 3C/3D)
//!
//! ## Expression Syntax
//!
//! Basic variables:
//! - `{{ foo }}` - Simple variable lookup
//! - `{{ user.name }}` - Nested property access
//! - `{{ ctx.today }}` - Context property (date/time values)
//! - `{{ env.HOME }}` - Environment variable
//!
//! Fallback values:
//! - `{{ color || "unknown" }}` - Use "unknown" if color is falsy
//!
//! Ternary expressions:
//! - `{{ color ? "known" : "unknown" }}` - Boolean switch
//! - `{{ show ? has_name ? name : "unnamed" : "hidden" }}` - Nested ternary
//!
//! Comparisons:
//! - `{{ count > 0 ? "has items" : "empty" }}` - Numeric comparison
//!
//! Function calls:
//! - `{{ length(items) }}` - Single-argument function
//! - `{{ number(value, 0) }}` - Multi-argument function with default
//!
//! ## Operator Precedence
//!
//! Precedence from highest to lowest:
//! 1. **Function calls** - `length(x)`, `number(x, 0)`
//! 2. **Comparison** - `==`, `!=`, `>`, `>=`, `<`
//! 3. **Fallback** - `||`
//! 4. **Ternary** - `? :`
//!
//! ## Code Exclusion
//!
//! Interpolation placeholders inside fenced or indented code blocks are
//! NOT processed. Inline code spans (single backticks) ARE processed by
//! default, since the templating pattern `` `var_{{ phase }}` `` is a
//! common use case.
//!
//! ## Examples
//!
//! ```
//! use darkmatter::markdown::compose::interpolation::{parse, Expr};
//!
//! // Parse a simple variable
//! let expr = parse("foo").unwrap();
//! assert!(matches!(expr, Expr::Variable(name) if name == "foo"));
//!
//! // Parse a fallback expression
//! let expr = parse(r#"color || "unknown""#).unwrap();
//! assert!(matches!(expr, Expr::Fallback { .. }));
//!
//! // Parse a ternary with comparison
//! let expr = parse(r#"count > 0 ? "items" : "empty""#).unwrap();
//! assert!(matches!(expr, Expr::Ternary { .. }));
//!
//! // Parse a nested ternary
//! let expr = parse(r#"a ? b ? "c" : "d" : "e""#).unwrap();
//! assert!(matches!(expr, Expr::Ternary { .. }));
//! ```

mod evaluator;
#[cfg(test)]
mod fatality_characterization;
pub(crate) mod rewrite;

// Re-export core expression types from the expression module for backward
// compatibility. The canonical location is now `compose::expression`.
pub use super::expression::{
    ArityBound, ComparisonOp, Expr, ExpressionError, ExpressionFinder, ExpressionLocation,
    FileRefFailure, FileReferenceDiagnostic, Lexer, LexerError, ParseError, ParseMode, Parser,
    Token, parse, parse_condition,
};

// Re-export the lookup trait with its old name for backward compatibility.
pub use super::expression::EvaluationLookup as InterpolationLookup;

pub use evaluator::{EvalResult, EvalValue, Evaluator};
pub(crate) use rewrite::{ScanMode, convert_literals, interpolate_text, interpolate_value};
