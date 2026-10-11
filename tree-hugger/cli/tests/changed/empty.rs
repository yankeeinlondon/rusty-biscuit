//! The empty-selection rows: with deleted-only, excluded, unsupported or
//! out-of-selection changes, every report mode exits 0 with the existing
//! empty representation.

use crate::Repo;

/// The report modes a declaration command can be in: `(name, extra args)`.
const MODES: &[(&str, &[&str])] = &[
    ("terminal", &[]),
    ("plain", &["--plain"]),
    ("table", &["--format", "table"]),
    ("tree", &["--tree"]),
    ("json", &["--json"]),
    ("json-lines", &["--jsonl"]),
    ("full-json", &["--json-full"]),
    ("count", &["--count"]),
    ("stats", &["--stats"]),
    ("stats-json", &["--json", "--stats"]),
];

/// Runs one mode (plus cause-specific `extra`) and asserts the empty
/// representation of that mode.
fn assert_empty_mode(repo: &Repo, cause: &str, mode: &str, args: &[&str]) {
    let mut all: Vec<&str> = vec!["--verbose", "symbols", "--changed"];
    all.extend_from_slice(args);
    let output = repo.hug(&all);
    assert!(crate::cache_stats(&output).starts_with("cache: 0 hits, 0 partial hits, 0 misses; parsed 0 files"), "{cause} / {mode}: no per-file cache work");
    let stdout = String::from_utf8(output.stdout).expect("utf-8 stdout");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "{cause} / {mode} (hug {all:?}) must exit 0:\n{stderr}"
    );
    match mode {
        "terminal" | "plain" | "table" | "tree" => {
            assert!(stdout.contains("(no symbols)"), "{cause} / {mode}: {stdout:?}");
        }
        "json" => {
            let report = crate::json_of(&stdout);
            assert_eq!(report["files"].as_array().map(Vec::len), Some(0), "{cause} / {mode}");
            assert_eq!(report["errors"].as_array().map(Vec::len), Some(0), "{cause} / {mode}");
        }
        "full-json" => {
            let report = crate::json_of(&stdout);
            assert_eq!(report["indexes"].as_array().map(Vec::len), Some(0), "{cause} / {mode}");
            assert_eq!(report["errors"].as_array().map(Vec::len), Some(0), "{cause} / {mode}");
        }
        "json-lines" => {
            assert!(stdout.is_empty(), "{cause} / {mode}: no lines, got {stdout:?}");
        }
        "count" => {
            assert_eq!(stdout, "0\n", "{cause} / {mode}");
        }
        "stats" => {
            assert!(
                stdout.contains("Doc coverage: 0 of 0 documented (n/a)"),
                "{cause} / {mode}: {stdout}"
            );
        }
        "stats-json" => {
            let report = crate::json_of(&stdout);
            assert_eq!(report["stats"]["doc_coverage"], serde_json::json!({"documented": 0, "total": 0, "unavailable": 0, "ratio": null}), "{cause} / {mode}");
            for field in ["by_kind", "by_visibility", "by_language"] {
                assert_eq!(report["stats"][field], serde_json::json!({}), "{cause} / {mode} / {field}");
            }
            assert_eq!(report["errors"].as_array().map(Vec::len), Some(0), "{cause} / {mode}");
        }
        other => panic!("unknown mode {other}"),
    }
}

fn assert_every_mode(repo: &Repo, cause: &str, extra: &[&str]) {
    for (mode, args) in MODES {
        let mut with_extra: Vec<&str> = args.to_vec();
        with_extra.extend_from_slice(extra);
        assert_empty_mode(repo, cause, mode, &with_extra);
    }
}

#[test]
fn deleted_only_changes_render_the_empty_report_in_every_mode() {
    let repo = Repo::committed(&[("gone.rs", "fn gone() {}\n")]);
    std::fs::remove_file(repo.dir.path().join("gone.rs")).unwrap();
    crate::stage_deletion(repo.dir.path(), "gone.rs");
    assert_every_mode(&repo, "deleted-only", &[]);
}

#[test]
fn excluded_changes_render_the_empty_report_in_every_mode() {
    let repo = Repo::committed(&[("clean.rs", "fn clean() {}\n")]);
    repo.write("changed.rs", "fn changed() {}\n");
    assert_every_mode(&repo, "excluded", &["--exclude-files", "changed.rs"]);
}

#[test]
fn unsupported_changes_render_the_empty_report_in_every_mode() {
    let repo = Repo::committed(&[("clean.rs", "fn clean() {}\n")]);
    repo.write("notes.txt", "x = 1\n");
    assert_every_mode(&repo, "unsupported", &[]);
}

#[test]
fn out_of_selection_changes_render_the_empty_report_in_every_mode() {
    let repo = Repo::committed(&[("clean.rs", "fn clean() {}\n")]);
    repo.write("changed.rs", "fn changed() {}\n");
    assert_every_mode(&repo, "out-of-selection", &["--file", "clean.rs"]);
}

/// `classes` renders its own (outline) empty representation: nothing in
/// terminal output, `0` for `--count`.
#[test]
fn classes_renders_the_empty_selection_in_its_outline() {
    let repo = Repo::committed(&[("gone.rs", "struct Gone;\n")]);
    std::fs::remove_file(repo.dir.path().join("gone.rs")).unwrap();
    crate::stage_deletion(repo.dir.path(), "gone.rs");

    let terminal = repo.hug(&["classes", "--changed"]);
    assert!(
        terminal.status.success(),
        "classes --changed exits 0:\n{}",
        String::from_utf8_lossy(&terminal.stderr)
    );
    assert_eq!(String::from_utf8(terminal.stdout).unwrap(), "");

    let count = repo.stdout(&["classes", "--changed", "--count"]);
    assert_eq!(count, "0\n");
}

#[test]
fn no_source_files_remains_an_error_without_changed() {
    let repo = Repo::unborn();
    repo.write("notes.txt", "unsupported source\n");
    let ordinary = repo.hug(&["--json", "symbols"]);
    assert_eq!(ordinary.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&ordinary.stderr).starts_with("error: No supported source files found in `"), "{}", String::from_utf8_lossy(&ordinary.stderr));
    let changed = repo.json(&["--json", "symbols", "--changed"]);
    assert_eq!(changed["files"], serde_json::json!([]));
    assert_eq!(changed["errors"], serde_json::json!([]));
}
