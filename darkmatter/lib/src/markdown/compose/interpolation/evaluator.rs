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
//!   - `{{ color || "unknown" }}` -> "unknown" if color is falsy
//!   - Chained: `{{ a || b || c }}` -> first truthy wins
//!
//! - **Ternary**: Boolean conditional
//!   - `{{ active ? "yes" : "no" }}` -> "yes" if active is truthy
//!
//! - **Comparison**: Evaluates to boolean for use in ternary
//!   - `{{ count > 0 ? "items" : "empty" }}`
//!   - Equality (`==`, `!=`): compares string representations
//!   - Ordering (`>`, `>=`, `<`, `<=`): numeric comparison, false if non-numeric
//!
//! - **Arithmetic**: `+`, `-`, `*`, `/`, `%` for numeric operands; `+`
//!   doubles as string concatenation when either operand is a string.
//!
//! - **Member / index access**: `foo.bar`, `items[0]`, `items[-1]`,
//!   `config["key"]`. Invalid access produces `null` rather than erroring.
//!
//! - **Functions**: Helper utilities for value transformation. The library
//!   includes `length`, `number`, `round`, plus type predicates
//!   (`is_string`, `is_array`, …), math helpers (`min`, `max`, `abs`),
//!   collection helpers (`first`, `last`), string predicates and case
//!   mutations, and date validators.
//!
//! ## Truthiness
//!
//! Falsy values: `null`, `false`, `0`, `0.0`, `""`, `[]`, `{}`. Everything
//! else is truthy.
//!
//! ## Examples
//!
//! ```
//! use darkmatter::markdown::compose::interpolation::{parse, Evaluator, EvalResult};
//! use darkmatter::markdown::compose::EffectiveStateBuilder;
//! use serde_json::json;
//! use std::collections::HashMap;
//!
//! let mut fm = HashMap::new();
//! fm.insert("name".to_string(), json!("Alice"));
//!
//! let state = EffectiveStateBuilder::new()
//!     .with_frontmatter(fm)
//!     .build()
//!     .unwrap();
//!
//! let evaluator = Evaluator::new(&state);
//! let expr = parse("name").unwrap();
//!
//! match evaluator.eval(&expr) {
//!     EvalResult::Value(s) => assert_eq!(s, "Alice"),
//!     EvalResult::Error { .. } => panic!("Expected Value"),
//! }
//! ```

use crate::catalog::{describe_for_error, suggest};
use crate::markdown::compose::context::catalog::CONTEXT_VARIABLE_DESCRIPTORS;
use crate::markdown::compose::expression::{
    EvaluationLookup, Expr, ExpressionError, evaluate, interpolation_output_string,
};
use crate::markdown::compose::ComposeWarning;
use serde_json::Value;
use tracing::{debug, trace};

/// Result of evaluating an expression.
#[derive(Debug, Clone)]
pub enum EvalResult {
    /// Successful evaluation producing a string.
    Value(String),

    /// Evaluation failed.
    ///
    /// The `original` field contains the expression string representation
    /// for error recovery (e.g., preserving unprocessed expressions).
    Error {
        /// Typed expression error.
        error: ExpressionError,
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
/// an [`EvaluationLookup`] implementation to produce string output.
pub struct Evaluator<'a, L: EvaluationLookup> {
    state: &'a L,
}

impl<'a, L: EvaluationLookup> Evaluator<'a, L> {
    /// Creates a new evaluator with the given lookup state.
    pub fn new(state: &'a L) -> Self {
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
    /// - Fallback expressions (`a || b`)
    /// - Ternary expressions (`a ? b : c`)
    /// - Comparison expressions (`a == b`, `a > b`, etc.)
    /// - Function calls (`length()`, `number()`, `round()`)
    pub fn eval(&self, expr: &Expr) -> EvalResult {
        trace!(expr = ?expr, "interpolation: evaluating expression");

        // Preserve debug/trace logging for variable resolution
        if let Expr::Variable(name) = expr {
            // Bare array interpolation renders line-separated (spec D4); every
            // other type keeps `get_string`'s scalar coercion — inlined here so
            // the lookup runs once instead of `get` + a discarded `get_string`
            // re-lookup (F31). This coercion is byte-identical to the trait's
            // `get_string` (every `EvaluationLookup` impl mirrors the default).
            let value = match self.state.get(name) {
                Some(array @ Value::Array(_)) => interpolation_output_string(&array),
                None | Some(Value::Null) => String::new(),
                Some(Value::String(s)) => s,
                Some(Value::Number(n)) => n.to_string(),
                Some(Value::Bool(b)) => b.to_string(),
                Some(v) => v.to_string(),
            };
            if value.is_empty() {
                debug!(variable = %name, "interpolation: unresolved variable");
            } else {
                trace!(result = %value, "interpolation: resolved");
            }
            return EvalResult::Value(value);
        }

        match evaluate(expr, self.state) {
            Ok(value) => EvalResult::Value(interpolation_output_string(&value)),
            Err(error) => EvalResult::Error {
                error,
                original: expr.to_string(),
            },
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
        match evaluate(expr, self.state) {
            Ok(value) => EvalValue::from_json(&value),
            Err(_) => EvalValue::Null,
        }
    }

    /// Evaluates an expression to its raw JSON `Value`, preserving type.
    ///
    /// Unlike [`Self::eval`], which stringifies the result via `scalar_string`,
    /// this returns the typed `serde_json::Value` (so `false` stays a boolean,
    /// `2` stays a number, `null` stays null). Whole-value frontmatter
    /// interpolation uses it to keep a single `{{ expr }}` value typed instead
    /// of collapsing it to a string. The `Err` is the evaluator's message, so
    /// callers can fall back to the string path on failure.
    pub fn eval_json(&self, expr: &Expr) -> Result<Value, ExpressionError> {
        evaluate(expr, self.state)
    }

    /// Collects warnings for `ctx.*` references that don't resolve to a known
    /// runtime context variable.
    ///
    /// The check is parser-aware: it walks the AST so typo detection works even
    /// when the surrounding expression would otherwise evaluate to an empty
    /// string (the default for unresolved variables).
    pub fn collect_context_warnings(&self,
        expr: &Expr,
        warning_stage: &'static str,
    ) -> Vec<ComposeWarning> {
        let mut warnings = Vec::new();
        self.walk_context_variables(expr, &mut |name, raw| {
            if self.state.is_valid_context_variable(name) {
                return;
            }
            let mut message = format!("unknown context variable '{}'", raw);
            if let Some(descriptor) = suggest(&CONTEXT_VARIABLE_DESCRIPTORS, name, 1).first() {
                message.push_str("\n  did you mean: ");
                message.push_str(&describe_for_error(*descriptor));
            }
            warnings.push(ComposeWarning::new(warning_stage, message));
        });
        warnings
    }

    fn walk_context_variables(&self,
        expr: &Expr,
        visitor: &mut dyn FnMut(&str, &str),
    ) {
        match expr {
            Expr::Variable(path) => {
                if let Some(name) = path.strip_prefix("ctx.") {
                    let first = name.split('.').next().unwrap_or(name);
                    if !first.is_empty() {
                        visitor(first, path);
                    }
                }
            }
            Expr::Paren(inner)
            | Expr::UnaryNot(inner)
            | Expr::UnaryMinus(inner) => self.walk_context_variables(inner, visitor),
            Expr::Binary { left, right, .. }
            | Expr::Comparison { left, right, .. }
            | Expr::Fallback { primary: left, fallback: right } => {
                self.walk_context_variables(left, visitor);
                self.walk_context_variables(right, visitor);
            }
            Expr::Ternary {
                condition,
                then_branch,
                else_branch,
            } => {
                self.walk_context_variables(condition, visitor);
                self.walk_context_variables(then_branch, visitor);
                self.walk_context_variables(else_branch, visitor);
            }
            Expr::Index { base, index } => {
                self.walk_context_variables(base, visitor);
                self.walk_context_variables(index, visitor);
            }
            Expr::MemberAccess { base, .. } => self.walk_context_variables(base, visitor),
            Expr::FunctionCall { args, .. } => {
                for arg in args {
                    self.walk_context_variables(arg, visitor);
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::compose::EffectiveState;
    use crate::markdown::compose::interpolation::parse;
    use crate::markdown::compose::EffectiveStateBuilder;
    use crate::markdown::compose::ComposeContext;
    use serde_json::json;
    use std::collections::HashMap;

    fn test_context() -> ComposeContext {
        ComposeContext::fixed_for_testing()
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
            .unwrap()
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
                error: ExpressionError::Parse("test error".to_string()),
                original: "foo || bar".to_string(),
            };
            assert_eq!(result.unwrap_or_original(), "{{ foo || bar }}");
        }

        #[test]
        fn is_value() {
            let value = EvalResult::Value("x".to_string());
            let error = EvalResult::Error {
                error: ExpressionError::Parse("e".to_string()),
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
            assert_eq!(EvalValue::Number(3.15).as_string(), "3.15");
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
            assert_eq!(EvalValue::from_json(&json!(3.15)), EvalValue::Number(3.15));
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
                EvalResult::Error { error, .. } => panic!("Expected Value, got error: {error}"),
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
        fn resolves_ctx_day() {
            let state = create_test_state(json!({}));
            let evaluator = Evaluator::new(&state);
            let expr = parse("ctx.day").unwrap();

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
            ctx.env_mut()
                .insert("HOME".to_string(), "/home/user".to_string());

            let fm = HashMap::new();
            let state = EffectiveStateBuilder::new()
                .with_frontmatter(fm)
                .with_context(ctx)
                .build()
                .unwrap();

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

    mod eval_array_interpolation {
        use super::*;

        /// Bare array interpolation renders line-separated (spec D4 default),
        /// so `{{ items }}` ≡ `{{ as_line_separated(items) }}`.
        #[test]
        fn bare_array_renders_line_separated() {
            let state = create_test_state(json!({"items": ["a", "b", "c"]}));
            let evaluator = Evaluator::new(&state);
            let expr = parse("items").unwrap();

            match evaluator.eval(&expr) {
                EvalResult::Value(s) => assert_eq!(s, "a\nb\nc"),
                EvalResult::Error { error, .. } => panic!("Expected Value, got error: {error}"),
            }
        }

        #[test]
        fn empty_array_renders_empty() {
            let state = create_test_state(json!({"items": []}));
            let evaluator = Evaluator::new(&state);
            let expr = parse("items").unwrap();

            match evaluator.eval(&expr) {
                EvalResult::Value(s) => assert_eq!(s, ""),
                _ => panic!("Expected Value"),
            }
        }

        /// `as_csv` reproduces the historical comma-joined bare-name output of
        /// the collapsed `ctx.*` list variables (spec criterion 7).
        #[test]
        fn as_csv_reproduces_comma_output() {
            let state = create_test_state(json!({"items": ["a", "b", "c"]}));
            let evaluator = Evaluator::new(&state);
            let expr = parse("as_csv(items)").unwrap();

            match evaluator.eval(&expr) {
                EvalResult::Value(s) => assert_eq!(s, "a, b, c"),
                _ => panic!("Expected Value"),
            }
        }

        /// `as_unordered_list` reproduces the old `_list` twin's bullet output.
        #[test]
        fn as_unordered_list_reproduces_bullets() {
            let state = create_test_state(json!({"items": ["a", "b"]}));
            let evaluator = Evaluator::new(&state);
            let expr = parse("as_unordered_list(items)").unwrap();

            match evaluator.eval(&expr) {
                EvalResult::Value(s) => assert_eq!(s, "- a\n- b"),
                _ => panic!("Expected Value"),
            }
        }

        /// The `depends_on` / `used_by` object-array shape renders as nested
        /// bullets (spec criterion 9).
        #[test]
        fn as_unordered_list_renders_object_array_nested() {
            let state = create_test_state(json!({
                "depends_on": [
                    { "package": "alpha", "dependencies": ["beta", "gamma"] }
                ]
            }));
            let evaluator = Evaluator::new(&state);
            let expr = parse("as_unordered_list(depends_on)").unwrap();

            match evaluator.eval(&expr) {
                EvalResult::Value(s) => assert_eq!(s, "- alpha\n  - beta\n  - gamma"),
                _ => panic!("Expected Value"),
            }
        }

        /// Equality comparison keeps `scalar_string`'s JSON-array form, so the
        /// interpolation output change does not leak into `==` (spec criterion 8).
        #[test]
        fn equality_comparison_unaffected_by_line_separated_output() {
            let state = create_test_state(json!({"items": ["a", "b"]}));
            let evaluator = Evaluator::new(&state);
            let expr = parse(r#"items == '["a","b"]' ? "yes" : "no""#).unwrap();

            match evaluator.eval(&expr) {
                EvalResult::Value(s) => assert_eq!(s, "yes"),
                _ => panic!("Expected Value"),
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
            let expr = parse(r#"color || "unknown""#).unwrap();

            match evaluator.eval(&expr) {
                EvalResult::Value(s) => assert_eq!(s, "blue"),
                _ => panic!("Expected Value"),
            }
        }

        #[test]
        fn uses_fallback_if_falsy() {
            let state = create_test_state(json!({}));
            let evaluator = Evaluator::new(&state);
            let expr = parse(r#"missing || "default""#).unwrap();

            match evaluator.eval(&expr) {
                EvalResult::Value(s) => assert_eq!(s, "default"),
                _ => panic!("Expected Value"),
            }
        }

        #[test]
        fn uses_fallback_for_empty_string() {
            let state = create_test_state(json!({"empty": ""}));
            let evaluator = Evaluator::new(&state);
            let expr = parse(r#"empty || "fallback""#).unwrap();

            match evaluator.eval(&expr) {
                EvalResult::Value(s) => assert_eq!(s, "fallback"),
                _ => panic!("Expected Value"),
            }
        }

        #[test]
        fn uses_fallback_for_null() {
            let state = create_test_state(json!({"nothing": null}));
            let evaluator = Evaluator::new(&state);
            let expr = parse(r#"nothing || "fallback""#).unwrap();

            match evaluator.eval(&expr) {
                EvalResult::Value(s) => assert_eq!(s, "fallback"),
                _ => panic!("Expected Value"),
            }
        }

        #[test]
        fn uses_fallback_for_false() {
            let state = create_test_state(json!({"flag": false}));
            let evaluator = Evaluator::new(&state);
            let expr = parse(r#"flag || "fallback""#).unwrap();

            match evaluator.eval(&expr) {
                EvalResult::Value(s) => assert_eq!(s, "fallback"),
                _ => panic!("Expected Value"),
            }
        }

        #[test]
        fn uses_fallback_for_zero() {
            let state = create_test_state(json!({"count": 0}));
            let evaluator = Evaluator::new(&state);
            let expr = parse(r#"count || "fallback""#).unwrap();

            match evaluator.eval(&expr) {
                EvalResult::Value(s) => assert_eq!(s, "fallback"),
                _ => panic!("Expected Value"),
            }
        }

        #[test]
        fn chained_fallback_uses_first_truthy() {
            let state = create_test_state(json!({"b": "second"}));
            let evaluator = Evaluator::new(&state);
            // a || b || c parses as Fallback(Fallback(a, b), c)
            let expr = parse(r#"a || b || "third""#).unwrap();

            match evaluator.eval(&expr) {
                EvalResult::Value(s) => assert_eq!(s, "second"),
                _ => panic!("Expected Value"),
            }
        }

        #[test]
        fn chained_fallback_uses_last_if_all_falsy() {
            let state = create_test_state(json!({}));
            let evaluator = Evaluator::new(&state);
            let expr = parse(r#"a || b || "default""#).unwrap();

            match evaluator.eval(&expr) {
                EvalResult::Value(s) => assert_eq!(s, "default"),
                _ => panic!("Expected Value"),
            }
        }

        #[test]
        fn fallback_to_variable() {
            let state = create_test_state(json!({"backup": "backup_value"}));
            let evaluator = Evaluator::new(&state);
            let expr = parse("missing || backup").unwrap();

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

        #[test]
        fn nested_ternary_true_branch() {
            // a ? b ? c : d : e
            // When a is truthy and b is truthy -> c
            let state = create_test_state(json!({"a": true, "b": true}));
            let evaluator = Evaluator::new(&state);
            let expr = parse(r#"a ? b ? "c" : "d" : "e""#).unwrap();

            match evaluator.eval(&expr) {
                EvalResult::Value(s) => assert_eq!(s, "c"),
                _ => panic!("Expected Value"),
            }
        }

        #[test]
        fn nested_ternary_inner_false_branch() {
            // a ? b ? c : d : e
            // When a is truthy and b is falsy -> d
            let state = create_test_state(json!({"a": true, "b": false}));
            let evaluator = Evaluator::new(&state);
            let expr = parse(r#"a ? b ? "c" : "d" : "e""#).unwrap();

            match evaluator.eval(&expr) {
                EvalResult::Value(s) => assert_eq!(s, "d"),
                _ => panic!("Expected Value"),
            }
        }

        #[test]
        fn nested_ternary_outer_false_branch() {
            // a ? b ? c : d : e
            // When a is falsy -> e
            let state = create_test_state(json!({"a": false}));
            let evaluator = Evaluator::new(&state);
            let expr = parse(r#"a ? b ? "c" : "d" : "e""#).unwrap();

            match evaluator.eval(&expr) {
                EvalResult::Value(s) => assert_eq!(s, "e"),
                _ => panic!("Expected Value"),
            }
        }

        #[test]
        fn nested_ternary_in_false_branch() {
            // a ? b : c ? d : e
            // When a is falsy and c is truthy -> d
            let state = create_test_state(json!({"a": false, "c": true}));
            let evaluator = Evaluator::new(&state);
            let expr = parse(r#"a ? "b" : c ? "d" : "e""#).unwrap();

            match evaluator.eval(&expr) {
                EvalResult::Value(s) => assert_eq!(s, "d"),
                _ => panic!("Expected Value"),
            }
        }

        #[test]
        fn deeply_nested_ternary() {
            // a ? b ? c ? d : e : f : g
            let state = create_test_state(json!({"a": true, "b": true, "c": false}));
            let evaluator = Evaluator::new(&state);
            let expr = parse(r#"a ? b ? c ? "d" : "e" : "f" : "g""#).unwrap();

            match evaluator.eval(&expr) {
                EvalResult::Value(s) => assert_eq!(s, "e"),
                _ => panic!("Expected Value"),
            }
        }

        #[test]
        fn nested_ternary_with_mixed_variables_and_literals() {
            let state = create_test_state(json!({
                "flag": true,
                "name": "Alice",
                "default": "unknown"
            }));
            let evaluator = Evaluator::new(&state);
            let expr = parse(r#"flag ? name ? name : default : "missing""#).unwrap();

            match evaluator.eval(&expr) {
                EvalResult::Value(s) => assert_eq!(s, "Alice"),
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
            let expr = parse("count || 0").unwrap();

            match evaluator.eval_value(&expr) {
                EvalValue::Number(n) => assert_eq!(n, 42.0),
                _ => panic!("Expected Number"),
            }
        }

        #[test]
        fn fallback_returns_fallback_type() {
            let state = create_test_state(json!({}));
            let evaluator = Evaluator::new(&state);
            let expr = parse(r#"missing || "default""#).unwrap();

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
                EvalValue::String("3.15".to_string()).as_number(),
                Some(3.15)
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
                EvalResult::Value(s) => assert_eq!(s, "0"),
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
                EvalResult::Error { error, original } => {
                    assert!(error.to_string().contains("Unknown function"));
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
            let expr = parse(r#"length(items) || "no items""#).unwrap();

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
            let expr = parse(r#"has_key(user, "name")"#).unwrap();

            match evaluator.eval(&expr) {
                EvalResult::Value(v) => assert_eq!(v, "true"),
                _ => panic!("Expected Value"),
            }
        }

        #[test]
        fn and_or_functions() {
            let state = create_test_state(json!({"a": true, "b": false}));
            let evaluator = Evaluator::new(&state);

            let and_expr = parse("and(a, b)").unwrap();
            let or_expr = parse("or(a, b)").unwrap();

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

    /// Integration coverage for the operators, access forms, and helpers
    /// added during the expression-syntax expansion. These tests exercise
    /// the full path from `{{ ... }}` parsing through evaluation to ensure
    /// new functionality is reachable from interpolation.
    mod expression_syntax_integration {
        use super::*;

        fn render(state: &EffectiveState, source: &str) -> String {
            let evaluator = Evaluator::new(state);
            let expr = parse(source).unwrap_or_else(|e| panic!("parse {source:?}: {e}"));
            match evaluator.eval(&expr) {
                EvalResult::Value(s) => s,
                EvalResult::Error { error, .. } => {
                    panic!("eval {source:?} returned error: {error}")
                }
            }
        }

        #[test]
        fn arithmetic_in_interpolation() {
            let state = create_test_state(json!({ "count": 10 }));
            assert_eq!(render(&state, "count + 5"), "15");
            assert_eq!(render(&state, "count - 4"), "6");
            assert_eq!(render(&state, "count * 2"), "20");
            assert_eq!(render(&state, "count / 4"), "2.5");
            assert_eq!(render(&state, "count % 3"), "1");
        }

        #[test]
        fn string_concatenation_via_plus() {
            let state = create_test_state(json!({ "name": "Alice", "n": 3 }));
            assert_eq!(render(&state, r#""hello, " + name"#), "hello, Alice");
            assert_eq!(render(&state, r#"n + " items""#), "3 items");
        }

        #[test]
        fn less_than_or_equal_in_ternary() {
            let state = create_test_state(json!({ "count": 3 }));
            assert_eq!(render(&state, r#"count <= 3 ? "small" : "big""#), "small");
            let state = create_test_state(json!({ "count": 4 }));
            assert_eq!(render(&state, r#"count <= 3 ? "small" : "big""#), "big");
        }

        #[test]
        fn bracket_index_access() {
            let state = create_test_state(json!({ "items": ["a", "b", "c"] }));
            assert_eq!(render(&state, "items[0]"), "a");
            assert_eq!(render(&state, "items[-1]"), "c");
            // out-of-range -> null -> empty string
            assert_eq!(render(&state, "items[99]"), "");
        }

        #[test]
        fn bracket_object_key_access() {
            let state = create_test_state(json!({ "config": { "theme": "dark" } }));
            assert_eq!(render(&state, r#"config["theme"]"#), "dark");
            assert_eq!(render(&state, r#"config["missing"]"#), "");
        }

        #[test]
        fn chained_index_and_member_access() {
            let state = create_test_state(json!({
                "items": [{ "name": "first" }, { "name": "second" }]
            }));
            assert_eq!(render(&state, "items[-1].name"), "second");
            assert_eq!(render(&state, "items[0].name"), "first");
        }

        #[test]
        fn type_predicates_in_interpolation() {
            let state = create_test_state(json!({
                "tags": ["one"], "title": "doc", "count": 3
            }));
            assert_eq!(render(&state, "is_array(tags)"), "true");
            assert_eq!(render(&state, "is_string(title)"), "true");
            assert_eq!(render(&state, "is_number(count)"), "true");
            assert_eq!(render(&state, "is_empty(missing)"), "true");
            assert_eq!(render(&state, "is_empty(tags)"), "false");
        }

        #[test]
        fn collection_and_math_helpers() {
            let state = create_test_state(json!({ "items": [10, 20, 30] }));
            assert_eq!(render(&state, "first(items)"), "10");
            assert_eq!(render(&state, "last(items)"), "30");
            assert_eq!(render(&state, "min(2, 5)"), "2");
            assert_eq!(render(&state, "max(2, 5)"), "5");
            assert_eq!(render(&state, "abs(-7)"), "7");
        }

        #[test]
        fn string_predicates_and_mutations() {
            let state = create_test_state(json!({ "title": "Hello World" }));
            assert_eq!(render(&state, r#"starts_with(title, "Hello")"#), "true");
            assert_eq!(render(&state, r#"ends_with(title, "World")"#), "true");
            assert_eq!(render(&state, "lower(title)"), "hello world");
            assert_eq!(render(&state, "upper(title)"), "HELLO WORLD");
            assert_eq!(render(&state, "kebab_case(title)"), "hello-world");
            assert_eq!(render(&state, "snake_case(title)"), "hello_world");
            assert_eq!(render(&state, "camel_case(title)"), "helloWorld");
            assert_eq!(render(&state, "pascal_case(title)"), "HelloWorld");
        }

        #[test]
        fn date_validators_in_interpolation() {
            let state = create_test_state(json!({
                "good_date": "2024-06-15",
                "bad_date": "06-15-2024",
                "good_dt": "2024-06-15T12:30:00Z"
            }));
            assert_eq!(render(&state, "is_date(good_date)"), "true");
            assert_eq!(render(&state, "is_date(bad_date)"), "false");
            assert_eq!(render(&state, "is_date_time(good_dt)"), "true");
            assert_eq!(render(&state, "is_date(missing)"), "false");
        }

        #[test]
        fn null_propagation_through_helpers() {
            let state = create_test_state(json!({}));
            assert_eq!(render(&state, "min(missing, 5)"), "");
            assert_eq!(render(&state, "lower(missing)"), "");
            assert_eq!(render(&state, "first(missing)"), "");
        }

        #[test]
        fn arithmetic_division_by_zero_surfaces_error() {
            let state = create_test_state(json!({}));
            let evaluator = Evaluator::new(&state);
            let expr = parse("10 / 0").unwrap();
            assert!(evaluator.eval(&expr).is_error());
        }

        #[test]
        fn arithmetic_non_numeric_operand_errors() {
            let state = create_test_state(json!({ "items": [1, 2, 3] }));
            let evaluator = Evaluator::new(&state);
            let expr = parse("items - 1").unwrap();
            assert!(evaluator.eval(&expr).is_error());
        }
    }
}
