//! Schema slots in the editor: a document whose `$schema` holds a
//! `{{ load_data(…) }}` slot is resolved on the slot worker, never on a
//! request, and served from the worker's last installed snapshot.
//!
//! Boundary: the LSP protocol (an in-memory `Connection` driving the real
//! server, router, overlay, slot worker, and grant store), the filesystem
//! (data files and the grant store under the fixture's own `HOME`), and Git
//! (a repository built with `git` for the automatic-grant cell). F2, L1: no
//! terminal, no editor process, and no window.
//!
//! The data file lies outside any repository in every cell but the Git one,
//! so its read needs a grant: the persisted grant store under the fixture
//! `HOME`, or an in-memory grant given through `dmls.grantSlotRead`.

use std::path::{Path, PathBuf};

use crate::common;

use common::{LspFixture, LspWorkspace};
use serde_json::{Value, json};

const SCHEMA_DOC: &str = "---\n$schema:\n  os: enum({{ load_data(\"./os.yaml\", \"string[]\") }})\nos: Linux\n---\n\n# Doc\n";

/// A workspace whose `os.yaml` lists `members`; returns the canonical
/// workspace folder.
fn workspace_with_data(workspace: &LspWorkspace, members: &[&str]) -> PathBuf {
    let root = biscuit_file::canonicalize_simplified(workspace.path()).unwrap();
    write_members(&root.join("os.yaml"), members);
    root
}

fn write_members(path: &Path, members: &[&str]) {
    let text: String = members.iter().map(|member| format!("- {member}\n")).collect();
    std::fs::write(path, text).unwrap();
}

/// The fixture `HOME`'s grant store.
fn grant_store(root: &Path) -> PathBuf {
    root.join("home").join(".config").join("darkmatter").join("schema-read-policy")
}

fn allow_file_persisted(root: &Path, file: &Path) {
    let store = grant_store(root);
    std::fs::create_dir_all(store.parent().unwrap()).unwrap();
    std::fs::write(&store, format!("allow file {}\n", file.display())).unwrap();
}

fn snapshot(root: &Path) -> darkmatter::markdown::compose::RequestSnapshot {
    darkmatter::markdown::compose::RequestSnapshot::new(root).with_home(Some(root.join("home")))
}

/// Initialize params for a client that watches files, with relative
/// patterns, so the server registers its slot watches.
fn params(root: &Path) -> Value {
    let root_uri = url::Url::from_directory_path(root).unwrap();
    json!({
        "processId": null,
        "capabilities": {
            "general": { "positionEncodings": ["utf-8"] },
            "workspace": {
                "didChangeWatchedFiles": { "dynamicRegistration": true, "relativePatternSupport": true }
            },
            "textDocument": { "hover": { "contentFormat": ["markdown"] } }
        },
        "workspaceFolders": [ { "uri": root_uri.as_str(), "name": "slots" } ]
    })
}

fn uri(path: &Path) -> String {
    url::Url::from_file_path(path).unwrap().to_string()
}

fn open(fixture: &LspFixture, path: &Path, text: &str, version: i32) {
    let language = if path.extension().is_some_and(|extension| extension == "yaml") { "yaml" } else { "markdown" };
    fixture.notify(
        "textDocument/didOpen",
        json!({ "textDocument": { "uri": uri(path), "languageId": language, "version": version, "text": text } }),
    );
}

fn change(fixture: &LspFixture, path: &Path, text: &str, version: i32) {
    fixture.notify(
        "textDocument/didChange",
        json!({ "textDocument": { "uri": uri(path), "version": version }, "contentChanges": [ { "text": text } ] }),
    );
}

fn watched_change(fixture: &LspFixture, path: &Path, kind: u32) {
    fixture.notify("workspace/didChangeWatchedFiles", json!({ "changes": [ { "uri": uri(path), "type": kind } ] }));
}

fn codes(diagnostics: &[Value]) -> Vec<String> {
    diagnostics.iter().filter_map(|diagnostic| diagnostic["code"].as_str().map(str::to_string)).collect()
}

fn with_code<'d>(diagnostics: &'d [Value], code: &str) -> Vec<&'d Value> {
    diagnostics.iter().filter(|diagnostic| diagnostic["code"] == json!(code)).collect()
}

/// Opens `doc` and returns the diagnostics published once its first slot
/// job is installed.
fn open_and_resolve(fixture: &mut LspFixture, doc: &Path, text: &str) -> Vec<Value> {
    open(fixture, doc, text, 1);
    assert!(fixture.wait_for_slot_prepared(), "the first job is installed");
    fixture.flush_server();
    fixture.wait_for_diagnostics(&uri(doc))
}

/// The message of the schema problem on `os`, if any.
fn invalid_os(diagnostics: &[Value]) -> Option<String> {
    diagnostics
        .iter()
        .find(|diagnostic| diagnostic["data"]["instance_path"] == json!("/os"))
        .map(|diagnostic| diagnostic["message"].as_str().unwrap().to_string())
}

#[test]
fn a_persisted_grant_resolves_the_slot_and_the_frontmatter_is_judged_against_the_loaded_members() {
    let workspace = LspWorkspace::new();
    let root = workspace_with_data(&workspace, &["Linux", "macOS"]);
    allow_file_persisted(&root, &root.join("os.yaml"));
    let doc = root.join("doc.md");
    let mut fixture = LspFixture::start_with_snapshot(&workspace, snapshot(&root));
    fixture.initialize(params(&root));

    let diagnostics = open_and_resolve(&mut fixture, &doc, SCHEMA_DOC);
    assert!(
        with_code(&diagnostics, "dm.schema.slot_pending").is_empty()
            && with_code(&diagnostics, "dm.schema.slot_approval_required").is_empty(),
        "{diagnostics:#?}"
    );
    assert_eq!(invalid_os(&diagnostics), None, "Linux is a loaded member: {diagnostics:#?}");

    change(&fixture, &doc, &SCHEMA_DOC.replace("os: Linux", "os: BeOS"), 2);
    fixture.flush_server();
    let diagnostics = fixture.wait_for_diagnostics(&uri(&doc));
    let message = invalid_os(&diagnostics).unwrap_or_else(|| panic!("BeOS is rejected: {diagnostics:#?}"));
    assert!(message.contains("macOS"), "the problem names the loaded members: {message}");
}

#[test]
fn an_unapproved_read_is_reported_at_the_slot_and_the_frontmatter_is_never_judged_unconstrained() {
    let workspace = LspWorkspace::new();
    let root = workspace_with_data(&workspace, &["Linux"]);
    let doc = root.join("doc.md");
    // `title: 3` would be a problem under any schema; an unresolved
    // template validates nothing, so it is not reported.
    let text = SCHEMA_DOC.replace("os: Linux", "os: BeOS\ntitle: 3");
    let mut fixture = LspFixture::start_with_snapshot(&workspace, snapshot(&root));
    fixture.initialize(params(&root));

    let diagnostics = open_and_resolve(&mut fixture, &doc, &text);
    let approval = with_code(&diagnostics, "dm.schema.slot_approval_required");
    assert_eq!(approval.len(), 1, "{diagnostics:#?}");
    let slot_start = text.find("{{").unwrap();
    let line = text[..slot_start].matches('\n').count();
    assert_eq!(approval[0]["range"]["start"]["line"], json!(line), "ranged at the slot: {:#?}", approval[0]);
    assert_eq!(approval[0]["severity"], json!(2));
    let choices = approval[0]["data"]["choices"].as_array().expect("grant choices");
    assert!(choices.iter().any(|choice| choice["decision"] == json!("allow") && choice["scope"] == json!("persisted")));
    assert!(choices.iter().any(|choice| choice["decision"] == json!("deny")));
    assert_eq!(
        codes(&diagnostics).iter().filter(|code| !code.starts_with("dm.schema.slot_")).count(),
        0,
        "no schema verdict without a resolved schema: {diagnostics:#?}"
    );
    // Nothing prompts: the server asked the client nothing but watcher
    // registrations.
    assert!(
        fixture
            .buffered_server_requests()
            .iter()
            .all(|request| request.method == "client/registerCapability" || request.method == "client/unregisterCapability"
                || request.method == "window/workDoneProgress/create"),
        "{:?}",
        fixture.buffered_server_requests().iter().map(|request| &request.method).collect::<Vec<_>>()
    );
}

#[test]
fn a_quick_fix_runs_the_grant_command_which_writes_the_store_and_refreshes_the_document() {
    let workspace = LspWorkspace::new();
    let root = workspace_with_data(&workspace, &["Linux", "macOS"]);
    let doc = root.join("doc.md");
    let text = SCHEMA_DOC.replace("os: Linux", "os: BeOS");
    let mut fixture = LspFixture::start_with_snapshot(&workspace, snapshot(&root));
    fixture.initialize(params(&root));
    let diagnostics = open_and_resolve(&mut fixture, &doc, &text);
    let approval = with_code(&diagnostics, "dm.schema.slot_approval_required")[0].clone();

    let actions = fixture.request(
        "textDocument/codeAction",
        json!({
            "textDocument": { "uri": uri(&doc) },
            "range": approval["range"],
            "context": { "diagnostics": [approval] }
        }),
    );
    let actions = actions.result.expect("code actions");
    let persisted = actions
        .as_array()
        .unwrap()
        .iter()
        .find(|action| {
            action["command"]["arguments"][0]["decision"] == json!("allow")
                && action["command"]["arguments"][0]["kind"] == json!("file")
                && action["command"]["arguments"][0]["scope"] == json!("persisted")
        })
        .unwrap_or_else(|| panic!("a persisted file grant: {actions:#?}"))
        .clone();
    assert_eq!(persisted["kind"], json!("quickfix"));
    assert!(persisted.get("edit").is_none(), "a grant never edits the workspace");
    assert_eq!(persisted["command"]["command"], json!("dmls.grantSlotRead"));

    let response = fixture.request(
        "workspace/executeCommand",
        json!({ "command": "dmls.grantSlotRead", "arguments": persisted["command"]["arguments"] }),
    );
    assert!(response.error.is_none(), "{:?}", response.error);
    let store = std::fs::read_to_string(grant_store(&root)).expect("the grant store is written");
    assert_eq!(store, format!("allow file {}\n", root.join("os.yaml").display()));

    assert!(fixture.wait_for_slot_prepared(), "the grant refreshes the document");
    fixture.flush_server();
    let diagnostics = fixture.wait_for_diagnostics(&uri(&doc));
    assert!(with_code(&diagnostics, "dm.schema.slot_approval_required").is_empty(), "{diagnostics:#?}");
    let message = invalid_os(&diagnostics).unwrap_or_else(|| panic!("BeOS is rejected: {diagnostics:#?}"));
    assert!(message.contains("Linux"), "{message}");
}

#[test]
fn a_once_grant_resolves_for_this_server_process_and_is_never_written_to_the_store() {
    let workspace = LspWorkspace::new();
    let root = workspace_with_data(&workspace, &["Linux"]);
    let doc = root.join("doc.md");
    let mut fixture = LspFixture::start_with_snapshot(&workspace, snapshot(&root));
    fixture.initialize(params(&root));
    let diagnostics = open_and_resolve(&mut fixture, &doc, SCHEMA_DOC);
    assert_eq!(with_code(&diagnostics, "dm.schema.slot_approval_required").len(), 1, "{diagnostics:#?}");

    let response = fixture.request(
        "workspace/executeCommand",
        json!({
            "command": "dmls.grantSlotRead",
            "arguments": [ {
                "decision": "allow",
                "kind": "file",
                "target": root.join("os.yaml").display().to_string(),
                "scope": "once"
            } ]
        }),
    );
    assert!(response.error.is_none(), "{:?}", response.error);
    assert!(fixture.wait_for_slot_prepared());
    fixture.flush_server();
    let diagnostics = fixture.wait_for_diagnostics(&uri(&doc));
    assert!(codes(&diagnostics).iter().all(|code| !code.starts_with("dm.schema.slot_")), "{diagnostics:#?}");
    assert!(!grant_store(&root).exists(), "a once grant is kept in memory only");
}

#[test]
fn a_grant_command_with_an_invalid_target_is_refused_and_changes_nothing() {
    let workspace = LspWorkspace::new();
    let root = workspace_with_data(&workspace, &["Linux"]);
    let mut fixture = LspFixture::start_with_snapshot(&workspace, snapshot(&root));
    fixture.initialize(params(&root));
    for argument in [
        json!({ "decision": "allow", "kind": "file", "target": "relative/os.yaml", "scope": "persisted" }),
        json!({ "decision": "maybe", "kind": "file", "target": root.join("os.yaml").display().to_string(), "scope": "once" }),
        json!({ "decision": "allow", "kind": "file", "target": root.join("os.yaml").display().to_string(), "scope": "forever" }),
        json!("allow file /x"),
    ] {
        let response = fixture.request("workspace/executeCommand", json!({ "command": "dmls.grantSlotRead", "arguments": [argument] }));
        assert!(response.error.is_some(), "refused: {argument}");
    }
    let response = fixture.request("workspace/executeCommand", json!({ "command": "dmls.noSuchCommand", "arguments": [] }));
    assert!(response.error.is_some());
    assert!(!grant_store(&root).exists());
}

#[test]
fn an_unsaved_edit_of_the_data_file_refreshes_the_snapshot() {
    let workspace = LspWorkspace::new();
    let root = workspace_with_data(&workspace, &["Linux"]);
    let data = root.join("os.yaml");
    allow_file_persisted(&root, &data);
    let doc = root.join("doc.md");
    let text = SCHEMA_DOC.replace("os: Linux", "os: Haiku");
    let mut fixture = LspFixture::start_with_snapshot(&workspace, snapshot(&root));
    fixture.initialize(params(&root));
    let diagnostics = open_and_resolve(&mut fixture, &doc, &text);
    assert!(invalid_os(&diagnostics).is_some(), "Haiku is not listed yet: {diagnostics:#?}");

    // Opening the data file refreshes once; the unsaved edit, again.
    open(&fixture, &data, "- Linux\n", 1);
    assert!(fixture.wait_for_slot_prepared());
    change(&fixture, &data, "- Linux\n- Haiku\n", 2);
    assert!(fixture.wait_for_slot_prepared(), "the buffer edit refreshes the document");
    fixture.flush_server();
    let diagnostics = fixture.wait_for_diagnostics(&uri(&doc));
    assert_eq!(invalid_os(&diagnostics), None, "the unsaved member is loaded: {diagnostics:#?}");
    assert_eq!(std::fs::read_to_string(&data).unwrap(), "- Linux\n", "the disk file is untouched");
}

#[test]
fn a_saved_change_to_the_data_file_refreshes_the_snapshot_through_its_watch() {
    let workspace = LspWorkspace::new();
    let root = workspace_with_data(&workspace, &["Linux"]);
    let data = root.join("os.yaml");
    allow_file_persisted(&root, &data);
    let doc = root.join("doc.md");
    let text = SCHEMA_DOC.replace("os: Linux", "os: Haiku");
    let mut fixture = LspFixture::start_with_snapshot(&workspace, snapshot(&root));
    fixture.initialize(params(&root));
    open_and_resolve(&mut fixture, &doc, &text);

    // The server registered a watcher for the data file itself.
    let registration = fixture
        .buffered_server_requests()
        .iter()
        .rev()
        .find(|request| request.params["registrations"][0]["id"] == json!("dmls-schema-slot-watchers"))
        .unwrap_or_else(|| panic!("a slot watch registration"))
        .clone();
    let watchers = registration.params["registrations"][0]["registerOptions"]["watchers"].as_array().unwrap().clone();
    assert!(
        watchers.iter().any(|watcher| watcher["globPattern"]["pattern"] == json!("os.yaml")),
        "{watchers:#?}"
    );

    write_members(&data, &["Linux", "Haiku"]);
    watched_change(&fixture, &data, 2);
    assert!(fixture.wait_for_slot_prepared());
    fixture.flush_server();
    let diagnostics = fixture.wait_for_diagnostics(&uri(&doc));
    assert_eq!(invalid_os(&diagnostics), None, "{diagnostics:#?}");
}

#[test]
fn creating_a_missing_data_file_refreshes_the_snapshot() {
    let workspace = LspWorkspace::new();
    let root = biscuit_file::canonicalize_simplified(workspace.path()).unwrap();
    let data = root.join("os.yaml");
    allow_file_persisted(&root, &data);
    let doc = root.join("doc.md");
    let mut fixture = LspFixture::start_with_snapshot(&workspace, snapshot(&root));
    fixture.initialize(params(&root));
    let diagnostics = open_and_resolve(&mut fixture, &doc, SCHEMA_DOC);
    let failures: Vec<&Value> = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic["severity"] == json!(1) && diagnostic["source"] == json!("darkmatter.schema"))
        .collect();
    assert!(!failures.is_empty(), "the missing file fails the slot: {diagnostics:#?}");

    write_members(&data, &["Linux"]);
    watched_change(&fixture, &data, 1);
    assert!(fixture.wait_for_slot_prepared(), "the created file refreshes the document");
    fixture.flush_server();
    let diagnostics = fixture.wait_for_diagnostics(&uri(&doc));
    assert!(
        diagnostics.iter().all(|diagnostic| diagnostic["source"] != json!("darkmatter.schema")),
        "{diagnostics:#?}"
    );
}

#[test]
fn while_a_refresh_is_held_the_last_good_snapshot_is_served_marked_stale() {
    let workspace = LspWorkspace::new();
    let root = workspace_with_data(&workspace, &["Linux"]);
    let data = root.join("os.yaml");
    allow_file_persisted(&root, &data);
    let doc = root.join("doc.md");
    let text = SCHEMA_DOC.replace("os: Linux", "os: Haiku");
    let mut fixture = LspFixture::start_with_slot_gate(&workspace, snapshot(&root));
    fixture.initialize(params(&root));
    open(&fixture, &doc, &text, 1);
    fixture.release_slot_job();
    assert!(fixture.wait_for_slot_prepared());
    fixture.flush_server();
    let first = fixture.wait_for_diagnostics(&uri(&doc));
    assert!(invalid_os(&first).is_some());
    assert!(with_code(&first, "dm.schema.slot_pending").is_empty(), "{first:#?}");

    write_members(&data, &["Linux", "Haiku"]);
    watched_change(&fixture, &data, 2);
    fixture.wait_for_event(dmls::StartupEvent::SlotPreparing);
    fixture.round_trip();
    let held = fixture.wait_for_diagnostics(&uri(&doc));
    assert!(invalid_os(&held).is_some(), "the last-good verdict is kept: {held:#?}");
    let pending = with_code(&held, "dm.schema.slot_pending");
    assert_eq!(pending.len(), 1, "marked stale, never current: {held:#?}");
    assert_eq!(pending[0]["severity"], json!(3));
    let hover = fixture.request(
        "textDocument/hover",
        json!({ "textDocument": { "uri": uri(&doc) }, "position": { "line": 3, "character": 1 } }),
    );
    let value = hover.result.expect("hover")["contents"]["value"].as_str().unwrap_or_default().to_string();
    assert!(value.contains("stale"), "{value}");

    fixture.release_slot_job();
    assert!(fixture.wait_for_slot_prepared());
    fixture.flush_server();
    let refreshed = fixture.wait_for_diagnostics(&uri(&doc));
    assert_eq!(invalid_os(&refreshed), None, "{refreshed:#?}");
    assert!(with_code(&refreshed, "dm.schema.slot_pending").is_empty(), "{refreshed:#?}");
}

#[test]
fn a_held_slot_job_never_delays_a_completion_and_the_template_offers_its_literal_members() {
    let workspace = LspWorkspace::new();
    let root = workspace_with_data(&workspace, &["Linux"]);
    allow_file_persisted(&root, &root.join("os.yaml"));
    let doc = root.join("doc.md");
    let text = "---\n$schema:\n  os: enum(Haiku, {{ load_data(\"./os.yaml\", \"string[]\") }})\nos: \n---\n";
    let mut fixture = LspFixture::start_with_slot_gate(&workspace, snapshot(&root));
    fixture.initialize(params(&root));
    let before = darkmatter::work_counters::snapshot().get(darkmatter::work_counters::Work::SlotResolution);
    open(&fixture, &doc, text, 1);
    fixture.wait_for_event(dmls::StartupEvent::SlotPreparing);

    let completion = fixture.request(
        "textDocument/completion",
        json!({ "textDocument": { "uri": uri(&doc) }, "position": { "line": 3, "character": 4 } }),
    );
    let items = completion.result.expect("completion answered while the job is held");
    let labels: Vec<&str> = items["items"]
        .as_array()
        .or_else(|| items.as_array())
        .into_iter()
        .flatten()
        .filter_map(|item| item["label"].as_str())
        .collect();
    assert!(labels.contains(&"Haiku"), "the literal member: {labels:?}");
    assert!(!labels.contains(&"Linux"), "no slot member before the slot resolves: {labels:?}");
    let pending = fixture.wait_for_diagnostics(&uri(&doc));
    assert!(!with_code(&pending, "dm.schema.slot_pending").is_empty(), "{pending:#?}");
    assert_eq!(
        darkmatter::work_counters::snapshot().get(darkmatter::work_counters::Work::SlotResolution),
        before,
        "no request evaluated a slot"
    );

    fixture.release_slot_job();
    assert!(fixture.wait_for_slot_prepared());
    assert!(
        darkmatter::work_counters::snapshot().get(darkmatter::work_counters::Work::SlotResolution) > before,
        "control: the worker's evaluation is counted"
    );
    let completion = fixture.request(
        "textDocument/completion",
        json!({ "textDocument": { "uri": uri(&doc) }, "position": { "line": 3, "character": 4 } }),
    );
    let items = completion.result.expect("completion");
    let labels: Vec<&str> = items["items"]
        .as_array()
        .or_else(|| items.as_array())
        .into_iter()
        .flatten()
        .filter_map(|item| item["label"].as_str())
        .collect();
    assert!(labels.contains(&"Linux") && labels.contains(&"Haiku"), "{labels:?}");
}

#[test]
fn a_newer_edit_of_the_template_cancels_the_job_in_flight() {
    let workspace = LspWorkspace::new();
    let root = workspace_with_data(&workspace, &["Linux"]);
    std::fs::write(root.join("other.yaml"), "- Haiku\n").unwrap();
    let store = grant_store(&root);
    std::fs::create_dir_all(store.parent().unwrap()).unwrap();
    std::fs::write(&store, format!("allow dir {}\n", root.display())).unwrap();
    let doc = root.join("doc.md");
    let mut fixture = LspFixture::start_with_slot_gate(&workspace, snapshot(&root));
    fixture.initialize(params(&root));
    open(&fixture, &doc, SCHEMA_DOC, 1);
    fixture.wait_for_event(dmls::StartupEvent::SlotPreparing);

    change(&fixture, &doc, &SCHEMA_DOC.replace("./os.yaml", "./other.yaml"), 2);
    fixture.round_trip();
    fixture.release_slot_job();
    assert!(!fixture.wait_for_slot_prepared(), "the superseded job is not installed");
    fixture.release_slot_job();
    assert!(fixture.wait_for_slot_prepared(), "the newer job is");
    fixture.flush_server();
    let diagnostics = fixture.wait_for_diagnostics(&uri(&doc));
    let message = invalid_os(&diagnostics).unwrap_or_else(|| panic!("Linux is not in other.yaml: {diagnostics:#?}"));
    assert!(message.contains("Haiku"), "{message}");
}

#[test]
fn staging_a_change_to_an_automatically_granted_file_refreshes_it_through_the_index_watch() {
    let workspace = LspWorkspace::new();
    let root = workspace_with_data(&workspace, &["Linux"]);
    std::fs::write(root.join("doc.md"), SCHEMA_DOC).unwrap();
    git(&root, &["init", "-q", "-b", "main"]);
    git(&root, &["add", "-A"]);
    git(&root, &["-c", "user.name=t", "-c", "user.email=t@t", "-c", "commit.gpgsign=false", "commit", "-q", "-m", "fixture"]);
    let doc = root.join("doc.md");
    let mut fixture = LspFixture::start_with_snapshot(&workspace, snapshot(&root));
    fixture.initialize(params(&root));
    let diagnostics = open_and_resolve(&mut fixture, &doc, SCHEMA_DOC);
    assert!(
        codes(&diagnostics).iter().all(|code| !code.starts_with("dm.schema.slot_")),
        "a clean, committed file in the schema's repository is read without asking: {diagnostics:#?}"
    );
    let registration = fixture
        .buffered_server_requests()
        .iter()
        .rev()
        .find(|request| request.params["registrations"][0]["id"] == json!("dmls-schema-slot-watchers"))
        .expect("a slot watch registration")
        .clone();
    let patterns: Vec<&str> = registration.params["registrations"][0]["registerOptions"]["watchers"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|watcher| watcher["globPattern"]["pattern"].as_str())
        .collect();
    assert!(patterns.contains(&"index") && patterns.contains(&"HEAD"), "{patterns:?}");

    write_members(&root.join("os.yaml"), &["Linux", "Haiku"]);
    git(&root, &["add", "os.yaml"]);
    watched_change(&fixture, &root.join(".git").join("index"), 2);
    assert!(fixture.wait_for_slot_prepared(), "the index event refreshes the document");
    fixture.flush_server();
    let diagnostics = fixture.wait_for_diagnostics(&uri(&doc));
    assert_eq!(
        with_code(&diagnostics, "dm.schema.slot_approval_required").len(),
        1,
        "the staged file no longer matches HEAD, so it needs a grant: {diagnostics:#?}"
    );
}

fn git(root: &Path, args: &[&str]) {
    let status = std::process::Command::new("git")
        .args(args)
        .current_dir(root)
        .env("GIT_CONFIG_GLOBAL", if cfg!(windows) { "NUL" } else { "/dev/null" })
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_TERMINAL_PROMPT", "0")
        .status()
        .expect("git runs");
    assert!(status.success(), "git {args:?}");
}

#[test]
fn a_slot_syntax_error_is_reported_passively_without_a_slot_job() {
    let workspace = LspWorkspace::new();
    let root = workspace_with_data(&workspace, &["Linux"]);
    let doc = root.join("doc.md");
    let text = SCHEMA_DOC.replace("load_data(\"./os.yaml\", \"string[]\")", "load_data(\"./os.yaml\",");
    let mut fixture = LspFixture::start_with_snapshot(&workspace, snapshot(&root));
    fixture.initialize(params(&root));
    open(&fixture, &doc, &text, 1);
    fixture.flush_server();
    let diagnostics = fixture.wait_for_diagnostics(&uri(&doc));
    let errors: Vec<&Value> = diagnostics.iter().filter(|diagnostic| diagnostic["severity"] == json!(1)).collect();
    assert_eq!(errors.len(), 1, "{diagnostics:#?}");
    assert!(errors[0]["message"].as_str().unwrap().contains("slot"), "{:#?}", errors[0]);
    assert_eq!(errors[0]["range"]["start"]["line"], json!(2), "at the definition: {:#?}", errors[0]);
    assert!(invalid_os(&diagnostics).is_none(), "nothing is validated against a broken schema");
}

/// A client without file watchers gets the save-driven fallback: a save
/// re-`stat`s every path the installed resolution read.
#[test]
fn without_watchers_a_save_re_checks_the_slot_inputs_by_stat() {
    let workspace = LspWorkspace::new();
    let root = workspace_with_data(&workspace, &["Linux"]);
    let data = root.join("os.yaml");
    allow_file_persisted(&root, &data);
    let doc = root.join("doc.md");
    let text = SCHEMA_DOC.replace("os: Linux", "os: Haiku");
    let mut fixture = LspFixture::start_with_snapshot(&workspace, snapshot(&root));
    let root_uri = url::Url::from_directory_path(&root).unwrap();
    fixture.initialize(json!({
        "processId": null,
        "capabilities": { "general": { "positionEncodings": ["utf-8"] } },
        "workspaceFolders": [ { "uri": root_uri.as_str(), "name": "slots" } ]
    }));
    let diagnostics = open_and_resolve(&mut fixture, &doc, &text);
    assert!(invalid_os(&diagnostics).is_some());

    write_members(&data, &["Linux", "Haiku"]);
    fixture.notify("textDocument/didSave", json!({ "textDocument": { "uri": uri(&doc) } }));
    assert!(fixture.wait_for_slot_prepared(), "the save re-check refreshes the document");
    fixture.flush_server();
    let diagnostics = fixture.wait_for_diagnostics(&uri(&doc));
    assert_eq!(invalid_os(&diagnostics), None, "{diagnostics:#?}");
}

/// A workspace (a repository with no commit, so `data/os.yaml` gets no
/// automatic grant) whose `a.trigger.yaml` selects `topic: sandbox` and
/// declares `os` as an enum read from that file; returns the canonical root.
fn trigger_slot_workspace(workspace: &LspWorkspace) -> PathBuf {
    let root = biscuit_file::canonicalize_simplified(workspace.path()).unwrap();
    std::fs::create_dir_all(root.join(".git/objects")).unwrap();
    std::fs::create_dir_all(root.join(".git/refs/heads")).unwrap();
    std::fs::write(root.join(".git/HEAD"), "ref: refs/heads/main\n").unwrap();
    std::fs::write(root.join(".git/config"), "[core]\n\trepositoryformatversion = 0\n\tbare = false\n").unwrap();
    std::fs::create_dir_all(root.join("schemas")).unwrap();
    std::fs::write(
        root.join("schemas/a.trigger.yaml"),
        "kind: schema-trigger\nmatch:\n  topic: literal(sandbox; required)\n$schema:\n    os: enum({{ load_data(\"../data/os.yaml\", \"string[]\") }}; required)\n",
    )
    .unwrap();
    std::fs::create_dir_all(root.join("data")).unwrap();
    write_members(&root.join("data/os.yaml"), &["macOS", "Linux"]);
    std::fs::create_dir_all(root.join("docs")).unwrap();
    root
}

/// A trigger slot under a property the document redeclares is discarded:
/// the editor never schedules a slot job for it or reports it.
#[test]
fn a_trigger_slot_the_document_redeclares_is_never_pending_or_reported() {
    let workspace = LspWorkspace::new();
    let root = trigger_slot_workspace(&workspace);
    let doc = root.join("docs/doc.md");
    let text = "---\n$schema:\n    os: string\ntopic: sandbox\nos: Windows\n---\n\n# Doc\n";
    let mut fixture = LspFixture::start_with_snapshot(&workspace, snapshot(&root));
    fixture.initialize(params(&root));

    open(&fixture, &doc, text, 1);
    fixture.flush_server();
    let diagnostics = fixture.wait_for_diagnostics(&uri(&doc));
    assert!(codes(&diagnostics).iter().all(|code| !code.starts_with("dm.schema.")), "{diagnostics:#?}");
}

/// The control: with nothing replacing `os`, the trigger's unapproved read
/// is reported at its slot.
#[test]
fn a_selected_trigger_slot_with_no_override_reports_its_unapproved_read() {
    let workspace = LspWorkspace::new();
    let root = trigger_slot_workspace(&workspace);
    let doc = root.join("docs/doc.md");
    let mut fixture = LspFixture::start_with_snapshot(&workspace, snapshot(&root));
    fixture.initialize(params(&root));

    let diagnostics = open_and_resolve(&mut fixture, &doc, "---\ntopic: sandbox\nos: Windows\n---\n\n# Doc\n");
    assert_eq!(with_code(&diagnostics, "dm.schema.slot_approval_required").len(), 1, "{diagnostics:#?}");
}
