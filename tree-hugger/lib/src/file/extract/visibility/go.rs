use super::*;

/// Go visibility: a package-level identifier, field or method whose first
/// character is a Unicode uppercase letter is exported (Public); any other is
/// Package. The package clause has no access level.
pub(super) fn extract_go_visibility(
    declaration: Node<'_>,
    ctx: &DeclarationContext<'_, '_>,
) -> Option<Visibility> {
    if declaration.kind() == "package_clause" {
        return None;
    }
    let name = ctx.name.utf8_text(ctx.source.as_bytes()).ok()?;
    go_identifier_visibility(name)
}

/// The level Go's export rule gives an identifier.
pub(crate) fn go_identifier_visibility(name: &str) -> Option<Visibility> {
    let first = name.chars().next()?;
    Some(if first.is_uppercase() {
        Visibility::Public
    } else {
        Visibility::Package
    })
}
