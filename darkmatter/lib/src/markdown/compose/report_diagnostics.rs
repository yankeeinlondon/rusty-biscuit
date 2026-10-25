//! A composition's warnings and its failure as shared [`Diagnostic`]
//! records: what `md compose --output report-json` reports.
//!
//! Every record keeps the code, severity, and message its producer gave it.
//! A warning that carries a shared record (a call finding) is reported as
//! that record; any other warning, and every failure, is classified by its
//! typed error into a code of the same `dm.<family>.<kind>` taxonomy DMLS
//! reports, with `source` and the types left `None` where the error holds no
//! authored position. Classification reads only the error and warning
//! values: it evaluates nothing and reads no file.
//!
//! See `darkmatter/docs/cli/report-json.md` for the report shape.

use std::error::Error;

use super::ShellExpansionError;
use super::TransclusionError;
use super::conditions::ConditionError;
use super::context::report::ComposeWarning;
use super::expression::{Diagnostic, DiagnosticSource, ExpressionError, Severity};
use super::page_blocks::PageBlockError;
use crate::markdown::reference::validate::{ReferenceIssue, ReferenceIssueCode, ReferenceSeverity};
use crate::markdown::types::{MarkdownError, SourceRef};

/// The frontmatter YAML could not be parsed, or its fence is malformed.
pub const FRONTMATTER_PARSE_CODE: &str = "dm.frontmatter.yaml_parse";
/// An expression calls a function the catalog does not have.
pub const UNKNOWN_FUNCTION_CODE: &str = "dm.expression.unknown_function";
/// The input document, or a file it needs, could not be read.
pub const INPUT_READ_CODE: &str = "dm.input.read_failure";
/// A directive could not be parsed.
pub const DIRECTIVE_PARSE_CODE: &str = "dm.directive.parse_failure";
/// A shell command failed, timed out, or was refused.
pub const SHELL_FAILURE_CODE: &str = "dm.shell.failure";
/// A transclusion failed for a reason other than its content's own error.
pub const TRANSCLUSION_FAILURE_CODE: &str = "dm.transclusion.failure";
/// A file reference failed to resolve.
pub const REFERENCE_FAILURE_CODE: &str = "dm.reference.failure";
/// A caller-supplied value did not bind to its parameter.
pub const PARAMETER_BINDING_CODE: &str = "dm.compose.parameter_binding";
/// A lifecycle event failed.
pub const LIFECYCLE_FAILURE_CODE: &str = "dm.lifecycle.failure";
/// A composition failure no narrower code classifies.
pub const COMPOSE_FAILURE_CODE: &str = "dm.compose.failure";
/// A composition warning whose producer declares no code.
pub const UNCODED_WARNING_CODE: &str = "dm.compose.warning";

/// `warning` as a shared record: its own record when it carries one,
/// otherwise its code (or [`UNCODED_WARNING_CODE`]) and message, with no
/// source or types.
pub fn warning_diagnostic(warning: &ComposeWarning) -> Diagnostic {
    if let Some(diagnostic) = warning.diagnostic.as_deref() {
        return diagnostic.clone();
    }
    unlocated(
        warning.code.as_deref().unwrap_or(UNCODED_WARNING_CODE),
        Severity::Warning,
        warning.message.clone(),
    )
}

/// A reported reference-validation `issue` as a shared record. A missing
/// local target takes its kind's recovery code
/// ([`missing_reference_code`](super::policy::missing_reference_code)).
///
/// The validator records an issue's line but not its column, so `source` is
/// `None`; the message names the target.
pub fn reference_issue_diagnostic(issue: &ReferenceIssue) -> Diagnostic {
    let code = match issue.code {
        ReferenceIssueCode::MissingLocalTarget => {
            super::policy::missing_reference_code(issue.kind).unwrap_or(REFERENCE_FAILURE_CODE)
        }
        _ => REFERENCE_FAILURE_CODE,
    };
    let severity = match issue.severity {
        ReferenceSeverity::Error => Severity::Error,
        ReferenceSeverity::Warning => Severity::Warning,
        ReferenceSeverity::Info => Severity::Info,
    };
    unlocated(code, severity, issue.message.clone())
}

/// The diagnostics a failed composition reports for `error`, found anywhere
/// in its source chain.
///
/// - A policy stop reports the warnings before the stop, then the blocking
///   ones, each with its own severity.
/// - A rejected call reports its located call record.
/// - A schema failure reports one record per problem.
/// - Any other Darkmatter error reports one record classified by its
///   innermost typed error; a foreign error reports [`INPUT_READ_CODE`] for an
///   I/O error and [`COMPOSE_FAILURE_CODE`] otherwise.
pub fn error_diagnostics(error: &(dyn Error + 'static)) -> Vec<Diagnostic> {
    let mut current = Some(error);
    while let Some(error) = current {
        if let Some(markdown) = as_markdown_error(error) {
            return markdown_error_diagnostics(markdown);
        }
        if let Some(shell) = as_shell_error(error) {
            return shell_error_diagnostics(shell);
        }
        if let Some(expression) = error
            .downcast_ref::<ExpressionError>()
            .or_else(|| error.downcast_ref::<Box<ExpressionError>>().map(AsRef::as_ref))
        {
            return vec![unlocated(expression_code(expression), Severity::Error, expression.to_string())];
        }
        current = error.source();
    }
    let mut current = Some(error);
    while let Some(cause) = current {
        if cause.downcast_ref::<std::io::Error>().is_some() {
            return vec![unlocated(INPUT_READ_CODE, Severity::Error, error.to_string())];
        }
        current = cause.source();
    }
    vec![unlocated(COMPOSE_FAILURE_CODE, Severity::Error, error.to_string())]
}

/// The diagnostics of a failed lifecycle event: its Darkmatter cause's
/// ([`error_diagnostics`]) when it has one, otherwise one
/// [`LIFECYCLE_FAILURE_CODE`] record with the failure's message.
pub fn lifecycle_failure_diagnostics(failure: &crate::lifecycle::LifecycleErrorInfo) -> Vec<Diagnostic> {
    match &failure.cause {
        Some(cause) => error_diagnostics(cause),
        None => vec![unlocated(LIFECYCLE_FAILURE_CODE, Severity::Error, failure.msg.clone())],
    }
}

fn markdown_error_diagnostics(error: &MarkdownError) -> Vec<Diagnostic> {
    if let Some((blocking, earlier)) = blocked(error) {
        return earlier.iter().chain(blocking).map(warning_diagnostic).collect();
    }
    if let Some(diagnostic) = error.call_diagnostic() {
        return vec![diagnostic];
    }
    let leaf = innermost(error);
    match leaf {
        MarkdownError::SchemaValidationFailed { path, problems, .. } if !problems.is_empty() => problems
            .iter()
            .map(|problem| Diagnostic {
                code: problem.code.diagnostic_code().to_string(),
                severity: Severity::Error,
                message: problem.message.clone(),
                source: problem.line.map(|line| DiagnosticSource {
                    file: Some(path.clone()),
                    line: line as usize,
                    column: problem.column.map_or(1, |column| column as usize),
                }),
                expected_type: None,
                actual_type: None,
            })
            .collect(),
        MarkdownError::ShellExpansion(shell) => shell_error_diagnostics(shell),
        _ => vec![leaf_diagnostic(leaf)],
    }
}

fn leaf_diagnostic(error: &MarkdownError) -> Diagnostic {
    let message = error.to_string();
    let (code, source) = match error {
        MarkdownError::FrontmatterParse { ctx, source } => (
            FRONTMATTER_PARSE_CODE,
            source.location().map(|location| DiagnosticSource {
                file: named(&ctx.display),
                line: frontmatter_file_line(ctx, location.line()),
                column: location.column().max(1),
            }),
        ),
        MarkdownError::FrontmatterFenceMismatch { ctx, line, .. } => (
            FRONTMATTER_PARSE_CODE,
            Some(DiagnosticSource { file: named(&ctx.display), line: *line, column: 1 }),
        ),
        MarkdownError::SchemaValidationFailed { .. } => (crate::markdown::schemas::SCHEMA_PREPARE_CODE, None),
        MarkdownError::FileLoad(_) | MarkdownError::UrlFetch(_) => (INPUT_READ_CODE, None),
        MarkdownError::Interpolation { source, cause, .. } => (
            expression_code(cause),
            match source.as_ref() {
                SourceRef::OnDiskSpan { context, span } => Some(DiagnosticSource {
                    file: named(&context.display),
                    line: span.line(),
                    column: span.column(),
                }),
                _ => None,
            },
        ),
        MarkdownError::Transclusion(inner) => (
            match inner.as_ref() {
                TransclusionError::ParseDirective { .. } => DIRECTIVE_PARSE_CODE,
                TransclusionError::ConditionEval { .. } => ComposeWarning::EXPRESSION_EVALUATION_FAILURE_CODE,
                _ => TRANSCLUSION_FAILURE_CODE,
            },
            None,
        ),
        MarkdownError::PageBlock(inner) => (
            match inner.as_ref() {
                PageBlockError::Condition(condition) => match condition.as_ref() {
                    ConditionError::Parse { .. } => ComposeWarning::EXPRESSION_PARSE_FAILURE_CODE,
                    ConditionError::Eval { .. } => ComposeWarning::EXPRESSION_EVALUATION_FAILURE_CODE,
                },
                _ => DIRECTIVE_PARSE_CODE,
            },
            None,
        ),
        MarkdownError::ShellBlock(_) => (SHELL_FAILURE_CODE, None),
        MarkdownError::Reference(_) => (REFERENCE_FAILURE_CODE, None),
        MarkdownError::ParameterBinding { .. } | MarkdownError::CallerFileClassificationChanged { .. } => {
            (PARAMETER_BINDING_CODE, None)
        }
        _ => (COMPOSE_FAILURE_CODE, None),
    };
    Diagnostic { code: code.to_string(), severity: Severity::Error, message, source, expected_type: None, actual_type: None }
}

fn shell_error_diagnostics(error: &ShellExpansionError) -> Vec<Diagnostic> {
    match error {
        ShellExpansionError::Preflight(inner) => markdown_error_diagnostics(inner),
        ShellExpansionError::ParseDirective { .. } => vec![unlocated(DIRECTIVE_PARSE_CODE, Severity::Error, error.to_string())],
        _ => vec![unlocated(SHELL_FAILURE_CODE, Severity::Error, error.to_string())],
    }
}

fn expression_code(cause: &ExpressionError) -> &'static str {
    match cause {
        ExpressionError::Parse(_) | ExpressionError::MalformedLiteralToken(_) => {
            ComposeWarning::EXPRESSION_PARSE_FAILURE_CODE
        }
        ExpressionError::UnknownFunction { .. } => UNKNOWN_FUNCTION_CODE,
        _ => ComposeWarning::EXPRESSION_EVALUATION_FAILURE_CODE,
    }
}

/// The policy stop in `error`'s chain: its blocking and earlier warnings.
fn blocked(error: &MarkdownError) -> Option<(&[ComposeWarning], &[ComposeWarning])> {
    let mut current: Option<&(dyn Error + 'static)> = Some(error);
    while let Some(error) = current {
        if let Some(MarkdownError::BlockedDiagnostics { blocking, earlier }) = as_markdown_error(error) {
            return Some((blocking, earlier));
        }
        current = error.source();
    }
    None
}

/// The deepest Darkmatter error `error` wraps (a transcluded child's), or
/// `error` itself.
fn innermost(error: &MarkdownError) -> &MarkdownError {
    let mut leaf = error;
    let mut current = error.source();
    while let Some(cause) = current {
        if let Some(markdown) = as_markdown_error(cause) {
            leaf = markdown;
        }
        current = cause.source();
    }
    leaf
}

fn as_markdown_error<'e>(error: &'e (dyn Error + 'static)) -> Option<&'e MarkdownError> {
    error
        .downcast_ref::<MarkdownError>()
        .or_else(|| error.downcast_ref::<Box<MarkdownError>>().map(AsRef::as_ref))
}

fn as_shell_error<'e>(error: &'e (dyn Error + 'static)) -> Option<&'e ShellExpansionError> {
    error
        .downcast_ref::<ShellExpansionError>()
        .or_else(|| error.downcast_ref::<Box<ShellExpansionError>>().map(AsRef::as_ref))
}

/// The file line of YAML line `yaml_line` (one-based) in the frontmatter of
/// `ctx`.
fn frontmatter_file_line(ctx: &biscuit_terminal::errors::SourceContext, yaml_line: usize) -> usize {
    match ctx.frontmatter.as_ref() {
        Some(range) => ctx.content[..range.start].matches('\n').count() + 1 + yaml_line,
        None => yaml_line,
    }
}

/// A context's display path, or `None` for input with no named origin.
fn named(display: &std::path::Path) -> Option<std::path::PathBuf> {
    (display != std::path::Path::new("unknown")).then(|| display.to_path_buf())
}

fn unlocated(code: &str, severity: Severity, message: String) -> Diagnostic {
    Diagnostic { code: code.to_string(), severity, message, source: None, expected_type: None, actual_type: None }
}
