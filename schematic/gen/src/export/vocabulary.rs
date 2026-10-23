//! Export diagnostics for definition vocabulary an export cannot express.
//!
//! Every finding is a [`Severity::Warning`]: the output is still written,
//! with the loss named here rather than made silently.

use schematic_define::{AuthMethod, AuthStrategy, EndpointAuth, OAuth2Config, RestApi};

use crate::diagnostics::{Feature, Output, Severity, ValidationDiagnostic};
use crate::export::auth::{ExportAuth, policy_alternatives};

/// The diagnostics `output` reports for vocabulary of `api` it cannot express.
///
/// - OpenAPI: an OAuth2 device-code grant (OpenAPI 3.0 has no device-code
///   flow), on the API or as an endpoint's replacement method; and each
///   header the operation does not carry as a header parameter
///   ([`schematic_define::openapi::header_gaps`]).
///   OpenAPI also skips each GraphQL operation, which is outside its round-trip
///   subset; Postman exports them as requests.
/// - Postman: an OAuth2 device-code grant (Postman has no device-code
///   grant), and a replacement policy with more than one alternative (a
///   request holds one auth).
pub(crate) fn diagnostics(api: &RestApi, output: Output) -> Vec<ValidationDiagnostic> {
    match output {
        Output::OpenApi => {
            let mut diagnostics = openapi_diagnostics(api);
            diagnostics.extend(graphql_skips(api));
            diagnostics
        }
        Output::Postman => postman_diagnostics(api),
        _ => Vec::new(),
    }
}

fn graphql_skips(api: &RestApi) -> Vec<ValidationDiagnostic> {
    let operations = api.graphql.iter().flat_map(|surface| &surface.operations);
    operations
        .map(|operation| warning(
            api,
            Some(&operation.id),
            Feature::GraphQlOperation,
            Output::OpenApi,
            format!("{} operation {}: GraphQL operation skipped in OpenAPI output", api.name, operation.id),
        ))
        .collect()
}

fn warning(api: &RestApi, endpoint: Option<&str>, feature: Feature, output: Output, message: String) -> ValidationDiagnostic {
    ValidationDiagnostic::new(Severity::Warning, &api.name, endpoint, feature, output, message)
}

fn is_device_code(method: &AuthMethod) -> bool {
    matches!(method, AuthMethod::OAuth2(OAuth2Config::DeviceCode(_)))
}

fn openapi_diagnostics(api: &RestApi) -> Vec<ValidationDiagnostic> {
    let mut diagnostics = Vec::new();
    if matches!(&api.auth, AuthStrategy::OAuth2(OAuth2Config::DeviceCode(_))) {
        diagnostics.push(warning(
            api,
            None,
            Feature::OAuth2Grant,
            Output::OpenApi,
            "OpenAPI 3.0 has no OAuth2 device-code flow; the document declares no security \
             scheme or requirement for the API's authentication"
                .to_string(),
        ));
    }
    for endpoint in &api.endpoints {
        let Some(EndpointAuth::Policy { policy, .. }) = &endpoint.auth else {
            continue;
        };
        if !policy.explicit.iter().any(is_device_code) {
            continue;
        }
        let expressible = policy.explicit.iter().any(|method| !is_device_code(method)) || policy.env_fallback.is_some();
        let consequence = if expressible {
            "the operation lists only its other accepted methods"
        } else {
            "the operation has no security of its own and shows the document's requirements"
        };
        diagnostics.push(warning(
            api,
            Some(&endpoint.id),
            Feature::OAuth2Grant,
            Output::OpenApi,
            format!(
                "OpenAPI 3.0 has no OAuth2 device-code flow; the endpoint's device-code \
                 method is omitted and {consequence}"
            ),
        ));
    }
    // An API that does not export reports its own error when the output is
    // built; it has no header gaps to add here.
    if let Ok(gaps) = schematic_define::openapi::header_gaps(api) {
        diagnostics.extend(gaps.iter().map(|gap| {
            warning(api, Some(&gap.endpoint_id), Feature::FixedHeader, Output::OpenApi, gap.to_string())
        }));
    }
    diagnostics
}

fn postman_diagnostics(api: &RestApi) -> Vec<ValidationDiagnostic> {
    let mut diagnostics = Vec::new();
    let api_device_code = matches!(&api.auth, AuthStrategy::OAuth2(OAuth2Config::DeviceCode(_)));
    if api_device_code {
        diagnostics.push(warning(api, None, Feature::OAuth2Grant, Output::Postman, DEVICE_CODE_IN_POSTMAN.to_string()));
    }
    for endpoint in &api.endpoints {
        if let Some(EndpointAuth::Policy { policy, .. }) = &endpoint.auth {
            let alternatives = policy_alternatives(policy);
            if alternatives.len() > 1 {
                diagnostics.push(warning(
                    api,
                    Some(&endpoint.id),
                    Feature::EndpointAuth,
                    Output::Postman,
                    format!(
                        "the endpoint accepts {} authentication alternatives, but a Postman \
                         request holds one auth; only the first is exported",
                        alternatives.len()
                    ),
                ));
            }
            if matches!(alternatives.first(), Some(ExportAuth::OAuth2 { config: OAuth2Config::DeviceCode(_), .. })) {
                diagnostics.push(warning(api, Some(&endpoint.id), Feature::OAuth2Grant, Output::Postman, DEVICE_CODE_IN_POSTMAN.to_string()));
            }
        }
    }
    diagnostics
}

const DEVICE_CODE_IN_POSTMAN: &str = "Postman has no OAuth2 device-code grant; the auth carries the token URL \
     and scopes, and an access token obtained outside Postman goes in its access-token variable";
