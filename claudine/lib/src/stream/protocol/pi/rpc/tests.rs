use serde_json::{Value, json};

use super::*;

/// A real Pi 0.84.4 `get_state` response (the fixture session file path and
/// model detail trimmed).
const GET_STATE: &str = r#"{"command":"get_state","data":{"autoCompactionEnabled":true,"followUpMode":"one-at-a-time","isCompacting":false,"isStreaming":false,"messageCount":0,"model":{"id":"fixture","provider":"claudine-probe"},"pendingMessageCount":0,"sessionFile":"/tmp/sessions/s.jsonl","sessionId":"01a0eae3-8823-705a-8978-46829cdf84e7","steeringMode":"one-at-a-time","thinkingLevel":"off"},"id":"s1","success":true,"type":"response"}"#;
/// A real Pi 0.84.4 refusal.
const REFUSAL: &str = r#"{"command":"no_such_command","error":"Unknown command: no_such_command","id":"u1","success":false,"type":"response"}"#;
/// A real Pi 0.84.4 dialog request.
const CONFIRM: &str = r#"{"id":"6d74892c-b9fc-49a5-944c-6533b5223f4a","message":"Approve the fixture probe?","method":"confirm","title":"Pi probe","type":"extension_ui_request"}"#;

fn edited(line: &str, edit: impl FnOnce(&mut serde_json::Map<String, Value>)) -> String {
    let mut value: Value = serde_json::from_str(line).unwrap();
    edit(value.as_object_mut().unwrap());
    value.to_string()
}

fn state_edited(edit: impl FnOnce(&mut serde_json::Map<String, Value>)) -> Result<SessionState, RecordError> {
    let line = edited(GET_STATE, |record| edit(record["data"].as_object_mut().unwrap()));
    let Record::Response(response) = read_record(&line)? else { panic!("not a response") };
    session_state(&response.outcome.unwrap())
}

#[test]
fn control_records_read_as_their_positive_results() {
    let Ok(Record::Response(response)) = read_record(GET_STATE) else { panic!("control get_state") };
    assert_eq!((response.id.as_str(), response.command.as_str()), ("s1", "get_state"));
    let state = session_state(response.outcome.as_ref().unwrap()).unwrap();
    assert_eq!(
        state,
        SessionState {
            session_id: "01a0eae3-8823-705a-8978-46829cdf84e7".into(),
            is_streaming: false,
            is_compacting: false,
            pending_messages: 0,
        }
    );
    assert!(state.is_quiescent());

    let Ok(Record::Response(refusal)) = read_record(REFUSAL) else { panic!("control refusal") };
    assert_eq!(refusal.outcome, Err("Unknown command: no_such_command".to_string()));

    assert_eq!(
        read_record(CONFIRM),
        Ok(Record::UiRequest(UiRequest { id: "6d74892c-b9fc-49a5-944c-6533b5223f4a".into(), method: "confirm".into() }))
    );
    assert_eq!(read_record(r#"{"type":"agent_start"}"#), Ok(Record::AgentStart));
    assert_eq!(read_record(r#"{"type":"agent_settled"}"#), Ok(Record::AgentSettled));
}

/// Load-bearing response fields: `id`, `command`, `success`.
#[test]
fn response_walks_the_input_robustness_matrix() {
    for field in ["id", "command", "success"] {
        let absent = edited(GET_STATE, |r| {
            r.remove(field);
        });
        assert_eq!(read_record(&absent), Err(RecordError::Missing(field)), "absent {field}");
        let null = edited(GET_STATE, |r| {
            r.insert(field.into(), Value::Null);
        });
        assert_eq!(read_record(&null), Err(RecordError::Null(field)), "null {field}");
        let wrong = edited(GET_STATE, |r| {
            r.insert(field.into(), if field == "success" { json!("true") } else { json!(7) });
        });
        assert!(matches!(read_record(&wrong), Err(RecordError::WrongType { .. })), "wrong type {field}");
    }
    // Empty: an empty id or command is still a string the owner can compare;
    // it simply correlates with nothing the owner sent.
    let empty_id = edited(GET_STATE, |r| {
        r.insert("id".into(), json!(""));
    });
    assert!(matches!(read_record(&empty_id), Ok(Record::Response(Response { id, .. })) if id.is_empty()));
    // Duplicate key: last wins, as JSON permits.
    let duplicate = GET_STATE.replacen(r#""success":true"#, r#""success":true,"success":false"#, 1);
    assert!(matches!(read_record(&duplicate), Ok(Record::Response(Response { outcome: Err(_), .. }))));
    // Trailing content: not a record the owner acts on, so the command stays
    // unanswered rather than being read as accepted.
    assert_eq!(read_record(&format!("{GET_STATE} trailing")), Ok(Record::Other));
    // A refusal without error text is still a refusal.
    let bare = edited(REFUSAL, |r| {
        r.remove("error");
    });
    assert!(matches!(read_record(&bare), Ok(Record::Response(Response { outcome: Err(_), .. }))));
}

/// Load-bearing `get_state` fields.
#[test]
fn session_state_walks_the_input_robustness_matrix() {
    for (field, wrong) in [
        ("sessionId", json!(12)),
        ("isStreaming", json!("false")),
        ("isCompacting", json!(0)),
        ("pendingMessageCount", json!(-1)),
    ] {
        assert_eq!(state_edited(|d| {
            d.remove(field);
        }), Err(RecordError::Missing(field)), "absent {field}");
        assert_eq!(state_edited(|d| {
            d.insert(field.into(), Value::Null);
        }), Err(RecordError::Null(field)), "null {field}");
        assert!(matches!(state_edited(|d| {
            d.insert(field.into(), wrong);
        }), Err(RecordError::WrongType { .. })), "wrong type {field}");
    }
    assert!(matches!(state_edited(|d| {
        d.insert("sessionId".into(), json!(""));
    }), Err(RecordError::WrongType { field: "sessionId", .. })), "an empty session id identifies nothing");
    assert!(matches!(state_edited(|d| {
        d.insert("pendingMessageCount".into(), json!(1.5));
    }), Err(RecordError::WrongType { .. })));
    // Non-quiescent control variants.
    assert!(!state_edited(|d| {
        d.insert("isStreaming".into(), json!(true));
    }).unwrap().is_quiescent());
    assert!(!state_edited(|d| {
        d.insert("pendingMessageCount".into(), json!(1));
    }).unwrap().is_quiescent());
    assert!(!state_edited(|d| {
        d.insert("isCompacting".into(), json!(true));
    }).unwrap().is_quiescent());
    assert_eq!(session_state(&json!([])), Err(RecordError::WrongType { field: "data", expected: "an object" }));
    assert_eq!(session_state(&Value::Null), Err(RecordError::WrongType { field: "data", expected: "an object" }));
}

/// Load-bearing UI request fields: `id`, `method`.
#[test]
fn ui_request_walks_the_input_robustness_matrix() {
    for field in ["id", "method"] {
        assert_eq!(read_record(&edited(CONFIRM, |r| {
            r.remove(field);
        })), Err(RecordError::Missing(field)));
        assert_eq!(read_record(&edited(CONFIRM, |r| {
            r.insert(field.into(), Value::Null);
        })), Err(RecordError::Null(field)));
        assert!(matches!(read_record(&edited(CONFIRM, |r| {
            r.insert(field.into(), json!(["confirm"]));
        })), Err(RecordError::WrongType { .. })));
    }
    // An empty method is not a documented one.
    let Ok(Record::UiRequest(empty)) = read_record(&edited(CONFIRM, |r| {
        r.insert("method".into(), json!(""));
    })) else {
        panic!("empty method still reads");
    };
    assert_eq!(ui_method_kind(&empty.method), UiMethodKind::Unsupported);
}

#[test]
fn ui_methods_are_classified_by_their_documented_behavior() {
    for method in ["select", "confirm", "input", "editor"] {
        assert_eq!(ui_method_kind(method), UiMethodKind::Dialog, "{method}");
    }
    for method in ["notify", "setStatus", "setWidget", "setTitle", "set_editor_text"] {
        assert_eq!(ui_method_kind(method), UiMethodKind::Notice, "{method}");
    }
    for method in ["Confirm", "custom", "prompt"] {
        assert_eq!(ui_method_kind(method), UiMethodKind::Unsupported, "{method}");
    }
}

#[test]
fn unrelated_or_invalid_lines_are_other() {
    for line in ["", "not json", "[1,2]", r#"{"type":"message_update"}"#, r#"{"no_type":true}"#] {
        assert_eq!(read_record(line), Ok(Record::Other), "{line:?}");
    }
}
