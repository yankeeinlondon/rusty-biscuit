//! Generic tree-sitter node navigation helpers carved out of `tree_file.rs`.

use super::*;
use crate::file::parents::IndexedParent as _;

/// Finds an ancestor node with the given kind by walking up the parent chain.
pub(crate) fn find_ancestor_by_kind<'a>(node: Node<'a>, kind: &str) -> Option<Node<'a>> {
    let mut current = node.indexed_parent();
    while let Some(parent) = current {
        if parent.kind() == kind {
            return Some(parent);
        }
        current = parent.indexed_parent();
    }
    None
}

/// Finds the closest ancestor matching any of the provided kinds.
pub(crate) fn find_ancestor_by_kinds<'a>(node: Node<'a>, kinds: &[&str]) -> Option<Node<'a>> {
    let mut current = node.indexed_parent();
    while let Some(parent) = current {
        if kinds.iter().any(|kind| parent.kind() == *kind) {
            return Some(parent);
        }
        current = parent.indexed_parent();
    }
    None
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


// A grammar extra (a comment) can sit between any two tokens and is a named
// node, so a reader that picks syntax by sibling position or adjacency must
// step over extras or it mistakes a comment for the semantic node.

/// The first sibling after `node` that is not a grammar extra (a comment).
pub(crate) fn next_syntax_sibling(node: Node<'_>) -> Option<Node<'_>> {
    let mut sibling = node.next_sibling();
    while let Some(next) = sibling.filter(Node::is_extra) {
        sibling = next.next_sibling();
    }
    sibling
}

/// The first named sibling after `node` that is not a grammar extra.
pub(crate) fn next_named_syntax_sibling(node: Node<'_>) -> Option<Node<'_>> {
    let mut sibling = node.next_named_sibling();
    while let Some(next) = sibling.filter(Node::is_extra) {
        sibling = next.next_named_sibling();
    }
    sibling
}

/// The first sibling before `node` that is not a grammar extra.
pub(crate) fn prev_syntax_sibling(node: Node<'_>) -> Option<Node<'_>> {
    let mut sibling = node.prev_sibling();
    while let Some(previous) = sibling.filter(Node::is_extra) {
        sibling = previous.prev_sibling();
    }
    sibling
}

/// The first named sibling before `node` that is not a grammar extra.
pub(crate) fn prev_named_syntax_sibling(node: Node<'_>) -> Option<Node<'_>> {
    let mut sibling = node.prev_named_sibling();
    while let Some(previous) = sibling.filter(Node::is_extra) {
        sibling = previous.prev_named_sibling();
    }
    sibling
}

/// The tokens of `node` without its grammar extras, joined with nothing
/// between them: `N.S` for the TypeScript entity `N /* c */ .S` or
/// `N. /* c */ S`. Only for syntax whose tokens need no separator.
pub(crate) fn tokens_without_extras(node: Node<'_>, source: &str) -> String {
    let mut text = String::new();
    let mut cursor = node.walk();
    'walk: loop {
        let current = cursor.node();
        let skip = current.is_extra();
        if !skip && current.child_count() == 0 {
            text.push_str(source.get(current.byte_range()).unwrap_or_default());
        }
        if !skip && cursor.goto_first_child() {
            continue;
        }
        while !cursor.goto_next_sibling() {
            if !cursor.goto_parent() || cursor.node().id() == node.id() {
                break 'walk;
            }
        }
    }
    text
}
