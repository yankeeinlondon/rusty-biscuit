//! Graph node model: the typed node identity, kinds, and per-kind payloads.
//!
//! The substrate indexer (Phase 3) emits only [`NodeKind::Document`],
//! [`NodeKind::Heading`], and [`NodeKind::Link`]; the overlay variants are
//! declared now so the single-graph-with-edge-kinds bet (AD-1/AD-2) is the
//! same seam the Darkmatter DSL overlay writes into in later phases, without a
//! second graph vocabulary.

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
///
/// Substrate kinds are produced by the Markdown indexer; overlay kinds are
/// reserved for the Darkmatter semantic overlay (Phases 7 and 9).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeKind {
    // ── Substrate (Markdown) ──
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

    // ── Overlay (Darkmatter) — declared, not yet emitted ──
    /// The frontmatter block.
    FrontmatterBlock,
    /// A frontmatter key path.
    FrontmatterKey,
    /// A `$schema` declaration.
    SchemaDeclaration,
    /// A `style:` declaration.
    StyleDeclaration,
    /// A `::` directive.
    Directive,
    /// A directive argument/option.
    DirectiveArgument,
    /// A `{{ … }}` interpolation expression.
    Interpolation,
    /// A `::block` condition expression.
    Condition,
    /// A transclusion target (`::file`/`::code`/prologue/epilogue).
    TransclusionTarget,
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
    /// Whether this kind is produced by the Markdown substrate indexer.
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
    /// Reserved for overlay kinds not yet emitted.
    Empty,
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
