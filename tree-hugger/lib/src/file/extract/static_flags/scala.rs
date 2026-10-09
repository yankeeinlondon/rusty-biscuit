use super::*;
use crate::file::parents::IndexedParent as _;

/// Checks if a Scala member belongs to an `object` (standalone or companion):
/// its nearest owning type is an `object_definition`. A member of a class,
/// trait, enum, anonymous class (`new T { … }`) or `given` instance is an
/// instance member, even when that type is itself declared in an object. An
/// `extension` block is a transparent wrapper (see [`member_slot`]), so its
/// methods belong to the type around it.
pub(crate) fn extract_scala_is_static(node: Node<'_>) -> bool {
    member_slot(node)
        .indexed_parent()
        .filter(|body| body.kind() == "template_body")
        .and_then(|body| body.indexed_parent())
        .is_some_and(|owner| owner.kind() == "object_definition")
}
