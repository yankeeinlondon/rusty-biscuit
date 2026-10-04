use super::*;

/// Go visibility. The identifier-case export rules are not implemented yet, so every
/// declaration reports Unknown.
pub(super) fn extract_go_visibility(
    _declaration: Node<'_>,
    _ctx: &DeclarationContext<'_>,
) -> Option<Visibility> {
    None
}
