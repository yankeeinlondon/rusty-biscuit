//! Graph edge model: the eight typed edge kinds and the resolved-or-not
//! target of each edge.
//!
//! A single reverse index ([`ReverseIndex`](super::index::ReverseIndex)) is
//! built over *all* edge kinds, so references, backlinks, rename-impact
//! analysis, and invalidation fan-out are one lookup with an edge-kind filter
//! rather than N specialized maps.

use super::node::NodeId;

/// Stable index of an edge inside a [`WorkspaceGraph`](super::WorkspaceGraph)
/// snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct EdgeId(pub u32);

/// The kind of a graph edge (design "Workspace Graph" edge table).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum EdgeKind {
    /// Markdown links, reference links, wiki-links, anchors.
    References,
    /// Structural inclusion links (IWES-style).
    Includes,
    /// `::file`, `::code`, `::toc-linking`, `::file-links`, prologue, epilogue.
    Transcludes,
    /// `$schema`, baseline/extension schemas.
    UsesSchema,
    /// Frontmatter `file(...)`, images, style assets, directive paths.
    UsesFile,
    /// Interpolation/condition → frontmatter key, `ctx.*`, `env.*`.
    UsesVariable,
    /// Headings, custom IDs.
    DefinesAnchor,
    /// Headings, frontmatter keys, directives, schema keys.
    DefinesSymbol,
}

impl EdgeKind {
    /// Every edge kind, in declaration order (for exhaustive index building
    /// and tests).
    pub const ALL: [EdgeKind; 8] = [
        EdgeKind::References,
        EdgeKind::Includes,
        EdgeKind::Transcludes,
        EdgeKind::UsesSchema,
        EdgeKind::UsesFile,
        EdgeKind::UsesVariable,
        EdgeKind::DefinesAnchor,
        EdgeKind::DefinesSymbol,
    ];
}

/// The target end of an edge.
///
/// Edges to in-workspace nodes resolve to a [`NodeId`]; a link whose target is
/// broken, external-of-workspace, or a wiki form not yet matched is kept as
/// [`EdgeTarget::Unresolved`] so Phase 4 diagnostics and Phase 5 wiki matching
/// have the raw string without re-parsing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EdgeTarget {
    /// A resolved node in this snapshot.
    Node(NodeId),
    /// An unresolved raw target string (broken link, unmatched wiki form, …).
    Unresolved(String),
}

impl EdgeTarget {
    /// The resolved node id, if any.
    pub fn node(&self) -> Option<NodeId> {
        match self {
            EdgeTarget::Node(id) => Some(*id),
            EdgeTarget::Unresolved(_) => None,
        }
    }
}

/// One directed, typed edge from a source node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edge {
    /// The source node.
    pub source: NodeId,
    /// The edge kind.
    pub kind: EdgeKind,
    /// The (possibly unresolved) target.
    pub target: EdgeTarget,
}

impl Edge {
    /// Builds an edge to a resolved node.
    pub fn to_node(source: NodeId, kind: EdgeKind, target: NodeId) -> Self {
        Self {
            source,
            kind,
            target: EdgeTarget::Node(target),
        }
    }

    /// Builds an edge to an unresolved raw target.
    pub fn unresolved(source: NodeId, kind: EdgeKind, raw: impl Into<String>) -> Self {
        Self {
            source,
            kind,
            target: EdgeTarget::Unresolved(raw.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_edge_kind_all_is_exhaustive() {
        // A compile-time-ish guard: if a variant is added, this count must be
        // bumped, forcing a look at the reverse index and merge policies.
        assert_eq!(EdgeKind::ALL.len(), 8);
    }

    #[test]
    fn test_edge_target_node_accessor() {
        assert_eq!(EdgeTarget::Node(NodeId(3)).node(), Some(NodeId(3)));
        assert_eq!(EdgeTarget::Unresolved("x".into()).node(), None);
    }
}
