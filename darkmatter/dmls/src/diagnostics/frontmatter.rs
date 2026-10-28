//! Frontmatter / schema diagnostics (the R-5 `dm.*` taxonomy).
//!
//! Ranging follows the R-5 convention: the validator identifies the semantic
//! failing node, but every range comes from the concrete syntax tree
//! ([`FrontmatterAst`]), never from message parsing or line maps. Missing-key
//! diagnostics range the parent mapping (a real visible range, not zero-width);
//! unknown-key diagnostics range the offending key; type/constraint/file
//! diagnostics range the value; `relatedInformation` points at the schema
//! origin. Deferred `$(...)` / `{{ … }}` values are never diagnosed as schema
//! or malformed-expression problems (their passivity is explained on hover,
//! not as a per-value squiggle); a nested span on a single-pass lifecycle
//! surface is the one exception (see `nested_span`).

use darkmatter::markdown::Markdown;
use darkmatter::markdown::schemas::format::is_pending_expression_value;
use darkmatter::markdown::schemas::{
    PositionMap, SchemaError, SchemaOriginKind, SuggestionLintProblem, SuggestionLintReason,
    ValidationOptions, ValidationProblem, ValidationProblemCode, ValidationReport,
    SchemaValueKind, SchemaValueNode, classify_schema_reference, flow_collection_end,
    SchemaSourcePath, SchemaSourcePathSegment, locate_schema_value,
    LocatedDefinitionFailure, LocatedRelated, locate_standalone_definition_failure, parse_property_definition,
    parse_property_definition_with_source, parse_schema_declaration,
    parse_schema_declaration_with_source, parse_yaml_schema,
};
use darkmatter::markdown::schemas::triggers::InvalidTriggerFinding;
use darkmatter::style::{self, StyleWarningKind};
use lsp_types::{
    Diagnostic, DiagnosticRelatedInformation, DiagnosticSeverity, Location, NumberOrString, Position,
    Range,
};

use crate::context::resolution_failure_data;
use crate::diagnostics::codes::{code, source};
use crate::diagnostics::named_types;
use crate::diagnostics::nested_span;
use crate::overlay::event_views::LifecycleHost;
use crate::overlay::{
    FrontmatterAst, SchemaAuthoringState, SchemaBundle, SchemaOutcome, SuggestionState,
};
use crate::overlay::named_types::AuthoredDeclaration;
use crate::overlay::schema::MetaSchemaKind;
use crate::providers::DocumentContext;
use crate::providers::frontmatter::ExpressionValue;
use crate::source_map::SourceMap;
use crate::workspace::file_path_to_uri;

/// All frontmatter/schema diagnostics for the current document, discovering
/// its lifecycle host when a lifecycle check needs it.
pub fn diagnostics(ctx: &DocumentContext) -> Vec<Diagnostic> {
    diagnostics_under(ctx, None)
}

/// [`diagnostics`] with every lifecycle check under `host`, which is not
/// rediscovered.
pub fn diagnostics_with_host(ctx: &DocumentContext, host: &LifecycleHost) -> Vec<Diagnostic> {
    diagnostics_under(ctx, Some(host))
}

fn diagnostics_under(ctx: &DocumentContext, host: Option<&LifecycleHost>) -> Vec<Diagnostic> {
    let Some(overlay) = ctx.overlay else {
        return Vec::new();
    };
    let _host_functions = crate::providers::dsl::enter_host_functions(ctx);
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
                schema_advisory_diagnostics(ctx, ast, bundle, &mut out);
                // One effective-schema validation drives both the generic schema
                // problems and the expression pass, which reads it to decide
                // whether a union as a whole rejects a malformed-expression value.
                let report = crate::validation_timing::part(|| {
                    bundle.effective.validate_with_options(
                        &bundle.frontmatter_json,
                        &PositionMap::new(),
                        &ValidationOptions::default(),
                    )
                });
                // Likewise one expression-value set per publication: it decodes
                // every Expression-typed value, and both passes read it.
                let expression_values =
                    crate::providers::frontmatter::expression_values(ctx);
                schema_problem_diagnostics(ctx, ast, bundle, &report, &expression_values, &mut out);
                style_diagnostics(ctx, ast, &mut out);
                // One lifecycle host and one library walk serve every
                // expression inside a stack, schema-typed or not.
                let discovered;
                let lifecycle = match host {
                    Some(host) => host,
                    None => {
                        discovered = LifecycleHost::for_document(ctx);
                        &discovered
                    }
                };
                let guarded = GuardedStackCalls::for_document(ctx, lifecycle);
                expression_diagnostics(ctx, ast, &report, &expression_values, lifecycle, &guarded, &mut out);
                stack_expression_diagnostics(ctx, ast, &expression_values, lifecycle, &guarded, &mut out);
                interpolation_call_diagnostics(ctx, ast, lifecycle, &mut out);
            }
        }
        SchemaOutcome::Ready(None) => {}
    }

    // A stale last-good tree does not describe the current buffer, so its
    // ranges and rewrites would land on the wrong bytes.
    if let Some(ast) = overlay.ast.as_deref().filter(|_| !overlay.stale) {
        crate::diagnostics::nested_span::whole_value_diagnostics(ctx, ast, &mut out);
    }

    suggestion_diagnostics(ctx, &overlay.suggestions, &mut out);

    if let Some(error) = &overlay.standalone_resolution
        && let SchemaAuthoringState::Standalone { model: Some(model), stale: false, .. } =
            &overlay.schema_authoring
    {
        let scope = named_types::NamedTypeScope::standalone(ctx, model);
        named_types::standalone_diagnostics(&scope, error, None, &mut out);
    }

    if let SuggestionState::TriggerWarning(finding) = &overlay.suggestions {
        let span = trigger_yaml_span(ctx.text, finding.cause()).unwrap_or(0..ctx.text.len());
        let range = ctx.source_map.byte_range_to_lsp(span).unwrap_or_else(zero_range);
        out.push(invalid_trigger_diagnostic_at(finding, range));
    }

    if let SuggestionState::TriggerError(error) = &overlay.suggestions {
        let span = trigger_yaml_span(ctx.text, error).unwrap_or(0..ctx.text.len());
        out.push(diagnostic(
            ctx.source_map
                .byte_range_to_lsp(span)
                .unwrap_or_else(zero_range),
            DiagnosticSeverity::ERROR,
            source::SCHEMA,
            code::SCHEMA_PREPARE,
            error.to_string(),
        ));
    }

    out
}

fn schema_advisory_diagnostics(
    ctx: &DocumentContext,
    ast: &FrontmatterAst,
    bundle: &SchemaBundle,
    out: &mut Vec<Diagnostic>,
) {
    let range = ast
        .schema_entry()
        .and_then(|entry| ctx.source_map.byte_range_to_lsp(entry.value_span.clone()))
        .unwrap_or_else(zero_range);
    out.extend(bundle.advisories.iter().map(|advisory| {
        diagnostic(
            range,
            DiagnosticSeverity::WARNING,
            advisory.source(),
            advisory.code(),
            advisory.message(),
        )
    }));
}

/// The warning a trigger discovery skipped publishes on its closed file:
/// at the start of a malformed envelope's YAML error line, else at the file
/// start.
pub fn invalid_trigger_diagnostic(finding: &InvalidTriggerFinding) -> Diagnostic {
    let range = finding.position().map_or_else(zero_range, |(line, _)| {
        let start = Position::new(u32::try_from(line.saturating_sub(1)).unwrap_or(u32::MAX), 0);
        Range::new(start, start)
    });
    invalid_trigger_diagnostic_at(finding, range)
}

/// The skipped-trigger warning at `range`, with the payload or import its
/// cause points into as related information.
fn invalid_trigger_diagnostic_at(finding: &InvalidTriggerFinding, range: Range) -> Diagnostic {
    let mut out = diagnostic(range, DiagnosticSeverity::WARNING, finding.source_family(), finding.code(), finding.message());
    out.related_information = finding
        .cause_path()
        .and_then(file_path_to_uri)
        .map(|uri| {
            vec![DiagnosticRelatedInformation {
                location: Location::new(uri, zero_range()),
                // The cause may sit in the payload or in a file it imports.
                message: "the trigger's schema failed to load here".to_string(),
            }]
        });
    out
}

/// A schema resolution/preparation failure, ranged at `$schema` (or the block).
fn schema_prepare_diagnostic(
    ctx: &DocumentContext,
    ast: Option<&FrontmatterAst>,
    error: &SchemaError,
    out: &mut Vec<Diagnostic>,
) {
    schema_prepare_diagnostic_with_origin(ctx, ast, error, None, out);
}

fn schema_prepare_diagnostic_with_origin(
    ctx: &DocumentContext,
    ast: Option<&FrontmatterAst>,
    error: &SchemaError,
    referenced_origin: Option<&std::path::Path>,
    out: &mut Vec<Diagnostic>,
) {
    if let SchemaError::Aggregate { errors } = error {
        for error in errors {
            schema_prepare_diagnostic_with_origin(ctx, ast, error, referenced_origin, out);
        }
        return;
    }
    if let SchemaError::ReferencedSchema { path, source } = error {
        schema_prepare_diagnostic_with_origin(ctx, ast, source, Some(path), out);
        return;
    }

    let fallback_span = ast
        .and_then(|ast| ast.schema_entry().map(|entry| entry.value_span.clone()))
        .or_else(|| ast.map(FrontmatterAst::block_span));
    if named_types::is_named_type_error(error) {
        let scope = named_types::NamedTypeScope::frontmatter(ctx, ast, fallback_span);
        out.push(scope.diagnostic(error, referenced_origin));
        return;
    }
    let mut related = Vec::new();
    let (code_value, message, span) = match error {
        SchemaError::Grammar { property, .. } | SchemaError::Convert { property, .. }
            if property != "<root>" =>
        {
            // A referenced file's failure has no range in this buffer; its
            // locations are related information in that file.
            let inline = referenced_origin
                .is_none()
                .then(|| ast.and_then(|ast| inline_definition_span(ctx, ast, error, property)))
                .flatten();
            let span = match inline {
                Some((span, located)) => {
                    related.extend(located_related(ctx, &located));
                    span
                }
                None => None,
            };
            if let Some(path) = referenced_origin
                && let Some(located) = named_types::referenced_definition_related(ctx, path, error)
            {
                related.extend(located);
            }
            (code::SCHEMA_INVALID_TYPE_DEFINITION, error.to_string(), span.or_else(|| fallback_span.clone()))
        }
        SchemaError::FrontmatterShape { message } => {
            (code::SCHEMA_INVALID_SHAPE, message.clone(), fallback_span.clone())
        }
        SchemaError::RemoteUnsupported { .. } => {
            (code::SCHEMA_INVALID_SHAPE, error.to_string(), fallback_span.clone())
        }
        SchemaError::Grammar { .. } | SchemaError::Convert { .. } => {
            (code::SCHEMA_INVALID_SHAPE, error.to_string(), fallback_span.clone())
        }
        other => {
            // A standalone schema file folds its payload failure into this
            // error; its locations are related information in that file.
            if let SchemaError::SchemaDocument { path, .. } = other {
                related.extend(named_types::referenced_definition_related(ctx, path, other).unwrap_or_default());
            }
            (code::SCHEMA_PREPARE, other.to_string(), fallback_span.clone())
        }
    };
    let range = span
        .and_then(|span| ctx.source_map.byte_range_to_lsp(span))
        .unwrap_or_else(zero_range);
    let mut diagnostic = diagnostic(
        range,
        DiagnosticSeverity::ERROR,
        source::SCHEMA,
        code_value,
        message,
    );
    if related.is_empty()
        && let Some(path) = referenced_origin
        && let Some(uri) = file_path_to_uri(path)
    {
        related.push(DiagnosticRelatedInformation {
            location: Location::new(uri, zero_range()),
            message: named_types::REFERENCED_DEFINITION.to_string(),
        });
    }
    if !related.is_empty() {
        diagnostic.related_information = Some(related);
    }
    out.push(diagnostic);
}

/// A located failure's secondary ranges in this document, as related
/// information.
fn located_related(ctx: &DocumentContext, related: &[LocatedRelated]) -> Vec<DiagnosticRelatedInformation> {
    related
        .iter()
        .filter_map(|entry| {
            Some(DiagnosticRelatedInformation {
                location: Location::new(ctx.uri.clone(), ctx.source_map.byte_range_to_lsp(entry.span.clone())?),
                message: entry.message.clone(),
            })
        })
        .collect()
}

/// The authored range of an inline `$schema` definition failure, with its
/// located secondary ranges.
///
/// When the source-aware declaration parser reports this same error, its
/// located token wins, then the rejected value, then the deepest entry along
/// its structural path, and its related locations come with it. An error that
/// parser does not report (a conversion failure) keeps the entry its property
/// label names, with none.
fn inline_definition_span(
    ctx: &DocumentContext,
    ast: &FrontmatterAst,
    error: &SchemaError,
    property: &str,
) -> Option<(Option<std::ops::Range<usize>>, Vec<LocatedRelated>)> {
    let fresh = ctx.overlay.is_some_and(|overlay| !overlay.stale);
    if let Some(failure) = fresh
        .then(|| AuthoredDeclaration::frontmatter_failure(ctx.text, ast))
        .flatten()
        .filter(|failure| failure.error.to_string() == error.to_string())
    {
        let span = failure
            .authored_span
            .clone()
            .or_else(|| failure.value_span.clone())
            .or_else(|| enclosing_entry_span(ctx.text, ast, &["$schema"], &failure.path));
        return Some((span, failure.related));
    }
    let span = ast
        .entry_by_key_path(&["$schema", property])
        .or_else(|| {
            let mut path = vec!["$schema"];
            path.extend(property.split('.'));
            ast.entry_by_key_path(&path)
        })
        .map(|entry| entry.value_span.clone());
    Some((span, Vec::new()))
}

/// Instance-validation problems mapped onto concrete ranges.
fn schema_problem_diagnostics(
    ctx: &DocumentContext,
    ast: &FrontmatterAst,
    bundle: &SchemaBundle,
    report: &ValidationReport,
    expression_values: &[ExpressionValue<'_>],
    out: &mut Vec<Diagnostic>,
) {
    // Exactly projected Expression-typed scalar values are owned by the
    // expression pass: `dm.expression.malformed` replaces the generic
    // format-constraint problem, and a native boolean/number is coerced at
    // compose time, so its type mismatch is not a real editor error. Both are
    // suppressed here. Block, tagged, and multi-line values get no dedicated
    // malformed diagnostic, so they keep the generic schema problem, as does
    // every unrelated problem (including a genuine mapping/sequence type
    // mismatch, which is not a scalar and never enters this set).
    let expression_pointers: std::collections::HashSet<&str> = expression_values
        .iter()
        .filter(|value| value.is_exact())
        .map(|value| value.entry.pointer.as_str())
        .collect();

    let mut specialized_paths = semantic_authored_diagnostics(ctx, ast, bundle, report, out);
    for problem in &report.problems {
        let Some(kind) = semantic_problem_kind(problem) else {
            continue;
        };
        if !specialized_paths.insert(problem.path.clone()) {
            continue;
        }
        let Some(range) = semantic_problem_range(ctx, ast, problem, kind) else {
            continue;
        };
        let mut specialized = diagnostic(
            range,
            DiagnosticSeverity::ERROR,
            source::SCHEMA,
            match kind {
                SemanticProblemKind::TypeDefinition => code::SCHEMA_INVALID_TYPE_DEFINITION,
                SemanticProblemKind::Schema => code::SCHEMA_INVALID_SHAPE,
            },
            problem.message.clone(),
        );
        specialized.related_information = schema_origin_related(ctx, bundle, problem);
        specialized.data = Some(schema_problem_data(problem));
        out.push(specialized);
    }

    for problem in &report.problems {
        if specialized_paths.contains(&problem.path) {
            continue;
        }
        let Some((code_value, severity)) = classify(problem.code, ctx.config.schema.strict) else {
            continue;
        };
        // Without a context no file value was resolved for this document; its
        // one context-failure diagnostic stands in for them.
        if problem.file_reference.is_some() && ctx.file_context().is_err() {
            continue;
        }
        if matches!(
            problem.code,
            ValidationProblemCode::TypeMismatch | ValidationProblemCode::ConstraintViolation
        ) && expression_pointers.contains(problem.path.as_str())
        {
            continue;
        }
        let Some(range) = problem_range(ctx.text, ast, ctx.source_map, problem) else {
            continue;
        };
        let mut diagnostic = diagnostic(
            range,
            severity,
            source::SCHEMA,
            code_value,
            problem.message.clone(),
        );
        diagnostic.related_information = schema_origin_related(ctx, bundle, problem);
        diagnostic.data = Some(schema_problem_data(problem));
        out.push(diagnostic);
    }

    // Deferred-composition values (`$(...)` / `{{ … }}`) are intentionally NOT
    // diagnosed: an informational squiggle on every interpolated property is
    // pure noise. The passive "never executed" guarantee is surfaced on hover
    // (schema description for the key; policy verdict for `$()` shell), not as a
    // per-value diagnostic. `report.pending` stays populated but unconsumed.
}

/// The `data` payload of a schema-problem diagnostic:
/// `{"instance_path": "/content_policy/1/rule"}`, plus `resolution_failure`
/// for a file-reference failure.
///
/// The range already marks the node, but a missing key ranges its parent
/// mapping and a list item shares its line with its fields, so the validator's
/// JSON Pointer travels with it. A missing key's pointer names the parent
/// object, as in `md schema validate --format json`; the message names the key.
fn schema_problem_data(problem: &ValidationProblem) -> serde_json::Value {
    let mut data = serde_json::Map::new();
    data.insert("instance_path".to_string(), problem.path.clone().into());
    if let Some(reference) = &problem.file_reference
        && let serde_json::Value::Object(failure) =
            resolution_failure_data(reference.resolution_failure())
    {
        data.extend(failure);
    }
    serde_json::Value::Object(data)
}

/// The instance paths at which schema validation reported a problem.
///
/// The invariant both specialized passes below (semantic meta-types and
/// expressions) rely on: **never report a dedicated diagnostic for a value the
/// schema validation accepted.** A path absent here was accepted — for a union
/// property, by at least one arm — so a specialized diagnostic there would be
/// a false positive on a valid document.
///
/// ## Notes
///
/// Activation is deliberately broader than rejection — a property typed
/// `[type-definition, string]` records the semantic kind even though only one
/// arm carries it — which is precisely why the gate is needed.
fn union_rejected_paths(report: &ValidationReport) -> std::collections::HashSet<&str> {
    report.problems.iter().map(|problem| problem.path.as_str()).collect()
}

fn semantic_authored_diagnostics(
    ctx: &DocumentContext,
    ast: &FrontmatterAst,
    bundle: &SchemaBundle,
    report: &ValidationReport,
    out: &mut Vec<Diagnostic>,
) -> std::collections::HashSet<String> {
    let mut diagnosed = std::collections::HashSet::new();
    let Some(overlay) = ctx.overlay else {
        return diagnosed;
    };
    if overlay.stale {
        return diagnosed;
    }
    let SchemaAuthoringState::Frontmatter(values) = &overlay.schema_authoring else {
        return diagnosed;
    };
    let union_rejected = union_rejected_paths(report);

    for value in values {
        if value.pointer == "/$schema" {
            continue;
        }
        if !union_rejected.contains(value.pointer.as_str()) {
            // A sibling arm accepts this value, so the union validates.
            continue;
        }
        let Some(entry) = ast.entry_by_pointer(&value.pointer) else {
            continue;
        };
        let value_span = complete_flow_value_span(ctx.text, entry.value_span.clone());
        let Some(raw) = ctx.text.get(value_span.clone()) else {
            continue;
        };
        let raw = dedent_entry_yaml(raw, source_column(ctx.text, value_span.start));
        let Ok(yaml) = serde_yaml_ng::from_str(&raw) else {
            continue;
        };

        let mut failures = Vec::new();
        let mut accepted = false;
        for kind in &value.kinds {
            let result = match kind {
                MetaSchemaKind::TypeDefinition => {
                    parse_property_definition(&entry.key, &yaml).map(|_| ())
                }
                MetaSchemaKind::Schema => parse_schema_declaration(&yaml).map(|_| ()),
            };
            match result {
                Ok(()) => {
                    accepted = true;
                    break;
                }
                Err(error) => failures.push((*kind, error)),
            }
        }
        if accepted {
            continue;
        }
        let Some((kind, error)) = failures.into_iter().next() else {
            continue;
        };
        // Acceptance stays with the source-free parsers above; the
        // source-aware parser only locates the failure they reported.
        // A declaration failure on no property is the declaration's own shape,
        // which keeps the whole value, as a standalone envelope's does.
        let located = locate_meta_type_failure(ctx.text, value_span.clone(), kind, &entry.key)
            .filter(|failure| failure.error.to_string() == error.to_string())
            .filter(|failure| {
                kind == MetaSchemaKind::TypeDefinition
                    || failure
                        .path
                        .segments()
                        .iter()
                        .any(|segment| matches!(segment, SchemaSourcePathSegment::Property(_)))
            });
        let related = located.as_ref().map(|failure| located_related(ctx, &failure.related)).unwrap_or_default();
        let span = located
            .and_then(|failure| failure.authored_span.or(failure.value_span))
            .unwrap_or_else(|| value_span.clone());
        let Some(range) = ctx.source_map.byte_range_to_lsp(span) else {
            continue;
        };
        let mut specialized = diagnostic(
            range,
            DiagnosticSeverity::ERROR,
            source::SCHEMA,
            match kind {
                MetaSchemaKind::TypeDefinition => code::SCHEMA_INVALID_TYPE_DEFINITION,
                MetaSchemaKind::Schema => code::SCHEMA_INVALID_SHAPE,
            },
            error.to_string(),
        );
        if let Some(problem) = report.problems.iter().find(|problem| problem.path == value.pointer) {
            specialized.related_information = schema_origin_related(ctx, bundle, problem);
            specialized.data = Some(schema_problem_data(problem));
        }
        if !related.is_empty() {
            specialized.related_information.get_or_insert_with(Vec::new).extend(related);
        }
        out.push(specialized);
        diagnosed.insert(value.pointer.clone());
    }
    diagnosed
}

/// The failure of a meta-typed frontmatter value at `value_span`, from the
/// source-aware parser for `kind`, with ranges in `text`'s bytes; `None` when
/// the value parses or is not YAML on its own.
///
/// The value is re-parsed in place (the text before it on its first line
/// replaced with spaces), so the parser reads the authored columns.
fn locate_meta_type_failure(
    text: &str,
    value_span: std::ops::Range<usize>,
    kind: MetaSchemaKind,
    key: &str,
) -> Option<LocatedDefinitionFailure> {
    let line_start = text[..value_span.start].rfind('\n').map_or(0, |index| index + 1);
    let mut source = " ".repeat(value_span.start - line_start);
    source.push_str(text.get(value_span)?);
    let yaml: serde_yaml_ng::Value = serde_yaml_ng::from_str(&source).ok()?;
    match kind {
        MetaSchemaKind::TypeDefinition => {
            parse_property_definition_with_source(key, &yaml, &source, line_start).err()
        }
        MetaSchemaKind::Schema => parse_schema_declaration_with_source(&yaml, &source, line_start).err(),
    }
}

fn complete_flow_value_span(text: &str, span: std::ops::Range<usize>) -> std::ops::Range<usize> {
    let start = span.start;
    flow_collection_end(text, start).map_or(span, |end| start..end + 1)
}

fn source_column(text: &str, offset: usize) -> usize {
    text[..offset].rsplit_once('\n').map_or(offset, |(_, line)| line.len())
}

fn dedent_entry_yaml(source: &str, indent: usize) -> String {
    let padding = " ".repeat(indent);
    let mut normalized = String::with_capacity(source.len());
    for (index, line) in source.lines().enumerate() {
        if index > 0 {
            normalized.push('\n');
        }
        normalized.push_str(if index == 0 {
            line
        } else {
            line.strip_prefix(&padding).unwrap_or(line)
        });
    }
    if source.ends_with('\n') {
        normalized.push('\n');
    }
    normalized
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SemanticProblemKind {
    TypeDefinition,
    Schema,
}

fn semantic_problem_kind(problem: &ValidationProblem) -> Option<SemanticProblemKind> {
    let segments = problem.schema_path.as_ref()?.segments();
    if segments.iter().any(|segment| segment == "x-darkmatter-type-definition") {
        Some(SemanticProblemKind::TypeDefinition)
    } else if segments.iter().any(|segment| segment == "x-darkmatter-schema") {
        Some(SemanticProblemKind::Schema)
    } else {
        None
    }
}

fn semantic_problem_range(
    ctx: &DocumentContext,
    ast: &FrontmatterAst,
    problem: &ValidationProblem,
    kind: SemanticProblemKind,
) -> Option<Range> {
    let entry = ast.entry_or_ancestor(&problem.path)?;
    let span = if kind == SemanticProblemKind::TypeDefinition {
        let value_span = complete_flow_value_span(ctx.text, entry.value_span.clone());
        locate_meta_type_failure(ctx.text, value_span, MetaSchemaKind::TypeDefinition, &entry.key)
            .and_then(|failure| failure.authored_span.or(failure.value_span))
            .unwrap_or_else(|| entry.value_span.clone())
    } else {
        entry.value_span.clone()
    };
    ctx.source_map.byte_range_to_lsp(span)
}

/// The concrete range for one validation problem (R-5 ranging rules).
fn problem_range(
    text: &str,
    ast: &FrontmatterAst,
    sm: &SourceMap,
    problem: &ValidationProblem,
) -> Option<Range> {
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
        // Type / constraint / file-reference failures range the value node,
        // all of it: the YAML loader ends an empty flow collection (`{}`)
        // after its opening delimiter.
        _ => complete_flow_value_span(text, ast.value_range(&problem.path)),
    };
    sm.byte_range_to_lsp(span)
}

/// Expression-typed frontmatter value diagnostics: `dm.expression.malformed`
/// (`ERROR`) for a value the expression grammar rejects,
/// `dm.expression.undeclared_property` (`WARNING`) for each unhandled
/// identifier, in any operand position, that is an undeclared document property,
/// `dm.expression.unknown_function` (`ERROR`) for each call to a function
/// outside Darkmatter's catalog, the design-time call check's
/// `dm.expression.incompatible_call` / `dm.expression.uncertain_call`
/// ([`FrontmatterCalls`]),
/// and `dm.expression.nested_span_in_literal` for a lifecycle predicate. All
/// carry source `darkmatter.frontmatter`; ranges go through the scalar
/// projection, exact (YAML quotes excluded) for untagged single-line plain or
/// quoted values and otherwise the whole scalar.
///
/// Severity follows the ladder in `codes.rs`: the schema declares the value
/// *is* an expression, so a parse failure will never evaluate.
fn expression_diagnostics(
    ctx: &DocumentContext,
    ast: &FrontmatterAst,
    report: &ValidationReport,
    expression_values: &[ExpressionValue<'_>],
    lifecycle: &crate::overlay::event_views::LifecycleHost,
    guarded: &GuardedStackCalls,
    out: &mut Vec<Diagnostic>,
) {
    let union_rejected = union_rejected_paths(report);
    let known_roots = crate::providers::dsl::KnownRoots::for_document(ctx);
    // An anchored value and each alias of it are one authored expression,
    // parsed at most once: whichever Expression-typed property reaches it
    // first settles it, and a later value with the same span is skipped. A
    // malformed one only a union arm accepted stays unsettled, its error kept
    // for a later alias whose property rejects it.
    let mut settled = std::collections::HashSet::new();
    let mut unreported_errors = std::collections::HashMap::new();
    let stale = ctx.overlay.is_some_and(|overlay| overlay.stale);
    let declared = crate::diagnostics::call_check::declared_types(ctx);
    let calls = FrontmatterCalls::new(ctx, &declared);

    for value in expression_values {
        let expression = value.expression();
        let path = ast.path_at(value.index);
        // A stored literal token is data, not authored expression text; its
        // source bytes cannot anchor a parse error in the decoded text.
        if matches!(darkmatter::markdown::literal_token::decode_leaf(expression), Some(Ok(_))) {
            continue;
        }
        if is_pending_expression_value(expression) {
            // Schema validation defers a pending `{{ … }}` / `$(…)` value, so
            // it is not yet a final expression to parse or resolve. Only a
            // pending value can hold a nested span, which a predicate never
            // re-interpolates.
            if !stale && nested_span::lifecycle::is_predicate(&path) {
                nested_span::predicate_diagnostic(ctx, value, out);
            }
            continue;
        }
        let expression_span = value.expression_span();
        if settled.contains(&expression_span) {
            continue;
        }
        let parsed = match unreported_errors.remove(&expression_span) {
            Some(error) => Err(error),
            None => {
                note_expression_diagnostic_parse();
                crate::overlay::expressions::parse_condition(expression)
            }
        };
        match parsed {
            Err(error) => {
                // Without an exact projection the message and range would
                // describe authored bytes the parser never saw (a `|-`
                // indicator, a tag); the schema problem stays instead.
                if !value.is_exact() {
                    continue;
                }
                if !union_rejected.contains(value.entry.pointer.as_str()) {
                    // Validation accepted this value (a sibling union arm), so
                    // reporting it malformed would outrank the schema.
                    unreported_errors.insert(expression_span, error);
                    continue;
                }
                settled.insert(expression_span);
                let at = error.position.min(expression.len());
                let span = value
                    .project(at..expression.len())
                    .filter(|span| span.start < span.end)
                    .unwrap_or_else(|| value.expression_span());
                if let Some(range) = ctx.source_map.byte_range_to_lsp(span) {
                    out.push(diagnostic(
                        range,
                        DiagnosticSeverity::ERROR,
                        source::FRONTMATTER,
                        code::EXPRESSION_MALFORMED,
                        format!("malformed expression: {}", error.message),
                    ));
                }
            }
            Ok(parsed) => {
                settled.insert(expression_span);
                match guarded.surface(ast, value.index) {
                    // A stack surface is typed at its site by the library's
                    // walk, under the `when` items around it.
                    Some(surface) => {
                        let checks: Vec<_> = surface.checks.iter().map(|found| found.check.clone()).collect();
                        for found in crate::diagnostics::call_check::check_diagnostics(&checks, calls.strict) {
                            let doc_span = value.project(found.span.clone()).unwrap_or_else(|| value.expression_span());
                            calls.push(ctx, doc_span, found, out);
                        }
                        for found in crate::diagnostics::call_check::incomplete_diagnostics(&parsed) {
                            let doc_span = value.project(found.span.clone()).unwrap_or_else(|| value.expression_span());
                            calls.push(ctx, doc_span, found, out);
                        }
                    }
                    None => {
                        for found in calls.check(&parsed) {
                            let doc_span = value.project(found.span.clone()).unwrap_or_else(|| value.expression_span());
                            calls.push(ctx, doc_span, found, out);
                        }
                    }
                }
                let event = lifecycle.stack_event(&path);
                for (name, span) in crate::overlay::expressions::unknown_function_calls(&parsed, expression) {
                    let doc_span = value.project(span).unwrap_or_else(|| value.expression_span());
                    if let Some(range) = ctx.source_map.byte_range_to_lsp(doc_span) {
                        out.push(diagnostic(
                            range,
                            DiagnosticSeverity::ERROR,
                            source::FRONTMATTER,
                            code::EXPRESSION_UNKNOWN_FUNCTION,
                            crate::overlay::expressions::unknown_function_message(&name),
                        ));
                    }
                }
                let Some(known_roots) = &known_roots else {
                    continue;
                };
                let view = known_roots.view(event.as_ref());
                for (span, message) in crate::overlay::expressions::unavailable_global_reads(&parsed, &view) {
                    let doc_span = value.project(span).unwrap_or_else(|| value.expression_span());
                    if let Some(range) = ctx.source_map.byte_range_to_lsp(doc_span) {
                        out.push(diagnostic(
                            range,
                            DiagnosticSeverity::ERROR,
                            source::FRONTMATTER,
                            code::EXPRESSION_UNAVAILABLE_GLOBAL,
                            message,
                        ));
                    }
                }
                let findings = known_roots.findings(
                    &view,
                    &parsed,
                    expression,
                    crate::overlay::expressions::parse_condition,
                    yaml_quote(ctx, value),
                );
                let first = out.len();
                for finding in findings {
                    let span = crate::providers::dsl::finding_span(&finding);
                    // A block, tagged, or multi-line value has no decoded-to-
                    // authored map, so its findings land on the whole scalar
                    // rather than being dropped; the fix check below then
                    // withholds the edit, since the texts cannot match.
                    let doc_span = value
                        .project(span.clone())
                        .unwrap_or_else(|| value.expression_span());
                    // A fix is an edit of authored bytes: withhold it when the
                    // projection crosses a YAML escape or is only an alias
                    // token, whose text never equals the expression's.
                    let finding = if ctx.text.get(doc_span.clone()) == expression.get(span.clone()) {
                        finding
                    } else {
                        without_fix(finding)
                    };
                    if let Some(range) = ctx.source_map.byte_range_to_lsp(doc_span) {
                        let diagnostic = crate::providers::dsl::undeclared_property_diagnostic(
                            range,
                            source::FRONTMATTER,
                            &finding,
                        );
                        // On an alias token every finding shares one range, so
                        // a repeated identifier would repeat the diagnostic. Only
                        // this value's findings can match: the token is its own.
                        if value.is_exact() || !out[first..].contains(&diagnostic) {
                            out.push(diagnostic);
                        }
                    }
                }
            }
        }
    }
}

/// Expression diagnostics for the `{{ … }}` spans and `when` guards inside
/// lifecycle stack values that are not Expression-typed values (those are
/// [`expression_diagnostics`]'), each classified against its event's view
/// ([`KnownRoots::view`](crate::providers::dsl::KnownRoots::view)): an unknown
/// function and a read of a global the event makes unavailable are errors, an
/// undeclared document property a warning, and each call gets the call check
/// ([`FrontmatterCalls`]). A span or guard that does not parse is left to the
/// runtime; a stale tree reports nothing.
fn stack_expression_diagnostics(
    ctx: &DocumentContext,
    ast: &FrontmatterAst,
    expression_values: &[ExpressionValue<'_>],
    lifecycle: &crate::overlay::event_views::LifecycleHost,
    guarded: &GuardedStackCalls,
    out: &mut Vec<Diagnostic>,
) {
    if ctx.overlay.is_some_and(|overlay| overlay.stale) {
        return;
    }
    let Some(known_roots) = crate::providers::dsl::KnownRoots::for_document(ctx) else {
        return;
    };
    let typed: std::collections::HashSet<usize> = expression_values.iter().map(|value| value.index).collect();
    let stacks =
        crate::overlay::event_views::stack_expressions(lifecycle, ctx.text, ast, |index| typed.contains(&index));
    let declared = crate::diagnostics::call_check::declared_types(ctx);
    let calls = FrontmatterCalls::new(ctx, &declared);
    let mut checked_entries = std::collections::HashSet::new();
    let mut views = std::collections::HashMap::new();
    for expression in stacks {
        let Ok(parsed) = expression.parse() else {
            continue;
        };
        match guarded.surface(ast, expression.index) {
            // The library checked the whole scalar at its site; report it once.
            Some(surface) => {
                if checked_entries.insert(expression.index) {
                    let checks: Vec<_> = surface.checks.iter().map(|found| found.check.clone()).collect();
                    for found in crate::diagnostics::call_check::check_diagnostics(&checks, calls.strict) {
                        let doc_span = expression.scalar_range(found.span.clone());
                        calls.push(ctx, doc_span, found, out);
                    }
                }
                // The library's checks skip a host function it knows nothing about.
                for found in crate::diagnostics::call_check::incomplete_diagnostics(&parsed) {
                    let doc_span = expression.range(found.span.clone());
                    calls.push(ctx, doc_span, found, out);
                }
            }
            None => {
                for found in calls.check(&parsed) {
                    let doc_span = expression.range(found.span.clone());
                    calls.push(ctx, doc_span, found, out);
                }
            }
        }
        let view = views.entry(expression.event.id()).or_insert_with(|| known_roots.view(Some(&expression.event)));
        let mut push = |span: std::ops::Range<usize>, severity, code_value: &str, message: String| {
            if let Some(range) = ctx.source_map.byte_range_to_lsp(expression.range(span)) {
                out.push(diagnostic(range, severity, source::FRONTMATTER, code_value, message));
            }
        };
        for (name, span) in crate::overlay::expressions::unknown_function_calls(&parsed, &expression.text) {
            push(
                span,
                DiagnosticSeverity::ERROR,
                code::EXPRESSION_UNKNOWN_FUNCTION,
                crate::overlay::expressions::unknown_function_message(&name),
            );
        }
        for (span, message) in crate::overlay::expressions::unavailable_global_reads(&parsed, view) {
            push(span, DiagnosticSeverity::ERROR, code::EXPRESSION_UNAVAILABLE_GLOBAL, message);
        }
        let quote = ctx
            .text
            .get(expression.entry.value_span.start..)
            .and_then(|value| value.chars().next())
            .filter(|first| matches!(first, '\'' | '"'));
        let findings = known_roots.findings(view, &parsed, &expression.text, expression.dialect.parser(), quote);
        for finding in findings {
            let span = crate::providers::dsl::finding_span(&finding).clone();
            // A fix edits authored bytes, so it rides only on an exact range
            // whose text is the expression's.
            let doc_span = expression.range(span.clone());
            let finding = if expression.is_exact() && ctx.text.get(doc_span.clone()) == expression.text.get(span) {
                finding
            } else {
                without_fix(finding)
            };
            if let Some(range) = ctx.source_map.byte_range_to_lsp(doc_span) {
                out.push(crate::providers::dsl::undeclared_property_diagnostic(range, source::FRONTMATTER, &finding));
            }
        }
    }
}

/// The library's call check of every surface of the document's lifecycle
/// under its host ([`LifecycleHost`](crate::overlay::event_views::LifecycleHost):
/// the host's recorded profile, else `md`), through
/// [`darkmatter::lifecycle::lifecycle_call_checks`]: each typed at its own
/// site, narrowed by the `when` items around it, with facts ended by writes
/// and never carried across events, exactly as the host's composition types
/// it.
///
/// Empty when the lifecycle does not parse under the host (the parser's
/// error to report); a stack scalar the library does not check (a `set`
/// shell value) keeps the unguarded per-expression check.
struct GuardedStackCalls {
    surfaces: std::collections::HashMap<Vec<darkmatter::lifecycle::AuthoredSegment>, darkmatter::lifecycle::LifecycleSurfaceCalls>,
}

impl GuardedStackCalls {
    fn for_document(ctx: &DocumentContext, host: &crate::overlay::event_views::LifecycleHost) -> Self {
        let mut surfaces = std::collections::HashMap::new();
        if let Some(bundle) = ctx.overlay.and_then(|overlay| overlay.bundle())
            && let Ok(lifecycle) = host.parse_lifecycle(&bundle.frontmatter_json, ctx.path)
        {
            let found = darkmatter::lifecycle::lifecycle_call_checks(
                &bundle.frontmatter_json,
                &lifecycle,
                host.registry(),
                Some(&bundle.effective),
            );
            for surface in found {
                surfaces.insert(surface.location.clone(), surface);
            }
        }
        Self { surfaces }
    }

    /// The library's surface at the frontmatter entry `index`.
    fn surface(&self, ast: &FrontmatterAst, index: usize) -> Option<&darkmatter::lifecycle::LifecycleSurfaceCalls> {
        let location: Vec<darkmatter::lifecycle::AuthoredSegment> = ast
            .path_at(index)
            .into_iter()
            .map(|segment| match segment {
                crate::overlay::FmPathSegment::Key(key) => darkmatter::lifecycle::AuthoredSegment::Key(key.to_string()),
                crate::overlay::FmPathSegment::Index(index) => darkmatter::lifecycle::AuthoredSegment::Index(index),
            })
            .collect();
        self.surfaces.get(&location)
    }
}

/// The design-time call check over the `{{ … }}` spans of frontmatter values
/// outside lifecycle stacks (those are [`stack_expression_diagnostics`]'),
/// which frontmatter interpolation evaluates. `$schema` declarations are not
/// interpolated; a span that does not parse, and a stale tree, report
/// nothing.
fn interpolation_call_diagnostics(
    ctx: &DocumentContext,
    ast: &FrontmatterAst,
    lifecycle: &crate::overlay::event_views::LifecycleHost,
    out: &mut Vec<Diagnostic>,
) {
    if ctx.overlay.is_some_and(|overlay| overlay.stale) {
        return;
    }
    let declared = crate::diagnostics::call_check::declared_types(ctx);
    let calls = FrontmatterCalls::new(ctx, &declared);
    for interpolation in crate::overlay::expressions::frontmatter_interpolations(ctx.text, ast) {
        let path = ast.path_at(interpolation.index);
        if matches!(path.first(), Some(crate::overlay::FmPathSegment::Key("$schema")))
            || lifecycle.stack_event(&path).is_some()
        {
            continue;
        }
        let Ok(parsed) = crate::overlay::expressions::parse(&interpolation.text) else {
            continue;
        };
        for found in calls.check(&parsed) {
            let doc_span = interpolation.project(found.span.clone()).unwrap_or_else(|| interpolation.outer_span());
            calls.push(ctx, doc_span, found, out);
        }
    }
}

/// The frontmatter's call-check site: unguarded over the declaration-only
/// types. A lifecycle stack's own surfaces are narrowed by its `when` items
/// through [`GuardedStackCalls`]; this site serves the rest.
struct FrontmatterCalls<'d> {
    site: darkmatter::markdown::compose::expression::TypeScope<'d>,
    strict: bool,
}

impl<'d> FrontmatterCalls<'d> {
    fn new(ctx: &DocumentContext, declared: &'d darkmatter::markdown::compose::expression::DeclaredTypes<'d>) -> Self {
        Self {
            site: darkmatter::markdown::compose::expression::TypeScope::unguarded(declared),
            strict: ctx.config.expressions.strict,
        }
    }

    fn check(
        &self,
        expr: &darkmatter::markdown::compose::expression::SpannedExpr,
    ) -> Vec<crate::diagnostics::call_check::CallDiagnostic> {
        crate::diagnostics::call_check::call_diagnostics(expr, &self.site, self.strict)
    }

    fn push(
        &self,
        ctx: &DocumentContext,
        doc_span: std::ops::Range<usize>,
        found: crate::diagnostics::call_check::CallDiagnostic,
        out: &mut Vec<Diagnostic>,
    ) {
        if let Some(range) = ctx.source_map.byte_range_to_lsp(doc_span) {
            out.push(crate::diagnostics::call_check::lsp_diagnostic(range, source::FRONTMATTER, found));
        }
    }
}

#[cfg(test)]
thread_local! {
    /// Expressions [`expression_diagnostics`] parsed on this thread; the
    /// shared-alias-target regressions read it.
    pub(crate) static EXPRESSION_DIAGNOSTIC_PARSES: std::cell::Cell<usize> =
        const { std::cell::Cell::new(0) };
}

#[inline]
fn note_expression_diagnostic_parse() {
    #[cfg(test)]
    EXPRESSION_DIAGNOSTIC_PARSES.with(|parses| parses.set(parses.get() + 1));
}

/// The YAML quote character around an expression value, which a quick-fix
/// replacement must not contain.
///
/// Read just before the expression's first authored byte rather than from the
/// entry's value span, which for an alias is the `*name` token, not the quoted
/// scalar the fix would edit.
fn yaml_quote(ctx: &DocumentContext, value: &ExpressionValue<'_>) -> Option<char> {
    ctx.text
        .get(..value.expression_span().start)
        .and_then(|before| before.chars().next_back())
        .filter(|last| matches!(last, '\'' | '"'))
}

fn without_fix(
    finding: crate::overlay::expressions::UndeclaredPropertyFinding,
) -> crate::overlay::expressions::UndeclaredPropertyFinding {
    match finding {
        crate::overlay::expressions::UndeclaredPropertyFinding::DashSeparatedKey {
            authored,
            key,
            span,
            ..
        } => crate::overlay::expressions::UndeclaredPropertyFinding::DashSeparatedKey {
            authored,
            key,
            span,
            fix: None,
        },
        other => other,
    }
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
    let path = origin.file()?;
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
        let Some(range) = entry
            .key_span
            .clone()
            .and_then(|key| ctx.source_map.byte_range_to_lsp(key))
        else {
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

/// Suggestion-lint diagnostics from `suggest(...)` candidate problems.
///
/// Emits one `dm.schema.invalid_suggestion` `WARNING` per invalid candidate on
/// the exact authored argument range, plus a `dm.schema.document_malformed`
/// `ERROR` for a malformed recognized standalone envelope. For a standalone
/// schema document the warnings are owned by that document and are never
/// duplicated onto consuming Markdown documents' `$schema` references.
fn suggestion_diagnostics(
    ctx: &DocumentContext,
    suggestions: &SuggestionState,
    out: &mut Vec<Diagnostic>,
) {
    match suggestions {
        SuggestionState::Inactive | SuggestionState::TriggerError(_) | SuggestionState::TriggerWarning(_) => {}
        SuggestionState::Inline(problems) => {
            for problem in problems {
                out.push(suggestion_warning(ctx, problem));
            }
        }
        SuggestionState::Standalone { problems, error, .. } => {
            for problem in problems {
                out.push(suggestion_warning(ctx, problem));
            }
            if let Some(error) = error {
                out.push(standalone_error_diagnostic(ctx, error));
            }
        }
    }
}

/// One `dm.schema.invalid_suggestion` warning ranged at the candidate argument.
fn suggestion_warning(ctx: &DocumentContext, problem: &SuggestionLintProblem) -> Diagnostic {
    let range = ctx
        .source_map
        .byte_range_to_lsp(problem.span.clone())
        .unwrap_or_else(zero_range);
    let message = suggestion_message(problem);
    diagnostic(
        range,
        DiagnosticSeverity::WARNING,
        source::SCHEMA,
        code::SCHEMA_INVALID_SUGGESTION,
        message,
    )
}

/// A reason-specific message for an invalid suggestion candidate.
fn suggestion_message(problem: &SuggestionLintProblem) -> String {
    let decoded = &problem.decoded;
    match problem.reason {
        SuggestionLintReason::InvalidDecimalSyntax => {
            format!("suggestion `{decoded}` is not a valid simple decimal number")
        }
        SuggestionLintReason::UnsupportedNumberRepresentation => {
            format!("suggestion `{decoded}` cannot be losslessly represented as a JSON number")
        }
        SuggestionLintReason::Range => {
            format!("suggestion `{decoded}` is outside the allowed range")
        }
        SuggestionLintReason::Integer => {
            format!("suggestion `{decoded}` must be an integer")
        }
        SuggestionLintReason::Length => {
            format!("suggestion `{decoded}` violates a length constraint")
        }
        SuggestionLintReason::NotEmpty => {
            format!("suggestion `{decoded}` must not be empty or whitespace-only")
        }
        SuggestionLintReason::Pattern => {
            format!("suggestion `{decoded}` does not match the required pattern")
        }
        SuggestionLintReason::Type => {
            format!("suggestion `{decoded}` does not match the target type")
        }
        _ => format!("suggestion `{decoded}` is invalid"),
    }
}

/// A malformed recognized standalone envelope.
///
/// The three outcomes are separate client contracts and are attempted in
/// specificity order: a rejected **inner** property definition is
/// `dm.schema.invalid_type_definition`, a rejected **outer** declaration is
/// `dm.schema.invalid_schema_shape`, and only a buffer that is not a usable
/// YAML envelope at all falls back to `dm.schema.document_malformed` over the
/// whole document. Collapsing the middle case into the fallback would cost
/// clients the ability to tell a declaration-shape failure from a document
/// YAML failure.
fn standalone_error_diagnostic(ctx: &DocumentContext, error: &SchemaError) -> Diagnostic {
    if let Some(diagnostic) = standalone_type_definition_diagnostic(ctx) {
        return diagnostic;
    }
    if let Some(diagnostic) = standalone_declaration_diagnostic(ctx) {
        return diagnostic;
    }
    let span = 0..ctx.text.len();
    let range = ctx
        .source_map
        .byte_range_to_lsp(span)
        .unwrap_or_else(zero_range);
    diagnostic(
        range,
        DiagnosticSeverity::ERROR,
        source::SCHEMA,
        code::SCHEMA_DOCUMENT_MALFORMED,
        error.to_string(),
    )
}

/// A standalone definition the shared parser rejects, ranged at the library's
/// located token, then the rejected value, then the enclosing definition, and
/// only then the whole document. `None` leaves the failure to the outer
/// declaration and document diagnostics.
fn standalone_type_definition_diagnostic(ctx: &DocumentContext) -> Option<Diagnostic> {
    let failure = locate_standalone_definition_failure(ctx.text)?;
    let span = failure
        .authored_span
        .clone()
        .or_else(|| failure.value_span.clone())
        .or_else(|| {
            enclosing_entry_span(ctx.text, &FrontmatterAst::parse_yaml(ctx.text).ast?, &[], &failure.path)
        })
        .unwrap_or(0..ctx.text.len());
    let range = ctx.source_map.byte_range_to_lsp(span).unwrap_or_else(zero_range);
    let mut diagnostic = diagnostic(
        range,
        DiagnosticSeverity::ERROR,
        source::SCHEMA,
        code::SCHEMA_INVALID_TYPE_DEFINITION,
        failure.error.to_string(),
    );
    let related = located_related(ctx, &failure.related);
    diagnostic.related_information = (!related.is_empty()).then_some(related);
    Some(diagnostic)
}

/// The value span of the deepest entry `ast` holds along `path`'s leading
/// property keys (under `prefix`), for a failure whose own node could not be
/// located. Keys are matched as whole segments, so a key containing `.` is
/// never split.
fn enclosing_entry_span(
    text: &str,
    ast: &FrontmatterAst,
    prefix: &[&str],
    path: &SchemaSourcePath,
) -> Option<std::ops::Range<usize>> {
    let mut keys: Vec<&str> = prefix.to_vec();
    keys.extend(
        path.segments()
            .iter()
            .map_while(|segment| match segment {
                SchemaSourcePathSegment::Property(key) => Some(key.as_str()),
                SchemaSourcePathSegment::UnionArm(_) => None,
            }),
    );
    (1..=keys.len())
        .rev()
        .find_map(|length| ast.entry_by_key_path(&keys[..length]))
        .map(|entry| complete_flow_value_span(text, entry.value_span.clone()))
}

/// An invalid **outer** standalone declaration, ranged at the smallest authored
/// value the shared declaration parser rejects.
///
/// Only a sequence payload (root union) or a scalar payload (whole-file
/// reference) is an outer declaration. A mapping payload's failure belongs to
/// one of its property definitions, and
/// [`standalone_type_definition_diagnostic`] has already had its chance to
/// range that — returning `None` here is what keeps the inner/outer code split
/// intact.
///
/// ## Notes
///
/// Spans come from the library's structural locator rather than a text search,
/// so an offending union arm keeps its exact authored range. Reference arms are
/// classified through the same passive path the declaration parser uses, which
/// performs no existence check or file I/O.
fn standalone_declaration_diagnostic(ctx: &DocumentContext) -> Option<Diagnostic> {
    let yaml: serde_yaml_ng::Value = serde_yaml_ng::from_str(ctx.text).ok()?;
    let mapping = yaml.as_mapping()?;
    let payload_key = standalone_payload_key(mapping);
    let payload = mapping.get(serde_yaml_ng::Value::String(payload_key.to_string()))?;
    if payload.as_mapping().is_some() {
        return None;
    }
    let error = parse_schema_declaration(payload).err()?;
    let node = standalone_payload_node(ctx.text, payload_key)?;
    let span = invalid_union_arm_span(payload, &node).unwrap_or_else(|| node.span.clone());
    let range = ctx.source_map.byte_range_to_lsp(span)?;
    Some(diagnostic(
        range,
        DiagnosticSeverity::ERROR,
        source::SCHEMA,
        code::SCHEMA_INVALID_SHAPE,
        error.to_string(),
    ))
}

/// The payload key of whichever envelope claimed a standalone document.
fn standalone_payload_key(mapping: &serde_yaml_ng::Mapping) -> &'static str {
    let tagged = mapping
        .get(serde_yaml_ng::Value::String("kind".to_string()))
        .and_then(serde_yaml_ng::Value::as_str)
        == Some("schema");
    if tagged { "types" } else { "$schema" }
}

/// The located payload value of a standalone envelope, in document offsets.
fn standalone_payload_node(text: &str, payload_key: &str) -> Option<SchemaValueNode> {
    let SchemaValueKind::Mapping(entries) = locate_schema_value(text, 0)?.kind else {
        return None;
    };
    entries
        .into_iter()
        .find(|entry| entry.key == payload_key)
        .map(|entry| entry.value)
}

/// The authored span of the first root-union arm the declaration parser
/// rejects, or `None` when the payload is not a union or every arm is
/// individually acceptable (an empty union fails as a whole, not per arm).
fn invalid_union_arm_span(
    payload: &serde_yaml_ng::Value,
    node: &SchemaValueNode,
) -> Option<std::ops::Range<usize>> {
    let (serde_yaml_ng::Value::Sequence(items), SchemaValueKind::Sequence(arms)) =
        (payload, &node.kind)
    else {
        return None;
    };
    if items.len() != arms.len() {
        return None;
    }
    items
        .iter()
        .zip(arms)
        .find(|(item, _)| arm_is_rejected(item))
        .map(|(_, arm)| arm.span.clone())
}

/// Whether one root-union arm is rejected on its own terms.
///
/// Mirrors `parse_schema_declaration`'s arm handling: a mapping is an inline
/// shape, a string is a file reference classified without I/O, and anything
/// else is not a legal arm.
fn arm_is_rejected(item: &serde_yaml_ng::Value) -> bool {
    match item {
        serde_yaml_ng::Value::Mapping(_) => parse_yaml_schema(item).is_err(),
        serde_yaml_ng::Value::String(reference) => classify_schema_reference(reference).is_err(),
        _ => true,
    }
}

/// The `(code, severity)` for a validation-problem category.
/// Maps a validation problem to its stable code + severity, or `None` when the
/// problem should not be diagnosed at edit time.
///
/// `MissingRequired` is `None` outside strict mode: `required` is a
/// **compose-time** contract (the value arrives via CLI `--set`, seed values, or
/// Claudine's interactive prompt), so a statically-absent required key is not an
/// editor error — the same "resolved at compose time, don't diagnose here" rule
/// the deferred `{{ }}` / `$(...)` values follow. `md compose` / `md schema
/// validate` still catch genuine omissions with the injected values present. In
/// strict mode it is an opt-in `ERROR`, mirroring how `UnknownKey` escalates.
fn classify(
    code: ValidationProblemCode,
    strict: bool,
) -> Option<(&'static str, DiagnosticSeverity)> {
    Some(match code {
        ValidationProblemCode::MissingRequired => {
            if !strict {
                return None;
            }
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
    })
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

/// The byte span from a claimed trigger envelope's YAML error position to the
/// end of that line, when the error (directly or inside a trigger load) is
/// [`SchemaError::TriggerEnvelopeYaml`] with a location.
///
/// The library reports a one-based line and a one-based column counted in
/// characters of the source with any initial BOM removed, so the column is
/// walked in characters here and the source map converts the byte span into
/// the negotiated position encoding.
fn trigger_yaml_span(text: &str, error: &SchemaError) -> Option<std::ops::Range<usize>> {
    let (line, column) = match error {
        SchemaError::TriggerEnvelopeYaml { line: Some(line), column, .. } => (*line, column.unwrap_or(1)),
        SchemaError::TriggerLoad { source, .. } => return trigger_yaml_span(text, source),
        _ => return None,
    };
    let mut line_start = 0;
    for _ in 1..line {
        line_start += text[line_start..].find('\n')? + 1;
    }
    if line == 1 && text.starts_with('\u{feff}') {
        line_start = '\u{feff}'.len_utf8();
    }
    let line_text = &text[line_start..];
    let line_end = line_start + line_text.find('\n').unwrap_or(line_text.len());
    let line_end = if text[..line_end].ends_with('\r') { line_end - 1 } else { line_end };
    let start = text[line_start..line_end]
        .char_indices()
        .nth(column.saturating_sub(1))
        .map_or(line_end, |(offset, _)| line_start + offset);
    Some(start..line_end)
}

/// A zero-width range at the document start (fallback when a span cannot
/// convert).
fn zero_range() -> Range {
    Range::default()
}

#[cfg(test)]
mod severity_tests;

#[cfg(test)]
mod position_parity_tests;

#[cfg(test)]
mod tests {
    use super::*;

    use std::collections::BTreeMap;
    use std::path::{Path, PathBuf};
    use std::sync::Arc;

    use lsp_types::{InitializeParams, NumberOrString, Uri};

    use crate::capabilities::ClientProfile;
    use crate::config::DmlsConfig;
    use crate::graph::WorkspaceGraph;
    use crate::overlay::OverlayState;
    use crate::source_map::PositionEncoding;

    /// Builds a real overlay-backed context for `text`, runs `diagnostics`, and
    /// hands the diagnostics to `f`.
    fn diagnostics_for<R>(text: &str, f: impl FnOnce(&[Diagnostic]) -> R) -> R {
        diagnostics_with(text, PositionEncoding::Utf16, &DmlsConfig::default(), |diagnostics, _| {
            f(diagnostics)
        })
    }

    /// [`diagnostics_for`] with the client's negotiated `encoding` and an
    /// explicit `config`; `f` also receives the context's source map, so a
    /// caller can convert the diagnostics' positions back to byte offsets.
    pub(super) fn diagnostics_with<R>(
        text: &str,
        encoding: PositionEncoding,
        config: &DmlsConfig,
        f: impl FnOnce(&[Diagnostic], &SourceMap) -> R,
    ) -> R {
        let path = Path::new("/w/doc.md");
        let uri: Uri = "file:///w/doc.md".parse().unwrap();
        let roots = [PathBuf::from("/w")];
        let state = OverlayState::default();
        let overlay = state.for_document(&uri, text, path, config, &roots, &crate::context::test_support::resolution_for(path));
        let source_map = SourceMap::new(uri.clone(), 1, encoding, Arc::from(text));
        let graph = WorkspaceGraph::build(&BTreeMap::new(), 1, &crate::context::test_support::workspace_contexts());
        let profile = ClientProfile::from_initialize(&InitializeParams::default(), encoding);
        let ctx = DocumentContext {
            uri: &uri,
            path,
            text,
            source_map: &source_map,
            graph: &graph,
            doc_id: None,
            config,
            profile: &profile,
            overlay: overlay.as_ref(),
            resolution: &crate::context::test_support::resolution_for(path),
        };
        f(&diagnostics(&ctx), &source_map)
    }

    fn expression_doc(value_line: &str) -> String {
        format!("---\n$schema:\n  when: expression\n{value_line}\n---\n\nbody\n")
    }

    fn code_of(diagnostic: &Diagnostic) -> Option<&str> {
        match &diagnostic.code {
            Some(NumberOrString::String(code)) => Some(code.as_str()),
            _ => None,
        }
    }

    /// The source text a diagnostic's range covers (single-line ranges only).
    fn ranged_text<'t>(text: &'t str, diagnostic: &Diagnostic) -> &'t str {
        let line = text.lines().nth(diagnostic.range.start.line as usize).unwrap();
        assert_eq!(diagnostic.range.start.line, diagnostic.range.end.line);
        &line[diagnostic.range.start.character as usize..diagnostic.range.end.character as usize]
    }

    #[test]
    fn malformed_expression_producer_is_gated_to_exactly_projected_styles() {
        // Exact styles: the dedicated diagnostic, ranged on the authored
        // expression text (quotes excluded), replacing the generic problem.
        for (line, expected) in [
            ("when: 1 +", "1 +"),
            ("when: '1 +'", "1 +"),
            ("when: \"1 +\"", "1 +"),
        ] {
            let text = expression_doc(line);
            diagnostics_for(&text, |diagnostics| {
                let malformed: Vec<&Diagnostic> = diagnostics
                    .iter()
                    .filter(|d| code_of(d) == Some(code::EXPRESSION_MALFORMED))
                    .collect();
                assert_eq!(malformed.len(), 1, "{line}: {diagnostics:#?}");
                assert_eq!(ranged_text(&text, malformed[0]), expected, "{line}");
                assert_eq!(malformed[0].severity, Some(DiagnosticSeverity::ERROR), "{line}");
                assert!(
                    diagnostics.iter().all(|d| d.source.as_deref() != Some(source::SCHEMA)),
                    "{line}: {diagnostics:#?}"
                );
            });
        }
        // No exact map: no dedicated diagnostic with a fabricated range or
        // message; the generic schema problem remains.
        for value in [
            "when: |\n  1 +",
            "when: |-\n  1 +",
            "when: |+\n  1 +\n",
            "when: |2-\n   1 +",
            "when: >\n  1 +",
            "when: >-\n  1 +",
            "when: !!str 1 +",
            "when: \"1\n  +\"",
        ] {
            let text = expression_doc(value);
            diagnostics_for(&text, |diagnostics| {
                assert!(
                    diagnostics.iter().all(|d| code_of(d) != Some(code::EXPRESSION_MALFORMED)),
                    "{value:?}: {diagnostics:#?}"
                );
                assert!(
                    diagnostics.iter().any(|d| d.source.as_deref() == Some(source::SCHEMA)),
                    "{value:?} keeps the schema problem: {diagnostics:#?}"
                );
            });
        }
        // An alias is not a scalar entry: no expression diagnostic at the alias.
        let text = "---\n$schema:\n  when: expression\n  other: string\nother: &bad '1 +'\nwhen: *bad\n---\n\nbody\n";
        diagnostics_for(text, |diagnostics| {
            assert!(
                diagnostics.iter().all(|d| code_of(d) != Some(code::EXPRESSION_MALFORMED)),
                "{diagnostics:#?}"
            );
        });
    }

    #[test]
    fn array_item_schema_problem_ranges_the_failing_item() {
        // Sequence descent (spec D6): the problem path `/tags/1` is an authored
        // item entry, so the range is that element, not the whole sequence.
        let text = "---\n$schema:\n  tags: number[]\ntags:\n  - 1\n  - oops\n  - 3\n---\n\nbody\n";
        diagnostics_for(text, |diagnostics| {
            let problems: Vec<&Diagnostic> = diagnostics
                .iter()
                .filter(|d| d.source.as_deref() == Some(source::SCHEMA))
                .collect();
            assert_eq!(problems.len(), 1, "{diagnostics:#?}");
            assert_eq!(ranged_text(text, problems[0]), "oops", "{problems:#?}");
            assert_eq!(problems[0].range.start.line, 5);
        });
    }

    #[test]
    fn malformed_expression_value_emits_one_frontmatter_diagnostic_and_no_schema_duplicate() {
        // A quoted, malformed expression: one `dm.expression.malformed` under
        // source `darkmatter.frontmatter`, and the generic `dm.schema.constraint`
        // that the format failure would raise is suppressed.
        let text = expression_doc("when: '1 +'");
        diagnostics_for(&text, |diagnostics| {
            let malformed: Vec<&Diagnostic> = diagnostics
                .iter()
                .filter(|diagnostic| code_of(diagnostic) == Some(code::EXPRESSION_MALFORMED))
                .collect();
            assert_eq!(malformed.len(), 1, "exactly one malformed diagnostic: {diagnostics:#?}");
            assert_eq!(malformed[0].source.as_deref(), Some(source::FRONTMATTER));
            assert!(
                diagnostics
                    .iter()
                    .all(|diagnostic| code_of(diagnostic) != Some(code::SCHEMA_CONSTRAINT)),
                "the generic format constraint must be suppressed: {diagnostics:#?}"
            );
            // The range excludes the YAML quotes: it sits inside `'1 +'`.
            let value_start = text.find("'1 +'").unwrap();
            let range = malformed[0].range;
            let quote_pos = crate::source_map::SourceMap::new(
                "file:///w/doc.md".parse().unwrap(),
                1,
                PositionEncoding::Utf16,
                Arc::from(text.as_str()),
            )
            .byte_to_lsp(value_start)
            .unwrap();
            assert!(
                range.start.character > quote_pos.character,
                "range must start past the opening quote"
            );
        });
    }

    #[test]
    fn nested_schema_conversion_error_points_to_the_nested_value() {
        let text = "---\n$schema:\n  meta:\n    prompt: string(integer)\n---\n\nbody\n";
        diagnostics_for(text, |diagnostics| {
            let diagnostic = diagnostics
                .iter()
                .find(|diagnostic| {
                    code_of(diagnostic) == Some(code::SCHEMA_INVALID_TYPE_DEFINITION)
                })
                .expect("nested schema diagnostic");
            assert_eq!(diagnostic.range.start.line, 3, "{diagnostics:#?}");
            assert_eq!(diagnostic.range.end.line, 3, "{diagnostics:#?}");
        });
    }

    /// Anchoring resolves a property *name*, so a nested leaf that shares its
    /// name with a valid top-level property is the case where an unqualified
    /// name would squiggle correct authoring.
    #[test]
    fn nested_conversion_error_does_not_anchor_on_a_same_named_top_level_property() {
        let text = concat!(
            "---\n",
            "$schema:\n",
            "  prompt: string(required)\n",
            "  meta:\n",
            "    prompt: string(integer)\n",
            "prompt: hello\n",
            "---\n",
            "\n",
            "body\n",
        );
        diagnostics_for(text, |diagnostics| {
            let diagnostic = diagnostics
                .iter()
                .find(|diagnostic| {
                    code_of(diagnostic) == Some(code::SCHEMA_INVALID_TYPE_DEFINITION)
                })
                .expect("nested schema diagnostic");
            assert_eq!(diagnostic.range.start.line, 4, "{diagnostics:#?}");
            assert_eq!(diagnostic.range.end.line, 4, "{diagnostics:#?}");
            assert!(
                diagnostics
                    .iter()
                    .all(|other| other.range.start.line == 4 && other.range.end.line == 4),
                "the valid top-level `prompt` must stay clean: {diagnostics:#?}"
            );
        });
    }

    #[test]
    fn unknown_expression_root_is_a_warning_frontmatter_diagnostic() {
        let text = expression_doc("when: mystery");
        diagnostics_for(&text, |diagnostics| {
            let unknown: Vec<&Diagnostic> = diagnostics
                .iter()
                .filter(|diagnostic| code_of(diagnostic) == Some(code::EXPRESSION_UNDECLARED_PROPERTY))
                .collect();
            assert_eq!(unknown.len(), 1, "{diagnostics:#?}");
            assert_eq!(unknown[0].source.as_deref(), Some(source::FRONTMATTER));
            assert_eq!(unknown[0].severity, Some(DiagnosticSeverity::WARNING));
        });
    }

    #[test]
    fn native_scalar_coercion_produces_no_expression_false_diagnostics() {
        // A native boolean is coerced to a canonical expression string at compose
        // time, so `when: true` is valid — no type mismatch, no malformed, no
        // undeclared-property squiggle.
        let text = expression_doc("when: true");
        diagnostics_for(&text, |diagnostics| {
            assert!(
                diagnostics.iter().all(|diagnostic| {
                    let code = code_of(diagnostic);
                    code != Some(code::SCHEMA_TYPE_MISMATCH)
                        && code != Some(code::EXPRESSION_MALFORMED)
                        && code != Some(code::EXPRESSION_UNDECLARED_PROPERTY)
                }),
                "native scalar must not produce false diagnostics: {diagnostics:#?}"
            );
        });
        // A native number likewise coerces cleanly.
        let text = expression_doc("when: 42");
        diagnostics_for(&text, |diagnostics| {
            assert!(
                diagnostics.iter().all(|diagnostic| code_of(diagnostic) != Some(code::SCHEMA_TYPE_MISMATCH)),
                "native number must not produce a type mismatch: {diagnostics:#?}"
            );
        });
    }

    #[test]
    fn mapping_value_on_expression_property_stays_a_type_mismatch() {
        // A non-scalar value is not an expression; the schema type mismatch is
        // retained rather than swallowed by the expression pass.
        let text = "---\n$schema:\n  when: expression\nwhen:\n  nested: 1\n---\n\nbody\n";
        diagnostics_for(text, |diagnostics| {
            // The non-scalar value is not in the suppressed expression set, so its
            // schema failure (a type/constraint problem) is retained, not swallowed.
            assert!(
                diagnostics.iter().any(|diagnostic| {
                    matches!(
                        code_of(diagnostic),
                        Some(code::SCHEMA_TYPE_MISMATCH | code::SCHEMA_CONSTRAINT)
                    )
                }),
                "a mapping on an expression property must stay a schema error: {diagnostics:#?}"
            );
        });
    }

    #[test]
    fn valid_expression_value_produces_no_diagnostics() {
        let text = expression_doc("when: ctx.today");
        diagnostics_for(&text, |diagnostics| {
            assert!(
                diagnostics.iter().all(|diagnostic| {
                    let code = code_of(diagnostic);
                    code != Some(code::EXPRESSION_MALFORMED)
                        && code != Some(code::EXPRESSION_UNDECLARED_PROPERTY)
                        && code != Some(code::SCHEMA_CONSTRAINT)
                        && code != Some(code::SCHEMA_TYPE_MISMATCH)
                }),
                "a valid expression must be clean: {diagnostics:#?}"
            );
        });
    }

    #[test]
    fn condition_dialect_and_expression_is_not_malformed() {
        // The `expression` schema format validates with `parse_condition`, which
        // accepts `&&`; the spec's own example must not be diagnosed malformed.
        let text = expression_doc(r#"when: 'is_agent() && os == "macos"'"#);
        diagnostics_for(&text, |diagnostics| {
            assert!(
                diagnostics.iter().all(|diagnostic| {
                    let code = code_of(diagnostic);
                    code != Some(code::EXPRESSION_MALFORMED)
                        && code != Some(code::SCHEMA_CONSTRAINT)
                }),
                "a condition-dialect `&&` expression must not be malformed: {diagnostics:#?}"
            );
        });
    }

    #[test]
    fn condition_dialect_or_expression_is_not_malformed() {
        let text = expression_doc(r#"when: 'os == "macos" || os == "linux"'"#);
        diagnostics_for(&text, |diagnostics| {
            assert!(
                diagnostics.iter().all(|diagnostic| {
                    let code = code_of(diagnostic);
                    code != Some(code::EXPRESSION_MALFORMED)
                        && code != Some(code::SCHEMA_CONSTRAINT)
                }),
                "a condition-dialect `||` expression must not be malformed: {diagnostics:#?}"
            );
        });
    }

    /// A discriminated inline-object union document whose `change` property is
    /// tagged by a shared literal `kind`.
    fn discriminated_union_doc(change_body: &str) -> String {
        format!(
            concat!(
                "---\n",
                "$schema:\n",
                "  change:\n",
                "    - \"{{ kind: literal(created), path: string }}\"\n",
                "    - \"{{ kind: literal(deleted), reason: string }}\"\n",
                "change:\n",
                "{change_body}",
                "---\n\nbody\n",
            ),
            change_body = change_body,
        )
    }

    #[test]
    fn discriminated_union_narrows_unknown_key_to_selected_arm() {
        // `kind: created` selects the created arm (only `path`), so `reason` is
        // an unknown key — the library's shared discriminant selector (Phase 4)
        // narrows the `anyOf` report and DMLS inherits it.
        let text = discriminated_union_doc("  kind: created\n  reason: oops\n");
        diagnostics_for(&text, |diagnostics| {
            assert!(
                diagnostics
                    .iter()
                    .any(|diagnostic| code_of(diagnostic) == Some(code::SCHEMA_UNKNOWN_KEY)),
                "reason is unknown under the created arm: {diagnostics:#?}"
            );
        });

        // Switching the discriminant to `deleted` selects the arm that declares
        // `reason`, so no unknown-key diagnostic remains.
        let text = discriminated_union_doc("  kind: deleted\n  reason: gone\n");
        diagnostics_for(&text, |diagnostics| {
            assert!(
                diagnostics
                    .iter()
                    .all(|diagnostic| code_of(diagnostic) != Some(code::SCHEMA_UNKNOWN_KEY)),
                "reason is valid under the deleted arm: {diagnostics:#?}"
            );
        });
    }

    /// A `change` union whose two arms type `when` divergently (string vs
    /// expression) under a shared literal `kind` discriminant. `deleted_first`
    /// swaps arm order so a test can select the expression arm from either
    /// position; `change_body` is the authored `change:` mapping.
    fn expression_arm_union_doc(deleted_first: bool, change_body: &str) -> String {
        let created = r#"    - "{ kind: literal(created), when: string }""#;
        let deleted = r#"    - "{ kind: literal(deleted), when: expression }""#;
        let (first, second) = if deleted_first { (deleted, created) } else { (created, deleted) };
        format!("---\n$schema:\n  change:\n{first}\n{second}\nchange:\n{change_body}---\n\nbody\n")
    }

    #[test]
    fn malformed_selected_arm_expression_value_emits_malformed_diagnostic() {
        // With `kind: deleted`, the deleted arm's `when: expression` governs, so a
        // malformed value there emits `dm.expression.malformed`. The value
        // resolver must select the nested arm from either position.
        for deleted_first in [false, true] {
            let text = expression_arm_union_doc(deleted_first, "  kind: deleted\n  when: '1 +'\n");
            diagnostics_for(&text, |diagnostics| {
                assert!(
                    diagnostics
                        .iter()
                        .any(|diagnostic| code_of(diagnostic) == Some(code::EXPRESSION_MALFORMED)),
                    "malformed selected-arm expression emits dm.expression.malformed (deleted_first={deleted_first}): {diagnostics:#?}"
                );
            });
        }
    }

    #[test]
    fn malformed_value_under_non_selected_arm_type_emits_no_expression_diagnostic() {
        // With `kind: created`, `when` is a string, so `'1 +'` is a valid value
        // and no expression diagnostic fires — the sibling deleted arm's
        // `expression` type must not leak into the selected created arm.
        for deleted_first in [false, true] {
            let text = expression_arm_union_doc(deleted_first, "  kind: created\n  when: '1 +'\n");
            diagnostics_for(&text, |diagnostics| {
                assert!(
                    diagnostics
                        .iter()
                        .all(|diagnostic| code_of(diagnostic) != Some(code::EXPRESSION_MALFORMED)),
                    "no expression diagnostic for a string-typed selected arm (deleted_first={deleted_first}): {diagnostics:#?}"
                );
            });
        }
    }

    /// A top-level `when` property typed as a **mixed union** of an `expression`
    /// arm and one other `arm`, in both arm orders (`expression_first`).
    fn mixed_expression_union_doc(expression_first: bool, arm: &str, value_line: &str) -> String {
        let expr = "    - expression".to_string();
        let other = format!("    - \"{arm}\"");
        let (first, second) = if expression_first { (expr, other) } else { (other, expr) };
        format!("---\n$schema:\n  when:\n{first}\n{second}\n{value_line}\n---\n\nbody\n")
    }

    #[test]
    fn alternate_valid_enum_arm_suppresses_malformed_expression() {
        // `'1 +'` is malformed as an expression but is a valid member of the enum
        // arm, so the union validates and no `dm.expression.malformed` fires.
        for expression_first in [false, true] {
            let text =
                mixed_expression_union_doc(expression_first, "enum('1 +', foo)", "when: '1 +'");
            diagnostics_for(&text, |diagnostics| {
                assert!(
                    diagnostics
                        .iter()
                        .all(|diagnostic| code_of(diagnostic) != Some(code::EXPRESSION_MALFORMED)),
                    "enum arm accepts the value; no malformed warning (expression_first={expression_first}): {diagnostics:#?}"
                );
            });
        }
    }

    #[test]
    fn alternate_valid_string_arm_suppresses_malformed_expression() {
        // A `string` arm accepts any scalar, so a value the expression arm rejects
        // is still union-valid — no malformed diagnostic.
        for expression_first in [false, true] {
            let text = mixed_expression_union_doc(expression_first, "string", "when: '1 +'");
            diagnostics_for(&text, |diagnostics| {
                assert!(
                    diagnostics
                        .iter()
                        .all(|diagnostic| code_of(diagnostic) != Some(code::EXPRESSION_MALFORMED)),
                    "string arm accepts the value; no malformed warning (expression_first={expression_first}): {diagnostics:#?}"
                );
            });
        }
    }

    #[test]
    fn value_invalid_under_every_arm_emits_one_malformed_and_no_duplicate() {
        // `'1 +'` is rejected by both the expression arm and the enum arm, so the
        // union fails: exactly one dedicated `dm.expression.malformed`, with the
        // generic format constraint suppressed (no duplicate).
        for expression_first in [false, true] {
            let text =
                mixed_expression_union_doc(expression_first, "enum(foo, bar)", "when: '1 +'");
            diagnostics_for(&text, |diagnostics| {
                let malformed = diagnostics
                    .iter()
                    .filter(|diagnostic| code_of(diagnostic) == Some(code::EXPRESSION_MALFORMED))
                    .count();
                assert_eq!(
                    malformed, 1,
                    "exactly one malformed diagnostic (expression_first={expression_first}): {diagnostics:#?}"
                );
                assert!(
                    diagnostics
                        .iter()
                        .all(|diagnostic| code_of(diagnostic) != Some(code::SCHEMA_CONSTRAINT)),
                    "the generic format constraint must be suppressed (expression_first={expression_first}): {diagnostics:#?}"
                );
            });
        }
    }

    #[test]
    fn meta_schema_diagnostics_replace_generic_keyword_failures() {
        let cases = [
            (
                "---\n$schema:\n  definition: type-definition\ndefinition: string(nope)\n---\n\nbody\n",
                "dm.schema.invalid_type_definition",
                "nope",
            ),
            (
                "---\n$schema:\n  definition: type-definition\ndefinition: 'string(nope)'\n---\n\nbody\n",
                "dm.schema.invalid_type_definition",
                "nope",
            ),
            (
                "---\n$schema:\n  definition: type-definition\ndefinition: 42\n---\n\nbody\n",
                "dm.schema.invalid_type_definition",
                "42",
            ),
            (
                "---\n$schema:\n  definition: type-definition\ndefinition: []\n---\n\nbody\n",
                "dm.schema.invalid_type_definition",
                "[]",
            ),
            (
                "---\n$schema:\n  definition: type-definition\ndefinition:\n  - nested: []\n---\n\nbody\n",
                "dm.schema.invalid_type_definition",
                "[]",
            ),
            (
                "---\n$schema:\n  declaration: schema\ndeclaration: https://example.com/schema.yaml\n---\n\nbody\n",
                code::SCHEMA_INVALID_SHAPE,
                "https://example.com/schema.yaml",
            ),
            (
                "---\n$schema:\n  declaration: schema\ndeclaration: ''\n---\n\nbody\n",
                code::SCHEMA_INVALID_SHAPE,
                "''",
            ),
        ];

        for (text, expected_code, authored_value) in cases {
            diagnostics_for(text, |diagnostics| {
                let specialized: Vec<&Diagnostic> = diagnostics
                    .iter()
                    .filter(|diagnostic| code_of(diagnostic) == Some(expected_code))
                    .collect();
                assert_eq!(
                    specialized.len(),
                    1,
                    "one specialized diagnostic for `{authored_value}`: {diagnostics:#?}"
                );
                assert!(
                    diagnostics
                        .iter()
                        .all(|diagnostic| code_of(diagnostic) != Some(code::SCHEMA_CONSTRAINT)),
                    "the generic custom-keyword diagnostic must be replaced: {diagnostics:#?}"
                );

                let source_map = SourceMap::new(
                    "file:///w/doc.md".parse().unwrap(),
                    1,
                    PositionEncoding::Utf16,
                    Arc::from(text),
                );
                let start = text.find(authored_value).unwrap();
                let expected = source_map
                    .byte_range_to_lsp(start..start + authored_value.len())
                    .expect("authored value range");
                assert_eq!(specialized[0].range, expected, "authored token, else the rejected value");
            });
        }

        let local = "---\n$schema:\n  declaration: schema\ndeclaration: ./missing.yaml\n---\n\nbody\n";
        diagnostics_for(local, |diagnostics| {
            assert!(
                diagnostics.iter().all(|diagnostic| {
                    code_of(diagnostic) != Some(code::SCHEMA_INVALID_SHAPE)
                        && code_of(diagnostic) != Some("dm.schema.invalid_type_definition")
                }),
                "a syntactically valid local schema reference is not a semantic-shape error: {diagnostics:#?}"
            );
        });

        let unresolved = "---\n$schema: ./missing.yaml\n---\n\nbody\n";
        diagnostics_for(unresolved, |diagnostics| {
            assert!(
                diagnostics
                    .iter()
                    .any(|diagnostic| code_of(diagnostic) == Some(code::SCHEMA_PREPARE)),
                "an unresolved control reference remains a preparation failure: {diagnostics:#?}"
            );
            assert!(
                diagnostics
                    .iter()
                    .all(|diagnostic| code_of(diagnostic) != Some(code::SCHEMA_INVALID_SHAPE)),
                "resolution is distinct from semantic reference syntax: {diagnostics:#?}"
            );
        });
    }

    #[test]
    fn valid_expression_under_mixed_union_is_clean() {
        // A value that parses cleanly is valid under the expression arm; no
        // expression diagnostic fires regardless of the sibling arm.
        for expression_first in [false, true] {
            let text = mixed_expression_union_doc(expression_first, "string", "when: ctx.today");
            diagnostics_for(&text, |diagnostics| {
                assert!(
                    diagnostics.iter().all(|diagnostic| {
                        let code = code_of(diagnostic);
                        code != Some(code::EXPRESSION_MALFORMED)
                            && code != Some(code::EXPRESSION_UNDECLARED_PROPERTY)
                    }),
                    "a valid expression under a mixed union is clean (expression_first={expression_first}): {diagnostics:#?}"
                );
            });
        }
    }

    #[test]
    fn missing_required_is_off_by_default_and_error_in_strict() {
        // `required` is a compose-time contract: no edit-time diagnostic unless
        // the workspace opts into strict schema mode.
        assert_eq!(classify(ValidationProblemCode::MissingRequired, false), None);
        assert_eq!(
            classify(ValidationProblemCode::MissingRequired, true),
            Some((code::SCHEMA_MISSING_REQUIRED, DiagnosticSeverity::ERROR)),
        );
    }

    #[test]
    fn unknown_key_warns_in_non_strict_and_errors_in_strict() {
        assert_eq!(
            classify(ValidationProblemCode::UnknownKey, false),
            Some((code::SCHEMA_UNKNOWN_KEY, DiagnosticSeverity::WARNING)),
        );
        assert_eq!(
            classify(ValidationProblemCode::UnknownKey, true),
            Some((code::SCHEMA_UNKNOWN_KEY, DiagnosticSeverity::ERROR)),
        );
    }

    #[test]
    fn value_problems_are_errors_in_either_mode() {
        for code in [
            ValidationProblemCode::TypeMismatch,
            ValidationProblemCode::ConstraintViolation,
            ValidationProblemCode::InvalidFileReference,
        ] {
            assert_eq!(classify(code, false).unwrap().1, DiagnosticSeverity::ERROR);
            assert_eq!(classify(code, true).unwrap().1, DiagnosticSeverity::ERROR);
        }
    }

    // ── Suggestion diagnostic unit tests ──

    #[test]
    fn suggestion_message_covers_each_reason() {
        let problem = |reason| SuggestionLintProblem {
            property: "x".into(),
            property_path: vec!["x".into()],
            root_arm: None,
            property_arm: None,
            decoded: "bad".into(),
            interpreted: serde_json::Value::Null,
            reason,
            span: 0..0,
        };
        assert!(suggestion_message(&problem(SuggestionLintReason::InvalidDecimalSyntax)).contains("simple decimal"));
        assert!(suggestion_message(&problem(SuggestionLintReason::UnsupportedNumberRepresentation)).contains("losslessly"));
        assert!(suggestion_message(&problem(SuggestionLintReason::Range)).contains("range"));
        assert!(suggestion_message(&problem(SuggestionLintReason::Integer)).contains("integer"));
        assert!(suggestion_message(&problem(SuggestionLintReason::Length)).contains("length"));
        assert!(suggestion_message(&problem(SuggestionLintReason::NotEmpty)).contains("empty"));
        assert!(suggestion_message(&problem(SuggestionLintReason::Pattern)).contains("pattern"));
        assert!(suggestion_message(&problem(SuggestionLintReason::Type)).contains("target type"));
    }

    /// UTF-8/UTF-16 LSP range conversion: a multibyte character (é = 2 bytes)
    /// before a candidate must produce different character offsets under UTF-8
    /// vs UTF-16, but the same line.
    #[test]
    fn suggestion_span_converts_under_both_position_encodings() {
        use crate::source_map::{PositionEncoding, SourceMap};
        use lsp_types::Uri;

        // `café` has é at byte 5 (2 bytes). `many` is on the same line, after
        // the multibyte char.
        let text = "---\n$schema:\n  café: number(suggest(1, many, 2))\n---\n\nbody\n";
        let ast = crate::overlay::FrontmatterAst::parse(text)
            .unwrap()
            .ast
            .unwrap();
        let problems = match crate::overlay::suggestions::inline_lints(text, Some(&ast)) {
            crate::overlay::SuggestionState::Inline(p) => p,
            other => panic!("expected Inline, got {other:?}"),
        };
        assert_eq!(problems.len(), 1);
        let span = problems[0].span.clone();
        assert_eq!(&text[span.start..span.end], "many");

        let uri: Uri = "file:///w/test.md".parse().unwrap();
        let text_arc: std::sync::Arc<str> = text.into();

        // UTF-8: character counts bytes. é = 2 bytes, so column is byte offset
        // minus line start.
        let sm_utf8 = SourceMap::new(uri.clone(), 1, PositionEncoding::Utf8, text_arc.clone());
        let range_utf8 = sm_utf8.byte_range_to_lsp(span.clone()).unwrap();
        assert_eq!(range_utf8.start.line, 2);

        // UTF-16: character counts UTF-16 code units. é = 1 UTF-16 unit, so
        // column is lower than the UTF-8 column (which counts 2 bytes).
        let sm_utf16 = SourceMap::new(uri, 1, PositionEncoding::Utf16, text_arc);
        let range_utf16 = sm_utf16.byte_range_to_lsp(span.clone()).unwrap();
        assert_eq!(range_utf16.start.line, 2);
        assert!(
            range_utf8.start.character > range_utf16.start.character,
            "UTF-8 char offset must be > UTF-16 for multibyte prefix: utf8={}, utf16={}",
            range_utf8.start.character,
            range_utf16.start.character
        );
    }

    /// CRLF line endings: byte spans must convert to correct LSP positions.
    #[test]
    fn suggestion_span_converts_with_crlf() {
        use crate::source_map::{PositionEncoding, SourceMap};
        use lsp_types::Uri;

        let text = "---\r\n$schema:\r\n  v: number(min(0); suggest(1, many, 2))\r\n---\r\n\r\nbody\r\n";
        let ast = crate::overlay::FrontmatterAst::parse(text)
            .unwrap()
            .ast
            .unwrap();
        let problems = match crate::overlay::suggestions::inline_lints(text, Some(&ast)) {
            crate::overlay::SuggestionState::Inline(p) => p,
            other => panic!("expected Inline, got {other:?}"),
        };
        assert_eq!(problems.len(), 1);
        let span = problems[0].span.clone();
        assert_eq!(&text[span.start..span.end], "many");

        let uri: Uri = "file:///w/test.md".parse().unwrap();
        let text_arc: std::sync::Arc<str> = text.into();
        let sm = SourceMap::new(uri, 1, PositionEncoding::Utf8, text_arc);
        let range = sm.byte_range_to_lsp(span).unwrap();
        assert_eq!(range.start.line, 2);
    }

    /// An inline `$schema` candidate whose own range crosses a fold projects
    /// through the frontmatter offset to one range from its opening quote to
    /// its closing quote, and converts to a two-line UTF-16 range. Non-ASCII
    /// text on the continuation line separates UTF-16 from byte columns.
    #[test]
    fn a_folded_inline_suggestion_converts_to_one_two_line_range() {
        use crate::source_map::{PositionEncoding, SourceMap};
        use lsp_types::{Position, Range, Uri};

        for newline in ["\n", "\r\n"] {
            let text = format!(
                "---{newline}$schema:{newline}  mode: string(min(40); suggest('first{newline}    sécond')){newline}---{newline}{newline}body{newline}"
            );
            let ast = crate::overlay::FrontmatterAst::parse(&text).unwrap().ast.unwrap();
            let problems = match crate::overlay::suggestions::inline_lints(&text, Some(&ast)) {
                crate::overlay::SuggestionState::Inline(problems) => problems,
                other => panic!("{newline:?}: expected Inline, got {other:?}"),
            };
            assert_eq!(problems.len(), 1, "{newline:?}: {problems:?}");
            let start = text.find("'first").unwrap();
            let end = text.find("sécond'").unwrap() + "sécond'".len();
            assert_eq!(problems[0].span, start..end, "{newline:?}");

            let uri: Uri = "file:///w/test.md".parse().unwrap();
            let source_map = SourceMap::new(uri, 1, PositionEncoding::Utf16, text.as_str().into());
            assert_eq!(
                source_map.byte_range_to_lsp(problems[0].span.clone()),
                Some(Range::new(Position::new(2, 32), Position::new(3, 11))),
                "{newline:?}"
            );
        }
    }

    /// Many aliases of one long malformed scalar whose union-typed properties
    /// accept it, then one alias under a plain `expression` property that
    /// rejects it. The scalar is parsed once, and the error kept from the
    /// accepting aliases still reports exactly once, through the rejecting one.
    #[test]
    #[ignore = "alias analysis is not yet ported onto ScalarProjection; see darkmatter/fixes/2026-09-20-alias-projection-port"]
    fn a_malformed_alias_target_is_parsed_once_and_reported_by_the_rejecting_alias() {
        use super::EXPRESSION_DIAGNOSTIC_PARSES;

        let aliases = 200_usize;
        for terms in [100_usize, 400] {
            for last_type in ["expression", "[expression, string]"] {
                let mut text = String::from("---\n$schema:\n");
                for index in 0..aliases {
                    text.push_str(&format!("  k{index}: [expression, string]\n"));
                }
                text.push_str(&format!("  last: {last_type}\n"));
                let expression = format!("{} +", vec!["known"; terms].join(" || "));
                text.push_str(&format!("known: 1\ndefs:\n  - &shared '{expression}'\n"));
                let definition_line = text.lines().count() as u32 - 1;
                for index in 0..aliases {
                    text.push_str(&format!("k{index}: *shared\n"));
                }
                text.push_str("last: *shared\n---\n\nbody\n");

                let parses = EXPRESSION_DIAGNOSTIC_PARSES.with(std::cell::Cell::get);
                let lines: Vec<u32> = diagnostics_for(&text, |diagnostics| {
                    diagnostics
                        .iter()
                        .filter(|diagnostic| code_of(diagnostic) == Some(code::EXPRESSION_MALFORMED))
                        .map(|diagnostic| diagnostic.range.start.line)
                        .collect()
                });
                let parses = EXPRESSION_DIAGNOSTIC_PARSES.with(std::cell::Cell::get) - parses;

                let expected = if last_type == "expression" { vec![definition_line] } else { Vec::new() };
                assert_eq!(lines, expected, "malformed warnings with `last: {last_type}` ({terms} terms)");
                assert_eq!(parses, 1, "expression parses with `last: {last_type}` ({terms} terms)");
            }
        }
    }

    /// A quote after a content colon (`a:'b`) is plain-scalar content, so it
    /// does not hide the collection's closing delimiter.
    #[test]
    fn a_flow_value_span_completes_past_a_quote_after_a_content_colon() {
        for value in [
            // Controls: genuinely quoted colons, adjacent JSON-like keys, and
            // a colon-bearing plain scalar without a quote.
            "['a:''b', \"v\"]",
            "{\"a\":b, c: 'v'}",
            "[http://x, 'v']",
            // One unmatched internal quote in a sequence, mapping value, or key.
            "[a:'b, \"v\"]",
            "[a:\"b, 'v']",
            "{a: a:'b, b: \"v\"}",
            "{a: a:\"b, b: 'v'}",
            "{a:'b: \"v\"}",
            "{a:\"b: 'v'}",
            "{list: [a:'b, \"v\"]}",
            // Balanced spellings.
            "[a:'b, c:'d, \"v\"]",
        ] {
            for newline in ["\n", "\r\n"] {
                let text = format!("k: {value}{newline}next: [x]{newline}");
                assert_eq!(complete_flow_value_span(&text, 3..4), 3..3 + value.len(), "{text:?}");
            }
        }
    }

    /// Parentheses are plain-scalar content; the collection reader nests
    /// only `[`/`{`, so the span still reaches the collection's close.
    #[test]
    fn a_flow_value_span_completes_past_parentheses() {
        for value in [
            "['a(b', \"v\"]",
            "[a(b, \"v\"]",
            "[a)b, \"v\"]",
            "[a(b, c)d, \"v\"]",
            "{a: a(b, b: \"v\"}",
            "{list: [a(b, \"v\"]}",
        ] {
            for newline in ["\n", "\r\n"] {
                let text = format!("k: {value}{newline}next: [x]{newline}");
                assert_eq!(complete_flow_value_span(&text, 3..4), 3..3 + value.len(), "{text:?}");
            }
        }
    }

    /// The published diagnostic covers the whole authored collection.
    #[test]
    fn a_diagnostic_range_covers_a_collection_holding_a_content_colon() {
        for value in ["[a:'b, 42]", "[a:\"b, 42]", "['a:''b', 42]"] {
            for newline in ["\n", "\r\n"] {
                let text = [
                    "---",
                    "$schema:",
                    "  declaration: schema",
                    &format!("declaration: {value}"),
                    "---",
                    "",
                    "body",
                    "",
                ]
                .join(newline);
                let start = text.find(value).unwrap();
                let source_map = SourceMap::new(
                    "file:///w/doc.md".parse().unwrap(),
                    1,
                    PositionEncoding::Utf16,
                    Arc::from(text.as_str()),
                );
                let expected = source_map.byte_range_to_lsp(start..start + value.len()).unwrap();
                diagnostics_for(&text, |diagnostics| {
                    let ranges: Vec<_> = diagnostics
                        .iter()
                        .filter(|diagnostic| code_of(diagnostic) == Some(code::SCHEMA_INVALID_SHAPE))
                        .map(|diagnostic| diagnostic.range)
                        .collect();
                    assert_eq!(ranges, [expected], "{text:?}: {diagnostics:#?}");
                });
            }
        }
    }

    #[test]
    fn a_flow_value_span_completes_past_a_quote_inside_a_plain_scalar() {
        for value in [
            "[ordinary, \"v\"]",
            "[don't, \"v\"]",
            "{a: say \"hi, b: 'v'}",
            "[\"a]\", 'b'']', [don't]]",
        ] {
            for newline in ["\n", "\r\n"] {
                let text = format!("k: {value}{newline}next: [x]{newline}");
                assert_eq!(complete_flow_value_span(&text, 3..4), 3..3 + value.len(), "{text:?}");
            }
        }
    }
}
