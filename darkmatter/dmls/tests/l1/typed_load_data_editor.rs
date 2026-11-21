//! Schema-typed `load_data` in the editor: whole-value frontmatter
//! properties that load data without a contract are evaluated on the slot
//! worker under the editor's read policy, and their observed values are
//! judged by the destination schema, typed for completion and hover, and
//! expired when what they read changes.
//!
//! Boundary: the LSP protocol (an in-memory `Connection` driving the real
//! server, router, overlay, slot worker, and `permissions` loading), the
//! filesystem (data files and the user `.darkmatter.yaml` under the
//! fixture's own `HOME`), and the transport cache (entries written under a
//! temporary cache root; nothing is fetched). Parity cells also compose the
//! same document in-process. F2, L1: no terminal, no editor process, no
//! window, no network.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use crate::common;

use common::{LspFixture, LspWorkspace};
use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::{ComposeContext, ComposeOptions, ComposeRequest, RequestSnapshot};
use serde_json::{Value, json};

/// The worked example's data: Grace's age (line 4) is valid.
const PEOPLE: &str = "- name: Ada\n  age: 36\n- name: Grace\n  age: 85\n";
/// The same data with Grace's age a string.
const PEOPLE_OLD: &str = "- name: Ada\n  age: 36\n- name: Grace\n  age: \"old\"\n";

/// The specification's worked example.
const WORKED: &str = concat!(
    "---\n",
    "$schema:\n",
    "    direct: \"{ name: string(required), age: number }[]\"\n",
    "    additive: \"{ name: string(required), age: number, color: string }[]\"\n",
    "    conditional:\n",
    "        - \"{ name: string(required), age: number }[]\"\n",
    "        - \"literal(foobar)\"\n",
    "direct: \"{{ load_data('data.yaml') }}\"\n",
    "additive: \"{{ map(load_data('data.yaml'), i => { ...i, color: 'blue' }) }}\"\n",
    "conditional: \"{{ file_exists('data.yaml') ? load_data('data.yaml') : 'foobar' }}\"\n",
    "---\n\n# Doc\n",
);

/// A `permissions` block pre-approving no read.
const NO_READS: &str = "permissions:\n    file-read: []\n";

/// A canonical workspace whose fixture `HOME` pre-approves `file_read`
/// (relative entries, judged from the reading document).
fn workspace_root(workspace: &LspWorkspace, file_read: &[&str]) -> PathBuf {
    let root = biscuit_file::canonicalize_simplified(workspace.path()).unwrap();
    write_user_config(&root, &permissions(file_read));
    root
}

fn permissions(file_read: &[&str]) -> String {
    if file_read.is_empty() {
        return NO_READS.to_string();
    }
    let list: String = file_read.iter().map(|entry| format!("        - \"{entry}\"\n")).collect();
    format!("permissions:\n    file-read:\n{list}")
}

fn write_user_config(root: &Path, text: &str) {
    let path = root.join("home").join(".darkmatter.yaml");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

fn write(root: &Path, name: &str, text: &str) -> PathBuf {
    let path = root.join(name);
    std::fs::write(&path, text).unwrap();
    path
}

fn snapshot(root: &Path) -> RequestSnapshot {
    RequestSnapshot::new(root).with_home(Some(root.join("home")))
}

/// Initialize params for a client that watches files (unless `watches` is
/// false), with UTF-16 positions and Markdown hovers.
fn params(root: &Path, watches: bool) -> Value {
    let root_uri = url::Url::from_directory_path(root).unwrap();
    let mut workspace = json!({});
    if watches {
        workspace = json!({ "didChangeWatchedFiles": { "dynamicRegistration": true, "relativePatternSupport": true } });
    }
    json!({
        "processId": null,
        "capabilities": {
            "workspace": workspace,
            "textDocument": {
                "hover": { "contentFormat": ["markdown"] },
                "completion": { "completionItem": { "snippetSupport": false } }
            }
        },
        "workspaceFolders": [ { "uri": root_uri.as_str(), "name": "typed" } ]
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

/// A server for `root` (watching files), initialized.
fn server<'w>(workspace: &'w LspWorkspace, root: &Path) -> LspFixture<'w> {
    let mut fixture = LspFixture::start_with_snapshot(workspace, snapshot(root));
    fixture.initialize(params(root, true));
    fixture
}

/// Opens `doc` and returns the diagnostics published once its first slot
/// job is installed.
fn open_and_observe(fixture: &mut LspFixture, doc: &Path, text: &str) -> Vec<Value> {
    open(fixture, doc, text, 1);
    settle(fixture, doc)
}

/// Waits for the next installed slot job (a result a later change
/// superseded is dropped and its refresh runs again), then returns the
/// document's latest diagnostics.
fn settle(fixture: &mut LspFixture, doc: &Path) -> Vec<Value> {
    while !fixture.wait_for_slot_prepared() {}
    fixture.flush_server();
    fixture.wait_for_diagnostics(&uri(doc))
}

/// The diagnostics whose `data.instance_path` starts at `/key`.
fn at_property<'d>(diagnostics: &'d [Value], key: &str) -> Vec<&'d Value> {
    let prefix = format!("/{key}");
    diagnostics
        .iter()
        .filter(|diagnostic| {
            diagnostic["data"]["instance_path"]
                .as_str()
                .is_some_and(|path| path == prefix || path.starts_with(&format!("{prefix}/")))
        })
        .collect()
}

fn with_code<'d>(diagnostics: &'d [Value], code: &str) -> Vec<&'d Value> {
    diagnostics.iter().filter(|diagnostic| diagnostic["code"] == json!(code)).collect()
}

/// The 0-based line of the first `needle` in `text`.
fn line_of(text: &str, needle: &str) -> usize {
    let at = text.find(needle).unwrap_or_else(|| panic!("no `{needle}`"));
    text[..at].matches('\n').count()
}

/// No destination finding (a schema instance path) on any property.
fn destination_findings(diagnostics: &[Value]) -> Vec<&Value> {
    diagnostics.iter().filter(|diagnostic| diagnostic["data"]["instance_path"].is_string()).collect()
}

/// The `(code, data pointer)` of composing `text` at `root/doc.md`'s
/// failure records, sorted; empty when it composes.
fn compose_findings(root: &Path, text: &str) -> Vec<(String, Option<String>)> {
    let path = write(root, "doc.md", text);
    let options =
        ComposeOptions::new_with_context(ComposeContext::capture_for_content(root, "")).with_source_file(path.clone());
    let request = ComposeRequest::prepare(options, &snapshot(root)).expect("request");
    let error = match Markdown::try_from(path.as_path()).expect("document loads").compose_with(&request) {
        Ok(_) => return Vec::new(),
        Err(error) => error,
    };
    let mut findings: Vec<_> = darkmatter::markdown::compose::report_diagnostics::error_diagnostics(&error)
        .into_iter()
        .map(|record| (record.code.clone(), record.data_location.and_then(|location| location.pointer)))
        .collect();
    findings.sort();
    findings
}

/// The editor's `(code, data pointer)` for the destination findings in
/// `diagnostics`, sorted.
fn editor_findings(diagnostics: &[Value]) -> Vec<(String, Option<String>)> {
    let mut findings: Vec<_> = destination_findings(diagnostics)
        .into_iter()
        .map(|diagnostic| {
            (
                diagnostic["code"].as_str().unwrap().to_string(),
                diagnostic["data"]["data_location"]["pointer"].as_str().map(str::to_string),
            )
        })
        .collect();
    findings.sort();
    findings
}

// ── The worked example ─────────────────────────────────────────────────────

/// With the data approved and valid, all three properties pass in the
/// editor once the observation is installed.
#[test]
fn the_worked_example_is_valid_in_the_editor() {
    let workspace = LspWorkspace::new();
    let root = workspace_root(&workspace, &["./data.yaml"]);
    write(&root, "data.yaml", PEOPLE);
    let doc = root.join("doc.md");
    let mut fixture = server(&workspace, &root);
    let diagnostics = open_and_observe(&mut fixture, &doc, WORKED);
    assert!(destination_findings(&diagnostics).is_empty(), "{diagnostics:#?}");
    assert!(with_code(&diagnostics, "dm.expression.evaluation_failure").is_empty(), "{diagnostics:#?}");
}

/// Grace's age `"old"` fails all three properties: the direct load and the
/// selected conditional arm point at the expression and at line 4 of the
/// data file; the spread row has the expression location alone.
#[test]
fn a_bad_age_fails_every_property_with_its_location() {
    let workspace = LspWorkspace::new();
    let root = workspace_root(&workspace, &["./data.yaml"]);
    let data = write(&root, "data.yaml", PEOPLE_OLD);
    let doc = root.join("doc.md");
    let mut fixture = server(&workspace, &root);
    let diagnostics = open_and_observe(&mut fixture, &doc, WORKED);

    for (key, direct) in [("direct", true), ("additive", false), ("conditional", true)] {
        let found = at_property(&diagnostics, key);
        assert_eq!(found.len(), 1, "{key}: {diagnostics:#?}");
        let finding = found[0];
        assert_eq!(finding["severity"], json!(1), "{finding:#?}");
        assert!(finding["code"].as_str().unwrap().starts_with("dm.schema."), "{finding:#?}");
        assert_eq!(finding["range"]["start"]["line"], json!(line_of(WORKED, &format!("{key}: \"{{{{"))), "{finding:#?}");
        if direct {
            let location = &finding["data"]["data_location"];
            assert_eq!(location["line"], json!(4), "{finding:#?}");
            assert_eq!(location["pointer"], json!("/1/age"), "{finding:#?}");
            let related = finding["relatedInformation"].as_array().expect("related information");
            assert_eq!(related[0]["location"]["uri"], json!(uri(&data)), "{finding:#?}");
            assert_eq!(related[0]["location"]["range"]["start"]["line"], json!(3), "{finding:#?}");
        } else {
            assert!(finding["data"]["data_location"].is_null(), "{finding:#?}");
            assert!(finding["relatedInformation"].is_null(), "{finding:#?}");
        }
    }
}

/// A parenthesized direct load keeps its data position too.
#[test]
fn a_parenthesized_load_keeps_its_data_position() {
    let workspace = LspWorkspace::new();
    let root = workspace_root(&workspace, &["./data.yaml"]);
    write(&root, "data.yaml", PEOPLE_OLD);
    let doc = root.join("doc.md");
    let text = "---\n$schema:\n    direct: \"{ name: string(required), age: number }[]\"\ndirect: \"{{ (load_data('data.yaml')) }}\"\n---\n";
    let mut fixture = server(&workspace, &root);
    let diagnostics = open_and_observe(&mut fixture, &doc, text);
    let found = at_property(&diagnostics, "direct");
    assert_eq!(found.len(), 1, "{diagnostics:#?}");
    assert_eq!(found[0]["data"]["data_location"]["line"], json!(4), "{:#?}", found[0]);
}

/// Each property composed on its own fails with the same code, instance
/// path, and data pointer the editor reports.
#[test]
fn the_editor_and_compose_agree_on_each_property() {
    let workspace = LspWorkspace::new();
    let root = workspace_root(&workspace, &["./data.yaml"]);
    write(&root, "data.yaml", PEOPLE_OLD);
    let declarations = [
        ("direct", "    direct: \"{ name: string(required), age: number }[]\"\n", "direct: \"{{ load_data('data.yaml') }}\"\n"),
        (
            "additive",
            "    additive: \"{ name: string(required), age: number, color: string }[]\"\n",
            "additive: \"{{ map(load_data('data.yaml'), i => { ...i, color: 'blue' }) }}\"\n",
        ),
        (
            "conditional",
            "    conditional:\n        - \"{ name: string(required), age: number }[]\"\n        - \"literal(foobar)\"\n",
            "conditional: \"{{ file_exists('data.yaml') ? load_data('data.yaml') : 'foobar' }}\"\n",
        ),
    ];
    let mut fixture = server(&workspace, &root);
    for (index, (key, declaration, value)) in declarations.into_iter().enumerate() {
        let text = format!("---\n$schema:\n{declaration}{value}---\n");
        let composed = compose_findings(&root, &text);
        assert!(!composed.is_empty(), "{key} fails in compose");
        let doc = root.join(format!("{key}.md"));
        let diagnostics = open_and_observe(&mut fixture, &doc, &text);
        assert_eq!(editor_findings(&diagnostics), composed, "{key} (#{index}): {diagnostics:#?}");
    }
}

// ── Missing inputs ─────────────────────────────────────────────────────────

/// A document whose `people` property is `expression`, declared as rows.
fn people_doc(expression: &str) -> String {
    format!(
        "---\n$schema:\n    people:\n        - \"{{ name: string(required) }}[]\"\n        - \"literal(none)\"\npeople: \"{{{{ {expression} }}}}\"\n---\n"
    )
}

/// A guarded or recovered missing input is no warning and no verdict.
#[test]
fn a_guarded_or_recovered_missing_input_is_no_warning() {
    let workspace = LspWorkspace::new();
    let root = workspace_root(&workspace, &["./data.yaml"]);
    let mut fixture = server(&workspace, &root);
    for (name, expression) in [
        ("guarded", "file_exists('data.yaml') ? load_data('data.yaml') : 'none'"),
        ("recovered", "try(load_data('data.yaml'), 'none')"),
    ] {
        let doc = root.join(format!("{name}.md"));
        let diagnostics = open_and_observe(&mut fixture, &doc, &people_doc(expression));
        assert!(with_code(&diagnostics, "dm.expression.evaluation_failure").is_empty(), "{name}: {diagnostics:#?}");
        assert!(destination_findings(&diagnostics).is_empty(), "{name}: {diagnostics:#?}");
    }
}

/// An executed, unrecovered load of a missing approved file is a warning at
/// the expression; creating the file (a watch event) refreshes and clears it.
#[test]
fn an_unrecovered_missing_load_warns_until_the_file_is_created() {
    let workspace = LspWorkspace::new();
    let root = workspace_root(&workspace, &["./data.yaml"]);
    let doc = root.join("doc.md");
    let text = people_doc("load_data('data.yaml')");
    let mut fixture = server(&workspace, &root);
    let diagnostics = open_and_observe(&mut fixture, &doc, &text);
    let warnings = with_code(&diagnostics, "dm.expression.evaluation_failure");
    assert_eq!(warnings.len(), 1, "{diagnostics:#?}");
    assert_eq!(warnings[0]["severity"], json!(2), "{:#?}", warnings[0]);
    assert_eq!(warnings[0]["range"]["start"]["line"], json!(line_of(&text, "people: \"{{")), "{:#?}", warnings[0]);
    assert!(destination_findings(&diagnostics).is_empty(), "{diagnostics:#?}");

    let data = write(&root, "data.yaml", "- name: Ada\n");
    watched_change(&fixture, &data, 1);
    let diagnostics = settle(&mut fixture, &doc);
    assert!(with_code(&diagnostics, "dm.expression.evaluation_failure").is_empty(), "{diagnostics:#?}");
    assert!(destination_findings(&diagnostics).is_empty(), "{diagnostics:#?}");
}

/// A client without file watches gets the save-driven stat fallback: a save
/// re-checks the missing candidate, which now exists.
#[test]
fn without_watchers_a_save_finds_the_created_file() {
    let workspace = LspWorkspace::new();
    let root = workspace_root(&workspace, &["./data.yaml"]);
    let doc = root.join("doc.md");
    let text = people_doc("load_data('data.yaml')");
    let mut fixture = LspFixture::start_with_snapshot(&workspace, snapshot(&root));
    fixture.initialize(params(&root, false));
    let diagnostics = open_and_observe(&mut fixture, &doc, &text);
    assert_eq!(with_code(&diagnostics, "dm.expression.evaluation_failure").len(), 1, "{diagnostics:#?}");

    write(&root, "data.yaml", "- age: 3\n");
    fixture.notify("textDocument/didSave", json!({ "textDocument": { "uri": uri(&doc) } }));
    let diagnostics = settle(&mut fixture, &doc);
    assert!(with_code(&diagnostics, "dm.expression.evaluation_failure").is_empty(), "{diagnostics:#?}");
    assert!(!at_property(&diagnostics, "people").is_empty(), "the created rows lack `name`: {diagnostics:#?}");
}

/// An open, unsaved data buffer is what the observation reads: the verdict
/// follows the buffer, not the saved file.
#[test]
fn an_unsaved_data_buffer_decides_the_verdict() {
    let workspace = LspWorkspace::new();
    let root = workspace_root(&workspace, &["./data.yaml"]);
    let data = write(&root, "data.yaml", PEOPLE);
    let doc = root.join("doc.md");
    let text = "---\n$schema:\n    direct: \"{ name: string(required), age: number }[]\"\ndirect: \"{{ load_data('data.yaml') }}\"\n---\n";
    let mut fixture = server(&workspace, &root);
    assert!(destination_findings(&open_and_observe(&mut fixture, &doc, text)).is_empty());

    open(&fixture, &data, PEOPLE_OLD, 1);
    let diagnostics = settle(&mut fixture, &doc);
    assert_eq!(at_property(&diagnostics, "direct").len(), 1, "the unsaved buffer fails: {diagnostics:#?}");
    assert_eq!(std::fs::read_to_string(&data).unwrap(), PEOPLE, "nothing was saved");

    change(&fixture, &data, PEOPLE, 2);
    let diagnostics = settle(&mut fixture, &doc);
    assert!(destination_findings(&diagnostics).is_empty(), "{diagnostics:#?}");
}

// ── Authorization and invalidation ─────────────────────────────────────────

/// An unapproved read is pending: no verdict and no invented member
/// finding. Approving it in the user configuration refreshes the document.
#[test]
fn a_refused_read_is_pending_until_the_configuration_approves_it() {
    let workspace = LspWorkspace::new();
    let root = workspace_root(&workspace, &[]);
    write(&root, "data.yaml", PEOPLE_OLD);
    let doc = root.join("doc.md");
    let text = concat!(
        "---\n$schema:\n    direct: \"{ name: string(required), age: number }[]\"\n",
        "direct: \"{{ load_data('data.yaml') }}\"\nnames: \"{{ map(direct, d => d.name) }}\"\n---\n",
    );
    let mut fixture = server(&workspace, &root);
    let diagnostics = open_and_observe(&mut fixture, &doc, text);
    assert!(destination_findings(&diagnostics).is_empty(), "{diagnostics:#?}");
    assert!(with_code(&diagnostics, "dm.expression.evaluation_failure").is_empty(), "{diagnostics:#?}");
    assert!(
        diagnostics.iter().all(|diagnostic| !diagnostic["code"].as_str().unwrap_or_default().contains("undeclared_member")),
        "{diagnostics:#?}"
    );

    write_user_config(&root, &permissions(&["./data.yaml"]));
    let config = root.join("home").join(".darkmatter.yaml");
    watched_change(&fixture, &config, 2);
    fixture.notify("workspace/didChangeConfiguration", json!({ "settings": {} }));
    let diagnostics = settle(&mut fixture, &doc);
    assert_eq!(at_property(&diagnostics, "direct").len(), 1, "now judged: {diagnostics:#?}");
}

/// An in-memory approval refreshes the document the same way.
#[test]
fn a_once_grant_refreshes_a_pending_observation() {
    let workspace = LspWorkspace::new();
    let root = workspace_root(&workspace, &[]);
    let data = write(&root, "data.yaml", PEOPLE_OLD);
    let doc = root.join("doc.md");
    let text = "---\n$schema:\n    direct: \"{ name: string(required), age: number }[]\"\ndirect: \"{{ load_data('data.yaml') }}\"\n---\n";
    let mut fixture = server(&workspace, &root);
    assert!(destination_findings(&open_and_observe(&mut fixture, &doc, text)).is_empty());

    let response = fixture.request(
        "workspace/executeCommand",
        json!({
            "command": "dmls.grantSlotRead",
            "arguments": [ { "decision": "allow", "kind": "file", "target": data.display().to_string(), "scope": "once" } ]
        }),
    );
    assert!(response.error.is_none(), "{:?}", response.error);
    let diagnostics = settle(&mut fixture, &doc);
    assert_eq!(at_property(&diagnostics, "direct").len(), 1, "{diagnostics:#?}");
}

/// A schema change refreshes the verdict: the same observed value is judged
/// by the new declaration.
#[test]
fn a_schema_change_rejudges_the_observed_value() {
    let workspace = LspWorkspace::new();
    let root = workspace_root(&workspace, &["./data.yaml"]);
    write(&root, "data.yaml", PEOPLE);
    let doc = root.join("doc.md");
    let text = "---\n$schema:\n    direct: \"{ name: string(required), age: number }[]\"\ndirect: \"{{ load_data('data.yaml') }}\"\n---\n";
    let mut fixture = server(&workspace, &root);
    assert!(destination_findings(&open_and_observe(&mut fixture, &doc, text)).is_empty());

    change(&fixture, &doc, &text.replace("age: number", "age: boolean"), 2);
    let diagnostics = settle(&mut fixture, &doc);
    assert_eq!(at_property(&diagnostics, "direct").len(), 2, "both ages are numbers: {diagnostics:#?}");
}

/// With the worker held, two edits supersede the first job: only the
/// newest result installs, and its verdict is the one published.
#[test]
fn a_superseded_job_never_installs() {
    let workspace = LspWorkspace::new();
    let root = workspace_root(&workspace, &["./data.yaml"]);
    write(&root, "data.yaml", PEOPLE);
    let doc = root.join("doc.md");
    let text = "---\n$schema:\n    direct: \"{ name: string(required), age: number }[]\"\ndirect: \"{{ load_data('data.yaml') }}\"\n---\n";
    let mut fixture = LspFixture::start_with_slot_options(&workspace, snapshot(&root), true, None, None);
    fixture.initialize(params(&root, true));
    open(&fixture, &doc, text, 1);
    fixture.round_trip();
    change(&fixture, &doc, &text.replace("age: number", "age: string"), 2);
    fixture.round_trip();
    change(&fixture, &doc, &text.replace("age: number", "age: boolean"), 3);
    fixture.round_trip();
    fixture.open_slot_gate();
    let mut installed = Vec::new();
    loop {
        installed.push(fixture.wait_for_slot_prepared());
        if installed.last() == Some(&true) {
            break;
        }
    }
    fixture.flush_server();
    let diagnostics = fixture.wait_for_diagnostics(&uri(&doc));
    let found = at_property(&diagnostics, "direct");
    assert_eq!(found.len(), 2, "{diagnostics:#?}");
    assert!(
        found.iter().all(|finding| finding["message"].as_str().unwrap().contains("boolean")),
        "judged by the newest schema: {found:#?}"
    );
    assert!(installed[..installed.len() - 1].iter().all(|installed| !installed), "{installed:?}");
}

// ── Remote reads ───────────────────────────────────────────────────────────

/// Nothing resolves here; the observation never connects anyway.
const REMOTE: &str = "https://example.invalid/people.yaml";

/// An allowed URL is answered from its fresh transport-cache entry and
/// judged; once the clock passes the entry's expiry, the observation is
/// withdrawn and the refresh finds no fresh entry, so the property is
/// pending. Nothing is fetched (the host does not exist).
#[test]
fn a_cached_remote_response_is_judged_until_it_expires() {
    let workspace = LspWorkspace::new();
    let root = workspace_root(&workspace, &[]);
    write_user_config(&root, "permissions:\n    file-read: []\n    network: allow\n");
    let cache = tempfile::tempdir().unwrap();
    let start = SystemTime::now();
    let clock = Arc::new(dmls::clock::ManualClock::starting_at(start));
    darkmatter::markdown::compose::cache_test_hooks::seed_remote_entry(
        cache.path(),
        &url::Url::parse(REMOTE).unwrap(),
        PEOPLE_OLD,
        start,
        start + Duration::from_secs(60),
    )
    .unwrap();
    let doc = root.join("doc.md");
    let text = format!(
        "---\n$schema:\n    direct: \"{{ name: string(required), age: number }}[]\"\ndirect: \"{{{{ load_data('{REMOTE}') }}}}\"\n---\n"
    );
    let mut fixture = LspFixture::start_with_slot_options(
        &workspace,
        snapshot(&root),
        false,
        Some(cache.path().to_path_buf()),
        Some(Arc::clone(&clock) as Arc<dyn dmls::clock::Clock>),
    );
    fixture.initialize(params(&root, true));
    let diagnostics = open_and_observe(&mut fixture, &doc, &text);
    let found = at_property(&diagnostics, "direct");
    assert_eq!(found.len(), 1, "judged from the cache: {diagnostics:#?}");
    assert_eq!(found[0]["data"]["data_location"]["pointer"], json!("/1/age"), "{:#?}", found[0]);
    assert!(found[0]["relatedInformation"].is_null(), "a URL has no local range: {:#?}", found[0]);

    clock.advance(Duration::from_secs(61));
    // Any message wakes the dispatch loop, which then sees the deadline.
    fixture.round_trip();
    let diagnostics = settle(&mut fixture, &doc);
    assert!(destination_findings(&diagnostics).is_empty(), "the expired response is withdrawn: {diagnostics:#?}");
}

/// A URL the request does not allow never consults the cache, even with a
/// fresh entry: the property stays pending.
#[test]
fn a_denied_url_is_pending_with_a_fresh_entry() {
    let workspace = LspWorkspace::new();
    let root = workspace_root(&workspace, &[]);
    let cache = tempfile::tempdir().unwrap();
    let start = SystemTime::now();
    darkmatter::markdown::compose::cache_test_hooks::seed_remote_entry(
        cache.path(),
        &url::Url::parse(REMOTE).unwrap(),
        PEOPLE_OLD,
        start,
        start + Duration::from_secs(600),
    )
    .unwrap();
    let doc = root.join("doc.md");
    let text = format!(
        "---\n$schema:\n    direct: \"{{ name: string(required), age: number }}[]\"\ndirect: \"{{{{ load_data('{REMOTE}') }}}}\"\n---\n"
    );
    let mut fixture =
        LspFixture::start_with_slot_options(&workspace, snapshot(&root), false, Some(cache.path().to_path_buf()), None);
    fixture.initialize(params(&root, true));
    let diagnostics = open_and_observe(&mut fixture, &doc, &text);
    assert!(destination_findings(&diagnostics).is_empty(), "{diagnostics:#?}");
}

// ── Pending observations ───────────────────────────────────────────────────

/// A document declaring `direct` as rows and authoring it as `expression`,
/// beside `extra` frontmatter lines.
fn direct_doc(expression: &str, extra: &str) -> String {
    format!(
        "---\n$schema:\n    direct: \"{{ name: string(required), age: number }}[]\"\ndirect: \"{{{{ {expression} }}}}\"\n{extra}---\n"
    )
}

/// An effectful function, a runtime-only binding, and properties that read
/// each other leave the property pending: the bad data is never judged and
/// nothing warns.
#[test]
fn unsupported_runtime_only_and_cyclic_properties_are_pending() {
    let workspace = LspWorkspace::new();
    let root = workspace_root(&workspace, &["./data.yaml"]);
    write(&root, "data.yaml", PEOPLE_OLD);
    let mut fixture = server(&workspace, &root);
    for (name, text) in [
        ("effectful", direct_doc("load_data('data.yaml') || as_markdown('other.md')", "")),
        ("runtime", direct_doc("ctx.cwd ? load_data('data.yaml') : []", "")),
        ("cycle", direct_doc("other ? load_data('data.yaml') : []", "other: \"{{ direct }}\"\n")),
    ] {
        let doc = root.join(format!("{name}.md"));
        let diagnostics = open_and_observe(&mut fixture, &doc, &text);
        assert!(at_property(&diagnostics, "direct").is_empty(), "{name}: {diagnostics:#?}");
        assert!(with_code(&diagnostics, "dm.expression.evaluation_failure").is_empty(), "{name}: {diagnostics:#?}");
    }
    // Control: the same data is judged once nothing blocks the evaluation.
    let doc = root.join("control.md");
    let diagnostics = open_and_observe(&mut fixture, &doc, &direct_doc("load_data('data.yaml')", ""));
    assert_eq!(at_property(&diagnostics, "direct").len(), 1, "{diagnostics:#?}");
}

/// A runaway expression stops at the editor's work allowance: no verdict,
/// no prompt, and the job reports the exhausted pass.
#[test]
fn an_exhausted_allowance_is_pending_without_a_prompt() {
    let logs = dmls::log_capture::CapturedLogs::global();
    let workspace = LspWorkspace::new();
    let root = workspace_root(&workspace, &["./list.yaml"]);
    let list: String = (0..200).map(|index| format!("- {index}\n")).collect();
    write(&root, "list.yaml", &list);
    let doc = root.join("doc.md");
    let text = concat!(
        "---\n$schema:\n    many: \"string[]\"\n",
        "many: \"{{ map(load_data('list.yaml'), x => first(map(load_data('list.yaml'), y => x))) }}\"\n---\n",
    );
    let mut fixture = server(&workspace, &root);
    let diagnostics = open_and_observe(&mut fixture, &doc, text);
    assert!(at_property(&diagnostics, "many").is_empty(), "{diagnostics:#?}");
    assert!(
        fixture.buffered_server_requests().iter().all(|request| matches!(
            request.method.as_str(),
            "client/registerCapability" | "client/unregisterCapability" | "window/workDoneProgress/create"
        )),
        "nothing prompts: {:?}",
        fixture.buffered_server_requests().iter().map(|request| &request.method).collect::<Vec<_>>()
    );
    let lines = logs.matching("schema slots resolved");
    assert!(lines.iter().any(|line| line.contains("exhausted=true")), "{lines:#?}");
}

/// Two properties loading the same file share one read and one parse.
#[test]
fn properties_loading_one_file_share_its_parse() {
    let logs = dmls::log_capture::CapturedLogs::global();
    let workspace = LspWorkspace::new();
    let root = workspace_root(&workspace, &["./data.yaml"]);
    write(&root, "data.yaml", PEOPLE);
    let doc = root.join("doc.md");
    let mut fixture = server(&workspace, &root);
    let diagnostics = open_and_observe(&mut fixture, &doc, WORKED);
    assert!(destination_findings(&diagnostics).is_empty(), "{diagnostics:#?}");
    let lines = logs.matching("schema slots resolved");
    let line = lines.iter().find(|line| line.contains("properties=3")).unwrap_or_else(|| panic!("{lines:#?}"));
    assert!(line.contains("data_reads=1") && line.contains("parses=1"), "{line}");
}

// ── Hover and completion ───────────────────────────────────────────────────

fn hover(fixture: &mut LspFixture, doc: &Path, line: usize, character: usize) -> String {
    let response = fixture.request(
        "textDocument/hover",
        json!({ "textDocument": { "uri": uri(doc) }, "position": { "line": line, "character": character } }),
    );
    response.result.map(|result| result["contents"]["value"].as_str().unwrap_or_default().to_string()).unwrap_or_default()
}

/// Hover on an observed property shows its declared type and a bounded
/// preview: eight rows, the rest counted, a long string cut.
#[test]
fn hover_shows_a_bounded_preview_after_the_declared_type() {
    let workspace = LspWorkspace::new();
    let root = workspace_root(&workspace, &["./data.yaml"]);
    let mut rows = format!("- name: \"{}\"\n  age: 1\n", "x".repeat(200));
    for index in 1..12 {
        rows.push_str(&format!("- name: n{index}\n  age: {index}\n"));
    }
    write(&root, "data.yaml", &rows);
    let doc = root.join("doc.md");
    let text = direct_doc("load_data('data.yaml')", "");
    let mut fixture = server(&workspace, &root);
    assert!(destination_findings(&open_and_observe(&mut fixture, &doc, &text)).is_empty());
    let shown = hover(&mut fixture, &doc, line_of(&text, "direct: \"{{"), 1);
    let preview = shown.find("Observed value").unwrap_or_else(|| panic!("{shown}"));
    assert!(preview > 0, "the declared type comes first: {shown}");
    assert!(shown.contains("…4 more"), "{shown}");
    assert!(shown.contains("…(+80 chars)"), "{shown}");
    assert!(!shown.contains("n9"), "rows past eight are not formatted: {shown}");
}

fn completion_labels(fixture: &mut LspFixture, doc: &Path, line: usize, character: usize) -> Vec<String> {
    let response = fixture.request(
        "textDocument/completion",
        json!({ "textDocument": { "uri": uri(doc) }, "position": { "line": line, "character": character } }),
    );
    let result = response.result.unwrap_or(Value::Null);
    let items = result.get("items").cloned().unwrap_or(result);
    items.as_array().map(|items| items.iter().filter_map(|item| item["label"].as_str().map(str::to_string)).collect()).unwrap_or_default()
}

/// A callback parameter fed by an observed load completes with the loaded
/// rows' members while the observation is current; before it is installed
/// the load's type is provisional and offers none.
#[test]
fn a_callback_parameter_completes_with_the_observed_members() {
    let workspace = LspWorkspace::new();
    let root = workspace_root(&workspace, &["./data.yaml"]);
    write(&root, "data.yaml", PEOPLE);
    let doc = root.join("doc.md");
    let text = "---\n$schema:\n    names: \"string[]\"\nnames: \"{{ map(load_data('data.yaml'), i => i.name) }}\"\n---\n";
    let line = line_of(text, "names: \"{{");
    let line_text = text.lines().nth(line).unwrap();
    let character = line_text.find("i.name").unwrap() + 2;

    let mut fixture = LspFixture::start_with_slot_options(&workspace, snapshot(&root), true, None, None);
    fixture.initialize(params(&root, true));
    open(&fixture, &doc, text, 1);
    fixture.round_trip();
    let provisional = completion_labels(&mut fixture, &doc, line, character);
    assert!(!provisional.iter().any(|label| label.ends_with("age")), "nothing observed yet: {provisional:?}");

    fixture.open_slot_gate();
    let diagnostics = settle(&mut fixture, &doc);
    assert!(destination_findings(&diagnostics).is_empty(), "{diagnostics:#?}");
    let observed = completion_labels(&mut fixture, &doc, line, character);
    assert!(
        observed.iter().any(|label| label.ends_with("name")) && observed.iter().any(|label| label.ends_with("age")),
        "{observed:?}"
    );
}

// ── Headerless tabular data ────────────────────────────────────────────────

/// An uncontracted `header: false` read is `unknown[][]` in the editor: its
/// cells get no editor verdict, while composition judges each realized cell
/// and fails on the bad one.
#[test]
fn a_headerless_uncontracted_read_has_no_editor_cell_verdict() {
    let workspace = LspWorkspace::new();
    let root = workspace_root(&workspace, &["./table.csv"]);
    write(&root, "table.csv", "1,2\n3,x\n");
    let text = "---\n$schema:\n    cells: \"tuple([number, number])[]\"\ncells: \"{{ load_data('table.csv', null, { header: false }) }}\"\n---\n";
    assert!(!compose_findings(&root, text).is_empty(), "compose fails on the bad cell");
    let doc = root.join("cells.md");
    let mut fixture = server(&workspace, &root);
    let diagnostics = open_and_observe(&mut fixture, &doc, text);
    assert!(at_property(&diagnostics, "cells").is_empty(), "{diagnostics:#?}");
    // Control: the same file read with a header record is judged, row by
    // row, from its observed value.
    let headed = "---\n$schema:\n    rows: \"{ a: number, b: number }[]\"\nrows: \"{{ load_data('table.csv') }}\"\n---\n";
    let doc = root.join("rows.md");
    let diagnostics = open_and_observe(&mut fixture, &doc, headed);
    assert_eq!(at_property(&diagnostics, "rows").len(), 1, "{diagnostics:#?}");
}
