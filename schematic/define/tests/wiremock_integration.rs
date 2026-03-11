//! Integration tests for Headers system using wiremock to verify actual HTTP header behavior.
//!
//! These tests verify that:
//! - Auth strategies produce correct Authorization headers
//! - Custom headers are sent correctly
//! - ENV fallback works in real HTTP scenarios
//! - Header overrides work as expected

use schematic_define::{ApiKeyEnv, EnvList, EnvMapping, Headers};
use std::env;
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{header, method, path},
};

// Helper function to make HTTP requests with headers
async fn make_request(url: &str, headers: Headers) -> Result<reqwest::Response, reqwest::Error> {
    let client = reqwest::Client::new();
    let mut builder = client.get(url);

    // Build headers and apply them
    let header_vec = headers.build().expect("headers should build successfully");
    for (name, value) in header_vec {
        builder = builder.header(name, value);
    }

    builder.send().await
}

#[tokio::test]
async fn test_bearer_auth_with_direct_token() {
    let mock_server = MockServer::start().await;

    // Expect Authorization: Bearer test-token-123
    Mock::given(method("GET"))
        .and(path("/api/test"))
        .and(header("Authorization", "Bearer test-token-123"))
        .respond_with(ResponseTemplate::new(200).set_body_string("success"))
        .mount(&mock_server)
        .await;

    let headers = Headers::default().use_bearer_token("test-token-123");

    let response = make_request(&format!("{}/api/test", mock_server.uri()), headers)
        .await
        .expect("request succeeded");

    assert_eq!(response.status(), 200);
    assert_eq!(response.text().await.unwrap(), "success");
}

#[tokio::test]
#[serial_test::serial]
async fn test_bearer_auth_with_env_fallback() {
    let mock_server = MockServer::start().await;

    // Set ENV variable
    unsafe {
        env::set_var("TEST_BEARER_TOKEN", "env-token-456");
    }

    // Expect Authorization: Bearer env-token-456
    Mock::given(method("GET"))
        .and(path("/api/test"))
        .and(header("Authorization", "Bearer env-token-456"))
        .respond_with(ResponseTemplate::new(200).set_body_string("success"))
        .mount(&mock_server)
        .await;

    let mapping = EnvMapping {
        bearer_token: Some(EnvList::single("TEST_BEARER_TOKEN")),
        basic_user: None,
        basic_pass: None,
        api_key: None,
    ..Default::default()
    };

    let headers = Headers::default().with_env_mapping(mapping).from_env();

    let response = make_request(&format!("{}/api/test", mock_server.uri()), headers)
        .await
        .expect("request succeeded");

    assert_eq!(response.status(), 200);

    // Clean up
    unsafe {
        env::remove_var("TEST_BEARER_TOKEN");
    }
}

#[tokio::test]
#[serial_test::serial]
async fn test_bearer_auth_direct_overrides_env() {
    let mock_server = MockServer::start().await;

    // Set ENV variable (should be ignored because direct token takes precedence)
    unsafe {
        env::set_var("TEST_BEARER_TOKEN", "env-token-ignored");
    }

    // Expect Authorization: Bearer direct-token-wins
    Mock::given(method("GET"))
        .and(path("/api/test"))
        .and(header("Authorization", "Bearer direct-token-wins"))
        .respond_with(ResponseTemplate::new(200).set_body_string("success"))
        .mount(&mock_server)
        .await;

    let mapping = EnvMapping {
        bearer_token: Some(EnvList::single("TEST_BEARER_TOKEN")),
        basic_user: None,
        basic_pass: None,
        api_key: None,
    ..Default::default()
    };

    // Direct token set after env mapping, so direct wins
    let headers = Headers::default()
        .with_env_mapping(mapping)
        .use_bearer_token("direct-token-wins");

    let response = make_request(&format!("{}/api/test", mock_server.uri()), headers)
        .await
        .expect("request succeeded");

    assert_eq!(response.status(), 200);

    // Clean up
    unsafe {
        env::remove_var("TEST_BEARER_TOKEN");
    }
}

#[tokio::test]
async fn test_api_key_header_with_direct_key() {
    let mock_server = MockServer::start().await;

    // Expect X-API-Key: my-secret-key
    Mock::given(method("GET"))
        .and(path("/api/test"))
        .and(header("X-API-Key", "my-secret-key"))
        .respond_with(ResponseTemplate::new(200).set_body_string("success"))
        .mount(&mock_server)
        .await;

    let headers = Headers::default().use_api_key("my-secret-key", "X-API-Key");

    let response = make_request(&format!("{}/api/test", mock_server.uri()), headers)
        .await
        .expect("request succeeded");

    assert_eq!(response.status(), 200);
}

#[tokio::test]
#[serial_test::serial]
async fn test_api_key_header_with_env_fallback() {
    let mock_server = MockServer::start().await;

    // Set ENV variable
    unsafe {
        env::set_var("TEST_API_KEY", "env-api-key-789");
    }

    // Expect X-API-Key: env-api-key-789
    Mock::given(method("GET"))
        .and(path("/api/test"))
        .and(header("X-API-Key", "env-api-key-789"))
        .respond_with(ResponseTemplate::new(200).set_body_string("success"))
        .mount(&mock_server)
        .await;

    let mapping = EnvMapping {
        bearer_token: None,
        basic_user: None,
        basic_pass: None,
        api_key: Some(ApiKeyEnv {
            names: EnvList::single("TEST_API_KEY"),
            header: "X-API-Key".to_string(),
        }),
    ..Default::default()
    };

    let headers = Headers::default().with_env_mapping(mapping).from_env();

    let response = make_request(&format!("{}/api/test", mock_server.uri()), headers)
        .await
        .expect("request succeeded");

    assert_eq!(response.status(), 200);

    // Clean up
    unsafe {
        env::remove_var("TEST_API_KEY");
    }
}

#[tokio::test]
async fn test_basic_auth() {
    let mock_server = MockServer::start().await;

    // Expect Authorization: Basic dXNlcm5hbWU6cGFzc3dvcmQ= (username:password in base64)
    Mock::given(method("GET"))
        .and(path("/api/test"))
        .and(header("Authorization", "Basic dXNlcm5hbWU6cGFzc3dvcmQ="))
        .respond_with(ResponseTemplate::new(200).set_body_string("success"))
        .mount(&mock_server)
        .await;

    let headers = Headers::default().use_basic_auth("username", "password");

    let response = make_request(&format!("{}/api/test", mock_server.uri()), headers)
        .await
        .expect("request succeeded");

    assert_eq!(response.status(), 200);
}

#[tokio::test]
async fn test_custom_header() {
    let mock_server = MockServer::start().await;

    // Expect X-Custom-Header: custom-value
    Mock::given(method("GET"))
        .and(path("/api/test"))
        .and(header("X-Custom-Header", "custom-value"))
        .respond_with(ResponseTemplate::new(200).set_body_string("success"))
        .mount(&mock_server)
        .await;

    let headers = Headers::default().header("X-Custom-Header", "custom-value");

    let response = make_request(&format!("{}/api/test", mock_server.uri()), headers)
        .await
        .expect("request succeeded");

    assert_eq!(response.status(), 200);
}

#[tokio::test]
async fn test_multiple_custom_headers() {
    let mock_server = MockServer::start().await;

    // Expect multiple headers
    Mock::given(method("GET"))
        .and(path("/api/test"))
        .and(header("X-Client-ID", "client-123"))
        .and(header("X-Request-ID", "req-456"))
        .and(header("X-Version", "v1"))
        .respond_with(ResponseTemplate::new(200).set_body_string("success"))
        .mount(&mock_server)
        .await;

    let headers = Headers::default()
        .header("X-Client-ID", "client-123")
        .header("X-Request-ID", "req-456")
        .header("X-Version", "v1");

    let response = make_request(&format!("{}/api/test", mock_server.uri()), headers)
        .await
        .expect("request succeeded");

    assert_eq!(response.status(), 200);
}

#[tokio::test]
async fn test_bearer_and_custom_headers_combined() {
    let mock_server = MockServer::start().await;

    // Expect both Authorization and custom headers
    Mock::given(method("GET"))
        .and(path("/api/test"))
        .and(header("Authorization", "Bearer combo-token"))
        .and(header("X-Client-ID", "client-789"))
        .respond_with(ResponseTemplate::new(200).set_body_string("success"))
        .mount(&mock_server)
        .await;

    let headers = Headers::default()
        .use_bearer_token("combo-token")
        .header("X-Client-ID", "client-789");

    let response = make_request(&format!("{}/api/test", mock_server.uri()), headers)
        .await
        .expect("request succeeded");

    assert_eq!(response.status(), 200);
}

#[tokio::test]
#[serial_test::serial]
async fn test_env_mapping_with_primary_and_fallback() {
    let mock_server = MockServer::start().await;

    // Set fallback ENV variable only (primary is missing)
    unsafe {
        env::set_var("FALLBACK_TOKEN", "fallback-value");
    }

    // Expect Authorization: Bearer fallback-value
    Mock::given(method("GET"))
        .and(path("/api/test"))
        .and(header("Authorization", "Bearer fallback-value"))
        .respond_with(ResponseTemplate::new(200).set_body_string("success"))
        .mount(&mock_server)
        .await;

    let env_list = EnvList::from_strs(&["PRIMARY_TOKEN", "FALLBACK_TOKEN"]);

    let mapping = EnvMapping {
        bearer_token: Some(env_list),
        basic_user: None,
        basic_pass: None,
        api_key: None,
    ..Default::default()
    };

    let headers = Headers::default().with_env_mapping(mapping).from_env();

    let response = make_request(&format!("{}/api/test", mock_server.uri()), headers)
        .await
        .expect("request succeeded");

    assert_eq!(response.status(), 200);

    // Clean up
    unsafe {
        env::remove_var("FALLBACK_TOKEN");
    }
}

#[tokio::test]
#[serial_test::serial]
async fn test_env_mapping_primary_wins_over_fallback() {
    let mock_server = MockServer::start().await;

    // Set both primary and fallback (primary should win)
    unsafe {
        env::set_var("PRIMARY_TOKEN", "primary-wins");
        env::set_var("FALLBACK_TOKEN", "fallback-ignored");
    }

    // Expect Authorization: Bearer primary-wins
    Mock::given(method("GET"))
        .and(path("/api/test"))
        .and(header("Authorization", "Bearer primary-wins"))
        .respond_with(ResponseTemplate::new(200).set_body_string("success"))
        .mount(&mock_server)
        .await;

    let env_list = EnvList::from_strs(&["PRIMARY_TOKEN", "FALLBACK_TOKEN"]);

    let mapping = EnvMapping {
        bearer_token: Some(env_list),
        basic_user: None,
        basic_pass: None,
        api_key: None,
    ..Default::default()
    };

    let headers = Headers::default().with_env_mapping(mapping).from_env();

    let response = make_request(&format!("{}/api/test", mock_server.uri()), headers)
        .await
        .expect("request succeeded");

    assert_eq!(response.status(), 200);

    // Clean up
    unsafe {
        env::remove_var("PRIMARY_TOKEN");
    }
    unsafe {
        env::remove_var("FALLBACK_TOKEN");
    }
}

#[tokio::test]
async fn test_unauthorized_when_bearer_missing() {
    let mock_server = MockServer::start().await;

    // Expect request WITHOUT Authorization header to fail
    Mock::given(method("GET"))
        .and(path("/api/test"))
        .and(header("Authorization", "Bearer valid-token"))
        .respond_with(ResponseTemplate::new(200).set_body_string("success"))
        .mount(&mock_server)
        .await;

    // Different mock for unauthorized requests (no Authorization header)
    Mock::given(method("GET"))
        .and(path("/api/test"))
        .respond_with(ResponseTemplate::new(401).set_body_string("unauthorized"))
        .mount(&mock_server)
        .await;

    let headers = Headers::default();

    let response = make_request(&format!("{}/api/test", mock_server.uri()), headers)
        .await
        .expect("request succeeded");

    assert_eq!(response.status(), 401);
}

#[tokio::test]
#[serial_test::serial]
async fn test_api_key_env_with_env_mapping() {
    let mock_server = MockServer::start().await;

    // Set ENV variable
    unsafe {
        env::set_var("TEST_API_KEY_PRIMARY", "mapped-key-value");
    }

    // Expect X-API-Key: mapped-key-value
    Mock::given(method("GET"))
        .and(path("/api/test"))
        .and(header("X-API-Key", "mapped-key-value"))
        .respond_with(ResponseTemplate::new(200).set_body_string("success"))
        .mount(&mock_server)
        .await;

    let mapping = EnvMapping {
        bearer_token: None,
        basic_user: None,
        basic_pass: None,
        api_key: Some(ApiKeyEnv {
            names: EnvList::single("TEST_API_KEY_PRIMARY"),
            header: "X-API-Key".to_string(),
        }),
    ..Default::default()
    };

    let headers = Headers::default().with_env_mapping(mapping).from_env();

    let response = make_request(&format!("{}/api/test", mock_server.uri()), headers)
        .await
        .expect("request succeeded");

    assert_eq!(response.status(), 200);

    // Clean up
    unsafe {
        env::remove_var("TEST_API_KEY_PRIMARY");
    }
}

#[tokio::test]
async fn test_sensitive_string_debug_redaction() {
    // This test verifies that SensitiveString redaction works correctly
    // by checking that the Debug impl doesn't expose the secret

    use schematic_define::SensitiveString;

    let sensitive = SensitiveString::new("super-secret-token".to_string());
    let debug = format!("{:?}", sensitive);

    assert!(!debug.contains("super-secret-token"));
    assert!(debug.contains("***") || debug.contains("SensitiveString"));
}

#[tokio::test]
async fn test_headers_builder_chaining() {
    // Verify that builder methods can be chained fluently
    let headers = Headers::default()
        .use_bearer_token("token-1")
        .header("X-Custom", "value-1")
        .header("X-Another", "value-2");

    let header_vec = headers.build().expect("headers build successfully");

    assert_eq!(header_vec.len(), 3); // Bearer + 2 custom headers

    // Check that expected headers are present
    let header_names: Vec<String> = header_vec.iter().map(|(name, _)| name.clone()).collect();
    assert!(header_names.contains(&"Authorization".to_string()));
    assert!(header_names.contains(&"X-Custom".to_string()));
    assert!(header_names.contains(&"X-Another".to_string()));
}

#[tokio::test]
#[serial_test::serial]
async fn test_complex_scenario_with_env_mapping() {
    let mock_server = MockServer::start().await;

    // Set up complex ENV scenario
    unsafe {
        env::set_var("OPENAI_API_KEY", "openai-key-123");
    }
    unsafe {
        env::set_var("CLIENT_ID", "client-456");
    }

    // Expect both headers
    Mock::given(method("GET"))
        .and(path("/api/test"))
        .and(header("Authorization", "Bearer openai-key-123"))
        .and(header("X-Client-ID", "client-456"))
        .respond_with(ResponseTemplate::new(200).set_body_string("success"))
        .mount(&mock_server)
        .await;

    let mapping = EnvMapping {
        bearer_token: Some(EnvList::from_strs(&["OPENAI_API_KEY", "OPENAI_KEY"])),
        basic_user: None,
        basic_pass: None,
        api_key: Some(ApiKeyEnv {
            names: EnvList::single("CLIENT_ID"),
            header: "X-Client-ID".to_string(),
        }),
    ..Default::default()
    };

    let headers = Headers::default().with_env_mapping(mapping).from_env();

    let response = make_request(&format!("{}/api/test", mock_server.uri()), headers)
        .await
        .expect("request succeeded");

    assert_eq!(response.status(), 200);

    // Clean up
    unsafe {
        env::remove_var("OPENAI_API_KEY");
    }
    unsafe {
        env::remove_var("CLIENT_ID");
    }
}
