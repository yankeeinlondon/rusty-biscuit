//! Readers and projection over records captured from Codex 0.157.1's
//! app-server (macOS, disposable `CODEX_HOME`, scripted model). Each matrix
//! test starts from one real record and edits one load-bearing field per
//! cell; the unedited control row proves the edit is what changes the result.

use serde_json::{Value, json};

use super::*;

const INITIALIZE: &str = r#"{"id":"claudine-1","result":{"userAgent":"claudine/0.157.1 (Mac OS 27.2.0; arm64) WezTerm/20260716-195552-76b606ec (claudine; 0.1.0)","codexHome":"/tmp/home","platformFamily":"unix","platformOs":"macos"}}"#;
const THREAD_START: &str = r#"{"id":"claudine-2","result":{"thread":{"id":"01a0eb8a-ecce-7a13-87ba-50f7e406c134","status":{"type":"idle"},"modelProvider":"mock"},"model":"mock-model"}}"#;
const TURN_START: &str = r#"{"id":"claudine-3","result":{"turn":{"id":"01a0eb8a-ecfd-7281-ba73-be1affa1a090","items":[],"itemsView":"notLoaded","status":"inProgress","error":null,"startedAt":null,"completedAt":null,"durationMs":null}}}"#;
const STEER: &str = r#"{"id":"claudine-5","result":{"turnId":"01a0eb8a-ecfd-74f0-90e9-1710f12cbae5"}}"#;
const STEER_REFUSED: &str = r#"{"error":{"code":-32600,"message":"expected active turn id `wrong` but found `01a0eb8a-ecfd-74f0-90e9-1710f12cbae5`"},"id":"claudine-4"}"#;
const TURN_COMPLETED: &str = r#"{"method":"turn/completed","params":{"threadId":"01a0eb8a-ecce-7a13-87ba-50f7e406c134","turn":{"id":"01a0eb8a-ecfd-7281-ba73-be1affa1a090","items":[],"itemsView":"summary","status":"completed","error":null,"startedAt":1790658145,"completedAt":1790658145,"durationMs":46}},"emittedAtMs":1790658145585}"#;
const THREAD_READ: &str = r#"{"id":"claudine-7","result":{"thread":{"id":"01a0eb8a-ecce-7432-81d1-6fb51dba267b","status":{"type":"active","activeFlags":[]}}}}"#;
const APPROVAL: &str = r#"{"id":0,"method":"item/commandExecution/requestApproval","params":{"threadId":"t","turnId":"u","itemId":"i"}}"#;

fn value(line: &str) -> Value {
    serde_json::from_str(line).unwrap()
}

fn result_of(line: &str) -> Value {
    match read_message(line).unwrap() {
        Message::Response(Response { outcome: Ok(result), .. }) => result,
        other => panic!("not a successful response: {other:?}"),
    }
}

/// Replaces (or with `None`, removes) the member at `path` of `line`.
fn edit(line: &str, path: &[&str], replacement: Option<Value>) -> String {
    let mut root = value(line);
    let (last, parents) = path.split_last().unwrap();
    let mut node = &mut root;
    for part in parents {
        node = node.get_mut(*part).unwrap();
    }
    let map = node.as_object_mut().unwrap();
    match replacement {
        Some(value) => {
            map.insert((*last).to_string(), value);
        }
        None => {
            map.remove(*last);
        }
    }
    root.to_string()
}

#[test]
fn messages_are_classified_by_json_rpc_shape() {
    assert!(matches!(read_message(INITIALIZE).unwrap(), Message::Response(Response { id: Some(id), outcome: Ok(_) }) if id == "claudine-1"));
    assert_eq!(
        read_message(STEER_REFUSED).unwrap(),
        Message::Response(Response {
            id: Some("claudine-4".into()),
            outcome: Err(RpcFailure {
                code: -32600,
                message: "expected active turn id `wrong` but found `01a0eb8a-ecfd-74f0-90e9-1710f12cbae5`".into()
            }),
        })
    );
    assert!(matches!(read_message(APPROVAL).unwrap(), Message::Request(ServerRequest { id: Value::Number(_), ref method, .. }) if method == "item/commandExecution/requestApproval"));
    assert!(matches!(read_message(TURN_COMPLETED).unwrap(), Message::Notification { ref method, .. } if method == "turn/completed"));
    for other in ["", "not json", "[1,2]", "2026-09-29T05:03:47Z ERROR codex_core::session", r#"{"emittedAtMs":1}"#] {
        assert_eq!(read_message(other).unwrap(), Message::Other, "{other:?}");
    }
}

#[test]
fn response_shape_walks_the_input_robustness_matrix() {
    // Control.
    assert!(matches!(read_message(STEER).unwrap(), Message::Response(Response { outcome: Ok(_), .. })));
    // A numeric or null id correlates with nothing; it is still a response.
    assert!(matches!(read_message(&edit(STEER, &["id"], Some(json!(5)))).unwrap(), Message::Response(Response { id: None, .. })));
    assert!(matches!(read_message(&edit(STEER, &["id"], Some(Value::Null))).unwrap(), Message::Response(Response { id: None, .. })));
    // Neither result nor error.
    assert_eq!(read_message(&edit(STEER, &["result"], None)), Err(MessageError::Missing("result")));
    // Both.
    let both = edit(STEER, &["error"], Some(json!({"code": 1, "message": "x"})));
    assert_eq!(read_message(&both), Err(MessageError::Ambiguous));
    // Error object: null, wrong type, code absent/null/wrong type.
    assert_eq!(read_message(&edit(STEER_REFUSED, &["error"], Some(Value::Null))), Err(MessageError::Null("error")));
    assert!(matches!(read_message(&edit(STEER_REFUSED, &["error"], Some(json!("boom")))), Err(MessageError::WrongType { field: "error", .. })));
    assert_eq!(read_message(&edit(STEER_REFUSED, &["error", "code"], None)), Err(MessageError::Missing("code")));
    assert_eq!(read_message(&edit(STEER_REFUSED, &["error", "code"], Some(Value::Null))), Err(MessageError::Null("code")));
    assert!(matches!(read_message(&edit(STEER_REFUSED, &["error", "code"], Some(json!("-32600")))), Err(MessageError::WrongType { field: "code", .. })));
    // A refusal without text is still a refusal.
    let silent = edit(STEER_REFUSED, &["error", "message"], None);
    assert!(matches!(read_message(&silent).unwrap(), Message::Response(Response { outcome: Err(RpcFailure { code: -32600, .. }), .. })));
    // A method that is not a string, and a request id that is null.
    assert!(matches!(read_message(&edit(APPROVAL, &["method"], Some(json!(7)))), Err(MessageError::WrongType { field: "method", .. })));
    assert_eq!(read_message(&edit(APPROVAL, &["id"], Some(Value::Null))), Err(MessageError::Null("id")));
    // Trailing content after a valid record is not a record.
    assert_eq!(read_message(&format!("{STEER} trailing")).unwrap(), Message::Other);
}

#[test]
fn initialize_reads_the_exact_version_or_none() {
    let info = initialize_info(&result_of(INITIALIZE)).unwrap();
    assert_eq!(info.provider_version.as_deref(), Some("0.157.1"));
    assert_eq!(initialize_info(&result_of(&edit(INITIALIZE, &["result", "userAgent"], None))), Err(MessageError::Missing("userAgent")));
    assert_eq!(initialize_info(&result_of(&edit(INITIALIZE, &["result", "userAgent"], Some(Value::Null)))), Err(MessageError::Null("userAgent")));
    assert!(initialize_info(&result_of(&edit(INITIALIZE, &["result", "userAgent"], Some(json!(157))))).is_err());
    for (agent, version) in [
        ("codex_cli_rs/0.153.4 (Linux 6.1; x86_64)", Some("0.153.4")),
        ("claudine/0.157.1", Some("0.157.1")),
        ("claudine/0.157.1-alpha.2 (Mac OS)", None),
        ("claudine/ (Mac OS)", None),
        ("claudine/0..1", None),
        ("claudine", None),
        ("", None),
    ] {
        assert_eq!(provider_version(agent).as_deref(), version, "{agent:?}");
    }
}

#[test]
fn thread_and_turn_results_walk_the_matrix() {
    assert_eq!(started_thread(&result_of(THREAD_START)).unwrap(), "01a0eb8a-ecce-7a13-87ba-50f7e406c134");
    for (path, replacement, expected) in [
        (&["result", "thread"][..], None, MessageError::Missing("thread")),
        (&["result", "thread"][..], Some(Value::Null), MessageError::Null("thread")),
        (&["result", "thread", "id"][..], None, MessageError::Missing("id")),
        (&["result", "thread", "id"][..], Some(Value::Null), MessageError::Null("id")),
        (&["result", "thread", "id"][..], Some(json!(12)), MessageError::WrongType { field: "id", expected: "a string" }),
        (&["result", "thread", "id"][..], Some(json!("")), MessageError::WrongType { field: "id", expected: "a non-empty string" }),
    ] {
        assert_eq!(started_thread(&result_of(&edit(THREAD_START, path, replacement.clone()))), Err(expected), "{path:?} {replacement:?}");
    }

    assert_eq!(
        started_turn(&result_of(TURN_START)).unwrap(),
        TurnRef { id: "01a0eb8a-ecfd-7281-ba73-be1affa1a090".into(), status: TurnStatus::InProgress }
    );
    assert_eq!(started_turn(&result_of(&edit(TURN_START, &["result", "turn", "status"], None))), Err(MessageError::Missing("status")));
    assert_eq!(started_turn(&result_of(&edit(TURN_START, &["result", "turn", "status"], Some(Value::Null)))), Err(MessageError::Null("status")));
    assert!(matches!(
        started_turn(&result_of(&edit(TURN_START, &["result", "turn", "status"], Some(json!("paused"))))),
        Err(MessageError::UnknownValue { field: "status", .. })
    ));
    assert!(matches!(started_turn(&result_of(&edit(TURN_START, &["result", "turn", "status"], Some(json!(1))))), Err(MessageError::WrongType { .. })));

    assert_eq!(steered_turn(&result_of(STEER)).unwrap(), "01a0eb8a-ecfd-74f0-90e9-1710f12cbae5");
    assert_eq!(steered_turn(&result_of(&edit(STEER, &["result", "turnId"], None))), Err(MessageError::Missing("turnId")));
    assert_eq!(steered_turn(&result_of(&edit(STEER, &["result"], Some(json!({}))))), Err(MessageError::Missing("turnId")));
    assert!(steered_turn(&result_of(&edit(STEER, &["result", "turnId"], Some(json!(["x"]))))).is_err());
    assert!(steered_turn(&Value::Null).is_err());
}

#[test]
fn turn_events_and_thread_activity_walk_the_matrix() {
    let params = value(TURN_COMPLETED)["params"].clone();
    let event = turn_event(&params).unwrap();
    assert_eq!(event.thread_id, "01a0eb8a-ecce-7a13-87ba-50f7e406c134");
    assert_eq!(event.turn.status, TurnStatus::Completed);
    let edited = |path: &[&str], replacement: Option<Value>| turn_event(&value(&edit(TURN_COMPLETED, path, replacement))["params"]);
    assert_eq!(edited(&["params", "threadId"], None), Err(MessageError::Missing("threadId")));
    assert_eq!(edited(&["params", "turn"], Some(Value::Null)), Err(MessageError::Null("turn")));
    assert_eq!(edited(&["params", "turn", "id"], Some(json!(""))), Err(MessageError::WrongType { field: "id", expected: "a non-empty string" }));
    assert_eq!(edited(&["params", "turn", "status"], Some(json!("interrupted"))).unwrap().turn.status, TurnStatus::Interrupted);
    assert_eq!(edited(&["params", "turn", "status"], Some(json!("failed"))).unwrap().turn.status, TurnStatus::Failed);

    let thread = "01a0eb8a-ecce-7432-81d1-6fb51dba267b";
    assert_eq!(thread_activity(&result_of(THREAD_READ), thread).unwrap(), ThreadActivity::Active);
    for (status, activity) in [("idle", ThreadActivity::Idle), ("notLoaded", ThreadActivity::NotLoaded), ("systemError", ThreadActivity::SystemError)] {
        let line = edit(THREAD_READ, &["result", "thread", "status", "type"], Some(json!(status)));
        assert_eq!(thread_activity(&result_of(&line), thread).unwrap(), activity);
    }
    // Another thread's state is never read as this one's.
    assert!(matches!(thread_activity(&result_of(THREAD_READ), "other"), Err(MessageError::UnknownValue { field: "thread.id", .. })));
    assert_eq!(thread_activity(&result_of(&edit(THREAD_READ, &["result", "thread", "status"], None)), thread), Err(MessageError::Missing("status")));
    assert_eq!(thread_activity(&result_of(&edit(THREAD_READ, &["result", "thread", "status", "type"], Some(Value::Null))), thread), Err(MessageError::Null("type")));
    assert!(matches!(
        thread_activity(&result_of(&edit(THREAD_READ, &["result", "thread", "status", "type"], Some(json!("busy")))), thread),
        Err(MessageError::UnknownValue { field: "status.type", .. })
    ));
}

fn project_line(projector: &mut Projector, line: &str) -> Vec<Projected> {
    match read_message(line).unwrap() {
        Message::Notification { method, params } => projector.project(&method, &params),
        other => panic!("not a notification: {other:?}"),
    }
}

#[test]
fn a_recorded_turn_projects_to_the_events_exec_prints() {
    let transcript = [
        r#"{"method":"turn/started","params":{"threadId":"thr","turn":{"id":"t1","items":[],"status":"inProgress"}}}"#,
        r#"{"method":"item/started","params":{"item":{"type":"userMessage","id":"u","clientId":null,"content":[]},"threadId":"thr","turnId":"t1"}}"#,
        r#"{"method":"item/started","params":{"item":{"type":"commandExecution","id":"cr1","command":"/bin/zsh -lc 'sleep 4'","cwd":"/tmp","processId":"54757","source":"unifiedExecStartup","status":"inProgress","commandActions":[],"aggregatedOutput":null,"exitCode":null,"durationMs":null},"threadId":"thr","turnId":"t1"}}"#,
        r#"{"method":"item/commandExecution/outputDelta","params":{"delta":"x"}}"#,
        r#"{"method":"item/completed","params":{"item":{"type":"commandExecution","id":"cr1","command":"/bin/zsh -lc 'sleep 4'","cwd":"/tmp","status":"completed","aggregatedOutput":"done\n","exitCode":0,"durationMs":3875},"threadId":"thr","turnId":"t1"}}"#,
        r#"{"method":"thread/tokenUsage/updated","params":{"threadId":"thr","turnId":"t1","tokenUsage":{"total":{"totalTokens":30,"inputTokens":20,"cachedInputTokens":4,"cacheWriteInputTokens":0,"outputTokens":10,"reasoningOutputTokens":0},"last":{}}}}"#,
        r#"{"method":"item/completed","params":{"item":{"type":"reasoning","id":"r","summary":["first","second"],"content":[]},"threadId":"thr","turnId":"t1"}}"#,
        r#"{"method":"item/completed","params":{"item":{"type":"agentMessage","id":"mr2","text":"Done after the tool.","phase":null},"threadId":"thr","turnId":"t1"}}"#,
        r#"{"method":"thread/status/changed","params":{"threadId":"thr","status":{"type":"idle"}}}"#,
        r#"{"method":"turn/completed","params":{"threadId":"thr","turn":{"id":"t1","items":[],"status":"completed","error":null,"durationMs":4102}}}"#,
    ];
    let mut projector = Projector::new();
    let events: Vec<Value> = transcript
        .iter()
        .flat_map(|line| project_line(&mut projector, line))
        .map(|projected| match projected {
            Projected::Event(event) => serde_json::to_value(event).unwrap(),
            other => panic!("unexpected {other:?}"),
        })
        .collect();
    let kinds: Vec<&str> = events.iter().map(|event| event["type"].as_str().unwrap()).collect();
    assert_eq!(
        kinds,
        ["thread.started", "turn.started", "item.started", "item.completed", "item.completed", "item.completed", "turn.completed"]
    );
    assert_eq!(events[0]["thread_id"], "thr");
    assert_eq!(events[2]["item"]["type"], "command_exec");
    assert_eq!(events[3]["item"]["aggregated_output"], "done\n");
    assert_eq!(events[3]["item"]["exit_code"], 0);
    assert_eq!(events[3]["item"]["status"], "completed");
    assert_eq!(events[4]["item"]["text"], "first\n\nsecond");
    assert_eq!(events[5]["item"]["text"], "Done after the tool.");
    assert_eq!(events[6]["usage"]["input_tokens"], 20);
    assert_eq!(events[6]["usage"]["cached_input_tokens"], 4);
    assert_eq!(events[6]["usage"]["output_tokens"], 10);
    assert_eq!(events[6]["duration_ms"], 4102);
}

#[test]
fn usage_is_per_turn_and_the_thread_is_announced_once() {
    let mut projector = Projector::new();
    let usage = |total: u64| {
        json!({"threadId": "thr", "tokenUsage": {"total": {"totalTokens": total, "inputTokens": total, "outputTokens": 0}}})
    };
    let completed = json!({"threadId": "thr", "turn": {"id": "t", "status": "completed"}});
    let started = json!({"threadId": "thr", "turn": {"id": "t", "status": "inProgress"}});

    assert_eq!(projector.project("turn/started", &started).len(), 2);
    projector.project("thread/tokenUsage/updated", &usage(15));
    let first = projector.project("turn/completed", &completed);
    assert_eq!(projector.project("turn/started", &started).len(), 1, "the thread is announced once");
    projector.project("thread/tokenUsage/updated", &usage(40));
    let second = projector.project("turn/completed", &completed);
    let input = |projected: &[Projected]| match &projected[0] {
        Projected::Event(event) => match event.as_ref() {
            CodexEvent::TurnCompleted(done) => done.usage.as_ref().and_then(|usage| usage.input_tokens),
            other => panic!("{other:?}"),
        },
        other => panic!("{other:?}"),
    };
    assert_eq!(input(&first), Some(15));
    assert_eq!(input(&second), Some(25));
}

#[test]
fn failures_interruptions_errors_and_warnings_project_distinctly() {
    let mut projector = Projector::new();
    let failed = json!({"threadId": "thr", "turn": {"id": "t", "status": "failed", "error": {"message": "stream disconnected", "codexErrorInfo": {"responseStreamDisconnected": {}}}}});
    match &projector.project("turn/completed", &failed)[..] {
        [Projected::Event(event)] => match event.as_ref() {
            CodexEvent::TurnFailed(error) => {
                assert_eq!(error.resolved_message().as_deref(), Some("stream disconnected"));
                assert_eq!(error.resolved_kind().as_deref(), Some("responseStreamDisconnected"));
            }
            other => panic!("{other:?}"),
        },
        other => panic!("{other:?}"),
    }
    let interrupted = json!({"threadId": "thr", "turn": {"id": "t", "status": "interrupted"}});
    assert!(matches!(&projector.project("turn/completed", &interrupted)[..], [Projected::Interrupted]));
    let retrying = json!({"error": {"message": "reconnecting"}, "willRetry": true, "threadId": "thr", "turnId": "t"});
    assert!(matches!(&projector.project("error", &retrying)[..], [Projected::Warning(message)] if message.contains("reconnecting")));
    let fatal = json!({"error": {"message": "quota", "codexErrorInfo": "usageLimitExceeded"}, "willRetry": false, "threadId": "thr", "turnId": "t"});
    match &projector.project("error", &fatal)[..] {
        [Projected::Event(event)] => match event.as_ref() {
            CodexEvent::Error(error) => assert_eq!(error.resolved_kind().as_deref(), Some("usageLimitExceeded")),
            other => panic!("{other:?}"),
        },
        other => panic!("{other:?}"),
    }
    let warning = json!({"threadId": "thr", "message": "Model metadata for `mock-model` not found."});
    assert!(matches!(&projector.project("warning", &warning)[..], [Projected::Warning(_)]));
    for quiet in ["item/agentMessage/delta", "thread/status/changed", "account/rateLimits/updated", "remoteControl/status/changed"] {
        assert!(projector.project(quiet, &json!({})).is_empty(), "{quiet}");
    }
}
