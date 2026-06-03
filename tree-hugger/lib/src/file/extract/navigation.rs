//! Generic tree-sitter node navigation helpers carved out of `tree_file.rs`.

use super::*;


/// Finds an ancestor node with the given kind by walking up the parent chain.
pub(crate) fn find_ancestor_by_kind<'a>(node: Node<'a>, kind: &str) -> Option<Node<'a>> {
    let mut current = node.parent();
    while let Some(parent) = current {
        if parent.kind() == kind {
            return Some(parent);
        }
        current = parent.parent();
    }
    None
}

/// Finds the closest ancestor matching any of the provided kinds.
pub(crate) fn find_ancestor_by_kinds<'a>(node: Node<'a>, kinds: &[&str]) -> Option<Node<'a>> {
    let mut current = node.parent();
    while let Some(parent) = current {
        if kinds.iter().any(|kind| parent.kind() == *kind) {
            return Some(parent);
        }
        current = parent.parent();
    }
    None
}

/// Extracts the last component of a dotted name (e.g., "foo.bar.baz" -> "baz").
pub(crate) fn extract_dotted_name(name: &str) -> String {
    name.rsplit('.').next().unwrap_or(name).to_string()
}

/// Finds the first child node with the given kind.
pub(crate) fn find_child_by_kind<'a>(node: Node<'a>, kind: &str) -> Option<Node<'a>> {
    let mut cursor = node.walk();
    node.children(&mut cursor)
        .find(|child| child.kind() == kind)
}

/// Finds the nth child node with the given kind (0-indexed).
pub(crate) fn find_nth_child_by_kind<'a>(node: Node<'a>, kind: &str, n: usize) -> Option<Node<'a>> {
    let mut cursor = node.walk();
    node.children(&mut cursor)
        .filter(|child| child.kind() == kind)
        .nth(n)
}

/// Checks if a node is inside an interface declaration.
///
/// Walks up the parent chain to find `interface_declaration` ancestors.
/// Used to infer implicit public visibility for interface members in
/// languages like C# and Java.
pub(crate) fn is_inside_interface(node: Node<'_>) -> bool {
    let mut current = node.parent();
    while let Some(parent) = current {
        if parent.kind() == "interface_declaration" {
            return true;
        }
        current = parent.parent();
    }
    false
}
