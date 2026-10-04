use super::*;

/// Extracts visibility from Swift function declarations.
pub(crate) fn extract_swift_visibility(node: Node<'_>, source: &str) -> Option<Visibility> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        let kind = child.kind();
        if kind == "modifiers" {
            let mut mod_cursor = child.walk();
            for modifier in child.children(&mut mod_cursor) {
                let text = modifier.utf8_text(source.as_bytes()).ok()?;
                match text {
                    "public" => return Some(Visibility::Public),
                    "internal" => return Some(Visibility::Internal),
                    "private" => return Some(Visibility::Private),
                    "fileprivate" => return Some(Visibility::Private),
                    _ => continue,
                }
            }
        }
    }
    None
}
