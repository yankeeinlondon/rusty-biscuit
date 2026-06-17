//! The canonical owned render tree node and its kinds.

use std::borrow::Cow;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::tree::attrs::NodeAttrs;
use crate::tree::source::SourceSpan;

/// Error returned when constructing an out-of-range [`HeadingDepth`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("heading depth must be between 1 and 6, got {0}")]
pub struct HeadingDepthError(pub u8);

/// A heading depth constrained to the valid range `1..=6`.
///
/// ## Examples
///
/// ```
/// use renderable::tree::HeadingDepth;
///
/// let depth = HeadingDepth::new(2).unwrap();
/// assert_eq!(depth.get(), 2);
/// assert!(HeadingDepth::new(0).is_err());
/// assert!(HeadingDepth::new(7).is_err());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "u8", into = "u8")]
pub struct HeadingDepth(u8);

impl HeadingDepth {
    /// Creates a heading depth, validating the `1..=6` range.
    ///
    /// ## Errors
    ///
    /// Returns [`HeadingDepthError`] if `depth` is `0` or greater than `6`.
    pub fn new(depth: u8) -> Result<Self, HeadingDepthError> {
        if (1..=6).contains(&depth) {
            Ok(Self(depth))
        } else {
            Err(HeadingDepthError(depth))
        }
    }

    /// Returns the underlying depth value.
    #[must_use]
    pub fn get(self) -> u8 {
        self.0
    }
}

impl TryFrom<u8> for HeadingDepth {
    type Error = HeadingDepthError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<HeadingDepth> for u8 {
    fn from(depth: HeadingDepth) -> Self {
        depth.0
    }
}

/// Horizontal alignment for a table column.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ColumnAlign {
    /// Left-aligned column.
    Left,
    /// Center-aligned column.
    Center,
    /// Right-aligned column.
    Right,
    /// No explicit alignment.
    None,
}

/// The kind of a [`RenderNode`], carrying kind-specific payload and children.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum NodeKind {
    /// The document root.
    Root {
        /// Child nodes.
        children: Vec<RenderNode>,
    },
    /// A heading.
    Heading {
        /// The heading depth.
        depth: HeadingDepth,
        /// Child nodes.
        children: Vec<RenderNode>,
    },
    /// A section: a heading with its body content.
    ///
    /// Unlike `Heading`, which contains only phrasing content (the heading
    /// text), `Section` groups a heading with the block-level content that
    /// follows it. The `heading` field holds phrasing content; `children`
    /// holds block-level body content.
    Section {
        /// The section depth (maps to h1–h6).
        depth: HeadingDepth,
        /// Phrasing content for the section heading.
        heading: Vec<RenderNode>,
        /// Block-level body content.
        children: Vec<RenderNode>,
    },
    /// A paragraph.
    Paragraph {
        /// Child nodes.
        children: Vec<RenderNode>,
    },
    /// A block quote.
    BlockQuote {
        /// Child nodes.
        children: Vec<RenderNode>,
    },
    /// An ordered or unordered list.
    List {
        /// Whether the list is ordered.
        ordered: bool,
        /// The starting number for ordered lists.
        start: Option<u64>,
        /// Child list items.
        children: Vec<RenderNode>,
    },
    /// A list item.
    ListItem {
        /// Task-list checked state, if any.
        checked: Option<bool>,
        /// Child nodes.
        children: Vec<RenderNode>,
    },
    /// A fenced or indented code block.
    Code {
        /// The code language, if specified.
        lang: Option<String>,
        /// Additional fence info, if any.
        meta: Option<String>,
        /// The literal code content.
        value: String,
    },
    /// A thematic break (horizontal rule).
    ThematicBreak,
    /// A disclosure block: a summary label and a disclosed body.
    ///
    /// The `summary` contains phrasing content shown by default; `children`
    /// holds the block-level body revealed by the disclosure. There is no
    /// terminal interactivity — the body is always rendered, styled to
    /// distinguish it from the summary.
    Disclosure {
        /// Phrasing content for the always-visible summary label.
        summary: Vec<RenderNode>,
        /// Block-level body content revealed by the disclosure.
        children: Vec<RenderNode>,
        /// Inline style hints parsed from `::disclosure key=value ...`.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        style: Option<Box<crate::tree::DisclosureStyleHints>>,
    },
    /// A table.
    Table {
        /// Per-column alignment.
        align: Vec<ColumnAlign>,
        /// Child table rows.
        children: Vec<RenderNode>,
    },
    /// A table row.
    TableRow {
        /// Child table cells.
        children: Vec<RenderNode>,
    },
    /// A table cell.
    TableCell {
        /// Child nodes.
        children: Vec<RenderNode>,
    },
    /// A footnote definition.
    FootnoteDefinition {
        /// The footnote identifier.
        identifier: String,
        /// Child nodes.
        children: Vec<RenderNode>,
    },
    /// Literal text content.
    Text {
        /// The text value.
        value: String,
    },
    /// Emphasized (italic) inline content.
    Emphasis {
        /// Child nodes.
        children: Vec<RenderNode>,
    },
    /// Strong (bold) inline content.
    Strong {
        /// Child nodes.
        children: Vec<RenderNode>,
    },
    /// Struck-through inline content.
    Delete {
        /// Child nodes.
        children: Vec<RenderNode>,
    },
    /// A generic inline span.
    Span {
        /// Child nodes.
        children: Vec<RenderNode>,
    },
    /// Inline code.
    InlineCode {
        /// The code value.
        value: String,
    },
    /// A hyperlink.
    Link {
        /// The link target URL.
        url: String,
        /// An optional link title.
        title: Option<String>,
        /// Child nodes.
        children: Vec<RenderNode>,
    },
    /// An image.
    Image {
        /// The image source URL.
        url: String,
        /// An optional image title.
        title: Option<String>,
        /// Alternative text.
        alt: String,
    },
    /// A reference to a footnote definition.
    FootnoteReference {
        /// The footnote identifier.
        identifier: String,
    },
    /// A soft line break.
    SoftBreak,
    /// A hard line break.
    HardBreak,
    /// Raw HTML content.
    Html {
        /// The raw HTML value.
        value: String,
        /// Whether the HTML is block-level.
        block: bool,
    },
    /// A target-agnostic extension node identified by a `token`.
    ///
    /// Carries nested inline `children` for wrap-style extensions (such as
    /// `mark` and `dim`) and an optional scalar `payload` for atomic
    /// extensions. Renderers dispatch on `token`; a token a renderer does not
    /// recognize falls back to a neutral default that preserves `children`.
    Extended {
        /// The extension identifier (for example `"mark"` or `"dim"`).
        ///
        /// `Cow<'static, str>` keeps built-in tokens zero-allocation literals
        /// while still allowing owned dynamic tokens.
        token: Cow<'static, str>,
        /// Nested inline content for wrap-style extensions.
        children: Vec<RenderNode>,
        /// A scalar value for atomic extensions; `None` for pure wrap tokens.
        payload: Option<String>,
    },
    /// Content that has no canonical representation.
    Unsupported {
        /// A label describing the unsupported content.
        label: String,
    },
}

/// A node in the canonical owned render tree.
///
/// Every node carries its [`kind`], a [`span`] describing provenance, and
/// optional presentational [`attrs`].
///
/// [`kind`]: RenderNode::kind
/// [`span`]: RenderNode::span
/// [`attrs`]: RenderNode::attrs
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderNode {
    /// The kind of node and its payload.
    pub kind: NodeKind,
    /// Source provenance for the node.
    pub span: SourceSpan,
    /// Presentational attributes.
    pub attrs: NodeAttrs,
}

impl RenderNode {
    /// Creates a synthetic node of the given kind.
    fn synthetic(kind: NodeKind) -> Self {
        Self {
            kind,
            span: SourceSpan::synthetic(),
            attrs: NodeAttrs::default(),
        }
    }

    /// Returns the child nodes of this node.
    ///
    /// Container kinds return their `children` slice; leaf kinds return an
    /// empty slice.
    #[must_use]
    pub fn children(&self) -> &[RenderNode] {
        match &self.kind {
            NodeKind::Root { children }
            | NodeKind::Heading { children, .. }
            | NodeKind::Section { children, .. }
            | NodeKind::Paragraph { children }
            | NodeKind::BlockQuote { children }
            | NodeKind::List { children, .. }
            | NodeKind::ListItem { children, .. }
            |             NodeKind::Table { children, .. }
            | NodeKind::TableRow { children }
            | NodeKind::TableCell { children }
            | NodeKind::FootnoteDefinition { children, .. }
            | NodeKind::Disclosure { children, .. }
            | NodeKind::Emphasis { children }
            | NodeKind::Strong { children }
            | NodeKind::Delete { children }
            | NodeKind::Span { children }
            | NodeKind::Extended { children, .. }
            | NodeKind::Link { children, .. } => children,
            NodeKind::Code { .. }
            | NodeKind::ThematicBreak
            | NodeKind::Text { .. }
            | NodeKind::InlineCode { .. }
            | NodeKind::Image { .. }
            | NodeKind::FootnoteReference { .. }
            | NodeKind::SoftBreak
            | NodeKind::HardBreak
            | NodeKind::Html { .. }
            | NodeKind::Unsupported { .. } => &[],
        }
    }

    /// Returns a mutable reference to this node's children.
    ///
    /// ## Returns
    ///
    /// `Some` for container kinds, `None` for leaf kinds.
    pub fn children_mut(&mut self) -> Option<&mut Vec<RenderNode>> {
        match &mut self.kind {
            NodeKind::Root { children }
            | NodeKind::Heading { children, .. }
            | NodeKind::Section { children, .. }
            | NodeKind::Paragraph { children }
            | NodeKind::BlockQuote { children }
            | NodeKind::List { children, .. }
            | NodeKind::ListItem { children, .. }
            |             NodeKind::Table { children, .. }
            | NodeKind::TableRow { children }
            | NodeKind::TableCell { children }
            | NodeKind::FootnoteDefinition { children, .. }
            | NodeKind::Disclosure { children, .. }
            | NodeKind::Emphasis { children }
            | NodeKind::Strong { children }
            | NodeKind::Delete { children }
            | NodeKind::Span { children }
            | NodeKind::Extended { children, .. }
            | NodeKind::Link { children, .. } => Some(children),
            NodeKind::Code { .. }
            | NodeKind::ThematicBreak
            | NodeKind::Text { .. }
            | NodeKind::InlineCode { .. }
            | NodeKind::Image { .. }
            | NodeKind::FootnoteReference { .. }
            | NodeKind::SoftBreak
            | NodeKind::HardBreak
            | NodeKind::Html { .. }
            | NodeKind::Unsupported { .. } => None,
        }
    }

    /// Creates a [`NodeKind::Root`] node.
    #[must_use]
    pub fn root(children: Vec<RenderNode>) -> Self {
        Self::synthetic(NodeKind::Root { children })
    }

    /// Creates a [`NodeKind::Heading`] node.
    #[must_use]
    pub fn heading(depth: HeadingDepth, children: Vec<RenderNode>) -> Self {
        Self::synthetic(NodeKind::Heading { depth, children })
    }

    /// Creates a [`NodeKind::Section`] node.
    ///
    /// The `heading` parameter holds phrasing content for the section title;
    /// `children` holds block-level body content.
    #[must_use]
    pub fn section(
        depth: HeadingDepth,
        heading: Vec<RenderNode>,
        children: Vec<RenderNode>,
    ) -> Self {
        Self::synthetic(NodeKind::Section {
            depth,
            heading,
            children,
        })
    }

    /// Creates a [`NodeKind::Paragraph`] node.
    #[must_use]
    pub fn paragraph(children: Vec<RenderNode>) -> Self {
        Self::synthetic(NodeKind::Paragraph { children })
    }

    /// Creates a [`NodeKind::BlockQuote`] node.
    #[must_use]
    pub fn block_quote(children: Vec<RenderNode>) -> Self {
        Self::synthetic(NodeKind::BlockQuote { children })
    }

    /// Creates a [`NodeKind::List`] node.
    #[must_use]
    pub fn list(ordered: bool, start: Option<u64>, children: Vec<RenderNode>) -> Self {
        Self::synthetic(NodeKind::List {
            ordered,
            start,
            children,
        })
    }

    /// Creates a [`NodeKind::ListItem`] node.
    #[must_use]
    pub fn list_item(checked: Option<bool>, children: Vec<RenderNode>) -> Self {
        Self::synthetic(NodeKind::ListItem { checked, children })
    }

    /// Creates a [`NodeKind::Code`] node.
    #[must_use]
    pub fn code(lang: Option<String>, meta: Option<String>, value: impl Into<String>) -> Self {
        Self::synthetic(NodeKind::Code {
            lang,
            meta,
            value: value.into(),
        })
    }

    /// Creates a [`NodeKind::ThematicBreak`] node.
    #[must_use]
    pub fn thematic_break() -> Self {
        Self::synthetic(NodeKind::ThematicBreak)
    }

    /// Creates a [`NodeKind::Disclosure`] node.
    ///
    /// `summary` holds phrasing content for the always-visible label; `children`
    /// holds the disclosed block-level body. `style` carries optional inline
    /// `key=value` hints parsed from the opener line.
    #[must_use]
    pub fn disclosure(
        summary: Vec<RenderNode>,
        children: Vec<RenderNode>,
        style: Option<crate::tree::DisclosureStyleHints>,
    ) -> Self {
        Self::synthetic(NodeKind::Disclosure {
            summary,
            children,
            style: style.map(Box::new),
        })
    }

    /// Creates a [`NodeKind::Table`] node.
    #[must_use]
    pub fn table(align: Vec<ColumnAlign>, children: Vec<RenderNode>) -> Self {
        Self::synthetic(NodeKind::Table { align, children })
    }

    /// Creates a [`NodeKind::TableRow`] node.
    #[must_use]
    pub fn table_row(children: Vec<RenderNode>) -> Self {
        Self::synthetic(NodeKind::TableRow { children })
    }

    /// Creates a [`NodeKind::TableCell`] node.
    #[must_use]
    pub fn table_cell(children: Vec<RenderNode>) -> Self {
        Self::synthetic(NodeKind::TableCell { children })
    }

    /// Creates a [`NodeKind::Text`] node.
    #[must_use]
    pub fn text(value: impl Into<String>) -> Self {
        Self::synthetic(NodeKind::Text {
            value: value.into(),
        })
    }

    /// Creates a [`NodeKind::Emphasis`] node.
    #[must_use]
    pub fn emphasis(children: Vec<RenderNode>) -> Self {
        Self::synthetic(NodeKind::Emphasis { children })
    }

    /// Creates a [`NodeKind::Strong`] node.
    #[must_use]
    pub fn strong(children: Vec<RenderNode>) -> Self {
        Self::synthetic(NodeKind::Strong { children })
    }

    /// Creates a [`NodeKind::Delete`] node.
    #[must_use]
    pub fn delete(children: Vec<RenderNode>) -> Self {
        Self::synthetic(NodeKind::Delete { children })
    }

    /// Creates a [`NodeKind::Span`] node with the given classes.
    #[must_use]
    pub fn span(classes: Vec<String>, children: Vec<RenderNode>) -> Self {
        let mut node = Self::synthetic(NodeKind::Span { children });
        node.attrs.classes = classes;
        node
    }

    /// Creates a [`NodeKind::InlineCode`] node.
    #[must_use]
    pub fn inline_code(value: impl Into<String>) -> Self {
        Self::synthetic(NodeKind::InlineCode {
            value: value.into(),
        })
    }

    /// Creates a [`NodeKind::Link`] node.
    #[must_use]
    pub fn link(url: impl Into<String>, title: Option<String>, children: Vec<RenderNode>) -> Self {
        Self::synthetic(NodeKind::Link {
            url: url.into(),
            title,
            children,
        })
    }

    /// Creates a [`NodeKind::Image`] node.
    #[must_use]
    pub fn image(url: impl Into<String>, title: Option<String>, alt: impl Into<String>) -> Self {
        Self::synthetic(NodeKind::Image {
            url: url.into(),
            title,
            alt: alt.into(),
        })
    }

    /// Creates a [`NodeKind::SoftBreak`] node.
    #[must_use]
    pub fn soft_break() -> Self {
        Self::synthetic(NodeKind::SoftBreak)
    }

    /// Creates a [`NodeKind::HardBreak`] node.
    #[must_use]
    pub fn hard_break() -> Self {
        Self::synthetic(NodeKind::HardBreak)
    }

    /// Creates a [`NodeKind::FootnoteReference`] node.
    #[must_use]
    pub fn footnote_reference(identifier: impl Into<String>) -> Self {
        Self::synthetic(NodeKind::FootnoteReference {
            identifier: identifier.into(),
        })
    }

    /// Creates a [`NodeKind::FootnoteDefinition`] node.
    #[must_use]
    pub fn footnote_definition(identifier: impl Into<String>, children: Vec<RenderNode>) -> Self {
        Self::synthetic(NodeKind::FootnoteDefinition {
            identifier: identifier.into(),
            children,
        })
    }

    /// Creates a [`NodeKind::Html`] node.
    #[must_use]
    pub fn html(value: impl Into<String>, block: bool) -> Self {
        Self::synthetic(NodeKind::Html {
            value: value.into(),
            block,
        })
    }

    /// Creates a [`NodeKind::Extended`] node for the given extension `token`.
    #[must_use]
    pub fn extended(
        token: impl Into<Cow<'static, str>>,
        children: Vec<RenderNode>,
        payload: Option<String>,
    ) -> Self {
        Self::synthetic(NodeKind::Extended {
            token: token.into(),
            children,
            payload,
        })
    }

    /// Creates a [`NodeKind::Unsupported`] node.
    #[must_use]
    pub fn unsupported(label: impl Into<String>) -> Self {
        Self::synthetic(NodeKind::Unsupported {
            label: label.into(),
        })
    }
}
