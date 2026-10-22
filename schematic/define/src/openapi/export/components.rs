use indexmap::IndexMap;
use openapiv3::ReferenceOr;

use super::super::options::ExportOptions;
use super::conversion::convert_component_schema;
use super::security::map_security;
use crate::types::RestApi;

/// Maps API components (schemas and security schemes).
pub(super) fn map_components(
    api: &RestApi,
    _options: &ExportOptions,
) -> openapiv3::Components {
    let mut security_schemes = IndexMap::new();

    if let Some((name, scheme)) = map_security(&api.auth) {
        security_schemes.insert(name, ReferenceOr::Item(scheme));
    }

    let schemas = api
        .schemas
        .iter()
        .map(|component| (component.name.clone(), ReferenceOr::Item(convert_component_schema(&component.schema))))
        .collect();

    openapiv3::Components {
        security_schemes,
        schemas,
        ..Default::default()
    }
}
