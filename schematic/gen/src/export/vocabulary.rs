//! Export diagnostics for definition vocabulary an export cannot express.
//!
//! Every finding is a [`Severity::Warning`]: the output is still written,
//! with the loss named here rather than made silently.

use schematic_define::{
    ApiRequest, ApiResponse, AuthMethod, AuthStrategy, EndpointAuth, OAuth2Config, RestApi,
};

use crate::diagnostics::{Feature, Output, Severity, ValidationDiagnostic};
use crate::export::auth::policy_alternatives;

/// The diagnostics `output` reports for vocabulary of `api` it cannot express.
///
/// - OpenAPI: an OAuth2 device-code grant (OpenAPI 3.0 has no device-code
///   flow), on the API or as an endpoint's replacement method.
/// - Postman: a replacement policy with more than one alternative (a request
///   holds one auth), and a JSON body or response naming a composed
///   (`oneOf`/`anyOf`/`allOf`) component.
/// - Both: each GraphQL operation, which neither output carries (R42).
pub(crate) fn diagnostics(api: &RestApi, output: Output) -> Vec<ValidationDiagnostic> {
    let mut diagnostics = match output {
        Output::OpenApi => openapi_diagnostics(api),
        Output::Postman => postman_diagnostics(api),
        _ => return Vec::new(),
    };
    diagnostics.extend(graphql_skips(api, output));
    diagnostics
}

fn graphql_skips(api: &RestApi, output: Output) -> Vec<ValidationDiagnostic> {
    let target = match output {
        Output::Postman => "Postman",
        _ => "OpenAPI",
    };
    let operations = api.graphql.iter().flat_map(|surface| &surface.operations);
    operations
        .map(|operation| warning(
            api,
            Some(&operation.id),
            Feature::GraphQlOperation,
            output,
            format!("{} operation {}: GraphQL operation skipped in {target} output", api.name, operation.id),
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
    diagnostics
}

fn postman_diagnostics(api: &RestApi) -> Vec<ValidationDiagnostic> {
    let mut diagnostics = Vec::new();
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
        }

        let bodies = [
            ("request body", match &endpoint.request {
                Some(ApiRequest::Json(type_ref)) => Some(type_ref),
                _ => None,
            }),
            ("response", match &endpoint.response {
                ApiResponse::Json(type_ref) => Some(type_ref),
                _ => None,
            }),
        ];
        for (role, type_ref) in bodies {
            let Some(type_ref) = type_ref else { continue };
            let mut seen: Vec<&str> = Vec::new();
            for name in type_ref.names() {
                if seen.contains(&name) {
                    continue;
                }
                seen.push(name);
                let Some(keyword) = api
                    .schemas
                    .iter()
                    .find(|component| component.name == name)
                    .and_then(|component| composition_keyword(&component.schema))
                else {
                    continue;
                };
                diagnostics.push(warning(
                    api,
                    Some(&endpoint.id),
                    Feature::SchemaComposition,
                    Output::Postman,
                    format!(
                        "the {role} uses component \"{name}\", a {keyword} composition; a \
                         Postman collection cannot declare schema composition, so the request \
                         keeps its existing example and no example payload is invented"
                    ),
                ));
            }
        }
    }
    diagnostics
}

/// The keyword of a composed component's stored schema: exactly one
/// `oneOf`/`anyOf`/`allOf` member listing `$ref`s, the form
/// `DefinitionContext::one_of`/`any_of`/`all_of` registers. Compositions
/// schemars emits inside ordinary types are not composed components.
fn composition_keyword(schema: &serde_json::Value) -> Option<&'static str> {
    let object = schema.as_object().filter(|object| object.len() == 1)?;
    let keyword = ["oneOf", "anyOf", "allOf"].into_iter().find(|keyword| object.contains_key(*keyword))?;
    let members = object[keyword].as_array().filter(|members| !members.is_empty())?;
    members
        .iter()
        .all(|member| member.as_object().is_some_and(|member| member.len() == 1 && member.get("$ref").is_some_and(serde_json::Value::is_string)))
        .then_some(keyword)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn only_pure_reference_compositions_are_composed_components() {
        assert_eq!(composition_keyword(&json!({"oneOf": [{"$ref": "#/components/schemas/A"}]})), Some("oneOf"));
        assert_eq!(composition_keyword(&json!({"allOf": [{"$ref": "#/components/schemas/A"}, {"$ref": "#/components/schemas/B"}]})), Some("allOf"));
        assert_eq!(composition_keyword(&json!({"anyOf": [{"$ref": "#/components/schemas/A"}, {"type": "null"}]})), None);
        assert_eq!(composition_keyword(&json!({"oneOf": [{"$ref": "#/components/schemas/A"}], "title": "A"})), None);
        assert_eq!(composition_keyword(&json!({"oneOf": []})), None);
        assert_eq!(composition_keyword(&json!({"type": "object"})), None);
    }
}
