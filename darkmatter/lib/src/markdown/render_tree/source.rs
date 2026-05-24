//! Source-registry construction for the render-tree fold.
//!
//! The fold consumes a single Markdown input drawn from one origin. This
//! module builds the [`SourceRegistry`] that holds that origin and hands back
//! the [`SourceId`] every parsed node's [`SourceLocation`] refers to.
//!
//! [`SourceLocation`]: renderable::tree::SourceLocation

use renderable::tree::{SourceDescriptor, SourceId, SourceRegistry};

/// Builds a single-source registry for a fold input.
///
/// The returned [`SourceId`] is the handle the fold attaches to every parsed
/// node's [`SourceLocation`].
///
/// ## Examples
///
/// ```
/// use darkmatter::markdown::render_tree::source::single_source_registry;
/// use renderable::tree::SourceDescriptor;
///
/// let (registry, id) =
///     single_source_registry(SourceDescriptor::Virtual { name: "doc".into() });
/// assert!(registry.resolve(id).is_some());
/// ```
///
/// [`SourceLocation`]: renderable::tree::SourceLocation
#[must_use]
pub fn single_source_registry(source: SourceDescriptor) -> (SourceRegistry, SourceId) {
    let mut registry = SourceRegistry::new();
    let id = registry.register(source);
    (registry, id)
}
