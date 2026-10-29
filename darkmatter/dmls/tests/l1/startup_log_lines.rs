//! Level-1 sessions for the startup and index log lines: the schema
//! registry line (and its failure variant), the file-list and rescan lines,
//! and the index store's revalidation line, each with its fields for a
//! fixture whose counts are known.
//!
//! Each test installs a process-wide capturing subscriber, because the lines
//! come from DMLS's dispatch thread; nextest runs each test in its own
//! process, so one global subscriber per test is allowed.

use crate::common;

use common::{LspFixture, LspWorkspace};
use dmls::log_capture::CapturedLogs;
use serde_json::{Value, json};

fn init_repository(root: &std::path::Path) {
    std::fs::create_dir_all(root.join(".git/objects")).unwrap();
    std::fs::create_dir_all(root.join(".git/refs/heads")).unwrap();
    std::fs::write(root.join(".git/HEAD"), "ref: refs/heads/main\n").unwrap();
    std::fs::write(root.join(".git/config"), "[core]\n\trepositoryformatversion = 0\n\tbare = false\n").unwrap();
}

fn write(root: &std::path::Path, name: &str, text: &str) {
    let path = root.join(name);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

fn initialize_params(root: &std::path::Path) -> Value {
    json!({
        "processId": null,
        "clientInfo": { "name": "Log Client", "version": "1.0" },
        "capabilities": {
            "general": { "positionEncodings": ["utf-16"] },
            "workspace": { "didChangeWatchedFiles": { "dynamicRegistration": true } }
        },
        "workspaceFolders": [ { "uri": workspace_uri(root), "name": "logs" } ]
    })
}

fn workspace_uri(root: &std::path::Path) -> String {
    url::Url::from_directory_path(root).unwrap().to_string()
}

/// The one captured line containing `message`, asserting it carries every
/// `key=value` in `fields`.
fn assert_line(logs: &CapturedLogs, message: &str, fields: &[String]) -> String {
    let lines = logs.matching(message);
    assert_eq!(lines.len(), 1, "one `{message}` line: {:#?}", logs.lines());
    let line = &lines[0];
    assert!(line.contains(" INFO "), "{line}");
    for field in fields {
        assert!(line.contains(&format!(" {field}")), "`{field}` missing from: {line}");
    }
    assert!(line.contains(" elapsed_ms="), "{line}");
    line.clone()
}

/// A repository whose tree schema root holds a named schema, a trigger
/// schema, and an unclassified file, with one named schema in the fixture
/// home's root, three Markdown files, and one other YAML file.
fn known_workspace(workspace: &LspWorkspace) {
    let root = workspace.path();
    init_repository(root);
    write(root, "schemas/named.yaml", "$schema:\n  model: string(required)\n");
    write(root, "schemas/notes.trigger.yaml", "kind: schema-trigger\nmatch:\n  title: string(required)\n$schema: named.yaml\n");
    write(root, "schemas/plain.yaml", "just: data\n");
    write(root, "home/schemas/home.yaml", "$schema:\n  home_model: string(required)\n");
    write(root, "config.yml", "setting: true\n");
    write(root, "a.md", "# A\n\n## Section\n\nSee [b](b.md) and [[c]].\n");
    write(root, "b.md", "# B\n");
    write(root, "docs/c.md", "# C\n\n[back](../a.md)\n");
}

#[test]
fn startup_and_revalidation_lines_carry_the_fixture_counts() {
    let logs = CapturedLogs::global();
    let workspace = LspWorkspace::new();
    known_workspace(&workspace);
    let root = workspace.path();
    let id = workspace_uri(root);
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(initialize_params(root));

    // Two searched roots: the tree's `schemas/` and the home's; the absent
    // `SCHEMAS_DIR` root is not counted.
    assert_line(&logs, "schema registry ready", &[
        "roots=2".into(),
        "tree_schemas=2".into(),
        "scoped_schemas=0".into(),
        "trigger_schemas=1".into(),
        "unclassified=1".into(),
        "failed_triggers=0".into(),
        "failed_scopes=0".into(),
        format!("workspace={id}"),
    ]);
    assert!(logs.matching("initialized with failures").is_empty());
    // Four schema-root YAML files plus `config.yml`; `.git` is skipped.
    assert_line(&logs, "workspace files listed", &[
        "markdown=3".into(),
        "yaml=5".into(),
        "duplicates=0".into(),
        format!("workspace={id}"),
    ]);
    assert!(logs.matching("index store").is_empty(), "startup opens no store: {:#?}", logs.lines());

    let response = fixture.request("workspace/symbol", json!({ "query": "" }));
    assert!(response.error.is_none(), "{:?}", response.error);
    // A first fill: every listed file is parsed and written.
    assert_line(&logs, "index store revalidated", &[
        "markdown=3".into(),
        "unchanged=0".into(),
        "reparsed=3".into(),
        "removed=0".into(),
        "trigger=workspace-symbols".into(),
        format!("workspace={id}"),
    ]);

    // An ignore-file change lists the workspace again.
    write(root, "d.md", "# D\n");
    write(root, ".gitignore", "build/\n");
    let gitignore = url::Url::from_file_path(root.join(".gitignore")).unwrap().to_string();
    fixture.notify("workspace/didChangeWatchedFiles", json!({ "changes": [ { "uri": gitignore, "type": 1 } ] }));
    fixture.request("textDocument/documentSymbol", json!({ "textDocument": { "uri": gitignore } }));
    assert_line(&logs, "workspace rescan complete", &[
        "markdown=4".into(),
        "yaml=5".into(),
        "duplicates=0".into(),
        format!("workspace={id}"),
    ]);
    assert!(logs.matching("startup index complete").is_empty(), "{:#?}", logs.lines());
    fixture.shutdown();
}

/// A skipped trigger leaves the registry line `ready`: its valid neighbor
/// still loads, the skipped file counts only as a failed trigger, and the
/// skip is logged once at WARN with the trigger's path and code.
#[test]
fn a_skipped_trigger_keeps_the_registry_line_ready() {
    let logs = CapturedLogs::global();
    let workspace = LspWorkspace::new();
    known_workspace(&workspace);
    let root = workspace.path();
    write(root, "schemas/broken.trigger.yaml", "kind: schema-trigger\nmatch: {}\n");
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(initialize_params(root));

    assert_line(&logs, "schema registry ready", &[
        "roots=2".into(),
        "trigger_schemas=1".into(),
        "failed_triggers=1".into(),
        "failed_scopes=0".into(),
        format!("workspace={}", workspace_uri(root)),
    ]);
    assert!(logs.matching("initialized with failures").is_empty(), "{:#?}", logs.lines());
    let skipped = logs.matching("ignoring invalid schema trigger");
    assert_eq!(skipped.len(), 1, "one skip warning: {:#?}", logs.lines());
    assert!(skipped[0].starts_with(" WARN "), "{}", skipped[0]);
    assert!(skipped[0].contains("trigger=") && skipped[0].contains("broken.trigger.yaml"), "{}", skipped[0]);
    assert!(skipped[0].contains("code=\"dm.schema.invalid_trigger\""), "{}", skipped[0]);
    fixture.shutdown();
}

/// A schema root that cannot be read fails its whole scope, which switches
/// the registry line to its failure variant.
#[test]
fn a_failed_scope_switches_the_registry_line_to_its_failure_variant() {
    let logs = CapturedLogs::global();
    let workspace = LspWorkspace::new();
    known_workspace(&workspace);
    let root = workspace.path();
    std::fs::write(root.join("schemas/not-utf8.yaml"), [0xff, 0xfe, 0x00]).unwrap();
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(initialize_params(root));

    assert_line(&logs, "schema registry initialized with failures", &[
        "roots=2".into(),
        "failed_scopes=1".into(),
        format!("workspace={}", workspace_uri(root)),
    ]);
    assert!(logs.matching("schema registry ready").is_empty(), "{:#?}", logs.lines());
    fixture.shutdown();
}

/// One trigger whose match requires `title` and whose payload is
/// `payload.yaml`, with `payload` as that file's text (`None` leaves it
/// missing). Returns the startup registry line.
fn payload_registry_line(payload: Option<&str>) -> (CapturedLogs, String) {
    let logs = CapturedLogs::global();
    let workspace = LspWorkspace::new();
    let root = workspace.path();
    init_repository(root);
    write(root, "schemas/notes.trigger.yaml", "kind: schema-trigger\nmatch:\n  title: string(required)\n$schema: payload.yaml\n");
    if let Some(payload) = payload {
        write(root, "schemas/payload.yaml", payload);
    }
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(initialize_params(root));
    let lines = logs.matching("schema registry");
    assert_eq!(lines.len(), 1, "one registry line: {:#?}", logs.lines());
    let line = lines[0].clone();
    fixture.shutdown();
    (logs, line)
}

/// Each row varies only the trigger's payload and asserts the registry
/// line's variant and its trigger counts. A payload that keeps its trigger
/// from loading skips that trigger without failing the scope, so the line
/// stays `ready` and only `failed_triggers` counts it.
macro_rules! payload_registry_cases {
    ($($name:ident: $payload:expr => $message:literal, $triggers:literal / $failed:literal / $scopes:literal;)+) => {$(
        #[test]
        fn $name() {
            let (logs, line) = payload_registry_line($payload);
            assert!(line.contains($message), "{line}");
            assert_line(&logs, $message, &[
                concat!("trigger_schemas=", $triggers).into(),
                concat!("failed_triggers=", $failed).into(),
                concat!("failed_scopes=", $scopes).into(),
            ]);
        }
    )+};
}

payload_registry_cases! {
    a_loading_payload_counts_a_ready_trigger:
        Some("$schema:\n  model: string(required)\n") => "schema registry ready", 1 / 0 / 0;
    a_missing_payload_counts_a_failed_trigger:
        None => "schema registry ready", 0 / 1 / 0;
    an_unparseable_payload_counts_a_failed_trigger:
        Some("$schema: [unclosed\n") => "schema registry ready", 0 / 1 / 0;
    an_unresolved_payload_import_counts_a_failed_trigger:
        Some("$schema:\n  model: Missing@./missing.yaml\n") => "schema registry ready", 0 / 1 / 0;
    an_unresolved_payload_example_counts_a_failed_trigger:
        Some("$schema:\n  when: \"date(example(../missing.json))\"\n") => "schema registry ready", 0 / 1 / 0;
}

/// A malformed envelope is skipped like a broken payload: counted as a failed
/// trigger in a scope that still prepares.
#[test]
fn a_malformed_envelope_counts_a_failed_trigger() {
    let logs = CapturedLogs::global();
    let workspace = LspWorkspace::new();
    let root = workspace.path();
    init_repository(root);
    write(root, "schemas/notes.trigger.yaml", "kind: schema-trigger\nmatch: {}\n");
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(initialize_params(root));
    assert_line(&logs, "schema registry ready", &[
        "trigger_schemas=0".into(),
        "failed_triggers=1".into(),
        "failed_scopes=0".into(),
    ]);
    assert!(logs.matching("initialized with failures").is_empty(), "{:#?}", logs.lines());
    fixture.shutdown();
}
