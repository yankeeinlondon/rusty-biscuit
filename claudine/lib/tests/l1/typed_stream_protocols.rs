//! Property-based tests ensuring typed stream deserializers accept all known
//! provider event shapes with arbitrary extra fields and missing optional
//! fields.

use proptest::prelude::*;
use serde_json::json;

use claudine::stream::protocol::claude::ClaudeEvent;
use claudine::stream::protocol::codex::CodexEvent;
use claudine::stream::protocol::gemini::GeminiEvent;
use claudine::stream::protocol::opencode::OpenCodeEvent;
use claudine::stream::protocol::qwen::QwenEvent;

/// Generate an arbitrary JSON object value (primitive, array, or object).
fn arbitrary_json_value() -> impl Strategy<Value = serde_json::Value> {
    let leaf = prop_oneof![
        any::<bool>().prop_map(serde_json::Value::Bool),
        any::<i64>().prop_map(|n| serde_json::Value::Number(n.into())),
        "\\w*".prop_map(serde_json::Value::String),
        Just(serde_json::Value::Null),
    ];
    leaf.prop_recursive(2, 4, 2, |inner| {
        prop_oneof![
            prop::collection::vec(inner.clone(), 0..3).prop_map(serde_json::Value::Array),
            prop::collection::hash_map("\\w+", inner, 0..3)
                .prop_map(|m| serde_json::Value::Object(m.into_iter().collect())),
        ]
    })
}

/// Inject arbitrary values under field names that cannot shadow known protocol keys.
fn inject_extras(value: serde_json::Value, extras: Vec<(String, serde_json::Value)>) -> String {
    let mut obj = match value {
        serde_json::Value::Object(o) => o,
        other => {
            let mut o = serde_json::Map::new();
            o.insert("_value".into(), other);
            o
        }
    };
    for (k, v) in extras {
        let k = format!("extra_{k}");
        if !obj.contains_key(&k) {
            obj.insert(k, v);
        }
    }
    serde_json::to_string(&serde_json::Value::Object(obj)).unwrap()
}

proptest! {
    #[test]
    fn claude_event_accepts_extras(
        extras in prop::collection::vec(("\\w+", arbitrary_json_value()), 0..5)
    ) {
        let base = json!({"type":"init","session_id":"s1","model":"claude"});
        let line = inject_extras(base, extras);
        let event: ClaudeEvent = serde_json::from_str(&line).unwrap();
        assert_eq!(event.type_str(), "init");
    }

    #[test]
    fn claude_assistant_accepts_extras(
        extras in prop::collection::vec(("\\w+", arbitrary_json_value()), 0..5)
    ) {
        let base = json!({"type":"assistant","content":[{"type":"text","text":"hi"}]});
        let line = inject_extras(base, extras);
        let event: ClaudeEvent = serde_json::from_str(&line).unwrap();
        assert_eq!(event.type_str(), "assistant");
    }

    #[test]
    fn claude_content_block_delta_accepts_extras(
        extras in prop::collection::vec(("\\w+", arbitrary_json_value()), 0..5)
    ) {
        let base = json!({"type":"content_block_delta","delta":{"type":"text_delta","text":"x"}});
        let line = inject_extras(base, extras);
        let event: ClaudeEvent = serde_json::from_str(&line).unwrap();
        assert_eq!(event.type_str(), "content_block_delta");
    }

    #[test]
    fn claude_tool_use_accepts_extras(
        extras in prop::collection::vec(("\\w+", arbitrary_json_value()), 0..5)
    ) {
        let base = json!({"type":"tool_use","id":"t1","name":"bash","input":{}});
        let line = inject_extras(base, extras);
        let event: ClaudeEvent = serde_json::from_str(&line).unwrap();
        assert_eq!(event.type_str(), "tool_use");
    }

    #[test]
    fn claude_result_accepts_extras(
        extras in prop::collection::vec(("\\w+", arbitrary_json_value()), 0..5)
    ) {
        let base = json!({"type":"result","stop_reason":"end_turn"});
        let line = inject_extras(base, extras);
        let event: ClaudeEvent = serde_json::from_str(&line).unwrap();
        assert_eq!(event.type_str(), "result");
    }

    #[test]
    fn gemini_event_accepts_extras(
        extras in prop::collection::vec(("\\w+", arbitrary_json_value()), 0..5)
    ) {
        let base = json!({"type":"init","session_id":"g1","model":"gemini"});
        let line = inject_extras(base, extras);
        let event: GeminiEvent = serde_json::from_str(&line).unwrap();
        assert_eq!(event.type_str(), "init");
    }

    #[test]
    fn gemini_message_accepts_extras(
        extras in prop::collection::vec(("\\w+", arbitrary_json_value()), 0..5)
    ) {
        let base = json!({"type":"message","role":"assistant","content":"hi"});
        let line = inject_extras(base, extras);
        let event: GeminiEvent = serde_json::from_str(&line).unwrap();
        assert_eq!(event.type_str(), "message");
    }

    #[test]
    fn gemini_result_accepts_extras(
        extras in prop::collection::vec(("\\w+", arbitrary_json_value()), 0..5)
    ) {
        let base = json!({"type":"result","status":"success"});
        let line = inject_extras(base, extras);
        let event: GeminiEvent = serde_json::from_str(&line).unwrap();
        assert_eq!(event.type_str(), "result");
    }

    #[test]
    fn codex_thread_started_accepts_extras(
        extras in prop::collection::vec(("\\w+", arbitrary_json_value()), 0..5)
    ) {
        let base = json!({"type":"thread.started","thread_id":"th1"});
        let line = inject_extras(base, extras);
        let event: CodexEvent = serde_json::from_str(&line).unwrap();
        assert_eq!(event.type_str(), "thread.started");
    }

    #[test]
    fn codex_turn_completed_accepts_extras(
        extras in prop::collection::vec(("\\w+", arbitrary_json_value()), 0..5)
    ) {
        let base = json!({"type":"turn.completed","status":"completed"});
        let line = inject_extras(base, extras);
        let event: CodexEvent = serde_json::from_str(&line).unwrap();
        assert_eq!(event.type_str(), "turn.completed");
    }

    #[test]
    fn codex_item_started_accepts_extras(
        extras in prop::collection::vec(("\\w+", arbitrary_json_value()), 0..5)
    ) {
        let base = json!({"type":"item.started","item":{"id":"i1","type":"reasoning"}});
        let line = inject_extras(base, extras);
        let event: CodexEvent = serde_json::from_str(&line).unwrap();
        assert_eq!(event.type_str(), "item.started");
    }

    #[test]
    fn opencode_event_accepts_extras(
        extras in prop::collection::vec(("\\w+", arbitrary_json_value()), 0..5)
    ) {
        let base = json!({"type":"init","session_id":"o1"});
        let line = inject_extras(base, extras);
        let event: OpenCodeEvent = serde_json::from_str(&line).unwrap();
        assert_eq!(event.type_str(), "init");
    }

    #[test]
    fn opencode_text_accepts_extras(
        extras in prop::collection::vec(("\\w+", arbitrary_json_value()), 0..5)
    ) {
        let base = json!({"type":"text","text":"hello"});
        let line = inject_extras(base, extras);
        let event: OpenCodeEvent = serde_json::from_str(&line).unwrap();
        assert_eq!(event.type_str(), "text");
    }

    #[test]
    fn opencode_tool_start_accepts_extras(
        extras in prop::collection::vec(("\\w+", arbitrary_json_value()), 0..5)
    ) {
        let base = json!({"type":"tool_start","part":{"id":"t1","tool_name":"bash"}});
        let line = inject_extras(base, extras);
        let event: OpenCodeEvent = serde_json::from_str(&line).unwrap();
        assert_eq!(event.type_str(), "tool_start");
    }

    #[test]
    fn qwen_event_accepts_extras(
        extras in prop::collection::vec(("\\w+", arbitrary_json_value()), 0..5)
    ) {
        let base = json!({"type":"init","session_id":"q1","model":"qwen"});
        let line = inject_extras(base, extras);
        let event: QwenEvent = serde_json::from_str(&line).unwrap();
        assert_eq!(event.type_str(), "init");
    }

    #[test]
    fn qwen_message_accepts_extras(
        extras in prop::collection::vec(("\\w+", arbitrary_json_value()), 0..5)
    ) {
        let base = json!({"type":"message","role":"assistant","content":"hi"});
        let line = inject_extras(base, extras);
        let event: QwenEvent = serde_json::from_str(&line).unwrap();
        assert_eq!(event.type_str(), "message");
    }

    #[test]
    fn qwen_tool_call_accepts_extras(
        extras in prop::collection::vec(("\\w+", arbitrary_json_value()), 0..5)
    ) {
        let base = json!({"type":"tool_call","id":"t1","name":"bash","input":{}});
        let line = inject_extras(base, extras);
        let event: QwenEvent = serde_json::from_str(&line).unwrap();
        assert_eq!(event.type_str(), "tool_call");
    }
}

#[test]
fn protocol_named_extras_do_not_shadow_known_fields() {
    let codex_line = inject_extras(
        json!({"type":"thread.started","thread_id":"th1"}),
        vec![("id".to_string(), json!([]))],
    );
    let codex_event: CodexEvent = serde_json::from_str(&codex_line).unwrap();
    assert_eq!(codex_event.type_str(), "thread.started");

    let opencode_line = inject_extras(
        json!({"type":"tool_start","part":{"id":"t1","tool_name":"bash"}}),
        vec![("id".to_string(), json!({}))],
    );
    let opencode_event: OpenCodeEvent = serde_json::from_str(&opencode_line).unwrap();
    assert_eq!(opencode_event.type_str(), "tool_start");
}

/// Ensure unknown event types fail typed deserialization for every provider.
/// This is the contract that lets semantic parsers fall back to
/// `ProviderExtension` safely.
#[test]
fn unknown_type_fails_typed_claude() {
    let line = r#"{"type":"future.event","payload":1}"#;
    assert!(serde_json::from_str::<ClaudeEvent>(line).is_err());
}

#[test]
fn unknown_type_fails_typed_gemini() {
    let line = r#"{"type":"future.event","payload":1}"#;
    assert!(serde_json::from_str::<GeminiEvent>(line).is_err());
}

#[test]
fn unknown_type_fails_typed_codex() {
    let line = r#"{"type":"future.event","payload":1}"#;
    assert!(serde_json::from_str::<CodexEvent>(line).is_err());
}

#[test]
fn unknown_type_fails_typed_opencode() {
    let line = r#"{"type":"future.event","payload":1}"#;
    assert!(serde_json::from_str::<OpenCodeEvent>(line).is_err());
}

#[test]
fn unknown_type_fails_typed_qwen() {
    let line = r#"{"type":"future.event","payload":1}"#;
    assert!(serde_json::from_str::<QwenEvent>(line).is_err());
}
