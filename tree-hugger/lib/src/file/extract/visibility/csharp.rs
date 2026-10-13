use super::*;
use crate::file::parents::IndexedParent as _;

/// Reads the C# access keywords on a declaration, combining
/// `protected internal` and `private protected`; `file` is ModulePrivate.
/// `None` when the declaration has none. Accessor-level modifiers
/// (`{ get; private set; }`) are not children of the declaration, so they
/// never set its level.
pub(crate) fn extract_csharp_visibility(node: Node<'_>, source: &str) -> Option<Visibility> {
    let (mut public, mut protected, mut private, mut internal, mut file) =
        (false, false, false, false, false);
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() != "modifier" {
            continue;
        }
        match child.utf8_text(source.as_bytes()).ok()? {
            "public" => public = true,
            "protected" => protected = true,
            "private" => private = true,
            "internal" => internal = true,
            "file" => file = true,
            _ => {}
        }
    }

    match (public, protected, private, internal, file) {
        (true, ..) => Some(Visibility::Public),
        (_, true, _, true, _) => Some(Visibility::ProtectedInternal),
        (_, true, true, ..) => Some(Visibility::PrivateProtected),
        (_, true, ..) => Some(Visibility::Protected),
        (_, _, true, ..) => Some(Visibility::Private),
        (_, _, _, true, _) => Some(Visibility::Internal),
        (.., true) => Some(Visibility::ModulePrivate),
        _ => None,
    }
}

/// C# visibility: access keywords first, then the defaults of the C#
/// reference. An explicit interface implementation is Private; any member of
/// an interface (default-implemented, static and nested types included) and
/// every enum member is Public; a class, struct or record member is Private; a top-level type is
/// Internal. Namespaces have no access level. The owner is read from the
/// [`member_slot`], so `#if`/`#elif`/`#else` around a declaration never hide
/// it.
pub(super) fn extract_csharp_declaration_visibility(
    declaration: Node<'_>,
    ctx: &DeclarationContext<'_, '_>,
) -> Option<Visibility> {
    match declaration.kind() {
        "namespace_declaration" | "file_scoped_namespace_declaration" => return None,
        // Enum members take no access keyword and are always public members.
        "enum_member_declaration" => return Some(Visibility::Public),
        _ => {}
    }
    if let Some(level) = extract_csharp_visibility(declaration, ctx.source) {
        return Some(level);
    }
    if find_child_by_kind(declaration, "explicit_interface_specifier").is_some() {
        return Some(Visibility::Private);
    }

    let parent = member_slot(declaration).indexed_parent()?;
    let owner = match parent.kind() {
        "compilation_unit" => return Some(Visibility::Internal),
        "declaration_list" => parent.indexed_parent()?,
        _ => return None,
    };
    match owner.kind() {
        "namespace_declaration" => Some(Visibility::Internal),
        "interface_declaration" => Some(Visibility::Public),
        "class_declaration" | "struct_declaration" | "record_declaration"
        | "record_struct_declaration" => Some(Visibility::Private),
        _ => None,
    }
}
