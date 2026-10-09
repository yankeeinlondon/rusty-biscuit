use super::*;
use crate::file::parents::IndexedParent as _;

/// Checks if a Python member is static: a method declared directly in a class
/// body and decorated with `@staticmethod` or `@classmethod`, or a class
/// attribute (see [`is_python_class_attribute`]).
pub(crate) fn extract_python_is_static(node: Node<'_>, source: &str) -> bool {
    if node.kind() == "assignment" {
        return is_python_class_attribute(node, source);
    }

    let slot = member_slot(node);
    if slot.kind() != "decorated_definition" || !is_in_class_body(slot) {
        return false;
    }
    let mut cursor = slot.walk();
    slot.children(&mut cursor)
        .filter(|child| child.kind() == "decorator")
        .any(|decorator| is_static_decorator(decorator, source))
}

/// Whether a decorator is the builtin `staticmethod` or `classmethod` itself,
/// bare or qualified as `builtins.…`. The decorator's expression is matched,
/// not its text: `@notstaticmethod`, `@staticmethod_helper`,
/// `@other.staticmethod` and a call such as `@wrap(staticmethod)` or
/// `@staticmethod()` apply some other callable.
fn is_static_decorator(decorator: Node<'_>, source: &str) -> bool {
    let text = |node: Node<'_>| node.utf8_text(source.as_bytes()).unwrap_or_default();
    let is_builtin_name = |node: Node<'_>| {
        node.kind() == "identifier" && matches!(text(node), "staticmethod" | "classmethod")
    };

    let Some(expression) = decorator.named_child(0) else {
        return false;
    };
    match expression.kind() {
        "identifier" => is_builtin_name(expression),
        "attribute" => {
            expression
                .child_by_field_name("object")
                .is_some_and(|object| object.kind() == "identifier" && text(object) == "builtins")
                && expression
                    .child_by_field_name("attribute")
                    .is_some_and(is_builtin_name)
        }
        _ => false,
    }
}

/// Whether `slot` is a statement of a class body (not of a function or a
/// nested block).
fn is_in_class_body(slot: Node<'_>) -> bool {
    slot.indexed_parent()
        .filter(|block| block.kind() == "block")
        .and_then(|block| block.indexed_parent())
        .is_some_and(|owner| owner.kind() == "class_definition")
}

/// Whether a class-body assignment declares a class attribute rather than an
/// instance field: `x: ClassVar[T]` (with or without a value) and unannotated
/// `x = v` are class attributes; `x: T` and `x: T = v` are instance fields
/// (PEP 526, dataclass semantics).
pub(crate) fn is_python_class_attribute(assignment: Node<'_>, source: &str) -> bool {
    let in_class_body = assignment
        .indexed_parent()
        .filter(|statement| statement.kind() == "expression_statement")
        .and_then(|statement| statement.indexed_parent())
        .filter(|block| block.kind() == "block")
        .and_then(|block| block.indexed_parent())
        .is_some_and(|owner| owner.kind() == "class_definition");
    if !in_class_body {
        return false;
    }

    match assignment.child_by_field_name("type") {
        None => true,
        Some(annotation) => is_class_var_annotation(annotation, source),
    }
}

/// Whether an annotation is `ClassVar`, bare or subscripted, under any
/// qualifier (`typing.ClassVar[int]`, `t.ClassVar`). The annotation's
/// expression is matched, so `ClassVarLike[int]`, `MyClassVar` and a string
/// annotation (`"ClassVar[int]"`) are not.
fn is_class_var_annotation(annotation: Node<'_>, source: &str) -> bool {
    let expression = if annotation.kind() == "type" {
        match annotation.named_child(0) {
            Some(inner) => inner,
            None => return false,
        }
    } else {
        annotation
    };
    let head = match expression.kind() {
        "generic_type" => expression.named_child(0),
        "subscript" => expression.child_by_field_name("value"),
        _ => Some(expression),
    };
    let name = head.and_then(|head| match head.kind() {
        "identifier" => Some(head),
        "attribute" => head.child_by_field_name("attribute"),
        _ => None,
    });
    name.is_some_and(|name| {
        name.kind() == "identifier" && name.utf8_text(source.as_bytes()) == Ok("ClassVar")
    })
}
