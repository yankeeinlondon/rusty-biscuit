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
    let language = match path.extension().and_then(|extension| extension.to_str()) {
        Some("md") | None => "markdown",
        Some(extension) => extension,
    };
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
    let current = completion_items(&mut fixture, &doc, 3, 4);
    assert_eq!(labels(&current), ["Linux"], "{current:#?}");
    assert!(current.iter().all(|item| item.get("detail").is_none()), "control: current: {current:#?}");

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
    let stale = completion_items(&mut fixture, &doc, 3, 4);
    assert!(
        !stale.is_empty() && stale.iter().all(|item| item["detail"].as_str().is_some_and(|detail| detail.contains(STALE_NOTE))),
        "R1-F5: every candidate says it is stale: {stale:#?}"
    );
    assert_eq!(without_detail(&stale), without_detail(&current), "labels, edits, and order are unchanged");

    fixture.release_slot_job();
    assert!(fixture.wait_for_slot_prepared());
    fixture.flush_server();
    let refreshed = fixture.wait_for_diagnostics(&uri(&doc));
    assert_eq!(invalid_os(&refreshed), None, "{refreshed:#?}");
    assert!(with_code(&refreshed, "dm.schema.slot_pending").is_empty(), "{refreshed:#?}");
    let refreshed = completion_items(&mut fixture, &doc, 3, 4);
    assert!(labels(&refreshed).contains(&"Haiku".to_string()), "{refreshed:#?}");
    assert!(refreshed.iter().all(|item| item.get("detail").is_none()), "control: current again: {refreshed:#?}");
}

/// The note a candidate served from a refreshing snapshot carries.
const STALE_NOTE: &str = "Schema slots stale";
/// The note a candidate offered before the template first resolves carries.
const PENDING_NOTE: &str = "Schema slots pending";

/// The completion items at `line`:`character` of `doc`, in response order.
fn completion_items(fixture: &mut LspFixture, doc: &Path, line: u32, character: u32) -> Vec<Value> {
    let response = fixture.request(
        "textDocument/completion",
        json!({ "textDocument": { "uri": uri(doc) }, "position": { "line": line, "character": character } }),
    );
    let result = response.result.expect("completion");
    result["items"].as_array().or_else(|| result.as_array()).cloned().unwrap_or_default()
}

fn labels(items: &[Value]) -> Vec<String> {
    items.iter().filter_map(|item| item["label"].as_str().map(str::to_string)).collect()
}

/// `items` with their `detail` removed, so a decorated list compares equal
/// to the undecorated one it came from.
fn without_detail(items: &[Value]) -> Vec<Value> {
    items
        .iter()
        .cloned()
        .map(|mut item| {
            item.as_object_mut().unwrap().remove("detail");
            item
        })
        .collect()
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

/// The LSP range (UTF-8 positions, which the fixture negotiates) of the
/// `nth` `{{ … }}` in `text`.
fn slot_range(text: &str, nth: usize) -> Value {
    let start = text.match_indices("{{").nth(nth).expect("a slot").0;
    let end = start + text[start..].find("}}").expect("a closed slot") + 2;
    let position = |offset: usize| {
        let before = &text[..offset];
        let line_start = before.rfind('\n').map_or(0, |at| at + 1);
        json!({ "line": before.matches('\n').count(), "character": offset - line_start })
    };
    json!({ "start": position(start), "end": position(end) })
}

/// R1-F3.9: an inline slot's record is ranged at the span the library
/// proved for it, the same one `md compose` reports. A double-quoted
/// carrier's escapes keep its authored text from equaling the expression,
/// and a second slot with the same expression is a different slot; each is
/// ranged at its own bytes.
#[test]
fn an_inline_slot_is_ranged_at_its_own_authored_bytes_through_a_quoted_carrier() {
    let workspace = LspWorkspace::new();
    let root = workspace_with_data(&workspace, &["Linux"]);
    let doc = root.join("doc.md");
    let text = "---\n$schema:\n  os: \"enum({{ load_data(\\\"./os.yaml\\\", \\\"string[]\\\") }})\"\n  arch: enum({{ load_data(\"./os.yaml\", \"string[]\") }})\nos: Linux\n---\n\n# Doc\n";
    let mut fixture = LspFixture::start_with_snapshot(&workspace, snapshot(&root));
    fixture.initialize(params(&root));

    let diagnostics = open_and_resolve(&mut fixture, &doc, text);
    let mut ranges: Vec<Value> =
        with_code(&diagnostics, "dm.schema.slot_approval_required").iter().map(|diagnostic| diagnostic["range"].clone()).collect();
    ranges.sort_by_key(|range| range["start"]["line"].as_u64());
    assert_eq!(ranges, vec![slot_range(text, 0), slot_range(text, 1)], "{diagnostics:#?}");
}

/// R1-F3.23: the unsaved buffer is the text positions are proven in. An
/// edit above the `$schema` moves the published range with it once the
/// edited buffer resolves, and a record of the earlier revision is never
/// placed at bytes it no longer names.
#[test]
fn an_unsaved_edit_above_the_schema_moves_the_slot_range() {
    let workspace = LspWorkspace::new();
    let root = workspace_with_data(&workspace, &["Linux"]);
    let doc = root.join("doc.md");
    let mut fixture = LspFixture::start_with_snapshot(&workspace, snapshot(&root));
    fixture.initialize(params(&root));
    let diagnostics = open_and_resolve(&mut fixture, &doc, SCHEMA_DOC);
    let approval = with_code(&diagnostics, "dm.schema.slot_approval_required");
    assert_eq!(approval.iter().map(|diagnostic| diagnostic["range"].clone()).collect::<Vec<_>>(), vec![slot_range(SCHEMA_DOC, 0)]);

    let edited = SCHEMA_DOC.replacen("---\n", "---\ntitle: Moved\nsummary: down\n", 1);
    change(&fixture, &doc, &edited, 2);
    assert!(fixture.wait_for_slot_prepared(), "the edited buffer resolves");
    fixture.flush_server();
    let diagnostics = fixture.wait_for_diagnostics(&uri(&doc));
    let approval = with_code(&diagnostics, "dm.schema.slot_approval_required");
    assert_eq!(approval.iter().map(|diagnostic| diagnostic["range"].clone()).collect::<Vec<_>>(), vec![slot_range(&edited, 0)], "{diagnostics:#?}");
}

/// R1-F3.27: a trigger slot is ranged at the document's frontmatter, and
/// its related information points at the slot in the trigger file.
#[test]
fn a_trigger_slot_points_at_its_position_in_the_trigger_file() {
    let workspace = LspWorkspace::new();
    let root = trigger_slot_workspace(&workspace);
    let doc = root.join("docs/doc.md");
    let trigger = std::fs::read_to_string(root.join("schemas/a.trigger.yaml")).unwrap();
    let mut fixture = LspFixture::start_with_snapshot(&workspace, snapshot(&root));
    fixture.initialize(params(&root));

    let diagnostics = open_and_resolve(&mut fixture, &doc, "---\ntopic: sandbox\nos: Windows\n---\n\n# Doc\n");
    let approval = with_code(&diagnostics, "dm.schema.slot_approval_required");
    assert_eq!(approval.len(), 1, "{diagnostics:#?}");
    let related = &approval[0]["relatedInformation"][0]["location"];
    assert_eq!(related["uri"], json!(uri(&root.join("schemas/a.trigger.yaml"))), "{:#?}", approval[0]);
    assert_eq!(related["range"]["start"], slot_range(&trigger, 0)["start"], "{:#?}", approval[0]);
}

/// R1-F5: every editor response projected from a template's snapshot says
/// so when that snapshot is not current: candidates from every completion
/// site, a body expression's hover, and a quick fix that inserts a value
/// the slots supplied. Each cell checks the current snapshot first (no
/// note), holds a refresh after the data changes (the note), and, where the
/// state can end, releases it (no note again).
mod stale_responses {
    use super::*;

    /// A gated server with `text` opened as `doc.md` over an `os.yaml`
    /// holding `before`, resolved once. Returns the fixture, the document,
    /// the data file, and the resolved diagnostics.
    fn resolved<'w>(workspace: &'w LspWorkspace, text: &str, before: &str) -> (LspFixture<'w>, PathBuf, PathBuf, Vec<Value>) {
        let root = biscuit_file::canonicalize_simplified(workspace.path()).unwrap();
        let data = root.join("os.yaml");
        std::fs::write(&data, before).unwrap();
        allow_file_persisted(&root, &data);
        let doc = root.join("doc.md");
        let mut fixture = LspFixture::start_with_slot_gate(workspace, snapshot(&root));
        fixture.initialize(params(&root));
        open(&fixture, &doc, text, 1);
        fixture.release_slot_job();
        assert!(fixture.wait_for_slot_prepared());
        fixture.flush_server();
        let diagnostics = fixture.wait_for_diagnostics(&uri(&doc));
        (fixture, doc, data, diagnostics)
    }

    /// Writes `after` to the data file, waits until its refresh is held,
    /// and returns the diagnostics published while it is.
    fn hold_refresh(fixture: &mut LspFixture, doc: &Path, data: &Path, after: &str) -> Vec<Value> {
        std::fs::write(data, after).unwrap();
        watched_change(fixture, data, 2);
        fixture.wait_for_event(dmls::StartupEvent::SlotPreparing);
        fixture.round_trip();
        fixture.wait_for_diagnostics(&uri(doc))
    }

    fn all_noted(items: &[Value], note: &str) -> bool {
        !items.is_empty() && items.iter().all(|item| item["detail"].as_str().is_some_and(|detail| detail.contains(note)))
    }

    fn hover_text(fixture: &mut LspFixture, doc: &Path, line: u32, character: u32) -> String {
        let hover = fixture.request(
            "textDocument/hover",
            json!({ "textDocument": { "uri": uri(doc) }, "position": { "line": line, "character": character } }),
        );
        hover.result.expect("hover")["contents"]["value"].as_str().unwrap_or_default().to_string()
    }

    #[test]
    fn an_array_item_candidate_from_a_refreshing_snapshot_is_marked_stale() {
        let workspace = LspWorkspace::new();
        let text = "---\n$schema:\n  os: enum({{ load_data(\"./os.yaml\", \"string[]\") }})[]\nos:\n  - \n---\n";
        let (mut fixture, doc, data, _) = resolved(&workspace, text, "- Linux\n");
        let current = completion_items(&mut fixture, &doc, 4, 4);
        assert_eq!(labels(&current), ["Linux"], "{current:#?}");
        assert!(current.iter().all(|item| item.get("detail").is_none()), "{current:#?}");

        hold_refresh(&mut fixture, &doc, &data, "- Linux\n- Haiku\n");
        let stale = completion_items(&mut fixture, &doc, 4, 4);
        assert!(all_noted(&stale, STALE_NOTE), "{stale:#?}");
        assert_eq!(without_detail(&stale), without_detail(&current));
    }

    #[test]
    fn a_union_value_candidate_from_a_refreshing_snapshot_is_marked_stale() {
        let workspace = LspWorkspace::new();
        let text = "---\n$schema:\n  os: enum({{ load_data(\"./os.yaml\", \"string[]\") }}) | number\nos: \n---\n";
        let (mut fixture, doc, data, _) = resolved(&workspace, text, "- Linux\n");
        let current = completion_items(&mut fixture, &doc, 3, 4);
        assert!(labels(&current).contains(&"Linux".to_string()), "{current:#?}");
        assert!(current.iter().all(|item| item.get("detail").is_none()), "{current:#?}");

        hold_refresh(&mut fixture, &doc, &data, "- Linux\n- Haiku\n");
        let stale = completion_items(&mut fixture, &doc, 3, 4);
        assert!(all_noted(&stale, STALE_NOTE), "{stale:#?}");
        assert_eq!(without_detail(&stale), without_detail(&current));
    }

    #[test]
    fn a_union_item_candidate_from_a_refreshing_snapshot_is_marked_stale() {
        let workspace = LspWorkspace::new();
        let text = "---\n$schema:\n  os: (enum({{ load_data(\"./os.yaml\", \"string[]\") }}) | number)[]\nos:\n  - \n---\n";
        let (mut fixture, doc, data, _) = resolved(&workspace, text, "- Linux\n");
        let current = completion_items(&mut fixture, &doc, 4, 4);
        assert!(labels(&current).contains(&"Linux".to_string()), "{current:#?}");
        assert!(current.iter().all(|item| item.get("detail").is_none()), "{current:#?}");

        hold_refresh(&mut fixture, &doc, &data, "- Linux\n- Haiku\n");
        let stale = completion_items(&mut fixture, &doc, 4, 4);
        assert!(all_noted(&stale, STALE_NOTE), "{stale:#?}");
        assert_eq!(without_detail(&stale), without_detail(&current));
    }

    #[test]
    fn a_suggestion_from_a_refreshing_snapshot_is_marked_stale() {
        let workspace = LspWorkspace::new();
        let text = "---\n$schema:\n  os: string(suggest({{ load_data(\"./os.yaml\", \"string[]\") }}))[]\nos:\n  - \n---\n";
        let (mut fixture, doc, data, _) = resolved(&workspace, text, "- Linux\n");
        let current = completion_items(&mut fixture, &doc, 4, 4);
        assert_eq!(labels(&current), ["Linux"], "{current:#?}");
        assert!(current.iter().all(|item| item.get("detail").is_none()), "{current:#?}");

        hold_refresh(&mut fixture, &doc, &data, "- Linux\n- Haiku\n");
        let stale = completion_items(&mut fixture, &doc, 4, 4);
        assert!(all_noted(&stale, STALE_NOTE), "{stale:#?}");
        assert_eq!(without_detail(&stale), without_detail(&current));
    }

    #[test]
    fn literal_candidates_offered_before_the_first_resolution_are_marked_pending() {
        let workspace = LspWorkspace::new();
        let root = workspace_with_data(&workspace, &["Linux"]);
        allow_file_persisted(&root, &root.join("os.yaml"));
        let doc = root.join("doc.md");
        let text = "---\n$schema:\n  os: enum(Haiku, {{ load_data(\"./os.yaml\", \"string[]\") }})\nos: \n---\n";
        let mut fixture = LspFixture::start_with_slot_gate(&workspace, snapshot(&root));
        fixture.initialize(params(&root));
        open(&fixture, &doc, text, 1);
        fixture.wait_for_event(dmls::StartupEvent::SlotPreparing);

        let pending = completion_items(&mut fixture, &doc, 3, 4);
        assert_eq!(labels(&pending), ["Haiku"], "{pending:#?}");
        assert!(all_noted(&pending, PENDING_NOTE), "the literal parts alone never look complete: {pending:#?}");

        fixture.release_slot_job();
        assert!(fixture.wait_for_slot_prepared());
        let resolved = completion_items(&mut fixture, &doc, 3, 4);
        assert!(labels(&resolved).contains(&"Linux".to_string()), "{resolved:#?}");
        assert!(resolved.iter().all(|item| item.get("detail").is_none()), "control: {resolved:#?}");
    }

    #[test]
    fn a_body_expression_hover_from_a_refreshing_snapshot_is_marked_stale() {
        let workspace = LspWorkspace::new();
        // `os` is unset, so the hover describes the schema's members rather
        // than a static value.
        let text = "---\n$schema:\n  os: enum({{ load_data(\"./os.yaml\", \"string[]\") }})\n---\n\n{{ os }}\n";
        let (mut fixture, doc, data, _) = resolved(&workspace, text, "- Linux\n");
        let current = hover_text(&mut fixture, &doc, 5, 3);
        assert!(current.contains("Linux") && !current.contains(STALE_NOTE), "{current}");

        hold_refresh(&mut fixture, &doc, &data, "- Linux\n- Haiku\n");
        let stale = hover_text(&mut fixture, &doc, 5, 3);
        assert!(stale.contains(STALE_NOTE), "{stale}");

        fixture.release_slot_job();
        assert!(fixture.wait_for_slot_prepared());
        let refreshed = hover_text(&mut fixture, &doc, 5, 3);
        assert!(refreshed.contains("Haiku") && !refreshed.contains(STALE_NOTE), "control: {refreshed}");
    }

    #[test]
    fn a_quick_fix_inserting_a_slot_value_from_a_refreshing_snapshot_is_marked_stale() {
        let workspace = LspWorkspace::new();
        // A missing required key is reported only in strict mode.
        std::fs::write(workspace.path().join(".dmls.toml"), "[schema]\nstrict = true\n").unwrap();
        let text = "---\n$schema:\n  os: literal({{ load_data(\"./os.yaml\", \"string[]\")[0] }}; required)\ntitle: Doc\n---\n";
        let (mut fixture, doc, data, diagnostics) = resolved(&workspace, text, "- Linux\n");
        let add_key = |fixture: &mut LspFixture, diagnostics: &[Value]| {
            let missing = with_code(diagnostics, "dm.schema.missing_required");
            assert_eq!(missing.len(), 1, "{diagnostics:#?}");
            let actions = fixture.request(
                "textDocument/codeAction",
                json!({
                    "textDocument": { "uri": uri(&doc) },
                    "range": missing[0]["range"],
                    "context": { "diagnostics": [missing[0]] }
                }),
            );
            let actions = actions.result.expect("code actions");
            actions
                .as_array()
                .unwrap()
                .iter()
                .find(|action| action["title"].as_str().is_some_and(|title| title.starts_with("Add required key")))
                .unwrap_or_else(|| panic!("an add-key fix: {actions:#?}"))
                .clone()
        };
        let current = add_key(&mut fixture, &diagnostics);
        assert!(current.to_string().contains("os: Linux"), "{current:#?}");
        assert_eq!(current["title"], json!("Add required key `os`"));

        let held = hold_refresh(&mut fixture, &doc, &data, "- macOS\n");
        let stale = add_key(&mut fixture, &held);
        assert!(stale.to_string().contains("os: Linux"), "the last-good value: {stale:#?}");
        assert!(stale["title"].as_str().unwrap().contains("stale"), "{stale:#?}");
    }
}

/// R1-F4: an open buffer is what a slot reads of any file it depends on,
/// whatever the file's extension. Each format row first rejects `Haiku`
/// against the data file on disk; the only edit then adds `Haiku` to the
/// unsaved buffer, and the completed refresh accepts it while the disk file
/// stays as it was.
mod unsaved_buffers {
    use super::*;
    use biscuit_file::DataFormat;

    /// A document whose `os` is an enum of what `expression` evaluates to,
    /// in a block carrier (the contracts hold `:` and quotes), set to `Haiku`.
    fn document(expression: &str) -> String {
        format!("---\n$schema:\n  os: |-\n    enum({{{{ {expression} }}}})\nos: Haiku\n---\n\n# Doc\n")
    }

    struct Case {
        file: &'static str,
        expression: &'static str,
        disk: &'static str,
        unsaved: &'static str,
    }

    /// A started server on a workspace holding `case.file` with its disk
    /// text (granted), whose document already rejected `Haiku`, and whose
    /// data file is open with its disk text.
    fn opened<'w>(workspace: &'w LspWorkspace, case: &Case) -> (LspFixture<'w>, PathBuf, PathBuf) {
        let root = biscuit_file::canonicalize_simplified(workspace.path()).unwrap();
        let data = root.join(case.file);
        std::fs::write(&data, case.disk).unwrap();
        allow_file_persisted(&root, &data);
        let doc = root.join("doc.md");
        let mut fixture = LspFixture::start_with_snapshot(workspace, snapshot(&root));
        fixture.initialize(params(&root));
        let text = document(case.expression);
        let diagnostics = open_and_resolve(&mut fixture, &doc, &text);
        assert!(invalid_os(&diagnostics).is_some(), "Haiku is not listed on disk: {diagnostics:#?}");
        open(&fixture, &data, case.disk, 1);
        assert!(fixture.wait_for_slot_prepared(), "opening the data file refreshes the document");
        (fixture, data, doc)
    }

    fn edit_accepts_haiku(fixture: &mut LspFixture, data: &Path, doc: &Path, case: &Case) {
        change(fixture, data, case.unsaved, 2);
        assert!(fixture.wait_for_slot_prepared(), "the buffer edit refreshes the document");
        fixture.flush_server();
        let diagnostics = fixture.wait_for_diagnostics(&uri(doc));
        assert!(codes(&diagnostics).iter().all(|code| !code.starts_with("dm.schema.slot_")), "{diagnostics:#?}");
        assert_eq!(invalid_os(&diagnostics), None, "the unsaved member is loaded: {diagnostics:#?}");
        assert_eq!(std::fs::read_to_string(data).unwrap(), case.disk, "the disk file is untouched");
    }

    fn unsaved_member_is_loaded(case: &Case) {
        let workspace = LspWorkspace::new();
        let (mut fixture, data, doc) = opened(&workspace, case);
        edit_accepts_haiku(&mut fixture, &data, &doc, case);
    }

    /// One `#[test]` per row, so nextest runs them in parallel; `FILES`
    /// lists the rows for [`every_loadable_format_has_a_row`].
    macro_rules! format_rows {
        ($($name:ident: $file:literal, $expression:literal, $disk:literal => $unsaved:literal;)*) => {
            $(
                #[test]
                fn $name() {
                    unsaved_member_is_loaded(&Case { file: $file, expression: $expression, disk: $disk, unsaved: $unsaved });
                }
            )*
            const FILES: &[&str] = &[$($file),*];
        };
    }

    format_rows! {
        yaml: "os.yaml", r#"load_data("./os.yaml", "string[]")"#, "- Linux\n" => "- Linux\n- Haiku\n";
        json: "os.json", r#"load_data("./os.json", "string[]")"#, "[\"Linux\"]\n" => "[\"Linux\", \"Haiku\"]\n";
        json5: "os.json5", r#"load_data("./os.json5", "string[]")"#, "['Linux',]\n" => "['Linux', 'Haiku',]\n";
        jsonl: "os.jsonl", r#"load_data("./os.jsonl", "string[]")"#, "\"Linux\"\n" => "\"Linux\"\n\"Haiku\"\n";
        toml: "os.toml", r#"load_data("./os.toml", "{ os: string[] }").os"#, "os = [\"Linux\"]\n" => "os = [\"Linux\", \"Haiku\"]\n";
        csv: "os.csv", r#"map(load_data("./os.csv", "{ os: string, kernel: string }[]"), row => row.os)"#, "os,kernel\nLinux,linux\n" => "os,kernel\nLinux,linux\nHaiku,haiku\n";
        tsv: "os.tsv", r#"map(load_data("./os.tsv", "{ os: string, kernel: string }[]"), row => row.os)"#, "os\tkernel\nLinux\tlinux\n" => "os\tkernel\nLinux\tlinux\nHaiku\thaiku\n";
        markdown: "os.md", r#"frontmatter("./os.md", "os")"#, "---\nos: [Linux]\n---\n" => "---\nos: [Linux, Haiku]\n---\n";
    }

    /// The extension of a format a slot can read a file in (`load_data`'s
    /// formats, and Markdown through `frontmatter`); a new format fails to
    /// compile here until it is given a row.
    fn readable_extension(format: DataFormat) -> Option<&'static str> {
        match format {
            DataFormat::Yaml => Some("yaml"),
            DataFormat::Json => Some("json"),
            DataFormat::Json5 => Some("json5"),
            DataFormat::Jsonl => Some("jsonl"),
            DataFormat::Toml => Some("toml"),
            DataFormat::Csv => Some("csv"),
            DataFormat::Tsv => Some("tsv"),
            DataFormat::Markdown => Some("md"),
            DataFormat::Pdf | DataFormat::Text => None,
        }
    }

    #[test]
    fn every_loadable_format_has_a_row() {
        let formats = [
            DataFormat::Yaml,
            DataFormat::Json,
            DataFormat::Json5,
            DataFormat::Jsonl,
            DataFormat::Toml,
            DataFormat::Csv,
            DataFormat::Tsv,
            DataFormat::Markdown,
            DataFormat::Pdf,
            DataFormat::Text,
        ];
        for extension in formats.into_iter().filter_map(readable_extension) {
            assert!(
                FILES.iter().any(|file| Path::new(file).extension().is_some_and(|found| found == extension)),
                "no unsaved-buffer row reads a .{extension} file"
            );
        }
    }

    /// Closing the buffer returns the slot to the file on disk.
    #[test]
    fn closing_the_buffer_reverts_to_the_disk_contents() {
        let case = Case {
            file: "os.toml",
            expression: r#"load_data("./os.toml", "{ os: string[] }").os"#,
            disk: "os = [\"Linux\"]\n",
            unsaved: "os = [\"Linux\", \"Haiku\"]\n",
        };
        let workspace = LspWorkspace::new();
        let (mut fixture, data, doc) = opened(&workspace, &case);
        edit_accepts_haiku(&mut fixture, &data, &doc, &case);

        fixture.notify("textDocument/didClose", json!({ "textDocument": { "uri": uri(&data) } }));
        assert!(fixture.wait_for_slot_prepared(), "closing the buffer refreshes the document");
        fixture.flush_server();
        let diagnostics = fixture.wait_for_diagnostics(&uri(&doc));
        assert!(invalid_os(&diagnostics).is_some(), "the disk file lists no Haiku: {diagnostics:#?}");
    }

    /// A contract's named-type import is read through its open buffer
    /// whatever its extension (a `.schema` file is a YAML schema document).
    #[test]
    fn an_imported_schema_with_an_unusual_extension_is_read_from_its_buffer() {
        let workspace = LspWorkspace::new();
        let root = biscuit_file::canonicalize_simplified(workspace.path()).unwrap();
        let (data, types) = (root.join("os.yaml"), root.join("os.schema"));
        std::fs::write(&data, "- Linux\n- Haiku\n").unwrap();
        let disk = "kind: schema\ntypes:\n  Os: enum(Linux)[]\n";
        std::fs::write(&types, disk).unwrap();
        let store = grant_store(&root);
        std::fs::create_dir_all(store.parent().unwrap()).unwrap();
        std::fs::write(&store, format!("allow file {}\nallow file {}\n", data.display(), types.display())).unwrap();
        let doc = root.join("doc.md");
        let mut fixture = LspFixture::start_with_snapshot(&workspace, snapshot(&root));
        fixture.initialize(params(&root));
        let text = document(r#"load_data("./os.yaml", "Os@./os.schema")"#);
        let diagnostics = open_and_resolve(&mut fixture, &doc, &text);
        assert!(
            invalid_os(&diagnostics).is_some() || !codes(&diagnostics).is_empty(),
            "the disk contract has no Haiku: {diagnostics:#?}"
        );

        open(&fixture, &types, disk, 1);
        assert!(fixture.wait_for_slot_prepared());
        change(&fixture, &types, "kind: schema\ntypes:\n  Os: enum(Linux, Haiku)[]\n", 2);
        assert!(fixture.wait_for_slot_prepared(), "the import's buffer edit refreshes the document");
        fixture.flush_server();
        let diagnostics = fixture.wait_for_diagnostics(&uri(&doc));
        assert!(codes(&diagnostics).is_empty(), "the unsaved contract admits Haiku: {diagnostics:#?}");
    }

    /// An unsaved buffer that differs from the committed file is dirty, so
    /// the automatic grant no longer covers it.
    #[test]
    fn a_dirty_buffer_loses_the_automatic_grant() {
        let workspace = LspWorkspace::new();
        let root = biscuit_file::canonicalize_simplified(workspace.path()).unwrap();
        let data = root.join("os.toml");
        std::fs::write(&data, "os = [\"Linux\"]\n").unwrap();
        let text = document(r#"load_data("./os.toml", "{ os: string[] }").os"#);
        let doc = root.join("doc.md");
        std::fs::write(&doc, &text).unwrap();
        git(&root, &["init", "-q", "-b", "main"]);
        git(&root, &["add", "-A"]);
        git(&root, &["-c", "user.name=t", "-c", "user.email=t@t", "-c", "commit.gpgsign=false", "commit", "-q", "-m", "fixture"]);
        let mut fixture = LspFixture::start_with_snapshot(&workspace, snapshot(&root));
        fixture.initialize(params(&root));
        let diagnostics = open_and_resolve(&mut fixture, &doc, &text);
        assert!(
            codes(&diagnostics).iter().all(|code| !code.starts_with("dm.schema.slot_")),
            "the clean, committed file is granted automatically: {diagnostics:#?}"
        );

        open(&fixture, &data, "os = [\"Linux\", \"Haiku\"]\n", 1);
        assert!(fixture.wait_for_slot_prepared(), "opening the data file refreshes the document");
        fixture.flush_server();
        let diagnostics = fixture.wait_for_diagnostics(&uri(&doc));
        assert_eq!(
            with_code(&diagnostics, "dm.schema.slot_approval_required").len(),
            1,
            "the unsaved text is not the committed file, so it needs a grant: {diagnostics:#?}"
        );
    }
}

/// A slot calling a captured-context function resolves on the worker: the
/// worker's session captures the group the slot's call reads from the
/// document's context when the slot runs. The defaults are typed on any
/// host (an address list's length, a boolean, `""` outside a repository);
/// the clock and `PATH` probes are the controls.
mod captured_functions {
    use super::*;

    fn resolves_on_the_worker(declaration: &str) {
        let workspace = LspWorkspace::new();
        let root = biscuit_file::canonicalize_simplified(workspace.path()).unwrap();
        let doc = root.join("doc.md");
        let text = format!("---\n$schema:\n  value: \"{declaration}\"\n---\n\n# Doc\n");
        let mut fixture = LspFixture::start_with_snapshot(&workspace, snapshot(&root));
        fixture.initialize(params(&root));
        let diagnostics = open_and_resolve(&mut fixture, &doc, &text);
        assert!(codes(&diagnostics).iter().all(|code| !code.starts_with("dm.schema.slot_")), "{diagnostics:#?}");
    }

    /// One `#[test]` per function, so each fails on its own.
    macro_rules! function_rows {
        ($($name:ident: $declaration:literal;)*) => {
            $(
                #[test]
                fn $name() {
                    resolves_on_the_worker($declaration);
                }
            )*
        };
    }

    function_rows! {
        ipv4: "number(default({{ length(ipv4()) }}))";
        ipv6: "number(default({{ length(ipv6()) }}))";
        has_agentic_cli: "boolean(default({{ has_agentic_cli('kimi') }}))";
        package: "string(default({{ package('./child') }}))";
        package_area: "string(default({{ package_area('./child') }}))";
        is_today: "boolean(default({{ is_today('2024-06-16') }}))";
        has_command: "boolean(default({{ has_command('git') }}))";
    }
}

/// Template types in the editor: a slot that supplies a property's whole
/// type (`status: "{{ … }}"`) is pending until the slot worker resolves it.
/// While it is, hover and completion name the property and say its type is
/// pending, claiming no requiredness, values, or nested keys, and the
/// frontmatter gets no verdict; static siblings keep everything they offer.
/// The returned definition's own reads (its imports, and an import's missing
/// candidate) are watched like any slot read, so an unsaved edit or a
/// created file re-resolves the document.
mod template_types {
    use super::*;

    /// `status` is a template type read from `status.yaml` (an object whose
    /// `type` is the definition), marked `(required)`; `kind` is static.
    const TEMPLATE_DOC: &str = "---\n$schema:\n  status: \"{{ load_data('./status.yaml', '{ type: string }').type }}(required)\"\n  kind: enum(note, task)\nstatus: bogus\nkind: \n---\n";
    /// The line of `status: bogus` and of `kind: `.
    const STATUS_LINE: u32 = 4;
    const KIND_LINE: u32 = 5;

    /// A workspace whose `status.yaml` returns `definition`, with its folder
    /// granted in the persisted store; returns the canonical root.
    fn granted_workspace(workspace: &LspWorkspace, definition: &str) -> PathBuf {
        let root = biscuit_file::canonicalize_simplified(workspace.path()).unwrap();
        write_type(&root, definition);
        allow_dir_persisted(&root);
        root
    }

    fn write_type(root: &Path, definition: &str) {
        std::fs::write(root.join("status.yaml"), format!("type: \"{definition}\"\n")).unwrap();
    }

    fn allow_dir_persisted(root: &Path) {
        let store = grant_store(root);
        std::fs::create_dir_all(store.parent().unwrap()).unwrap();
        std::fs::write(&store, format!("allow dir {}\n", root.display())).unwrap();
    }

    fn hover_text(fixture: &mut LspFixture, doc: &Path, line: u32, character: u32) -> String {
        let hover = fixture.request(
            "textDocument/hover",
            json!({ "textDocument": { "uri": uri(doc) }, "position": { "line": line, "character": character } }),
        );
        hover.result.expect("hover")["contents"]["value"].as_str().unwrap_or_default().to_string()
    }

    /// The diagnostic on the value at `pointer`, if any: a verdict on it.
    fn verdict_at<'d>(diagnostics: &'d [Value], pointer: &str) -> Option<&'d Value> {
        diagnostics.iter().find(|diagnostic| diagnostic["data"]["instance_path"] == json!(pointer))
    }

    #[test]
    fn a_pending_template_type_claims_nothing_and_the_resolved_type_replaces_it() {
        let workspace = LspWorkspace::new();
        let root = granted_workspace(&workspace, "enum(draft, published)");
        let doc = root.join("doc.md");
        let mut fixture = LspFixture::start_with_slot_gate(&workspace, snapshot(&root));
        fixture.initialize(params(&root));
        open(&fixture, &doc, TEMPLATE_DOC, 1);
        fixture.wait_for_event(dmls::StartupEvent::SlotPreparing);

        // Pending: the property, "type pending", nothing else claimed.
        let hover = hover_text(&mut fixture, &doc, STATUS_LINE, 1);
        assert!(hover.contains("**`status`**") && hover.contains("type pending"), "{hover}");
        assert!(!hover.contains("Required") && !hover.contains("Values") && !hover.contains("Default"), "{hover}");
        assert!(hover.contains(PENDING_NOTE), "{hover}");
        let values = completion_items(&mut fixture, &doc, STATUS_LINE, 8);
        assert!(values.is_empty(), "no value is invented for a pending type: {values:#?}");
        // A static sibling is unaffected.
        let kinds = completion_items(&mut fixture, &doc, KIND_LINE, 6);
        assert_eq!(labels(&kinds), ["note", "task"], "{kinds:#?}");
        let kind_hover = hover_text(&mut fixture, &doc, KIND_LINE, 1);
        assert!(kind_hover.contains("_note_") && kind_hover.contains("_task_"), "{kind_hover}");
        let pending = fixture.wait_for_diagnostics(&uri(&doc));
        assert!(!with_code(&pending, "dm.schema.slot_pending").is_empty(), "{pending:#?}");
        assert!(verdict_at(&pending, "/status").is_none(), "no verdict while pending: {pending:#?}");
        assert!(verdict_at(&pending, "/kind").is_none(), "nothing is judged while pending: {pending:#?}");

        fixture.release_slot_job();
        assert!(fixture.wait_for_slot_prepared());
        fixture.flush_server();
        let resolved = fixture.wait_for_diagnostics(&uri(&doc));
        assert!(with_code(&resolved, "dm.schema.slot_pending").is_empty(), "{resolved:#?}");
        let message = verdict_at(&resolved, "/status")
            .and_then(|diagnostic| diagnostic["message"].as_str())
            .unwrap_or_else(|| panic!("bogus is judged by the resolved type: {resolved:#?}"));
        assert!(message.contains("draft") && message.contains("published"), "{message}");
        let hover = hover_text(&mut fixture, &doc, STATUS_LINE, 1);
        assert!(!hover.contains("type pending") && !hover.contains(PENDING_NOTE), "{hover}");
        assert!(hover.contains("Required") && hover.contains("_draft_"), "{hover}");
        let values = completion_items(&mut fixture, &doc, STATUS_LINE, 8);
        assert_eq!(labels(&values), ["draft", "published"], "{values:#?}");
    }

    #[test]
    fn key_completion_marks_a_pending_template_type_and_never_its_suffix_requiredness() {
        let workspace = LspWorkspace::new();
        let root = granted_workspace(&workspace, "enum(draft, published)");
        let doc = root.join("doc.md");
        // No `status` key yet; the cursor is on the empty last line.
        let text = TEMPLATE_DOC.replace("status: bogus\nkind: \n", "kind: note\n\n");
        let mut fixture = LspFixture::start_with_slot_gate(&workspace, snapshot(&root));
        fixture.initialize(params(&root));
        open(&fixture, &doc, &text, 1);
        fixture.wait_for_event(dmls::StartupEvent::SlotPreparing);

        let keys = completion_items(&mut fixture, &doc, 5, 0);
        let status = keys.iter().find(|item| item["label"] == json!("status")).unwrap_or_else(|| panic!("{keys:#?}"));
        let detail = status["detail"].as_str().unwrap_or_default();
        assert!(detail.contains("type pending") && !detail.contains("(required)"), "{status:#?}");

        fixture.release_slot_job();
        assert!(fixture.wait_for_slot_prepared());
        fixture.flush_server();
        let keys = completion_items(&mut fixture, &doc, 5, 0);
        let status = keys.iter().find(|item| item["label"] == json!("status")).unwrap_or_else(|| panic!("{keys:#?}"));
        assert_eq!(status["detail"], json!("(required)"), "control: resolved: {status:#?}");
    }

    /// A pending arm of a property-level union hides nothing its static arm
    /// offers.
    #[test]
    fn a_static_union_arm_beside_a_pending_template_type_keeps_its_values_and_hover() {
        let workspace = LspWorkspace::new();
        let root = granted_workspace(&workspace, "enum(draft, published)");
        let doc = root.join("doc.md");
        let text = "---\n$schema:\n  status: \"{{ load_data('./status.yaml', '{ type: string }').type }} | boolean\"\nstatus: \n---\n";
        let mut fixture = LspFixture::start_with_slot_gate(&workspace, snapshot(&root));
        fixture.initialize(params(&root));
        open(&fixture, &doc, text, 1);
        fixture.wait_for_event(dmls::StartupEvent::SlotPreparing);

        let values = completion_items(&mut fixture, &doc, 3, 8);
        assert_eq!(labels(&values), ["true", "false"], "{values:#?}");
        let hover = hover_text(&mut fixture, &doc, 3, 1);
        assert!(hover.contains("type pending") && hover.contains("**boolean**"), "{hover}");

        fixture.release_slot_job();
        assert!(fixture.wait_for_slot_prepared());
        fixture.flush_server();
        let values = labels(&completion_items(&mut fixture, &doc, 3, 8));
        for expected in ["draft", "published", "true", "false"] {
            assert!(values.contains(&expected.to_string()), "{expected} once resolved: {values:?}");
        }
    }

    /// A template type in a grouped union nested as a union arm is pending
    /// in hover and diagnostics, then resolved and judged like a top-level
    /// one (before nested grouped unions were substituted, resolution
    /// failed with one slot unresolved).
    #[test]
    fn a_template_type_in_a_nested_grouped_union_is_pending_then_resolved() {
        let workspace = LspWorkspace::new();
        let root = granted_workspace(&workspace, "enum(draft, published)");
        let doc = root.join("doc.md");
        let text = "---\n$schema:\n  status: \"number | ({{ load_data('./status.yaml', '{ type: string }').type }} | boolean)[]\"\nstatus: [bogus]\n---\n";
        let mut fixture = LspFixture::start_with_slot_gate(&workspace, snapshot(&root));
        fixture.initialize(params(&root));
        open(&fixture, &doc, text, 1);
        fixture.wait_for_event(dmls::StartupEvent::SlotPreparing);

        // Hover describes a property-level union by its first arm; the
        // pending slot shows as the schema's pending note, never a presence.
        let hover = hover_text(&mut fixture, &doc, 3, 1);
        assert!(hover.contains(PENDING_NOTE) && !hover.contains("Required"), "{hover}");
        let pending = fixture.wait_for_diagnostics(&uri(&doc));
        assert!(!with_code(&pending, "dm.schema.slot_pending").is_empty(), "{pending:#?}");

        fixture.release_slot_job();
        assert!(fixture.wait_for_slot_prepared());
        fixture.flush_server();
        let resolved = fixture.wait_for_diagnostics(&uri(&doc));
        assert!(with_code(&resolved, "dm.schema.slot_pending").is_empty(), "{resolved:#?}");
        assert!(with_code(&resolved, "dm.schema.slot_failed").is_empty(), "{resolved:#?}");
        let judged = resolved
            .iter()
            .any(|diagnostic| diagnostic["data"]["instance_path"].as_str().is_some_and(|path| path.starts_with("/status")));
        assert!(judged, "bogus is judged by the resolved type: {resolved:#?}");
        let hover = hover_text(&mut fixture, &doc, 3, 1);
        assert!(!hover.contains(PENDING_NOTE), "{hover}");
    }

    #[test]
    fn a_newer_edit_cancels_the_template_type_job_in_flight() {
        let workspace = LspWorkspace::new();
        let root = granted_workspace(&workspace, "enum(draft, published)");
        std::fs::write(root.join("other.yaml"), "type: \"enum(bogus)\"\n").unwrap();
        let doc = root.join("doc.md");
        let mut fixture = LspFixture::start_with_slot_gate(&workspace, snapshot(&root));
        fixture.initialize(params(&root));
        open(&fixture, &doc, TEMPLATE_DOC, 1);
        fixture.wait_for_event(dmls::StartupEvent::SlotPreparing);

        change(&fixture, &doc, &TEMPLATE_DOC.replace("./status.yaml", "./other.yaml"), 2);
        fixture.round_trip();
        fixture.release_slot_job();
        assert!(!fixture.wait_for_slot_prepared(), "the superseded job publishes nothing");
        fixture.release_slot_job();
        assert!(fixture.wait_for_slot_prepared(), "the newer job is installed");
        fixture.flush_server();
        let diagnostics = fixture.wait_for_diagnostics(&uri(&doc));
        assert!(codes(&diagnostics).iter().all(|code| !code.starts_with("dm.schema.slot_")), "{diagnostics:#?}");
        assert!(verdict_at(&diagnostics, "/status").is_none(), "bogus is a member of other.yaml's type: {diagnostics:#?}");
    }

    /// A failed refresh first serves the last-good resolution marked stale,
    /// then the failure: an error at the slot, no verdict, and the template's
    /// literal parts with the type pending again.
    #[test]
    fn a_failed_refresh_serves_the_last_good_type_marked_stale_then_the_failure() {
        let workspace = LspWorkspace::new();
        let root = granted_workspace(&workspace, "enum(draft, published)");
        let doc = root.join("doc.md");
        let mut fixture = LspFixture::start_with_slot_gate(&workspace, snapshot(&root));
        fixture.initialize(params(&root));
        open(&fixture, &doc, TEMPLATE_DOC, 1);
        fixture.release_slot_job();
        assert!(fixture.wait_for_slot_prepared());
        fixture.flush_server();
        let first = fixture.wait_for_diagnostics(&uri(&doc));
        assert!(verdict_at(&first, "/status").is_some(), "{first:#?}");

        write_type(&root, "enum(");
        watched_change(&fixture, &root.join("status.yaml"), 2);
        fixture.wait_for_event(dmls::StartupEvent::SlotPreparing);
        fixture.round_trip();
        let held = fixture.wait_for_diagnostics(&uri(&doc));
        assert!(verdict_at(&held, "/status").is_some(), "the last-good verdict is kept: {held:#?}");
        assert_eq!(with_code(&held, "dm.schema.slot_pending").len(), 1, "marked stale: {held:#?}");
        let hover = hover_text(&mut fixture, &doc, STATUS_LINE, 1);
        assert!(hover.contains("_draft_") && hover.contains(STALE_NOTE), "{hover}");
        let stale = completion_items(&mut fixture, &doc, STATUS_LINE, 8);
        assert!(
            !stale.is_empty() && stale.iter().all(|item| item["detail"].as_str().is_some_and(|detail| detail.contains(STALE_NOTE))),
            "{stale:#?}"
        );

        fixture.release_slot_job();
        assert!(fixture.wait_for_slot_prepared());
        fixture.flush_server();
        let failed = fixture.wait_for_diagnostics(&uri(&doc));
        let errors = with_code(&failed, "dm.schema.slot_not_a_type");
        assert_eq!(errors.len(), 1, "{failed:#?}");
        assert_eq!(errors[0]["severity"], json!(1), "{failed:#?}");
        assert!(verdict_at(&failed, "/status").is_none(), "nothing is judged against a failed type: {failed:#?}");
        // The failed template offers its literal parts again: the type is
        // pending, and nothing of the last-good type is left.
        let hover = hover_text(&mut fixture, &doc, STATUS_LINE, 1);
        assert!(hover.contains("type pending") && hover.contains(PENDING_NOTE), "{hover}");
        assert!(!hover.contains("_draft_") && !hover.contains("Required"), "{hover}");
        let values = completion_items(&mut fixture, &doc, STATUS_LINE, 8);
        assert!(values.is_empty(), "{values:#?}");
        let kinds = completion_items(&mut fixture, &doc, KIND_LINE, 6);
        assert_eq!(labels(&kinds), ["note", "task"], "the static sibling is kept: {kinds:#?}");
    }

    /// `owner`'s type is `Person`, imported by the returned definition from
    /// `types.yaml`; `role` admits only what that file lists.
    const IMPORT_DOC: &str = "---\n$schema:\n  owner: \"{{ 'Person@./types.yaml' }}\"\nowner:\n  role: guest\n---\n";

    fn types_file(roles: &str) -> String {
        format!("kind: schema\ntypes:\n  Person:\n    role: enum({roles})\n")
    }

    #[test]
    fn an_unsaved_edit_to_a_returned_import_re_resolves_the_document() {
        let workspace = LspWorkspace::new();
        let root = biscuit_file::canonicalize_simplified(workspace.path()).unwrap();
        let types = root.join("types.yaml");
        let disk = types_file("admin");
        std::fs::write(&types, &disk).unwrap();
        allow_dir_persisted(&root);
        let doc = root.join("doc.md");
        let mut fixture = LspFixture::start_with_snapshot(&workspace, snapshot(&root));
        fixture.initialize(params(&root));
        let diagnostics = open_and_resolve(&mut fixture, &doc, IMPORT_DOC);
        assert!(verdict_at(&diagnostics, "/owner/role").is_some(), "guest is not on disk: {diagnostics:#?}");

        open(&fixture, &types, &disk, 1);
        assert!(fixture.wait_for_slot_prepared(), "opening the import refreshes the document");
        change(&fixture, &types, &types_file("admin, guest"), 2);
        assert!(fixture.wait_for_slot_prepared(), "the unsaved edit refreshes the document");
        fixture.flush_server();
        let diagnostics = fixture.wait_for_diagnostics(&uri(&doc));
        assert!(codes(&diagnostics).iter().all(|code| !code.starts_with("dm.schema.slot_")), "{diagnostics:#?}");
        assert!(verdict_at(&diagnostics, "/owner/role").is_none(), "the unsaved import is read: {diagnostics:#?}");
        assert_eq!(std::fs::read_to_string(&types).unwrap(), disk, "the disk file is untouched");
    }

    /// An unsaved buffer at an implicit relative import's first candidate
    /// (beside the document) is its file: the import reads it with a grant
    /// for the document's folder alone, never reaching the repository-root
    /// file after it.
    #[test]
    fn a_new_unsaved_file_at_a_returned_import_first_candidate_is_its_file() {
        let workspace = LspWorkspace::new();
        let root = biscuit_file::canonicalize_simplified(workspace.path()).unwrap();
        git(&root, &["init", "-q"]);
        std::fs::create_dir_all(root.join("lib")).unwrap();
        std::fs::create_dir_all(root.join("doc")).unwrap();
        std::fs::write(root.join("lib/types.yaml"), types_file("admin")).unwrap();
        allow_dir_persisted(&root.join("doc"));
        let doc = root.join("doc/doc.md");
        let mut fixture = LspFixture::start_with_snapshot(&workspace, snapshot(&root));
        fixture.initialize(params(&root));
        open(&fixture, &root.join("doc/lib/types.yaml"), &types_file("admin, guest"), 1);

        let diagnostics =
            open_and_resolve(&mut fixture, &doc, &IMPORT_DOC.replace("Person@./types.yaml", "Person@lib/types.yaml"));

        assert!(codes(&diagnostics).iter().all(|code| !code.starts_with("dm.schema.slot_")), "{diagnostics:#?}");
        assert!(verdict_at(&diagnostics, "/owner/role").is_none(), "the unsaved file admits guest: {diagnostics:#?}");
        assert!(!root.join("doc/lib/types.yaml").exists(), "the buffer is never saved");
    }

    #[test]
    fn creating_a_missing_returned_import_re_resolves_the_document() {
        let workspace = LspWorkspace::new();
        let root = biscuit_file::canonicalize_simplified(workspace.path()).unwrap();
        allow_dir_persisted(&root);
        let doc = root.join("doc.md");
        let mut fixture = LspFixture::start_with_snapshot(&workspace, snapshot(&root));
        fixture.initialize(params(&root));
        let missing = open_and_resolve(&mut fixture, &doc, IMPORT_DOC);
        let failures: Vec<&Value> = missing.iter().filter(|diagnostic| diagnostic["severity"] == json!(1)).collect();
        assert!(!failures.is_empty(), "a missing import fails the type: {missing:#?}");
        assert!(verdict_at(&missing, "/owner/role").is_none(), "{missing:#?}");

        let types = root.join("types.yaml");
        std::fs::write(&types, types_file("admin")).unwrap();
        watched_change(&fixture, &types, 1);
        assert!(fixture.wait_for_slot_prepared(), "the created candidate refreshes the document");
        fixture.flush_server();
        let diagnostics = fixture.wait_for_diagnostics(&uri(&doc));
        assert!(codes(&diagnostics).iter().all(|code| !code.starts_with("dm.schema.slot_")), "{diagnostics:#?}");
        let message = verdict_at(&diagnostics, "/owner/role")
            .and_then(|diagnostic| diagnostic["message"].as_str())
            .unwrap_or_else(|| panic!("guest is judged by the created import: {diagnostics:#?}"));
        assert!(message.contains("admin"), "{message}");
    }
}
/// A slot whose resolution failed still offers the template's literal
/// parts (said to be pending), never the failed list and never nothing.
#[test]
fn a_failed_slot_still_offers_the_template_literal_members() {
    let workspace = LspWorkspace::new();
    let root = biscuit_file::canonicalize_simplified(workspace.path()).unwrap();
    let data = root.join("os.yaml");
    std::fs::write(&data, "a: [\n").unwrap();
    allow_file_persisted(&root, &data);
    let doc = root.join("doc.md");
    let text = "---\n$schema:\n  os: enum(Haiku, {{ load_data(\"./os.yaml\", \"string[]\") }})\nos: \n---\n";
    let mut fixture = LspFixture::start_with_snapshot(&workspace, snapshot(&root));
    fixture.initialize(params(&root));
    let diagnostics = open_and_resolve(&mut fixture, &doc, text);
    let errors: Vec<&Value> = diagnostics.iter().filter(|diagnostic| diagnostic["severity"] == json!(1)).collect();
    assert_eq!(codes(&errors.into_iter().cloned().collect::<Vec<_>>()), ["dm.data.malformed"], "{diagnostics:#?}");
    let items = completion_items(&mut fixture, &doc, 3, 4);
    assert_eq!(labels(&items), ["Haiku"], "{items:#?}");
    assert!(items.iter().all(|item| item["detail"].as_str().is_some_and(|detail| detail.contains(PENDING_NOTE))), "{items:#?}");
}
