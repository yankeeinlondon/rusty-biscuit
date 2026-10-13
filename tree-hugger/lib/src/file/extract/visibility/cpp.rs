use super::c::{has_linkage, is_namespace_scope, linkage_visibility};
use super::*;
use crate::file::parents::IndexedParent as _;

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
    ctx: &DeclarationContext<'_, '_>,
) -> Option<Visibility> {
    if let Some(member) = class_member(declaration) {
        // A member of an anonymous union or struct is a member of the
        // enclosing type, in the section that holds the anonymous member.
        if let Some(anonymous) = member
            .indexed_parent()
            .and_then(|list| list.indexed_parent())
            .and_then(anonymous_member_declaration)
        {
            return extract_cpp_visibility(anonymous, ctx);
        }
        return member_access(declaration, member, ctx);
    }
    (has_linkage(ctx.kind) && is_namespace_scope(declaration))
        .then(|| linkage_visibility(declaration, ctx, true))
}

/// The access section in force before `level` among the children of
/// `parent`: `None` when no `access_specifier` precedes it, `Some(level)`
/// for the nearest one (`Some(None)` when its text names no level). Each
/// parent's children are read once per file into [`VisibilityMemo`].
fn section_before(
    level: Node<'_>,
    parent: Node<'_>,
    ctx: &DeclarationContext<'_, '_>,
) -> Option<Option<Visibility>> {
    count_shared_context_steps(1);
    let mut sections = ctx.memo.cpp_sections.borrow_mut();
    let in_force = sections.entry(parent.id()).or_insert_with(|| {
        let mut in_force = HashMap::new();
        let mut nearest = None;
        let mut cursor = parent.walk();
        for sibling in parent.children(&mut cursor) {
            in_force.insert(sibling.id(), nearest);
            if sibling.kind() == "access_specifier" {
                nearest = Some(sibling.utf8_text(ctx.source.as_bytes()).ok().and_then(|text| {
                    match text.trim_end_matches(':').trim() {
                        "public" => Some(Visibility::Public),
                        "protected" => Some(Visibility::Protected),
                        "private" => Some(Visibility::Private),
                        _ => None,
                    }
                }));
            }
        }
        count_shared_context_steps(in_force.len());
        in_force
    });
    in_force.get(&level.id()).copied().flatten()
}

/// The node that sits directly in a class body for `declaration`: the
/// declaration itself, the `field_declaration` that wraps a nested type, or a
/// transparent wrapper (template header, preprocessor conditional) around
/// either.
fn class_member(declaration: Node<'_>) -> Option<Node<'_>> {
    let mut member = declaration;
    loop {
        let parent = member.indexed_parent()?;
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
/// conditional governs the members after it in that branch. Each level's
/// sections are indexed once per file ([`section_before`]).
fn member_access(
    declaration: Node<'_>,
    member: Node<'_>,
    ctx: &DeclarationContext<'_, '_>,
) -> Option<Visibility> {
    let mut level = declaration;
    loop {
        // The siblings before an `#else`/`#elif` branch are the branch it
        // replaces, so their sections never apply to it.
        let replaces_siblings = matches!(
            level.kind(),
            "preproc_else" | "preproc_elif" | "preproc_elifdef"
        );
        if !replaces_siblings
            && let Some(parent) = level.indexed_parent()
            && let Some(section) = section_before(level, parent, ctx)
        {
            return section;
        }
        if level == member {
            break;
        }
        level = level.indexed_parent()?;
    }

    match member.indexed_parent()?.indexed_parent()?.kind() {
        "class_specifier" => Some(Visibility::Private),
        "struct_specifier" | "union_specifier" => Some(Visibility::Public),
        _ => None,
    }
}
