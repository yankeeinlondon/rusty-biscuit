use super::*;

#[test]
fn round_trip_json() {
    let provider = Provider::Claude;
    let json = serde_json::to_value(provider).unwrap();
    assert_eq!(json, serde_json::json!("claude"));
    let back: Provider = serde_json::from_value(json).unwrap();
    assert_eq!(back, Provider::Claude);
}

#[test]
fn all_variants_serialize_snake_case() {
    let cases = vec![
        (Provider::Claude, "claude"),
        (Provider::Codex, "codex"),
        (Provider::Gemini, "gemini"),
        (Provider::Goose, "goose"),
        (Provider::KimiCode, "kimi_code"),
        (Provider::OpenCode, "open_code"),
        (Provider::QwenCode, "qwen_code"),
    ];
    for (variant, expected) in cases {
        let json = serde_json::to_value(variant).unwrap();
        assert_eq!(json.as_str().unwrap(), expected, "Failed for {variant:?}");
    }
}

#[test]
fn display_uses_friendly_names() {
    assert_eq!(Provider::Claude.to_string(), "Claude");
    assert_eq!(Provider::Codex.to_string(), "Codex");
    assert_eq!(Provider::Gemini.to_string(), "Gemini");
    assert_eq!(Provider::Goose.to_string(), "Goose");
    assert_eq!(Provider::KimiCode.to_string(), "Kimi Code");
    assert_eq!(Provider::OpenCode.to_string(), "OpenCode");
    assert_eq!(Provider::QwenCode.to_string(), "Qwen Code");
}

#[test]
fn can_use_as_hashmap_key() {
    use std::collections::HashMap;
    let mut map = HashMap::new();
    map.insert(Provider::Claude, "test");
    assert_eq!(map.get(&Provider::Claude), Some(&"test"));
}

#[test]
fn supports_skills() {
    // Every compiled provider reports first-class skill support per the
    // skills research topic (generator v1 graduation; the old
    // Goose/Kimi `false` constants were ruled stale - Open question 3).
    for provider in crate::provider::PROVIDERS_DISPLAY_ORDER {
        assert!(
            provider.supports_skills(),
            "{provider:?} should support skills per the skills research topic"
        );
    }
}

#[test]
fn as_slug_returns_canonical_slugs() {
    assert_eq!(Provider::Claude.as_slug(), "claude");
    assert_eq!(Provider::Codex.as_slug(), "codex");
    assert_eq!(Provider::Gemini.as_slug(), "gemini");
    assert_eq!(Provider::Goose.as_slug(), "goose");
    assert_eq!(Provider::KimiCode.as_slug(), "kimi");
    assert_eq!(Provider::OpenCode.as_slug(), "opencode");
    assert_eq!(Provider::QwenCode.as_slug(), "qwen");
}

#[test]
fn docs_url_returns_valid_urls() {
    for provider in [
        Provider::Claude,
        Provider::Codex,
        Provider::Gemini,
        Provider::Goose,
        Provider::KimiCode,
        Provider::OpenCode,
        Provider::QwenCode,
    ] {
        let url = provider.docs_url();
        assert!(
            url.starts_with("https://"),
            "Provider {provider:?} URL should start with https://"
        );
    }
}

#[test]
fn supports_event_claude() {
    use crate::events::AgenticEvent::*;
    assert!(Provider::Claude.supports_event(&SessionStart));
    assert!(Provider::Claude.supports_event(&TurnComplete));
    assert!(Provider::Claude.supports_event(&ToolError));
    assert!(!Provider::Claude.supports_event(&BeforeModel));
    assert!(!Provider::Claude.supports_event(&AfterModel));
    assert!(!Provider::Claude.supports_event(&TurnError));
}

#[test]
fn supports_event_codex() {
    use crate::events::AgenticEvent::*;
    assert!(Provider::Codex.supports_event(&SessionStart));
    assert!(Provider::Codex.supports_event(&TurnComplete));
    assert!(Provider::Codex.supports_event(&TurnError));
    assert!(Provider::Codex.supports_event(&BeforeTool));
    assert!(Provider::Codex.supports_event(&AfterTool));
    assert!(Provider::Codex.supports_event(&AfterModel));
    assert!(Provider::Codex.supports_event(&Notification));
    assert!(!Provider::Codex.supports_event(&SessionEnd));
    assert!(!Provider::Codex.supports_event(&PermissionRequest));
    assert!(!Provider::Codex.supports_event(&SubagentStart));
    assert!(!Provider::Codex.supports_event(&SubagentStop));
    assert!(!Provider::Codex.supports_event(&BeforeModel));
    assert!(!Provider::Codex.supports_event(&BeforeCompact));
}

#[test]
fn supports_event_gemini() {
    use crate::events::AgenticEvent::*;
    assert!(Provider::Gemini.supports_event(&SessionStart));
    assert!(Provider::Gemini.supports_event(&BeforeModel));
    assert!(!Provider::Gemini.supports_event(&ToolError));
    assert!(!Provider::Gemini.supports_event(&PermissionRequest));
    assert!(!Provider::Gemini.supports_event(&SubagentStart));
}

#[test]
fn supports_event_goose() {
    use crate::events::AgenticEvent::*;
    assert!(Provider::Goose.supports_event(&TurnComplete));
    assert!(Provider::Goose.supports_event(&TurnError));
    assert!(Provider::Goose.supports_event(&AfterModel));
    assert!(Provider::Goose.supports_event(&Notification));
    assert!(Provider::Goose.supports_event(&SubagentStart));
    assert!(Provider::Goose.supports_event(&SubagentStop));
    assert!(!Provider::Goose.supports_event(&SessionStart));
    assert!(!Provider::Goose.supports_event(&SessionEnd));
    assert!(!Provider::Goose.supports_event(&BeforePrompt));
    assert!(!Provider::Goose.supports_event(&BeforeTool));
    assert!(!Provider::Goose.supports_event(&AfterTool));
    assert!(!Provider::Goose.supports_event(&ToolError));
    assert!(!Provider::Goose.supports_event(&PermissionRequest));
    assert!(!Provider::Goose.supports_event(&BeforeModel));
    assert!(!Provider::Goose.supports_event(&BeforeCompact));
}

#[test]
fn supports_event_kimicode() {
    use crate::events::AgenticEvent::*;
    assert!(Provider::KimiCode.supports_event(&TurnComplete));
    assert!(Provider::KimiCode.supports_event(&TurnError));
    assert!(Provider::KimiCode.supports_event(&BeforePrompt));
    assert!(Provider::KimiCode.supports_event(&BeforeTool));
    assert!(Provider::KimiCode.supports_event(&AfterTool));
    assert!(Provider::KimiCode.supports_event(&ToolError));
    assert!(Provider::KimiCode.supports_event(&PermissionRequest));
    assert!(Provider::KimiCode.supports_event(&SubagentStart));
    assert!(Provider::KimiCode.supports_event(&SubagentStop));
    assert!(Provider::KimiCode.supports_event(&AfterModel));
    assert!(Provider::KimiCode.supports_event(&BeforeCompact));
    assert!(!Provider::KimiCode.supports_event(&SessionStart));
    assert!(!Provider::KimiCode.supports_event(&SessionEnd));
    assert!(!Provider::KimiCode.supports_event(&BeforeModel));
}

#[test]
fn supports_event_opencode() {
    use crate::events::AgenticEvent::*;
    assert!(Provider::OpenCode.supports_event(&SessionStart));
    assert!(Provider::OpenCode.supports_event(&SessionEnd));
    assert!(Provider::OpenCode.supports_event(&BeforePrompt));
    assert!(Provider::OpenCode.supports_event(&BeforeTool));
    assert!(Provider::OpenCode.supports_event(&AfterTool));
    assert!(Provider::OpenCode.supports_event(&PermissionRequest));
    assert!(Provider::OpenCode.supports_event(&TurnComplete));
    assert!(Provider::OpenCode.supports_event(&TurnError));
    assert!(Provider::OpenCode.supports_event(&BeforeModel));
    assert!(Provider::OpenCode.supports_event(&AfterModel));
    assert!(Provider::OpenCode.supports_event(&BeforeCompact));
    assert!(Provider::OpenCode.supports_event(&Notification));
    assert!(!Provider::OpenCode.supports_event(&ToolError));
    assert!(!Provider::OpenCode.supports_event(&SubagentStart));
    assert!(!Provider::OpenCode.supports_event(&SubagentStop));
}

#[test]
fn supports_event_qwencode() {
    use crate::events::AgenticEvent::*;
    assert!(Provider::QwenCode.supports_event(&TurnComplete));
    assert!(Provider::QwenCode.supports_event(&TurnError));
    assert!(Provider::QwenCode.supports_event(&AfterModel));
    assert!(Provider::QwenCode.supports_event(&Notification));
    assert!(Provider::QwenCode.supports_event(&PermissionRequest));
    assert!(!Provider::QwenCode.supports_event(&SessionStart));
    assert!(!Provider::QwenCode.supports_event(&SessionEnd));
    assert!(!Provider::QwenCode.supports_event(&BeforePrompt));
    assert!(!Provider::QwenCode.supports_event(&BeforeTool));
    assert!(!Provider::QwenCode.supports_event(&AfterTool));
    assert!(!Provider::QwenCode.supports_event(&ToolError));
    assert!(!Provider::QwenCode.supports_event(&SubagentStart));
    assert!(!Provider::QwenCode.supports_event(&SubagentStop));
    assert!(!Provider::QwenCode.supports_event(&BeforeModel));
    assert!(!Provider::QwenCode.supports_event(&BeforeCompact));
}

#[test]
fn event_support_level_claude_all_hook() {
    use crate::events::AgenticEvent::*;
    assert!(
        Provider::Claude
            .event_support_level(&TurnComplete)
            .is_hook()
    );
    assert!(Provider::Claude.event_support_level(&BeforeTool).is_hook());
    assert!(
        !Provider::Claude
            .event_support_level(&BeforeModel)
            .is_supported()
    );
}

#[test]
fn event_support_level_codex_mixed() {
    use crate::events::AgenticEvent::*;
    assert!(Provider::Codex.event_support_level(&TurnComplete).is_hook());
    assert!(
        Provider::Codex
            .event_support_level(&BeforeTool)
            .is_supported()
            && !Provider::Codex.event_support_level(&BeforeTool).is_hook()
    );
    assert!(
        Provider::Codex
            .event_support_level(&AfterTool)
            .is_supported()
            && !Provider::Codex.event_support_level(&AfterTool).is_hook()
    );
    assert!(
        Provider::Codex
            .event_support_level(&SessionStart)
            .is_supported()
            && !Provider::Codex.event_support_level(&SessionStart).is_hook()
    );
    assert!(
        !Provider::Codex
            .event_support_level(&PermissionRequest)
            .is_supported()
    );
}

#[test]
fn event_support_level_goose_all_non_hook() {
    use crate::events::AgenticEvent::*;
    assert!(
        Provider::Goose
            .event_support_level(&TurnComplete)
            .is_supported()
            && !Provider::Goose.event_support_level(&TurnComplete).is_hook()
    );
    assert!(
        Provider::Goose
            .event_support_level(&Notification)
            .is_supported()
            && !Provider::Goose.event_support_level(&Notification).is_hook()
    );
    assert!(
        !Provider::Goose
            .event_support_level(&SessionStart)
            .is_supported()
    );
    assert!(
        !Provider::Goose
            .event_support_level(&BeforeTool)
            .is_supported()
    );
}

#[test]
fn event_support_level_kimicode_all_non_hook() {
    use crate::events::AgenticEvent::*;
    assert!(
        Provider::KimiCode
            .event_support_level(&TurnComplete)
            .is_supported()
            && !Provider::KimiCode
                .event_support_level(&TurnComplete)
                .is_hook()
    );
    assert!(
        Provider::KimiCode
            .event_support_level(&BeforeTool)
            .is_supported()
            && !Provider::KimiCode
                .event_support_level(&BeforeTool)
                .is_hook()
    );
    assert!(
        Provider::KimiCode
            .event_support_level(&PermissionRequest)
            .is_acp()
    );
    assert!(
        !Provider::KimiCode
            .event_support_level(&SessionStart)
            .is_supported()
    );
}

#[test]
fn event_support_level_qwencode_all_non_hook() {
    use crate::events::AgenticEvent::*;
    assert!(
        Provider::QwenCode
            .event_support_level(&TurnComplete)
            .is_supported()
            && !Provider::QwenCode
                .event_support_level(&TurnComplete)
                .is_hook()
    );
    assert!(
        Provider::QwenCode
            .event_support_level(&AfterModel)
            .is_supported()
            && !Provider::QwenCode
                .event_support_level(&AfterModel)
                .is_hook()
    );
    assert!(
        Provider::QwenCode
            .event_support_level(&PermissionRequest)
            .is_supported()
            && !Provider::QwenCode
                .event_support_level(&PermissionRequest)
                .is_hook()
    );
    assert!(
        !Provider::QwenCode
            .event_support_level(&BeforeTool)
            .is_supported()
    );
}

#[test]
fn supports_event_via_hook() {
    use crate::events::AgenticEvent::*;
    assert!(Provider::Claude.supports_event_via_hook(&TurnComplete));
    assert!(Provider::Gemini.supports_event_via_hook(&TurnComplete));
    assert!(Provider::OpenCode.supports_event_via_hook(&TurnComplete));
    assert!(Provider::Codex.supports_event_via_hook(&TurnComplete));

    assert!(!Provider::Codex.supports_event_via_hook(&BeforeTool));

    assert!(!Provider::Goose.supports_event_via_hook(&TurnComplete));
    assert!(!Provider::KimiCode.supports_event_via_hook(&TurnComplete));
    assert!(!Provider::QwenCode.supports_event_via_hook(&TurnComplete));
}

#[test]
fn opencode_shared_native_mappings_cover_core_hook_events() {
    use crate::events::AgenticEvent::*;

    assert_eq!(
        Provider::OpenCode.registration_native_event_name(&BeforePrompt),
        Some("chat.message")
    );
    assert_eq!(
        Provider::OpenCode.registration_native_event_name(&BeforeTool),
        Some("tool.execute.before")
    );
    assert_eq!(
        Provider::OpenCode.registration_native_event_name(&AfterTool),
        Some("tool.execute.after")
    );
    assert_eq!(
        Provider::OpenCode.registration_native_event_name(&BeforeModel),
        Some("chat.params")
    );
    assert_eq!(
        Provider::OpenCode.registration_native_event_name(&AfterModel),
        Some("message.updated")
    );

    assert_eq!(
        Provider::OpenCode
            .event_from_shared_native_name("experimental.chat.messages.transform"),
        Some(BeforeModel)
    );
    assert_eq!(
        Provider::OpenCode.event_from_shared_native_name("experimental.text.complete"),
        Some(AfterModel)
    );
}

#[test]
fn parse_cli_name_accepts_aliases() {
    assert_eq!(Provider::parse_cli_name("claude"), Some(Provider::Claude));
    assert_eq!(Provider::parse_cli_name("kimi"), Some(Provider::KimiCode));
    assert_eq!(
        Provider::parse_cli_name("open-code"),
        Some(Provider::OpenCode)
    );
    assert_eq!(
        Provider::parse_cli_name("QWEN_CODE"),
        Some(Provider::QwenCode)
    );
    assert_eq!(Provider::parse_cli_name(""), None);
}

#[test]
fn fuzzy_match_cli_name_supports_prefix_and_contains() {
    assert_eq!(
        Provider::fuzzy_match_cli_name("gemi"),
        Some(Provider::Gemini)
    );
    assert_eq!(
        Provider::fuzzy_match_cli_name("kimi"),
        Some(Provider::KimiCode)
    );
    assert_eq!(
        Provider::fuzzy_match_cli_name("open"),
        Some(Provider::OpenCode)
    );
    assert_eq!(Provider::fuzzy_match_cli_name("unknown"), None);
}

#[test]
fn sniff_ai_cli_maps_all_providers() {
    assert_eq!(Provider::Claude.sniff_ai_cli(), AiCli::Claude);
    assert_eq!(Provider::Codex.sniff_ai_cli(), AiCli::Codex);
    assert_eq!(Provider::Gemini.sniff_ai_cli(), AiCli::GeminiCli);
    assert_eq!(Provider::Goose.sniff_ai_cli(), AiCli::Goose);
    assert_eq!(Provider::KimiCode.sniff_ai_cli(), AiCli::KimiCli);
    assert_eq!(Provider::OpenCode.sniff_ai_cli(), AiCli::Opencode);
    assert_eq!(Provider::QwenCode.sniff_ai_cli(), AiCli::QwenCli);
}

#[test]
fn agent_offset_returns_correct_directories() {
    assert_eq!(Provider::Claude.agent_offset(), ".claude");
    assert_eq!(Provider::Codex.agent_offset(), ".codex");
    assert_eq!(Provider::Gemini.agent_offset(), ".gemini");
    assert_eq!(Provider::Goose.agent_offset(), ".goose");
    assert_eq!(Provider::KimiCode.agent_offset(), ".kimi");
    assert_eq!(Provider::OpenCode.agent_offset(), ".opencode");
    assert_eq!(Provider::QwenCode.agent_offset(), ".qwen");
}

#[test]
fn detect_from_payload_recognizes_known_shapes() {
    assert_eq!(
        Provider::detect_from_payload(&serde_json::json!({"hook_event_name":"Stop"})),
        Some(Provider::Claude)
    );
    assert_eq!(
        Provider::detect_from_payload(&serde_json::json!({"hook_event_name":"PreToolUse"})),
        Some(Provider::Claude)
    );
    assert_eq!(
        Provider::detect_from_payload(
            &serde_json::json!({"hook_event_name":"UserPromptSubmit"})
        ),
        Some(Provider::Claude)
    );

    assert_eq!(
        Provider::detect_from_payload(&serde_json::json!({"hook_event_name":"BeforeAgent"})),
        Some(Provider::Gemini)
    );
    assert_eq!(
        Provider::detect_from_payload(&serde_json::json!({"hook_event_name":"AfterAgent"})),
        Some(Provider::Gemini)
    );
    assert_eq!(
        Provider::detect_from_payload(&serde_json::json!({"hook_event_name":"BeforeModel"})),
        Some(Provider::Gemini)
    );
    assert_eq!(
        Provider::detect_from_payload(&serde_json::json!({"hook_event_name":"BeforeTool"})),
        Some(Provider::Gemini)
    );
    assert_eq!(
        Provider::detect_from_payload(&serde_json::json!({"hook_event_name":"AfterTool"})),
        Some(Provider::Gemini)
    );

    assert_eq!(
        Provider::detect_from_payload(&serde_json::json!({"hook_event_name":"SessionStart"})),
        Some(Provider::Claude)
    );
    assert_eq!(
        Provider::detect_from_payload(&serde_json::json!({"hook_event_name":"Notification"})),
        Some(Provider::Claude)
    );

    assert_eq!(
        Provider::detect_from_payload(&serde_json::json!({"event_name":"BeforeAgent"})),
        Some(Provider::Gemini)
    );

    assert_eq!(
        Provider::detect_from_payload(
            &serde_json::json!({"type":"turn.completed","thread_id":"t-1"})
        ),
        Some(Provider::Codex)
    );
    assert_eq!(
        Provider::detect_from_payload(
            &serde_json::json!({"type":"agent-turn-complete","thread-id":"t-1"})
        ),
        Some(Provider::Codex)
    );
    assert_eq!(
        Provider::detect_from_payload(&serde_json::json!({
            "session_id":"ses_123",
            "hook_event":{"event_type":"after_tool_use"}
        })),
        Some(Provider::Codex)
    );

    assert_eq!(
        Provider::detect_from_payload(&serde_json::json!({"event_type":"session.idle"})),
        Some(Provider::OpenCode)
    );

    assert_eq!(
        Provider::detect_from_payload(&serde_json::json!({"method":"notification"})),
        Some(Provider::KimiCode)
    );

    assert_eq!(
        Provider::detect_from_payload(&serde_json::json!({"unknown":true})),
        None
    );
}
