use indexmap::IndexMap;
use openapiv3::{ReferenceOr, SecurityRequirement};

use super::super::OpenApiError;
use super::super::options::ExportOptions;
use super::conversion::convert_component_schema;
use super::super::extensions::AuthDetail;
use super::security::policy_security;
use crate::types::RestApi;

/// The document's `security` and the auth details of the schemes it names.
pub(super) struct RootSecurity {
    pub requirements: Vec<SecurityRequirement>,
    pub details: IndexMap<String, AuthDetail>,
}

/// Maps API components (schemas and security schemes) and the root
/// security: one requirement per method of the API's effective auth policy,
/// registered before any endpoint override's scheme.
///
/// ## Errors
///
/// A schema keyword OpenAPI 3.0.3 cannot express, named with its path.
pub(super) fn map_components(
    api: &RestApi,
    _options: &ExportOptions,
) -> Result<(openapiv3::Components, RootSecurity), OpenApiError> {
    let mut security_schemes = IndexMap::new();
    let (requirements, details) = policy_security(&api.effective_auth_policy(), &mut security_schemes);

    let schemas = api
        .schemas
        .iter()
        .map(|component| Ok((component.name.clone(), ReferenceOr::Item(convert_component_schema(&component.name, &component.schema)?))))
        .collect::<Result<_, OpenApiError>>()?;

    let components = openapiv3::Components {
        security_schemes,
        schemas,
        ..Default::default()
    };
    Ok((components, RootSecurity { requirements, details }))
}
