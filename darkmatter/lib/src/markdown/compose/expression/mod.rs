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
//! - `user.name` - Nested property access (dot syntax for named properties)
//! - `items[0]` - Array index access (negative indexes count from end)
//! - `config["key"]` - Object key access
//! - `ctx.today` - Context variable
//! - `env.HOME` - Environment variable
//!
//! Operators (precedence high → low):
//! 1. Primary / member access (literals, variables, function calls,
//!    `foo.bar`, `foo[0]`, `(expr)`)
//! 2. Unary `!`, `-`
//! 3. Multiplicative `*`, `/`, `%`
//! 4. Additive `+`, `-` (`+` coerces a numeric string paired with a number,
//!    and otherwise doubles as string concatenation)
//! 5. Comparison `==`, `!=`, `>`, `>=`, `<`, `<=`
//! 6. Logical AND `&&`
//! 7. Logical OR / Fallback `||`
//! 8. Ternary `? :` (right-associative; all binary operators are
//!    left-associative)
//!
//! ## Parser Modes
//!
//! `&&` is logical AND in both modes. Only `||` differs:
//! - **Interpolation** (`ParseMode::Interpolation`) - `||` is fallback operator
//! - **Condition** (`ParseMode::Condition`) - `||` is logical OR
//!
//! ## Truthiness
//!
//! Falsy values: `null`, `false`, `0`, `0.0`, `""`, `[]`, `{}`. Everything
//! else is truthy.
//!
//! ## Null Propagation
//!
//! - Dot access on a `null` base or missing path returns `null` (no error).
//! - Bracket access never errors: out-of-bounds, `null` base, key on non-collection,
//!   and missing object keys all return `null`.
//! - Functions added in the expression-syntax expansion (math, collection,
//!   string predicates / mutations) propagate `null` arguments through to
//!   `null` results, and return errors for type mismatches.
//!
//! ## Arithmetic Errors
//!
//! Division by zero (`x / 0`) and remainder by zero (`x % 0`) raise
//! evaluator errors. Non-numeric operands for `-`, `*`, `/`, `%` also raise
//! errors; `+` concatenates operands that do not form a mixed numeric pair.
//!
//! For full grammar, helper catalog, and timezone behavior see the
//! [Darkmatter Expressions](../../../../docs/topics/darkmatter-expressions.md)
//! topic.

pub mod ast;
pub mod catalog;
pub mod ctx;
pub(crate) mod doc_namespace;
pub mod error;
pub mod file_suggestions;
pub mod functions;
pub mod lexer;
pub(crate) mod path_projection;
pub mod parser;
pub mod resolve_ctx;
pub mod semantics;

pub use ast::{BinaryOp, Expr, SpannedExpr, SpannedExprKind};
pub use catalog::{
    expression_function_descriptors, generate_expression_function_table,
    DataType, ExpressionFunctionDescriptor, ParamType, ReturnType,
};
pub use ctx::CtxLookup;
pub use error::{ArityBound, ExpressionError, FileRefFailure, FileReferenceDiagnostic};
pub use file_suggestions::{collect_sibling_candidates, suggest_sibling_files};
pub(crate) use path_projection::{make_portable_relative, make_relative};
pub use resolve_ctx::ResolutionContext;
pub use lexer::{
    ComparisonOp, ExpressionFinder, ExpressionLocation, ExpressionScanResult, InterpolationLiteral,
    Lexer, LexerError, ParseMode, Token, lex_spanned,
};
pub use parser::{
    ParseError, Parser, parse, parse_condition, parse_condition_spanned, parse_spanned,
};

use serde_json::Value;

use crate::catalog::{describe, describe_for_error, suggest, Described};

/// Converts a value to a number for arithmetic operations.
///
/// Accepts numbers and parseable strings; rejects booleans, null, arrays,
/// and objects.
fn to_number_arithmetic(value: &Value) -> Option<f64> {
    match value {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.parse::<f64>().ok(),
        _ => None,
    }
}

/// Converts an expression value into a number for arithmetic operations,
/// returning an error message tagged with the originating operator when the
/// value cannot be represented as a number.
fn require_number(value: &Value, op_label: &'static str) -> Result<f64, ExpressionError> {
    to_number_arithmetic(value).ok_or(ExpressionError::Arithmetic { op: op_label })
}

/// Converts a value to a number for array indexing.
///
/// Only actual numbers are accepted; all other types (including strings and
/// booleans) are rejected.
fn to_number_index(value: &Value) -> Option<f64> {
    match value {
        Value::Number(n) => n.as_f64(),
        _ => None,
    }
}

fn json_number(value: f64) -> Result<Value, String> {
    if !value.is_finite() {
        return Err(format!("Arithmetic produced a non-finite number: {value}"));
    }
    if value.fract() == 0.0 && value.abs() < (i64::MAX as f64) {
        return Ok(Value::Number(serde_json::Number::from(value as i64)));
    }
    serde_json::Number::from_f64(value)
        .map(Value::Number)
        .ok_or_else(|| format!("Unable to represent number: {value}"))
}

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

    /// Returns the resolution context for read-side filesystem, document, and
    /// repository functions.
    ///
    /// The default `None` is the **opt-out / test** case: a lookup that has no
    /// document anchor (or a unit test that does not exercise read-side
    /// functions). Every production surface that evaluates the grammar against
    /// a real document — frontmatter interpolation, body interpolation, `$()`
    /// ternary conditions, `when=` conditions, the public condition API, and
    /// claudine's loop/hook conditions — overrides this to return
    /// `Some(ctx)` so read-side functions resolve identically wherever the
    /// grammar runs. A `None`-returning lookup makes a read-side function
    /// return the recoverable "requires a document resolution context" error.
    fn resolution_context(&self) -> Option<ResolutionContext> {
        None
    }

    /// Borrowed companion to [`resolution_context`](Self::resolution_context).
    ///
    /// The owned method above is the public, compatibility-preserving accessor.
    /// This borrowed variant lets the evaluator dispatch a read-side function
    /// against a lookup's context **without cloning** it (Finding 12): a
    /// document with many `frontmatter()` / `file_exists()` calls would
    /// otherwise deep-clone the context — its `PathBuf`s, magic-path vector, and
    /// captured `ctx` map — once per call. Implementors that own or borrow a
    /// context override this to return `Some(&ctx)`; the evaluator prefers it
    /// and only falls back to the owned clone when it yields `None`.
    ///
    /// The default returns `None`, so a lookup that overrides only the owned
    /// method keeps working unchanged (the evaluator's owned fallback covers
    /// it).
    fn resolution_context_ref(&self) -> Option<&ResolutionContext> {
        None
    }

    /// Returns true when `name` is a known runtime context variable.
    ///
    /// The default implementation always returns `false`, which disables
    /// parser-aware typo detection for lookups that don't expose a context
    /// catalog.
    fn is_valid_context_variable(&self, _name: &str) -> bool {
        false
    }

    /// Returns the list of known runtime context variable names.
    ///
    /// The default implementation returns an empty slice. Implementors that
    /// expose a context catalog should override this so typo diagnostics can
    /// offer did-you-mean suggestions.
    fn context_variable_names(&self) -> &[&'static str] {
        &[]
    }

    /// Returns `true` when `root` is a known variable root for this lookup.
    ///
    /// `root` is the first dotted segment of a variable path (so `err.msg`
    /// contributes `err`, `ctx.today` contributes `ctx`). Strict-mode subtree
    /// compose ([`compose_subtree`](crate::markdown::compose::subtree::compose_subtree)
    /// with [`SubtreeStrictness::Strict`]) rejects a reference whose root is
    /// *not* known; a known root that resolves to `null`/empty still renders
    /// empty.
    ///
    /// The default `true` preserves existing lenient behavior for lookups that
    /// do not participate in strict-mode subtree compose.
    ///
    /// [`SubtreeStrictness::Strict`]: crate::markdown::compose::subtree::SubtreeStrictness::Strict
    fn is_known_variable_root(&self, _root: &str) -> bool {
        true
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

/// Renders a value for the interpolation output boundary.
///
/// Identical to [`scalar_string`] except a top-level array renders
/// line-separated (spec D4 default), so `{{ ctx.some_list }}` ≡
/// `{{ as_line_separated(ctx.some_list) }}`. Equality comparison and
/// frontmatter shell expansion keep calling [`scalar_string`] directly (the
/// byte-identical JSON-array form), so only interpolation output changes.
pub fn interpolation_output_string(value: &Value) -> String {
    match value {
        Value::Array(items) => items
            .iter()
            .map(scalar_string)
            .collect::<Vec<_>>()
            .join("\n"),
        other => scalar_string(other),
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
pub fn evaluate<L: EvaluationLookup>(expr: &Expr, lookup: &L) -> Result<Value, ExpressionError> {
    match expr {
        Expr::Variable(path) => Ok(lookup.get(path).unwrap_or(Value::Null)),
        Expr::StringLiteral(s) => Ok(Value::String(s.clone())),
        Expr::NumberLiteral(n) => {
            let num = if n.fract() == 0.0 {
                serde_json::Number::from(*n as i64)
            } else {
                serde_json::Number::from_f64(*n).ok_or_else(|| {
                    ExpressionError::Parse(format!("Invalid numeric literal: {n}"))
                })?
            };
            Ok(Value::Number(num))
        }
        Expr::BoolLiteral(b) => Ok(Value::Bool(*b)),
        Expr::Paren(inner) => evaluate(inner, lookup),
        Expr::UnaryNot(inner) => {
            let value = evaluate(inner, lookup)?;
            Ok(Value::Bool(!is_truthy(&value)))
        }
        Expr::UnaryMinus(inner) => {
            let value = evaluate(inner, lookup)?;
            if value.is_null() {
                return Ok(Value::Null);
            }
            let num = require_number(&value, "Unary '-'")?;
            json_number(-num).map_err(|message| ExpressionError::Other {
                function: "Unary '-'".to_string(),
                message,
            })
        }
        Expr::Binary { op, left, right } => {
            let left = evaluate(left, lookup)?;
            let right = evaluate(right, lookup)?;
            evaluate_binary(*op, &left, &right)
        }
        Expr::Index { base, index } => {
            let base = evaluate(base, lookup)?;
            let index = evaluate(index, lookup)?;
            Ok(evaluate_index(&base, &index))
        }
        Expr::MemberAccess { base, name } => {
            let base = evaluate(base, lookup)?;
            Ok(evaluate_member(&base, name))
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
                ComparisonOp::LessThanOrEqual => {
                    to_number_coerce(&left) <= to_number_coerce(&right)
                }
            };
            Ok(Value::Bool(outcome))
        }
        Expr::FunctionCall { name, args } => evaluate_function(name, args, lookup),
    }
}

fn evaluate_binary(op: BinaryOp, left: &Value, right: &Value) -> Result<Value, ExpressionError> {
    let mixed_number_and_numeric_string = op == BinaryOp::Add
        && matches!(
            (left, right),
            (Value::Number(_), Value::String(_)) | (Value::String(_), Value::Number(_))
        )
        && to_number_arithmetic(left).is_some()
        && to_number_arithmetic(right).is_some();
    if op == BinaryOp::Add
        && (left.is_string() || right.is_string())
        && !mixed_number_and_numeric_string
    {
        return Ok(Value::String(format!(
            "{}{}",
            scalar_string(left),
            scalar_string(right)
        )));
    }

    let label = match op {
        BinaryOp::Add => "Addition",
        BinaryOp::Sub => "Subtraction",
        BinaryOp::Mul => "Multiplication",
        BinaryOp::Div => "Division",
        BinaryOp::Mod => "Remainder",
    };
    let lhs = require_number(left, label)?;
    let rhs = require_number(right, label)?;

    let result = match op {
        BinaryOp::Add => lhs + rhs,
        BinaryOp::Sub => lhs - rhs,
        BinaryOp::Mul => lhs * rhs,
        BinaryOp::Div => {
            if rhs == 0.0 {
                return Err(ExpressionError::Other {
                    function: "division".to_string(),
                    message: "Division by zero".to_string(),
                });
            }
            lhs / rhs
        }
        BinaryOp::Mod => {
            if rhs == 0.0 {
                return Err(ExpressionError::Other {
                    function: "remainder".to_string(),
                    message: "Remainder by zero".to_string(),
                });
            }
            // C-style remainder: sign follows dividend (Rust's `%` already does this for f64).
            lhs % rhs
        }
    };
    json_number(result).map_err(|message| ExpressionError::Other {
        function: label.to_string(),
        message,
    })
}

fn evaluate_index(base: &Value, index: &Value) -> Value {
    match base {
        Value::Null => Value::Null,
        Value::Array(items) => {
            let Some(n) = to_number_index(index) else {
                return Value::Null;
            };
            if n.fract() != 0.0 {
                return Value::Null;
            }
            let len = items.len() as i64;
            let idx = n as i64;
            let resolved = if idx < 0 { len + idx } else { idx };
            if resolved < 0 || resolved >= len {
                Value::Null
            } else {
                items[resolved as usize].clone()
            }
        }
        Value::Object(map) => match index {
            Value::String(s) => map.get(s).cloned().unwrap_or(Value::Null),
            _ => Value::Null,
        },
        _ => Value::Null,
    }
}

fn evaluate_member(base: &Value, name: &str) -> Value {
    if base.is_null() {
        return Value::Null;
    }
    let mut current = base.clone();
    for segment in name.split('.') {
        match current {
            Value::Object(mut map) => {
                current = map.remove(segment).unwrap_or(Value::Null);
            }
            _ => return Value::Null,
        }
    }

    current
}

/// Error-message prefix for an unrecognized function name.
///
/// A stable contract: interpolation treats evaluation errors starting with
/// this prefix as fatal even in non-fail-fast mode, since an unknown symbol can
/// never resolve and would otherwise leak its literal `{{ … }}` text downstream.
pub(crate) const UNKNOWN_FUNCTION_PREFIX: &str = "Unknown function:";

/// Whether an evaluator error message is an arity (wrong-argument-count) error.
///
/// Arity errors read "… requires N argument(s)" / "… requires 1 or 2 arguments"
/// / "… requires at least 1 argument". Type errors also contain "requires …
/// argument" but name the rejected domain ("numeric"/"string"/"array"), so
/// those are excluded.
fn is_arity_error(message: &str) -> bool {
    let m = message.to_lowercase();
    m.contains("requires")
        && m.contains("argument")
        && !m.contains("numeric")
        && !m.contains("string argument")
        && !m.contains("array argument")
}

/// Returns a plain-text error for an unrecognized function name, with a fuzzy
/// did-you-mean suggestion when one exists.
fn unknown_function_error(name: &str) -> ExpressionError {
    let mut text = format!("{UNKNOWN_FUNCTION_PREFIX} {name}");
    if let Some(suggestion) = suggest(expression_function_descriptors(), name, 1).first() {
        text.push_str("\n\nDid you mean:\n  ");
        text.push_str(&describe_for_error(*suggestion));
    }
    ExpressionError::UnknownFunction {
        name: text
            .strip_prefix(UNKNOWN_FUNCTION_PREFIX)
            .map(str::trim)
            .unwrap_or(name)
            .to_string(),
    }
}

/// Appends the matched function descriptor's signature, description, and example
/// to an arity error message.
fn enrich_arity_error(name: &str, message: &str) -> String {
    let signature = expression_function_descriptors()
        .iter()
        .find(|d| d.key().starts_with(&format!("{name}(")) || d.key() == name)
        .map(|d| d.key());
    match signature {
        Some(sig) if let Some(descriptor) = describe(expression_function_descriptors(), sig) => {
            format!("{message}\n\nExpected:\n  {}", describe_for_error(descriptor))
        }
        _ => message.to_string(),
    }
}

fn classify_function_error(function: &str, message: String) -> ExpressionError {
    let message = if is_arity_error(&message) {
        enrich_arity_error(function, &message)
    } else {
        message
    };
    ExpressionError::Other {
        function: function.to_string(),
        message,
    }
}

fn evaluate_function<L: EvaluationLookup>(
    name: &str,
    args: &[Expr],
    lookup: &L,
) -> Result<Value, ExpressionError> {
    let name = name.to_ascii_lowercase();
    match name.as_str() {
        // `and`/`or` short-circuit, so they must evaluate their arguments
        // lazily and stay here rather than in the eagerly-evaluated registries.
        "and" => {
            match functions::lazy_arity_eligibility("and", args.len()) {
                Some(functions::LazyArityEligibility::Eligible) => {}
                Some(functions::LazyArityEligibility::Ineligible(message)) => {
                    return Err(classify_function_error("and", message));
                }
                None => return Err(unknown_function_error("and")),
            }
            for arg in args {
                let value = evaluate(arg, lookup)?;
                if !is_truthy(&value) {
                    return Ok(Value::Bool(false));
                }
            }
            Ok(Value::Bool(true))
        }
        "or" => {
            match functions::lazy_arity_eligibility("or", args.len()) {
                Some(functions::LazyArityEligibility::Eligible) => {}
                Some(functions::LazyArityEligibility::Ineligible(message)) => {
                    return Err(classify_function_error("or", message));
                }
                None => return Err(unknown_function_error("or")),
            }
            for arg in args {
                let value = evaluate(arg, lookup)?;
                if is_truthy(&value) {
                    return Ok(Value::Bool(true));
                }
            }
            Ok(Value::Bool(false))
        }
        // Every other function evaluates its arguments eagerly and resolves
        // through the authoritative dispatch tables in `functions`.
        other => {
            let evaluated: Vec<Value> = args
                .iter()
                .map(|arg| evaluate(arg, lookup))
                .collect::<Result<_, _>>()?;
            // Prefer the borrowed context (Finding 12) so a read-side function
            // dispatched here does not deep-clone the lookup's context; only
            // fall back to the owned clone for lookups that expose only the
            // owned accessor.
            let ctx = lookup
                .resolution_context_ref()
                .map(std::borrow::Cow::Borrowed)
                .or_else(|| lookup.resolution_context().map(std::borrow::Cow::Owned));
            if let Some(ctx) = ctx
                && let Some(result) = functions::dispatch_fs(other, &evaluated, &ctx)
            {
                return result.map_err(|error| match error {
                    ExpressionError::Other { function, message } if is_arity_error(&message) => {
                        ExpressionError::Other {
                            function,
                            message: enrich_arity_error(other, &message),
                        }
                    }
                    other => other,
                });
            }
            if let Some(result) = functions::dispatch(other, &evaluated) {
                return result.map_err(|message| classify_function_error(other, message));
            }
            // A known filesystem function reaches here only because the lookup
            // returned no resolution context — an opt-out or test lookup, not a
            // real document surface (all of which now supply one). Keep it
            // recoverable so it doesn't read as an unknown symbol.
            if functions::is_fs_function(other) {
                return Err(ExpressionError::Other {
                    function: other.to_string(),
                    message: format!(
                        "Filesystem function '{name}' requires a document resolution context, which is unavailable here"
                    ),
                });
            }
            Err(unknown_function_error(other))
        }
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

    mod output_rendering {
        use super::*;

        #[test]
        fn scalar_string_keeps_json_array_form() {
            // Equality comparison and frontmatter shell expansion rely on this
            // byte-identical JSON form (spec criterion 8); it must not change.
            assert_eq!(scalar_string(&json!(["a", "b", "c"])), r#"["a","b","c"]"#);
            assert_eq!(scalar_string(&json!([])), "[]");
        }

        #[test]
        fn interpolation_output_string_renders_arrays_line_separated() {
            assert_eq!(
                interpolation_output_string(&json!(["a", "b", "c"])),
                "a\nb\nc"
            );
            assert_eq!(interpolation_output_string(&json!([])), "");
        }

        #[test]
        fn interpolation_output_string_matches_scalar_string_for_non_arrays() {
            for value in [json!("hi"), json!(42), json!(true), json!(null), json!({"a": 1})] {
                assert_eq!(
                    interpolation_output_string(&value),
                    scalar_string(&value),
                    "non-array rendering must match scalar_string for {value:?}"
                );
            }
        }
    }

    mod error_enrichment {
        use super::*;

        #[test]
        fn unknown_function_includes_did_you_mean() {
            let err = evaluate(
                &parse("lenght(\"abc\")").unwrap(),
                &lookup(json!({})),
            )
            .unwrap_err();
            assert!(
                err.contains("Unknown function:"),
                "error should identify unknown function: {err}"
            );
            assert!(
                err.contains("Did you mean"),
                "error should offer a did-you-mean hint: {err}"
            );
            assert!(
                err.contains("length"),
                "error should suggest 'length': {err}"
            );
        }

        #[test]
        fn unknown_function_without_close_match_omits_suggestion() {
            // "xyzxyzxyz" is unrelated to every catalog function; the suggestion
            // quality gate (threshold max(2, 9/3) = 3) rejects every candidate,
            // so the error must carry the bare prefix and NO did-you-mean hint.
            let err = evaluate(
                &parse("xyzxyzxyz()").unwrap(),
                &lookup(json!({})),
            )
            .unwrap_err();
            assert!(err.starts_with("Unknown function:"), "{err}");
            assert!(
                !err.contains("Did you mean"),
                "unrelated typo must not emit a close-match suggestion: {err}"
            );
        }

        #[test]
        fn arity_error_includes_signature_and_example() {
            let err = evaluate(&parse("length()").unwrap(), &lookup(json!({}))).unwrap_err();
            assert!(
                err.contains("requires 1 argument"),
                "error should preserve arity message: {err}"
            );
            assert!(
                err.contains("length("),
                "error should include function signature: {err}"
            );
            assert!(
                err.contains("example:"),
                "error should include an example: {err}"
            );
        }
    }

    mod parity {
        use super::*;

        #[test]
        fn interpolation_fallback_versus_condition_or() {
            // Interpolation: a || "default" -> Fallback
            // Condition: a || "default" -> or(a, "default")
            // Both should return the truthy value
            let state = lookup(json!({"a": "present"}));
            let fallback_expr = Expr::Fallback {
                primary: Box::new(Expr::Variable("a".to_string())),
                fallback: Box::new(Expr::StringLiteral("default".to_string())),
            };
            let or_expr = Expr::FunctionCall {
                name: "or".to_string(),
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
                name: "or".to_string(),
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
                name: "and".to_string(),
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
        fn lazy_operators_accept_authored_minimum_arity() {
            let state = lookup(json!({}));
            assert_eq!(evaluate(&parse("and()").unwrap(), &state).unwrap(), json!(true));
            assert_eq!(evaluate(&parse("or()").unwrap(), &state).unwrap(), json!(false));
        }

        #[test]
        fn lazy_operators_accept_representative_variadic_arity() {
            let state = lookup(json!({}));
            assert_eq!(
                evaluate(&parse("and(true, 1, \"yes\")").unwrap(), &state).unwrap(),
                json!(true)
            );
            assert_eq!(
                evaluate(&parse("or(false, null, \"yes\")").unwrap(), &state).unwrap(),
                json!(true)
            );
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
                name: "has_key".to_string(),
                args: vec![
                    Expr::Variable("user".to_string()),
                    Expr::StringLiteral("name".to_string()),
                ],
            };
            assert_eq!(evaluate(&expr, &state).unwrap(), json!(true));
        }

        #[test]
        fn bool_literal_evaluates_to_json_bool() {
            assert_eq!(
                evaluate(&Expr::BoolLiteral(true), &lookup(json!({}))).unwrap(),
                json!(true)
            );
            assert_eq!(
                evaluate(&Expr::BoolLiteral(false), &lookup(json!({}))).unwrap(),
                json!(false)
            );
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

    mod comparison_operators {
        use super::*;

        fn cmp(left: Value, op: ComparisonOp, right: Value) -> Result<Value, String> {
            let state = lookup(json!({}));
            let expr = Expr::Comparison {
                left: Box::new(literal(left)),
                op,
                right: Box::new(literal(right)),
            };
            evaluate(&expr, &state).map_err(|error| error.to_string())
        }

        fn literal(value: Value) -> Expr {
            match value {
                Value::String(s) => Expr::StringLiteral(s),
                Value::Number(n) => Expr::NumberLiteral(n.as_f64().unwrap()),
                Value::Bool(b) => Expr::BoolLiteral(b),
                Value::Null => Expr::Variable("__missing__".to_string()),
                _ => panic!("only scalar literals supported in this helper"),
            }
        }

        #[test]
        fn equal_and_not_equal_numeric() {
            assert_eq!(
                cmp(json!(5), ComparisonOp::Equal, json!(5)).unwrap(),
                json!(true)
            );
            assert_eq!(
                cmp(json!(5), ComparisonOp::NotEqual, json!(6)).unwrap(),
                json!(true)
            );
        }

        #[test]
        fn greater_than_and_greater_than_or_equal() {
            assert_eq!(
                cmp(json!(6), ComparisonOp::GreaterThan, json!(5)).unwrap(),
                json!(true)
            );
            assert_eq!(
                cmp(json!(5), ComparisonOp::GreaterThan, json!(5)).unwrap(),
                json!(false)
            );
            assert_eq!(
                cmp(json!(5), ComparisonOp::GreaterThanOrEqual, json!(5)).unwrap(),
                json!(true)
            );
            assert_eq!(
                cmp(json!(4), ComparisonOp::GreaterThanOrEqual, json!(5)).unwrap(),
                json!(false)
            );
        }

        #[test]
        fn less_than_and_less_than_or_equal() {
            assert_eq!(
                cmp(json!(4), ComparisonOp::LessThan, json!(5)).unwrap(),
                json!(true)
            );
            assert_eq!(
                cmp(json!(5), ComparisonOp::LessThan, json!(5)).unwrap(),
                json!(false)
            );
            assert_eq!(
                cmp(json!(5), ComparisonOp::LessThanOrEqual, json!(5)).unwrap(),
                json!(true)
            );
            assert_eq!(
                cmp(json!(6), ComparisonOp::LessThanOrEqual, json!(5)).unwrap(),
                json!(false)
            );
        }

        #[test]
        fn comparisons_coerce_string_backed_numerics() {
            assert_eq!(
                cmp(json!("5"), ComparisonOp::GreaterThan, json!(3)).unwrap(),
                json!(true)
            );
            assert_eq!(
                cmp(json!("5"), ComparisonOp::LessThanOrEqual, json!(5)).unwrap(),
                json!(true)
            );
            assert_eq!(
                cmp(json!(2), ComparisonOp::LessThan, json!("3")).unwrap(),
                json!(true)
            );
            assert_eq!(
                cmp(json!("10"), ComparisonOp::GreaterThanOrEqual, json!("5")).unwrap(),
                json!(true)
            );
        }
    }

    mod arithmetic {
        use super::*;

        fn binary(op: BinaryOp, left: Value, right: Value) -> Result<Value, String> {
            let state = lookup(json!({}));
            let expr = Expr::Binary {
                op,
                left: Box::new(literal(left)),
                right: Box::new(literal(right)),
            };
            evaluate(&expr, &state).map_err(|error| error.to_string())
        }

        fn literal(value: Value) -> Expr {
            match value {
                Value::String(s) => Expr::StringLiteral(s),
                Value::Number(n) => Expr::NumberLiteral(n.as_f64().unwrap()),
                Value::Bool(b) => Expr::BoolLiteral(b),
                Value::Null => Expr::Variable("__missing__".to_string()),
                _ => panic!("only scalar literals supported in this helper"),
            }
        }

        #[test]
        fn addition_subtraction_multiplication_division() {
            assert_eq!(binary(BinaryOp::Add, json!(2), json!(3)).unwrap(), json!(5));
            assert_eq!(binary(BinaryOp::Sub, json!(7), json!(2)).unwrap(), json!(5));
            assert_eq!(
                binary(BinaryOp::Mul, json!(4), json!(3)).unwrap(),
                json!(12)
            );
            assert_eq!(
                binary(BinaryOp::Div, json!(10), json!(2)).unwrap(),
                json!(5)
            );
        }

        #[test]
        fn division_yields_fraction_when_not_integral() {
            let result = binary(BinaryOp::Div, json!(7), json!(2)).unwrap();
            assert_eq!(result.as_f64().unwrap(), 3.5);
        }

        #[test]
        fn modulus_basic_positive_operands() {
            assert_eq!(binary(BinaryOp::Mod, json!(7), json!(3)).unwrap(), json!(1));
            assert_eq!(
                binary(BinaryOp::Mod, json!(10), json!(5)).unwrap(),
                json!(0)
            );
        }

        #[test]
        fn c_style_remainder_negative_dividend() {
            // Sign follows the left operand (dividend).
            assert_eq!(
                binary(BinaryOp::Mod, json!(-5), json!(3)).unwrap(),
                json!(-2)
            );
            assert_eq!(
                binary(BinaryOp::Mod, json!(-7), json!(3)).unwrap(),
                json!(-1)
            );
            assert_eq!(
                binary(BinaryOp::Mod, json!(5), json!(-3)).unwrap(),
                json!(2)
            );
        }

        #[test]
        fn string_concatenation_when_either_operand_is_string() {
            assert_eq!(
                binary(BinaryOp::Add, json!("foo"), json!("bar")).unwrap(),
                json!("foobar")
            );
            assert_eq!(
                binary(BinaryOp::Add, json!("count: "), json!(5)).unwrap(),
                json!("count: 5")
            );
            assert_eq!(
                binary(BinaryOp::Add, json!(5), json!(" items")).unwrap(),
                json!("5 items")
            );
        }

        #[test]
        fn mixed_numeric_string_and_number_addition_is_arithmetic() {
            assert_eq!(
                binary(BinaryOp::Add, json!("2"), json!(1)).unwrap(),
                json!(3)
            );
            assert_eq!(
                binary(BinaryOp::Add, json!(1), json!("2")).unwrap(),
                json!(3)
            );
            assert_eq!(
                binary(BinaryOp::Add, json!("2.5"), json!(1)).unwrap(),
                json!(3.5)
            );
        }

        #[test]
        fn two_numeric_strings_still_concatenate() {
            assert_eq!(
                binary(BinaryOp::Add, json!("2"), json!("1")).unwrap(),
                json!("21")
            );
        }

        #[test]
        fn division_by_zero_returns_error() {
            let err = binary(BinaryOp::Div, json!(10), json!(0)).unwrap_err();
            assert!(err.contains("Division by zero"), "got: {err}");
        }

        #[test]
        fn remainder_by_zero_returns_error() {
            let err = binary(BinaryOp::Mod, json!(10), json!(0)).unwrap_err();
            assert!(err.contains("Remainder by zero"), "got: {err}");
        }

        #[test]
        fn arithmetic_errors_for_non_numeric_operands() {
            let state = lookup(json!({"arr": [1, 2, 3]}));
            // Subtraction on array is invalid
            let expr = Expr::Binary {
                op: BinaryOp::Sub,
                left: Box::new(Expr::Variable("arr".to_string())),
                right: Box::new(Expr::NumberLiteral(1.0)),
            };
            let err = evaluate(&expr, &state).unwrap_err();
            assert!(err.contains("Subtraction"), "got: {err}");

            // Multiplication on object is invalid
            let state = lookup(json!({"obj": {"a": 1}}));
            let expr = Expr::Binary {
                op: BinaryOp::Mul,
                left: Box::new(Expr::Variable("obj".to_string())),
                right: Box::new(Expr::NumberLiteral(2.0)),
            };
            let err = evaluate(&expr, &state).unwrap_err();
            assert!(err.contains("Multiplication"), "got: {err}");

            // Addition with two non-string non-numeric operands is invalid
            let state = lookup(json!({"arr": [1, 2]}));
            let expr = Expr::Binary {
                op: BinaryOp::Add,
                left: Box::new(Expr::Variable("arr".to_string())),
                right: Box::new(Expr::Variable("arr".to_string())),
            };
            let err = evaluate(&expr, &state).unwrap_err();
            assert!(err.contains("Addition"), "got: {err}");
        }

        #[test]
        fn arithmetic_with_boolean_operands_returns_error() {
            let state = lookup(json!({"flag": true}));

            // true + 1
            let expr = Expr::Binary {
                op: BinaryOp::Add,
                left: Box::new(Expr::Variable("flag".to_string())),
                right: Box::new(Expr::NumberLiteral(1.0)),
            };
            let err = evaluate(&expr, &state).unwrap_err();
            assert!(err.contains("Addition"), "got: {err}");

            // false - 1
            let expr = Expr::Binary {
                op: BinaryOp::Sub,
                left: Box::new(Expr::BoolLiteral(false)),
                right: Box::new(Expr::NumberLiteral(1.0)),
            };
            let err = evaluate(&expr, &state).unwrap_err();
            assert!(err.contains("Subtraction"), "got: {err}");

            // true * 2
            let expr = Expr::Binary {
                op: BinaryOp::Mul,
                left: Box::new(Expr::BoolLiteral(true)),
                right: Box::new(Expr::NumberLiteral(2.0)),
            };
            let err = evaluate(&expr, &state).unwrap_err();
            assert!(err.contains("Multiplication"), "got: {err}");

            // true / 2
            let expr = Expr::Binary {
                op: BinaryOp::Div,
                left: Box::new(Expr::BoolLiteral(true)),
                right: Box::new(Expr::NumberLiteral(2.0)),
            };
            let err = evaluate(&expr, &state).unwrap_err();
            assert!(err.contains("Division"), "got: {err}");

            // true % 2
            let expr = Expr::Binary {
                op: BinaryOp::Mod,
                left: Box::new(Expr::BoolLiteral(true)),
                right: Box::new(Expr::NumberLiteral(2.0)),
            };
            let err = evaluate(&expr, &state).unwrap_err();
            assert!(err.contains("Remainder"), "got: {err}");
        }

        #[test]
        fn unary_minus_with_boolean_returns_error() {
            let expr = Expr::UnaryMinus(Box::new(Expr::BoolLiteral(true)));
            let err = evaluate(&expr, &lookup(json!({}))).unwrap_err();
            assert!(err.contains("Unary '-'"), "got: {err}");
        }
    }

    mod access_semantics {
        use super::*;

        #[test]
        fn missing_member_path_evaluates_to_null() {
            let state = lookup(json!({"user": {"name": "Alice"}}));
            let expr = Expr::MemberAccess {
                base: Box::new(Expr::Variable("user".to_string())),
                name: "missing".to_string(),
            };
            assert_eq!(evaluate(&expr, &state).unwrap(), Value::Null);
        }

        #[test]
        fn member_access_on_null_base_returns_null() {
            let state = lookup(json!({}));
            let expr = Expr::MemberAccess {
                base: Box::new(Expr::Variable("missing".to_string())),
                name: "foo".to_string(),
            };
            assert_eq!(evaluate(&expr, &state).unwrap(), Value::Null);
        }

        #[test]
        fn member_access_on_non_object_returns_null() {
            let state = lookup(json!({"name": "Alice"}));
            let expr = Expr::MemberAccess {
                base: Box::new(Expr::Variable("name".to_string())),
                name: "foo".to_string(),
            };
            assert_eq!(evaluate(&expr, &state).unwrap(), Value::Null);
        }

        #[test]
        fn bracket_index_on_null_base_returns_null() {
            let state = lookup(json!({}));
            let expr = Expr::Index {
                base: Box::new(Expr::Variable("missing".to_string())),
                index: Box::new(Expr::NumberLiteral(0.0)),
            };
            assert_eq!(evaluate(&expr, &state).unwrap(), Value::Null);
        }

        #[test]
        fn out_of_bounds_index_returns_null() {
            let state = lookup(json!({"items": [1, 2, 3]}));
            let expr = Expr::Index {
                base: Box::new(Expr::Variable("items".to_string())),
                index: Box::new(Expr::NumberLiteral(10.0)),
            };
            assert_eq!(evaluate(&expr, &state).unwrap(), Value::Null);
        }

        #[test]
        fn negative_index_resolves_from_end() {
            let state = lookup(json!({"items": ["a", "b", "c"]}));
            let expr = Expr::Index {
                base: Box::new(Expr::Variable("items".to_string())),
                index: Box::new(Expr::UnaryMinus(Box::new(Expr::NumberLiteral(1.0)))),
            };
            assert_eq!(evaluate(&expr, &state).unwrap(), json!("c"));

            let expr = Expr::Index {
                base: Box::new(Expr::Variable("items".to_string())),
                index: Box::new(Expr::UnaryMinus(Box::new(Expr::NumberLiteral(2.0)))),
            };
            assert_eq!(evaluate(&expr, &state).unwrap(), json!("b"));
        }

        #[test]
        fn negative_index_out_of_range_returns_null() {
            let state = lookup(json!({"items": [1, 2]}));
            let expr = Expr::Index {
                base: Box::new(Expr::Variable("items".to_string())),
                index: Box::new(Expr::UnaryMinus(Box::new(Expr::NumberLiteral(5.0)))),
            };
            assert_eq!(evaluate(&expr, &state).unwrap(), Value::Null);
        }

        #[test]
        fn negative_index_on_empty_array_returns_null() {
            let state = lookup(json!({"items": []}));
            let expr = Expr::Index {
                base: Box::new(Expr::Variable("items".to_string())),
                index: Box::new(Expr::UnaryMinus(Box::new(Expr::NumberLiteral(1.0)))),
            };
            assert_eq!(evaluate(&expr, &state).unwrap(), Value::Null);
        }

        #[test]
        fn invalid_non_integer_index_returns_null() {
            let state = lookup(json!({"items": [1, 2, 3]}));
            let expr = Expr::Index {
                base: Box::new(Expr::Variable("items".to_string())),
                index: Box::new(Expr::NumberLiteral(1.5)),
            };
            assert_eq!(evaluate(&expr, &state).unwrap(), Value::Null);
        }

        #[test]
        fn string_key_access_on_object_returns_value() {
            let state = lookup(json!({"config": {"key": "value"}}));
            let expr = Expr::Index {
                base: Box::new(Expr::Variable("config".to_string())),
                index: Box::new(Expr::StringLiteral("key".to_string())),
            };
            assert_eq!(evaluate(&expr, &state).unwrap(), json!("value"));
        }

        #[test]
        fn string_key_access_on_non_collection_returns_null() {
            let state = lookup(json!({"config": "not-a-collection"}));
            let expr = Expr::Index {
                base: Box::new(Expr::Variable("config".to_string())),
                index: Box::new(Expr::StringLiteral("key".to_string())),
            };
            assert_eq!(evaluate(&expr, &state).unwrap(), Value::Null);
        }

        #[test]
        fn missing_object_key_returns_null() {
            let state = lookup(json!({"config": {"a": 1}}));
            let expr = Expr::Index {
                base: Box::new(Expr::Variable("config".to_string())),
                index: Box::new(Expr::StringLiteral("missing".to_string())),
            };
            assert_eq!(evaluate(&expr, &state).unwrap(), Value::Null);
        }

        #[test]
        fn chained_index_then_member_access() {
            let state = lookup(json!({"items": [{"name": "first"}, {"name": "second"}]}));
            // items[-1].name -> "second"
            let expr = Expr::MemberAccess {
                base: Box::new(Expr::Index {
                    base: Box::new(Expr::Variable("items".to_string())),
                    index: Box::new(Expr::UnaryMinus(Box::new(Expr::NumberLiteral(1.0)))),
                }),
                name: "name".to_string(),
            };
            assert_eq!(evaluate(&expr, &state).unwrap(), json!("second"));
        }

        #[test]
        fn bracket_index_with_non_numeric_returns_null() {
            let state = lookup(json!({"items": ["a", "b", "c"], "obj": {"x": 1}, "arr": [1]}));

            // Boolean index
            let expr = Expr::Index {
                base: Box::new(Expr::Variable("items".to_string())),
                index: Box::new(Expr::BoolLiteral(true)),
            };
            assert_eq!(evaluate(&expr, &state).unwrap(), Value::Null);

            let expr = Expr::Index {
                base: Box::new(Expr::Variable("items".to_string())),
                index: Box::new(Expr::BoolLiteral(false)),
            };
            assert_eq!(evaluate(&expr, &state).unwrap(), Value::Null);

            // String index
            let expr = Expr::Index {
                base: Box::new(Expr::Variable("items".to_string())),
                index: Box::new(Expr::StringLiteral("0".to_string())),
            };
            assert_eq!(evaluate(&expr, &state).unwrap(), Value::Null);

            // Null index
            let expr = Expr::Index {
                base: Box::new(Expr::Variable("items".to_string())),
                index: Box::new(Expr::Variable("missing".to_string())),
            };
            assert_eq!(evaluate(&expr, &state).unwrap(), Value::Null);

            // Object index
            let expr = Expr::Index {
                base: Box::new(Expr::Variable("items".to_string())),
                index: Box::new(Expr::Variable("obj".to_string())),
            };
            assert_eq!(evaluate(&expr, &state).unwrap(), Value::Null);

            // Array index
            let expr = Expr::Index {
                base: Box::new(Expr::Variable("items".to_string())),
                index: Box::new(Expr::Variable("arr".to_string())),
            };
            assert_eq!(evaluate(&expr, &state).unwrap(), Value::Null);
        }

        #[test]
        fn object_bracket_with_non_string_index_returns_null() {
            let state = lookup(json!({"obj": {"key": "value"}}));

            // Numeric index
            let expr = Expr::Index {
                base: Box::new(Expr::Variable("obj".to_string())),
                index: Box::new(Expr::NumberLiteral(0.0)),
            };
            assert_eq!(evaluate(&expr, &state).unwrap(), Value::Null);

            // Float index
            let expr = Expr::Index {
                base: Box::new(Expr::Variable("obj".to_string())),
                index: Box::new(Expr::NumberLiteral(1.5)),
            };
            assert_eq!(evaluate(&expr, &state).unwrap(), Value::Null);

            // Boolean true index
            let expr = Expr::Index {
                base: Box::new(Expr::Variable("obj".to_string())),
                index: Box::new(Expr::BoolLiteral(true)),
            };
            assert_eq!(evaluate(&expr, &state).unwrap(), Value::Null);

            // Boolean false index
            let expr = Expr::Index {
                base: Box::new(Expr::Variable("obj".to_string())),
                index: Box::new(Expr::BoolLiteral(false)),
            };
            assert_eq!(evaluate(&expr, &state).unwrap(), Value::Null);

            // Null index
            let expr = Expr::Index {
                base: Box::new(Expr::Variable("obj".to_string())),
                index: Box::new(Expr::Variable("missing".to_string())),
            };
            assert_eq!(evaluate(&expr, &state).unwrap(), Value::Null);

            // Array index
            let expr = Expr::Index {
                base: Box::new(Expr::Variable("obj".to_string())),
                index: Box::new(Expr::Variable("arr".to_string())),
            };
            assert_eq!(evaluate(&expr, &state).unwrap(), Value::Null);

            // Object index
            let expr = Expr::Index {
                base: Box::new(Expr::Variable("obj".to_string())),
                index: Box::new(Expr::Variable("obj".to_string())),
            };
            assert_eq!(evaluate(&expr, &state).unwrap(), Value::Null);
        }

        #[test]
        fn object_bracket_with_string_key_preserved() {
            let state = lookup(json!({"config": {"theme": "dark", "nested": {"key": "value"}}}));

            // Existing string key
            let expr = Expr::Index {
                base: Box::new(Expr::Variable("config".to_string())),
                index: Box::new(Expr::StringLiteral("theme".to_string())),
            };
            assert_eq!(evaluate(&expr, &state).unwrap(), json!("dark"));

            // Missing string key
            let expr = Expr::Index {
                base: Box::new(Expr::Variable("config".to_string())),
                index: Box::new(Expr::StringLiteral("missing".to_string())),
            };
            assert_eq!(evaluate(&expr, &state).unwrap(), Value::Null);

            // Nested object access via chained bracket
            let expr = Expr::Index {
                base: Box::new(Expr::Index {
                    base: Box::new(Expr::Variable("config".to_string())),
                    index: Box::new(Expr::StringLiteral("nested".to_string())),
                }),
                index: Box::new(Expr::StringLiteral("key".to_string())),
            };
            assert_eq!(evaluate(&expr, &state).unwrap(), json!("value"));
        }
    }

    mod truthiness {
        use super::*;

        #[test]
        fn falsy_null() {
            assert!(!is_truthy(&Value::Null));
        }

        #[test]
        fn falsy_false() {
            assert!(!is_truthy(&json!(false)));
        }

        #[test]
        fn falsy_zero_integer() {
            assert!(!is_truthy(&json!(0)));
        }

        #[test]
        fn falsy_zero_float() {
            assert!(!is_truthy(&json!(0.0)));
        }

        #[test]
        fn falsy_empty_string() {
            assert!(!is_truthy(&json!("")));
        }

        #[test]
        fn falsy_empty_array() {
            assert!(!is_truthy(&json!([])));
        }

        #[test]
        fn falsy_empty_object() {
            assert!(!is_truthy(&json!({})));
        }

        #[test]
        fn truthy_true() {
            assert!(is_truthy(&json!(true)));
        }

        #[test]
        fn truthy_non_empty_string() {
            assert!(is_truthy(&json!("hello")));
        }

        #[test]
        fn truthy_non_zero_positive_integer() {
            assert!(is_truthy(&json!(1)));
        }

        #[test]
        fn truthy_non_zero_negative_integer() {
            assert!(is_truthy(&json!(-1)));
        }

        #[test]
        fn truthy_non_zero_float() {
            assert!(is_truthy(&json!(0.1)));
        }

        #[test]
        fn truthy_non_empty_array() {
            assert!(is_truthy(&json!([0])));
        }

        #[test]
        fn truthy_non_empty_object() {
            assert!(is_truthy(&json!({"a": 1})));
        }
    }

    mod date_helpers {
        use super::*;

        fn eval_expr(expr_str: &str) -> Result<Value, String> {
            let expr = parse(expr_str).map_err(|e| e.message)?;
            evaluate(&expr, &lookup(json!({}))).map_err(|error| error.to_string())
        }

        fn eval_expr_with_data(expr_str: &str, data: Value) -> Result<Value, String> {
            let expr = parse(expr_str).map_err(|e| e.message)?;
            evaluate(&expr, &lookup(data)).map_err(|error| error.to_string())
        }

        // ── Strict date validators ─────────────────────────────────────

        #[test]
        fn isdate_accepts_valid_iso_date() {
            assert_eq!(eval_expr(r#"is_date("2024-06-15")"#).unwrap(), json!(true));
        }

        #[test]
        fn isdate_rejects_invalid_and_non_strings() {
            assert_eq!(eval_expr(r#"is_date("not-a-date")"#).unwrap(), json!(false));
            assert_eq!(eval_expr(r#"is_date("2024/06/15")"#).unwrap(), json!(false));
            assert_eq!(eval_expr("is_date(123)").unwrap(), json!(false));
            assert_eq!(eval_expr("is_date(null)").unwrap(), json!(false));
            assert_eq!(eval_expr("is_date(true)").unwrap(), json!(false));
        }

        #[test]
        fn isdateutc_same_contract_as_isdate() {
            assert_eq!(
                eval_expr(r#"is_date_utc("2024-06-15")"#).unwrap(),
                json!(true)
            );
            assert_eq!(eval_expr(r#"is_date_utc("bad")"#).unwrap(), json!(false));
            assert_eq!(eval_expr("is_date_utc(123)").unwrap(), json!(false));
        }

        #[test]
        fn isdatetime_accepts_iso_datetimes() {
            assert_eq!(
                eval_expr(r#"is_datetime("2024-06-15T12:30:00")"#).unwrap(),
                json!(true)
            );
            assert_eq!(
                eval_expr(r#"is_datetime("2024-06-15T12:30:00Z")"#).unwrap(),
                json!(true)
            );
            assert_eq!(
                eval_expr(r#"is_datetime("2024-06-15T12:30:00+02:00")"#).unwrap(),
                json!(true)
            );
        }

        #[test]
        fn isdatetime_rejects_plain_dates_and_non_strings() {
            assert_eq!(
                eval_expr(r#"is_datetime("2024-06-15")"#).unwrap(),
                json!(false)
            );
            assert_eq!(eval_expr("is_datetime(123)").unwrap(), json!(false));
            assert_eq!(eval_expr("is_datetime(null)").unwrap(), json!(false));
        }

        #[test]
        fn isdatetimeutc_same_contract_as_isdatetime() {
            assert_eq!(
                eval_expr(r#"is_datetime_utc("2024-06-15T12:30:00Z")"#).unwrap(),
                json!(true)
            );
            assert_eq!(
                eval_expr(r#"is_datetime_utc("2024-06-15")"#).unwrap(),
                json!(false)
            );
        }

        // ── Relative date validators (deterministic false cases) ───────

        #[test]
        fn istoday_returns_false_for_distant_dates() {
            // Using a date far in the past so it is never "today"
            assert_eq!(
                eval_expr(r#"is_today("1900-01-01")"#).unwrap(),
                json!(false)
            );
            assert_eq!(
                eval_expr(r#"is_today("2100-12-31")"#).unwrap(),
                json!(false)
            );
        }

        #[test]
        fn istoday_rejects_non_strings_and_null() {
            assert_eq!(eval_expr("is_today(123)").unwrap(), json!(false));
            assert_eq!(eval_expr("is_today(null)").unwrap(), json!(false));
            assert_eq!(eval_expr("is_today(true)").unwrap(), json!(false));
        }

        #[test]
        fn istodayutc_returns_false_for_distant_dates() {
            assert_eq!(
                eval_expr(r#"is_today_utc("1900-01-01")"#).unwrap(),
                json!(false)
            );
        }

        #[test]
        fn isyesterday_returns_false_for_distant_dates() {
            assert_eq!(
                eval_expr(r#"is_yesterday("1900-01-01")"#).unwrap(),
                json!(false)
            );
        }

        #[test]
        fn isyesterdayutc_returns_false_for_distant_dates() {
            assert_eq!(
                eval_expr(r#"is_yesterday_utc("1900-01-01")"#).unwrap(),
                json!(false)
            );
        }

        #[test]
        fn istomorrow_returns_false_for_distant_dates() {
            assert_eq!(
                eval_expr(r#"is_tomorrow("1900-01-01")"#).unwrap(),
                json!(false)
            );
        }

        #[test]
        fn istomorrowutc_returns_false_for_distant_dates() {
            assert_eq!(
                eval_expr(r#"is_tomorrow_utc("1900-01-01")"#).unwrap(),
                json!(false)
            );
        }

        #[test]
        fn isthismonth_returns_false_for_distant_dates() {
            assert_eq!(
                eval_expr(r#"is_this_month("1900-01-01")"#).unwrap(),
                json!(false)
            );
            assert_eq!(
                eval_expr(r#"is_this_month("2100-12-31")"#).unwrap(),
                json!(false)
            );
        }

        #[test]
        fn isthismonthutc_returns_false_for_distant_dates() {
            assert_eq!(
                eval_expr(r#"is_this_month_utc("1900-01-01")"#).unwrap(),
                json!(false)
            );
        }

        #[test]
        fn isthisyear_returns_false_for_distant_dates() {
            assert_eq!(
                eval_expr(r#"is_this_year("1900-01-01")"#).unwrap(),
                json!(false)
            );
            assert_eq!(
                eval_expr(r#"is_this_year("2100-12-31")"#).unwrap(),
                json!(false)
            );
        }

        #[test]
        fn isthisyearutc_returns_false_for_distant_dates() {
            assert_eq!(
                eval_expr(r#"is_this_year_utc("1900-01-01")"#).unwrap(),
                json!(false)
            );
        }

        #[test]
        fn relative_validators_accept_datetime_strings_and_return_bool() {
            // Verify datetime strings parse without error and return boolean
            let result = eval_expr(r#"is_today("1900-01-01T00:00:00")"#).unwrap();
            assert!(result.is_boolean());
            let result = eval_expr(r#"is_today_utc("1900-01-01T00:00:00Z")"#).unwrap();
            assert!(result.is_boolean());
        }

        #[test]
        fn all_date_helper_names_are_dispatchable() {
            // Smoke test: every required helper name parses and evaluates
            let helpers = [
                r#"is_date("2024-06-15")"#,
                r#"is_date_utc("2024-06-15")"#,
                r#"is_datetime("2024-06-15T12:00:00")"#,
                r#"is_datetime_utc("2024-06-15T12:00:00Z")"#,
                r#"is_today("1900-01-01")"#,
                r#"is_today_utc("1900-01-01")"#,
                r#"is_yesterday("1900-01-01")"#,
                r#"is_yesterday_utc("1900-01-01")"#,
                r#"is_tomorrow("1900-01-01")"#,
                r#"is_tomorrow_utc("1900-01-01")"#,
                r#"is_this_month("1900-01-01")"#,
                r#"is_this_month_utc("1900-01-01")"#,
                r#"is_this_year("1900-01-01")"#,
                r#"is_this_year_utc("1900-01-01")"#,
            ];
            for expr_str in &helpers {
                let result = eval_expr(expr_str);
                assert!(
                    result.is_ok(),
                    "{expr_str} should parse and evaluate, got: {result:?}"
                );
                assert!(
                    result.unwrap().is_boolean(),
                    "{expr_str} should return a boolean"
                );
            }
        }

        #[test]
        fn date_helpers_work_with_variables() {
            let data = json!({
                "date_str": "2024-06-15",
                "bad_str": "not-a-date",
                "distant": "1900-01-01"
            });
            assert_eq!(
                eval_expr_with_data("is_date(date_str)", data.clone()).unwrap(),
                json!(true)
            );
            assert_eq!(
                eval_expr_with_data("is_date(bad_str)", data.clone()).unwrap(),
                json!(false)
            );
            assert_eq!(
                eval_expr_with_data("is_today(distant)", data.clone()).unwrap(),
                json!(false)
            );
            assert_eq!(
                eval_expr_with_data("is_this_year(distant)", data.clone()).unwrap(),
                json!(false)
            );
        }
    }
}
