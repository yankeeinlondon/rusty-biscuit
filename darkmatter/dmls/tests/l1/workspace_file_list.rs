//! Level-1 sessions for the workspace file list and on-demand parsing: what
//! startup reads, when path diagnostics publish, which documents a feature
//! parses, and how disk changes, open buffers, and watch fallbacks keep the
//! list and the parses current.
//!
//! Parse and read counts come from `dmls::work_counters`, which are
//! process-wide; nextest runs each test in its own process, so a delta
//! counts only this test's server.

use crate::common;

use common::{LspFixture, LspWorkspace};
use dmls::work_counters::{WorkCounts, work_counts};
use lsp_server::Response;
use serde_json::{Value, json};

fn initialize_params(root: &std::path::Path, client: &str, watched: Option<Value>) -> Value {
    let root_uri = url::Url::from_directory_path(root).unwrap();
    let mut params = json!({
        "processId": null,
        "clientInfo": { "name": client, "version": "1.0" },
        "capabilities": {
            "general": { "positionEncodings": ["utf-16"] },
            "workspace": { "configuration": true }
        },
        "workspaceFolders": [ { "uri": root_uri.as_str(), "name": "scratch" } ]
    });
    if let Some(watched) = watched {
        params["capabilities"]["workspace"]["didChangeWatchedFiles"] = watched;
    }
    params
}

/// A client that watches files (`ClientWatched`).
fn watched_params(root: &std::path::Path) -> Value {
    initialize_params(root, "Watched Client", Some(json!({ "dynamicRegistration": true })))
}

/// A client without file watching (`ServerRescan`: the save fallback).
fn unwatched_params(root: &std::path::Path) -> Value {
    initialize_params(root, "Unwatched Client", None)
}

fn uri(path: &std::path::Path) -> String {
    url::Url::from_file_path(path).unwrap().to_string()
}

fn open(fixture: &LspFixture<'_>, uri: &str, text: &str) {
    fixture.notify(
        "textDocument/didOpen",
        json!({ "textDocument": { "uri": uri, "languageId": "markdown", "version": 1, "text": text } }),
    );
}

fn change(fixture: &LspFixture<'_>, uri: &str, version: i32, text: &str) {
    fixture.notify(
        "textDocument/didChange",
        json!({ "textDocument": { "uri": uri, "version": version }, "contentChanges": [ { "text": text } ] }),
    );
}

fn save(fixture: &LspFixture<'_>, uri: &str) {
    fixture.notify("textDocument/didSave", json!({ "textDocument": { "uri": uri } }));
}

/// `didChangeWatchedFiles` for one path (`kind`: 1 created, 2 changed, 3 deleted).
fn watched_change(fixture: &LspFixture<'_>, path: &std::path::Path, kind: u32) {
    fixture.notify(
        "workspace/didChangeWatchedFiles",
        json!({ "changes": [ { "uri": uri(path), "type": kind } ] }),
    );
}

/// The diagnostics last published for `uri` once the server has handled
/// every earlier message and finished the registry jobs they queued; older
/// buffered publications for `uri` are discarded. A response alone is no
/// barrier: a job can hold the refresh after the registry is installed.
/// The round trip is `documentSymbol`, which parses only `uri`.
fn latest(fixture: &mut LspFixture<'_>, uri: &str) -> Vec<Value> {
    fixture.flush_server_with("textDocument/documentSymbol", json!({ "textDocument": { "uri": uri } }));
    let latest = fixture.take_buffered_diagnostics(uri).expect("diagnostics were published");
    while fixture.take_buffered_diagnostics(uri).is_some() {}
    latest["diagnostics"].as_array().cloned().unwrap_or_default()
}

fn codes(diagnostics: &[Value]) -> Vec<String> {
    let mut codes: Vec<String> = diagnostics
        .iter()
        .filter_map(|diagnostic| diagnostic["code"].as_str().map(str::to_string))
        .collect();
    codes.sort();
    codes
}

fn work_since(before: WorkCounts) -> WorkCounts {
    work_counts().since(before)
}

#[test]
fn startup_parses_only_the_open_documents() {
    let workspace = LspWorkspace::new();
    for index in 0..300 {
        let text = format!("# Note {index}\n\n[next](note-{}.md) [[note-{}]]\n", index + 1, index + 2);
        std::fs::write(workspace.path().join(format!("note-{index}.md")), text).unwrap();
    }
    let source = workspace.path().join("note-0.md");
    let text = std::fs::read_to_string(&source).unwrap();
    let before = work_counts();

    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(watched_params(workspace.path()));
    assert_eq!(work_since(before).reads, 0, "listing reads no file contents");
    open(&fixture, &uri(&source), &text);
    let diagnostics = latest(&mut fixture, &uri(&source));

    // The links resolve against the list (note-1.md exists, note-2 is a
    // listed wiki target) without parsing either target.
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    assert_eq!(work_since(before).parses, 1, "only the open document is parsed");
    fixture.shutdown();
}

#[test]
fn path_diagnostics_wait_for_the_file_list() {
    let workspace = LspWorkspace::new();
    let source = workspace.path().join("source.md");
    // A broken link and a wiki link to nothing (both need the list), and a
    // duplicate heading (needs only the document).
    let text = "# Same\n\n# Same\n\n[bad](nope.md) [[missing]]\n";

    let mut fixture = LspFixture::start_with_file_list_gate(&workspace);
    fixture.initialize_unlisted(watched_params(workspace.path()));
    open(&fixture, &uri(&source), text);
    // Startup counts as an outstanding registry job until discovery returns,
    // so `latest`'s flush would wait for the gated walk; a document request
    // alone settles the open's diagnostics (the document has no frontmatter,
    // so it queues no registry job).
    let response = fixture.request("textDocument/documentSymbol", json!({ "textDocument": { "uri": uri(&source) } }));
    assert!(response.error.is_none(), "{response:?}");
    let early = fixture.take_buffered_diagnostics(&uri(&source)).expect("diagnostics were published");
    let early = codes(early["diagnostics"].as_array().expect("diagnostics array"));
    let duplicate = "dm.links.duplicate_heading".to_string();
    assert_eq!(early, vec![duplicate.clone(), duplicate.clone()], "before the list: {early:?}");

    fixture.release_file_list();
    fixture.wait_for_file_list();
    let ready = codes(&latest(&mut fixture, &uri(&source)));
    assert_eq!(
        ready,
        vec![
            "dm.links.broken_path".to_string(),
            duplicate.clone(),
            duplicate,
            "wiki.unresolved-target".to_string(),
        ],
        "once the list is ready: {ready:?}"
    );
    fixture.shutdown();
}

#[test]
fn a_missing_anchor_check_parses_exactly_its_target_and_reuses_it() {
    let workspace = LspWorkspace::new();
    let target = workspace.path().join("target.md");
    std::fs::write(&target, "# Real\n").unwrap();
    for name in ["other.md", "third.md"] {
        std::fs::write(workspace.path().join(name), "# Other\n\n[t](target.md#real)\n").unwrap();
    }
    let source = workspace.path().join("source.md");
    let missing = "[x](target.md#missing) [y](other.md)\n";
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(watched_params(workspace.path()));

    let before = work_counts();
    open(&fixture, &uri(&source), missing);
    let diagnostics = codes(&latest(&mut fixture, &uri(&source)));
    assert_eq!(diagnostics, vec!["dm.links.missing_anchor".to_string()]);
    let work = work_since(before);
    assert_eq!((work.parses, work.reads), (2, 1), "the source and its anchor target: {work:?}");

    // A second check reuses the target's parse without reading it.
    let before = work_counts();
    change(&fixture, &uri(&source), 2, "[x](target.md#real) [y](other.md)\n");
    assert!(latest(&mut fixture, &uri(&source)).is_empty());
    let work = work_since(before);
    assert_eq!((work.parses, work.reads), (1, 0), "only the edited source: {work:?}");

    // A watched change with unchanged bytes re-reads but does not re-parse.
    let before = work_counts();
    watched_change(&fixture, &target, 2);
    assert!(latest(&mut fixture, &uri(&source)).is_empty());
    let work = work_since(before);
    assert_eq!((work.parses, work.reads), (0, 1), "{work:?}");

    // A changed target is re-read and re-parsed.
    std::fs::write(&target, "# Renamed\n").unwrap();
    let before = work_counts();
    watched_change(&fixture, &target, 2);
    let diagnostics = codes(&latest(&mut fixture, &uri(&source)));
    assert_eq!(diagnostics, vec!["dm.links.missing_anchor".to_string()]);
    let work = work_since(before);
    assert_eq!((work.parses, work.reads), (1, 1), "{work:?}");

    // Deleting it leaves the link broken, never served from the old parse.
    std::fs::remove_file(&target).unwrap();
    watched_change(&fixture, &target, 3);
    let diagnostics = codes(&latest(&mut fixture, &uri(&source)));
    assert_eq!(diagnostics, vec!["dm.links.broken_path".to_string()]);
    fixture.shutdown();
}

#[test]
fn heading_completion_and_hover_parse_their_target_on_demand() {
    let workspace = LspWorkspace::new();
    std::fs::write(workspace.path().join("target.md"), "# Alpha\n\n## Beta\n").unwrap();
    std::fs::write(workspace.path().join("wiki.md"), "# Gamma\n").unwrap();
    let source = workspace.path().join("source.md");
    let text = "[t](target.md#\n[[wiki#\n";
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(watched_params(workspace.path()));
    open(&fixture, &uri(&source), text);
    latest(&mut fixture, &uri(&source));

    let labels = |fixture: &mut LspFixture<'_>, line: u32, character: u32| -> Vec<String> {
        let result = fixture
            .request(
                "textDocument/completion",
                json!({ "textDocument": { "uri": uri(&source) }, "position": { "line": line, "character": character } }),
            )
            .result
            .expect("completion result");
        let items = result.as_array().cloned().or_else(|| result["items"].as_array().cloned()).unwrap_or_default();
        let mut labels: Vec<String> =
            items.iter().filter_map(|item| item["label"].as_str().map(str::to_string)).collect();
        labels.sort();
        labels
    };
    let before = work_counts();
    assert_eq!(labels(&mut fixture, 0, 14), vec!["alpha".to_string(), "beta".to_string()]);
    assert_eq!(work_since(before).parses, 1, "the link's target only");
    assert_eq!(labels(&mut fixture, 1, 7), vec!["Gamma".to_string()]);
    assert_eq!(work_since(before).parses, 2, "then the wiki target");
    fixture.shutdown();
}

#[test]
fn an_open_buffer_overrides_disk_and_closing_it_restores_the_disk_view() {
    let workspace = LspWorkspace::new();
    let target = workspace.path().join("target.md");
    std::fs::write(&target, "# Disk\n").unwrap();
    let source = workspace.path().join("source.md");
    let unsaved = workspace.path().join("unsaved.md");
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(watched_params(workspace.path()));
    open(&fixture, &uri(&source), "[x](target.md#buffer) [[unsaved]]\n");
    let diagnostics = codes(&latest(&mut fixture, &uri(&source)));
    assert_eq!(
        diagnostics,
        vec!["dm.links.missing_anchor".to_string(), "wiki.unresolved-target".to_string()]
    );

    // Open buffers, including one never saved to disk, are authoritative.
    open(&fixture, &uri(&target), "# Buffer\n");
    open(&fixture, &uri(&unsaved), "# Unsaved\n");
    let before = work_counts();
    change(&fixture, &uri(&source), 2, "[x](target.md#buffer) [[unsaved]]\n\n");
    assert!(latest(&mut fixture, &uri(&source)).is_empty());
    assert_eq!(work_since(before).reads, 0, "buffers are not read from disk");

    // Closing restores the disk view; the unsaved document leaves the index.
    for closed in [&target, &unsaved] {
        fixture.notify("textDocument/didClose", json!({ "textDocument": { "uri": uri(closed) } }));
    }
    change(&fixture, &uri(&source), 3, "[x](target.md#buffer) [[unsaved]]\n");
    let diagnostics = codes(&latest(&mut fixture, &uri(&source)));
    assert_eq!(
        diagnostics,
        vec!["dm.links.missing_anchor".to_string(), "wiki.unresolved-target".to_string()]
    );
    fixture.shutdown();
}

#[test]
fn the_save_fallback_lists_without_parsing_and_rechecks_only_parsed_targets() {
    let workspace = LspWorkspace::new();
    let target = workspace.path().join("target.md");
    std::fs::write(&target, "# Real\n").unwrap();
    for index in 0..20 {
        std::fs::write(workspace.path().join(format!("unrelated-{index}.md")), "# U\n").unwrap();
    }
    let source = workspace.path().join("source.md");
    let text = "[x](target.md#real) [[created]]\n";
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(unwatched_params(workspace.path()));
    open(&fixture, &uri(&source), text);
    let diagnostics = codes(&latest(&mut fixture, &uri(&source)));
    assert_eq!(diagnostics, vec!["wiki.unresolved-target".to_string()]);

    // A save lists the workspace (adopting the created file) and re-reads
    // only the one parsed closed target, whose bytes are unchanged.
    std::fs::write(workspace.path().join("created.md"), "# Created\n").unwrap();
    let before = work_counts();
    save(&fixture, &uri(&source));
    assert!(latest(&mut fixture, &uri(&source)).is_empty());
    let work = work_since(before);
    assert_eq!((work.parses, work.reads), (0, 1), "{work:?}");

    // A changed target is re-parsed on the next save.
    std::fs::write(&target, "# Moved\n").unwrap();
    let before = work_counts();
    save(&fixture, &uri(&source));
    let diagnostics = codes(&latest(&mut fixture, &uri(&source)));
    assert_eq!(diagnostics, vec!["dm.links.missing_anchor".to_string()]);
    assert_eq!(work_since(before).parses, 1);

    // A discovery-configuration change re-lists without parsing.
    let before = work_counts();
    fixture.notify(
        "workspace/didChangeConfiguration",
        json!({ "settings": { "dmls": { "workspace": { "exclude": ["created.md"] } } } }),
    );
    let diagnostics = codes(&latest(&mut fixture, &uri(&source)));
    assert_eq!(
        diagnostics,
        vec!["dm.links.missing_anchor".to_string(), "wiki.unresolved-target".to_string()]
    );
    assert_eq!(work_since(before).parses, 0);
    fixture.shutdown();
}

#[test]
fn an_ignore_file_change_rebuilds_the_list_without_parsing() {
    let workspace = LspWorkspace::new();
    std::fs::create_dir(workspace.path().join("vendor")).unwrap();
    std::fs::write(workspace.path().join("vendor/lib.md"), "# Lib\n").unwrap();
    let source = workspace.path().join("source.md");
    let text = "[[lib]]\n";
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(watched_params(workspace.path()));
    open(&fixture, &uri(&source), text);
    assert!(latest(&mut fixture, &uri(&source)).is_empty());

    let gitignore = workspace.path().join(".gitignore");
    std::fs::write(&gitignore, "vendor/\n").unwrap();
    let before = work_counts();
    watched_change(&fixture, &gitignore, 1);
    let diagnostics = codes(&latest(&mut fixture, &uri(&source)));
    assert_eq!(diagnostics, vec!["wiki.unresolved-target".to_string()]);
    assert_eq!(work_since(before).parses, 0);

    // A watched event for a file the ignore rules exclude does not list it.
    std::fs::write(workspace.path().join("vendor/other.md"), "# Other\n").unwrap();
    watched_change(&fixture, &workspace.path().join("vendor/other.md"), 1);
    change(&fixture, &uri(&source), 2, "[[other]]\n");
    let diagnostics = codes(&latest(&mut fixture, &uri(&source)));
    assert_eq!(diagnostics, vec!["wiki.unresolved-target".to_string()]);
    fixture.shutdown();
}

#[test]
fn a_rejected_watcher_registration_falls_back_to_rescanning_on_save() {
    let workspace = LspWorkspace::new();
    let source = workspace.path().join("source.md");
    let text = "[[later]]\n";
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(watched_params(workspace.path()));
    let registration = fixture
        .take_server_request("client/registerCapability")
        .expect("workspace watchers are registered");
    let watchers = &registration.params["registrations"][0]["registerOptions"]["watchers"];
    let globs: Vec<&str> = watchers
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|watcher| watcher["globPattern"].as_str())
        .collect();
    for glob in ["**/*.yaml", "**/*.yml", "**/.gitignore"] {
        assert!(globs.contains(&glob), "{glob} watched: {globs:?}");
    }
    fixture.respond(Response::new_err(registration.id, -32601, "watching is disabled".to_string()));
    open(&fixture, &uri(&source), text);
    let diagnostics = codes(&latest(&mut fixture, &uri(&source)));
    assert_eq!(diagnostics, vec!["wiki.unresolved-target".to_string()]);

    // No watched event will arrive; the save rescans instead.
    std::fs::write(workspace.path().join("later.md"), "# Later\n").unwrap();
    save(&fixture, &uri(&source));
    assert!(latest(&mut fixture, &uri(&source)).is_empty());
    fixture.shutdown();
}

#[test]
fn yaml_is_listed_and_watched_but_never_a_document() {
    let workspace = LspWorkspace::new();
    std::fs::write(workspace.path().join("config.yaml"), "a: 1\n").unwrap();
    let source = workspace.path().join("source.md");
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(watched_params(workspace.path()));
    open(&fixture, &uri(&source), "[[config]]\n");
    let diagnostics = codes(&latest(&mut fixture, &uri(&source)));
    assert_eq!(diagnostics, vec!["wiki.unresolved-target".to_string()]);

    // A watched YAML change is listed, not parsed as Markdown.
    let before = work_counts();
    watched_change(&fixture, &workspace.path().join("config.yaml"), 2);
    let diagnostics = codes(&latest(&mut fixture, &uri(&source)));
    assert_eq!(diagnostics, vec!["wiki.unresolved-target".to_string()]);
    assert_eq!(work_since(before).parses, 0);
    fixture.shutdown();
}

/// A strict-mode workspace whose `schemas/prompt.trigger.yaml` loads a
/// payload from `external`, outside every workspace folder.
fn external_payload_workspace(workspace: &LspWorkspace, external: &std::path::Path) -> std::path::PathBuf {
    let schemas = workspace.path().join("schemas");
    std::fs::create_dir(&schemas).unwrap();
    std::fs::write(workspace.path().join(".dmls.toml"), "[schema]\nstrict = true\n").unwrap();
    let payload = external.join("prompt.yaml");
    std::fs::write(&payload, "$schema:\n  model: string(required)\n").unwrap();
    let payload_reference = biscuit_file::to_portable_string(&payload);
    std::fs::write(
        schemas.join("prompt.trigger.yaml"),
        format!("kind: trigger-schema\nmatch:\n  prompt: string(required)\n$schema: \"{payload_reference}\"\n"),
    )
    .unwrap();
    payload
}

fn requires_model(diagnostics: &[Value]) -> bool {
    diagnostics
        .iter()
        .any(|diagnostic| diagnostic["message"].as_str().is_some_and(|message| message.contains("model")))
}

#[test]
fn schema_inputs_outside_the_workspace_are_watched_by_folder() {
    let workspace = LspWorkspace::new();
    let external = tempfile::tempdir().unwrap();
    external_payload_workspace(&workspace, external.path());
    let document = workspace.path().join("prompt.md");
    let text = "---\nprompt: hello\n---\n\nBody\n";
    let watched = json!({ "dynamicRegistration": true, "relativePatternSupport": true });
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(initialize_params(workspace.path(), "Watched Client", Some(watched)));
    open(&fixture, &uri(&document), text);
    assert!(requires_model(&latest(&mut fixture, &uri(&document))));

    let registrations = fixture.buffered_server_requests().to_vec();
    let schema_watchers: Vec<(String, String)> = registrations
        .iter()
        .filter(|request| request.method == "client/registerCapability")
        .flat_map(|request| request.params["registrations"].as_array().cloned().unwrap_or_default())
        .filter(|registration| registration["id"] == json!("dmls-watch-schema-inputs"))
        .flat_map(|registration| registration["registerOptions"]["watchers"].as_array().cloned().unwrap_or_default())
        .map(|watcher| {
            let pattern = &watcher["globPattern"];
            (pattern["baseUri"].as_str().unwrap_or_default().to_string(), pattern["pattern"].as_str().unwrap_or_default().to_string())
        })
        .collect();
    let external_name = external.path().file_name().unwrap().to_str().unwrap();
    assert!(
        schema_watchers.iter().any(|(_, pattern)| *pattern == format!("{external_name}/*")),
        "the payload's folder is watched: {schema_watchers:?}"
    );
    assert!(
        schema_watchers.iter().any(|(_, pattern)| pattern == "schemas/*"),
        "the schema root is watched: {schema_watchers:?}"
    );
    fixture.shutdown();
}

#[test]
fn without_folder_watches_a_save_rechecks_external_schema_inputs() {
    let workspace = LspWorkspace::new();
    let external = tempfile::tempdir().unwrap();
    let payload = external_payload_workspace(&workspace, external.path());
    let document = workspace.path().join("prompt.md");
    let text = "---\nprompt: hello\n---\n\nBody\n";
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(watched_params(workspace.path()));
    open(&fixture, &uri(&document), text);
    assert!(requires_model(&latest(&mut fixture, &uri(&document))));
    assert!(
        !fixture.buffered_server_requests().iter().any(|request| {
            request.params["registrations"][0]["id"] == json!("dmls-watch-schema-inputs")
        }),
        "no folder watch without relativePatternSupport"
    );

    // No watcher sees the external edit; the next save rechecks it.
    std::fs::write(&payload, "$schema:\n  model: string\n").unwrap();
    save(&fixture, &uri(&document));
    assert!(!requires_model(&latest(&mut fixture, &uri(&document))));
    fixture.shutdown();
}

#[test]
fn transclusion_cycles_are_found_through_unopened_documents() {
    let workspace = LspWorkspace::new();
    std::fs::write(workspace.path().join("b.md"), "::file ./c.md\n").unwrap();
    std::fs::write(workspace.path().join("c.md"), "::file ./a.md\n").unwrap();
    std::fs::write(workspace.path().join("unrelated.md"), "::file ./a.md\n").unwrap();
    let source = workspace.path().join("a.md");
    let text = "::file ./b.md\n";
    std::fs::write(&source, text).unwrap();
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(watched_params(workspace.path()));
    let before = work_counts();
    open(&fixture, &uri(&source), text);
    let diagnostics = codes(&latest(&mut fixture, &uri(&source)));
    assert!(diagnostics.contains(&"dm.transclusion.cycle".to_string()), "{diagnostics:?}");
    assert_eq!(work_since(before).parses, 3, "a.md, then b.md and c.md outward; not unrelated.md");
    fixture.shutdown();
}
