//! `dm.expression.nested_span_in_literal`: a `{{ … }}` span authored inside a
//! quoted string literal on a surface Claudine evaluates exactly once, where
//! it survives as literal text instead of interpolating.
//!
//! Darkmatter's shared lint decides what a nested span is; this module decides
//! only *where* the lint is authoritative. Design:
//! `claudine/fixes/2026-09-13-better-static-analysis/spec.md` (D3, D7).
//!
//! ## Lifecycle-key inventory
//!
//! [`lifecycle`] is a documented stopgap: a static copy of the Claudine
//! lifecycle layout, kept in step with `darkmatter/docs/schemas/claudine.yaml`
//! and `claudine-types.yaml` by a parity test rather than by a crate dependency. Schema triggers that
//! scope variables are the intended replacement; until then this is the only
//! place DMLS knows Claudine's lifecycle keys.

use darkmatter::markdown::compose::expression::ParseMode;
use lsp_types::{Diagnostic, DiagnosticSeverity, NumberOrString, Range};
use serde::{Deserialize, Serialize};

use crate::diagnostics::codes::{code, source};
use crate::overlay::FrontmatterAst;
use crate::overlay::expressions::{self, ProjectedNestedSpan};
use crate::providers::DocumentContext;
use crate::providers::frontmatter::ExpressionValue;

/// The static Claudine lifecycle layout DMLS mirrors (see the module docs).
pub(crate) mod lifecycle {
    use crate::overlay::FmPathSegment::{self, Index, Key};

    /// The seven top-level frontmatter keys holding lifecycle events, in
    /// Claudine's `LifecycleSignal::ALL` order.
    pub(crate) const EVENT_KEYS: [&str; 7] =
        ["initialize", "start", "success", "blocked", "failure", "finalize", "loop"];

    /// An event's communication fields: string surfaces whose whole-value
    /// `{{ … }}` is evaluated once when the event fires. `effect` names a
    /// sound and `stack` is structured, so neither is a communication field.
    pub(crate) const COMMUNICATION_FIELDS: [&str; 9] =
        ["say", "say_first", "message", "stderr", "notify", "info", "warn", "success", "stdout"];

    /// Globals Claudine binds only while a lifecycle event fires.
    pub(crate) const LATE_BINDING_ROOTS: [&str; 3] = ["err", "timing", "current"];

    fn is_event(segment: Option<&FmPathSegment<'_>>) -> bool {
        matches!(segment, Some(Key(key)) if EVENT_KEYS.contains(key))
    }

    /// Whether `path` lies strictly beneath a lifecycle event key, where the
    /// late-binding roots are in scope.
    pub(crate) fn is_beneath_event(path: &[FmPathSegment<'_>]) -> bool {
        path.len() > 1 && is_event(path.first())
    }

    /// Whether a whole-value scalar at `path` is evaluated exactly once: an
    /// event communication field, or any stack item value (action operands
    /// and `proxy … with` values included) other than its predicate and its
    /// `no_error` flag.
    pub(crate) fn is_single_pass_value(path: &[FmPathSegment<'_>]) -> bool {
        match path {
            [event, Key(field)] => is_event(Some(event)) && COMMUNICATION_FIELDS.contains(field),
            [event, Key("stack"), Index(_), rest @ ..] => {
                is_event(Some(event))
                    && !rest.is_empty()
                    && !matches!(rest, [Key("when" | "no_error")])
            }
            _ => false,
        }
    }

    /// Whether `path` is a lifecycle predicate: a stack item's `when`, or the
    /// loop's `while` / `until`.
    pub(crate) fn is_predicate(path: &[FmPathSegment<'_>]) -> bool {
        match path {
            [event, Key("stack"), Index(_), Key("when")] => is_event(Some(event)),
            [Key("loop"), Key("while" | "until")] => true,
            _ => false,
        }
    }
}

/// Discriminator for [`NestedSpanRewrite`] payloads.
pub(crate) const REWRITE_ACTION: &str = "dm.expression.rewrite_concatenation";
/// Payload layout version; a payload of any other version is declined.
pub(crate) const REWRITE_VERSION: u32 = 1;

/// The typed `Diagnostic.data` of a nested-span diagnostic that has a rewrite.
///
/// The code action reads only this payload — never the message — and declines
/// it unless `document_version` matches the snapshot it is asked to edit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct NestedSpanRewrite {
    pub(crate) action: String,
    pub(crate) version: u32,
    /// The client document version the diagnostic was computed against.
    pub(crate) document_version: i32,
    /// The authored expression range the rewrite replaces.
    pub(crate) range: Range,
    /// The `+` rewrite, already encoded for the scalar's YAML style.
    pub(crate) new_text: String,
}

impl NestedSpanRewrite {
    /// Decodes a payload, rejecting any other action or layout version.
    pub(crate) fn from_data(data: &serde_json::Value) -> Option<Self> {
        serde_json::from_value::<Self>(data.clone())
            .ok()
            .filter(|payload| payload.action == REWRITE_ACTION && payload.version == REWRITE_VERSION)
    }
}

/// Nested-span diagnostics for whole-value scalars on single-pass lifecycle
/// surfaces. Body spans, mixed strings, and other frontmatter keys are never
/// checked: those surfaces rescan, or may.
pub(crate) fn whole_value_diagnostics(
    ctx: &DocumentContext,
    ast: &FrontmatterAst,
    out: &mut Vec<Diagnostic>,
) {
    for interpolation in expressions::frontmatter_interpolations(ctx.text, ast) {
        if !interpolation.whole_value
            || !lifecycle::is_single_pass_value(&ast.path_at(interpolation.index))
        {
            continue;
        }
        let lints = interpolation.nested_span_lints(ctx.text, ParseMode::Interpolation);
        push_first(ctx, lints, out);
    }
}

/// The nested-span diagnostic for an expression-typed lifecycle predicate,
/// which is condition text rather than a `{{ … }}` span.
pub(crate) fn predicate_diagnostic(
    ctx: &DocumentContext,
    value: &ExpressionValue,
    out: &mut Vec<Diagnostic>,
) {
    let lints =
        value.projection().nested_span_lints(ctx.text, value.expression(), 0, ParseMode::Condition);
    push_first(ctx, lints, out);
}

/// One diagnostic per expression, at its first nested span. Every lint of one
/// expression carries the same whole-expression rewrite, so one quick fix
/// repairs them all.
fn push_first(ctx: &DocumentContext, lints: Vec<ProjectedNestedSpan>, out: &mut Vec<Diagnostic>) {
    let Some(lint) = lints.into_iter().next() else {
        return;
    };
    let Some(range) = ctx.source_map.byte_range_to_lsp(lint.range) else {
        return;
    };
    let data = lint.replacement.and_then(|(span, new_text)| {
        let payload = NestedSpanRewrite {
            action: REWRITE_ACTION.to_string(),
            version: REWRITE_VERSION,
            document_version: ctx.source_map.version(),
            range: ctx.source_map.byte_range_to_lsp(span)?,
            new_text,
        };
        serde_json::to_value(payload).ok()
    });
    out.push(Diagnostic {
        range,
        severity: Some(DiagnosticSeverity::ERROR),
        code: Some(NumberOrString::String(code::EXPRESSION_NESTED_SPAN_IN_LITERAL.to_string())),
        source: Some(source::FRONTMATTER.to_string()),
        message: format!(
            "`{{{{{}}}}}` inside a quoted string is literal text and is never interpolated here; concatenate with `+` instead",
            lint.nested
        ),
        data,
        ..Default::default()
    });
}

#[cfg(test)]
mod nested_span_tests;
