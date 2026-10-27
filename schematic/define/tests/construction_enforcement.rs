//! Construction and stored-definition validation share the route boundary.
use schematic_define::{DefinitionContext, Endpoint, RestApi};

#[test]
fn query_bearing_paths_fail_endpoint_and_destination_api_build() {
    for path in ["/repos/{owner}/{repo}/git/trees/{sha}?recursive=true",
        "/projects/{id}/repository/files/{file_path}?ref={git_ref}",
        "/responses/{response_id}/input_items?beta=true"] {
        let error = Endpoint::get("Get", path).describes("Get").returns_empty().build().unwrap_err();
        let text = error.to_string();
        assert!(text.contains("Get") && text.contains(path), "{text}");
        let mut endpoint = Endpoint::get("Get", "/items").describes("Get").returns_empty().build().unwrap();
        endpoint.path = path.into();
        let ctx = DefinitionContext::types_in("::models");
        let error = RestApi::builder(&ctx, "FixtureApi", "https://example.test")
            .describes("Fixture").no_auth().endpoints([endpoint]).build().unwrap_err();
        let text = error.to_string();
        assert!(text.contains("FixtureApi") && text.contains("Get") && text.contains(path), "{text}");
    }
}
