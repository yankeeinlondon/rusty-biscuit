use super::*;

/// Checks if a Rust function is an associated function: declared directly in
/// an `impl` or trait body, with no `self` receiver.
pub(crate) fn extract_rust_is_static(node: Node<'_>, source: &str) -> bool {
    let is_associated = node
        .parent()
        .filter(|list| list.kind() == "declaration_list")
        .and_then(|list| list.parent())
        .is_some_and(|parent| matches!(parent.kind(), "impl_item" | "trait_item"));
    if !is_associated {
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

