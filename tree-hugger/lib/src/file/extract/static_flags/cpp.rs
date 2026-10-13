use super::*;
use crate::file::parents::IndexedParent as _;

/// Checks if a C++ method has the `static` specifier.
pub(crate) fn extract_cpp_is_static(node: Node<'_>, source: &str) -> bool {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "storage_class_specifier"
            && let Ok(text) = child.utf8_text(source.as_bytes())
            && text == "static"
        {
            return true;
        }
    }
    false
}

/// Whether a C++ declaration sits directly in a class, struct or union body,
/// looking through template headers and preprocessor conditionals.
pub(super) fn is_class_member(declaration: Node<'_>) -> bool {
    member_slot(declaration)
        .indexed_parent()
        .is_some_and(|parent| parent.kind() == "field_declaration_list")
}
