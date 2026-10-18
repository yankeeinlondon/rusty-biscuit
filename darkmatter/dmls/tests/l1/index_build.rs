//! Level-1 sessions for the on-demand index build: references, rename, file
//! rename, and workspace symbols revalidate the index store once on first
//! use, concurrent callers share the build, and cancellation, unreadable
//! files, and changes received during the build never leave a partial or
//! stale ready store.
//!
//! Parse and read counts come from `dmls::work_counters`, which are
//! process-wide; nextest runs each test in its own process. A session started
//! with [`LspFixture::start_with_index_round_gate`] holds each build round
//! after its store pass has read its files, so a test can change the
//! workspace while that reading is stale.

use crate::common;

use common::{LspFixture, LspWorkspace};
use dmls::StartupEvent;
use dmls::work_counters::work_counts;
use lsp_server::{RequestId, Response};
use serde_json::{Value, json};

/// `RequestCanceled`.
const REQUEST_CANCELED: i32 = -32800;
/// `RequestFailed`.
const REQUEST_FAILED: i32 = -32803;

const TARGET: &str = "# Target\n\nBody.\n";
const REFERRER: &str = "# Referrer\n\nSee [the target](target.md#target).\n";

fn initialize_params(root: &std::path::Path, progress: bool) -> Value {
    let root_uri = url::Url::from_directory_path(root).unwrap();
    json!({
        "processId": null,
        "clientInfo": { "name": "Index Client", "version": "1.0" },
        "capabilities": {
            "general": { "positionEncodings": ["utf-16"] },
            "window": { "workDoneProgress": progress },
            "workspace": { "didChangeWatchedFiles": { "dynamicRegistration": true } }
        },
        "workspaceFolders": [ { "uri": root_uri.as_str(), "name": "index" } ]
    })
}

fn uri(path: &std::path::Path) -> String {
    url::Url::from_file_path(path).unwrap().to_string()
}

fn write(workspace: &LspWorkspace, name: &str, text: &str) -> std::path::PathBuf {
    let path = workspace.path().join(name);
    std::fs::write(&path, text).unwrap();
    path
}

/// A workspace holding `target.md`, a never-opened `referrer.md` linking to
/// its heading, and `filler` unrelated documents.
fn workspace_with_referrer(filler: usize) -> LspWorkspace {
    let workspace = LspWorkspace::new();
    write(&workspace, "target.md", TARGET);
    write(&workspace, "referrer.md", REFERRER);
    for number in 0..filler {
        write(&workspace, &format!("filler-{number}.md"), &format!("# Filler {number}\n"));
    }
    workspace
}

fn open(fixture: &LspFixture<'_>, uri: &str, text: &str) {
    fixture.notify(
        "textDocument/didOpen",
        json!({ "textDocument": { "uri": uri, "languageId": "markdown", "version": 1, "text": text } }),
    );
}

/// Opens a document and waits until the server has handled the open (a
/// request round trip), so its parse falls outside a later count.
fn open_settled(fixture: &mut LspFixture<'_>, uri: &str, text: &str) {
    open(fixture, uri, text);
    let response = fixture.request("textDocument/documentSymbol", json!({ "textDocument": { "uri": uri } }));
    assert!(response.error.is_none(), "{:?}", response.error);
}

/// `didChangeWatchedFiles` (`kind`: 1 created, 2 changed, 3 deleted).
fn watched(fixture: &LspFixture<'_>, changes: &[(&std::path::Path, u32)]) {
    let changes: Vec<Value> =
        changes.iter().map(|(path, kind)| json!({ "uri": uri(path), "type": kind })).collect();
    fixture.notify("workspace/didChangeWatchedFiles", json!({ "changes": changes }));
}

/// References to `target.md`'s heading.
fn references_params(target: &str) -> Value {
    json!({
        "textDocument": { "uri": target },
        "position": { "line": 0, "character": 3 },
        "context": { "includeDeclaration": false }
    })
}

fn symbols_params() -> Value {
    json!({ "query": "" })
}

/// `SymbolKind::String`, which heading symbols carry (each document also
/// gets a `File` symbol for its title).
const HEADING_SYMBOL: u64 = 15;

/// The `(file name, heading)` pairs of a `workspace/symbol` response.
fn symbol_names(response: &Response) -> Vec<(String, String)> {
    assert!(response.error.is_none(), "{:?}", response.error);
    let mut names: Vec<(String, String)> = response.result.as_ref().unwrap().as_array().unwrap()
        .iter()
        .filter(|symbol| symbol["kind"] == json!(HEADING_SYMBOL))
        .map(|symbol| {
            let uri = symbol["location"]["uri"].as_str().unwrap();
            let file = uri.rsplit('/').next().unwrap().to_string();
            (file, symbol["name"].as_str().unwrap().to_string())
        })
        .collect();
    names.sort();
    names
}

fn referrer_files(response: &Response) -> Vec<String> {
    assert!(response.error.is_none(), "{:?}", response.error);
    response.result.as_ref().unwrap().as_array().unwrap()
        .iter()
        .map(|location| location["uri"].as_str().unwrap().rsplit('/').next().unwrap().to_string())
        .collect()
}

fn parses_during(body: impl FnOnce()) -> usize {
    let before = work_counts();
    body();
    work_counts().since(before).parses
}

/// Each request that needs the index store fills it once, on first use: the
/// first parses exactly the closed listed documents (the open one's buffer
/// is reused), a repeat parses nothing, and the answer includes a referrer
/// that was never opened.
#[test]
fn the_first_references_request_fills_the_store_once() {
    let workspace = workspace_with_referrer(5);
    let target = uri(&workspace.path().join("target.md"));
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(initialize_params(workspace.path(), false));
    open_settled(&mut fixture, &target, TARGET);

    let mut response = None;
    // Six closed listed documents: the referrer and the filler.
    assert_eq!(parses_during(|| response = Some(fixture.request("textDocument/references", references_params(&target)))), 6);
    assert_eq!(referrer_files(&response.unwrap()), vec!["referrer.md".to_string()]);
    assert_eq!(parses_during(|| {
        let again = fixture.request("textDocument/references", references_params(&target));
        assert_eq!(referrer_files(&again), vec!["referrer.md".to_string()]);
    }), 0);
    fixture.shutdown();
}

#[test]
fn the_first_rename_fills_the_store_once_and_edits_a_never_opened_referrer() {
    let workspace = workspace_with_referrer(3);
    let target = uri(&workspace.path().join("target.md"));
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(initialize_params(workspace.path(), false));
    open_settled(&mut fixture, &target, TARGET);
    let rename = json!({
        "textDocument": { "uri": target },
        "position": { "line": 0, "character": 3 },
        "newName": "Goal"
    });

    // Preparing the rename checks only the local target.
    assert_eq!(parses_during(|| {
        let prepared = fixture.request(
            "textDocument/prepareRename",
            json!({ "textDocument": { "uri": target }, "position": { "line": 0, "character": 3 } }),
        );
        assert_eq!(prepared.result.unwrap()["placeholder"], json!("Target"));
    }), 0);
    let mut edit = Value::Null;
    assert_eq!(parses_during(|| edit = fixture.request("textDocument/rename", rename.clone()).result.unwrap()), 4);
    let text = edit.to_string();
    assert!(text.contains("referrer.md") && text.contains("\"goal\""), "{text}");
    assert_eq!(parses_during(|| {
        fixture.request("textDocument/rename", rename.clone()).result.unwrap();
    }), 0);
    fixture.shutdown();
}

#[test]
fn the_first_workspace_symbols_request_fills_the_store_once() {
    let workspace = workspace_with_referrer(2);
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(initialize_params(workspace.path(), false));

    let mut response = None;
    assert_eq!(parses_during(|| response = Some(fixture.request("workspace/symbol", symbols_params()))), 4);
    let names = symbol_names(&response.unwrap());
    assert!(names.contains(&("referrer.md".to_string(), "Referrer".to_string())), "{names:?}");
    assert!(names.contains(&("filler-1.md".to_string(), "Filler 1".to_string())), "{names:?}");
    assert_eq!(parses_during(|| {
        fixture.request("workspace/symbol", symbols_params());
    }), 0);
    fixture.shutdown();
}

#[test]
fn the_first_file_rename_fills_the_store_and_rewrites_a_never_opened_wiki_referrer() {
    let workspace = workspace_with_referrer(1);
    write(&workspace, "wiki-referrer.md", "# Wiki\n\nSee [[target]].\n");
    let target = workspace.path().join("target.md");
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(initialize_params(workspace.path(), false));

    // File rename rewrites wiki links.
    let renamed = workspace.path().join("goal.md");
    let params = json!({ "files": [ { "oldUri": uri(&target), "newUri": uri(&renamed) } ] });
    let mut edit = Value::Null;
    assert_eq!(parses_during(|| edit = fixture.request("workspace/willRenameFiles", params.clone()).result.unwrap()), 4);
    let text = edit.to_string();
    assert!(text.contains("wiki-referrer.md") && text.contains("\"goal\""), "{text}");
    fixture.shutdown();
}

/// Requests that arrive while the build runs wait on that build: the
/// workspace is parsed once, and both are answered from the ready store.
/// The build reports `workDoneProgress` under its own token.
#[test]
fn simultaneous_callers_share_one_build() {
    let workspace = workspace_with_referrer(4);
    let target = uri(&workspace.path().join("target.md"));
    let mut fixture = LspFixture::start_with_index_round_gate(&workspace);
    fixture.initialize(initialize_params(workspace.path(), true));
    open_settled(&mut fixture, &target, TARGET);

    let before = work_counts();
    fixture.send_request_with_id(RequestId::from(101), "textDocument/references", references_params(&target));
    fixture.wait_for_event(StartupEvent::IndexRoundRead);
    fixture.send_request_with_id(RequestId::from(102), "workspace/symbol", symbols_params());
    fixture.release_index_round();
    let references = fixture.expect_response(RequestId::from(101));
    let symbols = fixture.expect_response(RequestId::from(102));
    assert_eq!(work_counts().since(before).parses, 5, "one build parsed each closed document once");
    assert_eq!(referrer_files(&references), vec!["referrer.md".to_string()]);
    assert_eq!(symbol_names(&symbols).len(), 6);
    assert!(
        fixture.buffered_server_requests().iter().any(|request| {
            request.method == "window/workDoneProgress/create"
                && request.params["token"] == json!("dmls/index")
        }),
        "{:?}",
        fixture.buffered_server_requests()
    );
    fixture.wait_for_progress_end("dmls/index");
    fixture.shutdown();
}

/// Cancelling one waiter answers it at once and leaves the build running for
/// the other.
#[test]
fn cancelling_one_waiter_leaves_the_shared_build_running() {
    let workspace = workspace_with_referrer(2);
    let target = uri(&workspace.path().join("target.md"));
    let mut fixture = LspFixture::start_with_index_round_gate(&workspace);
    fixture.initialize(initialize_params(workspace.path(), false));
    open_settled(&mut fixture, &target, TARGET);

    fixture.send_request_with_id(RequestId::from(201), "textDocument/references", references_params(&target));
    fixture.send_request_with_id(RequestId::from(202), "workspace/symbol", symbols_params());
    fixture.wait_for_event(StartupEvent::IndexRoundRead);
    fixture.notify("$/cancelRequest", json!({ "id": 201 }));
    // Answered while the round is still held.
    let cancelled = fixture.expect_response(RequestId::from(201));
    assert_eq!(cancelled.error.map(|error| error.code), Some(REQUEST_CANCELED));
    fixture.release_index_round();
    let symbols = fixture.expect_response(RequestId::from(202));
    assert_eq!(symbol_names(&symbols).len(), 4);
    fixture.shutdown();
}

/// A build whose only waiter cancels is abandoned and never becomes ready:
/// the next request builds again, reading every file again. The rows the
/// abandoned round wrote are a cache keyed on each file's bytes, so the
/// retry reuses them instead of parsing.
#[test]
fn cancelling_the_last_waiter_abandons_the_build_and_a_later_request_retries() {
    let workspace = workspace_with_referrer(2);
    let target = uri(&workspace.path().join("target.md"));
    let mut fixture = LspFixture::start_with_index_round_gate(&workspace);
    fixture.initialize(initialize_params(workspace.path(), false));
    open_settled(&mut fixture, &target, TARGET);

    fixture.send_request_with_id(RequestId::from(301), "textDocument/references", references_params(&target));
    fixture.wait_for_event(StartupEvent::IndexRoundRead);
    fixture.notify("$/cancelRequest", json!({ "id": 301 }));
    let cancelled = fixture.expect_response(RequestId::from(301));
    assert_eq!(cancelled.error.map(|error| error.code), Some(REQUEST_CANCELED));
    // The abandoned round's results reach the server and are discarded.
    fixture.release_index_round();

    let before = work_counts();
    fixture.send_request_with_id(RequestId::from(302), "textDocument/references", references_params(&target));
    fixture.wait_for_event(StartupEvent::IndexRoundRead);
    fixture.release_index_round();
    let references = fixture.expect_response(RequestId::from(302));
    assert_eq!(referrer_files(&references), vec!["referrer.md".to_string()]);
    let work = work_counts().since(before);
    assert_eq!(work.reads, 4, "the retry read every listed file again: {work:?}");
    assert_eq!(work.parses, 0, "and reused the abandoned round's rows: {work:?}");
    assert_eq!(parses_during(|| {
        fixture.request("textDocument/references", references_params(&target));
    }), 0);
    fixture.shutdown();
}

/// A listed file that cannot be read fails the build: every waiter gets an
/// error naming the file, rename issues no partial edit, and once the file
/// is readable a later request fills the store.
#[test]
fn an_unreadable_listed_file_fails_the_build_and_a_later_request_retries() {
    let workspace = workspace_with_referrer(1);
    let target = uri(&workspace.path().join("target.md"));
    let unreadable = workspace.path().join("binary.md");
    std::fs::write(&unreadable, [0xff, 0xfe, 0x00, 0x80]).unwrap();
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(initialize_params(workspace.path(), false));
    open_settled(&mut fixture, &target, TARGET);

    let references = fixture.request("textDocument/references", references_params(&target));
    let error = references.error.expect("the build failed");
    assert_eq!(error.code, REQUEST_FAILED);
    assert!(error.message.contains("binary.md"), "{}", error.message);
    let rename = fixture.request(
        "textDocument/rename",
        json!({ "textDocument": { "uri": target }, "position": { "line": 0, "character": 3 }, "newName": "Goal" }),
    );
    assert!(rename.result.is_none(), "no partial edit: {:?}", rename.result);
    assert_eq!(rename.error.map(|error| error.code), Some(REQUEST_FAILED));

    std::fs::write(&unreadable, "# Readable now\n").unwrap();
    watched(&fixture, &[(&unreadable, 2)]);
    let references = fixture.request("textDocument/references", references_params(&target));
    assert_eq!(referrer_files(&references), vec!["referrer.md".to_string()]);
    let symbols = fixture.request("workspace/symbol", symbols_params());
    assert!(symbol_names(&symbols).contains(&("binary.md".to_string(), "Readable now".to_string())));
    fixture.shutdown();
}

/// Disk edits, deletions, and buffer opens received while a round's reading
/// goes stale are reconciled before the store is ready.
#[test]
fn edits_and_deletes_during_a_build_never_reach_the_ready_index() {
    let workspace = LspWorkspace::new();
    let edited = write(&workspace, "edited.md", "# Old title\n");
    let deleted = write(&workspace, "deleted.md", "# Deleted\n");
    let opened = write(&workspace, "opened.md", "# Disk title\n");
    write(&workspace, "steady.md", "# Steady\n");
    let mut fixture = LspFixture::start_with_index_round_gate(&workspace);
    fixture.initialize(initialize_params(workspace.path(), false));

    fixture.send_request_with_id(RequestId::from(401), "workspace/symbol", symbols_params());
    fixture.wait_for_event(StartupEvent::IndexRoundRead);
    std::fs::write(&edited, "# New title\n").unwrap();
    std::fs::remove_file(&deleted).unwrap();
    watched(&fixture, &[(&edited, 2), (&deleted, 3)]);
    open(&fixture, &uri(&opened), "# Buffer title\n");
    fixture.open_index_round_gate();
    fixture.release_index_round();

    let symbols = symbol_names(&fixture.expect_response(RequestId::from(401)));
    assert_eq!(
        symbols,
        vec![
            ("edited.md".to_string(), "New title".to_string()),
            ("opened.md".to_string(), "Buffer title".to_string()),
            ("steady.md".to_string(), "Steady".to_string()),
        ]
    );
    fixture.shutdown();
}

/// A workspace file rename received during a build leaves no trace of the
/// old path, and the new path's contents are in the ready store.
#[test]
fn a_file_rename_during_a_build_cannot_leave_a_stale_snapshot() {
    let workspace = LspWorkspace::new();
    let old = write(&workspace, "old-name.md", "# Moving\n");
    write(&workspace, "steady.md", "# Steady\n");
    let mut fixture = LspFixture::start_with_index_round_gate(&workspace);
    fixture.initialize(initialize_params(workspace.path(), false));

    fixture.send_request_with_id(RequestId::from(501), "workspace/symbol", symbols_params());
    fixture.wait_for_event(StartupEvent::IndexRoundRead);
    let new = workspace.path().join("new-name.md");
    std::fs::rename(&old, &new).unwrap();
    watched(&fixture, &[(&old, 3), (&new, 1)]);
    fixture.open_index_round_gate();
    fixture.release_index_round();

    let symbols = symbol_names(&fixture.expect_response(RequestId::from(501)));
    assert_eq!(
        symbols,
        vec![
            ("new-name.md".to_string(), "Moving".to_string()),
            ("steady.md".to_string(), "Steady".to_string()),
        ]
    );
    fixture.shutdown();
}

/// A request that arrives before the file list waits for it, then for the
/// build, and is answered from the ready store.
#[test]
fn a_request_before_the_file_list_waits_for_it_and_the_build() {
    let workspace = workspace_with_referrer(1);
    let mut fixture = LspFixture::start_with_file_list_gate(&workspace);
    fixture.initialize_unlisted(initialize_params(workspace.path(), false));

    fixture.send_request_with_id(RequestId::from(601), "workspace/symbol", symbols_params());
    fixture.release_file_list();
    let symbols = symbol_names(&fixture.expect_response(RequestId::from(601)));
    assert_eq!(symbols.len(), 3, "{symbols:?}");
    fixture.shutdown();
}

/// Shutting down mid-build answers the waiter instead of leaving it
/// unanswered.
#[test]
fn shutdown_during_a_build_answers_its_waiters() {
    let workspace = workspace_with_referrer(1);
    let mut fixture = LspFixture::start_with_index_round_gate(&workspace);
    fixture.initialize(initialize_params(workspace.path(), false));

    fixture.send_request_with_id(RequestId::from(701), "workspace/symbol", symbols_params());
    fixture.wait_for_event(StartupEvent::IndexRoundRead);
    fixture.send_request_with_id(RequestId::from(702), "shutdown", Value::Null);
    let waiter = fixture.expect_response(RequestId::from(701));
    assert_eq!(waiter.error.map(|error| error.code), Some(-32802), "ServerCancelled");
    let shutdown = fixture.expect_response(RequestId::from(702));
    fixture.open_index_round_gate();
    fixture.exit_after_shutdown(shutdown);
}

/// Bytes that are not UTF-8: the deterministic read-failure seam.
const UNREADABLE: [u8; 4] = [0xff, 0xfe, 0x00, 0x80];

/// How a ready store loses a readable document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Transition {
    /// A watched change makes the closed referrer unreadable.
    WatchedModification,
    /// A watched creation adds an unreadable Markdown file.
    WatchedCreation,
    /// A save-driven rescan (no client watcher) with the closed referrer
    /// made unreadable on disk, which no event reports.
    RescanOnSave,
    /// The referrer's buffer closes over unreadable disk bytes.
    BufferClose,
    /// An ignore-file event relists the workspace, which adds an unreadable
    /// Markdown file.
    Relist,
}

/// A request that needs the index store.
#[derive(Debug, Clone, Copy)]
enum Consumer {
    References,
    HeadingRename,
    FileRename,
    WorkspaceSymbols,
}

/// A session over `target.md` (open) and `referrer.md` whose store is
/// ready, after `transition` has made one listed file unreadable.
///
/// ## Returns
///
/// The fixture, and the unreadable file's name.
fn ready_then(workspace: &LspWorkspace, transition: Transition) -> (LspFixture<'_>, &'static str) {
    let target = uri(&workspace.path().join("target.md"));
    let referrer = workspace.path().join("referrer.md");
    let mut fixture = LspFixture::start(workspace);
    let mut params = initialize_params(workspace.path(), false);
    if transition == Transition::RescanOnSave {
        params["capabilities"]["workspace"] = json!({});
    }
    fixture.initialize(params);
    open_settled(&mut fixture, &target, TARGET);
    if transition == Transition::BufferClose {
        open_settled(&mut fixture, &uri(&referrer), REFERRER);
    }
    let symbols = fixture.request("workspace/symbol", symbols_params());
    assert!(symbol_names(&symbols).contains(&("referrer.md".to_string(), "Referrer".to_string())));

    let unreadable = match transition {
        Transition::WatchedModification => {
            std::fs::write(&referrer, UNREADABLE).unwrap();
            watched(&fixture, &[(&referrer, 2)]);
            "referrer.md"
        }
        Transition::WatchedCreation => {
            let created = workspace.path().join("binary.md");
            std::fs::write(&created, UNREADABLE).unwrap();
            watched(&fixture, &[(&created, 1)]);
            "binary.md"
        }
        Transition::RescanOnSave => {
            std::fs::write(&referrer, UNREADABLE).unwrap();
            fixture.notify("textDocument/didSave", json!({ "textDocument": { "uri": target } }));
            "referrer.md"
        }
        Transition::BufferClose => {
            std::fs::write(&referrer, UNREADABLE).unwrap();
            fixture.notify("textDocument/didClose", json!({ "textDocument": { "uri": uri(&referrer) } }));
            "referrer.md"
        }
        Transition::Relist => {
            std::fs::write(workspace.path().join("binary.md"), UNREADABLE).unwrap();
            let ignore = write(workspace, ".ignore", "ignored/\n");
            watched(&fixture, &[(&ignore, 1)]);
            "binary.md"
        }
    };
    (fixture, unreadable)
}

fn rename_params(target: &str) -> Value {
    json!({ "textDocument": { "uri": target }, "position": { "line": 0, "character": 3 }, "newName": "Goal" })
}

fn file_rename_params(workspace: &LspWorkspace) -> Value {
    let target = workspace.path().join("target.md");
    json!({ "files": [ { "oldUri": uri(&target), "newUri": uri(&workspace.path().join("goal.md")) } ] })
}

fn consume(fixture: &mut LspFixture<'_>, workspace: &LspWorkspace, consumer: Consumer) -> Response {
    let target = uri(&workspace.path().join("target.md"));
    match consumer {
        Consumer::References => fixture.request("textDocument/references", references_params(&target)),
        Consumer::HeadingRename => fixture.request("textDocument/rename", rename_params(&target)),
        Consumer::FileRename => fixture.request("workspace/willRenameFiles", file_rename_params(workspace)),
        Consumer::WorkspaceSymbols => fixture.request("workspace/symbol", symbols_params()),
    }
}

/// Every consumer of the index store fails, with the first build's error
/// naming the unreadable file and no partial result, after each way a ready
/// store can learn that a listed document became unreadable.
macro_rules! readiness_withdrawn {
    ($($name:ident: $transition:ident, $consumer:ident;)*) => {$(
        #[test]
        fn $name() {
            let workspace = workspace_with_referrer(1);
            let (mut fixture, unreadable) = ready_then(&workspace, Transition::$transition);
            let response = consume(&mut fixture, &workspace, Consumer::$consumer);
            assert!(response.result.is_none(), "no partial result: {:?}", response.result);
            let error = response.error.expect("the store is not ready");
            assert_eq!(error.code, REQUEST_FAILED);
            assert!(error.message.contains(unreadable), "{}", error.message);
            fixture.shutdown();
        }
    )*};
}

readiness_withdrawn! {
    a_ready_store_after_a_watched_modification_fails_references: WatchedModification, References;
    a_ready_store_after_a_watched_modification_fails_heading_rename: WatchedModification, HeadingRename;
    a_ready_store_after_a_watched_modification_fails_file_rename: WatchedModification, FileRename;
    a_ready_store_after_a_watched_modification_fails_workspace_symbols: WatchedModification, WorkspaceSymbols;
    a_ready_store_after_a_watched_creation_fails_references: WatchedCreation, References;
    a_ready_store_after_a_watched_creation_fails_heading_rename: WatchedCreation, HeadingRename;
    a_ready_store_after_a_watched_creation_fails_file_rename: WatchedCreation, FileRename;
    a_ready_store_after_a_watched_creation_fails_workspace_symbols: WatchedCreation, WorkspaceSymbols;
    a_ready_store_after_a_save_rescan_fails_heading_rename: RescanOnSave, HeadingRename;
    a_ready_store_after_a_save_rescan_fails_file_rename: RescanOnSave, FileRename;
    a_ready_store_after_a_buffer_close_fails_references: BufferClose, References;
    a_ready_store_after_a_buffer_close_fails_heading_rename: BufferClose, HeadingRename;
    a_ready_store_after_a_buffer_close_fails_file_rename: BufferClose, FileRename;
    a_ready_store_after_a_buffer_close_fails_workspace_symbols: BufferClose, WorkspaceSymbols;
    a_ready_store_after_a_relist_fails_references: Relist, References;
    a_ready_store_after_a_relist_fails_heading_rename: Relist, HeadingRename;
    a_ready_store_after_a_relist_fails_file_rename: Relist, FileRename;
    a_ready_store_after_a_relist_fails_workspace_symbols: Relist, WorkspaceSymbols;
}

/// Restoring the bytes and sending another change retries the read: the
/// referrer is back in references and in the rename's edits, and nothing is
/// read again for the next request.
#[test]
fn a_repaired_file_after_readiness_is_read_again_and_complete_answers_return() {
    let workspace = workspace_with_referrer(1);
    let (mut fixture, _) = ready_then(&workspace, Transition::WatchedModification);
    let target = uri(&workspace.path().join("target.md"));
    let referrer = workspace.path().join("referrer.md");
    assert!(consume(&mut fixture, &workspace, Consumer::References).error.is_some());

    std::fs::write(&referrer, REFERRER).unwrap();
    watched(&fixture, &[(&referrer, 2)]);
    // A round trip, so the change has been handled before counting.
    fixture.request("textDocument/documentSymbol", json!({ "textDocument": { "uri": target } }));
    assert_eq!(parses_during(|| {
        let references = fixture.request("textDocument/references", references_params(&target));
        assert_eq!(referrer_files(&references), vec!["referrer.md".to_string()]);
    }), 0, "the change event read the repaired file");
    let edit = fixture.request("textDocument/rename", rename_params(&target)).result.unwrap();
    assert!(edit.to_string().contains("referrer.md"), "{edit}");
    fixture.shutdown();
}

/// In fallback watch mode a save re-reads only the saved file and the files
/// it knows changed, so a referrer made unreadable on disk with no event
/// keeps its row: references and workspace symbols answer from it. Every
/// rename re-reads every file first, so it finds the referrer unreadable and
/// fails, and after a repair it edits the referrer.
#[test]
fn in_fallback_mode_an_unannounced_unreadable_file_fails_only_renames() {
    let workspace = workspace_with_referrer(1);
    let (mut fixture, _) = ready_then(&workspace, Transition::RescanOnSave);
    let target = uri(&workspace.path().join("target.md"));
    let references = consume(&mut fixture, &workspace, Consumer::References);
    assert_eq!(referrer_files(&references), vec!["referrer.md".to_string()], "the referrer's old row answers");
    let symbols = consume(&mut fixture, &workspace, Consumer::WorkspaceSymbols);
    assert!(symbol_names(&symbols).contains(&("referrer.md".to_string(), "Referrer".to_string())));
    for consumer in [Consumer::HeadingRename, Consumer::FileRename] {
        let error = consume(&mut fixture, &workspace, consumer).error.expect("the rename's own pass fails");
        assert_eq!(error.code, REQUEST_FAILED);
        assert!(error.message.contains("referrer.md"), "{}", error.message);
    }

    std::fs::write(workspace.path().join("referrer.md"), REFERRER).unwrap();
    fixture.notify("textDocument/didSave", json!({ "textDocument": { "uri": target } }));
    let edit = fixture.request("textDocument/rename", rename_params(&target)).result.unwrap();
    assert!(edit.to_string().contains("referrer.md"), "{edit}");
    fixture.shutdown();
}

/// An unreadable file that is then deleted leaves the list: it is absent,
/// not unreadable, and the store is ready again.
#[test]
fn deleting_a_file_that_failed_after_readiness_makes_the_store_ready_again() {
    let workspace = workspace_with_referrer(1);
    let (mut fixture, _) = ready_then(&workspace, Transition::WatchedCreation);
    let created = workspace.path().join("binary.md");
    std::fs::remove_file(&created).unwrap();
    watched(&fixture, &[(&created, 3)]);
    let target = uri(&workspace.path().join("target.md"));
    let references = fixture.request("textDocument/references", references_params(&target));
    assert_eq!(referrer_files(&references), vec!["referrer.md".to_string()]);
    let symbols = consume(&mut fixture, &workspace, Consumer::WorkspaceSymbols);
    assert!(!symbol_names(&symbols).iter().any(|(file, _)| file == "binary.md"));
    fixture.shutdown();
}
