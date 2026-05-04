//! Regression tests for review-2 Finding 4: strict registry completeness in
//! the OpenAPI grouped export path.
//!
//! `SchemaRegistry::validate_completeness()` has existed for a long time but
//! used to be skipped by the generator. These tests assert that:
//!
//! 1. A synthetic single-API export with a missing JSON response schema fails
//!    `validate_completeness` and the resulting error contains the module
//!    name, the API name, and the missing type list.
//! 2. A synthetic grouped-module export rejects the entire module when one
//!    member API references a schema that is not in the merged registry.
//! 3. Real registries (`openai`, `samsung-smart-tv`, `ollama`, ...) still
//!    satisfy the strict check — i.e. the production data set is healthy.
//!
//! The CLI binary exercises the same `validate_completeness` call inside
//! `run_generate` / `run_generate_all`; this integration test pins the
//! registry-level contract so the binary keeps holding it.

use schematic_define::{ApiRequest, ApiResponse, AuthStrategy, Endpoint, RestApi, RestMethod};
use schematic_definitions::registry::{
    SchemaRegistry, get_registries_for_module, get_registry, registry_key_for,
};

/// Builds a tiny `RestApi` whose single endpoint references the supplied JSON
/// response type by name. Used by these tests to inject a missing-schema
/// scenario without depending on the real definitions in
/// `schematic-definitions`.
fn synthetic_api(name: &str, response_type: &str) -> RestApi {
    RestApi {
        name: name.to_string(),
        description: format!("Synthetic API for {} regression test", name),
        base_url: "https://example.test".to_string(),
        docs_url: None,
        auth: AuthStrategy::None,
        auth_policy: None,
        env_auth: vec![],
        env_username: None,
        headers: vec![],
        endpoints: vec![Endpoint {
            id: "DanglingResponse".to_string(),
            method: RestMethod::Get,
            path: "/dangling".to_string(),
            description: String::new(),
            request: None,
            response: ApiResponse::json_type(response_type),
            headers: vec![],
            params: None,
            oauth_scopes: None,
        }],
        module_path: None,
        request_suffix: None,
        version: None,
        env_mapping: None,
    }
}

/// Mirrors `schematic_gen::main::missing_schemas_error` exactly. Kept here so
/// that the integration test can assert the full diagnostic format without
/// depending on a private binary helper.
fn render_error(module_name: &str, api_name: &str, missing: &[String]) -> String {
    let first = missing.first().map(String::as_str).unwrap_or("MissingType");
    format!(
        "OpenAPI registry incomplete for module \"{module}\" (API \"{api}\"): \
         missing schema(s) {missing:?}. \
         Add JsonSchema derive + register::<T>(\"{first}\") entries in \
         schematic-definitions, or skip with --no-openapi.",
        module = module_name,
        api = api_name,
        missing = missing,
        first = first,
    )
}

#[test]
fn single_api_with_missing_schema_fails_validation() {
    let api = synthetic_api("SyntheticOne", "MissingResponseType");
    let registry = SchemaRegistry::new();

    let result = registry.validate_completeness(&api);
    assert!(result.is_err(), "expected validation to fail");

    let missing = result.unwrap_err();
    assert!(
        missing.iter().any(|m| m == "MissingResponseType"),
        "missing list should contain MissingResponseType, got {:?}",
        missing,
    );

    // Verify the rendered error message contains every diagnostic field that
    // a generator user needs: module, API, and missing list.
    let rendered = render_error("synthetic", &api.name, &missing);
    assert!(rendered.contains("\"synthetic\""), "expected module name in: {}", rendered);
    assert!(rendered.contains("\"SyntheticOne\""), "expected API name in: {}", rendered);
    assert!(rendered.contains("MissingResponseType"), "expected missing type in: {}", rendered);
    assert!(rendered.contains("--no-openapi"), "expected escape-hatch hint in: {}", rendered);
}

#[test]
fn grouped_module_with_missing_schema_in_one_member_fails() {
    // Two APIs sharing a module. The first is fine (its schema is registered);
    // the second references a type that is NOT in the merged registry.
    let good = synthetic_api("MemberA", "GoodResponse");
    let bad = synthetic_api("MemberB", "TotallyMissing");

    let mut merged = SchemaRegistry::new();
    // Use any concrete JsonSchema type — content does not matter, only the
    // name does, because validate_completeness only checks key presence.
    merged = merged.register::<u32>("GoodResponse");

    // Member A validates against the merged registry.
    merged
        .validate_completeness(&good)
        .expect("MemberA should validate");

    // Member B does not — exactly the regression we are guarding against.
    let result = merged.validate_completeness(&bad);
    let missing = result.expect_err("MemberB should fail validation");
    assert_eq!(missing, vec!["TotallyMissing".to_string()]);

    let rendered = render_error("synthetic-group", &bad.name, &missing);
    assert!(rendered.contains("\"synthetic-group\""));
    assert!(rendered.contains("\"MemberB\""));
    assert!(rendered.contains("TotallyMissing"));
}

#[test]
fn dangling_json_ref_cannot_be_emitted() {
    // This is the headline regression test for Finding 4: pretend a real
    // module's registry suddenly forgets one of its response schemas (as if
    // a future commit removed a `.register::<T>("T")` line). The grouped
    // export path must reject the module before any `$ref` is written.
    let api = synthetic_api("OpenAI", "ListModelsResponse");

    // Empty registry stands in for the broken state: caller forgot to
    // register the schema.
    let broken = SchemaRegistry::new();
    let missing = broken
        .validate_completeness(&api)
        .expect_err("dangling $ref must be rejected");

    let rendered = render_error("openai", &api.name, &missing);
    assert!(
        rendered.contains("ListModelsResponse"),
        "rendered error must name the dangling type, got: {}",
        rendered,
    );
    assert!(
        rendered.contains("OpenAPI registry incomplete"),
        "rendered error must use the canonical strict-mode prefix, got: {}",
        rendered,
    );
}

#[test]
fn real_registries_still_pass_strict_completeness() {
    // Sanity guard: every API exposed by the generator must still satisfy
    // validate_completeness against its own registry. If this test fails,
    // the generator itself is unable to emit OpenAPI for that API and a
    // human needs to fix the registry before merging.
    let api_names = [
        "Anthropic",
        "Bitbucket",
        "OpenAI",
        "ElevenLabs",
        "Gitea",
        "GitHub",
        "GitLab",
        "HuggingFaceHub",
        "LmStudio",
        "OllamaNative",
        "OllamaOpenAI",
        "EmqxBasic",
        "EmqxBearer",
        "Eversolo",
        "SamsungSmartTv",
        "UnfoldedCircleCoreRest",
    ];

    for api_name in api_names {
        let key = registry_key_for(api_name);
        let registry = get_registry(&key)
            .unwrap_or_else(|| panic!("get_registry({key:?}) returned None"));
        let api = lookup_api(api_name);
        registry.validate_completeness(&api).unwrap_or_else(|missing| {
            panic!(
                "{api_name}: per-API registry should be complete, missing {missing:?}",
            )
        });
    }
}

#[test]
fn real_module_registries_still_pass_strict_completeness() {
    // Mirrors what `run_generate_all` does: for each module, build the
    // merged registry and validate every member API against it.
    let grouped = schematic_definitions::apis_by_module();
    for (module_name, member_apis) in grouped.iter() {
        let registry = match get_registries_for_module(module_name) {
            Some(reg) => reg,
            None => continue, // Modules without a registry are not exported.
        };
        for api in member_apis.iter() {
            registry.validate_completeness(api).unwrap_or_else(|missing| {
                panic!(
                    "module {module_name:?}, API {api_name:?}: missing {missing:?}",
                    api_name = api.name,
                )
            });
        }
    }
}

#[test]
fn request_body_types_must_be_registered() {
    // An API with a JSON request body whose type is not in the registry must
    // fail validation. This is the request-side counterpart to the response
    // schema checks above.
    let api = RestApi {
        name: "RequestBodyTest".to_string(),
        description: "Test API with missing request body type".to_string(),
        base_url: "https://example.test".to_string(),
        docs_url: None,
        auth: AuthStrategy::None,
        auth_policy: None,
        env_auth: vec![],
        env_username: None,
        headers: vec![],
        endpoints: vec![Endpoint {
            id: "Create".to_string(),
            method: RestMethod::Post,
            path: "/create".to_string(),
            description: String::new(),
            request: Some(ApiRequest::json_type("SomeBody")),
            response: ApiResponse::Empty,
            headers: vec![],
            params: None,
            oauth_scopes: None,
        }],
        module_path: None,
        request_suffix: None,
        version: None,
        env_mapping: None,
    };

    let registry = SchemaRegistry::new();
    let result = registry.validate_completeness(&api);
    assert!(result.is_err(), "expected validation to fail for missing request body type");

    let missing = result.unwrap_err();
    assert!(
        missing.iter().any(|m| m == "SomeBody"),
        "missing list should contain SomeBody, got {:?}",
        missing,
    );
}

/// Resolves an API by its `RestApi::name` value. Mirrors the binary's
/// `resolve_api` switch but keyed on the struct-style names used inside
/// `apis_by_module`.
fn lookup_api(api_name: &str) -> RestApi {
    use schematic_definitions::{
        anthropic::define_anthropic_api,
        bitbucket::define_bitbucket_api,
        elevenlabs::define_elevenlabs_rest_api,
        emqx::{define_emqx_basic_api, define_emqx_bearer_api},
        eversolo::define_eversolo_api,
        gitea::define_gitea_api,
        github::define_github_api,
        gitlab::define_gitlab_api,
        huggingface::define_huggingface_hub_api,
        lmstudio::define_lmstudio_api,
        ollama::{define_ollama_native_api, define_ollama_openai_api},
        openai::define_openai_api,
        samsung_smart_tv::define_samsung_smart_tv_api,
        unfolded_circle::define_unfolded_circle_core_rest_api,
    };

    match api_name {
        "Anthropic" => define_anthropic_api(),
        "Bitbucket" => define_bitbucket_api(),
        "OpenAI" => define_openai_api(),
        "ElevenLabs" => define_elevenlabs_rest_api(),
        "Gitea" => define_gitea_api(),
        "GitHub" => define_github_api(),
        "GitLab" => define_gitlab_api(),
        "HuggingFaceHub" => define_huggingface_hub_api(),
        "LmStudio" => define_lmstudio_api(),
        "OllamaNative" => define_ollama_native_api(),
        "OllamaOpenAI" => define_ollama_openai_api(),
        "EmqxBasic" => define_emqx_basic_api(),
        "EmqxBearer" => define_emqx_bearer_api(),
        "Eversolo" => define_eversolo_api(),
        "SamsungSmartTv" => define_samsung_smart_tv_api(),
        "UnfoldedCircleCoreRest" => define_unfolded_circle_core_rest_api(),
        other => panic!("unknown API name {other:?}"),
    }
}
