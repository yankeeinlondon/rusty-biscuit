//! Passive preparation, validation, and evaluation of authored values.
//!
//! [`prepare_value`] parses every expression an authored value holds, once and
//! without evaluating anything. [`validate_prepared`] then checks those
//! expressions against a [`BindingView`]: it walks every branch, including the
//! ones a run would never take, reads no runtime value, and runs no lazy
//! provider. [`evaluate_prepared`] evaluates through an [`EvaluationSession`],
//! which applies the same root classification at runtime.
//!
//! ```
//! use std::sync::Arc;
//! use darkmatter::markdown::compose::expression::{
//!     AuthoredMode, Availability, BindingView, ScopeId, UnavailabilityReason,
//!     prepare_value, validate_prepared,
//! };
//! use serde_json::json;
//!
//! let view = BindingView::builder(ScopeId::new("start"))
//!     .declare(
//!         "err",
//!         "The error that ended the run.",
//!         Availability::Unavailable(UnavailabilityReason::new("host.event-has-no-error")),
//!     )
//!     .build()
//!     .unwrap();
//! // `err` sits in the branch a run with `ok` set never takes; it is still reported.
//! let prepared = prepare_value(
//!     &json!("{{ ok ? 'fine' : err.message }}"),
//!     AuthoredMode::InterpolatedValue,
//! )
//! .unwrap();
//! let diagnostics = validate_prepared(&prepared, &view);
//! assert_eq!(diagnostics.len(), 1);
//! ```

use std::ops::Range;

use serde_json::Value;

use super::absence::static_variable_reads;
use super::ast::{SpannedExpr, SpannedExprKind};
use super::binding::{
    Availability, BindingError, BindingView, EvaluationSession, RootClass, UnavailableBinding,
};
use super::lexer::{ExpressionFinder, ParseMode};
use super::parser::{parse_condition_spanned, parse_spanned};
use super::{ExpressionError, evaluate, functions, unknown_function_error};
use crate::markdown::compose::interpolation::{Evaluator, ExpressionFailurePolicy, interpolate_value};
use crate::markdown::span::SourceSpan;
use crate::markdown::types::{MarkdownError, SourceRef};

/// How an authored value holds its expressions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthoredMode {
    /// A string that is one bare expression (a `when:` condition), parsed in
    /// the given mode. A non-string value is a constant.
    Expression(ParseMode),
    /// A string interpolated as a whole value: one `{{ … }}` span keeps its
    /// typed result, anything else renders to a string. A non-string value is
    /// a constant.
    InterpolatedValue,
    /// A JSON tree whose every string leaf is an interpolated value.
    Subtree,
}

/// One parsed expression of a [`PreparedValue`].
#[derive(Debug, Clone, PartialEq)]
pub struct PreparedExpression {
    pointer: String,
    offset: usize,
    text: String,
    expr: SpannedExpr,
}

impl PreparedExpression {
    /// JSON pointer of the string leaf holding the expression (`""` for the
    /// root value).
    pub fn pointer(&self) -> &str {
        &self.pointer
    }

    /// The parsed expression text.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// The parsed expression; its spans are relative to [`text`](Self::text).
    pub fn expr(&self) -> &SpannedExpr {
        &self.expr
    }

    /// Maps a span of [`text`](Self::text) onto the string leaf.
    fn leaf_span(&self, span: &SourceSpan) -> SourceSpan {
        self.offset + span.start..self.offset + span.end
    }
}

/// An authored value whose expressions have all been parsed.
///
/// It holds no runtime value and no view-derived state, so one prepared value
/// can be validated against several views and evaluated in several sessions.
#[derive(Debug, Clone, PartialEq)]
pub struct PreparedValue {
    mode: AuthoredMode,
    input: Value,
    expressions: Vec<PreparedExpression>,
}

impl PreparedValue {
    pub fn mode(&self) -> AuthoredMode {
        self.mode
    }

    /// The authored value as given to [`prepare_value`].
    pub fn input(&self) -> &Value {
        &self.input
    }

    /// Every parsed expression, in document order.
    pub fn expressions(&self) -> &[PreparedExpression] {
        &self.expressions
    }
}

/// An authored expression that does not parse.
#[derive(Debug, Clone, thiserror::Error)]
#[error("cannot parse `{text}` at `{pointer}`: {cause}")]
pub struct PreparationError {
    /// JSON pointer of the string leaf.
    pub pointer: String,
    /// Byte range of the failing expression (its `{{ … }}` for an
    /// interpolated value) in the leaf.
    pub span: Range<usize>,
    /// The expression text that failed.
    pub text: String,
    /// The parse failure.
    pub cause: Box<ExpressionError>,
}

/// A definite problem [`validate_prepared`] found without evaluating.
#[derive(Debug, Clone)]
pub struct ValidationDiagnostic {
    /// JSON pointer of the string leaf.
    pub pointer: String,
    /// Byte range of the offending node in the leaf.
    pub span: SourceSpan,
    /// The same typed error evaluation raises: [`ExpressionError::Binding`]
    /// for an unavailable global, [`ExpressionError::UnknownFunction`] for an
    /// unknown function.
    pub error: ExpressionError,
}

/// Parses every expression `input` holds under `mode`.
///
/// Nothing is evaluated, no environment or host fact is read, and no lazy
/// global runs.
///
/// ## Errors
///
/// The first expression that does not parse, located by pointer and span.
pub fn prepare_value(input: &Value, mode: AuthoredMode) -> Result<PreparedValue, PreparationError> {
    let mut expressions = Vec::new();
    match (mode, input) {
        (AuthoredMode::Expression(parse_mode), Value::String(text)) => {
            let expr = parse_in_mode(text, parse_mode).map_err(|cause| PreparationError {
                pointer: String::new(),
                span: 0..text.len(),
                text: text.clone(),
                cause: Box::new(cause),
            })?;
            expressions.push(PreparedExpression {
                pointer: String::new(),
                offset: 0,
                text: text.clone(),
                expr,
            });
        }
        (AuthoredMode::Expression(_), _) => {}
        (AuthoredMode::InterpolatedValue, Value::String(text)) => {
            scan_leaf(text, "", &mut expressions)?;
        }
        (AuthoredMode::InterpolatedValue, _) => {}
        (AuthoredMode::Subtree, value) => scan_tree(value, &mut String::new(), &mut expressions)?,
    }
    Ok(PreparedValue {
        mode,
        input: input.clone(),
        expressions,
    })
}

fn parse_in_mode(text: &str, mode: ParseMode) -> Result<SpannedExpr, ExpressionError> {
    match mode {
        ParseMode::Interpolation => parse_spanned(text),
        ParseMode::Condition => parse_condition_spanned(text),
    }
    .map_err(|error| ExpressionError::Parse(error.to_string()))
}

fn scan_tree(
    value: &Value,
    pointer: &mut String,
    expressions: &mut Vec<PreparedExpression>,
) -> Result<(), PreparationError> {
    match value {
        Value::String(text) => scan_leaf(text, pointer, expressions),
        Value::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                let length = pointer.len();
                pointer.push('/');
                pointer.push_str(&index.to_string());
                scan_tree(item, pointer, expressions)?;
                pointer.truncate(length);
            }
            Ok(())
        }
        Value::Object(map) => {
            for (key, item) in map {
                let length = pointer.len();
                pointer.push('/');
                pointer.push_str(&key.replace('~', "~0").replace('/', "~1"));
                scan_tree(item, pointer, expressions)?;
                pointer.truncate(length);
            }
            Ok(())
        }
        Value::Null | Value::Bool(_) | Value::Number(_) => Ok(()),
    }
}

/// Parses each `{{ … }}` span the interpolator would evaluate in `text`.
/// Literals (`{{{ … }}}`) and literal tokens are not expressions.
fn scan_leaf(
    text: &str,
    pointer: &str,
    expressions: &mut Vec<PreparedExpression>,
) -> Result<(), PreparationError> {
    for location in ExpressionFinder::scan_plain(text).expressions {
        let inner = &text[location.start + 2..location.end - 2];
        let offset = location.start + 2 + (inner.len() - inner.trim_start().len());
        let expr = parse_in_mode(&location.expression, ParseMode::Interpolation).map_err(|cause| {
            PreparationError {
                pointer: pointer.to_string(),
                span: location.start..location.end,
                text: location.expression.clone(),
                cause: Box::new(cause),
            }
        })?;
        expressions.push(PreparedExpression {
            pointer: pointer.to_string(),
            offset,
            text: location.expression,
            expr,
        });
    }
    Ok(())
}

/// Reports every definitely unavailable global and unknown function in
/// `prepared`, in every branch.
///
/// An `execution-dependent` global is deferred to runtime. Nothing is
/// evaluated and no provider runs.
pub fn validate_prepared(prepared: &PreparedValue, view: &BindingView) -> Vec<ValidationDiagnostic> {
    prepared
        .expressions
        .iter()
        .flat_map(|expression| {
            validate_expression(&expression.expr, view)
                .into_iter()
                .map(|(span, error)| ValidationDiagnostic {
                    pointer: expression.pointer.clone(),
                    span: expression.leaf_span(&span),
                    error: relocate(error, expression.leaf_span(&span)),
                })
        })
        .collect()
}

/// The [`validate_prepared`] check for one already-parsed expression, for a
/// static consumer that parses its own text (DMLS): each definitely
/// unavailable global and unknown function, in every branch, with its span in
/// `expr`'s text and the typed error evaluation raises.
pub fn validate_expression(expr: &SpannedExpr, view: &BindingView) -> Vec<(SourceSpan, ExpressionError)> {
    let mut diagnostics = Vec::new();
    for read in static_variable_reads(expr) {
        if let RootClass::Global(Availability::Unavailable(reason)) = view.classify_root(read.root()) {
            diagnostics.push((
                read.span.clone(),
                ExpressionError::Binding(Box::new(BindingError::Unavailable(Box::new(
                    UnavailableBinding {
                        root: read.root().to_string(),
                        path: read.path.to_string(),
                        scope: view.scope().clone(),
                        reason: reason.clone(),
                        span: Some(read.span.clone()),
                    },
                )))),
            ));
        }
    }
    visit_calls(expr, &mut |name, span| {
        if !functions::is_dispatchable(name) {
            diagnostics.push((span.clone(), unknown_function_error(name)));
        }
    });
    diagnostics
}

/// Points an unavailable-global error's span at `span` (the leaf range).
fn relocate(mut error: ExpressionError, span: SourceSpan) -> ExpressionError {
    if let ExpressionError::Binding(binding) = &mut error
        && let BindingError::Unavailable(unavailable) = binding.as_mut()
    {
        unavailable.span = Some(span);
    }
    error
}

/// Calls `visit` for every function call in `expr`, in every branch.
fn visit_calls<'e>(expr: &'e SpannedExpr, visit: &mut impl FnMut(&'e str, &'e SourceSpan)) {
    match &expr.kind {
        SpannedExprKind::Variable(_)
        | SpannedExprKind::StringLiteral(_)
        | SpannedExprKind::NumberLiteral(_)
        | SpannedExprKind::BoolLiteral(_) => {}
        SpannedExprKind::ArrayLiteral(items) => items.iter().for_each(|item| visit_calls(item, visit)),
        SpannedExprKind::ObjectLiteral(entries) => {
            entries.iter().for_each(|(_, value)| visit_calls(value, visit));
        }
        SpannedExprKind::Paren(inner)
        | SpannedExprKind::UnaryNot(inner)
        | SpannedExprKind::UnaryMinus(inner)
        | SpannedExprKind::MemberAccess { base: inner, .. } => visit_calls(inner, visit),
        SpannedExprKind::Binary { left, right, .. }
        | SpannedExprKind::Comparison { left, right, .. } => {
            visit_calls(left, visit);
            visit_calls(right, visit);
        }
        SpannedExprKind::Index { base, index } => {
            visit_calls(base, visit);
            visit_calls(index, visit);
        }
        SpannedExprKind::Fallback { primary, fallback } => {
            visit_calls(primary, visit);
            visit_calls(fallback, visit);
        }
        SpannedExprKind::Ternary {
            condition,
            then_branch,
            else_branch,
        } => {
            visit_calls(condition, visit);
            visit_calls(then_branch, visit);
            visit_calls(else_branch, visit);
        }
        SpannedExprKind::FunctionCall { name, args } => {
            visit(name, &expr.span);
            args.iter().for_each(|arg| visit_calls(arg, visit));
        }
    }
}

/// Evaluates `prepared` through `session`.
///
/// An expression is evaluated once to its typed value. An interpolated value
/// follows composition's whole-value and mixed-string rules, failing on any
/// parse or evaluation error. A subtree interpolates each string leaf and
/// keeps every other value.
///
/// ## Errors
///
/// [`MarkdownError::Interpolation`] carrying the typed [`ExpressionError`],
/// including [`ExpressionError::Binding`] for a read of an unavailable global.
pub fn evaluate_prepared(
    prepared: &PreparedValue,
    session: &EvaluationSession<'_>,
) -> Result<Value, MarkdownError> {
    match (prepared.mode, &prepared.input) {
        (AuthoredMode::Expression(_), Value::String(_)) => {
            let expression = &prepared.expressions[0];
            evaluate(&expression.expr.erase(), session).map_err(|cause| MarkdownError::Interpolation {
                key: None,
                expression: expression.text.clone(),
                source: Box::new(SourceRef::Effective {
                    rendered: expression.text.clone(),
                    origin_key: None,
                }),
                cause: Box::new(cause),
            })
        }
        (AuthoredMode::InterpolatedValue, Value::String(text)) => interpolate_leaf(text, session),
        (AuthoredMode::Subtree, value) => interpolate_tree(value, session),
        (_, constant) => Ok(constant.clone()),
    }
}

fn interpolate_tree(value: &Value, session: &EvaluationSession<'_>) -> Result<Value, MarkdownError> {
    match value {
        Value::String(text) => interpolate_leaf(text, session),
        Value::Array(items) => items
            .iter()
            .map(|item| interpolate_tree(item, session))
            .collect::<Result<_, _>>()
            .map(Value::Array),
        Value::Object(map) => map
            .iter()
            .map(|(key, item)| Ok((key.clone(), interpolate_tree(item, session)?)))
            .collect::<Result<_, MarkdownError>>()
            .map(Value::Object),
        other => Ok(other.clone()),
    }
}

fn interpolate_leaf(text: &str, session: &EvaluationSession<'_>) -> Result<Value, MarkdownError> {
    let evaluator = Evaluator::new(session);
    interpolate_value(text, &evaluator, ExpressionFailurePolicy::Strict, "prepared-value")
        .map(|(value, _replacements, _warnings)| value)
}
