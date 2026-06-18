mod common;
mod history;
mod speech;
mod voices;
mod websocket;
mod workspace;

pub use common::*;
pub use history::*;
pub use speech::*;
pub use voices::*;
pub use websocket::*;
pub use workspace::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_format_serialization() {
        assert_eq!(
            serde_json::to_string(&OutputFormat::Mp3_44100_128).unwrap(),
            "\"mp3_44100_128\""
        );
        assert_eq!(
            serde_json::to_string(&OutputFormat::Pcm16000).unwrap(),
            "\"pcm_16000\""
        );
    }

    #[test]
    fn output_format_deserialization() {
        let format: OutputFormat = serde_json::from_str("\"mp3_44100_128\"").unwrap();
        assert_eq!(format, OutputFormat::Mp3_44100_128);

        let format: OutputFormat = serde_json::from_str("\"pcm_16000\"").unwrap();
        assert_eq!(format, OutputFormat::Pcm16000);
    }

    #[test]
    fn output_format_default() {
        assert_eq!(OutputFormat::default(), OutputFormat::Mp3_44100_128);
    }

    #[test]
    fn voice_category_serialization() {
        assert_eq!(
            serde_json::to_string(&VoiceCategory::Premade).unwrap(),
            "\"premade\""
        );
        assert_eq!(
            serde_json::to_string(&VoiceCategory::HighQuality).unwrap(),
            "\"high_quality\""
        );
    }

    #[test]
    fn text_normalization_default() {
        assert_eq!(TextNormalization::default(), TextNormalization::Auto);
    }

    #[test]
    fn subscription_status_serialization() {
        assert_eq!(
            serde_json::to_string(&SubscriptionStatus::Active).unwrap(),
            "\"active\""
        );
        assert_eq!(
            serde_json::to_string(&SubscriptionStatus::FreeDisabled).unwrap(),
            "\"free_disabled\""
        );
    }

    #[test]
    fn billing_period_serialization() {
        assert_eq!(
            serde_json::to_string(&BillingPeriod::Monthly).unwrap(),
            "\"monthly\""
        );
        assert_eq!(
            serde_json::to_string(&BillingPeriod::ThreeMonth).unwrap(),
            "\"3-month\""
        );
    }

    #[test]
    fn voice_settings_default() {
        let settings = VoiceSettings::default();
        assert_eq!(settings.stability, 0.5);
        assert_eq!(settings.similarity_boost, 0.75);
        assert!(settings.use_speaker_boost);
    }

    #[test]
    fn voice_settings_roundtrip() {
        let settings = VoiceSettings {
            stability: 0.7,
            similarity_boost: 0.8,
            style: Some(0.5),
            speed: Some(1.1),
            use_speaker_boost: false,
        };

        let json = serde_json::to_string(&settings).unwrap();
        let parsed: VoiceSettings = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.stability, settings.stability);
        assert_eq!(parsed.similarity_boost, settings.similarity_boost);
        assert_eq!(parsed.style, settings.style);
        assert_eq!(parsed.speed, settings.speed);
        assert_eq!(parsed.use_speaker_boost, settings.use_speaker_boost);
    }

    #[test]
    fn http_alignment_roundtrip() {
        let alignment = HttpAlignment {
            characters: vec!["H".into(), "i".into()],
            character_start_times_seconds: vec![0.0, 0.1],
            character_end_times_seconds: vec![0.1, 0.2],
        };

        let json = serde_json::to_string(&alignment).unwrap();
        assert!(json.contains("characterStartTimesSeconds"));

        let parsed: HttpAlignment = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.characters, alignment.characters);
    }

    #[test]
    fn websocket_alignment_roundtrip() {
        let alignment = WebSocketAlignment {
            chars: vec!["H".into(), "i".into()],
            char_start_times_ms: vec![0, 100],
            char_durations_ms: vec![100, 150],
        };

        let json = serde_json::to_string(&alignment).unwrap();
        assert!(json.contains("charStartTimesMs"));

        let parsed: WebSocketAlignment = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.chars, alignment.chars);
    }

    #[test]
    fn pronunciation_dictionary_locator_roundtrip() {
        let locator = PronunciationDictionaryLocator {
            pronunciation_dictionary_id: "dict-123".to_string(),
            version_id: "v1".to_string(),
        };

        let json = serde_json::to_string(&locator).unwrap();
        let parsed: PronunciationDictionaryLocator = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.pronunciation_dictionary_id, "dict-123");
        assert_eq!(parsed.version_id, "v1");
    }

    #[test]
    fn status_response_roundtrip() {
        let response = StatusResponse {
            status: "ok".to_string(),
        };

        let json = serde_json::to_string(&response).unwrap();
        let parsed: StatusResponse = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.status, "ok");
    }

    #[test]
    fn generation_config_default() {
        let config = GenerationConfig::default();
        assert_eq!(config.chunk_length_schedule, Some(vec![120, 160, 250, 290]));
    }

    #[test]
    fn api_permission_serialization() {
        assert_eq!(
            serde_json::to_string(&ApiPermission::TextToSpeech).unwrap(),
            "\"text_to_speech\""
        );
        assert_eq!(
            serde_json::to_string(&ApiPermission::SpeechHistoryRead).unwrap(),
            "\"speech_history_read\""
        );
    }

    #[test]
    fn webhook_auth_type_serialization() {
        assert_eq!(
            serde_json::to_string(&WebhookAuthType::Hmac).unwrap(),
            "\"hmac\""
        );
        assert_eq!(
            serde_json::to_string(&WebhookAuthType::Oauth2).unwrap(),
            "\"oauth2\""
        );
    }

    #[test]
    fn token_type_serialization() {
        assert_eq!(
            serde_json::to_string(&TokenType::RealtimeScribe).unwrap(),
            "\"realtime_scribe\""
        );
        assert_eq!(
            serde_json::to_string(&TokenType::TtsWebsocket).unwrap(),
            "\"tts_websocket\""
        );
    }

    #[test]
    fn fine_tuning_state_deserializes_all_api_values() {
        let api_values = [
            ("\"not_started\"", FineTuningState::NotStarted),
            ("\"queued\"", FineTuningState::Queued),
            ("\"fine_tuning\"", FineTuningState::FineTuning),
            ("\"fine_tuned\"", FineTuningState::FineTuned),
            ("\"failed\"", FineTuningState::Failed),
            ("\"delayed\"", FineTuningState::Delayed),
        ];

        for (json, expected) in api_values {
            let deserialized: FineTuningState = serde_json::from_str(json)
                .unwrap_or_else(|e| panic!("Failed to deserialize {}: {}", json, e));
            assert_eq!(deserialized, expected, "Mismatch for JSON {}", json);
        }
    }

    #[test]
    fn fine_tuning_state_roundtrip() {
        let states = [
            FineTuningState::NotStarted,
            FineTuningState::Queued,
            FineTuningState::FineTuning,
            FineTuningState::FineTuned,
            FineTuningState::Failed,
            FineTuningState::Delayed,
        ];

        for state in states {
            let json = serde_json::to_string(&state).unwrap();
            let roundtrip: FineTuningState = serde_json::from_str(&json).unwrap();
            assert_eq!(roundtrip, state);
        }
    }

    #[test]
    fn safety_control_deserializes_screaming_case_values() {
        let api_values = [
            ("\"NONE\"", SafetyControl::None),
            ("\"BAN\"", SafetyControl::Ban),
            ("\"CAPTCHA\"", SafetyControl::Captcha),
            ("\"ENTERPRISE_BAN\"", SafetyControl::EnterpriseBan),
            ("\"ENTERPRISE_CAPTCHA\"", SafetyControl::EnterpriseCaptcha),
        ];

        for (json, expected) in api_values {
            let deserialized: SafetyControl = serde_json::from_str(json)
                .unwrap_or_else(|e| panic!("Failed to deserialize {}: {}", json, e));
            assert_eq!(deserialized, expected, "Mismatch for JSON {}", json);
        }
    }

    #[test]
    fn voice_response_model_deserializes_with_new_fields() {
        let json = r#"{
            "voice_id": "test123",
            "name": "Test Voice",
            "category": "premade",
            "preview_url": "https://example.com/preview.mp3",
            "available_for_tiers": ["free", "starter", "creator"],
            "high_quality_base_model_ids": ["eleven_multilingual_v2"],
            "collection_ids": ["abc", "def"],
            "is_legacy": false,
            "is_mixed": true,
            "is_owner": true,
            "permission_on_resource": "owner",
            "favorited_at_unix": 1706000000,
            "created_at_unix": 1700000000,
            "safety_control": "NONE",
            "fine_tuning": {
                "state": {
                    "eleven_multilingual_v2": "fine_tuned",
                    "eleven_turbo_v2": "fine_tuned"
                },
                "model_id": "eleven_multilingual_v2",
                "is_allowed_to_fine_tune": true
            }
        }"#;

        let voice: VoiceResponseModel = serde_json::from_str(json)
            .expect("VoiceResponseModel should deserialize all v2 API fields");

        assert_eq!(voice.voice_id, "test123");
        assert_eq!(voice.name, "Test Voice");
        assert_eq!(voice.category, Some(VoiceCategory::Premade));
        assert_eq!(
            voice.preview_url,
            Some("https://example.com/preview.mp3".to_string())
        );
        assert_eq!(
            voice.available_for_tiers,
            Some(vec![
                "free".to_string(),
                "starter".to_string(),
                "creator".to_string()
            ])
        );
        assert!(voice.is_mixed);
        assert_eq!(voice.is_owner, Some(true));
        assert_eq!(voice.safety_control, Some(SafetyControl::None));

        let fine_tuning = voice.fine_tuning.expect("fine_tuning should be present");
        let state_map = fine_tuning.state.expect("state should be present");
        assert_eq!(
            state_map.get("eleven_multilingual_v2"),
            Some(&FineTuningState::FineTuned)
        );
    }

    #[test]
    fn list_voices_response_deserializes_realistic_payload() {
        let json = r#"{
            "voices": [
                {
                    "voice_id": "21m00Tcm4TlvDq8ikWAM",
                    "name": "Rachel",
                    "category": "premade",
                    "is_legacy": false,
                    "is_mixed": false,
                    "labels": {
                        "gender": "female",
                        "accent": "american"
                    }
                },
                {
                    "voice_id": "custom123",
                    "name": "My Clone",
                    "category": "cloned",
                    "is_legacy": false,
                    "is_mixed": false,
                    "fine_tuning": {
                        "state": {
                            "eleven_turbo_v2": "not_started"
                        },
                        "is_allowed_to_fine_tune": true
                    }
                }
            ],
            "has_more": true,
            "total_count": 100,
            "next_page_token": "abc123"
        }"#;

        let response: ListVoicesResponse = serde_json::from_str(json)
            .expect("ListVoicesResponse should deserialize v2 API response");

        assert_eq!(response.voices.len(), 2);
        assert!(response.has_more);
        assert_eq!(response.total_count, Some(100));
        assert_eq!(response.next_page_token, Some("abc123".to_string()));

        assert_eq!(response.voices[0].voice_id, "21m00Tcm4TlvDq8ikWAM");
        assert_eq!(response.voices[0].category, Some(VoiceCategory::Premade));

        let cloned = &response.voices[1];
        assert_eq!(cloned.category, Some(VoiceCategory::Cloned));
        let ft = cloned
            .fine_tuning
            .as_ref()
            .expect("fine_tuning should exist");
        let state_map = ft.state.as_ref().expect("state should exist");
        assert_eq!(
            state_map.get("eleven_turbo_v2"),
            Some(&FineTuningState::NotStarted)
        );
    }
}
