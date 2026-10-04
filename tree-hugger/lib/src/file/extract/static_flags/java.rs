use super::*;

/// Checks if a Java or C# member is static: it has the `static` modifier, or
/// the language makes it static without one: a Java interface constant
/// (implicitly `public static final`) or a C# `const` field.
pub(crate) fn extract_java_csharp_is_static(node: Node<'_>, source: &str) -> bool {
    if node.kind() == "constant_declaration" {
        return true;
    }

    // Java: modifiers child containing "static"
    if let Some(modifiers) = find_child_by_kind(node, "modifiers") {
        let mut cursor = modifiers.walk();
        for child in modifiers.children(&mut cursor) {
            if let Ok(text) = child.utf8_text(source.as_bytes())
                && text == "static"
            {
                return true;
            }
        }
    }

    // C#: direct modifier child containing "static"
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "modifier"
            && let Ok(text) = child.utf8_text(source.as_bytes())
            && matches!(text, "static" | "const")
        {
            return true;
        }
    }

    false
}
