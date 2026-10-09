use super::*;
use crate::file::parents::IndexedParent as _;

/// Extracts the `public` / `protected` / `private` keyword from a Java
/// declaration's `modifiers`; `None` when it has none.
pub(crate) fn extract_java_visibility(node: Node<'_>, source: &str) -> Option<Visibility> {
    let modifiers = find_child_by_kind(node, "modifiers")?;
    let mut cursor = modifiers.walk();
    modifiers
        .children(&mut cursor)
        .find_map(|child| match child.utf8_text(source.as_bytes()).ok()? {
            "public" => Some(Visibility::Public),
            "protected" => Some(Visibility::Protected),
            "private" => Some(Visibility::Private),
            _ => None,
        })
}

/// Java visibility: an access keyword, else Public for an enum constant and
/// for a member declared directly in an interface (methods, constants and
/// nested types alike), Private for an enum's constructor, else Package. The
/// `package` clause itself has no access level.
pub(super) fn extract_java_declaration_visibility(
    declaration: Node<'_>,
    ctx: &DeclarationContext<'_, '_>,
) -> Option<Visibility> {
    match declaration.kind() {
        "package_declaration" => return None,
        "enum_constant" => return Some(Visibility::Public),
        _ => {}
    }
    if let Some(level) = extract_java_visibility(declaration, ctx.source) {
        return Some(level);
    }
    // An enum constructor with no access keyword is private (JLS 8.9.2).
    if declaration.kind() == "constructor_declaration"
        && declaration
            .indexed_parent()
            .is_some_and(|parent| parent.kind() == "enum_body_declarations")
    {
        return Some(Visibility::Private);
    }
    let in_interface_body = declaration.indexed_parent().is_some_and(|parent| {
        matches!(parent.kind(), "interface_body" | "annotation_type_body")
    });
    Some(if in_interface_body {
        Visibility::Public
    } else {
        Visibility::Package
    })
}
