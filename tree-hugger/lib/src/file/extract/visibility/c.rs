use super::*;

/// C visibility. The file-scope linkage rules are not implemented yet, so every
/// declaration reports Unknown.
pub(super) fn extract_c_visibility(
    _declaration: Node<'_>,
    _ctx: &DeclarationContext<'_>,
) -> Option<Visibility> {
    None
}
