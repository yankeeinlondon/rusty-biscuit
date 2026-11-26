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
    DataLocation, Diagnostic, DiagnosticRelated, DiagnosticSource, ExpressionError, FileRefFailure, FileReferenceDiagnostic,
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
/// A `::toc-linking` `level=` names no heading level from `h1` to `h6`.
pub const TOC_LINKING_INVALID_LEVEL_CODE: &str = "dm.toc_linking.invalid_level";
/// A `::toc-linking` `cleanup=` names an unknown cleanup service.
pub const TOC_LINKING_INVALID_CLEANUP_CODE: &str = "dm.toc_linking.invalid_cleanup";
/// A `::toc-linking` `keep=` or `filter=` pattern does not compile.
pub const TOC_LINKING_INVALID_FILTER_CODE: &str = "dm.toc_linking.invalid_filter";
/// A `::file-links` `--depth` is not a non-negative integer.
pub const FILE_LINKS_INVALID_DEPTH_CODE: &str = "dm.file_links.invalid_depth";
/// A `::file-links` directive is in a document with no source file.
pub const FILE_LINKS_MISSING_SOURCE_CODE: &str = "dm.file_links.missing_source";
/// A `::file-links --dir` target exists but is not a directory.
pub const FILE_LINKS_NOT_A_DIRECTORY_CODE: &str = "dm.file_links.not_a_directory";
/// A `::file-links` glob is invalid, leaves the file tree, or cannot be rooted.
pub const FILE_LINKS_INVALID_GLOB_CODE: &str = "dm.file_links.invalid_glob";
/// Re-leveling would push a heading past H6.
pub const NORMALIZATION_LEVEL_OVERFLOW_CODE: &str = "dm.normalization.level_overflow";
/// A heading structure rule was violated.
pub const NORMALIZATION_VALIDATION_FAILED_CODE: &str = "dm.normalization.validation_failed";
/// A transcluded heading re-leveled past H6 became bold text.
pub const TRANSCLUSION_HEADING_OVERFLOW_CODE: &str = "dm.transclusion.heading_overflow";
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

/// A reported reference-validation `issue` as a shared record. A missing or
/// empty local target takes its kind's recovery code
/// ([`missing_reference_code`](super::policy::missing_reference_code)).
///
/// `source` is never `None`. A document with no authored positions (built in
/// memory or fetched) reports its name, or its URL, with a null line and
/// column.
pub fn reference_issue_diagnostic(issue: &ReferenceIssue) -> Diagnostic {
    let code = match issue.code {
        ReferenceIssueCode::MissingLocalTarget | ReferenceIssueCode::EmptyTarget => {
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
            base: None,
            resolved: None,
            searched: None,
            suggestions: None,
        }),
        data_location: None,
        grants_needed: None,
        schema_default: None,
        related: Vec::new(),
        hint: None,
        notes: Vec::new(),
        trail: crate::findings::Trail::default(),
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
///   innermost typed error, including a reference-analysis error carried
///   without a [`MarkdownError`] around it (a `FileTreeError`; one that
///   carries composition's own failure, such as an inclusion's target, is
///   projected as composition projects it) and the
///   `NormalizationError` `Markdown::normalize` and `relevel` return; a foreign
///   error reports [`INPUT_READ_CODE`] for an I/O error and
///   [`COMPOSE_FAILURE_CODE`] otherwise.
/// - An inclusion's target failure ([`TransclusionError::Located`]) is placed
///   at the inclusion that named the target, in the file that wrote it.
/// - Every record of a Darkmatter failure carries the iteration path of the
///   copies it happened in, and a record with no authored position their
///   authored site (see `with_iteration_context`).
///
/// A record's `source` comes only from `error`; see [`with_document_fallback`]
/// for one that names no file.
pub fn error_diagnostics(error: &(dyn Error + 'static)) -> Vec<Diagnostic> {
    let top = error;
    let mut current = Some(error);
    while let Some(error) = current {
        if let Some(config) = error.downcast_ref::<crate::config::load::ConfigError>() {
            return config_error_diagnostics(config);
        }
        if let Some(markdown) = as_markdown_error(error) {
            return markdown_error_diagnostics(markdown);
        }
        if let Some(shell) = as_shell_error(error) {
            return shell_error_diagnostics(shell);
        }
        if let Some(reference) = error
            .downcast_ref::<ReferenceError>()
            .or_else(|| error.downcast_ref::<Box<ReferenceError>>().map(AsRef::as_ref))
        {
            // A failure composition also raises is projected as it is
            // there, under this error's own words.
            if let ReferenceError::Compose(inner) = reference {
                let mut records = markdown_error_diagnostics(inner);
                if let [record] = records.as_mut_slice() {
                    record.message = top.to_string();
                }
                return records;
            }
            return vec![reference_diagnostic(reference, top.to_string())];
        }
        if let Some(normalization) = error.downcast_ref::<crate::markdown::normalize::NormalizationError>() {
            return vec![unlocated(normalization_code(normalization), Severity::Error, top.to_string())];
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
        if let Some(records) = error
            .downcast_ref::<simplified_schema::SchemaError>()
            .and_then(|schema| schema_error_diagnostics(schema, Path::new("unknown")))
        {
            return records;
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

/// A configuration load failure as one error record per located problem,
/// each with its file, line, and column, under the error's code
/// (`dm.config.invalid` or `dm.config.reference`).
pub fn config_error_diagnostics(error: &crate::config::load::ConfigError) -> Vec<Diagnostic> {
    error
        .problems()
        .iter()
        .map(|problem| Diagnostic {
            source: Some(config_problem_source(problem)),
            ..unlocated(error.code(), Severity::Error, problem.to_string())
        })
        .collect()
}

/// A finding a configuration load recorded without failing (the missing
/// user file, a permissions error, an omitted registration) as warnings: one
/// per located problem, or one naming the file when it has none. The created
/// user file ([`crate::config::load::CONFIG_CREATED_CODE`]) is info.
pub fn config_notice_diagnostics(notice: &crate::config::load::ConfigDiagnostic) -> Vec<Diagnostic> {
    if notice.problems.is_empty() {
        let severity =
            if notice.code == crate::config::load::CONFIG_CREATED_CODE { Severity::Info } else { Severity::Warning };
        return vec![Diagnostic {
            source: Some(DiagnosticSource::file_only(Some(notice.file.clone()))),
            ..unlocated(notice.code, severity, notice.message.clone())
        }];
    }
    notice
        .problems
        .iter()
        .map(|problem| Diagnostic {
            source: Some(config_problem_source(problem)),
            ..unlocated(notice.code, Severity::Warning, problem.to_string())
        })
        .collect()
}

/// A refused registration at its authored position, with the other
/// declarations of a conflict attached to the same diagnostic.
pub fn registration_diagnostic(diagnostic: &crate::config::registration::RegistrationDiagnostic) -> Diagnostic {
    let source = |file: &Path, line: Option<usize>, column: Option<usize>| {
        let file = Some(file.to_path_buf());
        match (line, column) {
            (Some(line), Some(column)) => DiagnosticSource::at(file, line, column),
            _ => DiagnosticSource::file_only(file),
        }
    };
    let related = diagnostic.related.iter().map(|related| DiagnosticRelated {
        message: related.message.clone(),
        source: source(&related.file, related.line, related.column),
    }).collect();
    Diagnostic {
        source: Some(source(&diagnostic.file, diagnostic.line, diagnostic.column)),
        related,
        ..unlocated(diagnostic.code, diagnostic.severity, diagnostic.message.clone())
    }
}

fn config_problem_source(problem: &crate::config::load::ConfigProblem) -> DiagnosticSource {
    match (problem.line, problem.column) {
        (Some(line), Some(column)) => DiagnosticSource::at(Some(problem.file.clone()), line, column),
        _ => DiagnosticSource::file_only(Some(problem.file.clone())),
    }
}

/// The diagnostics of a failed lifecycle event: its Darkmatter cause's
/// ([`error_diagnostics`]) when it has one, otherwise one
/// [`LIFECYCLE_FAILURE_CODE`] record carrying the failure's complete `err.msg`.
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

/// The warning record of one near-miss key advice on `path`'s frontmatter,
/// located at the key as a failure's problem is.
pub(crate) fn key_advice_diagnostic(problem: &simplified_schema::ValidationProblem, path: &Path) -> Diagnostic {
    let source = match problem.line {
        Some(line) => Some(DiagnosticSource {
            line: Some(line as usize),
            column: problem.column.map(|column| column as usize),
            ..DiagnosticSource::file_only(named(path))
        }),
        None => file_only(named(path)),
    };
    Diagnostic {
        source,
        hint: problem.hint.clone(),
        ..unlocated(problem.code.diagnostic_code(), Severity::Warning, problem.message.clone())
    }
}

/// The records of `leaf`, the deepest Darkmatter error of a failure.
///
/// A schema problem on a property one whole-value `{{ … }}` produced is
/// placed at that expression ([`ExpressionProjector`]), and a direct load's
/// problem also names its data file position.
fn failure_diagnostics(leaf: &MarkdownError) -> Vec<Diagnostic> {
    match leaf {
        MarkdownError::SchemaValidationFailed { path, problems, expression_origins, .. } if !problems.is_empty() => {
            let mut projector = ExpressionProjector::new(expression_origins);
            problems
                .iter()
                .map(|problem| {
                    let expression = projector.project(problem);
                    let authored = expression.as_ref().and_then(|expression| expression.span.as_ref());
                    let source = match (authored, problem.line) {
                        (Some(span), _) => Some(DiagnosticSource::at(named(path), span.line(), span.column())),
                        (None, Some(line)) => Some(DiagnosticSource {
                            line: Some(line as usize),
                            column: problem.column.map(|column| column as usize),
                            ..DiagnosticSource::file_only(named(path))
                        }),
                        (None, None) => file_only(named(path)),
                    };
                    Diagnostic {
                        code: problem.code.diagnostic_code().to_string(),
                        severity: Severity::Error,
                        message: problem.message.clone(),
                        source,
                        expected_type: None,
                        actual_type: None,
                        span: authored.map(crate::markdown::AuthoredSpan::range),
                        iteration_path: IterationPath::empty(),
                        reference_details: schema_file_reference_details(problem),
                        data_location: expression.and_then(|expression| expression.data),
                        grants_needed: None,
                        related: Vec::new(),
                        schema_default: problem
                            .default
                            .as_ref()
                            .map(crate::markdown::compose::expression::SchemaDefaultOrigin::from),
                        hint: problem.hint.clone(),
                        notes: Vec::new(),
                        trail: crate::findings::Trail::default(),
                    }
                })
                .collect()
        }
        MarkdownError::ShellExpansion(shell) => shell_error_diagnostics(shell),
        MarkdownError::SchemaValidationFailed { path, source: Some(source), .. }
            if let Some(records) = source.downcast_ref::<simplified_schema::SchemaError>().and_then(|schema| schema_error_diagnostics(schema, path)) =>
        {
            records
        }
        _ => with_expression_cause(leaf_diagnostic(leaf), expression_cause(leaf)),
    }
}

/// The reference fields of a schema problem about a `file`-typed value: the
/// value as written and its failure class, which a host prints as its
/// `failure:` row. `None` for every other problem.
fn schema_file_reference_details(problem: &simplified_schema::ValidationProblem) -> Option<ReferenceDetails> {
    let diagnostic = problem.file_reference.as_ref()?;
    let raw = match diagnostic {
        simplified_schema::FileReferenceDiagnostic::InvalidSyntax { raw }
        | simplified_schema::FileReferenceDiagnostic::ResolutionFailed { raw, .. }
        | simplified_schema::FileReferenceDiagnostic::NoMatch { raw, .. } => raw,
    };
    Some(ReferenceDetails {
        reference: Some(raw.clone()),
        failure: Some(resolution_failure_name(diagnostic.resolution_failure()).to_string()),
        ..ReferenceDetails::default()
    })
}

/// The coded records of a schema preparation failure: a template's
/// ([`template_diagnostics`]), or one
/// [`HOST_TRIGGER_CONFLICT_CODE`](crate::config::registration::merge::HOST_TRIGGER_CONFLICT_CODE)
/// error on the document whose registered host's matched triggers conflict.
/// `None` for any other schema error.
pub fn schema_error_diagnostics(error: &simplified_schema::SchemaError, document: &Path) -> Option<Vec<Diagnostic>> {
    if let simplified_schema::SchemaError::HostTriggerConflict(conflict) = error {
        let file = conflict.document.clone().or_else(|| named(document));
        return Some(vec![Diagnostic {
            source: file_only(file),
            ..unlocated(crate::config::registration::merge::HOST_TRIGGER_CONFLICT_CODE, Severity::Error, error.to_string())
        }]);
    }
    template_diagnostics(error, document)
}

/// The records of a schema that is a template: each slot's own failure,
/// at the slot, when its resolution failed (an unapproved read's record
/// names the `grants_needed`), and `dm.schema.slot_pending` at `document`
/// when it was not resolved for this run. `None` for any other schema
/// error.
///
/// A host that validates without composing (`md schema validate`) reports a
/// template's failure through this.
pub fn template_diagnostics(error: &simplified_schema::SchemaError, document: &Path) -> Option<Vec<Diagnostic>> {
    use crate::markdown::schemas::slots::codes::SLOT_PENDING_CODE;
    match error {
        simplified_schema::SchemaError::SlotResolution(failure) => {
            Some(slot_resolution_diagnostics(&failure.error, &|slot| failure_text(failure, slot)))
        }
        simplified_schema::SchemaError::UnresolvedSlots(_) => Some(vec![Diagnostic {
            source: file_only(named(document)),
            ..unlocated(SLOT_PENDING_CODE, Severity::Error, error.to_string())
        }]),
        _ => None,
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
    if let Some(data) = cause.and_then(ExpressionError::data_failure) {
        return match crate::markdown::schemas::slots::host::contract_template_failure(data) {
            Some(failure) => contract_template_records(record, failure),
            None => data_records(record, data),
        };
    }
    match cause {
        Some(ExpressionError::FileReference(diagnostic)) => vec![with_file_reference(record, diagnostic)],
        Some(ExpressionError::RemoteRead { function, reference, .. }) => {
            vec![with_failed_fetch(record, function, reference)]
        }
        Some(ExpressionError::SchemaFileRead { file, .. }) => vec![with_file_reference(record, file)],
        Some(ExpressionError::SlotReadRefused {
            refusal: crate::markdown::compose::expression::read_gate::GateRefusal::ApprovalRequired { needed },
            ..
        }) => vec![Diagnostic {
            grants_needed: Some(needed.iter().map(crate::markdown::compose::expression::diagnostics::GrantNeeded::of).collect()),
            ..record
        }],
        _ => vec![record],
    }
}

/// `record`, a slot whose `load_data` contract named a template, once per
/// record of that template's failed slots: each keeps its own code and what
/// its cause adds (a `data_location`, the `grants_needed`, reference
/// fields), as it would had it failed in this slot, while `record`'s
/// `source`, `span`, and message stay primary. The diagnostic model has no
/// secondary location, so the inner slot's own position is not published.
fn contract_template_records(record: Diagnostic, failure: &simplified_schema::SlotResolutionFailure) -> Vec<Diagnostic> {
    slot_resolution_records(&failure.error, &|slot| failure_text(failure, slot))
        .into_iter()
        .map(|(_, inner)| Diagnostic {
            message: record.message.clone(),
            source: record.source.clone(),
            span: record.span.clone(),
            iteration_path: record.iteration_path.clone(),
            ..inner
        })
        .collect()
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
        base: None,
        resolved: None,
        searched: None,
        suggestions: None,
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
        base: None,
        resolved: None,
        searched: None,
        suggestions: None,
    }
}

/// The records of a `load_data` failure reported at `record`'s site.
fn data_records(record: Diagnostic, error: &DataError) -> Vec<Diagnostic> {
    let Some(code) = error.code() else {
        return match error {
            DataError::FileReference(diagnostic) | DataError::ContractImport(diagnostic) => {
                vec![with_file_reference(record, diagnostic)]
            }
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
        DataError::ContractMismatch { identity, format, written, failure, text, table, wrapped } => {
            let mismatch =
                Mismatch { file: identity.read_set_entry(), written, text, table: table.as_ref(), wrapped: *wrapped };
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
    /// The root was a scalar wrapped in a one-element array.
    wrapped: bool,
}

impl Mismatch<'_> {
    fn message(&self, detail: impl std::fmt::Display) -> String {
        format!("load_data(): `{}` does not match `{}`: {detail}", self.file, self.written)
    }
}

/// Added to an unexpected `<<` key's record: the YAML loader keeps a merge
/// key as an ordinary key, so an author expecting inheritance learns why.
const MERGE_KEY_HINT: &str = "YAML merge keys are not applied, so `<<` is read as an ordinary key.";

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
    use simplified_schema::coercion::{ActualKind, FailureReason, PathSegment};
    let mut locator = SourceLocator::new(format, mismatch.text, mismatch.table);
    let mut placed: Vec<_> = failure
        .located_failures()
        .into_iter()
        .map(|violation| {
            let source = source_path(&violation.failure.path, mismatch.wrapped);
            let place = locator.place(source, violation.actual == ActualKind::Missing);
            let detail = violation.failure.to_string();
            let detail = match violation.failure.reason {
                FailureReason::NoOptionAccepts { .. } => cut_union_detail(&detail),
                FailureReason::UnexpectedProperty
                    if format == biscuit_file::DataFormat::Yaml
                        && matches!(violation.failure.path.last(), Some(PathSegment::Key(key)) if key == "<<") =>
                {
                    format!("{detail}. {MERGE_KEY_HINT}")
                }
                _ => detail,
            };
            let expected = if violation.pointer.is_empty() {
                mismatch.written.to_string()
            } else {
                violation.expected.to_string()
            };
            // A wrapped scalar's pointers name the document, not the array.
            let pointer =
                if mismatch.wrapped { simplified_schema::coercion::json_pointer(source) } else { violation.pointer };
            let data_location = DataLocation {
                file: mismatch.file.clone(),
                pointer: Some(pointer),
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

/// Where a destination schema problem's value came from, when a whole-value
/// expression produced its top-level property.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProblemExpression {
    /// The expression's authored span, the problem's primary location.
    pub(crate) span: Option<crate::markdown::AuthoredSpan>,
    /// The value's position in the data file, for a direct load whose value
    /// the problem's path names; `None` for a derived value, a value a schema
    /// default supplied, and a path the loaded value does not hold.
    pub(crate) data: Option<DataLocation>,
}

/// The data-file position of each of `problems` (destination schema
/// problems of top-level properties) whose property `origins` records as a
/// direct load, in the problems' order; `None` where no position is proven.
/// The same projection composition's records use, so an editor names the
/// same place.
pub fn data_locations(
    problems: &[simplified_schema::ValidationProblem],
    origins: &std::collections::BTreeMap<String, crate::markdown::ExpressionMetadata>,
) -> Vec<Option<DataLocation>> {
    let mut projector = ExpressionProjector::new(origins);
    problems.iter().map(|problem| projector.project(problem).and_then(|projected| projected.data)).collect()
}

/// Projects the problems of one schema failure onto the expressions that
/// produced their properties, with one [`SourceLocator`] per property's
/// source. The structured records and the status block both use it, so they
/// name the same positions.
pub(crate) struct ExpressionProjector<'a> {
    origins: &'a std::collections::BTreeMap<String, crate::markdown::ExpressionMetadata>,
    locators: std::collections::HashMap<&'a str, SourceLocator<'a>>,
}

impl<'a> ExpressionProjector<'a> {
    pub(crate) fn new(origins: &'a std::collections::BTreeMap<String, crate::markdown::ExpressionMetadata>) -> Self {
        Self { origins, locators: std::collections::HashMap::new() }
    }

    /// The expression that produced `problem`'s property, or `None` when no
    /// whole-value expression did.
    ///
    /// The data position follows the loader's convention: the problem's path
    /// without its property, read as keys and indexes against the loaded
    /// value; a missing member at its parent; an unknown member at the member
    /// itself; a wrapped scalar's element at the document root.
    pub(crate) fn project(&mut self, problem: &simplified_schema::ValidationProblem) -> Option<ProblemExpression> {
        use simplified_schema::ValidationProblemKind;
        use simplified_schema::coercion::{PathSegment, json_pointer};
        let (key, rest) = problem.instance_path.segments().split_first()?;
        let (key, metadata) = self.origins.get_key_value(key.as_str())?;
        let data = metadata.load.as_deref().filter(|_| problem.default.is_none()).and_then(|load| {
            let mut segments = typed_segments(&metadata.value, rest)?;
            let missing = problem.kind == ValidationProblemKind::Missing;
            match (missing, &problem.property, &problem.offending_property) {
                (true, Some(property), _) => segments.push(PathSegment::Key(property.clone())),
                (false, _, Some(unknown)) => segments.push(PathSegment::Key(unknown.clone())),
                (true, None, _) => return None,
                (false, _, None) => {}
            }
            let source = source_path(&segments, load.wrapped);
            let locator = self
                .locators
                .entry(key.as_str())
                .or_insert_with(|| SourceLocator::new(load.format, &load.text, load.table.as_ref()));
            let place = locator.place(source, missing);
            Some(DataLocation {
                file: load.identity.read_set_entry(),
                pointer: Some(json_pointer(source)),
                line: place.position.map(|(line, _)| line),
                column: place.position.map(|(_, column)| column),
                byte_offset: None,
            })
        });
        Some(ProblemExpression { span: metadata.span.clone(), data })
    }
}

/// `pointer`'s segments as the keys and indexes of `value`, or `None` when
/// `value` does not hold that path.
fn typed_segments(value: &serde_json::Value, pointer: &[String]) -> Option<Vec<simplified_schema::coercion::PathSegment>> {
    use simplified_schema::coercion::PathSegment;
    let mut node = value;
    let mut segments = Vec::with_capacity(pointer.len());
    for segment in pointer {
        match node {
            serde_json::Value::Array(items) => {
                let index: usize = segment.parse().ok()?;
                node = items.get(index)?;
                segments.push(PathSegment::Index(index));
            }
            serde_json::Value::Object(members) => {
                node = members.get(segment)?;
                segments.push(PathSegment::Key(segment.clone()));
            }
            _ => return None,
        }
    }
    Some(segments)
}

/// The path in the parsed document of the value at `path` of the loaded
/// value: a wrapped scalar's index 0 is the document's root, and a later
/// index, which the document does not hold, is placed at the root too.
fn source_path(path: &[simplified_schema::coercion::PathSegment], wrapped: bool) -> &[simplified_schema::coercion::PathSegment] {
    match path.split_first() {
        Some((simplified_schema::coercion::PathSegment::Index(0), rest)) if wrapped => rest,
        Some((simplified_schema::coercion::PathSegment::Index(_), _)) if wrapped => &path[..0],
        _ => path,
    }
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
        MarkdownError::Interpolation { source, cause, expression, .. } => (
            expression_code(cause),
            match spread_operand_source(source, expression, cause) {
                Some((source, range)) => {
                    span = Some(range);
                    Some(source)
                }
                None => proven(Some(source.as_ref())),
            },
        ),
        // A target failure's code is its own; its site, when it names one,
        // is the inclusion that named the target.
        MarkdownError::Transclusion(located) => {
            let inner = located.without_site();
            (
            transclusion_code(inner),
            match inner {
                _ if let Some(site) = located.inclusion_site() => inclusion_site_source(site, &mut span),
                TransclusionError::ConditionEval { ctx, authored, .. }
                | TransclusionError::ConditionParse { ctx, authored, .. } => condition_source(ctx, authored.as_ref(), &mut span),
                TransclusionError::ParseDirective { ctx, line, authored, .. } => {
                    parse_failure_source(ctx, *line, authored.as_ref(), &mut span)
                }
                // The directive's line is in the current body, so only the
                // file is authored.
                TransclusionError::InvalidReference { ctx, .. }
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
            )
        }
        MarkdownError::PageBlock(inner) => match inner.as_ref() {
            PageBlockError::Condition(condition) => match condition.as_ref() {
                ConditionError::Parse { ctx, authored, .. } => {
                    (ComposeWarning::EXPRESSION_PARSE_FAILURE_CODE, condition_source(ctx, authored.as_ref(), &mut span))
                }
                ConditionError::Eval { ctx, authored, .. } => {
                    (ComposeWarning::EXPRESSION_EVALUATION_FAILURE_CODE, condition_source(ctx, authored.as_ref(), &mut span))
                }
            },
            // The parser counts lines in the current body: published only
            // where the stage placed them in the file.
            PageBlockError::ParseDirective { ctx, authored, .. }
            | PageBlockError::UnmatchedEnd { ctx, authored, .. }
            | PageBlockError::UnterminatedBlock { ctx, authored, .. } => {
                (DIRECTIVE_PARSE_CODE, condition_source(ctx, authored.as_ref(), &mut span))
            }
        },
        MarkdownError::Collection(inner) => (inner.code(), proven(inner.site().map(|site| site.source.as_ref()))),
        MarkdownError::ShellBlock(inner) => (SHELL_FAILURE_CODE, shell_block_source(inner, &mut span)),
        MarkdownError::Reference(inner) => (reference_error_code(inner), reference_error_source(inner, &mut span)),
        MarkdownError::ParameterBinding { .. } | MarkdownError::CallerFileClassificationChanged { .. } => {
            (PARAMETER_BINDING_CODE, None)
        }
        MarkdownError::Setter { .. } => (SETTER_INVALID_CODE, None),
        // The document holding the directive, which for a transcluded child
        // is not the composed one, at the directive's opener when its line is
        // authored. A context identity is no file.
        MarkdownError::TocLinking(inner) => {
            (toc_linking_code(inner), directive_source(inner.document(), inner.authored(), &mut span))
        }
        MarkdownError::FileLinks(inner) => {
            (file_links_code(inner), directive_source(inner.document(), inner.authored(), &mut span))
        }
        MarkdownError::Normalization(inner) => (normalization_code(inner), None),
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
            // A target's file-reference failure is a reference check that
            // could not run, as the reference graph reports it.
            MarkdownError::Transclusion(inner) => inner.resolution_failure().map(|failure| ReferenceDetails {
                failure: Some(resolution_failure_name(failure).to_string()),
                ..Default::default()
            }),
            _ => None,
        },
        data_location: None,
        grants_needed: None,
        schema_default: None,
        related: Vec::new(),
        hint: None,
        notes: Vec::new(),
        trail: crate::findings::Trail::default(),
    }
}

/// The code of a transclusion failure, past any inclusion site.
pub(crate) fn transclusion_code(error: &TransclusionError) -> &'static str {
    match error.without_site() {
        TransclusionError::ParseDirective { .. } => DIRECTIVE_PARSE_CODE,
        TransclusionError::ConditionEval { .. } => ComposeWarning::EXPRESSION_EVALUATION_FAILURE_CODE,
        TransclusionError::ConditionParse { .. } => ComposeWarning::EXPRESSION_PARSE_FAILURE_CODE,
        TransclusionError::InvalidReference { .. } | TransclusionError::FileReference(_) => REFERENCE_FAILURE_CODE,
        TransclusionError::Io(_) if error.resolution_failure() == Some(biscuit_file::ResolutionFailure::NoMatch) => {
            crate::markdown::compose::policy::MISSING_TRANSCLUSION_CODE
        }
        TransclusionError::Io(_) => INPUT_READ_CODE,
        _ => TRANSCLUSION_FAILURE_CODE,
    }
}

/// The code of a reference-analysis failure.
fn reference_error_code(error: &ReferenceError) -> &'static str {
    match error {
        ReferenceError::ParseDirective { .. } => DIRECTIVE_PARSE_CODE,
        _ => REFERENCE_FAILURE_CODE,
    }
}

/// Where a reference-analysis failure is authored: a directive's opener or a
/// frontmatter key's line when proven, otherwise its file.
fn reference_error_source(error: &ReferenceError, span: &mut Option<std::ops::Range<usize>>) -> Option<DiagnosticSource> {
    match error {
        ReferenceError::ParseDirective { ctx, line, authored, .. } => {
            parse_failure_source(ctx, *line, authored.as_ref(), span)
        }
        _ => None,
    }
}

/// A `::toc-linking` or `::file-links` failure's `document`, at the
/// directive's `authored` opener when it is proven.
fn directive_source(
    document: Option<&Path>,
    authored: Option<&crate::markdown::types::AuthoredSpan>,
    span: &mut Option<std::ops::Range<usize>>,
) -> Option<DiagnosticSource> {
    let file = document.and_then(named)?;
    Some(match authored {
        Some(authored) => {
            *span = Some(authored.range());
            DiagnosticSource::at(Some(file), authored.line(), authored.column())
        }
        None => DiagnosticSource::file_only(Some(file)),
    })
}

/// A reference-analysis failure that reached the caller without a
/// [`MarkdownError`] around it (a `FileTree` or validator boundary), with
/// `message`, its outermost error's text.
fn reference_diagnostic(error: &ReferenceError, message: String) -> Diagnostic {
    let mut span = None;
    let source = reference_error_source(error, &mut span);
    Diagnostic {
        source,
        span,
        reference_details: reference_error_details(error),
        ..unlocated(reference_error_code(error), Severity::Error, message)
    }
}

/// The file that wrote a failed inclusion, at its authored position when it
/// is proven.
fn inclusion_site_source(
    site: &crate::markdown::compose::InclusionSite,
    span: &mut Option<std::ops::Range<usize>>,
) -> Option<DiagnosticSource> {
    let file = named(site.file());
    match site.authored() {
        Some(authored) => {
            *span = Some(authored.range());
            Some(DiagnosticSource::at(file, authored.line(), authored.column()))
        }
        None => file_only(file),
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

/// A shell failure's file and, where it is proven, its line: a frontmatter
/// command's key line, or the authored position composition placed a body or
/// shell-block command at ([`ShellCommandOrigin::Authored`]). An unplaced body
/// or shell-block origin counts lines in the current body, so it names only the
/// file, and a command with no file of its own keeps its enclosing block's.
fn shell_source(error: &ShellExpansionError, span: &mut Option<std::ops::Range<usize>>) -> Option<DiagnosticSource> {
    if let ShellExpansionError::PolicyIo { path, .. } = error {
        return file_only(named(path));
    }
    let file = context_file(error.context()?)?;
    let line = match error.origin() {
        Some(ShellCommandOrigin::Frontmatter { line, .. }) => *line,
        Some(ShellCommandOrigin::Authored { at, .. }) => {
            *span = Some(at.range());
            return Some(DiagnosticSource::at(Some(file), at.line(), at.column()));
        }
        _ => None,
    };
    Some(DiagnosticSource { line, ..DiagnosticSource::file_only(Some(file)) })
}

/// A shell-block failure's file, at its command's authored line when
/// composition placed the command ([`shell_source`]), or at the authored
/// position a parse failure was placed at.
fn shell_block_source(error: &ShellBlockError, span: &mut Option<std::ops::Range<usize>>) -> Option<DiagnosticSource> {
    let (file, command) = match error {
        ShellBlockError::Parse { source_file, authored: Some(at), .. }
        | ShellBlockError::Unterminated { source_file, authored: Some(at), .. } => {
            let file = source_file.as_deref().and_then(named)?;
            *span = Some(at.range());
            return Some(DiagnosticSource::at(Some(file), at.line(), at.column()));
        }
        ShellBlockError::Parse { source_file, .. } | ShellBlockError::Unterminated { source_file, .. } => (source_file, None),
        ShellBlockError::Command { source_file, source, .. } => (source_file, Some(source)),
    };
    if let Some(command) = command
        && matches!(command.origin(), Some(ShellCommandOrigin::Authored { .. }))
    {
        return shell_source(command, span);
    }
    file_only(file.as_deref().and_then(named)).or_else(|| command.and_then(|command| shell_source(command, span)))
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
        // An invalid `permissions` block reports its own located problems.
        ShellExpansionError::Config(config) => config_error_diagnostics(config),
        _ => vec![shell_record(SHELL_FAILURE_CODE, error)],
    }
}

/// The file a slot was written in and its text, as `failure` kept them
/// ([`simplified_schema::SlotResolutionFailure::slot_text`]).
fn failure_text(
    failure: &simplified_schema::SlotResolutionFailure,
    slot: &simplified_schema::simplified::Slot,
) -> Option<SlotText> {
    failure.slot_text(slot).map(|(file, text)| (file.map(Path::to_path_buf), std::sync::Arc::clone(text)))
}

/// The file a slot's spans point into (`None`: in-memory text) and that
/// file's text.
type SlotText = (Option<PathBuf>, std::sync::Arc<str>);

/// The diagnostics of a schema-slot resolution that produced no schema, one
/// per refused or failed slot.
///
/// Each record's primary position is the slot as authored in its defining
/// file: an imported or referenced schema's own file, a trigger file for
/// its inline payload, or the document an inline `$schema` is written in.
/// `text_of` returns that file and its text as the resolution kept them;
/// a slot with no text keeps its origin as the file, and its position
/// stays `null`. A failed slot keeps its typed cause: a `load_data` failure
/// keeps its `dm.data.*` code and its `data_location`, a missing or
/// unreadable file its reference fields.
pub(crate) fn slot_resolution_diagnostics(
    error: &simplified_schema::slots::SlotResolutionError,
    text_of: &dyn Fn(&simplified_schema::simplified::Slot) -> Option<SlotText>,
) -> Vec<Diagnostic> {
    slot_resolution_records(error, text_of).into_iter().map(|(_, record)| record).collect()
}

/// The records of a template's schema error, each with the slot it belongs
/// to (`None`: a record of the whole resolution, such as a cancellation):
/// one per refused or failed slot of a failed resolution, and one
/// `dm.schema.slot_pending` per slot of a template not resolved for this
/// run. `None` for any other schema error.
///
/// Each record's `source` and `span` are the slot in its defining file,
/// when the resolution kept that file's text ([`template_slot_text`]), as
/// [`template_diagnostics`] reports them. An editor publishes a record of a
/// slot written in its own document at that span, once it has checked
/// that the text is the one it is showing.
pub fn template_slot_records(
    error: &simplified_schema::SchemaError,
) -> Option<Vec<(Option<simplified_schema::simplified::Slot>, Diagnostic)>> {
    use crate::markdown::schemas::slots::codes::SLOT_PENDING_CODE;
    match error {
        simplified_schema::SchemaError::SlotResolution(failure) => {
            Some(slot_resolution_records(&failure.error, &|slot| failure_text(failure, slot)))
        }
        simplified_schema::SchemaError::UnresolvedSlots(unresolved) => Some(
            unresolved
                .slots
                .iter()
                .map(|slot| {
                    let text_of = |slot: &simplified_schema::simplified::Slot| template_slot_text(error, slot);
                    let message = pending_slot_message(error, unresolved, slot);
                    (Some(slot.clone()), slot_record(slot, &slot.span, SLOT_PENDING_CODE, message, &text_of))
                })
                .collect(),
        ),
        _ => None,
    }
}

/// The pending report of `slot`, one of `unresolved`'s: `error`'s message,
/// naming the property that holds the slot and whether its whole type is
/// pending (a template type) or one constraint is (a template constraint),
/// when the kept template shows it.
fn pending_slot_message(
    error: &simplified_schema::SchemaError,
    unresolved: &simplified_schema::UnresolvedSlots,
    slot: &simplified_schema::simplified::Slot,
) -> String {
    match crate::markdown::schemas::slots::pending_report::pending_context(unresolved, slot) {
        Some(context) => format!("{context}: {error}"),
        None => error.to_string(),
    }
}

/// The file `slot` of the template `error` reports was written in, and the
/// text its spans are byte ranges of, as the resolution kept them: a schema
/// file it read, or the document or trigger file an inline `$schema` is
/// written in. `None` when no kept text proves where the slot is; nothing
/// is read to find out.
pub fn template_slot_text(
    error: &simplified_schema::SchemaError,
    slot: &simplified_schema::simplified::Slot,
) -> Option<(Option<PathBuf>, std::sync::Arc<str>)> {
    match error {
        simplified_schema::SchemaError::SlotResolution(failure) => failure_text(failure, slot),
        simplified_schema::SchemaError::UnresolvedSlots(unresolved) => unresolved
            .pending
            .as_ref()?
            .slot_text(slot)
            .map(|(file, text)| (file.map(Path::to_path_buf), std::sync::Arc::clone(text))),
        _ => None,
    }
}

/// One record of `slot` at `span` (a range of the slot's), positioned in
/// the text `text_of` gives when that text proves it; otherwise naming only
/// the slot's origin.
fn slot_record(
    slot: &simplified_schema::simplified::Slot,
    span: &simplified_schema::span::SourceSpan,
    code: &str,
    message: String,
    text_of: &dyn Fn(&simplified_schema::simplified::Slot) -> Option<SlotText>,
) -> Diagnostic {
    let (file, text) = match text_of(slot) {
        Some((file, text)) => (file, Some(text)),
        None => (slot.origin.clone(), None),
    };
    let span = text.as_deref().and_then(|text| proven_slot_span(text, slot, span));
    let position = span.as_ref().zip(text.as_deref()).map(|(span, text)| crate::config::load::line_column(text, span.start));
    Diagnostic {
        source: Some(DiagnosticSource {
            file,
            line: position.as_ref().map(|position| position.line),
            column: position.as_ref().map(|position| position.column),
            section: None,
            inserted: false,
        }),
        span,
        ..unlocated(code, Severity::Error, message)
    }
}

fn slot_resolution_records(
    error: &simplified_schema::slots::SlotResolutionError,
    text_of: &dyn Fn(&simplified_schema::simplified::Slot) -> Option<SlotText>,
) -> Vec<(Option<simplified_schema::simplified::Slot>, Diagnostic)> {
    use crate::markdown::schemas::slots::codes;
    use simplified_schema::slots::SlotResolutionError;
    let at = |slot: &simplified_schema::simplified::Slot, span: &simplified_schema::span::SourceSpan, code: &str, message: String| {
        slot_record(slot, span, code, message, text_of)
    };
    match error {
        SlotResolutionError::Refused(refused) => refused
            .iter()
            .map(|(slot, refusal)| {
                let span = refusal.span.clone().unwrap_or_else(|| slot.expression_span.clone());
                (Some(slot.clone()), at(slot, &span, refusal.code.unwrap_or(codes::SLOT_FAILED_CODE), refusal.message.clone()))
            })
            .collect(),
        SlotResolutionError::Failed(failed) => failed
            .iter()
            .flat_map(|(slot, failure)| {
                let record = at(slot, &slot.span, failure.code.unwrap_or(codes::SLOT_FAILED_CODE), failure.to_string());
                with_expression_cause(record, failure.downcast_ref::<ExpressionError>().map(ExpressionError::without_callbacks))
                    .into_iter()
                    .map(|record| (Some(slot.clone()), record))
            })
            .collect(),
        SlotResolutionError::Schema(schema) => match schema.as_ref() {
            simplified_schema::SchemaError::SlotResult { slot, kind, .. } => {
                vec![(Some(slot.as_ref().clone()), at(slot, &slot.span, codes::slot_result_code(kind), kind.to_string()))]
            }
            other => vec![(None, unlocated(codes::SLOT_FAILED_CODE, Severity::Error, other.to_string()))],
        },
        SlotResolutionError::Canceled => vec![(None, unlocated(codes::SLOT_CANCELED_CODE, Severity::Error, error.to_string()))],
        _ => vec![(None, unlocated(codes::SLOT_FAILED_CODE, Severity::Error, error.to_string()))],
    }
}

/// `span`, a range of `slot`'s in `text`, when `text` proves it: the slot's
/// own bytes must be a `{{ … }}` there. A range inside the expression (a
/// refused read) is offset from the expression's start, which holds only
/// while the authored expression is the expression's text; through a YAML
/// escape or fold it is not, and the whole expression is the proven range.
fn proven_slot_span(
    text: &str,
    slot: &simplified_schema::simplified::Slot,
    span: &simplified_schema::span::SourceSpan,
) -> Option<simplified_schema::span::SourceSpan> {
    let delimited = text.get(slot.span.clone()).is_some_and(|whole| whole.starts_with("{{") && whole.ends_with("}}"));
    if !delimited {
        return None;
    }
    let inside = *span != slot.span && *span != slot.expression_span;
    if inside && text.get(slot.expression_span.clone()) != Some(slot.expression.as_str()) {
        return Some(slot.expression_span.clone());
    }
    text.get(span.clone()).map(|_| span.clone())
}

fn shell_record(code: &str, error: &ShellExpansionError) -> Diagnostic {
    let mut span = None;
    let source = shell_source(error, &mut span);
    Diagnostic { source, span, ..unlocated(code, Severity::Error, error.to_string()) }
}

fn expression_code(cause: &ExpressionError) -> &'static str {
    match cause.without_callbacks() {
        ExpressionError::IterationLimit(error) => error.kind.code(),
        ExpressionError::Parse(_) | ExpressionError::MalformedLiteralToken(_) => {
            ComposeWarning::EXPRESSION_PARSE_FAILURE_CODE
        }
        ExpressionError::UnknownFunction { .. } => UNKNOWN_FUNCTION_CODE,
        ExpressionError::InvalidCallback { .. } => simplified_schema::expression::parser::INVALID_CALLBACK_CODE,
        ExpressionError::InvalidSpreadOperand { .. } => super::expression::INVALID_SPREAD_OPERAND_CODE,
        _ => ComposeWarning::EXPRESSION_EVALUATION_FAILURE_CODE,
    }
}

/// The authored position and range of the spread operand an
/// [`ExpressionError::InvalidSpreadOperand`] names, inside the anchored
/// `{{ … }}` holding `expression`. `None` for any other cause, an unplaced
/// site, or a construct whose text does not hold the expression as written
/// (an escaped YAML scalar), which keeps the construct's own position.
fn spread_operand_source(
    source: &SourceRef,
    expression: &str,
    cause: &ExpressionError,
) -> Option<(DiagnosticSource, std::ops::Range<usize>)> {
    let ExpressionError::InvalidSpreadOperand { site, .. } = cause.without_callbacks() else {
        return None;
    };
    let SourceRef::OnDiskSpan { context, span } = source else {
        return None;
    };
    let construct = span.range();
    let start = construct.start + context.content.get(construct)?.find(expression)?;
    let operand = super::expression::spread_operand_span(&super::expression::parse_spanned(expression).ok()?, site.ordinal()?)?;
    let range = start + operand.start..start + operand.end;
    let (line, column) = super::expression::one_based_position(&context.content, range.start)?;
    Some((DiagnosticSource::at(context_file(context), line, column), range))
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

/// A parse failure's file, at the body directive's `authored` opener when
/// that is proven, or at its line when that line lies in the frontmatter
/// `ctx` holds: a `prologue` or `epilogue` field's error names its key's line
/// in the file as written (`parse_frontmatter_refs`), which no stage
/// rewrites. Any other line counts the current body and stays unpublished.
fn parse_failure_source(
    ctx: &SourceContext,
    line: usize,
    authored: Option<&crate::markdown::types::AuthoredSpan>,
    span: &mut Option<std::ops::Range<usize>>,
) -> Option<DiagnosticSource> {
    let file = context_file(ctx)?;
    if let Some(authored) = authored {
        *span = Some(authored.range());
        return Some(DiagnosticSource::at(Some(file), authored.line(), authored.column()));
    }
    let frontmatter_lines = ctx.frontmatter.as_ref().map_or(0, |range| ctx.content[..range.end].lines().count());
    Some(if (1..=frontmatter_lines).contains(&line) {
        DiagnosticSource { line: Some(line), ..DiagnosticSource::file_only(Some(file)) }
    } else {
        DiagnosticSource::file_only(Some(file))
    })
}

/// The code a tolerated `::toc-linking` or `::file-links` failure keeps as a
/// warning: the one its failure reports, so recovering never changes how an
/// author tells failures apart. `None` for any other error.
pub(crate) fn directive_failure_code(error: &MarkdownError) -> Option<&'static str> {
    match error {
        MarkdownError::TocLinking(inner) => Some(toc_linking_code(inner)),
        MarkdownError::FileLinks(inner) => Some(file_links_code(inner)),
        _ => None,
    }
}

/// The code of a `::toc-linking` failure: one per option it rejects, so an
/// author can tell a bad `level=` from a bad `cleanup=` or filter.
fn toc_linking_code(error: &super::TocLinkingError) -> &'static str {
    use super::TocLinkingError;
    match error {
        TocLinkingError::ParseDirective { .. } => DIRECTIVE_PARSE_CODE,
        TocLinkingError::InvalidLevel { .. } => TOC_LINKING_INVALID_LEVEL_CODE,
        TocLinkingError::InvalidCleanupService { .. } => TOC_LINKING_INVALID_CLEANUP_CODE,
        TocLinkingError::InvalidGlob { .. } => TOC_LINKING_INVALID_FILTER_CODE,
        TocLinkingError::Unresolved { .. } => REFERENCE_FAILURE_CODE,
        TocLinkingError::Io(_) => INPUT_READ_CODE,
    }
}

/// The code of a heading normalization failure.
pub(crate) fn normalization_code(error: &crate::markdown::normalize::NormalizationError) -> &'static str {
    use crate::markdown::normalize::NormalizationError;
    match error {
        NormalizationError::LevelOverflow { .. } => NORMALIZATION_LEVEL_OVERFLOW_CODE,
        NormalizationError::ValidationFailed(_) => NORMALIZATION_VALIDATION_FAILED_CODE,
    }
}

/// The code of a `::file-links` failure.
pub(crate) fn file_links_code(error: &super::FileLinksError) -> &'static str {
    use super::FileLinksError;
    match error {
        FileLinksError::ParseDirective { .. } => DIRECTIVE_PARSE_CODE,
        FileLinksError::InvalidDepth { .. } => FILE_LINKS_INVALID_DEPTH_CODE,
        FileLinksError::MissingSourceContext { .. } => FILE_LINKS_MISSING_SOURCE_CODE,
        FileLinksError::TargetNotFound { .. } => REFERENCE_FAILURE_CODE,
        FileLinksError::TargetNotDirectory { .. } => FILE_LINKS_NOT_A_DIRECTORY_CODE,
        FileLinksError::GlobReference { source, .. } if matches!(source.as_ref(), biscuit_file::GlobReferenceError::Io { .. }) => {
            INPUT_READ_CODE
        }
        FileLinksError::GlobReference { .. } => FILE_LINKS_INVALID_GLOB_CODE,
        FileLinksError::Unreadable { .. } | FileLinksError::Io(_) => INPUT_READ_CODE,
    }
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
        grants_needed: None,
        schema_default: None,
        related: Vec::new(),
        hint: None,
        notes: Vec::new(),
        trail: crate::findings::Trail::default(),
    }
}
