use super::c::{has_linkage, is_namespace_scope, linkage_visibility};
use super::*;

/// C++ visibility.
///
/// A class, struct or union member takes the nearest preceding access section
/// (`public:` / `protected:` / `private:`), else its class key's default:
/// Private for `class`, Public for `struct` and `union`. A namespace-scope
/// function or object follows the linkage rule shared with C. Everything else
/// (namespaces, namespace-scope types, aliases, enumerators, macros) has no
/// access modifier and reports Unknown.
pub(super) fn extract_cpp_visibility(
    declaration: Node<'_>,
    ctx: &DeclarationContext<'_>,
) -> Option<Visibility> {
    if let Some(member) = class_member(declaration) {
        return member_access(member, ctx.source);
    }
    (has_linkage(ctx.kind) && is_namespace_scope(declaration))
        .then(|| linkage_visibility(declaration, ctx, true))
}

/// The node that sits directly in a class body for `declaration`: the
/// declaration itself, or the `field_declaration` / `template_declaration`
/// that wraps a nested type or member template.
fn class_member(declaration: Node<'_>) -> Option<Node<'_>> {
    let mut member = declaration;
    loop {
        let parent = member.parent()?;
        match parent.kind() {
            "field_declaration_list" => return Some(member),
            "field_declaration" | "template_declaration" => member = parent,
            _ => return None,
        }
    }
}

fn member_access(member: Node<'_>, source: &str) -> Option<Visibility> {
    let mut previous = member.prev_sibling();
    while let Some(sibling) = previous {
        if sibling.kind() == "access_specifier" {
            let text = sibling.utf8_text(source.as_bytes()).ok()?;
            return match text.trim_end_matches(':').trim() {
                "public" => Some(Visibility::Public),
                "protected" => Some(Visibility::Protected),
                "private" => Some(Visibility::Private),
                _ => None,
            };
        }
        previous = sibling.prev_sibling();
    }

    match member.parent()?.parent()?.kind() {
        "class_specifier" => Some(Visibility::Private),
        "struct_specifier" | "union_specifier" => Some(Visibility::Public),
        _ => None,
    }
}
