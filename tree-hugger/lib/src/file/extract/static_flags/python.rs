use super::*;

/// Checks if a Python member is static: a method decorated with
/// `@staticmethod` or `@classmethod`, or a class attribute (see
/// [`is_python_class_attribute`]).
pub(crate) fn extract_python_is_static(node: Node<'_>, source: &str) -> bool {
    if node.kind() == "assignment" {
        return is_python_class_attribute(node, source);
    }

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

/// Whether a class-body assignment declares a class attribute rather than an
/// instance field: `x: ClassVar[T]` (with or without a value) and unannotated
/// `x = v` are class attributes; `x: T` and `x: T = v` are instance fields
/// (PEP 526, dataclass semantics).
pub(crate) fn is_python_class_attribute(assignment: Node<'_>, source: &str) -> bool {
    let in_class_body = assignment
        .parent()
        .filter(|statement| statement.kind() == "expression_statement")
        .and_then(|statement| statement.parent())
        .filter(|block| block.kind() == "block")
        .and_then(|block| block.parent())
        .is_some_and(|owner| owner.kind() == "class_definition");
    if !in_class_body {
        return false;
    }

    match assignment.child_by_field_name("type") {
        None => true,
        Some(annotation) => annotation
            .utf8_text(source.as_bytes())
            .map(|text| {
                let text = text.trim();
                let unqualified = text.strip_prefix("typing.").unwrap_or(text);
                unqualified == "ClassVar" || unqualified.starts_with("ClassVar[")
            })
            .unwrap_or(false),
    }
}
