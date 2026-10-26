//! Level-1 sessions for startup's trigger-registry preparation on the
//! registry worker: file discovery runs beside the initial phase and its
//! list installs at once, the discovered-scopes phase waits for both, no two
//! preparations overlap, a change during an in-flight preparation leaves its
//! scope stale, a request's document job overtakes the remaining startup
//! probes, and shutdown during either phase releases everything.
//!
//! A session started with [`LspFixture::start_with_startup_probe_gate`]
//! holds each startup probe preparation after it has read its inputs and
//! before it installs. The work counters (`registry_scans`,
//! `peak_concurrent_registry_scans`, `registry_scan_threads`) are
//! process-wide, which nextest's process-per-test model keeps private to one
//! test; a test that compares two sessions takes deltas.

use crate::common;

use std::path::{Path, PathBuf};

use common::{LspFixture, LspWorkspace};
use dmls::log_capture::CapturedLogs;
use dmls::work_counters::{peak_concurrent_registry_scans, registry_scan_threads, work_counts};
use dmls::{RegistryJobKind, StartupEvent, StartupPhase};
use lsp_server::RequestId;
use serde_json::{Value, json};

/// `ServerCancelled`.
const SERVER_CANCELLED: i32 = -32802;

const ENVELOPE: &str = "kind: trigger-schema\nmatch:\n  prompt: string(required)\n$schema: prompt.yaml\n";
const BROKEN_ENVELOPE: &str = "kind: trigger-schema\nmatch: {}\n";
const PAYLOAD: &str = "$schema:\n  model: string(required)\n";
const DOC: &str = "---\nprompt: hello\n---\n\n# Heading\n\nBody.\n";

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
        "clientInfo": { "name": "Startup Client", "version": "1.0" },
        "capabilities": {
            "general": { "positionEncodings": ["utf-16"] },
            "workspace": { "didChangeWatchedFiles": { "dynamicRegistration": true } }
        },
        "workspaceFolders": folders
    })
}

/// A workspace folder that is not a repository, holding the repositories
/// `one/` and `two/`, which only file discovery finds.
fn folder_of_repositories() -> (LspWorkspace, PathBuf) {
    let workspace = LspWorkspace::new();
    let folder = workspace.path().to_path_buf();
    trigger_repository(&folder.join("one"));
    trigger_repository(&folder.join("two"));
    (workspace, folder)
}

/// Two repositories, each its own workspace folder, so the initial phase has
/// two probes.
fn two_folder_workspace() -> (LspWorkspace, PathBuf, PathBuf) {
    let workspace = LspWorkspace::new();
    let first = workspace.path().join("first");
    let second = workspace.path().join("second");
    trigger_repository(&first);
    trigger_repository(&second);
    (workspace, first, second)
}

fn open(fixture: &LspFixture<'_>, uri: &str) {
    fixture.notify(
        "textDocument/didOpen",
        json!({ "textDocument": { "uri": uri, "languageId": "markdown", "version": 1, "text": DOC } }),
    );
}

fn watched(fixture: &LspFixture<'_>, path: &Path) {
    fixture.notify(
        "workspace/didChangeWatchedFiles",
        json!({ "changes": [ { "uri": uri(path), "type": 2 } ] }),
    );
}

fn is_probe(phase: StartupPhase) -> impl Fn(&StartupEvent) -> bool {
    move |event| *event == StartupEvent::StartupProbePreparing(phase)
}

fn is_startup_prepared(event: &StartupEvent) -> bool {
    matches!(event, StartupEvent::RegistryPrepared { job: RegistryJobKind::Startup, .. })
}

fn position(events: &[StartupEvent], wanted: StartupEvent) -> usize {
    events.iter().position(|event| *event == wanted).unwrap_or_else(|| panic!("no {wanted:?} in {events:?}"))
}

/// The registry scans a plain startup of `workspace` performs, settled.
fn plain_startup_scans(workspace: &LspWorkspace, folders: &[&Path]) -> usize {
    let before = work_counts();
    let mut fixture = LspFixture::start(workspace);
    fixture.initialize(params(folders));
    fixture.flush_server();
    let scans = work_counts().since(before).registry_scans;
    fixture.shutdown();
    scans
}

// ── Concurrent discovery ────────────────────────────────────────────────────

/// With the initial phase held in flight, discovery finishes and its file
/// list is installed (and published: `FileListInstalled` follows the
/// progress end) before the hold is released. The discovered-scopes phase
/// starts only after both, and the one registry line follows it and counts
/// both repositories' roots.
#[test]
fn the_file_list_installs_while_the_initial_phase_is_held_and_the_discovered_phase_follows_both() {
    let logs = CapturedLogs::global();
    let (workspace, folder) = folder_of_repositories();
    let mut fixture = LspFixture::start_with_file_list_and_startup_probe_gates(&workspace);
    fixture.initialize_unlisted(params(&[&folder]));

    // Discovery runs only once the initial phase has listed its probes, so
    // the repositories it finds are left to the discovered-scopes phase.
    let mut listed = fixture.startup_events_until(is_probe(StartupPhase::Initial));
    fixture.release_file_list();
    listed.extend(fixture.startup_events_until(|event| *event == StartupEvent::FileListInstalled));
    assert!(
        !listed.iter().any(|event| matches!(
            event,
            StartupEvent::StartupPhaseFinished(_) | StartupEvent::StartupProbePreparing(StartupPhase::Discovered)
        )),
        "the list installed while the initial phase is held: {listed:?}"
    );
    assert!(logs.matching("schema registry").is_empty(), "no registry line before startup's last phase");

    fixture.open_startup_probe_gate();
    let rest = fixture.startup_events_until(is_startup_prepared);
    let initial = position(&rest, StartupEvent::StartupPhaseFinished(StartupPhase::Initial));
    let discovered_probe = position(&rest, StartupEvent::StartupProbePreparing(StartupPhase::Discovered));
    let discovered = position(&rest, StartupEvent::StartupPhaseFinished(StartupPhase::Discovered));
    let ready = position(&rest, StartupEvent::RegistryReady);
    assert!(initial < discovered_probe && discovered_probe < discovered && discovered < ready, "{rest:?}");

    let lines = logs.matching("schema registry");
    assert_eq!(lines.len(), 1, "{:#?}", logs.lines());
    for field in ["roots=2", "trigger_schemas=2", "failed_scopes=0"] {
        assert!(lines[0].contains(&format!(" {field}")), "`{field}` missing from: {}", lines[0]);
    }
    fixture.shutdown();
}

// ── One preparation at a time ───────────────────────────────────────────────

/// A refresh scheduled while startup's initial phase is in flight runs on
/// the same worker after it: no two registry scans ever overlap, all run on
/// one thread other than the dispatch thread, the discovered repositories
/// are prepared before startup completes, and a probe one pass prepared is
/// skipped by the other, so the session scans exactly as much as a plain
/// startup does.
#[test]
fn startup_and_a_refresh_share_one_worker_and_skip_fresh_probes() {
    let (control, control_folder) = folder_of_repositories();
    let plain = plain_startup_scans(&control, &[&control_folder]);

    let logs = CapturedLogs::global();
    let (workspace, folder) = folder_of_repositories();
    let before = work_counts();
    let control_threads = registry_scan_threads();
    let mut fixture = LspFixture::start_with_startup_probe_gate(&workspace);
    let dispatch = fixture.server_thread_id();
    fixture.initialize_unlisted(params(&[&folder]));
    fixture.startup_events_until(is_probe(StartupPhase::Initial));

    let trigger = folder.join("two/schemas/prompt.trigger.yaml");
    write(&trigger, &ENVELOPE.replace("prompt: string", "headline: string"));
    watched(&fixture, &trigger);
    fixture.round_trip();

    fixture.open_startup_probe_gate();
    let mut jobs = fixture.registry_jobs_until_idle();
    jobs.sort_by_key(|job| format!("{job:?}"));
    assert_eq!(jobs, vec![RegistryJobKind::Startup, RegistryJobKind::Workspace], "{jobs:?}");
    assert_eq!(peak_concurrent_registry_scans(), 1, "two registry scans overlapped");
    let threads: Vec<_> =
        registry_scan_threads().into_iter().filter(|thread| !control_threads.contains(thread)).collect();
    assert_eq!(threads.len(), 1, "every scan on the one registry worker: {threads:?}");
    assert_ne!(threads[0], dispatch);
    assert_eq!(work_counts().since(before).registry_scans, plain, "a fresh probe is prepared again");

    let lines = logs.matching("schema registry");
    assert_eq!(lines.len(), 1, "{:#?}", logs.lines());
    assert!(lines[0].contains(" roots=2"), "the discovered repositories are counted: {}", lines[0]);
    fixture.shutdown();
}

/// A trigger change recorded while startup's preparation of its scope is in
/// flight (its inputs already read) leaves that scope stale, so the refresh
/// the change scheduled prepares it again and publishes the broken
/// trigger's diagnostic. Were the outdated reading installed as fresh, the
/// refresh would skip the scope and nothing would be published.
#[test]
fn a_change_during_an_in_flight_startup_preparation_leaves_the_scope_stale() {
    let workspace = LspWorkspace::new();
    let root = workspace.path().to_path_buf();
    trigger_repository(&root);
    let trigger = root.join("schemas/prompt.trigger.yaml");
    let mut fixture = LspFixture::start_with_startup_probe_gate(&workspace);
    fixture.initialize_unlisted(params(&[&root]));
    fixture.startup_events_until(is_probe(StartupPhase::Initial));

    write(&trigger, BROKEN_ENVELOPE);
    watched(&fixture, &trigger);
    fixture.round_trip();
    assert!(fixture.take_buffered_diagnostics(&uri(&trigger)).is_none(), "nothing prepared the change yet");

    fixture.open_startup_probe_gate();
    // Waits for startup too: it counts as an outstanding registry job.
    fixture.flush_server_with("textDocument/documentSymbol", json!({ "textDocument": { "uri": uri(&root.join("a.md")) } }));
    let published = fixture.take_buffered_diagnostics(&uri(&trigger)).expect("the stale scope was prepared again");
    assert!(
        !published["diagnostics"].as_array().unwrap().is_empty(),
        "the broken trigger is reported: {published:#?}"
    );
    fixture.shutdown();
}

// ── Request ordering ────────────────────────────────────────────────────────

/// The reported incident: startup's preparation is running when the client
/// sends an unchanged configuration, opens a document, and asks for semantic
/// tokens. The document's job runs as soon as the probe in flight installs,
/// before the next startup probe: the response arrives while that next probe
/// is still held. Afterwards the unchanged notification has queued no
/// workspace pass and cost no preparation beyond a plain startup's plus the
/// document's own.
#[test]
fn a_request_overtakes_the_remaining_startup_probes_after_an_unchanged_configuration() {
    let (control, control_first, control_second) = two_folder_workspace();
    let plain = plain_startup_scans(&control, &[&control_first, &control_second]);

    let logs = CapturedLogs::global();
    let (workspace, first, second) = two_folder_workspace();
    let doc = uri(&first.join("a.md"));
    let before = work_counts();
    let mut fixture = LspFixture::start_with_startup_probe_gate(&workspace);
    fixture.initialize_unlisted(params(&[&first, &second]));
    fixture.startup_events_until(is_probe(StartupPhase::Initial));

    fixture.notify("workspace/didChangeConfiguration", json!({ "settings": {} }));
    open(&fixture, &doc);
    fixture.send_request_with_id(
        RequestId::from(900),
        "textDocument/semanticTokens/full",
        json!({ "textDocument": { "uri": doc } }),
    );
    // Handled while the first probe is held, so the document job is queued
    // before that probe installs. (A document request would itself wait on
    // that job.)
    fixture.round_trip();

    fixture.release_startup_probe();
    let tokens = fixture.expect_response(RequestId::from(900));
    assert!(tokens.error.is_none(), "{tokens:?}");
    let between = fixture.startup_events_until(is_probe(StartupPhase::Initial));
    assert!(
        !between.iter().any(|event| matches!(event, StartupEvent::StartupPhaseFinished(_))),
        "the response came while the second startup probe is held: {between:?}"
    );

    fixture.open_startup_probe_gate();
    let jobs = fixture.registry_jobs_until_idle();
    assert!(!jobs.contains(&RegistryJobKind::Workspace), "the unchanged configuration queued a pass: {jobs:?}");
    assert_eq!(jobs.iter().filter(|job| **job == RegistryJobKind::Startup).count(), 1, "{jobs:?}");
    assert_eq!(
        work_counts().since(before).registry_scans,
        plain + 1,
        "a plain startup's scans plus the document's own"
    );
    let completion: Vec<String> =
        logs.matching("request complete").into_iter().filter(|line| line.contains(" id=900 ")).collect();
    assert_eq!(completion.len(), 1, "{:#?}", logs.lines());
    fixture.shutdown();
}

// ── Lifecycle ───────────────────────────────────────────────────────────────

/// Shutdown while the initial phase is held answers the request waiting on
/// a document job once, reports nothing outstanding without waiting for
/// startup, and exits cleanly; startup never reports ready.
#[test]
fn shutdown_during_the_initial_phase_releases_waiters_and_startup() {
    let (workspace, first, second) = two_folder_workspace();
    let doc = uri(&first.join("a.md"));
    let mut fixture = LspFixture::start_with_startup_probe_gate(&workspace);
    fixture.initialize_unlisted(params(&[&first, &second]));
    fixture.startup_events_until(is_probe(StartupPhase::Initial));
    open(&fixture, &doc);
    fixture.send_request_with_id(
        RequestId::from(601),
        "textDocument/hover",
        json!({ "textDocument": { "uri": doc }, "position": { "line": 4, "character": 3 } }),
    );

    fixture.send_request_with_id(RequestId::from(602), "shutdown", Value::Null);
    let waiter = fixture.expect_response(RequestId::from(601));
    assert_eq!(waiter.error.map(|error| error.code), Some(SERVER_CANCELLED));
    let shutdown = fixture.expect_response(RequestId::from(602));
    let events = fixture.startup_events_until(|event| *event == StartupEvent::RegistryJobsOutstanding(0));
    assert!(!events.contains(&StartupEvent::RegistryReady), "{events:?}");
    fixture.open_startup_probe_gate();
    fixture.exit_after_shutdown(shutdown);
}

/// Shutdown while the discovered-scopes phase is held reports nothing
/// outstanding and exits cleanly; startup never reports ready.
#[test]
fn shutdown_during_the_discovered_phase_releases_startup() {
    let (workspace, folder) = folder_of_repositories();
    let mut fixture = LspFixture::start_with_file_list_and_startup_probe_gates(&workspace);
    fixture.initialize_unlisted(params(&[&folder]));
    // Discovery runs only once the initial phase has listed its probes, so
    // the repositories it finds are left to the discovered-scopes phase.
    fixture.startup_events_until(is_probe(StartupPhase::Initial));
    fixture.release_file_list();
    loop {
        fixture.release_startup_probe();
        let events = fixture.startup_events_until(|event| matches!(event, StartupEvent::StartupProbePreparing(_)));
        if events.last() == Some(&StartupEvent::StartupProbePreparing(StartupPhase::Discovered)) {
            break;
        }
    }

    fixture.send_request_with_id(RequestId::from(602), "shutdown", Value::Null);
    let shutdown = fixture.expect_response(RequestId::from(602));
    let events = fixture.startup_events_until(|event| *event == StartupEvent::RegistryJobsOutstanding(0));
    assert!(!events.contains(&StartupEvent::RegistryReady), "{events:?}");
    fixture.open_startup_probe_gate();
    fixture.exit_after_shutdown(shutdown);
}

/// A manifest change while a request's document job is held gives the
/// document a different context (it now lies in a package). The finished
/// job's registry was prepared in the old context, so the request is parked
/// on a new job rather than answered from it or stranded, and is answered
/// once that job finishes.
#[test]
fn a_context_change_while_a_document_job_is_queued_requeues_its_waiter() {
    let workspace = LspWorkspace::new();
    let root = workspace.path().to_path_buf();
    trigger_repository(&root);
    write(&root.join("pkg/doc.md"), DOC);
    let doc = uri(&root.join("pkg/doc.md"));
    let mut fixture = LspFixture::start_with_registry_gate(&workspace);
    fixture.initialize(params(&[&root]));
    open(&fixture, &doc);
    fixture.wait_for_event(StartupEvent::RegistryPreparing);
    fixture.send_request_with_id(
        RequestId::from(700),
        "textDocument/hover",
        json!({ "textDocument": { "uri": doc }, "position": { "line": 4, "character": 3 } }),
    );

    write(&root.join("Cargo.toml"), "[workspace]\nmembers = [\"pkg\"]\n");
    write(&root.join("pkg/Cargo.toml"), "[package]\nname = \"pkg\"\nversion = \"0.1.0\"\nedition = \"2024\"\n");
    write(&root.join("pkg/src/lib.rs"), "");
    watched(&fixture, &root.join("Cargo.toml"));
    watched(&fixture, &root.join("pkg/Cargo.toml"));
    fixture.round_trip();

    fixture.open_registry_gate();
    let answer = fixture.expect_response(RequestId::from(700));
    assert!(answer.error.is_none(), "{answer:?}");
    let jobs = fixture.registry_jobs_until_idle();
    let documents = jobs.iter().filter(|job| **job == RegistryJobKind::Document).count();
    assert!(documents >= 2, "the waiter was parked on a second document job: {jobs:?}");
    fixture.shutdown();
}
