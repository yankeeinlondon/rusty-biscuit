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
pub mod ctx;
pub mod lexer;
pub mod parser;

pub use ast::Expr;
pub use ctx::CtxLookup;
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
    /// ## Returns
    ///
    /// - `Some(Value)` when the path resolves to a value
    /// - `None` when the path does not resolve
    fn get(&self, path: &str) -> Option<Value>;

    /// Looks up a value by path, coercing to a string.
    ///
    /// ## Returns
    ///
    /// - The string representation of the value when the path resolves
    /// - An empty string when the path does not resolve or the value is null
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
        Value::Number(n) => {
            if let Some(f) = n.as_f64()
                && f.fract() == 0.0
            {
                return format!("{}", f as i64);
            }
            n.to_string()
        }
        Value::String(s) => s.clone(),
        Value::Array(_) | Value::Object(_) => value.to_string(),
    }
}

/// Evaluates an expression against a lookup to produce a JSON value.
///
/// This is the core expression evaluator shared by both condition and
/// interpolation evaluation. It handles all expression types including
/// literals, variables, operators, comparisons, and function calls.
///
/// ## Returns
///
/// - `Ok(Value)` with the evaluated JSON value on success
/// - `Err(String)` with a human-readable message on evaluation failure
///
/// ## Examples
///
/// ```
/// use darkmatter::markdown::compose::expression::{evaluate, EvaluationLookup, Expr};
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
/// let expr = Expr::Variable("name".to_string());
/// assert_eq!(evaluate(&expr, &lookup).unwrap(), json!("Alice"));
/// ```
pub fn evaluate<L: EvaluationLookup>(expr: &Expr, lookup: &L) -> Result<Value, String> {
    match expr {
        Expr::Variable(path) => Ok(lookup.get(path).unwrap_or(Value::Null)),
        Expr::StringLiteral(s) => Ok(Value::String(s.clone())),
        Expr::NumberLiteral(n) => {
            let num = if n.fract() == 0.0 {
                serde_json::Number::from(*n as i64)
            } else {
                serde_json::Number::from_f64(*n)
                    .ok_or_else(|| format!("Invalid numeric literal: {n}"))?
            };
            Ok(Value::Number(num))
        }
        Expr::BoolLiteral(b) => Ok(Value::Bool(*b)),
        Expr::Paren(inner) => evaluate(inner, lookup),
        Expr::UnaryNot(inner) => {
            let value = evaluate(inner, lookup)?;
            Ok(Value::Bool(!is_truthy(&value)))
        }
        Expr::Fallback { primary, fallback } => {
            let primary = evaluate(primary, lookup)?;
            if is_truthy(&primary) {
                Ok(primary)
            } else {
                evaluate(fallback, lookup)
            }
        }
        Expr::Ternary {
            condition,
            then_branch,
            else_branch,
        } => {
            let condition = evaluate(condition, lookup)?;
            if is_truthy(&condition) {
                evaluate(then_branch, lookup)
            } else {
                evaluate(else_branch, lookup)
            }
        }
        Expr::Comparison { left, op, right } => {
            let left = evaluate(left, lookup)?;
            let right = evaluate(right, lookup)?;

            // Null-safe comparisons: when both sides are Null (undefined),
            // equality and inequality both return false. Comparing two
            // unknown values yields no meaningful result.
            let both_null = left.is_null() && right.is_null();

            let outcome = match op {
                ComparisonOp::Equal => !both_null && scalar_string(&left) == scalar_string(&right),
                ComparisonOp::NotEqual => {
                    !both_null && scalar_string(&left) != scalar_string(&right)
                }
                ComparisonOp::GreaterThan => to_number_coerce(&left) > to_number_coerce(&right),
                ComparisonOp::GreaterThanOrEqual => {
                    to_number_coerce(&left) >= to_number_coerce(&right)
                }
                ComparisonOp::LessThan => to_number_coerce(&left) < to_number_coerce(&right),
            };
            Ok(Value::Bool(outcome))
        }
        Expr::FunctionCall { name, args } => evaluate_function(name, args, lookup),
    }
}

fn evaluate_function<L: EvaluationLookup>(
    name: &str,
    args: &[Expr],
    lookup: &L,
) -> Result<Value, String> {
    let name = name.to_ascii_lowercase();
    match name.as_str() {
        "and" => {
            for arg in args {
                let value = evaluate(arg, lookup)?;
                if !is_truthy(&value) {
                    return Ok(Value::Bool(false));
                }
            }
            Ok(Value::Bool(true))
        }
        "or" => {
            for arg in args {
                let value = evaluate(arg, lookup)?;
                if is_truthy(&value) {
                    return Ok(Value::Bool(true));
                }
            }
            Ok(Value::Bool(false))
        }
        "haskey" | "has_key" => {
            if args.len() < 2 {
                return Err("HasKey() requires 2 arguments".to_string());
            }
            let object = evaluate(&args[0], lookup)?;
            let key = scalar_string(&evaluate(&args[1], lookup)?);
            let has = object
                .as_object()
                .map(|obj| obj.contains_key(&key))
                .unwrap_or(false);
            Ok(Value::Bool(has))
        }
        "contains" => {
            if args.len() < 2 {
                return Err("Contains() requires 2 arguments".to_string());
            }
            let haystack = evaluate(&args[0], lookup)?;
            let needle = evaluate(&args[1], lookup)?;
            let found = match haystack {
                Value::Array(values) => values
                    .iter()
                    .any(|value| scalar_string(value) == scalar_string(&needle)),
                Value::Object(values) => values
                    .values()
                    .any(|value| scalar_string(value) == scalar_string(&needle)),
                Value::String(value) => value.contains(&scalar_string(&needle)),
                value => scalar_string(&value).contains(&scalar_string(&needle)),
            };
            Ok(Value::Bool(found))
        }
        "length" => {
            if args.is_empty() {
                return Err("Length() requires 1 argument".to_string());
            }
            let value = evaluate(&args[0], lookup)?;
            let len = match value {
                Value::String(s) => s.chars().count(),
                Value::Array(arr) => arr.len(),
                Value::Object(obj) => obj.len(),
                Value::Number(n) => n.to_string().chars().count(),
                Value::Bool(_) => 0,
                Value::Null => 0,
            };
            Ok(Value::Number(serde_json::Number::from(len)))
        }
        "number" => {
            if args.is_empty() {
                return Err("number() requires at least 1 argument".to_string());
            }
            let value = evaluate(&args[0], lookup)?;
            let default = if args.len() > 1 {
                to_number_coerce(&evaluate(&args[1], lookup)?)
            } else {
                0.0
            };
            let number = to_number(&value).unwrap_or(default);
            let json_number = if number.fract() == 0.0 {
                serde_json::Number::from(number as i64)
            } else {
                serde_json::Number::from_f64(number)
                    .ok_or_else(|| "Unable to represent number".to_string())?
            };
            Ok(Value::Number(json_number))
        }
        "round" => {
            if args.is_empty() {
                return Err("round() requires at least 1 argument".to_string());
            }
            let value = evaluate(&args[0], lookup)?;
            let default = if args.len() > 1 {
                to_number_coerce(&evaluate(&args[1], lookup)?)
            } else {
                0.0
            };
            let number = to_number(&value).unwrap_or(default).round() as i64;
            Ok(Value::Number(serde_json::Number::from(number)))
        }
        _ => Err(format!("Unknown function: {name}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::collections::HashMap;

    struct TestLookup {
        data: HashMap<String, Value>,
    }

    impl EvaluationLookup for TestLookup {
        fn get(&self, path: &str) -> Option<Value> {
            self.data.get(path).cloned()
        }
    }

    fn lookup(data: Value) -> TestLookup {
        let map: HashMap<String, Value> = match data {
            Value::Object(obj) => obj.into_iter().collect(),
            _ => HashMap::new(),
        };
        TestLookup { data: map }
    }

    mod parity {
        use super::*;

        #[test]
        fn interpolation_fallback_versus_condition_or() {
            // Interpolation: a || "default" -> Fallback
            // Condition: a || "default" -> Or(a, "default")
            // Both should return the truthy value
            let state = lookup(json!({"a": "present"}));
            let fallback_expr = Expr::Fallback {
                primary: Box::new(Expr::Variable("a".to_string())),
                fallback: Box::new(Expr::StringLiteral("default".to_string())),
            };
            let or_expr = Expr::FunctionCall {
                name: "Or".to_string(),
                args: vec![
                    Expr::Variable("a".to_string()),
                    Expr::StringLiteral("default".to_string()),
                ],
            };

            assert_eq!(evaluate(&fallback_expr, &state).unwrap(), json!("present"));
            assert_eq!(evaluate(&or_expr, &state).unwrap(), json!(true));
        }

        #[test]
        fn interpolation_fallback_uses_fallback_when_falsy() {
            let state = lookup(json!({}));
            let expr = Expr::Fallback {
                primary: Box::new(Expr::Variable("missing".to_string())),
                fallback: Box::new(Expr::StringLiteral("default".to_string())),
            };

            assert_eq!(evaluate(&expr, &state).unwrap(), json!("default"));
        }

        #[test]
        fn condition_or_short_circuits_on_true() {
            let state = lookup(json!({"truthy_flag": true}));
            let expr = Expr::FunctionCall {
                name: "Or".to_string(),
                args: vec![
                    Expr::Variable("truthy_flag".to_string()),
                    Expr::FunctionCall {
                        name: "UnknownFn".to_string(),
                        args: vec![Expr::Variable("x".to_string())],
                    },
                ],
            };

            assert_eq!(evaluate(&expr, &state).unwrap(), json!(true));
        }

        #[test]
        fn condition_and_short_circuits_on_false() {
            let state = lookup(json!({}));
            let expr = Expr::FunctionCall {
                name: "And".to_string(),
                args: vec![
                    Expr::Variable("false_flag".to_string()),
                    Expr::FunctionCall {
                        name: "UnknownFn".to_string(),
                        args: vec![Expr::Variable("x".to_string())],
                    },
                ],
            };

            assert_eq!(evaluate(&expr, &state).unwrap(), json!(false));
        }

        #[test]
        fn comparison_behavior_is_consistent() {
            let state = lookup(json!({"count": 5, "name": "alice"}));

            // Numeric comparison
            let expr = Expr::Comparison {
                left: Box::new(Expr::Variable("count".to_string())),
                op: ComparisonOp::GreaterThan,
                right: Box::new(Expr::NumberLiteral(0.0)),
            };
            assert_eq!(evaluate(&expr, &state).unwrap(), json!(true));

            // String equality
            let expr = Expr::Comparison {
                left: Box::new(Expr::Variable("name".to_string())),
                op: ComparisonOp::Equal,
                right: Box::new(Expr::StringLiteral("alice".to_string())),
            };
            assert_eq!(evaluate(&expr, &state).unwrap(), json!(true));

            // Null-safe equality: null == null is false
            let expr = Expr::Comparison {
                left: Box::new(Expr::Variable("missing_a".to_string())),
                op: ComparisonOp::Equal,
                right: Box::new(Expr::Variable("missing_b".to_string())),
            };
            assert_eq!(evaluate(&expr, &state).unwrap(), json!(false));
        }

        #[test]
        fn helper_functions_return_same_values() {
            let state = lookup(json!({"items": [1, 2, 3], "user": {"name": "Alice"}}));

            // length
            let expr = Expr::FunctionCall {
                name: "length".to_string(),
                args: vec![Expr::Variable("items".to_string())],
            };
            assert_eq!(evaluate(&expr, &state).unwrap(), json!(3));

            // haskey
            let expr = Expr::FunctionCall {
                name: "HasKey".to_string(),
                args: vec![
                    Expr::Variable("user".to_string()),
                    Expr::StringLiteral("name".to_string()),
                ],
            };
            assert_eq!(evaluate(&expr, &state).unwrap(), json!(true));
        }

        #[test]
        fn bool_literal_evaluates_to_json_bool() {
            assert_eq!(evaluate(&Expr::BoolLiteral(true), &lookup(json!({}))).unwrap(), json!(true));
            assert_eq!(evaluate(&Expr::BoolLiteral(false), &lookup(json!({}))).unwrap(), json!(false));
        }

        #[test]
        fn paren_evaluates_inner_expression() {
            let state = lookup(json!({"name": "Alice"}));
            let expr = Expr::Paren(Box::new(Expr::Variable("name".to_string())));
            assert_eq!(evaluate(&expr, &state).unwrap(), json!("Alice"));
        }

        #[test]
        fn ternary_with_bool_literals() {
            let state = lookup(json!({}));
            let expr = Expr::Ternary {
                condition: Box::new(Expr::BoolLiteral(true)),
                then_branch: Box::new(Expr::StringLiteral("yes".to_string())),
                else_branch: Box::new(Expr::StringLiteral("no".to_string())),
            };
            assert_eq!(evaluate(&expr, &state).unwrap(), json!("yes"));

            let expr = Expr::Ternary {
                condition: Box::new(Expr::BoolLiteral(false)),
                then_branch: Box::new(Expr::StringLiteral("yes".to_string())),
                else_branch: Box::new(Expr::StringLiteral("no".to_string())),
            };
            assert_eq!(evaluate(&expr, &state).unwrap(), json!("no"));
        }
    }
}
