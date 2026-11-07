//! A composition's warnings and its failure as shared [`Diagnostic`]
//! records: what `md compose`'s data outputs report.
//!
//! Every record keeps the code, severity, and message its producer gave it.
//! A warning that carries a shared record (a call finding or a skipped
//! schema trigger) is reported as that record; any other warning, and every
//! failure, is classified by its
//! typed error into a code of the same `dm.<family>.<kind>` taxonomy DMLS
//! reports, with the types left `None`.
//!
//! `source` says as much as the value proves: a file and an authored position,
//! a file alone with a null line and column when only the file is known (a
//! line counted in a body a stage rewrote is not authored, so it is never
//! published), and `None` for a value naming no file.
//! [`with_document_fallback`] gives a caller that knows the composed document
//! a file for those. A reference issue always has a `source`. A failure inside
//! a repeated `::map` / `::filter` copy also carries the typed `span` and
//! `iteration_path` its error holds, so every host reports the same
//! provenance. Classification reads only the error and warning values: it
//! evaluates nothing and reads no file.
//!
//! A failure caused by a read-side function's file reference keeps
//! `dm.expression.evaluation_failure` and its message, and gains the
//! reference fields (`reason`, `construct`, `reference`, `failure`). A
//! `load_data` failure takes its `dm.data.*` code; a malformed document and
//! each contract violation also carry a `data_location` inside the data, kept
//! apart from `source`, which stays the call site. A contract mismatch
//! reports one record per violating location in file order, at most
//! [`CONTRACT_RECORD_CAP`] of them, then one record counting the rest.

use std::error::Error;
use std::path::{Path, PathBuf};

use biscuit_terminal::errors::SourceContext;

use super::ShellExpansionError;
use super::TransclusionError;
use super::conditions::ConditionError;
use super::context::report::ComposeWarning;
use super::expression::iteration::IterationPath;
use super::expression::data::locate::SourceLocator;
use super::expression::data::{DataError, DataSourceLocation, TabularSource};
use super::expression::{
    DataLocation, Diagnostic, DiagnosticSource, ExpressionError, FileRefFailure, FileReferenceDiagnostic,
    ReferenceDetails, Severity,
};
use super::page_blocks::PageBlockError;
use super::shell_blocks::ShellBlockError;
use super::shell_expansion::ShellCommandOrigin;
use crate::markdown::errors::resolution_failure_name;
use crate::markdown::reference::ReferenceError;
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
/// A command-line setter could not be applied. Its `source` stays null, because
/// a command-line value has no place in any file.
pub const SETTER_INVALID_CODE: &str = "dm.setter.invalid";
/// A lifecycle event failed.
pub const LIFECYCLE_FAILURE_CODE: &str = "dm.lifecycle.failure";
/// A composition failure no narrower code classifies.
pub const COMPOSE_FAILURE_CODE: &str = "dm.compose.failure";
/// A composition warning whose producer declares no code.
pub const UNCODED_WARNING_CODE: &str = "dm.compose.warning";

/// The most contract-violation records one `load_data` failure reports; one
/// further record counts the violations past it.
pub const CONTRACT_RECORD_CAP: usize = 20;

/// The most lines of per-arm detail a union violation's message keeps.
const UNION_DETAIL_LINES: usize = 6;

/// `warning` as a shared record: its own record when it carries one,
/// otherwise its code (or [`UNCODED_WARNING_CODE`]) and message, with its
/// document and proven authored position as `source`.
pub fn warning_diagnostic(warning: &ComposeWarning) -> Diagnostic {
    if let Some(diagnostic) = warning.diagnostic.as_deref() {
        return diagnostic.clone();
    }
    let mut record = unlocated(
        warning.code.as_deref().unwrap_or(UNCODED_WARNING_CODE),
        Severity::Warning,
        warning.message.clone(),
    );
    let file = warning.document.as_deref().and_then(named);
    record.span = warning.location.as_ref().and_then(|location| location.span.clone());
    record.source = file.map(|file| DiagnosticSource {
        line: warning.location.as_ref().map(|location| location.line),
        column: warning.location.as_ref().and_then(|location| location.column),
        ..DiagnosticSource::file_only(Some(file))
    });
    record
}

/// Names `document`, the composed document, as the file of each of
/// `diagnostics` that names none. For a host that knows the document it asked
/// for: a failure's typed error may hold no file of its own. `None` changes
/// nothing, so input with no named origin keeps a null `source`, and so does a
/// [`SETTER_INVALID_CODE`] record, whose value the document never held.
pub fn with_document_fallback(diagnostics: &mut [Diagnostic], document: Option<&Path>) {
    let Some(document) = document else {
        return;
    };
    for diagnostic in diagnostics.iter_mut().filter(|diagnostic| diagnostic.source.is_none() && diagnostic.code != SETTER_INVALID_CODE) {
        diagnostic.source = Some(DiagnosticSource::file_only(Some(document.to_path_buf())));
    }
}

/// A reported reference-validation `issue` as a shared record. A missing
/// local target takes its kind's recovery code
/// ([`missing_reference_code`](super::policy::missing_reference_code)).
///
/// `source` is never `None`. A document with no authored positions (built in
/// memory or fetched) reports its name, or its URL, with a null line and
/// column.
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
    let authored = issue.origin.authored.as_ref();
    Diagnostic {
        code: code.to_string(),
        severity,
        message: issue.message.clone(),
        source: Some(DiagnosticSource {
            file: issue.origin.source.document_path(),
            line: authored.map(|location| location.line),
            column: authored.map(|location| location.column),
            section: issue.origin.section.clone(),
            inserted: authored.is_some_and(|location| location.inserted),
        }),
        expected_type: None,
        actual_type: None,
        span: authored.map(|location| location.span.clone()),
        iteration_path: IterationPath::empty(),
        reference_details: Some(ReferenceDetails {
            reason: Some(issue.code.reason_phrase().to_string()),
            construct: Some(issue.origin.syntax.construct_phrase().trim_start_matches("in ").to_string()),
            reference: Some(issue.reference_display.clone()),
            failure: issue.resolution_failure().map(|failure| resolution_failure_name(failure).to_string()),
            included_via: Some(issue.included_via.clone()),
        }),
        data_location: None,
    }
}

/// The diagnostics a failed composition reports for `error`, found anywhere
/// in its source chain.
///
/// - A policy stop reports the warnings before the stop, then the blocking
///   ones, each with its own severity.
/// - A rejected call, in the document or its lifecycle, reports its located
///   call record; a lifecycle rejection first reports the lifecycle warnings
///   found before it. (A document rejection's earlier warnings travel beside
///   the error, in [`ComposeFailure::warnings`](super::ComposeFailure).)
/// - A schema failure reports one record per problem, and a `load_data`
///   contract mismatch one per violating location (see the module docs).
/// - Any other Darkmatter error reports one record classified by its
///   innermost typed error; a foreign error reports [`INPUT_READ_CODE`] for an
///   I/O error and [`COMPOSE_FAILURE_CODE`] otherwise.
/// - Every record of a Darkmatter failure carries the iteration path of the
///   copies it happened in, and a record with no authored position their
///   authored site (see `with_iteration_context`).
///
/// A record's `source` comes only from `error`; see [`with_document_fallback`]
/// for one that names no file.
pub fn error_diagnostics(error: &(dyn Error + 'static)) -> Vec<Diagnostic> {
    let mut current = Some(error);
    while let Some(error) = current {
        if let Some(markdown) = as_markdown_error(error) {
            return markdown_error_diagnostics(markdown);
        }
        if let Some(shell) = as_shell_error(error) {
            return shell_error_diagnostics(shell);
        }
        if let Some(lifecycle) = error.downcast_ref::<crate::lifecycle::error::LifecycleError>()
            && let Some(diagnostic) = lifecycle.call_diagnostic()
        {
            return lifecycle.earlier_warnings().iter().map(warning_diagnostic).chain([diagnostic.clone()]).collect();
        }
        if let Some(expression) = error
            .downcast_ref::<ExpressionError>()
            .or_else(|| error.downcast_ref::<Box<ExpressionError>>().map(AsRef::as_ref))
        {
            let record = unlocated(expression_code(expression), Severity::Error, expression.to_string());
            return with_expression_cause(record, expression_cause(error));
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
    let mut diagnostics = match error.call_diagnostic() {
        Some(diagnostic) => vec![diagnostic],
        None => failure_diagnostics(innermost(error)),
    };
    with_iteration_context(error, &mut diagnostics);
    diagnostics
}

/// Gives every record of a failure the provenance the chain of `error`
/// carries: each enclosing repeated copy's items
/// ([`IterationFailure`](super::collection_blocks::IterationFailure)),
/// outermost first, followed by the items a collection failure itself names;
/// and, for a record with no position of its own or only its directive's
/// opener, the authored site of the innermost copy that proved one. A message that does not already name the
/// path ends with `at <path>`.
fn with_iteration_context(error: &MarkdownError, diagnostics: &mut [Diagnostic]) {
    let mut path = super::expression::iteration::IterationPath::empty();
    let mut location = None;
    let mut current: Option<&(dyn Error + 'static)> = Some(error);
    while let Some(cause) = current {
        if let Some(MarkdownError::InIteration(failure)) = as_markdown_error(cause) {
            path = path.joined(&failure.iteration_path);
            if let Some(found) = failure.location.as_deref().and_then(located) {
                location = Some(found);
            }
        }
        current = cause.source();
    }
    // A repeated gate's site names its expression, narrower than the opener.
    let gate_failure = matches!(innermost(error), MarkdownError::PageBlock(_));
    match innermost(error) {
        MarkdownError::Collection(inner) => path = path.joined(&inner.iteration_path()),
        MarkdownError::Interpolation { cause, .. } => path = path.joined(&cause.callback_path()),
        _ => {}
    }
    let rendered = path.to_string();
    for diagnostic in diagnostics {
        let positioned = diagnostic.source.as_ref().is_some_and(|source| source.line.is_some());
        if (!positioned || gate_failure)
            && let Some((source, span)) = location.clone()
        {
            diagnostic.source = Some(source);
            diagnostic.span = span;
        }
        if !path.is_empty() {
            if !diagnostic.message.contains(&rendered) {
                diagnostic.message = format!("{} at {rendered}", diagnostic.message);
            }
            diagnostic.iteration_path = path.clone();
        }
    }
}

/// The records of `leaf`, the deepest Darkmatter error of a failure.
fn failure_diagnostics(leaf: &MarkdownError) -> Vec<Diagnostic> {
    match leaf {
        MarkdownError::SchemaValidationFailed { path, problems, .. } if !problems.is_empty() => problems
            .iter()
            .map(|problem| Diagnostic {
                code: problem.code.diagnostic_code().to_string(),
                severity: Severity::Error,
                message: problem.message.clone(),
                source: match problem.line {
                    Some(line) => Some(DiagnosticSource {
                        line: Some(line as usize),
                        column: problem.column.map(|column| column as usize),
                        ..DiagnosticSource::file_only(named(path))
                    }),
                    None => file_only(named(path)),
                },
                expected_type: None,
                actual_type: None,
                span: None,
                iteration_path: IterationPath::empty(),
                reference_details: None,
                data_location: None,
            })
            .collect(),
        MarkdownError::ShellExpansion(shell) => shell_error_diagnostics(shell),
        _ => with_expression_cause(leaf_diagnostic(leaf), expression_cause(leaf)),
    }
}

/// The deepest expression error in `error`'s source chain, callbacks
/// unwrapped: the evaluation failure a stage's wrapper reports.
fn expression_cause<'e>(error: &'e (dyn Error + 'static)) -> Option<&'e ExpressionError> {
    let mut found = None;
    let mut current = Some(error);
    while let Some(cause) = current {
        if let Some(expression) = cause
            .downcast_ref::<ExpressionError>()
            .or_else(|| cause.downcast_ref::<Box<ExpressionError>>().map(AsRef::as_ref))
        {
            found = Some(expression.without_callbacks());
        }
        current = cause.source();
    }
    found
}

/// `record`, the failure's record, with what its expression `cause` adds:
/// the reference fields of a file-reference failure, or a `load_data`
/// failure's code and data positions (several records for a contract
/// mismatch). Every record keeps `record`'s `source` and `span`.
fn with_expression_cause(record: Diagnostic, cause: Option<&ExpressionError>) -> Vec<Diagnostic> {
    match cause {
        Some(ExpressionError::FileReference(diagnostic)) => vec![with_file_reference(record, diagnostic)],
        Some(ExpressionError::RemoteRead { function, reference, .. }) => {
            vec![with_failed_fetch(record, function, reference)]
        }
        Some(ExpressionError::SchemaFileRead { file, .. }) => vec![with_file_reference(record, file)],
        Some(ExpressionError::Data(data)) => data_records(record, data),
        _ => vec![record],
    }
}

fn with_file_reference(record: Diagnostic, diagnostic: &FileReferenceDiagnostic) -> Diagnostic {
    let details = record.reference_details.clone().unwrap_or_else(|| file_reference_details(diagnostic));
    Diagnostic { reference_details: Some(details), ..record }
}

/// A read-side function's failed fetch from an allowed host, in the words a
/// reference check uses for a remote that answered with an error or not at
/// all: `unreachable`, and the `io` class of a file that exists but could not
/// be read. The host was allowed, so it is never `remote check disabled`.
fn with_failed_fetch(record: Diagnostic, function: &str, reference: &str) -> Diagnostic {
    let details = record.reference_details.clone().unwrap_or_else(|| ReferenceDetails {
        reason: Some(ReferenceIssueCode::RemoteUnreachable.reason_phrase().to_string()),
        construct: Some(format!("{function}()")),
        reference: Some(reference.to_string()),
        failure: Some(resolution_failure_name(biscuit_file::ResolutionFailure::Io).to_string()),
        included_via: None,
    });
    Diagnostic { reference_details: Some(details), ..record }
}

/// A read-side function's failed file reference in the reference records'
/// vocabulary. `reason` is left out for a class no reference phrase names
/// (an I/O failure, a missing anchor, other invalid syntax).
fn file_reference_details(diagnostic: &FileReferenceDiagnostic) -> ReferenceDetails {
    use biscuit_file::{FileReferenceError, ResolutionFailure};
    let failure = diagnostic.resolution_failure();
    let reason = match (diagnostic.kind, failure, diagnostic.source.as_deref()) {
        (FileRefFailure::RemoteNotEnabled | FileRefFailure::RemoteDenied, ..) => {
            Some(ReferenceIssueCode::RemoteDisallowed)
        }
        (_, ResolutionFailure::NoMatch, _) => Some(ReferenceIssueCode::MissingLocalTarget),
        (_, _, Some(FileReferenceError::UnsupportedScheme { .. })) => Some(ReferenceIssueCode::UnsupportedScheme),
        (_, _, Some(FileReferenceError::InvalidUrl(_))) => Some(ReferenceIssueCode::InvalidUrl),
        _ => None,
    };
    ReferenceDetails {
        reason: reason.map(|code| code.reason_phrase().to_string()),
        construct: Some(format!("{}()", diagnostic.function)),
        reference: Some(diagnostic.reference.clone()),
        failure: Some(resolution_failure_name(failure).to_string()),
        included_via: None,
    }
}

/// The records of a `load_data` failure reported at `record`'s site.
fn data_records(record: Diagnostic, error: &DataError) -> Vec<Diagnostic> {
    let Some(code) = error.code() else {
        return match error {
            DataError::FileReference(diagnostic) => vec![with_file_reference(record, diagnostic)],
            _ => vec![record],
        };
    };
    let record = Diagnostic { code: code.to_string(), ..record };
    match error {
        DataError::Malformed { identity, location, .. } => {
            let DataSourceLocation { byte_offset, pointer, line, column } = location.clone();
            let data_location = DataLocation { file: identity.read_set_entry(), pointer, line, column, byte_offset };
            vec![Diagnostic { data_location: Some(data_location), ..record }]
        }
        DataError::ContractMismatch { identity, format, written, failure, text, table } => {
            let mismatch = Mismatch { file: identity.read_set_entry(), written, text, table: table.as_ref() };
            contract_records(record, &mismatch, *format, failure)
        }
        _ => vec![record],
    }
}

/// What every record of one contract mismatch shares.
struct Mismatch<'a> {
    file: String,
    written: &'a str,
    text: &'a str,
    table: Option<&'a TabularSource>,
}

impl Mismatch<'_> {
    fn message(&self, detail: impl std::fmt::Display) -> String {
        format!("load_data(): `{}` does not match `{}`: {detail}", self.file, self.written)
    }
}

/// One record per violating location, in file order: by source position,
/// then (for YAML the file cannot position, and for a missing member or
/// slot) by parsed container order, the engine's declaration order breaking
/// ties. Past [`CONTRACT_RECORD_CAP`], one record at the call site counts
/// the rest.
fn contract_records(
    record: Diagnostic,
    mismatch: &Mismatch<'_>,
    format: biscuit_file::DataFormat,
    failure: &simplified_schema::coercion::ConversionError,
) -> Vec<Diagnostic> {
    use simplified_schema::coercion::{ActualKind, FailureReason};
    let mut locator = SourceLocator::new(format, mismatch.text, mismatch.table);
    let mut placed: Vec<_> = failure
        .located_failures()
        .into_iter()
        .map(|violation| {
            let place = locator.place(&violation.failure.path, violation.actual == ActualKind::Missing);
            let detail = violation.failure.to_string();
            let detail = match violation.failure.reason {
                FailureReason::NoOptionAccepts { .. } => cut_union_detail(&detail),
                _ => detail,
            };
            let expected = if violation.pointer.is_empty() {
                mismatch.written.to_string()
            } else {
                violation.expected.to_string()
            };
            let data_location = DataLocation {
                file: mismatch.file.clone(),
                pointer: Some(violation.pointer),
                line: place.position.map(|(line, _)| line),
                column: place.position.map(|(_, column)| column),
                byte_offset: None,
            };
            let diagnostic = Diagnostic {
                message: mismatch.message(detail),
                expected_type: Some(expected),
                actual_type: Some(violation.actual.as_str().to_string()),
                data_location: Some(data_location),
                ..record.clone()
            };
            (place.order, diagnostic)
        })
        .collect();
    placed.sort_by(|left, right| left.0.cmp(&right.0));
    let omitted = placed.len().saturating_sub(CONTRACT_RECORD_CAP);
    let mut records: Vec<Diagnostic> = placed.into_iter().take(CONTRACT_RECORD_CAP).map(|(_, record)| record).collect();
    if omitted > 0 {
        let more = if omitted == 1 { "1 more violation is".to_string() } else { format!("{omitted} more violations are") };
        records.push(Diagnostic { message: mismatch.message(format_args!("{more} not listed")), ..record });
    }
    records
}

/// `detail`, a union violation's text, with at most [`UNION_DETAIL_LINES`]
/// lines after its first, and an ellipsis line when any were cut.
fn cut_union_detail(detail: &str) -> String {
    let mut lines = detail.lines();
    let mut kept: Vec<&str> = lines.by_ref().take(1 + UNION_DETAIL_LINES).collect();
    if lines.next().is_some() {
        kept.push("    …");
    }
    kept.join("\n")
}

/// Where `source` is in its file and the bytes it covers: a file alone when
/// its span was not proven.
fn located(source: &SourceRef) -> Option<(DiagnosticSource, Option<std::ops::Range<usize>>)> {
    match source {
        SourceRef::OnDiskSpan { context, span } => Some((
            DiagnosticSource::at(context_file(context), span.line(), span.column()),
            Some(span.range()),
        )),
        SourceRef::OnDisk(context) => context_source(context).map(|source| (source, None)),
        SourceRef::Supplied { .. } | SourceRef::Effective { .. } => None,
    }
}

fn leaf_diagnostic(error: &MarkdownError) -> Diagnostic {
    let message = error.to_string();
    let mut span = None;
    let mut proven = |source: Option<&SourceRef>| {
        let (source, range) = source.and_then(located)?;
        span = range;
        Some(source)
    };
    let (code, source) = match error {
        MarkdownError::FrontmatterParse { ctx, source } => (
            FRONTMATTER_PARSE_CODE,
            match source.location() {
                Some(location) => Some(DiagnosticSource::at(
                    context_file(ctx),
                    frontmatter_file_line(ctx, location.line()),
                    location.column().max(1),
                )),
                None => context_source(ctx),
            },
        ),
        MarkdownError::FrontmatterFenceMismatch { ctx, line, .. } => (
            FRONTMATTER_PARSE_CODE,
            Some(DiagnosticSource::at(context_file(ctx), *line, 1)),
        ),
        MarkdownError::SchemaValidationFailed { path, .. } => (simplified_schema::SCHEMA_PREPARE_CODE, file_only(named(path))),
        MarkdownError::FileLoad(_) | MarkdownError::UrlFetch(_) => (INPUT_READ_CODE, None),
        MarkdownError::Interpolation { source, cause, .. } => (expression_code(cause), proven(Some(source.as_ref()))),
        MarkdownError::Transclusion(inner) => (
            match inner.as_ref() {
                TransclusionError::ParseDirective { .. } => DIRECTIVE_PARSE_CODE,
                TransclusionError::ConditionEval { .. } => ComposeWarning::EXPRESSION_EVALUATION_FAILURE_CODE,
                _ => TRANSCLUSION_FAILURE_CODE,
            },
            match inner.as_ref() {
                TransclusionError::ConditionEval { ctx, authored, .. }
                | TransclusionError::ConditionParse { ctx, authored, .. } => condition_source(ctx, authored.as_ref(), &mut span),
                // The directive's line is in the current body, so only the
                // file is authored.
                TransclusionError::ParseDirective { ctx, .. }
                | TransclusionError::InvalidReference { ctx, .. }
                | TransclusionError::InvalidFrontmatterAssignment { ctx, .. }
                | TransclusionError::InvalidReassignedFrontmatterProperty { ctx, .. } => context_source(ctx),
                // The last document on the chain before the repeat wrote the
                // directive that closes the cycle. A chain's line is not
                // that directive's.
                TransclusionError::CycleDetected { chain } => {
                    file_only(chain.iter().rev().nth(1).and_then(|(path, _)| named(path)))
                }
                _ => None,
            },
        ),
        MarkdownError::PageBlock(inner) => match inner.as_ref() {
            PageBlockError::Condition(condition) => match condition.as_ref() {
                ConditionError::Parse { ctx, authored, .. } => {
                    (ComposeWarning::EXPRESSION_PARSE_FAILURE_CODE, condition_source(ctx, authored.as_ref(), &mut span))
                }
                ConditionError::Eval { ctx, authored, .. } => {
                    (ComposeWarning::EXPRESSION_EVALUATION_FAILURE_CODE, condition_source(ctx, authored.as_ref(), &mut span))
                }
            },
            // The parser counts lines in the current body.
            PageBlockError::ParseDirective { ctx, .. }
            | PageBlockError::UnmatchedEnd { ctx, .. }
            | PageBlockError::UnterminatedBlock { ctx, .. } => (DIRECTIVE_PARSE_CODE, context_source(ctx)),
        },
        MarkdownError::Collection(inner) => (inner.code(), proven(inner.site().map(|site| site.source.as_ref()))),
        MarkdownError::ShellBlock(inner) => (SHELL_FAILURE_CODE, shell_block_source(inner)),
        MarkdownError::Reference(inner) => (
            REFERENCE_FAILURE_CODE,
            // The directive's line is in the prepared body, so only the file
            // is authored.
            match inner.as_ref() {
                ReferenceError::ParseDirective { ctx, .. } => context_source(ctx),
                _ => None,
            },
        ),
        MarkdownError::ParameterBinding { .. } | MarkdownError::CallerFileClassificationChanged { .. } => {
            (PARAMETER_BINDING_CODE, None)
        }
        MarkdownError::Setter { .. } => (SETTER_INVALID_CODE, None),
        _ => (COMPOSE_FAILURE_CODE, None),
    };
    Diagnostic {
        code: code.to_string(),
        severity: Severity::Error,
        message,
        source,
        expected_type: None,
        actual_type: None,
        span,
        iteration_path: IterationPath::empty(),
        reference_details: match error {
            MarkdownError::Reference(inner) => reference_error_details(inner),
            _ => None,
        },
        data_location: None,
    }
}

/// A condition failure's opener when it is proven, otherwise its file.
fn condition_source(
    ctx: &SourceContext,
    authored: Option<&crate::markdown::types::AuthoredSpan>,
    span: &mut Option<std::ops::Range<usize>>,
) -> Option<DiagnosticSource> {
    match authored {
        Some(authored) => {
            *span = Some(authored.range());
            Some(DiagnosticSource::at(context_file(ctx), authored.line(), authored.column()))
        }
        None => context_source(ctx),
    }
}

/// A shell failure's file, and a frontmatter command's line. A body or
/// shell-block origin counts lines in the prepared body, and a command with no
/// file of its own keeps its enclosing block's.
fn shell_source(error: &ShellExpansionError) -> Option<DiagnosticSource> {
    if let ShellExpansionError::PolicyIo { path, .. } = error {
        return file_only(named(path));
    }
    let file = context_file(error.context()?)?;
    let line = match error.origin() {
        Some(ShellCommandOrigin::Frontmatter { line, .. }) => *line,
        _ => None,
    };
    Some(DiagnosticSource { line, ..DiagnosticSource::file_only(Some(file)) })
}

fn shell_block_source(error: &ShellBlockError) -> Option<DiagnosticSource> {
    let (file, command) = match error {
        ShellBlockError::Parse { source_file, .. } | ShellBlockError::Unterminated { source_file, .. } => (source_file, None),
        ShellBlockError::Command { source_file, source, .. } => (source_file, Some(source)),
    };
    file_only(file.as_deref().and_then(named)).or_else(|| command.and_then(|command| shell_source(command)))
}

/// What a reference check that could not run knows about its reference.
fn reference_error_details(error: &ReferenceError) -> Option<ReferenceDetails> {
    let failure = error.resolution_failure().map(|failure| resolution_failure_name(failure).to_string());
    let (reason, reference) = match error {
        ReferenceError::MissingSourceContext { reference, .. } => {
            (Some(ReferenceIssueCode::MissingSourceContext.reason_phrase().to_string()), Some(reference.clone()))
        }
        _ => (None, None),
    };
    (failure.is_some() || reason.is_some()).then_some(ReferenceDetails { reason, reference, failure, ..Default::default() })
}

fn shell_error_diagnostics(error: &ShellExpansionError) -> Vec<Diagnostic> {
    match error {
        ShellExpansionError::Preflight(inner) => markdown_error_diagnostics(inner),
        ShellExpansionError::ParseDirective { .. } => vec![shell_record(DIRECTIVE_PARSE_CODE, error)],
        _ => vec![shell_record(SHELL_FAILURE_CODE, error)],
    }
}

fn shell_record(code: &str, error: &ShellExpansionError) -> Diagnostic {
    Diagnostic { source: shell_source(error), ..unlocated(code, Severity::Error, error.to_string()) }
}

fn expression_code(cause: &ExpressionError) -> &'static str {
    match cause.without_callbacks() {
        ExpressionError::IterationLimit(error) => error.kind.code(),
        ExpressionError::Parse(_) | ExpressionError::MalformedLiteralToken(_) => {
            ComposeWarning::EXPRESSION_PARSE_FAILURE_CODE
        }
        ExpressionError::UnknownFunction { .. } => UNKNOWN_FUNCTION_CODE,
        ExpressionError::InvalidCallback { .. } => simplified_schema::expression::parser::INVALID_CALLBACK_CODE,
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
fn frontmatter_file_line(ctx: &SourceContext, yaml_line: usize) -> usize {
    match ctx.frontmatter.as_ref() {
        Some(range) => ctx.content[..range.start].matches('\n').count() + 1 + yaml_line,
        None => yaml_line,
    }
}

/// `path`, unless it is a placeholder for input with no named origin
/// (`unknown`, `<stdin>`, `<transient>`).
fn named(path: &Path) -> Option<PathBuf> {
    let placeholder = path == Path::new("unknown") || path.to_str().is_some_and(|text| text.starts_with('<'));
    (!placeholder).then(|| path.to_path_buf())
}

/// A context's document. A frontmatter value's own context is named after its
/// key, with no absolute path, and is no file.
fn context_file(ctx: &SourceContext) -> Option<PathBuf> {
    named(&ctx.display).filter(|_| ctx.absolute.is_absolute())
}

fn context_source(ctx: &SourceContext) -> Option<DiagnosticSource> {
    file_only(context_file(ctx))
}

fn file_only(file: Option<PathBuf>) -> Option<DiagnosticSource> {
    file.map(|file| DiagnosticSource::file_only(Some(file)))
}

fn unlocated(code: &str, severity: Severity, message: String) -> Diagnostic {
    Diagnostic {
        code: code.to_string(),
        severity,
        message,
        source: None,
        expected_type: None,
        actual_type: None,
        span: None,
        iteration_path: IterationPath::empty(),
        reference_details: None,
        data_location: None,
    }
}
