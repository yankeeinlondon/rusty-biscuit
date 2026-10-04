use super::*;

/// Extracts visibility from C# method_declaration.
///
/// In C#, interface members are implicitly public and cannot have explicit
/// visibility modifiers. This function infers `Public` for members declared
/// inside an interface.
pub(crate) fn extract_csharp_visibility(node: Node<'_>, source: &str) -> Option<Visibility> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "modifier" {
            let text = child.utf8_text(source.as_bytes()).ok()?;
            match text {
                "public" => return Some(Visibility::Public),
                "protected" => return Some(Visibility::Protected),
                "private" => return Some(Visibility::Private),
                "internal" => return Some(Visibility::Internal),
                _ => continue,
            }
        }
    }

    // C# interface members are implicitly public
    if is_inside_interface(node) {
        return Some(Visibility::Public);
    }

    None
}
