//! Typed evaluation and rewriting for interpolated transclusion targets.

use crate::markdown::compose::ComposeWarning;
use crate::markdown::compose::site_scope::SiteHook;
use crate::markdown::compose::body_origin::{DataRanges, TextEdit};
use crate::markdown::compose::directives_api::{
    BlockScanError, DirectiveKind, scan_darkmatter_blocks, scan_darkmatter_directives,
    scan_darkmatter_directives_in,
};
use crate::markdown::compose::expression::{
    BindingId, DeclaredType, DeclaredTypes, EvaluationLookup, Expr, Member, Subject, TypeScope, parse,
    scalar_string,
};
use crate::markdown::compose::collection_blocks::BodyScopes;
use crate::markdown::compose::pipeline::operations::ComposeOperationSet;
use crate::markdown::compose::expression::absence;
use crate::markdown::compose::expression::lint::whole_value_span;
use crate::markdown::compose::interpolation::rewrite::{BeforeEval, interpolate_value_guarded, reborrow, show_new_warnings};
use crate::markdown::compose::interpolation::{
    Evaluator, ExpressionFailurePolicy, LocatedInterpolationError,
};
use crate::markdown::schemas::EffectiveSchema;
use crate::markdown::span::SourceSpan;

/// A normalized property path used by passive target and guard analysis.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ExpressionPath(String);

impl ExpressionPath {
    /// Returns the normalized dotted path.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Whether a parsed whole-value target can be null without evaluating it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TargetNullability {
    /// The available static authority proves the target is present.
    NonNullable,
    /// The target may be null at this use site.
    Nullable {
        /// The nullable property path.
        root: ExpressionPath,
    },
    /// Static authority is absent or the expression/schema shape is unsupported.
    Unknown,
}

/// Passive analysis of one whole-value transclusion target.
#[derive(Debug, Clone, PartialEq)]
pub struct DirectiveTargetAnalysis {
    /// The directive family owning the target.
    pub kind: DirectiveKind,
    /// Byte span of the authored target token.
    pub target_span: SourceSpan,
    /// Byte span of the complete `{{ ... }}` expression.
    pub expression_span: SourceSpan,
    /// The parsed expression, without evaluating it.
    pub expression: Expr,
    /// Normalized property root when the expression is a supported path.
    pub root: Option<ExpressionPath>,
    /// Schema-, descriptor-, and frontmatter-derived nullability, before any
    /// guard.
    pub nullability: TargetNullability,
    /// The paths the enclosing page-block guards prove present at the target,
    /// sorted.
    pub narrowed_paths: Vec<ExpressionPath>,
}

impl DirectiveTargetAnalysis {
    /// Whether the enclosing page-block guards prove this target's root
    /// present.
    pub fn is_narrowed(&self) -> bool {
        self.root
            .as_ref()
            .is_some_and(|root| self.narrowed_paths.binary_search(root).is_ok())
    }
}

/// Classifies a parsed target expression from already-assembled static state.
///
/// The target's type is read from an unguarded [`TypeScope`] over the
/// document's authored schema projection (`EffectiveSchema.simplified`) and
/// `static_frontmatter`: a top-level schema property, a cataloged `ctx`
/// member (by its descriptor), or a concrete static scalar. Anything else,
/// including a nested path and a property no authored schema declares, is
/// [`TargetNullability::Unknown`].
pub fn classify_target_nullability(
    expression: &Expr,
    effective_schema: Option<&EffectiveSchema>,
    static_frontmatter: &serde_json::Value,
) -> TargetNullability {
    let declared = declared_types(effective_schema, static_frontmatter);
    target_nullability(&TypeScope::unguarded(&declared), expression)
}

/// The declared layer of every layer of `effective_schema`, and
/// `static_frontmatter`.
fn declared_types<'s>(
    effective_schema: Option<&'s EffectiveSchema>,
    static_frontmatter: &'s serde_json::Value,
) -> DeclaredTypes<'s> {
    DeclaredTypes::for_schema(effective_schema).with_static_frontmatter(static_frontmatter)
}

/// The nullability of target `expression` at a site whose types are `scope`.
fn target_nullability(scope: &TypeScope<'_>, expression: &Expr) -> TargetNullability {
    let Some(root) = expression_path(expression) else {
        return TargetNullability::Unknown;
    };
    let Some(target) = scope.type_of_expr(expression) else {
        return TargetNullability::Unknown;
    };
    match target.declared() {
        _ if target.proves_present() => TargetNullability::NonNullable,
        DeclaredType::Property(_) | DeclaredType::Context { .. } => TargetNullability::Nullable { root },
        _ => TargetNullability::Unknown,
    }
}

/// Passively analyzes whole-value targets and all enclosing guards and
/// collection regions.
///
/// Each target's site is typed by [`BodyScopes`]: on the true side of every
/// `::block when=` and `::map` / `::filter when=` gate around it, and inside
/// every enclosing region's body frame, the facts composition can rely on
/// once those regions were selected. A target in a collection body may read
/// the region's locals; they never narrow a document property.
/// [`DirectiveTargetAnalysis::narrowed_paths`] lists the document paths those
/// guards prove present there.
///
/// This function parses only in-memory source and supplied schema/frontmatter
/// state. It does not evaluate expressions, resolve paths, fetch URLs, execute
/// processes, or mutate its inputs.
pub fn analyze_directive_targets(
    source: &str,
    effective_schema: Option<&EffectiveSchema>,
    static_frontmatter: &serde_json::Value,
) -> Result<Vec<DirectiveTargetAnalysis>, BlockScanError> {
    let directives = scan_darkmatter_directives(source);
    // Unpaired block markers are this analysis's error; `BodyScopes` would
    // only drop their facts.
    scan_darkmatter_blocks(source)?;
    let guards = BodyScopes::scan(source, &ComposeOperationSet::all());
    let declared = declared_types(effective_schema, static_frontmatter);

    let mut analyses = Vec::new();
    for directive in directives {
        if !matches!(directive.kind, DirectiveKind::File | DirectiveKind::Code | DirectiveKind::Url)
        {
            continue;
        }
        let Some(target) = directive.target else {
            continue;
        };
        let raw = &source[target.span.clone()];
        let Some(location) = whole_value_span(raw) else {
            continue;
        };
        let Ok(expression) = parse(&location.expression) else {
            continue;
        };
        let root = expression_path(&expression);
        let nullability = target_nullability(&TypeScope::unguarded(&declared), &expression);
        let site = guards.scope_at(TypeScope::unguarded(&declared), target.span.start);
        let mut narrowed: Vec<ExpressionPath> =
            site.guarded_subjects().iter().filter_map(subject_path).collect();
        narrowed.sort();
        narrowed.dedup();
        analyses.push(DirectiveTargetAnalysis {
            kind: directive.kind,
            target_span: target.span.clone(),
            expression_span: target.span.start + location.start
                ..target.span.start + location.end,
            expression,
            root,
            nullability,
            narrowed_paths: narrowed,
        });
    }
    Ok(analyses)
}

/// The dotted path of `subject`, as [`expression_path`] names a target root:
/// a document property without its `fm.` prefix, a namespace member with its
/// root. A subject with an index step has no dotted path.
fn subject_path(subject: &Subject) -> Option<ExpressionPath> {
    let mut path = match subject.binding() {
        BindingId::Document(key) => key.clone(),
        BindingId::Namespace(root) | BindingId::Global(root) => root.clone(),
        BindingId::Local(_) => return None,
    };
    for member in subject.path() {
        let Member::Field(name) = member else {
            return None;
        };
        path.push('.');
        path.push_str(name);
    }
    Some(ExpressionPath(path))
}

fn expression_path(expression: &Expr) -> Option<ExpressionPath> {
    let path = match expression {
        Expr::Variable(path) => path.as_str(),
        Expr::Paren(inner) => return expression_path(inner),
        _ => return None,
    };
    if let Some(crate::markdown::compose::expression::FmPath::Member(document_path)) =
        crate::markdown::compose::expression::fm_namespace_path(path)
    {
        if document_path.is_empty() || document_path.contains('.') {
            return None;
        }
        return Some(ExpressionPath(document_path.to_string()));
    }
    if path.is_empty() || absence::is_reserved_root(path) {
        return None;
    }
    Some(ExpressionPath(path.to_string()))
}

/// Why a successfully evaluated directive target has no target value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AbsentTargetReason {
    /// The expression evaluated to JSON `null`.
    Null,
    /// The expression evaluated to an empty string.
    EmptyString,
}

impl AbsentTargetReason {
    fn description(self) -> &'static str {
        match self {
            Self::Null => "null",
            Self::EmptyString => "an empty string",
        }
    }
}

/// The typed result of evaluating one whole-value directive target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvaluatedDirectiveTarget {
    /// A target that remains eligible for ordinary directive parsing.
    Concrete(String),
    /// A target whose value intentionally denotes absence.
    Absent {
        /// The sentinel produced by evaluation.
        reason: AbsentTargetReason,
        /// The referenced expression path when the expression is a property path.
        root: Option<String>,
    },
}

/// One evaluated target with its authored directive identity and source span.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectiveTargetEvaluation {
    /// The directive family owning the target.
    pub kind: DirectiveKind,
    /// Byte span of the authored target in the scanned body.
    pub span: SourceSpan,
    /// The typed evaluation result.
    pub target: EvaluatedDirectiveTarget,
}

pub(crate) struct DirectiveTargetRewrite {
    pub output: String,
    pub replacements: usize,
    pub warnings: Vec<ComposeWarning>,
    /// The edits that turned `source` into `output`, in source order.
    pub edits: Vec<TextEdit>,
}

/// Evaluates whole-value `::file`, `::code`, and `::url` targets before the
/// ordinary body interpolation pass consumes their typed values.
///
/// A target is part of a full document, so every expression failure is fatal
/// and located at the target's span in `source`.
///
/// Only authored directives are considered, and only a target that lies wholly
/// in authored text is evaluated; its value is data.
///
/// `before_eval` is checked before each target is evaluated and after the
/// last (see [`BeforeEval`]).
pub(crate) fn rewrite_directive_targets<L: EvaluationLookup>(
    source: &str,
    data: Option<&DataRanges>,
    evaluator: &Evaluator<L>,
    line_offset: usize,
    mut before_eval: BeforeEval<'_>,
    site: Option<&dyn SiteHook>,
) -> Result<DirectiveTargetRewrite, LocatedInterpolationError> {
    let mut output = source.to_string();
    let mut replacements = 0usize;
    let mut warnings = Vec::new();
    let mut shown = 0;
    let mut edits = Vec::new();

    for directive in scan_darkmatter_directives_in(source, data).into_iter().rev() {
        if !matches!(directive.kind, DirectiveKind::File | DirectiveKind::Code | DirectiveKind::Url)
        {
            continue;
        }
        let Some(target) = directive.target else {
            continue;
        };
        if data.is_some_and(|data| data.intersects(&target.span)) {
            continue;
        }
        let raw = &source[target.span.clone()];
        let Some(location) = whole_value_span(raw) else {
            continue;
        };

        show_new_warnings(&mut before_eval, &warnings, &mut shown)?;
        if let Some(site) = site {
            site.enter(target.span.start);
        }
        let expression = parse(&location.expression).ok();
        let (value, count, target_warnings) = interpolate_value_guarded(
            raw,
            evaluator,
            ExpressionFailurePolicy::Strict,
            "interpolation",
            reborrow(&mut before_eval),
        )
        .map_err(|failure| LocatedInterpolationError {
            // A policy stop is about no one target.
            span: (!matches!(*failure.error, crate::markdown::types::MarkdownError::BlockedDiagnostics { .. }))
                .then(|| target.span.clone()),
            error: failure.error,
        })?;
        replacements += count;
        warnings.extend(target_warnings);
        shown = warnings.len();

        let evaluated_target = match value {
            serde_json::Value::Null => EvaluatedDirectiveTarget::Absent {
                reason: AbsentTargetReason::Null,
                root: expression.as_ref().and_then(expression_root),
            },
            serde_json::Value::String(ref value) if value.is_empty() => {
                EvaluatedDirectiveTarget::Absent {
                    reason: AbsentTargetReason::EmptyString,
                    root: expression.as_ref().and_then(expression_root),
                }
            }
            value => EvaluatedDirectiveTarget::Concrete(scalar_string(&value)),
        };
        let evaluation = DirectiveTargetEvaluation {
            kind: directive.kind,
            span: target.span.clone(),
            target: evaluated_target,
        };

        match &evaluation.target {
            EvaluatedDirectiveTarget::Concrete(value) => {
                crate::markdown::compose::interpolation::rewrite::charge(
                    site,
                    evaluation.span.start,
                    value.len(),
                    Some(evaluation.span.clone()),
                )?;
                output.replace_range(evaluation.span.clone(), value);
                edits.push(TextEdit::data(evaluation.span.clone(), value.len()));
            }
            EvaluatedDirectiveTarget::Absent { reason, root } => {
                let expression_name = root.as_deref().unwrap_or(location.expression.trim());
                warnings.push(
                    ComposeWarning::new(
                        "interpolation",
                        format!(
                            "nullable target `{expression_name}` for {} evaluated to {}; directive skipped",
                            evaluation.kind.keyword(),
                            reason.description(),
                        ),
                    )
                    .at_line(directive.line + line_offset),
                );
                let line = directive.span.start..line_end_including_terminator(source, directive.span.end);
                output.replace_range(line.clone(), "");
                edits.push(TextEdit::authored(line, 0));
            }
        }
    }
    show_new_warnings(&mut before_eval, &warnings, &mut shown)?;
    // Directives were rewritten end to start.
    edits.reverse();

    Ok(DirectiveTargetRewrite {
        output,
        replacements,
        warnings,
        edits,
    })
}

fn expression_root(expression: &Expr) -> Option<String> {
    match expression {
        Expr::Variable(path) => Some(path.clone()),
        Expr::Paren(inner) => expression_root(inner),
        _ => None,
    }
}

fn line_end_including_terminator(source: &str, line_end: usize) -> usize {
    match source.as_bytes().get(line_end..) {
        Some([b'\r', b'\n', ..]) => line_end + 2,
        Some([b'\n', ..]) => line_end + 1,
        _ => line_end,
    }
}
