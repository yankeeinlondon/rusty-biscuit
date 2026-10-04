use super::*;

/// Lua visibility. The chunk-scope rules are not implemented yet, so every
/// declaration reports Unknown.
pub(super) fn extract_lua_visibility(
    _declaration: Node<'_>,
    _ctx: &DeclarationContext<'_>,
) -> Option<Visibility> {
    None
}
