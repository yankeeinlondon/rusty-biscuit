//! Frontmatter / schema diagnostics (the R-5 `dm.*` taxonomy).
//!
//! Ranging follows the R-5 convention: the validator identifies the semantic
//! failing node, but every range comes from the concrete syntax tree
//! ([`FrontmatterAst`]), never from message parsing or line maps. Missing-key
//! diagnostics range the parent mapping (a real visible range, not zero-width);
//! unknown-key diagnostics range the offending key; type/constraint/file
//! diagnostics range the value; `relatedInformation` points at the schema
//! origin. Pending `$(...)` values are reported for information and never
//! executed.

use darkmatter::markdown::Markdown;
use darkmatter::markdown::schemas::{
    PositionMap, SchemaError, SchemaOriginKind, ValidationOptions, ValidationProblem,
    ValidationProblemCode,
};
use darkmatter::style::{self, StyleWarningKind};
use lsp_types::{
    Diagnostic, DiagnosticRelatedInformation, DiagnosticSeverity, Location, NumberOrString, Range,
};

use crate::diagnostics::codes::{code, source};
use crate::overlay::{FrontmatterAst, SchemaBundle, SchemaOutcome};
use crate::providers::DocumentContext;
use crate::source_map::SourceMap;
use crate::workspace::file_path_to_uri;

/// All frontmatter/schema diagnostics for the current document.
pub fn diagnostics(ctx: &DocumentContext) -> Vec<Diagnostic> {
    let Some(overlay) = ctx.overlay else {
        return Vec::new();
    };
    let mut out = Vec::new();

    // A hard YAML parse error from the *current* buffer, ranged at the parser's
    // position (the last-good tree keeps completion/hover alive meanwhile).
    if let Some(error) = &overlay.parse_error {
        let range = ctx
            .source_map
            .byte_range_to_lsp(error.span.clone())
            .unwrap_or_else(zero_range);
        out.push(diagnostic(
            range,
            DiagnosticSeverity::ERROR,
            source::FRONTMATTER,
            code::FM_YAML_PARSE,
            error.message.clone(),
        ));
    }

    match &overlay.schema {
        SchemaOutcome::Failed(error) => {
            schema_prepare_diagnostic(ctx, overlay.ast.as_deref(), error, &mut out);
        }
        SchemaOutcome::Ready(Some(bundle)) => {
            if let Some(ast) = overlay.ast.as_deref() {
                schema_problem_diagnostics(ctx, ast, bundle, &mut out);
                style_diagnostics(ctx, ast, &mut out);
            }
        }
        SchemaOutcome::Ready(None) => {}
    }

    out
}

/// A schema resolution/preparation failure, ranged at `$schema` (or the block).
fn schema_prepare_diagnostic(
    ctx: &DocumentContext,
    ast: Option<&FrontmatterAst>,
    error: &SchemaError,
    out: &mut Vec<Diagnostic>,
) {
    let span = ast
        .and_then(|ast| ast.schema_entry().map(|entry| entry.value_span.clone()))
        .or_else(|| ast.map(FrontmatterAst::block_span));
    let range = span
        .and_then(|span| ctx.source_map.byte_range_to_lsp(span))
        .unwrap_or_else(zero_range);
    let (code_value, message) = match error {
        SchemaError::FrontmatterShape { message } => (code::SCHEMA_INVALID_SHAPE, message.clone()),
        other => (code::SCHEMA_PREPARE, other.to_string()),
    };
    out.push(diagnostic(range, DiagnosticSeverity::ERROR, source::SCHEMA, code_value, message));
}

/// Instance-validation problems mapped onto concrete ranges.
fn schema_problem_diagnostics(
    ctx: &DocumentContext,
    ast: &FrontmatterAst,
    bundle: &SchemaBundle,
    out: &mut Vec<Diagnostic>,
) {
    let report = bundle.effective.validate_with_options(
        &bundle.frontmatter_json,
        &PositionMap::new(),
        &ValidationOptions::default(),
    );

    for problem in &report.problems {
        let Some(range) = problem_range(ast, ctx.source_map, problem) else {
            continue;
        };
        let (code_value, severity) = classify(problem.code, ctx.config.schema.strict);
        let mut diagnostic = diagnostic(
            range,
            severity,
            source::SCHEMA,
            code_value,
            problem.message.clone(),
        );
        diagnostic.related_information = schema_origin_related(ctx, bundle, problem);
        out.push(diagnostic);
    }

    // Deferred-composition values, reported for information (never executed).
    for pending in &report.pending {
        let span = ast.value_range(&pending.path.as_pointer_string());
        let Some(range) = ctx.source_map.byte_range_to_lsp(span) else {
            continue;
        };
        out.push(diagnostic(
            range,
            DiagnosticSeverity::INFORMATION,
            source::SCHEMA,
            code::SCHEMA_PENDING_SHELL_VALUE,
            format!(
                "`{}` holds a deferred value; DMLS never executes it",
                pending.key
            ),
        ));
    }
}

/// The concrete range for one validation problem (R-5 ranging rules).
fn problem_range(ast: &FrontmatterAst, sm: &SourceMap, problem: &ValidationProblem) -> Option<Range> {
    let span = match problem.code {
        // A missing key has no value node; range the parent mapping so the
        // squiggle is visible.
        ValidationProblemCode::MissingRequired => ast.parent_mapping_range(&problem.path),
        // The offending key node, recovered from `offending_property` (the
        // failure's `path` points at the parent object).
        ValidationProblemCode::UnknownKey => problem
            .offending_property
            .as_deref()
            .and_then(|key| ast.key_span_for(&problem.path, key))
            .unwrap_or_else(|| ast.value_range(&problem.path)),
        // Type / constraint / file-reference failures range the value node.
        _ => ast.value_range(&problem.path),
    };
    sm.byte_range_to_lsp(span)
}

/// `relatedInformation` pointing at a problem's schema origin, when it is a
/// referenced schema file.
fn schema_origin_related(
    ctx: &DocumentContext,
    bundle: &SchemaBundle,
    problem: &ValidationProblem,
) -> Option<Vec<DiagnosticRelatedInformation>> {
    let key = problem
        .property
        .clone()
        .or_else(|| problem.instance_path.first().map(str::to_string))?;
    let origin = bundle.effective.origins.get(&key)?;
    if origin.kind != SchemaOriginKind::ReferencedFile {
        // Document / baseline origins have no separate file to point at; the
        // diagnostic range already sits on the failing frontmatter.
        let _ = ctx;
        return None;
    }
    let path = origin.uri.as_ref()?;
    let uri = file_path_to_uri(path)?;
    Some(vec![DiagnosticRelatedInformation {
        location: Location::new(uri, zero_range()),
        message: format!("`{key}` is declared by this schema"),
    }])
}

/// `style:` key warnings (unknown / deprecated), ranged by dotted path.
fn style_diagnostics(ctx: &DocumentContext, ast: &FrontmatterAst, out: &mut Vec<Diagnostic>) {
    let md: Markdown = ctx.text.into();
    let Ok((_, warnings)) = style::from_frontmatter(md.frontmatter()) else {
        // A structural style error is a shape problem the schema validator
        // already covers; the warnings path only carries key-level advice.
        return;
    };
    for warning in warnings {
        let Some(entry) = ast.entry_by_dotted(&warning.path) else {
            continue;
        };
        let Some(range) = ctx.source_map.byte_range_to_lsp(entry.key_span.clone()) else {
            continue;
        };
        let (code_value, message) = match &warning.kind {
            StyleWarningKind::UnknownKey => (
                code::STYLE_UNKNOWN_KEY,
                format!("unknown style key `{}`", warning.path),
            ),
            StyleWarningKind::Deprecated { replacement } => (
                code::STYLE_DEPRECATED_KEY,
                format!("deprecated style key `{}`; use `{replacement}`", warning.path),
            ),
            // A key valid in a future wiring phase is neither unknown nor an
            // error — do not flag it.
            StyleWarningKind::KnownButInactive { .. } => continue,
        };
        out.push(diagnostic(
            range,
            DiagnosticSeverity::WARNING,
            source::STYLE,
            code_value,
            message,
        ));
    }
}

/// The `(code, severity)` for a validation-problem category.
fn classify(code: ValidationProblemCode, strict: bool) -> (&'static str, DiagnosticSeverity) {
    match code {
        ValidationProblemCode::MissingRequired => {
            (code::SCHEMA_MISSING_REQUIRED, DiagnosticSeverity::ERROR)
        }
        ValidationProblemCode::TypeMismatch => {
            (code::SCHEMA_TYPE_MISMATCH, DiagnosticSeverity::ERROR)
        }
        ValidationProblemCode::ConstraintViolation => {
            (code::SCHEMA_CONSTRAINT, DiagnosticSeverity::ERROR)
        }
        ValidationProblemCode::UnknownKey => (
            code::SCHEMA_UNKNOWN_KEY,
            if strict {
                DiagnosticSeverity::ERROR
            } else {
                DiagnosticSeverity::WARNING
            },
        ),
        ValidationProblemCode::InvalidFileReference => {
            (code::SCHEMA_INVALID_FILE_REFERENCE, DiagnosticSeverity::ERROR)
        }
    }
}

/// Builds a diagnostic with a stable source + code.
fn diagnostic(
    range: Range,
    severity: DiagnosticSeverity,
    source_value: &str,
    code_value: &str,
    message: String,
) -> Diagnostic {
    Diagnostic {
        range,
        severity: Some(severity),
        code: Some(NumberOrString::String(code_value.to_string())),
        source: Some(source_value.to_string()),
        message,
        ..Default::default()
    }
}

/// A zero-width range at the document start (fallback when a span cannot
/// convert).
fn zero_range() -> Range {
    Range::default()
}
