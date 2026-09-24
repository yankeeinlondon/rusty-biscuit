//! Layer-3 interpolation overlay: request-time analysis of `{{ … }}`
//! expressions.
//!
//! Interpolation regions come from the library [`ExpressionFinder`] (which skips
//! fenced code exactly as compose does); the inner expression is parsed with the
//! Phase-8 [`parse_spanned`] so the language server sees the same AST — and the
//! same byte-offset [`ParseError`] — that `md compose` does. Nothing here reads
//! `env`, captures `ctx`, or executes anything: static value resolution is
//! frontmatter-backed only.

use darkmatter::markdown::compose::context::{
    ContextVariableDescriptor, context_variable_descriptors,
};
use darkmatter::markdown::compose::expression::{
    BinaryOp, ExpressionFinder, ExpressionFunctionDescriptor, ExpressionLintKind, ParseError,
    ParseMode, SpannedExpr, SpannedExprKind, expression_function_descriptors,
    identifier_prefix_start, is_statically_known_root, is_whole_value_span, lint_expression,
    parse_condition_spanned, parse_spanned, static_variable_reads,
};
use darkmatter::markdown::schemas::{DecodedScalar, decode_scalar};
use darkmatter::markdown::span::SourceSpan;
use serde::{Deserialize, Serialize};

use crate::overlay::{FmEntry, FmScalarStyle, FmValueKind, FrontmatterAst};

/// One `{{{ … }}}` interpolation literal with a document-relative span.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Literal {
    /// Byte span of the whole `{{{ … }}}` construct (braces included).
    pub outer: SourceSpan,
    /// The literal content between `{{{` and `}}}`, preserved verbatim.
    pub content: String,
}

/// All body interpolation literals at or after `body_base` (frontmatter literals
/// are not body literals), in document order.
pub fn literals(text: &str, body_base: usize) -> Vec<Literal> {
    ExpressionFinder::new(text)
        .scan()
        .literals
        .into_iter()
        .filter(|literal| literal.start >= body_base)
        .map(|literal| Literal {
            outer: literal.start..literal.end,
            content: literal.content,
        })
        .collect()
}

/// The literal whose span contains `offset`, if any.
pub fn literal_at(text: &str, body_base: usize, offset: usize) -> Option<Literal> {
    literals(text, body_base)
        .into_iter()
        .find(|literal| literal.outer.start <= offset && offset <= literal.outer.end)
}

/// One `{{ … }}` interpolation with document-relative spans.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Interpolation {
    /// Byte span of the whole `{{ … }}` construct (braces included).
    pub outer: SourceSpan,
    /// Byte span of the trimmed inner expression text.
    pub inner: SourceSpan,
    /// The trimmed inner expression text.
    pub text: String,
}

/// All body interpolations at or after `body_base` (frontmatter interpolation is
/// the frontmatter provider's concern), in document order.
pub fn interpolations(text: &str, body_base: usize) -> Vec<Interpolation> {
    ExpressionFinder::new(text)
        .find_all()
        .into_iter()
        .filter(|location| location.start >= body_base)
        .map(|location| {
            let inner_start = location.start + 2;
            let inner_slice = &text[inner_start..location.end - 2];
            let leading = inner_slice.len() - inner_slice.trim_start().len();
            let trimmed_start = inner_start + leading;
            Interpolation {
                outer: location.start..location.end,
                inner: trimmed_start..trimmed_start + location.expression.len(),
                text: location.expression,
            }
        })
        .collect()
}

/// The interpolation whose inner expression span contains `offset`, if any.
pub fn interpolation_at(text: &str, body_base: usize, offset: usize) -> Option<Interpolation> {
    interpolations(text, body_base)
        .into_iter()
        .find(|interpolation| interpolation.outer.start <= offset && offset <= interpolation.outer.end)
}

// ── Frontmatter scalar projection ──────────────────────────────────────────

/// How offsets in one frontmatter scalar's analyzed text map back to authored
/// document bytes.
///
/// There is no single correct rule for every YAML scalar: flow quotes and
/// escapes must be decoded before parsing, literal blocks keep raw line breaks
/// behind a header and indentation, and folding or a tag changes the text the
/// evaluator sees. Only styles with an exact map are ever ranged precisely or
/// edited; everything else widens to the whole scalar and offers no edit.
/// Matching a decoded span to a separately scanned raw span by ordinal is never
/// used: repeated spans, escapes, and folding make "the nth span" too weak.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScalarProjection {
    /// The authored value node span — the fallback range.
    scalar: SourceSpan,
    mode: ProjectionMode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ProjectionMode {
    /// Analyzed text is the authored slice starting at `base`: an untagged
    /// single-line plain scalar.
    Plain { base: usize },
    /// Analyzed text is the authored slice starting at `base`, header and
    /// indentation included: an untagged literal block. Only individual
    /// `{{ … }}` spans are analyzed in these coordinates; YAML indentation is
    /// ordinary expression whitespace.
    Literal { base: usize },
    /// Analyzed text is the decoded value of an untagged single-line quoted
    /// scalar, mapped through `map` (raw offsets relative to `base`).
    Quoted { base: usize, double: bool, map: DecodedScalar },
    /// No exact map: folded, tagged, multi-line flow, or a decode mismatch.
    Whole,
}

impl ScalarProjection {
    /// The projection and analyzed text for a whole scalar value, such as an
    /// expression-typed schema value.
    ///
    /// Exact only for untagged, single-line plain, single-quoted, and
    /// double-quoted scalars whose decoded text equals the parser's value; a
    /// literal block is not exact here because its header precedes the value.
    /// Returns `None` for a non-scalar entry.
    pub fn for_value(entry: &FmEntry, document: &str) -> Option<(Self, String)> {
        let scalar = entry.scalar.as_ref().filter(|_| entry.kind == FmValueKind::Scalar)?;
        let whole = || {
            (Self { scalar: entry.value_span.clone(), mode: ProjectionMode::Whole }, scalar.clone())
        };
        let Some(raw) = document.get(entry.value_span.clone()) else {
            return Some(whole());
        };
        if entry.tagged || raw.contains(['\n', '\r']) {
            return Some(whole());
        }
        let base = entry.value_span.start;
        let exact = match entry.scalar_style? {
            FmScalarStyle::Plain if raw == scalar => ProjectionMode::Plain { base },
            style @ (FmScalarStyle::SingleQuoted | FmScalarStyle::DoubleQuoted) => {
                match decode_scalar(raw).filter(|map| map.decoded() == scalar) {
                    Some(map) => ProjectionMode::Quoted {
                        base,
                        double: style == FmScalarStyle::DoubleQuoted,
                        map,
                    },
                    None => return Some(whole()),
                }
            }
            _ => return Some(whole()),
        };
        Some((Self { scalar: entry.value_span.clone(), mode: exact }, scalar.clone()))
    }

    /// Whether ranges and edits in this scalar are exact.
    pub fn is_exact(&self) -> bool {
        self.mode != ProjectionMode::Whole
    }

    /// The authored value node span.
    pub fn scalar_span(&self) -> SourceSpan {
        self.scalar.clone()
    }

    /// Projects an analyzed-text range to authored document bytes, or `None`
    /// without an exact map.
    pub fn project(&self, range: std::ops::Range<usize>) -> Option<SourceSpan> {
        match &self.mode {
            ProjectionMode::Plain { base } | ProjectionMode::Literal { base } => {
                Some(base + range.start..base + range.end)
            }
            ProjectionMode::Quoted { base, map, .. } => {
                map.project(range).map(|raw| base + raw.start..base + raw.end)
            }
            ProjectionMode::Whole => None,
        }
    }

    /// The exact authored range, else the whole scalar.
    pub fn range(&self, range: std::ops::Range<usize>) -> SourceSpan {
        self.project(range).unwrap_or_else(|| self.scalar.clone())
    }

    /// The analyzed-text offset for a document offset inside the scalar, or
    /// `None` without an exact map.
    pub fn analyzed_offset(&self, document_offset: usize) -> Option<usize> {
        match &self.mode {
            ProjectionMode::Plain { base } | ProjectionMode::Literal { base } => {
                Some(document_offset.saturating_sub(*base))
            }
            ProjectionMode::Quoted { base, map, .. } => {
                Some(map.decoded_offset(document_offset.saturating_sub(*base)))
            }
            ProjectionMode::Whole => None,
        }
    }

    /// Encodes analyzed text as an authored fragment of this scalar's style, so
    /// it can replace a projected range without changing how the rest of the
    /// scalar decodes. `None` when no safe encoding exists.
    pub fn encode_fragment(&self, text: &str) -> Option<String> {
        match &self.mode {
            ProjectionMode::Literal { .. } => Some(text.to_string()),
            ProjectionMode::Plain { .. } => {
                let unsafe_plain = text.contains(['\n', '\r', '\t', ',', '[', ']', '{', '}'])
                    || text.contains(": ")
                    || text.contains(" #")
                    || text.ends_with(':');
                (!unsafe_plain).then(|| text.to_string())
            }
            ProjectionMode::Quoted { double: false, .. } => {
                (!text.contains(['\n', '\r'])).then(|| text.replace('\'', "''"))
            }
            ProjectionMode::Quoted { double: true, .. } => {
                let mut encoded = String::with_capacity(text.len());
                for character in text.chars() {
                    match character {
                        '\\' => encoded.push_str("\\\\"),
                        '"' => encoded.push_str("\\\""),
                        '\n' => encoded.push_str("\\n"),
                        '\t' => encoded.push_str("\\t"),
                        other if other.is_control() => return None,
                        other => encoded.push(other),
                    }
                }
                Some(encoded)
            }
            ProjectionMode::Whole => None,
        }
    }

    /// Nested `{{ … }}` spans inside `text`'s quoted string literals, through
    /// Darkmatter's shared lint, projected to authored bytes. `text` is the
    /// analyzed expression starting at analyzed offset `offset`; the rewrite,
    /// when offered, replaces exactly that expression's authored range.
    pub fn nested_span_lints(
        &self,
        document: &str,
        text: &str,
        offset: usize,
        mode: ParseMode,
    ) -> Vec<ProjectedNestedSpan> {
        let project =
            |range: std::ops::Range<usize>| self.project(offset + range.start..offset + range.end);
        lint_expression(text, mode)
            .into_iter()
            .map(|lint| {
                let ExpressionLintKind::NestedSpanInStringLiteral { literal, nested } = lint.kind;
                let literal_is_single_line = project(literal)
                    .and_then(|span| document.get(span))
                    .is_some_and(|authored| !authored.contains(['\n', '\r']));
                let replacement = lint
                    .suggestion
                    .filter(|_| literal_is_single_line)
                    .and_then(|suggestion| {
                        Some((project(0..text.len())?, self.encode_fragment(&suggestion)?))
                    });
                ProjectedNestedSpan {
                    range: project(lint.span).unwrap_or_else(|| self.scalar.clone()),
                    nested,
                    replacement,
                }
            })
            .collect()
    }
}

/// One `{{ … }}` span inside a string-valued frontmatter scalar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrontmatterInterpolation<'a> {
    /// The scalar entry holding the span.
    pub entry: &'a FmEntry,
    /// The entry's arena index, for O(depth) [`FrontmatterAst::path_at`].
    pub index: usize,
    /// The trimmed expression text the analysis parses.
    pub text: String,
    /// Whether the span is the scalar's entire (trimmed) decoded value — the
    /// syntax shape an owner may evaluate as a whole value. Whether that owner
    /// is single-pass is the caller's decision.
    pub whole_value: bool,
    /// Outer `{{ … }}` range in analyzed-text coordinates.
    outer: std::ops::Range<usize>,
    /// Trimmed inner expression range in analyzed-text coordinates.
    inner: std::ops::Range<usize>,
    projection: ScalarProjection,
}

/// A nested-span lint projected into authored document coordinates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectedNestedSpan {
    /// The nested `{{ … }}` range, or the whole scalar without an exact map.
    pub range: SourceSpan,
    /// The nested span's trimmed expression text.
    pub nested: String,
    /// The whole-expression rewrite as a YAML-style-encoded replacement for the
    /// linted expression's authored range (for an interpolation, its
    /// [`FrontmatterInterpolation::inner_span`]). `None` when the lint has no
    /// suggestion, the scalar has no exact map, the flagged literal's authored
    /// slice spans a line break, or no safe encoding exists.
    pub replacement: Option<(SourceSpan, String)>,
}

impl FrontmatterInterpolation<'_> {
    /// The projection this span's ranges go through.
    pub fn projection(&self) -> &ScalarProjection {
        &self.projection
    }

    /// The authored `{{ … }}` range, else the whole scalar.
    pub fn outer_span(&self) -> SourceSpan {
        self.projection.range(self.outer.clone())
    }

    /// The exact authored range of the trimmed inner expression.
    pub fn inner_span(&self) -> Option<SourceSpan> {
        self.projection.project(self.inner.clone())
    }

    /// Projects a range of [`text`](Self::text) to authored bytes, or `None`
    /// without an exact map.
    pub fn project(&self, range: std::ops::Range<usize>) -> Option<SourceSpan> {
        self.projection
            .project(self.inner.start + range.start..self.inner.start + range.end)
    }

    /// Nested `{{ … }}` spans inside this expression's quoted string literals,
    /// through Darkmatter's shared lint, projected to authored bytes.
    pub fn nested_span_lints(&self, document: &str, mode: ParseMode) -> Vec<ProjectedNestedSpan> {
        self.projection.nested_span_lints(document, &self.text, self.inner.start, mode)
    }
}

/// Every `{{ … }}` span inside a string-valued frontmatter scalar, at any depth
/// (sequence items included), in document order.
///
/// Passive: scanning and projection only, with no composition, shell, or I/O.
/// Aliases are not scalar entries and contribute nothing at the alias site.
pub fn frontmatter_interpolations<'a>(
    document: &str,
    ast: &'a FrontmatterAst,
) -> Vec<FrontmatterInterpolation<'a>> {
    let mut out = Vec::new();
    for (index, entry) in ast.entries().iter().enumerate() {
        let Some(scalar) = entry.scalar.as_deref().filter(|_| entry.kind == FmValueKind::Scalar)
        else {
            continue;
        };
        if !scalar.contains("{{") {
            continue;
        }
        let whole_value = is_whole_value_span(scalar);
        let raw = document.get(entry.value_span.clone());
        let (projection, analyzed, scan_from) = match (entry.scalar_style, entry.tagged, raw) {
            (Some(FmScalarStyle::Literal), false, Some(raw)) => {
                // Skip the block header line, which may carry a comment.
                let body = raw.find('\n').map_or(raw.len(), |newline| newline + 1);
                let projection = ScalarProjection {
                    scalar: entry.value_span.clone(),
                    mode: ProjectionMode::Literal { base: entry.value_span.start },
                };
                (projection, raw.to_string(), body)
            }
            _ => {
                let Some((projection, analyzed)) = ScalarProjection::for_value(entry, document)
                else {
                    continue;
                };
                (projection, analyzed, 0)
            }
        };
        for location in ExpressionFinder::find_all_plain(&analyzed[scan_from..]) {
            let start = scan_from + location.start;
            let end = scan_from + location.end;
            let inner_slice = &analyzed[start + 2..end - 2];
            let leading = inner_slice.len() - inner_slice.trim_start().len();
            let inner_start = start + 2 + leading;
            out.push(FrontmatterInterpolation {
                entry,
                index,
                inner: inner_start..inner_start + location.expression.len(),
                text: location.expression,
                whole_value,
                outer: start..end,
                projection: projection.clone(),
            });
        }
    }
    out
}

/// Parses a **value-dialect** expression, span-carrying. This is the body
/// `{{ }}` interpolation parser: it accepts `&&` (logical AND, identical to the
/// condition dialect) and `||` (as fallback / first-truthy, whereas the
/// condition dialect reads `||` as logical OR) — matching `md compose`'s
/// interpolation grammar.
///
/// ## Errors
///
/// Returns the [`ParseError`] (with a byte-offset `position` into the
/// expression text) exactly as the compose parser would.
pub fn parse(expression: &str) -> Result<SpannedExpr, ParseError> {
    parse_spanned(expression)
}

/// Parses a **condition-dialect** expression, span-carrying. This is the parse
/// authority for an Expression-typed frontmatter value: it accepts `&&`/`||`
/// (lowered to synthetic `and`/`or` calls), matching the `expression` schema
/// format's `parse_condition` validation, so a value the schema accepts is never
/// diagnosed as malformed here. Body `{{ }}` interpolation stays on [`parse`].
///
/// ## Errors
///
/// Returns the [`ParseError`] (with a byte-offset `position` into the
/// expression text) exactly as the compose condition parser would.
pub fn parse_condition(expression: &str) -> Result<SpannedExpr, ParseError> {
    parse_condition_spanned(expression)
}

/// The leading identifier of an expression — a bare `Variable` name, or the
/// root of a member-access / index / function chain. Used to classify an
/// expression and to navigate a bare variable to its frontmatter key.
pub fn root_identifier(expr: &SpannedExpr) -> Option<String> {
    match &expr.kind {
        SpannedExprKind::Variable(name) => Some(name.clone()),
        SpannedExprKind::MemberAccess { base, .. } | SpannedExprKind::Index { base, .. } => {
            root_identifier(base)
        }
        _ => None,
    }
}

/// The name of the deepest [`SpannedExprKind::FunctionCall`] whose
/// function-name identifier contains `offset` (a byte offset into `source`),
/// if any.
///
/// Only a cursor on the function-name identifier itself resolves; an offset on
/// an argument, parenthesis, or comma does not, so the caller's ctx/frontmatter
/// hover can take over for the inner expression. Nested calls like
/// `as_csv(length(x))` answer for the inner call only when the cursor sits on
/// the inner name.
///
/// `source` is the expression text the spans index into; the
/// `source.get(name_span) == name` check filters out synthetic `and`/`or`
/// calls lowered from `&&`/`||` operators, whose call span begins at the left
/// operand rather than at a name token.
pub fn function_call_at<'a>(
    expr: &'a SpannedExpr,
    source: &str,
    offset: usize,
) -> Option<&'a str> {
    if offset < expr.span.start || offset > expr.span.end {
        return None;
    }
    let children: Vec<&SpannedExpr> = match &expr.kind {
        SpannedExprKind::UnaryNot(inner)
        | SpannedExprKind::UnaryMinus(inner)
        | SpannedExprKind::Paren(inner) => vec![inner],
        SpannedExprKind::Binary { left, right, .. }
        | SpannedExprKind::Comparison { left, right, .. } => vec![left, right],
        SpannedExprKind::Index { base, index } => vec![base, index],
        SpannedExprKind::MemberAccess { base, .. } => vec![base],
        SpannedExprKind::Fallback { primary, fallback } => vec![primary, fallback],
        SpannedExprKind::Ternary {
            condition,
            then_branch,
            else_branch,
        } => vec![condition, then_branch, else_branch],
        SpannedExprKind::ArrayLiteral(items) => items.iter().collect(),
        SpannedExprKind::ObjectLiteral(entries) => {
            entries.iter().map(|(_, value)| value).collect()
        }
        SpannedExprKind::FunctionCall { args, .. } => args.iter().collect(),
        _ => Vec::new(),
    };
    for child in children {
        if let Some(name) = function_call_at(child, source, offset) {
            return Some(name);
        }
    }
    match &expr.kind {
        SpannedExprKind::FunctionCall { name, .. } => {
            let name_span = expr.span.start..expr.span.start + name.len();
            // Exclusive end so a cursor on `(` does not match; the source-text
            // check rejects synthetic `and`/`or` calls whose span starts at the
            // left operand, not at a name token.
            if offset >= name_span.start
                && offset < name_span.end
                && source.get(name_span) == Some(name.as_str())
            {
                Some(name)
            } else {
                None
            }
        }
        _ => None,
    }
}

/// The deepest sub-expression whose span contains `offset` (a byte offset into
/// the expression text), if any.
///
/// Used by the interpolation hover to find which sub-expression (e.g. a
/// `ctx.packages` argument inside `as_csv(ctx.packages)`) is under the cursor,
/// so the ctx/frontmatter hover path can serve it after
/// [`function_call_at`] declines on a non-name offset.
pub fn expression_at(expr: &SpannedExpr, offset: usize) -> Option<&SpannedExpr> {
    if offset < expr.span.start || offset > expr.span.end {
        return None;
    }
    let children: Vec<&SpannedExpr> = match &expr.kind {
        SpannedExprKind::UnaryNot(inner)
        | SpannedExprKind::UnaryMinus(inner)
        | SpannedExprKind::Paren(inner) => vec![inner],
        SpannedExprKind::Binary { left, right, .. }
        | SpannedExprKind::Comparison { left, right, .. } => vec![left, right],
        SpannedExprKind::Index { base, index } => vec![base, index],
        SpannedExprKind::MemberAccess { base, .. } => vec![base],
        SpannedExprKind::Fallback { primary, fallback } => vec![primary, fallback],
        SpannedExprKind::Ternary {
            condition,
            then_branch,
            else_branch,
        } => vec![condition, then_branch, else_branch],
        SpannedExprKind::ArrayLiteral(items) => items.iter().collect(),
        SpannedExprKind::ObjectLiteral(entries) => {
            entries.iter().map(|(_, value)| value).collect()
        }
        SpannedExprKind::FunctionCall { args, .. } => args.iter().collect(),
        _ => Vec::new(),
    };
    for child in children {
        if let Some(found) = expression_at(child, offset) {
            return Some(found);
        }
    }
    Some(expr)
}

/// The completion partial being typed inside an open `{{` at `offset`, and the
/// document offset where it begins. `None` when the cursor is not inside an
/// interpolation.
pub fn completion_partial(text: &str, offset: usize) -> Option<(usize, &str)> {
    let before = &text[..offset];
    let open = before.rfind("{{")?;
    // A `}}` between the last `{{` and the cursor closes the region.
    if before[open..].contains("}}") {
        return None;
    }
    let token_start = identifier_prefix_start(before).max(open + 2);
    Some((token_start, &text[token_start..offset]))
}

/// All context-variable descriptors, in catalog display order.
///
/// The descriptor-returning access surface for `ctx.*` completion and hover:
/// it preserves the full [`ContextVariableDescriptor`] — name, rendered
/// `display_type`, ownership flags, and description — instead of the lossy
/// `(name, description)` projection it replaces.
pub fn context_descriptors() -> &'static [ContextVariableDescriptor] {
    context_variable_descriptors()
}

/// The context-variable descriptor whose bare tail name is `name` (e.g.
/// `"today"`, `"packages"`), if any. The lookup is exact and case-sensitive.
pub fn ctx_descriptor(name: &str) -> Option<&'static ContextVariableDescriptor> {
    context_variable_descriptors()
        .iter()
        .find(|descriptor| descriptor.name == name)
}

/// All expression-function descriptors, in catalog display order.
///
/// The descriptor-returning access surface for function completion and hover:
/// each [`ExpressionFunctionDescriptor`] carries the untyped `signature`, the
/// typed [`ExpressionFunctionDescriptor::typed_signature`], and the description.
pub fn function_descriptors() -> &'static [ExpressionFunctionDescriptor] {
    expression_function_descriptors()
}

/// The expression-function descriptor whose bare name (the leading identifier
/// of its `signature`) is `name`, if any. Overloads share a bare name; the
/// first catalog entry wins, matching the pre-descriptor lookup order.
pub fn function_descriptor(name: &str) -> Option<&'static ExpressionFunctionDescriptor> {
    expression_function_descriptors()
        .iter()
        .find(|descriptor| function_name(descriptor.signature) == name)
}

/// The bare function name — the leading identifier of a catalog `signature`.
fn function_name(signature: &str) -> &str {
    signature.split('(').next().unwrap_or(signature)
}

/// Whether `name` (a `ctx.NAME` tail) is a known context variable.
///
/// Thin wrapper over [`ctx_descriptor`]; new work that needs any other datum
/// (type, description) must take the descriptor directly.
pub fn is_ctx_name(name: &str) -> bool {
    ctx_descriptor(name).is_some()
}

/// The description of a function by its bare name (`length`, `relative`, …).
///
/// Thin wrapper over [`function_descriptor`]; new work that needs the typed
/// signature must take the descriptor directly.
pub fn function_description(name: &str) -> Option<&'static str> {
    function_descriptor(name).map(|descriptor| descriptor.description)
}

/// Renders the shared catalog-backed Markdown block for a `ctx.*` hover.
///
/// This is the single authority for that block: both the interpolation hover
/// ([`crate::providers::dsl`]) and the frontmatter `ctx.*` hover
/// ([`crate::providers::frontmatter`]) render it, so the two surfaces can never
/// drift. It carries the qualified name (`ctx.<name>`), the rendered
/// `display_type`, the read-only/Darkmatter-owned ownership note, and the
/// description — but not any surface-specific trailer (the interpolation
/// compose-time note is appended by its caller).
pub fn format_ctx_hover_block(descriptor: &ContextVariableDescriptor) -> String {
    format!(
        "**`ctx.{}`** ({}) — read-only, Darkmatter-owned\n\n{}",
        descriptor.name, descriptor.display_type, descriptor.description
    )
}

/// Renders the shared catalog-backed Markdown block for a function-call hover.
///
/// Carries the typed signature (e.g. `as_csv(list: any[]) -> string | error`)
/// and the description. Used by the D5 function-call hover and available as the
/// D4 completion documentation source.
///
/// The provider-query functions additionally carry
/// [`query_vocabulary_block`]'s inline vocabulary, so hover answers the
/// authoring question without navigation — including in a workspace where the
/// linked topic doc does not ship. Completion documentation deliberately keeps
/// only the (response-boundary–resolved) link: a completion popup is a
/// one-line surface, not a reference table.
pub fn format_function_block(descriptor: &ExpressionFunctionDescriptor) -> String {
    let mut block = format!(
        "**`{}`**\n\n{}",
        descriptor.typed_signature(),
        descriptor.description
    );
    if let Some(vocabulary) = query_vocabulary_block(function_name(descriptor.signature)) {
        block.push_str("\n\n");
        block.push_str(vocabulary);
    }
    block
}

/// The compact authored-query vocabulary for `pr_list` / `cicd_list` — the keys,
/// the closed enum values, and the bounds an author needs while typing a query
/// object.
///
/// Kept in sync with the `## Provider Query Vocabulary` section of
/// `docs/topics/darkmatter-expressions.md` by
/// `embedded_vocabulary_matches_the_topic_doc`, which fails on any key or enum
/// value that exists in one and not the other. That doc remains the authority
/// for the prose; this is the summary that fits in a hover.
fn query_vocabulary_block(function: &str) -> Option<&'static str> {
    match function {
        "pr_list" => Some(concat!(
            "**Query keys** — `remote`, `state`, `draft`, `source_branch`, ",
            "`target_branch`, `author`, `assignee`, `reviewer`, `labels`, ",
            "`milestone`, `search`, `commit`, `created_after`, `created_before`, ",
            "`updated_after`, `updated_before`, `sort`, `direction`, `limit`\n\n",
            "**Enums** — `state`: `open`, `closed`, `merged` · ",
            "`sort`: `created`, `updated`, `provider-default` · ",
            "`direction`: `ascending`, `descending`\n\n",
            "**Bounds** — `limit` defaults to 20, hard maximum 100; results are ",
            "newest-first. Unknown keys, invalid enum values, inverted date ",
            "ranges, and `sort: \"provider-default\"` combined with `direction` ",
            "are rejected before any provider request.",
        )),
        "cicd_list" => Some(concat!(
            "**Query keys** — `remote`, `statuses`, `name`, `stage`, `workflow`, ",
            "`parent`, `branch`, `commit`, `actor`, `trigger`, `created_after`, ",
            "`created_before`, `updated_after`, `updated_before`, `direction`, ",
            "`limit`\n\n",
            "**Enums** — `statuses`: `success`, `failed`, `cancelled`, `queued`, ",
            "`running`, `manual`, `skipped` · ",
            "`direction`: `ascending`, `descending`\n\n",
            "**Bounds** — `limit` defaults to 20, hard maximum 100; results are ",
            "newest-first. `stage` is honored only where the provider exposes ",
            "stage data (GitLab). Unknown keys, invalid enum values, and ",
            "inverted date ranges are rejected before any provider request.",
        )),
        _ => None,
    }
}

/// The kind of an [`ExprCompletion`] candidate, so a caller can map it onto its
/// own LSP `CompletionItemKind` without this module depending on `lsp-types`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExprCompletionKind {
    /// A same-document top-level frontmatter key.
    FrontmatterKey,
    /// A fully qualified `ctx.<name>` context variable.
    ContextVariable,
    /// An expression function.
    Function,
}

/// One completion candidate for an expression context — either a body `{{ }}`
/// interpolation or an Expression-typed frontmatter value. Presentation-neutral:
/// every field is derived from the one catalog descriptor (or a frontmatter
/// key), and LSP lowering happens in the provider.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExprCompletion {
    /// The completion label — the untyped `signature` for functions.
    pub label: String,
    /// The eagerly inserted text — the bare name for functions (no snippet, no
    /// synthesized parentheses), the fully qualified `ctx.<name>` for context
    /// variables, the key for frontmatter keys.
    pub insert_text: String,
    /// The candidate kind.
    pub kind: ExprCompletionKind,
    /// The rendered `display_type` (`ctx.*`) or `typed_signature()` (functions).
    pub detail: Option<String>,
    /// The catalog description, ready to lower to eager Markdown documentation.
    pub documentation: Option<String>,
}

/// Catalog + frontmatter-key completion candidates matching `partial`
/// (prefix-based, case-sensitive): same-document top-level frontmatter keys,
/// fully qualified `ctx.*` variables, and expression functions.
///
/// The single authority for expression completion so the body-interpolation and
/// Expression-typed-frontmatter surfaces can never drift.
pub fn completion_candidates(partial: &str, frontmatter_keys: &[String]) -> Vec<ExprCompletion> {
    let mut items = Vec::new();

    for key in frontmatter_keys {
        if key.starts_with(partial) {
            items.push(ExprCompletion {
                label: key.clone(),
                insert_text: key.clone(),
                kind: ExprCompletionKind::FrontmatterKey,
                detail: Some("frontmatter key".to_string()),
                documentation: None,
            });
        }
    }

    for descriptor in context_descriptors() {
        let label = format!("ctx.{}", descriptor.name);
        if label.starts_with(partial) {
            items.push(ExprCompletion {
                insert_text: label.clone(),
                label,
                kind: ExprCompletionKind::ContextVariable,
                detail: Some(descriptor.display_type.to_string()),
                documentation: Some(descriptor.description.to_string()),
            });
        }
    }

    for descriptor in function_descriptors() {
        let signature = descriptor.signature;
        let name = function_name(signature);
        if name.starts_with(partial) {
            items.push(ExprCompletion {
                label: signature.to_string(),
                insert_text: name.to_string(),
                kind: ExprCompletionKind::Function,
                detail: Some(descriptor.typed_signature()),
                documentation: Some(descriptor.description.to_string()),
            });
        }
    }

    items
}

/// The Markdown body of a **value-dialect** expression hover — the body `{{ }}`
/// interpolation hover. `expr_offset` is the cursor's byte offset into
/// `expression`. Pure so the D2/D5 classification is unit-testable.
///
/// The single authority for value-dialect expression hover markdown. See
/// [`hover_markdown_from`] for the shared classification rule, and
/// [`hover_markdown_condition`] for the condition-dialect companion used by
/// Expression-typed frontmatter values.
pub fn hover_markdown(
    expression: &str,
    expr_offset: usize,
    frontmatter_scalar: impl Fn(&str) -> Option<String>,
    schema_property: impl Fn(&str) -> Option<String>,
) -> String {
    hover_markdown_from(
        parse(expression),
        expression,
        expr_offset,
        frontmatter_scalar,
        schema_property,
    )
}

/// The Markdown body of a **condition-dialect** expression hover — the
/// Expression-typed frontmatter value hover. Identical formatting to
/// [`hover_markdown`], but parses with [`parse_condition`] so `&&`/`||`
/// expressions (which the `expression` schema format accepts) receive the same
/// intelligence as any other expression instead of an unparsed fallback.
pub fn hover_markdown_condition(
    expression: &str,
    expr_offset: usize,
    frontmatter_scalar: impl Fn(&str) -> Option<String>,
    schema_property: impl Fn(&str) -> Option<String>,
) -> String {
    hover_markdown_from(
        parse_condition(expression),
        expression,
        expr_offset,
        frontmatter_scalar,
        schema_property,
    )
}

/// The shared hover-markdown formatter over an already-parsed expression, so the
/// value- and condition-dialect entry points render identically once parsed.
/// `expr_offset` is the cursor's byte offset into `expression`.
///
/// The D2 classification rule: only an explicitly `ctx.`-qualified root receives
/// context-variable metadata — a bare identifier is a frontmatter variable even
/// when its name matches a known `ctx.*` tail, and an unknown `ctx.<name>` keeps
/// the generic hover without borrowing a similarly named bare key's value. A bare
/// identifier with no static frontmatter value falls back to `schema_property` —
/// the effective schema's type/constraints/description for a declared-but-unset
/// property — before the generic function-name description. Nothing here
/// evaluates the expression or reads `ctx.*`.
fn hover_markdown_from(
    parsed: Result<SpannedExpr, ParseError>,
    expression: &str,
    expr_offset: usize,
    frontmatter_scalar: impl Fn(&str) -> Option<String>,
    schema_property: impl Fn(&str) -> Option<String>,
) -> String {
    let mut value = match &parsed {
        Ok(expr) => format!("**Expression**\n\n`{}`", expr.erase()),
        Err(_) => format!("**Expression** (unparsed)\n\n`{expression}`"),
    };
    let Ok(expr) = parsed else {
        return value;
    };

    // D5: a cursor on a known function-name identifier wins.
    if let Some(name) = function_call_at(&expr, expression, expr_offset)
        && let Some(descriptor) = function_descriptor(name)
    {
        value.push_str(&format!("\n\n{}", format_function_block(descriptor)));
        return value;
    }

    // Resolve the identifier against the sub-expression under the cursor (e.g.
    // a `ctx.packages` argument inside a call) rather than the top-level
    // expression, so an argument's ctx/frontmatter hover is not shadowed by
    // the enclosing call.
    let Some(sub_expr) = expression_at(&expr, expr_offset) else {
        return value;
    };
    let Some(name) = root_identifier(sub_expr) else {
        return value;
    };
    if let Some(tail) = name.strip_prefix("ctx.") {
        if let Some(descriptor) = ctx_descriptor(tail) {
            value.push_str(&format!(
                "\n\n{}\n\nThe `ctx` variable is evaluated at _compose_ time (rather than now).",
                format_ctx_hover_block(descriptor)
            ));
        }
    } else if let Some(scalar) = frontmatter_scalar(&name) {
        value.push_str(&format!("\n\nStatic value: `{scalar}` (from frontmatter `{name}`)"));
    } else if let Some(block) = schema_property(&name) {
        value.push_str(&format!("\n\n{block}"));
    } else if let Some(descriptor) = function_descriptor(&name) {
        value.push_str(&format!("\n\nFunction: {}", descriptor.description));
    }
    value
}

/// Whether a bare identifier `name` names nothing DMLS can resolve — no
/// frontmatter key, schema property, reserved root, bare runtime-context name,
/// or expression function. The single authority for the unknown-root check so
/// the body-interpolation and frontmatter-expression diagnostics agree.
///
/// Reserved roots (`ctx`, `env`, `doc`, `current`, `current_env`, `null`) and
/// bare runtime-context names come from the library's
/// [`is_statically_known_root`], so the editor never flags a root the compose
/// runtime knows. Any dotted path is treated as known; callers pass a root.
/// The caller supplies frontmatter-key and schema-property membership.
pub fn is_unknown_root(
    name: &str,
    is_frontmatter_key: impl Fn(&str) -> bool,
    is_schema_property: impl Fn(&str) -> bool,
) -> bool {
    if is_statically_known_root(name) || name.contains('.') {
        return false;
    }
    if function_description(name).is_some() {
        return false;
    }
    if is_frontmatter_key(name) {
        return false;
    }
    !is_schema_property(name)
}

/// The replacement a dash-separated-key quick-fix applies over its
/// diagnostic's range. Carried as `Diagnostic.data` (Ruling R-8) so the
/// code-action provider never reads the message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyReferenceFix {
    /// The frontmatter key the subtraction's source spells.
    pub key: String,
    /// The expression text that references `key`: the bare key when it lexes
    /// as one identifier, otherwise `doc['key']`.
    pub replacement: String,
}

/// One `dm.expression.unknown_identifier` finding. Spans index the expression
/// text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnknownIdentifierFinding {
    /// An unhandled `Variable` whose root names nothing DMLS can resolve.
    Identifier {
        /// The root (first dotted segment).
        root: String,
        /// The `Variable` node's span.
        span: SourceSpan,
    },
    /// A subtraction whose whitespace-free source is exactly a frontmatter
    /// key. It replaces the generic findings for its operands.
    DashSeparatedKey {
        /// The subtraction as authored.
        authored: String,
        /// The key its whitespace-free source spells.
        key: String,
        /// The subtraction's span.
        span: SourceSpan,
        /// `None` when no replacement could be proven to reparse as a
        /// reference to the key.
        fix: Option<KeyReferenceFix>,
    },
}

/// Every unknown-identifier finding for one parsed expression.
///
/// `reparse` is the dialect `expr` was parsed with; a quick-fix is offered
/// only when the edited expression reparses with the replacement as a single
/// reference to the key. `is_unknown` classifies a root; `is_key` answers
/// whether a top-level frontmatter key is present or declared.
/// `forbidden_quote` is a quote character the replacement must not contain
/// (the YAML quote style around a frontmatter value).
///
/// Absence handling follows the library's static walk
/// ([`static_variable_reads`]), so the editor suppresses what the runtime
/// suppresses; it flags both branches because it cannot know which runs.
pub fn unknown_identifier_findings(
    expr: &SpannedExpr,
    source: &str,
    reparse: fn(&str) -> Result<SpannedExpr, ParseError>,
    is_unknown: impl Fn(&str) -> bool,
    is_key: impl Fn(&str) -> bool,
    forbidden_quote: Option<char>,
) -> Vec<UnknownIdentifierFinding> {
    let unknown: Vec<(String, SourceSpan)> = static_variable_reads(expr)
        .into_iter()
        .filter(|read| !read.handles_absence && is_unknown(read.root()))
        .map(|read| (read.root().to_string(), read.span))
        .collect();
    if unknown.is_empty() {
        return Vec::new();
    }

    let mut dash_keys = Vec::new();
    collect_dash_keys(expr, source, &unknown, &is_key, &mut dash_keys);

    let mut findings: Vec<UnknownIdentifierFinding> = unknown
        .into_iter()
        .filter(|(_, span)| !dash_keys.iter().any(|(chain, _)| contains(chain, span)))
        .map(|(root, span)| UnknownIdentifierFinding::Identifier { root, span })
        .collect();
    findings.extend(dash_keys.into_iter().map(|(span, key)| {
        UnknownIdentifierFinding::DashSeparatedKey {
            authored: source[span.clone()].to_string(),
            fix: key_reference_fix(source, &span, &key, reparse, forbidden_quote),
            key,
            span,
        }
    }));
    findings.sort_by_key(|finding| match finding {
        UnknownIdentifierFinding::Identifier { span, .. }
        | UnknownIdentifierFinding::DashSeparatedKey { span, .. } => span.start,
    });
    findings
}

fn contains(outer: &SourceSpan, inner: &SourceSpan) -> bool {
    outer.start <= inner.start && inner.end <= outer.end
}

/// The outermost subtraction chains (`a- b`, `foo--bar`, `a - b - c`) whose
/// whitespace-free source is a key and which contain an unknown read. A chain
/// holds only variables, unary minus, and subtraction; any literal or other
/// operator makes it ordinary arithmetic.
fn collect_dash_keys(
    expr: &SpannedExpr,
    source: &str,
    unknown: &[(String, SourceSpan)],
    is_key: &impl Fn(&str) -> bool,
    out: &mut Vec<(SourceSpan, String)>,
) {
    if matches!(expr.kind, SpannedExprKind::Binary { op: BinaryOp::Sub, .. })
        && is_dash_chain(expr)
        && unknown.iter().any(|(_, span)| contains(&expr.span, span))
        && let Some(authored) = source.get(expr.span.clone())
    {
        let key: String = authored.chars().filter(|c| !c.is_whitespace()).collect();
        if is_key(&key) {
            out.push((expr.span.clone(), key));
            return;
        }
    }
    for child in children(expr) {
        collect_dash_keys(child, source, unknown, is_key, out);
    }
}

fn is_dash_chain(expr: &SpannedExpr) -> bool {
    match &expr.kind {
        SpannedExprKind::Variable(path) => !path.contains('.'),
        SpannedExprKind::UnaryMinus(inner) => is_dash_chain(inner),
        SpannedExprKind::Binary {
            op: BinaryOp::Sub,
            left,
            right,
        } => is_dash_chain(left) && is_dash_chain(right),
        _ => false,
    }
}

/// The first replacement for `span` that reparses as exactly one reference
/// to `key`: the bare key, then `doc['key']`.
fn key_reference_fix(
    source: &str,
    span: &SourceSpan,
    key: &str,
    reparse: fn(&str) -> Result<SpannedExpr, ParseError>,
    forbidden_quote: Option<char>,
) -> Option<KeyReferenceFix> {
    let bare = key.to_string();
    let quote = ['\'', '"']
        .into_iter()
        .find(|quote| Some(*quote) != forbidden_quote && !key.contains(*quote))
        .filter(|_| !key.contains('\\'));
    let bracket = quote.map(|quote| format!("doc[{quote}{key}{quote}]"));
    std::iter::once(bare)
        .chain(bracket)
        .find(|replacement| {
            let edited = format!("{}{replacement}{}", &source[..span.start], &source[span.end..]);
            let Ok(parsed) = reparse(&edited) else {
                return false;
            };
            let target = span.start..span.start + replacement.len();
            node_with_span(&parsed, &target).is_some_and(|node| references_key(node, key))
        })
        .map(|replacement| KeyReferenceFix {
            key: key.to_string(),
            replacement,
        })
}

fn references_key(node: &SpannedExpr, key: &str) -> bool {
    match &node.kind {
        SpannedExprKind::Variable(path) => path == key,
        SpannedExprKind::Index { base, index } => {
            matches!(&base.kind, SpannedExprKind::Variable(root) if root == "doc")
                && matches!(&index.kind, SpannedExprKind::StringLiteral(literal) if literal == key)
        }
        _ => false,
    }
}

fn node_with_span<'a>(expr: &'a SpannedExpr, span: &SourceSpan) -> Option<&'a SpannedExpr> {
    if expr.span == *span {
        return Some(expr);
    }
    children(expr)
        .into_iter()
        .find_map(|child| node_with_span(child, span))
}

fn children(expr: &SpannedExpr) -> Vec<&SpannedExpr> {
    match &expr.kind {
        SpannedExprKind::UnaryNot(inner)
        | SpannedExprKind::UnaryMinus(inner)
        | SpannedExprKind::Paren(inner) => vec![inner],
        SpannedExprKind::Binary { left, right, .. }
        | SpannedExprKind::Comparison { left, right, .. } => vec![left, right],
        SpannedExprKind::Index { base, index } => vec![base, index],
        SpannedExprKind::MemberAccess { base, .. } => vec![base],
        SpannedExprKind::Fallback { primary, fallback } => vec![primary, fallback],
        SpannedExprKind::Ternary {
            condition,
            then_branch,
            else_branch,
        } => vec![condition, then_branch, else_branch],
        SpannedExprKind::ArrayLiteral(items) => items.iter().collect(),
        SpannedExprKind::ObjectLiteral(entries) => {
            entries.iter().map(|(_, value)| value).collect()
        }
        SpannedExprKind::FunctionCall { args, .. } => args.iter().collect(),
        _ => Vec::new(),
    }
}

/// The human-readable message for a finding. Code actions read
/// [`KeyReferenceFix`] from `Diagnostic.data`, never this text.
pub fn unknown_identifier_message(finding: &UnknownIdentifierFinding) -> String {
    match finding {
        UnknownIdentifierFinding::Identifier { root, .. } => format!(
            "`{root}` matches no frontmatter key, schema property, `ctx.*`, `env.*`, or function"
        ),
        UnknownIdentifierFinding::DashSeparatedKey {
            authored, key, fix: Some(fix), ..
        } => format!(
            "`{authored}` is a subtraction, but frontmatter key `{key}` exists; reference it as `{}`",
            fix.replacement
        ),
        UnknownIdentifierFinding::DashSeparatedKey { authored, key, .. } => format!(
            "`{authored}` is a subtraction, but frontmatter key `{key}` exists"
        ),
    }
}

/// The expression completion partial being typed inside an Expression-typed
/// frontmatter value, and the document offset where it begins.
///
/// `value_text` is the authored text of the value from its start to the cursor;
/// `value_start` is that value start's document offset. The token is the
/// trailing identifier partial under the lexer's identifier rule (so `ctx.to`,
/// `as_csv(ctx.pa`'s `ctx.pa`, a bare `len`, and a kebab `spec-na` all resolve). Always `Some` — an empty partial
/// offers the full catalog — so the caller decides whether the value is
/// Expression-typed before calling.
pub fn value_completion_partial(value_text: &str, value_start: usize) -> (usize, &str) {
    let token_rel = identifier_prefix_start(value_text);
    (value_start + token_rel, &value_text[token_rel..])
}

#[cfg(test)]
mod frontmatter_inventory_tests;

#[cfg(test)]
mod tests {
    use super::*;

    /// Findings for a value-dialect `source` where `keys` are the document's
    /// frontmatter keys.
    fn findings_for(source: &str, keys: &[&str], forbidden_quote: Option<char>) -> Vec<UnknownIdentifierFinding> {
        let expr = parse(source).unwrap();
        unknown_identifier_findings(
            &expr,
            source,
            parse,
            |root| is_unknown_root(root, |name| keys.contains(&name), |_| false),
            |key| keys.contains(&key),
            forbidden_quote,
        )
    }

    fn generic(source: &str, keys: &[&str]) -> Vec<(String, String)> {
        findings_for(source, keys, None)
            .into_iter()
            .map(|finding| match finding {
                UnknownIdentifierFinding::Identifier { root, span } => (root, source[span].to_string()),
                other => panic!("{source}: expected only generic findings, got {other:?}"),
            })
            .collect()
    }

    fn dash(source: &str, keys: &[&str], forbidden_quote: Option<char>) -> (String, String, Option<String>) {
        match findings_for(source, keys, forbidden_quote).as_slice() {
            [UnknownIdentifierFinding::DashSeparatedKey { authored, key, span, fix }] => {
                assert_eq!(&source[span.clone()], authored);
                (authored.clone(), key.clone(), fix.as_ref().map(|fix| fix.replacement.clone()))
            }
            other => panic!("{source}: expected one dash-separated-key finding, got {other:?}"),
        }
    }

    fn pair(root: &str, text: &str) -> (String, String) {
        (root.to_string(), text.to_string())
    }

    #[test]
    fn every_operand_position_is_checked_at_its_own_span() {
        let keys = ["known"];
        assert_eq!(generic("known - bin", &keys), [pair("bin", "bin")]);
        assert_eq!(
            generic("known ? then_b : else_b", &keys),
            [pair("then_b", "then_b"), pair("else_b", "else_b")]
        );
        assert_eq!(generic("lower(arg)", &keys), [pair("arg", "arg")]);
        assert_eq!(generic("known || rhs", &keys), [pair("rhs", "rhs")]);
        assert_eq!(generic("user.name + 1", &keys), [pair("user", "user.name")]);
    }

    #[test]
    fn handled_absence_and_known_roots_are_silent() {
        for source in [
            "maybe || \"d\"",
            "maybe ? maybe : \"none\"",
            "is_null(maybe)",
            "isEmpty(maybe)",
            "ok ? known : null",
            "repo",
            "current.cwd",
            "doc.anything-at-all",
            "ctx.nope",
            "length",
        ] {
            let found = generic(source, &["known", "ok"]);
            assert!(found.is_empty(), "{source}: {found:?}");
        }
        assert_eq!(generic("is_empty(lower(nested))", &[]), [pair("nested", "nested")]);
    }

    #[test]
    fn a_subtraction_spelling_a_key_is_one_finding_with_the_right_replacement() {
        assert_eq!(
            dash("foo--bar", &["foo--bar"], None),
            ("foo--bar".into(), "foo--bar".into(), Some("doc['foo--bar']".into()))
        );
        assert_eq!(
            dash("a- b", &["a-b"], None),
            ("a- b".into(), "a-b".into(), Some("a-b".into()))
        );
        // The YAML quote style around a frontmatter value picks the other quote.
        assert_eq!(dash("foo--bar", &["foo--bar"], Some('\'')).2.as_deref(), Some("doc[\"foo--bar\"]"));
        // Only the chain that spells the key is replaced; the outer subtraction
        // stays arithmetic after the fix.
        let (authored, _, fix) = match findings_for("a- b - known", &["a-b", "known"], None).as_slice() {
            [UnknownIdentifierFinding::DashSeparatedKey { authored, key, fix, .. }] => {
                (authored.clone(), key.clone(), fix.clone())
            }
            other => panic!("{other:?}"),
        };
        assert_eq!(authored, "a- b");
        assert_eq!(fix.map(|fix| fix.replacement).as_deref(), Some("a-b"));
    }

    #[test]
    fn ambiguous_arithmetic_keeps_generic_findings_and_no_fix() {
        // No key spells the subtraction.
        assert_eq!(generic("c - d", &[]), [pair("c", "c"), pair("d", "d")]);
        // A literal operand is ordinary arithmetic even when a key matches.
        assert_eq!(generic("iteration - 1", &["iteration-1"]), [pair("iteration", "iteration")]);
        // Both operands known: intentional arithmetic, nothing to report.
        assert!(findings_for("a - b", &["a", "b", "a-b"], None).is_empty());
    }

    #[test]
    fn a_key_no_replacement_can_reference_gets_no_fix() {
        assert_eq!(key_reference_fix("x", &(0..1), "it's \"q\"", parse, None), None);
        let fix = key_reference_fix("x", &(0..1), "it's", parse, None).expect("double-quoted bracket");
        assert_eq!(fix.replacement, "doc[\"it's\"]");
    }

    #[test]
    fn the_fix_payload_round_trips_through_json() {
        let fix = KeyReferenceFix {
            key: "foo--bar".into(),
            replacement: "doc['foo--bar']".into(),
        };
        let value = serde_json::to_value(&fix).unwrap();
        assert_eq!(value, serde_json::json!({ "key": "foo--bar", "replacement": "doc['foo--bar']" }));
        assert_eq!(serde_json::from_value::<KeyReferenceFix>(value).unwrap(), fix);
    }

    #[test]
    fn test_interpolations_found_and_spanned() {
        let text = "# Body\n\nHello {{ title }} and {{ ctx.today }}.\n";
        let found = interpolations(text, 0);
        assert_eq!(found.len(), 2);
        assert_eq!(found[0].text, "title");
        assert_eq!(&text[found[0].inner.clone()], "title");
        assert_eq!(&text[found[0].outer.clone()], "{{ title }}");
    }

    #[test]
    fn test_body_base_filters_frontmatter_region() {
        let text = "---\ntitle: {{ seed }}\n---\n\n{{ body_var }}\n";
        let body_base = text.find("\n\n").unwrap() + 2;
        let found = interpolations(text, body_base);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].text, "body_var");
    }

    #[test]
    fn test_root_identifier_keeps_dotted_variable() {
        // The lexer keeps dotted paths as a single `Variable` token.
        let expr = parse("ctx.today").unwrap();
        assert_eq!(root_identifier(&expr).as_deref(), Some("ctx.today"));
        let bare = parse("title").unwrap();
        assert_eq!(root_identifier(&bare).as_deref(), Some("title"));
    }

    #[test]
    fn test_completion_partial_inside_open_interpolation() {
        let text = "value {{ ctx.to";
        let (start, partial) = completion_partial(text, text.len()).unwrap();
        // The dotted namespace is part of the completion partial.
        assert_eq!(partial, "ctx.to");
        assert_eq!(&text[start..], "ctx.to");
        // Closed region offers nothing.
        assert!(completion_partial("value {{ x }} after", 19).is_none());
    }

    #[test]
    fn test_parse_error_position_is_byte_offset() {
        let error = parse("1 +").unwrap_err();
        assert!(error.position <= 3);
    }

    #[test]
    fn test_ctx_descriptor_returns_full_catalog_entry() {
        let today = ctx_descriptor("today").expect("`today` is a known context variable");
        assert_eq!(today.name, "today");
        assert!(!today.description.is_empty());
        // Thin wrapper agrees with the descriptor lookup.
        assert!(is_ctx_name("today"));
        assert!(ctx_descriptor("definitely_not_a_ctx_var").is_none());
        assert!(!is_ctx_name("definitely_not_a_ctx_var"));
    }

    #[test]
    fn test_ctx_descriptor_preserves_rendered_array_type() {
        // The lossy `(name, description)` projection discarded this; the
        // descriptor accessor keeps the rendered `string[]` array type.
        let packages = ctx_descriptor("packages").expect("`packages` is a known context variable");
        assert_eq!(packages.display_type.to_string(), "string[]");
    }

    #[test]
    fn test_function_descriptor_returns_typed_signature() {
        let length = function_descriptor("length").expect("`length` is a known function");
        assert_eq!(function_name(length.signature), "length");
        // Thin wrapper agrees with the descriptor lookup.
        assert_eq!(function_description("length"), Some(length.description));
        assert!(function_descriptor("definitely_not_a_function").is_none());
        assert!(function_description("definitely_not_a_function").is_none());
    }

    #[test]
    fn test_function_descriptor_fallible_signature_has_error_suffix() {
        // `length` is fallible, so its typed signature carries the
        // `| error` union suffix that the untyped `signature` lacks.
        let length = function_descriptor("length").expect("`length` is a known function");
        assert!(length.signature.contains("length("));
        assert!(!length.signature.contains("| error"));
        assert!(length.typed_signature().ends_with("| error"));
    }

    #[test]
    fn remote_functions_and_closed_enum_reach_passive_descriptors() {
        for name in [
            "branch_exists_on_remote",
            "remote_vendor",
            "pr",
            "pr_list",
            "cicd",
            "cicd_list",
        ] {
            assert!(function_descriptor(name).is_some(), "missing {name}");
        }

        let vendor = function_descriptor("remote_vendor").unwrap();
        let typed = vendor.typed_signature();
        assert!(typed.contains("enum(\"\", github, gitlab, gitea, forgejo"));
        assert!(typed.ends_with("| error"));
    }

    #[test]
    fn shipped_catalog_descriptors_all_reach_completion() {
        let candidates = completion_candidates("", &[]);

        for descriptor in context_descriptors() {
            let label = format!("ctx.{}", descriptor.name);
            let candidate = candidates
                .iter()
                .find(|candidate| candidate.label == label)
                .unwrap_or_else(|| panic!("context descriptor `{label}` missing from completion"));
            assert_eq!(candidate.kind, ExprCompletionKind::ContextVariable);
            assert_eq!(candidate.insert_text, label);
            assert_eq!(candidate.detail.as_deref(), Some(descriptor.display_type.to_string().as_str()));
            assert_eq!(candidate.documentation.as_deref(), Some(descriptor.description));
        }

        for descriptor in function_descriptors() {
            let name = function_name(descriptor.signature);
            let candidate = candidates
                .iter()
                .find(|candidate| {
                    candidate.label == descriptor.signature && candidate.insert_text == name
                })
                .unwrap_or_else(|| {
                    panic!("function descriptor `{}` missing from completion", descriptor.signature)
                });
            assert_eq!(candidate.kind, ExprCompletionKind::Function);
            assert_eq!(candidate.detail, Some(descriptor.typed_signature()));
            assert_eq!(candidate.documentation.as_deref(), Some(descriptor.description));
        }
    }

    /// Hover must answer the authoring question on its own: the compact
    /// vocabulary is inline, so no navigation — and no shipped topic doc — is
    /// required to read the keys, enums, and bounds.
    #[test]
    fn list_query_hover_embeds_the_vocabulary_and_completion_does_not() {
        for (name, expected_key) in [("pr_list", "`target_branch`"), ("cicd_list", "`workflow`")] {
            let descriptor = function_descriptor(name).unwrap();
            let block = format_function_block(descriptor);
            assert!(block.contains("**Query keys**"), "{block}");
            assert!(block.contains(expected_key), "{block}");
            assert!(block.contains("**Enums**"), "{block}");
            assert!(block.contains("hard maximum 100"), "{block}");
        }
        // The other function's keys never leak into this one's block.
        assert!(!format_function_block(function_descriptor("pr_list").unwrap()).contains("`workflow`"));
        assert!(!format_function_block(function_descriptor("cicd_list").unwrap()).contains("`draft`"));
        // A non-query function carries no vocabulary at all.
        assert!(!format_function_block(function_descriptor("length").unwrap()).contains("**Query keys**"));

        // Completion documentation stays the one-line catalog description.
        for candidate in completion_candidates("pr_list", &[]) {
            let documentation = candidate.documentation.as_deref().unwrap_or_default();
            assert!(!documentation.contains("**Query keys**"), "{documentation}");
        }
    }

    /// The embedded vocabulary is a summary of
    /// `docs/topics/darkmatter-expressions.md`, which stays the authority. A key
    /// or enum value present in one and absent from the other is drift, and the
    /// hover would then teach an authoring vocabulary the engine does not honor.
    #[test]
    fn embedded_vocabulary_matches_the_topic_doc() {
        let doc = std::fs::read_to_string(
            biscuit_test_harness::manifest_dir!()
                .join("../docs/topics/darkmatter-expressions.md"),
        )
        .expect("darkmatter-expressions.md should be readable");

        for (function, section) in [
            ("pr_list", "### `pr_list(query)` keys"),
            ("cicd_list", "### `cicd_list(query)` keys"),
        ] {
            let start = doc.find(section).expect("vocabulary section exists");
            let table = &doc[start..];
            let end = table[section.len()..]
                .find("\n### ")
                .map(|offset| offset + section.len())
                .unwrap_or(table.len());
            let documented = table_keys(&table[..end]);
            let embedded = backticked(query_vocabulary_block(function).expect("embedded block"));

            for key in &documented {
                assert!(
                    embedded.contains(key),
                    "`{key}` is documented for {function} but missing from the embedded vocabulary"
                );
            }
            // Every embedded key must be a real one; the enum/bounds paragraphs
            // reference keys the table also lists, so the check is symmetric.
            for key in &embedded {
                assert!(
                    documented.contains(key) || is_enum_value_or_prose(key),
                    "`{key}` is embedded for {function} but not documented in the topic doc"
                );
            }
        }

        // Closed enum values must agree with the doc's enum table.
        let enums = doc
            .find("### Closed enum values")
            .map(|start| backticked(&doc[start..start + 600]))
            .expect("closed enum table exists");
        for value in ["open", "closed", "merged", "created", "updated", "provider-default",
            "ascending", "descending", "success", "failed", "cancelled", "queued",
            "running", "manual", "skipped"]
        {
            assert!(
                enums.iter().any(|found| found == value),
                "`{value}` missing from the doc's enum table"
            );
            let blocks = format!(
                "{}{}",
                query_vocabulary_block("pr_list").unwrap(),
                query_vocabulary_block("cicd_list").unwrap()
            );
            assert!(blocks.contains(value), "`{value}` missing from the embedded enums");
        }
    }

    /// The key names in the first column of a Markdown key table — a row's
    /// leading cell, which may name a pair (`created_after` / `created_before`).
    /// The section heading is not a row, so its own backticks stay out.
    fn table_keys(section: &str) -> Vec<String> {
        section
            .lines()
            .filter(|line| line.starts_with('|'))
            .filter_map(|line| line.split('|').nth(1))
            .flat_map(backticked)
            .collect()
    }

    /// Every backtick-quoted token in `text`, deduplicated.
    fn backticked(text: &str) -> Vec<String> {
        let mut found = Vec::new();
        let mut rest = text;
        while let Some(open) = rest.find('`') {
            let after = &rest[open + 1..];
            let Some(close) = after.find('`') else { break };
            let token = &after[..close];
            if !token.is_empty() && !found.iter().any(|seen: &String| seen == token) {
                found.push(token.to_string());
            }
            rest = &after[close + 1..];
        }
        found
    }

    /// Tokens the embedded block quotes that are enum values or bound examples
    /// rather than query keys — they are checked against the doc's enum table
    /// and prose separately.
    fn is_enum_value_or_prose(token: &str) -> bool {
        matches!(
            token,
            "open" | "closed" | "merged"
                | "created" | "updated" | "provider-default"
                | "ascending" | "descending"
                | "success" | "failed" | "cancelled" | "queued" | "running" | "manual" | "skipped"
                | "sort: \"provider-default\""
        )
    }

    /// The spec requires function hover to link to the authored query
    /// vocabulary. Both promised surfaces — D5 hover and D4 completion
    /// documentation — must carry the link for `pr_list` and `cicd_list`.
    /// Both emit `MarkupKind::Markdown`, so the Markdown link renders.
    ///
    /// This asserts the *authored* spelling, which is what the catalog carries.
    /// [`crate::overlay::doc_links`] rewrites it to a resolvable target at the
    /// LSP response boundary; `tests/l1/lsp_session.rs` proves that end of it.
    #[test]
    fn list_query_functions_link_to_the_vocabulary_in_hover_and_completion() {
        const LINK: &str = "(darkmatter-expressions.md#provider-query-vocabulary)";

        let candidates = completion_candidates("", &[]);
        for name in ["pr_list", "cicd_list"] {
            let descriptor =
                function_descriptor(name).unwrap_or_else(|| panic!("`{name}` must be in the catalog"));

            // D5 function-call hover.
            let block = format_function_block(descriptor);
            assert!(
                block.contains(LINK),
                "`{name}` hover block must link to the query vocabulary: {block}"
            );

            // D4 completion documentation.
            let documented = candidates
                .iter()
                .filter(|candidate| function_name(&candidate.label) == name)
                .collect::<Vec<_>>();
            assert!(!documented.is_empty(), "`{name}` must reach completion");
            for candidate in documented {
                assert_eq!(candidate.kind, ExprCompletionKind::Function);
                let documentation = candidate
                    .documentation
                    .as_deref()
                    .unwrap_or_else(|| panic!("`{name}` completion must carry documentation"));
                assert!(
                    documentation.contains(LINK),
                    "`{name}` completion documentation must link to the query vocabulary: {documentation}"
                );
            }
        }
    }

    #[test]
    fn test_function_call_at_finds_deepest_call() {
        let source = "as_csv(length(items))";
        let expr = parse(source).unwrap();
        let inner = source.find("length").unwrap();
        // Cursor on the inner call's name resolves to that call.
        assert_eq!(function_call_at(&expr, source, inner + 1), Some("length"));
        // Cursor on the outer name resolves to the outer call.
        assert_eq!(function_call_at(&expr, source, 1), Some("as_csv"));
        // Cursor on an argument, parenthesis, or comma claims nothing — the
        // caller's ctx/frontmatter hover path takes over.
        assert_eq!(function_call_at(&expr, source, source.find("items").unwrap()), None);
        assert_eq!(function_call_at(&expr, source, source.find("(").unwrap()), None);
        assert_eq!(function_call_at(&expr, source, source.rfind(")").unwrap()), None);
        // Outside the expression entirely, and on a non-call expression: none.
        assert_eq!(function_call_at(&expr, source, source.len() + 5), None);
        let plain = parse("title").unwrap();
        assert_eq!(function_call_at(&plain, "title", 2), None);
    }

    #[test]
    fn function_and_expression_lookup_descend_into_collection_literals() {
        let source = "pr_list({ state: [fallback(\"open\", status)] })";
        let expr = parse(source).unwrap();
        let fallback = source.find("fallback").unwrap();
        assert_eq!(
            function_call_at(&expr, source, fallback + 1),
            Some("fallback")
        );

        let status = source.find("status").unwrap();
        let sub = expression_at(&expr, status + 1).unwrap();
        assert_eq!(root_identifier(sub).as_deref(), Some("status"));
    }

    #[test]
    fn test_expression_at_finds_deepest_subexpression() {
        let source = "as_csv(ctx.packages)";
        let expr = parse(source).unwrap();
        // Cursor on `ctx.packages` (the argument) finds that Variable.
        let sub = expression_at(&expr, 8).expect("cursor on argument finds sub-expression");
        assert_eq!(root_identifier(sub).as_deref(), Some("ctx.packages"));
        // Cursor on the function name finds the FunctionCall itself.
        let sub = expression_at(&expr, 1).expect("cursor on name finds function call");
        assert!(matches!(sub.kind, SpannedExprKind::FunctionCall { .. }));
        // Cursor outside the expression finds nothing.
        assert!(expression_at(&expr, source.len() + 5).is_none());
    }

    #[test]
    fn test_format_ctx_hover_block_carries_name_type_ownership_description() {
        let packages = ctx_descriptor("packages").expect("`packages` is a known context variable");
        let block = format_ctx_hover_block(packages);
        assert!(block.contains("**`ctx.packages`**"));
        assert!(block.contains("(string[])"));
        assert!(block.contains("read-only, Darkmatter-owned"));
        assert!(block.contains(packages.description));
    }

    #[test]
    fn test_format_function_block_carries_typed_signature_and_description() {
        let length = function_descriptor("length").expect("`length` is a known function");
        let block = format_function_block(length);
        assert!(block.contains(&length.typed_signature()));
        assert!(block.contains("| error"));
        assert!(block.contains(length.description));
    }

    /// More-context plan 4.7: every added `ctx.*` variable and function is
    /// discoverable by completion and hover from descriptors alone. The
    /// runtime bindings for most of these functions are still pending and fail
    /// when called, so hover succeeding with the typed signature and no
    /// runtime error text shows DMLS never evaluates them.
    #[test]
    fn more_context_names_reach_completion_and_hover_without_evaluation() {
        let candidates = completion_candidates("", &[]);
        for variable in [
            "self", "last_updated", "hash", "id", "sid", "hostname", "tailnet", "gateway",
            "gateway_v6", "recent_commits",
        ] {
            let label = format!("ctx.{variable}");
            assert!(candidates.iter().any(|candidate| candidate.label == label), "missing {label}");
        }

        for (expression, name, typed) in [
            ("recent_commits(3)", "recent_commits", "recent_commits(count: number(integer)) -> string[] | error"),
            (
                "ping_under(\"10.0.0.1\", 50)",
                "ping_under",
                "ping_under(address: ip-address, timeout: number, [attempts: number(integer)]) -> boolean | \"unstable\" | null | error",
            ),
            ("has_agentic_cli(\"kimi_code\")", "has_agentic_cli", "\"kimi-code\""),
            ("as_markdown(\"x\")", "as_markdown", "as_markdown(content: string) -> string | error"),
            ("has_binary(\"sh\")", "has_binary", "has_binary(name_or_path: string) -> boolean"),
        ] {
            assert!(
                candidates.iter().any(|candidate| candidate.insert_text == name
                    && candidate.detail.as_deref().is_some_and(|detail| detail.contains(typed))),
                "completion for {name} must carry `{typed}`"
            );
            let hover = hover_markdown(expression, 1, |_| None, |_| None);
            assert!(hover.contains(typed), "{hover}");
            assert!(!hover.contains("not implemented"), "hover must not evaluate: {hover}");
        }

        let recent = ctx_descriptor("recent_commits").unwrap();
        assert_eq!(function_descriptor("recent_commits").unwrap().description, recent.description);
    }

    #[test]
    fn literals_finds_simple_literal_and_excludes_expressions() {
        let text = "Hello {{{ name }}} and {{ title }}.";
        let found = literals(text, 0);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].content, " name ");
        assert_eq!(&text[found[0].outer.clone()],
            "{{{ name }}}"
        );
    }

    #[test]
    fn literals_inside_inline_code_are_found() {
        let text = "Code: `{{{ also_this }}}`";
        let found = literals(text, 0);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].content, " also_this ");
    }

    #[test]
    fn literals_body_base_filters_frontmatter_region() {
        let text = "---\ntitle: {{{ seed }}}\n---\n\n{{{ body_var }}}\n";
        let body_base = text.find("\n\n").unwrap() + 2;
        let found = literals(text, body_base);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].content, " body_var ");
    }

    #[test]
    fn literal_at_matches_cursor_on_braces_and_content() {
        let text = "See {{{ name }}}.";
        let literal = literal_at(text, 0, text.find("{{{").unwrap()).expect("on opening brace");
        assert_eq!(literal.content, " name ");

        let inside = literal_at(text, 0, text.find('n').unwrap()).expect("on content");
        assert_eq!(inside.content, " name ");

        let end = literal_at(text, 0, text.find("}}}.").unwrap() + 2).expect("on closing brace");
        assert_eq!(end.content, " name ");

        assert!(literal_at(text, 0, 2).is_none());
    }

    #[test]
    fn literal_containing_expression_is_inert() {
        let text = "{{{ {{ x }} }}}";
        let found = literals(text, 0);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].content, " {{ x }} ");
        assert!(interpolations(text, 0).is_empty());
    }

    #[test]
    fn interpolations_inside_fenced_code_are_not_surfaced() {
        // Semantic-token parity with diagnostics: a `{{ … }}` written inside a
        // fenced code block must not become a token.
        let text = "Real {{ shown }}\n\n```\n{{ hidden }}\n```\n";
        let found = interpolations(text, 0);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].text, "shown");
    }

    #[test]
    fn literals_inside_fenced_code_are_not_surfaced() {
        let text = "Real {{{ shown }}}\n\n```\n{{{ hidden }}}\n```\n";
        let found = literals(text, 0);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].content, " shown ");
    }

    #[test]
    fn unclosed_interpolation_yields_no_token() {
        // An unclosed or otherwise malformed construct emits nothing; guessing an
        // end span would make token output flicker while the user types.
        assert!(interpolations("Hello {{ title", 0).is_empty());
        assert!(interpolations("Hello {{ title }", 0).is_empty());
    }

    #[test]
    fn condition_dialect_lowers_logical_or_where_value_dialect_treats_it_as_fallback() {
        // Both dialects accept the `expression`-format corpus (`&&`/`||`), so
        // neither the spec example nor a `||` value is malformed. The dialects
        // differ in meaning: the value dialect (body `{{ }}`) reads `||` as
        // fallback, while the condition dialect (Expression-typed frontmatter
        // values, matching the schema format's `parse_condition`) lowers it to a
        // logical `or(...)`.
        assert_eq!(parse("a && b").unwrap().erase().to_string(), "and(a, b)");
        assert_eq!(parse_condition("a && b").unwrap().erase().to_string(), "and(a, b)");
        assert_eq!(parse("a || b").unwrap().erase().to_string(), "a || b");
        assert_eq!(parse_condition("a || b").unwrap().erase().to_string(), "or(a, b)");
        // The spec's own example parses cleanly in both dialects.
        assert!(parse(r#"is_agent() && os == "macos""#).is_ok());
        assert!(parse_condition(r#"is_agent() && os == "macos""#).is_ok());
    }

    #[test]
    fn condition_hover_uses_condition_grammar_body_hover_stays_value_grammar() {
        let no_fm = |_: &str| None;
        let no_schema = |_: &str| None;
        // Body (value dialect) hover renders `||` as fallback — the behavior this
        // change must preserve.
        let body = hover_markdown("a || b", 0, no_fm, no_schema);
        assert!(body.contains("`a || b`"), "{body}");
        // Frontmatter (condition dialect) hover lowers `||` to a logical `or(...)`.
        let cond = hover_markdown_condition("a || b", 0, no_fm, no_schema);
        assert!(cond.contains("`or(a, b)`"), "{cond}");
        // A function-call name inside a condition expression still enriches.
        let source = "is_empty(title) || is_string(title)";
        let fn_offset = source.find("is_empty").unwrap() + 2;
        let enriched = hover_markdown_condition(source, fn_offset, no_fm, no_schema);
        let is_empty = function_descriptor("is_empty").expect("`is_empty` is a known function");
        assert!(enriched.contains(&format_function_block(is_empty)), "{enriched}");
    }

    /// Dasherized identifiers: every DMLS cursor scan agrees with the lexer.
    mod dasherized_identifiers {
        use super::*;

        fn kebab_scalar(name: &str) -> Option<String> {
            match name {
                "spec-name" => Some("alpha".to_string()),
                "foo" => Some("fval".to_string()),
                "bar" => Some("bval".to_string()),
                _ => None,
            }
        }

        #[test]
        fn completion_partial_joins_a_kebab_name_at_every_cursor_position() {
            let text = "K: {{ spec-name }}";
            let start = text.find("spec").unwrap();
            for offset in start..=start + "spec-name".len() {
                let (token_start, partial) = completion_partial(text, offset).unwrap();
                assert_eq!(token_start, start, "offset {offset}");
                assert_eq!(partial, &text[start..offset], "offset {offset}");
            }
        }

        #[test]
        fn completion_partial_does_not_merge_non_joining_dashes() {
            for (text, partial) in [
                ("{{ foo--ba", "ba"),
                ("{{ foo--", ""),
                ("{{ a - b", "b"),
                ("{{ a -b", "b"),
                ("{{ a- b", "b"),
                ("{{ a -", ""),
                ("{{ 4-2", "2"),
            ] {
                let (token_start, found) = completion_partial(text, text.len()).unwrap();
                assert_eq!(found, partial, "for {text:?}");
                assert_eq!(token_start, text.len() - partial.len(), "for {text:?}");
            }
        }

        #[test]
        fn value_completion_partial_follows_the_same_rule() {
            for (value, partial) in [
                ("spec-na", "spec-na"),
                ("upper(doc.spec-", "doc.spec-"),
                ("is-draft && has-rev", "has-rev"),
                ("foo--ba", "ba"),
                ("a - b", "b"),
                ("ctx.to", "ctx.to"),
            ] {
                let (start, found) = value_completion_partial(value, 100);
                assert_eq!(found, partial, "for {value:?}");
                assert_eq!(start, 100 + value.len() - partial.len(), "for {value:?}");
            }
        }

        #[test]
        fn hover_resolves_a_kebab_name_at_every_offset_in_both_dialects() {
            let expression = "spec-name || foo";
            for offset in 0..="spec-name".len() {
                for markdown in [
                    hover_markdown(expression, offset, kebab_scalar, |_| None),
                    hover_markdown_condition(expression, offset, kebab_scalar, |_| None),
                ] {
                    assert!(
                        markdown.contains("from frontmatter `spec-name`"),
                        "offset {offset}: {markdown}"
                    );
                }
            }
        }

        #[test]
        fn hover_keeps_double_dash_operands_separate() {
            let expression = "foo--bar";
            let on_foo = hover_markdown(expression, 1, kebab_scalar, |_| None);
            assert!(on_foo.contains("from frontmatter `foo`"), "{on_foo}");
            let on_bar = hover_markdown(expression, expression.len() - 1, kebab_scalar, |_| None);
            assert!(on_bar.contains("from frontmatter `bar`"), "{on_bar}");
        }

        #[test]
        fn root_identifier_keeps_the_whole_kebab_path() {
            let expr = parse("doc.spec-name").unwrap();
            assert_eq!(root_identifier(&expr).as_deref(), Some("doc.spec-name"));
        }
    }
}
