use super::*;
use crate::file::parents::IndexedParent as _;

/// Checks if a Rust item is a static member: declared directly in an `impl`
/// or trait body, and either an associated `const` (always used through the
/// type) or a function with no `self` receiver.
pub(crate) fn extract_rust_is_static(node: Node<'_>) -> bool {
    let is_associated = node
        .indexed_parent()
        .filter(|list| list.kind() == "declaration_list")
        .and_then(|list| list.indexed_parent())
        .is_some_and(|parent| matches!(parent.kind(), "impl_item" | "trait_item"));
    if !is_associated {
        return false;
    }
    if node.kind() == "const_item" {
        return true;
    }

    // Look for parameters node
    if let Some(params) = find_child_by_kind(node, "parameters") {
        let mut cursor = params.walk();
        for child in params.children(&mut cursor) {
            // Check for self_parameter
            if child.kind() == "self_parameter" {
                return false;
            }
            // A typed receiver (`self: Box<Self>`) is a `parameter` whose
            // pattern is the `self` keyword node, not an identifier.
            if child.kind() == "parameter"
                && child
                    .child_by_field_name("pattern")
                    .is_some_and(|pattern| pattern.kind() == "self")
            {
                return false;
            }
        }
        // Has parameters but no self -> associated function
        return true;
    }

    // No parameters at all means it's an associated function
    true
}

