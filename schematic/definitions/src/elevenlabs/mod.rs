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

use crate::registry::assemble_module_group;
use schematic_define::{DefinitionContext, DefinitionError};
#[cfg(test)]
use schematic_define::ResponseShape;
use schematic_define::{
    ApiKeyEnv, AuthStrategy, CrossOrigin, Endpoint, EnvList, EnvMapping, RestApi, Schema,
    pagination::{Collection, CursorSource, MoreSignal, PageSize, Strategy},
};

/// Assemble the APIs and ordered schemas in one destination context.
pub fn module_group() -> Result<Vec<RestApi>, DefinitionError> {
    assemble_module_group("::schematic_definitions::elevenlabs", |ctx| {
        ctx.register::<CreateSpeechBody>()?;
        ctx.register::<AddSharedVoiceBody>()?;
        ctx.register::<CreatePvcVoiceBody>()?;
        ctx.register::<TrainPvcVoiceBody>()?;
        ctx.register::<CreateSoundEffectBody>()?;
        ctx.register::<DownloadHistoryBody>()?;
        ctx.register::<ShareResourceBody>()?;
        ctx.register::<UnshareResourceBody>()?;
        ctx.register::<CopyResourceBody>()?;
        ctx.register::<CreateApiKeyBody>()?;
        ctx.register::<UpdateApiKeyBody>()?;
        ctx.register::<CreateWebhookBody>()?;
        ctx.register::<UpdateWebhookBody>()?;
        ctx.register::<SpeechWithTimestampsResponse>()?;
        ctx.register::<ListVoicesResponse>()?;
        ctx.register::<VoiceResponseModel>()?;
        ctx.register::<StatusResponse>()?;
        ctx.register::<VoiceSettings>()?;
        ctx.register::<AddSampleResponse>()?;
        ctx.register::<ListSharedVoicesResponse>()?;
        ctx.register::<AddSharedVoiceResponse>()?;
        ctx.register::<Vec<ModelInfo>>()?;
        ctx.register::<SingleUseTokenResponse>()?;
        ctx.register::<GetHistoryResponse>()?;
        ctx.register::<SpeechHistoryItemResponseModel>()?;
        ctx.register::<UsageStatsResponse>()?;
        ctx.register::<UserResponse>()?;
        ctx.register::<SubscriptionModel>()?;
        ctx.register::<ResourceResponse>()?;
        ctx.register::<ListServiceAccountsResponse>()?;
        ctx.register::<ListApiKeysResponse>()?;
        ctx.register::<CreateApiKeyResponse>()?;
        ctx.register::<ListWebhooksResponse>()?;
        ctx.register::<CreateWebhookResponse>()?;
        Ok(vec![build_elevenlabs_rest_api(ctx)?])
    })
}

/// History's cursor and shared voices' page number are independent provider strategies.
fn history_collection() -> Collection {
    Collection::at("history")
        .item_type("SpeechHistoryItemResponseModel")
        .paginate(Strategy::cursor("start_after_history_item_id", CursorSource::body("last_history_item_id"))
            .more_when(MoreSignal::has_more("has_more"))
            .page_size(PageSize::new("page_size").server_default(100).max(1000))
            .resumable())
}

fn shared_voices_collection() -> Collection {
    Collection::at("voices")
        .item_type("LibraryVoiceResponseModel")
        .paginate(Strategy::page_number("page", MoreSignal::has_more("has_more"))
            .zero_based()
            .page_size(PageSize::new("page_size").server_default(30).max(100)))
}

fn voices_collection() -> Collection {
    // The definition sends `page_token`, while the reference names `next_page_token`.
    Collection::at("voices").item_type("VoiceResponseModel").unpaginated()
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
    module_group().expect("valid shipped API").remove(0)
}

fn build_elevenlabs_rest_api(ctx: &mut DefinitionContext) -> Result<RestApi, DefinitionError> {
    let mut endpoints: Vec<Endpoint> = vec![];
    endpoints.extend(endpoints::speech::all(ctx)?);
    endpoints.extend(endpoints::voices::all(ctx)?);
    endpoints.extend(endpoints::sound_generation::all(ctx)?);
    endpoints.extend(endpoints::history::all(ctx)?);
    endpoints.extend(endpoints::workspace::all(ctx)?);

    {
        RestApi::builder(ctx, "ElevenLabs", "https://api.elevenlabs.io")
            .describes("ElevenLabs Creative Platform API for text-to-speech, voice management, and sound generation")
            .authentication(AuthStrategy::ApiKey {
            header: "xi-api-key".to_string(),
            value_prefix: None,
        }, vec![
            "ELEVEN_LABS_API_KEY".to_string(),
            "ELEVENLABS_API_KEY".to_string(),
        ])
            .env_mapping(EnvMapping {
            api_key: Some(ApiKeyEnv {
                names: EnvList::new(vec![
                    "ELEVEN_LABS_API_KEY".to_string(),
                    "ELEVENLABS_API_KEY".to_string(),
                ]),
                header: "xi-api-key".to_string(),
            }),
            ..Default::default()
        })
            .docs_url("https://elevenlabs.io/docs/api-reference".to_string())
            .cross_origin(CrossOrigin::same_origin())
            .endpoints(endpoints)
            .build()
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
    use schematic_define::{EndpointParams, ParamDef, QueryParamType};

    let optional = |name: &str, param_type: QueryParamType, description: &str| {
        ParamDef::new(name, param_type).describes(description)
    };
    let voice_id = || {
        ParamDef::path("voice_id", QueryParamType::String)
            .describes("Voice identifier used in the websocket path")
    };
    let message = |name: &str, direction: MessageDirection, schema: &str, description: &str| {
        MessageSchema::new(name, direction, Schema::new(schema)).describe(description)
    };

    WebSocketApi {
        name: "ElevenLabsTTS".to_string(),
        description: "ElevenLabs Text-to-Speech WebSocket API for real-time streaming".to_string(),
        base_url: "wss://api.elevenlabs.io".to_string(),
        docs_url: Some("https://elevenlabs.io/docs/api-reference/websockets".to_string()),
        auth: AuthStrategy::ApiKey {
            header: "xi-api-key".to_string(),
            value_prefix: None,
        },
        auth_policy: None,
        env_auth: vec![
            "ELEVEN_LABS_API_KEY".to_string(),
            "ELEVENLABS_API_KEY".to_string(),
        ],
        env_mapping: Some(EnvMapping {
            api_key: Some(ApiKeyEnv {
                names: EnvList::from_strs(&["ELEVEN_LABS_API_KEY", "ELEVENLABS_API_KEY"]),
                header: "xi-api-key".to_string(),
            }),
            ..Default::default()
        }),
        headers: vec![],
        version: None,
        endpoints: vec![
            WebSocketEndpoint {
                id: "TextToSpeech".to_string(),
                path: "/v1/text-to-speech/{voice_id}/stream-input".to_string(),
                description: "Stream text and receive audio chunks in real-time".to_string(),
                params: EndpointParams {
                    path: vec![voice_id()],
                    query: vec![
                        optional("model_id", QueryParamType::String, "TTS model to use"),
                        optional("language_code", QueryParamType::String, "Target language code"),
                        optional("enable_logging", QueryParamType::Boolean, "Enable logging (default: true)"),
                        optional("enable_ssml_parsing", QueryParamType::Boolean, "Parse SSML tags"),
                        optional("output_format", QueryParamType::String, "Audio output format"),
                        optional("inactivity_timeout", QueryParamType::Integer, "Timeout in seconds (default: 20)"),
                        optional("sync_alignment", QueryParamType::Boolean, "Synchronize alignment data"),
                        optional("auto_mode", QueryParamType::Boolean, "Auto mode for generation"),
                        optional("apply_text_normalization", QueryParamType::String, "Text normalization: auto, on, off"),
                        optional("seed", QueryParamType::Integer, "Reproducibility seed"),
                    ],
                    ..EndpointParams::default()
                },
                lifecycle: ConnectionLifecycle {
                    open: Some(message("BOS", MessageDirection::Client, "TtsInitMessage",
                        "Begin-of-stream with voice settings")),
                    close: Some(message("EOS", MessageDirection::Client, "TtsCloseMessage",
                        "End-of-stream signal")),
                    keepalive: None,
                    // The begin-of-stream message may also carry the API key.
                    open_credential_field: Some("xi-api-key".to_string()),
                },
                // One inbound message: no selector is needed to choose it.
                messages: vec![
                    message("TextChunk", MessageDirection::Client, "TtsTextMessage",
                        "Text to synthesize"),
                    message("AudioChunk", MessageDirection::Server, "TtsAudioResponse",
                        "Audio data with alignment"),
                ],
                ..WebSocketEndpoint::default()
            },
            WebSocketEndpoint {
                id: "MultiContextTextToSpeech".to_string(),
                path: "/v1/text-to-speech/{voice_id}/multi-stream-input".to_string(),
                description: "Manage multiple audio streams over a single connection".to_string(),
                params: EndpointParams {
                    path: vec![voice_id()],
                    query: vec![
                        optional("model_id", QueryParamType::String, "TTS model to use"),
                        optional("output_format", QueryParamType::String, "Audio output format"),
                    ],
                    ..EndpointParams::default()
                },
                lifecycle: ConnectionLifecycle {
                    open: Some(message("InitContext", MessageDirection::Client,
                        "MultiContextInitMessage", "Initialize a context with settings")),
                    close: Some(message("CloseSocket", MessageDirection::Client,
                        "MultiContextCloseSocketMessage", "Close the entire socket")),
                    keepalive: None,
                    open_credential_field: None,
                },
                messages: vec![
                    message("TextChunk", MessageDirection::Client, "MultiContextTextMessage",
                        "Text to synthesize for a context"),
                    message("CloseContext", MessageDirection::Client, "MultiContextCloseMessage",
                        "Close a specific context"),
                    message("AudioChunk", MessageDirection::Server, "MultiContextAudioResponse",
                        "Audio data for a context"),
                ],
                ..WebSocketEndpoint::default()
            },
        ],
        runtime: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use schematic_define::{RestMethod, StreamEnd, StreamFraming, TypeRef};

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

    fn endpoint<'a>(api: &'a RestApi, id: &str) -> &'a Endpoint {
        api.endpoints
            .iter()
            .find(|e| e.id == id)
            .unwrap_or_else(|| panic!("Missing endpoint: {id}"))
    }

    #[test]
    fn audio_endpoints_store_one_binary_shape_without_content_type() {
        let api = define_elevenlabs_rest_api();

        for id in [
            "CreateSpeech",
            "GetVoiceSampleAudio",
            "CreateSoundEffect",
            "GetHistoryItemAudio",
            "DownloadHistoryItems",
        ] {
            let response = &endpoint(&api, id).response;
            assert_eq!(response.len(), 1, "{id} should declare one shape");
            assert_eq!(response[0].shape, ResponseShape::Binary, "{id}");
            assert_eq!(response[0].content_type, None, "{id}");
        }
    }

    #[test]
    fn stream_speech_stores_one_byte_stream_without_content_type() {
        let api = define_elevenlabs_rest_api();

        let response = &endpoint(&api, "StreamSpeech").response;
        assert_eq!(response.len(), 1);
        assert_eq!(
            response[0].shape,
            ResponseShape::Stream {
                framing: StreamFraming::Bytes,
                end: StreamEnd::ConnectionClose,
                keep: false,
            }
        );
        assert_eq!(response[0].content_type, None);
    }

    #[test]
    fn stream_speech_with_timestamps_stores_one_ndjson_stream_ending_at_close() {
        let api = define_elevenlabs_rest_api();

        let response = &endpoint(&api, "StreamSpeechWithTimestamps").response;
        assert_eq!(response.len(), 1);
        assert_eq!(
            response[0].shape,
            ResponseShape::Stream {
                framing: StreamFraming::Ndjson(TypeRef::Named(
                    "SpeechWithTimestampsResponse".into()
                )),
                end: StreamEnd::ConnectionClose,
                keep: false,
            }
        );
        assert_eq!(response[0].content_type, None);
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
    #[test]
    fn typed_module_group_preserves_endpoint_order_and_inline_components() {
        let group = module_group().unwrap();
        let ids = |api: &RestApi| api.endpoints.iter().map(|ep| ep.id.clone()).collect::<Vec<_>>();
        assert_eq!(ids(&group[0]), ["CreateSpeech", "StreamSpeech", "CreateSpeechWithTimestamps", "StreamSpeechWithTimestamps", "ListVoices", "GetVoice", "DeleteVoice", "GetDefaultVoiceSettings", "GetVoiceSettings", "UpdateVoiceSettings", "GetVoiceSampleAudio", "DeleteVoiceSample", "AddVoiceSample", "ListSharedVoices", "AddSharedVoice", "CreatePvcVoice", "UpdatePvcVoice", "TrainPvcVoice", "CreateSoundEffect", "ListModels", "CreateSingleUseToken", "GetHistory", "GetHistoryItem", "DeleteHistoryItem", "GetHistoryItemAudio", "DownloadHistoryItems", "GetUsageStats", "GetUser", "GetUserSubscription", "GetResource", "ShareResource", "UnshareResource", "CopyResourceToWorkspace", "ListServiceAccounts", "ListServiceAccountApiKeys", "CreateApiKey", "UpdateApiKey", "DeleteApiKey", "ListWebhooks", "CreateWebhook", "UpdateWebhook", "DeleteWebhook"]);
        for api in &group {
            api.validate_references().unwrap();
            assert_eq!(api.schemas, group[0].schemas);
        }
        assert!(group[0].component("Vec<ModelInfo>").is_none(), "container component must be inline");
        for id in ["GetHistory", "ListSharedVoices", "ListVoices"] {
            assert!(group[0].endpoints.iter().find(|ep| ep.id == id).unwrap().pagination.is_some());
        }
    }

}
