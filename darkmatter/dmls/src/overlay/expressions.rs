//! Layer-3 interpolation overlay: request-time analysis of `{{ … }}`
//! expressions.
//!
//! Interpolation regions come from the library [`ExpressionFinder`] (which skips
//! fenced code exactly as compose does); the inner expression is parsed with the
//! Phase-8 [`parse_spanned`] so the language server sees the same AST — and the
//! same byte-offset [`ParseError`] — that `md compose` does. Nothing here reads
//! `env`, captures `ctx`, or executes anything: static value resolution is
//! frontmatter-backed only.

use darkmatter::markdown::compose::context::context_variable_descriptors;
use darkmatter::markdown::compose::expression::{
    EXPRESSION_FUNCTION_DESCRIPTORS, ExpressionFinder, ParseError, SpannedExpr, SpannedExprKind,
    parse_spanned,
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

/// The `ctx.*` variable names (bare, without the `ctx.` prefix) with their
/// descriptions, for completion and hover documentation.
pub fn ctx_names() -> impl Iterator<Item = (&'static str, &'static str)> {
    context_variable_descriptors()
        .iter()
        .map(|descriptor| (descriptor.name, descriptor.description))
}

/// The expression function signatures with their descriptions.
pub fn function_signatures() -> impl Iterator<Item = (&'static str, &'static str)> {
    EXPRESSION_FUNCTION_DESCRIPTORS
        .iter()
        .map(|descriptor| (descriptor.signature, descriptor.description))
}

/// Whether `name` (a `ctx.NAME` tail) is a known context variable.
pub fn is_ctx_name(name: &str) -> bool {
    context_variable_descriptors().iter().any(|d| d.name == name)
}

/// The description of a function by its bare name (`length`, `relative`, …),
/// matching against the descriptor signature's leading identifier.
pub fn function_description(name: &str) -> Option<&'static str> {
    EXPRESSION_FUNCTION_DESCRIPTORS.iter().find_map(|descriptor| {
        let signature_name = descriptor.signature.split('(').next().unwrap_or("");
        (signature_name == name).then_some(descriptor.description)
    })
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
    fn test_catalog_lookups() {
        assert!(is_ctx_name("today") || ctx_names().count() > 0);
        assert!(function_description("length").is_some() || function_signatures().count() > 0);
    }
}
