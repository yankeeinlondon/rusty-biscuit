//! Evaluator for interpolation expressions.
//!
//! This module evaluates parsed AST expressions against state to produce
//! string output. The evaluator traverses the AST and resolves variables,
//! applies operators, and calls functions.
//!
//! ## Expression Evaluation
//!
//! - **Variables**: Resolved against `EffectiveState`
//!   - `{{ title }}` -> frontmatter/state lookup
//!   - `{{ user.name }}` -> nested object lookup
//!   - `{{ ctx.today }}` -> runtime context
//!   - `{{ env.HOME }}` -> environment variable
//!   - Unresolved variables evaluate to empty string
//!
//! - **Literals**: Return their string representation
//!   - `{{ "hello" }}` -> `"hello"`
//!   - `{{ 42 }}` -> `"42"`
//!
//! - **Fallback**: Returns primary if truthy, otherwise fallback
//!   - `{{ color | "unknown" }}` -> "unknown" if color is falsy
//!   - Chained: `{{ a | b | c }}` -> first truthy wins
//!
//! - **Ternary**: Boolean conditional
//!   - `{{ active ? "yes" : "no" }}` -> "yes" if active is truthy
//!
//! - **Comparison**: Evaluates to boolean for use in ternary
//!   - `{{ count > 0 ? "items" : "empty" }}`
//!   - Equality (`==`, `!=`): compares string representations
//!   - Ordering (`>`, `>=`, `<`): numeric comparison, false if non-numeric
//!
//! - **Functions**: Helper utilities for value transformation
//!   - `{{ length(name) }}` - character/array/object length
//!   - `{{ number("42") }}` - convert to number
//!   - `{{ round(3.7) }}` - round to integer
//!
//! ## Truthiness
//!
//! Values are considered falsy if they are:
//! - Empty string
//! - Null
//! - Boolean false
//! - Number 0
//!
//! ## Examples
//!
//! ```
//! use darkmatter::markdown::transform::interpolation::{parse, Evaluator, EvalResult};
//! use darkmatter::markdown::transform::EffectiveStateBuilder;
//! use serde_json::json;
//! use std::collections::HashMap;
//!
//! let mut fm = HashMap::new();
//! fm.insert("name".to_string(), json!("Alice"));
//!
//! let state = EffectiveStateBuilder::new()
//!     .with_frontmatter(fm)
//!     .build();
//!
//! let evaluator = Evaluator::new(&state);
//! let expr = parse("name").unwrap();
//!
//! match evaluator.eval(&expr) {
//!     EvalResult::Value(s) => assert_eq!(s, "Alice"),
//!     EvalResult::Error { .. } => panic!("Expected Value"),
//! }
//! ```

use super::super::state::EffectiveState;
use super::ComparisonOp;
use super::ast::Expr;
use serde_json::Value;

/// Result of evaluating an expression.
#[derive(Debug, Clone, PartialEq)]
pub enum EvalResult {
    /// Successful evaluation producing a string.
    Value(String),

    /// Evaluation failed.
    ///
    /// The `original` field contains the expression string representation
    /// for error recovery (e.g., preserving unprocessed expressions).
    Error {
        /// Human-readable error message.
        message: String,
        /// Original expression for error recovery.
        original: String,
    },
}

impl EvalResult {
    /// Returns the value if successful, or the original expression if error.
    pub fn unwrap_or_original(self) -> String {
        match self {
            EvalResult::Value(s) => s,
            EvalResult::Error { original, .. } => format!("{{{{ {} }}}}", original),
        }
    }

    /// Returns true if this is a successful evaluation.
    pub fn is_value(&self) -> bool {
        matches!(self, EvalResult::Value(_))
    }

    /// Returns true if this is an error.
    pub fn is_error(&self) -> bool {
        matches!(self, EvalResult::Error { .. })
    }
}

/// Value representation for truthiness checks and comparisons.
///
/// This enum represents the runtime value of an expression before
/// string conversion, allowing truthiness tests and type-aware comparisons.
#[derive(Debug, Clone, PartialEq)]
pub enum EvalValue {
    /// String value.
    String(String),
    /// Numeric value.
    Number(f64),
    /// Null/missing value.
    Null,
    /// Boolean value.
    Bool(bool),
}

impl EvalValue {
    /// Checks if the value is truthy.
    ///
    /// ## Falsy Values
    ///
    /// - Empty string
    /// - Null
    /// - Boolean `false`
    /// - Number `0` (or `0.0`)
    ///
    /// All other values are truthy.
    pub fn is_truthy(&self) -> bool {
        match self {
            EvalValue::String(s) => !s.is_empty(),
            EvalValue::Number(n) => *n != 0.0,
            EvalValue::Null => false,
            EvalValue::Bool(b) => *b,
        }
    }

    /// Converts the value to its string representation.
    ///
    /// - `String` -> the string itself
    /// - `Number` -> string representation (integers without decimal)
    /// - `Null` -> empty string
    /// - `Bool` -> `"true"` or `"false"`
    pub fn as_string(&self) -> String {
        match self {
            EvalValue::String(s) => s.clone(),
            EvalValue::Number(n) => {
                if n.fract() == 0.0 {
                    format!("{}", *n as i64)
                } else {
                    n.to_string()
                }
            }
            EvalValue::Null => String::new(),
            EvalValue::Bool(b) => b.to_string(),
        }
    }

    /// Creates an `EvalValue` from a JSON `Value`.
    pub fn from_json(value: &Value) -> Self {
        match value {
            Value::Null => EvalValue::Null,
            Value::Bool(b) => EvalValue::Bool(*b),
            Value::Number(n) => EvalValue::Number(n.as_f64().unwrap_or(0.0)),
            Value::String(s) => EvalValue::String(s.clone()),
            // Arrays and objects convert to their JSON string representation
            v => EvalValue::String(v.to_string()),
        }
    }

    /// Attempts to convert the value to a number for comparison.
    ///
    /// - `Number` -> `Some(n)`
    /// - `String` -> `Some(n)` if parseable as f64
    /// - `Bool` -> `Some(1.0)` for true, `Some(0.0)` for false
    /// - `Null` -> `None`
    pub fn as_number(&self) -> Option<f64> {
        match self {
            EvalValue::Number(n) => Some(*n),
            EvalValue::String(s) => s.parse::<f64>().ok(),
            EvalValue::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
            EvalValue::Null => None,
        }
    }
}

/// Evaluator for interpolation expressions.
///
/// The evaluator takes parsed AST expressions and evaluates them against
/// an `EffectiveState` to produce string output.
pub struct Evaluator<'a> {
    state: &'a EffectiveState,
}

impl<'a> Evaluator<'a> {
    /// Creates a new evaluator with the given state.
    pub fn new(state: &'a EffectiveState) -> Self {
        Self { state }
    }

    /// Evaluates an expression to a string result.
    ///
    /// ## Returns
    ///
    /// - `EvalResult::Value` with the evaluated string on success
    /// - `EvalResult::Error` with details if evaluation fails
    ///
    /// ## Supported Expressions
    ///
    /// - Variable resolution (simple and nested paths)
    /// - String and number literals
    /// - Fallback expressions (`a | b`)
    /// - Ternary expressions (`a ? b : c`)
    /// - Comparison expressions (`a == b`, `a > b`, etc.)
    /// - Function calls (`length()`, `number()`, `round()`)
    pub fn eval(&self, expr: &Expr) -> EvalResult {
        match expr {
            Expr::Variable(name) => {
                let value = self.state.get_string(name);
                EvalResult::Value(value)
            }

            Expr::StringLiteral(s) => EvalResult::Value(s.clone()),

            Expr::NumberLiteral(n) => {
                let s = if n.fract() == 0.0 {
                    format!("{}", *n as i64)
                } else {
                    n.to_string()
                };
                EvalResult::Value(s)
            }

            Expr::UnaryNot(expr) => {
                let v = self.eval_value(expr);
                EvalResult::Value((!v.is_truthy()).to_string())
            }

            Expr::Fallback { primary, fallback } => {
                // Evaluate primary; if truthy, use it; otherwise evaluate fallback
                let pv = self.eval_value(primary);
                if pv.is_truthy() {
                    EvalResult::Value(pv.as_string())
                } else {
                    // Recursively evaluate fallback (handles chained fallbacks)
                    self.eval(fallback)
                }
            }

            Expr::Ternary {
                condition,
                then_branch,
                else_branch,
            } => {
                // Evaluate condition; pick branch based on truthiness
                let cv = self.eval_value(condition);
                if cv.is_truthy() {
                    self.eval(then_branch)
                } else {
                    self.eval(else_branch)
                }
            }

            Expr::Comparison { left, op, right } => {
                // Comparison evaluates to a boolean string representation
                let result = self.compare(left, op, right);
                EvalResult::Value(result.to_string())
            }

            Expr::FunctionCall { name, args } => self.eval_function(name, args),
        }
    }

    /// Evaluates an expression to a value (for truthiness checks).
    ///
    /// Unlike `eval()`, this returns an `EvalValue` that preserves type
    /// information for truthiness tests and comparisons.
    ///
    /// ## Supported Expressions
    ///
    /// - Variable resolution (with type preservation from JSON)
    /// - String literals
    /// - Number literals
    /// - Fallback expressions (returns first truthy value)
    /// - Ternary expressions (returns selected branch value)
    /// - Comparison expressions (returns Bool)
    /// - Function calls (returns appropriate type based on function)
    pub fn eval_value(&self, expr: &Expr) -> EvalValue {
        match expr {
            Expr::Variable(name) => match self.state.get(name) {
                Some(v) => EvalValue::from_json(&v),
                None => EvalValue::Null,
            },

            Expr::StringLiteral(s) => EvalValue::String(s.clone()),

            Expr::NumberLiteral(n) => EvalValue::Number(*n),

            Expr::UnaryNot(expr) => EvalValue::Bool(!self.eval_value(expr).is_truthy()),

            Expr::Fallback { primary, fallback } => {
                let pv = self.eval_value(primary);
                if pv.is_truthy() {
                    pv
                } else {
                    self.eval_value(fallback)
                }
            }

            Expr::Ternary {
                condition,
                then_branch,
                else_branch,
            } => {
                if self.eval_value(condition).is_truthy() {
                    self.eval_value(then_branch)
                } else {
                    self.eval_value(else_branch)
                }
            }

            Expr::Comparison { left, op, right } => {
                let result = self.compare(left, op, right);
                EvalValue::Bool(result)
            }

            Expr::FunctionCall { name, args } => {
                // Functions return numbers - extract from eval result
                match self.eval_function(name, args) {
                    EvalResult::Value(s) => s
                        .parse::<f64>()
                        .map(EvalValue::Number)
                        .unwrap_or(EvalValue::String(s)),
                    EvalResult::Error { .. } => EvalValue::Null,
                }
            }
        }
    }

    /// Evaluates a function call.
    ///
    /// ## Supported Functions
    ///
    /// - `length(value)` - Returns length of string, array, or object
    /// - `number(value, default = 0)` - Converts to number
    /// - `round(value, default = 0)` - Rounds to nearest integer
    fn eval_function(&self, name: &str, args: &[Expr]) -> EvalResult {
        let normalized = name.to_ascii_lowercase();
        match normalized.as_str() {
            "length" => self.fn_length(args),
            "number" => self.fn_number(args),
            "round" => self.fn_round(args),
            "haskey" | "has_key" => self.fn_has_key(args),
            "contains" => self.fn_contains(args),
            "and" => self.fn_and(args),
            "or" => self.fn_or(args),
            _ => EvalResult::Error {
                message: format!("Unknown function: {}", name),
                original: format!("{}(...)", name),
            },
        }
    }

    /// Evaluates the `length()` function.
    ///
    /// Returns the length of a value:
    /// - String: character count
    /// - Array: element count
    /// - Object: key count
    /// - Number: digit count of string representation
    /// - Null/missing: 0
    /// - Bool: 1
    fn fn_length(&self, args: &[Expr]) -> EvalResult {
        if args.is_empty() {
            return EvalResult::Error {
                message: "length() requires 1 argument".to_string(),
                original: "length()".to_string(),
            };
        }

        // Check for array/object first by looking at raw JSON
        if let Expr::Variable(path) = &args[0]
            && let Some(json) = self.state.get(path)
        {
            if let Some(arr) = json.as_array() {
                return EvalResult::Value(arr.len().to_string());
            }
            if let Some(obj) = json.as_object() {
                return EvalResult::Value(obj.len().to_string());
            }
        }

        // For other expressions, evaluate and compute length
        let value = self.eval_value(&args[0]);
        let len = match value {
            EvalValue::String(s) => s.chars().count(),
            EvalValue::Number(n) => {
                // Format as integer if no fractional part
                if n.fract() == 0.0 {
                    format!("{}", n as i64).len()
                } else {
                    n.to_string().len()
                }
            }
            EvalValue::Null => 0,
            EvalValue::Bool(_) => 1,
        };

        EvalResult::Value(len.to_string())
    }

    /// Evaluates the `number()` function.
    ///
    /// Converts a value to a number:
    /// - Already numeric: return as-is
    /// - String: parse as f64
    /// - Other: return default (0 if not specified)
    fn fn_number(&self, args: &[Expr]) -> EvalResult {
        if args.is_empty() {
            return EvalResult::Error {
                message: "number() requires at least 1 argument".to_string(),
                original: "number()".to_string(),
            };
        }

        let default = if args.len() > 1 {
            match self.eval_value(&args[1]) {
                EvalValue::Number(n) => n,
                _ => 0.0,
            }
        } else {
            0.0
        };

        let value = self.eval_value(&args[0]);
        let num = match value {
            EvalValue::Number(n) => n,
            EvalValue::String(s) => s.parse::<f64>().unwrap_or(default),
            _ => default,
        };

        // Format as integer if no fractional part
        let result = if num.fract() == 0.0 {
            format!("{}", num as i64)
        } else {
            num.to_string()
        };

        EvalResult::Value(result)
    }

    /// Evaluates the `round()` function.
    ///
    /// Rounds a number to the nearest integer:
    /// - Numeric: round and return
    /// - String: try parse, then round
    /// - Other: return default (0 if not specified)
    fn fn_round(&self, args: &[Expr]) -> EvalResult {
        if args.is_empty() {
            return EvalResult::Error {
                message: "round() requires at least 1 argument".to_string(),
                original: "round()".to_string(),
            };
        }

        let default = if args.len() > 1 {
            match self.eval_value(&args[1]) {
                EvalValue::Number(n) => n.round() as i64,
                _ => 0,
            }
        } else {
            0
        };

        let value = self.eval_value(&args[0]);
        let result = match value {
            EvalValue::Number(n) => n.round() as i64,
            EvalValue::String(s) => s
                .parse::<f64>()
                .map(|n| n.round() as i64)
                .unwrap_or(default),
            _ => default,
        };

        EvalResult::Value(result.to_string())
    }

    /// Evaluates `HasKey(object, key)`.
    fn fn_has_key(&self, args: &[Expr]) -> EvalResult {
        if args.len() < 2 {
            return EvalResult::Error {
                message: "HasKey() requires 2 arguments".to_string(),
                original: "HasKey()".to_string(),
            };
        }

        let key = self.eval_value(&args[1]).as_string();
        let found = match &args[0] {
            Expr::Variable(path) => self
                .state
                .get(path)
                .and_then(|v| v.as_object().map(|obj| obj.contains_key(&key)))
                .unwrap_or(false),
            _ => false,
        };

        EvalResult::Value(found.to_string())
    }

    /// Evaluates `Contains(collection, value)`.
    fn fn_contains(&self, args: &[Expr]) -> EvalResult {
        if args.len() < 2 {
            return EvalResult::Error {
                message: "Contains() requires 2 arguments".to_string(),
                original: "Contains()".to_string(),
            };
        }

        let needle = self.eval_value(&args[1]).as_string();
        let found = match &args[0] {
            Expr::Variable(path) => match self.state.get(path) {
                Some(Value::Array(values)) => values
                    .iter()
                    .any(|v| EvalValue::from_json(v).as_string() == needle),
                Some(Value::Object(values)) => values
                    .values()
                    .any(|v| EvalValue::from_json(v).as_string() == needle),
                Some(Value::String(s)) => s.contains(&needle),
                Some(v) => EvalValue::from_json(&v).as_string().contains(&needle),
                None => false,
            },
            _ => self.eval_value(&args[0]).as_string().contains(&needle),
        };

        EvalResult::Value(found.to_string())
    }

    /// Evaluates `And(a, b, c...)`.
    fn fn_and(&self, args: &[Expr]) -> EvalResult {
        let result = args.iter().all(|arg| self.eval_value(arg).is_truthy());
        EvalResult::Value(result.to_string())
    }

    /// Evaluates `Or(a, b, c...)`.
    fn fn_or(&self, args: &[Expr]) -> EvalResult {
        let result = args.iter().any(|arg| self.eval_value(arg).is_truthy());
        EvalResult::Value(result.to_string())
    }

    /// Performs a comparison between two expressions.
    ///
    /// ## Equality (`==`, `!=`)
    ///
    /// Compares string representations of both values.
    ///
    /// ## Ordering (`>`, `>=`, `<`)
    ///
    /// Attempts numeric comparison:
    /// - If both values can be converted to numbers, compares numerically
    /// - If either value is non-numeric, returns `false`
    fn compare(&self, left: &Expr, op: &ComparisonOp, right: &Expr) -> bool {
        let lv = self.eval_value(left);
        let rv = self.eval_value(right);

        match op {
            ComparisonOp::Equal => lv.as_string() == rv.as_string(),
            ComparisonOp::NotEqual => lv.as_string() != rv.as_string(),
            ComparisonOp::GreaterThan => match (lv.as_number(), rv.as_number()) {
                (Some(l), Some(r)) => l > r,
                _ => false,
            },
            ComparisonOp::GreaterThanOrEqual => match (lv.as_number(), rv.as_number()) {
                (Some(l), Some(r)) => l >= r,
                _ => false,
            },
            ComparisonOp::LessThan => match (lv.as_number(), rv.as_number()) {
                (Some(l), Some(r)) => l < r,
                _ => false,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::transform::interpolation::parse;
    use crate::markdown::transform::state::EffectiveStateBuilder;
    use crate::markdown::transform::types::TransformContext;
    use serde_json::json;
    use std::collections::HashMap;

    fn test_context() -> TransformContext {
        TransformContext::fixed_for_testing()
    }

    fn create_test_state(data: serde_json::Value) -> EffectiveState {
        let fm: HashMap<String, serde_json::Value> = match data {
            serde_json::Value::Object(obj) => obj.into_iter().collect(),
            _ => HashMap::new(),
        };

        EffectiveStateBuilder::new()
            .with_frontmatter(fm)
            .with_context(test_context())
            .build()
    }

    mod eval_result {
        use super::*;

        #[test]
        fn unwrap_or_original_value() {
            let result = EvalResult::Value("hello".to_string());
            assert_eq!(result.unwrap_or_original(), "hello");
        }

        #[test]
        fn unwrap_or_original_error() {
            let result = EvalResult::Error {
                message: "test error".to_string(),
                original: "foo | bar".to_string(),
            };
            assert_eq!(result.unwrap_or_original(), "{{ foo | bar }}");
        }

        #[test]
        fn is_value() {
            let value = EvalResult::Value("x".to_string());
            let error = EvalResult::Error {
                message: "e".to_string(),
                original: "o".to_string(),
            };

            assert!(value.is_value());
            assert!(!value.is_error());
            assert!(!error.is_value());
            assert!(error.is_error());
        }
    }

    mod eval_value {
        use super::*;

        #[test]
        fn string_truthiness() {
            assert!(EvalValue::String("hello".to_string()).is_truthy());
            assert!(!EvalValue::String(String::new()).is_truthy());
        }

        #[test]
        fn number_truthiness() {
            assert!(EvalValue::Number(1.0).is_truthy());
            assert!(EvalValue::Number(-1.0).is_truthy());
            assert!(EvalValue::Number(0.5).is_truthy());
            assert!(!EvalValue::Number(0.0).is_truthy());
        }

        #[test]
        fn null_truthiness() {
            assert!(!EvalValue::Null.is_truthy());
        }

        #[test]
        fn bool_truthiness() {
            assert!(EvalValue::Bool(true).is_truthy());
            assert!(!EvalValue::Bool(false).is_truthy());
        }

        #[test]
        fn as_string_string() {
            assert_eq!(EvalValue::String("hello".to_string()).as_string(), "hello");
        }

        #[test]
        fn as_string_number_integer() {
            assert_eq!(EvalValue::Number(42.0).as_string(), "42");
        }

        #[test]
        fn as_string_number_float() {
            assert_eq!(EvalValue::Number(3.14).as_string(), "3.14");
        }

        #[test]
        fn as_string_null() {
            assert_eq!(EvalValue::Null.as_string(), "");
        }

        #[test]
        fn as_string_bool() {
            assert_eq!(EvalValue::Bool(true).as_string(), "true");
            assert_eq!(EvalValue::Bool(false).as_string(), "false");
        }

        #[test]
        fn from_json_null() {
            assert_eq!(EvalValue::from_json(&json!(null)), EvalValue::Null);
        }

        #[test]
        fn from_json_bool() {
            assert_eq!(EvalValue::from_json(&json!(true)), EvalValue::Bool(true));
            assert_eq!(EvalValue::from_json(&json!(false)), EvalValue::Bool(false));
        }

        #[test]
        fn from_json_number() {
            assert_eq!(EvalValue::from_json(&json!(42)), EvalValue::Number(42.0));
            assert_eq!(EvalValue::from_json(&json!(3.14)), EvalValue::Number(3.14));
        }

        #[test]
        fn from_json_string() {
            assert_eq!(
                EvalValue::from_json(&json!("hello")),
                EvalValue::String("hello".to_string())
            );
        }

        #[test]
        fn from_json_array() {
            let arr = json!([1, 2, 3]);
            match EvalValue::from_json(&arr) {
                EvalValue::String(s) => assert_eq!(s, "[1,2,3]"),
                _ => panic!("Expected String"),
            }
        }

        #[test]
        fn from_json_object() {
            let obj = json!({"a": 1});
            match EvalValue::from_json(&obj) {
                EvalValue::String(s) => assert!(s.contains("\"a\"")),
                _ => panic!("Expected String"),
            }
        }
    }

    mod eval_simple_variable {
        use super::*;

        #[test]
        fn resolves_string_variable() {
            let state = create_test_state(json!({"name": "Alice"}));
            let evaluator = Evaluator::new(&state);
            let expr = parse("name").unwrap();

            match evaluator.eval(&expr) {
                EvalResult::Value(s) => assert_eq!(s, "Alice"),
                EvalResult::Error { message, .. } => panic!("Expected Value, got error: {message}"),
            }
        }

        #[test]
        fn resolves_number_variable() {
            let state = create_test_state(json!({"count": 42}));
            let evaluator = Evaluator::new(&state);
            let expr = parse("count").unwrap();

            match evaluator.eval(&expr) {
                EvalResult::Value(s) => assert_eq!(s, "42"),
                _ => panic!("Expected Value"),
            }
        }

        #[test]
        fn resolves_bool_variable() {
            let state = create_test_state(json!({"active": true}));
            let evaluator = Evaluator::new(&state);
            let expr = parse("active").unwrap();

            match evaluator.eval(&expr) {
                EvalResult::Value(s) => assert_eq!(s, "true"),
                _ => panic!("Expected Value"),
            }
        }

        #[test]
        fn missing_variable_returns_empty() {
            let state = create_test_state(json!({}));
            let evaluator = Evaluator::new(&state);
            let expr = parse("missing").unwrap();

            match evaluator.eval(&expr) {
                EvalResult::Value(s) => assert_eq!(s, ""),
                _ => panic!("Expected empty Value"),
            }
        }

        #[test]
        fn null_variable_returns_empty() {
            let state = create_test_state(json!({"nothing": null}));
            let evaluator = Evaluator::new(&state);
            let expr = parse("nothing").unwrap();

            match evaluator.eval(&expr) {
                EvalResult::Value(s) => assert_eq!(s, ""),
                _ => panic!("Expected empty Value"),
            }
        }
    }

    mod eval_nested_variable {
        use super::*;

        #[test]
        fn resolves_nested_path() {
            let state = create_test_state(json!({
                "user": {
                    "name": "Bob",
                    "address": {
                        "city": "London"
                    }
                }
            }));
            let evaluator = Evaluator::new(&state);

            let expr = parse("user.name").unwrap();
            match evaluator.eval(&expr) {
                EvalResult::Value(s) => assert_eq!(s, "Bob"),
                _ => panic!("Expected Value"),
            }

            let expr = parse("user.address.city").unwrap();
            match evaluator.eval(&expr) {
                EvalResult::Value(s) => assert_eq!(s, "London"),
                _ => panic!("Expected Value"),
            }
        }

        #[test]
        fn missing_nested_path_returns_empty() {
            let state = create_test_state(json!({"user": {"name": "Alice"}}));
            let evaluator = Evaluator::new(&state);
            let expr = parse("user.missing.deep").unwrap();

            match evaluator.eval(&expr) {
                EvalResult::Value(s) => assert_eq!(s, ""),
                _ => panic!("Expected empty Value"),
            }
        }
    }

    mod eval_context_variable {
        use super::*;

        #[test]
        fn resolves_ctx_today() {
            let state = create_test_state(json!({}));
            let evaluator = Evaluator::new(&state);
            let expr = parse("ctx.today").unwrap();

            match evaluator.eval(&expr) {
                EvalResult::Value(s) => assert_eq!(s, "2024-06-15"), // From fixed_for_testing
                _ => panic!("Expected Value"),
            }
        }

        #[test]
        fn resolves_ctx_year() {
            let state = create_test_state(json!({}));
            let evaluator = Evaluator::new(&state);
            let expr = parse("ctx.year").unwrap();

            match evaluator.eval(&expr) {
                EvalResult::Value(s) => assert_eq!(s, "2024"),
                _ => panic!("Expected Value"),
            }
        }

        #[test]
        fn resolves_ctx_dow() {
            let state = create_test_state(json!({}));
            let evaluator = Evaluator::new(&state);
            let expr = parse("ctx.dow").unwrap();

            match evaluator.eval(&expr) {
                EvalResult::Value(s) => assert_eq!(s, "Saturday"),
                _ => panic!("Expected Value"),
            }
        }

        #[test]
        fn ctx_unknown_returns_empty() {
            let state = create_test_state(json!({}));
            let evaluator = Evaluator::new(&state);
            let expr = parse("ctx.unknown").unwrap();

            match evaluator.eval(&expr) {
                EvalResult::Value(s) => assert_eq!(s, ""),
                _ => panic!("Expected empty Value"),
            }
        }
    }

    mod eval_env_variable {
        use super::*;

        #[test]
        fn resolves_env_variable() {
            let mut ctx = test_context();
            ctx.env.insert("HOME".to_string(), "/home/user".to_string());

            let fm = HashMap::new();
            let state = EffectiveStateBuilder::new()
                .with_frontmatter(fm)
                .with_context(ctx)
                .build();

            let evaluator = Evaluator::new(&state);
            let expr = parse("env.HOME").unwrap();

            match evaluator.eval(&expr) {
                EvalResult::Value(s) => assert_eq!(s, "/home/user"),
                _ => panic!("Expected Value"),
            }
        }

        #[test]
        fn missing_env_returns_empty() {
            let state = create_test_state(json!({}));
            let evaluator = Evaluator::new(&state);
            let expr = parse("env.NONEXISTENT_VAR_12345").unwrap();

            match evaluator.eval(&expr) {
                EvalResult::Value(s) => assert_eq!(s, ""),
                _ => panic!("Expected empty Value"),
            }
        }
    }

    mod eval_literals {
        use super::*;

        #[test]
        fn evaluates_string_literal() {
            let state = create_test_state(json!({}));
            let evaluator = Evaluator::new(&state);
            let expr = parse(r#""hello world""#).unwrap();

            match evaluator.eval(&expr) {
                EvalResult::Value(s) => assert_eq!(s, "hello world"),
                _ => panic!("Expected Value"),
            }
        }

        #[test]
        fn evaluates_single_quoted_string() {
            let state = create_test_state(json!({}));
            let evaluator = Evaluator::new(&state);
            let expr = parse("'single quoted'").unwrap();

            match evaluator.eval(&expr) {
                EvalResult::Value(s) => assert_eq!(s, "single quoted"),
                _ => panic!("Expected Value"),
            }
        }

        #[test]
        fn evaluates_empty_string_literal() {
            let state = create_test_state(json!({}));
            let evaluator = Evaluator::new(&state);
            let expr = parse(r#""""#).unwrap();

            match evaluator.eval(&expr) {
                EvalResult::Value(s) => assert_eq!(s, ""),
                _ => panic!("Expected Value"),
            }
        }

        #[test]
        fn evaluates_integer() {
            let state = create_test_state(json!({}));
            let evaluator = Evaluator::new(&state);
            let expr = parse("42").unwrap();

            match evaluator.eval(&expr) {
                EvalResult::Value(s) => assert_eq!(s, "42"),
                _ => panic!("Expected Value"),
            }
        }

        #[test]
        fn evaluates_negative_integer() {
            let state = create_test_state(json!({}));
            let evaluator = Evaluator::new(&state);
            let expr = parse("-17").unwrap();

            match evaluator.eval(&expr) {
                EvalResult::Value(s) => assert_eq!(s, "-17"),
                _ => panic!("Expected Value"),
            }
        }

        #[test]
        fn evaluates_float() {
            let state = create_test_state(json!({}));
            let evaluator = Evaluator::new(&state);
            let expr = parse("3.14").unwrap();

            match evaluator.eval(&expr) {
                EvalResult::Value(s) => assert_eq!(s, "3.14"),
                _ => panic!("Expected Value"),
            }
        }

        #[test]
        fn evaluates_zero() {
            let state = create_test_state(json!({}));
            let evaluator = Evaluator::new(&state);
            let expr = parse("0").unwrap();

            match evaluator.eval(&expr) {
                EvalResult::Value(s) => assert_eq!(s, "0"),
                _ => panic!("Expected Value"),
            }
        }
    }

    mod eval_value_tests {
        use super::*;

        #[test]
        fn variable_preserves_type_string() {
            let state = create_test_state(json!({"name": "Alice"}));
            let evaluator = Evaluator::new(&state);
            let expr = parse("name").unwrap();

            match evaluator.eval_value(&expr) {
                EvalValue::String(s) => assert_eq!(s, "Alice"),
                _ => panic!("Expected String"),
            }
        }

        #[test]
        fn variable_preserves_type_number() {
            let state = create_test_state(json!({"count": 42}));
            let evaluator = Evaluator::new(&state);
            let expr = parse("count").unwrap();

            match evaluator.eval_value(&expr) {
                EvalValue::Number(n) => assert_eq!(n, 42.0),
                _ => panic!("Expected Number"),
            }
        }

        #[test]
        fn variable_preserves_type_bool() {
            let state = create_test_state(json!({"active": true}));
            let evaluator = Evaluator::new(&state);
            let expr = parse("active").unwrap();

            match evaluator.eval_value(&expr) {
                EvalValue::Bool(b) => assert!(b),
                _ => panic!("Expected Bool"),
            }
        }

        #[test]
        fn missing_variable_returns_null() {
            let state = create_test_state(json!({}));
            let evaluator = Evaluator::new(&state);
            let expr = parse("missing").unwrap();

            assert_eq!(evaluator.eval_value(&expr), EvalValue::Null);
        }

        #[test]
        fn string_literal_value() {
            let state = create_test_state(json!({}));
            let evaluator = Evaluator::new(&state);
            let expr = parse(r#""test""#).unwrap();

            match evaluator.eval_value(&expr) {
                EvalValue::String(s) => assert_eq!(s, "test"),
                _ => panic!("Expected String"),
            }
        }

        #[test]
        fn number_literal_value() {
            let state = create_test_state(json!({}));
            let evaluator = Evaluator::new(&state);
            let expr = parse("99").unwrap();

            match evaluator.eval_value(&expr) {
                EvalValue::Number(n) => assert_eq!(n, 99.0),
                _ => panic!("Expected Number"),
            }
        }
    }

    mod eval_fallback {
        use super::*;

        #[test]
        fn uses_primary_if_truthy() {
            let state = create_test_state(json!({"color": "blue"}));
            let evaluator = Evaluator::new(&state);
            let expr = parse(r#"color | "unknown""#).unwrap();

            match evaluator.eval(&expr) {
                EvalResult::Value(s) => assert_eq!(s, "blue"),
                _ => panic!("Expected Value"),
            }
        }

        #[test]
        fn uses_fallback_if_falsy() {
            let state = create_test_state(json!({}));
            let evaluator = Evaluator::new(&state);
            let expr = parse(r#"missing | "default""#).unwrap();

            match evaluator.eval(&expr) {
                EvalResult::Value(s) => assert_eq!(s, "default"),
                _ => panic!("Expected Value"),
            }
        }

        #[test]
        fn uses_fallback_for_empty_string() {
            let state = create_test_state(json!({"empty": ""}));
            let evaluator = Evaluator::new(&state);
            let expr = parse(r#"empty | "fallback""#).unwrap();

            match evaluator.eval(&expr) {
                EvalResult::Value(s) => assert_eq!(s, "fallback"),
                _ => panic!("Expected Value"),
            }
        }

        #[test]
        fn uses_fallback_for_null() {
            let state = create_test_state(json!({"nothing": null}));
            let evaluator = Evaluator::new(&state);
            let expr = parse(r#"nothing | "fallback""#).unwrap();

            match evaluator.eval(&expr) {
                EvalResult::Value(s) => assert_eq!(s, "fallback"),
                _ => panic!("Expected Value"),
            }
        }

        #[test]
        fn uses_fallback_for_false() {
            let state = create_test_state(json!({"flag": false}));
            let evaluator = Evaluator::new(&state);
            let expr = parse(r#"flag | "fallback""#).unwrap();

            match evaluator.eval(&expr) {
                EvalResult::Value(s) => assert_eq!(s, "fallback"),
                _ => panic!("Expected Value"),
            }
        }

        #[test]
        fn uses_fallback_for_zero() {
            let state = create_test_state(json!({"count": 0}));
            let evaluator = Evaluator::new(&state);
            let expr = parse(r#"count | "fallback""#).unwrap();

            match evaluator.eval(&expr) {
                EvalResult::Value(s) => assert_eq!(s, "fallback"),
                _ => panic!("Expected Value"),
            }
        }

        #[test]
        fn chained_fallback_uses_first_truthy() {
            let state = create_test_state(json!({"b": "second"}));
            let evaluator = Evaluator::new(&state);
            // a | b | c parses as Fallback(Fallback(a, b), c)
            let expr = parse(r#"a | b | "third""#).unwrap();

            match evaluator.eval(&expr) {
                EvalResult::Value(s) => assert_eq!(s, "second"),
                _ => panic!("Expected Value"),
            }
        }

        #[test]
        fn chained_fallback_uses_last_if_all_falsy() {
            let state = create_test_state(json!({}));
            let evaluator = Evaluator::new(&state);
            let expr = parse(r#"a | b | "default""#).unwrap();

            match evaluator.eval(&expr) {
                EvalResult::Value(s) => assert_eq!(s, "default"),
                _ => panic!("Expected Value"),
            }
        }

        #[test]
        fn fallback_to_variable() {
            let state = create_test_state(json!({"backup": "backup_value"}));
            let evaluator = Evaluator::new(&state);
            let expr = parse("missing | backup").unwrap();

            match evaluator.eval(&expr) {
                EvalResult::Value(s) => assert_eq!(s, "backup_value"),
                _ => panic!("Expected Value"),
            }
        }
    }

    mod eval_ternary {
        use super::*;

        #[test]
        fn true_branch() {
            let state = create_test_state(json!({"active": true}));
            let evaluator = Evaluator::new(&state);
            let expr = parse(r#"active ? "yes" : "no""#).unwrap();

            match evaluator.eval(&expr) {
                EvalResult::Value(s) => assert_eq!(s, "yes"),
                _ => panic!("Expected Value"),
            }
        }

        #[test]
        fn false_branch() {
            let state = create_test_state(json!({"active": false}));
            let evaluator = Evaluator::new(&state);
            let expr = parse(r#"active ? "yes" : "no""#).unwrap();

            match evaluator.eval(&expr) {
                EvalResult::Value(s) => assert_eq!(s, "no"),
                _ => panic!("Expected Value"),
            }
        }

        #[test]
        fn truthy_string_condition() {
            let state = create_test_state(json!({"name": "Alice"}));
            let evaluator = Evaluator::new(&state);
            let expr = parse(r#"name ? "present" : "absent""#).unwrap();

            match evaluator.eval(&expr) {
                EvalResult::Value(s) => assert_eq!(s, "present"),
                _ => panic!("Expected Value"),
            }
        }

        #[test]
        fn missing_condition_uses_else() {
            let state = create_test_state(json!({}));
            let evaluator = Evaluator::new(&state);
            let expr = parse(r#"missing ? "yes" : "no""#).unwrap();

            match evaluator.eval(&expr) {
                EvalResult::Value(s) => assert_eq!(s, "no"),
                _ => panic!("Expected Value"),
            }
        }

        #[test]
        fn branches_can_be_variables() {
            let state = create_test_state(json!({
                "active": true,
                "yes_val": "YES",
                "no_val": "NO"
            }));
            let evaluator = Evaluator::new(&state);
            let expr = parse("active ? yes_val : no_val").unwrap();

            match evaluator.eval(&expr) {
                EvalResult::Value(s) => assert_eq!(s, "YES"),
                _ => panic!("Expected Value"),
            }
        }

        #[test]
        fn nonzero_number_is_truthy() {
            let state = create_test_state(json!({"count": 5}));
            let evaluator = Evaluator::new(&state);
            let expr = parse(r#"count ? "has items" : "empty""#).unwrap();

            match evaluator.eval(&expr) {
                EvalResult::Value(s) => assert_eq!(s, "has items"),
                _ => panic!("Expected Value"),
            }
        }

        #[test]
        fn zero_is_falsy() {
            let state = create_test_state(json!({"count": 0}));
            let evaluator = Evaluator::new(&state);
            let expr = parse(r#"count ? "has items" : "empty""#).unwrap();

            match evaluator.eval(&expr) {
                EvalResult::Value(s) => assert_eq!(s, "empty"),
                _ => panic!("Expected Value"),
            }
        }
    }

    mod eval_comparison {
        use super::*;

        #[test]
        fn equal_strings_match() {
            let state = create_test_state(json!({"x": "hello"}));
            let evaluator = Evaluator::new(&state);
            let expr = parse(r#"x == "hello" ? "match" : "no match""#).unwrap();

            match evaluator.eval(&expr) {
                EvalResult::Value(s) => assert_eq!(s, "match"),
                _ => panic!("Expected Value"),
            }
        }

        #[test]
        fn equal_strings_no_match() {
            let state = create_test_state(json!({"x": "hello"}));
            let evaluator = Evaluator::new(&state);
            let expr = parse(r#"x == "world" ? "match" : "no match""#).unwrap();

            match evaluator.eval(&expr) {
                EvalResult::Value(s) => assert_eq!(s, "no match"),
                _ => panic!("Expected Value"),
            }
        }

        #[test]
        fn equal_numbers() {
            let state = create_test_state(json!({"count": 5}));
            let evaluator = Evaluator::new(&state);
            let expr = parse(r#"count == 5 ? "five" : "other""#).unwrap();

            match evaluator.eval(&expr) {
                EvalResult::Value(s) => assert_eq!(s, "five"),
                _ => panic!("Expected Value"),
            }
        }

        #[test]
        fn not_equal() {
            let state = create_test_state(json!({"status": "active"}));
            let evaluator = Evaluator::new(&state);
            let expr = parse(r#"status != "inactive" ? "running" : "stopped""#).unwrap();

            match evaluator.eval(&expr) {
                EvalResult::Value(s) => assert_eq!(s, "running"),
                _ => panic!("Expected Value"),
            }
        }

        #[test]
        fn greater_than() {
            let state = create_test_state(json!({"count": 10}));
            let evaluator = Evaluator::new(&state);
            let expr = parse(r#"count > 5 ? "many" : "few""#).unwrap();

            match evaluator.eval(&expr) {
                EvalResult::Value(s) => assert_eq!(s, "many"),
                _ => panic!("Expected Value"),
            }
        }

        #[test]
        fn greater_than_equal_boundary() {
            let state = create_test_state(json!({"count": 5}));
            let evaluator = Evaluator::new(&state);
            let expr = parse(r#"count >= 5 ? "enough" : "not enough""#).unwrap();

            match evaluator.eval(&expr) {
                EvalResult::Value(s) => assert_eq!(s, "enough"),
                _ => panic!("Expected Value"),
            }
        }

        #[test]
        fn less_than() {
            let state = create_test_state(json!({"count": 3}));
            let evaluator = Evaluator::new(&state);
            let expr = parse(r#"count < 5 ? "few" : "many""#).unwrap();

            match evaluator.eval(&expr) {
                EvalResult::Value(s) => assert_eq!(s, "few"),
                _ => panic!("Expected Value"),
            }
        }

        #[test]
        fn comparison_with_string_numbers() {
            // String "10" should be coerced to number for comparison
            let state = create_test_state(json!({"count": "10"}));
            let evaluator = Evaluator::new(&state);
            let expr = parse(r#"count > 5 ? "big" : "small""#).unwrap();

            match evaluator.eval(&expr) {
                EvalResult::Value(s) => assert_eq!(s, "big"),
                _ => panic!("Expected Value"),
            }
        }

        #[test]
        fn comparison_non_numeric_string_returns_false() {
            // Non-numeric string comparison should return false
            let state = create_test_state(json!({"name": "alice"}));
            let evaluator = Evaluator::new(&state);
            let expr = parse(r#"name > 5 ? "yes" : "no""#).unwrap();

            match evaluator.eval(&expr) {
                EvalResult::Value(s) => assert_eq!(s, "no"),
                _ => panic!("Expected Value"),
            }
        }

        #[test]
        fn comparison_both_missing_returns_false() {
            let state = create_test_state(json!({}));
            let evaluator = Evaluator::new(&state);
            let expr = parse(r#"a > b ? "yes" : "no""#).unwrap();

            match evaluator.eval(&expr) {
                EvalResult::Value(s) => assert_eq!(s, "no"),
                _ => panic!("Expected Value"),
            }
        }

        #[test]
        fn comparison_alone_returns_boolean_string() {
            // Direct comparison (not in ternary) returns "true" or "false"
            let state = create_test_state(json!({"x": 5}));
            let evaluator = Evaluator::new(&state);
            let expr = parse("x > 3").unwrap();

            match evaluator.eval(&expr) {
                EvalResult::Value(s) => assert_eq!(s, "true"),
                _ => panic!("Expected Value"),
            }
        }
    }

    mod eval_value_operators {
        use super::*;

        #[test]
        fn fallback_preserves_type() {
            let state = create_test_state(json!({"count": 42}));
            let evaluator = Evaluator::new(&state);
            let expr = parse("count | 0").unwrap();

            match evaluator.eval_value(&expr) {
                EvalValue::Number(n) => assert_eq!(n, 42.0),
                _ => panic!("Expected Number"),
            }
        }

        #[test]
        fn fallback_returns_fallback_type() {
            let state = create_test_state(json!({}));
            let evaluator = Evaluator::new(&state);
            let expr = parse(r#"missing | "default""#).unwrap();

            match evaluator.eval_value(&expr) {
                EvalValue::String(s) => assert_eq!(s, "default"),
                _ => panic!("Expected String"),
            }
        }

        #[test]
        fn ternary_preserves_branch_type() {
            let state = create_test_state(json!({"active": true}));
            let evaluator = Evaluator::new(&state);
            let expr = parse("active ? 1 : 0").unwrap();

            match evaluator.eval_value(&expr) {
                EvalValue::Number(n) => assert_eq!(n, 1.0),
                _ => panic!("Expected Number"),
            }
        }

        #[test]
        fn comparison_returns_bool() {
            let state = create_test_state(json!({"x": 5}));
            let evaluator = Evaluator::new(&state);
            let expr = parse("x > 3").unwrap();

            match evaluator.eval_value(&expr) {
                EvalValue::Bool(b) => assert!(b),
                _ => panic!("Expected Bool"),
            }
        }
    }

    mod eval_value_as_number {
        use super::*;

        #[test]
        fn number_to_number() {
            assert_eq!(EvalValue::Number(42.0).as_number(), Some(42.0));
        }

        #[test]
        fn string_number_to_number() {
            assert_eq!(EvalValue::String("42".to_string()).as_number(), Some(42.0));
            assert_eq!(
                EvalValue::String("3.14".to_string()).as_number(),
                Some(3.14)
            );
            assert_eq!(EvalValue::String("-5".to_string()).as_number(), Some(-5.0));
        }

        #[test]
        fn non_numeric_string_returns_none() {
            assert_eq!(EvalValue::String("hello".to_string()).as_number(), None);
            assert_eq!(EvalValue::String("".to_string()).as_number(), None);
        }

        #[test]
        fn bool_to_number() {
            assert_eq!(EvalValue::Bool(true).as_number(), Some(1.0));
            assert_eq!(EvalValue::Bool(false).as_number(), Some(0.0));
        }

        #[test]
        fn null_returns_none() {
            assert_eq!(EvalValue::Null.as_number(), None);
        }
    }

    mod fn_length {
        use super::*;

        #[test]
        fn length_string() {
            let state = create_test_state(json!({"name": "Alice"}));
            let evaluator = Evaluator::new(&state);
            let expr = parse("length(name)").unwrap();

            match evaluator.eval(&expr) {
                EvalResult::Value(s) => assert_eq!(s, "5"),
                _ => panic!("Expected Value"),
            }
        }

        #[test]
        fn length_array() {
            let state = create_test_state(json!({"items": [1, 2, 3]}));
            let evaluator = Evaluator::new(&state);
            let expr = parse("length(items)").unwrap();

            match evaluator.eval(&expr) {
                EvalResult::Value(s) => assert_eq!(s, "3"),
                _ => panic!("Expected Value"),
            }
        }

        #[test]
        fn length_object() {
            let state = create_test_state(json!({"data": {"a": 1, "b": 2}}));
            let evaluator = Evaluator::new(&state);
            let expr = parse("length(data)").unwrap();

            match evaluator.eval(&expr) {
                EvalResult::Value(s) => assert_eq!(s, "2"),
                _ => panic!("Expected Value"),
            }
        }

        #[test]
        fn length_number() {
            let state = create_test_state(json!({"count": 12345}));
            let evaluator = Evaluator::new(&state);
            let expr = parse("length(count)").unwrap();

            match evaluator.eval(&expr) {
                EvalResult::Value(s) => assert_eq!(s, "5"),
                _ => panic!("Expected Value"),
            }
        }

        #[test]
        fn length_null() {
            let state = create_test_state(json!({"nothing": null}));
            let evaluator = Evaluator::new(&state);
            let expr = parse("length(nothing)").unwrap();

            match evaluator.eval(&expr) {
                EvalResult::Value(s) => assert_eq!(s, "0"),
                _ => panic!("Expected Value"),
            }
        }

        #[test]
        fn length_missing_variable() {
            let state = create_test_state(json!({}));
            let evaluator = Evaluator::new(&state);
            let expr = parse("length(missing)").unwrap();

            match evaluator.eval(&expr) {
                EvalResult::Value(s) => assert_eq!(s, "0"),
                _ => panic!("Expected Value"),
            }
        }

        #[test]
        fn length_bool() {
            let state = create_test_state(json!({"flag": true}));
            let evaluator = Evaluator::new(&state);
            let expr = parse("length(flag)").unwrap();

            match evaluator.eval(&expr) {
                EvalResult::Value(s) => assert_eq!(s, "1"),
                _ => panic!("Expected Value"),
            }
        }

        #[test]
        fn length_empty_string() {
            let state = create_test_state(json!({"empty": ""}));
            let evaluator = Evaluator::new(&state);
            let expr = parse("length(empty)").unwrap();

            match evaluator.eval(&expr) {
                EvalResult::Value(s) => assert_eq!(s, "0"),
                _ => panic!("Expected Value"),
            }
        }

        #[test]
        fn length_empty_array() {
            let state = create_test_state(json!({"arr": []}));
            let evaluator = Evaluator::new(&state);
            let expr = parse("length(arr)").unwrap();

            match evaluator.eval(&expr) {
                EvalResult::Value(s) => assert_eq!(s, "0"),
                _ => panic!("Expected Value"),
            }
        }

        #[test]
        fn length_unicode_string() {
            // "Hello 🌍" = 5 chars ("Hello") + 1 space + 1 emoji = 7 characters
            let state = create_test_state(json!({"emoji": "Hello 🌍"}));
            let evaluator = Evaluator::new(&state);
            let expr = parse("length(emoji)").unwrap();

            match evaluator.eval(&expr) {
                EvalResult::Value(s) => assert_eq!(s, "7"),
                _ => panic!("Expected Value"),
            }
        }

        #[test]
        fn length_no_args_error() {
            let state = create_test_state(json!({}));
            let evaluator = Evaluator::new(&state);
            let expr = parse("length()").unwrap();

            assert!(evaluator.eval(&expr).is_error());
        }

        #[test]
        fn length_string_literal() {
            let state = create_test_state(json!({}));
            let evaluator = Evaluator::new(&state);
            let expr = parse(r#"length("hello")"#).unwrap();

            match evaluator.eval(&expr) {
                EvalResult::Value(s) => assert_eq!(s, "5"),
                _ => panic!("Expected Value"),
            }
        }
    }

    mod fn_number {
        use super::*;

        #[test]
        fn number_string() {
            let state = create_test_state(json!({}));
            let evaluator = Evaluator::new(&state);
            let expr = parse(r#"number("42")"#).unwrap();

            match evaluator.eval(&expr) {
                EvalResult::Value(s) => assert_eq!(s, "42"),
                _ => panic!("Expected Value"),
            }
        }

        #[test]
        fn number_string_float() {
            let state = create_test_state(json!({}));
            let evaluator = Evaluator::new(&state);
            let expr = parse(r#"number("3.14")"#).unwrap();

            match evaluator.eval(&expr) {
                EvalResult::Value(s) => assert_eq!(s, "3.14"),
                _ => panic!("Expected Value"),
            }
        }

        #[test]
        fn number_already_numeric() {
            let state = create_test_state(json!({"count": 42}));
            let evaluator = Evaluator::new(&state);
            let expr = parse("number(count)").unwrap();

            match evaluator.eval(&expr) {
                EvalResult::Value(s) => assert_eq!(s, "42"),
                _ => panic!("Expected Value"),
            }
        }

        #[test]
        fn number_with_default() {
            let state = create_test_state(json!({}));
            let evaluator = Evaluator::new(&state);
            let expr = parse(r#"number("abc", -1)"#).unwrap();

            match evaluator.eval(&expr) {
                EvalResult::Value(s) => assert_eq!(s, "-1"),
                _ => panic!("Expected Value"),
            }
        }

        #[test]
        fn number_null_uses_default() {
            let state = create_test_state(json!({"nothing": null}));
            let evaluator = Evaluator::new(&state);
            let expr = parse("number(nothing, 99)").unwrap();

            match evaluator.eval(&expr) {
                EvalResult::Value(s) => assert_eq!(s, "99"),
                _ => panic!("Expected Value"),
            }
        }

        #[test]
        fn number_missing_uses_zero_default() {
            let state = create_test_state(json!({}));
            let evaluator = Evaluator::new(&state);
            let expr = parse("number(missing)").unwrap();

            match evaluator.eval(&expr) {
                EvalResult::Value(s) => assert_eq!(s, "0"),
                _ => panic!("Expected Value"),
            }
        }

        #[test]
        fn number_string_variable() {
            let state = create_test_state(json!({"str_num": "100"}));
            let evaluator = Evaluator::new(&state);
            let expr = parse("number(str_num)").unwrap();

            match evaluator.eval(&expr) {
                EvalResult::Value(s) => assert_eq!(s, "100"),
                _ => panic!("Expected Value"),
            }
        }

        #[test]
        fn number_negative_string() {
            let state = create_test_state(json!({}));
            let evaluator = Evaluator::new(&state);
            let expr = parse(r#"number("-42")"#).unwrap();

            match evaluator.eval(&expr) {
                EvalResult::Value(s) => assert_eq!(s, "-42"),
                _ => panic!("Expected Value"),
            }
        }

        #[test]
        fn number_no_args_error() {
            let state = create_test_state(json!({}));
            let evaluator = Evaluator::new(&state);
            let expr = parse("number()").unwrap();

            assert!(evaluator.eval(&expr).is_error());
        }
    }

    mod fn_round {
        use super::*;

        #[test]
        fn round_number() {
            let state = create_test_state(json!({}));
            let evaluator = Evaluator::new(&state);
            let expr = parse("round(3.7)").unwrap();

            match evaluator.eval(&expr) {
                EvalResult::Value(s) => assert_eq!(s, "4"),
                _ => panic!("Expected Value"),
            }
        }

        #[test]
        fn round_down() {
            let state = create_test_state(json!({}));
            let evaluator = Evaluator::new(&state);
            let expr = parse("round(3.2)").unwrap();

            match evaluator.eval(&expr) {
                EvalResult::Value(s) => assert_eq!(s, "3"),
                _ => panic!("Expected Value"),
            }
        }

        #[test]
        fn round_string() {
            let state = create_test_state(json!({}));
            let evaluator = Evaluator::new(&state);
            let expr = parse(r#"round("2.8")"#).unwrap();

            match evaluator.eval(&expr) {
                EvalResult::Value(s) => assert_eq!(s, "3"),
                _ => panic!("Expected Value"),
            }
        }

        #[test]
        fn round_variable() {
            let state = create_test_state(json!({"value": 4.5}));
            let evaluator = Evaluator::new(&state);
            let expr = parse("round(value)").unwrap();

            match evaluator.eval(&expr) {
                EvalResult::Value(s) => assert_eq!(s, "5"), // banker's rounding rounds 0.5 up
                _ => panic!("Expected Value"),
            }
        }

        #[test]
        fn round_with_default() {
            let state = create_test_state(json!({}));
            let evaluator = Evaluator::new(&state);
            let expr = parse(r#"round("abc", 42)"#).unwrap();

            match evaluator.eval(&expr) {
                EvalResult::Value(s) => assert_eq!(s, "42"),
                _ => panic!("Expected Value"),
            }
        }

        #[test]
        fn round_missing_uses_zero() {
            let state = create_test_state(json!({}));
            let evaluator = Evaluator::new(&state);
            let expr = parse("round(missing)").unwrap();

            match evaluator.eval(&expr) {
                EvalResult::Value(s) => assert_eq!(s, "0"),
                _ => panic!("Expected Value"),
            }
        }

        #[test]
        fn round_negative() {
            let state = create_test_state(json!({}));
            let evaluator = Evaluator::new(&state);
            let expr = parse("round(-3.7)").unwrap();

            match evaluator.eval(&expr) {
                EvalResult::Value(s) => assert_eq!(s, "-4"),
                _ => panic!("Expected Value"),
            }
        }

        #[test]
        fn round_integer() {
            let state = create_test_state(json!({"n": 5}));
            let evaluator = Evaluator::new(&state);
            let expr = parse("round(n)").unwrap();

            match evaluator.eval(&expr) {
                EvalResult::Value(s) => assert_eq!(s, "5"),
                _ => panic!("Expected Value"),
            }
        }

        #[test]
        fn round_no_args_error() {
            let state = create_test_state(json!({}));
            let evaluator = Evaluator::new(&state);
            let expr = parse("round()").unwrap();

            assert!(evaluator.eval(&expr).is_error());
        }
    }

    mod unknown_function {
        use super::*;

        #[test]
        fn unknown_function_error() {
            let state = create_test_state(json!({}));
            let evaluator = Evaluator::new(&state);
            let expr = parse("unknown(x)").unwrap();

            match evaluator.eval(&expr) {
                EvalResult::Error { message, original } => {
                    assert!(message.contains("Unknown function"));
                    assert!(original.contains("unknown"));
                }
                _ => panic!("Expected Error"),
            }
        }

        #[test]
        fn unknown_function_eval_value_returns_null() {
            let state = create_test_state(json!({}));
            let evaluator = Evaluator::new(&state);
            let expr = parse("unknown(x)").unwrap();

            assert_eq!(evaluator.eval_value(&expr), EvalValue::Null);
        }
    }

    mod function_in_expressions {
        use super::*;

        #[test]
        fn length_in_ternary() {
            let state = create_test_state(json!({"name": "Alice"}));
            let evaluator = Evaluator::new(&state);
            let expr = parse(r#"length(name) >= 10 ? "long" : "short""#).unwrap();

            match evaluator.eval(&expr) {
                EvalResult::Value(s) => assert_eq!(s, "short"),
                _ => panic!("Expected Value"),
            }
        }

        #[test]
        fn length_in_ternary_long() {
            let state = create_test_state(json!({"name": "Christopher"}));
            let evaluator = Evaluator::new(&state);
            let expr = parse(r#"length(name) >= 10 ? "long" : "short""#).unwrap();

            match evaluator.eval(&expr) {
                EvalResult::Value(s) => assert_eq!(s, "long"),
                _ => panic!("Expected Value"),
            }
        }

        #[test]
        fn function_in_comparison() {
            let state = create_test_state(json!({"items": [1, 2, 3, 4, 5]}));
            let evaluator = Evaluator::new(&state);
            let expr = parse(r#"length(items) > 3 ? "many" : "few""#).unwrap();

            match evaluator.eval(&expr) {
                EvalResult::Value(s) => assert_eq!(s, "many"),
                _ => panic!("Expected Value"),
            }
        }

        #[test]
        fn function_standalone_eval_value() {
            let state = create_test_state(json!({"name": "Bob"}));
            let evaluator = Evaluator::new(&state);
            let expr = parse("length(name)").unwrap();

            match evaluator.eval_value(&expr) {
                EvalValue::Number(n) => assert_eq!(n, 3.0),
                _ => panic!("Expected Number"),
            }
        }

        #[test]
        fn function_in_fallback() {
            let state = create_test_state(json!({"items": []}));
            let evaluator = Evaluator::new(&state);
            // length(items) is 0 which is falsy, so fallback is used
            let expr = parse(r#"length(items) | "no items""#).unwrap();

            match evaluator.eval(&expr) {
                EvalResult::Value(s) => assert_eq!(s, "no items"),
                _ => panic!("Expected Value"),
            }
        }
    }

    mod condition_helpers {
        use super::*;

        #[test]
        fn unary_not_truthiness() {
            let state = create_test_state(json!({}));
            let evaluator = Evaluator::new(&state);
            let expr = parse("!missing").unwrap();

            match evaluator.eval(&expr) {
                EvalResult::Value(v) => assert_eq!(v, "true"),
                _ => panic!("Expected Value"),
            }
        }

        #[test]
        fn has_key_function() {
            let state = create_test_state(json!({"user": {"name": "Alice"}}));
            let evaluator = Evaluator::new(&state);
            let expr = parse(r#"HasKey(user, "name")"#).unwrap();

            match evaluator.eval(&expr) {
                EvalResult::Value(v) => assert_eq!(v, "true"),
                _ => panic!("Expected Value"),
            }
        }

        #[test]
        fn and_or_functions() {
            let state = create_test_state(json!({"a": true, "b": false}));
            let evaluator = Evaluator::new(&state);

            let and_expr = parse("And(a, b)").unwrap();
            let or_expr = parse("Or(a, b)").unwrap();

            match evaluator.eval(&and_expr) {
                EvalResult::Value(v) => assert_eq!(v, "false"),
                _ => panic!("Expected Value"),
            }
            match evaluator.eval(&or_expr) {
                EvalResult::Value(v) => assert_eq!(v, "true"),
                _ => panic!("Expected Value"),
            }
        }
    }
}
