//! Darkmatter's layering of authored frontmatter shapes: its runtime
//! baseline under each extension, trigger, and document shape. The lookups
//! themselves are the library's ([`simplified_schema::frontmatter_shape`]).

use std::sync::Arc;

use serde_json::Value;

use super::{SchemaRoot, SchemaShape, darkmatter_base_schema_ref};

/// The effective top-level property shape for one document: the Darkmatter
/// base properties, overlaid with each extension baseline shape in order, then
/// the document's own `$schema` (document > extension > base, the compose
/// precedence).
///
/// A root `$schema` union overlays the arm its shared literal discriminant
/// selects in `frontmatter`; when no arm is unambiguously selected, every
/// inline arm's properties merge, and a key contributed by more than one arm
/// resolves to the union of those arms' atoms. A file-reference arm
/// contributes no properties.
///
/// `frontmatter` is the authored frontmatter mapping as JSON; pass
/// [`Value::Null`] when none is available, which disables discriminant
/// narrowing.
///
/// No trigger layer is applied; for a resolved document, use
/// [`EffectiveSchema::property_shape`](super::EffectiveSchema::property_shape).
pub fn effective_property_shape(
    extension_shapes: &[SchemaShape],
    document: Option<&SchemaRoot>,
    frontmatter: &Value,
) -> SchemaShape {
    layered_property_shape(extension_shapes, &[], document, frontmatter)
}

/// [`effective_property_shape`] with the matching trigger payload shapes
/// layered between the extensions and the document.
pub(super) fn layered_property_shape(
    extension_shapes: &[SchemaShape],
    trigger_shapes: &[Arc<SchemaShape>],
    document: Option<&SchemaRoot>,
    frontmatter: &Value,
) -> SchemaShape {
    // The definition-free baseline: this shape is read with the document's
    // and payloads' tables (`EffectiveSchema::layered_definitions`), which
    // hold none of the baseline's stored types.
    let base = match darkmatter_base_schema_ref() {
        SchemaRoot::Single(shape) => shape.clone(),
        SchemaRoot::Union(_) => SchemaShape::default(),
    };
    simplified_schema::layered_property_shape(base, extension_shapes, trigger_shapes, document, frontmatter)
}
