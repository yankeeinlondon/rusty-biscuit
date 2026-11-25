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
//! | `namespace-schema` | the preparation a host's registration runs ([`crate::resolve::prepare_namespace_document`]): its labels' exact forms (an explicit `null` `description` or `types` is refused), a `$schema` written as one mapping, and its resolution with the file's private `types` |
//! | `expression-schema`, `mutation-schema` | syntax only: the same parse without envelope admission (labels' exact forms, null entries, category entries, undeclared categories, each type string's grammar, duplicate overloads); nothing is resolved or imported, and the callable rules belong to the host's catalog reader |
//!
//! A namespace file is the one exception to the rule above: its check is
//! the registration's own preparation, which admits the envelope against
//! the compiled definition again, so a file this check accepts is one every
//! host registration accepts. That is also why a namespace is checked here
//! and never resolved as a `$schema` target: naming one from another
//! document stays refused.
//!
//! Every problem is a [`ValidationProblemCode::KindCheck`] located in the
//! document's text. In the file itself it is an error; the same trigger
//! failure found while discovering triggers for a Markdown document is that
//! document's skipped-trigger warning instead (see
//! [`crate::triggers::discovery`]). A user's own kind has no checks
//! here: its definition is its whole contract.

use std::ops::Range;
use std::path::{Path, PathBuf};

use biscuit_file::FileResolutionContext;
use crate::{
    SchemaError, ValidationProblem, ValidationProblemCode,
    ValidationProblemKind,
};

use crate::embedded::ShippedSources;
use crate::triggers::envelope::{check_trigger_semantics, trigger_error_segments};
use super::declaration::{claim_segments, source_path_segments};
use super::load::{LoadedYaml, locating_problem};

/// The document kinds that have checks here.
pub const CHECKED_KINDS: &[&str] =
    &["schema", "schema-trigger", "yaml-schema", "namespace-schema", "expression-schema", "mutation-schema"];

/// The checked kinds whose checks resolve what the document imports, so
/// read files; the `schema-trigger` checks are passive.
pub const FILE_READING_KINDS: &[&str] = &["schema", "yaml-schema", "namespace-schema"];

/// Runs `kind`'s checks over `document`; `accepted` is the copy of it the
/// selected definition accepted. `schema_roots`, `context`, and the host's
/// shipped `sources` resolve what a schema or definition imports. Empty for
/// a kind without checks.
pub(crate) fn kind_checks(
    kind: &str,
    document: &super::LoadedYamlDocument,
    accepted: &serde_json::Value,
    schema_roots: &[PathBuf],
    context: &FileResolutionContext,
    sources: ShippedSources,
) -> Vec<ValidationProblem> {
    let (text, path, loaded): (&str, &Path, &LoadedYaml) = (document.text(), document.path(), document.loaded());
    let body = &text[loaded.offset..];
    if FILE_READING_KINDS.contains(&kind) {
        crate::note_work!(YamlKindCheck);
    }
    let findings = match kind {
        "schema-trigger" => trigger_checks(loaded),
        "schema" => schema_checks(body, path, accepted, schema_roots, context, sources),
        "yaml-schema" => yaml_schema_checks(body, path, accepted, schema_roots, context, sources),
        "namespace-schema" => namespace_checks(body, path, context, sources),
        "expression-schema" | "mutation-schema" => callable_catalog_checks(body, path, accepted),
        _ => Vec::new(),
    };
    findings.into_iter().map(|finding| finding.at_reference(text, path, loaded).into_problem(text, loaded)).collect()
}

/// [`super::LoadedYamlDocument::locate_declaration_problems`]: a problem at
/// a carrier whose own parse fails with the same message moves to the
/// parser's located failure.
pub(crate) fn locate_carrier_problems(text: &str, problems: &mut [ValidationProblem]) {
    for problem in problems {
        let Some(key) = problem.path.strip_prefix('/').filter(|key| matches!(*key, "$schema" | "types")) else {
            continue;
        };
        let Some(located) = crate::locate_carrier_definition_failure(text, key) else {
            continue;
        };
        if !problem.message.contains(&located.error.to_string()) {
            continue;
        }
        let relocated = locating_problem(&source_path_segments(&located.path));
        problem.path = relocated.path;
        problem.instance_path = relocated.instance_path;
        if let Some(span) = located.authored_span.or(located.value_span) {
            let (line, column) = crate::line_col_of_offset(text, span.start);
            problem.line = u32::try_from(line).ok();
            problem.column = u32::try_from(column).ok();
        }
    }
}

/// One check failure before it is located.
struct Finding {
    /// The node it is about, as instance-path segments.
    segments: Vec<String>,
    /// The exact byte range in the YAML text, when the failure carries one.
    span: Option<Range<usize>>,
    message: String,
    /// The resolver's reference site, when the failure records one and no
    /// span located it: [`Self::at_reference`] moves the finding there.
    site: Option<(crate::SchemaReferenceSite, Option<String>)>,
}

impl Finding {
    fn at(segments: Vec<String>, error: &SchemaError) -> Self {
        let span = match error {
            SchemaError::SchemaDocument { span, .. } => span.clone(),
            _ => None,
        };
        let site = super::declaration::reference_site(error).map(|(site, name)| (site.clone(), name.map(str::to_string)));
        Self { segments, span, message: error.to_string(), site }
    }

    /// This finding at its reference site's declaration, when the site is in
    /// this document (`path`, or the inline payload being prepared): the
    /// carrier the finding named is tried first, and the spelled name keeps
    /// a same-named declaration in the other carrier from standing in for it.
    fn at_reference(mut self, text: &str, path: &Path, loaded: &LoadedYaml) -> Self {
        let Some((site, name)) = self.site.take() else {
            return self;
        };
        let here = match &site.source {
            crate::SchemaSourceId::File(source) => source == path,
            crate::SchemaSourceId::InlineDocument => true,
            crate::SchemaSourceId::Embedded(_) => false,
        };
        if !here || self.span.is_some() {
            return self;
        }
        let carriers: &[&str] = match self.segments.first().map(String::as_str) {
            Some("types") => &["types", "$schema"],
            _ => &["$schema", "types"],
        };
        if let Some(segments) = super::declaration::declaration_segments(loaded, text, &site.path, carriers, name.as_deref()) {
            self.segments = segments;
        }
        self
    }

    fn into_problem(self, text: &str, loaded: &LoadedYaml) -> ValidationProblem {
        let mut problem = locating_problem(&self.segments);
        problem.message = self.message;
        problem.kind = ValidationProblemKind::Invalid;
        problem.code = ValidationProblemCode::KindCheck;
        loaded.positions.locate_all(std::slice::from_mut(&mut problem));
        if let Some(span) = self.span {
            let (line, column) = crate::line_col_of_offset(text, span.start + loaded.offset);
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
    sources: ShippedSources,
) -> Vec<Finding> {
    let document = match crate::parse_selected_standalone_schema_document(body, path, accepted) {
        Ok(Some(document)) => document,
        // A document its definition admitted is always recognized.
        Ok(None) => return Vec::new(),
        Err(error) => return vec![definition_failure(body, &error)],
    };
    match crate::resolve::check_standalone_named_types(&document, schema_roots, context, sources) {
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
    sources: ShippedSources,
) -> Vec<Finding> {
    let prepared = crate::parse_selected_standalone_schema_document(body, path, accepted).and_then(|document| {
        // A document its definition admitted is always recognized.
        let Some(document) = document else { return Ok(None) };
        crate::resolve::prepare_standalone_yaml_schema(document, schema_roots, context, sources)
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
                    crate::slots::slots_not_allowed_message("a kind definition", unresolved);
            }
            return vec![finding];
        }
    };
    match crate::claimed_values(&resolved, "kind") {
        Ok(_) => Vec::new(),
        Err(problem) => vec![Finding::at(claim_segments(&problem), &super::registry::claim_error(path, &problem, body))],
    }
}

/// A callable catalog's syntax: the parse of the accepted document, which
/// reads no file. Named types are not expanded; a catalog is never an import
/// library.
fn callable_catalog_checks(body: &str, path: &Path, accepted: &serde_json::Value) -> Vec<Finding> {
    match crate::parse_selected_standalone_schema_document(body, path, accepted) {
        Ok(_) => Vec::new(),
        Err(error) => vec![definition_failure(body, &error)],
    }
}

fn namespace_checks(body: &str, path: &Path, context: &FileResolutionContext, sources: ShippedSources) -> Vec<Finding> {
    use crate::resolve::NamespaceDocumentError;
    let error = match crate::resolve::prepare_namespace_document(body, path, context, sources) {
        Ok(_) => return Vec::new(),
        Err(error) => error,
    };
    let mut finding = match &error {
        NamespaceDocumentError::Invalid(cause) | NamespaceDocumentError::Unresolved(cause) => definition_failure(body, cause),
        NamespaceDocumentError::NotOneObject => Finding { segments: Vec::new(), span: None, message: String::new(), site: None },
    };
    if finding.segments.is_empty() && finding.span.is_none() {
        finding.segments = vec!["$schema".to_string()];
    }
    finding.message = error.to_string();
    vec![finding]
}

/// A standalone document's parse or preparation failure, at the definition
/// the library locates it in when it can.
fn definition_failure(body: &str, error: &SchemaError) -> Finding {
    match crate::locate_standalone_definition_failure(body) {
        Some(located) => Finding {
            segments: source_path_segments(&located.path),
            span: located.authored_span.or(located.value_span),
            message: error.to_string(),
            site: None,
        },
        None => {
            let mut finding = Finding::at(named_field(error), error);
            if let SchemaError::SchemaDocument { span, .. } | SchemaError::CallableCatalog { span, .. } = error {
                finding.span = span.clone();
            }
            finding
        }
    }
}

/// The node a library message names in its leading `` `types.NAME` `` or
/// `` `categories[N]` `` field, as instance-path segments; the root
/// otherwise.
fn named_field(error: &SchemaError) -> Vec<String> {
    let message = match error {
        SchemaError::SchemaDocument { message, .. } | SchemaError::CallableCatalog { message, .. } => message,
        _ => return Vec::new(),
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
