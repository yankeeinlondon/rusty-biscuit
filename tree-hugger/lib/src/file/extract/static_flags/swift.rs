use super::*;

/// Checks if a Swift member is `static` or `class`. For a property the keyword
/// sits in `modifiers`; `class func` puts a bare `class` token on the function.
pub(crate) fn extract_swift_is_static(node: Node<'_>, source: &str) -> bool {
    let mut cursor = node.walk();
    node.children(&mut cursor).any(|child| match child.kind() {
        "modifiers" => {
            let mut mod_cursor = child.walk();
            child.children(&mut mod_cursor).any(|modifier| {
                matches!(
                    modifier.utf8_text(source.as_bytes()),
                    Ok("static" | "class")
                )
            })
        }
        "class" => node.kind() == "function_declaration",
        _ => false,
    })
}
