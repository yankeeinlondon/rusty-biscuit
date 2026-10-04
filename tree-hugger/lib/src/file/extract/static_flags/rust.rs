use super::*;

/// Checks if a Rust method is an associated function (no self parameter).
pub(crate) fn extract_rust_is_static(node: Node<'_>, source: &str) -> bool {
    // A Rust method is "static" (associated function) if it doesn't have self parameter
    // Only applies to methods inside impl blocks
    if !is_inside_impl_block(node) {
        return false;
    }

    // Look for parameters node
    if let Some(params) = find_child_by_kind(node, "parameters") {
        let mut cursor = params.walk();
        for child in params.children(&mut cursor) {
            // Check for self_parameter
            if child.kind() == "self_parameter" {
                return false;
            }
            // Check for `self` in the first parameter
            if child.kind() == "parameter"
                && let Some(pattern) = find_child_by_kind(child, "identifier")
                && let Ok(text) = pattern.utf8_text(source.as_bytes())
                && text == "self"
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

/// Checks if a Rust function is inside an impl block.
pub(crate) fn is_inside_impl_block(node: Node<'_>) -> bool {
    let mut current = node.parent();
    while let Some(parent) = current {
        if parent.kind() == "impl_item" {
            return true;
        }
        current = parent.parent();
    }
    false
}
