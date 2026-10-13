//! Integration tests for the Artificial Analysis API clients.
//!
//! These tests use [`wiremock`] to verify the user-observable HTTP behavior
//! of the generated `ArtificialAnalysisData` and `ArtificialAnalysisCritPt`
//! clients without touching the network. They cover query-string serialization,
//! credential precedence (client credential over the env-var fallback), the
//! anonymous send when no credential resolves, and JSON body serialization for
//! the CritPt evaluation endpoint (including `skip_serializing_if` on
//! `batch_metadata`).
//!
//! Tests that read or write `ARTIFICIAL_ANALYSIS_API_KEY` hold an
//! [`EnvGuard`] and are serialized via `serial_test::serial`.

mod common;

use common::EnvGuard;
use schematic_schema::artificial_analysis::{
    CritPtEvaluateBody, CritPtEvaluateResponse, CritPtMessage, CritPtSubmission,
    EvaluateCritPtRequest, ListImageEditingModelsRequest, ListLlmModelsRequest,
    ListTextToImageModelsRequest, LlmModelsResponse, MediaModelsResponse,
};
use schematic_schema::prelude::*;
use schematic_schema::schematic_runtime::{AuthMethod, ErrorKind, NoDeclaredError, Outcome};
use wiremock::matchers::{body_json, header, method, path, query_param};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

const ENV_KEY: &str = "ARTIFICIAL_ANALYSIS_API_KEY";

fn data_client(base_url: &str) -> ArtificialAnalysisData {
    ArtificialAnalysisData::new()
        .base_url(base_url)
        .build()
        .expect("client builds")
}

fn critpt_client(base_url: &str) -> ArtificialAnalysisCritPt {
    ArtificialAnalysisCritPt::new()
        .base_url(base_url)
        .build()
        .expect("client builds")
}

/// The decoded success value of a send, failing the test on a transport
/// error or a failure outcome.
fn ok<T>(sent: Result<Response<T, NoDeclaredError>, TransportError>) -> T {
    let response = sent.expect("a response head arrived");
    let status = response.meta.status;
    match response.into_result() {
        Ok(value) => value,
        Err(error) => panic!("expected success, got {error} with status {status}"),
    }
}

/// Minimal valid `LlmModelsResponse` JSON: empty `data`, populated
/// `prompt_options`. Required fields: `status`, `prompt_options`, `data`.
fn llm_models_response_empty() -> serde_json::Value {
    serde_json::json!({
        "status": 200,
        "prompt_options": {
            "parallel_queries": 1,
            "prompt_length": "medium"
        },
        "data": []
    })
}

/// Minimal valid `MediaModelsResponse` JSON: empty `data`. The optional
/// `include_categories` field is included only when the caller passes it.
fn media_models_response_empty(include_categories: Option<bool>) -> serde_json::Value {
    let mut value = serde_json::json!({ "status": 200, "data": [] });
    if let Some(include) = include_categories {
        value.as_object_mut().expect("object").insert(
            "include_categories".to_string(),
            serde_json::Value::Bool(include),
        );
    }
    value
}

/// Minimal valid `CritPtEvaluateResponse` JSON. Required fields: `accuracy`,
/// `timeout_rate`, `server_timeout_count`, `judge_error_count`.
fn critpt_evaluate_response_zero() -> serde_json::Value {
    serde_json::json!({
        "accuracy": 0.0,
        "timeout_rate": 0.0,
        "server_timeout_count": 0,
        "judge_error_count": 0
    })
}

/// Realistic non-zero `CritPtEvaluateResponse` JSON. Uses exactly-representable
/// f64 values (`0.5`, `0.25`) to avoid `clippy::float_cmp` issues in
/// `assert_eq!` comparisons against decoded fields.
fn critpt_evaluate_response_realistic() -> serde_json::Value {
    serde_json::json!({
        "accuracy": 0.5,
        "timeout_rate": 0.25,
        "server_timeout_count": 3,
        "judge_error_count": 1
    })
}

/// Valid `LlmModelsResponse` JSON populated with a single fully-formed
/// `LlmModel`, including every required sub-field on `ModelCreator`,
/// `LlmEvaluations`, and `LlmPricing`.
fn llm_models_response_one_populated() -> serde_json::Value {
    serde_json::json!({
        "status": 200,
        "prompt_options": {
            "parallel_queries": 4,
            "prompt_length": "medium"
        },
        "data": [
            {
                "id": "gpt-test",
                "name": "GPT Test",
                "slug": "gpt-test",
                "model_creator": {
                    "id": "creator-1",
                    "name": "Test Lab",
                    "slug": "test-lab"
                },
                "evaluations": {
                    "artificial_analysis_intelligence_index": 0.5,
                    "artificial_analysis_coding_index": 0.5,
                    "artificial_analysis_math_index": 0.5,
                    "mmlu_pro": 0.5,
                    "gpqa": 0.5,
                    "hle": 0.5,
                    "livecodebench": 0.5,
                    "scicode": 0.5,
                    "math_500": 0.5,
                    "aime": 0.5
                },
                "pricing": {
                    "price_1m_blended_3_to_1": 0.5,
                    "price_1m_input_tokens": 0.25,
                    "price_1m_output_tokens": 0.5
                },
                "median_output_tokens_per_second": 100.0,
                "median_time_to_first_token_seconds": 0.5
            }
        ]
    })
}

/// Valid `MediaModelsResponse` JSON populated with one `MediaModel` whose
/// `categories` array carries a single `CategoryBreakdown`, including the
/// optional `format_category` field. `include_categories` is set to `true`.
fn media_models_response_one_populated_categories() -> serde_json::Value {
    serde_json::json!({
        "status": 200,
        "include_categories": true,
        "data": [
            {
                "id": "media-test",
                "name": "Media Test",
                "slug": "media-test",
                "model_creator": {
                    "id": "creator-1",
                    "name": "Test Lab",
                    "slug": "test-lab"
                },
                "elo": 1500,
                "rank": 1,
                "ci95": "+/-10",
                "appearances": 200,
                "release_date": "2026-01-01",
                "categories": [
                    {
                        "style_category": "photorealistic",
                        "subject_matter_category": "people",
                        "format_category": "portrait",
                        "elo": 1510,
                        "ci95": "+/-12",
                        "appearances": 50
                    }
                ]
            }
        ]
    })
}

/// Valid `MediaModelsResponse` JSON populated with one `MediaModel` that
/// **omits** the optional `release_date` and `categories` keys. Used to verify
/// `Option<...>` decode-from-absent behaviour.
fn media_models_response_one_without_optionals() -> serde_json::Value {
    serde_json::json!({
        "status": 200,
        "data": [
            {
                "id": "edit-test",
                "name": "Edit Test",
                "slug": "edit-test",
                "model_creator": {
                    "id": "creator-2",
                    "name": "Other Lab",
                    "slug": "other-lab"
                },
                "elo": 1400,
                "rank": 2,
                "ci95": "+/-15",
                "appearances": 80
            }
        ]
    })
}

/// Data API exercise of `ListTextToImageModelsRequest` with the
/// `include_categories=true` query parameter.
#[tokio::test]
async fn list_text_to_image_models_emits_include_categories_query() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/data/media/text-to-image"))
        .and(query_param("include_categories", "true"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(media_models_response_empty(Some(true))),
        )
        .mount(&mock_server)
        .await;

    let client = data_client(&mock_server.uri());
    let request = ListTextToImageModelsRequest::default().with_include_categories(true);
    let body: MediaModelsResponse = ok(client.send(request, &CallOptions::default()).await);

    assert_eq!(body.status, 200);
    assert_eq!(body.include_categories, Some(true));
    assert!(body.data.is_empty());
}

/// `ListLlmModelsRequest::default()` hits `GET /data/llms/models` with no
/// query string at all.
#[tokio::test]
async fn list_llm_models_uses_no_query_string() {
    let mock_server = MockServer::start().await;

    // wiremock has no built-in "no query string" matcher.
    let no_query = |request: &Request| -> bool { request.url.query().is_none() };

    Mock::given(method("GET"))
        .and(path("/data/llms/models"))
        .and(no_query)
        .respond_with(ResponseTemplate::new(200).set_body_json(llm_models_response_empty()))
        .mount(&mock_server)
        .await;

    let client = data_client(&mock_server.uri());
    let body: LlmModelsResponse = ok(client.send(ListLlmModelsRequest::default(), &CallOptions::default()).await);

    assert_eq!(body.status, 200);
    assert_eq!(body.prompt_options.parallel_queries, 1);
    assert_eq!(body.prompt_options.prompt_length, "medium");
    assert!(body.data.is_empty());
}

/// With no client credential and `ARTIFICIAL_ANALYSIS_API_KEY` unset, the
/// request goes out without `x-api-key`, and the provider's 401 comes back
/// as an envelope failure with its status and raw body.
#[tokio::test]
#[serial_test::serial]
async fn missing_credential_sends_anonymously_and_returns_the_401() {
    let _env = EnvGuard::clear(&[ENV_KEY]);
    let mock_server = MockServer::start().await;
    let rejection = br#"{"error":"missing x-api-key"}"#;

    Mock::given(method("GET"))
        .and(path("/data/llms/models"))
        .respond_with(
            ResponseTemplate::new(401).set_body_raw(rejection.as_slice(), "application/json"),
        )
        .expect(1)
        .mount(&mock_server)
        .await;

    let client = data_client(&mock_server.uri());
    let response = client
        .send(ListLlmModelsRequest::default(), &CallOptions::default())
        .await
        .expect("a 401 is a response, not a transport error");

    assert_eq!(response.meta.status.as_u16(), 401);
    assert_eq!(response.meta.request.auth, AuthMethod::Anonymous);
    match &response.outcome {
        Outcome::Failure(error) => {
            assert!(
                matches!(error.kind, ErrorKind::Undeclared),
                "kind: {}",
                error.kind
            );
            assert_eq!(error.body.as_ref(), rejection.as_slice());
        }
        Outcome::Success(_) => panic!("a 401 must not decode as success"),
    }
    let received = mock_server
        .received_requests()
        .await
        .expect("recording is on");
    assert!(received[0].headers.get("x-api-key").is_none());
}

/// A client-level `Credential::api_key` beats a non-empty env-var fallback.
#[tokio::test]
#[serial_test::serial]
async fn client_api_key_overrides_env_fallback() {
    let env = EnvGuard::clear(&[ENV_KEY]);
    env.set(ENV_KEY, "env-key");
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/data/llms/models"))
        .and(header("x-api-key", "explicit-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(llm_models_response_empty()))
        .expect(1)
        .mount(&mock_server)
        .await;

    let client = ArtificialAnalysisData::new()
        .base_url(mock_server.uri())
        .credentials(Credential::api_key("explicit-key"))
        .build()
        .expect("client builds");
    let response = client
        .send(ListLlmModelsRequest::default(), &CallOptions::default())
        .await
        .expect("a response head arrived");

    assert_eq!(response.meta.request.auth, AuthMethod::ClientLevel);
    assert_eq!(response.into_result().expect("success").status, 200);
}

/// The env-var fallback is consulted when no client credential is set.
#[tokio::test]
#[serial_test::serial]
async fn env_api_key_is_used_when_no_explicit_key() {
    let env = EnvGuard::clear(&[ENV_KEY]);
    env.set(ENV_KEY, "env-key");
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/data/llms/models"))
        .and(header("x-api-key", "env-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(llm_models_response_empty()))
        .expect(1)
        .mount(&mock_server)
        .await;

    let client = data_client(&mock_server.uri());
    let response = client
        .send(ListLlmModelsRequest::default(), &CallOptions::default())
        .await
        .expect("a response head arrived");

    assert_eq!(
        response.meta.request.auth,
        AuthMethod::EnvVar(ENV_KEY.to_string())
    );
    assert_eq!(response.into_result().expect("success").status, 200);
}

/// The CritPt API serializes the full evaluation body (including populated
/// `batch_metadata`) exactly as expected.
#[tokio::test]
async fn critpt_evaluate_serializes_populated_batch_metadata() {
    let mock_server = MockServer::start().await;

    let expected = serde_json::json!({
        "submissions": [
            {
                "problem_id": "p-1",
                "generated_code": "print('hi')",
                "model": "test-model",
                "generation_config": { "temperature": 0.3, "top_p": 0.9 },
                "messages": [
                    { "role": "user", "content": "solve this" }
                ]
            }
        ],
        "batch_metadata": { "run_id": "abc" }
    });

    Mock::given(method("POST"))
        .and(path("/critpt/evaluate"))
        .and(body_json(&expected))
        .respond_with(ResponseTemplate::new(200).set_body_json(critpt_evaluate_response_zero()))
        .mount(&mock_server)
        .await;

    let client = critpt_client(&mock_server.uri());

    let body = CritPtEvaluateBody {
        submissions: vec![CritPtSubmission {
            problem_id: "p-1".to_string(),
            generated_code: "print('hi')".to_string(),
            model: "test-model".to_string(),
            generation_config: serde_json::json!({ "temperature": 0.3, "top_p": 0.9 }),
            messages: Some(vec![CritPtMessage {
                role: "user".to_string(),
                content: "solve this".to_string(),
            }]),
        }],
        batch_metadata: serde_json::json!({ "run_id": "abc" }),
    };

    let body: CritPtEvaluateResponse = ok(client.send(EvaluateCritPtRequest::new(body), &CallOptions::default()).await);

    assert_eq!(body.accuracy, 0.0);
    assert_eq!(body.timeout_rate, 0.0);
    assert_eq!(body.server_timeout_count, 0);
    assert_eq!(body.judge_error_count, 0);
}

/// The CritPt API drops `batch_metadata` from the wire body when it is
/// `serde_json::Value::Null`, honoring the `skip_serializing_if =
/// "serde_json::Value::is_null"` rule on the field.
#[tokio::test]
async fn critpt_evaluate_omits_null_batch_metadata() {
    let mock_server = MockServer::start().await;

    // wiremock has no built-in absent-field matcher; present-but-null must
    // not match either.
    let assert_field_absent = |request: &Request| -> bool {
        serde_json::from_slice::<serde_json::Value>(&request.body)
            .ok()
            .and_then(|parsed| {
                parsed
                    .as_object()
                    .map(|object| !object.contains_key("batch_metadata"))
            })
            .unwrap_or(false)
    };

    Mock::given(method("POST"))
        .and(path("/critpt/evaluate"))
        .and(assert_field_absent)
        .respond_with(ResponseTemplate::new(200).set_body_json(critpt_evaluate_response_zero()))
        .mount(&mock_server)
        .await;

    let client = critpt_client(&mock_server.uri());

    let body = CritPtEvaluateBody {
        submissions: vec![CritPtSubmission {
            problem_id: "p-1".to_string(),
            generated_code: "print('hi')".to_string(),
            model: "test-model".to_string(),
            generation_config: serde_json::json!({}),
            messages: None,
        }],
        batch_metadata: serde_json::Value::Null,
    };

    ok(client.send(EvaluateCritPtRequest::new(body), &CallOptions::default()).await);

    let received = mock_server
        .received_requests()
        .await
        .expect("mock server records requests by default");
    assert_eq!(received.len(), 1, "expected exactly one recorded request");
    let recorded: serde_json::Value =
        serde_json::from_slice(&received[0].body).expect("recorded body is JSON");
    assert!(
        recorded.get("batch_metadata").is_none(),
        "batch_metadata should be absent when null, got: {recorded}"
    );
}

/// Decodes a populated `LlmModelsResponse` with a fully-formed `LlmModel`
/// entry, exercising every required nested struct (`ModelCreator`,
/// `LlmEvaluations`, `LlmPricing`).
#[tokio::test]
async fn list_llm_models_decodes_populated_response() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/data/llms/models"))
        .respond_with(ResponseTemplate::new(200).set_body_json(llm_models_response_one_populated()))
        .mount(&mock_server)
        .await;

    let client = data_client(&mock_server.uri());
    let body: LlmModelsResponse = ok(client.send(ListLlmModelsRequest::default(), &CallOptions::default()).await);

    assert_eq!(body.status, 200);
    assert_eq!(body.prompt_options.parallel_queries, 4);
    assert_eq!(body.data.len(), 1);

    let model = &body.data[0];
    assert_eq!(model.id, "gpt-test");
    assert_eq!(model.name, "GPT Test");
    assert_eq!(model.slug, "gpt-test");
    assert_eq!(model.model_creator.id, "creator-1");
    assert_eq!(model.model_creator.name, "Test Lab");
    assert_eq!(model.pricing.price_1m_input_tokens, 0.25);
    assert_eq!(
        model.evaluations.artificial_analysis_intelligence_index,
        0.5
    );
}

/// Decodes a populated `MediaModelsResponse` whose single `MediaModel` carries
/// a non-empty `categories` array including the optional `format_category`
/// field.
#[tokio::test]
async fn list_text_to_image_models_decodes_populated_categories() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/data/media/text-to-image"))
        .and(query_param("include_categories", "true"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(media_models_response_one_populated_categories()),
        )
        .mount(&mock_server)
        .await;

    let client = data_client(&mock_server.uri());
    let request = ListTextToImageModelsRequest::default().with_include_categories(true);
    let body: MediaModelsResponse = ok(client.send(request, &CallOptions::default()).await);

    assert_eq!(body.status, 200);
    assert_eq!(body.include_categories, Some(true));
    assert_eq!(body.data.len(), 1);

    let model = &body.data[0];
    assert_eq!(model.id, "media-test");
    assert_eq!(model.release_date.as_deref(), Some("2026-01-01"));
    let categories = model.categories.as_ref().expect("categories present");
    assert_eq!(categories.len(), 1);
    assert_eq!(categories[0].style_category, "photorealistic");
    assert_eq!(categories[0].subject_matter_category, "people");
    assert_eq!(
        categories[0].format_category.as_deref(),
        Some("portrait"),
        "format_category should round-trip when present"
    );
    assert_eq!(categories[0].elo, 1510);
    assert_eq!(categories[0].appearances, 50);
}

/// Decodes a `MediaModelsResponse` whose `MediaModel` omits the optional
/// `release_date` and `categories` fields entirely.
#[tokio::test]
async fn list_image_editing_models_decodes_response_without_optional_fields() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/data/media/image-editing"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(media_models_response_one_without_optionals()),
        )
        .mount(&mock_server)
        .await;

    let client = data_client(&mock_server.uri());
    let body: MediaModelsResponse =
        ok(client.send(ListImageEditingModelsRequest::default(), &CallOptions::default()).await);

    assert_eq!(body.status, 200);
    assert_eq!(body.include_categories, None);
    assert_eq!(body.data.len(), 1);

    let model = &body.data[0];
    assert_eq!(model.id, "edit-test");
    assert!(
        model.release_date.is_none(),
        "release_date should decode to None when key absent, got: {:?}",
        model.release_date
    );
    assert!(
        model.categories.is_none(),
        "categories should decode to None when key absent"
    );
}

/// Decodes a realistic non-zero `CritPtEvaluateResponse`, complementing the
/// all-zero payload of `critpt_evaluate_serializes_populated_batch_metadata`.
#[tokio::test]
async fn critpt_evaluate_decodes_realistic_response() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/critpt/evaluate"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(critpt_evaluate_response_realistic()),
        )
        .mount(&mock_server)
        .await;

    let client = critpt_client(&mock_server.uri());

    let body = CritPtEvaluateBody {
        submissions: vec![CritPtSubmission {
            problem_id: "p-1".to_string(),
            generated_code: "print('hi')".to_string(),
            model: "test-model".to_string(),
            generation_config: serde_json::json!({}),
            messages: None,
        }],
        batch_metadata: serde_json::Value::Null,
    };

    let body: CritPtEvaluateResponse = ok(client.send(EvaluateCritPtRequest::new(body), &CallOptions::default()).await);

    assert_eq!(body.accuracy, 0.5);
    assert_eq!(body.timeout_rate, 0.25);
    assert_eq!(body.server_timeout_count, 3);
    assert_eq!(body.judge_error_count, 1);
}
