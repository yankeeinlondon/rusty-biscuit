//! The `Endpoint` and `EndpointParams` literals as the definitions spell them
//! on 2026-10-13. Pagination added no field to either; only `RestApi` gained
//! `default_pagination` and `pagination_exceptions`.
//!
//! Frozen to that surface: the definition-API work that moves exception rows
//! onto endpoints replaces or retires this test.

use schematic_define::pagination::{Resolution, resolve};
use schematic_define::{
    ApiResponse, AuthStrategy, CrossOrigin, Endpoint, EndpointParams, ParamDef, ParamStyle,
    QueryParamType, RestApi, RestMethod,
};

fn endpoint_literal() -> Endpoint {
    Endpoint {
        id: "ListReleases".to_string(),
        method: RestMethod::Get,
        path: "/repos/{owner}/{repo}/releases".to_string(),
        description: "List releases".to_string(),
        request: None,
        response: ApiResponse::json_type("Vec<Release>"),
        headers: vec![],
        params: Some(EndpointParams {
            query: vec![ParamDef {
                name: "per_page".to_string(),
                required: false,
                description: None,
                param_type: QueryParamType::Integer,
                explode: true,
                style: ParamStyle::Form,
            }],
            header: vec![],
            cookie: vec![],
            pagination: None,
            response_pagination: None,
        }),
        oauth_scopes: None,
    }
}

#[test]
fn endpoint_and_params_literals_compile_unchanged_and_behave_the_same() {
    let endpoint = endpoint_literal();
    let params = endpoint.params.as_ref().unwrap();
    assert_eq!(params.query[0].name, "per_page");
    assert!(params.pagination.is_none() && params.response_pagination.is_none());
    assert_eq!(
        EndpointParams {
            query: vec![],
            header: vec![],
            cookie: vec![],
            pagination: None,
            response_pagination: None,
        },
        EndpointParams::default()
    );

    // An API without authored pagination resolves the literal as before:
    // an unpaginated list that exposes nothing.
    let api = RestApi {
        name: "GitHub".to_string(),
        description: "GitHub".to_string(),
        base_url: "https://api.github.com".to_string(),
        docs_url: None,
        auth: AuthStrategy::None,
        auth_policy: None,
        env_auth: vec![],
        env_username: None,
        headers: vec![],
        endpoints: vec![endpoint.clone()],
        module_path: None,
        request_suffix: None,
        version: None,
        env_mapping: None,
        cross_origin: CrossOrigin::default(),
        request_id_header: None,
        secret_names: vec![],
        default_pagination: None,
        pagination_exceptions: vec![],
    };
    assert!(matches!(resolve(&api, &endpoint), Resolution::Unpaginated { .. }));
}
