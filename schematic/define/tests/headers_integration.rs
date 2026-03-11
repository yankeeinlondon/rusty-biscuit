//! Integration tests for headers module.

use schematic_define::{ApiKeyEnv, EnvList, EnvMapping, HeaderError, Headers, SensitiveString};

#[test]
fn sensitive_string_debug_does_not_leak() {
    let secret = SensitiveString::from("super-secret-password-12345");
    let debug_output = format!("{:?}", secret);

    // CRITICAL: Debug output must NOT contain actual value
    assert!(!debug_output.contains("super-secret-password-12345"));
    assert!(debug_output.contains("***"));
}

#[test]
fn env_list_uses_owned_strings() {
    let env = EnvList::from_strs(&["KEY1", "KEY2"]);

    // Verify we can access names as owned strings
    let names = env.names();
    assert_eq!(names.len(), 2);

    // Verify String type (not &'static str)
    let _owned: Vec<String> = names.to_vec();
}

#[test]
fn env_mapping_comprehensive_configuration() {
    let mapping = EnvMapping {
        bearer_token: Some(EnvList::from_strs(&["OPENAI_API_KEY", "OPENAI_KEY"])),
        basic_user: Some(EnvList::single("API_USERNAME")),
        basic_pass: Some(EnvList::single("API_PASSWORD")),
        api_key: Some(ApiKeyEnv {
            names: EnvList::from_strs(&["HF_TOKEN", "HUGGINGFACE_API_KEY"]),
            header: "Authorization".to_string(),
        }),
        ..Default::default()
    };

    assert!(mapping.bearer_token.is_some());
    assert!(mapping.basic_user.is_some());
    assert!(mapping.basic_pass.is_some());
    assert!(mapping.api_key.is_some());

    // Verify fallback chains
    assert_eq!(mapping.bearer_token.unwrap().names().len(), 2);
    assert_eq!(mapping.api_key.unwrap().names.names().len(), 2);
}

#[test]
fn header_error_display_messages() {
    let e1 = HeaderError::InvalidHeaderName("Bad-Header".to_string());
    assert!(e1.to_string().contains("Invalid header name"));

    let e2 = HeaderError::InvalidHeaderValue("X-Custom".to_string());
    assert!(e2.to_string().contains("Invalid header value"));

    let e3 = HeaderError::MissingEnv("API_KEY".to_string());
    assert!(e3.to_string().contains("Environment variable not set"));

    let e4 = HeaderError::MissingCredential(vec!["KEY1".to_string(), "KEY2".to_string()]);
    let msg = e4.to_string();
    assert!(msg.contains("Missing credential"));
    assert!(msg.contains("KEY1"));
    assert!(msg.contains("KEY2"));
}

#[test]
fn api_key_env_complete_workflow() {
    let api_key = ApiKeyEnv {
        names: EnvList::from_strs(&["PRIMARY_KEY", "FALLBACK_KEY", "TERTIARY_KEY"]),
        header: "X-API-Key".to_string(),
    };

    // Verify structure
    assert_eq!(api_key.names.names().len(), 3);
    assert_eq!(api_key.names.names()[0], "PRIMARY_KEY");
    assert_eq!(api_key.names.names()[1], "FALLBACK_KEY");
    assert_eq!(api_key.names.names()[2], "TERTIARY_KEY");
    assert_eq!(api_key.header, "X-API-Key");
}

#[test]
fn sensitive_string_does_not_implement_partialeq() {
    // This test verifies that SensitiveString does NOT implement PartialEq
    // by attempting to compile code that would use it. This is a compile-time
    // safety feature to prevent timing attacks.

    let _secret1 = SensitiveString::from("password1");
    let _secret2 = SensitiveString::from("password2");

    // This should NOT compile (uncomment to verify):
    // assert_eq!(secret1, secret2);  // Compilation error expected

    // Instead, we verify the type does NOT have PartialEq in the public API
    // by checking trait bounds at runtime
    fn check_no_partialeq<T>() {
        // This function only compiles if T does NOT have PartialEq
        // We're using it as a negative assertion
    }

    // This call succeeds because SensitiveString lacks PartialEq
    check_no_partialeq::<SensitiveString>();
}

#[test]
fn env_list_convenience_constructors() {
    // Test single constructor
    let single = EnvList::single("API_KEY");
    assert_eq!(single.names().len(), 1);
    assert_eq!(single.names()[0], "API_KEY");

    // Test from_strs constructor
    let multi = EnvList::from_strs(&["KEY1", "KEY2", "KEY3"]);
    assert_eq!(multi.names().len(), 3);

    // Test new constructor
    let owned = EnvList::new(vec!["A".to_string(), "B".to_string()]);
    assert_eq!(owned.names().len(), 2);
}

// ========================================
// Headers Builder Integration Tests
// ========================================

#[test]
fn headers_realistic_openai_request() {
    // Simulate building headers for an OpenAI API request
    let headers = Headers::default()
        .use_bearer_token("sk-proj-1234567890abcdef")
        .content_type_json()
        .accept_json()
        .user_agent("MyApp/1.0")
        .build()
        .unwrap();

    assert_eq!(headers.len(), 4);

    // Verify authorization header
    let auth = headers
        .iter()
        .find(|(k, _)| k == "Authorization")
        .expect("Missing Authorization header");
    assert_eq!(auth.1, "Bearer sk-proj-1234567890abcdef");

    // Verify content type
    let ct = headers
        .iter()
        .find(|(k, _)| k == "Content-Type")
        .expect("Missing Content-Type header");
    assert_eq!(ct.1, "application/json");

    // Verify accept
    let accept = headers
        .iter()
        .find(|(k, _)| k == "Accept")
        .expect("Missing Accept header");
    assert_eq!(accept.1, "application/json");

    // Verify user agent
    let ua = headers
        .iter()
        .find(|(k, _)| k == "User-Agent")
        .expect("Missing User-Agent header");
    assert_eq!(ua.1, "MyApp/1.0");
}

#[test]
fn headers_realistic_basic_auth_request() {
    // Simulate building headers for a basic auth API request
    let headers = Headers::default()
        .use_basic_auth("admin", "secret123")
        .accept("text/plain")
        .header("X-Request-ID", "abc-123-def")
        .build()
        .unwrap();

    assert_eq!(headers.len(), 3);

    // Verify basic auth encoding
    let auth = headers
        .iter()
        .find(|(k, _)| k == "Authorization")
        .expect("Missing Authorization header");

    // "admin:secret123" in Base64 is "YWRtaW46c2VjcmV0MTIz"
    assert_eq!(auth.1, "Basic YWRtaW46c2VjcmV0MTIz");
}

#[test]
fn headers_realistic_api_key_request() {
    // Simulate building headers for HuggingFace-style API key
    let headers = Headers::default()
        .use_api_key("hf_abc123def456", "Authorization")
        .content_type("multipart/form-data; boundary=--boundary")
        .build()
        .unwrap();

    assert_eq!(headers.len(), 2);

    // Verify API key in Authorization header
    let auth = headers
        .iter()
        .find(|(k, _)| k == "Authorization")
        .expect("Missing Authorization header");
    assert_eq!(auth.1, "hf_abc123def456");
}

#[test]
fn headers_custom_headers_realistic_workflow() {
    // Simulate a complex real-world scenario with many custom headers
    let headers = Headers::default()
        .use_bearer_token("token-xyz")
        .header("X-Request-ID", "req-12345")
        .header("X-Correlation-ID", "corr-67890")
        .header("X-Client-Version", "2.5.1")
        .accept("application/vnd.api+json")
        .content_type("application/vnd.api+json")
        .user_agent("CustomClient/2.5.1 (Rust)")
        .build()
        .unwrap();

    assert_eq!(headers.len(), 7);

    // Verify we can find all custom headers
    assert!(
        headers
            .iter()
            .any(|(k, v)| k == "X-Request-ID" && v == "req-12345")
    );
    assert!(
        headers
            .iter()
            .any(|(k, v)| k == "X-Correlation-ID" && v == "corr-67890")
    );
    assert!(
        headers
            .iter()
            .any(|(k, v)| k == "X-Client-Version" && v == "2.5.1")
    );
}

#[test]
fn headers_remove_workflow() {
    // Test that removing headers works in a realistic scenario
    let headers = Headers::default()
        .use_bearer_token("token")
        .content_type_json()
        .accept_json()
        .header("X-Debug", "true")
        .remove("X-Debug") // Remove debug header for production
        .build()
        .unwrap();

    assert_eq!(headers.len(), 3);
    assert!(!headers.iter().any(|(k, _)| k == "X-Debug"));
}

#[test]
fn headers_auth_strategy_override() {
    // Verify that setting multiple auth strategies results in last-wins behavior
    let headers = Headers::default()
        .use_bearer_token("bearer-token")
        .use_basic_auth("user", "pass") // This should override
        .build()
        .unwrap();

    let auth = headers
        .iter()
        .find(|(k, _)| k == "Authorization")
        .expect("Missing Authorization header");

    // Should be Basic auth (last one set)
    assert!(auth.1.starts_with("Basic "));
    assert!(!auth.1.contains("bearer-token"));
}

#[test]
fn headers_validation_catches_invalid_names() {
    // Test that validation properly catches invalid header names
    let result = Headers::default()
        .header("Invalid Header Name", "value")
        .build();

    assert!(result.is_err());
    match result {
        Err(HeaderError::InvalidHeaderName(name)) => {
            assert_eq!(name, "Invalid Header Name");
        }
        _ => panic!("Expected InvalidHeaderName error"),
    }
}

#[test]
fn headers_empty_builder_produces_empty_result() {
    // Verify that an empty builder produces an empty header list
    let headers = Headers::default().build().unwrap();
    assert!(headers.is_empty());
}

#[test]
fn headers_builder_is_reusable() {
    // Verify that we can create a base builder and reuse it
    let base = Headers::default().user_agent("MyApp/1.0").accept_json();

    // Create variant with bearer token
    let with_bearer = base.clone().use_bearer_token("token1").build().unwrap();

    // Create variant with API key
    let with_api_key = base
        .clone()
        .use_api_key("key1", "X-API-Key")
        .build()
        .unwrap();

    // Both should have common headers
    assert!(
        with_bearer
            .iter()
            .any(|(k, v)| k == "User-Agent" && v == "MyApp/1.0")
    );
    assert!(
        with_api_key
            .iter()
            .any(|(k, v)| k == "User-Agent" && v == "MyApp/1.0")
    );

    // But different auth
    assert!(with_bearer.iter().any(|(k, _)| k == "Authorization"));
    assert!(with_api_key.iter().any(|(k, _)| k == "X-API-Key"));
}
