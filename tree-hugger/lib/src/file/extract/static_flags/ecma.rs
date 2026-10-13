use super::*;

/// Checks if a TypeScript/JavaScript method has the `static` keyword.
pub(crate) fn extract_ts_is_static(node: Node<'_>) -> bool {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "static" {
            return true;
        }
    }
    false
}
