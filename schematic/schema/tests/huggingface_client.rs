//! Integration tests for the HuggingFace Hub API client.
//!
//! These tests use wiremock to mock HTTP responses and verify
//! that the generated client makes correct requests.

use schematic_schema::huggingfacehub::{HuggingFaceHub, HuggingFaceHubRequest};
use schematic_schema::shared::SchematicError;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Helper to set up environment for tests.
fn setup_test_env() {
    // SAFETY: Tests run in isolation, setting env vars is safe here
    unsafe {
        std::env::set_var("HF_TOKEN", "test-token");
    }
}

/// Test that ListModels request is formatted correctly.
#[tokio::test]
async fn test_list_models_request() {
    setup_test_env();
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/models"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([])))
        .mount(&mock_server)
        .await;

    let client = HuggingFaceHub::with_base_url(&mock_server.uri());

    let request = HuggingFaceHubRequest::ListModels(Default::default());
    let result: Result<Vec<serde_json::Value>, _> = client.request(request).await;

    assert!(result.is_ok(), "Request failed: {:?}", result.err());
}

/// Test that GetModel correctly substitutes path parameter.
#[tokio::test]
async fn test_get_model_path_parameter() {
    setup_test_env();
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/models/bert-base-uncased"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "modelId": "bert-base-uncased",
            "downloads": 1000000
        })))
        .mount(&mock_server)
        .await;

    let client = HuggingFaceHub::with_base_url(&mock_server.uri());

    let mut request: schematic_schema::huggingfacehub::GetModelRequest = Default::default();
    request.repo_id = "bert-base-uncased".to_string();

    let request = HuggingFaceHubRequest::GetModel(request);
    let result: Result<serde_json::Value, _> = client.request(request).await;

    assert!(result.is_ok(), "Request failed: {:?}", result.err());
}

/// Test that CreateRepo sends correct request body.
#[tokio::test]
async fn test_create_repo_request_body() {
    use schematic_schema::huggingfacehub::{CreateRepoBody, CreateRepoRequest};

    setup_test_env();
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/repos/create"))
        .and(wiremock::matchers::body_json(serde_json::json!({
            "name": "my-test-model",
            "private": true
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "url": "https://huggingface.co/test-user/my-test-model"
        })))
        .mount(&mock_server)
        .await;

    let client = HuggingFaceHub::with_base_url(&mock_server.uri());

    let body = CreateRepoBody {
        name: "my-test-model".to_string(),
        private: true,
        ..Default::default()
    };

    let request = HuggingFaceHubRequest::CreateRepo(CreateRepoRequest { body });
    let result: Result<serde_json::Value, _> = client.request(request).await;

    assert!(result.is_ok(), "Request failed: {:?}", result.err());
}

/// Test that Authorization header is present when API key is set.
#[tokio::test]
async fn test_auth_header_present() {
    setup_test_env();
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/whoami-v2"))
        .and(header("Authorization", "Bearer test-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "type": "user",
            "name": "testuser"
        })))
        .mount(&mock_server)
        .await;

    let client = HuggingFaceHub::with_base_url(&mock_server.uri());

    let request = HuggingFaceHubRequest::WhoAmI(Default::default());
    let result: Result<serde_json::Value, _> = client.request(request).await;

    assert!(result.is_ok(), "Request failed: {:?}", result.err());
}

/// Test that API errors are properly propagated.
#[tokio::test]
async fn test_api_error_handling() {
    setup_test_env();
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/models/nonexistent-model"))
        .respond_with(
            ResponseTemplate::new(404)
                .set_body_json(serde_json::json!({"error": "Repository not found"})),
        )
        .mount(&mock_server)
        .await;

    let client = HuggingFaceHub::with_base_url(&mock_server.uri());

    let mut request: schematic_schema::huggingfacehub::GetModelRequest = Default::default();
    request.repo_id = "nonexistent-model".to_string();

    let request = HuggingFaceHubRequest::GetModel(request);
    let result: Result<serde_json::Value, _> = client.request(request).await;

    assert!(result.is_err());
    if let Err(SchematicError::ApiError { status, .. }) = result {
        assert_eq!(status, 404);
    } else {
        panic!("Expected ApiError, got {:?}", result);
    }
}
