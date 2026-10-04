use super::c::{has_linkage, is_namespace_scope, linkage_visibility};
use super::*;

/// C++ visibility.
///
/// A class, struct or union member takes the nearest preceding access section
/// (`public:` / `protected:` / `private:`), else its class key's default:
/// Private for `class`, Public for `struct` and `union`. A member of an
/// anonymous union or struct takes the access of that anonymous member. A namespace-scope
/// function or object follows the linkage rule shared with C. Everything else
/// (namespaces, namespace-scope types, aliases, enumerators, macros) has no
/// access modifier and reports Unknown.
pub(super) fn extract_cpp_visibility(
    declaration: Node<'_>,
    ctx: &DeclarationContext<'_>,
) -> Option<Visibility> {
    if let Some(member) = class_member(declaration) {
        // A member of an anonymous union or struct is a member of the
        // enclosing type, in the section that holds the anonymous member.
        if let Some(anonymous) = member
            .parent()
            .and_then(|list| list.parent())
            .and_then(anonymous_member_declaration)
        {
            return extract_cpp_visibility(anonymous, ctx);
        }
        return member_access(declaration, member, ctx.source);
    }
    (has_linkage(ctx.kind) && is_namespace_scope(declaration))
        .then(|| linkage_visibility(declaration, ctx, true))
}

/// The node that sits directly in a class body for `declaration`: the
/// declaration itself, the `field_declaration` that wraps a nested type, or a
/// transparent wrapper (template header, preprocessor conditional) around
/// either.
fn class_member(declaration: Node<'_>) -> Option<Node<'_>> {
    let mut member = declaration;
    loop {
        let parent = member.parent()?;
        match parent.kind() {
            "field_declaration_list" => return Some(member),
            "field_declaration" => member = parent,
            kind if is_transparent_wrapper_kind(kind) => member = parent,
            _ => return None,
        }
    }
}

/// The nearest access section before `declaration`, searched at every level
/// from the declaration out to `member`, so a section inside a preprocessor
/// conditional governs the members after it in that branch.
fn member_access(declaration: Node<'_>, member: Node<'_>, source: &str) -> Option<Visibility> {
    let mut level = declaration;
    loop {
        // The siblings before an `#else`/`#elif` branch are the branch it
        // replaces, so their sections never apply to it.
        let mut previous = if matches!(
            level.kind(),
            "preproc_else" | "preproc_elif" | "preproc_elifdef"
        ) {
            None
        } else {
            level.prev_sibling()
        };
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
        if level == member {
            break;
        }
        level = level.parent()?;
    }

    match member.parent()?.parent()?.kind() {
        "class_specifier" => Some(Visibility::Private),
        "struct_specifier" | "union_specifier" => Some(Visibility::Public),
        _ => None,
    }
}
