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
//! - `{{ color | "unknown" }}` - Use "unknown" if color is falsy
//!
//! Ternary expressions:
//! - `{{ color ? "known" : "unknown" }}` - Boolean switch
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
//! 3. **Fallback** - `|`
//! 4. **Ternary** - `? :`
//!
//! ## Code Exclusion
//!
//! Interpolation placeholders inside code spans or fenced code blocks
//! are NOT processed. This preserves code examples that might contain
//! template syntax.
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
//! let expr = parse(r#"color | "unknown""#).unwrap();
//! assert!(matches!(expr, Expr::Fallback { .. }));
//!
//! // Parse a ternary with comparison
//! let expr = parse(r#"count > 0 ? "items" : "empty""#).unwrap();
//! assert!(matches!(expr, Expr::Ternary { .. }));
//! ```

mod ast;
mod evaluator;
mod lexer;
mod parser;
pub(crate) mod rewrite;

pub use ast::Expr;
pub use evaluator::{EvalResult, EvalValue, Evaluator, InterpolationLookup};
pub use lexer::{
    ComparisonOp, ExpressionFinder, ExpressionLocation, Lexer, LexerError, ParseMode, Token,
};
pub use parser::{ParseError, Parser, parse, parse_condition};
pub(crate) use rewrite::{ScanMode, interpolate_text};
