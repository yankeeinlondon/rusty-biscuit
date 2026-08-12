//! The single reverse index over all edge kinds.
//!
//! One structure indexes every edge by both ends, so `references`, backlinks,
//! rename-impact analysis, and invalidation fan-out are the same lookup with
//! an [`EdgeKind`](super::edge::EdgeKind) filter — never N specialized maps
//! (design "Workspace Graph").

use std::collections::HashMap;

use super::edge::{Edge, EdgeId};
use super::node::NodeId;

/// Bidirectional edge index keyed by node.
///
/// Stores only [`EdgeId`]s; callers resolve them against the owning graph's
/// edge slice so edge data lives in exactly one place.
#[derive(Debug, Default)]
pub struct ReverseIndex {
    outgoing: HashMap<NodeId, Vec<EdgeId>>,
    incoming: HashMap<NodeId, Vec<EdgeId>>,
}

impl ReverseIndex {
    /// Builds the index over an edge slice. Resolved-node targets populate the
    /// incoming map; unresolved targets contribute only an outgoing entry.
    pub fn build(edges: &[Edge]) -> Self {
        let mut index = Self::default();
        for (position, edge) in edges.iter().enumerate() {
            let id = EdgeId(position as u32);
            index.outgoing.entry(edge.source).or_default().push(id);
            if let Some(target) = edge.target.node() {
                index.incoming.entry(target).or_default().push(id);
            }
        }
        index
    }

    /// Edge ids leaving `source` (its outgoing edges).
    pub fn outgoing(&self, source: NodeId) -> &[EdgeId] {
        self.outgoing.get(&source).map_or(&[], Vec::as_slice)
    }

    /// Edge ids arriving at `target` (its incoming/back edges).
    pub fn incoming(&self, target: NodeId) -> &[EdgeId] {
        self.incoming.get(&target).map_or(&[], Vec::as_slice)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::edge::EdgeKind;

    fn edges() -> Vec<Edge> {
        vec![
            Edge::to_node(NodeId(0), EdgeKind::References, NodeId(1)),
            Edge::to_node(NodeId(0), EdgeKind::DefinesAnchor, NodeId(1)),
            Edge::unresolved(NodeId(2), EdgeKind::References, "missing.md"),
        ]
    }

    #[test]
    fn test_outgoing_collects_all_kinds() {
        let edges = edges();
        let index = ReverseIndex::build(&edges);
        assert_eq!(index.outgoing(NodeId(0)), &[EdgeId(0), EdgeId(1)]);
        assert_eq!(index.outgoing(NodeId(2)), &[EdgeId(2)]);
    }

    #[test]
    fn test_incoming_only_for_resolved_targets() {
        let edges = edges();
        let index = ReverseIndex::build(&edges);
        assert_eq!(index.incoming(NodeId(1)), &[EdgeId(0), EdgeId(1)]);
        // Unresolved edge target has no incoming entry anywhere.
        assert!(index.incoming(NodeId(2)).is_empty());
    }

    #[test]
    fn test_absent_node_returns_empty() {
        let index = ReverseIndex::build(&[]);
        assert!(index.outgoing(NodeId(99)).is_empty());
        assert!(index.incoming(NodeId(99)).is_empty());
    }
}
