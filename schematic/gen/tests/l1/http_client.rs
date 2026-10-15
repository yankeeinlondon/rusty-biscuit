//! Integration tests for the generated client module of a REST API.
//!
//! A generated module no longer carries an HTTP client: sending, status
//! checks, credential application, and query encoding live in
//! `schematic-runtime` and are tested there. These tests verify that the
//! module:
//! - declares the API's one client struct over the runtime client, its tag
//!   type, and the struct's constants and infallible constructor
//! - scopes every request to the API by naming the tag in its
//!   `EndpointSpec` implementation, with the endpoint's HTTP method
//! - carries the definition's authentication translation in `definition()`
//! - has no send path, request enum, or credential store of its own; its
//!   only sends are the endpoint methods' calls through the runtime client

use proc_macro2::TokenStream;
use schematic_define::{
    ApiResponse, AuthMethod, AuthPolicy, AuthStrategy, Endpoint, EnvAuthStrategy,
    OAuth2ClientAuthMethod, OAuth2Config, OAuth2GrantType, PkceRequirement, RestApi, RestMethod,
};
use schematic_gen::output::assemble::assemble_shared_module;
use schematic_gen::output::assemble_api_code;

/// Formats generated tokens into readable code for assertions.
fn format_tokens(tokens: &TokenStream) -> String {
    let file = syn::parse2::<syn::File>(tokens.clone()).expect("Generated code should be valid");
    prettyplease::unparse(&file)
}

/// `code` with all whitespace, and any trailing comma before `)`, `]`, or
/// `}`, removed, so assertions survive line wrapping.
fn squash(code: &str) -> String {
    let flat: String = code.split_whitespace().collect();
    flat.replace(",)", ")").replace(",]", "]").replace(",}", "}")
}

/// `code` without its doc comments, whose usage examples legitimately
/// mention `send` and `async fn`.
fn without_docs(code: &str) -> String {
    code.lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// The generated `definition()`, squashed, up to the doc comment of the
/// `new()` that follows it.
fn definition_body(code: &str) -> String {
    let flat = squash(code);
    let start = flat
        .find("pubfndefinition()")
        .unwrap_or_else(|| panic!("definition() is missing:\n{code}"));
    let end = flat[start..]
        .find("///")
        .unwrap_or_else(|| panic!("new() is missing:\n{code}"));
    flat[start..start + end].to_string()
}

/// Creates a test API with the given auth strategy.
fn make_api(name: &str, auth: AuthStrategy, env_auth: Vec<String>) -> RestApi {
    {
        let mut api = schematic_define::RestApi::builder(&schematic_define::DefinitionContext::types_in("fixture"), name.to_string(), "https://api.example.com/v1".to_string())
            .describes(format!("{} API", name))
            .authentication(auth, env_auth)
            .build().expect("valid API fixture");
        api.endpoints = vec![
            {
                let mut endpoint = Endpoint::new("GetItems".to_string(), RestMethod::Get, "/items".to_string())
                    .describes("Get items".to_string())
                    .returns_empty()
                    .build().expect("valid endpoint fixture");
                endpoint.response = ApiResponse::json_type("ItemsResponse");
                endpoint
            },
            {
                let mut endpoint = Endpoint::new("GetItem".to_string(), RestMethod::Get, "/items/{item_id}".to_string())
                    .describes("Get a single item".to_string())
                    .returns_empty()
                    .build().expect("valid endpoint fixture");
                endpoint.response = ApiResponse::json_type("Item");
                endpoint
            },
        ];
        api
    }
}

/// Creates a test API with basic auth.
/// Password comes from env_auth[0].
fn make_basic_auth_api(name: &str, username_env: &str, password_env: &str) -> RestApi {
    {
        let mut api = schematic_define::RestApi::builder(&schematic_define::DefinitionContext::types_in("fixture"), name.to_string(), "https://api.example.com/v1".to_string())
            .describes(format!("{} API", name))
            .authentication(AuthStrategy::Basic, vec![password_env.to_string()])
            .build().expect("valid API fixture");
        api.env_username = Some(username_env.to_string());
        api.endpoints = vec![{
            let mut endpoint = Endpoint::new("GetItems".to_string(), RestMethod::Get, "/items".to_string())
                .describes("Get items".to_string())
                .returns_empty()
                .build().expect("valid endpoint fixture");
            endpoint.response = ApiResponse::json_type("ItemsResponse");
            endpoint
        }];
        api
    }
}

fn make_dual_auth_api() -> RestApi {
    let oauth = OAuth2Config {
        grant_type: OAuth2GrantType::AuthorizationCodePkce,
        authorization_url: Some("https://example.com/oauth/authorize".to_string()),
        token_url: "https://example.com/oauth/token".to_string(),
        revocation_url: None,
        device_authorization_url: None,
        default_scopes: vec!["read".to_string()],
        pkce: PkceRequirement::Required,
        client_auth: OAuth2ClientAuthMethod::ClientSecretPost,
    };

    {
        let mut api = schematic_define::RestApi::builder(&schematic_define::DefinitionContext::types_in("fixture"), "DualAuthApi".to_string(), "https://api.example.com/v1".to_string())
            .describes("Dual auth API".to_string())
            .authentication(AuthStrategy::ApiKey {
            header: "X-API-Key".to_string(),
            value_prefix: None,
        }, vec!["DUAL_AUTH_API_KEY".to_string()])
            .build().expect("valid API fixture");
        api.auth_policy = Some(AuthPolicy {
            explicit: vec![
                AuthMethod::ApiKey {
                    header: "X-API-Key".to_string(),
                    value_prefix: None,
                },
                AuthMethod::OAuth2(oauth),
            ],
            env_fallback: Some(EnvAuthStrategy::ApiKey {
                header: "X-API-Key".to_string(),
                value_prefix: None,
            }),
        });
        api.endpoints = vec![{
            let mut endpoint = Endpoint::new("GetItems".to_string(), RestMethod::Get, "/items".to_string())
                .describes("Get items".to_string())
                .returns_empty()
                .build().expect("valid endpoint fixture");
            endpoint.response = ApiResponse::json_type("ItemsResponse");
            endpoint
        }];
        api
    }
}

// =============================================================================
// API client struct and constructor
// =============================================================================

#[test]
fn module_declares_one_client_struct_and_a_tag_with_the_api_impl() {
    let api = make_api("TestApi", AuthStrategy::None, vec![]);
    let code = format_tokens(&assemble_api_code(&api));

    for expected in [
        "pub struct TestApi(schematic_runtime::Client<TestApiApi>);",
        "pub enum TestApiApi {}",
        "impl schematic_runtime::Api for TestApiApi {",
        r#"const NAME: &'static str = "TestApi";"#,
        "schematic_runtime::api_client!(TestApi, TestApiApi);",
    ] {
        assert!(code.contains(expected), "missing `{expected}`\nGenerated code:\n{code}");
    }
    assert!(!code.contains("TestApiClient"), "no client alias\nGenerated code:\n{code}");
}

#[test]
fn client_struct_holds_base_url_and_an_infallible_new() {
    let api = make_api("TestApi", AuthStrategy::None, vec![]);
    let code = format_tokens(&assemble_api_code(&api));
    let flat = squash(&code);

    assert!(
        code.contains(r#"pub const BASE_URL: &'static str = "https://api.example.com/v1";"#),
        "The client struct should hold the base URL\nGenerated code:\n{code}"
    );
    assert!(
        code.contains("pub const DOCS_URL: Option<&'static str> = None;"),
        "The client struct should hold the docs URL\nGenerated code:\n{code}"
    );
    assert!(
        flat.contains(&squash(
            "pub fn new() -> Self { Self(schematic_runtime::Client::new(Self::definition())) }"
        )),
        "new() should wrap a runtime client seeded from definition()\nGenerated code:\n{code}"
    );
    assert!(!flat.contains("fnbuilder()"), "no per-API builder\nGenerated code:\n{code}");
}

// =============================================================================
// Request scoping
// =============================================================================

#[test]
fn requests_name_the_api_marker() {
    let api = make_api("TestApi", AuthStrategy::None, vec![]);
    let code = format_tokens(&assemble_api_code(&api));

    for request in ["GetItemsRequest", "GetItemRequest"] {
        assert!(
            code.contains(&format!("impl schematic_runtime::EndpointSpec for {request} {{")),
            "{request} should implement EndpointSpec\nGenerated code:\n{code}"
        );
    }
    // One `type Api` per endpoint, each naming this API's marker, so its
    // client accepts these requests and no other API's client does.
    assert_eq!(
        code.matches("type Api = TestApiApi;").count(),
        api.endpoints.len(),
        "Generated code:\n{code}"
    );
    assert!(!code.contains("type Api = Api;"), "Generated code:\n{code}");
}

#[test]
fn each_http_method_becomes_the_request_method_constant() {
    let methods = [
        (RestMethod::Get, "GET"),
        (RestMethod::Post, "POST"),
        (RestMethod::Put, "PUT"),
        (RestMethod::Patch, "PATCH"),
        (RestMethod::Delete, "DELETE"),
        (RestMethod::Head, "HEAD"),
        (RestMethod::Options, "OPTIONS"),
    ];
    let mut api = make_api("TestApi", AuthStrategy::None, vec![]);
    api.endpoints = methods
        .iter()
        .map(|(method, name)| {
            let mut endpoint = Endpoint::new(format!("Do{name}"), *method, "/items".to_string())
                .describes(format!("{name} items"))
                .returns_empty()
                .build().expect("valid endpoint fixture");
            endpoint.response = ApiResponse::json_type("ItemsResponse");
            endpoint
        })
        .collect();
    let flat = squash(&format_tokens(&assemble_api_code(&api)));

    for (_, name) in methods {
        assert!(
            flat.contains(&squash(&format!(
                r#"const ENDPOINT_ID: &'static str = "Do{name}"; const METHOD: schematic_runtime::Method = schematic_runtime::Method::{name};"#
            ))),
            "Do{name} should declare Method::{name}"
        );
    }
}

// =============================================================================
// Authentication translation
// =============================================================================

#[test]
fn no_auth_definition_has_no_auth_config() {
    let api = make_api("NoAuthApi", AuthStrategy::None, vec![]);
    let code = format_tokens(&assemble_api_code(&api));

    assert_eq!(
        definition_body(&code),
        squash(
            "pub fn definition() -> schematic_runtime::ApiDefinition { schematic_runtime::ApiDefinition::new(Self::BASE_URL) }"
        ),
        "An API without credentials keeps the runtime default\nGenerated code:\n{code}"
    );
}

#[test]
fn definition_carries_the_auth_translation() {
    let cases = [
        (
            make_api(
                "BearerApi",
                AuthStrategy::BearerToken { header: None },
                vec!["BEARER_TOKEN".to_string()],
            ),
            r#"schematic_runtime::EnvFallback::new(schematic_runtime::CredentialPlacement::Bearer { header: None }, ["BEARER_TOKEN"])"#,
        ),
        (
            make_api(
                "CustomBearerApi",
                AuthStrategy::BearerToken {
                    header: Some("X-Custom-Auth".to_string()),
                },
                vec!["MY_TOKEN".to_string()],
            ),
            r#"schematic_runtime::CredentialPlacement::Bearer { header: Some("X-Custom-Auth".to_string()) }, ["MY_TOKEN"]"#,
        ),
        (
            make_api(
                "ApiKeyApi",
                AuthStrategy::ApiKey {
                    header: "X-API-Key".to_string(),
                    value_prefix: None,
                },
                vec!["API_SECRET".to_string()],
            ),
            r#"schematic_runtime::CredentialPlacement::ApiKeyHeader { header: "X-API-Key".to_string(), prefix: None }, ["API_SECRET"]"#,
        ),
        (
            make_basic_auth_api("BasicAuthApi", "API_USER", "API_PASS"),
            r#"schematic_runtime::EnvFallback::new(schematic_runtime::CredentialPlacement::Basic, ["API_PASS"]).username_vars(["API_USER"])"#,
        ),
    ];

    for (api, expected) in cases {
        let code = format_tokens(&assemble_api_code(&api));
        let definition = definition_body(&code);
        assert!(
            definition.contains(".auth(schematic_runtime::AuthConfig::none()"),
            "{} should configure auth\nGenerated code:\n{code}",
            api.name
        );
        assert!(
            definition.contains(&squash(expected)),
            "{} should translate to `{expected}`\nGenerated code:\n{code}",
            api.name
        );
    }
}

#[test]
fn dual_auth_definition_accepts_api_key_and_oauth_bearer_tokens() {
    let api = make_dual_auth_api();
    let code = format_tokens(&assemble_api_code(&api));
    let definition = definition_body(&code);

    // Each explicit method is accepted; the OAuth2 token travels as a bearer.
    assert!(
        definition.contains(&squash(
            r#".accept(schematic_runtime::CredentialPlacement::ApiKeyHeader { header: "X-API-Key".to_string(), prefix: None })"#
        )),
        "Should accept the API key\nGenerated code:\n{code}"
    );
    assert!(
        definition.contains(&squash(
            ".accept(schematic_runtime::CredentialPlacement::Bearer { header: None })"
        )),
        "Should accept the OAuth token as a bearer token\nGenerated code:\n{code}"
    );
    assert!(
        definition.contains(r#"["DUAL_AUTH_API_KEY"]"#),
        "Should read the API key variable as the fallback\nGenerated code:\n{code}"
    );
    // Credentials are supplied to the runtime client, not set on the module.
    assert!(!code.contains("pub fn api_key("), "Generated code:\n{code}");
    assert!(!code.contains("pub fn oauth_token("), "Generated code:\n{code}");
}

// =============================================================================
// What the module no longer contains
// =============================================================================

#[test]
fn module_has_no_send_path_of_its_own() {
    let api = make_api(
        "TestApi",
        AuthStrategy::BearerToken { header: None },
        vec!["BEARER_TOKEN".to_string()],
    );
    let code = without_docs(&format_tokens(&assemble_api_code(&api)));

    for removed in [
        "reqwest",
        "async fn",
        "pub enum TestApiRequest",
        "SchematicError",
        "ResponseKind",
        "RequestParts =",
        "fn request",
        "build_and_send_request",
        "merge_headers",
        "append_query_pairs",
        "fn with_client",
        "fn with_base_url",
        "fn http_client",
        "fn api_base_url",
        "fn auth_policy",
        "fn bearer_token",
        "fn variant",
        "VariantHooks",
        "resolve_request_auth",
        "schematic_define::",
    ] {
        assert!(!code.contains(removed), "`{removed}` is still emitted:\n{code}");
    }

    // The only sends are the endpoint methods', through the runtime client,
    // one per endpoint, each returning that endpoint's call type.
    let flat = squash(&code);
    assert_eq!(flat.matches(".send(").count(), 2, "{code}");
    assert_eq!(flat.matches("self.0.send(").count(), 2, "{code}");
    assert!(
        flat.contains("pubfnget_items(&self)->GetItemsCall<'_>{GetItemsCall(self.0.send(GetItemsRequest::default()))}"),
        "{code}"
    );
    assert!(
        flat.contains("pubfnget_item(&self,item_id:implInto<String>)->GetItemCall<'_>{GetItemCall(self.0.send(GetItemRequest::new(item_id)))}"),
        "{code}"
    );
}

#[test]
fn shared_module_holds_no_transport_or_auth_resolution() {
    let shared = format_tokens(&assemble_shared_module());

    assert!(shared.contains("pub fn combine_cookie_headers"), "{shared}");
    for removed in ["resolve_request_auth", "SchematicError", "reqwest::", "schematic_define::"] {
        assert!(!shared.contains(removed), "`{removed}` is still emitted:\n{shared}");
    }
}
