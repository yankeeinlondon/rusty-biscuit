use super::*;
use crate::file::parents::IndexedParent as _;

/// Reads a Swift declaration's access keyword: `open` and `public` are Public,
/// `package` Package, `internal` Internal, `fileprivate` ModulePrivate and
/// `private` Private. A setter-only modifier (`private(set)`) restricts
/// writes only, so it never sets the level. `None` when no keyword applies.
pub(crate) fn extract_swift_visibility(node: Node<'_>, source: &str) -> Option<Visibility> {
    let modifiers = find_child_by_kind(node, "modifiers")?;
    let mut cursor = modifiers.walk();
    modifiers
        .children(&mut cursor)
        .filter(|modifier| modifier.kind() == "visibility_modifier")
        .find_map(|modifier| match modifier.utf8_text(source.as_bytes()).ok()? {
            "open" | "public" => Some(Visibility::Public),
            "package" => Some(Visibility::Package),
            "internal" => Some(Visibility::Internal),
            "fileprivate" => Some(Visibility::ModulePrivate),
            "private" => Some(Visibility::Private),
            _ => None,
        })
}

/// Swift visibility: the declaration's own access keyword, else a default.
///
/// - an enum case or protocol requirement inherits its container's level;
/// - any other member takes the default of the nearest enclosing type or
///   extension with an explicit keyword: ModulePrivate inside a `private` or
///   `fileprivate` type, the extension's level inside an extension (a
///   `private extension` behaves as `fileprivate`), Internal inside a
///   `public` or `open` type;
/// - everything else is Internal.
pub(super) fn extract_swift_declaration_visibility(
    declaration: Node<'_>,
    ctx: &DeclarationContext<'_, '_>,
) -> Option<Visibility> {
    if let Some(level) = extract_swift_visibility(declaration, ctx.source) {
        return Some(level);
    }

    let Some(container) = enclosing_type(declaration) else {
        return Some(Visibility::Internal);
    };
    if matches!(
        declaration.kind(),
        "enum_entry" | "protocol_function_declaration" | "protocol_property_declaration"
    ) {
        return extract_swift_declaration_visibility(container, ctx);
    }

    Some(member_default(container, ctx.source))
}

fn member_default(container: Node<'_>, source: &str) -> Visibility {
    let mut current = Some(container);
    while let Some(node) = current {
        if let Some(level) = extract_swift_visibility(node, source) {
            let is_extension = node
                .child_by_field_name("declaration_kind")
                .and_then(|kind| kind.utf8_text(source.as_bytes()).ok())
                == Some("extension");
            return match level {
                // A top-level `private` behaves as `fileprivate`.
                Visibility::Private | Visibility::ModulePrivate => Visibility::ModulePrivate,
                level if is_extension => level,
                _ => Visibility::Internal,
            };
        }
        current = enclosing_type(node);
    }
    Visibility::Internal
}

/// The type, extension or protocol whose body directly declares
/// `declaration`.
fn enclosing_type(declaration: Node<'_>) -> Option<Node<'_>> {
    let body = declaration.indexed_parent()?;
    if !matches!(body.kind(), "class_body" | "enum_class_body" | "protocol_body") {
        return None;
    }
    body.indexed_parent()
        .filter(|owner| matches!(owner.kind(), "class_declaration" | "protocol_declaration"))
}
