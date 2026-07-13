//! ElevenLabs API definition.
//!
//! This module provides a complete definition of the ElevenLabs API,
//! including both REST and WebSocket endpoints for text-to-speech,
//! voice management, sound effects, and workspace operations.
//!
//! ## Endpoints
//!
//! ### Text-to-Speech
//! - `CreateSpeech` - POST /v1/text-to-speech/{voice_id}
//! - `StreamSpeech` - POST /v1/text-to-speech/{voice_id}/stream
//! - `CreateSpeechWithTimestamps` - POST /v1/text-to-speech/{voice_id}/with-timestamps
//! - `StreamSpeechWithTimestamps` - POST /v1/text-to-speech/{voice_id}/stream/with-timestamps
//!
//! ### Voices
//! - `ListVoices` - GET /v2/voices
//! - `GetVoice` - GET /v1/voices/{voice_id}
//! - `DeleteVoice` - DELETE /v1/voices/{voice_id}
//! - And more...
//!
//! ## Examples
//!
//! ```rust
//! use schematic_definitions::elevenlabs::define_elevenlabs_rest_api;
//!
//! let api = define_elevenlabs_rest_api();
//! assert_eq!(api.name, "ElevenLabs");
//! assert!(api.endpoints.len() >= 35);
//! ```

mod endpoints;
mod types;

pub use types::*;

use crate::registry::SchemaRegistry;
#[cfg(test)]
use schematic_define::ApiResponse;
use schematic_define::{ApiKeyEnv, AuthStrategy, Endpoint, EnvList, EnvMapping, RestApi, Schema};

/// Creates a schema registry containing all ElevenLabs response types.
///
/// This registry can be used to generate OpenAPI schemas for the ElevenLabs API.
/// All response types used by the API endpoints are registered.
///
/// ## Examples
///
/// ```
/// use schematic_definitions::elevenlabs::{openapi_registry, define_elevenlabs_rest_api};
///
/// let registry = openapi_registry();
/// let api = define_elevenlabs_rest_api();
///
/// // Registry contains all response types
/// assert!(registry.get("SpeechWithTimestampsResponse").is_some());
/// assert!(registry.get("ListVoicesResponse").is_some());
/// assert!(registry.get("VoiceResponseModel").is_some());
///
/// // Registry is complete for the API
/// assert!(registry.validate_completeness(&api).is_ok());
/// ```
#[must_use]
pub fn openapi_registry() -> SchemaRegistry {
    SchemaRegistry::new()
        .register::<CreateSpeechBody>("CreateSpeechBody")
        .register::<AddSharedVoiceBody>("AddSharedVoiceBody")
        .register::<CreatePvcVoiceBody>("CreatePvcVoiceBody")
        .register::<TrainPvcVoiceBody>("TrainPvcVoiceBody")
        .register::<CreateSoundEffectBody>("CreateSoundEffectBody")
        .register::<DownloadHistoryBody>("DownloadHistoryBody")
        .register::<ShareResourceBody>("ShareResourceBody")
        .register::<UnshareResourceBody>("UnshareResourceBody")
        .register::<CopyResourceBody>("CopyResourceBody")
        .register::<CreateApiKeyBody>("CreateApiKeyBody")
        .register::<UpdateApiKeyBody>("UpdateApiKeyBody")
        .register::<CreateWebhookBody>("CreateWebhookBody")
        .register::<UpdateWebhookBody>("UpdateWebhookBody")
        .register::<SpeechWithTimestampsResponse>("SpeechWithTimestampsResponse")
        .register::<ListVoicesResponse>("ListVoicesResponse")
        .register::<VoiceResponseModel>("VoiceResponseModel")
        .register::<StatusResponse>("StatusResponse")
        .register::<VoiceSettings>("VoiceSettings")
        .register::<AddSampleResponse>("AddSampleResponse")
        .register::<ListSharedVoicesResponse>("ListSharedVoicesResponse")
        .register::<AddSharedVoiceResponse>("AddSharedVoiceResponse")
        .register::<Vec<ModelInfo>>("Vec<ModelInfo>")
        .register::<SingleUseTokenResponse>("SingleUseTokenResponse")
        .register::<GetHistoryResponse>("GetHistoryResponse")
        .register::<SpeechHistoryItemResponseModel>("SpeechHistoryItemResponseModel")
        .register::<UsageStatsResponse>("UsageStatsResponse")
        .register::<UserResponse>("UserResponse")
        .register::<SubscriptionModel>("SubscriptionModel")
        .register::<ResourceResponse>("ResourceResponse")
        .register::<ListServiceAccountsResponse>("ListServiceAccountsResponse")
        .register::<ListApiKeysResponse>("ListApiKeysResponse")
        .register::<CreateApiKeyResponse>("CreateApiKeyResponse")
        .register::<ListWebhooksResponse>("ListWebhooksResponse")
        .register::<CreateWebhookResponse>("CreateWebhookResponse")
}

/// Creates the ElevenLabs REST API definition.
///
/// This defines the ElevenLabs REST API with endpoints for text-to-speech,
/// voice management, sound effects, models, history, workspace, service
/// accounts, and webhooks.
///
/// ## Endpoints
///
/// - **Text-to-Speech**: 4 endpoints (create, stream, timestamps)
/// - **Voices**: 8 endpoints (list, get, delete, settings, samples, library)
/// - **PVC**: 3 endpoints (create, update, train)
/// - **Sound Effects**: 1 endpoint
/// - **Models**: 1 endpoint
/// - **Tokens**: 1 endpoint
/// - **History**: 5 endpoints
/// - **Workspace**: 7 endpoints (usage, user, resources)
/// - **Service Accounts**: 5 endpoints
/// - **Webhooks**: 4 endpoints
///
/// ## Examples
///
/// ```rust
/// use schematic_definitions::elevenlabs::define_elevenlabs_rest_api;
///
/// let api = define_elevenlabs_rest_api();
/// assert_eq!(api.name, "ElevenLabs");
/// assert_eq!(api.base_url, "https://api.elevenlabs.io");
/// ```
pub fn define_elevenlabs_rest_api() -> RestApi {
    let mut endpoints: Vec<Endpoint> = vec![];
    endpoints.extend(endpoints::speech::all());
    endpoints.extend(endpoints::voices::all());
    endpoints.extend(endpoints::sound_generation::all());
    endpoints.extend(endpoints::history::all());
    endpoints.extend(endpoints::workspace::all());

    RestApi {
        name: "ElevenLabs".to_string(),
        description: "ElevenLabs Creative Platform API for text-to-speech, voice management, and sound generation".to_string(),
        base_url: "https://api.elevenlabs.io".to_string(),
        docs_url: Some("https://elevenlabs.io/docs/api-reference".to_string()),
        auth: AuthStrategy::ApiKey {
            header: "xi-api-key".to_string(),
            value_prefix: None,
        },
        auth_policy: None,
        env_auth: vec![
            "ELEVEN_LABS_API_KEY".to_string(),
            "ELEVENLABS_API_KEY".to_string(),
        ],
        env_username: None,
        headers: vec![],
        endpoints,
        module_path: None,
        request_suffix: None,
        version: None,
        env_mapping: Some(EnvMapping {
            api_key: Some(ApiKeyEnv {
                names: EnvList::new(vec![
                    "ELEVEN_LABS_API_KEY".to_string(),
                    "ELEVENLABS_API_KEY".to_string(),
                ]),
                header: "xi-api-key".to_string(),
            }),
            ..Default::default()
        }),
    }
}

/// Creates the ElevenLabs WebSocket API definition.
///
/// This defines the ElevenLabs WebSocket API with endpoints for real-time
/// text-to-speech streaming.
///
/// ## Endpoints
///
/// - `TextToSpeech` - /v1/text-to-speech/{voice_id}/stream-input
/// - `MultiContextTextToSpeech` - /v1/text-to-speech/{voice_id}/multi-stream-input
///
/// ## Examples
///
/// ```rust
/// use schematic_definitions::elevenlabs::define_elevenlabs_websocket_api;
///
/// let api = define_elevenlabs_websocket_api();
/// assert_eq!(api.name, "ElevenLabsTTS");
/// assert_eq!(api.endpoints.len(), 2);
/// ```
pub fn define_elevenlabs_websocket_api() -> schematic_define::WebSocketApi {
    use schematic_define::websocket::*;

    WebSocketApi {
        name: "ElevenLabsTTS".to_string(),
        description: "ElevenLabs Text-to-Speech WebSocket API for real-time streaming".to_string(),
        base_url: "wss://api.elevenlabs.io".to_string(),
        docs_url: Some("https://elevenlabs.io/docs/api-reference/websockets".to_string()),
        auth: AuthStrategy::ApiKey {
            header: "xi-api-key".to_string(),
            value_prefix: None,
        },
        env_auth: vec![
            "ELEVEN_LABS_API_KEY".to_string(),
            "ELEVENLABS_API_KEY".to_string(),
        ],
        version: None,
        endpoints: vec![
            WebSocketEndpoint {
                id: "TextToSpeech".to_string(),
                path: "/v1/text-to-speech/{voice_id}/stream-input".to_string(),
                description: "Stream text and receive audio chunks in real-time".to_string(),
                connection_params: vec![
                    ConnectionParam {
                        name: "voice_id".to_string(),
                        param_type: ParamType::String,
                        required: true,
                        description: Some(
                            "Voice identifier used in the websocket path".to_string(),
                        ),
                    },
                    ConnectionParam {
                        name: "model_id".to_string(),
                        param_type: ParamType::String,
                        required: false,
                        description: Some("TTS model to use".to_string()),
                    },
                    ConnectionParam {
                        name: "language_code".to_string(),
                        param_type: ParamType::String,
                        required: false,
                        description: Some("Target language code".to_string()),
                    },
                    ConnectionParam {
                        name: "enable_logging".to_string(),
                        param_type: ParamType::Boolean,
                        required: false,
                        description: Some("Enable logging (default: true)".to_string()),
                    },
                    ConnectionParam {
                        name: "enable_ssml_parsing".to_string(),
                        param_type: ParamType::Boolean,
                        required: false,
                        description: Some("Parse SSML tags".to_string()),
                    },
                    ConnectionParam {
                        name: "output_format".to_string(),
                        param_type: ParamType::String,
                        required: false,
                        description: Some("Audio output format".to_string()),
                    },
                    ConnectionParam {
                        name: "inactivity_timeout".to_string(),
                        param_type: ParamType::Integer,
                        required: false,
                        description: Some("Timeout in seconds (default: 20)".to_string()),
                    },
                    ConnectionParam {
                        name: "sync_alignment".to_string(),
                        param_type: ParamType::Boolean,
                        required: false,
                        description: Some("Synchronize alignment data".to_string()),
                    },
                    ConnectionParam {
                        name: "auto_mode".to_string(),
                        param_type: ParamType::Boolean,
                        required: false,
                        description: Some("Auto mode for generation".to_string()),
                    },
                    ConnectionParam {
                        name: "apply_text_normalization".to_string(),
                        param_type: ParamType::String,
                        required: false,
                        description: Some("Text normalization: auto, on, off".to_string()),
                    },
                    ConnectionParam {
                        name: "seed".to_string(),
                        param_type: ParamType::Integer,
                        required: false,
                        description: Some("Reproducibility seed".to_string()),
                    },
                ],
                lifecycle: ConnectionLifecycle {
                    open: Some(MessageSchema {
                        name: "BOS".to_string(),
                        direction: MessageDirection::Client,
                        schema: Schema::new("TtsInitMessage"),
                        description: Some("Begin-of-stream with voice settings".to_string()),
                    }),
                    close: Some(MessageSchema {
                        name: "EOS".to_string(),
                        direction: MessageDirection::Client,
                        schema: Schema::new("TtsCloseMessage"),
                        description: Some("End-of-stream signal".to_string()),
                    }),
                    keepalive: None,
                },
                messages: vec![
                    MessageSchema {
                        name: "TextChunk".to_string(),
                        direction: MessageDirection::Client,
                        schema: Schema::new("TtsTextMessage"),
                        description: Some("Text to synthesize".to_string()),
                    },
                    MessageSchema {
                        name: "AudioChunk".to_string(),
                        direction: MessageDirection::Server,
                        schema: Schema::new("TtsAudioResponse"),
                        description: Some("Audio data with alignment".to_string()),
                    },
                ],
                runtime: None,
            },
            WebSocketEndpoint {
                id: "MultiContextTextToSpeech".to_string(),
                path: "/v1/text-to-speech/{voice_id}/multi-stream-input".to_string(),
                description: "Manage multiple audio streams over a single connection".to_string(),
                connection_params: vec![
                    ConnectionParam {
                        name: "voice_id".to_string(),
                        param_type: ParamType::String,
                        required: true,
                        description: Some(
                            "Voice identifier used in the websocket path".to_string(),
                        ),
                    },
                    ConnectionParam {
                        name: "model_id".to_string(),
                        param_type: ParamType::String,
                        required: false,
                        description: Some("TTS model to use".to_string()),
                    },
                    ConnectionParam {
                        name: "output_format".to_string(),
                        param_type: ParamType::String,
                        required: false,
                        description: Some("Audio output format".to_string()),
                    },
                ],
                lifecycle: ConnectionLifecycle {
                    open: Some(MessageSchema {
                        name: "InitContext".to_string(),
                        direction: MessageDirection::Client,
                        schema: Schema::new("MultiContextInitMessage"),
                        description: Some("Initialize a context with settings".to_string()),
                    }),
                    close: Some(MessageSchema {
                        name: "CloseSocket".to_string(),
                        direction: MessageDirection::Client,
                        schema: Schema::new("MultiContextCloseSocketMessage"),
                        description: Some("Close the entire socket".to_string()),
                    }),
                    keepalive: None,
                },
                messages: vec![
                    MessageSchema {
                        name: "TextChunk".to_string(),
                        direction: MessageDirection::Client,
                        schema: Schema::new("MultiContextTextMessage"),
                        description: Some("Text to synthesize for a context".to_string()),
                    },
                    MessageSchema {
                        name: "CloseContext".to_string(),
                        direction: MessageDirection::Client,
                        schema: Schema::new("MultiContextCloseMessage"),
                        description: Some("Close a specific context".to_string()),
                    },
                    MessageSchema {
                        name: "AudioChunk".to_string(),
                        direction: MessageDirection::Server,
                        schema: Schema::new("MultiContextAudioResponse"),
                        description: Some("Audio data for a context".to_string()),
                    },
                ],
                runtime: None,
            },
        ],
        runtime: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use schematic_define::RestMethod;

    #[test]
    fn rest_api_has_correct_metadata() {
        let api = define_elevenlabs_rest_api();

        assert_eq!(api.name, "ElevenLabs");
        assert_eq!(api.base_url, "https://api.elevenlabs.io");
        assert!(api.docs_url.is_some());
    }

    #[test]
    fn rest_api_uses_api_key_auth() {
        let api = define_elevenlabs_rest_api();

        match &api.auth {
            AuthStrategy::ApiKey { header, .. } => {
                assert_eq!(header, "xi-api-key");
            }
            _ => panic!("Expected ApiKey auth strategy"),
        }
        assert!(api.env_auth.contains(&"ELEVEN_LABS_API_KEY".to_string()));
        assert!(api.env_auth.contains(&"ELEVENLABS_API_KEY".to_string()));
    }

    #[test]
    fn rest_api_has_minimum_endpoints() {
        let api = define_elevenlabs_rest_api();
        assert!(
            api.endpoints.len() >= 35,
            "Expected at least 35 endpoints, got {}",
            api.endpoints.len()
        );
    }

    #[test]
    fn tts_endpoints_exist() {
        let api = define_elevenlabs_rest_api();

        let tts_endpoints = [
            "CreateSpeech",
            "StreamSpeech",
            "CreateSpeechWithTimestamps",
            "StreamSpeechWithTimestamps",
        ];

        for id in &tts_endpoints {
            assert!(
                api.endpoints.iter().any(|e| &e.id == id),
                "Missing TTS endpoint: {}",
                id
            );
        }
    }

    #[test]
    fn voice_endpoints_exist() {
        let api = define_elevenlabs_rest_api();

        let voice_endpoints = [
            "ListVoices",
            "GetVoice",
            "DeleteVoice",
            "GetDefaultVoiceSettings",
            "GetVoiceSettings",
            "UpdateVoiceSettings",
        ];

        for id in &voice_endpoints {
            assert!(
                api.endpoints.iter().any(|e| &e.id == id),
                "Missing voice endpoint: {}",
                id
            );
        }
    }

    #[test]
    fn service_account_endpoints_use_correct_methods() {
        let api = define_elevenlabs_rest_api();

        let update_key = api
            .endpoints
            .iter()
            .find(|e| e.id == "UpdateApiKey")
            .expect("UpdateApiKey endpoint missing");
        assert_eq!(update_key.method, RestMethod::Patch);

        let delete_key = api
            .endpoints
            .iter()
            .find(|e| e.id == "DeleteApiKey")
            .expect("DeleteApiKey endpoint missing");
        assert_eq!(delete_key.method, RestMethod::Delete);
    }

    #[test]
    fn webhook_endpoints_use_correct_methods() {
        let api = define_elevenlabs_rest_api();

        let update_webhook = api
            .endpoints
            .iter()
            .find(|e| e.id == "UpdateWebhook")
            .expect("UpdateWebhook endpoint missing");
        assert_eq!(update_webhook.method, RestMethod::Patch);
    }

    #[test]
    fn binary_response_endpoints() {
        let api = define_elevenlabs_rest_api();

        let binary_endpoints = [
            "CreateSpeech",
            "StreamSpeech",
            "GetVoiceSampleAudio",
            "CreateSoundEffect",
            "GetHistoryItemAudio",
            "DownloadHistoryItems",
        ];

        for id in &binary_endpoints {
            let endpoint = api
                .endpoints
                .iter()
                .find(|e| &e.id == id)
                .unwrap_or_else(|| panic!("Missing endpoint: {}", id));
            assert!(
                matches!(endpoint.response, ApiResponse::Binary),
                "Endpoint {} should have Binary response",
                id
            );
        }
    }

    #[test]
    fn add_voice_sample_uses_form_data() {
        use schematic_define::{ApiRequest, FormFieldKind};

        let api = define_elevenlabs_rest_api();

        let add_sample = api
            .endpoints
            .iter()
            .find(|e| e.id == "AddVoiceSample")
            .expect("AddVoiceSample endpoint missing");

        assert_eq!(add_sample.method, RestMethod::Post);
        assert!(add_sample.path.contains("/samples"));

        if let Some(ApiRequest::FormData { fields }) = &add_sample.request {
            assert_eq!(fields.len(), 2);

            assert_eq!(fields[0].name, "audio");
            assert!(fields[0].required);
            assert!(matches!(fields[0].kind, FormFieldKind::File { .. }));

            assert_eq!(fields[1].name, "name");
            assert!(!fields[1].required);
        } else {
            panic!("AddVoiceSample should have FormData request");
        }
    }

    #[test]
    fn websocket_api_has_correct_metadata() {
        let api = define_elevenlabs_websocket_api();

        assert_eq!(api.name, "ElevenLabsTTS");
        assert_eq!(api.base_url, "wss://api.elevenlabs.io");
        assert!(api.docs_url.is_some());
    }

    #[test]
    fn websocket_api_has_two_endpoints() {
        let api = define_elevenlabs_websocket_api();
        assert_eq!(api.endpoints.len(), 2);
    }

    #[test]
    fn websocket_tts_endpoint_has_lifecycle() {
        let api = define_elevenlabs_websocket_api();

        let tts = api
            .endpoints
            .iter()
            .find(|e| e.id == "TextToSpeech")
            .expect("TextToSpeech endpoint missing");

        assert!(tts.lifecycle.open.is_some());
        assert!(tts.lifecycle.close.is_some());
    }

    #[test]
    fn websocket_multi_context_endpoint_exists() {
        let api = define_elevenlabs_websocket_api();

        let multi = api
            .endpoints
            .iter()
            .find(|e| e.id == "MultiContextTextToSpeech")
            .expect("MultiContextTextToSpeech endpoint missing");

        assert!(multi.path.contains("multi-stream-input"));
    }
}
