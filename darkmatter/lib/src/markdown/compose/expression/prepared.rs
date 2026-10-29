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
use std::sync::OnceLock;

use serde_json::Value;

use super::absence::static_variable_reads_in_scope;
use super::ast::{Expr, SpannedBindingPattern, SpannedCallback, SpannedExpr, SpannedExprKind};
use super::binding::{
    Availability, BindingError, BindingView, EvaluationSession, RootClass, UnavailableBinding,
};
use super::lexer::{ExpressionFinder, ExpressionLocation, ParseMode};
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
    pub(crate) fn leaf_span(&self, span: &SourceSpan) -> SourceSpan {
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
    .map_err(ExpressionError::from)
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

/// One `{{ … }}` occurrence the interpolator would evaluate, parsed once.
struct ScannedOccurrence {
    /// The whole placeholder, braces included.
    span: Range<usize>,
    /// Byte offset of the trimmed expression text in the scanned source.
    offset: usize,
    /// The trimmed expression text.
    text: String,
    parsed: Result<SpannedExpr, ExpressionError>,
}

/// Parses each occurrence in `locations` lazily, in source order, so a caller
/// that stops at the first failure parses nothing after it.
fn parse_occurrences<'t>(
    text: &'t str,
    locations: Vec<ExpressionLocation>,
) -> impl Iterator<Item = ScannedOccurrence> + 't {
    locations.into_iter().map(move |location| {
        let inner = &text[location.start + 2..location.end - 2];
        let offset = location.start + 2 + (inner.len() - inner.trim_start().len());
        let parsed = parse_in_mode(&location.expression, ParseMode::Interpolation);
        ScannedOccurrence {
            span: location.start..location.end,
            offset,
            text: location.expression,
            parsed,
        }
    })
}

/// Parses each `{{ … }}` span the interpolator would evaluate in `text`.
/// Literals (`{{{ … }}}`) and literal tokens are not expressions.
fn scan_leaf(
    text: &str,
    pointer: &str,
    expressions: &mut Vec<PreparedExpression>,
) -> Result<(), PreparationError> {
    for occurrence in parse_occurrences(text, ExpressionFinder::scan_plain(text).expressions) {
        let expr = occurrence.parsed.map_err(|cause| PreparationError {
            pointer: pointer.to_string(),
            span: occurrence.span.clone(),
            text: occurrence.text.clone(),
            cause: Box::new(cause),
        })?;
        expressions.push(PreparedExpression {
            pointer: pointer.to_string(),
            offset: occurrence.offset,
            text: occurrence.text,
            expr,
        });
    }
    Ok(())
}

/// One `{{ … }}` occurrence of a [`PreparedInterpolation`]: its place in the
/// source and either its parsed syntax or the reason it does not parse.
#[derive(Debug, Clone)]
pub struct PreparedPlaceholder {
    span: Range<usize>,
    offset: usize,
    parsed: Result<PreparedExpression, PreparationError>,
    erased: OnceLock<Expr>,
}

impl PreparedPlaceholder {
    /// The whole placeholder, `{{` and `}}` included, in the source.
    pub fn span(&self) -> Range<usize> {
        self.span.clone()
    }

    /// The trimmed expression text, whether or not it parses.
    pub fn text(&self) -> &str {
        match &self.parsed {
            Ok(expression) => expression.text(),
            Err(error) => &error.text,
        }
    }

    /// Byte offset of [`text`](Self::text) in the source.
    pub fn offset(&self) -> usize {
        self.offset
    }

    /// The parsed syntax, or the failure retained for this occurrence.
    pub fn parsed(&self) -> Result<&PreparedExpression, &PreparationError> {
        self.parsed.as_ref()
    }

    /// The span-erased expression, erased on first use and kept; `None` when
    /// the occurrence does not parse.
    pub fn expr(&self) -> Option<&Expr> {
        let expression = self.parsed.as_ref().ok()?;
        Some(self.erased.get_or_init(|| expression.expr.erase()))
    }
}

/// An interpolated text whose every `{{ … }}` occurrence has been scanned and
/// parsed once, malformed ones included.
///
/// It owns its source and holds no runtime value, no schema, and no approval
/// decision, so any number of static analyses can read it without parsing
/// again. Occurrences are identified by their span, never by their text.
#[derive(Debug, Clone)]
pub struct PreparedInterpolation {
    text: String,
    placeholders: Vec<PreparedPlaceholder>,
    /// Spans of `{{{ … }}}` literals.
    literals: Vec<Range<usize>>,
    /// Spans of `{{!data:…}}` literal tokens.
    tokens: Vec<Range<usize>>,
}

impl PreparedInterpolation {
    /// Scans `text` and parses each occurrence exactly once, retaining
    /// failures in source order. Nothing is evaluated.
    pub fn new(text: &str) -> Self {
        let scan = ExpressionFinder::scan_plain(text);
        let placeholders = parse_occurrences(text, scan.expressions)
            .map(|occurrence| {
                let parsed = match occurrence.parsed {
                    Ok(expr) => Ok(PreparedExpression {
                        pointer: String::new(),
                        offset: occurrence.offset,
                        text: occurrence.text,
                        expr,
                    }),
                    Err(cause) => Err(PreparationError {
                        pointer: String::new(),
                        span: occurrence.span.clone(),
                        text: occurrence.text,
                        cause: Box::new(cause),
                    }),
                };
                PreparedPlaceholder {
                    span: occurrence.span,
                    offset: occurrence.offset,
                    parsed,
                    erased: OnceLock::new(),
                }
            })
            .collect();
        Self {
            text: text.to_string(),
            placeholders,
            literals: scan.literals.into_iter().map(|literal| literal.start..literal.end).collect(),
            tokens: scan.tokens.into_iter().map(|token| token.start..token.end).collect(),
        }
    }

    /// The source text.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Every occurrence, in source order.
    pub fn placeholders(&self) -> &[PreparedPlaceholder] {
        &self.placeholders
    }

    /// A view of the occurrences inside `range`, in `range`-relative
    /// coordinates.
    ///
    /// ## Panics
    ///
    /// When `range` is reversed, past the end of the source, or not on
    /// character boundaries. That is a caller bug, not authored input.
    pub fn view(&self, range: Range<usize>) -> PreparedInterpolationView<'_> {
        assert!(
            range.start <= range.end && self.text.is_char_boundary(range.start) && self.text.is_char_boundary(range.end),
            "view {range:?} is not a character-boundary range of the {} byte source",
            self.text.len()
        );
        PreparedInterpolationView { parent: self, range }
    }
}

/// A byte range of a [`PreparedInterpolation`] seen in its own coordinates.
///
/// A view carries coordinates only: it selects the occurrences wholly inside
/// its range and subtracts the range start from their positions. The parent
/// keeps the raw coordinates.
#[derive(Debug, Clone)]
pub struct PreparedInterpolationView<'a> {
    parent: &'a PreparedInterpolation,
    range: Range<usize>,
}

impl<'a> PreparedInterpolationView<'a> {
    /// The viewed slice of the source.
    pub fn text(&self) -> &'a str {
        &self.parent.text[self.range.clone()]
    }

    /// The viewed range in the parent's coordinates.
    pub fn range(&self) -> Range<usize> {
        self.range.clone()
    }

    /// The collection this view reads.
    pub fn parent(&self) -> &'a PreparedInterpolation {
        self.parent
    }

    /// The occurrences wholly inside the view, in source order.
    pub fn placeholders(&self) -> impl Iterator<Item = PlaceholderView<'a>> + 'a {
        let (range, base) = (self.range.clone(), self.range.start);
        self.parent
            .placeholders
            .iter()
            .filter(move |placeholder| contains(&range, &placeholder.span))
            .map(move |placeholder| PlaceholderView {
                placeholder,
                base,
            })
    }

    /// The occurrence whose whole span is `span` in the view's coordinates.
    pub fn placeholder_at(&self, span: &Range<usize>) -> Option<PlaceholderView<'a>> {
        self.placeholders().find(|placeholder| placeholder.span() == *span)
    }

    /// Whether the view holds a `{{{ … }}}` literal or a `{{!data:…}}` token,
    /// neither of which template mode accepts.
    pub fn has_literal_syntax(&self) -> bool {
        self.parent.literals.iter().chain(&self.parent.tokens).any(|span| contains(&self.range, span))
    }
}

fn contains(range: &Range<usize>, inner: &Range<usize>) -> bool {
    range.start <= inner.start && inner.end <= range.end
}

/// An occurrence seen through a [`PreparedInterpolationView`].
#[derive(Debug, Clone, Copy)]
pub struct PlaceholderView<'a> {
    placeholder: &'a PreparedPlaceholder,
    base: usize,
}

impl<'a> PlaceholderView<'a> {
    /// The whole placeholder in the view's coordinates.
    pub fn span(&self) -> Range<usize> {
        self.placeholder.span.start - self.base..self.placeholder.span.end - self.base
    }

    /// The trimmed expression text's offset in the view's coordinates.
    pub fn offset(&self) -> usize {
        self.placeholder.offset() - self.base
    }

    /// The underlying occurrence, in the parent's coordinates.
    pub fn placeholder(&self) -> &'a PreparedPlaceholder {
        self.placeholder
    }

    /// The trimmed expression text.
    pub fn text(&self) -> &'a str {
        self.placeholder.text()
    }

    /// The span-erased expression; `None` when the occurrence does not parse.
    pub fn expr(&self) -> Option<&'a Expr> {
        self.placeholder.expr()
    }
}

/// The [`validate_prepared`] check for every occurrence of `prepared`,
/// without parsing anything again. Spans are in the source's coordinates.
///
/// ## Errors
///
/// The first retained [`PreparationError`], in which case no diagnostics are
/// reported.
pub fn validate_interpolation(
    prepared: &PreparedInterpolation,
    view: &BindingView,
) -> Result<Vec<ValidationDiagnostic>, PreparationError> {
    let mut diagnostics = Vec::new();
    for placeholder in &prepared.placeholders {
        let expression = placeholder.parsed.as_ref().map_err(Clone::clone)?;
        diagnostics.extend(expression_diagnostics(expression, view));
    }
    Ok(diagnostics)
}

/// Reports unavailable globals, unknown functions, and protected callback
/// bindings in every branch of `prepared`.
///
/// An `execution-dependent` global is deferred to runtime. Nothing is
/// evaluated and no provider runs.
pub fn validate_prepared(prepared: &PreparedValue, view: &BindingView) -> Vec<ValidationDiagnostic> {
    prepared
        .expressions
        .iter()
        .flat_map(|expression| expression_diagnostics(expression, view))
        .collect()
}

/// The diagnostics of one prepared expression, relocated onto its leaf.
fn expression_diagnostics<'a>(
    expression: &'a PreparedExpression,
    view: &'a BindingView,
) -> impl Iterator<Item = ValidationDiagnostic> + 'a {
    validate_expression(&expression.expr, view)
        .into_iter()
        .map(|(span, error)| ValidationDiagnostic {
            pointer: expression.pointer.clone(),
            span: expression.leaf_span(&span),
            error: relocate(error, expression.leaf_span(&span)),
        })
}

/// The [`validate_prepared`] check for one already-parsed expression, for a
/// static consumer that parses its own text (DMLS): each definitely
/// unavailable global and unknown function, in every branch, with its span in
/// `expr`'s text and the typed error evaluation raises.
pub fn validate_expression(expr: &SpannedExpr, view: &BindingView) -> Vec<(SourceSpan, ExpressionError)> {
    validate_expression_in_scope(expr, view, &[])
}

/// [`validate_expression`] for an expression evaluated inside lexical scopes
/// that bind `locals`: a read of a local is never a global or a document read,
/// so it is never reported, while every free capture (`ctx`, `current`, host
/// globals) is still checked.
pub fn validate_expression_in_scope(
    expr: &SpannedExpr,
    view: &BindingView,
    locals: &[&str],
) -> Vec<(SourceSpan, ExpressionError)> {
    let mut diagnostics = Vec::new();
    for read in static_variable_reads_in_scope(expr, locals) {
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
    super::call_check::visit_call_nodes(expr, &mut |call| {
        let SpannedExprKind::CallbackCall { args, .. } = &call.kind else { return; };
        for argument in args {
            let super::ast::SpannedCallArgument::Callback(callback) = argument else { continue; };
            if let Err(error) = super::check_local_names(callback.pattern.erase().names(), Some(view)) {
                diagnostics.push((callback.pattern_span.clone(), ExpressionError::InvalidCallback {
                    span: callback.pattern_span.clone(),
                    message: error.to_string(),
                }));
            }
        }
    });
    visit_calls(expr, &mut |name, span| {
        if !functions::is_dispatchable(name) {
            diagnostics.push((span.clone(), unknown_function_error(name)));
        }
    });
    diagnostics
}

/// [`validate_expression_in_scope`] for a callback body, with its pattern
/// names added to `outer_locals`. Pattern-name collisions are not reported
/// here; see [`check_local_names`](super::check_local_names).
pub fn validate_callback(
    callback: &SpannedCallback,
    view: &BindingView,
    outer_locals: &[&str],
) -> Vec<(SourceSpan, ExpressionError)> {
    let mut locals: Vec<&str> = outer_locals.to_vec();
    match &callback.pattern {
        SpannedBindingPattern::Identifier(name) => locals.push(&name.value),
        SpannedBindingPattern::Fields(fields) => locals.extend(fields.iter().map(|field| field.value.as_str())),
    }
    validate_expression_in_scope(&callback.body, view, &locals)
}

/// Locates binding and callback failures within their string leaf.
fn relocate(mut error: ExpressionError, span: SourceSpan) -> ExpressionError {
    if let ExpressionError::Binding(binding) = &mut error
        && let BindingError::Unavailable(unavailable) = binding.as_mut()
    {
        unavailable.span = Some(span.clone());
    }
    if let ExpressionError::InvalidCallback { span: located, .. } = &mut error {
        *located = span;
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
        SpannedExprKind::CallbackCall { name, args } => {
            visit(name, &expr.span);
            args.iter().for_each(|arg| visit_calls(arg.expression(), visit));
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

#[cfg(test)]
mod interpolation_tests {
    use super::*;
    use crate::markdown::compose::expression::{ScopeId, UnavailabilityReason};
    use serde_json::json;

    fn spans(prepared: &PreparedInterpolation) -> Vec<Range<usize>> {
        prepared.placeholders().iter().map(PreparedPlaceholder::span).collect()
    }

    fn err_view() -> BindingView {
        BindingView::builder(ScopeId::new("start"))
            .declare(
                "err",
                "The error that ended the run.",
                Availability::Unavailable(UnavailabilityReason::new("host.event-has-no-error")),
            )
            .build()
            .unwrap()
    }

    #[test]
    fn mixed_valid_and_invalid_occurrences_are_all_retained_in_source_order() {
        let text = "a {{ x }} b {{ 1 + }} c {{ y }}";
        let prepared = PreparedInterpolation::new(text);
        assert_eq!(spans(&prepared), vec![2..9, 12..21, 24..31]);
        let found: Vec<(bool, &str)> =
            prepared.placeholders().iter().map(|p| (p.parsed().is_ok(), p.text())).collect();
        assert_eq!(found, vec![(true, "x"), (false, "1 +"), (true, "y")]);
        let failure = prepared.placeholders()[1].parsed().unwrap_err();
        assert_eq!(failure.span, 12..21);
        assert_eq!(failure.text, "1 +");
        assert!(prepared.placeholders()[1].expr().is_none());
        assert_eq!(prepared.placeholders()[1].offset(), 15);
    }

    #[test]
    fn prepare_value_still_stops_at_the_first_error() {
        let error = prepare_value(&json!("{{ 1 + }} {{ also bad ( }}"), AuthoredMode::InterpolatedValue).unwrap_err();
        assert_eq!(error.text, "1 +");
        assert_eq!(error.span, 0..9);
        let ok = prepare_value(&json!("{{ a }} {{ b }}"), AuthoredMode::InterpolatedValue).unwrap();
        assert_eq!(ok.expressions().iter().map(PreparedExpression::text).collect::<Vec<_>>(), ["a", "b"]);
    }

    #[test]
    fn repeated_text_keeps_one_occurrence_per_span() {
        let prepared = PreparedInterpolation::new("{{ name }} and {{ name }}");
        assert_eq!(spans(&prepared), vec![0..10, 15..25]);
        assert_eq!(prepared.placeholders()[0].text(), prepared.placeholders()[1].text());
        assert_eq!(prepared.placeholders()[0].offset(), 3);
        assert_eq!(prepared.placeholders()[1].offset(), 18);
    }

    #[test]
    fn erasure_is_cached_and_equals_the_parsed_expression() {
        let prepared = PreparedInterpolation::new("{{ a.b }}");
        let placeholder = &prepared.placeholders()[0];
        let first = placeholder.expr().unwrap();
        assert!(std::ptr::eq(first, placeholder.expr().unwrap()));
        assert_eq!(*first, crate::markdown::compose::expression::parse("a.b").unwrap());
    }

    #[test]
    fn no_placeholders_triple_braces_and_literal_tokens() {
        assert!(PreparedInterpolation::new("plain text").placeholders().is_empty());
        let literal = PreparedInterpolation::new("{{{ keep }}} {{ x }}");
        assert_eq!(literal.placeholders().len(), 1);
        assert!(literal.view(0..literal.text().len()).has_literal_syntax());
        let token = PreparedInterpolation::new("{{!data:abc}} {{ x }}");
        assert_eq!(token.placeholders().len(), 1);
        assert!(token.view(0..token.text().len()).has_literal_syntax());
        let none = PreparedInterpolation::new("{{ x }}");
        assert!(!none.view(0..none.text().len()).has_literal_syntax());
    }

    #[test]
    fn a_view_translates_coordinates_and_selects_contained_occurrences() {
        // `é` is two bytes and the NBSP padding is two bytes: offsets are bytes.
        let text = "é set(tool {{\u{a0}a\u{a0}}} {{ b }}) {{ c }}";
        let prepared = PreparedInterpolation::new(text);
        let start = text.find("tool").unwrap();
        let end = text.find(')').unwrap();
        let view = prepared.view(start..end);
        assert_eq!(view.text(), "tool {{\u{a0}a\u{a0}}} {{ b }}");
        let inner: Vec<_> = view.placeholders().map(|p| (p.text(), p.span(), p.offset())).collect();
        let a_span = text.find("{{\u{a0}a").unwrap() - start;
        let a_end = a_span + "{{\u{a0}a\u{a0}}}".len();
        let b_span = text.find("{{ b").unwrap() - start;
        assert_eq!(
            inner,
            vec![("a", a_span..a_end, a_span + 4), ("b", b_span..b_span + 7, b_span + 3)]
        );
        for placeholder in view.placeholders() {
            let parent = placeholder.placeholder();
            assert_eq!(&view.text()[placeholder.offset()..][..placeholder.text().len()], placeholder.text());
            assert_eq!(&text[parent.offset()..][..parent.text().len()], parent.text());
        }
        assert_eq!(view.placeholder_at(&(b_span..b_span + 7)).unwrap().text(), "b");
        assert!(view.placeholder_at(&(0..1)).is_none());
        let full = prepared.view(0..text.len());
        assert_eq!(full.placeholders().count(), 3);
        assert_eq!(full.placeholders().next().unwrap().span(), prepared.placeholders()[0].span());
    }

    #[test]
    fn view_literal_syntax_considers_only_contained_metadata() {
        let text = "{{{ lit }}} tool {{ x }}";
        let prepared = PreparedInterpolation::new(text);
        let start = text.find("tool").unwrap();
        assert!(!prepared.view(start..text.len()).has_literal_syntax());
        assert!(prepared.view(0..text.len()).has_literal_syntax());
    }

    #[test]
    #[should_panic(expected = "character-boundary")]
    fn a_view_range_inside_a_multibyte_character_is_a_programmer_error() {
        PreparedInterpolation::new("é {{ x }}").view(1..4);
    }

    #[test]
    fn validation_reports_dead_branch_reads_at_source_coordinates() {
        let text = "é {{ ok ? 'fine' : err.message }}";
        let prepared = PreparedInterpolation::new(text);
        let diagnostics = validate_interpolation(&prepared, &err_view()).unwrap();
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(&text[diagnostics[0].span.clone()], "err.message");
        let expected = validate_prepared(
            &prepare_value(&json!(text), AuthoredMode::InterpolatedValue).unwrap(),
            &err_view(),
        );
        assert_eq!(diagnostics[0].span, expected[0].span);
    }

    #[test]
    fn retained_callbacks_validate_locals_and_free_captures_at_source_coordinates() {
        let text = "é {{ map([], item => item.message || err.message) }}";
        let prepared = PreparedInterpolation::new(text);
        let diagnostics = validate_interpolation(&prepared, &err_view()).unwrap();
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(&text[diagnostics[0].span.clone()], "err.message");
        let text = "{{ map([], err => err.message) }}";
        let diagnostics = validate_interpolation(&PreparedInterpolation::new(text), &err_view()).unwrap();
        assert_eq!(diagnostics.len(), 1);
        assert!(matches!(diagnostics[0].error, ExpressionError::InvalidCallback { .. }));
        assert_eq!(&text[diagnostics[0].span.clone()], "err");
    }

    #[test]
    fn a_retained_failure_suppresses_every_diagnostic() {
        let prepared = PreparedInterpolation::new("{{ err.message }} {{ 1 + }}");
        let error = validate_interpolation(&prepared, &err_view()).unwrap_err();
        assert_eq!(error.text, "1 +");
        assert_eq!(error.span, 18..27);
    }

    #[cfg(feature = "work-counters")]
    mod counted {
        use super::*;
        use crate::work_counters::{Work, snapshot};

        fn parses_during(run: impl FnOnce()) -> u64 {
            let before = snapshot();
            run();
            snapshot().since(&before).get(Work::ExpressionParse)
        }

        #[test]
        fn construction_parses_each_occurrence_once_including_failures() {
            let mut prepared = None;
            let parses = parses_during(|| {
                prepared = Some(PreparedInterpolation::new("{{ a }} {{ a }} {{ 1 + }} {{{ lit }}} {{!data:x}}"));
            });
            assert_eq!(parses, 3);
            assert_eq!(prepared.unwrap().placeholders().len(), 3);
        }

        #[test]
        fn views_erasure_and_validation_parse_nothing() {
            let prepared = PreparedInterpolation::new("{{ a }} {{ b }} {{ err.x }}");
            let view = err_view();
            let parses = parses_during(|| {
                let full = prepared.view(0..prepared.text().len());
                for placeholder in full.placeholders() {
                    let _ = placeholder.expr();
                    let _ = placeholder.expr();
                }
                let _ = prepared.view(8..15).placeholders().count();
                let _ = validate_interpolation(&prepared, &view).unwrap();
                let _ = validate_interpolation(&prepared, &view).unwrap();
            });
            assert_eq!(parses, 0);
        }

        #[test]
        fn prepare_value_parses_up_to_and_including_the_first_failure() {
            let parses = parses_during(|| {
                let _ = prepare_value(&json!("{{ a }} {{ 1 + }} {{ b }}"), AuthoredMode::InterpolatedValue);
            });
            assert_eq!(parses, 2);
        }
    }
}

#[cfg(test)]
mod lexical_tests {
    use super::*;
    use crate::markdown::compose::expression::binding::{ScopeId, UnavailabilityReason};
    use crate::markdown::span::Spanned;

    fn view() -> BindingView {
        BindingView::builder(ScopeId::new("lexical"))
            .declare("gone", "unavailable", Availability::Unavailable(UnavailabilityReason::new("host.gone")))
            .build()
            .unwrap()
    }

    fn callback(body: &str) -> SpannedCallback {
        SpannedCallback {
            pattern_span: 0..1,
            pattern: SpannedBindingPattern::Identifier(Spanned::new("p".to_string(), 0..1)),
            arrow: 2..4,
            body: parse_spanned(body).unwrap(),
        }
    }

    #[test]
    fn a_callback_body_still_reports_an_unavailable_host_capture() {
        let diagnostics = validate_callback(&callback("p.x || gone.y || local.gone || local.p"), &view(), &[]);
        let reads: Vec<String> = diagnostics
            .iter()
            .map(|(_, error)| match error {
                ExpressionError::Binding(binding) => binding.root().to_string(),
                other => panic!("unexpected {other:?}"),
            })
            .collect();
        assert_eq!(reads, ["gone"], "only the free capture counts; `local.gone` is lexical");
    }

    #[test]
    fn an_unknown_function_in_a_callback_body_is_still_reported() {
        let diagnostics = validate_callback(&callback("no_such_fn(p)"), &view(), &[]);
        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn inline_callback_validation_checks_nested_calls_and_protected_names_passively() {
        for arrow in ["=>", "->"] {
            let text = format!("map([], gone {arrow} gone.value)");
            let prepared = prepare_value(&serde_json::json!(text), AuthoredMode::Expression(ParseMode::Interpolation)).unwrap();
            let diagnostics = validate_prepared(&prepared, &view());
            assert_eq!(diagnostics.len(), 1, "{text}: {diagnostics:?}");
            assert!(matches!(&diagnostics[0].error, ExpressionError::InvalidCallback { message, .. } if message.contains("gone")));
            assert_eq!(&text[diagnostics[0].span.clone()], "gone");
            let text = format!("map([], p {arrow} no_such_fn(p.value))");
            let expression = parse_spanned(&text).unwrap();
            let diagnostics = validate_expression(&expression, &view());
            assert_eq!(diagnostics.len(), 1);
            assert!(matches!(&diagnostics[0].1, ExpressionError::UnknownFunction { name } if name == "no_such_fn"));
        }
    }

    #[test]
    fn unscoped_validation_is_unchanged() {
        let expr = parse_spanned("gone.y").unwrap();
        assert_eq!(validate_expression(&expr, &view()).len(), 1);
        assert_eq!(validate_expression_in_scope(&expr, &view(), &["gone"]).len(), 0);
    }
}
