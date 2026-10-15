//! Required-fact endpoint authoring retains empty parameters and no pagination.

use schematic_define::pagination::{Resolution, resolve};
use schematic_define::{
    DefinitionContext, Endpoint, EndpointParams, ParamDef, ParamStyle,
    QueryParamType, RestApi, definition_type,
};

definition_type!(#[derive(schemars::JsonSchema)] pub struct Release { pub id: u64 });

fn endpoint_literal(ctx: &mut DefinitionContext) -> Endpoint {
    Endpoint::get("ListReleases", "/repos/{owner}/{repo}/releases")
        .describes("List releases")
        .returns_json::<Vec<Release>>(ctx)
        .params(EndpointParams {
            query: vec![ParamDef { rust_name: None, secret: false, pager_managed: false,
                name: "per_page".to_string(), required: false, description: None,
                param_type: QueryParamType::Integer, explode: true, style: ParamStyle::Form,
            }],
            ..EndpointParams::default()
        })
        .build().unwrap()
}

#[test]
fn endpoint_builder_and_parameter_default_preserve_unpaginated_behavior() {
    let mut ctx = DefinitionContext::types_in("::models");
    let endpoint = endpoint_literal(&mut ctx);
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

    let api = RestApi::builder(&ctx, "GitHub", "https://api.github.com")
        .describes("GitHub").no_auth().endpoints([endpoint.clone()]).build().unwrap();
    assert!(matches!(resolve(&api, &endpoint), Resolution::Unpaginated { .. }));
}
