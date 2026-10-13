use super::*;
use crate::file::parents::IndexedParent as _;

/// Checks if a PHP member is static: it has the `static_modifier`, or it is a
/// class, interface, trait or enum constant (`const X = 1;`), which belongs
/// to the type. A top-level `const` is not a member.
pub(crate) fn extract_php_is_static(node: Node<'_>) -> bool {
    if node.kind() == "const_declaration" {
        return node
            .indexed_parent()
            .is_some_and(|body| matches!(body.kind(), "declaration_list" | "enum_declaration_list"));
    }
    find_child_by_kind(node, "static_modifier").is_some()
}
