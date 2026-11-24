//! Diagnostics drawn from the slot worker's current observation of a
//! document's eligible whole-value properties ([`crate::slot_jobs::PropertyView`]).
//!
//! Only a current observation speaks: a stale, refreshing, or pending one
//! contributes nothing, so no verdict and no missing-member error is ever
//! drawn from a value the editor has not seen. For each eligible property:
//!
//! - a destination schema problem of its observed value is an error with the
//!   ordinary schema code, ranged at the authored `{{ … }}`; when the value
//!   is a direct load, the problem's data-file position is attached as
//!   `relatedInformation` (a local file) and as `data.data_location` (the
//!   structured record composition reports);
//! - an executed, unrecovered read of a provably missing local file is a
//!   warning, `dm.expression.evaluation_failure`, with the file-reference
//!   details composition reports (composition fails the document there);
//! - any other evaluation failure (malformed data, a contract violation)
//!   keeps its own code and data location, unless a passive diagnostic with
//!   the same code already marks the site.

use darkmatter::markdown::compose::expression::DataLocation;
use darkmatter::markdown::compose::expression::data::DataSourceIdentity;
use darkmatter::markdown::compose::report_diagnostics::{data_locations, error_diagnostics};
use darkmatter::markdown::schemas::ObservedVerdict;
use darkmatter::markdown::schemas::slots::observe::PropertyOutcome;
use lsp_types::{Diagnostic, DiagnosticRelatedInformation, DiagnosticSeverity, Location, NumberOrString, Range};
use simplified_schema::ValidationProblemCode;

use super::codes::{code, source};
use super::frontmatter::{diagnostic, schema_origin_related, schema_problem_data};
use crate::overlay::frontmatter::FrontmatterAst;
use crate::overlay::properties::located_sites;
use crate::overlay::schema::SchemaBundle;
use crate::providers::DocumentContext;
use crate::slot_jobs::PropertyView;

/// The code composition gives an expression's evaluation failure.
const EVALUATION_FAILURE: &str = darkmatter::markdown::compose::ComposeWarning::EXPRESSION_EVALUATION_FAILURE_CODE;

/// Appends the observation's diagnostics for `ctx`'s eligible properties.
pub(crate) fn property_diagnostics(
    ctx: &DocumentContext,
    ast: &FrontmatterAst,
    bundle: Option<&SchemaBundle>,
    out: &mut Vec<Diagnostic>,
) {
    let Some(view) = ctx.overlay.and_then(|overlay| overlay.properties.as_deref()) else {
        return;
    };
    for located in located_sites(ctx.text, ast).into_iter().filter(|located| located.site.eligible) {
        let key = &located.site.key;
        let Some(range) = ctx.source_map.byte_range_to_lsp(located.span.clone()) else {
            continue;
        };
        match view.observations.get(key) {
            Some(PropertyOutcome::Observed { .. }) => {
                if let (Some(ObservedVerdict::Judged(problems)), Some(bundle)) = (view.verdicts.get(key), bundle) {
                    verdict_diagnostics(ctx, view, bundle, problems, range, out);
                }
            }
            Some(PropertyOutcome::Missing(error)) => {
                out.push(failure_diagnostic(error, range, DiagnosticSeverity::WARNING, Some(EVALUATION_FAILURE)));
            }
            Some(PropertyOutcome::Failed(error)) => {
                let found = failure_diagnostic(error, range, DiagnosticSeverity::ERROR, None);
                let duplicate = out.iter().any(|known| {
                    known.code == found.code && known.range.start >= range.start && known.range.end <= range.end
                });
                if !duplicate {
                    out.push(found);
                }
            }
            Some(PropertyOutcome::Pending(_)) | None => {}
        }
    }
}

fn verdict_diagnostics(
    ctx: &DocumentContext,
    view: &PropertyView,
    bundle: &SchemaBundle,
    problems: &[simplified_schema::ValidationProblem],
    range: Range,
    out: &mut Vec<Diagnostic>,
) {
    let locations = data_locations(problems, &view.origins);
    for (problem, location) in problems.iter().zip(locations) {
        let (code_value, severity) = observed_code(problem.code, ctx.config.schema.strict);
        let mut found = diagnostic(range, severity, source::SCHEMA, code_value, problem.message.clone());
        let mut related = schema_origin_related(ctx, bundle, problem).unwrap_or_default();
        let mut data = schema_problem_data(problem);
        if let Some(location) = location {
            if let Some(information) = data_related(ctx, view, problem, &location) {
                related.push(information);
            }
            data["data_location"] = serde_json::to_value(&location).unwrap_or_default();
        }
        found.related_information = (!related.is_empty()).then_some(related);
        found.data = Some(data);
        out.push(found);
    }
}

/// The code and severity of an observed value's schema problem. Unlike an
/// authored property's, a missing required member is a real error: the
/// value was evaluated, and nothing later supplies it.
fn observed_code(problem: ValidationProblemCode, strict: bool) -> (&'static str, DiagnosticSeverity) {
    match problem {
        ValidationProblemCode::MissingRequired => (code::SCHEMA_MISSING_REQUIRED, DiagnosticSeverity::ERROR),
        ValidationProblemCode::TypeMismatch => (code::SCHEMA_TYPE_MISMATCH, DiagnosticSeverity::ERROR),
        ValidationProblemCode::ConstraintViolation => (code::SCHEMA_CONSTRAINT, DiagnosticSeverity::ERROR),
        ValidationProblemCode::UnknownKey => {
            (code::SCHEMA_UNKNOWN_KEY, if strict { DiagnosticSeverity::ERROR } else { DiagnosticSeverity::WARNING })
        }
        ValidationProblemCode::InvalidFileReference => (code::SCHEMA_INVALID_FILE_REFERENCE, DiagnosticSeverity::ERROR),
        ValidationProblemCode::KindCheck => (code::SCHEMA_KIND_CHECK, DiagnosticSeverity::ERROR),
    }
}

/// The related location of a problem at `location` in a local data file,
/// converted with the text the observation read (so an unsaved buffer's
/// position is the one shown) into the client's position encoding.
fn data_related(
    ctx: &DocumentContext,
    view: &PropertyView,
    problem: &simplified_schema::ValidationProblem,
    location: &DataLocation,
) -> Option<DiagnosticRelatedInformation> {
    let key = problem.instance_path.first()?;
    let load = view.origins.get(key)?.load.as_ref()?;
    let DataSourceIdentity::Local(path) = &load.identity else {
        return None;
    };
    let (line, column) = (location.line?, location.column?);
    let offset = char_position_offset(&load.text, line, column)?;
    let uri = crate::workspace::file_path_to_uri(path)?;
    let map = crate::source_map::SourceMap::new(uri.clone(), 0, ctx.source_map.encoding(), std::sync::Arc::clone(&load.text));
    let position = map.byte_to_lsp(offset)?;
    Some(DiagnosticRelatedInformation {
        location: Location::new(uri, Range::new(position, position)),
        message: match &location.pointer {
            Some(pointer) if !pointer.is_empty() => format!("the loaded value at `{pointer}`"),
            _ => "the loaded value".to_string(),
        },
    })
}

/// The byte offset of the 1-based `line` and character `column` in `text`.
fn char_position_offset(text: &str, line: usize, column: usize) -> Option<usize> {
    let start = if line <= 1 {
        0
    } else {
        text.match_indices('\n').nth(line - 2).map(|(index, _)| index + 1)?
    };
    let rest = &text[start..];
    let within = rest.char_indices().nth(column.checked_sub(1)?).map_or(rest.len(), |(index, _)| index);
    Some(start + within)
}

/// The diagnostic of an evaluation failure, with composition's record
/// (code, message, data location, file-reference details) as its `data`.
fn failure_diagnostic(
    error: &darkmatter::markdown::compose::expression::ExpressionError,
    range: Range,
    severity: DiagnosticSeverity,
    code_override: Option<&str>,
) -> Diagnostic {
    let record = error_diagnostics(error).into_iter().next();
    let code_value = code_override.map(str::to_string).or_else(|| record.as_ref().map(|record| record.code.clone()));
    let message = record.as_ref().map_or_else(|| error.to_string(), |record| record.message.clone());
    let data = record.and_then(|record| {
        let mut value = serde_json::to_value(&record).ok()?;
        if let Some(object) = value.as_object_mut() {
            object.remove("source");
            object.remove("span");
        }
        Some(value)
    });
    Diagnostic {
        range,
        severity: Some(severity),
        code: code_value.map(NumberOrString::String),
        source: Some(source::COMPOSE.to_string()),
        message,
        data,
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_character_position_is_found_past_multibyte_text() {
        let text = "- name: Ádá\n  age: 36\n- name: Grace\n  age: \"old\"\n";
        let offset = char_position_offset(text, 4, 8).unwrap();
        assert_eq!(&text[offset..offset + 5], "\"old\"");
        assert_eq!(char_position_offset(text, 1, 1), Some(0));
        assert_eq!(char_position_offset(text, 9, 1), None);
    }
}
