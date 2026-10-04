use super::*;

/// Checks if a PHP method has the `static_modifier`.
pub(crate) fn extract_php_is_static(node: Node<'_>) -> bool {
    find_child_by_kind(node, "static_modifier").is_some()
}
