use indexmap::IndexMap;
use openapiv3::ReferenceOr;

use super::super::options::ExportOptions;
use super::security::map_security;
use crate::types::RestApi;

/// Trait for schema registry abstraction.
///
/// This allows the export function to work with different registry implementations.
pub trait SchemaRegistryLike {
    /// Returns OpenAPI schemas indexed by name.
    fn to_openapi_schemas(&self) -> IndexMap<String, openapiv3::Schema>;
}

/// Maps API components (schemas and security schemes).
pub(super) fn map_components<R: SchemaRegistryLike>(
    api: &RestApi,
    registry: &R,
    _options: &ExportOptions,
) -> openapiv3::Components {
    let mut security_schemes = IndexMap::new();

    if let Some((name, scheme)) = map_security(&api.auth) {
        security_schemes.insert(name, ReferenceOr::Item(scheme));
    }

    let schemas = registry
        .to_openapi_schemas()
        .into_iter()
        .map(|(name, schema)| (name, ReferenceOr::Item(schema)))
        .collect();

    openapiv3::Components {
        security_schemes,
        schemas,
        ..Default::default()
    }
}
