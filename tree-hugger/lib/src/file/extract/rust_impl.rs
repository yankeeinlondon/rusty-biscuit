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

/// The self type's path as written, one entry per segment, without generic
/// arguments (`impl<T> right::Wrapper<T>` → `["right", "Wrapper"]`), for the
/// qualified name. Path keywords (`crate`, `self`, `super`) are kept for the
/// caller to resolve. A self type that is not a path (tuple, array,
/// qualified `<T as Trait>` path) is its last segment alone, as in
/// [`impl_self_type_name`], which stays the bare container name.
pub(crate) fn impl_self_type_path(impl_item: Node<'_>, source: &str) -> Option<Vec<String>> {
    let self_type = impl_item.child_by_field_name("type")?;
    path_segments(self_type, source)
        .filter(|segments| !segments.is_empty())
        .or_else(|| last_path_segment(self_type, source).map(|segment| vec![segment]))
}

fn path_segments(node: Node<'_>, source: &str) -> Option<Vec<String>> {
    match node.kind() {
        "generic_type" | "reference_type" | "pointer_type" => {
            path_segments(node.child_by_field_name("type")?, source)
        }
        "scoped_type_identifier" | "scoped_identifier" => {
            let mut segments = match node.child_by_field_name("path") {
                Some(path) => path_segments(path, source)?,
                None => Vec::new(),
            };
            segments.extend(path_segments(node.child_by_field_name("name")?, source)?);
            Some(segments)
        }
        "type_identifier" | "identifier" | "primitive_type" | "crate" | "self" | "super" => {
            let text = node.utf8_text(source.as_bytes()).ok()?.trim();
            (!text.is_empty()).then(|| vec![text.to_string()])
        }
        _ => None,
    }
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
