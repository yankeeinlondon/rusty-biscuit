//! Diagnostics for a whole YAML document (`.yaml`, `.yml`), projected from
//! the library's judgment of it against the schema it selects
//! ([`YamlDocumentOverlay`]).
//!
//! - A document that does not load gets one located parse error and nothing
//!   else: no schema is judged. It is the loader's `dm.frontmatter.yaml_parse`,
//!   unless a schema envelope claims the text and reports the failure itself
//!   (under its own `dm.schema.*` code).
//! - A `kind` that cannot select a definition (claimed twice, reserved by a
//!   failed definition, or unavailable) is one `dm.schema.prepare` error on
//!   the `kind` value, with the definitions involved as related information.
//!   A root `$schema` that does not resolve is ranged and related exactly as
//!   in Markdown frontmatter: on the `$schema` value, with a failure inside a
//!   referenced definition as related information in that file.
//! - Each problem of the selected schema's report is coded and ranged as a
//!   frontmatter problem is (a missing key ranges its parent mapping, an
//!   unknown key the key, a value problem the value), except that a missing
//!   required key is always an error, and a kind check
//!   (`dm.schema.kind_check`) is ranged where the library located it.
//!
//! The selected schema is the only judge of the document's shape. Only a
//! claimed schema kind makes a document a schema document, so a root
//! `$schema` with no such `kind` never adds envelope findings: its other
//! fields are data. Beside the report, a `kind: schema` / `kind: yaml-schema`
//! document keeps the findings of reading its declarations (as its selected
//! definition accepted it, never admitted again), and each problem is
//! published once:
//!
//! - a definition problem is the report's: the document's own reading, which
//!   cannot interpret a shape the definition rejects, is not published
//!   beside it ([`reports_definition_problems`]);
//! - once that reading has reported any other error, which locates the
//!   offending token, the report's coarser meta-type problem for the same
//!   text is not repeated;
//! - a schema document's own parse or named-type failure (ranged at the
//!   reference, with its declarations as related information) stands in for
//!   the `schema` / `yaml-schema` kind check reporting the same failure;
//! - a schema-trigger file's strict envelope error is published only while no
//!   definition has judged the document; once one has, its report (with the
//!   `schema-trigger` kind check located on the offending node) is the only
//!   verdict.

use darkmatter::markdown::schemas::ValidationReport;
use lsp_types::{Diagnostic, DiagnosticRelatedInformation, DiagnosticSeverity, Location, Range};
use simplified_schema::{KindSelectionCause, SchemaError, ValidationProblemCode};

use crate::diagnostics::codes::{code, source};
use crate::diagnostics::frontmatter::{
    ProblemGrading, diagnostic, is_meta_type_problem, schema_advisory_diagnostics, schema_prepare_diagnostic,
    schema_problem_diagnostics,
};
use crate::overlay::{SuggestionState, YamlDocumentOutcome, YamlDocumentOverlay};
use crate::providers::DocumentContext;
use crate::workspace::file_path_to_uri;

/// Appends the diagnostics of the YAML document in `ctx` to `out`.
pub(crate) fn diagnostics(ctx: &DocumentContext, yaml: &YamlDocumentOverlay, out: &mut Vec<Diagnostic>) {
    let Some(overlay) = ctx.overlay else {
        return;
    };
    match &yaml.outcome {
        // A schema envelope that claims the text reports its own parse
        // failure, under the code its clients already key on.
        YamlDocumentOutcome::LoadFailed(_) if envelope_error(&overlay.suggestions) => {}
        YamlDocumentOutcome::LoadFailed(error) => out.push(load_diagnostic(ctx, error)),
        YamlDocumentOutcome::Unvalidated | YamlDocumentOutcome::Pending | YamlDocumentOutcome::NoSchema => {}
        YamlDocumentOutcome::Failed(error) => match &**error {
            SchemaError::KindSelection { cause, .. } => out.push(selection_diagnostic(ctx, error, cause)),
            _ => schema_prepare_diagnostic(ctx, overlay.ast.as_deref(), error, out),
        },
        YamlDocumentOutcome::Validated(validation) => {
            let (Some(ast), Some(bundle)) = (overlay.ast.as_deref(), overlay.bundle()) else {
                return;
            };
            schema_advisory_diagnostics(ctx, ast, bundle, out);
            let standalone_reported = overlay.standalone_resolution.is_some()
                || matches!(&overlay.suggestions, SuggestionState::Standalone { error: Some(_), .. });
            let envelope_reported = !reports_definition_problems(yaml)
                && matches!(
                    &overlay.suggestions,
                    SuggestionState::Standalone { error: Some(_), .. }
                        | SuggestionState::TriggerError(_)
                        | SuggestionState::TriggerWarning(_)
                );
            let report = reported_problems(&validation.report, envelope_reported, standalone_reported);
            schema_problem_diagnostics(ctx, ast, bundle, &report, &[], ProblemGrading::YamlDocument(validation), out);
        }
    }
}

/// Whether the YAML document's `kind` selected a definition that reported
/// structural problems with the document: neither kind checks (which run
/// only once there are none) nor meta-type problems (a declaration the type
/// grammar rejects, which the envelope locates at its token). The envelope
/// parser of a shipped kind admits the document against that same
/// definition, so its error would repeat them.
pub(crate) fn reports_definition_problems(yaml: &YamlDocumentOverlay) -> bool {
    yaml.kind_route()
        && yaml.validation().is_some_and(|validation| {
            validation
                .report
                .problems
                .iter()
                .any(|problem| problem.code != ValidationProblemCode::KindCheck && !is_meta_type_problem(problem))
        })
}

/// `report` without the problems an envelope diagnostic already reports:
/// the meta-type problems of a published envelope error
/// (`envelope_reported`), and the kind checks of a standalone schema's own
/// failure (`standalone_reported`).
fn reported_problems(report: &ValidationReport, envelope_reported: bool, standalone_reported: bool) -> ValidationReport {
    let mut report = report.clone();
    report.problems.retain(|problem| {
        !(envelope_reported && is_meta_type_problem(problem))
            && !(standalone_reported && problem.code == ValidationProblemCode::KindCheck)
    });
    report
}

/// Whether a schema envelope claiming the document reported its own error.
fn envelope_error(suggestions: &SuggestionState) -> bool {
    matches!(suggestions, SuggestionState::Standalone { error: Some(_), .. } | SuggestionState::TriggerError(_))
}

/// The loader's error, ranged at the position it names.
fn load_diagnostic(ctx: &DocumentContext, error: &SchemaError) -> Diagnostic {
    let (message, span) = match error {
        SchemaError::SchemaDocument { message, span, .. } => (message.clone(), span.clone()),
        other => (other.to_string(), None),
    };
    let range = span.and_then(|span| ctx.source_map.byte_range_to_lsp(span)).unwrap_or_default();
    diagnostic(range, DiagnosticSeverity::ERROR, source::FRONTMATTER, code::FM_YAML_PARSE, message)
}

/// A `kind` that cannot select a definition, on the `kind` value, with each
/// definition file involved as related information.
fn selection_diagnostic(ctx: &DocumentContext, error: &SchemaError, cause: &KindSelectionCause) -> Diagnostic {
    let range = ctx
        .overlay
        .and_then(|overlay| overlay.ast.as_deref())
        .and_then(|ast| ast.entry_by_pointer("/kind"))
        .and_then(|entry| ctx.source_map.byte_range_to_lsp(entry.value_span.clone()))
        .unwrap_or_default();
    let code_value =
        darkmatter::markdown::schemas::slots::codes::slot_prohibition_code(error).unwrap_or(code::SCHEMA_PREPARE);
    let mut out = diagnostic(range, DiagnosticSeverity::ERROR, source::SCHEMA, code_value, error.to_string());
    let (files, message) = match cause {
        KindSelectionCause::Duplicate { files } => (files.clone(), "a definition claiming this kind"),
        KindSelectionCause::Reserved { file, .. } => (vec![file.clone()], "the failed definition claiming this kind"),
        KindSelectionCause::Unavailable { file, .. } => (vec![file.clone()], "a failed definition"),
        KindSelectionCause::RootUnreadable { root, .. } => (vec![root.clone()], "the unreadable schema root"),
    };
    // A compiled-in definition is named by its repository-relative path,
    // which no editor can open.
    let related: Vec<DiagnosticRelatedInformation> = files
        .iter()
        .filter(|file| file.is_absolute())
        .filter_map(|file| {
            Some(DiagnosticRelatedInformation {
                location: Location::new(file_path_to_uri(file)?, Range::default()),
                message: message.to_string(),
            })
        })
        .collect();
    if !related.is_empty() {
        out.related_information = Some(related);
    }
    out
}
