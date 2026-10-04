use super::*;

/// Extracts visibility from C++ method declarations.
///
/// C++ visibility is handled via access specifiers in the class (public:, private:, etc.)
/// not as part of the method itself. For now, we check for inline access specifiers.
pub(crate) fn extract_cpp_visibility(node: Node<'_>, source: &str) -> Option<Visibility> {
    // C++ uses access specifiers at the section level, not per-method
    // Check if there's an access_specifier sibling before this node
    let mut prev = node.prev_sibling();
    while let Some(sibling) = prev {
        if sibling.kind() == "access_specifier" {
            let text = sibling.utf8_text(source.as_bytes()).ok()?;
            return match text.trim_end_matches(':') {
                "public" => Some(Visibility::Public),
                "protected" => Some(Visibility::Protected),
                "private" => Some(Visibility::Private),
                _ => None,
            };
        }
        // Stop if we hit another method or declaration
        if sibling.kind() == "function_definition"
            || sibling.kind() == "declaration"
            || sibling.kind() == "field_declaration"
        {
            break;
        }
        prev = sibling.prev_sibling();
    }
    None
}
