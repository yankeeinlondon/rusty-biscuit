use super::*;

/// Checks if a Swift method has `static` or `class` modifier.
pub(crate) fn extract_swift_is_static(node: Node<'_>, source: &str) -> bool {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "modifiers" {
            let mut mod_cursor = child.walk();
            for modifier in child.children(&mut mod_cursor) {
                if let Ok(text) = modifier.utf8_text(source.as_bytes())
                    && (text == "static" || text == "class")
                {
                    return true;
                }
            }
        }
    }
    false
}
