//! The workspace graph arena and its snapshot builder.
//!
//! A [`WorkspaceGraph`] is an immutable snapshot: documents, nodes, edges, the
//! reverse index, and the wiki key index, all minted together and swapped in
//! atomically by the invalidation engine (AD-3). The arena-and-build-pass
//! shape (ID-indexed `Vec`s, resolution deferred to a single assembly pass) is
//! a fresh implementation of the pattern proven by the Apache-2.0 `iwe`
//! project (IWE, Dmytro Halichenko) per the AD-1 decision record; DMLS owns
//! the multi-edge-kind graph outright rather than overlaying a second index on
//! a two-edge-kind model.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

use super::edge::{Edge, EdgeId, EdgeKind};
use super::index::ReverseIndex;
use super::key_index::KeyIndex;
use super::node::{
    FileRefPayload, FrontmatterKeyPayload, HeadingPayload, LinkPayload, LinkTarget, Node, NodeId,
    NodeKind, NodePayload, TransclusionPayload, VariableUsePayload, WikiInfo, WikiLinkPayload,
    WikiResolution,
};
use super::substrate::{DocumentIndex, WikiLinkFact};
use crate::wiki::{self, Match, ParseOutcome, WikiDoc};

/// Stable index of a document inside a [`WorkspaceGraph`] snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DocumentId(pub u32);

/// Per-document metadata in the snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentRecord {
    /// Logical (workspace) path.
    pub path: PathBuf,
    /// xxHash of the source at index time (invalidation identity).
    pub content_hash: u64,
    /// The document's root node.
    pub root: NodeId,
    /// Canonical wiki logical-path segments (NFC, final extension elided).
    pub canonical: Vec<String>,
    /// Owning wiki root index, or [`WikiDoc::NO_ROOT`] when outside every root.
    pub workspace_id: usize,
}

/// An immutable workspace graph snapshot.
#[derive(Debug)]
pub struct WorkspaceGraph {
    documents: Vec<DocumentRecord>,
    by_path: HashMap<PathBuf, DocumentId>,
    nodes: Vec<Node>,
    edges: Vec<Edge>,
    reverse: ReverseIndex,
    keys: KeyIndex,
    /// Documents whose canonical logical paths collide under case-fold/NFC —
    /// the R-8 `wiki.portability-collision` set (each maps to its twins).
    portability: HashMap<DocumentId, Vec<DocumentId>>,
    generation: u64,
}

/// The product of the node/edge assembly pass, handed to
/// [`WorkspaceGraph::finalize`].
///
/// Exists so the bench harness can time graph construction ([`assemble`]) and
/// reverse-index construction ([`finalize`]) as the separate R-6 stages the
/// design's stage list calls for, without duplicating the build logic —
/// production runs both back to back through [`build_with_roots`].
///
/// [`assemble`]: WorkspaceGraph::assemble
/// [`finalize`]: WorkspaceGraph::finalize
/// [`build_with_roots`]: WorkspaceGraph::build_with_roots
pub(crate) struct GraphAssembly {
    documents: Vec<DocumentRecord>,
    by_path: HashMap<PathBuf, DocumentId>,
    nodes: Vec<Node>,
    edges: Vec<Edge>,
    generation: u64,
}

impl WorkspaceGraph {
    /// Assembles a snapshot from per-document parse products (no wiki roots —
    /// wiki resolution falls back to absolute-path tails; see
    /// [`WorkspaceGraph::build_with_roots`]).
    pub fn build(indices: &BTreeMap<PathBuf, DocumentIndex>, generation: u64) -> Self {
        Self::build_with_roots(indices, generation, &[])
    }

    /// Assembles a snapshot, resolving wiki links against `wiki_roots`.
    ///
    /// `indices` is keyed by logical path (a `BTreeMap` so document ids are
    /// assigned in a deterministic, path-sorted order regardless of discovery
    /// order). `generation` stamps the snapshot for supersession checks.
    /// `wiki_roots` are the R-8 wiki roots each document's canonical logical
    /// path is computed relative to.
    pub fn build_with_roots(
        indices: &BTreeMap<PathBuf, DocumentIndex>,
        generation: u64,
        wiki_roots: &[PathBuf],
    ) -> Self {
        Self::finalize(Self::assemble(indices, generation, wiki_roots))
    }

    /// Node/edge assembly pass: mints every node and resolves every edge,
    /// deferring reverse-index and key-index construction to [`finalize`].
    ///
    /// Split from [`finalize`] purely so the bench harness can time the two
    /// as separate R-6 stages (`graph_build` vs `reverse_index`);
    /// [`build_with_roots`] runs them back to back.
    ///
    /// [`finalize`]: WorkspaceGraph::finalize
    /// [`build_with_roots`]: WorkspaceGraph::build_with_roots
    pub(crate) fn assemble(
        indices: &BTreeMap<PathBuf, DocumentIndex>,
        generation: u64,
        wiki_roots: &[PathBuf],
    ) -> GraphAssembly {
        let mut nodes: Vec<Node> = Vec::new();
        let mut documents: Vec<DocumentRecord> = Vec::new();
        let mut by_path: HashMap<PathBuf, DocumentId> = HashMap::new();
        // (DocumentId, slug) → heading node, for anchor resolution.
        let mut heading_by_slug: HashMap<(DocumentId, String), NodeId> = HashMap::new();
        // Per-document heading facts (node, title, slug) for wiki heading rules.
        let mut headings_by_doc: HashMap<DocumentId, Vec<(NodeId, String, String)>> = HashMap::new();
        // Deferred link resolution: (source link node, owning doc, target).
        let mut pending_links: Vec<(NodeId, DocumentId, LinkTarget, String)> = Vec::new();
        // Deferred wiki resolution: (source wiki node, owning doc, fact).
        let mut pending_wiki: Vec<(NodeId, DocumentId, WikiLinkFact)> = Vec::new();
        // Deferred transclusion resolution: (source node, owning doc, local path).
        let mut pending_transclusions: Vec<(NodeId, DocumentId, String)> = Vec::new();
        // Deferred file-reference resolution: (source node, owning doc, local path).
        let mut pending_schema_uses: Vec<(NodeId, DocumentId, String)> = Vec::new();
        let mut pending_file_uses: Vec<(NodeId, DocumentId, String)> = Vec::new();
        // Deferred variable resolution: (source node, owning doc, identifier).
        let mut pending_variable_uses: Vec<(NodeId, DocumentId, String)> = Vec::new();
        // (DocumentId, top-level key) → frontmatter-key node, for variable
        // resolution.
        let mut key_by_name: HashMap<(DocumentId, String), NodeId> = HashMap::new();
        // Document root + its heading nodes, to emit defines_* edges after all
        // nodes exist.
        let mut heading_nodes: Vec<(NodeId, NodeId)> = Vec::new();

        for (position, (path, index)) in indices.iter().enumerate() {
            let doc_id = DocumentId(position as u32);
            by_path.insert(path.clone(), doc_id);
            let (workspace_id, canonical) = document_wiki_meta(path, wiki_roots);

            let root = NodeId(nodes.len() as u32);
            nodes.push(Node {
                document: doc_id,
                kind: NodeKind::Document,
                span: 0..0,
                payload: NodePayload::Document,
            });
            documents.push(DocumentRecord {
                path: path.clone(),
                content_hash: index.content_hash,
                root,
                canonical,
                workspace_id,
            });

            let mut doc_headings: Vec<(NodeId, String, String)> = Vec::new();
            for heading in &index.headings {
                let node_id = NodeId(nodes.len() as u32);
                nodes.push(Node {
                    document: doc_id,
                    kind: NodeKind::Heading,
                    span: heading.span.clone(),
                    payload: NodePayload::Heading(HeadingPayload {
                        level: heading.level,
                        title: heading.title.clone(),
                        slug: heading.slug.clone(),
                        title_span: heading.title_span.clone(),
                        line: heading.line,
                    }),
                });
                heading_by_slug.insert((doc_id, heading.slug.clone()), node_id);
                doc_headings.push((node_id, heading.title.clone(), heading.slug.clone()));
                heading_nodes.push((root, node_id));
            }
            headings_by_doc.insert(doc_id, doc_headings);

            for link in &index.links {
                let node_id = NodeId(nodes.len() as u32);
                nodes.push(Node {
                    document: doc_id,
                    kind: NodeKind::Link,
                    span: link.span.clone(),
                    payload: NodePayload::Link(LinkPayload {
                        raw_target: link.raw_target.clone(),
                        display: String::new(),
                        target: link.target.clone(),
                        line: link.line,
                    }),
                });
                pending_links.push((node_id, doc_id, link.target.clone(), link.raw_target.clone()));
            }

            for wiki in &index.wiki_links {
                let node_id = NodeId(nodes.len() as u32);
                nodes.push(Node {
                    document: doc_id,
                    kind: NodeKind::WikiLink,
                    span: wiki.span.clone(),
                    payload: NodePayload::WikiLink(WikiLinkPayload {
                        target: wiki.target.clone(),
                        heading: wiki.heading.clone(),
                        alias: wiki.alias.clone(),
                        target_span: wiki.target_span.clone(),
                        heading_span: wiki.heading_span.clone(),
                        line: wiki.line,
                        resolution: WikiResolution::Unresolved,
                        info: None,
                    }),
                });
                pending_wiki.push((node_id, doc_id, wiki.clone()));
            }

            for transclusion in &index.transclusions {
                let node_id = NodeId(nodes.len() as u32);
                nodes.push(Node {
                    document: doc_id,
                    kind: NodeKind::TransclusionTarget,
                    span: transclusion.span.clone(),
                    payload: NodePayload::Transclusion(TransclusionPayload {
                        raw_target: transclusion.raw_target.clone(),
                        path: transclusion.path.clone(),
                        line: transclusion.line,
                    }),
                });
                pending_transclusions.push((node_id, doc_id, transclusion.path.clone()));
            }

            for key in &index.frontmatter_keys {
                let node_id = NodeId(nodes.len() as u32);
                nodes.push(Node {
                    document: doc_id,
                    kind: NodeKind::FrontmatterKey,
                    span: key.span.clone(),
                    payload: NodePayload::FrontmatterKey(FrontmatterKeyPayload {
                        key: key.key.clone(),
                        line: key.line,
                    }),
                });
                // First occurrence wins (a duplicate authored key is invalid YAML
                // and would already be a parse error upstream).
                key_by_name.entry((doc_id, key.key.clone())).or_insert(node_id);
            }

            for schema in &index.schema_uses {
                let node_id = NodeId(nodes.len() as u32);
                nodes.push(Node {
                    document: doc_id,
                    kind: NodeKind::SchemaDeclaration,
                    span: schema.span.clone(),
                    payload: NodePayload::FileRef(FileRefPayload {
                        raw_target: schema.raw_target.clone(),
                        path: schema.path.clone(),
                        line: schema.line,
                    }),
                });
                pending_schema_uses.push((node_id, doc_id, schema.path.clone()));
            }

            for file in &index.file_uses {
                let node_id = NodeId(nodes.len() as u32);
                nodes.push(Node {
                    document: doc_id,
                    kind: NodeKind::FileReference,
                    span: file.span.clone(),
                    payload: NodePayload::FileRef(FileRefPayload {
                        raw_target: file.raw_target.clone(),
                        path: file.path.clone(),
                        line: file.line,
                    }),
                });
                pending_file_uses.push((node_id, doc_id, file.path.clone()));
            }

            for variable in &index.variable_uses {
                let node_id = NodeId(nodes.len() as u32);
                nodes.push(Node {
                    document: doc_id,
                    kind: NodeKind::Interpolation,
                    span: variable.span.clone(),
                    payload: NodePayload::VariableUse(VariableUsePayload {
                        name: variable.name.clone(),
                        line: variable.line,
                    }),
                });
                pending_variable_uses.push((node_id, doc_id, variable.name.clone()));
            }
        }

        let mut edges: Vec<Edge> = Vec::new();

        // Each heading defines an anchor and a symbol on its document.
        for (root, heading) in heading_nodes {
            edges.push(Edge::to_node(root, EdgeKind::DefinesAnchor, heading));
            edges.push(Edge::to_node(root, EdgeKind::DefinesSymbol, heading));
        }

        // Resolve every Markdown link into a `references` edge.
        for (source, doc_id, target, raw) in pending_links {
            let resolved = resolve_link(
                &target,
                doc_id,
                &documents,
                &by_path,
                &heading_by_slug,
            );
            match resolved {
                LinkResolution::Node(node) => {
                    edges.push(Edge::to_node(source, EdgeKind::References, node));
                }
                LinkResolution::Unresolved => {
                    edges.push(Edge::unresolved(source, EdgeKind::References, raw));
                }
                // External links carry no graph edge.
                LinkResolution::External => {}
            }
        }

        // Resolve wiki links against the workspace (R-8), emitting a
        // `references` edge to every resolved (or candidate) target node.
        let wiki_docs: Vec<WikiDoc> = documents
            .iter()
            .map(|record| WikiDoc {
                canonical: record.canonical.clone(),
                workspace_id: record.workspace_id,
            })
            .collect();
        let doc_roots: Vec<NodeId> = documents.iter().map(|record| record.root).collect();
        for (source, doc_id, fact) in pending_wiki {
            let (resolution, info) = resolve_wiki_link(
                &fact,
                doc_id,
                &wiki_docs,
                &doc_roots,
                &headings_by_doc,
            );
            for target in wiki_edge_targets(&resolution) {
                edges.push(Edge::to_node(source, EdgeKind::References, target));
            }
            if let NodePayload::WikiLink(payload) = &mut nodes[source.0 as usize].payload {
                payload.resolution = resolution;
                payload.info = info;
            }
        }

        // Resolve `::file`/`::code` transclusions: a `transcludes` edge is emitted
        // only when the path names an indexed document. A non-`.md` target
        // (`::code ./mod.rs`) or a broken path carries no edge — request-time
        // diagnostics distinguish "missing" from "not a workspace document".
        for (source, doc_id, path) in pending_transclusions {
            let Some(base_dir) = documents
                .get(doc_id.0 as usize)
                .and_then(|record| record.path.parent())
            else {
                continue;
            };
            let resolved = normalize_join(base_dir, &path);
            if let Some(&target_doc) = by_path.get(&resolved)
                && let Some(record) = documents.get(target_doc.0 as usize)
            {
                edges.push(Edge::to_node(source, EdgeKind::Transcludes, record.root));
            }
        }

        // Resolve `$schema` (uses_schema) and file uses (uses_file). Schema
        // files and assets (`.yaml`, images, directories) are not `.md` graph
        // documents, so they stay `Unresolved`; a `file(...)` value naming an
        // indexed document resolves to its root, feeding invalidation fan-out.
        for (source, doc_id, path) in pending_schema_uses {
            edges.push(resolve_file_edge(source, EdgeKind::UsesSchema, doc_id, &path, &documents, &by_path));
        }
        for (source, doc_id, path) in pending_file_uses {
            edges.push(resolve_file_edge(source, EdgeKind::UsesFile, doc_id, &path, &documents, &by_path));
        }

        // Resolve interpolation variables (uses_variable) against a same-document
        // top-level frontmatter key. The leading segment of a dotted identifier
        // (`author` of `author.name`) names the key; `ctx.*`/`env.*`/functions and
        // unknown names stay `Unresolved` so the use is still recorded.
        for (source, doc_id, name) in pending_variable_uses {
            let first_segment = name.split('.').next().unwrap_or(&name);
            match key_by_name.get(&(doc_id, first_segment.to_string())) {
                Some(&target) => edges.push(Edge::to_node(source, EdgeKind::UsesVariable, target)),
                None => edges.push(Edge::unresolved(source, EdgeKind::UsesVariable, name)),
            }
        }

        GraphAssembly {
            documents,
            by_path,
            nodes,
            edges,
            generation,
        }
    }

    /// Reverse-index / key-index construction pass: builds the lookups over the
    /// assembled nodes and edges and freezes the immutable snapshot.
    ///
    /// See [`assemble`] for why the two passes are separate.
    ///
    /// [`assemble`]: WorkspaceGraph::assemble
    pub(crate) fn finalize(assembly: GraphAssembly) -> Self {
        let GraphAssembly {
            documents,
            by_path,
            nodes,
            edges,
            generation,
        } = assembly;
        let reverse = ReverseIndex::build(&edges);
        let keys = KeyIndex::build(
            documents
                .iter()
                .enumerate()
                .map(|(position, record)| (DocumentId(position as u32), record.path.as_path())),
        );
        let portability = portability_collisions(&documents);

        Self {
            documents,
            by_path,
            nodes,
            edges,
            reverse,
            keys,
            portability,
            generation,
        }
    }

    /// The snapshot generation.
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// Number of documents.
    pub fn document_count(&self) -> usize {
        self.documents.len()
    }

    /// Number of nodes.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Number of edges.
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    /// The wiki basename index.
    pub fn key_index(&self) -> &KeyIndex {
        &self.keys
    }

    /// A node by id.
    pub fn node(&self, id: NodeId) -> Option<&Node> {
        self.nodes.get(id.0 as usize)
    }

    /// A document record by id.
    pub fn document(&self, id: DocumentId) -> Option<&DocumentRecord> {
        self.documents.get(id.0 as usize)
    }

    /// The document id for a logical path.
    pub fn document_id(&self, path: &Path) -> Option<DocumentId> {
        self.by_path.get(path).copied()
    }

    /// All document records.
    pub fn documents(&self) -> &[DocumentRecord] {
        &self.documents
    }

    /// An edge by id.
    pub fn edge(&self, id: EdgeId) -> Option<&Edge> {
        self.edges.get(id.0 as usize)
    }

    /// Outgoing edges of `node`, filtered to `kind`.
    pub fn outgoing(&self, node: NodeId, kind: EdgeKind) -> impl Iterator<Item = &Edge> {
        self.reverse
            .outgoing(node)
            .iter()
            .filter_map(move |id| self.edges.get(id.0 as usize))
            .filter(move |edge| edge.kind == kind)
    }

    /// Incoming (back) edges of `node`, filtered to `kind` — the backlink and
    /// reference-fan-in query.
    pub fn incoming(&self, node: NodeId, kind: EdgeKind) -> impl Iterator<Item = &Edge> {
        self.reverse
            .incoming(node)
            .iter()
            .filter_map(move |id| self.edges.get(id.0 as usize))
            .filter(move |edge| edge.kind == kind)
    }

    /// Heading nodes of a document, in document order.
    pub fn headings(&self, document: DocumentId) -> impl Iterator<Item = (NodeId, &Node)> {
        self.nodes
            .iter()
            .enumerate()
            .map(|(position, node)| (NodeId(position as u32), node))
            .filter(move |(_, node)| node.document == document && node.kind == NodeKind::Heading)
    }

    /// Link nodes of a document, in document order.
    pub fn links(&self, document: DocumentId) -> impl Iterator<Item = (NodeId, &Node)> {
        self.nodes
            .iter()
            .enumerate()
            .map(|(position, node)| (NodeId(position as u32), node))
            .filter(move |(_, node)| node.document == document && node.kind == NodeKind::Link)
    }

    /// Wiki-link nodes of a document, in document order.
    pub fn wiki_links(&self, document: DocumentId) -> impl Iterator<Item = (NodeId, &Node)> {
        self.nodes
            .iter()
            .enumerate()
            .map(|(position, node)| (NodeId(position as u32), node))
            .filter(move |(_, node)| node.document == document && node.kind == NodeKind::WikiLink)
    }

    /// Transclusion-target nodes of a document, in document order.
    pub fn transclusions(&self, document: DocumentId) -> impl Iterator<Item = (NodeId, &Node)> {
        self.nodes
            .iter()
            .enumerate()
            .map(|(position, node)| (NodeId(position as u32), node))
            .filter(move |(_, node)| {
                node.document == document && node.kind == NodeKind::TransclusionTarget
            })
    }

    /// `$schema` file-reference nodes of a document (`uses_schema` sources).
    pub fn schema_uses(&self, document: DocumentId) -> impl Iterator<Item = (NodeId, &Node)> {
        self.nodes_of_kind(document, NodeKind::SchemaDeclaration)
    }

    /// File-use nodes of a document (`uses_file` sources).
    pub fn file_uses(&self, document: DocumentId) -> impl Iterator<Item = (NodeId, &Node)> {
        self.nodes_of_kind(document, NodeKind::FileReference)
    }

    /// Interpolation nodes of a document (`uses_variable` sources).
    pub fn variable_uses(&self, document: DocumentId) -> impl Iterator<Item = (NodeId, &Node)> {
        self.nodes_of_kind(document, NodeKind::Interpolation)
    }

    /// Top-level frontmatter-key nodes of a document (`uses_variable` targets).
    pub fn frontmatter_keys(&self, document: DocumentId) -> impl Iterator<Item = (NodeId, &Node)> {
        self.nodes_of_kind(document, NodeKind::FrontmatterKey)
    }

    /// Nodes of `document` of a given kind, in document order.
    fn nodes_of_kind(&self, document: DocumentId, kind: NodeKind) -> impl Iterator<Item = (NodeId, &Node)> {
        self.nodes
            .iter()
            .enumerate()
            .map(|(position, node)| (NodeId(position as u32), node))
            .filter(move |(_, node)| node.document == document && node.kind == kind)
    }

    /// Documents that transclude `document` via `::file`/`::code` — the source
    /// document of every incoming `transcludes` edge to the document's root.
    pub fn transcluded_by(&self, document: DocumentId) -> Vec<NodeId> {
        let Some(root) = self.document(document).map(|record| record.root) else {
            return Vec::new();
        };
        self.incoming(root, EdgeKind::Transcludes)
            .map(|edge| edge.source)
            .collect()
    }

    /// Detects a transclusion cycle reachable from `start`, returning the
    /// document chain that closes the loop (`start … → start`) when one exists.
    ///
    /// The walk follows `transcludes` edges document-to-document; the first time
    /// a document is revisited on the current path, the slice from that document
    /// onward (plus the repeated document) is the cycle. Only cycles that pass
    /// through `start` are reported, so a diagnostic ranges the document the user
    /// is editing.
    pub fn transclusion_cycle(&self, start: DocumentId) -> Option<Vec<DocumentId>> {
        let mut path = vec![start];
        let mut visited = std::collections::HashSet::new();
        self.walk_cycle(start, start, &mut path, &mut visited)
    }

    /// Depth-first search for a `transcludes` cycle returning to `target`.
    fn walk_cycle(
        &self,
        current: DocumentId,
        target: DocumentId,
        path: &mut Vec<DocumentId>,
        visited: &mut std::collections::HashSet<DocumentId>,
    ) -> Option<Vec<DocumentId>> {
        for next in self.transclusion_targets(current) {
            if next == target {
                let mut cycle = path.clone();
                cycle.push(target);
                return Some(cycle);
            }
            if !visited.insert(next) {
                continue;
            }
            path.push(next);
            if let Some(cycle) = self.walk_cycle(next, target, path, visited) {
                return Some(cycle);
            }
            path.pop();
        }
        None
    }

    /// Documents `document` transcludes directly (the target document of every
    /// outgoing `transcludes` edge from one of its transclusion-target nodes).
    fn transclusion_targets(&self, document: DocumentId) -> Vec<DocumentId> {
        let mut targets = Vec::new();
        for (node_id, _) in self.transclusions(document) {
            for edge in self.outgoing(node_id, EdgeKind::Transcludes) {
                if let Some(target) = edge.target.node()
                    && let Some(node) = self.node(target)
                {
                    targets.push(node.document);
                }
            }
        }
        targets
    }

    /// The documents as the wiki matcher sees them (canonical path + wiki root),
    /// indexed by `DocumentId`, for completion-time suffix resolution.
    pub fn wiki_docs(&self) -> Vec<WikiDoc> {
        self.documents
            .iter()
            .map(|record| WikiDoc {
                canonical: record.canonical.clone(),
                workspace_id: record.workspace_id,
            })
            .collect()
    }

    /// Documents whose canonical logical path collides with `document`'s under
    /// case-fold/NFC normalization (empty when there is no collision) — the
    /// R-8 `wiki.portability-collision` twins.
    pub fn portability_twins(&self, document: DocumentId) -> &[DocumentId] {
        self.portability.get(&document).map_or(&[], Vec::as_slice)
    }

    /// Classifies why a link target failed to resolve, so link diagnostics can
    /// distinguish a missing file from a present file with a missing anchor.
    ///
    /// Returns `None` for external targets and for targets that actually
    /// resolve (those carry a resolved [`EdgeTarget::Node`] instead).
    pub fn diagnose_unresolved(
        &self,
        source: DocumentId,
        target: &LinkTarget,
    ) -> Option<LinkDiagnostic> {
        match target {
            LinkTarget::External => None,
            LinkTarget::SameDocumentAnchor { slug } => {
                self.heading_in(source, slug).is_none().then_some(LinkDiagnostic::MissingAnchor)
            }
            LinkTarget::RelativePath { path, fragment } => {
                let base_dir = self.document(source).and_then(|record| {
                    record.path.parent().map(Path::to_path_buf)
                })?;
                let resolved = normalize_join(&base_dir, path);
                match self.document_id(&resolved) {
                    None => Some(LinkDiagnostic::BrokenPath),
                    Some(target_doc) => match fragment {
                        Some(slug) if self.heading_in(target_doc, slug).is_none() => {
                            Some(LinkDiagnostic::MissingAnchor)
                        }
                        _ => None,
                    },
                }
            }
        }
    }

    /// The heading node in `document` with anchor `slug`, if any.
    pub fn heading_in(&self, document: DocumentId, slug: &str) -> Option<NodeId> {
        self.headings(document).find_map(|(id, node)| {
            (node.as_heading().is_some_and(|heading| heading.slug == slug)).then_some(id)
        })
    }
}

/// Why an unresolved link target could not be resolved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkDiagnostic {
    /// The relative path did not match any indexed document.
    BrokenPath,
    /// The document resolved but the `#fragment`/anchor did not.
    MissingAnchor,
}

/// Outcome of resolving a link target to a graph node.
enum LinkResolution {
    Node(NodeId),
    Unresolved,
    External,
}

fn resolve_link(
    target: &LinkTarget,
    source_doc: DocumentId,
    documents: &[DocumentRecord],
    by_path: &HashMap<PathBuf, DocumentId>,
    heading_by_slug: &HashMap<(DocumentId, String), NodeId>,
) -> LinkResolution {
    match target {
        LinkTarget::External => LinkResolution::External,
        LinkTarget::SameDocumentAnchor { slug } => {
            match heading_by_slug.get(&(source_doc, slug.clone())) {
                Some(node) => LinkResolution::Node(*node),
                None => LinkResolution::Unresolved,
            }
        }
        LinkTarget::RelativePath { path, fragment } => {
            let Some(base_dir) = documents
                .get(source_doc.0 as usize)
                .and_then(|record| record.path.parent())
            else {
                return LinkResolution::Unresolved;
            };
            let resolved_path = normalize_join(base_dir, path);
            let Some(&target_doc) = by_path.get(&resolved_path) else {
                return LinkResolution::Unresolved;
            };
            match fragment {
                Some(slug) => match heading_by_slug.get(&(target_doc, slug.clone())) {
                    Some(node) => LinkResolution::Node(*node),
                    None => LinkResolution::Unresolved,
                },
                None => match documents.get(target_doc.0 as usize) {
                    Some(record) => LinkResolution::Node(record.root),
                    None => LinkResolution::Unresolved,
                },
            }
        }
    }
}

/// Resolves a `uses_schema`/`uses_file` reference to an edge: a resolved edge to
/// an indexed Markdown document's root when `path` names one, else an
/// `Unresolved` edge carrying the raw path (an asset, schema YAML, or directory
/// that is not a graph document). Purely lexical — no filesystem access.
fn resolve_file_edge(
    source: NodeId,
    kind: EdgeKind,
    source_doc: DocumentId,
    path: &str,
    documents: &[DocumentRecord],
    by_path: &HashMap<PathBuf, DocumentId>,
) -> Edge {
    if let Some(base_dir) = documents
        .get(source_doc.0 as usize)
        .and_then(|record| record.path.parent())
    {
        let resolved = normalize_join(base_dir, path);
        if let Some(&target_doc) = by_path.get(&resolved)
            && let Some(record) = documents.get(target_doc.0 as usize)
        {
            return Edge::to_node(source, kind, record.root);
        }
    }
    Edge::unresolved(source, kind, path.to_string())
}

/// A document's wiki root membership: the index of the first `wiki_roots`
/// entry that is an ancestor of `path` (with the canonical logical path
/// relative to it), else [`WikiDoc::NO_ROOT`] with an absolute-path-tail
/// canonical.
fn document_wiki_meta(path: &Path, wiki_roots: &[PathBuf]) -> (usize, Vec<String>) {
    for (index, root) in wiki_roots.iter().enumerate() {
        if path.starts_with(root) {
            return (index, wiki::canonical_segments(path, Some(root)));
        }
    }
    (WikiDoc::NO_ROOT, wiki::canonical_segments(path, None))
}

/// The target nodes a wiki link's `references` edges point at.
fn wiki_edge_targets(resolution: &WikiResolution) -> Vec<NodeId> {
    match resolution {
        WikiResolution::Resolved(node)
        | WikiResolution::HeadingMissing(node)
        | WikiResolution::EmptyHeading(Some(node)) => vec![*node],
        WikiResolution::Ambiguous(nodes) => nodes.clone(),
        _ => Vec::new(),
    }
}

/// Resolves one wiki link against the workspace per R-8: file matching first,
/// then heading semantics within the resolved file.
fn resolve_wiki_link(
    fact: &WikiLinkFact,
    source: DocumentId,
    wiki_docs: &[WikiDoc],
    doc_roots: &[NodeId],
    headings_by_doc: &HashMap<DocumentId, Vec<(NodeId, String, String)>>,
) -> (WikiResolution, Option<WikiInfo>) {
    if fact.unsupported {
        return (WikiResolution::Unsupported, None);
    }

    // A headings-only `[[#heading]]` targets the source document.
    if fact.target.is_empty() {
        return match &fact.heading {
            None => (WikiResolution::EmptyTarget, None),
            Some(heading) => resolve_in_document(
                source,
                doc_roots[source.0 as usize],
                heading,
                headings_by_doc,
                None,
            ),
        };
    }

    let parsed = match wiki::parse_file_target(&fact.target) {
        ParseOutcome::Empty => return (WikiResolution::EmptyTarget, None),
        ParseOutcome::Rejected => return (WikiResolution::Unresolved, None),
        ParseOutcome::Target(parsed) => parsed,
    };
    let percent_info = parsed
        .invalid_percent
        .then_some(WikiInfo::InvalidPercentEscape);

    match wiki::resolve_file(&parsed, wiki_docs, source.0 as usize) {
        Match::None => (WikiResolution::Unresolved, percent_info),
        Match::Many(indices) => {
            let candidates = indices
                .into_iter()
                .map(|index| doc_roots[index])
                .collect();
            (WikiResolution::Ambiguous(candidates), percent_info)
        }
        Match::One(index) => {
            let target_doc = DocumentId(index as u32);
            let file_root = doc_roots[index];
            // A file whose name keeps a Markdown extension after elision.
            let confusing = wiki_docs[index]
                .canonical
                .last()
                .is_some_and(|last| wiki::has_markdown_extension(last))
                .then_some(WikiInfo::ConfusingExtension);
            let base_info = percent_info.or(confusing);
            match &fact.heading {
                None => (WikiResolution::Resolved(file_root), base_info),
                Some(heading) => {
                    resolve_in_document(target_doc, file_root, heading, headings_by_doc, base_info)
                }
            }
        }
    }
}

/// Resolves a heading fragment inside a known document (R-8 heading semantics):
/// exact visible text first, GitHub slug fallback, exact wins on conflict.
fn resolve_in_document(
    document: DocumentId,
    file_root: NodeId,
    heading: &str,
    headings_by_doc: &HashMap<DocumentId, Vec<(NodeId, String, String)>>,
    base_info: Option<WikiInfo>,
) -> (WikiResolution, Option<WikiInfo>) {
    if heading.is_empty() {
        return (WikiResolution::EmptyHeading(Some(file_root)), base_info);
    }
    let headings = match headings_by_doc.get(&document) {
        Some(headings) => headings,
        None => return (WikiResolution::HeadingMissing(file_root), base_info),
    };
    let query = wiki::nfc(heading);
    let exact: Vec<NodeId> = headings
        .iter()
        .filter(|(_, title, _)| wiki::nfc(title) == query)
        .map(|(node, _, _)| *node)
        .collect();
    let slug: Vec<NodeId> = headings
        .iter()
        .filter(|(_, _, slug)| *slug == query)
        .map(|(node, _, _)| *node)
        .collect();

    if exact.len() == 1 {
        let conflict = slug.iter().any(|node| *node != exact[0]);
        let info = base_info.or(conflict.then_some(WikiInfo::HeadingSpellingConflict));
        (WikiResolution::Resolved(exact[0]), info)
    } else if slug.len() == 1 && exact.is_empty() {
        (WikiResolution::Resolved(slug[0]), base_info)
    } else {
        (WikiResolution::HeadingMissing(file_root), base_info)
    }
}

/// Groups documents whose canonical logical paths collide under case-fold/NFC
/// normalization, mapping each colliding document to its twins.
fn portability_collisions(documents: &[DocumentRecord]) -> HashMap<DocumentId, Vec<DocumentId>> {
    let mut by_key: HashMap<String, Vec<DocumentId>> = HashMap::new();
    for (position, record) in documents.iter().enumerate() {
        by_key
            .entry(wiki::portability_key(&record.canonical))
            .or_default()
            .push(DocumentId(position as u32));
    }
    let mut collisions = HashMap::new();
    for members in by_key.into_values() {
        if members.len() < 2 {
            continue;
        }
        for &member in &members {
            let twins: Vec<DocumentId> =
                members.iter().copied().filter(|&other| other != member).collect();
            collisions.insert(member, twins);
        }
    }
    collisions
}

/// Lexically joins `rel` onto `base_dir` and normalizes `.`/`..` without any
/// filesystem access (cross-platform, deterministic, and safe for open
/// buffers that may not exist on disk).
///
/// A `rel` starting with `/` resets to the base's root component.
pub(crate) fn normalize_join(base_dir: &Path, rel: &str) -> PathBuf {
    use std::path::Component;

    let mut components: Vec<std::ffi::OsString> = Vec::new();
    let mut root: Option<PathBuf> = None;

    let rel_is_absolute = rel.starts_with('/');
    if !rel_is_absolute {
        for component in base_dir.components() {
            match component {
                Component::Prefix(_) | Component::RootDir => {
                    root.get_or_insert_with(PathBuf::new).push(component.as_os_str());
                }
                Component::CurDir => {}
                Component::ParentDir => {
                    components.pop();
                }
                Component::Normal(segment) => components.push(segment.to_os_string()),
            }
        }
    } else {
        // Preserve the base's root (drive/prefix) but drop its directories.
        for component in base_dir.components() {
            match component {
                Component::Prefix(_) | Component::RootDir => {
                    root.get_or_insert_with(PathBuf::new).push(component.as_os_str());
                }
                _ => break,
            }
        }
    }

    for raw_segment in rel.split('/') {
        match raw_segment {
            "" | "." => {}
            ".." => {
                components.pop();
            }
            segment => components.push(std::ffi::OsString::from(segment)),
        }
    }

    let mut result = root.unwrap_or_default();
    for segment in components {
        result.push(segment);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::edge::EdgeTarget;
    use crate::graph::substrate::index_document;

    fn graph(docs: &[(&str, &str)]) -> WorkspaceGraph {
        let mut indices = BTreeMap::new();
        for (path, source) in docs {
            let path = PathBuf::from(path);
            indices.insert(path.clone(), index_document(&path, source));
        }
        WorkspaceGraph::build(&indices, 1)
    }

    #[test]
    fn test_document_and_heading_nodes() {
        let g = graph(&[("/w/a.md", "# Title\n\n## Sub\n")]);
        assert_eq!(g.document_count(), 1);
        let doc = g.document_id(Path::new("/w/a.md")).unwrap();
        assert_eq!(g.headings(doc).count(), 2);
        // Each heading contributes defines_anchor + defines_symbol.
        let root = g.document(doc).unwrap().root;
        assert_eq!(g.outgoing(root, EdgeKind::DefinesAnchor).count(), 2);
        assert_eq!(g.outgoing(root, EdgeKind::DefinesSymbol).count(), 2);
    }

    #[test]
    fn test_same_document_anchor_resolves() {
        let g = graph(&[("/w/a.md", "# Overview\n\n[jump](#overview)\n")]);
        let doc = g.document_id(Path::new("/w/a.md")).unwrap();
        let (heading_id, _) = g.headings(doc).next().unwrap();
        // The heading has one incoming `references` edge (the link).
        assert_eq!(g.incoming(heading_id, EdgeKind::References).count(), 1);
    }

    #[test]
    fn test_cross_document_path_and_fragment_resolve() {
        let g = graph(&[
            ("/w/a.md", "[to b](b.md#target)\n[to b root](b.md)\n"),
            ("/w/b.md", "# Target\n"),
        ]);
        let a = g.document_id(Path::new("/w/a.md")).unwrap();
        let b = g.document_id(Path::new("/w/b.md")).unwrap();
        let b_root = g.document(b).unwrap().root;
        let (b_heading, _) = g.headings(b).next().unwrap();
        // Fragment link points at the heading; bare link points at the root.
        assert_eq!(g.incoming(b_heading, EdgeKind::References).count(), 1);
        assert_eq!(g.incoming(b_root, EdgeKind::References).count(), 1);
        // Both link nodes live in document a.
        let a_links = g
            .outgoing(g.document(a).unwrap().root, EdgeKind::References)
            .count();
        assert_eq!(a_links, 0); // links are their own nodes, not doc-root edges
    }

    #[test]
    fn test_broken_link_stays_unresolved() {
        let g = graph(&[("/w/a.md", "[missing](nope.md)\n")]);
        let unresolved: Vec<_> = (0..g.edge_count())
            .filter_map(|i| g.edge(EdgeId(i as u32)))
            .filter(|e| matches!(&e.target, EdgeTarget::Unresolved(raw) if raw == "nope.md"))
            .collect();
        assert_eq!(unresolved.len(), 1);
    }

    #[test]
    fn test_normalize_join_parent_traversal() {
        assert_eq!(
            normalize_join(Path::new("/w/docs"), "../notes/x.md"),
            PathBuf::from("/w/notes/x.md")
        );
        assert_eq!(
            normalize_join(Path::new("/w/docs"), "./y.md"),
            PathBuf::from("/w/docs/y.md")
        );
    }

    #[test]
    fn test_transclusion_edge_resolves_to_target_document() {
        let g = graph(&[
            ("/w/a.md", "# A\n\n::file ./b.md\n"),
            ("/w/b.md", "# B\n"),
        ]);
        let a = g.document_id(Path::new("/w/a.md")).unwrap();
        let b = g.document_id(Path::new("/w/b.md")).unwrap();
        // A has one transclusion-target node pointing at B's root.
        assert_eq!(g.transclusions(a).count(), 1);
        assert_eq!(g.transcluded_by(b).len(), 1);
    }

    #[test]
    fn test_transclusion_of_non_document_target_carries_no_edge() {
        // `::code ./mod.rs` names a non-`.md` file: a transclusion node exists but
        // no `transcludes` edge (nothing to resolve in the document graph).
        let g = graph(&[("/w/a.md", "::code ./mod.rs\n")]);
        let a = g.document_id(Path::new("/w/a.md")).unwrap();
        assert_eq!(g.transclusions(a).count(), 1);
        let (node_id, _) = g.transclusions(a).next().unwrap();
        assert_eq!(g.outgoing(node_id, EdgeKind::Transcludes).count(), 0);
    }

    #[test]
    fn test_transclusion_cycle_detected() {
        let g = graph(&[
            ("/w/a.md", "::file ./b.md\n"),
            ("/w/b.md", "::file ./a.md\n"),
        ]);
        let a = g.document_id(Path::new("/w/a.md")).unwrap();
        let cycle = g.transclusion_cycle(a).expect("cycle exists");
        // The chain closes back on the starting document.
        assert_eq!(cycle.first(), Some(&a));
        assert_eq!(cycle.last(), Some(&a));
    }

    #[test]
    fn test_no_transclusion_cycle_for_acyclic_chain() {
        let g = graph(&[
            ("/w/a.md", "::file ./b.md\n"),
            ("/w/b.md", "# B\n"),
        ]);
        let a = g.document_id(Path::new("/w/a.md")).unwrap();
        assert!(g.transclusion_cycle(a).is_none());
    }

    #[test]
    fn test_uses_schema_edge_emitted() {
        let g = graph(&[("/w/a.md", "---\n$schema: ./schema.yaml\n---\n\n# Title\n")]);
        let a = g.document_id(Path::new("/w/a.md")).unwrap();
        assert_eq!(g.schema_uses(a).count(), 1);
        let (node_id, node) = g.schema_uses(a).next().unwrap();
        assert_eq!(node.as_file_ref().unwrap().path, "./schema.yaml");
        let edges: Vec<_> = g.outgoing(node_id, EdgeKind::UsesSchema).collect();
        assert_eq!(edges.len(), 1);
        // A schema YAML is not a `.md` graph document, so the target stays raw.
        assert!(matches!(edges[0].target, EdgeTarget::Unresolved(_)));
    }

    #[test]
    fn test_uses_file_edges_emitted_for_each_source() {
        // Frontmatter `file(...)` value (inline-schema-declared), a style asset,
        // a body image, and a `::file-links` directive path.
        let source = "---\n$schema:\n  cover: \"file -> art\"\ncover: ./art/cover.png\n\
                      style:\n  page:\n    stylesheet: ./theme.css\n---\n\n\
                      # Doc\n\n![diagram](./img/d.png)\n\n::file-links ./notes\n";
        let g = graph(&[("/w/a.md", source)]);
        let a = g.document_id(Path::new("/w/a.md")).unwrap();
        assert_eq!(g.file_uses(a).count(), 4);
        for (node_id, _) in g.file_uses(a) {
            assert_eq!(g.outgoing(node_id, EdgeKind::UsesFile).count(), 1);
        }
    }

    #[test]
    fn test_uses_file_resolves_to_indexed_document_and_reverse_index() {
        let g = graph(&[
            ("/w/a.md", "---\n$schema:\n  include: \"file\"\ninclude: ./b.md\n---\n\n# A\n"),
            ("/w/b.md", "# B\n"),
        ]);
        let a = g.document_id(Path::new("/w/a.md")).unwrap();
        let b = g.document_id(Path::new("/w/b.md")).unwrap();
        let (node_id, _) = g.file_uses(a).next().expect("one file use");
        let edge = g.outgoing(node_id, EdgeKind::UsesFile).next().expect("one uses_file edge");
        let b_root = g.document(b).unwrap().root;
        // The `file(...)` value names an indexed document, so it resolves to b's
        // root and is visible as an incoming edge (invalidation fan-out).
        assert_eq!(edge.target.node(), Some(b_root));
        assert_eq!(g.incoming(b_root, EdgeKind::UsesFile).count(), 1);
    }

    #[test]
    fn test_uses_file_edge_for_extensionless_file_value() {
        // A schema-typed `file` value that is a bare extensionless filename
        // (`LICENSE`) must still emit a `uses_file` edge — the dot/slash
        // heuristic must not drop it once the schema confirms the `file` type.
        let g = graph(&[(
            "/w/a.md",
            "---\n$schema:\n  license: \"file\"\nlicense: LICENSE\n---\n\n# A\n",
        )]);
        let a = g.document_id(Path::new("/w/a.md")).unwrap();
        let (node_id, node) = g.file_uses(a).next().expect("one file use");
        assert_eq!(node.as_file_ref().unwrap().path, "LICENSE");
        assert_eq!(g.outgoing(node_id, EdgeKind::UsesFile).count(), 1);
    }

    #[test]
    fn test_uses_variable_edges_emitted_and_reverse_index() {
        let g = graph(&[(
            "/w/a.md",
            "---\ntitle: Hello\n---\n\n# Body\n\nSee {{ title }} and {{ ctx.today }}.\n",
        )]);
        let a = g.document_id(Path::new("/w/a.md")).unwrap();
        assert_eq!(g.variable_uses(a).count(), 2);
        // `{{ title }}` resolves to the `title` frontmatter-key node; the reverse
        // index answers "who uses key `title`".
        let (title_key, _) = g
            .frontmatter_keys(a)
            .find(|(_, node)| node.as_frontmatter_key().unwrap().key == "title")
            .expect("a title key node");
        assert_eq!(g.incoming(title_key, EdgeKind::UsesVariable).count(), 1);
        // `{{ ctx.today }}` names an external namespace: unresolved, still recorded.
        let unresolved = g
            .variable_uses(a)
            .filter(|(id, _)| {
                g.outgoing(*id, EdgeKind::UsesVariable)
                    .any(|edge| matches!(edge.target, EdgeTarget::Unresolved(_)))
            })
            .count();
        assert_eq!(unresolved, 1);
    }

    #[test]
    fn test_interpolation_literal_creates_no_graph_node_or_edge() {
        let g = graph(&[(
            "/w/a.md",
            "---\ntitle: Hello\n---\n\n# Body\n\nSee {{{ title }}} and {{{ ctx.today }}}.\n",
        )]);
        let a = g.document_id(Path::new("/w/a.md")).unwrap();

        assert_eq!(g.variable_uses(a).count(), 0);
        let uses_variable_edges = (0..g.node_count())
            .map(|index| g.outgoing(NodeId(index as u32), EdgeKind::UsesVariable).count())
            .sum::<usize>();
        assert_eq!(uses_variable_edges, 0);
        assert_eq!(
            (0..g.node_count())
                .filter_map(|index| g.node(NodeId(index as u32)))
                .filter(|node| node.kind == NodeKind::Interpolation)
                .count(),
            0
        );
    }

    #[test]
    fn test_deterministic_document_ids_regardless_of_insert_order() {
        let g1 = graph(&[("/w/a.md", "# A\n"), ("/w/b.md", "# B\n")]);
        let g2 = graph(&[("/w/b.md", "# B\n"), ("/w/a.md", "# A\n")]);
        assert_eq!(
            g1.document_id(Path::new("/w/a.md")),
            g2.document_id(Path::new("/w/a.md"))
        );
    }
}
