use super::*;
use crate::file::parents::IndexedParent as _;

/// Lua visibility: a chunk-level `local` is ModulePrivate, a `local` in a
/// nested block is Unknown, and a global binding is Public. Table fields
/// (`M.f`, `M:m`, `M.x = …`) have no enforced access and are Unknown.
pub(super) fn extract_lua_visibility(
    declaration: Node<'_>,
    ctx: &DeclarationContext<'_, '_>,
) -> Option<Visibility> {
    if ctx.name.indexed_parent().is_some_and(|parent| {
        matches!(
            parent.kind(),
            "dot_index_expression" | "method_index_expression"
        )
    }) {
        return None;
    }

    let (statement, is_local) = match declaration.kind() {
        "function_declaration" => (declaration, find_child_by_kind(declaration, "local").is_some()),
        "assignment_statement" => match declaration.indexed_parent() {
            Some(parent) if parent.kind() == "variable_declaration" => (parent, true),
            _ => (declaration, false),
        },
        _ => return None,
    };

    if !is_local {
        return Some(Visibility::Public);
    }
    statement
        .indexed_parent()
        .is_some_and(|parent| parent.kind() == "chunk")
        .then_some(Visibility::ModulePrivate)
}
