//! The declaration commands' JSON report: the envelope and its records, the
//! command kind sets, `--json-full`, failed files, files outside the scan
//! root, and output failures.

use std::io::Read;
use std::path::PathBuf;
use std::process::{Output, Stdio};

use serde_json::Value;
use tree_hugger::{SymbolRecord, file_identity, symbol_id_from_stable_key};

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
        stdout.starts_with(r#"{"report_schema_version":1,"schema_version":{"major":5,"minor":2},"root_dir":"#),
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
        assert_eq!(
            file["completed_capabilities"],
            serde_json::json!(["declarations", "flags"]),
            "{name}: `--json` always computes the flags"
        );
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
            for flag in ["is_test", "is_deprecated"] {
                assert!(record["visibility"][flag].is_boolean(), "{name}: {flag}");
            }
            assert!(record.get("metrics").is_none(), "{name}: metrics were not requested");
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

/// `functions` and `types` select exactly their kind sets in every scope;
/// `symbols --all-scopes` selects every kind and lists each record of both.
#[test]
fn functions_and_types_select_exactly_their_kind_sets() {
    let scratch = Scratch::new(MULTI_FIXTURES);
    let all = json_of(&scratch.stdout(&["--json", "symbols", "--all-scopes"]));
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

const GENERIC_BAG_RS: &str = "pub struct Bag<T: Clone = String> {\n    value: T,\n}\n\
impl<T: Clone> Bag<T> {\n    pub fn get<U>(&self, value: U) -> U where U: Copy, T: Default { value }\n}\n";

/// The `[name, kind, constraints, default, position]` of each generic
/// parameter of `record`.
fn generic_params(record: &Value) -> Value {
    Value::from(
        record["type_info"]["generic_params"]
            .as_array()
            .map(|params| {
                params
                    .iter()
                    .map(|param| {
                        serde_json::json!([param["name"], param["kind"], param["constraints"], param["default"], param["position"]])
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default(),
    )
}

/// Written generic parameters and declaration-level requirements reach
/// `--json`, `--jsonl` and `--json-full` alike; a method lists only its own
/// parameters, never its `impl` block's.
#[test]
fn generic_parameters_reach_every_json_format() {
    let scratch = Scratch::new(&[("bag.rs", GENERIC_BAG_RS)]);
    let bag = serde_json::json!([["T", "Type", ["Clone"], "String", 0]]);
    let get = serde_json::json!([["U", "Type", ["Copy"], null, 0]]);

    let types = json_of(&scratch.stdout(&["--json", "types", "bag.rs"]));
    let bag_record = records(&types).into_iter().find(|record| record["identity"]["name"] == "Bag").expect("Bag");
    assert_eq!(generic_params(bag_record), bag, "--json types");
    assert!(bag_record["type_info"].get("constraints").is_none(), "Bag has no declaration-level requirement");

    let functions = json_of(&scratch.stdout(&["--json", "functions", "bag.rs"]));
    let get_record = records(&functions).into_iter().find(|record| record["identity"]["name"] == "get").expect("get");
    assert_eq!(generic_params(get_record), get, "--json functions");
    // `T` is the impl's parameter, so its requirement is the method's own.
    assert_eq!(get_record["type_info"]["constraints"], serde_json::json!(["T: Default"]));

    let lines = scratch.stdout(&["functions", "--jsonl", "bag.rs"]);
    let line = lines
        .lines()
        .map(json_of)
        .find(|line| line["record"]["identity"]["name"] == "get")
        .expect("a `get` line");
    assert_eq!(generic_params(&line["record"]), get, "--jsonl functions");

    let full = json_of(&scratch.stdout(&["--json-full", "symbols", "bag.rs"]));
    let indexed: Vec<&Value> = full["indexes"]
        .as_array()
        .expect("indexes")
        .iter()
        .flat_map(|index| index["symbols"].as_array().expect("symbols"))
        .collect();
    for (name, expected) in [("Bag", &bag), ("get", &get)] {
        let record = indexed.iter().find(|record| record["identity"]["name"] == name).expect("record");
        assert_eq!(&generic_params(record), expected, "--json-full `{name}`");
    }
}

const WRITTEN_FACTS_RS: &str = "#[inline]\n#[must_use = \"keep  it\"]\npub const unsafe fn both() -> u8 {\n    0\n}\n\npub struct Cell;\n\nimpl Cell {\n    pub fn set(&mut self) {}\n}\n";
const WRITTEN_FACTS_TS: &str = "@sealed\nexport abstract class Shape {\n  @log() override describe(): string { return \"abstract\"; }\n  readonly sides: number = 0;\n}\n";

/// A record's `[modifiers, [[attribute name, value], …]]`.
fn written_facts(record: &Value) -> Value {
    let attributes: Vec<Value> = record["attributes"]
        .as_array()
        .map(|attributes| {
            attributes
                .iter()
                .map(|attribute| serde_json::json!([attribute["name"], attribute["value"]]))
                .collect()
        })
        .unwrap_or_default();
    serde_json::json!([record["modifiers"].as_array().cloned().unwrap_or_default(), attributes])
}

/// Written modifiers and attributes reach `--json`, `--jsonl` and
/// `--json-full` alike, and a warm run serves the same facts from the
/// persistent cache without parsing.
#[test]
fn written_modifiers_and_attributes_reach_every_json_format() {
    let scratch = Scratch::new(&[("facts.rs", WRITTEN_FACTS_RS), ("shape.ts", WRITTEN_FACTS_TS)]);
    let custom = |keyword: &str| serde_json::json!({ "type": "Custom", "value": keyword });
    let expected = [
        (
            "both",
            serde_json::json!([[{ "type": "Const" }, { "type": "Unsafe" }], [["inline", null], ["must_use", "\"keep  it\""]]]),
        ),
        ("set", serde_json::json!([[{ "type": "Mut" }], []])),
        ("Shape", serde_json::json!([[custom("abstract")], [["sealed", null]]])),
        ("describe", serde_json::json!([[custom("override")], [["log", ""]]])),
        ("sides", serde_json::json!([[{ "type": "Readonly" }], []])),
    ];
    let find = |records: &[&Value], name: &str| -> Value {
        let matches: Vec<&&Value> = records.iter().filter(|record| record["identity"]["name"] == name).collect();
        assert_eq!(matches.len(), 1, "one `{name}` record");
        written_facts(matches[0])
    };

    let json = json_of(&scratch.stdout(&["--json", "symbols", "facts.rs", "shape.ts"]));
    let listed = records(&json);
    for (name, facts) in &expected {
        assert_eq!(&find(&listed, name), facts, "--json `{name}`");
    }

    let lines = scratch.stdout(&["symbols", "--jsonl", "facts.rs", "shape.ts"]);
    let lines: Vec<Value> = lines.lines().map(json_of).collect();
    let line_records: Vec<&Value> = lines.iter().map(|line| &line["record"]).collect();
    for (name, facts) in &expected {
        assert_eq!(&find(&line_records, name), facts, "--jsonl `{name}`");
    }

    let full_args = ["--verbose", "--json-full", "symbols", "facts.rs", "shape.ts"];
    for run in ["cold", "warm"] {
        let output = scratch.hug(&full_args);
        assert!(output.status.success(), "{run}: {}", stderr_of(&output));
        if run == "warm" {
            let stderr = stderr_of(&output);
            assert!(stderr.starts_with("cache: 2 hits, 0 partial hits, 0 misses; parsed 0 files"), "{stderr}");
        }
        let full = json_of(&String::from_utf8(output.stdout).expect("utf-8"));
        let indexed: Vec<&Value> = full["indexes"]
            .as_array()
            .expect("indexes")
            .iter()
            .flat_map(|index| index["symbols"].as_array().expect("symbols"))
            .collect();
        for (name, facts) in &expected {
            assert_eq!(&find(&indexed, name), facts, "--json-full {run} `{name}`");
        }
    }
}

const BODY_LINES_C: &str = "int add(int a, int b)\n{\n  return a + b;\n}\nint proto(int a);\nstruct Pair { int a; int b; };\n";
const BODY_LINES_LUA: &str = "local function empty() end\nlocal function two()\n  -- note\n  return 2\nend\n";

/// `metrics.lines` reaches `--json-full` through the normal invocation:
/// counted (C definition, Lua comment and body), `0` (Lua empty body),
/// `null` (prototype, struct). An entry written by `--json functions`
/// without metrics is widened (reparsed once), then a warm run is a hit
/// with the same values; `--json` itself carries no metrics facet.
#[test]
fn body_lines_reach_the_full_index_across_cache_widening() {
    let scratch = Scratch::new(&[("body.c", BODY_LINES_C), ("body.lua", BODY_LINES_LUA)]);
    let expected = [
        ("add", Value::from(3)),
        ("proto", Value::Null),
        ("Pair", Value::Null),
        ("empty", Value::from(0)),
        ("two", Value::from(2)),
    ];

    let json = json_of(&scratch.stdout(&["--json", "functions", "body.c", "body.lua"]));
    let listed = records(&json);
    assert!(!listed.is_empty(), "functions are listed");
    for record in &listed {
        assert!(record.get("metrics").is_none(), "--json carries no metrics: {record}");
    }

    let full_args = ["--verbose", "--json-full", "symbols", "body.c", "body.lua"];
    for run in ["widened", "warm"] {
        let output = scratch.hug(&full_args);
        assert!(output.status.success(), "{run}: {}", stderr_of(&output));
        let stderr = stderr_of(&output);
        let cache_line = match run {
            "widened" => "cache: 0 hits, 2 partial hits, 0 misses; parsed 2 files",
            _ => "cache: 2 hits, 0 partial hits, 0 misses; parsed 0 files",
        };
        assert!(stderr.starts_with(cache_line), "{run}: {stderr}");
        let full = json_of(&String::from_utf8(output.stdout).expect("utf-8"));
        let indexed: Vec<&Value> = full["indexes"]
            .as_array()
            .expect("indexes")
            .iter()
            .flat_map(|index| index["symbols"].as_array().expect("symbols"))
            .collect();
        for (name, lines) in &expected {
            let matching: Vec<&&Value> = indexed.iter().filter(|record| record["identity"]["name"] == *name).collect();
            assert_eq!(matching.len(), 1, "{run}: one `{name}` record");
            let metrics = matching[0]["metrics"].as_object().expect("metrics facet");
            assert_eq!(metrics.get("lines"), Some(lines), "{run}: `{name}` lines");
        }
    }
}

/// A declaration exceeding the serializable generic-position range, beside
/// a type that commands can select without reading that declaration.
fn oversized_generic_fixture() -> Scratch {
    let params = vec!["T"; usize::from(u16::MAX) + 2];
    let wide = format!("fn wide<{}>() {{}}\npub struct Narrow;\n", params.join(", "));
    Scratch::new(&[("wide.rs", wide.as_str())])
}

mod an_unrepresentable_generic_list_fails_only_a_command_that_selects_it {
    use super::*;
    #[test]
    fn selected_functions_fail() {
        let scratch = oversized_generic_fixture();
        let output = scratch.hug(&["--json", "functions", "wide.rs"]);
        assert_eq!(output.status.code(), Some(1), "functions exits 1");
        assert!(stderr_of(&output).contains("wide"), "stderr names the declaration");
        let json = json_of(&String::from_utf8(output.stdout).expect("utf-8"));
        let errors = json["errors"].as_array().expect("errors");
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert_eq!((&errors[0]["file"], &errors[0]["kind"]), (&Value::from("wide.rs"), &Value::from("too_many_generic_parameters")));

    }
    #[test]
    fn unselected_types_succeed() {
        let scratch = oversized_generic_fixture();
        let output = scratch.hug(&["--json", "types", "wide.rs"]);
        assert_eq!(output.status.code(), Some(0), "types does not read `wide`: {}", stderr_of(&output));
        let json = json_of(&String::from_utf8(output.stdout).expect("utf-8"));
        let names: Vec<&Value> = records(&json).into_iter().map(|record| &record["identity"]["name"]).collect();
        assert_eq!(names, [&Value::from("Narrow")]);
    }
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

/// The multi/ Rust fixture, for writing outside the scan root.
fn multi_rust() -> &'static str {
    MULTI_FIXTURES
        .iter()
        .find(|(name, _)| *name == "rust.rs")
        .map(|(_, contents)| *contents)
        .expect("multi/ has rust.rs")
}

/// `external.rs` in a test-owned directory beside the scratch repository,
/// returned with the path a report must give it: `../{dir}/external.rs`.
fn external_file(scratch: &Scratch, contents: &[u8]) -> (PathBuf, String) {
    let repo = biscuit_file::canonicalize_simplified(scratch.repo.path()).expect("canonical repo");
    let beside = biscuit_file::canonicalize_simplified(scratch.home.path()).expect("canonical home");
    assert_eq!(repo.parent(), beside.parent(), "both scratch directories share the temp root");
    let dir_name = beside.file_name().expect("dir name").to_string_lossy().into_owned();
    // Written through the uncanonicalized temp path (`/var/...` on macOS),
    // so the run sees another spelling of the root's parent.
    let file = scratch.home.path().join("external.rs");
    std::fs::write(&file, contents).expect("write external file");
    (file, format!("../{dir_name}/external.rs"))
}

/// Every record of a report: `files[].symbols`, `files[].members` and
/// `indexes[].symbols`.
fn report_records(json: &Value) -> Vec<&Value> {
    let mut found = Vec::new();
    for key in ["files", "indexes"] {
        for entry in json[key].as_array().into_iter().flatten() {
            found.extend(entry["symbols"].as_array().into_iter().flatten());
            for members in entry["members"].as_object().into_iter().flat_map(|map| map.values()) {
                found.extend(members.as_array().expect("members"));
            }
        }
    }
    found
}

/// The file segment of a stable key (`{language}::{file}::{kind}::...`).
fn key_file(record: &Value) -> String {
    let key = record["identity"]["stable_key"].as_str().expect("stable_key");
    let kind = record["kind"].as_str().expect("kind");
    let rest = key.strip_prefix("rust::").unwrap_or_else(|| panic!("a Rust key: {key}"));
    let end = rest.find(&format!("::{kind}::")).unwrap_or_else(|| panic!("{key} names its kind"));
    rest[..end].to_string()
}

/// A file outside the scan root is reported relative to `root_dir` for all
/// four commands in both JSON modes: its entry, each record's file and its
/// stable key's file. IDs stay those of the key as analyzed.
#[test]
fn a_file_outside_the_root_is_reported_relative_to_it() {
    let scratch = Scratch::new(&[("rust.rs", multi_rust())]);
    let (external, expected) = external_file(&scratch, multi_rust().as_bytes());
    let external_arg = external.to_str().expect("utf-8 temp path");
    // The file as `hug` analyzed it (its identity, with symlinks such as
    // macOS `/var` resolved), which its IDs were derived from.
    let analyzed = file_identity(&external).to_string_lossy().replace('\\', "/");

    for command in ["symbols", "functions", "types", "classes"] {
        for (mode, key) in [("--json", "files"), ("--json-full", "indexes")] {
            let args = [mode, command, "--language", "rust", "--no-cache", "rust.rs", external_arg];
            let json = json_of(&scratch.stdout(&args));
            let mut names = file_names(&json, key);
            names.sort();
            let mut wanted = vec![expected.clone(), "rust.rs".to_string()];
            wanted.sort();
            assert_eq!(names, wanted, "hug {args:?}: entry paths");

            let records = report_records(&json);
            let outside: Vec<&&Value> =
                records.iter().filter(|record| record["source"]["file_path"] == expected.as_str()).collect();
            assert!(!outside.is_empty(), "hug {args:?}: records of the outside file");
            for record in records.iter() {
                let file = record["source"]["file_path"].as_str().expect("file_path");
                assert!(file == expected || file == "rust.rs", "hug {args:?}: record path {file}");
                assert_eq!(key_file(record), file, "hug {args:?}: key path of {record}");
            }
            for record in outside {
                let key = record["identity"]["stable_key"].as_str().expect("stable_key");
                let as_analyzed = key.replacen(&format!("::{expected}::"), &format!("::{analyzed}::"), 1);
                let id = serde_json::to_value(symbol_id_from_stable_key(&as_analyzed)).expect("id");
                assert_eq!(record["id"], id, "hug {args:?}: the ID is unchanged by the projection");
            }
        }
    }
}

/// A file outside the scan root that cannot be read is listed in `errors`
/// relative to `root_dir`, and the run exits 1, for all four commands in both
/// JSON modes.
#[test]
fn a_failed_file_outside_the_root_is_reported_relative_to_it() {
    let scratch = Scratch::new(&[("good.rs", "pub fn listed() {}\n")]);
    let mut contents = multi_rust().as_bytes().to_vec();
    contents.push(0xff);
    let (external, expected) = external_file(&scratch, &contents);
    let external_arg = external.to_str().expect("utf-8 temp path");

    for command in ["symbols", "functions", "types", "classes"] {
        for mode in ["--json", "--json-full"] {
            let args = [mode, command, "--language", "rust", "--no-cache", external_arg];
            let output = scratch.hug(&args);
            assert_eq!(output.status.code(), Some(1), "hug {args:?} exits 1");
            let json = json_of(&String::from_utf8(output.stdout).expect("utf-8"));
            let errors = json["errors"].as_array().expect("errors");
            assert_eq!(errors.len(), 1, "hug {args:?}: {errors:?}");
            assert_eq!(errors[0]["file"], expected.as_str(), "hug {args:?}");
            assert_eq!(errors[0]["kind"], "io");
        }
    }
}

/// A reader that closes the pipe early ends `hug` quietly and successfully,
/// in JSON and terminal output alike.
#[test]
fn a_closed_stdout_ends_the_run_quietly() {
    let scratch = Scratch::new(MULTI_FIXTURES);
    for args in [
        &["--json-full", "symbols", "--no-cache"][..],
        &["--plain", "symbols", "--no-cache"],
        &["symbols", "--jsonl", "--no-cache"],
    ] {
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

/// `imports` and `lint` end quietly and successfully when stdout is a pipe
/// whose reader has gone (`hug imports | head -1`), in text and JSON alike;
/// they used to panic on the first write.
fn check_closed_imports_or_lint_pipe(args: &[&str]) {
    let scratch = Scratch::new(MULTI_FIXTURES);

    let output = hug_into_closed_pipe(&scratch, args);
    let stderr = stderr_of(&output);
    assert!(!stderr.contains("panicked"), "hug {args:?} panicked: {stderr}");
    assert!(!stderr.contains("cannot write output"), "hug {args:?}: {stderr}");
    assert_eq!(output.status.code(), Some(0), "hug {args:?}: {stderr}");
}

/// The module copied to build trees that span many chunks.
const PYTHON_MODULE: &str = include_str!("../../../lib/tests/fixtures/symbols/multi/nested/python.py");

/// Copies of [`PYTHON_MODULE`] in the scale trees: twelve chunks.
const COPIES: usize = 96;

/// Files per analysis chunk with the harness's two rayon workers.
const CHUNK: usize = 8;

/// A tree of [`COPIES`] copies of [`PYTHON_MODULE`], spread evenly over
/// `directories` directories (`1` puts them all at the root), plus `extra`.
fn python_copies(directories: usize, extra: &[(&str, &str)]) -> Scratch {
    let names: Vec<String> = (0..COPIES)
        .map(|index| match directories {
            1 => format!("m{index:03}.py"),
            _ => format!("d{:02}/m{index:03}.py", index / (COPIES / directories)),
        })
        .collect();
    let mut files: Vec<(&str, &str)> = names.iter().map(|name| (name.as_str(), PYTHON_MODULE)).collect();
    files.extend_from_slice(extra);
    Scratch::new(&files)
}

/// The file count of the `--verbose` work-counter line.
fn parsed_files(stderr: &str) -> usize {
    let line = stderr
        .lines()
        .rfind(|line| line.starts_with("cache: "))
        .unwrap_or_else(|| panic!("no work-counter line: {stderr}"));
    line.split("parsed ")
        .nth(1)
        .and_then(|rest| rest.split(' ').next())
        .and_then(|count| count.parse().ok())
        .unwrap_or_else(|| panic!("no parsed count: {line}"))
}

/// Runs `hug` with stdout a pipe whose reader closed before it started, so
/// its first write fails whatever the timing.
fn hug_into_closed_pipe(scratch: &Scratch, args: &[&str]) -> Output {
    let (reader, writer) = std::io::pipe().expect("create a pipe");
    drop(reader);
    scratch
        .process(args)
        .stdin(Stdio::null())
        .stdout(writer)
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn hug")
        .wait_with_output()
        .expect("wait for hug")
}

/// Asserts a quiet, successful exit that parsed at most `bound` files.
fn assert_stopped_early(args: &[&str], output: &Output, bound: usize) {
    let stderr = stderr_of(output);
    assert!(!stderr.contains("panicked"), "hug {args:?} panicked: {stderr}");
    assert_eq!(output.status.code(), Some(0), "hug {args:?}: {stderr}");
    let parsed = parsed_files(&stderr);
    assert!(parsed <= bound, "hug {args:?} parsed {parsed} files (at most {bound} expected)");
}

/// The terminal listing streams: a reader that closes after the first bytes
/// stops the scan long before its end, for every declaration command. The
/// writer can run ahead of the reader's close by a few chunks, so the bound
/// is half the tree rather than an exact count.
#[test]
fn a_reader_closing_early_stops_the_terminal_scan() {
    let scratch = python_copies(1, &[]);
    for command in ["functions", "types", "symbols", "classes"] {
        let args = ["--plain", command, "--no-cache", "--verbose"];
        let mut child = scratch
            .process(&args)
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
        // Dropping the pipe's read end closes it while `hug` is still scanning.
        let output = child.wait_with_output().expect("wait for hug");
        assert_stopped_early(&args, &output, COPIES / 2);
    }
}

/// A failing stdout stops scheduling at the first chunk whose output is
/// flushed: the first chunk for a per-file listing, the second when the
/// first chunk is one unit still held (one directory group).
#[test]
fn a_failing_stdout_stops_every_streamed_layout_within_two_chunks() {
    let flat = python_copies(1, &[]);
    for command in ["functions", "types", "symbols", "classes"] {
        let args = ["--plain", command, "--no-cache", "--verbose"];
        assert_stopped_early(&args, &hug_into_closed_pipe(&flat, &args), CHUNK);
    }
    // JSON Lines, tables and outlines are written per file as well.
    for layout in [
        &["symbols", "--jsonl"][..],
        &["classes", "--jsonl"],
        &["--plain", "symbols", "--format", "table"],
        &["--plain", "symbols", "--tree"],
    ] {
        let mut args = layout.to_vec();
        args.extend(["--no-cache", "--verbose"]);
        assert_stopped_early(&args, &hug_into_closed_pipe(&flat, &args), CHUNK);
    }

    // Twelve directories of one chunk each, one package.
    let grouped = python_copies(12, &[("Cargo.toml", "[package]\nname = \"copies\"\nversion = \"0.0.0\"\n")]);
    for grouping in [&["--group-by-file"][..], &["--group-by-module"], &["--group-by-module", "--group-by-file"]] {
        let mut args = vec!["--plain", "symbols", "--no-cache", "--verbose"];
        args.extend_from_slice(grouping);
        assert_stopped_early(&args, &hug_into_closed_pipe(&grouped, &args), 2 * CHUNK);
    }

    // Without a manifest every directory is its own package: sections.
    let sectioned = python_copies(12, &[]);
    let args = ["--plain", "symbols", "--no-cache", "--verbose"];
    assert_stopped_early(&args, &hug_into_closed_pipe(&sectioned, &args), 2 * CHUNK);
}

/// Runs `hug` with stdout a pipe whose reader takes the first 16 bytes and
/// then closes it while `hug` is still scanning.
fn hug_until_reader_closes(scratch: &Scratch, args: &[&str]) -> Output {
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
    child.wait_with_output().expect("wait for hug")
}

/// Package `a` holds all [`COPIES`] copies, eight per directory, so it spans
/// every chunk; later packages follow it in listing order: `b` declares,
/// `c` has only a comment. With `declaring: false`, `b` is left out.
fn uneven_packages(declaring: bool) -> Scratch {
    let manifest = |name: &str| format!("[package]\nname = \"{name}\"\nversion = \"0.0.0\"\n");
    let manifests = [manifest("a"), manifest("b"), manifest("c")];
    let names: Vec<String> = (0..COPIES)
        .map(|index| format!("a/d{:02}/m{index:03}.py", index / CHUNK))
        .collect();
    let mut files: Vec<(&str, &str)> = names.iter().map(|name| (name.as_str(), PYTHON_MODULE)).collect();
    files.extend([
        ("a/Cargo.toml", manifests[0].as_str()),
        ("c/Cargo.toml", manifests[2].as_str()),
        ("c/empty.py", "# no declarations\n"),
    ]);
    if declaring {
        files.extend([("b/Cargo.toml", manifests[1].as_str()), ("b/last.py", PYTHON_MODULE)]);
    }
    Scratch::new(&files)
}

const GROUPINGS: [&[&str]; 4] = [&[], &["--group-by-file"], &["--group-by-module"], &["--group-by-module", "--group-by-file"]];

/// The files a grouping holds before its first unit is written: one chunk,
/// or two when a unit is one directory (here exactly one chunk) that is
/// written when the next directory's first file arrives.
fn chunk_bound(grouping: &[&str]) -> usize {
    if grouping.contains(&"--group-by-module") { 2 * CHUNK } else { CHUNK }
}

/// A large first package is written as it is analyzed, in every listing
/// layout: the package heading never waits for a later package, so a failing
/// stdout stops the scan within the grouping's chunk bound, whether the
/// later packages declare something or not. `classes` and both JSON
/// reports over the same tree are controls.
fn check_uneven_package_stdout(command: &str, declaring: bool) {
    let scratch = uneven_packages(declaring);
    if command == "classes" {
        for format in ["--plain", "--json", "--json-full"] {
            let args = [format, command, "--no-cache", "--verbose"];
            assert_stopped_early(&args, &hug_into_closed_pipe(&scratch, &args), CHUNK);
        }
        return;
    }
    for grouping in GROUPINGS {
        let mut args = vec!["--plain", command, "--no-cache", "--verbose"];
        args.extend_from_slice(grouping);
        assert_stopped_early(&args, &hug_into_closed_pipe(&scratch, &args), chunk_bound(grouping));
    }
    for format in ["--json", "--json-full"] {
        let args = [format, command, "--no-cache", "--verbose"];
        assert_stopped_early(&args, &hug_into_closed_pipe(&scratch, &args), CHUNK);
    }
}

/// The same uneven tree with a reader that closes after the first bytes.
/// The writer can run ahead of the reader's close by a few chunks, so the
/// bound is half the tree rather than an exact count; holding package `a`
/// until `b` is seen parses all of it.
#[test]
fn an_uneven_first_package_streams_to_a_reader_closing_early() {
    let scratch = uneven_packages(true);
    for command in ["functions", "types", "symbols"] {
        for grouping in GROUPINGS {
            let mut args = vec!["--plain", command, "--no-cache", "--verbose"];
            args.extend_from_slice(grouping);
            assert_stopped_early(&args, &hug_until_reader_closes(&scratch, &args), COPIES / 2);
        }
    }
    for format in ["--plain", "--json", "--json-full"] {
        let args = [format, "classes", "--no-cache", "--verbose"];
        assert_stopped_early(&args, &hug_until_reader_closes(&scratch, &args), COPIES / 2);
    }
    for format in ["--json", "--json-full"] {
        for command in ["functions", "types", "symbols"] {
            let args = [format, command, "--no-cache", "--verbose"];
            assert_stopped_early(&args, &hug_until_reader_closes(&scratch, &args), COPIES / 2);
        }
    }
}

/// JSON output is flushed at every chunk end, so a failing stdout is
/// noticed within one chunk of the envelope's first bytes even when the
/// files contribute no entries to fill the output buffer.
#[test]
fn a_json_report_without_entries_stops_at_a_failing_stdout() {
    let names: Vec<String> = (0..COPIES).map(|index| format!("m{index:03}.py")).collect();
    let files: Vec<(&str, &str)> = names.iter().map(|name| (name.as_str(), "# no declarations\n")).collect();
    let scratch = Scratch::new(&files);
    for format in ["--json", "--json-full"] {
        for command in ["functions", "types", "symbols", "classes"] {
            let args = [format, command, "--no-cache", "--verbose"];
            assert_stopped_early(&args, &hug_into_closed_pipe(&scratch, &args), CHUNK);
        }
    }
}

/// `--json` always carries computed test and deprecation flags, without any
/// filter asking for them: a file-name rule, a test attribute, `cfg(test)`
/// and a deprecation marker set them, and look-alikes (`test_helper`,
/// `latest.rs`, a deprecation word in an ordinary comment) do not.
#[test]
fn json_records_carry_computed_test_and_deprecation_flags() {
    let scratch = Scratch::new(&[
        ("pkg/Cargo.toml", "[package]\nname = \"pkg\"\n"),
        (
            "pkg/src/lib.rs",
            "// deprecated: use new_api\npub fn test_helper() {}\n#[deprecated]\npub fn old_api() {}\n#[test]\nfn unit() {}\n#[cfg(test)]\nmod tests {\n    fn inner() {}\n}\n",
        ),
        ("pkg/src/latest.rs", "pub fn latest() {}\n"),
        ("pkg/tests/it.rs", "fn integration_helper() {}\n"),
    ]);
    let json = json_of(&scratch.stdout(&["--json", "functions"]));
    for file in json["files"].as_array().expect("files") {
        assert_eq!(file["completed_capabilities"], serde_json::json!(["declarations", "flags"]));
    }
    let flags: Vec<(String, bool, bool)> = records(&json)
        .into_iter()
        .map(|record| {
            (
                record["identity"]["name"].as_str().expect("name").to_string(),
                record["visibility"]["is_test"].as_bool().expect("is_test"),
                record["visibility"]["is_deprecated"].as_bool().expect("is_deprecated"),
            )
        })
        .collect();
    let expected: Vec<(String, bool, bool)> = [
        ("latest", false, false),
        ("test_helper", false, false),
        ("old_api", false, true),
        ("unit", true, false),
        ("inner", true, false),
        ("integration_helper", true, false),
    ]
    .into_iter()
    .map(|(name, is_test, is_deprecated)| (name.to_string(), is_test, is_deprecated))
    .collect();
    assert_eq!(flags, expected);
}

/// A Rust file with one `unwrap-call` diagnostic under `--warn unwrap-call`.
const UNWRAPPING_RS: &str = "fn f() { let x: Option<u8> = None; x.unwrap(); }\n";

/// Text `imports` and `lint` are written file by file, so a failing stdout
/// stops the scan within one window, as the declaration listings do.
#[test]
fn imports_and_lint_text_stop_at_a_failing_stdout() {
    let modules = python_copies(1, &[]);
    let args = ["--plain", "imports", "--no-cache", "--verbose"];
    assert_stopped_early(&args, &hug_into_closed_pipe(&modules, &args), CHUNK);

    let names: Vec<String> = (0..COPIES).map(|index| format!("u{index:03}.rs")).collect();
    let files: Vec<(&str, &str)> = names.iter().map(|name| (name.as_str(), UNWRAPPING_RS)).collect();
    let unwrapping = Scratch::new(&files);
    let args = ["--plain", "lint", "--no-cache", "--verbose", "--warn", "unwrap-call"];
    assert_stopped_early(&args, &hug_into_closed_pipe(&unwrapping, &args), CHUNK);
}

/// Failed files beside a slow, successful first file: the head's report
/// sits buffered while every later file fails. Each failed delivery must
/// still flush, or the scan runs past a closed stdout to the last file.
const FAILED_SIBLINGS: usize = 120;

/// `a.rs`, slow to analyze and with imports and an `unwrap-call` to report,
/// followed by `extra`.
fn slow_head_then(extra: &[(&str, &str)]) -> Scratch {
    let mut head = format!("use std::fmt;\nuse std::io;\n{UNWRAPPING_RS}{}\nfn slow() {{\n", crate::RUST_FIXTURE);
    head.push_str(&"    let x = 1;\n".repeat(500));
    head.push_str("}\n");
    let mut files = vec![("Cargo.toml", "[package]\nname = \"mixed\"\nversion = \"0.0.0\"\n"), ("a.rs", head.as_str())];
    files.extend_from_slice(extra);
    Scratch::new(&files)
}

/// The `error:` lines on stderr: one per delivered failed file.
fn error_lines(output: &Output) -> usize {
    stderr_of(output).lines().filter(|line| line.starts_with("error:")).count()
}

/// Asserts a quiet, successful exit that delivered fewer than one window of
/// failed files: at most one window is started before the first flush that
/// must fail, and the head is one of them.
fn assert_failures_stopped_early(args: &[&str], output: &Output) {
    let stderr = stderr_of(output);
    assert!(!stderr.contains("panicked"), "hug {args:?} panicked: {stderr}");
    assert_eq!(output.status.code(), Some(0), "hug {args:?}: {stderr}");
    let errors = error_lines(output);
    assert!(errors < CHUNK, "hug {args:?} delivered {errors} failed files (fewer than {CHUNK} expected): {stderr}");
}

/// Text `imports` and `lint`, and every streamed declaration report, flush
/// after a failed file as after a good one, so a closed stdout stops the
/// scan within one window of failures.
fn check_failed_files_stop_at_stdout(args: &[&str]) {
    let scratch = slow_head_then(&[]);
    for index in 0..FAILED_SIBLINGS {
        let mut bytes = crate::RUST_FIXTURE.as_bytes().to_vec();
        bytes.push(0xff);
        std::fs::write(scratch.repo.path().join(format!("b{index:03}.rs")), bytes).expect("write a non-UTF-8 file");
    }
    // Control: with a reader, every failure is reported.
    let open = scratch.hug(&["--plain", "imports", "--no-cache"]);
    assert_eq!(error_lines(&open), FAILED_SIBLINGS, "{}", stderr_of(&open));
    assert_failures_stopped_early(args, &hug_into_closed_pipe(&scratch, args));
}

/// An external adapter that fails on every later file does not keep being
/// run after stdout has closed: the adapter error path flushes too.
/// Unix only: the stub is a shell script found as
/// `node_modules/.bin/oxlint`, which Windows would not execute.
#[cfg(unix)]
fn check_adapter_failures_stop_at_stdout(plain: bool) {
    use std::os::unix::fs::PermissionsExt;

    const TYPESCRIPT: &str = "export function h(): number { return 1; }\n";
    let names: Vec<String> = (0..FAILED_SIBLINGS).map(|index| format!("t{index:03}.ts")).collect();
    let typescript: Vec<(&str, &str)> = names.iter().map(|name| (name.as_str(), TYPESCRIPT)).collect();
    let scratch = slow_head_then(&typescript);
    let log = scratch.repo.path().join("oxlint-runs.log");
    let stub = scratch.repo.path().join("node_modules/.bin/oxlint");
    std::fs::create_dir_all(stub.parent().expect("bin directory")).expect("create node_modules/.bin");
    std::fs::write(
        &stub,
        format!(
            "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then echo \"Version: 1.0.0\"; exit 0; fi\necho run >> '{}'\necho invalid-json\n",
            log.display()
        ),
    )
    .expect("write the oxlint stub");
    std::fs::set_permissions(&stub, std::fs::Permissions::from_mode(0o755)).expect("make the stub executable");
    let runs = || std::fs::read_to_string(&log).map_or(0, |text| text.lines().count());

    // Control: with a reader, the stub runs and fails for every TS file.
    let open = scratch.hug(&["--plain", "lint", "--no-cache"]);
    assert_eq!(error_lines(&open), FAILED_SIBLINGS, "{}", stderr_of(&open));
    assert_eq!(runs(), FAILED_SIBLINGS);
    std::fs::remove_file(&log).expect("reset the run log");

    let args: &[&str] = if plain {
        &["--plain", "lint", "--no-cache", "--verbose", "--warn", "unwrap-call"]
    } else {
        &["lint", "--no-cache", "--verbose", "--warn", "unwrap-call"]
    };
    {

        assert_failures_stopped_early(args, &hug_into_closed_pipe(&scratch, args));
        let started = runs();
        assert!(started < CHUNK, "hug {args:?} ran the adapter {started} times");
        let _ = std::fs::remove_file(&log);
    }
}

/// Text lint hides clean files whenever more than one file is selected,
/// decided before any is analyzed: a failed file still counts as selected,
/// and the closing line counts the files that were linted.
#[test]
fn lint_text_hides_clean_files_by_the_selected_file_count() {
    let scratch = Scratch::new(&[("clean.rs", "fn g() {}\n"), ("other.rs", "fn h() {}\n"), ("unwrap.rs", UNWRAPPING_RS)]);
    std::fs::write(scratch.repo.path().join("bad.rs"), NOT_UTF8).expect("write bad file");
    let lint = |files: &[&str]| {
        let mut args = vec!["--plain", "lint", "--no-cache", "--warn", "unwrap-call"];
        args.extend_from_slice(files);
        let output = scratch.hug(&args);
        let stderr = stderr_of(&output);
        (output.status.code(), String::from_utf8(output.stdout).expect("utf-8"), stderr)
    };

    let (status, stdout, _) = lint(&["clean.rs"]);
    assert_eq!((status, stdout.as_str()), (Some(0), "clean.rs (Rust)\n  (no diagnostics)\n\n"));

    let (status, stdout, _) = lint(&["clean.rs", "other.rs"]);
    assert_eq!((status, stdout.as_str()), (Some(0), "No lint diagnostics in 2 files.\n"));

    let (status, stdout, stderr) = lint(&["bad.rs", "clean.rs"]);
    assert_eq!(status, Some(1), "{stderr}");
    assert_eq!(stdout, "No lint diagnostics in 1 files.\n", "two files were selected");
    assert!(stderr.contains("bad.rs"), "{stderr}");

    let (status, stdout, _) = lint(&["clean.rs", "unwrap.rs"]);
    assert_eq!(status, Some(0));
    assert!(stdout.starts_with("unwrap.rs (Rust)\n"), "{stdout}");
    assert!(stdout.contains("[unwrap-call]"), "{stdout}");
    assert!(!stdout.contains("clean.rs") && !stdout.contains("No lint diagnostics"), "{stdout}");
}

/// An unavailable external adapter is warned about once, on stderr, right
/// before the report of the first file it was asked to lint: earlier files'
/// output is already written.
#[test]
fn an_unavailable_adapter_is_warned_once_before_its_first_file() {
    let scratch = Scratch::new(&[
        ("a.rs", UNWRAPPING_RS),
        ("b.ts", "export function h() { debugger; }\n"),
        ("c.ts", "export function k() { debugger; }\n"),
    ]);
    // No `oxlint` on an empty `PATH` and no `node_modules` in the scratch.
    let empty = tempfile::tempdir().expect("empty PATH directory");
    let (mut reader, writer) = std::io::pipe().expect("create a pipe");
    let mut child = {
        let mut command = scratch.process(&["--plain", "lint", "--no-cache", "--warn", "unwrap-call"]);
        command
            .env("PATH", empty.path())
            .stdin(Stdio::null())
            .stderr(writer.try_clone().expect("clone the pipe"))
            .stdout(writer);
        command.spawn().expect("spawn hug")
    };
    let mut combined = String::new();
    reader.read_to_string(&mut combined).expect("read the combined output");
    assert_eq!(child.wait().expect("wait for hug").code(), Some(0), "{combined}");

    let warning = "warning: external lint adapter 'oxlint' is unavailable; native diagnostics were used";
    assert_eq!(combined.matches(warning).count(), 1, "{combined}");
    let at = |text: &str| combined.find(text).unwrap_or_else(|| panic!("{text}: {combined}"));
    assert!(at("a.rs (Rust)") < at(warning), "{combined}");
    assert!(at(warning) < at("b.ts (TypeScript)"), "{combined}");
    assert!(at("b.ts (TypeScript)") < at("c.ts (TypeScript)"), "{combined}");
}

mod failed_files_still_stop_imports_and_lint_at_a_failing_stdout {
    use super::*;
    macro_rules! failed_stdout_tests {
        ($($test:ident => $args:expr),* $(,)?) => {
            $(#[test] fn $test() { check_failed_files_stop_at_stdout($args); })*
        };
    }
    failed_stdout_tests! {
        imports => &["imports", "--no-cache", "--verbose"],
        plain_imports => &["--plain", "imports", "--no-cache", "--verbose"],
        lint => &["lint", "--no-cache", "--verbose", "--warn", "unwrap-call"],
        plain_lint => &["--plain", "lint", "--no-cache", "--verbose", "--warn", "unwrap-call"],
        functions => &["--plain", "functions", "--no-cache", "--verbose"],
        classes => &["--plain", "classes", "--no-cache", "--verbose"],
        json => &["--json", "functions", "--no-cache", "--verbose"],
        json_full => &["--json-full", "functions", "--no-cache", "--verbose"],
        jsonl => &["functions", "--jsonl", "--no-cache", "--verbose"],
        table => &["--plain", "functions", "--format", "table", "--no-cache", "--verbose"],
        tree => &["--plain", "functions", "--tree", "--no-cache", "--verbose"],
        tsv => &["functions", "--format", "tsv", "--no-cache", "--verbose"],
        ctags => &["functions", "--format", "ctags", "--no-cache", "--verbose"],
        markdown => &["functions", "--format", "markdown", "--no-cache", "--verbose"],
        prose => &["functions", "--format", "prose", "--no-cache", "--verbose"],
    }
}

mod imports_and_lint_end_quietly_on_a_closed_pipe {
    use super::*;
    #[test]
    fn imports() { check_closed_imports_or_lint_pipe(&["imports", "--no-cache"]); }
    #[test]
    fn plain_imports() { check_closed_imports_or_lint_pipe(&["--plain", "imports", "--no-cache"]); }
    #[test]
    fn json_imports() { check_closed_imports_or_lint_pipe(&["--json", "imports", "--no-cache"]); }
    #[test]
    fn lint() { check_closed_imports_or_lint_pipe(&["lint", "--no-cache"]); }
    #[test]
    fn plain_lint() { check_closed_imports_or_lint_pipe(&["--plain", "lint", "--no-cache", "--deny", "all"]); }
    #[test]
    fn json_lint() { check_closed_imports_or_lint_pipe(&["lint", "--json", "--no-cache"]); }
}

mod an_uneven_first_package_stops_at_a_failing_stdout {
    use super::*;
    #[test]
    fn functions_declaring() { check_uneven_package_stdout("functions", true); }
    #[test]
    fn functions_empty() { check_uneven_package_stdout("functions", false); }
    #[test]
    fn types_declaring() { check_uneven_package_stdout("types", true); }
    #[test]
    fn types_empty() { check_uneven_package_stdout("types", false); }
    #[test]
    fn symbols_declaring() { check_uneven_package_stdout("symbols", true); }
    #[test]
    fn symbols_empty() { check_uneven_package_stdout("symbols", false); }
    #[test]
    fn classes_declaring() { check_uneven_package_stdout("classes", true); }
    #[test]
    fn classes_empty() { check_uneven_package_stdout("classes", false); }
}

#[cfg(unix)]
mod adapter_failures_still_stop_lint_at_a_failing_stdout {
    use super::*;
    #[test]
    fn plain() { check_adapter_failures_stop_at_stdout(true); }
    #[test]
    fn automatic() { check_adapter_failures_stop_at_stdout(false); }
}
