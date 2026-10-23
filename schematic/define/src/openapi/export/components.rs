use indexmap::IndexMap;
use openapiv3::ReferenceOr;

use super::super::OpenApiError;
use super::super::options::ExportOptions;
use super::conversion::convert_component_schema;
use super::security::map_security;
use crate::types::RestApi;

/// Maps API components (schemas and security schemes).
///
/// ## Errors
///
/// A schema keyword OpenAPI 3.0.3 cannot express, named with its path.
pub(super) fn map_components(
    api: &RestApi,
    _options: &ExportOptions,
) -> Result<openapiv3::Components, OpenApiError> {
    let mut security_schemes = IndexMap::new();

    if let Some((name, scheme)) = map_security(&api.auth) {
        security_schemes.insert(name, ReferenceOr::Item(scheme));
    }

    let schemas = api
        .schemas
        .iter()
        .map(|component| Ok((component.name.clone(), ReferenceOr::Item(convert_component_schema(&component.name, &component.schema)?))))
        .collect::<Result<_, OpenApiError>>()?;

    Ok(openapiv3::Components {
        security_schemes,
        schemas,
        ..Default::default()
    })
}
