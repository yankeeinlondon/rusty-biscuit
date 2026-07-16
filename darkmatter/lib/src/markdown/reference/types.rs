//! Core type definitions for the reference analysis subsystem.

use super::provenance::{
    ReferenceDependencyManifest, ReferenceDocumentIdentity, ReferenceGraphMode,
    ReferenceGraphProvenance,
};
use crate::markdown::Markdown;
use crate::markdown::compose::{ComposeSource, ReferenceGraphOptionsIdentity};
use crate::markdown::normalize::HeadingLevel;
use serde::Serialize;
use std::ops::Range;
use std::path::PathBuf;

// ── Node identifier ─────────────────────────────────────────────────

/// Unique identifier for a node in the reference graph.
///
/// Wraps a string identifier to provide type safety and prevent accidental
/// confusion with other string types.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NodeId(pub String);

impl std::fmt::Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<String> for NodeId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for NodeId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl AsRef<str> for NodeId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

// ── Reference classification ────────────────────────────────────────

/// The semantic kind of a reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReferenceKind {
    /// A hyperlink (`<a>` or markdown link).
    Hyperlink,
    /// An image (`<img>` or markdown image).
    Image,
    /// A video (`<video>`).
    HtmlVideo,
    /// An audio element (`<audio>`).
    HtmlAudio,
    /// A source element (`<source>`).
    HtmlSource,
    /// An iframe (`<iframe>`).
    HtmlIframe,
    /// A transclusion directive (`::file`, `::url`, `::code`, prologue/epilogue).
    Transclusion,
    /// A CSS import (`<link rel="stylesheet">` or `@import`).
    CssImport,
    /// An inline `<style>` block.
    InlineCss,
    /// A `<script src="...">` import.
    ScriptImport,
    /// An inline `<script>` block (no `src`).
    InlineScript,
    /// A font import (`@font-face src` or `<link ... as="font">`).
    FontImport,
    /// A `<meta>` tag.
    MetaTag,
}

impl ReferenceKind {
    /// Human-readable category label used when grouping validation issues.
    pub fn validation_category_label(self) -> &'static str {
        match self {
            ReferenceKind::Hyperlink => "Invalid Hyperlink(s)",
            ReferenceKind::Image => "Invalid Image Reference(s)",
            ReferenceKind::HtmlVideo
            | ReferenceKind::HtmlAudio
            | ReferenceKind::HtmlSource => "Invalid Media Reference(s)",
            ReferenceKind::HtmlIframe => "Invalid Iframe(s)",
            ReferenceKind::Transclusion => "Invalid Transclusion Target(s)",
            ReferenceKind::CssImport | ReferenceKind::InlineCss => "Invalid CSS Import(s)",
            ReferenceKind::ScriptImport | ReferenceKind::InlineScript => "Invalid Script Import(s)",
            ReferenceKind::FontImport => "Invalid Font Import(s)",
            ReferenceKind::MetaTag => "Invalid Meta Tag(s)",
        }
    }

    /// Stable ordering key used when grouping validation issues by kind.
    pub fn validation_order(self) -> u8 {
        match self {
            ReferenceKind::Transclusion => 0,
            ReferenceKind::Hyperlink => 1,
            ReferenceKind::Image => 2,
            ReferenceKind::HtmlVideo | ReferenceKind::HtmlAudio | ReferenceKind::HtmlSource => 3,
            ReferenceKind::HtmlIframe => 4,
            ReferenceKind::CssImport | ReferenceKind::InlineCss => 5,
            ReferenceKind::ScriptImport | ReferenceKind::InlineScript => 6,
            ReferenceKind::FontImport => 7,
            ReferenceKind::MetaTag => 8,
        }
    }
}

/// The syntactic form in which a reference appears.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReferenceSyntax {
    /// `[text](url)` markdown link.
    MarkdownLink,
    /// `<a href="...">` HTML anchor.
    HtmlAnchor,
    /// `![alt](src)` markdown image.
    MarkdownImage,
    /// `<img src="...">` HTML image.
    HtmlImage,
    /// `<video src="...">` HTML video.
    HtmlVideoTag,
    /// `<audio src="...">` HTML audio.
    HtmlAudioTag,
    /// `<source src="...">` HTML source.
    HtmlSourceTag,
    /// `<iframe src="...">` HTML iframe.
    HtmlIframeTag,
    /// `::file path` directive.
    DirectiveFile,
    /// `::url https://...` directive.
    DirectiveUrl,
    /// `::code path` directive.
    DirectiveCode,
    /// `::toc-linking path` directive.
    DirectiveTocLinking,
    /// `::file-links` directive.
    DirectiveFileLinks,
    /// `prologue: path` frontmatter reference.
    FrontmatterPrologue,
    /// `epilogue: path` frontmatter reference.
    FrontmatterEpilogue,
    /// `<link href="...">` HTML link tag.
    HtmlLinkTag,
    /// `<script src="...">` HTML script tag.
    HtmlScriptTag,
    /// `<style>...</style>` HTML style tag.
    HtmlStyleTag,
    /// `@import url(...)` CSS at-import.
    CssAtImport,
    /// `@font-face { src: url(...) }` CSS font-face source.
    CssFontFaceSrc,
    /// `<meta ...>` HTML meta tag.
    HtmlMetaTag,
}

impl ReferenceSyntax {
    /// Returns `true` if this syntax kind represents a transclusion that can
    /// be expanded into a nested `FileTree` subtree.
    ///
    /// Only local Markdown transclusions are followable:
    /// `::file`, `::toc-linking`, `prologue`, `epilogue`.
    pub fn is_followable_transclusion(&self) -> bool {
        matches!(
            self,
            ReferenceSyntax::DirectiveFile
                | ReferenceSyntax::DirectiveTocLinking
                | ReferenceSyntax::FrontmatterPrologue
                | ReferenceSyntax::FrontmatterEpilogue
        )
    }
}

/// Classification of a reference's target.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ReferenceTarget {
    /// A local filesystem path.
    LocalPath { raw: PathBuf },
    /// A remote HTTP/HTTPS URL.
    RemoteUrl { raw: String },
    /// A fragment-only reference (`#section`).
    Fragment { raw: String },
    /// A data URI (`data:...`).
    DataUri { raw: String },
    /// A non-HTTP scheme (`mailto:`, `tel:`, etc.).
    OtherScheme { raw: String, scheme: String },
    /// Inline content (e.g., `<style>` or `<script>` body).
    Inline,
}

impl ReferenceTarget {
    /// Returns the raw target string, or `None` for inline targets.
    pub fn raw(&self) -> Option<&str> {
        match self {
            Self::LocalPath { raw } => raw.to_str(),
            Self::RemoteUrl { raw }
            | Self::Fragment { raw }
            | Self::DataUri { raw }
            | Self::OtherScheme { raw, .. } => Some(raw),
            Self::Inline => None,
        }
    }
}

/// Classifies a raw URL/path string into a [`ReferenceTarget`].
pub fn classify_target(raw: &str) -> ReferenceTarget {
    if raw.starts_with('#') {
        ReferenceTarget::Fragment { raw: raw.into() }
    } else if raw.starts_with("data:") {
        ReferenceTarget::DataUri { raw: raw.into() }
    } else if raw.starts_with("http://") || raw.starts_with("https://") || raw.starts_with("//") {
        ReferenceTarget::RemoteUrl { raw: raw.into() }
    } else if raw.starts_with("mailto:") || raw.starts_with("tel:") || raw.contains("://") {
        let scheme = if raw.contains("://") {
            raw.split("://").next().unwrap_or("")
        } else {
            raw.split(':').next().unwrap_or("")
        };
        ReferenceTarget::OtherScheme {
            raw: raw.into(),
            scheme: scheme.into(),
        }
    } else {
        ReferenceTarget::LocalPath {
            raw: PathBuf::from(raw),
        }
    }
}

// ── Provenance ──────────────────────────────────────────────────────

/// Records where a reference was found.
#[derive(Debug, Clone, Serialize)]
pub struct ReferenceOrigin {
    /// The compose source (file, URL, or unknown).
    pub source: ComposeSource,
    /// 1-based line number.
    pub line: usize,
    /// Byte-offset span in the source content.
    pub span: Range<usize>,
    /// Syntactic form of the reference.
    pub syntax: ReferenceSyntax,
}

// ── Record ──────────────────────────────────────────────────────────

/// A single reference with full provenance.
#[derive(Debug, Clone)]
pub struct ReferenceRecord {
    /// Stable unique identifier (`{source_hash:016x}:{line}:{span_start}`).
    pub id: String,
    /// Semantic kind.
    pub kind: ReferenceKind,
    /// Classified target.
    pub target: ReferenceTarget,
    /// Where this reference was found.
    pub origin: ReferenceOrigin,
    /// Extra attributes (`data-*`, CSS classes, etc.).
    pub attributes: serde_json::Map<String, serde_json::Value>,
}

impl Serialize for ReferenceRecord {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("ReferenceRecord", 6)?;
        state.serialize_field("id", &self.id)?;
        state.serialize_field("kind", &self.kind)?;
        state.serialize_field("target", &self.target)?;
        state.serialize_field("syntax", &self.origin.syntax)?;
        state.serialize_field("line", &self.origin.line)?;
        if !self.attributes.is_empty() {
            state.serialize_field("attributes", &self.attributes)?;
        }
        state.end()
    }
}

/// Generates a stable reference ID from source, line, and span start.
pub fn make_reference_id(source: &ComposeSource, line: usize, span_start: usize) -> String {
    let source_hash = match source {
        ComposeSource::Unknown => 0u64,
        ComposeSource::File(p) => {
            let s = p.to_string_lossy();
            simple_hash(s.as_bytes())
        }
        ComposeSource::Url(u) => simple_hash(u.as_str().as_bytes()),
    };
    format!("{source_hash:016x}:{line}:{span_start}")
}

/// Simple non-cryptographic hash for ID generation.
fn simple_hash(bytes: &[u8]) -> u64 {
    // FNV-1a 64-bit
    let mut hash: u64 = 0xcbf29ce484222325;
    for &b in bytes {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

// ── Collection ──────────────────────────────────────────────────────

/// A collection of reference records.
#[derive(Debug, Clone, Default)]
pub struct ReferenceSet {
    /// The reference records.
    pub records: Vec<ReferenceRecord>,
}

impl ReferenceSet {
    /// Returns the number of references.
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// Returns `true` if the set is empty.
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// Returns references filtered by kind.
    pub fn by_kind(&self, kind: ReferenceKind) -> Vec<&ReferenceRecord> {
        self.records.iter().filter(|r| r.kind == kind).collect()
    }

    /// Returns references filtered to hyperlinks only.
    pub fn hyperlinks(&self) -> Vec<&ReferenceRecord> {
        self.by_kind(ReferenceKind::Hyperlink)
    }

    /// Returns references filtered to images only.
    pub fn images(&self) -> Vec<&ReferenceRecord> {
        self.by_kind(ReferenceKind::Image)
    }

    /// Returns references filtered to transclusions only.
    pub fn transclusions(&self) -> Vec<&ReferenceRecord> {
        self.by_kind(ReferenceKind::Transclusion)
    }

    /// Consumes the set and returns records of the given kind, converted to `T`.
    pub fn filter_convert<T: From<ReferenceRecord>>(self, kind: ReferenceKind) -> Vec<T> {
        self.records
            .into_iter()
            .filter(|r| r.kind == kind)
            .map(T::from)
            .collect()
    }
}

impl IntoIterator for ReferenceSet {
    type Item = ReferenceRecord;
    type IntoIter = std::vec::IntoIter<ReferenceRecord>;

    fn into_iter(self) -> Self::IntoIter {
        self.records.into_iter()
    }
}

impl<'a> IntoIterator for &'a ReferenceSet {
    type Item = &'a ReferenceRecord;
    type IntoIter = std::slice::Iter<'a, ReferenceRecord>;

    fn into_iter(self) -> Self::IntoIter {
        self.records.iter()
    }
}

// ── Graph ───────────────────────────────────────────────────────────

/// Contextual metadata about where a transclusion directive appears
/// within the parent document's section structure.
#[derive(Debug, Clone, Default, Serialize)]
pub struct ReferenceInsertionContext {
    /// The syntactic form of the directive that triggered insertion.
    pub directive_kind: Option<ReferenceSyntax>,
    /// The heading text of the section containing the directive (if any).
    pub section_heading_text: Option<String>,
    /// The heading level of the containing section (1–6).
    pub section_heading_level: Option<HeadingLevel>,
}

/// Records a child document insertion within a graph node.
#[derive(Debug, Clone)]
pub struct ReferenceInsertion {
    /// ID of the child node in the graph.
    pub child_node_id: NodeId,
    /// Line number of the directive that triggers insertion.
    pub directive_line: usize,
    /// Order among sibling insertions (0-based).
    pub insertion_order: usize,
    /// Links this insertion to its corresponding [`ReferenceRecord::id`].
    ///
    /// Used by the `FileTree` model builder to match each transclusion
    /// record to its child insertion without relying on line numbers alone
    /// (which are ambiguous for frontmatter prologues/epilogues).
    pub reference_id: Option<String>,
    /// Section context for the directive location.
    pub context: ReferenceInsertionContext,
}

impl ReferenceInsertion {
    /// Returns the directive kind as a short string used in JSON output.
    pub(crate) fn directive_kind_str(&self) -> &'static str {
        match self.context.directive_kind {
            Some(ReferenceSyntax::DirectiveFile) => "file",
            Some(ReferenceSyntax::DirectiveUrl) => "url",
            Some(ReferenceSyntax::DirectiveCode) => "code",
            Some(ReferenceSyntax::DirectiveTocLinking) => "toc_linking",
            Some(ReferenceSyntax::DirectiveFileLinks) => "file_links",
            Some(ReferenceSyntax::FrontmatterPrologue) => "prologue",
            Some(ReferenceSyntax::FrontmatterEpilogue) => "epilogue",
            _ => "unknown",
        }
    }
}

/// A single document node in the reference graph.
#[derive(Debug, Clone)]
pub struct ReferenceGraphNode {
    /// Unique node identifier.
    pub node_id: NodeId,
    /// Source of this document.
    pub source: ComposeSource,
    /// References local to this document.
    pub local_references: ReferenceSet,
    /// Child document insertions (transclusions).
    pub child_insertions: Vec<ReferenceInsertion>,
}

/// The full reference graph rooted at the entry document.
///
/// A `ReferenceGraph` is an opaque, immutable artifact: it is only ever
/// produced by the crate's graph builders (which route through
/// [`ReferenceGraph::from_build`]) and carries private build provenance used by
/// prebuilt-graph validation. Downstream callers inspect it through the
/// read-only accessors ([`root`](Self::root), [`nodes`](Self::nodes),
/// [`iter`](Self::iter), [`node_by_id`](Self::node_by_id),
/// [`node_count`](Self::node_count)) but cannot construct or mutate one.
#[derive(Clone)]
pub struct ReferenceGraph {
    /// The root document node.
    root: ReferenceGraphNode,
    /// All non-root nodes (children, grandchildren, etc.).
    nodes: Vec<ReferenceGraphNode>,
    /// Private build provenance (root/source/mode/options/dependencies).
    ///
    /// Never presented via `Debug` or serialization; read by prebuilt-graph
    /// validation, which is wired in Phase 3.
    #[allow(dead_code)]
    provenance: ReferenceGraphProvenance,
}

impl std::fmt::Debug for ReferenceGraph {
    /// Presents only the root and node list, exactly as the previous derived
    /// `Debug` did; the private provenance never appears.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ReferenceGraph")
            .field("root", &self.root)
            .field("nodes", &self.nodes)
            .finish()
    }
}

/// A view of a [`ReferenceGraphNode`] with the graph context needed to
/// serialize child insertions.
///
/// Crate-internal: public JSON serialization routes through the public
/// [`ReferenceGraphView`] instead.
pub(crate) struct ReferenceGraphNodeView<'a> {
    /// The node to serialize.
    pub(crate) node: &'a ReferenceGraphNode,
    /// The containing graph, used to resolve child node sources.
    pub(crate) graph: &'a ReferenceGraph,
    /// Whether to recursively expand child nodes.
    pub(crate) follow: bool,
}

/// A serializable, root-anchored view of a [`ReferenceGraph`].
///
/// Produced by [`ReferenceGraph::view`]. Its JSON is byte-for-byte
/// shape-compatible with the prior node-view output (`file`, `source`,
/// `references`, `transclusions`, and a nested `node` only when `follow` is
/// set); build provenance, hashes, graph mode, and the dependency manifest
/// never appear.
pub struct ReferenceGraphView<'a> {
    graph: &'a ReferenceGraph,
    follow: bool,
}

impl Serialize for ReferenceGraphView<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        ReferenceGraphNodeView {
            node: self.graph.root(),
            graph: self.graph,
            follow: self.follow,
        }
        .serialize(serializer)
    }
}

impl Serialize for ReferenceGraphNodeView<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("ReferenceGraphNode", 4)?;

        let file_name = match &self.node.source {
            ComposeSource::File(p) => p
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default(),
            ComposeSource::Url(u) => u.to_string(),
            ComposeSource::Unknown => String::new(),
        };

        let references: Vec<serde_json::Value> = self
            .node
            .local_references
            .records
            .iter()
            .filter(|r| r.kind != ReferenceKind::Transclusion)
            .map(|r| serde_json::to_value(r).map_err(serde::ser::Error::custom))
            .collect::<Result<Vec<_>, _>>()?;

        let transclusions: Vec<serde_json::Value> = self
            .node
            .child_insertions
            .iter()
            .map(|ins| {
                let child = self.graph.node_by_id(ins.child_node_id.as_ref());
                let target = child
                    .map(|n| serde_json::to_value(&n.source).map_err(serde::ser::Error::custom))
                    .transpose()?
                    .unwrap_or(serde_json::Value::Null);

                let mut obj = serde_json::Map::new();
                obj.insert("kind".to_string(), ins.directive_kind_str().into());
                obj.insert("target".to_string(), target);
                obj.insert("line".to_string(), ins.directive_line.into());
                obj.insert(
                    "followable".to_string(),
                    ins.context
                        .directive_kind
                        .map(|s| s.is_followable_transclusion())
                        .unwrap_or(false)
                        .into(),
                );
                if let Some(ref section) = ins.context.section_heading_text {
                    obj.insert("section".to_string(), section.clone().into());
                }
                if let Some(level) = ins.context.section_heading_level {
                    obj.insert("section_level".to_string(), level.as_u8().into());
                }
                if self.follow && let Some(child) = child {
                    obj.insert(
                        "node".to_string(),
                        serde_json::to_value(ReferenceGraphNodeView {
                            node: child,
                            graph: self.graph,
                            follow: self.follow,
                        })
                        .map_err(serde::ser::Error::custom)?,
                    );
                }
                Ok(serde_json::Value::Object(obj))
            })
            .collect::<Result<Vec<_>, _>>()?;

        state.serialize_field("file", &file_name)?;
        state.serialize_field("source", &self.node.source)?;
        state.serialize_field("references", &references)?;
        state.serialize_field("transclusions", &transclusions)?;
        state.end()
    }
}

impl ReferenceGraph {
    /// Assembles an opaque graph from a completed build, computing provenance
    /// itself so identities can never be mislabeled.
    ///
    /// Root/source/mode/options provenance is derived from `document`,
    /// `document.source()`, `mode`, and the options identity captured from
    /// `options.compose`; the visited-descendant `dependencies` manifest is
    /// stored as produced by the build runtime.
    pub(crate) fn from_build(
        document: &Markdown,
        options: &ReferenceGraphOptions,
        mode: ReferenceGraphMode,
        root: ReferenceGraphNode,
        nodes: Vec<ReferenceGraphNode>,
        dependencies: ReferenceDependencyManifest,
    ) -> ReferenceGraph {
        let provenance = ReferenceGraphProvenance::new(
            ReferenceDocumentIdentity::capture(document),
            document.source().clone(),
            mode,
            ReferenceGraphOptionsIdentity::capture(&options.compose),
            dependencies,
        );
        ReferenceGraph {
            root,
            nodes,
            provenance,
        }
    }

    /// The root document node.
    pub fn root(&self) -> &ReferenceGraphNode {
        &self.root
    }

    /// The non-root nodes (children, grandchildren, etc.).
    pub fn nodes(&self) -> &[ReferenceGraphNode] {
        &self.nodes
    }

    /// Iterates over every node, yielding the root exactly once first.
    pub fn iter(&self) -> impl Iterator<Item = &ReferenceGraphNode> + '_ {
        std::iter::once(&self.root).chain(self.nodes.iter())
    }

    /// The private build provenance, used by prebuilt-graph validation
    /// (wired in Phase 3).
    #[allow(dead_code)]
    pub(crate) fn provenance(&self) -> &ReferenceGraphProvenance {
        &self.provenance
    }

    /// Returns a serializable, root-anchored view of this graph.
    pub fn view(&self, follow: bool) -> ReferenceGraphView<'_> {
        ReferenceGraphView { graph: self, follow }
    }

    /// Finds a node by its ID.
    pub fn node_by_id(&self, id: &str) -> Option<&ReferenceGraphNode> {
        if self.root.node_id.as_ref() == id {
            return Some(&self.root);
        }
        self.nodes.iter().find(|n| n.node_id.as_ref() == id)
    }

    /// Total number of nodes (including root).
    pub fn node_count(&self) -> usize {
        1 + self.nodes.len()
    }

    /// Renders the graph as a Mermaid flowchart.
    pub fn to_mermaid(&self) -> String {
        let mut out = String::from("flowchart TD\n");

        // Emit nodes
        self.emit_mermaid_node(&self.root, &mut out);
        for node in &self.nodes {
            self.emit_mermaid_node(node, &mut out);
        }

        // Emit edges
        for node in std::iter::once(&self.root).chain(self.nodes.iter()) {
            for insertion in &node.child_insertions {
                let from = mermaid_safe_id(node.node_id.as_ref());
                let to = mermaid_safe_id(insertion.child_node_id.as_ref());
                out.push_str(&format!("    {from} --> {to}\n"));
            }
        }

        out
    }

    fn emit_mermaid_node(&self, node: &ReferenceGraphNode, out: &mut String) {
        let id = mermaid_safe_id(node.node_id.as_ref());
        let label = short_label(node.node_id.as_ref());
        let ref_count = node.local_references.len();
        out.push_str(&format!("    {id}[\"{label}<br/>{ref_count} refs\"]\n"));
    }

    /// Renders the graph as a DOT (Graphviz) diagram.
    pub fn to_dot(&self) -> String {
        let mut out = String::from("digraph reference_graph {\n");
        out.push_str("    rankdir=TD;\n");
        out.push_str("    node [shape=box, style=rounded];\n\n");

        // Emit nodes
        self.emit_dot_node(&self.root, &mut out);
        for node in &self.nodes {
            self.emit_dot_node(node, &mut out);
        }

        out.push('\n');

        // Emit edges
        for node in std::iter::once(&self.root).chain(self.nodes.iter()) {
            for insertion in &node.child_insertions {
                let from = dot_safe_id(node.node_id.as_ref());
                let to = dot_safe_id(insertion.child_node_id.as_ref());
                out.push_str(&format!("    {from} -> {to};\n"));
            }
        }

        out.push_str("}\n");
        out
    }

    fn emit_dot_node(&self, node: &ReferenceGraphNode, out: &mut String) {
        let id = dot_safe_id(node.node_id.as_ref());
        let label = short_label(node.node_id.as_ref());
        let ref_count = node.local_references.len();
        out.push_str(&format!(
            "    {id} [label=\"{label}\\n{ref_count} refs\"];\n"
        ));
    }
}

/// Creates a Mermaid-safe node ID.
fn mermaid_safe_id(id: &str) -> String {
    id.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// Creates a DOT-safe node ID.
fn dot_safe_id(id: &str) -> String {
    let safe: String = id
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    format!("n_{safe}")
}

/// Extracts a short label from a full path/URL.
fn short_label(id: &str) -> String {
    if let Some(file_name) = std::path::Path::new(id).file_name() {
        file_name.to_string_lossy().to_string()
    } else {
        id.to_string()
    }
}

// ── Options ─────────────────────────────────────────────────────────

use crate::markdown::compose::ComposeOptions;

/// Options for building a reference graph.
///
/// Use `with_compose()` to supply pre-configured compose options that
/// share an already-captured context, avoiding redundant capture work.
#[derive(Clone)]
pub struct ReferenceGraphOptions {
    /// Compose options controlling InlinePre execution, cache settings,
    /// external state, shell settings, and other pipeline behavior.
    pub compose: ComposeOptions,
}

impl Default for ReferenceGraphOptions {
    /// Creates default options, which eagerly captures runtime context.
    ///
    /// Prefer `ReferenceGraphOptions::with_compose()` when a
    /// `ComposeOptions` (with its captured context) is already available.
    fn default() -> Self {
        Self {
            compose: ComposeOptions::default(),
        }
    }
}

impl ReferenceGraphOptions {
    /// Creates options that share the compose options (and their captured
    /// context) instead of triggering a new capture.
    pub fn with_compose(compose: ComposeOptions) -> Self {
        Self { compose }
    }
}

// ── Transclusion query types ────────────────────────────────────────

/// A local transclusion reference (not yet resolved into a graph).
#[derive(Debug, Clone)]
pub struct TransclusionRef {
    /// The kind of transclusion.
    pub kind: TransclusionRefKind,
    /// Raw target string from the directive.
    pub raw_target: String,
    /// Resolved target path/URL (if source context available).
    pub resolved_target: Option<String>,
    /// Parsed directive options.
    pub options: TransclusionRefOptions,
    /// Provenance information.
    pub origin: ReferenceOrigin,
}

/// Kind of transclusion reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransclusionRefKind {
    /// `::file` directive.
    File,
    /// `::code` directive.
    Code,
    /// `::url` directive.
    Url,
    /// `::toc-linking` directive.
    TocLinking,
    /// `::file-links` directive.
    FileLinks,
    /// Frontmatter `prologue` reference.
    Prologue,
    /// Frontmatter `epilogue` reference.
    Epilogue,
}

/// Options parsed from a transclusion directive.
#[derive(Debug, Clone, Default)]
pub struct TransclusionRefOptions {
    /// Optional `when` condition expression.
    pub when_expr: Option<String>,
    /// How frontmatter key conflicts are resolved during transclusion.
    pub replace: crate::markdown::compose::transclusion::ReplaceOption,
    /// Quotation wrapper text.
    pub quotation: Option<String>,
    /// Disclosure summary text.
    pub disclosure: Option<String>,
    /// Heading sections to exclude.
    pub exclude: Vec<String>,
}

// ── Convenience wrappers ────────────────────────────────────────────

/// A hyperlink reference with display text.
#[derive(Debug, Clone)]
pub struct LinkReference {
    /// Underlying reference record.
    pub record: ReferenceRecord,
    /// Link display text.
    pub display: String,
    /// Link title attribute.
    pub title: Option<String>,
}

impl From<ReferenceRecord> for LinkReference {
    fn from(record: ReferenceRecord) -> Self {
        let display = record
            .attributes
            .get("display")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let title = record
            .attributes
            .get("title")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        Self {
            record,
            display,
            title,
        }
    }
}

/// An image reference with alt text and dimensions.
#[derive(Debug, Clone)]
pub struct ImageReference {
    /// Underlying reference record.
    pub record: ReferenceRecord,
    /// Image alt text.
    pub alt: String,
    /// Image title attribute.
    pub title: Option<String>,
    /// Width hint (from alt text or attributes).
    pub width: Option<u32>,
    /// Height hint (from attributes).
    pub height: Option<u32>,
}

impl From<ReferenceRecord> for ImageReference {
    fn from(record: ReferenceRecord) -> Self {
        let alt = record
            .attributes
            .get("alt")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let title = record
            .attributes
            .get("title")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let width = record
            .attributes
            .get("width")
            .and_then(|v| v.as_u64())
            .map(|v| v as u32);
        let height = record
            .attributes
            .get("height")
            .and_then(|v| v.as_u64())
            .map(|v| v as u32);
        Self {
            record,
            alt,
            title,
            width,
            height,
        }
    }
}

// ── Phase 2 convenience wrappers ─────────────────────────────────────

/// An inline CSS block extracted from the document.
#[derive(Debug, Clone)]
pub struct InlineCssBlock {
    /// Underlying reference record.
    pub record: ReferenceRecord,
    /// Raw CSS content.
    pub css_content: String,
}

impl From<ReferenceRecord> for InlineCssBlock {
    fn from(record: ReferenceRecord) -> Self {
        let css_content = record
            .attributes
            .get("css_content")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        Self {
            record,
            css_content,
        }
    }
}

/// An inline script block extracted from the document.
#[derive(Debug, Clone)]
pub struct InlineScriptBlock {
    /// Underlying reference record.
    pub record: ReferenceRecord,
    /// Raw script content.
    pub script_content: String,
}

impl From<ReferenceRecord> for InlineScriptBlock {
    fn from(record: ReferenceRecord) -> Self {
        let script_content = record
            .attributes
            .get("script_content")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        Self {
            record,
            script_content,
        }
    }
}

/// An import reference (CSS, script, or font).
#[derive(Debug, Clone)]
pub struct ImportReference {
    /// Underlying reference record.
    pub record: ReferenceRecord,
    /// The import URL/path.
    pub href: String,
}

impl From<ReferenceRecord> for ImportReference {
    fn from(record: ReferenceRecord) -> Self {
        let href = record.target.raw().unwrap_or("").to_string();
        Self { record, href }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_local_path() {
        let target = classify_target("./file.md");
        assert!(matches!(target, ReferenceTarget::LocalPath { .. }));
        assert_eq!(target.raw(), Some("./file.md"));
    }

    #[test]
    fn classify_remote_url() {
        let target = classify_target("https://example.com/page");
        assert!(matches!(target, ReferenceTarget::RemoteUrl { .. }));
    }

    #[test]
    fn classify_http_url() {
        let target = classify_target("http://example.com");
        assert!(matches!(target, ReferenceTarget::RemoteUrl { .. }));
    }

    #[test]
    fn classify_protocol_relative_url() {
        let target = classify_target("//cdn.example.com/app.js");
        assert!(matches!(target, ReferenceTarget::RemoteUrl { .. }));
    }

    #[test]
    fn classify_fragment() {
        let target = classify_target("#section");
        assert!(matches!(target, ReferenceTarget::Fragment { .. }));
    }

    #[test]
    fn classify_data_uri() {
        let target = classify_target("data:image/png;base64,abc");
        assert!(matches!(target, ReferenceTarget::DataUri { .. }));
    }

    #[test]
    fn classify_mailto() {
        let target = classify_target("mailto:user@example.com");
        match &target {
            ReferenceTarget::OtherScheme { scheme, .. } => assert_eq!(scheme, "mailto"),
            _ => panic!("expected OtherScheme, got {target:?}"),
        }
    }

    #[test]
    fn classify_tel() {
        let target = classify_target("tel:+1234567890");
        assert!(matches!(target, ReferenceTarget::OtherScheme { .. }));
    }

    #[test]
    fn inline_target_has_no_raw() {
        assert_eq!(ReferenceTarget::Inline.raw(), None);
    }

    #[test]
    fn reference_set_filtering() {
        let set = ReferenceSet {
            records: vec![
                ReferenceRecord {
                    id: "a".into(),
                    kind: ReferenceKind::Hyperlink,
                    target: ReferenceTarget::RemoteUrl {
                        raw: "https://example.com".into(),
                    },
                    origin: ReferenceOrigin {
                        source: ComposeSource::Unknown,
                        line: 1,
                        span: 0..10,
                        syntax: ReferenceSyntax::MarkdownLink,
                    },
                    attributes: serde_json::Map::new(),
                },
                ReferenceRecord {
                    id: "b".into(),
                    kind: ReferenceKind::Image,
                    target: ReferenceTarget::LocalPath {
                        raw: "img.png".into(),
                    },
                    origin: ReferenceOrigin {
                        source: ComposeSource::Unknown,
                        line: 2,
                        span: 10..20,
                        syntax: ReferenceSyntax::MarkdownImage,
                    },
                    attributes: serde_json::Map::new(),
                },
            ],
        };

        assert_eq!(set.len(), 2);
        assert_eq!(set.hyperlinks().len(), 1);
        assert_eq!(set.images().len(), 1);
        assert_eq!(set.transclusions().len(), 0);
    }

    #[test]
    fn make_reference_id_stable() {
        let id1 = make_reference_id(&ComposeSource::Unknown, 5, 42);
        let id2 = make_reference_id(&ComposeSource::Unknown, 5, 42);
        assert_eq!(id1, id2);

        // Different line → different ID
        let id3 = make_reference_id(&ComposeSource::Unknown, 6, 42);
        assert_ne!(id1, id3);
    }

    #[test]
    fn reference_record_serializes_to_expected_shape() {
        let record = ReferenceRecord {
            id: "abc:1:2".into(),
            kind: ReferenceKind::Hyperlink,
            target: ReferenceTarget::LocalPath {
                raw: "./file.md".into(),
            },
            origin: ReferenceOrigin {
                source: ComposeSource::Unknown,
                line: 3,
                span: 0..10,
                syntax: ReferenceSyntax::MarkdownLink,
            },
            attributes: serde_json::Map::new(),
        };

        let value = serde_json::to_value(&record).unwrap();
        let obj = value.as_object().unwrap();
        assert_eq!(obj.get("id").unwrap(), "abc:1:2");
        assert_eq!(obj.get("kind").unwrap(), "hyperlink");
        assert_eq!(obj.get("line").unwrap(), 3);
        assert_eq!(obj.get("syntax").unwrap(), "markdown_link");

        let target = obj.get("target").unwrap().as_object().unwrap();
        assert_eq!(target.get("type").unwrap(), "local_path");
        assert_eq!(target.get("raw").unwrap(), "./file.md");
    }

    #[test]
    fn reference_graph_node_view_serializes_transclusion_target() {
        let child = ReferenceGraphNode {
            node_id: NodeId::from("child"),
            source: ComposeSource::Url("https://example.com/page.md".parse().unwrap()),
            local_references: ReferenceSet { records: Vec::new() },
            child_insertions: Vec::new(),
        };
        let root = ReferenceGraphNode {
            node_id: NodeId::from("root"),
            source: ComposeSource::File(PathBuf::from("/tmp/root.md")),
            local_references: ReferenceSet { records: Vec::new() },
            child_insertions: vec![ReferenceInsertion {
                child_node_id: NodeId::from("child"),
                directive_line: 5,
                insertion_order: 0,
                reference_id: None,
                context: ReferenceInsertionContext {
                    directive_kind: Some(ReferenceSyntax::DirectiveFile),
                    section_heading_text: None,
                    section_heading_level: None,
                },
            }],
        };
        let graph = ReferenceGraph::from_build(
            &Markdown::new(""),
            &ReferenceGraphOptions::default(),
            ReferenceGraphMode::Full,
            root,
            vec![child],
            ReferenceDependencyManifest::default(),
        );

        let value = serde_json::to_value(graph.view(false)).unwrap();
        let obj = value.as_object().unwrap();
        assert_eq!(obj.get("file").unwrap(), "root.md");
        assert_eq!(
            obj.get("source").unwrap(),
            "/tmp/root.md"
        );

        let transclusions = obj.get("transclusions").unwrap().as_array().unwrap();
        assert_eq!(transclusions.len(), 1);
        let ins = transclusions[0].as_object().unwrap();
        assert_eq!(ins.get("kind").unwrap(), "file");
        assert_eq!(ins.get("line").unwrap(), 5);
        assert_eq!(ins.get("followable").unwrap(), true);
        assert_eq!(
            ins.get("target").unwrap(),
            "https://example.com/page.md"
        );
        assert!(!ins.contains_key("node"));
    }

    #[test]
    fn reference_graph_node_view_follow_expands_child_node() {
        let child = ReferenceGraphNode {
            node_id: NodeId::from("child"),
            source: ComposeSource::File(PathBuf::from("/tmp/child.md")),
            local_references: ReferenceSet { records: Vec::new() },
            child_insertions: Vec::new(),
        };
        let root = ReferenceGraphNode {
            node_id: NodeId::from("root"),
            source: ComposeSource::File(PathBuf::from("/tmp/root.md")),
            local_references: ReferenceSet { records: Vec::new() },
            child_insertions: vec![ReferenceInsertion {
                child_node_id: NodeId::from("child"),
                directive_line: 1,
                insertion_order: 0,
                reference_id: None,
                context: ReferenceInsertionContext {
                    directive_kind: Some(ReferenceSyntax::DirectiveFile),
                    section_heading_text: None,
                    section_heading_level: None,
                },
            }],
        };
        let graph = ReferenceGraph::from_build(
            &Markdown::new(""),
            &ReferenceGraphOptions::default(),
            ReferenceGraphMode::Full,
            root,
            vec![child],
            ReferenceDependencyManifest::default(),
        );

        let value = serde_json::to_value(graph.view(true)).unwrap();
        let ins = value["transclusions"][0].as_object().unwrap();
        assert!(ins.contains_key("node"));
        assert_eq!(ins["node"]["file"], "child.md");
    }

    /// Builds a two-node opaque graph for accessor/presentation assertions.
    fn two_node_graph() -> ReferenceGraph {
        let child = ReferenceGraphNode {
            node_id: NodeId::from("child"),
            source: ComposeSource::File(PathBuf::from("/tmp/child.md")),
            local_references: ReferenceSet { records: Vec::new() },
            child_insertions: Vec::new(),
        };
        let root = ReferenceGraphNode {
            node_id: NodeId::from("root"),
            source: ComposeSource::File(PathBuf::from("/tmp/root.md")),
            local_references: ReferenceSet { records: Vec::new() },
            child_insertions: Vec::new(),
        };
        ReferenceGraph::from_build(
            &Markdown::new(""),
            &ReferenceGraphOptions::default(),
            ReferenceGraphMode::Full,
            root,
            vec![child],
            ReferenceDependencyManifest::default(),
        )
    }

    #[test]
    fn iter_yields_root_exactly_once_then_nodes_in_order() {
        let graph = two_node_graph();
        let ids: Vec<&str> = graph.iter().map(|n| n.node_id.as_ref()).collect();
        assert_eq!(ids, vec!["root", "child"]);
        // `iter()` visits every node once and agrees with `node_count()`.
        assert_eq!(graph.iter().count(), graph.node_count());
        assert_eq!(graph.iter().count(), 2);
        // Root appears exactly once across the whole iteration.
        assert_eq!(ids.iter().filter(|id| **id == "root").count(), 1);
    }

    #[test]
    fn accessors_agree_with_iteration() {
        let graph = two_node_graph();
        assert_eq!(graph.root().node_id.as_ref(), "root");
        assert_eq!(graph.nodes().len(), 1);
        assert_eq!(graph.nodes()[0].node_id.as_ref(), "child");
        assert!(graph.node_by_id("child").is_some());
        assert!(graph.node_by_id("root").is_some());
        assert!(graph.node_by_id("absent").is_none());
    }

    #[test]
    fn debug_presents_root_and_nodes_without_leaking_provenance() {
        let graph = two_node_graph();
        let dbg = format!("{graph:?}");
        // Preserves the previous derived presentation …
        assert!(dbg.contains("ReferenceGraph"));
        assert!(dbg.contains("root"));
        assert!(dbg.contains("nodes"));
        // … and never exposes private provenance or its hashes/mode/manifest.
        assert!(!dbg.contains("provenance"));
        assert!(!dbg.to_lowercase().contains("fingerprint"));
        assert!(!dbg.contains("dependencies"));
    }
}
