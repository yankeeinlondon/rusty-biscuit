//! R13: the index store's log lines, each emitted once with its fields and
//! the `workspace` field, through the server.
//!
//! | Line | Level | Test |
//! | --- | --- | --- |
//! | `index store opened` (created) | `info` | `a_fill_and_a_warm_restart_log_their_lines_once` |
//! | `index store opened` (existing, with `rows`) | `info` | same |
//! | `index store revalidated` | `info` | same |
//! | `index store unavailable, using memory` | `warn` | `a_fallback_to_memory_warns_once_per_process` |
//! | `index store recreated` | `warn` | `a_recreated_store_and_a_sweep_log_their_lines_once` |
//! | `index store sweep` (removed stores) | `info` | same |
//!
//! Each test installs a process-wide capturing subscriber, because the lines
//! come from DMLS's build-round and dispatch threads; nextest runs each test
//! in its own process, so one global subscriber per test is allowed, and the
//! once-per-process sweep and fallback `warn` start fresh in each.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use crate::common;

use common::{LspFixture, LspWorkspace};
use dmls::index_store::{FACTS_VERSION, store_path, touch_mtime};
use dmls::log_capture::CapturedLogs;
use serde_json::{Value, json};

/// Text that must never reach a log line: a heading title and body words.
const SECRET_HEADING: &str = "Quarterly Secret Plans";
const SECRET_BODY: &str = "confidential-body-words";

fn initialize_params(roots: &[&Path]) -> Value {
    let folders: Vec<Value> = roots
        .iter()
        .map(|root| json!({ "uri": url::Url::from_directory_path(root).unwrap().as_str(), "name": "logs" }))
        .collect();
    json!({
        "processId": null,
        "clientInfo": { "name": "Log Client", "version": "1.0" },
        "capabilities": {
            "general": { "positionEncodings": ["utf-16"] },
            "workspace": { "didChangeWatchedFiles": { "dynamicRegistration": true } }
        },
        "workspaceFolders": folders
    })
}

fn workspace_id(root: &Path) -> String {
    url::Url::from_directory_path(root).unwrap().to_string()
}

fn write(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

/// Three Markdown files, one holding the secret heading and body.
fn known_workspace() -> LspWorkspace {
    let workspace = LspWorkspace::new();
    let root = workspace.path();
    write(&root.join("a.md"), "# A\n\nSee [b](b.md#b).\n");
    write(&root.join("b.md"), "# B\n");
    write(&root.join("notes/secret.md"), &format!("# {SECRET_HEADING}\n\n{SECRET_BODY}\n"));
    workspace
}

/// A session over `roots` whose stores live in `cache` (or are `off`).
fn start<'w>(workspace: &'w LspWorkspace, cache: &str, roots: &[&Path]) -> LspFixture<'w> {
    let env = HashMap::from([("DMLS_CACHE_DIR".to_string(), cache.to_string())]);
    let snapshot = LspFixture::fixture_snapshot(workspace).with_env(env);
    let mut fixture = LspFixture::start_with_snapshot(workspace, snapshot);
    fixture.initialize(initialize_params(roots));
    fixture
}

fn symbols(fixture: &mut LspFixture<'_>) {
    let response = fixture.request("workspace/symbol", json!({ "query": "" }));
    assert!(response.error.is_none(), "{:?}", response.error);
}

fn references(fixture: &mut LspFixture<'_>, path: &Path) {
    let response = fixture.request(
        "textDocument/references",
        json!({
            "textDocument": { "uri": url::Url::from_file_path(path).unwrap().as_str() },
            "position": { "line": 0, "character": 2 },
            "context": { "includeDeclaration": false }
        }),
    );
    assert!(response.error.is_none(), "{:?}", response.error);
}

/// The one captured line containing `message` at `level`, asserting it
/// carries every `key=value` in `fields` and none of `absent` (field names).
fn one_line(logs: &CapturedLogs, level: &str, message: &str, fields: &[String], absent: &[&str]) -> String {
    one_line_in(&logs.lines(), level, message, fields, absent)
}

/// [`one_line`] over `lines`.
fn one_line_in(lines: &[String], level: &str, message: &str, fields: &[String], absent: &[&str]) -> String {
    let matching: Vec<&String> = lines.iter().filter(|line| line.contains(message)).collect();
    assert_eq!(matching.len(), 1, "one `{message}` line: {lines:#?}");
    let line = matching[0].clone();
    assert!(line.contains(&format!(" {level} ")), "level {level}: {line}");
    for field in fields {
        assert_eq!(line.matches(&format!(" {field}")).count(), 1, "`{field}` once in: {line}");
    }
    for name in absent {
        assert!(!line.contains(&format!(" {name}=")), "no `{name}` in: {line}");
    }
    line
}

/// No line names a heading title or body text.
fn assert_no_contents(logs: &CapturedLogs) {
    for line in logs.lines() {
        assert!(!line.contains(SECRET_HEADING) && !line.contains(SECRET_BODY), "contents logged: {line}");
    }
}

/// The store's file name inside its directory, as the `path` field shows it
/// (the directory spelling depends on the OS, the file name does not).
fn store_file_name(cache: &Path, root: &Path) -> String {
    store_path(cache, root).file_name().unwrap().to_string_lossy().into_owned()
}

/// A cold fill logs a created store and a revalidation that parsed every
/// file; a restart over the same store logs an existing store with its row
/// count and a revalidation that parsed nothing. Nothing falls back or is
/// swept.
#[test]
fn a_fill_and_a_warm_restart_log_their_lines_once() {
    let logs = CapturedLogs::global();
    let workspace = known_workspace();
    let root = workspace.path().to_path_buf();
    let cache = workspace.path().join("cache");
    let id = workspace_id(&root);
    let store = store_file_name(&cache, &root);

    let mut fixture = start(&workspace, cache.to_str().unwrap(), &[&root]);
    symbols(&mut fixture);
    fixture.shutdown();
    let opened = one_line(&logs, "INFO", "index store opened", &[
        "created=true".into(),
        format!("facts_version={FACTS_VERSION}"),
        format!("workspace={id}"),
    ], &["rows"]);
    assert!(opened.contains(&store), "{opened}");
    one_line(&logs, "INFO", "index store revalidated", &[
        "markdown=3".into(),
        "unchanged=0".into(),
        "reparsed=3".into(),
        "removed=0".into(),
        "trigger=workspace-symbols".into(),
        format!("workspace={id}"),
    ], &[]);
    assert!(logs.matching("elapsed_ms=").iter().any(|line| line.contains("index store revalidated")));

    // The restart: a second session in this process, over the same store.
    let before = logs.lines().len();
    let mut fixture = start(&workspace, cache.to_str().unwrap(), &[&root]);
    references(&mut fixture, &root.join("b.md"));
    fixture.shutdown();
    let restart = logs.lines()[before..].to_vec();
    let reopened = one_line_in(&restart, "INFO", "index store opened", &[
        "created=false".into(),
        format!("facts_version={FACTS_VERSION}"),
        "rows=3".into(),
        format!("workspace={id}"),
    ], &[]);
    assert!(reopened.contains(&store), "{reopened}");
    one_line_in(&restart, "INFO", "index store revalidated", &[
        "markdown=3".into(),
        "unchanged=3".into(),
        "reparsed=0".into(),
        "removed=0".into(),
        "trigger=references".into(),
        format!("workspace={id}"),
    ], &[]);

    assert!(logs.matching("using memory").is_empty(), "{:#?}", logs.lines());
    assert!(logs.matching("index store recreated").is_empty(), "{:#?}", logs.lines());
    assert!(logs.matching("index store sweep").iter().all(|line| line.trim_start().starts_with("DEBUG")), "nothing to sweep");
    assert_no_contents(&logs);
}

/// `DMLS_CACHE_DIR=off` over two roots: one `warn` for the whole process
/// (the second root's fallback is `debug`), and the revalidation line still
/// reports the fill.
#[test]
fn a_fallback_to_memory_warns_once_per_process() {
    let logs = CapturedLogs::global();
    let workspace = known_workspace();
    let root = workspace.path().to_path_buf();
    let second = root.join("notes");
    let id = workspace_id(&root);

    let mut fixture = start(&workspace, "off", &[&root, &second]);
    symbols(&mut fixture);
    references(&mut fixture, &root.join("b.md"));
    fixture.shutdown();

    let warns: Vec<String> = logs.matching("index store unavailable").into_iter().filter(|line| line.contains(" WARN ")).collect();
    assert_eq!(warns.len(), 1, "{:#?}", logs.lines());
    let warn = one_line(&logs, "WARN", "index store unavailable, using memory", &[
        "reason=disabled".into(),
        "path=".into(),
        format!("workspace={id}"),
    ], &[]);
    assert!(warn.contains("path= ") || warn.ends_with("path="), "no store path when disabled: {warn}");
    assert_eq!(logs.matching("already using memory").len(), 1, "the second root: {:#?}", logs.lines());
    assert!(logs.matching("index store opened").is_empty(), "{:#?}", logs.lines());
    one_line(&logs, "INFO", "index store revalidated", &[
        "markdown=3".into(),
        "reparsed=3".into(),
        "trigger=workspace-symbols".into(),
        format!("workspace={id}"),
    ], &[]);
    assert_no_contents(&logs);
}

/// A corrupt store is recreated with one `warn` naming it and the reason,
/// then opened as created; a stale store of another root in the same
/// directory is swept, logged once at `info` with its counts.
#[test]
fn a_recreated_store_and_a_sweep_log_their_lines_once() {
    let logs = CapturedLogs::global();
    let workspace = known_workspace();
    let root = workspace.path().to_path_buf();
    let cache = workspace.path().join("cache");
    let id = workspace_id(&root);
    std::fs::create_dir_all(&cache).unwrap();
    std::fs::write(store_path(&cache, &root), b"this is not a database").unwrap();
    let stale: PathBuf = store_path(&cache, &workspace.path().join("gone"));
    let stale_lock = PathBuf::from(format!("{}.lock", stale.display()));
    let month_ago = SystemTime::now() - Duration::from_secs(31 * 24 * 60 * 60);
    for file in [&stale, &stale_lock] {
        std::fs::write(file, b"").unwrap();
        touch_mtime(file, month_ago).unwrap();
    }

    let mut fixture = start(&workspace, cache.to_str().unwrap(), &[&root]);
    symbols(&mut fixture);
    fixture.shutdown();

    let recreated = one_line(&logs, "WARN", "index store recreated", &[
        "reason=corrupt".into(),
        format!("workspace={id}"),
    ], &[]);
    assert!(recreated.contains(&store_file_name(&cache, &root)), "{recreated}");
    one_line(&logs, "INFO", "index store opened", &["created=true".into()], &["rows"]);
    one_line(&logs, "INFO", "index store sweep", &[
        "removed=1".into(),
        "skipped_in_use=0".into(),
        "elapsed_ms=".into(),
        format!("workspace={id}"),
    ], &[]);
    assert!(!stale.exists() && !stale_lock.exists());
    assert!(logs.matching("using memory").is_empty(), "{:#?}", logs.lines());
    assert_no_contents(&logs);
}
