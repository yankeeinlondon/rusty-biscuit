//! The declaration commands' JSON report: the envelope and its records, the
//! command kind sets, `--json-full`, failed files, and output failures.

use std::io::Read;
use std::process::{Output, Stdio};

use serde_json::Value;
use tree_hugger::SymbolRecord;

use crate::golden::MULTI_FIXTURES;
use crate::{Scratch, json_of};

const FUNCTION_KINDS: [&str; 3] = ["Function", "Method", "Constructor"];
const TYPE_KINDS: [&str; 6] = ["Type", "Class", "Interface", "Enum", "Trait", "TypeAlias"];

/// Bytes that are not UTF-8: `hug` cannot read the file as source.
const NOT_UTF8: &[u8] = &[0x66, 0x6e, 0x20, 0xff, 0xfe, 0x0a];

fn records(json: &Value) -> Vec<&Value> {
    json["files"]
        .as_array()
        .expect("files")
        .iter()
        .flat_map(|file| file["symbols"].as_array().expect("symbols"))
        .collect()
}

fn file_names(json: &Value, key: &str) -> Vec<String> {
    json[key]
        .as_array()
        .unwrap_or_else(|| panic!("{key}"))
        .iter()
        .map(|entry| entry["file"].as_str().expect("file").to_string())
        .collect()
}

fn stderr_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// The envelope carries its versions, the scan root, and per file the
/// language, the passes and capabilities that ran, and only selected
/// records, which read back as `SymbolRecord`s; `errors` is present and last.
#[test]
fn the_json_envelope_holds_only_selected_records_and_ends_with_errors() {
    let scratch = Scratch::new(MULTI_FIXTURES);
    let stdout = scratch.stdout(&["--json", "functions"]);
    assert!(
        stdout.starts_with(r#"{"report_schema_version":1,"schema_version":{"major":4,"minor":0},"root_dir":"#),
        "envelope head: {}",
        &stdout[..120.min(stdout.len())]
    );
    assert!(stdout.ends_with("],\"errors\":[]}\n"), "`errors` closes the report");
    let json = json_of(&stdout);
    assert!(json["root_dir"].as_str().is_some_and(|root| !root.is_empty()));

    let files = json["files"].as_array().expect("files");
    assert!(files.len() >= 14, "most multi/ files declare a function ({})", files.len());
    for file in files {
        let name = file["file"].as_str().expect("file");
        assert!(!name.contains('\\') && !name.starts_with('/'), "{name}: portable and relative");
        assert_eq!(file["completed_passes"], serde_json::json!(["Parse"]), "{name}");
        assert_eq!(file["completed_capabilities"], serde_json::json!(["declarations"]), "{name}");
        assert!(file.get("members").is_none(), "{name}: only `classes` reports members");
        for record in file["symbols"].as_array().expect("symbols") {
            assert!(FUNCTION_KINDS.contains(&record["kind"].as_str().unwrap_or_default()), "{name}: {record}");
            assert_eq!(record["source"]["file_path"], file["file"], "{name}: record path");
            assert_eq!(record["language"], file["language"], "{name}: record language");
            for absent in ["declaration_text", "body_text", "source_text"] {
                assert!(record["source"].get(absent).is_none(), "{name}: {absent} omitted");
            }
            assert!(record["relations"].as_object().is_some_and(|map| map.is_empty()), "{name}: no relations");
            assert!(record["semantics"].as_object().is_some_and(|map| map.is_empty()), "{name}: no semantics");
            let parsed: SymbolRecord =
                serde_json::from_value(record.clone()).expect("a report record is a SymbolRecord");
            assert_eq!(serde_json::to_value(&parsed).expect("serialize"), *record, "{name}: round trip");
        }
    }
    let language_of = |file: &str| {
        files
            .iter()
            .find(|entry| entry["file"] == file)
            .map(|entry| entry["language"].clone())
            .unwrap_or_else(|| panic!("{file} has an entry"))
    };
    assert_eq!(language_of("nested/go.go"), "Go");
    assert_eq!(language_of("rust.rs"), "Rust");
}

/// `functions` and `types` select exactly their kind sets; `symbols`
/// selects every kind and lists each record of both.
#[test]
fn functions_and_types_select_exactly_their_kind_sets() {
    let scratch = Scratch::new(MULTI_FIXTURES);
    let all = json_of(&scratch.stdout(&["--json", "symbols"]));
    let ids_of = |json: &Value, kinds: &[&str]| -> Vec<Value> {
        let mut ids: Vec<Value> = records(json)
            .into_iter()
            .filter(|record| kinds.is_empty() || kinds.contains(&record["kind"].as_str().unwrap_or_default()))
            .map(|record| record["id"].clone())
            .collect();
        ids.sort_by_key(Value::to_string);
        ids
    };
    for (command, kinds) in [("functions", &FUNCTION_KINDS[..]), ("types", &TYPE_KINDS[..])] {
        let json = json_of(&scratch.stdout(&["--json", command]));
        let listed = ids_of(&json, &[]);
        assert!(!listed.is_empty(), "{command} lists records");
        assert_eq!(listed, ids_of(&json, kinds), "{command}: only its kinds");
        assert_eq!(listed, ids_of(&all, kinds), "{command}: every record of its kinds");
    }
}

/// A selection with no matches is `files: []` and `errors: []`.
#[test]
fn an_empty_selection_is_an_empty_files_array() {
    let scratch = Scratch::new(&[("values.py", "LIMIT = 3\n")]);
    for command in ["functions", "types", "classes"] {
        let stdout = scratch.stdout(&["--json", command, "values.py"]);
        let json = json_of(&stdout);
        assert_eq!(json["files"], serde_json::json!([]), "{command}");
        assert_eq!(json["errors"], serde_json::json!([]), "{command}");
    }
}

/// A file that cannot be read is listed once in `errors` (portable path,
/// `kind`, message) after the good files, reported on stderr, and makes the
/// run exit non-zero, for `--json`, `--json-full` and the terminal listing.
#[test]
fn a_failed_file_is_reported_last_on_stderr_and_in_the_exit_status() {
    let scratch = Scratch::new(&[("good.rs", "pub fn listed() {}\n"), ("nested/keep.py", "def kept():\n    pass\n")]);
    std::fs::write(scratch.repo.path().join("nested/bad.rs"), NOT_UTF8).expect("write bad file");

    for (args, key) in [
        (&["--json", "functions"][..], "files"),
        (&["--json-full", "functions"][..], "indexes"),
    ] {
        let output = scratch.hug(args);
        assert_eq!(output.status.code(), Some(1), "hug {args:?} exits 1");
        let stderr = stderr_of(&output);
        assert!(stderr.contains("error:") && stderr.contains("bad.rs"), "hug {args:?} stderr: {stderr}");
        let stdout = String::from_utf8(output.stdout).expect("utf-8");
        let json = json_of(&stdout);
        assert_eq!(file_names(&json, key), ["good.rs", "nested/keep.py"], "hug {args:?}: good files listed");
        let errors = json["errors"].as_array().expect("errors");
        assert_eq!(errors.len(), 1, "hug {args:?}: {errors:?}");
        assert_eq!(errors[0]["file"], "nested/bad.rs");
        assert_eq!(errors[0]["kind"], "io");
        assert!(errors[0]["message"].as_str().is_some_and(|message| message.contains("bad.rs")));
        assert!(stdout.trim_end().ends_with("}]}"), "hug {args:?}: `errors` is last");
    }

    let output = scratch.hug(&["--plain", "functions"]);
    assert_eq!(output.status.code(), Some(1), "terminal listing exits 1");
    assert!(stderr_of(&output).contains("bad.rs"));
    let stdout = String::from_utf8(output.stdout).expect("utf-8");
    assert!(stdout.contains("listed()") && stdout.contains("kept()"), "{stdout}");
}

/// Failures are reported in file order, on stderr and in `errors`, whatever
/// the number of workers.
#[test]
fn failures_are_reported_in_file_order_with_any_thread_count() {
    let mut files: Vec<(String, String)> = Vec::new();
    for index in 0..24 {
        files.push((format!("good_{index:02}.rs"), format!("pub fn good_{index:02}() {{}}\n")));
    }
    let owned: Vec<(&str, &str)> = files.iter().map(|(name, body)| (name.as_str(), body.as_str())).collect();
    let scratch = Scratch::new(&owned);
    for name in ["b_bad.rs", "m_bad.rs", "zz/bad.rs", "a_bad.rs"] {
        let path = scratch.repo.path().join(name);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("dir");
        std::fs::write(path, NOT_UTF8).expect("write bad file");
    }
    let expected = ["a_bad.rs", "b_bad.rs", "m_bad.rs", "zz/bad.rs"];

    let args = ["--json", "functions", "--no-cache"];
    let mut reports = Vec::new();
    for threads in ["1", "8"] {
        let output = scratch.hug_with_env(&args, &[("RAYON_NUM_THREADS", threads)]);
        assert_eq!(output.status.code(), Some(1));
        // The messages name native paths; compare them `/`-separated.
        let stderr = stderr_of(&output).replace('\\', "/");
        let stderr_order: Vec<&str> = stderr
            .lines()
            .filter_map(|line| expected.iter().copied().find(|name| line.contains(&format!("{name}`"))))
            .collect::<Vec<_>>();
        assert_eq!(stderr_order, expected, "{threads} threads: stderr order");
        let json = json_of(&String::from_utf8(output.stdout.clone()).expect("utf-8"));
        let errors: Vec<&str> = json["errors"]
            .as_array()
            .expect("errors")
            .iter()
            .map(|error| error["file"].as_str().expect("file"))
            .collect();
        assert_eq!(errors, expected, "{threads} threads: errors order");
        reports.push(output.stdout);
    }
    assert!(reports[0] == reports[1], "one and eight workers print the same report");
}

/// `--json-full` carries every file's complete index (diagnostics included,
/// declaration text kept) and accepts file selectors.
#[test]
fn json_full_reports_complete_indexes() {
    let scratch = Scratch::new(MULTI_FIXTURES);
    let json = json_of(&scratch.stdout(&["--json-full", "functions", "rust.rs", "nested/go.go"]));
    assert!(json.get("files").is_none(), "--json-full has `indexes`, not `files`");
    assert_eq!(file_names(&json, "indexes"), ["nested/go.go", "rust.rs"]);
    for index in json["indexes"].as_array().expect("indexes") {
        assert_eq!(
            index["completed_passes"],
            serde_json::json!(["Parse", "Bind", "Semantic", "Docs", "Diagnostics"]),
            "{}",
            index["file"]
        );
        let kinds: Vec<&str> = index["symbols"]
            .as_array()
            .expect("symbols")
            .iter()
            .map(|record| record["kind"].as_str().expect("kind"))
            .collect();
        assert!(kinds.iter().any(|kind| !FUNCTION_KINDS.contains(kind)), "a complete index holds every kind");
        assert!(
            index["symbols"]
                .as_array()
                .expect("symbols")
                .iter()
                .all(|record| record["source"]["declaration_text"].is_string()),
            "declaration_text kept"
        );
    }
}

/// `--json-full` is never filtered: symbol selectors and commands without
/// declaration indexes are argument errors.
#[test]
fn json_full_rejects_symbol_selectors_and_other_commands() {
    let scratch = Scratch::new(&[("lib.rs", "pub struct Widget;\nimpl Widget { pub fn grow(&self) {} }\n")]);
    let rejected: [&[&str]; 10] = [
        &["--json-full", "functions", "grow"],
        &["--json-full", "symbols", "--exported"],
        &["--json-full", "symbols", "--prelude"],
        &["--json-full", "types", "--exclude-symbols", "Widget"],
        &["--json-full", "classes", "--name", "Widget"],
        &["--json-full", "classes", "--static-only"],
        &["--json-full", "classes", "--instance-only"],
        &["--json-full", "classes", "Widget!"],
        &["--json-full", "lint"],
        &["--json-full", "imports"],
    ];
    for args in rejected {
        let output = scratch.hug(args);
        assert_eq!(output.status.code(), Some(2), "hug {args:?} is an argument error");
        assert!(stderr_of(&output).contains("--json-full"), "hug {args:?}: {}", stderr_of(&output));
        assert!(output.stdout.is_empty(), "hug {args:?} printed a report");
    }
    let json = json_of(&scratch.stdout(&["--json-full", "classes", "lib.rs", "--sort", "name"]));
    assert_eq!(file_names(&json, "indexes"), ["lib.rs"], "file selectors and --sort are allowed");
}

/// A reader that closes the pipe early ends `hug` quietly and successfully,
/// in JSON and terminal output alike.
#[test]
fn a_closed_stdout_ends_the_run_quietly() {
    let scratch = Scratch::new(MULTI_FIXTURES);
    for args in [&["--json-full", "symbols", "--no-cache"][..], &["--plain", "symbols", "--no-cache"]] {
        let mut child = scratch
            .process(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn hug");
        let mut first = [0u8; 16];
        child
            .stdout
            .take()
            .expect("stdout")
            .read_exact(&mut first)
            .expect("read the first bytes");
        // Dropping the pipe's read end closes it while `hug` is writing.
        let output = child.wait_with_output().expect("wait for hug");
        let stderr = stderr_of(&output);
        assert!(!stderr.contains("panicked"), "hug {args:?} panicked: {stderr}");
        assert_eq!(output.status.code(), Some(0), "hug {args:?}: {stderr}");
    }
}
