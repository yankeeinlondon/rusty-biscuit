//! Level-1 sessions for trigger-registry preparation off the dispatch
//! thread: a request waits only for its own document's registry, the server
//! keeps handling edits, cancellation, other requests, and shutdown while a
//! preparation is held, and every preparation (startup, a first document,
//! and each refresh) runs on a thread other than the dispatch thread.
//!
//! A session started with [`LspFixture::start_with_registry_gate`] holds
//! each registry job after startup until the test releases it. Thread
//! placement comes from `dmls::work_counters::registry_scan_threads`, which
//! names every thread a registry scan ran on; the counters are process-wide,
//! and nextest runs each test in its own process.

use crate::common;

use std::path::{Path, PathBuf};

use common::{LspFixture, LspWorkspace};
use dmls::log_capture::CapturedLogs;
use dmls::work_counters::{registry_scan_threads, work_counts};
use dmls::{RegistryJobKind, StartupEvent};
use lsp_server::RequestId;
use serde_json::{Value, json};

/// `RequestCanceled`.
const REQUEST_CANCELED: i32 = -32800;
/// `ServerCancelled`.
const SERVER_CANCELLED: i32 = -32802;

const ENVELOPE: &str = "kind: schema-trigger\nmatch:\n  prompt: string(required)\n$schema: prompt.yaml\n";
const PAYLOAD: &str = "$schema:\n  model: string(required)\n";
const DOC: &str = "---\nprompt: hello\n---\n\n# Heading\n\nBody.\n";
const PLAIN: &str = "# Plain\n\nNo frontmatter.\n";

/// Editor settings that change the effective configuration, and nothing a
/// trigger registry reads.
#[allow(non_snake_case)]
fn CHANGED_SETTINGS() -> Value {
    json!({ "dmls": { "formatting": { "markdown": { "unwrap": true } } } })
}

fn write(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

/// A repository with a trigger, its payload, a frontmatter document the
/// trigger matches, and a plain document. Strict schema mode reports the
/// payload's missing required property.
fn trigger_workspace() -> LspWorkspace {
    let workspace = LspWorkspace::new();
    let root = workspace.path();
    std::fs::create_dir_all(root.join(".git/objects")).unwrap();
    std::fs::create_dir_all(root.join(".git/refs/heads")).unwrap();
    write(&root.join(".git/HEAD"), "ref: refs/heads/main\n");
    write(&root.join(".git/config"), "[core]\n\trepositoryformatversion = 0\n\tbare = false\n");
    write(&root.join(".dmls.toml"), "[schema]\nstrict = true\n");
    write(&envelope(&workspace), ENVELOPE);
    write(&root.join("schemas/prompt.yaml"), PAYLOAD);
    write(&root.join("doc.md"), DOC);
    write(&root.join("plain.md"), PLAIN);
    workspace
}

fn envelope(workspace: &LspWorkspace) -> PathBuf {
    workspace.path().join("schemas/prompt.trigger.yaml")
}

fn uri(path: &Path) -> String {
    url::Url::from_file_path(path).unwrap().to_string()
}

fn doc_uri(workspace: &LspWorkspace) -> String {
    uri(&workspace.path().join("doc.md"))
}

fn plain_uri(workspace: &LspWorkspace) -> String {
    uri(&workspace.path().join("plain.md"))
}

/// A client that watches files.
fn watched_params(root: &Path) -> Value {
    json!({
        "processId": null,
        "clientInfo": { "name": "Registry Client", "version": "1.0" },
        "capabilities": {
            "general": { "positionEncodings": ["utf-16"] },
            "workspace": { "didChangeWatchedFiles": { "dynamicRegistration": true } }
        },
        "workspaceFolders": [ { "uri": url::Url::from_directory_path(root).unwrap().as_str(), "name": "r" } ]
    })
}

/// A client with no file watcher, so DMLS rescans on save.
fn rescan_params(root: &Path) -> Value {
    let mut params = watched_params(root);
    params["clientInfo"] = json!({ "name": "Neovim", "version": "0.11.0" });
    params["capabilities"]["workspace"].as_object_mut().unwrap().remove("didChangeWatchedFiles");
    params
}

fn open(fixture: &LspFixture<'_>, uri: &str, language: &str, text: &str) {
    fixture.notify(
        "textDocument/didOpen",
        json!({ "textDocument": { "uri": uri, "languageId": language, "version": 1, "text": text } }),
    );
}

fn symbol_names(response: &lsp_server::Response) -> Vec<String> {
    assert!(response.error.is_none(), "{:?}", response.error);
    fn collect(symbols: &[Value], names: &mut Vec<String>) {
        for symbol in symbols {
            names.push(symbol["name"].as_str().unwrap_or_default().to_string());
            collect(symbol["children"].as_array().map(Vec::as_slice).unwrap_or_default(), names);
        }
    }
    let mut names = Vec::new();
    collect(response.result.as_ref().unwrap().as_array().unwrap(), &mut names);
    names
}

/// Asserts that no registry scan in this process ran on the session's
/// dispatch thread.
fn assert_no_scan_on_dispatch(fixture: &LspFixture<'_>) {
    let dispatch = fixture.server_thread_id();
    let threads = registry_scan_threads();
    assert!(!threads.is_empty(), "no registry scan ran");
    assert!(!threads.contains(&dispatch), "a registry scan ran on the dispatch thread {dispatch:?}: {threads:?}");
}

// ── A held preparation ──────────────────────────────────────────────────────

/// The document's preparation is held: an edit to it, a request for another
/// document, and the cancellation of one waiting request are all handled,
/// and the other waiting request is answered for the edited text once the
/// preparation is released. Its completion event records the wait.
#[test]
fn a_held_preparation_leaves_edits_cancellation_and_other_requests_running() {
    let logs = CapturedLogs::global();
    let workspace = trigger_workspace();
    let doc = doc_uri(&workspace);
    let plain = plain_uri(&workspace);
    let mut fixture = LspFixture::start_with_registry_gate(&workspace);
    fixture.initialize(watched_params(workspace.path()));
    open(&fixture, &doc, "markdown", DOC);
    fixture.wait_for_event(StartupEvent::RegistryPreparing);

    fixture.send_request_with_id(
        RequestId::from(501),
        "textDocument/hover",
        json!({ "textDocument": { "uri": doc }, "position": { "line": 4, "character": 3 } }),
    );
    fixture.send_request_with_id(
        RequestId::from(502),
        "textDocument/documentSymbol",
        json!({ "textDocument": { "uri": doc } }),
    );
    fixture.notify(
        "textDocument/didChange",
        json!({
            "textDocument": { "uri": doc, "version": 2 },
            "contentChanges": [ { "text": DOC.replace("# Heading", "# Edited") } ]
        }),
    );
    // Answered while both waiters wait: an earlier answer to either would
    // fail this `request` as an unexpected message.
    open(&fixture, &plain, "markdown", PLAIN);
    let other = fixture.request("textDocument/documentSymbol", json!({ "textDocument": { "uri": plain } }));
    assert!(symbol_names(&other).contains(&"Plain".to_string()), "{other:?}");

    fixture.notify("$/cancelRequest", json!({ "id": 501 }));
    let cancelled = fixture.expect_response(RequestId::from(501));
    assert_eq!(cancelled.error.map(|error| error.code), Some(REQUEST_CANCELED));

    fixture.release_registry_job();
    let resumed = fixture.expect_response(RequestId::from(502));
    let names = symbol_names(&resumed);
    assert!(names.contains(&"Edited".to_string()) && !names.contains(&"Heading".to_string()), "{names:?}");
    assert_no_scan_on_dispatch(&fixture);
    fixture.open_registry_gate();
    fixture.shutdown();

    // 502's elapsed time includes the held preparation, so on a loaded host it
    // crosses `SLOW_THRESHOLD` and completes as a `slow request` instead.
    let completion = |id: &str| -> String {
        let lines: Vec<String> = logs
            .lines()
            .into_iter()
            .filter(|line| line.contains("request complete") || line.contains("slow request"))
            .filter(|line| line.contains(&format!(" id={id} ")))
            .collect();
        assert_eq!(lines.len(), 1, "{:#?}", logs.lines());
        lines[0].clone()
    };
    assert!(completion("502").contains("waited_for_trigger_registry=true"), "{}", completion("502"));
    assert!(completion("501").contains("outcome=\"cancelled\""), "{}", completion("501"));
}

#[test]
fn shutdown_while_a_preparation_is_held_answers_its_waiter() {
    let workspace = trigger_workspace();
    let doc = doc_uri(&workspace);
    let mut fixture = LspFixture::start_with_registry_gate(&workspace);
    fixture.initialize(watched_params(workspace.path()));
    open(&fixture, &doc, "markdown", DOC);
    fixture.wait_for_event(StartupEvent::RegistryPreparing);
    fixture.send_request_with_id(
        RequestId::from(601),
        "textDocument/hover",
        json!({ "textDocument": { "uri": doc }, "position": { "line": 4, "character": 3 } }),
    );

    fixture.send_request_with_id(RequestId::from(602), "shutdown", Value::Null);
    let waiter = fixture.expect_response(RequestId::from(601));
    assert_eq!(waiter.error.map(|error| error.code), Some(SERVER_CANCELLED));
    let shutdown = fixture.expect_response(RequestId::from(602));
    fixture.open_registry_gate();
    fixture.exit_after_shutdown(shutdown);
}

/// The diagnostics refresh a held preparation blocks is published when it
/// is released, and reflects the trigger.
#[test]
fn a_held_diagnostics_refresh_is_published_when_its_registry_installs() {
    let workspace = trigger_workspace();
    let doc = doc_uri(&workspace);
    let mut fixture = LspFixture::start_with_registry_gate(&workspace);
    fixture.initialize(watched_params(workspace.path()));
    open(&fixture, &doc, "markdown", DOC);
    fixture.wait_for_event(StartupEvent::RegistryPreparing);
    fixture.round_trip();
    assert!(fixture.take_buffered_diagnostics(&doc).is_none(), "nothing published before the registry installs");

    fixture.release_registry_job();
    let diagnostics = fixture.wait_for_diagnostics(&doc);
    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic["code"] == json!("dm.schema.missing_required")
                && diagnostic["message"].as_str().is_some_and(|message| message.contains("model"))
        }),
        "the trigger's payload requires `model`: {diagnostics:#?}"
    );
    fixture.open_registry_gate();
    fixture.shutdown();
}

/// A response is no barrier for a held refresh, but a flush through the
/// same request is: it returns only after the job's publication is buffered.
#[test]
fn a_flush_through_any_request_buffers_a_held_refresh() {
    let workspace = trigger_workspace();
    let doc = doc_uri(&workspace);
    let plain = plain_uri(&workspace);
    let mut fixture = LspFixture::start_with_registry_gate(&workspace);
    fixture.initialize(watched_params(workspace.path()));
    open(&fixture, &plain, "markdown", PLAIN);
    open(&fixture, &doc, "markdown", DOC);
    fixture.wait_for_event(StartupEvent::RegistryPreparing);
    let symbols = fixture.request("textDocument/documentSymbol", document(&plain));
    assert!(symbols.error.is_none(), "{symbols:?}");
    assert!(fixture.take_buffered_diagnostics(&doc).is_none(), "answered while the refresh is held");

    fixture.open_registry_gate();
    fixture.flush_server_with("textDocument/documentSymbol", document(&plain));
    let refreshed = fixture.take_buffered_diagnostics(&doc).expect("the held refresh was published");
    assert!(
        refreshed["diagnostics"].as_array().unwrap().iter().any(|diagnostic| {
            diagnostic["code"] == json!("dm.schema.missing_required")
        }),
        "{refreshed:#?}"
    );
    fixture.shutdown();
}

// ── Schema-dependent requests wait instead of preparing ────────────────────

/// Each schema-dependent request for a document whose registry is held
/// waits for it (a later request for a plain document is answered first),
/// and is answered once the preparation is released.
macro_rules! waits_for_its_registry {
    ($($name:ident: $method:literal, $document:ident, $params:expr;)+) => {$(
        #[test]
        fn $name() {
            let workspace = trigger_workspace();
            let target = $document(&workspace);
            let plain = plain_uri(&workspace);
            let mut fixture = LspFixture::start_with_registry_gate(&workspace);
            fixture.initialize(watched_params(workspace.path()));
            // Fills the index store first, so references wait only for the
            // registry.
            fixture.round_trip();
            open(&fixture, &plain, "markdown", PLAIN);
            open(&fixture, &target.0, target.1, target.2);
            fixture.wait_for_event(StartupEvent::RegistryPreparing);

            let params: fn(&str) -> Value = $params;
            fixture.send_request_with_id(RequestId::from(900), $method, params(&target.0));
            let control = fixture.request(
                "textDocument/prepareRename",
                json!({ "textDocument": { "uri": plain }, "position": { "line": 0, "character": 3 } }),
            );
            assert!(control.error.is_none(), "{control:?}");

            fixture.release_registry_job();
            let answer = fixture.expect_response(RequestId::from(900));
            assert!(answer.error.is_none(), "{answer:?}");
            assert_no_scan_on_dispatch(&fixture);
            fixture.open_registry_gate();
            fixture.shutdown();
        }
    )+};
}

/// The frontmatter document: (uri, language, text).
fn markdown(workspace: &LspWorkspace) -> (String, &'static str, &'static str) {
    (doc_uri(workspace), "markdown", DOC)
}

/// The open trigger envelope, whose overlay reads its trigger-load error.
fn envelope_yaml(workspace: &LspWorkspace) -> (String, &'static str, &'static str) {
    (uri(&envelope(workspace)), "yaml", ENVELOPE)
}

fn document(uri: &str) -> Value {
    json!({ "textDocument": { "uri": uri } })
}

fn at(uri: &str) -> Value {
    json!({ "textDocument": { "uri": uri }, "position": { "line": 1, "character": 3 } })
}

fn range(uri: &str) -> Value {
    json!({
        "textDocument": { "uri": uri },
        "range": { "start": { "line": 0, "character": 0 }, "end": { "line": 3, "character": 0 } }
    })
}

waits_for_its_registry! {
    document_symbols_wait_for_the_registry: "textDocument/documentSymbol", markdown, document;
    definition_waits_for_the_registry: "textDocument/definition", markdown, at;
    document_links_wait_for_the_registry: "textDocument/documentLink", markdown, document;
    references_wait_for_the_registry: "textDocument/references", markdown, |uri| {
        let mut params = at(uri);
        params["context"] = json!({ "includeDeclaration": false });
        params
    };
    document_highlights_wait_for_the_registry: "textDocument/documentHighlight", markdown, at;
    folding_ranges_wait_for_the_registry: "textDocument/foldingRange", markdown, document;
    hover_waits_for_the_registry: "textDocument/hover", markdown, at;
    full_semantic_tokens_wait_for_the_registry: "textDocument/semanticTokens/full", markdown, document;
    ranged_semantic_tokens_wait_for_the_registry: "textDocument/semanticTokens/range", markdown, range;
    completion_waits_for_the_registry: "textDocument/completion", markdown, at;
    code_actions_wait_for_the_registry: "textDocument/codeAction", markdown, |uri| {
        let mut params = range(uri);
        params["context"] = json!({ "diagnostics": [] });
        params
    };
    formatting_waits_for_the_registry: "textDocument/formatting", markdown, |uri| {
        let mut params = document(uri);
        params["options"] = json!({ "tabSize": 2, "insertSpaces": true });
        params
    };
    the_trigger_authoring_overlay_waits_for_the_registry: "textDocument/documentSymbol", envelope_yaml, document;
}

// ── Every preparation runs off the dispatch thread ─────────────────────────

/// Runs `refresh` after the session has settled with the frontmatter
/// document open, waits for a preparation of each kind in `jobs`, and
/// asserts that preparation ran (counted scans) and that neither those
/// preparations nor any other ran on the dispatch thread.
fn assert_refresh_prepares_off_dispatch(
    params: fn(&Path) -> Value,
    jobs: &[RegistryJobKind],
    refresh: impl FnOnce(&LspWorkspace, &mut LspFixture<'_>),
) {
    let workspace = trigger_workspace();
    let doc = doc_uri(&workspace);
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(params(workspace.path()));
    open(&fixture, &doc, "markdown", DOC);
    fixture.flush_server();

    let before = work_counts();
    refresh(&workspace, &mut fixture);
    let dispatch = fixture.server_thread_id();
    let mut remaining = jobs.to_vec();
    while !remaining.is_empty() {
        let (job, thread) = fixture.next_registry_prepared();
        assert_ne!(thread, dispatch, "{job:?} prepared on the dispatch thread");
        if let Some(position) = remaining.iter().position(|kind| *kind == job) {
            remaining.remove(position);
        }
    }
    fixture.flush_server();
    assert!(work_counts().since(before).registry_scans > 0, "the refresh prepared nothing");
    assert_no_scan_on_dispatch(&fixture);
    fixture.shutdown();
}

fn watched(fixture: &LspFixture<'_>, path: &Path, kind: u32) {
    fixture.notify(
        "workspace/didChangeWatchedFiles",
        json!({ "changes": [ { "uri": uri(path), "type": kind } ] }),
    );
}

#[test]
fn startup_preparation_runs_off_the_dispatch_thread() {
    let workspace = trigger_workspace();
    let mut fixture = LspFixture::start(&workspace);
    let dispatch = fixture.server_thread_id();
    fixture.initialize_unlisted(watched_params(workspace.path()));
    assert_ne!(fixture.wait_for_registry_prepared(RegistryJobKind::Startup), dispatch);
    assert_no_scan_on_dispatch(&fixture);
    fixture.shutdown();
}

#[test]
fn a_first_document_registry_is_prepared_off_the_dispatch_thread() {
    let workspace = trigger_workspace();
    let doc = doc_uri(&workspace);
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(watched_params(workspace.path()));
    let before = work_counts();
    open(&fixture, &doc, "markdown", DOC);
    assert_ne!(fixture.wait_for_registry_prepared(RegistryJobKind::Document), fixture.server_thread_id());
    assert_eq!(work_counts().since(before).registry_scans, 1, "one preparation for the document");
    assert_no_scan_on_dispatch(&fixture);
    fixture.shutdown();
}

#[test]
fn a_watched_schema_reload_prepares_off_the_dispatch_thread() {
    assert_refresh_prepares_off_dispatch(
        watched_params,
        &[RegistryJobKind::Workspace, RegistryJobKind::Document],
        |workspace, fixture| {
            write(&envelope(workspace), &ENVELOPE.replace("prompt: string", "headline: string"));
            watched(fixture, &envelope(workspace), 2);
        },
    );
}

#[test]
fn a_saved_yaml_reload_prepares_off_the_dispatch_thread() {
    assert_refresh_prepares_off_dispatch(
        watched_params,
        &[RegistryJobKind::Workspace, RegistryJobKind::Document],
        |workspace, fixture| {
            write(&envelope(workspace), &ENVELOPE.replace("prompt: string", "headline: string"));
            fixture.notify("textDocument/didSave", json!({ "textDocument": { "uri": uri(&envelope(workspace)) } }));
        },
    );
}

#[test]
fn a_configuration_reload_prepares_off_the_dispatch_thread() {
    assert_refresh_prepares_off_dispatch(
        watched_params,
        &[RegistryJobKind::Workspace, RegistryJobKind::Document],
        |_, fixture| fixture.notify("workspace/didChangeConfiguration", json!({ "settings": CHANGED_SETTINGS() })),
    );
}

#[test]
fn a_save_fallback_rescan_prepares_off_the_dispatch_thread() {
    assert_refresh_prepares_off_dispatch(
        rescan_params,
        &[RegistryJobKind::Workspace, RegistryJobKind::Document],
        |workspace, fixture| {
            fixture.notify("textDocument/didSave", json!({ "textDocument": { "uri": doc_uri(workspace) } }));
        },
    );
}

#[test]
fn the_post_refresh_workspace_preparation_runs_off_the_dispatch_thread() {
    assert_refresh_prepares_off_dispatch(watched_params, &[RegistryJobKind::Workspace], |workspace, fixture| {
        let manifest = workspace.path().join("Cargo.toml");
        write(&manifest, "[workspace]\nmembers = []\n");
        watched(fixture, &manifest, 1);
    });
}

// ── Control: a ready registry is only looked up ─────────────────────────────

#[test]
fn requests_on_a_ready_document_do_no_registry_work() {
    let workspace = trigger_workspace();
    let doc = doc_uri(&workspace);
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(watched_params(workspace.path()));
    open(&fixture, &doc, "markdown", DOC);
    fixture.flush_server();

    let before = work_counts();
    for _ in 0..3 {
        let hover = fixture.request("textDocument/hover", at(&doc));
        assert!(hover.error.is_none(), "{hover:?}");
    }
    let work = work_counts().since(before);
    assert_eq!((work.registry_scans, work.enumerations), (0, 0), "{work:?}");
    fixture.shutdown();
}

// ── Configuration reloads ───────────────────────────────────────────────────

/// A `didChangeConfiguration` that leaves the effective configuration as it
/// was (a client sending `{"settings": {}}` after `initialized`) rebuilds the
/// dropped contexts identically, so every registry stays fresh: no job is
/// queued and nothing is scanned, then or on the next request.
#[test]
fn an_unchanged_configuration_notification_prepares_nothing() {
    let workspace = trigger_workspace();
    let doc = doc_uri(&workspace);
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(watched_params(workspace.path()));
    open(&fixture, &doc, "markdown", DOC);
    fixture.flush_server();

    let before = work_counts();
    fixture.notify("workspace/didChangeConfiguration", json!({ "settings": {} }));
    fixture.round_trip();
    assert_eq!(fixture.registry_jobs_until_idle(), Vec::<RegistryJobKind>::new(), "no job was queued");
    let hover = fixture.request("textDocument/hover", at(&doc));
    assert!(hover.error.is_none(), "{hover:?}");
    let work = work_counts().since(before);
    assert_eq!((work.registry_scans, work.enumerations), (0, 0), "{work:?}");
    fixture.shutdown();
}

/// Control for the unchanged case: an effective change marks every registry
/// stale and queues exactly one workspace pass, and the open document's
/// registry is prepared again.
#[test]
fn an_effective_configuration_change_prepares_one_workspace_pass_and_the_open_document() {
    let workspace = trigger_workspace();
    let doc = doc_uri(&workspace);
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(watched_params(workspace.path()));
    open(&fixture, &doc, "markdown", DOC);
    fixture.flush_server();

    let before = work_counts();
    fixture.notify("workspace/didChangeConfiguration", json!({ "settings": CHANGED_SETTINGS() }));
    fixture.round_trip();
    let mut jobs = fixture.registry_jobs_until_idle();
    jobs.sort_by_key(|job| format!("{job:?}"));
    assert_eq!(jobs, vec![RegistryJobKind::Document, RegistryJobKind::Workspace], "{jobs:?}");
    assert!(work_counts().since(before).registry_scans > 0);
    fixture.shutdown();
}

// ── Job order ───────────────────────────────────────────────────────────────

/// Two repositories, each its own workspace folder, with a trigger, its
/// payload, and a frontmatter document in each.
fn two_repository_workspace() -> (LspWorkspace, PathBuf, PathBuf) {
    let workspace = LspWorkspace::new();
    let first = workspace.path().join("first");
    let second = workspace.path().join("second");
    for repo in [&first, &second] {
        std::fs::create_dir_all(repo.join(".git/objects")).unwrap();
        std::fs::create_dir_all(repo.join(".git/refs/heads")).unwrap();
        write(&repo.join(".git/HEAD"), "ref: refs/heads/main\n");
        write(&repo.join(".git/config"), "[core]\n\trepositoryformatversion = 0\n\tbare = false\n");
        write(&repo.join("schemas/prompt.trigger.yaml"), ENVELOPE);
        write(&repo.join("schemas/prompt.yaml"), PAYLOAD);
        write(&repo.join("a.md"), DOC);
        write(&repo.join("b.md"), DOC);
    }
    (workspace, first, second)
}

fn two_folder_params(first: &Path, second: &Path) -> Value {
    let mut params = watched_params(first);
    params["workspaceFolders"] = json!([
        { "uri": url::Url::from_directory_path(first).unwrap().as_str(), "name": "first" },
        { "uri": url::Url::from_directory_path(second).unwrap().as_str(), "name": "second" },
    ]);
    params
}

/// Edits the second repository's trigger and reports it, which marks only
/// the second repository's scopes stale and schedules a workspace pass.
fn touch_second_trigger(fixture: &LspFixture<'_>, second: &Path, field: &str) {
    let trigger = second.join("schemas/prompt.trigger.yaml");
    write(&trigger, &ENVELOPE.replace("prompt: string", &format!("{field}: string")));
    watched(fixture, &trigger, 2);
}

/// With a document job held, two workspace schedules and a second document
/// are queued: the second document runs before the workspace pass, and the
/// two schedules make one pass.
#[test]
fn a_queued_document_runs_before_a_merged_workspace_pass() {
    let (workspace, first, second) = two_repository_workspace();
    let mut fixture = LspFixture::start_with_registry_gate(&workspace);
    fixture.initialize(two_folder_params(&first, &second));
    open(&fixture, &uri(&first.join("a.md")), "markdown", DOC);
    fixture.wait_for_event(StartupEvent::RegistryPreparing);

    touch_second_trigger(&fixture, &second, "headline");
    touch_second_trigger(&fixture, &second, "summary");
    open(&fixture, &uri(&first.join("b.md")), "markdown", DOC);
    fixture.send_request_with_id(RequestId::from(700), "textDocument/hover", at(&uri(&first.join("b.md"))));
    fixture.round_trip();

    fixture.open_registry_gate();
    let answer = fixture.expect_response(RequestId::from(700));
    assert!(answer.error.is_none(), "{answer:?}");
    let jobs = fixture.registry_jobs_until_idle();
    assert_eq!(
        jobs,
        vec![RegistryJobKind::Document, RegistryJobKind::Document, RegistryJobKind::Workspace],
        "both documents first, then one pass for both schedules"
    );
    fixture.shutdown();
}

/// A document job queued while a workspace pass holds its first probe runs
/// once that probe finishes and before the next one: its waiting request is
/// answered while the pass is still held at the second probe.
#[test]
fn a_document_job_runs_between_two_probes_of_a_running_pass() {
    let (workspace, first, second) = two_repository_workspace();
    let doc = uri(&first.join("a.md"));
    let mut fixture = LspFixture::start_with_registry_probe_gate(&workspace);
    fixture.initialize(two_folder_params(&first, &second));
    fixture.flush_server();

    touch_second_trigger(&fixture, &second, "headline");
    fixture.wait_for_event(StartupEvent::WorkspaceProbePreparing);
    open(&fixture, &doc, "markdown", DOC);
    fixture.send_request_with_id(RequestId::from(800), "textDocument/hover", at(&doc));
    // Handled while the first probe is held, so the document job is queued
    // before that probe runs.
    fixture.round_trip();

    fixture.release_registry_probe();
    let answer = fixture.expect_response(RequestId::from(800));
    assert!(answer.error.is_none(), "{answer:?}");
    fixture.wait_for_event(StartupEvent::WorkspaceProbePreparing);
    let (job, _) = fixture.next_registry_prepared();
    assert_eq!(job, RegistryJobKind::Document, "the document finished while the pass is held");

    fixture.open_registry_probe_gate();
    assert_eq!(fixture.registry_jobs_until_idle(), vec![RegistryJobKind::Workspace]);
    fixture.shutdown();
}

/// Workspace schedules made while a pass runs (held at a probe) add exactly
/// one follow-up pass.
#[test]
fn schedules_during_a_running_pass_add_one_follow_up() {
    let (workspace, first, second) = two_repository_workspace();
    let mut fixture = LspFixture::start_with_registry_probe_gate(&workspace);
    fixture.initialize(two_folder_params(&first, &second));
    fixture.flush_server();

    touch_second_trigger(&fixture, &second, "headline");
    fixture.wait_for_event(StartupEvent::WorkspaceProbePreparing);
    touch_second_trigger(&fixture, &second, "summary");
    touch_second_trigger(&fixture, &second, "title");
    fixture.round_trip();

    fixture.open_registry_probe_gate();
    let jobs = fixture.registry_jobs_until_idle();
    assert_eq!(jobs, vec![RegistryJobKind::Workspace, RegistryJobKind::Workspace], "the pass and one follow-up");
    fixture.shutdown();
}
