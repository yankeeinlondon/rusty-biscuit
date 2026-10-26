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

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use biscuit_file::{FileResolutionContext, ResolutionFailure};

use super::edge::{Edge, EdgeId, EdgeKind};
use super::index::ReverseIndex;
use super::key_index::KeyIndex;
use super::node::{
    FileRefPayload, FrontmatterKeyPayload, HeadingPayload, LinkPayload, LinkTarget, Node, NodeId,
    NodeKind, NodePayload, TransclusionPayload, VariableUsePayload, WikiInfo, WikiLinkPayload,
    WikiResolution,
};
use super::substrate::{DocumentIndex, WikiLinkFact};
use crate::context::{ContextFailure, ReferenceTarget, plan_reference, resolve_reference};
use crate::wiki::{self, Match, ParseOutcome, WikiDoc, WikiUniverse};

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
    /// Whether the document's contents were parsed. An unparsed document is
    /// a file-list entry: it has a root node (links and wiki links land on
    /// it) but no headings or outgoing references yet, and
    /// `content_hash` is 0.
    pub parsed: bool,
    /// Whether the document is a referenced dependency: parsed because an
    /// open document's link, heading wiki link, or transclusion resolved to
    /// it, while it is neither in the file list nor open. It carries only
    /// its headings and outgoing transclusions, and it is never a wiki
    /// candidate, path completion, workspace symbol, or transclusion
    /// reference, so answers drawn from the file list are unchanged by it.
    pub dependency: bool,
}

/// Whether `path` names a Markdown file (`.md` or `.markdown`, any case): the
/// only kind of resolved target DMLS parses as a dependency, so a `::code`
/// target in another format is never read as Markdown.
pub fn is_markdown_path(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(wiki::has_markdown_extension)
}

/// Whether the workspace file list behind a snapshot is ready.
///
/// Diagnostics that depend on which paths exist wait for
/// [`Listing::Ready`]; an incomplete list never proves a target missing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Listing {
    /// The startup walk has not finished.
    Pending,
    /// The walk finished. `complete` is false when part of the workspace
    /// could not be listed.
    Ready {
        /// Every directory was listed.
        complete: bool,
    },
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
    wiki: WikiSlots,
    listing: Listing,
    generation: u64,
    /// Every planned target of a path reference that resolved to nothing.
    awaited: HashSet<PathBuf>,
}

/// The wiki universe a snapshot resolved against, and how its slots map to
/// the snapshot's documents.
///
/// Invariant: every document has a slot. A document missing from the
/// supplied universe is appended to it at build time (a dependency as a slot
/// no target matches), so a universe derived from the documents themselves
/// resolves exactly as the graph did before the universe was external.
#[derive(Debug, Clone)]
struct WikiSlots {
    universe: Arc<WikiUniverse>,
    /// Indexed by `DocumentId`.
    document_slots: Vec<usize>,
    slot_documents: HashMap<usize, DocumentId>,
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
    wiki: WikiSlots,
    listing: Listing,
    generation: u64,
    /// Every planned target of a path reference that resolved to nothing
    /// (see [`WorkspaceGraph::awaits`]).
    awaited: HashSet<PathBuf>,
}

impl WorkspaceGraph {
    /// Assembles a snapshot from per-document parse products whose wiki
    /// universe is exactly those documents, with no wiki roots (wiki
    /// resolution falls back to absolute-path tails; see
    /// [`WorkspaceGraph::build_with_roots`]).
    pub fn build(
        indices: &BTreeMap<PathBuf, DocumentIndex>,
        generation: u64,
        contexts: &dyn DocumentContexts,
    ) -> Self {
        let universe = WikiUniverse::new(indices.keys().cloned(), &[]);
        Self::build_with_roots(indices, generation, universe, contexts)
    }

    /// Assembles a snapshot, resolving wiki links against `universe`.
    ///
    /// `indices` is keyed by logical path (a `BTreeMap` so document ids are
    /// assigned in a deterministic, path-sorted order regardless of discovery
    /// order). `generation` stamps the snapshot for supersession checks.
    /// `universe` is every file a wiki link may name, canonicalized against
    /// its wiki roots (R-8); it may be far larger than `indices`, so a graph
    /// over a few documents judges ambiguity like a graph over all of them.
    /// Path references (links, transclusions, `$schema` and file uses)
    /// resolve through each document's context from `contexts`; a document
    /// without one resolves none of them.
    pub fn build_with_roots(
        indices: &BTreeMap<PathBuf, DocumentIndex>,
        generation: u64,
        universe: impl Into<Arc<WikiUniverse>>,
        contexts: &dyn DocumentContexts,
    ) -> Self {
        Self::finalize(Self::assemble(indices, generation, universe.into(), contexts))
    }

    /// Assembles a snapshot over parsed documents plus `unparsed` file-list
    /// entries (each a document with a root node and no contents), under the
    /// file list's `listing` state. Each parsed document in `dependencies`
    /// becomes a [dependency](DocumentRecord::dependency).
    ///
    /// [`build`](Self::build) and [`build_with_roots`](Self::build_with_roots)
    /// are this with no unparsed entries, no dependencies, and a complete
    /// listing.
    pub fn build_listed(
        indices: &BTreeMap<PathBuf, DocumentIndex>,
        unparsed: &BTreeSet<PathBuf>,
        dependencies: &BTreeSet<PathBuf>,
        listing: Listing,
        generation: u64,
        universe: impl Into<Arc<WikiUniverse>>,
        contexts: &dyn DocumentContexts,
    ) -> Self {
        let mut assembly = Self::assemble_listed(
            indices,
            unparsed,
            dependencies,
            generation,
            universe.into(),
            contexts,
        );
        assembly.listing = listing;
        Self::finalize(assembly)
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
        universe: Arc<WikiUniverse>,
        contexts: &dyn DocumentContexts,
    ) -> GraphAssembly {
        let none = BTreeSet::new();
        Self::assemble_listed(indices, &none, &none, generation, universe, contexts)
    }

    /// [`assemble`](Self::assemble) over parsed documents plus unparsed
    /// file-list entries, all assigned ids in one path-sorted order.
    fn assemble_listed(
        indices: &BTreeMap<PathBuf, DocumentIndex>,
        unparsed: &BTreeSet<PathBuf>,
        dependencies: &BTreeSet<PathBuf>,
        generation: u64,
        mut universe: Arc<WikiUniverse>,
        contexts: &dyn DocumentContexts,
    ) -> GraphAssembly {
        let mut entries: Vec<(&PathBuf, Option<&DocumentIndex>)> = indices
            .iter()
            .map(|(path, index)| (path, Some(index)))
            .chain(
                unparsed
                    .iter()
                    .filter(|path| !indices.contains_key(*path))
                    .map(|path| (path, None)),
            )
            .collect();
        entries.sort_by(|left, right| left.0.cmp(right.0));
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
        let mut document_slots: Vec<usize> = Vec::with_capacity(entries.len());
        let mut slot_documents: HashMap<usize, DocumentId> = HashMap::with_capacity(entries.len());

        for (position, (path, index)) in entries.into_iter().enumerate() {
            let doc_id = DocumentId(position as u32);
            by_path.insert(path.clone(), doc_id);
            let dependency = index.is_some() && dependencies.contains(path);
            let WikiDoc { canonical, workspace_id } = WikiDoc::for_path(path, universe.wiki_roots());
            let slot = match universe.slot(path) {
                Some(slot) => slot,
                None => {
                    let doc = if dependency {
                        WikiDoc::unmatchable()
                    } else {
                        WikiDoc { canonical: canonical.clone(), workspace_id }
                    };
                    Arc::make_mut(&mut universe).push(path.clone(), doc)
                }
            };
            document_slots.push(slot);
            slot_documents.insert(slot, doc_id);

            let root = NodeId(nodes.len() as u32);
            nodes.push(Node {
                document: doc_id,
                kind: NodeKind::Document,
                span: 0..0,
                payload: NodePayload::Document,
            });
            documents.push(DocumentRecord {
                path: path.clone(),
                content_hash: index.map_or(0, |index| index.content_hash),
                root,
                canonical,
                workspace_id,
                parsed: index.is_some(),
                dependency,
            });
            let Some(index) = index else {
                headings_by_doc.insert(doc_id, Vec::new());
                continue;
            };

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

            // A dependency is read for its headings and transclusions only;
            // its other references would add backlinks and file uses that
            // the file list does not contain.
            let links: &[_] = if dependency { &[] } else { &index.links };
            let wiki_links: &[_] = if dependency { &[] } else { &index.wiki_links };

            for link in links {
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

            for wiki in wiki_links {
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
            if dependency {
                continue;
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
        let mut resolver = ReferenceResolver::new(contexts, &documents, &by_path);

        // Each heading defines an anchor and a symbol on its document.
        for (root, heading) in heading_nodes {
            edges.push(Edge::to_node(root, EdgeKind::DefinesAnchor, heading));
            edges.push(Edge::to_node(root, EdgeKind::DefinesSymbol, heading));
        }

        // Resolve every Markdown link into a `references` edge.
        for (source, doc_id, target, raw) in pending_links {
            let resolved = resolve_link(&target, doc_id, &mut resolver, &heading_by_slug);
            match resolved {
                LinkResolution::Node(node) => {
                    edges.push(Edge::to_node(source, EdgeKind::References, node));
                }
                LinkResolution::File(path) => {
                    edges.push(Edge::to_file(source, EdgeKind::References, path));
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
        let wiki = WikiSlots {
            universe,
            document_slots,
            slot_documents,
        };
        let doc_roots: Vec<NodeId> = documents.iter().map(|record| record.root).collect();
        let parsed: Vec<bool> = documents.iter().map(|record| record.parsed).collect();
        for (source, doc_id, fact) in pending_wiki {
            let (resolution, info) = resolve_wiki_link(
                &fact,
                doc_id,
                &wiki,
                &doc_roots,
                &headings_by_doc,
                &parsed,
            );
            for target in wiki_edge_targets(&resolution) {
                edges.push(Edge::to_node(source, EdgeKind::References, target));
            }
            if let NodePayload::WikiLink(payload) = &mut nodes[source.0 as usize].payload {
                payload.resolution = resolution;
                payload.info = info;
            }
        }

        // Resolve `::file`/`::code` transclusions to the graph document's
        // root, or to an existing file outside the graph (`::code ./mod.rs`). A broken
        // path carries no edge; request-time diagnostics report it.
        for (source, doc_id, path) in pending_transclusions {
            match resolver.locate(doc_id, &path) {
                Some(Located::Document(target_doc)) => {
                    if let Some(record) = documents.get(target_doc.0 as usize) {
                        edges.push(Edge::to_node(source, EdgeKind::Transcludes, record.root));
                    }
                }
                Some(Located::File(file)) => {
                    edges.push(Edge::to_file(source, EdgeKind::Transcludes, file));
                }
                Some(Located::Missing(_)) | None => {}
            }
        }

        // Resolve `$schema` (uses_schema) and file uses (uses_file). A value
        // naming a graph document resolves to its root, feeding invalidation
        // fan-out; an existing schema file or asset resolves to that file.
        for (source, doc_id, path) in pending_schema_uses {
            edges.push(resolve_file_edge(source, EdgeKind::UsesSchema, doc_id, &path, &mut resolver, &documents));
        }
        for (source, doc_id, path) in pending_file_uses {
            edges.push(resolve_file_edge(source, EdgeKind::UsesFile, doc_id, &path, &mut resolver, &documents));
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
            awaited: std::mem::take(&mut resolver.awaited),
            documents,
            by_path,
            nodes,
            edges,
            wiki,
            listing: Listing::Ready { complete: true },
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
            wiki,
            listing,
            generation,
            awaited,
        } = assembly;
        let reverse = ReverseIndex::build(&edges);
        let keys = KeyIndex::build(
            documents
                .iter()
                .enumerate()
                .filter(|(_, record)| !record.dependency)
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
            wiki,
            listing,
            generation,
            awaited,
        }
    }

    /// The snapshot generation.
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// Whether a path reference in this snapshot resolved to nothing and
    /// would resolve to `path` once it exists: the one case in which a file
    /// the index does not hold changes the graph by appearing.
    pub fn awaits(&self, path: &Path) -> bool {
        self.awaited.contains(path)
    }

    /// The state of the file list behind this snapshot.
    pub fn listing(&self) -> Listing {
        self.listing
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
    /// document of every incoming `transcludes` edge to the document's root,
    /// except a [dependency](DocumentRecord::dependency).
    pub fn transcluded_by(&self, document: DocumentId) -> Vec<NodeId> {
        let Some(root) = self.document(document).map(|record| record.root) else {
            return Vec::new();
        };
        self.incoming(root, EdgeKind::Transcludes)
            .map(|edge| edge.source)
            .filter(|source| {
                self.node(*source)
                    .and_then(|node| self.document(node.document))
                    .is_some_and(|record| !record.dependency)
            })
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

    /// The wiki universe this snapshot resolved wiki links against. It holds
    /// every document of the snapshot and may hold files that are not.
    pub fn wiki_universe(&self) -> &WikiUniverse {
        &self.wiki.universe
    }

    /// [`wiki_universe`](Self::wiki_universe), shared: what a request-scoped
    /// graph over a few of these documents resolves wiki links against.
    pub fn shared_wiki_universe(&self) -> Arc<WikiUniverse> {
        Arc::clone(&self.wiki.universe)
    }

    /// `document`'s slot in [`wiki_universe`](Self::wiki_universe).
    pub fn wiki_slot(&self, document: DocumentId) -> Option<usize> {
        self.wiki.document_slots.get(document.0 as usize).copied()
    }

    /// The document holding universe `slot`, or `None` when that file is
    /// not a document of this snapshot.
    pub fn slot_document(&self, slot: usize) -> Option<DocumentId> {
        self.wiki.slot_documents.get(&slot).copied()
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
    /// `context` is the source document's file-resolution context; a path
    /// target is resolved through it exactly as the graph build resolves
    /// links.
    ///
    /// Returns `None` for external targets, for targets that actually
    /// resolve (those carry a resolved [`EdgeTarget::Node`] instead), and for
    /// path targets when the document has no context (its context-failure
    /// diagnostic stands in for every reference).
    pub fn diagnose_unresolved(
        &self,
        source: DocumentId,
        target: &LinkTarget,
        context: Result<&FileResolutionContext, &ContextFailure>,
    ) -> Option<LinkDiagnostic> {
        match target {
            LinkTarget::External => None,
            LinkTarget::SameDocumentAnchor { slug } => {
                self.heading_in(source, slug).is_none().then_some(LinkDiagnostic::MissingAnchor)
            }
            LinkTarget::RelativePath { path, fragment } => {
                let context = context.ok()?;
                match locate(context, path, &self.by_path) {
                    Located::Missing(failure) => Some(LinkDiagnostic::BrokenPath { failure }),
                    // An unindexed file's headings are unknown, so its anchor
                    // is not checked.
                    Located::File(_) => None,
                    Located::Document(target_doc) => match fragment {
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
    /// The path names no existing file and no graph document.
    BrokenPath {
        /// The failure class: [`ResolutionFailure::NoMatch`] when no planned
        /// candidate exists, otherwise why the reference could not be
        /// resolved.
        failure: ResolutionFailure,
    },
    /// The document resolved but the `#fragment`/anchor did not.
    MissingAnchor,
}

/// Outcome of resolving a link target to a graph node.
enum LinkResolution {
    Node(NodeId),
    File(PathBuf),
    Unresolved,
    External,
}

fn resolve_link(
    target: &LinkTarget,
    source_doc: DocumentId,
    resolver: &mut ReferenceResolver<'_>,
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
            let target_doc = match resolver.locate(source_doc, path) {
                Some(Located::Document(target_doc)) => target_doc,
                Some(Located::File(file)) => return LinkResolution::File(file),
                Some(Located::Missing(_)) | None => return LinkResolution::Unresolved,
            };
            let Some(record) = resolver.documents.get(target_doc.0 as usize) else {
                return LinkResolution::Unresolved;
            };
            match fragment {
                // An unparsed target's headings are unknown, so its anchor is
                // not checked; the link lands on the document until a feature
                // needs the target parsed.
                Some(_) if !record.parsed => LinkResolution::Node(record.root),
                Some(slug) => match heading_by_slug.get(&(target_doc, slug.clone())) {
                    Some(node) => LinkResolution::Node(*node),
                    None => LinkResolution::Unresolved,
                },
                None => LinkResolution::Node(record.root),
            }
        }
    }
}

/// Resolves a `uses_schema`/`uses_file` reference to an edge: to an indexed
/// Markdown document's root when `path` names one, to the file when it names
/// an existing unindexed file (schema YAML, an asset), else an `Unresolved`
/// edge carrying the raw path.
fn resolve_file_edge(
    source: NodeId,
    kind: EdgeKind,
    source_doc: DocumentId,
    path: &str,
    resolver: &mut ReferenceResolver<'_>,
    documents: &[DocumentRecord],
) -> Edge {
    match resolver.locate(source_doc, path) {
        Some(Located::Document(target_doc)) => match documents.get(target_doc.0 as usize) {
            Some(record) => Edge::to_node(source, kind, record.root),
            None => Edge::unresolved(source, kind, path.to_string()),
        },
        Some(Located::File(file)) => Edge::to_file(source, kind, file),
        Some(Located::Missing(_)) | None => Edge::unresolved(source, kind, path.to_string()),
    }
}

/// Supplies each document's file-resolution context to the graph build.
///
/// DMLS's [`RepositoryContexts`](crate::context::RepositoryContexts) is the
/// production source. The graph never builds or guesses a context itself.
pub trait DocumentContexts: Send + Sync + std::fmt::Debug {
    /// The context references in `document` resolve against, or why the
    /// document has none.
    fn context_for(&self, document: &Path) -> Result<FileResolutionContext, ContextFailure>;
}

/// A [`DocumentContexts`] with no context for any document: every path
/// reference stays unresolved. For graphs whose documents carry no path
/// references, such as an empty graph.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoContexts;

impl DocumentContexts for NoContexts {
    fn context_for(&self, _document: &Path) -> Result<FileResolutionContext, ContextFailure> {
        Err(ContextFailure::NotProvided)
    }
}

/// Where a path reference lands.
#[derive(Debug, Clone)]
enum Located {
    /// A graph document (listed or open, parsed or not).
    Document(DocumentId),
    /// An existing file that is not a graph document.
    File(PathBuf),
    /// Nothing: the failure class.
    Missing(ResolutionFailure),
}

/// Locates `raw` the way composition resolves it, so an existing file is
/// never reported missing because the workspace folder does not index it.
///
/// The shared resolver ([`resolve_reference`]) decides existence. Two index
/// lookups keep it cheap and buffer-aware: a first planned candidate that is
/// indexed needs no probe, and a reference that matched no file (not one whose
/// probe failed) resolves to its first candidate that is an open, not yet
/// saved, buffer.
fn locate(
    context: &FileResolutionContext,
    raw: &str,
    by_path: &HashMap<PathBuf, DocumentId>,
) -> Located {
    let candidates = match plan_reference(context, raw) {
        Ok(candidates) => candidates,
        Err(failure) => return Located::Missing(failure),
    };
    if let Some(&document) = candidates.first().and_then(|candidate| by_path.get(candidate)) {
        return Located::Document(document);
    }
    match resolve_reference(context, raw) {
        ReferenceTarget::Found(path) => match by_path.get(&path) {
            Some(&document) => Located::Document(document),
            None => Located::File(path),
        },
        // A candidate that could not be probed stops the search, as it stops
        // composition; a later buffer never answers for it.
        ReferenceTarget::Missing { failure: ResolutionFailure::NoMatch, .. } => candidates
            .iter()
            .find_map(|candidate| by_path.get(candidate).copied())
            .map_or(Located::Missing(ResolutionFailure::NoMatch), Located::Document),
        ReferenceTarget::Missing { failure, .. } => Located::Missing(failure),
    }
}

/// Locates path references during one graph build (see [`locate`]).
///
/// Contexts are fetched once per document folder, since planning depends
/// only on the folder and the repository, and each `(folder, reference)`
/// pair is located once per build.
struct ReferenceResolver<'a> {
    contexts: &'a dyn DocumentContexts,
    documents: &'a [DocumentRecord],
    by_path: &'a HashMap<PathBuf, DocumentId>,
    by_folder: HashMap<PathBuf, Result<FileResolutionContext, ContextFailure>>,
    located: HashMap<(PathBuf, String), Option<Located>>,
    /// The planned candidates of every reference that located nothing.
    awaited: HashSet<PathBuf>,
}

impl<'a> ReferenceResolver<'a> {
    fn new(
        contexts: &'a dyn DocumentContexts,
        documents: &'a [DocumentRecord],
        by_path: &'a HashMap<PathBuf, DocumentId>,
    ) -> Self {
        Self {
            contexts,
            documents,
            by_path,
            by_folder: HashMap::new(),
            located: HashMap::new(),
            awaited: HashSet::new(),
        }
    }

    /// Where `raw`, authored in `source`, lands; `None` when the document has
    /// no context.
    fn locate(&mut self, source: DocumentId, raw: &str) -> Option<Located> {
        let path = &self.documents.get(source.0 as usize)?.path;
        let folder = path.parent().unwrap_or(path).to_path_buf();
        let key = (folder.clone(), raw.to_string());
        if let Some(located) = self.located.get(&key) {
            return located.clone();
        }
        let contexts = self.contexts;
        let context = self.by_folder.entry(folder).or_insert_with(|| contexts.context_for(path)).as_ref().ok();
        let located = context.map(|context| {
            let located = locate(context, raw, self.by_path);
            if matches!(located, Located::Missing(_))
                && let Ok(candidates) = plan_reference(context, raw)
            {
                self.awaited.extend(candidates);
            }
            located
        });
        self.located.insert(key, located.clone());
        located
    }
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
    wiki: &WikiSlots,
    doc_roots: &[NodeId],
    headings_by_doc: &HashMap<DocumentId, Vec<(NodeId, String, String)>>,
    parsed_docs: &[bool],
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

    let wiki_docs = wiki.universe.docs();
    let source_slot = wiki.document_slots[source.0 as usize];
    match wiki::resolve_file(&parsed, wiki_docs, source_slot) {
        Match::None => (WikiResolution::Unresolved, percent_info),
        Match::Many(slots) => {
            // Ambiguity is judged against the whole universe; only the
            // candidates that are documents of this snapshot get an edge.
            let candidates = slots
                .into_iter()
                .filter_map(|slot| wiki.slot_documents.get(&slot))
                .map(|document| doc_roots[document.0 as usize])
                .collect();
            (WikiResolution::Ambiguous(candidates), percent_info)
        }
        Match::One(slot) => {
            let Some(&target_doc) = wiki.slot_documents.get(&slot) else {
                return (WikiResolution::OutsideGraph, percent_info);
            };
            let index = target_doc.0 as usize;
            let file_root = doc_roots[index];
            // A file whose name keeps a Markdown extension after elision.
            let confusing = wiki_docs[slot]
                .canonical
                .last()
                .is_some_and(|last| wiki::has_markdown_extension(last))
                .then_some(WikiInfo::ConfusingExtension);
            let base_info = percent_info.or(confusing);
            match &fact.heading {
                None => (WikiResolution::Resolved(file_root), base_info),
                // An unparsed target's headings are unknown: the link lands on
                // the file until a feature needs the target parsed.
                Some(_) if !parsed_docs[index] => (WikiResolution::Resolved(file_root), base_info),
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
    for (position, record) in documents.iter().enumerate().filter(|(_, record)| !record.dependency) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::edge::EdgeTarget;
    use crate::context::test_support::{abs, workspace_contexts};
    use crate::graph::substrate::index_document;

    fn graph(docs: &[(&str, &str)]) -> WorkspaceGraph {
        let mut indices = BTreeMap::new();
        for (path, source) in docs {
            let path = abs(path);
            indices.insert(path.clone(), index_document(&path, source));
        }
        WorkspaceGraph::build(&indices, 1, &workspace_contexts())
    }

    #[test]
    fn test_document_and_heading_nodes() {
        let g = graph(&[("/w/a.md", "# Title\n\n## Sub\n")]);
        assert_eq!(g.document_count(), 1);
        let doc = g.document_id(&abs("/w/a.md")).unwrap();
        assert_eq!(g.headings(doc).count(), 2);
        // Each heading contributes defines_anchor + defines_symbol.
        let root = g.document(doc).unwrap().root;
        assert_eq!(g.outgoing(root, EdgeKind::DefinesAnchor).count(), 2);
        assert_eq!(g.outgoing(root, EdgeKind::DefinesSymbol).count(), 2);
    }

    #[test]
    fn test_same_document_anchor_resolves() {
        let g = graph(&[("/w/a.md", "# Overview\n\n[jump](#overview)\n")]);
        let doc = g.document_id(&abs("/w/a.md")).unwrap();
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
        let a = g.document_id(&abs("/w/a.md")).unwrap();
        let b = g.document_id(&abs("/w/b.md")).unwrap();
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
    fn test_transclusion_edge_resolves_to_target_document() {
        let g = graph(&[
            ("/w/a.md", "# A\n\n::file ./b.md\n"),
            ("/w/b.md", "# B\n"),
        ]);
        let a = g.document_id(&abs("/w/a.md")).unwrap();
        let b = g.document_id(&abs("/w/b.md")).unwrap();
        // A has one transclusion-target node pointing at B's root.
        assert_eq!(g.transclusions(a).count(), 1);
        assert_eq!(g.transcluded_by(b).len(), 1);
    }

    #[test]
    fn test_transclusion_of_non_document_target_carries_no_edge() {
        // `::code ./mod.rs` names a non-`.md` file: a transclusion node exists but
        // no `transcludes` edge (nothing to resolve in the document graph).
        let g = graph(&[("/w/a.md", "::code ./mod.rs\n")]);
        let a = g.document_id(&abs("/w/a.md")).unwrap();
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
        let a = g.document_id(&abs("/w/a.md")).unwrap();
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
        let a = g.document_id(&abs("/w/a.md")).unwrap();
        assert!(g.transclusion_cycle(a).is_none());
    }

    #[test]
    fn test_uses_schema_edge_emitted() {
        let g = graph(&[("/w/a.md", "---\n$schema: ./schema.yaml\n---\n\n# Title\n")]);
        let a = g.document_id(&abs("/w/a.md")).unwrap();
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
        let a = g.document_id(&abs("/w/a.md")).unwrap();
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
        let a = g.document_id(&abs("/w/a.md")).unwrap();
        let b = g.document_id(&abs("/w/b.md")).unwrap();
        let (node_id, _) = g.file_uses(a).next().expect("one file use");
        let edge = g.outgoing(node_id, EdgeKind::UsesFile).next().expect("one uses_file edge");
        let b_root = g.document(b).unwrap().root;
        // The `file(...)` value names a graph document, so it resolves to b's
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
        let a = g.document_id(&abs("/w/a.md")).unwrap();
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
        let a = g.document_id(&abs("/w/a.md")).unwrap();
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
        let a = g.document_id(&abs("/w/a.md")).unwrap();

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
    fn test_repository_sigils_resolve_through_the_document_context() {
        // `&` anchors on the context's repository root, never the document's
        // directory, for links, transclusions, and file uses alike.
        let mut indices = BTreeMap::new();
        for (path, source) in [
            ("/w/root.md", "# Root\n"),
            (
                "/w/docs/deep/a.md",
                "---\n$schema:\n  include: \"file\"\ninclude: \"&root.md\"\n---\n\n[root](&root.md)\n\n::file &root.md\n",
            ),
        ] {
            let path = abs(path);
            indices.insert(path.clone(), index_document(&path, source));
        }
        let contexts = crate::context::FixedContext(
            crate::context::test_support::workspace_context().with_repository_root(abs("/w")),
        );
        let g = WorkspaceGraph::build(&indices, 1, &contexts);
        let a = g.document_id(&abs("/w/docs/deep/a.md")).unwrap();
        let root = g.document(g.document_id(&abs("/w/root.md")).unwrap()).unwrap().root;
        assert_eq!(g.incoming(root, EdgeKind::References).count(), 1);
        assert_eq!(g.incoming(root, EdgeKind::Transcludes).count(), 1);
        assert_eq!(g.incoming(root, EdgeKind::UsesFile).count(), 1);

        // The same document with no context resolves none of them, and its
        // link diagnostic is withheld (the context failure stands in).
        let g = WorkspaceGraph::build(&indices, 1, &NoContexts);
        let root = g.document(g.document_id(&abs("/w/root.md")).unwrap()).unwrap().root;
        assert_eq!(g.incoming(root, EdgeKind::References).count(), 0);
        assert_eq!(g.incoming(root, EdgeKind::Transcludes).count(), 0);
        assert_eq!(g.incoming(root, EdgeKind::UsesFile).count(), 0);
        let (_, link) = g.links(a).next().unwrap();
        assert_eq!(
            g.diagnose_unresolved(a, &link.as_link().unwrap().target, Err(&ContextFailure::NotProvided)),
            None
        );
    }

    #[test]
    fn test_unplannable_link_reports_its_failure_class() {
        // `&` with no repository cannot be planned: a broken path whose class
        // is `MissingContext`, not `NoMatch`.
        let g = graph(&[("/w/a.md", "[root](&root.md)\n")]);
        let a = g.document_id(&abs("/w/a.md")).unwrap();
        let (_, link) = g.links(a).next().unwrap();
        let context = crate::context::test_support::resolution_for(abs("/w/a.md"));
        assert_eq!(
            g.diagnose_unresolved(a, &link.as_link().unwrap().target, context.context()),
            Some(LinkDiagnostic::BrokenPath { failure: ResolutionFailure::MissingContext })
        );
    }

    #[test]
    fn test_existing_unindexed_targets_resolve_to_their_files() {
        // Only `docs/` is indexed; the targets above it exist on disk. Each
        // resolves as composition resolves it, never as a broken path.
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join("docs")).unwrap();
        std::fs::write(root.join("target.md"), "# Target\n").unwrap();
        std::fs::write(root.join("schema.yaml"), "title: string\n").unwrap();
        let doc = root.join("docs/doc.md");
        let source = "---\n$schema: ../schema.yaml\n---\n\n[t](../target.md) [a](../target.md#nope) [m](../missing.md)\n\n::file ../target.md\n";
        let indices = BTreeMap::from([(doc.clone(), index_document(&doc, source))]);
        let contexts = crate::context::FixedContext(
            FileResolutionContext::from_snapshot(root, None, HashMap::new()),
        );
        let g = WorkspaceGraph::build(&indices, 1, &contexts);
        let id = g.document_id(&doc).unwrap();
        let target = root.join("target.md");
        let edge_targets = |kind| -> Vec<EdgeTarget> {
            (0..g.node_count())
                .flat_map(|index| g.outgoing(NodeId(index as u32), kind))
                .map(|edge| edge.target.clone())
                .collect()
        };
        assert_eq!(
            edge_targets(EdgeKind::References),
            vec![
                EdgeTarget::File(target.clone()),
                EdgeTarget::File(target.clone()),
                EdgeTarget::Unresolved("../missing.md".into()),
            ]
        );
        assert_eq!(edge_targets(EdgeKind::Transcludes), vec![EdgeTarget::File(target.clone())]);
        assert_eq!(edge_targets(EdgeKind::UsesSchema), vec![EdgeTarget::File(root.join("schema.yaml"))]);

        // An unindexed file's anchor is unknown, so only the missing file is
        // diagnosed.
        let context = contexts.0.for_source(&doc);
        let diagnoses: Vec<Option<LinkDiagnostic>> = g
            .links(id)
            .map(|(_, node)| g.diagnose_unresolved(id, &node.as_link().unwrap().target, Ok(&context)))
            .collect();
        assert_eq!(
            diagnoses,
            vec![None, None, Some(LinkDiagnostic::BrokenPath { failure: ResolutionFailure::NoMatch })]
        );
    }

    #[test]
    fn test_an_earlier_existing_candidate_outranks_a_later_indexed_one() {
        // Bare `t.md` in `docs/` plans `docs/t.md` before the root's `t.md`.
        // Composition takes the first that exists, so the graph must too,
        // even though only the root's copy is indexed.
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join("docs")).unwrap();
        std::fs::write(root.join("docs/t.md"), "# Near\n").unwrap();
        let doc = root.join("docs/doc.md");
        let far = root.join("t.md");
        let indices = BTreeMap::from([
            (doc.clone(), index_document(&doc, "[t](t.md)\n")),
            (far.clone(), index_document(&far, "# Far\n")),
        ]);
        let contexts = crate::context::FixedContext(
            FileResolutionContext::from_snapshot(root, None, HashMap::new()).with_repository_root(root),
        );
        let g = WorkspaceGraph::build(&indices, 1, &contexts);
        let id = g.document_id(&doc).unwrap();
        let (link, _) = g.links(id).next().unwrap();
        let targets: Vec<EdgeTarget> =
            g.outgoing(link, EdgeKind::References).map(|edge| edge.target.clone()).collect();
        assert_eq!(targets, vec![EdgeTarget::File(root.join("docs/t.md"))]);
    }

    #[test]
    fn test_an_unprobeable_earlier_candidate_is_not_answered_by_a_later_buffer() {
        // `docs/t.md` is a symlink to itself, so probing it fails with an I/O
        // error; the root's `t.md` is an open, unsaved buffer. Composition
        // stops at the failure, so the link stays unresolved.
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join("docs")).unwrap();
        let looped = root.join("docs/t.md");
        #[cfg(unix)]
        std::os::unix::fs::symlink("t.md", &looped).unwrap();
        #[cfg(windows)]
        std::os::windows::fs::symlink_file("t.md", &looped)
            .expect("creating a file symlink needs Developer Mode or SeCreateSymbolicLinkPrivilege");
        let doc = root.join("docs/doc.md");
        let buffer = root.join("t.md");
        let indices = BTreeMap::from([
            (doc.clone(), index_document(&doc, "[t](t.md)\n")),
            (buffer.clone(), index_document(&buffer, "# Buffer\n")),
        ]);
        let contexts = crate::context::FixedContext(
            FileResolutionContext::from_snapshot(root, None, HashMap::new()).with_repository_root(root),
        );
        let g = WorkspaceGraph::build(&indices, 1, &contexts);
        let id = g.document_id(&doc).unwrap();
        let (link, node) = g.links(id).next().unwrap();
        let targets: Vec<EdgeTarget> =
            g.outgoing(link, EdgeKind::References).map(|edge| edge.target.clone()).collect();
        assert_eq!(targets, vec![EdgeTarget::Unresolved("t.md".into())]);
        let context = contexts.0.for_source(&doc);
        assert_eq!(
            g.diagnose_unresolved(id, &node.as_link().unwrap().target, Ok(&context)),
            Some(LinkDiagnostic::BrokenPath { failure: ResolutionFailure::Io })
        );
    }

    #[test]
    fn test_deterministic_document_ids_regardless_of_insert_order() {
        let g1 = graph(&[("/w/a.md", "# A\n"), ("/w/b.md", "# B\n")]);
        let g2 = graph(&[("/w/b.md", "# B\n"), ("/w/a.md", "# A\n")]);
        assert_eq!(
            g1.document_id(&abs("/w/a.md")),
            g2.document_id(&abs("/w/a.md"))
        );
    }

    fn wiki_resolution(g: &WorkspaceGraph, path: &str) -> WikiResolution {
        let doc = g.document_id(&abs(path)).unwrap();
        let (_, node) = g.wiki_links(doc).next().unwrap();
        node.as_wiki_link().unwrap().resolution.clone()
    }

    fn indices(docs: &[(&str, &str)]) -> BTreeMap<PathBuf, DocumentIndex> {
        docs.iter()
            .map(|(path, source)| (abs(path), index_document(&abs(path), source)))
            .collect()
    }

    #[test]
    fn test_supplied_universe_judges_wiki_links_beyond_the_documents() {
        let docs = indices(&[("/w/src.md", "[[topic]] [[solo]]\n"), ("/w/a/topic.md", "# A\n")]);
        let universe = WikiUniverse::new(
            [abs("/w/src.md"), abs("/w/a/topic.md"), abs("/w/b/topic.md"), abs("/w/solo.md")],
            &[abs("/w")],
        );
        let g = WorkspaceGraph::build_with_roots(&docs, 1, universe, &workspace_contexts());
        let source = g.document_id(&abs("/w/src.md")).unwrap();
        let alpha_root = g.document(g.document_id(&abs("/w/a/topic.md")).unwrap()).unwrap().root;
        let resolutions: Vec<WikiResolution> = g
            .wiki_links(source)
            .map(|(_, node)| node.as_wiki_link().unwrap().resolution.clone())
            .collect();
        // The rival b/topic.md is not a document: still ambiguous, with an
        // edge to the one candidate that is.
        assert_eq!(resolutions[0], WikiResolution::Ambiguous(vec![alpha_root]));
        // solo.md is the unique match but not a document.
        assert_eq!(resolutions[1], WikiResolution::OutsideGraph);
        assert_eq!(g.incoming(alpha_root, EdgeKind::References).count(), 1);
        assert_eq!(g.slot_document(g.wiki_universe().slot(&abs("/w/solo.md")).unwrap()), None);
    }

    #[test]
    fn test_document_missing_from_the_universe_is_a_wiki_candidate() {
        // An open buffer outside the file list is still a candidate, as it was
        // when the graph derived its candidates from its own documents.
        let docs = indices(&[("/w/src.md", "[[extra]]\n"), ("/w/extra.md", "# Extra\n")]);
        let universe = WikiUniverse::new([abs("/w/src.md")], &[abs("/w")]);
        let g = WorkspaceGraph::build_with_roots(&docs, 1, universe, &workspace_contexts());
        let extra = g.document_id(&abs("/w/extra.md")).unwrap();
        assert_eq!(wiki_resolution(&g, "/w/src.md"), WikiResolution::Resolved(g.document(extra).unwrap().root));
        assert_eq!(g.wiki_universe().len(), 2);
        let slot = g.wiki_slot(extra).unwrap();
        assert_eq!(g.slot_document(slot), Some(extra));
        assert_eq!(g.wiki_universe().path(slot), Some(abs("/w/extra.md").as_path()));
    }
}
