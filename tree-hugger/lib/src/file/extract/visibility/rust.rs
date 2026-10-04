use super::*;

/// Extracts visibility from a Rust item's `pub` modifier.
pub(crate) fn extract_rust_visibility(node: Node<'_>, source: &str) -> Option<Visibility> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "visibility_modifier" {
            let text = child.utf8_text(source.as_bytes()).ok()?;
            if text.starts_with("pub") {
                return Some(Visibility::Public);
            }
        }
    }
    None
}
