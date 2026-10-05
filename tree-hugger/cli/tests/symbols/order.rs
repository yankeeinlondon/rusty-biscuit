//! `--sort` and grouping: each order against an oracle built here from the
//! JSON records and the documented keys (name / kind, then portable path,
//! start byte, end byte, kind and ID), JSON's per-file ordering, and the
//! argument errors of the removed and misplaced flags.

use std::cmp::Ordering;

use serde_json::Value;

use crate::golden::MULTI_FIXTURES;
use crate::{Scratch, json_of};

/// The documented `--sort kind` order.
const KIND_ORDER: [&str; 19] = [
    "Module", "Namespace", "Type", "Class", "Interface", "Trait", "Enum", "TypeAlias", "Function",
    "Method", "Constructor", "Macro", "Constant", "Field", "Property", "EnumVariant", "Variable",
    "Parameter", "Unknown",
];

/// `multi/` as one package (a root manifest), so the listing is one list.
fn scratch() -> Scratch {
    let mut files = MULTI_FIXTURES.to_vec();
    files.push(("Cargo.toml", "[package]\nname = \"multi\"\nversion = \"0.0.0\"\n"));
    Scratch::new(&files)
}

fn kind_rank(record: &Value) -> usize {
    let kind = record["kind"].as_str().expect("kind");
    KIND_ORDER.iter().position(|candidate| *candidate == kind).expect("known kind")
}

fn path_of(record: &Value) -> &str {
    record["source"]["file_path"].as_str().expect("file_path")
}

/// The oracle: `order`'s key, then the shared tie-breakers.
fn compare(order: &str, left: &Value, right: &Value) -> Ordering {
    let primary = match order {
        "file" => Ordering::Equal,
        "name" => left["identity"]["name"].as_str().cmp(&right["identity"]["name"].as_str()),
        "kind" => kind_rank(left).cmp(&kind_rank(right)),
        other => panic!("unknown order {other}"),
    };
    let span = |record: &Value, key: &str| record["source"]["declaration_span"][key].as_u64();
    primary
        .then_with(|| path_of(left).split('/').cmp(path_of(right).split('/')))
        .then_with(|| span(left, "start_byte").cmp(&span(right, "start_byte")))
        .then_with(|| span(left, "end_byte").cmp(&span(right, "end_byte")))
        .then_with(|| kind_rank(left).cmp(&kind_rank(right)))
        .then_with(|| left["id"].as_str().cmp(&right["id"].as_str()))
}

fn all_records(json: &Value) -> Vec<Value> {
    json["files"]
        .as_array()
        .expect("files")
        .iter()
        .flat_map(|file| file["symbols"].as_array().expect("symbols").iter().cloned())
        .collect()
}

/// `path:line:col` as the flat listing prints it.
fn location(record: &Value) -> String {
    let start = &record["source"]["declaration_span"]["start"];
    format!("{}:{}:{}", path_of(record), start["line"], start["column"])
}

/// The last token of each bullet line: its location.
fn listed_locations(stdout: &str) -> Vec<String> {
    stdout
        .lines()
        .filter(|line| line.trim_start().starts_with("- "))
        .map(|line| line.rsplit(' ').next().expect("location").to_string())
        .collect()
}

#[test]
fn each_sort_order_lists_declarations_by_its_keys_and_tie_breakers() {
    let scratch = scratch();
    let records = all_records(&json_of(&scratch.stdout(&["--json", "symbols"])));
    assert!(records.len() > 100, "multi/ has many records ({})", records.len());

    for order in ["file", "name", "kind"] {
        let mut expected = records.clone();
        expected.sort_by(|left, right| compare(order, left, right));
        let expected: Vec<String> = expected.iter().map(location).collect();

        let explicit = scratch.stdout(&["--plain", "symbols", "--sort", order]);
        assert_eq!(listed_locations(&explicit), expected, "--sort {order}");
        if order == "file" {
            let default = scratch.stdout(&["--plain", "symbols"]);
            assert_eq!(default, explicit, "`--sort file` is the default");
        }
    }
}

/// `--sort name` puts same-named declarations of different files together,
/// the first file's first (`nested/go.go` sorts before `rust.rs`).
#[test]
fn sort_name_keeps_same_named_types_together_in_path_order() {
    let scratch = scratch();
    let stdout = scratch.stdout(&["--plain", "types", "--sort", "name"]);
    let widgets: Vec<(usize, &str)> = stdout
        .lines()
        .enumerate()
        .filter(|(_, line)| line.contains(" Widget "))
        .collect();
    assert_eq!(widgets.len(), 2, "{stdout}");
    assert_eq!(widgets[1].0, widgets[0].0 + 1, "the two `Widget` types are adjacent");
    assert!(widgets[0].1.ends_with("nested/go.go:5:1"), "{}", widgets[0].1);
    assert!(widgets[1].1.ends_with("rust.rs:4:1"), "{}", widgets[1].1);
}

/// With `--json` (and `--json-full`), file entries stay in portable path
/// order whatever `--sort` says; `--sort` orders each file's records.
#[test]
fn json_keeps_file_order_and_sorts_within_each_file() {
    let scratch = scratch();
    let files_of = |json: &Value, key: &str| -> Vec<Value> {
        json[key].as_array().expect(key).iter().map(|file| file["file"].clone()).collect()
    };
    let default = json_of(&scratch.stdout(&["--json", "symbols"]));
    let default_files = files_of(&default, "files");
    let mut path_order = default_files.clone();
    path_order.sort_by(|left, right| {
        let (left, right) = (left.as_str().expect("file"), right.as_str().expect("file"));
        left.split('/').cmp(right.split('/'))
    });
    assert_eq!(default_files, path_order, "file entries are in portable path order");
    assert_eq!(default_files.last(), Some(&Value::from("zsh.zsh")));
    let nested = default_files.iter().position(|file| file == "nested/go.go").expect("nested/go.go");
    assert!(default_files[nested - 1] == "lua.lua", "nested/ sorts as a directory, after lua.lua");

    for order in ["file", "name", "kind"] {
        let sorted = json_of(&scratch.stdout(&["--json", "symbols", "--sort", order]));
        assert_eq!(files_of(&sorted, "files"), default_files, "--sort {order}: file order");
        for (file, default_file) in sorted["files"].as_array().expect("files").iter().zip(default["files"].as_array().expect("files")) {
            let listed = file["symbols"].as_array().expect("symbols").clone();
            let mut expected = default_file["symbols"].as_array().expect("symbols").clone();
            expected.sort_by(|left, right| compare(order, left, right));
            assert_eq!(listed, expected, "--sort {order}: {} records", file["file"]);
        }
        let full = json_of(&scratch.stdout(&["--json-full", "functions", "--sort", order]));
        assert_eq!(files_of(&full, "indexes"), default_files, "--sort {order}: --json-full file order");
    }
}

/// Grouping is independent of `--sort`: groups in path order, `--sort`
/// orders the declarations inside each group.
#[test]
fn sort_orders_declarations_within_file_and_directory_groups() {
    let scratch = scratch();
    let records = all_records(&json_of(&scratch.stdout(&["--json", "symbols"])));

    // `--group-by-file`: one `path` heading per file, items `[line:col]`.
    let stdout = scratch.stdout(&["--plain", "symbols", "--group-by-file", "--sort", "name"]);
    let mut headings = Vec::new();
    let mut current = String::new();
    let mut listed: Vec<(String, String)> = Vec::new();
    for line in stdout.lines() {
        if line.is_empty() {
            continue;
        }
        if !line.starts_with(' ') {
            current = line.to_string();
            headings.push(current.clone());
        } else if line.trim_start().starts_with("- ") {
            let position = line.rsplit(' ').next().expect("position").trim_matches(['[', ']']);
            listed.push((current.clone(), format!("{current}:{position}")));
        }
    }
    let mut by_path = headings.clone();
    by_path.sort_by(|left, right| left.split('/').cmp(right.split('/')));
    assert_eq!(headings, by_path, "file groups in path order");
    for heading in &headings {
        let mut expected: Vec<&Value> = records.iter().filter(|record| path_of(record) == heading).collect();
        expected.sort_by(|left, right| compare("name", left, right));
        let expected: Vec<String> = expected.iter().map(|record| location(record)).collect();
        let actual: Vec<String> = listed
            .iter()
            .filter(|(group, _)| group == heading)
            .map(|(_, location)| location.clone())
            .collect();
        assert_eq!(actual, expected, "{heading}: `--sort name` within the file group");
    }

    // `--group-by-module`: one directory heading, items with full locations.
    let stdout = scratch.stdout(&["--plain", "symbols", "--group-by-module", "--sort", "kind"]);
    let module_headings: Vec<&str> = stdout.lines().filter(|line| !line.is_empty() && !line.starts_with(' ')).collect();
    assert_eq!(module_headings, ["(root)", "nested"], "directory groups in path order");
    let in_nested = |record: &&Value| path_of(record).starts_with("nested/");
    let mut expected: Vec<&Value> = records.iter().filter(|record| !in_nested(record)).collect();
    expected.sort_by(|left, right| compare("kind", left, right));
    let mut nested: Vec<&Value> = records.iter().filter(in_nested).collect();
    nested.sort_by(|left, right| compare("kind", left, right));
    expected.extend(nested);
    let expected: Vec<String> = expected.iter().map(|record| location(record)).collect();
    assert_eq!(listed_locations(&stdout), expected, "`--sort kind` within directory groups");

    // Both flags: directory, then file headings.
    let stdout = scratch.stdout(&["--plain", "symbols", "--group-by-module", "--group-by-file"]);
    assert!(stdout.starts_with("(root)\n  bash.sh\n"), "{}", &stdout[..80.min(stdout.len())]);
    assert!(stdout.contains("\nnested\n  nested/go.go\n"), "{stdout}");
}

/// `--sort` takes one of `file`, `name`, `kind`, once, on the declaration
/// commands only; the old sort flags are gone.
#[test]
fn removed_and_misplaced_sort_flags_are_argument_errors() {
    let scratch = Scratch::new(&[("lib.rs", "pub fn one() {}\n")]);
    let rejected: [&[&str]; 8] = [
        &["symbols", "--sort", "container"],
        &["symbols", "--sort-by-kind"],
        &["symbols", "--sort-by-module"],
        &["functions", "--sort", "name", "--sort", "kind"],
        &["classes", "--sort", "name", "--sort", "name"],
        &["lint", "--sort", "file"],
        &["imports", "--sort", "file"],
        &["types", "--sort"],
    ];
    for args in rejected {
        let output = scratch.hug(args);
        assert_eq!(output.status.code(), Some(2), "hug {args:?} must be an argument error");
        assert!(output.stdout.is_empty(), "hug {args:?} printed output");
    }
    for command in ["functions", "types", "symbols", "classes"] {
        for order in ["file", "name", "kind"] {
            let output = scratch.hug(&[command, "--sort", order]);
            assert!(output.status.success(), "hug {command} --sort {order}");
        }
    }
}
