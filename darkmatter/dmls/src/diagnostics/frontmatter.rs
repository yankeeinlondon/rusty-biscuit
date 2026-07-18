//! Frontmatter / schema diagnostics (the R-5 `dm.*` taxonomy).
//!
//! Ranging follows the R-5 convention: the validator identifies the semantic
//! failing node, but every range comes from the concrete syntax tree
//! ([`FrontmatterAst`]), never from message parsing or line maps. Missing-key
//! diagnostics range the parent mapping (a real visible range, not zero-width);
//! unknown-key diagnostics range the offending key; type/constraint/file
//! diagnostics range the value; `relatedInformation` points at the schema
//! origin. Deferred `$(...)` / `{{ … }}` values are never diagnosed (their
//! passivity is explained on hover, not as a per-value squiggle).

use darkmatter::markdown::Markdown;
use darkmatter::markdown::schemas::{
    PositionMap, SchemaError, SchemaOriginKind, SuggestionLintProblem, SuggestionLintReason,
    ValidationOptions, ValidationProblem, ValidationProblemCode, ValidationReport,
    parse_property_definition,
};
use darkmatter::style::{self, StyleWarningKind};
use lsp_types::{
    Diagnostic, DiagnosticRelatedInformation, DiagnosticSeverity, Location, NumberOrString, Range,
};

use crate::diagnostics::codes::{code, source};
use crate::overlay::{FrontmatterAst, SchemaBundle, SchemaOutcome, SuggestionState};
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
                // One effective-schema validation drives both the generic schema
                // problems and the expression pass, which reads it to decide
                // whether a union as a whole rejects a malformed-expression value.
                let report = bundle.effective.validate_with_options(
                    &bundle.frontmatter_json,
                    &PositionMap::new(),
                    &ValidationOptions::default(),
                );
                schema_problem_diagnostics(ctx, ast, bundle, &report, &mut out);
                style_diagnostics(ctx, ast, &mut out);
                expression_diagnostics(ctx, ast, &report, &mut out);
            }
        }
        SchemaOutcome::Ready(None) => {}
    }

    suggestion_diagnostics(ctx, &overlay.suggestions, &mut out);

    if let SuggestionState::TriggerError(error) = &overlay.suggestions {
        out.push(diagnostic(
            ctx.source_map
                .byte_range_to_lsp(0..ctx.text.len())
                .unwrap_or_else(zero_range),
            DiagnosticSeverity::ERROR,
            source::SCHEMA,
            code::SCHEMA_PREPARE,
            error.to_string(),
        ));
    }

    out
}

/// Builds the file-level diagnostic for a failed trigger-registry scan.
pub fn trigger_load_diagnostic(error: &SchemaError) -> Diagnostic {
    diagnostic(
        zero_range(),
        DiagnosticSeverity::ERROR,
        source::SCHEMA,
        code::SCHEMA_PREPARE,
        error.to_string(),
    )
}

/// A schema resolution/preparation failure, ranged at `$schema` (or the block).
fn schema_prepare_diagnostic(
    ctx: &DocumentContext,
    ast: Option<&FrontmatterAst>,
    error: &SchemaError,
    out: &mut Vec<Diagnostic>,
) {
    let fallback_span = ast
        .and_then(|ast| ast.schema_entry().map(|entry| entry.value_span.clone()))
        .or_else(|| ast.map(FrontmatterAst::block_span));
    let (code_value, message, span) = match error {
        SchemaError::Grammar { property, .. } if property != "<root>" => {
            let span = ast
                .and_then(|ast| ast.entry_by_dotted(&format!("$schema.{property}")))
                .map(|entry| entry.value_span.clone())
                .or_else(|| fallback_span.clone());
            (code::SCHEMA_INVALID_TYPE_DEFINITION, error.to_string(), span)
        }
        SchemaError::FrontmatterShape { message } => {
            (code::SCHEMA_INVALID_SHAPE, message.clone(), fallback_span.clone())
        }
        SchemaError::RemoteUnsupported { .. } => {
            (code::SCHEMA_INVALID_SHAPE, error.to_string(), fallback_span.clone())
        }
        SchemaError::Grammar { .. } => {
            (code::SCHEMA_INVALID_SHAPE, error.to_string(), fallback_span.clone())
        }
        other => (code::SCHEMA_PREPARE, other.to_string(), fallback_span.clone()),
    };
    let range = span
        .and_then(|span| ctx.source_map.byte_range_to_lsp(span))
        .unwrap_or_else(zero_range);
    out.push(diagnostic(range, DiagnosticSeverity::ERROR, source::SCHEMA, code_value, message));
}

/// Instance-validation problems mapped onto concrete ranges.
fn schema_problem_diagnostics(
    ctx: &DocumentContext,
    ast: &FrontmatterAst,
    bundle: &SchemaBundle,
    report: &ValidationReport,
    out: &mut Vec<Diagnostic>,
) {
    // Expression-typed scalar values are owned by the expression pass:
    // `dm.expression.malformed` replaces the generic format-constraint problem,
    // and a native boolean/number is coerced at compose time, so its type
    // mismatch is not a real editor error. Both are suppressed here; every
    // unrelated problem (including a genuine mapping/sequence type mismatch,
    // which is not a scalar and never enters this set) is retained.
    let expression_values =
        crate::providers::frontmatter::expression_values(ctx, ast);
    let expression_pointers: std::collections::HashSet<&str> = expression_values
        .iter()
        .map(|value| value.entry.pointer.as_str())
        .collect();

    let mut specialized_paths = std::collections::HashSet::new();
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
        out.push(specialized);
    }

    for problem in &report.problems {
        if specialized_paths.contains(&problem.path) {
            continue;
        }
        let Some((code_value, severity)) = classify(problem.code, ctx.config.schema.strict) else {
            continue;
        };
        if matches!(
            problem.code,
            ValidationProblemCode::TypeMismatch | ValidationProblemCode::ConstraintViolation
        ) && expression_pointers.contains(problem.path.as_str())
        {
            continue;
        }
        let Some(range) = problem_range(ast, ctx.source_map, problem) else {
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
        out.push(diagnostic);
    }

    // Deferred-composition values (`$(...)` / `{{ … }}`) are intentionally NOT
    // diagnosed: an informational squiggle on every interpolated property is
    // pure noise. The passive "never executed" guarantee is surfaced on hover
    // (schema description for the key; policy verdict for `$()` shell), not as a
    // per-value diagnostic. `report.pending` stays populated but unconsumed.
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
        let raw = ctx.text.get(entry.value_span.clone())?;
        let yaml: serde_yaml_ng::Value = serde_yaml_ng::from_str(raw).ok()?;
        match parse_property_definition(&entry.key, &yaml) {
            Err(SchemaError::Grammar { property, .. }) if property != entry.key => ast
                .entry_by_dotted(&format!("{}.{}", entry.dotted, property))
                .map(|nested| nested.value_span.clone())
                .unwrap_or_else(|| entry.value_span.clone()),
            _ => entry.value_span.clone(),
        }
    } else {
        entry.value_span.clone()
    };
    ctx.source_map.byte_range_to_lsp(span)
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

/// Expression-typed frontmatter value diagnostics: `dm.expression.malformed`
/// for a value the expression grammar rejects, and
/// `dm.expression.unknown_identifier` for a bare root that names nothing DMLS
/// can resolve. Both carry source `darkmatter.frontmatter`; ranges are projected
/// through the shared YAML-scalar mapper so YAML quotes are excluded.
///
/// `expression_values` includes any property with *any* Expression union arm, so
/// a value the Expression arm rejects but a sibling Enum/String arm accepts must
/// not be flagged malformed. The effective-schema `report` is the arbiter: a
/// value whose pointer carries a real validation problem is rejected by every
/// arm (the union fails), so the dedicated `malformed` diagnostic is emitted;
/// otherwise an alternate arm accepts it and the diagnostic is suppressed.
fn expression_diagnostics(
    ctx: &DocumentContext,
    ast: &FrontmatterAst,
    report: &ValidationReport,
    out: &mut Vec<Diagnostic>,
) {
    let union_rejected: std::collections::HashSet<&str> =
        report.problems.iter().map(|problem| problem.path.as_str()).collect();

    for value in crate::providers::frontmatter::expression_values(ctx, ast) {
        let expression = value.expression();
        match crate::overlay::expressions::parse_condition(expression) {
            Err(error) => {
                if !union_rejected.contains(value.entry.pointer.as_str()) {
                    // A non-Expression arm validly accepts this value, so the
                    // union validates — a malformed-expression warning would be a
                    // false positive.
                    continue;
                }
                let at = error.position.min(expression.len());
                let span = value
                    .project(at..expression.len())
                    .filter(|span| span.start < span.end)
                    .unwrap_or_else(|| value.expression_span());
                if let Some(range) = ctx.source_map.byte_range_to_lsp(span) {
                    out.push(diagnostic(
                        range,
                        DiagnosticSeverity::WARNING,
                        source::FRONTMATTER,
                        code::EXPRESSION_MALFORMED,
                        format!("malformed expression: {}", error.message),
                    ));
                }
            }
            Ok(parsed) => {
                let Some(name) = crate::overlay::expressions::root_identifier(&parsed) else {
                    continue;
                };
                if !expression_root_is_unknown(ctx, ast, &name) {
                    continue;
                }
                if let Some(range) = ctx.source_map.byte_range_to_lsp(value.expression_span()) {
                    out.push(diagnostic(
                        range,
                        DiagnosticSeverity::INFORMATION,
                        source::FRONTMATTER,
                        code::EXPRESSION_UNKNOWN_IDENTIFIER,
                        format!(
                            "`{name}` matches no frontmatter key, schema property, `ctx.*`, `env.*`, or function"
                        ),
                    ));
                }
            }
        }
    }
}

/// Whether a bare expression root resolves to nothing DMLS can name, reusing the
/// same authority as the body-interpolation diagnostic.
fn expression_root_is_unknown(ctx: &DocumentContext, ast: &FrontmatterAst, name: &str) -> bool {
    crate::overlay::expressions::is_unknown_root(
        name,
        |name| ast.entry_by_dotted(name).is_some(),
        |name| {
            crate::providers::frontmatter::known_shape(ctx)
                .properties
                .contains_key(name)
        },
    )
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
        SuggestionState::Inactive | SuggestionState::TriggerError(_) => {}
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

/// A malformed recognized standalone envelope, ranged at the whole buffer.
fn standalone_error_diagnostic(ctx: &DocumentContext, error: &SchemaError) -> Diagnostic {
    if let Some(diagnostic) = standalone_type_definition_diagnostic(ctx) {
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

fn standalone_type_definition_diagnostic(ctx: &DocumentContext) -> Option<Diagnostic> {
    let yaml: serde_yaml_ng::Value = serde_yaml_ng::from_str(ctx.text).ok()?;
    let mapping = yaml.as_mapping()?;
    let tagged = mapping
        .get(serde_yaml_ng::Value::String("kind".to_string()))
        .and_then(serde_yaml_ng::Value::as_str)
        == Some("schema");
    let payload_key = if tagged { "types" } else { "$schema" };
    let payload = mapping.get(serde_yaml_ng::Value::String(payload_key.to_string()))?;
    let payload = payload.as_mapping()?;
    let parsed = FrontmatterAst::parse_yaml(ctx.text);
    let ast = parsed.ast?;

    for (key, value) in payload {
        let key = key.as_str()?;
        let Err(error) = parse_property_definition(key, value) else {
            continue;
        };
        let entry = ast.entry_by_dotted(&format!("{payload_key}.{key}"))?;
        let span = match &error {
            SchemaError::Grammar { property, .. } if property != key => ast
                .entry_by_dotted(&format!("{payload_key}.{key}.{property}"))
                .map(|nested| nested.value_span.clone())
                .unwrap_or_else(|| entry.value_span.clone()),
            _ => entry.value_span.clone(),
        };
        let range = ctx.source_map.byte_range_to_lsp(span)?;
        return Some(diagnostic(
            range,
            DiagnosticSeverity::ERROR,
            source::SCHEMA,
            code::SCHEMA_INVALID_TYPE_DEFINITION,
            error.to_string(),
        ));
    }
    None
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

/// A zero-width range at the document start (fallback when a span cannot
/// convert).
fn zero_range() -> Range {
    Range::default()
}

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
        let path = Path::new("/w/doc.md");
        let uri: Uri = "file:///w/doc.md".parse().unwrap();
        let config = DmlsConfig::default();
        let roots = [PathBuf::from("/w")];
        let state = OverlayState::default();
        let overlay = state.for_document(&uri, text, path, &config, &roots);
        let source_map = SourceMap::new(uri.clone(), 1, PositionEncoding::Utf16, Arc::from(text));
        let graph = WorkspaceGraph::build(&BTreeMap::new(), 1);
        let profile = ClientProfile::from_initialize(&InitializeParams::default(), PositionEncoding::Utf16);
        let ctx = DocumentContext {
            uri: &uri,
            path,
            text,
            source_map: &source_map,
            graph: &graph,
            doc_id: None,
            config: &config,
            profile: &profile,
            overlay: overlay.as_ref(),
        };
        f(&diagnostics(&ctx))
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
    fn unknown_expression_root_is_informational_frontmatter_diagnostic() {
        let text = expression_doc("when: mystery");
        diagnostics_for(&text, |diagnostics| {
            let unknown: Vec<&Diagnostic> = diagnostics
                .iter()
                .filter(|diagnostic| code_of(diagnostic) == Some(code::EXPRESSION_UNKNOWN_IDENTIFIER))
                .collect();
            assert_eq!(unknown.len(), 1, "{diagnostics:#?}");
            assert_eq!(unknown[0].source.as_deref(), Some(source::FRONTMATTER));
            assert_eq!(unknown[0].severity, Some(DiagnosticSeverity::INFORMATION));
        });
    }

    #[test]
    fn native_scalar_coercion_produces_no_expression_false_diagnostics() {
        // A native boolean is coerced to a canonical expression string at compose
        // time, so `when: true` is valid — no type mismatch, no malformed, no
        // unknown-identifier squiggle.
        let text = expression_doc("when: true");
        diagnostics_for(&text, |diagnostics| {
            assert!(
                diagnostics.iter().all(|diagnostic| {
                    let code = code_of(diagnostic);
                    code != Some(code::SCHEMA_TYPE_MISMATCH)
                        && code != Some(code::EXPRESSION_MALFORMED)
                        && code != Some(code::EXPRESSION_UNKNOWN_IDENTIFIER)
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
                        && code != Some(code::EXPRESSION_UNKNOWN_IDENTIFIER)
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
                "string(nope)",
            ),
            (
                "---\n$schema:\n  declaration: schema\ndeclaration: https://example.com/schema.yaml\n---\n\nbody\n",
                code::SCHEMA_INVALID_SHAPE,
                "https://example.com/schema.yaml",
            ),
        ];

        for (text, expected_code, authored_value) in cases {
            diagnostics_for(text, |diagnostics| {
                let specialized: Vec<&Diagnostic> = diagnostics
                    .iter()
                    .filter(|diagnostic| code_of(diagnostic) == Some(expected_code))
                    .collect();
                assert_eq!(specialized.len(), 1, "one specialized diagnostic: {diagnostics:#?}");
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
                assert_eq!(specialized[0].range, expected, "smallest authored value range");
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
                            && code != Some(code::EXPRESSION_UNKNOWN_IDENTIFIER)
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
}
