//! Rust `impl` blocks: the implemented type names the container of everything
//! inside the block, and a trait impl's members record the trait in
//! `relations.implements`.

use super::*;

/// The `impl_item` whose body directly declares `declaration`, if any.
pub(crate) fn impl_of_member(declaration: Node<'_>) -> Option<Node<'_>> {
    declaration
        .parent()
        .filter(|list| list.kind() == "declaration_list")?
        .parent()
        .filter(|parent| parent.kind() == "impl_item")
}

/// The implemented type's name: the last path segment of the self type,
/// without generic arguments (`impl<T> crate::geo::Wrapper<T>` → `Wrapper`).
pub(crate) fn impl_self_type_name(impl_item: Node<'_>, source: &str) -> Option<String> {
    last_path_segment(impl_item.child_by_field_name("type")?, source)
}

/// The implemented trait's name, by the same rule as the self type
/// (`impl fmt::Display for Point` → `Display`); `None` for an inherent impl.
pub(crate) fn impl_trait_name(impl_item: Node<'_>, source: &str) -> Option<String> {
    last_path_segment(impl_item.child_by_field_name("trait")?, source)
}

fn last_path_segment(node: Node<'_>, source: &str) -> Option<String> {
    match node.kind() {
        "generic_type" | "reference_type" | "pointer_type" => {
            last_path_segment(node.child_by_field_name("type")?, source)
        }
        "scoped_type_identifier" | "scoped_identifier" => {
            last_path_segment(node.child_by_field_name("name")?, source)
        }
        _ => {
            let text = node.utf8_text(source.as_bytes()).ok()?;
            let without_generics = text.split('<').next().unwrap_or(text);
            let segment = without_generics.rsplit("::").next()?.trim();
            (!segment.is_empty()).then(|| segment.to_string())
        }
    }
}
