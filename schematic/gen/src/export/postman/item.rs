//! Request item builder.

use schematic_define::response::ResponseShape;
use schematic_define::{ApiRequest, Backoff, Endpoint, ResetUnit, RestApi, StatusSelector};

use crate::export::auth::{ExportAuth, endpoint_auth_override, with_variable_suffix};
use crate::export::body::map_body;
use crate::export::examples::ExampleSource;
use crate::export::path_params::strip_reserved_markers;
use crate::parser::extract_path_params;

use super::auth::build_collection_auth;
use super::request::{build_body, build_headers, build_url, rest_method_to_string};
use super::responses::saved_responses;
use super::types::*;
use super::variables::Collected;

/// One API of a collection, as its requests are built.
pub(crate) struct Member<'a> {
    pub api: &'a RestApi,
    /// The API's auth, with member-suffixed variables in a shared module.
    pub auth: ExportAuth,
    /// The collection variable holding the API's base URL.
    pub base_url_var: String,
    /// Whether each request carries the member's auth (a shared module
    /// whose members' auths differ) instead of inheriting the collection's.
    pub request_level_auth: bool,
    /// Appended to every credential variable of the member's endpoints in a
    /// shared module (`bearerToken_EmqxBearer`), so members never share one.
    pub variable_suffix: Option<String>,
    pub examples: ExampleSource,
}

/// Builds a request item from an endpoint.
///
/// When the member's `request_level_auth` is set, its auth is serialized on
/// the request; otherwise the request inherits the collection's auth. An
/// endpoint's own authentication ([`endpoint_auth_override`]) is always
/// emitted on the request and takes precedence.
///
/// When `disambiguate_with_api_name` is `true`, the request name is
/// suffixed with the owning API name (e.g. `"ListAlarms (EmqxBasic)"`)
/// so duplicate IDs across grouped APIs become distinguishable in
/// Postman's UI and exports.
///
/// A JSON body holds an example built from the request schema, and the item
/// saves one example response per success shape and declared error. An
/// example that cannot be built leaves its body empty and is reported in
/// `collected`.
pub(crate) fn build_request_item(
    endpoint: &Endpoint,
    member: &Member<'_>,
    disambiguate_with_api_name: bool,
    collected: &mut Collected,
) -> PostmanItem {
    let api = member.api;
    let own_auth = endpoint_auth_override(endpoint).map(|own| match &member.variable_suffix {
        Some(suffix) => with_variable_suffix(own, suffix),
        None => own,
    });
    if let Some(own) = &own_auth {
        collected.auth(own);
    }
    let effective_auth = own_auth.as_ref().unwrap_or(&member.auth);
    let path_params = extract_path_params(&endpoint.path);
    // Reserved-expansion markers (`{+name}`) are a runtime encoding hint; the
    // exported collection must render valid `{name}` / `:name` templates.
    let export_path = strip_reserved_markers(&endpoint.path);
    let url = build_url(&export_path, &path_params, endpoint, &member.base_url_var, effective_auth, collected);
    let export_body = endpoint.request.as_ref().map(map_body);
    let headers = build_headers(endpoint, api, effective_auth, export_body.as_ref(), collected);

    let example = match &endpoint.request {
        Some(ApiRequest::Json(reference)) => match member.examples.example(reference) {
            Ok(value) => Some(value),
            Err(error) => {
                collected.example_failure(api, &endpoint.id, "request body", &error);
                None
            }
        },
        _ => None,
    };
    let body = export_body.as_ref().map(|body| build_body(body, example.as_ref()));

    let (response, failures) = saved_responses(&member.examples, endpoint.success_shapes(), &api.effective_errors(endpoint));
    for (role, error) in failures {
        collected.example_failure(api, &endpoint.id, &role, &error);
    }

    let name = if disambiguate_with_api_name {
        format!("{} ({})", endpoint.id, api.name)
    } else {
        endpoint.id.clone()
    };

    let auth = match &own_auth {
        Some(own) => build_collection_auth(own),
        None if member.request_level_auth => build_collection_auth(&member.auth),
        None => None,
    };

    PostmanItem::Request {
        name,
        request: Box::new(PostmanRequest {
            method: rest_method_to_string(&endpoint.method),
            url,
            auth,
            header: headers,
            body,
            description: request_description(endpoint, api),
        }),
        response,
    }
}

/// The first success shape's media type when it says more than the shape
/// kind: a declared content type or a stream's. A plain JSON, text, binary,
/// or empty shape records nothing.
fn first_success_content_type(endpoint: &Endpoint) -> Option<String> {
    let first = endpoint.success_shapes().first()?;
    let telling = first.content_type.is_some() || matches!(first.shape, ResponseShape::Stream { .. });
    if telling { first.media_type() } else { None }
}

/// The endpoint description followed by one line per piece of definition
/// vocabulary Postman has no field for: the first success shape's content
/// type, endpoint and parameter deprecation, and the effective
/// rate-limit and retry metadata. `None` when all are absent, so an endpoint
/// declaring none of them keeps its description alone.
fn request_description(endpoint: &Endpoint, api: &RestApi) -> Option<String> {
    let mut notes: Vec<String> = Vec::new();
    if let Some(media) = first_success_content_type(endpoint) {
        notes.push(format!("Response content type: `{media}`."));
    }
    if endpoint.deprecated {
        notes.push("Deprecated.".to_string());
    }
    if let Some(params) = &endpoint.params {
        let deprecated: Vec<&str> = [&params.path, &params.query, &params.header, &params.cookie]
            .into_iter()
            .flatten()
            .filter(|param| param.deprecated)
            .map(|param| param.name.as_str())
            .collect();
        if !deprecated.is_empty() {
            notes.push(format!("Deprecated parameters: {}.", deprecated.join(", ")));
        }
    }
    if let Some(rate_limit) = api.effective_rate_limit(endpoint) {
        let unit = match rate_limit.reset_unit {
            ResetUnit::Seconds => "seconds",
            ResetUnit::EpochSeconds => "epoch seconds",
            ResetUnit::HttpDate => "HTTP date",
        };
        let mut line = format!(
            "Rate limit headers: limit `{}`, remaining `{}`, reset `{}` ({unit})",
            rate_limit.limit_header, rate_limit.remaining_header, rate_limit.reset_header
        );
        if let Some(retry_after) = &rate_limit.retry_after_header {
            line.push_str(&format!(", retry after `{retry_after}`"));
        }
        line.push('.');
        notes.push(line);
    }
    if let Some(retry) = api.effective_retry_guidance(endpoint) {
        let statuses: Vec<String> = retry
            .retry_on
            .iter()
            .map(|selector| match selector {
                StatusSelector::Code(code) => code.to_string(),
                StatusSelector::Range { start, end } => format!("{start}-{end}"),
            })
            .collect();
        let statuses = if statuses.is_empty() { "none".to_string() } else { statuses.join(", ") };
        let backoff = match retry.backoff {
            Backoff::None => "none".to_string(),
            Backoff::Fixed { delay_ms } => format!("fixed {delay_ms} ms"),
            Backoff::Exponential { base_ms, cap_ms } => format!("exponential from {base_ms} ms up to {cap_ms} ms"),
        };
        notes.push(format!(
            "Retry guidance: retry on {statuses}; honor Retry-After: {}; max attempts {} (including the first); backoff {backoff}.",
            if retry.honor_retry_after { "yes" } else { "no" },
            retry.max_attempts,
        ));
    }

    let description = (!endpoint.description.is_empty()).then(|| endpoint.description.clone());
    match (description, notes.is_empty()) {
        (description, true) => description,
        (Some(description), false) => Some(format!("{description}\n\n{}", notes.join("\n"))),
        (None, false) => Some(notes.join("\n")),
    }
}
