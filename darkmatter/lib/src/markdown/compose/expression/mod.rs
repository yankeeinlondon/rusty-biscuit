//! Core expression parsing and evaluation foundation.
//!
//! This module provides the shared AST, lexer, parser, and lookup trait
//! used by both interpolation and condition evaluation. It is the foundation
//! for expression processing in the compose pipeline.
//!
//! ## Components
//!
//! - **AST** (`ast::Expr`) - Abstract syntax tree for expressions
//! - **Lexer** (`lexer::Lexer`) - Tokenizes expression strings
//! - **Parser** (`parser::Parser`) - Builds AST from tokens
//! - **EvaluationLookup** - Trait for resolving variable lookups
//!
//! ## Expression Syntax
//!
//! Variables:
//! - `foo` - Simple variable
//! - `user.name` - Nested property access
//! - `ctx.today` - Context variable
//! - `env.HOME` - Environment variable
//!
//! Operators:
//! - `||` - Fallback (interpolation mode) or logical OR (condition mode)
//! - `&&` - Logical AND (condition mode only)
//! - `==`, `!=`, `>`, `>=`, `<` - Comparisons
//! - `!` - Unary NOT
//! - `? :` - Ternary conditional
//!
//! ## Parser Modes
//!
//! - **Interpolation** (`ParseMode::Interpolation`) - `||` is fallback operator
//! - **Condition** (`ParseMode::Condition`) - `||` is logical OR, `&&` is logical AND

pub mod ast;
pub mod lexer;
pub mod parser;

pub use ast::Expr;
pub use lexer::{
    ComparisonOp, ExpressionFinder, ExpressionLocation, Lexer, LexerError, ParseMode, Token,
};
pub use parser::{ParseError, Parser, parse, parse_condition};

use serde_json::Value;

/// Trait for types that can resolve expression variable lookups.
///
/// Implementors provide path-based access to values for use during
/// expression evaluation. Both interpolation and condition evaluation
/// use this trait to resolve variable references.
///
/// ## Lookup Paths
///
/// Paths use dot notation for nested access:
/// - `name` - Top-level key
/// - `user.email` - Nested object property
/// - `ctx.today` - Runtime context value
/// - `env.HOME` - Environment variable
///
/// ## Examples
///
/// ```
/// use darkmatter::markdown::compose::expression::EvaluationLookup;
/// use serde_json::{Value, json};
/// use std::collections::HashMap;
///
/// struct SimpleLookup {
///     data: HashMap<String, Value>,
/// }
///
/// impl EvaluationLookup for SimpleLookup {
///     fn get(&self, path: &str) -> Option<Value> {
///         self.data.get(path).cloned()
///     }
/// }
///
/// let lookup = SimpleLookup {
///     data: [("name".to_string(), json!("Alice"))].into(),
/// };
/// assert_eq!(lookup.get("name"), Some(json!("Alice")));
/// ```
pub trait EvaluationLookup {
    /// Looks up a value by dotted path.
    ///
    /// Returns `None` if the path does not resolve to a value.
    fn get(&self, path: &str) -> Option<Value>;

    /// Looks up a value by path, coercing to a string.
    ///
    /// Returns an empty string if the path does not resolve.
    fn get_string(&self, path: &str) -> String {
        match self.get(path) {
            None | Some(Value::Null) => String::new(),
            Some(Value::String(s)) => s,
            Some(Value::Number(n)) => n.to_string(),
            Some(Value::Bool(b)) => b.to_string(),
            Some(v) => v.to_string(),
        }
    }
}

/// Checks if a JSON value is truthy.
///
/// ## Truthy Values
///
/// - Non-empty strings
/// - Non-zero numbers
/// - Boolean `true`
/// - Non-empty arrays
/// - Non-empty objects
///
/// ## Falsy Values
///
/// - Null
/// - Empty string
/// - Number `0` (or `0.0`)
/// - Boolean `false`
/// - Empty array
/// - Empty object
pub fn is_truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(v) => *v,
        Value::Number(n) => n.as_f64().unwrap_or(0.0) != 0.0,
        Value::String(s) => !s.is_empty(),
        Value::Array(values) => !values.is_empty(),
        Value::Object(values) => !values.is_empty(),
    }
}

/// Converts a JSON value to a number.
///
/// - `Number` -> `Some(n)`
/// - `String` -> `Some(n)` if parseable as f64
/// - `Bool` -> `Some(1.0)` for true, `Some(0.0)` for false
/// - `Null` -> `None`
/// - `Array` | `Object` -> `None`
pub fn to_number(value: &Value) -> Option<f64> {
    match value {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.parse::<f64>().ok(),
        Value::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
        Value::Null => None,
        Value::Array(_) | Value::Object(_) => None,
    }
}

/// Converts a JSON value to a number, coercing unparseable values to 0.0.
pub fn to_number_coerce(value: &Value) -> f64 {
    to_number(value).unwrap_or(0.0)
}

/// Converts a JSON value to its scalar string representation.
///
/// - `Null` -> empty string
/// - `Bool` -> `"true"` or `"false"`
/// - `Number` -> string representation
/// - `String` -> the string itself
/// - `Array` | `Object` -> JSON string representation
pub fn scalar_string(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => s.clone(),
        Value::Array(_) | Value::Object(_) => value.to_string(),
    }
}
