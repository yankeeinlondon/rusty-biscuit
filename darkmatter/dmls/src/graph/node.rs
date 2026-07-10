//! Graph node model: the typed node identity, kinds, and per-kind payloads.
//!
//! The substrate indexer emits the Markdown structural kinds
//! ([`NodeKind::Document`], [`NodeKind::Heading`], [`NodeKind::Link`],
//! [`NodeKind::WikiLink`]) plus the Darkmatter semantic sources the single
//! graph carries as typed edges: [`NodeKind::TransclusionTarget`],
//! [`NodeKind::SchemaDeclaration`] (`uses_schema`), [`NodeKind::FileReference`]
//! (`uses_file`), and [`NodeKind::Interpolation`] → [`NodeKind::FrontmatterKey`]
//! (`uses_variable`). Every source is extracted through the side-effect-free
//! `darkmatter` span APIs, so the single-graph-with-edge-kinds bet (AD-1/AD-2)
//! is one vocabulary, not two.

use darkmatter::markdown::span::SourceSpan;

use super::arena::DocumentId;

/// Stable index of a node inside a [`WorkspaceGraph`](super::WorkspaceGraph)
/// snapshot.
///
/// A `NodeId` is only meaningful within the snapshot generation that produced
/// it; snapshot swaps mint fresh ids (readers hold whole snapshots, not bare
/// ids across generations).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NodeId(pub u32);

/// The kind of a graph node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeKind {
    // ── Markdown structural ──
    /// A whole document (the root node for one file).
    Document,
    /// A heading (ATX or setext), the anchor/symbol authority.
    Heading,
    /// An inline or reference-style Markdown link.
    Link,
    /// A `[[wiki]]` link (Layer 1 / R-8).
    WikiLink,
    /// A link-reference definition (`[id]: url`).
    LinkDefinition,

    // ── Darkmatter semantic sources (emitted by the substrate indexer) ──
    /// A top-level frontmatter key — the target a `uses_variable` edge resolves
    /// to.
    FrontmatterKey,
    /// A `$schema` file reference — the source of a `uses_schema` edge.
    SchemaDeclaration,
    /// A `{{ … }}` interpolation — the source of a `uses_variable` edge.
    Interpolation,
    /// A file use (image, frontmatter `file(...)` value, style asset, or
    /// `::file-links`/`::toc-linking` directive path) — the source of a
    /// `uses_file` edge.
    FileReference,
    /// A transclusion target (`::file`/`::code`/prologue/epilogue) — the source
    /// of a `transcludes` edge.
    TransclusionTarget,

    // ── Declared for later phases, not yet emitted ──
    /// The frontmatter block.
    FrontmatterBlock,
    /// A `style:` declaration.
    StyleDeclaration,
    /// A `::` directive.
    Directive,
    /// A directive argument/option.
    DirectiveArgument,
    /// A `::block` condition expression.
    Condition,
    /// A `::shell` directive.
    ShellDirective,
    /// A disclosure block.
    Disclosure,
    /// A `::block` page block.
    PageBlock,
    /// A fenced code block's info string.
    FenceInfoString,
}

impl NodeKind {
    /// Whether this kind is a Markdown structural node (document, heading, or
    /// link), as opposed to a Darkmatter semantic source or a
    /// declared-but-unemitted overlay kind.
    pub fn is_substrate(self) -> bool {
        matches!(
            self,
            NodeKind::Document
                | NodeKind::Heading
                | NodeKind::Link
                | NodeKind::WikiLink
                | NodeKind::LinkDefinition
        )
    }
}

/// Parsed intent of a Markdown link target, before graph resolution.
///
/// Resolution to a concrete [`NodeId`] happens at snapshot assembly against
/// the document/path map and heading slugs; wiki-link forms and richer
/// ranking land in Phase 5.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkTarget {
    /// `#heading-slug` — an anchor within the same document.
    SameDocumentAnchor {
        /// The referenced slug (leading `#` stripped).
        slug: String,
    },
    /// A workspace-local path, with an optional `#fragment`.
    RelativePath {
        /// The raw path portion (before any `#`).
        path: String,
        /// The optional fragment (leading `#` stripped).
        fragment: Option<String>,
    },
    /// An `http(s)://`, `mailto:`, `data:`, or other non-local target — never
    /// a graph target.
    External,
}

/// Kind-specific node data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodePayload {
    /// The document root carries no extra payload (path lives on the
    /// [`DocumentRecord`](super::arena::DocumentRecord)).
    Document,
    /// A heading's structured facts.
    Heading(HeadingPayload),
    /// A link's raw target, display text, and parsed intent.
    Link(LinkPayload),
    /// A `[[wiki]]` link's parse product and resolution outcome.
    WikiLink(WikiLinkPayload),
    /// A `::file`/`::code` transclusion target (Layer 3 overlay).
    Transclusion(TransclusionPayload),
    /// A `$schema` or file use's raw target and resolved path
    /// ([`NodeKind::SchemaDeclaration`] / [`NodeKind::FileReference`]).
    FileRef(FileRefPayload),
    /// A `{{ … }}` interpolation's leading variable identifier
    /// ([`NodeKind::Interpolation`]).
    VariableUse(VariableUsePayload),
    /// A top-level frontmatter key ([`NodeKind::FrontmatterKey`]).
    FrontmatterKey(FrontmatterKeyPayload),
    /// Reserved for overlay kinds not yet emitted.
    Empty,
}

/// Structured data for a [`NodeKind::SchemaDeclaration`] or
/// [`NodeKind::FileReference`] node — a source position that uses an external
/// file (a `$schema` file reference, an image, a frontmatter `file(...)` value,
/// a `style.page.stylesheet` asset, or a `::file-links`/`::toc-linking`
/// directive path).
///
/// A `uses_schema`/`uses_file` edge links this node to the referenced
/// document's root when `path` names an indexed Markdown document; a non-`.md`
/// asset (image, schema YAML, directory) or a path outside the workspace
/// carries an [`EdgeTarget::Unresolved`](super::edge::EdgeTarget) target
/// instead, so the reference is recorded without any filesystem touch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileRefPayload {
    /// Raw reference exactly as authored (may carry a `#fragment`).
    pub raw_target: String,
    /// The workspace-local path portion (any `#fragment` stripped).
    pub path: String,
    /// 1-indexed source line the reference is on.
    pub line: usize,
}

/// Structured data for a [`NodeKind::Interpolation`] node — one `{{ … }}`
/// variable use.
///
/// A `uses_variable` edge links this node to the same-document
/// [`NodeKind::FrontmatterKey`] node when the leading identifier names a
/// top-level frontmatter key; a `ctx.*` / `env.*` / function / unknown
/// identifier carries an [`EdgeTarget::Unresolved`](super::edge::EdgeTarget)
/// target instead.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VariableUsePayload {
    /// The leading identifier of the interpolation expression (`title`,
    /// `ctx.today`, `author.name`), exactly as the compose parser sees it.
    pub name: String,
    /// 1-indexed source line the interpolation is on.
    pub line: usize,
}

/// Structured data for a [`NodeKind::FrontmatterKey`] node — a top-level
/// authored frontmatter key, the target a `uses_variable` edge resolves to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrontmatterKeyPayload {
    /// The key name.
    pub key: String,
    /// 1-indexed source line the key is on.
    pub line: usize,
}

/// Structured data for a [`NodeKind::Heading`] node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeadingPayload {
    /// Heading level (1–6).
    pub level: u8,
    /// Rendered inline text (markup stripped).
    pub title: String,
    /// Document-unique GitHub anchor slug.
    pub slug: String,
    /// Byte span of the heading's inline text.
    pub title_span: SourceSpan,
    /// 1-indexed source line the heading starts on.
    pub line: usize,
}

/// Structured data for a [`NodeKind::Link`] node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkPayload {
    /// Raw target string exactly as authored.
    pub raw_target: String,
    /// The link's visible display text.
    pub display: String,
    /// Parsed target intent.
    pub target: LinkTarget,
    /// 1-indexed source line the link is on (for cross-document locations
    /// where no source map is available).
    pub line: usize,
}

/// Structured data for a [`NodeKind::TransclusionTarget`] node.
///
/// Emitted by the Layer-3 overlay for `::file`/`::code` directives whose target
/// is a workspace-local path. A `transcludes` edge links this node to the
/// resolved target document root when the path matches an indexed document; a
/// non-`.md` target (e.g. `::code ./mod.rs`) or a broken path carries no edge
/// and is diagnosed at request time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransclusionPayload {
    /// Raw directive target exactly as authored (may carry a `#fragment`).
    pub raw_target: String,
    /// The workspace-local path portion (any `#fragment` stripped).
    pub path: String,
    /// 1-indexed source line the directive is on.
    pub line: usize,
}

/// Structured data for a [`NodeKind::WikiLink`] node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WikiLinkPayload {
    /// Unescaped file-target text (empty for a headings-only `[[#heading]]`).
    pub target: String,
    /// Unescaped heading text, when the link had a `#`.
    pub heading: Option<String>,
    /// Unescaped alias text, when the link had a `|`.
    pub alias: Option<String>,
    /// Document-relative byte span of the file-target text (completion edits
    /// replace this range).
    pub target_span: SourceSpan,
    /// Document-relative byte span of the heading text, when present.
    pub heading_span: Option<SourceSpan>,
    /// 1-indexed source line the link is on.
    pub line: usize,
    /// The resolution outcome computed at snapshot assembly.
    pub resolution: WikiResolution,
    /// A low-severity note about the resolution (never blocks navigation).
    pub info: Option<WikiInfo>,
}

/// The outcome of resolving one wiki link against the workspace (R-8).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WikiResolution {
    /// Resolved to exactly one target node (a document root or a heading).
    Resolved(NodeId),
    /// The file resolved but the `#heading` fragment matched no heading; the
    /// node is the resolved file's root so navigation still works.
    HeadingMissing(NodeId),
    /// Multiple file candidates (document roots), in stable display order.
    Ambiguous(Vec<NodeId>),
    /// No file candidate matched.
    Unresolved,
    /// `[[]]` / `[[|alias]]` — an empty target.
    EmptyTarget,
    /// `[[target#]]` — an empty heading query; `file` is the resolved target.
    EmptyHeading(Option<NodeId>),
    /// A v1-unsupported form (embed, block reference).
    Unsupported,
}

/// A low-severity note attached to a resolved wiki link.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WikiInfo {
    /// The heading resolved by exact text, but a *different* heading would have
    /// matched by GitHub slug (R-8 heading rule 7).
    HeadingSpellingConflict,
    /// The target resolved to a file whose name still carries a Markdown
    /// extension after elision, e.g. `note.md.md` (R-8 extension rule 5).
    ConfusingExtension,
    /// The target's percent escape was malformed and treated literally.
    InvalidPercentEscape,
}

/// One node in the graph arena.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    /// Owning document.
    pub document: DocumentId,
    /// Node kind.
    pub kind: NodeKind,
    /// Document-relative byte span of the node's source.
    pub span: SourceSpan,
    /// Kind-specific payload.
    pub payload: NodePayload,
}

impl Node {
    /// The heading payload, if this is a heading node.
    pub fn as_heading(&self) -> Option<&HeadingPayload> {
        match &self.payload {
            NodePayload::Heading(payload) => Some(payload),
            _ => None,
        }
    }

    /// The link payload, if this is a link node.
    pub fn as_link(&self) -> Option<&LinkPayload> {
        match &self.payload {
            NodePayload::Link(payload) => Some(payload),
            _ => None,
        }
    }

    /// The wiki-link payload, if this is a wiki-link node.
    pub fn as_wiki_link(&self) -> Option<&WikiLinkPayload> {
        match &self.payload {
            NodePayload::WikiLink(payload) => Some(payload),
            _ => None,
        }
    }

    /// The transclusion payload, if this is a transclusion-target node.
    pub fn as_transclusion(&self) -> Option<&TransclusionPayload> {
        match &self.payload {
            NodePayload::Transclusion(payload) => Some(payload),
            _ => None,
        }
    }

    /// The file-reference payload, if this is a `$schema`/file-use node.
    pub fn as_file_ref(&self) -> Option<&FileRefPayload> {
        match &self.payload {
            NodePayload::FileRef(payload) => Some(payload),
            _ => None,
        }
    }

    /// The variable-use payload, if this is an interpolation node.
    pub fn as_variable_use(&self) -> Option<&VariableUsePayload> {
        match &self.payload {
            NodePayload::VariableUse(payload) => Some(payload),
            _ => None,
        }
    }

    /// The frontmatter-key payload, if this is a frontmatter-key node.
    pub fn as_frontmatter_key(&self) -> Option<&FrontmatterKeyPayload> {
        match &self.payload {
            NodePayload::FrontmatterKey(payload) => Some(payload),
            _ => None,
        }
    }
}

/// Classifies a raw link target string into a [`LinkTarget`].
///
/// This is pure string classification (no filesystem access): a leading `#`
/// is a same-document anchor, an `http(s)`/`mailto`/`data`/scheme target is
/// [`LinkTarget::External`], and everything else is a workspace-relative path
/// with an optional fragment.
pub fn classify_link_target(raw: &str) -> LinkTarget {
    if let Some(anchor) = raw.strip_prefix('#') {
        return LinkTarget::SameDocumentAnchor {
            slug: anchor.to_string(),
        };
    }
    if is_external(raw) {
        return LinkTarget::External;
    }
    match raw.split_once('#') {
        Some((path, fragment)) => LinkTarget::RelativePath {
            path: path.to_string(),
            fragment: (!fragment.is_empty()).then(|| fragment.to_string()),
        },
        None => LinkTarget::RelativePath {
            path: raw.to_string(),
            fragment: None,
        },
    }
}

/// Whether a raw target is a non-local (external) reference.
fn is_external(raw: &str) -> bool {
    raw.starts_with("http://")
        || raw.starts_with("https://")
        || raw.starts_with("//")
        || raw.starts_with("mailto:")
        || raw.starts_with("tel:")
        || raw.starts_with("data:")
        // Any other `scheme:` prefix (but not a bare Windows drive `c:` path —
        // those never appear as authored Markdown link targets).
        || raw.contains("://")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_same_document_anchor() {
        assert_eq!(
            classify_link_target("#getting-started"),
            LinkTarget::SameDocumentAnchor {
                slug: "getting-started".to_string()
            }
        );
    }

    #[test]
    fn test_classify_relative_path_with_fragment() {
        assert_eq!(
            classify_link_target("guide/setup.md#install"),
            LinkTarget::RelativePath {
                path: "guide/setup.md".to_string(),
                fragment: Some("install".to_string()),
            }
        );
    }

    #[test]
    fn test_classify_relative_path_without_fragment() {
        assert_eq!(
            classify_link_target("../notes.md"),
            LinkTarget::RelativePath {
                path: "../notes.md".to_string(),
                fragment: None,
            }
        );
    }

    #[test]
    fn test_classify_trailing_empty_fragment_is_none() {
        assert_eq!(
            classify_link_target("page.md#"),
            LinkTarget::RelativePath {
                path: "page.md".to_string(),
                fragment: None,
            }
        );
    }

    #[test]
    fn test_classify_external() {
        assert_eq!(classify_link_target("https://example.com"), LinkTarget::External);
        assert_eq!(classify_link_target("mailto:x@y.z"), LinkTarget::External);
        assert_eq!(classify_link_target("//cdn.example/x"), LinkTarget::External);
    }

    #[test]
    fn test_substrate_kind_classification() {
        assert!(NodeKind::Heading.is_substrate());
        assert!(!NodeKind::Directive.is_substrate());
    }
}
