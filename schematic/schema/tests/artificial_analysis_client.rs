//! Integration tests for the Artificial Analysis API clients.
//!
//! These tests use [`wiremock`] to verify the user-observable HTTP behaviour
//! of the generated `ArtificialAnalysisData` and `ArtificialAnalysisCritPt`
//! clients without touching the network. They cover query-string serialization,
//! authentication header injection (explicit override and env-var fallback),
//! the missing-credentials error path, and JSON body serialization for the
//! CritPt evaluation endpoint (including `skip_serializing_if` on
//! `batch_metadata`).
//!
//! Tests that mutate `ARTIFICIAL_ANALYSIS_API_KEY` are serialized via
//! `serial_test::serial` so concurrent test execution does not see torn
//! environment state.

#![allow(
    clippy::needless_borrows_for_generic_args,
    clippy::field_reassign_with_default
)]

use schematic_schema::artificial_analysis::{
    ArtificialAnalysisCritPt, ArtificialAnalysisCritPtRequest, ArtificialAnalysisData,
    ArtificialAnalysisDataRequest, CritPtEvaluateBody, CritPtEvaluateResponse, CritPtMessage,
    CritPtSubmission, EvaluateCritPtRequest, ListImageEditingModelsRequest, ListLlmModelsRequest,
    ListTextToImageModelsRequest, LlmModelsResponse, MediaModelsResponse,
};
use schematic_schema::shared::SchematicError;
use wiremock::matchers::{body_json, header, method, path, query_param};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

const ENV_KEY: &str = "ARTIFICIAL_ANALYSIS_API_KEY";

/// Removes `ARTIFICIAL_ANALYSIS_API_KEY` from the environment.
///
/// ## Safety
///
/// Tests that touch the env are gated by `#[serial_test::serial]` to avoid
/// concurrent reads/writes of process-wide env state.
fn clear_env() {
    // SAFETY: gated by `#[serial]`, so no other test thread is touching the env.
    unsafe {
        std::env::remove_var(ENV_KEY);
    }
}

/// Sets `ARTIFICIAL_ANALYSIS_API_KEY` to `value`.
///
/// ## Safety
///
/// Tests that touch the env are gated by `#[serial_test::serial]` to avoid
/// concurrent reads/writes of process-wide env state.
fn set_env(value: &str) {
    // SAFETY: gated by `#[serial]`, so no other test thread is touching the env.
    unsafe {
        std::env::set_var(ENV_KEY, value);
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

/// Test 1: Data API exercise of `ListTextToImageModelsRequest` with the
/// `include_categories=true` query parameter.
///
/// Verifies that:
/// - The generated client hits `GET /data/media/text-to-image`.
/// - The `include_categories=true` query string is serialized.
#[tokio::test]
#[serial_test::serial]
async fn list_text_to_image_models_emits_include_categories_query() {
    set_env("test-key");
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/data/media/text-to-image"))
        .and(query_param("include_categories", "true"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(media_models_response_empty(Some(true))),
        )
        .mount(&mock_server)
        .await;

    let client = ArtificialAnalysisData::with_base_url(&mock_server.uri());

    let request = ArtificialAnalysisDataRequest::ListTextToImageModels(
        ListTextToImageModelsRequest::default().with_include_categories(true),
    );
    let result: Result<MediaModelsResponse, _> = client.request(request).await;

    assert!(result.is_ok(), "Request failed: {:?}", result.err());
    let body = result.expect("ok");
    assert_eq!(body.status, 200);
    assert_eq!(body.include_categories, Some(true));
    assert!(body.data.is_empty());

    clear_env();
}

/// Test 2: Data API exercise of `ListLlmModelsRequest::default()` — no query
/// parameters.
///
/// Verifies the generated client hits `GET /data/llms/models` with no query
/// string at all.
#[tokio::test]
#[serial_test::serial]
async fn list_llm_models_uses_no_query_string() {
    set_env("test-key");
    let mock_server = MockServer::start().await;

    // Use a custom matcher to assert query is empty (wiremock has no
    // built-in "no query string" matcher).
    let no_query = |request: &Request| -> bool { request.url.query().is_none() };

    Mock::given(method("GET"))
        .and(path("/data/llms/models"))
        .and(no_query)
        .respond_with(ResponseTemplate::new(200).set_body_json(llm_models_response_empty()))
        .mount(&mock_server)
        .await;

    let client = ArtificialAnalysisData::with_base_url(&mock_server.uri());

    let request = ArtificialAnalysisDataRequest::ListLlmModels(ListLlmModelsRequest::default());
    let result: Result<LlmModelsResponse, _> = client.request(request).await;

    assert!(result.is_ok(), "Request failed: {:?}", result.err());
    let body = result.expect("ok");
    assert_eq!(body.status, 200);
    assert_eq!(body.prompt_options.parallel_queries, 1);
    assert_eq!(body.prompt_options.prompt_length, "medium");
    assert!(body.data.is_empty());

    clear_env();
}

/// Test 3: Data API missing-auth behaviour.
///
/// With `ARTIFICIAL_ANALYSIS_API_KEY` unset and no explicit `.api_key(...)`
/// call, the client must fail with `SchematicError::AuthenticationRequired`
/// before any HTTP request is made.
#[tokio::test]
#[serial_test::serial]
async fn missing_auth_returns_authentication_required_error() {
    clear_env();
    let mock_server = MockServer::start().await;

    // No mock is registered: if the client erroneously sent a request, the
    // mock server would reply with 404 and we'd see `ApiError`, not
    // `AuthenticationRequired`.

    let client = ArtificialAnalysisData::with_base_url(&mock_server.uri());

    let request = ArtificialAnalysisDataRequest::ListLlmModels(ListLlmModelsRequest::default());
    // Intentionally typed as `serde_json::Value`: this test exercises the
    // pre-flight `AuthenticationRequired` error path, where no HTTP request
    // is made and no response body is decoded. Typed-response decoding is
    // covered by other tests in this file.
    let result: Result<serde_json::Value, _> = client.request(request).await;

    match result {
        Err(SchematicError::AuthenticationRequired {
            ref env_fallback_vars,
            ..
        }) => {
            assert!(
                env_fallback_vars.iter().any(|name| name == ENV_KEY),
                "expected env_fallback_vars to mention {ENV_KEY}, got {env_fallback_vars:?}"
            );
        }
        other => panic!("expected AuthenticationRequired, got {other:?}"),
    }
}

/// Test 4: Explicit `.api_key("...")` injection beats the env-var fallback.
///
/// `ARTIFICIAL_ANALYSIS_API_KEY` is set to an empty string, while
/// `.api_key("explicit-key")` is invoked on the builder. The mock asserts
/// that the wire request carries `x-api-key: explicit-key`.
#[tokio::test]
#[serial_test::serial]
async fn explicit_api_key_overrides_env_fallback() {
    set_env("");
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/data/llms/models"))
        .and(header("x-api-key", "explicit-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(llm_models_response_empty()))
        .mount(&mock_server)
        .await;

    let client = ArtificialAnalysisData::with_base_url(&mock_server.uri()).api_key("explicit-key");

    let request = ArtificialAnalysisDataRequest::ListLlmModels(ListLlmModelsRequest::default());
    let result: Result<LlmModelsResponse, _> = client.request(request).await;

    assert!(result.is_ok(), "Request failed: {:?}", result.err());
    let body = result.expect("ok");
    assert_eq!(body.status, 200);

    clear_env();
}

/// Test 5: Env-var fallback is consulted when no explicit `.api_key(...)`
/// is set.
///
/// Sets `ARTIFICIAL_ANALYSIS_API_KEY=env-key` and asserts the request goes
/// out with `x-api-key: env-key`.
#[tokio::test]
#[serial_test::serial]
async fn env_api_key_is_used_when_no_explicit_key() {
    set_env("env-key");
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/data/llms/models"))
        .and(header("x-api-key", "env-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(llm_models_response_empty()))
        .mount(&mock_server)
        .await;

    let client = ArtificialAnalysisData::with_base_url(&mock_server.uri());

    let request = ArtificialAnalysisDataRequest::ListLlmModels(ListLlmModelsRequest::default());
    let result: Result<LlmModelsResponse, _> = client.request(request).await;

    assert!(result.is_ok(), "Request failed: {:?}", result.err());
    let body = result.expect("ok");
    assert_eq!(body.status, 200);

    clear_env();
}

/// Test 6a: CritPt API serializes the full evaluation body (including
/// populated `batch_metadata`) exactly as expected.
#[tokio::test]
#[serial_test::serial]
async fn critpt_evaluate_serializes_populated_batch_metadata() {
    set_env("test-key");
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

    let client = ArtificialAnalysisCritPt::with_base_url(&mock_server.uri());

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

    let request = ArtificialAnalysisCritPtRequest::EvaluateCritPt(EvaluateCritPtRequest::new(body));
    let result: Result<CritPtEvaluateResponse, _> = client.request(request).await;

    assert!(result.is_ok(), "Request failed: {:?}", result.err());
    let body = result.expect("ok");
    assert_eq!(body.accuracy, 0.0);
    assert_eq!(body.timeout_rate, 0.0);
    assert_eq!(body.server_timeout_count, 0);
    assert_eq!(body.judge_error_count, 0);

    clear_env();
}

/// Test 6b: CritPt API drops `batch_metadata` from the wire body when it is
/// `serde_json::Value::Null`, honouring the `skip_serializing_if =
/// "serde_json::Value::is_null"` rule on the field.
///
/// Wiremock has no built-in absent-field matcher; we use a closure-based
/// `Match` to inspect the recorded request body.
#[tokio::test]
#[serial_test::serial]
async fn critpt_evaluate_omits_null_batch_metadata() {
    set_env("test-key");
    let mock_server = MockServer::start().await;

    // Closure matcher: parse body as JSON and assert `batch_metadata` is
    // not present at all (not just present-but-null).
    let assert_field_absent = |request: &Request| -> bool {
        let parsed: serde_json::Value = match serde_json::from_slice(&request.body) {
            Ok(value) => value,
            Err(_) => return false,
        };
        let object = match parsed.as_object() {
            Some(map) => map,
            None => return false,
        };
        !object.contains_key("batch_metadata")
    };

    Mock::given(method("POST"))
        .and(path("/critpt/evaluate"))
        .and(assert_field_absent)
        .respond_with(ResponseTemplate::new(200).set_body_json(critpt_evaluate_response_zero()))
        .mount(&mock_server)
        .await;

    let client = ArtificialAnalysisCritPt::with_base_url(&mock_server.uri());

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

    let request = ArtificialAnalysisCritPtRequest::EvaluateCritPt(EvaluateCritPtRequest::new(body));
    // Intentionally typed as `serde_json::Value`: this test asserts on the
    // **request body** the client emits (specifically that
    // `batch_metadata` is omitted when null). Typed-response decoding is
    // covered by `critpt_evaluate_serializes_populated_batch_metadata`.
    let result: Result<serde_json::Value, _> = client.request(request).await;

    assert!(result.is_ok(), "Request failed: {:?}", result.err());

    // Cross-check by inspecting the recorded request body directly.
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

    clear_env();
}

/// Test 7: Decode a populated `LlmModelsResponse` with a fully-formed
/// `LlmModel` entry, exercising every required nested struct
/// (`ModelCreator`, `LlmEvaluations`, `LlmPricing`).
#[tokio::test]
#[serial_test::serial]
async fn list_llm_models_decodes_populated_response() {
    set_env("test-key");
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/data/llms/models"))
        .respond_with(ResponseTemplate::new(200).set_body_json(llm_models_response_one_populated()))
        .mount(&mock_server)
        .await;

    let client = ArtificialAnalysisData::with_base_url(&mock_server.uri());

    let request = ArtificialAnalysisDataRequest::ListLlmModels(ListLlmModelsRequest::default());
    let result: Result<LlmModelsResponse, _> = client.request(request).await;

    assert!(result.is_ok(), "Request failed: {:?}", result.err());
    let body = result.expect("ok");
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

    clear_env();
}

/// Test 8: Decode a populated `MediaModelsResponse` whose single `MediaModel`
/// carries a non-empty `categories` array including the optional
/// `format_category` field. Exercises `Option<Vec<CategoryBreakdown>>` and
/// `Option<String>` decode in their populated form.
#[tokio::test]
#[serial_test::serial]
async fn list_text_to_image_models_decodes_populated_categories() {
    set_env("test-key");
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

    let client = ArtificialAnalysisData::with_base_url(&mock_server.uri());

    let request = ArtificialAnalysisDataRequest::ListTextToImageModels(
        ListTextToImageModelsRequest::default().with_include_categories(true),
    );
    let result: Result<MediaModelsResponse, _> = client.request(request).await;

    assert!(result.is_ok(), "Request failed: {:?}", result.err());
    let body = result.expect("ok");
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

    clear_env();
}

/// Test 9: Decode a `MediaModelsResponse` whose `MediaModel` omits the
/// optional `release_date` and `categories` fields entirely. Exercises
/// `Option<...>` decode-from-absent behaviour, complementing the populated
/// case in `list_text_to_image_models_decodes_populated_categories`.
#[tokio::test]
#[serial_test::serial]
async fn list_image_editing_models_decodes_response_without_optional_fields() {
    set_env("test-key");
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/data/media/image-editing"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(media_models_response_one_without_optionals()),
        )
        .mount(&mock_server)
        .await;

    let client = ArtificialAnalysisData::with_base_url(&mock_server.uri());

    let request = ArtificialAnalysisDataRequest::ListImageEditingModels(
        ListImageEditingModelsRequest::default(),
    );
    let result: Result<MediaModelsResponse, _> = client.request(request).await;

    assert!(result.is_ok(), "Request failed: {:?}", result.err());
    let body = result.expect("ok");
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

    clear_env();
}

/// Test 10: Decode a realistic non-zero `CritPtEvaluateResponse`. Complements
/// `critpt_evaluate_serializes_populated_batch_metadata` (all-zero payload)
/// by proving non-zero numeric decoding round-trips correctly. Uses
/// exactly-representable f64 values (`0.5`, `0.25`) so `assert_eq!` is safe.
#[tokio::test]
#[serial_test::serial]
async fn critpt_evaluate_decodes_realistic_response() {
    set_env("test-key");
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/critpt/evaluate"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(critpt_evaluate_response_realistic()),
        )
        .mount(&mock_server)
        .await;

    let client = ArtificialAnalysisCritPt::with_base_url(&mock_server.uri());

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

    let request = ArtificialAnalysisCritPtRequest::EvaluateCritPt(EvaluateCritPtRequest::new(body));
    let result: Result<CritPtEvaluateResponse, _> = client.request(request).await;

    assert!(result.is_ok(), "Request failed: {:?}", result.err());
    let body = result.expect("ok");
    assert_eq!(body.accuracy, 0.5);
    assert_eq!(body.timeout_rate, 0.25);
    assert_eq!(body.server_timeout_count, 3);
    assert_eq!(body.judge_error_count, 1);

    clear_env();
}
