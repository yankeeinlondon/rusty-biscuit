//! Request item builder.

use schematic_define::{Endpoint, RestApi};

use crate::export::auth::ExportAuth;
use crate::export::body::map_body;
use crate::export::path_params::strip_reserved_markers;
use crate::parser::extract_path_params;

use super::auth::build_collection_auth;
use super::request::{build_headers, build_url, rest_method_to_string};
use super::types::*;

/// Builds a request item from an endpoint.
///
/// `owning_auth` is the auth that should govern this specific request.
/// When `emit_request_level_auth` is `true`, that auth is serialized
/// directly on the request (used in mixed-auth grouped collections so
/// each request advertises its own auth even though the collection has
/// none). When `false`, the request inherits collection-level auth and
/// emits no `auth` block.
///
/// When `disambiguate_with_api_name` is `true`, the request name is
/// suffixed with the owning API name (e.g. `"ListAlarms (EmqxBasic)"`)
/// so duplicate IDs across grouped APIs become distinguishable in
/// Postman's UI and exports.
pub(crate) fn build_request_item(
    endpoint: &Endpoint,
    api: &RestApi,
    owning_auth: &ExportAuth,
    emit_request_level_auth: bool,
    disambiguate_with_api_name: bool,
    base_url_var: &str,
) -> PostmanItem {
    let path_params = extract_path_params(&endpoint.path);
    // Reserved-expansion markers (`{+name}`) are a runtime encoding hint; the
    // exported collection must render valid `{name}` / `:name` templates.
    let export_path = strip_reserved_markers(&endpoint.path);
    let url = build_url(
        &api.base_url,
        &export_path,
        &path_params,
        endpoint,
        base_url_var,
    );
    let headers = build_headers(endpoint, api);
    let body = endpoint
        .request
        .as_ref()
        .map(|req| super::request::build_body(&map_body(req)));

    let name = if disambiguate_with_api_name {
        format!("{} ({})", endpoint.id, api.name)
    } else {
        endpoint.id.clone()
    };

    let auth = if emit_request_level_auth {
        build_collection_auth(owning_auth)
    } else {
        None
    };

    PostmanItem::Request {
        name,
        request: Box::new(PostmanRequest {
            method: rest_method_to_string(&endpoint.method),
            url,
            auth,
            header: headers,
            body,
            description: if endpoint.description.is_empty() {
                None
            } else {
                Some(endpoint.description.clone())
            },
        }),
    }
}
