use super::*;

/// Scala visibility. The modifier and qualifier rules are not implemented yet, so every
/// declaration reports Unknown.
pub(super) fn extract_scala_visibility(
    _declaration: Node<'_>,
    _ctx: &DeclarationContext<'_>,
) -> Option<Visibility> {
    None
}
