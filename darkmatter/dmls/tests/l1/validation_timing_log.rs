//! Level-1 sessions for the slow-validation log: the one event each
//! diagnostics refresh logs for its document's schema validation
//! (`slow schema validation` at `warn`, `schema validation complete` at
//! `debug`), its threshold setting `schema.slow_validation_ms`, and the
//! compile-cache state it reports.
//!
//! Assertions name fields, never their timing values, which depend on host
//! load. Each test installs the process-wide capture, which nextest's
//! process-per-test model permits.

use crate::common;

use std::path::Path;

use common::{LspFixture, LspWorkspace};
use dmls::config::{CONFIG_FILE_NAME, ConfigSeverity, ConfigSource, ConfigState};
use dmls::log_capture::CapturedLogs;
use serde_json::{Value, json};

const SLOW: &str = "slow schema validation";
const FAST: &str = "schema validation complete";

/// Every field the event carries, in order.
const FIELDS: [&str; 10] = [
    "path",
    "matched_triggers",
    "discover_ms",
    "match_ms",
    "resolve_ms",
    "compile_ms",
    "validate_ms",
    "total_ms",
    "unassigned_ms",
    "compile_cache",
];

/// A structural-only schema: no file format and no `match()`.
const SHAPE: &str = "$schema:\n  title: string\n  count: number\n";

/// A trigger whose payload declares `title`, matching every document with one.
const ENVELOPE: &str = "kind: schema-trigger\nmatch:\n  title: string(required)\n$schema: titled.yaml\n";
const TITLED: &str = "$schema:\n  title: string\n";

/// A value no log line may carry.
const SECRET: &str = "secretvalue";

fn write(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

/// A minimal repository Git discovery accepts, without running `git`.
fn repository(root: &Path) {
    std::fs::create_dir_all(root.join(".git/objects")).unwrap();
    std::fs::create_dir_all(root.join(".git/refs/heads")).unwrap();
    write(&root.join(".git/HEAD"), "ref: refs/heads/main\n");
    write(&root.join(".git/config"), "[core]\n\trepositoryformatversion = 0\n\tbare = false\n");
}

fn uri(path: &Path) -> String {
    url::Url::from_file_path(path).unwrap().to_string()
}

fn params(folder: &Path) -> Value {
    json!({
        "processId": null,
        "clientInfo": { "name": "Slow Validation Client", "version": "1.0" },
        "capabilities": { "general": { "positionEncodings": ["utf-16"] } },
        "workspaceFolders": [
            { "uri": url::Url::from_directory_path(folder).unwrap().as_str(), "name": "w" }
        ]
    })
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
        json!({
            "textDocument": { "uri": uri, "version": version },
            "contentChanges": [ { "text": text } ]
        }),
    );
}

/// The validation events logged for the document at `path`, in order.
fn events(logs: &CapturedLogs, path: &Path) -> Vec<String> {
    let path = format!("path={:?}", path.to_string_lossy());
    logs.lines()
        .into_iter()
        .filter(|line| (line.contains(SLOW) || line.contains(FAST)) && line.contains(&path))
        .collect()
}

/// Waits for `uri`'s next diagnostics and returns the one new validation
/// event the refresh logged for `path`, after checking its level, message,
/// and that it carries exactly [`FIELDS`] and no document value.
fn next_event(
    fixture: &mut LspFixture<'_>,
    logs: &CapturedLogs,
    uri: &str,
    path: &Path,
    seen: &mut usize,
    slow: bool,
) -> (String, Vec<Value>) {
    let diagnostics = fixture.wait_for_diagnostics(uri);
    let all = events(logs, path);
    assert_eq!(all.len(), *seen + 1, "one event per refresh: {all:#?}");
    *seen += 1;
    let line = all.last().unwrap().clone();
    let (level, message) = if slow { (" WARN ", SLOW) } else { ("DEBUG ", FAST) };
    assert!(line.starts_with(level) && line.contains(message), "{line}");
    assert_eq!(field_names(&line, message), FIELDS, "{line}");
    assert!(!line.contains(SECRET), "a document value leaked: {line}");
    (line, diagnostics)
}

/// The `key` of every `key=value` field after `message`.
fn field_names(line: &str, message: &str) -> Vec<String> {
    let fields = line.split_once(message).expect("message").1;
    fields
        .split_whitespace()
        .filter_map(|field| field.split_once('=').map(|(key, _)| key.to_string()))
        .collect()
}

fn compile_cache(line: &str) -> &str {
    let value = line.split_once("compile_cache=").expect("compile_cache").1;
    value.trim_matches('"')
}

fn session_settings(fixture: &LspFixture<'_>, threshold: Value) {
    fixture.notify(
        "workspace/didChangeConfiguration",
        json!({ "settings": { "dmls": { "schema": { "slow_validation_ms": threshold } } } }),
    );
}

fn type_mismatch_lines(diagnostics: &[Value]) -> Vec<u64> {
    diagnostics
        .iter()
        .filter(|diagnostic| diagnostic["code"] == json!("dm.schema.type_mismatch"))
        .map(|diagnostic| diagnostic["range"]["start"]["line"].as_u64().unwrap())
        .collect()
}

// ── The event ───────────────────────────────────────────────────────────────

/// With a zero threshold every validation is a `warn`; the first one of a
/// cold process compiles (`miss`), and validating the unchanged document
/// again reuses the overlay's bundle (`hit`). The trigger that matched is
/// named by its file name.
#[test]
fn a_zero_threshold_logs_every_validation_at_warn() {
    let logs = CapturedLogs::global();
    let workspace = LspWorkspace::new();
    let root = workspace.path().to_path_buf();
    repository(&root);
    write(&root.join(CONFIG_FILE_NAME), "[schema]\nslow_validation_ms = 0\n");
    write(&root.join("schemas/titled.trigger.yaml"), ENVELOPE);
    write(&root.join("schemas/titled.yaml"), TITLED);
    write(&root.join("shape.yaml"), SHAPE);
    let doc = root.join("a.md");
    let text = format!("---\n$schema: ./shape.yaml\ntitle: {SECRET}\ncount: 1\n---\n\n# A\n");
    write(&doc, &text);

    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(params(&root));
    let doc_uri = uri(&doc);
    let mut seen = 0;
    open(&fixture, &doc_uri, &text);
    let (first, _) = next_event(&mut fixture, &logs, &doc_uri, &doc, &mut seen, true);
    assert_eq!(compile_cache(&first), "miss", "{first}");
    assert!(first.contains("matched_triggers=\"titled.trigger.yaml\""), "{first}");

    change(&fixture, &doc_uri, 2, &text);
    let (second, _) = next_event(&mut fixture, &logs, &doc_uri, &doc, &mut seen, true);
    assert_eq!(compile_cache(&second), "hit", "{second}");
    fixture.shutdown();
}

/// A threshold no validation reaches logs each one at `debug`.
#[test]
fn a_validation_below_the_threshold_logs_at_debug() {
    let logs = CapturedLogs::global();
    let workspace = LspWorkspace::new();
    let root = workspace.path().to_path_buf();
    repository(&root);
    write(&root.join(CONFIG_FILE_NAME), "[schema]\nslow_validation_ms = 3600000\n");
    let doc = root.join("a.md");
    let text = format!("---\ntitle: {SECRET}\n---\n\n# A\n");

    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(params(&root));
    let doc_uri = uri(&doc);
    let mut seen = 0;
    open(&fixture, &doc_uri, &text);
    let (line, _) = next_event(&mut fixture, &logs, &doc_uri, &doc, &mut seen, false);
    assert!(line.contains("matched_triggers=\"\""), "no trigger matched: {line}");
    fixture.shutdown();
}

/// The editor's threshold overrides the file's, and an invalid editor value
/// falls back to the file's.
#[test]
fn the_editor_setting_overrides_the_file_threshold() {
    let logs = CapturedLogs::global();
    let workspace = LspWorkspace::new();
    let root = workspace.path().to_path_buf();
    repository(&root);
    write(&root.join(CONFIG_FILE_NAME), "[schema]\nslow_validation_ms = 0\n");
    let doc = root.join("a.md");
    let text = "---\ntitle: A\n---\n\n# A\n";

    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(params(&root));
    let doc_uri = uri(&doc);
    let mut seen = 0;
    open(&fixture, &doc_uri, text);
    next_event(&mut fixture, &logs, &doc_uri, &doc, &mut seen, true);

    // A configuration change re-publishes every open document.
    session_settings(&fixture, json!(3_600_000));
    next_event(&mut fixture, &logs, &doc_uri, &doc, &mut seen, false);

    session_settings(&fixture, json!(-1));
    next_event(&mut fixture, &logs, &doc_uri, &doc, &mut seen, true);
    fixture.shutdown();
}

// ── Compile-cache states ────────────────────────────────────────────────────

/// Two documents with the same structural-only schema: the second reuses
/// the first one's compiled validator (`hit`), and each document's
/// diagnostic sits on its own line.
#[test]
fn a_second_document_with_the_same_structural_schema_hits_and_keeps_its_positions() {
    let logs = CapturedLogs::global();
    let workspace = LspWorkspace::new();
    let root = workspace.path().to_path_buf();
    repository(&root);
    write(&root.join(CONFIG_FILE_NAME), "[schema]\nslow_validation_ms = 0\n");
    write(&root.join("shape.yaml"), SHAPE);
    let first = root.join("first.md");
    let second = root.join("second.md");
    // `count` is invalid on line 3 of the first and line 4 of the second.
    let first_text = "---\n$schema: ./shape.yaml\ntitle: One\ncount: many\n---\n\n# One\n";
    let second_text = "---\n$schema: ./shape.yaml\ntitle: Two\n\ncount: several\n---\n\n# Two\n";

    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(params(&root));
    let (first_uri, second_uri) = (uri(&first), uri(&second));
    let (mut first_seen, mut second_seen) = (0, 0);
    open(&fixture, &first_uri, first_text);
    let (line, diagnostics) = next_event(&mut fixture, &logs, &first_uri, &first, &mut first_seen, true);
    assert_eq!(compile_cache(&line), "miss", "{line}");
    assert_eq!(type_mismatch_lines(&diagnostics), vec![3], "{diagnostics:#?}");

    open(&fixture, &second_uri, second_text);
    let (line, diagnostics) = next_event(&mut fixture, &logs, &second_uri, &second, &mut second_seen, true);
    assert_eq!(compile_cache(&line), "hit", "{line}");
    assert_eq!(type_mismatch_lines(&diagnostics), vec![4], "{diagnostics:#?}");
    fixture.shutdown();
}

/// A schema whose file arm sits below a union judges each document in its
/// own folder: the same value is valid beside the file it names and invalid
/// where that file is absent. The second document's request builds its own
/// context-keyed validator, so it is a `miss`.
#[test]
fn a_file_arm_below_a_union_keeps_each_documents_context() {
    let logs = CapturedLogs::global();
    let workspace = LspWorkspace::new();
    let root = workspace.path().to_path_buf();
    repository(&root);
    write(&root.join(CONFIG_FILE_NAME), "[schema]\nslow_validation_ms = 0\n");
    write(&root.join("refs.yaml"), "$schema:\n  include: file(eager) | number\n");
    write(&root.join("present/beside.md"), "# Beside\n");
    write(&root.join("absent/other.md"), "# Other\n");
    let present = root.join("present").join("doc.md");
    let absent = root.join("absent").join("doc.md");
    let text = "---\n$schema: ../refs.yaml\ninclude: ./beside.md\n---\n\n# Doc\n";

    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(params(&root));
    let (present_uri, absent_uri) = (uri(&present), uri(&absent));
    let (mut present_seen, mut absent_seen) = (0, 0);
    open(&fixture, &present_uri, text);
    let (_, diagnostics) = next_event(&mut fixture, &logs, &present_uri, &present, &mut present_seen, true);
    let codes = |diagnostics: &[Value]| -> Vec<String> {
        diagnostics.iter().map(|diagnostic| diagnostic["code"].as_str().unwrap_or_default().to_string()).collect()
    };
    assert!(!codes(&diagnostics).iter().any(|code| code.starts_with("dm.schema")), "{diagnostics:#?}");

    open(&fixture, &absent_uri, text);
    let (line, diagnostics) = next_event(&mut fixture, &logs, &absent_uri, &absent, &mut absent_seen, true);
    assert!(
        codes(&diagnostics).iter().any(|code| code.starts_with("dm.schema")),
        "the absent file is reported: {diagnostics:#?}"
    );
    assert_eq!(compile_cache(&line), "miss", "{line}");
    fixture.shutdown();
}

/// A root union of a structural arm and a file arm, opened in two folders:
/// the second request finds the structural arm's validator cached and builds
/// its own context-keyed file arm. A request that built any validator is a
/// `miss`, however many it found cached.
#[test]
fn a_request_mixing_cached_and_built_validators_is_a_miss() {
    let logs = CapturedLogs::global();
    let workspace = LspWorkspace::new();
    let root = workspace.path().to_path_buf();
    repository(&root);
    write(&root.join(CONFIG_FILE_NAME), "[schema]\nslow_validation_ms = 0\n");
    write(&root.join("shape.yaml"), SHAPE);
    write(&root.join("refs.yaml"), "$schema:\n  include: file(eager)\n");
    let first = root.join("one").join("doc.md");
    let second = root.join("two").join("doc.md");
    write(&first, "# One\n");
    write(&second, "# Two\n");
    let text = "---\n$schema:\n  - ../shape.yaml\n  - ../refs.yaml\ntitle: A\n---\n\n# Doc\n";

    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(params(&root));
    let (first_uri, second_uri) = (uri(&first), uri(&second));
    let (mut first_seen, mut second_seen) = (0, 0);
    open(&fixture, &first_uri, text);
    let (line, _) = next_event(&mut fixture, &logs, &first_uri, &first, &mut first_seen, true);
    assert_eq!(compile_cache(&line), "miss", "{line}");
    open(&fixture, &second_uri, text);
    let (line, diagnostics) = next_event(&mut fixture, &logs, &second_uri, &second, &mut second_seen, true);
    assert_eq!(compile_cache(&line), "miss", "{line}");
    assert!(
        !diagnostics.iter().any(|diagnostic| diagnostic["code"] == json!("dm.schema.prepare")),
        "the union resolves: {diagnostics:#?}"
    );
    fixture.shutdown();
}

/// The runtime baseline applies to every document with frontmatter, so a
/// document gets no schema at all only when its context cannot be built: it
/// looks no validator up (`not requested`), and the event is still logged.
#[test]
fn a_document_without_a_schema_requests_no_compilation() {
    let logs = CapturedLogs::global();
    let workspace = LspWorkspace::new();
    let root = workspace.path().to_path_buf();
    repository(&root);
    // A syntactically corrupt config makes repository discovery fail.
    write(&root.join(".git/config"), "[core\nthis is not = = valid\n");
    write(&root.join(CONFIG_FILE_NAME), "[schema]\nslow_validation_ms = 0\n");
    let doc = root.join("a.md");
    let text = "---\ntitle: A\n---\n\n# A\n";

    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(params(&root));
    let doc_uri = uri(&doc);
    let mut seen = 0;
    open(&fixture, &doc_uri, text);
    let (line, diagnostics) = next_event(&mut fixture, &logs, &doc_uri, &doc, &mut seen, true);
    assert_eq!(compile_cache(&line), "not requested", "{line}");
    assert!(
        diagnostics.iter().any(|diagnostic| diagnostic["code"] == json!("dm.context.build_failure")),
        "{diagnostics:#?}"
    );
    fixture.shutdown();
}

/// An unresolvable `$schema` fails the request: the event still carries
/// every field, and the document's schema diagnostic is unchanged. Its
/// compile state is not asserted: the first request in a process builds the
/// baseline's own validator before resolving `$schema`.
#[test]
fn an_unresolvable_schema_still_logs_its_partial_timings() {
    let logs = CapturedLogs::global();
    let workspace = LspWorkspace::new();
    let root = workspace.path().to_path_buf();
    repository(&root);
    write(&root.join(CONFIG_FILE_NAME), "[schema]\nslow_validation_ms = 0\n");
    let doc = root.join("a.md");
    let text = "---\n$schema: ./missing.yaml\ntitle: A\n---\n\n# A\n";

    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(params(&root));
    let doc_uri = uri(&doc);
    let mut seen = 0;
    open(&fixture, &doc_uri, text);
    let (_, diagnostics) = next_event(&mut fixture, &logs, &doc_uri, &doc, &mut seen, true);
    let prepare: Vec<&Value> =
        diagnostics.iter().filter(|diagnostic| diagnostic["code"] == json!("dm.schema.prepare")).collect();
    assert_eq!(prepare.len(), 1, "{diagnostics:#?}");
    assert_eq!(prepare[0]["source"], json!("darkmatter.schema"));
    assert_eq!(prepare[0]["severity"], json!(1));
    assert!(prepare[0]["message"].as_str().unwrap().contains("missing.yaml"), "{diagnostics:#?}");
    fixture.shutdown();
}

// ── The setting ─────────────────────────────────────────────────────────────

fn load(toml: Option<&str>) -> (tempfile::TempDir, ConfigState) {
    let temp = tempfile::tempdir().unwrap();
    if let Some(toml) = toml {
        std::fs::write(temp.path().join(CONFIG_FILE_NAME), toml).unwrap();
    }
    let state = ConfigState::load(vec![temp.path().to_path_buf()], None);
    (temp, state)
}

fn threshold(state: &ConfigState) -> u64 {
    state.effective().schema.slow_validation_ms
}

#[test]
fn the_threshold_defaults_to_200_ms() {
    for toml in [None, Some(""), Some("[schema]\nstrict = true\n")] {
        let (_temp, state) = load(toml);
        assert_eq!(threshold(&state), 200, "{toml:?}");
        assert!(state.diagnostics().is_empty(), "{toml:?}");
    }
}

#[test]
fn the_editor_threshold_overrides_the_file_and_null_settings_restore_it() {
    let (_temp, mut state) = load(Some("[schema]\nslow_validation_ms = 0\n"));
    assert_eq!(threshold(&state), 0);
    state.apply_client_settings(json!({ "dmls": { "schema": { "slow_validation_ms": 750 } } }));
    assert_eq!(threshold(&state), 750);
    // Other `schema` keys from the editor leave the file's threshold.
    state.apply_client_settings(json!({ "dmls": { "schema": { "strict": true } } }));
    assert_eq!(threshold(&state), 0);
    state.apply_client_settings(Value::Null);
    assert_eq!(threshold(&state), 0);
    assert!(state.diagnostics().is_empty(), "{:?}", state.diagnostics());
}

/// An invalid editor value is the existing invalid-settings error and the
/// file layer stays; an invalid file value rejects the file, so the
/// default stays.
#[test]
fn an_invalid_threshold_is_a_config_error_and_keeps_the_lower_layer() {
    for value in [json!(-1), json!(1.5), json!("200"), Value::Null] {
        let (_temp, mut state) = load(Some("[schema]\nslow_validation_ms = 40\n"));
        state.apply_client_settings(json!({ "dmls": { "schema": { "slow_validation_ms": value } } }));
        assert_eq!(threshold(&state), 40, "{value}");
        let [finding] = state.diagnostics() else {
            panic!("{value}: one finding: {:?}", state.diagnostics());
        };
        assert_eq!((finding.severity, &finding.source), (ConfigSeverity::Error, &ConfigSource::EditorSettings));
        assert!(finding.message.contains("the file and default settings remain"), "{finding}");
    }
    for toml in ["slow_validation_ms = -1", "slow_validation_ms = 1.5", "slow_validation_ms = \"200\""] {
        let (_temp, state) = load(Some(&format!("[schema]\n{toml}\n")));
        assert_eq!(threshold(&state), 200, "{toml}");
        assert_eq!(state.file_path(), None, "{toml}: the file was rejected");
        let [finding] = state.diagnostics() else {
            panic!("{toml}: one finding: {:?}", state.diagnostics());
        };
        assert_eq!(finding.severity, ConfigSeverity::Error, "{toml}");
        assert!(matches!(finding.source, ConfigSource::File(_)), "{toml}");
    }
}
