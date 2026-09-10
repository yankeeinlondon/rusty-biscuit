use super::*;
use serde_json::json;

fn all_variants() -> Vec<SemanticEvent> {
    vec![
        SemanticEvent::SessionStart {
            session_id: Some("s1".into()),
            model: Some("claude".into()),
            extra: json!({"provider": "claude"}),
        },
        SemanticEvent::TurnStart { extra: json!({}) },
        SemanticEvent::TurnComplete {
            provider_status: Some("ok".into()),
            token_usage: Some(NormalizedTokenUsage {
                input: Some(10),
                output: Some(20),
                total: Some(30),
                cache_read: None,
            }),
            cost_usd: Some(0.001),
            duration_ms: Some(1500),
            extra: json!({}),
        },
        SemanticEvent::OutputText {
            text: "hello".into(),
            extra: json!({}),
        },
        SemanticEvent::Reasoning {
            text: "thinking".into(),
            extra: json!({}),
        },
        SemanticEvent::ToolCall {
            name: Some("bash".into()),
            id: Some("t_1".into()),
            input: Some(json!({"cmd": "ls"})),
            extra: json!({}),
        },
        SemanticEvent::ToolResult {
            name: Some("bash".into()),
            id: Some("t_1".into()),
            status: Some("success".into()),
            exit_code: Some(0),
            output: Some(json!({"stdout": "file.txt"})),
            extra: json!({}),
        },
        SemanticEvent::PermissionRequest {
            kind: Some("shell".into()),
            tool_name: Some("bash".into()),
            extra: json!({}),
        },
        SemanticEvent::SubagentStart {
            name: Some("researcher".into()),
            id: Some("sa_1".into()),
            extra: json!({}),
        },
        SemanticEvent::SubagentStop {
            name: Some("researcher".into()),
            id: Some("sa_1".into()),
            status: Some("success".into()),
            extra: json!({}),
        },
        SemanticEvent::FileChange {
            path: Some("src/lib.rs".into()),
            change_kind: Some("modified".into()),
            extra: json!({}),
        },
        SemanticEvent::PlanUpdate {
            message: Some("Step 2 of 5".into()),
            extra: json!({}),
        },
        SemanticEvent::Info {
            message: "rate limit reset".into(),
            extra: json!({}),
        },
        SemanticEvent::Warning {
            message: "rate limited".into(),
            extra: json!({}),
        },
        SemanticEvent::Error {
            message: "quota exceeded".into(),
            terminal: true,
            kind: SemanticErrorKind::ApiRemote,
            extra: json!({}),
        },
        SemanticEvent::ProviderExtension {
            provider: Provider::Codex,
            kind: "web_search".into(),
            payload: json!({"query": "rust"}),
        },
    ]
}

#[test]
fn kind_str_is_unique_per_variant() {
    let kinds: Vec<&str> = all_variants().iter().map(|e| e.kind_str()).collect();
    let mut sorted = kinds.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(
        sorted.len(),
        kinds.len(),
        "kind_str must be unique per variant"
    );
}

#[test]
fn is_activity_excludes_envelope_events() {
    assert!(
        !SemanticEvent::SessionStart {
            session_id: None,
            model: None,
            extra: json!({}),
        }
        .is_activity()
    );
    assert!(!SemanticEvent::TurnStart { extra: json!({}) }.is_activity());
    assert!(
        !SemanticEvent::TurnComplete {
            provider_status: None,
            token_usage: None,
            cost_usd: None,
            duration_ms: None,
            extra: json!({}),
        }
        .is_activity()
    );
    assert!(
        !SemanticEvent::PermissionRequest {
            kind: None,
            tool_name: None,
            extra: json!({}),
        }
        .is_activity()
    );
}

#[test]
fn is_activity_includes_work_events() {
    assert!(
        SemanticEvent::OutputText {
            text: "x".into(),
            extra: json!({}),
        }
        .is_activity()
    );
    assert!(
        SemanticEvent::ToolCall {
            name: None,
            id: None,
            input: None,
            extra: json!({}),
        }
        .is_activity()
    );
    assert!(
        SemanticEvent::ProviderExtension {
            provider: Provider::Claude,
            kind: "x".into(),
            payload: json!({}),
        }
        .is_activity()
    );
}

#[test]
fn round_trip_serde_preserves_value_for_every_variant() {
    for event in all_variants() {
        let value = serde_json::to_value(&event).expect("serialize");
        let decoded: SemanticEvent =
            serde_json::from_value(value.clone()).expect("deserialize");
        let value2 = serde_json::to_value(&decoded).expect("re-serialize");
        assert_eq!(
            value,
            value2,
            "round-trip lost fidelity for kind {}",
            event.kind_str()
        );
    }
}

#[test]
fn provider_extension_preserves_arbitrary_payload() {
    let payload = json!({
        "deeply": {
            "nested": ["values", 1, 2.5, null, true],
            "with_weird_keys": {"kebab-key": "ok"}
        }
    });
    let event = SemanticEvent::ProviderExtension {
        provider: Provider::Codex,
        kind: "item.updated".into(),
        payload: payload.clone(),
    };
    let value = serde_json::to_value(&event).unwrap();
    let decoded: SemanticEvent = serde_json::from_value(value).unwrap();
    match decoded {
        SemanticEvent::ProviderExtension {
            payload: decoded_payload,
            ..
        } => assert_eq!(decoded_payload, payload),
        other => panic!("expected ProviderExtension, got {other:?}"),
    }
}

#[test]
fn null_sink_is_send() {
    fn assert_send<T: Send>() {}
    assert_send::<NullSemanticSink>();
}

#[test]
fn error_kind_round_trips_through_serde() {
    for kind in [
        SemanticErrorKind::Configuration,
        SemanticErrorKind::AgentNative,
        SemanticErrorKind::ApiRemote,
        SemanticErrorKind::Interrupted,
        SemanticErrorKind::Unknown,
    ] {
        let event = SemanticEvent::Error {
            message: "boom".into(),
            terminal: true,
            kind,
            extra: json!({}),
        };
        let value = serde_json::to_value(&event).unwrap();
        let decoded: SemanticEvent = serde_json::from_value(value).unwrap();
        match decoded {
            SemanticEvent::Error {
                kind: decoded_kind, ..
            } => {
                assert_eq!(decoded_kind, kind);
            }
            other => panic!("expected Error, got {other:?}"),
        }
    }
}

#[test]
fn error_kind_default_is_unknown_when_field_missing() {
    let raw = json!({
        "type": "error",
        "message": "legacy payload",
        "terminal": true,
        "extra": {}
    });
    let decoded: SemanticEvent = serde_json::from_value(raw).unwrap();
    match decoded {
        SemanticEvent::Error { kind, .. } => {
            assert_eq!(kind, SemanticErrorKind::Unknown);
        }
        other => panic!("expected Error, got {other:?}"),
    }
}

#[test]
fn error_kind_as_str_is_stable() {
    assert_eq!(SemanticErrorKind::Configuration.as_str(), "configuration");
    assert_eq!(SemanticErrorKind::AgentNative.as_str(), "agent_native");
    assert_eq!(SemanticErrorKind::ApiRemote.as_str(), "api_remote");
    assert_eq!(SemanticErrorKind::Interrupted.as_str(), "interrupted");
    assert_eq!(SemanticErrorKind::Unknown.as_str(), "unknown");
}

#[derive(Default)]
struct CollectingSink {
    events: Vec<SemanticEvent>,
}

impl SemanticEventSink for CollectingSink {
    fn on_semantic_event(&mut self, event: SemanticEvent) {
        self.events.push(event);
    }
}

#[test]
fn shared_sink_forwards_events_to_inner() {
    let mut sink = SharedSemanticSink::new(CollectingSink::default());
    sink.on_semantic_event(SemanticEvent::Info {
        message: "one".into(),
        extra: json!({}),
    });
    sink.on_semantic_event(SemanticEvent::Warning {
        message: "two".into(),
        extra: json!({}),
    });
    let guard = sink.inner().lock().unwrap();
    assert_eq!(guard.events.len(), 2);
    assert_eq!(guard.events[0].kind_str(), "info");
    assert_eq!(guard.events[1].kind_str(), "warning");
}

#[test]
fn shared_sink_is_clone_shares_inner_state() {
    let sink = SharedSemanticSink::new(CollectingSink::default());
    let mut a = sink.clone();
    let mut b = sink.clone();
    a.on_semantic_event(SemanticEvent::Info {
        message: "from-a".into(),
        extra: json!({}),
    });
    b.on_semantic_event(SemanticEvent::Info {
        message: "from-b".into(),
        extra: json!({}),
    });
    let guard = sink.inner().lock().unwrap();
    assert_eq!(guard.events.len(), 2);
}

#[derive(Default)]
struct PanicOnceSink {
    events: Vec<SemanticEvent>,
    has_panicked: bool,
}

impl SemanticEventSink for PanicOnceSink {
    fn on_semantic_event(&mut self, event: SemanticEvent) {
        if !self.has_panicked {
            self.has_panicked = true;
            panic!("intentional panic to poison shared sink mutex");
        }
        self.events.push(event);
    }
}

#[test]
fn shared_sink_recovers_after_poisoned_lock() {
    let sink = SharedSemanticSink::new(PanicOnceSink::default());
    let mut first = sink.clone();
    let mut second = sink.clone();

    let first_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        first.on_semantic_event(SemanticEvent::Info {
            message: "boom".into(),
            extra: json!({}),
        });
    }));
    assert!(first_result.is_err());

    second.on_semantic_event(SemanticEvent::Warning {
        message: "still alive".into(),
        extra: json!({}),
    });

    let guard = match sink.inner().lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    assert_eq!(guard.events.len(), 1);
    assert_eq!(guard.events[0].kind_str(), "warning");
}

#[test]
fn observed_sink_sets_flag_on_first_event() {
    let flag = Arc::new(AtomicBool::new(false));
    let mut sink = ObservedSemanticSink::new(CollectingSink::default(), flag.clone());
    assert!(!flag.load(Ordering::SeqCst));
    sink.on_semantic_event(SemanticEvent::SessionStart {
        session_id: None,
        model: None,
        extra: json!({}),
    });
    assert!(flag.load(Ordering::SeqCst));
}

#[test]
fn observed_sink_forwards_every_event() {
    let flag = Arc::new(AtomicBool::new(false));
    let mut sink = ObservedSemanticSink::new(CollectingSink::default(), flag);
    sink.on_semantic_event(SemanticEvent::TurnStart { extra: json!({}) });
    sink.on_semantic_event(SemanticEvent::OutputText {
        text: "hi".into(),
        extra: json!({}),
    });
    sink.on_semantic_event(SemanticEvent::TurnComplete {
        provider_status: None,
        token_usage: None,
        cost_usd: None,
        duration_ms: None,
        extra: json!({}),
    });
    assert_eq!(sink.inner.events.len(), 3);
}

#[test]
fn observed_sink_exposes_shared_flag() {
    let flag = Arc::new(AtomicBool::new(false));
    let sink = ObservedSemanticSink::new(CollectingSink::default(), flag.clone());
    let observed_flag = sink.stdout_event_seen();
    assert!(Arc::ptr_eq(&flag, &observed_flag));
}
