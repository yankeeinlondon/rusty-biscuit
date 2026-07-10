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
    EXPRESSION_FUNCTION_DESCRIPTORS, ExpressionFinder, ExpressionFunctionDescriptor, ParseError,
    SpannedExpr, SpannedExprKind, parse_spanned,
};
use darkmatter::markdown::span::SourceSpan;

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

/// Parses an interpolation's inner expression, span-carrying.
///
/// ## Errors
///
/// Returns the [`ParseError`] (with a byte-offset `position` into the
/// expression text) exactly as the compose parser would.
pub fn parse(expression: &str) -> Result<SpannedExpr, ParseError> {
    parse_spanned(expression)
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

/// The name of the deepest [`SpannedExprKind::FunctionCall`] whose span
/// contains `offset` (a byte offset into the expression text), if any.
///
/// The D5 cursor-to-function resolver: hovering anywhere inside a call — the
/// name, parentheses, or an argument — resolves to the innermost enclosing
/// call, so nested calls like `as_csv(length(x))` answer for the inner call
/// when the cursor sits inside it.
pub fn function_call_at(expr: &SpannedExpr, offset: usize) -> Option<&str> {
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
        SpannedExprKind::FunctionCall { args, .. } => args.iter().collect(),
        _ => Vec::new(),
    };
    for child in children {
        if let Some(name) = function_call_at(child, offset) {
            return Some(name);
        }
    }
    match &expr.kind {
        SpannedExprKind::FunctionCall { name, .. } => Some(name),
        _ => None,
    }
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
    let token_start = before
        .rfind(|c: char| !(c.is_alphanumeric() || c == '_' || c == '.'))
        .map(|index| index + 1)
        .unwrap_or(0)
        .max(open + 2);
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
    EXPRESSION_FUNCTION_DESCRIPTORS
}

/// The expression-function descriptor whose bare name (the leading identifier
/// of its `signature`) is `name`, if any. Overloads share a bare name; the
/// first catalog entry wins, matching the pre-descriptor lookup order.
pub fn function_descriptor(name: &str) -> Option<&'static ExpressionFunctionDescriptor> {
    EXPRESSION_FUNCTION_DESCRIPTORS
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
pub fn format_function_block(descriptor: &ExpressionFunctionDescriptor) -> String {
    format!(
        "**`{}`**\n\n{}",
        descriptor.typed_signature(),
        descriptor.description
    )
}

#[cfg(test)]
mod tests {
    use super::*;

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
        // `length` is fallible (`R_NUM_ERR`), so its typed signature carries the
        // `| error` union suffix that the untyped `signature` lacks.
        let length = function_descriptor("length").expect("`length` is a known function");
        assert!(length.signature.contains("length("));
        assert!(!length.signature.contains("| error"));
        assert!(length.typed_signature().ends_with("| error"));
    }

    #[test]
    fn test_function_call_at_finds_deepest_call() {
        let source = "as_csv(length(items))";
        let expr = parse(source).unwrap();
        let inner = source.find("length").unwrap();
        // Inside the nested call (name or argument) the inner call wins.
        assert_eq!(function_call_at(&expr, inner + 1), Some("length"));
        assert_eq!(function_call_at(&expr, source.find("items").unwrap()), Some("length"));
        // Inside the outer name, only the outer call contains the offset.
        assert_eq!(function_call_at(&expr, 1), Some("as_csv"));
        // Outside the expression entirely, and on a non-call expression: none.
        assert_eq!(function_call_at(&expr, source.len() + 5), None);
        let plain = parse("title").unwrap();
        assert_eq!(function_call_at(&plain, 2), None);
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
}
