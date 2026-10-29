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

pub(crate) mod absence;
pub mod ast;
pub mod binding;
pub mod call_binding;
pub mod call_check;
pub mod catalog;
pub mod ctx;
mod deferred;
pub mod diagnostics;
pub(crate) mod equality;
pub(crate) mod fm_namespace;
pub mod error;
pub mod file_suggestions;
pub mod functions;
pub mod host;
pub mod iterable;
pub mod iteration;
pub mod lexer;
pub mod lint;
pub(crate) mod path_projection;
pub mod parser;
pub mod prepared;
pub mod resolve_ctx;
pub mod scope;
pub mod semantics;

pub use absence::{
    AccessStep, StaticAccessRead, StaticVariableRead, branch_scope, static_access_reads, static_callback_reads,
    static_variable_reads, static_variable_reads_in_scope,
};
pub use ast::{
    BinaryOp, BindingPattern, CallArgument, Callback, Expr, SpannedCallArgument, SpannedBindingPattern, SpannedCallback, SpannedExpr, SpannedExprKind,
};
pub use binding::{
    Availability, BindingError, BindingView, BindingViewBuilder, EvaluationSession,
    GlobalDeclaration, HostGlobals, LOCAL_ROOT, LazyProvider, LexicalScope, LocalBinding, LocalId, ProtectedLocalName,
    ProtectedNameKind, ResolvedBinding, RootClass, RuntimeBinding, ScopeId, ScopedLookup, UnavailabilityReason,
    UnavailableBinding, check_local_names, is_local_path, is_reserved_namespace,
};
pub use call_binding::{
    BoundArgs, BoundCall, CallBindingError, CallErrorKind, CompiledSignature, EvaluationMode, FunctionRegistration,
    LazyCall, RegistrationError,
};
pub use call_check::{
    ALWAYS_FALSE_CALL_CODE, AlwaysFalseFinding, ArgumentCategory, ArgumentCheck, ArgumentType, CallCheck, CallFinding,
    CallProblem, PreparedCallCheck, check_call,
    check_call_with_definitions, check_expression_calls, check_prepared_calls, classify_argument,
    classify_argument_with_definitions, expression_type,
};
pub use catalog::{
    CatalogCategory, ExpressionFunctionDescriptor, FunctionCatalog, FunctionExample, FunctionParameterDescriptor,
    ReservedRootDescriptor, RootEvaluation, RootMembers, expression_function_catalog, expression_function_descriptors,
    function_descriptors, generate_expression_function_table, reserved_root_descriptors,
};
#[cfg(any(test, feature = "effects-instrumentation"))]
pub use functions::test_hooks as function_test_hooks;
pub use ctx::CtxLookup;
pub use diagnostics::{
    Diagnostic, DiagnosticSource, ExpressionDiagnostic, INCOMPATIBLE_CALL_CODE, Severity, UNCERTAIN_CALL_CODE,
    call_severity, expression_call_diagnostics, one_based_position,
};
pub use host::{
    FactsArtifact, HostCatalogError, HostFunctionCatalog, HostFunctionError, HostFunctionScope, HostFunctions,
    HostFunctionsBuilder, HostHandler, INCOMPLETE_ANALYSIS_CODE, IncompleteHostCall, incomplete_host_calls,
    install_host_functions,
};
pub use deferred::{defer_templates, is_deferred_template, is_deferred_value, is_malformed_template};
pub use fm_namespace::{
    FRONTMATTER_ROOT, FmPath, fm_namespace_path, resolve_fm_namespace_in_object,
};
pub use error::{
    CallerFileDiagnosticProvenance, ExpressionError, FileRefFailure,
    FileReferenceDiagnostic, ProviderFailureKind,
};
pub use file_suggestions::{collect_sibling_candidates, suggest_sibling_files};
pub(crate) use path_projection::{
    make_portable_relative_in_context,
};
pub use resolve_ctx::ResolutionContext;
pub use scope::{
    BindingId, DeclaredType, DeclaredTypes, GuardOrigin, Member, Polarity, PresenceEvidence, RawDeclarations,
    ItemFrame, LexicalTypes, PatternMismatch, ScopedType, Shape, SortKeySupport, SourceProjection, Subject, TypeScope, TypedLocal,
    WriteSet, project_pattern, project_source,
};
pub use lint::{ExpressionLint, ExpressionLintKind, is_whole_value_span, lint_expression, lint_spanned};
pub use lexer::{
    ComparisonOp, ExpressionFinder, ExpressionLocation, ExpressionScanResult, InterpolationLiteral,
    Lexer, LiteralTokenLocation, LexerError, ParseMode, Token, identifier_prefix_start, lex_spanned,
};
pub use parser::{
    ParseError, Parser, parse, parse_callback, parse_callback_spanned, parse_condition, parse_condition_spanned,
    parse_spanned,
};
pub use prepared::{
    AuthoredMode, PreparationError, PreparedExpression, PreparedValue, ValidationDiagnostic,
    evaluate_prepared, prepare_value, validate_callback, validate_expression, validate_expression_in_scope,
    validate_prepared,
};

use absence::{AbsenceScope, MissingRootObserver};
use serde_json::Value;

use crate::markdown::numeric::{self, NumberTextError, NumericAdmissionError};

use crate::catalog::{describe_for_error, suggest};

/// A value read as an arithmetic operand.
enum ArithmeticOperand {
    Number(f64),
    /// Not a number and not numeric text: booleans, null, arrays, objects,
    /// and text such as `"pear"`.
    NotNumeric,
    /// A number, or numeric text, that admission rejects
    /// (`"9007199254740993"`).
    Inadmissible(NumericAdmissionError),
}

/// Reads a value as an arithmetic operand. Text converts through the one
/// canonical numeric-text parser, and a stored number is admitted again
/// because a value supplied from outside the loaders (`--set`) may be an
/// exact integer no `f64` holds.
fn arithmetic_operand(value: &Value) -> ArithmeticOperand {
    let admitted = match value {
        Value::Number(number) => numeric::admit_json_number(number).map_err(NumberTextError::Admission),
        Value::String(text) => numeric::admit_number_text(text),
        _ => Err(NumberTextError::NotANumber),
    };
    match admitted {
        Ok(number) => ArithmeticOperand::Number(number),
        Err(NumberTextError::NotANumber) => ArithmeticOperand::NotNumeric,
        Err(NumberTextError::Admission(reason)) => ArithmeticOperand::Inadmissible(reason),
    }
}

/// Converts an expression value into a number for arithmetic operations,
/// returning an error tagged with the originating operator when the value is
/// not a number, or carrying the admission reason when it is a number that
/// cannot be represented.
fn require_number(value: &Value, op_label: &'static str) -> Result<f64, ExpressionError> {
    match arithmetic_operand(value) {
        ArithmeticOperand::Number(number) => Ok(number),
        ArithmeticOperand::NotNumeric => Err(ExpressionError::Arithmetic { op: op_label }),
        ArithmeticOperand::Inadmissible(reason) => Err(ExpressionError::Other {
            function: op_label.to_string(),
            message: format!("The number {} {reason}", scalar_string(value)),
        }),
    }
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

/// The value a numeric literal evaluates to. The guard analysis models
/// literal operands with this, so it must stay the evaluator's only literal
/// conversion: the lexer already admitted the literal, and it is stored by
/// [`numeric::json_number`] (a whole value as an integer, never narrowed).
fn number_literal(n: f64) -> Option<Value> {
    numeric::json_number(n).map(Value::Number)
}

/// Stores a computed number; a non-finite result (overflow, `0 / 0`) is an
/// error, never infinity or a saturated value.
fn json_number(value: f64) -> Result<Value, String> {
    numeric::json_number(value)
        .map(Value::Number)
        .ok_or_else(|| format!("Arithmetic result is not finite: {value}"))
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

    /// Resolves a dotted path on the evaluator's error channel, classifying
    /// it as a document property, a reserved namespace, or a host global.
    ///
    /// Expression evaluation and interpolation read every variable through
    /// this method. The default wraps [`get`](Self::get) as a
    /// [`ResolvedBinding::Document`] and never fails; it declares no globals.
    /// Composition lookups override it so a known `ctx.*` variable with no
    /// captured value surfaces as [`ExpressionError::ContextNotCaptured`] or
    /// [`ExpressionError::ContextProjectionInvariant`], and an
    /// [`EvaluationSession`] fails a read of an unavailable global with
    /// [`ExpressionError::Binding`].
    ///
    /// ## Errors
    ///
    /// Only overriding implementations fail.
    fn resolve(&self, path: &str) -> Result<ResolvedBinding, ExpressionError> {
        Ok(ResolvedBinding::Document {
            value: self.get(path),
        })
    }

    /// The binding view this lookup was associated against; `None` for a
    /// lookup that declares no globals.
    fn binding_view(&self) -> Option<&BindingView> {
        None
    }

    /// Formats a value already obtained from [`resolve`](Self::resolve) for
    /// string interpolation, so the interpolator never looks `path` up twice.
    ///
    /// The default renders `null` as empty, strings verbatim, numbers and
    /// booleans with `to_string`, and arrays and objects as JSON. Lookups with
    /// configured name coercion override it.
    fn format_resolved(&self, _path: &str, value: &Value) -> String {
        default_format(value)
    }

    /// Looks up a value by path, coercing to a string.
    ///
    /// ## Returns
    ///
    /// - The string representation of the value when the path resolves
    /// - An empty string when the path does not resolve or the value is null
    fn get_string(&self, path: &str) -> String {
        self.get(path)
            .map(|value| self.format_resolved(path, &value))
            .unwrap_or_default()
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

    /// Opens a new expression-evaluation scope.
    ///
    /// The lazy reserved root `current` memoizes **per expression evaluation,
    /// per key** (spec Q2/D5): repeated reads of one key inside a single
    /// `{{ … }}` span, `when=` condition, or `$()` branch agree, while the next
    /// expression observes the fact afresh. Every surface that evaluates one
    /// parsed expression calls this first; the default is a no-op for lookups
    /// that hold no lazy root.
    fn begin_expression_scope(&self) {}

    /// The enclosing request or standalone session's shared collection allowance.
    fn iteration_budget(&self) -> Option<&iteration::IterationBudget> {
        None
    }

    /// The innermost lexical scope this lookup reads locals from, if any.
    ///
    /// Delegating lookups forward their inner lookup's retained frame.
    fn lexical_scope(&self) -> Option<&std::sync::Arc<LexicalScope>> {
        None
    }
}

/// The default [`EvaluationLookup::format_resolved`] rendering.
pub(crate) fn default_format(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::String(s) => s.clone(),
        Value::Number(n) => format_json_number(n),
        Value::Bool(b) => b.to_string(),
        other => other.to_string(),
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
/// - `String` -> `Some(n)` when the canonical numeric-text parser
///   ([`numeric::admit_number_text`]) admits it; `None` for other text,
///   including `"inf"`, `"NaN"`, and inexact text such as `"9007199254740993"`
/// - `Bool` -> `Some(1.0)` for true, `Some(0.0)` for false
/// - `Null` -> `None`
/// - `Array` | `Object` -> `None`
pub fn to_number(value: &Value) -> Option<f64> {
    match value {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => numeric::admit_number_text(s).ok(),
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
/// - `Number` -> [`numeric::format_number`] (`4.0` is `"4"`); an integer
///   stored exactly keeps all its digits
/// - `String` -> the string itself
/// - `Array` | `Object` -> JSON string representation
pub fn scalar_string(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => format_json_number(n),
        Value::String(s) => s.clone(),
        Value::Array(_) | Value::Object(_) => value.to_string(),
    }
}

/// The text form of a stored number: [`numeric::format_number`] for float
/// storage (`4.0` is `"4"`, `1e20` has no exponent), and every digit of an
/// integer stored exactly.
pub(crate) fn format_json_number(number: &serde_json::Number) -> String {
    match number.as_f64() {
        Some(value) if number.is_f64() => numeric::format_number(value),
        _ => number.to_string(),
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
/// - `Err(ExpressionError)` retaining the typed failure cause
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
    // One call is one expression evaluation, which is the memo scope the lazy
    // `current` root refreshes at (Q2). The recursion below is
    // inside that scope, so it never reopens it.
    lookup.begin_expression_scope();
    let budget = evaluation_budget(lookup);
    evaluate_expr(expr, lookup, AbsenceScope::default(), &mut (), &budget)
}

/// [`evaluate`], reporting to `observer` every evaluated read of an unknown
/// root that no absence construct handles (spec Requirement 4).
///
/// Observation follows evaluation: an unchosen ternary branch or a
/// short-circuited operand is never read, so it is never reported. The value
/// and error are exactly those of [`evaluate`].
pub(crate) fn evaluate_observed<L: EvaluationLookup, O: MissingRootObserver>(
    expr: &Expr,
    lookup: &L,
    observer: &mut O,
) -> Result<Value, ExpressionError> {
    lookup.begin_expression_scope();
    let budget = evaluation_budget(lookup);
    evaluate_expr(expr, lookup, AbsenceScope::default(), observer, &budget)
}

fn evaluation_budget<L: EvaluationLookup>(lookup: &L) -> iteration::IterationBudget {
    lookup.iteration_budget().cloned()
        .or_else(|| lookup.resolution_context_ref().map(|context| context.iteration.clone()))
        .or_else(|| lookup.resolution_context().map(|context| context.iteration))
        .unwrap_or_else(|| iteration::IterationBudget::new(Default::default()))
}

/// Reports a document-property read of `path` that found no value, unless
/// `scope` handles its absence or the root is reserved. Only a
/// [`ResolvedBinding::Document`] read is a candidate: a namespace or a global
/// is never an undeclared property. Whether the property is declared anywhere
/// is decided once per document, against the final state and schema
/// (`unknown_identifiers::reconcile`).
pub(crate) fn observe_missing<O: MissingRootObserver>(
    path: &str,
    binding: &ResolvedBinding,
    scope: AbsenceScope<'_>,
    observer: &mut O,
) {
    if !O::OBSERVES
        || !matches!(binding, ResolvedBinding::Document { value: None })
        || scope.handles(path)
    {
        return;
    }
    let root = absence::root_of(path);
    if !absence::is_reserved_root(root) {
        observer.missing_root(root);
    }
}

/// Recursive evaluation inside one already-open expression scope. `scope`
/// says whether this position handles absence; see [`absence`].
fn evaluate_expr<L: EvaluationLookup, O: MissingRootObserver>(
    expr: &Expr,
    lookup: &L,
    scope: AbsenceScope<'_>,
    observer: &mut O,
    budget: &iteration::IterationBudget,
) -> Result<Value, ExpressionError> {
    let operand = scope.operand();
    match expr {
        Expr::Variable(path) => {
            let binding = lookup.resolve(path)?;
            observe_missing(path, &binding, scope, observer);
            Ok(binding.into_value().unwrap_or(Value::Null))
        }
        Expr::StringLiteral(s) => Ok(Value::String(s.clone())),
        Expr::NumberLiteral(n) => number_literal(*n)
            .ok_or_else(|| ExpressionError::Parse(format!("Invalid numeric literal: {n}"))),
        Expr::BoolLiteral(b) => Ok(Value::Bool(*b)),
        Expr::ArrayLiteral(items) => items
            .iter()
            .map(|item| evaluate_expr(item, lookup, operand, observer, budget))
            .collect::<Result<Vec<_>, _>>()
            .map(Value::Array),
        Expr::ObjectLiteral(entries) => entries
            .iter()
            .map(|(key, value)| Ok((key.clone(), evaluate_expr(value, lookup, operand, observer, budget)?)))
            .collect::<Result<serde_json::Map<_, _>, ExpressionError>>()
            .map(Value::Object),
        Expr::Paren(inner) => evaluate_expr(inner, lookup, scope, observer, budget),
        Expr::UnaryNot(inner) => {
            let value = evaluate_expr(inner, lookup, operand, observer, budget)?;
            Ok(Value::Bool(!is_truthy(&value)))
        }
        Expr::UnaryMinus(inner) => {
            let value = evaluate_expr(inner, lookup, operand, observer, budget)?;
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
            let left = evaluate_expr(left, lookup, operand, observer, budget)?;
            let right = evaluate_expr(right, lookup, operand, observer, budget)?;
            evaluate_binary(*op, &left, &right)
        }
        Expr::Index { base, index } => {
            // Access on a handled value is still that value's absence check.
            let base = evaluate_expr(base, lookup, scope, observer, budget)?;
            let index = evaluate_expr(index, lookup, operand, observer, budget)?;
            Ok(evaluate_index(&base, &index))
        }
        Expr::MemberAccess { base, name } => {
            let base = evaluate_expr(base, lookup, scope, observer, budget)?;
            Ok(evaluate_member(&base, name))
        }
        Expr::Fallback { primary: primary_expr, fallback } => {
            let primary = evaluate_expr(primary_expr, lookup, scope.absence_check(), observer, budget)?;
            if is_truthy(&primary) {
                Ok(primary)
            } else if O::OBSERVES {
                // Keeps `scope`'s handling: in a chain `a || b || "d"` that is
                // itself a primary, `b` is handled too. The fallback is read
                // only when the primary failed, so it gets the false side.
                let facts = scope.facts().assume(primary_expr, scope::Polarity::False);
                evaluate_expr(fallback, lookup, scope.with_facts(&facts), observer, budget)
            } else {
                evaluate_expr(fallback, lookup, scope, observer, budget)
            }
        }
        Expr::Ternary {
            condition: condition_expr,
            then_branch,
            else_branch,
        } => {
            let condition = evaluate_expr(condition_expr, lookup, scope.absence_check(), observer, budget)?;
            let (branch, polarity) = if is_truthy(&condition) {
                (then_branch, scope::Polarity::True)
            } else {
                (else_branch, scope::Polarity::False)
            };
            if O::OBSERVES {
                // The chosen branch reads under its own side's facts only.
                let facts = scope.facts().assume(condition_expr, polarity);
                evaluate_expr(branch, lookup, scope.with_facts(&facts), observer, budget)
            } else {
                evaluate_expr(branch, lookup, scope, observer, budget)
            }
        }
        Expr::Comparison { left, op, right } => {
            let left = evaluate_expr(left, lookup, operand, observer, budget)?;
            let right = evaluate_expr(right, lookup, operand, observer, budget)?;

            let outcome = match op {
                ComparisonOp::Equal => equality::values_equal(&left, &right),
                ComparisonOp::NotEqual => !equality::values_equal(&left, &right),
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
        Expr::CallbackCall { name, args } => {
            evaluate_callback_function(name, args, lookup, scope, observer, budget)
        },
        Expr::FunctionCall { name, args } => {
            evaluate_function(name, args, lookup, scope, observer, budget)
        }
    }
}

fn evaluate_binary(op: BinaryOp, left: &Value, right: &Value) -> Result<Value, ExpressionError> {
    // Numeric text that admission rejects still counts as numeric here, so
    // `"9007199254740993" + 1` reports the reason instead of concatenating.
    let mixed_number_and_numeric_string = op == BinaryOp::Add
        && matches!(
            (left, right),
            (Value::Number(_), Value::String(_)) | (Value::String(_), Value::Number(_))
        )
        && !matches!(arithmetic_operand(left), ArithmeticOperand::NotNumeric)
        && !matches!(arithmetic_operand(right), ArithmeticOperand::NotNumeric);
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
            to_number_index(index)
                .and_then(|n| resolve_array_index(items.len(), n))
                .map_or(Value::Null, |resolved| items[resolved].clone())
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

/// Returns a plain-text error for an unrecognized function name, with a fuzzy
/// did-you-mean suggestion when one exists.
pub(crate) fn unknown_function_error(name: &str) -> ExpressionError {
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

fn evaluate_callback_function<L: EvaluationLookup, O: MissingRootObserver>(
    name: &str,
    args: &[CallArgument],
    lookup: &L,
    scope: AbsenceScope<'_>,
    observer: &mut O,
    budget: &iteration::IterationBudget,
) -> Result<Value, ExpressionError> {
    let name = name.to_ascii_lowercase();
    let registration = functions::function_registration(&name).ok_or_else(|| unknown_function_error(&name))?;
    registration.check_arity(args.len()).map_err(|error| ExpressionError::Call(Box::new(error)))?;
    let invalid = |message: String| ExpressionError::InvalidCallback { span: 0..0, message };
    if registration.evaluation() != EvaluationMode::Callback {
        return Err(invalid(format!("{name} does not accept inline callbacks")));
    }
    let [CallArgument::Value(input), CallArgument::Callback(callback)] = args else {
        return Err(invalid(format!("{name} requires an iterable and one inline callback")));
    };
    check_local_names(callback.pattern.names(), lookup.binding_view())
        .map_err(|error| invalid(error.to_string()))?;
    let value = evaluate_expr(input, lookup, scope.call_argument(&name, 0, args.len()), observer, budget)?;
    let value = registration.bind_callback_value(0, value)
        .map_err(|error| ExpressionError::Call(Box::new(error)))?;
    let source = iterable::IterableSource::capture(value)
        .expect("the registered iterable parameter admits only collections or null");
    let locals = callback.pattern.names().map(|name| {
        scope::TypedLocal::new(name, scope::Shape::Unknown { non_null: false })
    }).collect();
    let facts = scope.facts();
    let lexical = match facts.lexical() {
        Some(parent) => parent.child(locals),
        None => scope::LexicalTypes::new(locals),
    };
    let facts = facts.within(lexical);
    let body_scope = scope.operand().with_facts(&facts);
    let mut result = Vec::new();
    for entry in source.entries() {
        let path = iteration::IterationPath::empty().pushed(iteration::IterationFrame {
            collection: name.clone(), position: entry.position(),
        });
        budget.reserve_visit(&path).map_err(|error| ExpressionError::IterationLimit(Box::new(error)))?;
        let wrap = |source| ExpressionError::Callback {
            function: name.clone(), position: entry.position(), source: Box::new(source),
        };
        let bindings = match &callback.pattern {
            BindingPattern::Identifier(name) => vec![(name.clone(), entry.retained())],
            BindingPattern::Fields(fields) => fields.iter().map(|field| {
                entry.field(field).map(|value| (field.clone(), value.into_owned()))
                    .ok_or_else(|| invalid(format!("item has no field `{field}` required by the callback pattern")))
            }).collect::<Result<Vec<_>, _>>().map_err(&wrap)?,
        };
        let frame = match lookup.lexical_scope() {
            Some(parent) => parent.child(bindings),
            None => LexicalScope::new(bindings),
        };
        let local = ScopedLookup::new(lookup, frame);
        let value = evaluate_expr(&callback.body, &local, body_scope, observer, budget).map_err(wrap)?;
        if name == "map" {
            result.push(value);
        } else if is_truthy(&value) {
            result.push(entry.retained());
        }
    }
    Ok(Value::Array(result))
}

fn evaluate_function<L: EvaluationLookup, O: MissingRootObserver>(
    name: &str,
    args: &[Expr],
    lookup: &L,
    scope: AbsenceScope<'_>,
    observer: &mut O,
    budget: &iteration::IterationBudget,
) -> Result<Value, ExpressionError> {
    // Only an observer reads absence handling, so a plain evaluation skips
    // the registration lookup per argument.
    let argument = |index: usize| {
        if O::OBSERVES { scope.call_argument(name, index, args.len()) } else { scope.operand() }
    };
    let name = name.to_ascii_lowercase();
    if let Some((registration, short_circuit)) = functions::lazy_function(&name) {
        return evaluate_lazy_function(registration, short_circuit, args, lookup, argument, observer, budget);
    }
    let mut caller_occurrence = None;
    let evaluated: Vec<Value> = if functions::is_context_function(&name) {
        let mut values = Vec::with_capacity(args.len());
        for (index, arg) in args.iter().enumerate() {
            let argument = argument(index);
            if index == 0 {
                let (value, occurrence) = evaluate_caller_file_argument(arg, lookup, argument, observer, budget)?;
                values.push(value);
                caller_occurrence = occurrence;
            } else {
                values.push(evaluate_expr(arg, lookup, argument, observer, budget)?);
            }
        }
        values
    } else {
        args.iter()
            .enumerate()
            .map(|(index, arg)| evaluate_expr(arg, lookup, argument(index), observer, budget))
            .collect::<Result<_, _>>()?
    };
    // Prefer the borrowed context (Finding 12) so a read-side function
    // dispatched here does not deep-clone the lookup's context.
    let owned_ctx = owned_call_context(lookup, caller_occurrence);
    let ctx = owned_ctx.as_deref().or_else(|| lookup.resolution_context_ref());
    match functions::call_function(&name, &evaluated, ctx) {
        Some(result) => result,
        None => Err(unknown_function_error(&name)),
    }
}

/// The context a call needs owned: the lookup's own clone when it exposes no
/// borrowed context, or one selecting the caller file `caller_occurrence`
/// names. `None` means the borrowed context serves the call as it is.
///
/// Out of line and boxed: a context held by value in
/// [`evaluate_function`] would sit in a frame that recurs once per nested
/// composition.
#[inline(never)]
fn owned_call_context<L: EvaluationLookup>(
    lookup: &L,
    caller_occurrence: Option<String>,
) -> Option<Box<ResolutionContext>> {
    let borrowed = lookup.resolution_context_ref();
    let owned = if borrowed.is_none() { lookup.resolution_context().map(Box::new) } else { None };
    let context = borrowed.or(owned.as_deref())?;
    if let Some(occurrence) = caller_occurrence
        && let Some(provenance) = context.caller_file_provenance.get(&occurrence).cloned()
    {
        let mut selected = Box::new(context.clone());
        selected.active_caller_file_provenance = Some(provenance);
        return Some(selected);
    }
    owned
}

/// Evaluates a lazy function's operands one at a time, binding each as it is
/// evaluated; an operand after the short-circuit is never evaluated or bound.
fn evaluate_lazy_function<'a, L: EvaluationLookup, O: MissingRootObserver>(
    registration: &call_binding::FunctionRegistration,
    short_circuit: functions::ShortCircuit,
    args: &[Expr],
    lookup: &L,
    argument: impl Fn(usize) -> AbsenceScope<'a>,
    observer: &mut O,
    budget: &iteration::IterationBudget,
) -> Result<Value, ExpressionError> {
    let call_error = |error| ExpressionError::Call(Box::new(error));
    registration.check_arity(args.len()).map_err(call_error)?;
    let mut call = registration.lazy_call().expect("a lazy registration starts a lazy call");
    let (stop_on_truthy, polarity) = match short_circuit {
        functions::ShortCircuit::OnFalsy => (false, scope::Polarity::True),
        functions::ShortCircuit::OnTruthy => (true, scope::Polarity::False),
    };
    // Each operand reads under the facts the earlier operands established.
    let mut facts = argument(0).facts();
    for (index, arg) in args.iter().enumerate() {
        let value = evaluate_expr(arg, lookup, argument(index).with_facts(&facts), observer, budget)?;
        let bound = call.bind_next(value).map_err(call_error)?;
        if is_truthy(bound) == stop_on_truthy {
            return Ok(Value::Bool(stop_on_truthy));
        }
        if O::OBSERVES {
            facts = facts.assume(arg, polarity);
        }
    }
    call.finish().map_err(call_error)?;
    Ok(Value::Bool(!stop_on_truthy))
}

fn evaluate_caller_file_argument<L: EvaluationLookup, O: MissingRootObserver>(
    expr: &Expr,
    lookup: &L,
    scope: AbsenceScope<'_>,
    observer: &mut O,
    budget: &iteration::IterationBudget,
) -> Result<(Value, Option<String>), ExpressionError> {
    match expr {
        Expr::Index { base, index } => {
            let (base_value, base_occurrence) =
                evaluate_caller_file_argument(base, lookup, scope, observer, budget)?;
            let index_value = evaluate_expr(index, lookup, scope.operand(), observer, budget)?;
            let value = evaluate_index(&base_value, &index_value);
            let occurrence = base_occurrence.and_then(|mut occurrence| {
                selected_array_index(&base_value, &index_value).map(|index| {
                    occurrence.push('/');
                    occurrence.push_str(&index.to_string());
                    occurrence
                })
            });
            Ok((value, occurrence))
        }
        Expr::Paren(inner) => evaluate_caller_file_argument(inner, lookup, scope, observer, budget),
        _ => Ok((evaluate_expr(expr, lookup, scope, observer, budget)?, caller_file_occurrence(expr))),
    }
}

fn selected_array_index(base: &Value, index: &Value) -> Option<usize> {
    let Value::Array(items) = base else {
        return None;
    };
    resolve_array_index(items.len(), to_number_index(index)?)
}

/// The position an index selects in a list of `len` items, counting a
/// negative index from the end; `None` for a non-whole or out-of-range index.
/// Computed in `f64` so a huge index is out of range rather than saturated.
fn resolve_array_index(len: usize, index: f64) -> Option<usize> {
    if index.fract() != 0.0 {
        return None;
    }
    let len = len as f64;
    let resolved = if index < 0.0 { len + index } else { index };
    (resolved >= 0.0 && resolved < len).then_some(resolved as usize)
}

fn caller_file_occurrence(expr: &Expr) -> Option<String> {
    match expr {
        Expr::Variable(property) => Some(format!(
            "/{}",
            property.replace('~', "~0").replace('/', "~1")
        )),
        Expr::Paren(inner) => caller_file_occurrence(inner),
        _ => None,
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

        /// Arity is the binder's: a typed call error naming every candidate
        /// signature, never a handler's text.
        #[test]
        fn arity_error_is_typed_and_names_the_signatures() {
            let err = evaluate(&parse("length()").unwrap(), &lookup(json!({}))).unwrap_err();
            let ExpressionError::Call(call) = &err else {
                panic!("an arity error is a call error: {err:?}");
            };
            assert_eq!(call.kind(), call_binding::CallErrorKind::Arity);
            assert!(err.contains("no signature takes 0 arguments"), "{err}");
            assert!(err.contains("length("), "error should include function signature: {err}");
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

            // Two missing values are both null, and null equals null
            let expr = Expr::Comparison {
                left: Box::new(Expr::Variable("missing_a".to_string())),
                op: ComparisonOp::Equal,
                right: Box::new(Expr::Variable("missing_b".to_string())),
            };
            assert_eq!(evaluate(&expr, &state).unwrap(), json!(true));
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
