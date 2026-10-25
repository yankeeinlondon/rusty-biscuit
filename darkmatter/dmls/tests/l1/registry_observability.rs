//! Level-1 sessions for the trigger-registry figures DMLS logs: the `debug`
//! line each workspace pass ends with (`trigger registries prepared`), and
//! `trigger_registry_wait_ms` on a request's completion event.
//!
//! The pass line counts only the pass's own probes and payload lookups: a
//! document job that runs between two probes is not part of it. The wait
//! figure is measured on an injected [`ManualClock`], which moves only when
//! a test advances it, so each assertion is exact.

use crate::common;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use common::{LspFixture, LspWorkspace};
use dmls::clock::ManualClock;
use dmls::log_capture::CapturedLogs;
use dmls::{RegistryJobKind, StartupEvent};
use lsp_server::RequestId;
use serde_json::{Value, json};

/// `RequestCanceled`.
const REQUEST_CANCELED: i32 = -32800;
/// `ServerCancelled`.
const SERVER_CANCELLED: i32 = -32802;

const ENVELOPE: &str = "kind: schema-trigger\nmatch:\n  prompt: string(required)\n$schema: prompt.yaml\n";
/// A trigger whose payload file does not exist: discovery skips it.
const MISSING_PAYLOAD_ENVELOPE: &str =
    "kind: schema-trigger\nmatch:\n  prompt: string(required)\n$schema: absent-payload.yaml\n";
/// The payload names a property no log line may carry.
const PAYLOAD: &str = "$schema:\n  payloadsecretfield: string(required)\n";
const EDITED_PAYLOAD: &str = "$schema:\n  payloadsecretfield: string\n";
const DOC: &str = "---\nprompt: hello\n---\n\n# Heading\n\nBody.\n";

/// The pass line's fields, in order.
const PASS_FIELDS: [&str; 8] =
    ["probes", "prepared", "fresh", "failed", "payload_hits", "payload_misses", "elapsed_ms", "workspace"];

fn write(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

/// A minimal repository Git discovery accepts, without running `git`, with
/// a trigger, its payload, and a document the trigger matches.
fn trigger_repository(root: &Path) {
    std::fs::create_dir_all(root.join(".git/objects")).unwrap();
    std::fs::create_dir_all(root.join(".git/refs/heads")).unwrap();
    write(&root.join(".git/HEAD"), "ref: refs/heads/main\n");
    write(&root.join(".git/config"), "[core]\n\trepositoryformatversion = 0\n\tbare = false\n");
    write(&root.join("schemas/prompt.trigger.yaml"), ENVELOPE);
    write(&root.join("schemas/prompt.yaml"), PAYLOAD);
    write(&root.join("a.md"), DOC);
}

/// Two repositories, each its own workspace folder, so a workspace pass has
/// a probe for each.
fn two_folder_workspace() -> (LspWorkspace, PathBuf, PathBuf) {
    let workspace = LspWorkspace::new();
    let first = workspace.path().join("first");
    let second = workspace.path().join("second");
    trigger_repository(&first);
    trigger_repository(&second);
    (workspace, first, second)
}

fn uri(path: &Path) -> String {
    url::Url::from_file_path(path).unwrap().to_string()
}

fn params(folders: &[&Path]) -> Value {
    let folders: Vec<Value> = folders
        .iter()
        .map(|folder| json!({ "uri": url::Url::from_directory_path(folder).unwrap().as_str(), "name": "f" }))
        .collect();
    json!({
        "processId": null,
        "clientInfo": { "name": "Observability Client", "version": "1.0" },
        "capabilities": {
            "general": { "positionEncodings": ["utf-16"] },
            "workspace": { "didChangeWatchedFiles": { "dynamicRegistration": true } }
        },
        "workspaceFolders": folders
    })
}

fn open(fixture: &LspFixture<'_>, uri: &str) {
    fixture.notify(
        "textDocument/didOpen",
        json!({ "textDocument": { "uri": uri, "languageId": "markdown", "version": 1, "text": DOC } }),
    );
}

fn hover(fixture: &LspFixture<'_>, id: i32, uri: &str) {
    fixture.send_request_with_id(
        RequestId::from(id),
        "textDocument/hover",
        json!({ "textDocument": { "uri": uri }, "position": { "line": 4, "character": 3 } }),
    );
}

/// Settings that change the effective configuration, so every registry is
/// invalidated and one workspace pass is queued.
fn change_settings(fixture: &LspFixture<'_>) {
    fixture.notify(
        "workspace/didChangeConfiguration",
        json!({ "settings": { "dmls": { "formatting": { "markdown": { "unwrap": true } } } } }),
    );
}

/// The `trigger registries prepared` lines, each as its `key=value` fields
/// after the message.
fn pass_lines(logs: &CapturedLogs) -> Vec<Vec<(String, String)>> {
    logs.matching("trigger registries prepared")
        .into_iter()
        .map(|line| {
            assert!(line.starts_with("DEBUG "), "the pass line is debug: {line}");
            let fields = line.split_once("trigger registries prepared").expect("message").1;
            fields
                .split_whitespace()
                .map(|field| {
                    let (key, value) = field.split_once('=').unwrap_or_else(|| panic!("{field} in {line}"));
                    (key.to_string(), value.to_string())
                })
                .collect()
        })
        .collect()
}

/// The one pass line, its counts (every field but `elapsed_ms` and
/// `workspace`) as `key=value`, after checking it carries exactly the pass
/// fields and nothing but numbers and the workspace identifier.
fn only_pass_counts(logs: &CapturedLogs) -> String {
    let lines = pass_lines(logs);
    assert_eq!(lines.len(), 1, "one workspace pass: {:#?}", logs.lines());
    let fields = &lines[0];
    let keys: Vec<&str> = fields.iter().map(|(key, _)| key.as_str()).collect();
    assert_eq!(keys, PASS_FIELDS, "{fields:?}");
    for (key, value) in fields.iter().filter(|(key, _)| key != "workspace") {
        assert!(value.parse::<u64>().is_ok(), "{key}={value} is a count");
    }
    fields
        .iter()
        .filter(|(key, _)| key != "elapsed_ms" && key != "workspace")
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join(" ")
}

/// No captured line carries payload, trigger, or document text.
fn assert_no_schema_text(logs: &CapturedLogs) {
    for line in logs.matching("trigger registries prepared") {
        for text in ["payloadsecretfield", "prompt: string", "kind: schema-trigger", "absent-payload", "hello"] {
            assert!(!line.contains(text), "{text:?} leaked into {line}");
        }
    }
}

// ── The pass line ───────────────────────────────────────────────────────────

/// A workspace pass held before its second probe runs a document job whose
/// payload lookup misses (its payload changed on disk); the request is
/// answered while the pass still holds. The pass line reports its own two
/// probes and their two hits only, not the interrupting job's miss.
#[test]
fn the_pass_line_excludes_a_document_job_that_interrupts_it() {
    let logs = CapturedLogs::global();
    let (workspace, first, second) = two_folder_workspace();
    let doc = uri(&second.join("a.md"));
    let mut fixture = LspFixture::start_with_registry_probe_gate(&workspace);
    fixture.initialize(params(&[&first, &second]));

    change_settings(&fixture);
    fixture.wait_for_event(StartupEvent::WorkspaceProbePreparing);
    // Not watched: only a preparation that reads it notices, by fingerprint.
    write(&second.join("schemas/prompt.yaml"), EDITED_PAYLOAD);
    open(&fixture, &doc);
    hover(&fixture, 800, &doc);
    // The document job is queued before the first probe is released.
    fixture.round_trip();
    fixture.release_registry_probe();
    fixture.wait_for_event(StartupEvent::WorkspaceProbePreparing);
    let answer = fixture.expect_response(RequestId::from(800));
    assert!(answer.error.is_none(), "answered while the second probe is held: {answer:?}");
    assert!(pass_lines(&logs).is_empty(), "the pass is still running: {:#?}", logs.lines());

    fixture.open_registry_probe_gate();
    fixture.registry_jobs_until_idle();
    assert_eq!(
        only_pass_counts(&logs),
        "probes=2 prepared=2 fresh=0 failed=0 payload_hits=2 payload_misses=0"
    );
    assert_no_schema_text(&logs);
    fixture.shutdown();
}

/// A watched payload edit makes only the first folder's scope stale: the
/// pass prepares it (a miss, since the payload changed) and reports the
/// second, still fresh, as fresh with no lookup of its own.
#[test]
fn a_fresh_probe_records_neither_a_hit_nor_a_miss() {
    let logs = CapturedLogs::global();
    let (workspace, first, second) = two_folder_workspace();
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(params(&[&first, &second]));
    fixture.flush_server();

    let payload = first.join("schemas/prompt.yaml");
    write(&payload, EDITED_PAYLOAD);
    fixture.notify(
        "workspace/didChangeWatchedFiles",
        json!({ "changes": [ { "uri": uri(&payload), "type": 2 } ] }),
    );
    fixture.round_trip();
    fixture.registry_jobs_until_idle();
    assert_eq!(
        only_pass_counts(&logs),
        "probes=2 prepared=1 fresh=1 failed=0 payload_hits=0 payload_misses=1"
    );
    assert_no_schema_text(&logs);
    fixture.shutdown();
}

/// A folder whose trigger names a missing payload skips that trigger and
/// still prepares: the pass counts it as prepared, not failed, and its
/// attempted resolution as a miss.
#[test]
fn a_probe_that_skips_a_trigger_is_counted_as_prepared() {
    let logs = CapturedLogs::global();
    let (workspace, first, second) = two_folder_workspace();
    write(&second.join("schemas/prompt.trigger.yaml"), MISSING_PAYLOAD_ENVELOPE);
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(params(&[&first, &second]));
    fixture.flush_server();

    change_settings(&fixture);
    fixture.round_trip();
    fixture.registry_jobs_until_idle();
    assert_eq!(
        only_pass_counts(&logs),
        "probes=2 prepared=2 fresh=0 failed=0 payload_hits=1 payload_misses=1"
    );
    assert_no_schema_text(&logs);
    fixture.shutdown();
}

/// A folder whose schema root holds a file that is not UTF-8 fails its
/// whole preparation: the pass counts it as failed, not prepared or fresh.
#[test]
fn a_failed_probe_is_counted_apart_from_prepared_and_fresh() {
    let logs = CapturedLogs::global();
    let (workspace, first, second) = two_folder_workspace();
    std::fs::write(second.join("schemas/not-utf8.yaml"), [0xff, 0xfe, 0x00]).unwrap();
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(params(&[&first, &second]));
    fixture.flush_server();

    change_settings(&fixture);
    fixture.round_trip();
    fixture.registry_jobs_until_idle();
    assert_eq!(
        only_pass_counts(&logs),
        "probes=2 prepared=1 fresh=0 failed=1 payload_hits=1 payload_misses=0"
    );
    assert_no_schema_text(&logs);
    fixture.shutdown();
}

// ── The wait figure ─────────────────────────────────────────────────────────

/// A trigger workspace whose document's registry job the session holds, on
/// a manual clock. Returns the document's URI.
fn held_document_session<'w>(workspace: &'w LspWorkspace, clock: &Arc<ManualClock>) -> (LspFixture<'w>, String) {
    let root = workspace.path().to_path_buf();
    trigger_repository(&root);
    let doc = uri(&root.join("a.md"));
    let mut fixture = LspFixture::start_with_registry_gate_and_clock(workspace, clock.clone());
    fixture.initialize(params(&[&root]));
    open(&fixture, &doc);
    fixture.wait_for_event(StartupEvent::RegistryPreparing);
    (fixture, doc)
}

/// The one completion line of request `id`.
fn completion(logs: &CapturedLogs, id: i32) -> String {
    let lines: Vec<String> = logs
        .lines()
        .into_iter()
        .filter(|line| line.contains("request complete") || line.contains("slow request"))
        .filter(|line| line.contains(&format!(" id={id} ")))
        .collect();
    assert_eq!(lines.len(), 1, "one completion event for {id}: {:#?}", logs.lines());
    lines[0].clone()
}

fn assert_fields(line: &str, fields: &[&str]) {
    for field in fields {
        assert!(line.contains(field), "missing {field}: {line}");
    }
}

/// Parked 100 ms on a job that finished in a context the document no longer
/// has, the request is parked again on a second job for 50 ms more. Its one
/// completion event sums both intervals.
#[test]
fn a_requeued_request_reports_the_sum_of_its_parked_intervals() {
    let logs = CapturedLogs::global();
    let clock = Arc::new(ManualClock::new());
    let workspace = LspWorkspace::new();
    let (mut fixture, doc) = held_document_session(&workspace, &clock);
    let root = workspace.path().to_path_buf();
    hover(&fixture, 900, &doc);
    fixture.round_trip();
    clock.advance(Duration::from_millis(100));

    // The document now lies in a package: the held job's context is stale.
    write(&root.join("Cargo.toml"), "[workspace]\nmembers = [\".\"]\n[package]\nname = \"p\"\nversion = \"0.1.0\"\nedition = \"2024\"\n");
    write(&root.join("src/lib.rs"), "");
    fixture.notify(
        "workspace/didChangeWatchedFiles",
        json!({ "changes": [ { "uri": uri(&root.join("Cargo.toml")), "type": 1 } ] }),
    );
    fixture.round_trip();
    fixture.release_registry_job();
    fixture.wait_for_registry_prepared(RegistryJobKind::Document);
    // The finished job's report re-parked the request before this event.
    clock.advance(Duration::from_millis(50));
    fixture.open_registry_gate();
    let answer = fixture.expect_response(RequestId::from(900));
    assert!(answer.error.is_none(), "{answer:?}");
    fixture.shutdown();

    assert_fields(
        &completion(&logs, 900),
        &["outcome=\"ok\"", "elapsed_ms=150", "waited_for_trigger_registry=true", "trigger_registry_wait_ms=150"],
    );
}

/// A request cancelled while parked reports the time it was parked, on its
/// one (cancelled) completion event.
#[test]
fn a_cancelled_request_reports_its_parked_time() {
    let logs = CapturedLogs::global();
    let clock = Arc::new(ManualClock::new());
    let workspace = LspWorkspace::new();
    let (mut fixture, doc) = held_document_session(&workspace, &clock);
    hover(&fixture, 901, &doc);
    fixture.round_trip();
    clock.advance(Duration::from_millis(70));
    fixture.notify("$/cancelRequest", json!({ "id": 901 }));
    let cancelled = fixture.expect_response(RequestId::from(901));
    assert_eq!(cancelled.error.map(|error| error.code), Some(REQUEST_CANCELED));
    // Time after the answer is not part of the wait.
    clock.advance(Duration::from_millis(500));
    fixture.open_registry_gate();
    fixture.shutdown();

    assert_fields(
        &completion(&logs, 901),
        &["outcome=\"cancelled\"", "cancelled=true", "elapsed_ms=70", "trigger_registry_wait_ms=70"],
    );
}

/// A request still parked at shutdown is answered once, and its completion
/// event reports the time it was parked.
#[test]
fn a_request_parked_at_shutdown_reports_its_parked_time() {
    let logs = CapturedLogs::global();
    let clock = Arc::new(ManualClock::new());
    let workspace = LspWorkspace::new();
    let (mut fixture, doc) = held_document_session(&workspace, &clock);
    hover(&fixture, 902, &doc);
    fixture.round_trip();
    clock.advance(Duration::from_millis(30));
    fixture.send_request_with_id(RequestId::from(903), "shutdown", Value::Null);
    let answered = fixture.expect_response(RequestId::from(902));
    assert_eq!(answered.error.map(|error| error.code), Some(SERVER_CANCELLED));
    let shutdown = fixture.expect_response(RequestId::from(903));
    fixture.open_registry_gate();
    fixture.exit_after_shutdown(shutdown);

    assert_fields(&completion(&logs, 902), &["outcome=\"cancelled\"", "trigger_registry_wait_ms=30"]);
}

/// A request that never parks carries no wait figure.
#[test]
fn a_request_that_never_parks_has_no_wait_figure() {
    let logs = CapturedLogs::global();
    let clock = Arc::new(ManualClock::new());
    let workspace = LspWorkspace::new();
    let (mut fixture, _doc) = held_document_session(&workspace, &clock);
    let plain = uri(&workspace.path().join("plain.md"));
    write(&workspace.path().join("plain.md"), "# Plain\n");
    fixture.notify(
        "textDocument/didOpen",
        json!({ "textDocument": { "uri": plain, "languageId": "markdown", "version": 1, "text": "# Plain\n" } }),
    );
    fixture.send_request_with_id(
        RequestId::from(904),
        "textDocument/documentSymbol",
        json!({ "textDocument": { "uri": plain } }),
    );
    let answer = fixture.expect_response(RequestId::from(904));
    assert!(answer.error.is_none(), "{answer:?}");
    fixture.open_registry_gate();
    fixture.shutdown();

    let line = completion(&logs, 904);
    assert!(line.contains("waited_for_trigger_registry=false"), "{line}");
    assert!(!line.contains("trigger_registry_wait_ms"), "{line}");
}
