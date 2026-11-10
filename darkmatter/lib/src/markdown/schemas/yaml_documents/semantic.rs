//! Checks of Darkmatter's own document kinds that their definitions cannot
//! state.
//!
//! A definition states a kind's whole shape; these are the rules about
//! meaning that need Rust. They run only after the document satisfies its
//! definition, on the same loaded document, so a document with a structural
//! problem reports only that problem, and a structurally valid one reports
//! only additional problems: nothing is reported twice.
//!
//! The selected definition (a checkout's edited copy included) is the only
//! judge of shape: no check here admits the document against the compiled
//! envelope definitions or a hard-coded key list again, so a key the
//! selected definition allows is never a kind check. What the document
//! imports is a direct load, still admitted against the compiled
//! definitions.
//!
//! | Kind | Checks |
//! |---|---|
//! | `schema` | the library's parse of the accepted document without envelope admission (category entries, undeclared categories, duplicate overloads) and its named-type imports |
//! | `schema-trigger` | the `match` grammar: `$path` globs, empty or vacuous `match`, `min-match` count bounds, match-safe conditions |
//! | `yaml-schema` | the same parse; its export prepares and claims a finite set of kinds |
//!
//! Every problem is a [`ValidationProblemCode::KindCheck`] located in the
//! document's text. In the file itself it is an error; the same trigger
//! failure found while discovering triggers for a Markdown document is that
//! document's skipped-trigger warning instead (see
//! [`super::super::triggers::discovery`]). A user's own kind has no checks
//! here: its definition is its whole contract.

use std::ops::Range;
use std::path::{Path, PathBuf};

use biscuit_file::FileResolutionContext;
use simplified_schema::{
    SchemaError, SchemaSourcePath, SchemaSourcePathSegment, ValidationProblem, ValidationProblemCode,
    ValidationProblemKind,
};

use super::super::embedded::SHIPPED_SOURCES;
use super::super::triggers::envelope::{check_trigger_semantics, trigger_error_segments};
use super::load::{LoadedYaml, locating_problem};

/// The document kinds that have checks here.
pub(crate) const CHECKED_KINDS: &[&str] = &["schema", "schema-trigger", "yaml-schema"];

/// The checked kinds whose checks resolve what the document imports, so
/// read files; the `schema-trigger` checks are passive.
pub(crate) const FILE_READING_KINDS: &[&str] = &["schema", "yaml-schema"];

/// Runs `kind`'s checks over `loaded`, the document at `path` whose text is
/// `text`; `accepted` is the copy of it the selected definition accepted.
/// `schema_roots` and `context` resolve what a schema or definition imports.
/// Empty for a kind without checks.
pub(crate) fn kind_checks(
    kind: &str,
    text: &str,
    path: &Path,
    loaded: &LoadedYaml,
    accepted: &serde_json::Value,
    schema_roots: &[PathBuf],
    context: &FileResolutionContext,
) -> Vec<ValidationProblem> {
    let body = &text[loaded.offset..];
    if FILE_READING_KINDS.contains(&kind) {
        crate::note_work!(YamlKindCheck);
    }
    let findings = match kind {
        "schema-trigger" => trigger_checks(loaded),
        "schema" => schema_checks(body, path, accepted, schema_roots, context),
        "yaml-schema" => yaml_schema_checks(body, path, accepted, schema_roots, context),
        _ => Vec::new(),
    };
    findings.into_iter().map(|finding| finding.into_problem(text, loaded)).collect()
}

/// One check failure before it is located.
struct Finding {
    /// The node it is about, as instance-path segments.
    segments: Vec<String>,
    /// The exact byte range in the YAML text, when the failure carries one.
    span: Option<Range<usize>>,
    message: String,
}

impl Finding {
    fn at(segments: Vec<String>, error: &SchemaError) -> Self {
        let span = match error {
            SchemaError::SchemaDocument { span, .. } => span.clone(),
            _ => None,
        };
        Self { segments, span, message: error.to_string() }
    }

    fn into_problem(self, text: &str, loaded: &LoadedYaml) -> ValidationProblem {
        let mut problem = locating_problem(&self.segments);
        problem.message = self.message;
        problem.kind = ValidationProblemKind::Invalid;
        problem.code = ValidationProblemCode::KindCheck;
        loaded.positions.locate_all(std::slice::from_mut(&mut problem));
        if let Some(span) = self.span {
            let (line, column) = simplified_schema::line_col_of_offset(text, span.start + loaded.offset);
            problem.line = u32::try_from(line).ok();
            problem.column = u32::try_from(column).ok();
        }
        problem
    }
}

fn trigger_checks(loaded: &LoadedYaml) -> Vec<Finding> {
    match check_trigger_semantics(&loaded.value) {
        Ok(()) => Vec::new(),
        Err(error) => vec![Finding::at(trigger_error_segments(&error), &error)],
    }
}

fn schema_checks(
    body: &str,
    path: &Path,
    accepted: &serde_json::Value,
    schema_roots: &[PathBuf],
    context: &FileResolutionContext,
) -> Vec<Finding> {
    let document = match simplified_schema::parse_selected_standalone_schema_document(body, path, accepted) {
        Ok(Some(document)) => document,
        // A document its definition admitted is always recognized.
        Ok(None) => return Vec::new(),
        Err(error) => return vec![definition_failure(body, &error)],
    };
    match simplified_schema::resolve::check_standalone_named_types(&document, schema_roots, context, SHIPPED_SOURCES) {
        Ok(_) => Vec::new(),
        Err(error) => {
            let carrier = if document.local_types.is_some() || !document.exports_schema { "types" } else { "$schema" };
            vec![Finding::at(vec![carrier.to_string()], &error)]
        }
    }
}

fn yaml_schema_checks(
    body: &str,
    path: &Path,
    accepted: &serde_json::Value,
    schema_roots: &[PathBuf],
    context: &FileResolutionContext,
) -> Vec<Finding> {
    let prepared = simplified_schema::parse_selected_standalone_schema_document(body, path, accepted).and_then(|document| {
        // A document its definition admitted is always recognized.
        let Some(document) = document else { return Ok(None) };
        simplified_schema::resolve::prepare_standalone_yaml_schema(document, schema_roots, context, SHIPPED_SOURCES)
            .map(Some)
    });
    let resolved = match prepared {
        Ok(Some(resolved)) => resolved,
        Ok(None) => return Vec::new(),
        Err(error) => {
            let mut finding = definition_failure(body, &error);
            if finding.segments.is_empty() && finding.span.is_none() {
                finding.segments = vec!["$schema".to_string()];
            }
            if let SchemaError::UnresolvedSlots(unresolved) = &error {
                finding.message =
                    crate::markdown::schemas::slots::codes::slots_not_allowed_message("a kind definition", unresolved);
            }
            return vec![finding];
        }
    };
    match simplified_schema::claimed_values(&resolved, "kind") {
        Ok(_) => Vec::new(),
        Err(problem) => {
            let mut segments = vec!["$schema".to_string()];
            segments.extend(source_path_segments(&problem.path));
            vec![Finding::at(segments, &super::registry::claim_error(path, &problem))]
        }
    }
}

/// A standalone document's parse or preparation failure, at the definition
/// the library locates it in when it can.
fn definition_failure(body: &str, error: &SchemaError) -> Finding {
    match simplified_schema::locate_standalone_definition_failure(body) {
        Some(located) => Finding {
            segments: source_path_segments(&located.path),
            span: located.authored_span.or(located.value_span),
            message: error.to_string(),
        },
        None => Finding::at(named_field(error), error),
    }
}

/// The node a library message names in its leading `` `types.NAME` `` or
/// `` `categories[N]` `` field, as instance-path segments; the root
/// otherwise.
fn named_field(error: &SchemaError) -> Vec<String> {
    let SchemaError::SchemaDocument { message, .. } = error else {
        return Vec::new();
    };
    let Some((field, _)) = message.strip_prefix('`').and_then(|rest| rest.split_once("`:")) else {
        return Vec::new();
    };
    field
        .split('.')
        .flat_map(|part| part.split(['[', ']']).filter(|segment| !segment.is_empty()))
        .map(str::to_string)
        .collect()
}

/// A schema source path as instance-path segments: a property is its name,
/// a union arm its index (a root-union arm is a sequence item).
fn source_path_segments(path: &SchemaSourcePath) -> Vec<String> {
    path.segments()
        .iter()
        .map(|segment| match segment {
            SchemaSourcePathSegment::Property(name) => name.clone(),
            SchemaSourcePathSegment::UnionArm(index) => index.to_string(),
        })
        .collect()
}
