use super::*;

/// Checks if a Scala method is inside an object (companion object).
pub(crate) fn extract_scala_is_static(node: Node<'_>) -> bool {
    let mut current = node.parent();
    while let Some(parent) = current {
        if parent.kind() == "object_definition" {
            return true;
        }
        // Stop if we hit a class definition
        if parent.kind() == "class_definition" {
            return false;
        }
        current = parent.parent();
    }
    false
}
