//! The Claude Code session registry reader. Records follow the shape
//! observed on Claude Code 2.1.284 (field names and types; the values here
//! are synthetic). Each matrix row edits one load-bearing field of the
//! control record.

use serde_json::{Value, json};

use super::*;

const STARTED_AT_MS: u64 = 1_790_658_000_123;

fn control() -> Value {
    json!({
        "pid": 4242,
        "sessionId": "3f1c9b1e-7a2d-4e53-9b8e-0c1d2e3f4a5b",
        "cwd": "/work/project",
        "entrypoint": "cli",
        "kind": "interactive",
        "messagingSocketPath": "/tmp/cc-socks/4242.sock",
        "name": "fix the parser",
        "nameSince": 1_790_658_100_000u64,
        "nameSource": "summary",
        "peerFeatures": ["a", "b", "c"],
        "peerProtocol": 1,
        "pidDomain": "darwin",
        "procStart": "Mon Sep 28 10:00:00 2026",
        "startedAt": STARTED_AT_MS,
        "status": "busy",
        "statusUpdatedAt": 1_790_658_200_000u64,
        "updatedAt": 1_790_658_200_000u64,
        "version": "2.1.284",
    })
}

fn with(edit: impl FnOnce(&mut serde_json::Map<String, Value>)) -> String {
    let mut record = control();
    edit(record.as_object_mut().unwrap());
    record.to_string()
}

#[test]
fn a_record_walks_the_input_robustness_matrix() {
    let record = read_record(&control().to_string()).unwrap();
    assert_eq!(record.pid, 4242);
    assert_eq!(record.session_id, "3f1c9b1e-7a2d-4e53-9b8e-0c1d2e3f4a5b");
    assert_eq!(record.started_at_ms, STARTED_AT_MS);
    assert_eq!(record.status.as_deref(), Some("busy"));

    for (name, expected) in [
        ("pid", RecordError::Missing("pid")),
        ("sessionId", RecordError::Missing("sessionId")),
        ("startedAt", RecordError::Missing("startedAt")),
        ("kind", RecordError::Missing("kind")),
        ("entrypoint", RecordError::Missing("entrypoint")),
    ] {
        assert_eq!(read_record(&with(|r| { r.remove(name); })), Err(expected), "absent {name}");
        assert_eq!(read_record(&with(|r| { r.insert(name.into(), Value::Null); })), Err(RecordError::Null(name)), "null {name}");
    }
    for (name, wrong) in [
        ("pid", json!("4242")),
        ("pid", json!(-1)),
        ("pid", json!(0)),
        ("pid", json!(4_294_967_296u64)),
        ("pid", json!(42.5)),
        ("sessionId", json!(7)),
        ("sessionId", json!("")),
        ("startedAt", json!("1790658000123")),
        ("startedAt", json!(-5)),
        ("kind", json!(["interactive"])),
        ("entrypoint", json!(1)),
    ] {
        assert!(matches!(read_record(&with(|r| { r.insert(name.into(), wrong.clone()); })), Err(RecordError::WrongType { .. })), "{name} = {wrong}");
    }
    // Optional fields: absent is fine, null or a wrong type is not.
    for name in ["status", "cwd", "name", "version"] {
        let absent = read_record(&with(|r| { r.remove(name); })).unwrap();
        assert!(match name {
            "status" => absent.status.is_none(),
            "cwd" => absent.cwd.is_none(),
            "name" => absent.name.is_none(),
            _ => absent.version.is_none(),
        });
        assert_eq!(read_record(&with(|r| { r.insert(name.into(), Value::Null); })), Err(RecordError::Null(name)));
        assert!(matches!(read_record(&with(|r| { r.insert(name.into(), json!(12)); })), Err(RecordError::WrongType { .. })));
    }
    // Not an object, and trailing content.
    for text in ["[]", "\"record\"", "", "not json", &format!("{} trailing", control())] {
        assert_eq!(read_record(text), Err(RecordError::NotAnObject), "{text:?}");
    }
}

fn record() -> RegistryRecord {
    read_record(&control().to_string()).unwrap()
}

#[test]
fn a_live_interactive_session_maps_to_the_researched_profile() {
    let started = STARTED_AT_MS / 1000 - 30;
    let observed = observation(&record(), Some(started)).unwrap();
    assert_eq!(observed.process, ProcessStartIdentity::new(4242, started.to_string()).unwrap());
    assert_eq!(observed.conversation, "3f1c9b1e-7a2d-4e53-9b8e-0c1d2e3f4a5b");
    assert_eq!(observed.launch_profile.as_deref(), Some(INTERACTIVE_PROFILE));
    assert_eq!(observed.launch_mode, LaunchMode::Interactive);
    assert_eq!(observed.state, ExecutionState::Working);
    assert_eq!(observed.provider_version.as_deref(), Some("2.1.284"));
    assert_eq!(observed.cwd.as_deref(), Some("/work/project"));
    assert_eq!(observed.name.as_deref(), Some("fix the parser"));
}

#[test]
fn a_dead_or_reused_process_names_no_session() {
    assert!(observation(&record(), None).is_none(), "no such process");
    let reused = STARTED_AT_MS / 1000 + 60;
    assert!(observation(&record(), Some(reused)).is_none(), "a process that started after the session is another process");
    let tolerated = STARTED_AT_MS / 1000 + START_TOLERANCE_SECS;
    assert!(observation(&record(), Some(tolerated)).is_some(), "whole-second rounding is tolerated");
}

#[test]
fn state_is_only_what_the_record_says() {
    for (status, state) in [
        (Some("busy"), ExecutionState::Working),
        (Some("idle"), ExecutionState::Idle),
        (Some("shell"), ExecutionState::Unknown),
        (Some(""), ExecutionState::Unknown),
        (None, ExecutionState::Unknown),
    ] {
        let record = RegistryRecord { status: status.map(str::to_string), ..record() };
        assert_eq!(observation(&record, Some(0)).unwrap().state, state, "{status:?}");
    }
}

#[test]
fn sessions_the_registry_cannot_classify_have_no_profile() {
    for (kind, entrypoint, mode) in [
        ("interactive", "sdk-cli", LaunchMode::NonInteractive),
        ("interactive", "sdk-ts", LaunchMode::NonInteractive),
        ("print", "cli", LaunchMode::NonInteractive),
        ("interactive", "desktop", LaunchMode::Interactive),
    ] {
        let record = RegistryRecord { kind: kind.into(), entrypoint: entrypoint.into(), ..record() };
        let observed = observation(&record, Some(0)).unwrap();
        assert_eq!(observed.launch_profile, None, "{kind}/{entrypoint}");
        assert_eq!(observed.launch_mode, mode, "{kind}/{entrypoint}");
    }
}

#[test]
fn the_registry_lives_under_the_claude_config_directory() {
    // Absolute on every OS (a drive-letter path on Windows).
    let root = tempfile::tempdir().unwrap();
    let home = root.path().join("home");
    let custom = root.path().join("custom").join("claude");
    let default = home.join(".claude").join("sessions");
    assert_eq!(registry_dir(None, Some(&home)), Some(default.clone()));
    assert_eq!(registry_dir(Some(custom.to_str().unwrap()), Some(&home)), Some(custom.join("sessions")));
    assert_eq!(registry_dir(Some("relative"), Some(&home)), Some(default), "only an absolute override counts");
    assert_eq!(registry_dir(None, None), None);
}

#[test]
fn a_scan_reads_pid_named_records_and_keeps_live_sessions() {
    let dir = tempfile::tempdir().unwrap();
    let write = |name: &str, text: &str| std::fs::write(dir.path().join(name), text).unwrap();
    write("4242.json", &control().to_string());
    // Another live session, idle.
    write("5151.json", &with(|r| {
        r.insert("pid".into(), json!(5151));
        r.insert("sessionId".into(), json!("second"));
        r.insert("status".into(), json!("idle"));
    }));
    // A stale record: its process is gone.
    write("6161.json", &with(|r| { r.insert("pid".into(), json!(6161)); }));
    // Not records, or damaged ones, never hide the others.
    write("7171.json", "{ not json");
    write("8181.json", &with(|r| { r.insert("pid".into(), json!(9999)); }));
    write("notes.json", &control().to_string());
    write("4242.lock", "");
    std::fs::create_dir(dir.path().join("1234")).unwrap();

    let alive = |pid: u32| matches!(pid, 4242 | 5151 | 9999).then_some(STARTED_AT_MS / 1000);
    let mut found = scan(dir.path(), alive).unwrap();
    found.sort_by(|a, b| a.conversation.cmp(&b.conversation));
    let conversations: Vec<_> = found.iter().map(|o| o.conversation.as_str()).collect();
    assert_eq!(conversations, ["3f1c9b1e-7a2d-4e53-9b8e-0c1d2e3f4a5b", "second"]);

    assert!(scan(&dir.path().join("absent"), alive).unwrap().is_empty(), "no registry means no sessions");
}

#[tokio::test]
async fn the_discoverer_implements_the_researched_interactive_registry_method() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("4242.json"), control().to_string()).unwrap();
    fn alive(pid: u32) -> Option<u64> {
        (pid == 4242).then_some(STARTED_AT_MS / 1000)
    }
    for os in [HostOs::Macos, HostOs::Linux, HostOs::Windows] {
        let discoverer = ClaudeRegistryDiscoverer::for_host(dir.path().to_path_buf(), os, alive).unwrap();
        let record = crate::steering::facts(Provider::Claude).discovery.iter().find(|r| r.id == discoverer.discovery_id()).unwrap();
        assert_eq!((record.os, record.origin, record.method), (os, LaunchOrigin::Native, DiscoveryMethod::ProviderRegistry));
        assert_eq!(record.profile_id, INTERACTIVE_PROFILE);
    }
    let discoverer = ClaudeRegistryDiscoverer::for_host(dir.path().to_path_buf(), HostOs::Macos, alive).unwrap();
    assert_eq!(discoverer.discover().await.unwrap().len(), 1);
}
