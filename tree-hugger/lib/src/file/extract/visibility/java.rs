use super::*;

/// Extracts visibility from Java method_declaration.
///
/// Java AST structure has modifiers containing visibility:
/// ```text
/// method_declaration
///   modifiers
///     public/protected/private
///   type_identifier
///   identifier
///   ...
/// ```
///
/// In Java, interface members are implicitly public and cannot have explicit
/// visibility modifiers. This function infers `Public` for members declared
/// inside an interface.
pub(crate) fn extract_java_visibility(node: Node<'_>, source: &str) -> Option<Visibility> {
    if let Some(modifiers) = find_child_by_kind(node, "modifiers") {
        let mut cursor = modifiers.walk();
        for child in modifiers.children(&mut cursor) {
            let text = child.utf8_text(source.as_bytes()).ok()?;
            match text {
                "public" => return Some(Visibility::Public),
                "protected" => return Some(Visibility::Protected),
                "private" => return Some(Visibility::Private),
                _ => continue,
            }
        }
    }

    // Java interface members are implicitly public
    if is_inside_interface(node) {
        return Some(Visibility::Public);
    }

    None
}
