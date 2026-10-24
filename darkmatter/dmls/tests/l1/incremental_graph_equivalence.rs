//! A live `WorkspaceIndex` snapshot, which may share its document table and
//! wiki universe with the snapshot before it, answers exactly like a graph
//! built fresh from the same inputs.
//!
//! [`Harness`] applies each transition to a live index and to an explicit
//! model of its inputs (listed paths, parse products, open buffers, stale and
//! unlisted reads, listing state, wiki roots). After every transition it builds
//! a reference with the public `WorkspaceGraph::build_listed` over the
//! model's inputs, with a wiki universe constructed from the model's
//! candidates, never taken from the live snapshot, and compares both graphs
//! through [`GraphFacts`]. Node identity is normalized to
//! `(document path, per-document ordinal, kind, span)`, so global node and
//! edge numbers and generations never enter a comparison.
//!
//! The request-graph test compares scoped graphs over the live universe and
//! over a freshly built one; the corpus test compares provider answers over a
//! live snapshot that reused its table with those of the test `Oracle`.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use biscuit_file::FileResolutionContext;
use dmls::capabilities::ClientProfile;
use dmls::config::ConfigState;
use dmls::context::{ContextFailure, FixedContext, RepositoryContexts};
use dmls::graph::key_index::basename_of;
use dmls::graph::{
    DocumentContexts, DocumentId, DocumentIndex, EdgeId, EdgeKind, EdgeTarget, Invalidation, LinkDiagnostic,
    Listing, NodeId, NodeKind, NodePayload, WikiResolution, WorkspaceGraph, WorkspaceIndex, index_document,
};
use dmls::providers::symbols::graph_symbol_documents;
use dmls::providers::{DocumentContext, ProviderRegistry};
use dmls::source_map::{PositionEncoding, SourceMap};
use dmls::wiki::WikiUniverse;
use dmls::work_counters::work_counts;
use dmls::workspace::file_path_to_uri;
use lsp_types::InitializeParams;

use crate::common::oracle::{Corpus, normalize_locations};

// ---------------------------------------------------------------------------
// Normalized graph facts
// ---------------------------------------------------------------------------

/// A node as two independently built graphs can agree on it.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct NodeKey {
    path: PathBuf,
    /// Position in the document's own node sequence; the root is 0.
    ordinal: usize,
    kind: String,
    span: (usize, usize),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum TargetFact {
    Node(NodeKey),
    File(PathBuf),
    Unresolved(String),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct EdgeFact {
    source: NodeKey,
    kind: EdgeKind,
    target: TargetFact,
}

/// [`WikiResolution`] with every node id replaced by its [`NodeKey`].
#[derive(Debug, Clone, PartialEq, Eq)]
enum WikiFact {
    Resolved(NodeKey),
    HeadingMissing(NodeKey),
    Ambiguous(Vec<NodeKey>),
    OutsideGraph,
    Unresolved,
    EmptyTarget,
    EmptyHeading(Option<NodeKey>),
    Unsupported,
}

#[derive(Debug, PartialEq, Eq)]
struct NodeFact {
    key: NodeKey,
    /// The payload, with a wiki link's resolution blanked (it is in `wiki`).
    payload: NodePayload,
    wiki: Option<WikiFact>,
    outgoing: Vec<EdgeFact>,
    incoming: Vec<EdgeFact>,
    /// For a link node, why its target is unresolved, judged through the
    /// document's context.
    link_diagnostic: Option<Option<LinkDiagnostic>>,
}

#[derive(Debug, PartialEq, Eq)]
struct DocumentFacts {
    path: PathBuf,
    canonical: Vec<String>,
    workspace_id: usize,
    dependency: bool,
    parsed: bool,
    content_hash: u64,
    /// The universe path of the document's wiki slot.
    slot_path: Option<PathBuf>,
    portability_twins: Vec<PathBuf>,
    /// The basename and every document its bucket holds.
    basename_bucket: Option<(String, Vec<PathBuf>)>,
    transcluded_by: Vec<NodeKey>,
    transclusion_cycle: Option<Vec<PathBuf>>,
    /// The per-document node sequence, root first.
    nodes: Vec<NodeFact>,
}

#[derive(Debug, PartialEq, Eq)]
struct GraphFacts {
    listing: Listing,
    node_count: usize,
    edge_count: usize,
    basename_count: usize,
    documents: Vec<DocumentFacts>,
    edges: Vec<EdgeFact>,
    /// Every universe slot as `(path, canonical, owning root)`, sorted.
    universe: Vec<(PathBuf, Vec<String>, usize)>,
    universe_roots: Vec<PathBuf>,
    /// The probe paths the graph awaits.
    awaited: Vec<PathBuf>,
}

impl GraphFacts {
    /// Normalizes `graph`. `probes` are the candidate paths whose awaited
    /// state is compared; `contexts` judges unresolved links.
    fn of(graph: &WorkspaceGraph, contexts: &dyn DocumentContexts, probes: &[PathBuf]) -> Self {
        let keys = node_keys(graph);
        let key = |id: NodeId| keys[id.0 as usize].clone();
        let target = |edge_target: &EdgeTarget| match edge_target {
            EdgeTarget::Node(id) => TargetFact::Node(key(*id)),
            EdgeTarget::File(path) => TargetFact::File(path.clone()),
            EdgeTarget::Unresolved(raw) => TargetFact::Unresolved(raw.clone()),
        };
        let edge_fact = |edge: &dmls::graph::Edge| EdgeFact {
            source: key(edge.source),
            kind: edge.kind,
            target: target(&edge.target),
        };
        let path_of = |document: DocumentId| graph.document(document).expect("a document").path.clone();

        let mut nodes_by_document: BTreeMap<DocumentId, Vec<NodeFact>> = BTreeMap::new();
        for position in 0..graph.node_count() {
            let id = NodeId(position as u32);
            let node = graph.node(id).expect("a node");
            let mut payload = node.payload.clone();
            let wiki = match &mut payload {
                NodePayload::WikiLink(link) => {
                    let resolution = std::mem::replace(&mut link.resolution, WikiResolution::Unresolved);
                    Some(wiki_fact(&resolution, &key))
                }
                _ => None,
            };
            let mut outgoing: Vec<EdgeFact> = EdgeKind::ALL
                .iter()
                .flat_map(|kind| graph.outgoing(id, *kind).map(edge_fact).collect::<Vec<_>>())
                .collect();
            outgoing.sort();
            let mut incoming: Vec<EdgeFact> = EdgeKind::ALL
                .iter()
                .flat_map(|kind| graph.incoming(id, *kind).map(edge_fact).collect::<Vec<_>>())
                .collect();
            incoming.sort();
            let link_diagnostic = node.as_link().map(|link| {
                let context = contexts.context_for(&path_of(node.document));
                graph.diagnose_unresolved(node.document, &link.target, context.as_ref())
            });
            nodes_by_document.entry(node.document).or_default().push(NodeFact {
                key: key(id),
                payload,
                wiki,
                outgoing,
                incoming,
                link_diagnostic,
            });
        }

        let mut documents = Vec::with_capacity(graph.document_count());
        for (position, record) in graph.documents().enumerate() {
            let id = DocumentId(position as u32);
            // Roots first: `DocumentId(d)`'s root is `NodeId(d)`.
            assert_eq!(record.root, NodeId(id.0), "{}: root numbering", record.path.display());
            let root = graph.node(record.root).expect("a root node");
            assert!(root.kind == NodeKind::Document && root.document == id, "{}", record.path.display());
            let slot = graph.wiki_slot(id);
            if let Some(slot) = slot {
                assert_eq!(graph.slot_document(slot), Some(id), "{}: slot round trip", record.path.display());
            }
            let basename_bucket = basename_of(record.path).map(|basename| {
                let members: Vec<PathBuf> =
                    graph.key_index().documents_for_basename(&basename).iter().map(|doc| path_of(*doc)).collect();
                (basename, members)
            });
            let mut twins: Vec<PathBuf> = graph.portability_twins(id).iter().map(|doc| path_of(*doc)).collect();
            twins.sort();
            let mut transcluded_by: Vec<NodeKey> = graph.transcluded_by(id).into_iter().map(&key).collect();
            transcluded_by.sort();
            documents.push(DocumentFacts {
                path: record.path.clone(),
                canonical: record.canonical.clone(),
                workspace_id: record.workspace_id,
                dependency: record.dependency,
                parsed: record.parsed,
                content_hash: record.content_hash,
                slot_path: slot.and_then(|slot| graph.wiki_universe().path(slot)).map(Path::to_path_buf),
                portability_twins: twins,
                basename_bucket,
                transcluded_by,
                transclusion_cycle: graph
                    .transclusion_cycle(id)
                    .map(|cycle| cycle.into_iter().map(&path_of).collect()),
                nodes: nodes_by_document.remove(&id).unwrap_or_default(),
            });
        }

        let mut edges: Vec<EdgeFact> = (0..graph.edge_count())
            .map(|position| edge_fact(graph.edge(EdgeId(position as u32)).expect("an edge")))
            .collect();
        edges.sort();
        let universe_view = graph.wiki_universe();
        let mut universe: Vec<(PathBuf, Vec<String>, usize)> = (0..universe_view.len())
            .map(|slot| {
                let doc = &universe_view.docs()[slot];
                (universe_view.path(slot).expect("a slot path").to_path_buf(), doc.canonical.clone(), doc.workspace_id)
            })
            .collect();
        universe.sort();
        Self {
            listing: graph.listing(),
            node_count: graph.node_count(),
            edge_count: graph.edge_count(),
            basename_count: graph.key_index().len(),
            documents,
            edges,
            universe,
            universe_roots: universe_view.wiki_roots().to_vec(),
            awaited: probes.iter().filter(|path| graph.awaits(path)).cloned().collect(),
        }
    }

    fn document(&self, path: &Path) -> &DocumentFacts {
        self.documents
            .iter()
            .find(|document| document.path == path)
            .unwrap_or_else(|| panic!("{} is not a document", path.display()))
    }

    /// The wiki outcomes of `path`'s wiki links, in document order.
    fn wiki(&self, path: &Path) -> Vec<WikiFact> {
        self.document(path).nodes.iter().filter_map(|node| node.wiki.clone()).collect()
    }

    /// The `references` targets of `path`'s Markdown links, in document order.
    fn link_targets(&self, path: &Path) -> Vec<Vec<TargetFact>> {
        self.document(path)
            .nodes
            .iter()
            .filter(|node| node.key.kind == "Link")
            .map(|node| node.outgoing.iter().map(|edge| edge.target.clone()).collect())
            .collect()
    }
}

/// Every node's [`NodeKey`], indexed by `NodeId`.
fn node_keys(graph: &WorkspaceGraph) -> Vec<NodeKey> {
    let mut ordinals: HashMap<DocumentId, usize> = HashMap::new();
    (0..graph.node_count())
        .map(|position| {
            let node = graph.node(NodeId(position as u32)).expect("a node");
            let ordinal = ordinals.entry(node.document).or_default();
            let key = NodeKey {
                path: graph.document(node.document).expect("a document").path.clone(),
                ordinal: *ordinal,
                kind: format!("{:?}", node.kind),
                span: (node.span.start, node.span.end),
            };
            *ordinal += 1;
            key
        })
        .collect()
}

fn wiki_fact(resolution: &WikiResolution, key: &impl Fn(NodeId) -> NodeKey) -> WikiFact {
    match resolution {
        WikiResolution::Resolved(node) => WikiFact::Resolved(key(*node)),
        WikiResolution::HeadingMissing(node) => WikiFact::HeadingMissing(key(*node)),
        WikiResolution::Ambiguous(nodes) => WikiFact::Ambiguous(nodes.iter().map(|node| key(*node)).collect()),
        WikiResolution::OutsideGraph => WikiFact::OutsideGraph,
        WikiResolution::Unresolved => WikiFact::Unresolved,
        WikiResolution::EmptyTarget => WikiFact::EmptyTarget,
        WikiResolution::EmptyHeading(node) => WikiFact::EmptyHeading(node.map(key)),
        WikiResolution::Unsupported => WikiFact::Unsupported,
    }
}

fn root_key(path: &Path) -> NodeKey {
    NodeKey { path: path.to_path_buf(), ordinal: 0, kind: "Document".into(), span: (0, 0) }
}

// ---------------------------------------------------------------------------
// Contexts and repositories
// ---------------------------------------------------------------------------

/// One fixed repository context, with a context failure injected for chosen
/// documents (`ContextFailure::NotProvided`, as `NoContexts` reports).
#[derive(Debug)]
struct InjectedContexts {
    inner: FixedContext,
    failing: Mutex<BTreeSet<PathBuf>>,
}

impl InjectedContexts {
    fn fail(&self, document: &Path, failing: bool) {
        let mut set = self.failing.lock().expect("contexts lock");
        if failing {
            set.insert(document.to_path_buf());
        } else {
            set.remove(document);
        }
    }
}

impl DocumentContexts for InjectedContexts {
    fn context_for(&self, document: &Path) -> Result<FileResolutionContext, ContextFailure> {
        if self.failing.lock().expect("contexts lock").contains(document) {
            return Err(ContextFailure::NotProvided);
        }
        self.inner.context_for(document)
    }
}

/// A temporary repository at a canonical root.
struct Repo {
    _dir: tempfile::TempDir,
    root: PathBuf,
}

impl Repo {
    fn new(files: &[(&str, &str)]) -> Self {
        let dir = tempfile::tempdir().expect("temp dir");
        let root = biscuit_file::canonicalize_simplified(dir.path()).expect("canonical temp dir");
        let repo = Self { _dir: dir, root };
        for (name, text) in files {
            repo.write(name, text);
        }
        repo
    }

    fn path(&self, name: &str) -> PathBuf {
        self.root.join(name)
    }

    fn paths(&self, names: &[&str]) -> Vec<PathBuf> {
        names.iter().map(|name| self.path(name)).collect()
    }

    fn write(&self, name: &str, text: &str) {
        let path = self.path(name);
        std::fs::create_dir_all(path.parent().expect("a parent")).expect("create parent");
        std::fs::write(path, text).expect("write file");
    }

    fn remove(&self, name: &str) {
        std::fs::remove_file(self.path(name)).expect("remove file");
    }

    /// Contexts rooted at this repository.
    fn contexts(&self) -> Arc<InjectedContexts> {
        let context = FileResolutionContext::from_snapshot(self.root.clone(), None, HashMap::new())
            .with_repository_root(self.root.clone());
        Arc::new(InjectedContexts { inner: FixedContext(context), failing: Mutex::new(BTreeSet::new()) })
    }
}

// ---------------------------------------------------------------------------
// Transition harness
// ---------------------------------------------------------------------------

/// A live index and an independent model of its graph inputs.
struct Harness {
    index: WorkspaceIndex,
    contexts: Arc<InjectedContexts>,
    listed: BTreeSet<PathBuf>,
    listing: Listing,
    wiki_roots: Vec<PathBuf>,
    parsed: BTreeMap<PathBuf, DocumentIndex>,
    open: BTreeSet<PathBuf>,
    stale: BTreeSet<PathBuf>,
    unlisted_reads: BTreeSet<PathBuf>,
    probes: Vec<PathBuf>,
}

impl Harness {
    /// An index whose listing is still pending, with no wiki roots.
    fn pending(repo: &Repo) -> Self {
        let contexts = repo.contexts();
        Self {
            index: WorkspaceIndex::new(contexts.clone()),
            contexts,
            listed: BTreeSet::new(),
            listing: Listing::Pending,
            wiki_roots: Vec::new(),
            parsed: BTreeMap::new(),
            open: BTreeSet::new(),
            stale: BTreeSet::new(),
            unlisted_reads: BTreeSet::new(),
            probes: Vec::new(),
        }
    }

    /// An index over `listed` (complete listing), wiki-rooted at the repository.
    fn listed(repo: &Repo, listed: &[&str]) -> Self {
        let mut harness = Self::pending(repo);
        harness.set_wiki_roots(vec![repo.root.clone()]);
        harness.install_listing(&repo.paths(listed), true);
        harness
    }

    fn dependencies(&self) -> BTreeSet<PathBuf> {
        self.unlisted_reads
            .iter()
            .filter(|path| {
                self.parsed.contains_key(*path) && !self.listed.contains(*path) && !self.open.contains(*path)
            })
            .cloned()
            .collect()
    }

    fn set_wiki_roots(&mut self, roots: Vec<PathBuf>) {
        self.wiki_roots = roots.clone();
        self.index.set_wiki_roots(roots);
    }

    fn install_listing(&mut self, markdown: &[PathBuf], complete: bool) {
        let markdown: BTreeSet<PathBuf> = markdown.iter().cloned().collect();
        for vanished in self.listed.difference(&markdown) {
            if !self.open.contains(vanished) {
                self.parsed.remove(vanished);
                self.stale.remove(vanished);
            }
        }
        self.listed = markdown.clone();
        self.listing = Listing::Ready { complete };
        self.index.install_listing(markdown, BTreeSet::new(), complete);
    }

    fn open(&mut self, path: &Path, text: &str) -> Invalidation {
        self.open.insert(path.to_path_buf());
        self.stale.remove(path);
        let fresh = index_document(path, text);
        let unchanged = self.parsed.get(path).is_some_and(|held| held.content_hash == fresh.content_hash);
        if !unchanged {
            self.parsed.insert(path.to_path_buf(), fresh);
        }
        let outcome = self.index.open_buffer(path, text);
        assert_eq!(matches!(outcome, Invalidation::Unchanged), unchanged, "open {}", path.display());
        outcome
    }

    fn close(&mut self, path: &Path) {
        if self.open.remove(path) {
            self.parsed.remove(path);
        }
        self.index.close_buffer(path);
    }

    fn disk_changed(&mut self, path: &Path) -> bool {
        let joined = self.listed.insert(path.to_path_buf());
        if !self.open.contains(path) && self.parsed.contains_key(path) {
            self.stale.insert(path.to_path_buf());
        }
        let changed = self.index.disk_changed(path);
        assert_eq!(changed, joined && !self.open.contains(path), "disk change {}", path.display());
        changed
    }

    fn disk_deleted(&mut self, path: &Path) -> bool {
        self.listed.remove(path);
        self.stale.remove(path);
        self.unlisted_reads.remove(path);
        if !self.open.contains(path) {
            self.parsed.remove(path);
        }
        self.index.disk_deleted(path)
    }

    fn unlisted_changed(&mut self, path: &Path, created: bool) -> bool {
        if self.dependencies().contains(path) {
            self.stale.insert(path.to_path_buf());
        }
        self.index.unlisted_changed(path, created)
    }

    /// Mirrors `WorkspaceIndex::ensure_parsed` on the model.
    fn model_ensure_parsed(&mut self, paths: &[PathBuf]) -> Vec<PathBuf> {
        let mut changed = Vec::new();
        for path in paths {
            let needs = !self.open.contains(path) && (self.stale.contains(path) || !self.parsed.contains_key(path));
            if !needs {
                continue;
            }
            self.stale.remove(path);
            if !self.listed.contains(path) {
                self.unlisted_reads.insert(path.clone());
            }
            match std::fs::read_to_string(path) {
                Ok(text) => {
                    let fresh = index_document(path, &text);
                    if self.parsed.get(path).is_some_and(|held| held.content_hash == fresh.content_hash) {
                        continue;
                    }
                    self.parsed.insert(path.clone(), fresh);
                    changed.push(path.clone());
                }
                Err(_) => {
                    self.unlisted_reads.remove(path);
                    if self.parsed.remove(path).is_some() {
                        changed.push(path.clone());
                    }
                }
            }
        }
        changed
    }

    fn ensure_parsed(&mut self, paths: &[PathBuf]) -> Vec<PathBuf> {
        let expected = self.model_ensure_parsed(paths);
        let changed = self.index.ensure_parsed(paths);
        assert_eq!(changed, expected, "ensure_parsed {paths:?}");
        changed
    }

    /// `prepare_targets` on the index; the model reads the `expected` targets
    /// the test states, and [`check`](Self::check) catches any disagreement.
    fn prepare_targets(&mut self, path: &Path, expected: &[PathBuf]) -> bool {
        let model = !self.model_ensure_parsed(expected).is_empty();
        let changed = self.index.prepare_targets(path);
        assert_eq!(changed, model, "prepare_targets {}", path.display());
        changed
    }

    /// The reference graph: `build_listed` over the model's inputs, with a
    /// universe built from the model's candidates.
    fn fresh(&self) -> WorkspaceGraph {
        let dependencies = self.dependencies();
        let candidates = self
            .listed
            .iter()
            .chain(self.parsed.keys().filter(|path| !dependencies.contains(*path)))
            .cloned();
        let universe = WikiUniverse::new(candidates, &self.wiki_roots);
        WorkspaceGraph::build_listed(
            &self.parsed,
            &self.listed,
            &dependencies,
            self.listing,
            0,
            universe,
            self.contexts.as_ref(),
        )
    }

    fn facts(&self, graph: &WorkspaceGraph) -> GraphFacts {
        GraphFacts::of(graph, self.contexts.as_ref(), &self.probes)
    }

    /// Asserts that the model and the index agree on their inputs and that the
    /// live snapshot equals a fresh build; returns the live snapshot's facts.
    fn check(&self, step: &str) -> GraphFacts {
        assert_eq!(self.index.listed(), &self.listed, "{step}: listed");
        assert_eq!(self.index.listing(), self.listing, "{step}: listing");
        let dependencies: BTreeSet<PathBuf> = self.index.dependencies().cloned().collect();
        assert_eq!(dependencies, self.dependencies(), "{step}: dependencies");
        assert_eq!(self.index.len(), self.parsed.len(), "{step}: parsed count");
        for (path, parse) in &self.parsed {
            let held = match self.index.disk_parse(path) {
                Some(held) => held.content_hash,
                None => self
                    .index
                    .open_buffers()
                    .find(|buffer| buffer.path == *path)
                    .unwrap_or_else(|| panic!("{step}: {} is not parsed", path.display()))
                    .content_hash,
            };
            assert_eq!(held, parse.content_hash, "{step}: {}", path.display());
        }
        let live = self.facts(&self.index.snapshot());
        let fresh = self.facts(&self.fresh());
        assert_eq!(live, fresh, "{step}: live snapshot differs from a fresh build");
        live
    }
}

/// Graph-work counters across `transition`, taken before any fresh build.
fn tables_and_universes(harness: &mut Harness, transition: impl FnOnce(&mut Harness)) -> (usize, usize) {
    let before = work_counts();
    transition(harness);
    let work = work_counts().since(before);
    (work.document_tables, work.wiki_universes)
}

// ---------------------------------------------------------------------------
// Live transitions
// ---------------------------------------------------------------------------

#[test]
fn content_edits_and_listed_buffers_match_a_fresh_build() {
    let source_text =
        "---\ntitle: T\n---\n# Source\n\n[a](target.md#beta) [self](#source) [[target]] {{ title }}\n\n::file ./target.md\n";
    let repo = Repo::new(&[
        ("source.md", source_text),
        ("target.md", "# Alpha\n\n## Beta\n"),
        ("other.md", "# Other\n\n## Gamma\n"),
    ]);
    let [source, target, other] = ["source.md", "target.md", "other.md"].map(|name| repo.path(name));
    let mut h = Harness::listed(&repo, &["source.md", "target.md", "other.md"]);
    h.check("listed, unparsed");

    h.open(&source, source_text);
    h.check("listed source opened");
    h.prepare_targets(&source, std::slice::from_ref(&target));
    h.check("target prepared");

    // Heading, link target, and frontmatter key change; the paths do not.
    let edited =
        "---\nheading: T\n---\n# Renamed\n\n[a](other.md#gamma) [self](#source) [[other]] {{ title }}\n\n::file ./target.md\n";
    let (tables, universes) = tables_and_universes(&mut h, |h| {
        h.open(&source, edited);
    });
    assert_eq!((tables, universes), (0, 0), "a content edit reuses the table and universe");
    let facts = h.check("content edit");
    assert_eq!(
        facts.link_targets(&source)[1],
        vec![TargetFact::Unresolved("#source".into())],
        "#source no longer names a heading"
    );
    h.ensure_parsed(std::slice::from_ref(&other));
    h.check("other parsed");

    h.open(&target, "# Alpha\n\n## Delta\n");
    h.check("listed target opened with new headings");
    h.close(&target);
    h.check("listed target closed back to unparsed");
    h.close(&source);
    h.check("source closed");
}

#[test]
fn unsaved_buffers_and_listed_membership_match_a_fresh_build() {
    let repo = Repo::new(&[("source.md", "[[draft]] [[joined]] [j](joined.md)\n"), ("other.md", "# Other\n")]);
    let [source, other, draft, joined] = ["source.md", "other.md", "draft.md", "joined.md"].map(|name| repo.path(name));
    let mut h = Harness::listed(&repo, &["source.md", "other.md"]);
    h.open(&source, "[[draft]] [[joined]] [j](joined.md)\n");
    h.check("source open");

    // An unsaved buffer outside the list is a wiki candidate while open.
    h.open(&draft, "# Draft\n\n[back](source.md) [[source]]\n");
    let facts = h.check("unsaved buffer opened");
    assert!(matches!(facts.wiki(&source)[0], WikiFact::Resolved(_)), "{:?}", facts.wiki(&source));
    h.close(&draft);
    let facts = h.check("unsaved buffer closed");
    assert_eq!(facts.wiki(&source)[0], WikiFact::Unresolved);

    repo.write("joined.md", "# Joined\n");
    assert!(h.disk_changed(&joined));
    h.check("listed join");
    h.ensure_parsed(std::slice::from_ref(&joined));
    h.check("joined parsed");
    repo.remove("joined.md");
    assert!(h.disk_deleted(&joined));
    h.check("listed leave");

    h.install_listing(std::slice::from_ref(&source), true);
    h.check("relist drops other");
    h.install_listing(&[source, other], true);
    h.check("relist restores other");
}

#[test]
fn dependencies_open_join_and_return_to_referenced_only_like_a_fresh_build() {
    let dep_text = "# Alpha\n\n## Beta\n\n[x](source.md) [[source]]\n";
    let repo = Repo::new(&[("source.md", "[t](dep.md#beta)\n\n::file ./dep.md\n"), ("dep.md", dep_text)]);
    let [source, dep] = ["source.md", "dep.md"].map(|name| repo.path(name));
    let mut h = Harness::listed(&repo, &["source.md"]);
    h.open(&source, "[t](dep.md#beta)\n\n::file ./dep.md\n");
    h.prepare_targets(&source, std::slice::from_ref(&dep));
    let facts = h.check("referenced dependency parsed");
    assert!(facts.document(&dep).dependency);

    // Known quirk: opening a dependency with its disk bytes is `Unchanged`,
    // so no rebuild runs and the snapshot still flags it as a dependency,
    // where a fresh build over the same inputs does not.
    let generation = h.index.generation();
    assert_eq!(h.open(&dep, dep_text), Invalidation::Unchanged);
    assert_eq!(h.index.generation(), generation);
    assert!(h.facts(&h.index.snapshot()).document(&dep).dependency);
    assert!(!h.facts(&h.fresh()).document(&dep).dependency);
    h.open(&dep, "# Alpha\n\n## Beta\n\n[x](source.md) [[source]] edited\n");
    let facts = h.check("dependency opened with new text");
    assert!(!facts.document(&dep).dependency);

    h.close(&dep);
    let facts = h.check("dependency closed: referenced only");
    assert_eq!(facts.link_targets(&source)[0], vec![TargetFact::File(dep.clone())]);
    h.ensure_parsed(std::slice::from_ref(&dep));
    h.check("dependency again");

    assert!(h.disk_changed(&dep));
    let facts = h.check("dependency joined the list");
    assert!(!facts.document(&dep).dependency);
    h.ensure_parsed(std::slice::from_ref(&dep));
    h.check("listed former dependency re-read");

    h.install_listing(std::slice::from_ref(&source), true);
    h.check("left the list: referenced only");
    h.ensure_parsed(std::slice::from_ref(&dep));
    let facts = h.check("dependency once more");
    assert!(facts.document(&dep).dependency);
}

#[test]
fn stale_unchanged_changed_and_failed_reads_match_a_fresh_build() {
    let repo = Repo::new(&[
        ("source.md", "[t](target.md#beta)\n\n::file ./dep.md\n"),
        ("target.md", "# Alpha\n\n## Beta\n"),
        ("dep.md", "# Dep\n"),
    ]);
    let [source, target, dep] = ["source.md", "target.md", "dep.md"].map(|name| repo.path(name));
    let mut h = Harness::listed(&repo, &["source.md", "target.md"]);
    h.open(&source, "[t](target.md#beta)\n\n::file ./dep.md\n");
    h.ensure_parsed(&[target.clone(), dep.clone()]);
    h.check("target and dependency parsed");

    assert!(!h.disk_changed(&target));
    h.check("target stale");
    assert_eq!(h.ensure_parsed(std::slice::from_ref(&target)), Vec::<PathBuf>::new());
    h.check("stale target re-read unchanged");

    repo.write("target.md", "# Alpha\n\n## Gamma\n");
    h.disk_changed(&target);
    assert_eq!(h.ensure_parsed(std::slice::from_ref(&target)), vec![target.clone()]);
    let facts = h.check("stale target re-read changed");
    assert_eq!(facts.link_targets(&source)[0], vec![TargetFact::Unresolved("target.md#beta".into())]);

    repo.remove("target.md");
    h.disk_changed(&target);
    assert_eq!(h.ensure_parsed(std::slice::from_ref(&target)), vec![target.clone()]);
    let facts = h.check("stale target read failed");
    assert!(!facts.document(&target).parsed);

    repo.write("dep.md", "# Dep two\n");
    assert!(!h.unlisted_changed(&dep, false));
    assert_eq!(h.ensure_parsed(std::slice::from_ref(&dep)), vec![dep.clone()]);
    h.check("stale dependency re-read changed");
    repo.remove("dep.md");
    assert!(!h.unlisted_changed(&dep, false));
    assert_eq!(h.ensure_parsed(std::slice::from_ref(&dep)), vec![dep.clone()]);
    let facts = h.check("stale dependency read failed");
    assert!(facts.documents.iter().all(|document| document.path != dep));
}

#[test]
fn every_listing_state_matches_a_fresh_build() {
    let repo = Repo::new(&[("source.md", "[t](target.md) [[target]]\n"), ("target.md", "# Target\n")]);
    let [source, target] = ["source.md", "target.md"].map(|name| repo.path(name));
    let mut h = Harness::pending(&repo);
    h.set_wiki_roots(vec![repo.root.clone()]);
    h.open(&source, "[t](target.md) [[target]]\n");
    assert_eq!(h.check("pending").listing, Listing::Pending);

    let listed = [source, target];
    let (tables, universes) = tables_and_universes(&mut h, |h| h.install_listing(&listed, false));
    assert_eq!((tables, universes), (1, 1));
    assert_eq!(h.check("ready, incomplete").listing, Listing::Ready { complete: false });
    let (tables, universes) = tables_and_universes(&mut h, |h| h.install_listing(&listed, true));
    assert_eq!((tables, universes), (1, 0), "a listing-state change alone needs a new table only");
    assert_eq!(h.check("ready, complete").listing, Listing::Ready { complete: true });
}

#[test]
fn reordered_and_narrowed_wiki_roots_match_a_fresh_build() {
    let text = "[[/notes/Target]] [[Target]] [[sub/Deep]] [[notes/sub/Deep]]\n";
    let repo = Repo::new(&[("source.md", text), ("notes/Target.md", "# Target\n"), ("notes/sub/Deep.md", "# Deep\n")]);
    let source = repo.path("source.md");
    let notes = repo.path("notes");
    let mut h = Harness::listed(&repo, &["source.md", "notes/Target.md", "notes/sub/Deep.md"]);
    h.open(&source, text);

    let mut outcomes = Vec::new();
    for (step, roots) in [
        ("root only", vec![repo.root.clone()]),
        ("root then notes", vec![repo.root.clone(), notes.clone()]),
        ("notes then root", vec![notes.clone(), repo.root.clone()]),
        ("narrowed to notes", vec![notes.clone()]),
        ("no roots", Vec::new()),
    ] {
        h.set_wiki_roots(roots);
        outcomes.push(h.check(step).wiki(&source));
    }
    assert_ne!(outcomes[1], outcomes[2], "root order decides canonical paths");
}

#[test]
fn context_relink_matches_a_fresh_build() {
    let repo = Repo::new(&[
        ("source.md", "[t](target.md#beta)\n\n::file ./target.md\n"),
        ("target.md", "# Alpha\n\n## Beta\n"),
    ]);
    let [source, target] = ["source.md", "target.md"].map(|name| repo.path(name));
    let mut h = Harness::listed(&repo, &["source.md", "target.md"]);
    h.open(&source, "[t](target.md#beta)\n\n::file ./target.md\n");
    h.ensure_parsed(std::slice::from_ref(&target));
    let resolved = h.check("contexts available");

    h.contexts.fail(&source, true);
    let (tables, _) = tables_and_universes(&mut h, |h| h.index.relink());
    assert_eq!(tables, 0, "a relink over equal paths reuses the table");
    let failed = h.check("source context failed");
    assert_eq!(failed.link_targets(&source)[0], vec![TargetFact::Unresolved("target.md#beta".into())]);
    assert!(failed.document(&target).transcluded_by.is_empty());

    h.contexts.fail(&source, false);
    h.index.relink();
    assert_eq!(h.check("source context restored"), resolved);
}

#[test]
fn awaited_creation_matches_a_fresh_build() {
    let repo = Repo::new(&[("source.md", "[n](new.md) [m](gone/missing.md)\n")]);
    let source = repo.path("source.md");
    let [new, missing, output] = ["new.md", "gone/missing.md", "build/out.md"].map(|name| repo.path(name));
    let mut h = Harness::listed(&repo, &["source.md"]);
    h.probes = vec![new.clone(), missing.clone(), output.clone()];
    h.open(&source, "[n](new.md) [m](gone/missing.md)\n");
    let facts = h.check("both links await");
    assert_eq!(facts.awaited, vec![new.clone(), missing.clone()]);

    repo.write("build/out.md", "# Output\n");
    let generation = h.index.generation();
    assert!(!h.unlisted_changed(&output, true));
    assert_eq!(h.index.generation(), generation, "an unreferenced creation does not rebuild");
    h.check("unreferenced creation");

    repo.write("new.md", "# New\n");
    assert!(h.unlisted_changed(&new, true));
    let facts = h.check("awaited creation");
    assert_eq!(facts.link_targets(&source)[0], vec![TargetFact::File(new)]);
    assert_eq!(facts.awaited, vec![missing]);
}

#[test]
fn missing_and_unprobeable_targets_match_a_fresh_build() {
    let repo = Repo::new(&[("docs/doc.md", "[t](t.md)\n")]);
    let [doc, looped, buffer] = ["docs/doc.md", "docs/t.md", "t.md"].map(|name| repo.path(name));
    let mut h = Harness::listed(&repo, &["docs/doc.md"]);
    h.probes = vec![looped.clone(), buffer.clone()];
    h.open(&doc, "[t](t.md)\n");
    // `t.md` plans `docs/t.md`, then the repository root's `t.md`.
    h.open(&buffer, "# Buffer\n");
    let facts = h.check("missing first candidate: the unsaved later buffer answers");
    assert_eq!(facts.link_targets(&doc)[0], vec![TargetFact::Node(root_key(&buffer))]);

    // A self-referencing symlink cannot be probed; composition stops there.
    #[cfg(unix)]
    std::os::unix::fs::symlink("t.md", &looped).expect("create a symlink loop");
    #[cfg(windows)]
    std::os::windows::fs::symlink_file("t.md", &looped)
        .expect("creating a file symlink needs Developer Mode or SeCreateSymbolicLinkPrivilege");
    h.index.relink();
    let facts = h.check("unprobeable first candidate");
    assert_eq!(facts.link_targets(&doc)[0], vec![TargetFact::Unresolved("t.md".into())]);
    let link = facts.document(&doc).nodes.iter().find(|node| node.key.kind == "Link").expect("a link");
    assert_eq!(
        link.link_diagnostic,
        Some(Some(LinkDiagnostic::BrokenPath { failure: biscuit_file::ResolutionFailure::Io }))
    );

    std::fs::remove_file(&looped).expect("remove the loop");
    repo.write("docs/t.md", "# Real\n");
    assert!(h.unlisted_changed(&looped, true), "the unresolved link awaits its first candidate");
    let facts = h.check("first candidate created");
    assert_eq!(facts.link_targets(&doc)[0], vec![TargetFact::File(looped)]);
}

#[test]
fn wiki_matches_turn_ambiguous_and_back_like_a_fresh_build() {
    let repo = Repo::new(&[("source.md", "[[topic]] [[note]]\n"), ("a/topic.md", "# A\n")]);
    let [source, a_topic, b_topic, c_topic] =
        ["source.md", "a/topic.md", "b/topic.md", "c/topic.md"].map(|name| repo.path(name));
    let mut h = Harness::listed(&repo, &["source.md", "a/topic.md"]);
    h.open(&source, "[[topic]] [[note]]\n");
    assert_eq!(h.check("unique").wiki(&source)[0], WikiFact::Resolved(root_key(&a_topic)));

    repo.write("b/topic.md", "# B\n");
    assert!(h.disk_changed(&b_topic));
    let ambiguous = WikiFact::Ambiguous(vec![root_key(&a_topic), root_key(&b_topic)]);
    assert_eq!(h.check("listed rival joined").wiki(&source)[0], ambiguous);
    repo.remove("b/topic.md");
    assert!(h.disk_deleted(&b_topic));
    assert_eq!(h.check("listed rival left").wiki(&source)[0], WikiFact::Resolved(root_key(&a_topic)));

    h.open(&c_topic, "# C\n");
    assert!(matches!(h.check("unsaved rival opened").wiki(&source)[0], WikiFact::Ambiguous(_)));
    h.close(&c_topic);
    assert_eq!(h.check("unsaved rival closed").wiki(&source)[0], WikiFact::Resolved(root_key(&a_topic)));

    // Logical paths only: a case pair need not exist as two files on disk.
    let [upper, lower] = ["docs/Note.md", "docs/note.md"].map(|name| repo.path(name));
    let mut listed: Vec<PathBuf> = h.listed.iter().cloned().collect();
    listed.extend([upper.clone(), lower.clone()]);
    h.install_listing(&listed, true);
    let facts = h.check("case pair listed");
    assert_eq!(facts.document(&upper).portability_twins, vec![lower.clone()]);
    listed.retain(|path| *path != lower);
    h.install_listing(&listed, true);
    let facts = h.check("case pair split");
    assert!(facts.document(&upper).portability_twins.is_empty());
}

#[test]
fn a_batch_of_joins_matches_a_fresh_build() {
    let repo = Repo::new(&[("source.md", "[[topic]]\n"), ("a/topic.md", "# A\n")]);
    let source = repo.path("source.md");
    let mut h = Harness::listed(&repo, &["source.md", "a/topic.md"]);
    h.open(&source, "[[topic]]\n");
    h.check("before the batch");

    let joined = repo.paths(&["b/topic.md", "c/topic.md", "d/other.md"]);
    for path in &joined {
        std::fs::create_dir_all(path.parent().expect("a parent")).expect("create parent");
        std::fs::write(path, "# Joined\n").expect("write");
    }
    let (tables, universes) = tables_and_universes(&mut h, |h| {
        h.index.begin_batch();
        for path in &joined {
            h.disk_changed(path);
        }
        assert!(h.index.end_batch());
    });
    assert_eq!((tables, universes), (1, 1), "one rebuild for the whole batch");
    let facts = h.check("after the batch");
    assert!(matches!(&facts.wiki(&source)[0], WikiFact::Ambiguous(nodes) if nodes.len() == 3));
}

// ---------------------------------------------------------------------------
// Request graphs
// ---------------------------------------------------------------------------

#[test]
fn request_graphs_over_the_live_universe_match_a_fresh_universe() {
    let wiki_text = "[[topic]] [[solo]] [[Note]]\n\n::file ../dep.md\n";
    let repo = Repo::new(&[
        ("notes/wiki.md", wiki_text),
        ("a/topic.md", "# A\n"),
        ("b/topic.md", "# B\n"),
        ("solo.md", "# Solo\n"),
        ("docs/Note.md", "# Note\n"),
        ("dep.md", "# Dependency\n"),
    ]);
    let wiki = repo.path("notes/wiki.md");
    // `docs/note.md` is a logical case twin of `docs/Note.md`.
    let listed = ["notes/wiki.md", "a/topic.md", "b/topic.md", "solo.md", "docs/Note.md", "docs/note.md"];
    let mut h = Harness::listed(&repo, &listed);
    h.open(&wiki, wiki_text);
    h.prepare_targets(&wiki, &[repo.path("dep.md")]);
    h.check("live index with a dependency");
    let live_universe = h.index.snapshot().shared_wiki_universe();

    // In the request graph: the source, one of the `topic` rivals, and one
    // member of the case pair. Out of it: the other rival, the unique `solo`
    // target, and the other twin.
    let scoped: BTreeMap<PathBuf, DocumentIndex> = [
        ("notes/wiki.md", wiki_text),
        ("a/topic.md", "# A\n"),
        ("docs/Note.md", "# Note\n"),
    ]
    .into_iter()
    .map(|(name, text)| (repo.path(name), index_document(&repo.path(name), text)))
    .collect();
    let contexts = h.contexts.as_ref();
    let request = |universe: Arc<WikiUniverse>| WorkspaceGraph::build_with_roots(&scoped, 1, universe, contexts);
    let fresh_universe = WikiUniverse::new(repo.paths(&listed), std::slice::from_ref(&repo.root));

    let cached = GraphFacts::of(&request(Arc::clone(&live_universe)), contexts, &[]);
    let fresh = GraphFacts::of(&request(Arc::new(fresh_universe)), contexts, &[]);
    assert_eq!(cached.documents, fresh.documents);
    assert_eq!(cached.edges, fresh.edges);
    let [topic, solo, note] = <[WikiFact; 3]>::try_from(cached.wiki(&wiki)).expect("three wiki links");
    assert_eq!(topic, WikiFact::Ambiguous(vec![root_key(&repo.path("a/topic.md"))]));
    assert_eq!(solo, WikiFact::OutsideGraph);
    // Matching is case-sensitive: the twin outside the graph is no rival.
    assert_eq!(note, WikiFact::Resolved(root_key(&repo.path("docs/Note.md"))));
    assert!(cached.document(&repo.path("docs/Note.md")).portability_twins.is_empty());

    // Negative control: judged against its own documents, the answers move.
    let own_universe = WikiUniverse::new(scoped.keys().cloned(), std::slice::from_ref(&repo.root));
    let own = GraphFacts::of(&request(Arc::new(own_universe)), contexts, &[]);
    assert_ne!(own.documents, cached.documents);
    assert_eq!(own.wiki(&wiki)[0], WikiFact::Resolved(root_key(&repo.path("a/topic.md"))));
    assert_eq!(own.wiki(&wiki)[1], WikiFact::Unresolved);
}

// ---------------------------------------------------------------------------
// Provider answers over the corpus
// ---------------------------------------------------------------------------

/// Everything the provider registry answers at the sampled offsets of one
/// document, serialized so two graphs' answers compare whole.
#[derive(Debug, PartialEq, Eq)]
struct ProviderAnswers {
    diagnostics: Vec<String>,
    per_offset: Vec<(usize, String)>,
}

struct ProviderEnv {
    config: ConfigState,
    profile: ClientProfile,
    contexts: RepositoryContexts,
    registry: ProviderRegistry,
}

impl ProviderEnv {
    fn answers(&self, graph: &WorkspaceGraph, path: &Path, text: &str, offsets: &[usize]) -> ProviderAnswers {
        let uri = file_path_to_uri(path).expect("absolute document path");
        let source_map = SourceMap::new(uri.clone(), 1, PositionEncoding::Utf16, Arc::from(text));
        let resolution = self.contexts.for_document(path);
        let ctx = DocumentContext {
            uri: &uri,
            path,
            text,
            source_map: &source_map,
            graph,
            doc_id: graph.document_id(path),
            config: self.config.effective(),
            profile: &self.profile,
            overlay: None,
            resolution: &resolution,
        };
        let json = |value: serde_json::Value| value.to_string();
        let mut diagnostics: Vec<String> = self
            .registry
            .diagnostics(&ctx)
            .into_iter()
            .map(|diagnostic| json(serde_json::to_value(diagnostic).expect("serialize")))
            .collect();
        diagnostics.sort();
        let mut per_offset = Vec::new();
        for &offset in offsets {
            let definition = normalize_locations(self.registry.definition(&ctx, offset));
            per_offset.push((offset, format!("definition {definition:?}")));
            let hover = self.registry.hover(&ctx, offset);
            per_offset.push((offset, format!("hover {}", json(serde_json::to_value(hover).expect("serialize")))));
            let mut completion: Vec<String> = self
                .registry
                .completion(&ctx, offset)
                .into_iter()
                .map(|item| json(serde_json::to_value(item).expect("serialize")))
                .collect();
            completion.sort();
            per_offset.push((offset, format!("completion {completion:?}")));
        }
        ProviderAnswers { diagnostics, per_offset }
    }
}

/// Offsets just inside every link, wiki link, anchor, and directive opener.
fn sample_offsets(text: &str) -> Vec<usize> {
    let mut offsets = BTreeSet::new();
    for (offset, character) in text.char_indices() {
        if matches!(character, '[' | '(' | '#' | ':') {
            for step in [1, 2] {
                let candidate = offset + step;
                if candidate <= text.len() && text.is_char_boundary(candidate) {
                    offsets.insert(candidate);
                }
            }
        }
    }
    offsets.into_iter().collect()
}

#[test]
fn corpus_provider_answers_over_a_reused_live_snapshot_equal_the_oracle() {
    let corpus = Corpus::materialize(&biscuit_test_harness::manifest_dir!().join("tests/fixtures/index_store_corpus"));
    let oracle = corpus.oracle();
    let files = oracle.files();
    let contexts: Arc<RepositoryContexts> = Arc::new(RepositoryContexts::new(corpus.snapshot()));

    let mut index = WorkspaceIndex::new(contexts.clone());
    index.set_wiki_roots(oracle.file_list_universe().wiki_roots().to_vec());
    index.install_listing(files.iter().cloned().collect(), BTreeSet::new(), true);
    index.ensure_parsed(&files);
    assert!(index.dependencies().next().is_none(), "every parsed corpus file is listed");

    // An edit and its undo: the live snapshot that answers below reused its
    // table and universe, and its parse inputs equal the oracle's.
    let wiki = corpus.ws.join("notes/wiki.md");
    let wiki_text = std::fs::read_to_string(&wiki).expect("read wiki.md");
    let before = work_counts();
    index.open_buffer(&wiki, &format!("{wiki_text}\n## Added\n[[topic]]\n"));
    index.open_buffer(&wiki, &wiki_text);
    let work = work_counts().since(before);
    assert_eq!((work.document_tables, work.wiki_universes), (0, 0), "{work:?}");
    let live = index.snapshot();

    assert_eq!(
        GraphFacts::of(&live, contexts.as_ref(), &[]),
        GraphFacts::of(oracle.graph(), contexts.as_ref(), &[]),
        "graph facts"
    );

    let b = corpus.ws.join("docs/b.md");
    let note = corpus.ws.join("notes/note.md");
    let spec = corpus.ws.join("notes/spec.md");
    for (path, needle) in [
        (&wiki, "[[topic]]"),
        (&wiki, "[[note]]"),
        (&wiki, "[[b#Some Heading]]"),
        (&b, "## Some Heading"),
        (&b, "[self anchor]"),
    ] {
        let position = oracle.position_of(path, needle);
        for include_declaration in [false, true] {
            assert_eq!(
                oracle.references_over(&live, path, position, include_declaration),
                oracle.references(path, position, include_declaration),
                "references at {needle}"
            );
        }
    }
    let heading = oracle.position_of(&b, "Some Heading");
    assert_eq!(oracle.rename_over(&live, &b, heading, "Renamed"), oracle.rename(&b, heading, "Renamed"));
    for rename in [
        [(spec.clone(), corpus.ws.join("notes/deep/topic.md"))],
        [(note.clone(), corpus.ws.join("notes/renamed-note.md"))],
        [(spec.clone(), corpus.ws.join("alpha/topic.md"))],
    ] {
        assert_eq!(oracle.will_rename_files_over(&live, &rename), oracle.will_rename_files(&rename), "{rename:?}");
    }
    let symbols = graph_symbol_documents(&live);
    for query in ["", "Some", "topic"] {
        assert_eq!(oracle.workspace_symbols_over(&symbols, query), oracle.workspace_symbols(query), "{query:?}");
    }

    let env = ProviderEnv {
        config: ConfigState::load(corpus.roots(), None),
        profile: ClientProfile::from_initialize(&InitializeParams::default(), PositionEncoding::Utf16),
        contexts: RepositoryContexts::new(corpus.snapshot()),
        registry: ProviderRegistry::with_substrate(),
    };
    let mut answered = 0;
    for path in &files {
        assert_eq!(oracle.wiki_diagnostics_over(&live, path), oracle.wiki_diagnostics(path), "{}", path.display());
        let text = std::fs::read_to_string(path).expect("read corpus file");
        let offsets = sample_offsets(&text);
        let live_answers = env.answers(&live, path, &text, &offsets);
        let oracle_answers = env.answers(oracle.graph(), path, &text, &offsets);
        assert_eq!(live_answers, oracle_answers, "{}", path.display());
        answered += live_answers.per_offset.len();
    }
    assert!(answered > 100, "only {answered} provider answers were compared");
}
