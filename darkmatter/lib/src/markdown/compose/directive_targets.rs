//! Typed evaluation and rewriting for interpolated transclusion targets.

use std::collections::BTreeSet;

use crate::markdown::compose::ComposeWarning;
use crate::markdown::compose::body_origin::TextEdit;
use crate::markdown::compose::context::context_variable_descriptors;
use crate::markdown::compose::directives_api::{
    BlockKind, BlockScanError, DirectiveKind, scan_darkmatter_blocks,
    scan_darkmatter_directives,
};
use crate::markdown::compose::expression::{
    ComparisonOp, EvaluationLookup, Expr, parse, parse_condition, scalar_string,
};
use crate::markdown::compose::interpolation::rewrite::{interpolate_value, whole_value_span};
use crate::markdown::compose::interpolation::{
    Evaluator, ExpressionFailurePolicy, LocatedInterpolationError,
};
use crate::markdown::schemas::{EffectiveSchema, SchemaOriginKind, SimplifiedSchema};
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
    /// Schema- and frontmatter-derived nullability.
    pub nullability: TargetNullability,
    /// Paths narrowed by every enclosing page block.
    pub narrowed_paths: Vec<ExpressionPath>,
}

impl DirectiveTargetAnalysis {
    /// Whether an enclosing page-block condition narrows this target's root.
    pub fn is_narrowed(&self) -> bool {
        self.root
            .as_ref()
            .is_some_and(|root| self.narrowed_paths.binary_search(root).is_ok())
    }
}

/// Classifies a parsed target expression from already-assembled static state.
///
/// The classifier is intentionally conservative: document properties must be
/// top-level SimplifiedSchema declarations owned by the document or a
/// referenced schema file. Unsupported authority returns [`TargetNullability::Unknown`].
pub fn classify_target_nullability(
    expression: &Expr,
    effective_schema: Option<&EffectiveSchema>,
    static_frontmatter: &serde_json::Value,
) -> TargetNullability {
    let Some(root) = expression_path(expression) else {
        return TargetNullability::Unknown;
    };

    if let Some(name) = root.as_str().strip_prefix("ctx.") {
        if name.contains('.') {
            return TargetNullability::Unknown;
        }
        let Some(descriptor) = context_variable_descriptors()
            .iter()
            .find(|descriptor| descriptor.name == name)
        else {
            return TargetNullability::Unknown;
        };
        return if descriptor.required
            || descriptor.default.as_ref().is_some_and(|value| !value.is_null())
        {
            TargetNullability::NonNullable
        } else {
            TargetNullability::Nullable { root }
        };
    }

    if root.as_str().contains('.') {
        return TargetNullability::Unknown;
    }
    if static_frontmatter
        .as_object()
        .and_then(|frontmatter| frontmatter.get(root.as_str()))
        .is_some_and(is_concrete_scalar)
    {
        return TargetNullability::NonNullable;
    }

    let Some(effective_schema) = effective_schema else {
        return TargetNullability::Unknown;
    };
    let Some(SimplifiedSchema::Single(shape)) = effective_schema.simplified.as_ref() else {
        return TargetNullability::Unknown;
    };
    let Some(origin) = effective_schema.origins.get(root.as_str()) else {
        return TargetNullability::Unknown;
    };
    if !matches!(origin.kind, SchemaOriginKind::Document | SchemaOriginKind::ReferencedFile) {
        return TargetNullability::Unknown;
    }
    let Some(property) = shape.properties.get(root.as_str()) else {
        return TargetNullability::Unknown;
    };

    if property.is_required() {
        TargetNullability::NonNullable
    } else {
        TargetNullability::Nullable { root }
    }
}

/// Returns property paths proven present when a parsed condition is true.
pub fn narrowed_expression_paths(condition: &Expr) -> Vec<ExpressionPath> {
    let mut paths = BTreeSet::new();
    collect_narrowed_paths(condition, &mut paths);
    paths.into_iter().collect()
}

/// Passively analyzes whole-value targets and all enclosing page-block guards.
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
    let blocks = scan_darkmatter_blocks(source)?;
    let page_guards: Vec<_> = blocks
        .iter()
        .filter(|block| block.kind == BlockKind::Page)
        .filter_map(|block| {
            let opener = directives.iter().find(|directive| {
                directive.kind == DirectiveKind::Block && directive.line == block.start_line
            })?;
            let condition = opener
                .options
                .iter()
                .find(|option| option.key.value == "when")?
                .value
                .as_ref()?;
            let parsed = parse_condition(&condition.value).ok()?;
            Some((block.body_span.clone(), narrowed_expression_paths(&parsed)))
        })
        .collect();

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
        let nullability = classify_target_nullability(
            &expression,
            effective_schema,
            static_frontmatter,
        );
        let mut narrowed = BTreeSet::new();
        for (body_span, paths) in &page_guards {
            if body_span.start <= target.span.start && target.span.end <= body_span.end {
                narrowed.extend(paths.iter().cloned());
            }
        }
        analyses.push(DirectiveTargetAnalysis {
            kind: directive.kind,
            target_span: target.span.clone(),
            expression_span: target.span.start + location.start
                ..target.span.start + location.end,
            expression,
            root,
            nullability,
            narrowed_paths: narrowed.into_iter().collect(),
        });
    }
    Ok(analyses)
}

fn is_concrete_scalar(value: &serde_json::Value) -> bool {
    matches!(
        value,
        serde_json::Value::Bool(_)
            | serde_json::Value::Number(_)
            | serde_json::Value::String(_)
    )
}

fn expression_path(expression: &Expr) -> Option<ExpressionPath> {
    let path = match expression {
        Expr::Variable(path) => path.as_str(),
        Expr::Paren(inner) => return expression_path(inner),
        _ => return None,
    };
    if let Some(document_path) = path.strip_prefix("doc.") {
        if document_path.is_empty() || document_path.contains('.') {
            return None;
        }
        return Some(ExpressionPath(document_path.to_string()));
    }
    if path.is_empty() || matches!(path, "null" | "doc" | "ctx" | "env") {
        return None;
    }
    Some(ExpressionPath(path.to_string()))
}

fn collect_narrowed_paths(expression: &Expr, paths: &mut BTreeSet<ExpressionPath>) {
    match expression {
        Expr::Paren(inner) => collect_narrowed_paths(inner, paths),
        Expr::Variable(_) => {
            if let Some(path) = expression_path(expression) {
                paths.insert(path);
            }
        }
        Expr::UnaryNot(inner) => {
            if let Expr::UnaryNot(candidate) = inner.as_ref()
                && let Some(path) = expression_path(candidate)
            {
                paths.insert(path);
            }
        }
        Expr::Comparison {
            left,
            op: ComparisonOp::NotEqual,
            right,
        } if is_absence_comparison_value(right) => {
            if let Some(path) = expression_path(left) {
                paths.insert(path);
            }
        }
        Expr::FunctionCall { name, args } if name.eq_ignore_ascii_case("file_exists") => {
            if let [candidate] = args.as_slice()
                && let Some(path) = expression_path(candidate)
            {
                paths.insert(path);
            }
        }
        Expr::FunctionCall { name, args } if name.eq_ignore_ascii_case("and") => {
            for arg in args {
                collect_narrowed_paths(arg, paths);
            }
        }
        _ => {}
    }
}

fn is_absence_comparison_value(expression: &Expr) -> bool {
    match expression {
        Expr::Paren(inner) => is_absence_comparison_value(inner),
        Expr::Variable(name) => name == "null",
        Expr::StringLiteral(value) => value.is_empty(),
        _ => false,
    }
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
pub(crate) fn rewrite_directive_targets<L: EvaluationLookup>(
    source: &str,
    evaluator: &Evaluator<L>,
    line_offset: usize,
) -> Result<DirectiveTargetRewrite, LocatedInterpolationError> {
    let mut output = source.to_string();
    let mut replacements = 0usize;
    let mut warnings = Vec::new();
    let mut edits = Vec::new();

    for directive in scan_darkmatter_directives(source).into_iter().rev() {
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

        let expression = parse(&location.expression).ok();
        let (value, count, target_warnings) = interpolate_value(
            raw,
            evaluator,
            ExpressionFailurePolicy::Strict,
            "interpolation",
        )
        .map_err(|error| LocatedInterpolationError {
            error: Box::new(error),
            span: Some(target.span.clone()),
        })?;
        replacements += count;
        warnings.extend(target_warnings);

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
                output.replace_range(evaluation.span.clone(), value);
                edits.push(TextEdit { range: evaluation.span.clone(), replacement_len: value.len() });
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
                edits.push(TextEdit { range: line, replacement_len: 0 });
            }
        }
    }
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
