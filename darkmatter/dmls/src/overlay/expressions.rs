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
    expression_function_descriptors, ExpressionFinder, ExpressionFunctionDescriptor, ParseError,
    SpannedExpr, SpannedExprKind, parse_spanned,
};
use darkmatter::markdown::span::SourceSpan;

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
        // `length` is fallible, so its typed signature carries the
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
}
