//! Presentational attributes attached to a [`RenderNode`].
//!
//! [`RenderNode`]: crate::tree::RenderNode

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Optional presentational attributes carried by every render node.
///
/// All fields are optional; the [`Default`] value is an empty set of
/// attributes (no id, no classes, no data).
///
/// ## Examples
///
/// ```
/// use renderable::tree::NodeAttrs;
///
/// let attrs = NodeAttrs::default();
/// assert!(attrs.id.is_none());
/// assert!(attrs.classes.is_empty());
/// assert!(attrs.data.is_empty());
/// ```
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeAttrs {
    /// Optional unique identifier for the node.
    pub id: Option<String>,
    /// CSS-style class names associated with the node.
    pub classes: Vec<String>,
    /// Arbitrary structured data keyed by name.
    pub data: BTreeMap<String, serde_json::Value>,
}
