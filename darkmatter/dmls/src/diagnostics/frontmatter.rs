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
    ValidationOptions, ValidationProblem, ValidationProblemCode,
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
                schema_problem_diagnostics(ctx, ast, bundle, &mut out);
                style_diagnostics(ctx, ast, &mut out);
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
        let Some((code_value, severity)) = classify(problem.code, ctx.config.schema.strict) else {
            continue;
        };
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
