use super::helpers::generate_headers_init;
use super::*;
use crate::codegen::request_structs::{format_generated_code, validate_generated_code};
use schematic_define::AuthStrategy;

fn make_api(name: &str, base_url: &str, description: &str) -> RestApi {
    RestApi {
        name: name.to_string(),
        description: description.to_string(),
        base_url: base_url.to_string(),
        docs_url: None,
        auth: AuthStrategy::None,
        auth_policy: None,
        env_auth: vec![],
        env_username: None,
        env_mapping: None,
        headers: vec![],
        endpoints: vec![],
        module_path: None,
        request_suffix: None,
        version: None,
    }
}

#[test]
fn generate_api_struct_basic() {
    let api = make_api("OpenAi", "https://api.openai.com/v1", "OpenAI API");
    let tokens = generate_api_struct(&api);
    let code = format_generated_code(&tokens).expect("Failed to format code");

    // Check struct definition
    assert!(code.contains("pub struct OpenAi"));
    assert!(code.contains("client: reqwest::Client"));
    assert!(code.contains("base_url: String"));
    assert!(code.contains("env_auth: Vec<String>"));
    assert!(code.contains("auth_strategy: schematic_define::AuthStrategy"));
    assert!(code.contains("env_username: Option<String>"));

    // Check BASE_URL constant
    assert!(code.contains("pub const BASE_URL: &'static str"));
    assert!(code.contains("https://api.openai.com/v1"));

    // Check new() constructor
    assert!(code.contains("pub fn new() -> Self"));
    // The default client carries a request timeout, falling back to an untimed
    // client only if the builder fails.
    assert!(code.contains("reqwest::Client::builder()"));
    assert!(code.contains(".timeout("));
    assert!(code.contains("reqwest::Client::new()"));
    assert!(code.contains("Self::BASE_URL.to_string()"));

    // Check with_base_url() constructor
    assert!(code.contains("pub fn with_base_url(base_url: impl Into<String>) -> Self"));
    assert!(code.contains("base_url.into()"));

    // Check Default impl
    assert!(code.contains("impl Default for OpenAi"));
    assert!(code.contains("Self::new()"));
}

#[test]
fn generate_api_struct_validates_syntax() {
    let api = make_api("TestApi", "https://example.com/api", "Test API");
    let tokens = generate_api_struct(&api);
    assert!(validate_generated_code(&tokens).is_ok());
}

#[test]
fn generate_api_struct_with_different_names() {
    let test_cases = [
        ("Gemini", "https://generativelanguage.googleapis.com"),
        ("Anthropic", "https://api.anthropic.com/v1"),
        ("GitHub", "https://api.github.com"),
    ];

    for (name, base_url) in test_cases {
        let api = make_api(name, base_url, &format!("{} API", name));
        let tokens = generate_api_struct(&api);
        let code = format_generated_code(&tokens).expect("Failed to format code");

        assert!(
            code.contains(&format!("pub struct {}", name)),
            "Expected struct {} in generated code",
            name
        );
        assert!(
            code.contains(base_url),
            "Expected BASE_URL {} in generated code",
            base_url
        );
    }
}

#[test]
fn generate_api_struct_doc_comment_includes_description() {
    let api = make_api("Custom", "https://api.custom.com", "Custom Service API");
    let tokens = generate_api_struct(&api);
    let code = format_generated_code(&tokens).expect("Failed to format code");

    // Doc comment should include the description
    assert!(code.contains("Custom Service API client"));
}

#[test]
fn generate_api_struct_with_special_url_characters() {
    let api = make_api(
        "SpecialApi",
        "https://api.example.com:8443/v2/beta",
        "API with port and path",
    );
    let tokens = generate_api_struct(&api);
    let code = format_generated_code(&tokens).expect("Failed to format code");

    assert!(code.contains("https://api.example.com:8443/v2/beta"));
}

#[test]
fn generate_api_struct_has_with_client_constructor() {
    let api = make_api("TestApi", "https://api.test.com", "Test API");
    let tokens = generate_api_struct(&api);
    let code = format_generated_code(&tokens).expect("Failed to format code");

    // Check with_client() constructor
    assert!(code.contains("pub fn with_client(client: reqwest::Client) -> Self"));
    assert!(code.contains("Self::BASE_URL.to_string()"));
}

#[test]
fn generate_api_struct_has_with_client_and_base_url_constructor() {
    let api = make_api("TestApi", "https://api.test.com", "Test API");
    let tokens = generate_api_struct(&api);
    let code = format_generated_code(&tokens).expect("Failed to format code");

    // Check with_client_and_base_url() constructor
    assert!(code.contains("pub fn with_client_and_base_url"));
    assert!(code.contains("client: reqwest::Client"));
    assert!(code.contains("base_url: impl Into<String>"));
}

#[test]
fn generate_api_struct_with_bearer_auth() {
    let api = RestApi {
        name: "BearerApi".to_string(),
        description: "Bearer Auth API".to_string(),
        base_url: "https://api.bearer.com".to_string(),
        docs_url: None,
        auth: AuthStrategy::BearerToken { header: None },
        auth_policy: None,
        env_auth: vec!["BEARER_TOKEN".to_string()],
        env_username: None,
        env_mapping: None,
        headers: vec![],
        endpoints: vec![],
        module_path: None,
        request_suffix: None,
        version: None,
    };
    let tokens = generate_api_struct(&api);
    let code = format_generated_code(&tokens).expect("Failed to format code");

    assert!(code.contains("schematic_define::AuthStrategy::BearerToken"));
    assert!(code.contains("BEARER_TOKEN"));
}

#[test]
fn generate_api_struct_with_api_key_auth() {
    let api = RestApi {
        name: "ApiKeyApi".to_string(),
        description: "API Key Auth API".to_string(),
        base_url: "https://api.apikey.com".to_string(),
        docs_url: None,
        auth: AuthStrategy::ApiKey {
            header: "X-API-Key".to_string(),
            value_prefix: None,
        },
        auth_policy: None,
        env_auth: vec!["API_KEY".to_string()],
        env_username: None,
        env_mapping: None,
        headers: vec![],
        endpoints: vec![],
        module_path: None,
        request_suffix: None,
        version: None,
    };
    let tokens = generate_api_struct(&api);
    let code = format_generated_code(&tokens).expect("Failed to format code");

    assert!(code.contains("schematic_define::AuthStrategy::ApiKey"));
    assert!(code.contains("X-API-Key"));
    assert!(code.contains("API_KEY"));
}

#[test]
fn generate_api_struct_with_basic_auth() {
    let api = RestApi {
        name: "BasicApi".to_string(),
        description: "Basic Auth API".to_string(),
        base_url: "https://api.basic.com".to_string(),
        docs_url: None,
        auth: AuthStrategy::Basic,
        auth_policy: None,
        env_auth: vec!["BASIC_PASS".to_string()],
        env_username: Some("BASIC_USER".to_string()),
        env_mapping: None,
        headers: vec![],
        endpoints: vec![],
        module_path: None,
        request_suffix: None,
        version: None,
    };
    let tokens = generate_api_struct(&api);
    let code = format_generated_code(&tokens).expect("Failed to format code");

    assert!(code.contains("schematic_define::AuthStrategy::Basic"));
    assert!(code.contains("BASIC_PASS"));
    assert!(code.contains("BASIC_USER"));
}

#[test]
fn generate_auth_strategy_init_none() {
    let tokens = generate_auth_strategy_init(&AuthStrategy::None);
    let code = tokens.to_string();
    assert!(code.contains("AuthStrategy :: None"));
}

#[test]
fn generate_auth_strategy_init_bearer_without_header() {
    let tokens = generate_auth_strategy_init(&AuthStrategy::BearerToken { header: None });
    let code = tokens.to_string();
    assert!(code.contains("AuthStrategy :: BearerToken"));
    assert!(code.contains("header : None"));
}

#[test]
fn generate_auth_strategy_init_bearer_with_header() {
    let tokens = generate_auth_strategy_init(&AuthStrategy::BearerToken {
        header: Some("X-Custom".to_string()),
    });
    let code = tokens.to_string();
    assert!(code.contains("AuthStrategy :: BearerToken"));
    assert!(code.contains("X-Custom"));
}

#[test]
fn generate_auth_strategy_init_api_key() {
    let tokens = generate_auth_strategy_init(&AuthStrategy::ApiKey {
        header: "X-API-Key".to_string(),
        value_prefix: None,
    });
    let code = tokens.to_string();
    assert!(code.contains("AuthStrategy :: ApiKey"));
    assert!(code.contains("X-API-Key"));
}

#[test]
fn generate_auth_strategy_init_basic() {
    let tokens = generate_auth_strategy_init(&AuthStrategy::Basic);
    let code = tokens.to_string();
    assert!(code.contains("AuthStrategy :: Basic"));
}

#[test]
fn generate_api_struct_has_variant_method() {
    let api = make_api("TestApi", "https://api.test.com", "Test API");
    let tokens = generate_api_struct(&api);
    let code = format_generated_code(&tokens).expect("Failed to format code");

    // Check variant() method returns a builder
    assert!(code.contains("pub fn variant(&self) -> TestApiVariantBuilder"));
    assert!(code.contains("TestApiVariantBuilder::new(self)"));
}

#[test]
fn generate_api_struct_has_variant_with_method() {
    let api = make_api("TestApi", "https://api.test.com", "Test API");
    let tokens = generate_api_struct(&api);
    let code = format_generated_code(&tokens).expect("Failed to format code");

    // Check variant_with() convenience method exists
    assert!(code.contains("pub fn variant_with("));
    assert!(code.contains("base_url: impl Into<String>"));
    assert!(code.contains("env_auth: Vec<String>"));
    assert!(code.contains("strategy: schematic_define::UpdateStrategy"));
}

#[test]
fn variant_builder_handles_no_change() {
    let api = make_api("TestApi", "https://api.test.com", "Test API");
    let tokens = generate_api_struct(&api);
    let code = format_generated_code(&tokens).expect("Failed to format code");

    // Check builder's build() method handles UpdateStrategy::NoChange
    assert!(code.contains("UpdateStrategy::NoChange => self.base.auth_strategy.clone()"));
}

#[test]
fn variant_builder_handles_change_to() {
    let api = make_api("TestApi", "https://api.test.com", "Test API");
    let tokens = generate_api_struct(&api);
    let code = format_generated_code(&tokens).expect("Failed to format code");

    // Check builder's build() method handles UpdateStrategy::ChangeTo
    assert!(code.contains("UpdateStrategy::ChangeTo(auth) => auth"));
}

#[test]
fn variant_builder_clones_client() {
    let api = make_api("TestApi", "https://api.test.com", "Test API");
    let tokens = generate_api_struct(&api);
    let code = format_generated_code(&tokens).expect("Failed to format code");

    // Builder's build() should clone the client
    assert!(code.contains("client: self.base.client.clone()"));
}

#[test]
fn variant_builder_clones_env_username() {
    let api = make_api("TestApi", "https://api.test.com", "Test API");
    let tokens = generate_api_struct(&api);
    let code = format_generated_code(&tokens).expect("Failed to format code");

    // Builder's build() should clone env_username
    assert!(code.contains("env_username: self.base.env_username.clone()"));
}

#[test]
fn generate_api_struct_has_headers_field() {
    let api = make_api("TestApi", "https://api.test.com", "Test API");
    let tokens = generate_api_struct(&api);
    let code = format_generated_code(&tokens).expect("Failed to format code");

    // Should have headers field in struct (now uses Headers type)
    assert!(code.contains("headers: schematic_define::Headers"));
}

#[test]
fn generate_api_struct_with_headers() {
    let api = RestApi {
        name: "HeaderApi".to_string(),
        description: "API with headers".to_string(),
        base_url: "https://api.headers.com".to_string(),
        docs_url: None,
        auth: AuthStrategy::None,
        auth_policy: None,
        env_auth: vec![],
        env_username: None,
        env_mapping: None,
        headers: vec![
            ("X-Api-Version".to_string(), "2024-01".to_string()),
            ("X-Custom-Header".to_string(), "custom-value".to_string()),
        ],
        endpoints: vec![],
        module_path: None,
        request_suffix: None,
        version: None,
    };
    let tokens = generate_api_struct(&api);
    let code = format_generated_code(&tokens).expect("Failed to format code");

    // Should contain the header keys and values
    assert!(code.contains("X-Api-Version"));
    assert!(code.contains("2024-01"));
    assert!(code.contains("X-Custom-Header"));
    assert!(code.contains("custom-value"));
}

#[test]
fn generate_headers_init_empty() {
    let headers: Vec<(String, String)> = vec![];
    let tokens = generate_headers_init(&headers);
    let code = tokens.to_string();
    assert!(code.contains("vec !"));
}

#[test]
fn generate_headers_init_with_values() {
    let headers = vec![
        ("Header-One".to_string(), "value-one".to_string()),
        ("Header-Two".to_string(), "value-two".to_string()),
    ];
    let tokens = generate_headers_init(&headers);
    let code = tokens.to_string();
    assert!(code.contains("Header-One"));
    assert!(code.contains("value-one"));
    assert!(code.contains("Header-Two"));
    assert!(code.contains("value-two"));
}

#[test]
fn variant_builder_clones_headers() {
    let api = make_api("TestApi", "https://api.test.com", "Test API");
    let tokens = generate_api_struct(&api);
    let code = format_generated_code(&tokens).expect("Failed to format code");

    // Builder's build() should clone headers via unwrap_or_else
    assert!(code.contains("self.base.headers.clone()"));
}

#[test]
fn variant_builder_checks_env_override_before_consuming_option() {
    let api = make_api("TestApi", "https://api.test.com", "Test API");
    let tokens = generate_api_struct(&api);
    let code = format_generated_code(&tokens).expect("Failed to format code");

    assert!(code.contains("let has_env_auth_override = self.env_auth.is_some();"));
    assert!(code.contains("let env_auth = self.env_auth.unwrap_or_else"));
    assert!(code.contains("None if has_env_auth_override"));
}

#[test]
fn variant_builder_has_mutate_response_method() {
    let api = make_api("TestApi", "https://api.test.com", "Test API");
    let tokens = generate_api_struct(&api);
    let code = format_generated_code(&tokens).expect("Failed to format code");

    // Check mutate_response method exists
    assert!(code.contains("pub fn mutate_response<R, F>(mut self, hook: F) -> Self"));
    assert!(code.contains("R: crate::shared::EndpointSpec"));
}

#[test]
fn variant_builder_has_pre_response_json_method() {
    let api = make_api("TestApi", "https://api.test.com", "Test API");
    let tokens = generate_api_struct(&api);
    let code = format_generated_code(&tokens).expect("Failed to format code");

    // Check pre_response_json method exists
    assert!(code.contains("pub fn pre_response_json<F>(mut self, hook: F) -> Self"));
}

#[test]
fn generate_api_struct_has_docs_url_constant() {
    let api = RestApi {
        name: "DocsApi".to_string(),
        description: "API with documentation URL".to_string(),
        base_url: "https://api.example.com/v1".to_string(),
        docs_url: Some("https://docs.example.com/api".to_string()),
        auth: AuthStrategy::None,
        auth_policy: None,
        env_auth: vec![],
        env_username: None,
        env_mapping: None,
        headers: vec![],
        endpoints: vec![],
        module_path: None,
        request_suffix: None,
        version: None,
    };
    let tokens = generate_api_struct(&api);
    let code = format_generated_code(&tokens).expect("Failed to format code");

    // Check DOCS_URL constant exists and is Some
    assert!(
        code.contains("pub const DOCS_URL: Option<&'static str>"),
        "Expected DOCS_URL constant declaration"
    );
    assert!(
        code.contains("Some(\"https://docs.example.com/api\")"),
        "Expected DOCS_URL to contain the documentation URL"
    );
}

#[test]
fn generate_api_struct_docs_url_none() {
    let api = make_api(
        "NoDocsApi",
        "https://api.nodocs.com",
        "API without documentation",
    );
    let tokens = generate_api_struct(&api);
    let code = format_generated_code(&tokens).expect("Failed to format code");

    // Check DOCS_URL constant exists and is None
    assert!(
        code.contains("pub const DOCS_URL: Option<&'static str>"),
        "Expected DOCS_URL constant declaration"
    );
    assert!(
        code.contains("DOCS_URL: Option<&'static str> = None"),
        "Expected DOCS_URL to be None when docs_url is not provided"
    );
}
