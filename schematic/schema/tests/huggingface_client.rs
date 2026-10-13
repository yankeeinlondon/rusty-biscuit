//! Integration tests for the HuggingFace Hub API client.
//!
//! These tests use wiremock to mock HTTP responses and verify that the
//! generated client makes correct requests, reports non-2xx replies inside the
//! envelope, and runs the client's response hooks on JSON endpoints only.

mod common;

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use common::EnvGuard;
use schematic_schema::huggingface::{CreateRepoBody, WhoAmIRequest};
use schematic_schema::prelude::*;
use schematic_schema::schematic_runtime::{
    AuthMethod, ErrorKind, HookError, Outcome, ResponseMeta,
};
use wiremock::matchers::{body_json, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const TOKEN_VARS: [&str; 3] = ["HF_TOKEN", "HUGGING_FACE_API_KEY", "HF_API_KEY"];

fn client(base_url: &str) -> HuggingFaceHub {
    HuggingFaceHub::new()
        .base_url(base_url)
        .build()
        .expect("client builds")
}

/// `ListModels` hits `GET /models` and decodes an empty list.
#[tokio::test]
async fn list_models_request() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/models"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([])))
        .expect(1)
        .mount(&mock_server)
        .await;

    let response = client(&mock_server.uri())
        .list_models()
        .full()
        .await
        .expect("a response head arrived");

    assert_eq!(response.meta.request.endpoint_id, "ListModels");
    assert!(response.into_result().expect("success").is_empty());
}

/// `GetModel` substitutes its path parameter.
#[tokio::test]
async fn get_model_path_parameter() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/models/bert-base-uncased"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "modelId": "bert-base-uncased",
            "downloads": 1000000
        })))
        .expect(1)
        .mount(&mock_server)
        .await;

    let model = client(&mock_server.uri())
        .get_model("bert-base-uncased")
        .await
        .expect("success");

    assert_eq!(model.model_id, "bert-base-uncased");
    assert_eq!(model.downloads, 1_000_000);
}

/// `CreateRepo` sends its body as JSON.
#[tokio::test]
async fn create_repo_request_body() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/repos/create"))
        .and(body_json(serde_json::json!({
            "name": "my-test-model",
            "private": true
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "url": "https://huggingface.co/test-user/my-test-model"
        })))
        .expect(1)
        .mount(&mock_server)
        .await;

    let body = CreateRepoBody {
        name: "my-test-model".to_string(),
        private: true,
        ..Default::default()
    };
    let repo = client(&mock_server.uri())
        .create_repo(body)
        .await
        .expect("success");

    assert_eq!(repo.url, "https://huggingface.co/test-user/my-test-model");
}

/// The `HF_TOKEN` fallback is sent as a bearer token and reported as its
/// source.
#[tokio::test]
#[serial_test::serial]
async fn auth_header_present_from_env_fallback() {
    let env = EnvGuard::clear(&TOKEN_VARS);
    env.set("HF_TOKEN", "test-token");
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/whoami-v2"))
        .and(header("Authorization", "Bearer test-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "type": "user",
            "name": "testuser"
        })))
        .expect(1)
        .mount(&mock_server)
        .await;

    let response = client(&mock_server.uri())
        .who_am_i()
        .full()
        .await
        .expect("a response head arrived");

    assert_eq!(
        response.meta.request.auth,
        AuthMethod::EnvVar("HF_TOKEN".to_string())
    );
    assert_eq!(response.into_result().expect("success").name, "testuser");
}

/// A 404 is a response, not a transport error: the envelope carries the
/// status and the provider's body unchanged, and an awaited call returns
/// them as `CallError::Api`.
#[tokio::test]
async fn not_found_is_an_envelope_failure_with_status_and_body() {
    let mock_server = MockServer::start().await;
    let body = br#"{"error":"Repository not found"}"#;

    Mock::given(method("GET"))
        .and(path("/models/nonexistent-model"))
        .respond_with(ResponseTemplate::new(404).set_body_raw(body.as_slice(), "application/json"))
        .mount(&mock_server)
        .await;

    let client = client(&mock_server.uri());
    let response = client
        .get_model("nonexistent-model")
        .full()
        .await
        .expect("a 404 is a response, not a transport error");

    assert_eq!(response.meta.status.as_u16(), 404);
    let error = response.error().expect("failure outcome");
    assert!(
        matches!(error.kind, ErrorKind::Undeclared),
        "kind: {}",
        error.kind
    );
    assert_eq!(error.body.as_ref(), body.as_slice());
    assert!(!response.is_decode_failure());

    match client.get_model("nonexistent-model").await {
        Err(CallError::Api { error, meta }) => {
            assert!(matches!(error.kind, ErrorKind::Undeclared), "kind: {}", error.kind);
            assert_eq!(error.body.as_ref(), body.as_slice());
            assert_eq!(meta.status.as_u16(), 404);
            assert_eq!(meta.request.endpoint_id, "GetModel");
        }
        other => panic!("expected CallError::Api, got {other:?}"),
    }
}

/// The client-global `pre_response_json` hook reshapes a successful JSON body
/// before it is decoded into the endpoint's success type.
#[tokio::test]
async fn pre_response_json_reshapes_body_before_decode() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/whoami-v2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "envelope": { "name": "wrapped-user" }
        })))
        .mount(&mock_server)
        .await;

    let client = HuggingFaceHub::new()
        .base_url(mock_server.uri())
        .pre_response_json(|_meta: &ResponseMeta, mut value: serde_json::Value| {
            value
                .get_mut("envelope")
                .map(serde_json::Value::take)
                .ok_or(HookError::new())
        })
        .build()
        .expect("client builds");
    let user = client
        .who_am_i()
        .await
        .expect("the hook made the body decodable");

    assert_eq!(user.name, "wrapped-user");
}

/// A `mutate_response::<WhoAmIRequest>` mutator runs on `WhoAmI` and not on
/// `GetUser`, although both decode into the same `UserInfo` type.
#[tokio::test]
async fn mutate_response_runs_only_for_its_endpoint() {
    let mock_server = MockServer::start().await;
    let user = serde_json::json!({ "name": "someone", "fullname": "Original Name" });

    Mock::given(method("GET"))
        .and(path("/whoami-v2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(user.clone()))
        .mount(&mock_server)
        .await;
    Mock::given(method("GET"))
        .and(path("/users/someone"))
        .respond_with(ResponseTemplate::new(200).set_body_json(user))
        .mount(&mock_server)
        .await;

    let client = HuggingFaceHub::new()
        .base_url(mock_server.uri())
        .mutate_response::<WhoAmIRequest, _>(|meta, user| {
            assert_eq!(meta.request.endpoint_id, "WhoAmI");
            user.fullname = Some("Mutated Name".to_string());
            Ok(())
        })
        .build()
        .expect("client builds");

    let whoami = client.who_am_i().await.expect("success");
    let other = client.get_user("someone").await.expect("success");

    assert_eq!(whoami.fullname.as_deref(), Some("Mutated Name"));
    assert_eq!(other.fullname.as_deref(), Some("Original Name"));
}

/// A hook that refuses the body turns the outcome into `ErrorKind::Hook` and
/// keeps the original body bytes; awaited directly, that is a
/// `CallError::Api` although the status is 200.
#[tokio::test]
async fn refusing_hook_yields_hook_failure_with_original_body() {
    let mock_server = MockServer::start().await;
    let body = br#"{"name":"someone"}"#;

    Mock::given(method("GET"))
        .and(path("/whoami-v2"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(body.as_slice(), "application/json"))
        .mount(&mock_server)
        .await;

    let client = HuggingFaceHub::new()
        .base_url(mock_server.uri())
        .mutate_response::<WhoAmIRequest, _>(|_, _| Err(HookError::new()))
        .build()
        .expect("client builds");
    let response = client
        .who_am_i()
        .full()
        .await
        .expect("a response head arrived");

    assert_eq!(response.meta.status.as_u16(), 200);
    match client.who_am_i().await {
        Err(CallError::Api { error, meta }) => {
            assert!(matches!(error.kind, ErrorKind::Hook), "kind: {}", error.kind);
            assert_eq!(error.body.as_ref(), body.as_slice());
            assert_eq!(meta.status.as_u16(), 200);
        }
        other => panic!("expected CallError::Api, got {other:?}"),
    }
    match &response.outcome {
        Outcome::Failure(error) => {
            assert!(
                matches!(error.kind, ErrorKind::Hook),
                "kind: {}",
                error.kind
            );
            assert_eq!(error.body.as_ref(), body.as_slice());
        }
        Outcome::Success(_) => panic!("a refused body must not be a success"),
    }
}

/// The global hook is skipped for a non-2xx reply and for a text endpoint.
#[tokio::test]
async fn pre_response_json_skips_error_replies_and_text_endpoints() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/models/missing"))
        .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({"error": "x"})))
        .mount(&mock_server)
        .await;
    Mock::given(method("GET"))
        .and(path("/models/some-model/resolve/main/README.md"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(r#"{"looks":"like json"}"#, "application/json"),
        )
        .mount(&mock_server)
        .await;

    let calls = Arc::new(AtomicUsize::new(0));
    let counted = Arc::clone(&calls);
    let client = HuggingFaceHub::new()
        .base_url(mock_server.uri())
        .pre_response_json(move |_: &ResponseMeta, value: serde_json::Value| {
            counted.fetch_add(1, Ordering::SeqCst);
            Ok(value)
        })
        .build()
        .expect("client builds");

    let missing = client
        .get_model("missing")
        .full()
        .await
        .expect("a response head arrived");
    let readme = client
        .get_model_readme("some-model", "main")
        .await
        .expect("a text endpoint decodes any content type");

    assert_eq!(missing.meta.status.as_u16(), 404);
    assert_eq!(readme, r#"{"looks":"like json"}"#);
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}
