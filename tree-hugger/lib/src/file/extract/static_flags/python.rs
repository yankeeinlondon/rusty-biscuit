use super::*;

/// Checks if a Python method has @staticmethod or @classmethod decorator.
pub(crate) fn extract_python_is_static(node: Node<'_>, source: &str) -> bool {
    // Look at the decorated_definition parent if exists
    if let Some(parent) = node.parent()
        && parent.kind() == "decorated_definition"
    {
        let mut cursor = parent.walk();
        for child in parent.children(&mut cursor) {
            if child.kind() == "decorator"
                && let Ok(text) = child.utf8_text(source.as_bytes())
                && (text.contains("staticmethod") || text.contains("classmethod"))
            {
                return true;
            }
        }
    }
    false
}
