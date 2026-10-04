use super::*;

/// Extracts visibility from TypeScript/JavaScript method_definition.
///
/// TypeScript AST structure has accessibility_modifier as a child:
/// ```text
/// method_definition
///   accessibility_modifier (public/protected/private)
///   property_identifier
///   formal_parameters
///   ...
/// ```
pub(crate) fn extract_ts_visibility(node: Node<'_>, source: &str) -> Option<Visibility> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "accessibility_modifier" {
            let text = child.utf8_text(source.as_bytes()).ok()?;
            return match text {
                "public" => Some(Visibility::Public),
                "protected" => Some(Visibility::Protected),
                "private" => Some(Visibility::Private),
                _ => None,
            };
        }
    }
    None
}
