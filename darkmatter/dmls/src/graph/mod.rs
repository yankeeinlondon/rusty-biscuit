//! The workspace graph substrate.
//!
//! A single in-memory graph carries every node kind and every edge kind
//! (AD-1/AD-2). The Markdown substrate indexer writes documents, headings,
//! links, and wiki-links, plus the Darkmatter semantic sources the graph
//! carries as typed edges: transclusions (`transcludes`), `$schema` uses
//! (`uses_schema`), file uses (`uses_file` — images, frontmatter `file(...)`
//! values, style assets, `::file-links`/`::toc-linking` paths), and
//! interpolation variables (`uses_variable`). Providers query one graph with
//! an edge-kind filter — never two vocabularies.
//!
//! Module map:
//!
//! - [`node`] / [`edge`] — the typed node and edge model.
//! - [`arena`] — [`WorkspaceGraph`], the immutable snapshot and its builder.
//! - [`index`] — the single reverse index over all edge kinds.
//! - [`key_index`] — wiki basename storage (resolution rules land in Phase 5).
//! - [`substrate`] — the Markdown indexer (`darkmatter` is the parser).
//! - [`invalidate`] — content-hash invalidation and snapshot swap.

pub mod arena;
pub mod edge;
pub mod index;
pub mod key_index;
pub mod invalidate;
pub mod node;
pub mod substrate;

pub use arena::{DocumentId, DocumentRecord, LinkDiagnostic, WorkspaceGraph};
pub(crate) use arena::normalize_join;
pub use edge::{Edge, EdgeId, EdgeKind, EdgeTarget};
pub use invalidate::{Invalidation, WorkspaceIndex};
pub use key_index::KeyIndex;
pub use node::{
    FileRefPayload, FrontmatterKeyPayload, HeadingPayload, LinkPayload, LinkTarget, Node, NodeId,
    NodeKind, NodePayload, TransclusionPayload, VariableUsePayload, WikiInfo, WikiLinkPayload,
    WikiResolution, classify_link_target,
};
pub use substrate::{
    DocumentIndex, FileRefFact, FrontmatterKeyFact, HeadingFact, IndexTimings, LinkFact,
    TransclusionFact, VariableUseFact, WikiLinkFact, index_document, index_document_timed,
};
