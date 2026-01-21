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

mod types;

pub use types::*;

use schematic_define::{
    ApiRequest, ApiResponse, AuthStrategy, Endpoint, FormField, RestApi, RestMethod, Schema,
};

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
    RestApi {
        name: "ElevenLabs".to_string(),
        description: "ElevenLabs Creative Platform API for text-to-speech, voice management, and sound generation".to_string(),
        base_url: "https://api.elevenlabs.io".to_string(),
        docs_url: Some("https://elevenlabs.io/docs/api-reference".to_string()),
        auth: AuthStrategy::ApiKey {
            header: "xi-api-key".to_string(),
        },
        env_auth: vec![
            "ELEVEN_LABS_API_KEY".to_string(),
            "ELEVENLABS_API_KEY".to_string(),
        ],
        env_username: None,
        headers: vec![],
        endpoints: vec![
            // =================================================================
            // Text-to-Speech Endpoints
            // =================================================================
            Endpoint {
                id: "CreateSpeech".to_string(),
                method: RestMethod::Post,
                path: "/v1/text-to-speech/{voice_id}".to_string(),
                description: "Converts text into speech and returns audio".to_string(),
                request: Some(ApiRequest::json_type("CreateSpeechBody")),
                response: ApiResponse::Binary,
                headers: vec![],
            },
            Endpoint {
                id: "StreamSpeech".to_string(),
                method: RestMethod::Post,
                path: "/v1/text-to-speech/{voice_id}/stream".to_string(),
                description: "Streams audio as it's generated".to_string(),
                request: Some(ApiRequest::json_type("CreateSpeechBody")),
                response: ApiResponse::Binary,
                headers: vec![],
            },
            Endpoint {
                id: "CreateSpeechWithTimestamps".to_string(),
                method: RestMethod::Post,
                path: "/v1/text-to-speech/{voice_id}/with-timestamps".to_string(),
                description: "Returns audio with character-level timing information".to_string(),
                request: Some(ApiRequest::json_type("CreateSpeechBody")),
                response: ApiResponse::json_type("SpeechWithTimestampsResponse"),
                headers: vec![],
            },
            Endpoint {
                id: "StreamSpeechWithTimestamps".to_string(),
                method: RestMethod::Post,
                path: "/v1/text-to-speech/{voice_id}/stream/with-timestamps".to_string(),
                description: "Streams audio chunks with timing information".to_string(),
                request: Some(ApiRequest::json_type("CreateSpeechBody")),
                response: ApiResponse::json_type("SpeechWithTimestampsResponse"),
                headers: vec![],
            },

            // =================================================================
            // Voice Management Endpoints
            // =================================================================
            Endpoint {
                id: "ListVoices".to_string(),
                method: RestMethod::Get,
                path: "/v2/voices".to_string(),
                description: "Lists all available voices".to_string(),
                request: None,
                response: ApiResponse::json_type("ListVoicesResponse"),
                headers: vec![],
            },
            Endpoint {
                id: "GetVoice".to_string(),
                method: RestMethod::Get,
                path: "/v1/voices/{voice_id}".to_string(),
                description: "Retrieves a voice by ID".to_string(),
                request: None,
                response: ApiResponse::json_type("VoiceResponseModel"),
                headers: vec![],
            },
            Endpoint {
                id: "DeleteVoice".to_string(),
                method: RestMethod::Delete,
                path: "/v1/voices/{voice_id}".to_string(),
                description: "Deletes a voice".to_string(),
                request: None,
                response: ApiResponse::json_type("StatusResponse"),
                headers: vec![],
            },

            // =================================================================
            // Voice Settings Endpoints
            // =================================================================
            Endpoint {
                id: "GetDefaultVoiceSettings".to_string(),
                method: RestMethod::Get,
                path: "/v1/voices/settings/default".to_string(),
                description: "Gets default voice settings".to_string(),
                request: None,
                response: ApiResponse::json_type("VoiceSettings"),
                headers: vec![],
            },
            Endpoint {
                id: "GetVoiceSettings".to_string(),
                method: RestMethod::Get,
                path: "/v1/voices/{voice_id}/settings".to_string(),
                description: "Gets voice settings for a specific voice".to_string(),
                request: None,
                response: ApiResponse::json_type("VoiceSettings"),
                headers: vec![],
            },
            Endpoint {
                id: "UpdateVoiceSettings".to_string(),
                method: RestMethod::Post,
                path: "/v1/voices/{voice_id}/settings/edit".to_string(),
                description: "Updates voice settings".to_string(),
                request: Some(ApiRequest::json_type("VoiceSettings")),
                response: ApiResponse::json_type("StatusResponse"),
                headers: vec![],
            },

            // =================================================================
            // Voice Samples Endpoints
            // =================================================================
            Endpoint {
                id: "GetVoiceSampleAudio".to_string(),
                method: RestMethod::Get,
                path: "/v1/voices/{voice_id}/samples/{sample_id}/audio".to_string(),
                description: "Gets audio for a voice sample".to_string(),
                request: None,
                response: ApiResponse::Binary,
                headers: vec![],
            },
            Endpoint {
                id: "DeleteVoiceSample".to_string(),
                method: RestMethod::Delete,
                path: "/v1/voices/{voice_id}/samples/{sample_id}".to_string(),
                description: "Deletes a voice sample".to_string(),
                request: None,
                response: ApiResponse::json_type("StatusResponse"),
                headers: vec![],
            },
            Endpoint {
                id: "AddVoiceSample".to_string(),
                method: RestMethod::Post,
                path: "/v1/voices/{voice_id}/samples".to_string(),
                description: "Upload audio sample for voice cloning".to_string(),
                request: Some(ApiRequest::form_data(vec![
                    FormField::file_accept("audio", vec!["audio/*".into()])
                        .with_description("Audio file (mp3, wav, ogg, m4a)"),
                    FormField::text("name")
                        .optional()
                        .with_description("Name for the sample"),
                ])),
                response: ApiResponse::json_type("AddSampleResponse"),
                headers: vec![],
            },

            // =================================================================
            // Voice Library Endpoints
            // =================================================================
            Endpoint {
                id: "ListSharedVoices".to_string(),
                method: RestMethod::Get,
                path: "/v1/shared-voices".to_string(),
                description: "Lists voices from the public library".to_string(),
                request: None,
                response: ApiResponse::json_type("ListSharedVoicesResponse"),
                headers: vec![],
            },
            Endpoint {
                id: "AddSharedVoice".to_string(),
                method: RestMethod::Post,
                path: "/v1/voices/add/{public_user_id}/{voice_id}".to_string(),
                description: "Adds a shared voice to your library".to_string(),
                request: Some(ApiRequest::json_type("AddSharedVoiceBody")),
                response: ApiResponse::json_type("AddSharedVoiceResponse"),
                headers: vec![],
            },

            // =================================================================
            // Professional Voice Cloning (PVC) Endpoints
            // =================================================================
            Endpoint {
                id: "CreatePvcVoice".to_string(),
                method: RestMethod::Post,
                path: "/v1/voices/pvc".to_string(),
                description: "Creates a professional voice clone".to_string(),
                request: Some(ApiRequest::json_type("CreatePvcVoiceBody")),
                response: ApiResponse::json_type("AddSharedVoiceResponse"),
                headers: vec![],
            },
            Endpoint {
                id: "UpdatePvcVoice".to_string(),
                method: RestMethod::Post,
                path: "/v1/voices/pvc/{voice_id}".to_string(),
                description: "Updates a PVC voice".to_string(),
                request: Some(ApiRequest::json_type("CreatePvcVoiceBody")),
                response: ApiResponse::json_type("StatusResponse"),
                headers: vec![],
            },
            Endpoint {
                id: "TrainPvcVoice".to_string(),
                method: RestMethod::Post,
                path: "/v1/voices/pvc/{voice_id}/train".to_string(),
                description: "Starts training a PVC voice".to_string(),
                request: Some(ApiRequest::json_type("TrainPvcVoiceBody")),
                response: ApiResponse::json_type("StatusResponse"),
                headers: vec![],
            },

            // =================================================================
            // Sound Effects Endpoint
            // =================================================================
            Endpoint {
                id: "CreateSoundEffect".to_string(),
                method: RestMethod::Post,
                path: "/v1/sound-generation".to_string(),
                description: "Generates a sound effect from text".to_string(),
                request: Some(ApiRequest::json_type("CreateSoundEffectBody")),
                response: ApiResponse::Binary,
                headers: vec![],
            },

            // =================================================================
            // Models Endpoint
            // =================================================================
            Endpoint {
                id: "ListModels".to_string(),
                method: RestMethod::Get,
                path: "/v1/models".to_string(),
                description: "Lists all available models".to_string(),
                request: None,
                response: ApiResponse::json_type("Vec<ModelInfo>"),
                headers: vec![],
            },

            // =================================================================
            // Single-Use Tokens Endpoint
            // =================================================================
            Endpoint {
                id: "CreateSingleUseToken".to_string(),
                method: RestMethod::Post,
                path: "/v1/single-use-token/{token_type}".to_string(),
                description: "Creates a single-use token for WebSocket auth".to_string(),
                request: None,
                response: ApiResponse::json_type("SingleUseTokenResponse"),
                headers: vec![],
            },

            // =================================================================
            // History Endpoints
            // =================================================================
            Endpoint {
                id: "GetHistory".to_string(),
                method: RestMethod::Get,
                path: "/v1/history".to_string(),
                description: "Gets generated items history".to_string(),
                request: None,
                response: ApiResponse::json_type("GetHistoryResponse"),
                headers: vec![],
            },
            Endpoint {
                id: "GetHistoryItem".to_string(),
                method: RestMethod::Get,
                path: "/v1/history/{history_item_id}".to_string(),
                description: "Gets a specific history item".to_string(),
                request: None,
                response: ApiResponse::json_type("SpeechHistoryItemResponseModel"),
                headers: vec![],
            },
            Endpoint {
                id: "DeleteHistoryItem".to_string(),
                method: RestMethod::Delete,
                path: "/v1/history/{history_item_id}".to_string(),
                description: "Deletes a history item".to_string(),
                request: None,
                response: ApiResponse::json_type("StatusResponse"),
                headers: vec![],
            },
            Endpoint {
                id: "GetHistoryItemAudio".to_string(),
                method: RestMethod::Get,
                path: "/v1/history/{history_item_id}/audio".to_string(),
                description: "Gets audio for a history item".to_string(),
                request: None,
                response: ApiResponse::Binary,
                headers: vec![],
            },
            Endpoint {
                id: "DownloadHistoryItems".to_string(),
                method: RestMethod::Post,
                path: "/v1/history/download".to_string(),
                description: "Downloads multiple history items as ZIP".to_string(),
                request: Some(ApiRequest::json_type("DownloadHistoryBody")),
                response: ApiResponse::Binary,
                headers: vec![],
            },

            // =================================================================
            // Workspace - Usage Statistics
            // =================================================================
            Endpoint {
                id: "GetUsageStats".to_string(),
                method: RestMethod::Get,
                path: "/v1/usage/character-stats".to_string(),
                description: "Gets usage statistics".to_string(),
                request: None,
                response: ApiResponse::json_type("UsageStatsResponse"),
                headers: vec![],
            },

            // =================================================================
            // Workspace - User Information
            // =================================================================
            Endpoint {
                id: "GetUser".to_string(),
                method: RestMethod::Get,
                path: "/v1/user".to_string(),
                description: "Gets current user information".to_string(),
                request: None,
                response: ApiResponse::json_type("UserResponse"),
                headers: vec![],
            },
            Endpoint {
                id: "GetUserSubscription".to_string(),
                method: RestMethod::Get,
                path: "/v1/user/subscription".to_string(),
                description: "Gets user subscription information".to_string(),
                request: None,
                response: ApiResponse::json_type("SubscriptionModel"),
                headers: vec![],
            },

            // =================================================================
            // Workspace - Resources
            // =================================================================
            Endpoint {
                id: "GetResource".to_string(),
                method: RestMethod::Get,
                path: "/v1/workspace/resources/{resource_id}".to_string(),
                description: "Gets resource information".to_string(),
                request: None,
                response: ApiResponse::json_type("ResourceResponse"),
                headers: vec![],
            },
            Endpoint {
                id: "ShareResource".to_string(),
                method: RestMethod::Post,
                path: "/v1/workspace/resources/{resource_id}/share".to_string(),
                description: "Shares a resource".to_string(),
                request: Some(ApiRequest::json_type("ShareResourceBody")),
                response: ApiResponse::json_type("StatusResponse"),
                headers: vec![],
            },
            Endpoint {
                id: "UnshareResource".to_string(),
                method: RestMethod::Post,
                path: "/v1/workspace/resources/{resource_id}/unshare".to_string(),
                description: "Removes sharing for a resource".to_string(),
                request: Some(ApiRequest::json_type("UnshareResourceBody")),
                response: ApiResponse::json_type("StatusResponse"),
                headers: vec![],
            },
            Endpoint {
                id: "CopyResourceToWorkspace".to_string(),
                method: RestMethod::Post,
                path: "/v1/workspace/resources/{resource_id}/copy-to-workspace".to_string(),
                description: "Copies a resource to another workspace".to_string(),
                request: Some(ApiRequest::json_type("CopyResourceBody")),
                response: ApiResponse::json_type("StatusResponse"),
                headers: vec![],
            },

            // =================================================================
            // Service Accounts Endpoints
            // =================================================================
            Endpoint {
                id: "ListServiceAccounts".to_string(),
                method: RestMethod::Get,
                path: "/v1/service-accounts".to_string(),
                description: "Lists service accounts".to_string(),
                request: None,
                response: ApiResponse::json_type("ListServiceAccountsResponse"),
                headers: vec![],
            },
            Endpoint {
                id: "ListServiceAccountApiKeys".to_string(),
                method: RestMethod::Get,
                path: "/v1/service-accounts/{service_account_user_id}/api-keys".to_string(),
                description: "Lists API keys for a service account".to_string(),
                request: None,
                response: ApiResponse::json_type("ListApiKeysResponse"),
                headers: vec![],
            },
            Endpoint {
                id: "CreateApiKey".to_string(),
                method: RestMethod::Post,
                path: "/v1/service-accounts/{service_account_user_id}/api-keys".to_string(),
                description: "Creates an API key for a service account".to_string(),
                request: Some(ApiRequest::json_type("CreateApiKeyBody")),
                response: ApiResponse::json_type("CreateApiKeyResponse"),
                headers: vec![],
            },
            Endpoint {
                id: "UpdateApiKey".to_string(),
                method: RestMethod::Patch,
                path: "/v1/service-accounts/{service_account_user_id}/api-keys/{api_key_id}".to_string(),
                description: "Updates an API key".to_string(),
                request: Some(ApiRequest::json_type("UpdateApiKeyBody")),
                response: ApiResponse::json_type("StatusResponse"),
                headers: vec![],
            },
            Endpoint {
                id: "DeleteApiKey".to_string(),
                method: RestMethod::Delete,
                path: "/v1/service-accounts/{service_account_user_id}/api-keys/{api_key_id}".to_string(),
                description: "Deletes an API key".to_string(),
                request: None,
                response: ApiResponse::json_type("StatusResponse"),
                headers: vec![],
            },

            // =================================================================
            // Webhooks Endpoints
            // =================================================================
            Endpoint {
                id: "ListWebhooks".to_string(),
                method: RestMethod::Get,
                path: "/v1/workspace/webhooks".to_string(),
                description: "Lists webhooks".to_string(),
                request: None,
                response: ApiResponse::json_type("ListWebhooksResponse"),
                headers: vec![],
            },
            Endpoint {
                id: "CreateWebhook".to_string(),
                method: RestMethod::Post,
                path: "/v1/workspace/webhooks".to_string(),
                description: "Creates a webhook".to_string(),
                request: Some(ApiRequest::json_type("CreateWebhookBody")),
                response: ApiResponse::json_type("CreateWebhookResponse"),
                headers: vec![],
            },
            Endpoint {
                id: "UpdateWebhook".to_string(),
                method: RestMethod::Patch,
                path: "/v1/workspace/webhooks/{webhook_id}".to_string(),
                description: "Updates a webhook".to_string(),
                request: Some(ApiRequest::json_type("UpdateWebhookBody")),
                response: ApiResponse::json_type("StatusResponse"),
                headers: vec![],
            },
            Endpoint {
                id: "DeleteWebhook".to_string(),
                method: RestMethod::Delete,
                path: "/v1/workspace/webhooks/{webhook_id}".to_string(),
                description: "Deletes a webhook".to_string(),
                request: None,
                response: ApiResponse::json_type("StatusResponse"),
                headers: vec![],
            },
        ],
        module_path: None,
        request_suffix: None,
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
        },
        env_auth: vec![
            "ELEVEN_LABS_API_KEY".to_string(),
            "ELEVENLABS_API_KEY".to_string(),
        ],
        endpoints: vec![
            WebSocketEndpoint {
                id: "TextToSpeech".to_string(),
                path: "/v1/text-to-speech/{voice_id}/stream-input".to_string(),
                description: "Stream text and receive audio chunks in real-time".to_string(),
                connection_params: vec![
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
            },
            WebSocketEndpoint {
                id: "MultiContextTextToSpeech".to_string(),
                path: "/v1/text-to-speech/{voice_id}/multi-stream-input".to_string(),
                description: "Manage multiple audio streams over a single connection".to_string(),
                connection_params: vec![
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
            },
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use schematic_define::RestMethod;

    // =========================================================================
    // REST API Tests
    // =========================================================================

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
            AuthStrategy::ApiKey { header } => {
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
        // Plan specifies 35+ endpoints
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

        // Verify it's a form-data request with file upload
        if let Some(ApiRequest::FormData { fields }) = &add_sample.request {
            assert_eq!(fields.len(), 2);

            // First field should be the audio file
            assert_eq!(fields[0].name, "audio");
            assert!(fields[0].required);
            assert!(matches!(fields[0].kind, FormFieldKind::File { .. }));

            // Second field should be optional name
            assert_eq!(fields[1].name, "name");
            assert!(!fields[1].required);
        } else {
            panic!("AddVoiceSample should have FormData request");
        }
    }

    // =========================================================================
    // WebSocket API Tests
    // =========================================================================

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
