//! C and C++ declarator traversal shared by every reader that needs the
//! function a declaration declares, or the name a declarator declares:
//! kinds (`classify.rs`), linkage (`visibility/c.rs`), signatures
//! (`signatures.rs`), field names (`type_metadata.rs`) and declarator
//! attributes (`attributes.rs`).

use super::*;

/// A function a declaration declares: the `function_declarator` that
/// directly declares `name`, and the name as written (qualified or not).
pub(crate) struct DeclaredFunction<'tree> {
    pub(crate) function: Node<'tree>,
    pub(crate) name: Node<'tree>,
}

/// One [`DeclaredFunction`] per declarator of `declaration` that declares a
/// function (see [`declared_function`]).
pub(crate) fn declared_functions(declaration: Node<'_>) -> Vec<DeclaredFunction<'_>> {
    let mut cursor = declaration.walk();
    declaration
        .children_by_field_name("declarator", &mut cursor)
        .filter_map(declared_function)
        .collect()
}

/// The function `declarator` declares, if it declares one. The function is
/// the declarator nearest the name with only parentheses or attributes
/// between them: in `int (*f(int a))(long)` that is `f(int a)`, and `(long)`
/// belongs to the returned pointer's type; `int f [[deprecated]] (int)` and
/// `int (f)(int)` declare `f`. A function pointer, reference or array
/// (`int (*p)(int)`) declares no function.
pub(crate) fn declared_function(mut declarator: Node<'_>) -> Option<DeclaredFunction<'_>> {
    let mut function = None;
    loop {
        match declarator.kind() {
            "function_declarator" => function = Some(declarator),
            "pointer_declarator" | "reference_declarator" | "array_declarator" => function = None,
            "parenthesized_declarator" | "attributed_declarator" | "init_declarator" => {}
            _ => {
                return function.map(|function| DeclaredFunction {
                    function,
                    name: declarator,
                });
            }
        }
        declarator = inner_declarator(declarator)?;
    }
}

/// The [`DeclaredFunction`] of `declaration` whose name contains `name`
/// (the captured identifier, which may sit inside a qualified name), so one
/// declarator of `int a(int), b(long);` is never read for the other.
pub(crate) fn declared_function_named<'tree>(
    declaration: Node<'tree>,
    name: Node<'_>,
) -> Option<DeclaredFunction<'tree>> {
    declared_functions(declaration).into_iter().find(|function| {
        function.name.start_byte() <= name.start_byte() && name.end_byte() <= function.name.end_byte()
    })
}

/// The name `declarator` declares: the first node below the pointer,
/// reference, array, parenthesized, attributed, function and pack wrappers.
/// A function declarator is followed through its `declarator` field only, so
/// the name of a callback parameter `int (*callback)(long ignored)` is
/// `callback`, never a name from its own parameter list. An abstract
/// declarator (an unnamed parameter) has no name.
pub(crate) fn declarator_name(mut declarator: Node<'_>) -> Option<Node<'_>> {
    loop {
        match declarator.kind() {
            "pointer_declarator"
            | "reference_declarator"
            | "array_declarator"
            | "parenthesized_declarator"
            | "attributed_declarator"
            | "function_declarator"
            | "variadic_declarator"
            | "init_declarator" => declarator = inner_declarator(declarator)?,
            kind if kind.starts_with("abstract_") => return None,
            _ => return Some(declarator),
        }
    }
}

/// The `attribute_declaration`s written right after `name` in `declaration`'s
/// declarator (`C [[deprecated]] (int x)`, `int a [[deprecated]], b`), which
/// the C++ standard makes appertain to the declared entity, as leading ones
/// do. Only the declarator holding `name` is read, along its `declarator`
/// path (never a parameter list), and only an attribute whose declarator is
/// the name itself behind parentheses: one after a function's parameters
/// (`int f(int) [[a]]`), an array bound or a returned pointer's parentheses
/// appertains to a type, not to the entity.
pub(crate) fn declarator_id_attributes<'tree>(declaration: Node<'tree>, name: Node<'_>) -> Vec<Node<'tree>> {
    let holds_name = |node: &Node<'_>| node.start_byte() <= name.start_byte() && name.end_byte() <= node.end_byte();
    let mut cursor = declaration.walk();
    let mut declarator = declaration
        .children_by_field_name("declarator", &mut cursor)
        .find(holds_name);
    let mut attributes = Vec::new();
    while let Some(layer) = declarator {
        if !is_wrapping_declarator(layer.kind()) {
            break;
        }
        if layer.kind() == "attributed_declarator" && !is_wrapping_declarator(transparent_declarator(layer).kind()) {
            let mut layer_cursor = layer.walk();
            attributes.extend(
                layer
                    .children(&mut layer_cursor)
                    .filter(|child| child.kind() == "attribute_declaration"),
            );
        }
        declarator = inner_declarator(layer).filter(holds_name);
    }
    attributes
}

/// A declarator that wraps another one rather than naming the entity.
fn is_wrapping_declarator(kind: &str) -> bool {
    matches!(
        kind,
        "pointer_declarator"
            | "reference_declarator"
            | "array_declarator"
            | "parenthesized_declarator"
            | "attributed_declarator"
            | "function_declarator"
            | "variadic_declarator"
            | "init_declarator"
    )
}

/// `declarator` with its parenthesized and attributed wrappers removed:
/// `(f)` and `f [[deprecated]]` are `f`; a pointer, reference or array is
/// kept.
pub(crate) fn transparent_declarator(mut declarator: Node<'_>) -> Node<'_> {
    while matches!(declarator.kind(), "parenthesized_declarator" | "attributed_declarator")
        && let Some(inner) = inner_declarator(declarator)
    {
        declarator = inner;
    }
    declarator
}

/// The declarator a wrapping declarator wraps: its `declarator` field, or
/// (for reference, parenthesized, attributed and pack declarators, which
/// have no such field) its first named child that is not a modifier or
/// comment.
pub(crate) fn inner_declarator(node: Node<'_>) -> Option<Node<'_>> {
    node.child_by_field_name("declarator").or_else(|| {
        let mut cursor = node.walk();
        node.named_children(&mut cursor).find(|child| {
            !matches!(
                child.kind(),
                "comment"
                    | "attribute_declaration"
                    | "attribute_specifier"
                    | "ms_call_modifier"
                    | "ms_based_modifier"
                    | "ms_pointer_modifier"
                    | "type_qualifier"
            )
        })
    })
}
