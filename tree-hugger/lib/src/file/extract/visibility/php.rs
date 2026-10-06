use super::*;

/// Extracts the `public` / `protected` / `private` keyword of a PHP member;
/// `None` when it has none.
pub(crate) fn extract_php_visibility(node: Node<'_>, source: &str) -> Option<Visibility> {
    let modifier = find_child_by_kind(node, "visibility_modifier")?;
    match modifier.utf8_text(source.as_bytes()).ok()? {
        "public" => Some(Visibility::Public),
        "protected" => Some(Visibility::Protected),
        "private" => Some(Visibility::Private),
        _ => None,
    }
}

/// PHP visibility: an access keyword, else Public (members, including `var`
/// properties, default to public, and top-level functions and types are
/// global). Namespaces have no access level.
pub(super) fn extract_php_declaration_visibility(
    declaration: Node<'_>,
    ctx: &DeclarationContext<'_, '_>,
) -> Option<Visibility> {
    if matches!(
        declaration.kind(),
        "namespace_definition" | "namespace_use_declaration"
    ) {
        return None;
    }
    Some(extract_php_visibility(declaration, ctx.source).unwrap_or(Visibility::Public))
}
