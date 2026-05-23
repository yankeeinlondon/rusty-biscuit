//! The top-level render document and its metadata.

use serde::{Deserialize, Serialize};

use crate::tree::node::RenderNode;
use crate::tree::source::SourceRegistry;

/// The serialization format of a document's frontmatter block.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FrontmatterFormat {
    /// YAML frontmatter.
    Yaml,
    /// TOML frontmatter.
    Toml,
    /// JSON frontmatter.
    Json,
}

/// A document's raw frontmatter block.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Frontmatter {
    /// The format the frontmatter was written in.
    pub format: FrontmatterFormat,
    /// The raw, unparsed frontmatter content.
    pub raw: String,
}

/// Metadata describing a [`Document`].
///
/// The [`Default`] value carries no frontmatter.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentMetadata {
    /// The document's frontmatter, if any.
    pub frontmatter: Option<Frontmatter>,
}

/// A complete render document: source registry, metadata, and a root node.
///
/// The [`root`] node is expected to be a [`NodeKind::Root`].
///
/// [`root`]: Document::root
/// [`NodeKind::Root`]: crate::tree::NodeKind::Root
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Document {
    /// The registry of source origins referenced by the tree.
    pub sources: SourceRegistry,
    /// Document-level metadata.
    pub metadata: DocumentMetadata,
    /// The root node of the render tree.
    pub root: RenderNode,
}
