//! Fixture builders shared by the pagination test binaries.
#![allow(dead_code)]

use schematic_define::pagination::{
    Collection, PaginationError, Resolution, Strategy, resolve,
    validate_pagination,
};
use schematic_define::{
    ApiRequest, ApiResponse, AuthStrategy, CrossOrigin, Endpoint, EndpointParams, ParamDef,
    QueryParamType, RestApi, RestMethod,
};

/// An API with no pagination and no endpoints.
pub fn api(name: &str) -> RestApi {
    { let mut fixture = RestApi::builder(&schematic_define::DefinitionContext::types_in("::fixture"), "Fixture", "https://example.test").describes("Fixture").no_auth().build().unwrap(); fixture.name = name.to_string(); fixture.description = format!("{name} fixture"); fixture.base_url = "https://api.example.test".to_string(); fixture.docs_url = None; fixture.auth = AuthStrategy::None; fixture.auth_policy = None; fixture.env_auth = vec![]; fixture.env_username = None; fixture.headers = vec![]; fixture.endpoints = vec![]; fixture.module_path = None; fixture.request_suffix = None; fixture.version = None; fixture.env_mapping = None; fixture.cross_origin = CrossOrigin::default(); fixture.request_id_header = None; fixture.secret_names = vec![]; fixture.default_pagination = None; fixture }
}

pub fn endpoint(id: &str, method: RestMethod, path: &str, response: &str) -> Endpoint {
    { let mut fixture = Endpoint::get("Fixture", "/").describes("Fixture").returns_empty().build().unwrap(); fixture.id = id.to_string(); fixture.method = method; fixture.path = path.to_string(); fixture.description = String::new(); fixture.request = (method != RestMethod::Get).then(|| ApiRequest::json_type("Body")); fixture.response = ApiResponse::json_type(response); fixture.headers = vec![]; fixture.params = None; fixture.cross_origin = None; fixture.pagination = None; fixture }
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

/// An API holding an endpoint with its explicit collection.
pub fn listed(mut endpoint: Endpoint, collection: Collection) -> RestApi {
    let mut api = api("Fixture");
    endpoint.pagination = Some(collection);
    api.endpoints = vec![endpoint];
    api
}

/// The errors of every diagnostic, in order.
pub fn errors(api: &RestApi) -> Vec<PaginationError> {
    validate_pagination(api).into_iter().map(|d| d.error).collect()
}

/// The errors of `endpoint` paged by `strategy` through its explicit collection.
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
