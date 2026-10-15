//! Fixture builders shared by the pagination test binaries.
#![allow(dead_code)]

use schematic_define::pagination::{
    Collection, PaginationError, PaginationException, Resolution, Strategy, resolve,
    validate_pagination,
};
use schematic_define::{
    ApiRequest, ApiResponse, AuthStrategy, CrossOrigin, Endpoint, EndpointParams, ParamDef,
    QueryParamType, RestApi, RestMethod,
};

/// An API with no pagination and no endpoints.
pub fn api(name: &str) -> RestApi {
    RestApi {
        name: name.to_string(),
        description: format!("{name} fixture"),
        base_url: "https://api.example.test".to_string(),
        docs_url: None,
        auth: AuthStrategy::None,
        auth_policy: None,
        env_auth: vec![],
        env_username: None,
        headers: vec![],
        endpoints: vec![],
        module_path: None,
        request_suffix: None,
        version: None,
        env_mapping: None,
        cross_origin: CrossOrigin::default(),
        request_id_header: None,
        secret_names: vec![],
        default_pagination: None,
        pagination_exceptions: vec![],
    }
}

pub fn endpoint(id: &str, method: RestMethod, path: &str, response: &str) -> Endpoint {
    Endpoint {
        id: id.to_string(),
        method,
        path: path.to_string(),
        description: String::new(),
        request: (method != RestMethod::Get).then(|| ApiRequest::json_type("Body")),
        response: ApiResponse::json_type(response),
        headers: vec![],
        params: None,
        oauth_scopes: None, cross_origin: None,
    }
}

pub fn get(id: &str, path: &str, response: &str) -> Endpoint {
    endpoint(id, RestMethod::Get, path, response)
}

pub fn post(id: &str, path: &str, response: &str) -> Endpoint {
    endpoint(id, RestMethod::Post, path, response)
}

fn param(name: &str, required: bool) -> ParamDef {
    let params = EndpointParams::default().with_query_param(name, QueryParamType::String, required, None);
    params.query.into_iter().next().unwrap()
}

/// Declares optional query parameters.
pub fn with_query(mut endpoint: Endpoint, names: &[&str]) -> Endpoint {
    let params = endpoint.params.get_or_insert_with(EndpointParams::default);
    params.query.extend(names.iter().map(|name| param(name, false)));
    endpoint
}

/// Declares one required query parameter.
pub fn with_required_query(mut endpoint: Endpoint, name: &str) -> Endpoint {
    let params = endpoint.params.get_or_insert_with(EndpointParams::default);
    params.query.push(param(name, true));
    endpoint
}

/// Declares optional header parameters.
pub fn with_headers(mut endpoint: Endpoint, names: &[&str]) -> Endpoint {
    let params = endpoint.params.get_or_insert_with(EndpointParams::default);
    params.header.extend(names.iter().map(|name| param(name, false)));
    endpoint
}

pub fn row(endpoint: &str, collection: Collection) -> PaginationException {
    PaginationException::new(endpoint, collection)
}

/// An API holding `endpoint`, with `collection` as its only exceptions row.
pub fn listed(endpoint: Endpoint, collection: Collection) -> RestApi {
    let mut api = api("Fixture");
    api.pagination_exceptions = vec![row(&endpoint.id, collection)];
    api.endpoints = vec![endpoint];
    api
}

/// The errors of every diagnostic, in order.
pub fn errors(api: &RestApi) -> Vec<PaginationError> {
    validate_pagination(api).into_iter().map(|d| d.error).collect()
}

/// The errors of `endpoint` paged by `strategy` through an exceptions row.
pub fn endpoint_errors(endpoint: Endpoint, items: &str, strategy: impl Into<Strategy>) -> Vec<PaginationError> {
    let collection = if items.is_empty() {
        Collection::root()
    } else {
        Collection::at(items).item_type("Item")
    };
    errors(&listed(endpoint, collection.paginate(strategy)))
}

pub fn resolution(api: &RestApi, id: &str) -> Resolution {
    let endpoint = api
        .endpoints
        .iter()
        .find(|endpoint| endpoint.id == id)
        .unwrap_or_else(|| panic!("{id} is not an endpoint of {}", api.name));
    resolve(api, endpoint)
}
