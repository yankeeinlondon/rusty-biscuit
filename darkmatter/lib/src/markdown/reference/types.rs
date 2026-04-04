//! Core type definitions for the reference analysis subsystem.

use crate::markdown::compose::ComposeSource;
use crate::markdown::normalize::HeadingLevel;
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReferenceKind {
    /// A hyperlink (`<a>` or markdown link).
    Hyperlink,
    /// An image (`<img>` or markdown image).
    Image,
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

/// The syntactic form in which a reference appears.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReferenceSyntax {
    /// `[text](url)` markdown link.
    MarkdownLink,
    /// `<a href="...">` HTML anchor.
    HtmlAnchor,
    /// `![alt](src)` markdown image.
    MarkdownImage,
    /// `<img src="...">` HTML image.
    HtmlImage,
    /// `::file path` directive.
    DirectiveFile,
    /// `::url https://...` directive.
    DirectiveUrl,
    /// `::code path` directive.
    DirectiveCode,
    /// `::toc-linking path` directive.
    DirectiveTocLinking,
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
#[derive(Debug, Clone, PartialEq, Eq)]
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
#[derive(Debug, Clone)]
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
#[derive(Debug, Clone, Default)]
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
#[derive(Debug, Clone)]
pub struct ReferenceGraph {
    /// The root document node.
    pub root: ReferenceGraphNode,
    /// All non-root nodes (children, grandchildren, etc.).
    pub nodes: Vec<ReferenceGraphNode>,
}

impl ReferenceGraph {
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
}
