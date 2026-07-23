use super::*;

#[test]
fn sound_effect_deserializes_defaults() {
    let json = serde_json::json!({
        "type": "sound_effect",
        "effect": "success"
    });

    let action: HookAction = serde_json::from_value(json).unwrap();
    let HookAction::SoundEffect {
        effect,
        volume,
        speed,
        when,
    } = action
    else {
        panic!("expected sound_effect");
    };

    assert_eq!(effect, "success");
    assert_eq!(volume, 1.0);
    assert_eq!(speed, 1.0);
    assert!(when.is_none());
}

#[test]
fn sound_effect_with_effect_field() {
    let json = serde_json::json!({
        "type": "sound_effect",
        "effect": "ding",
        "volume": 0.5,
        "speed": 1.5
    });

    let action: HookAction = serde_json::from_value(json).unwrap();
    let HookAction::SoundEffect {
        effect,
        volume,
        speed,
        when,
    } = action
    else {
        panic!("expected sound_effect");
    };

    assert_eq!(effect, "ding");
    assert_eq!(volume, 0.5);
    assert_eq!(speed, 1.5);
    assert!(when.is_none());
}

#[test]
fn speak_with_voice_and_gender() {
    let json = serde_json::json!({
        "type": "speak",
        "message": "Hello world",
        "voice": "Samantha",
        "gender": "female"
    });

    let action: HookAction = serde_json::from_value(json).unwrap();
    let HookAction::Speak {
        message,
        voice,
        gender,
        when,
    } = action
    else {
        panic!("expected speak");
    };

    assert_eq!(message, "Hello world");
    assert_eq!(voice.as_deref(), Some("Samantha"));
    assert_eq!(gender, Some(Gender::Female));
    assert!(when.is_none());
}

#[test]
fn speak_minimal() {
    let json = serde_json::json!({
        "type": "speak",
        "message": "Hello"
    });

    let action: HookAction = serde_json::from_value(json).unwrap();
    let HookAction::Speak {
        message,
        voice,
        gender,
        when,
    } = action
    else {
        panic!("expected speak");
    };

    assert_eq!(message, "Hello");
    assert!(voice.is_none());
    assert!(gender.is_none());
    assert!(when.is_none());
}

#[test]
fn speak_skips_serializing_none_fields() {
    let action = HookAction::Speak {
        message: "test".to_string(),
        voice: None,
        gender: None,
        when: None,
    };

    let json = serde_json::to_value(&action).unwrap();
    assert!(json.get("voice").is_none());
    assert!(json.get("gender").is_none());
    assert!(json.get("when").is_none());
}

#[test]
fn bash_deserializes() {
    let json = serde_json::json!({
        "type": "bash",
        "command": "notify-send",
        "params": "{{tool_name}}"
    });

    let action: HookAction = serde_json::from_value(json).unwrap();
    let HookAction::Bash {
        command,
        params,
        when,
    } = action
    else {
        panic!("expected bash");
    };

    assert_eq!(command, "notify-send");
    assert_eq!(params, "{{tool_name}}");
    assert!(when.is_none());
}

#[test]
fn bash_default_params() {
    let json = serde_json::json!({
        "type": "bash",
        "command": "echo hello"
    });

    let action: HookAction = serde_json::from_value(json).unwrap();
    let HookAction::Bash {
        command,
        params,
        when,
    } = action
    else {
        panic!("expected bash");
    };

    assert_eq!(command, "echo hello");
    assert_eq!(params, "");
    assert!(when.is_none());
}

#[test]
fn bash_type_labels() {
    let action = HookAction::Bash {
        command: "echo".to_string(),
        params: String::new(),
        when: None,
    };

    assert_eq!(action.type_slug(), "bash");
    assert_eq!(action.type_pascal_case(), "Bash");
}

#[test]
fn call_round_trip() {
    let action = HookAction::Call {
        command: "security-check".to_string(),
        args: Some(vec!["--quick".to_string()]),
        timeout_ms: Some(4_000),
        mapper: Some(Mapper::JsonField {
            field: "decision".to_string(),
        }),
        when: None,
    };

    let json = serde_json::to_value(&action).unwrap();
    assert_eq!(json["type"], "call");

    let back: HookAction = serde_json::from_value(json).unwrap();
    assert_eq!(back, action);
}

#[test]
fn message_deserializes_with_required_fields() {
    let json = serde_json::json!({
        "type": "message",
        "message": "Deploy complete"
    });

    let action: HookAction = serde_json::from_value(json).unwrap();
    let HookAction::Message {
        message,
        image,
        when,
    } = action
    else {
        panic!("expected message");
    };

    assert_eq!(message, "Deploy complete");
    assert!(image.is_none());
    assert!(when.is_none());
}

#[test]
fn message_deserializes_with_image() {
    let json = serde_json::json!({
        "type": "message",
        "message": "Screenshot attached",
        "image": "/tmp/screenshot.png"
    });

    let action: HookAction = serde_json::from_value(json).unwrap();
    let HookAction::Message {
        message,
        image,
        when,
    } = action
    else {
        panic!("expected message");
    };

    assert_eq!(message, "Screenshot attached");
    assert_eq!(image.as_deref(), Some("/tmp/screenshot.png"));
    assert!(when.is_none());
}

#[test]
fn message_round_trip() {
    let action = HookAction::Message {
        message: "**build** done".to_string(),
        image: Some("~/artifacts/build.png".to_string()),
        when: None,
    };

    let json = serde_json::to_value(&action).unwrap();
    assert_eq!(json["type"], "message");
    assert_eq!(json["message"], "**build** done");
    assert_eq!(json["image"], "~/artifacts/build.png");

    let back: HookAction = serde_json::from_value(json).unwrap();
    assert_eq!(back, action);
}

#[test]
fn message_type_labels() {
    let action = HookAction::Message {
        message: "test".to_string(),
        image: None,
        when: None,
    };

    assert_eq!(action.type_slug(), "message");
    assert_eq!(action.type_pascal_case(), "Message");
}

// =========================================================================
// `when` round-trip and accessor tests (Phase 3 of leverage-dm-parser)
// =========================================================================

#[test]
fn old_action_configs_without_when_still_deserialize() {
    let json = serde_json::json!({
        "type": "bash",
        "command": "echo hi"
    });
    let action: HookAction = serde_json::from_value(json).unwrap();
    assert!(action.when().is_none());
}

#[test]
fn sound_effect_round_trips_with_when() {
    let action = HookAction::SoundEffect {
        effect: "ding".to_string(),
        volume: 1.0,
        speed: 1.0,
        when: Some("tool_name == 'Bash'".to_string()),
    };
    let json = serde_json::to_value(&action).unwrap();
    assert_eq!(json["when"], "tool_name == 'Bash'");
    let back: HookAction = serde_json::from_value(json).unwrap();
    assert_eq!(back, action);
    assert_eq!(back.when(), Some("tool_name == 'Bash'"));
}

#[test]
fn speak_round_trips_with_when() {
    let action = HookAction::Speak {
        message: "ready".to_string(),
        voice: None,
        gender: None,
        when: Some("provider == 'claude'".to_string()),
    };
    let json = serde_json::to_value(&action).unwrap();
    let back: HookAction = serde_json::from_value(json).unwrap();
    assert_eq!(back, action);
    assert_eq!(back.when(), Some("provider == 'claude'"));
}

#[test]
fn bash_round_trips_with_when() {
    let action = HookAction::Bash {
        command: "echo".to_string(),
        params: String::new(),
        when: Some("git.is_dirty".to_string()),
    };
    let json = serde_json::to_value(&action).unwrap();
    let back: HookAction = serde_json::from_value(json).unwrap();
    assert_eq!(back, action);
}

#[test]
fn call_round_trips_with_when() {
    let action = HookAction::Call {
        command: "security-check".to_string(),
        args: None,
        timeout_ms: None,
        mapper: None,
        when: Some("tool_name == 'Bash'".to_string()),
    };
    let json = serde_json::to_value(&action).unwrap();
    let back: HookAction = serde_json::from_value(json).unwrap();
    assert_eq!(back, action);
}

#[test]
fn report_round_trips_with_when() {
    let action = HookAction::Report {
        handler: None,
        when: Some("event == 'tool_error'".to_string()),
    };
    let json = serde_json::to_value(&action).unwrap();
    let back: HookAction = serde_json::from_value(json).unwrap();
    assert_eq!(back, action);
}

#[test]
fn message_round_trips_with_when() {
    let action = HookAction::Message {
        message: "alert".to_string(),
        image: None,
        when: Some("error".to_string()),
    };
    let json = serde_json::to_value(&action).unwrap();
    let back: HookAction = serde_json::from_value(json).unwrap();
    assert_eq!(back, action);
}

#[test]
fn when_field_omitted_from_serialization_when_none() {
    let action = HookAction::Bash {
        command: "echo".to_string(),
        params: String::new(),
        when: None,
    };
    let json = serde_json::to_value(&action).unwrap();
    assert!(json.get("when").is_none());
}
