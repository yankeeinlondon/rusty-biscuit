//! Components for fixture APIs whose endpoints reference schemas by name.

use schematic_define::{RestApi, SchemaComponent};

/// Gives `api` a component for every name its endpoints reference that it
/// does not carry yet. Fixture names are not real Rust types, so each is
/// spelled by its bare name and described as an unconstrained object.
pub fn with_referenced_components(mut api: RestApi) -> RestApi {
    let mut names: Vec<String> = Vec::new();
    for endpoint in &api.endpoints {
        for reference in endpoint.json_references() {
            names.extend(reference.names().into_iter().map(str::to_owned));
        }
    }
    for name in names {
        if api.component(&name).is_none() {
            api.schemas.push(SchemaComponent {
                rust_path: Some(name.clone()),
                name,
                schema: serde_json::json!({"type": "object"}),
            });
        }
    }
    api
}

/// A fixture API carrying `endpoint` and a component for every name it
/// references, for generating that endpoint's request struct.
pub fn api_for(endpoint: &schematic_define::Endpoint) -> RestApi {
    let mut api = RestApi::builder(
        &schematic_define::DefinitionContext::types_in("::fixture"),
        "Fixture",
        "https://example.test",
    )
    .describes("Fixture")
    .no_auth()
    .build()
    .expect("valid fixture API");
    api.endpoints.push(endpoint.clone());
    with_referenced_components(api)
}
