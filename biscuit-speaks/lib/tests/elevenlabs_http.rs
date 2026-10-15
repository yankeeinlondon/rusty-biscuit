//! ElevenLabs provider HTTP behavior against a local mock server.
//!
//! Every test points the provider at a `wiremock` server through
//! `ElevenLabsProvider::with_base_url`; nothing reaches the real API. The API
//! key comes from the environment, so each test holds an `EnvGuard` under
//! `#[serial]`.

use biscuit_speaks::{ElevenLabsProvider, TtsConfig, TtsError};
use serial_test::serial;
use test_toolkit::EnvGuard;
use wiremock::matchers::{
    body_partial_json, header, method, path, query_param, query_param_is_missing,
};
use wiremock::{Mock, MockServer, ResponseTemplate};

const TEST_KEY: &str = "elevenlabs-mock-key";

/// Isolates both ElevenLabs key variables, exposing `key` under the
/// preferred name only.
fn key_env(key: &str) -> (EnvGuard, EnvGuard) {
    (
        EnvGuard::set_safe("ELEVEN_LABS_API_KEY", key),
        EnvGuard::remove_safe("ELEVENLABS_API_KEY"),
    )
}

fn provider_for(server: &MockServer) -> ElevenLabsProvider {
    ElevenLabsProvider::with_base_url(server.uri()).expect("provider must build")
}

fn voice(id: &str, name: &str) -> serde_json::Value {
    serde_json::json!({ "voice_id": id, "name": name })
}

#[tokio::test]
#[serial]
async fn generate_audio_returns_exact_response_bytes_and_sends_key_header() {
    let _env = key_env(TEST_KEY);
    let server = MockServer::start().await;
    let audio: Vec<u8> = vec![0xFF, 0xF3, 0x44, 0xC4, 0x00, 0x01, 0x02, 0x80];
    Mock::given(method("POST"))
        .and(path("/v1/text-to-speech/voice-abc"))
        .and(header("xi-api-key", TEST_KEY))
        .and(body_partial_json(serde_json::json!({
            "text": "hello mock",
            "model_id": "eleven_multilingual_v2",
        })))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "audio/mpeg")
                .set_body_bytes(audio.clone()),
        )
        .expect(1)
        .mount(&server)
        .await;

    let config = TtsConfig::new().with_voice("voice-abc");
    let bytes = provider_for(&server)
        .generate_audio("hello mock", &config)
        .await
        .expect("speech must succeed");

    assert_eq!(bytes, audio);
}

#[tokio::test]
#[serial]
async fn create_sound_effect_returns_exact_response_bytes() {
    let _env = key_env(TEST_KEY);
    let server = MockServer::start().await;
    let audio: Vec<u8> = vec![0x49, 0x44, 0x33, 0x04, 0x00, 0x7F];
    Mock::given(method("POST"))
        .and(path("/v1/sound-generation"))
        .and(header("xi-api-key", TEST_KEY))
        .and(body_partial_json(serde_json::json!({
            "text": "dog barking",
            "duration_seconds": 2.5,
        })))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "audio/mpeg")
                .set_body_bytes(audio.clone()),
        )
        .expect(1)
        .mount(&server)
        .await;

    let bytes = provider_for(&server)
        .create_sound_effect("dog barking", Some(2.5))
        .await
        .expect("sound effect must succeed");

    assert_eq!(bytes, audio);
}

#[tokio::test]
#[serial]
async fn list_models_decodes_json_response() {
    let _env = key_env(TEST_KEY);
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/models"))
        .and(header("xi-api-key", TEST_KEY))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
            {
                "model_id": "eleven_multilingual_v2",
                "name": "Multilingual v2",
                "description": "test model",
                "can_do_text_to_speech": true
            }
        ])))
        .expect(1)
        .mount(&server)
        .await;

    let models = provider_for(&server)
        .list_models()
        .await
        .expect("models must decode");

    assert_eq!(models.len(), 1);
    assert_eq!(models[0].model_id, "eleven_multilingual_v2");
    assert_eq!(models[0].name, "Multilingual v2");
    assert!(models[0].can_do_text_to_speech);
}

#[tokio::test]
#[serial]
async fn list_voices_raw_follows_page_token_and_aggregates() {
    let _env = key_env(TEST_KEY);
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v2/voices"))
        .and(header("xi-api-key", TEST_KEY))
        .and(query_param("page_size", "100"))
        .and(query_param_is_missing("page_token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "voices": [voice("v1", "First"), voice("v2", "Second")],
            "has_more": true,
            "total_count": 3,
            "next_page_token": "token-page-2"
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v2/voices"))
        .and(header("xi-api-key", TEST_KEY))
        .and(query_param("page_size", "100"))
        .and(query_param("page_token", "token-page-2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "voices": [voice("v3", "Third")],
            "has_more": false,
            "total_count": 3
        })))
        .expect(1)
        .mount(&server)
        .await;

    let response = provider_for(&server)
        .list_voices_raw()
        .await
        .expect("voices must aggregate");

    let ids: Vec<&str> = response
        .voices
        .iter()
        .map(|voice| voice.voice_id.as_str())
        .collect();
    assert_eq!(ids, ["v1", "v2", "v3"]);
    assert_eq!(response.total_count, Some(3));
    assert!(!response.has_more);
    assert_eq!(response.next_page_token, None);
}

#[tokio::test]
#[serial]
async fn list_voices_raw_stops_when_more_voices_have_no_next_token() {
    let _env = key_env(TEST_KEY);
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v2/voices"))
        .and(query_param("page_size", "100"))
        .and(query_param_is_missing("page_token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "voices": [voice("v1", "First")],
            "has_more": true,
            "total_count": 2
        })))
        .expect(1)
        .mount(&server)
        .await;

    let response = tokio::time::timeout(
        std::time::Duration::from_secs(2),
        provider_for(&server).list_voices_raw(),
    )
    .await
    .expect("a missing token must terminate pagination")
    .expect("the available page must succeed");

    assert_eq!(response.voices.len(), 1);
    assert_eq!(response.voices[0].voice_id, "v1");
    assert_eq!(response.total_count, Some(2));
    assert!(!response.has_more);
    assert_eq!(response.next_page_token, None);
}

#[tokio::test]
#[serial]
async fn provider_keeps_the_credential_read_at_construction() {
    let _env = key_env(TEST_KEY);
    let server = MockServer::start().await;
    let provider = provider_for(&server);
    let _changed_key = EnvGuard::set_safe("ELEVEN_LABS_API_KEY", "another-account-key");
    Mock::given(method("POST"))
        .and(path("/v1/text-to-speech/voice-abc"))
        .and(header("xi-api-key", TEST_KEY))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "audio/mpeg")
                .set_body_bytes(vec![1, 2, 3]),
        )
        .expect(1)
        .mount(&server)
        .await;

    let bytes = provider
        .generate_audio(
            "credential stays pinned",
            &TtsConfig::new().with_voice("voice-abc"),
        )
        .await
        .expect("synthesis must use the original credential");
    assert_eq!(bytes, [1, 2, 3]);
}

#[tokio::test]
#[serial]
async fn list_voices_raw_maps_402_to_paid_plan_required_with_body() {
    let _env = key_env(TEST_KEY);
    let server = MockServer::start().await;
    let body = r#"{"detail":{"status":"payment_required"}}"#;
    Mock::given(method("GET"))
        .and(path("/v2/voices"))
        .respond_with(ResponseTemplate::new(402).set_body_string(body))
        .expect(1)
        .mount(&server)
        .await;

    let error = provider_for(&server)
        .list_voices_raw()
        .await
        .expect_err("402 must fail");

    match error {
        TtsError::PaidPlanRequired { provider, detail } => {
            assert_eq!(provider, "ElevenLabs");
            assert_eq!(detail, body);
        }
        other => panic!("expected PaidPlanRequired, got {other:?}"),
    }
}

#[tokio::test]
#[serial]
async fn generate_audio_maps_402_to_paid_plan_required_with_body() {
    let _env = key_env(TEST_KEY);
    let server = MockServer::start().await;
    let body = r#"{"detail":{"status":"paid_plan_required"}}"#;
    Mock::given(method("POST"))
        .and(path("/v1/text-to-speech/voice-abc"))
        .respond_with(ResponseTemplate::new(402).set_body_string(body))
        .expect(1)
        .mount(&server)
        .await;

    let config = TtsConfig::new().with_voice("voice-abc");
    let error = provider_for(&server)
        .generate_audio("hello", &config)
        .await
        .expect_err("402 must fail");

    match error {
        TtsError::PaidPlanRequired { detail, .. } => assert_eq!(detail, body),
        other => panic!("expected PaidPlanRequired, got {other:?}"),
    }
}

#[tokio::test]
#[serial]
async fn list_models_maps_500_to_http_error_with_status_and_body() {
    let _env = key_env(TEST_KEY);
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/models"))
        .respond_with(ResponseTemplate::new(500).set_body_string("upstream exploded"))
        .expect(1)
        .mount(&server)
        .await;

    let error = provider_for(&server)
        .list_models()
        .await
        .expect_err("500 must fail");

    match error {
        TtsError::HttpError { provider, message } => {
            assert_eq!(provider, "elevenlabs");
            assert_eq!(message, "API error (500): upstream exploded");
        }
        other => panic!("expected HttpError, got {other:?}"),
    }
}

#[tokio::test]
#[serial]
async fn list_models_maps_malformed_success_body_to_parse_error() {
    let _env = key_env(TEST_KEY);
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/models"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "application/json")
                .set_body_string("{not json"),
        )
        .expect(1)
        .mount(&server)
        .await;

    let error = provider_for(&server)
        .list_models()
        .await
        .expect_err("malformed body must fail");

    match error {
        TtsError::HttpError { message, .. } => assert!(
            message.starts_with("Failed to parse response: "),
            "unexpected message: {message}"
        ),
        other => panic!("expected HttpError, got {other:?}"),
    }
}

#[tokio::test]
#[serial]
async fn api_key_never_appears_in_error_display_or_debug() {
    const SENTINEL: &str = "sentinel-key-6b1f0c-must-not-leak";
    let _env = key_env(SENTINEL);
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/models"))
        .and(header("xi-api-key", SENTINEL))
        .respond_with(ResponseTemplate::new(401).set_body_string("invalid api key"))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v2/voices"))
        .and(header("xi-api-key", SENTINEL))
        .respond_with(ResponseTemplate::new(500).set_body_string("server error"))
        .mount(&server)
        .await;
    let provider = provider_for(&server);

    let unauthorized = provider.list_models().await.expect_err("401 must fail");
    let server_error = provider.list_voices_raw().await.expect_err("500 must fail");

    assert!(matches!(unauthorized, TtsError::HttpError { .. }));
    assert!(matches!(server_error, TtsError::HttpError { .. }));
    for rendered in [
        unauthorized.to_string(),
        format!("{unauthorized:?}"),
        server_error.to_string(),
        format!("{server_error:?}"),
        format!("{provider:?}"),
    ] {
        assert!(!rendered.contains(SENTINEL), "key leaked into: {rendered}");
    }
}

#[tokio::test]
#[serial]
async fn transport_failure_maps_to_http_error_without_key() {
    const SENTINEL: &str = "sentinel-key-transport-must-not-leak";
    let _env = key_env(SENTINEL);
    // A port that was bound and released refuses the connection.
    let closed_port = std::net::TcpListener::bind("127.0.0.1:0")
        .and_then(|listener| listener.local_addr())
        .expect("bind ephemeral port")
        .port();
    let provider = ElevenLabsProvider::with_base_url(format!("http://127.0.0.1:{closed_port}"))
        .expect("provider must build");

    let error = provider
        .list_models()
        .await
        .expect_err("refused connection must fail");

    match &error {
        TtsError::HttpError { provider, message } => {
            assert_eq!(provider, "elevenlabs");
            assert!(
                message.contains("/v1/models"),
                "unexpected message: {message}"
            );
        }
        other => panic!("expected HttpError, got {other:?}"),
    }
    assert!(!error.to_string().contains(SENTINEL));
    assert!(!format!("{error:?}").contains(SENTINEL));
}

#[test]
#[serial]
fn with_base_url_without_key_is_missing_api_key() {
    let _preferred = EnvGuard::remove_safe("ELEVEN_LABS_API_KEY");
    let _alternate = EnvGuard::remove_safe("ELEVENLABS_API_KEY");

    let result = ElevenLabsProvider::with_base_url("http://127.0.0.1:9");

    assert!(matches!(result, Err(TtsError::MissingApiKey { .. })));
}

#[tokio::test]
#[serial]
async fn alternate_key_variable_is_sent_when_preferred_is_unset() {
    let _preferred = EnvGuard::remove_safe("ELEVEN_LABS_API_KEY");
    let _alternate = EnvGuard::set_safe("ELEVENLABS_API_KEY", "alternate-mock-key");
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/models"))
        .and(header("xi-api-key", "alternate-mock-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([])))
        .expect(1)
        .mount(&server)
        .await;

    let models = provider_for(&server)
        .list_models()
        .await
        .expect("alternate key must authenticate");

    assert!(models.is_empty());
}
