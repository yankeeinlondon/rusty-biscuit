//! `--sort` and grouping: each order against an oracle built here from the
//! JSON records and the documented keys (name / kind, then portable path,
//! start byte, end byte, kind and ID), JSON's per-file ordering, and the
//! argument errors of the removed and misplaced flags.

use std::cmp::Ordering;

use serde_json::Value;

use crate::golden::MULTI_FIXTURES;
use crate::{JAVA_FIXTURE, Scratch, json_of};

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
    // Every scope: locals (parameters) carry the tie-breaker cases.
    let records = all_records(&json_of(&scratch.stdout(&["--json", "symbols", "--all-scopes"])));
    assert!(records.len() > 100, "multi/ has many records ({})", records.len());

    for order in ["file", "name", "kind"] {
        let mut expected = records.clone();
        expected.sort_by(|left, right| compare(order, left, right));
        let expected: Vec<String> = expected.iter().map(location).collect();

        let explicit = scratch.stdout(&["--plain", "symbols", "--all-scopes", "--sort", order]);
        assert_eq!(listed_locations(&explicit), expected, "--sort {order}");
        if order == "file" {
            let default = scratch.stdout(&["--plain", "symbols", "--all-scopes"]);
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

/// With `--json`, file entries stay in portable path order whatever
/// `--sort` says; `--sort` orders each file's records.
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
    }

}

/// Every scanned file has an index. Each command keeps those files in
/// portable path order while sorting the records within each file.
mod json_full_keeps_file_order_and_sorts_within_each_file {
    use super::*;

    fn check(command: &str) {
        let scratch = scratch();
        let files_of = |json: &Value| -> Vec<Value> {
            json["indexes"].as_array().expect("indexes").iter().map(|file| file["file"].clone()).collect()
        };
        let full_files = files_of(&json_of(&scratch.stdout(&["--json-full", "symbols"])));
        let mut path_order = full_files.clone();
        path_order.sort_by(|left, right| {
            let (left, right) = (left.as_str().expect("file"), right.as_str().expect("file"));
            left.split('/').cmp(right.split('/'))
        });
        assert_eq!(full_files, path_order, "--json-full file entries are in portable path order");
        for order in ["file", "name", "kind"] {
            let full = json_of(&scratch.stdout(&["--json-full", command, "--sort", order]));
            assert_eq!(files_of(&full), full_files, "{command} --sort {order}: --json-full file order");
            assert_records_sorted(&full, "indexes", order, &format!("{command} --json-full"));
        }
    }

    #[test]
    fn functions() {
        check("functions");
    }
    #[test]
    fn types() {
        check("types");
    }
    #[test]
    fn symbols() {
        check("symbols");
    }
    #[test]
    fn classes() {
        check("classes");
    }
}

/// Asserts each entry's `symbols` array is in `order` with every tie-breaker.
fn assert_records_sorted(json: &Value, key: &str, order: &str, context: &str) {
    for entry in json[key].as_array().expect(key) {
        let listed = entry["symbols"].as_array().expect("symbols");
        let mut expected = listed.clone();
        expected.sort_by(|left, right| compare(order, left, right));
        let ids = |records: &[Value]| -> Vec<Value> { records.iter().map(|record| record["id"].clone()).collect() };
        assert_eq!(ids(listed), ids(&expected), "{context} --sort {order}: {} records", entry["file"]);
    }
}

/// Extraction order is not source order: C emits the `struct point` type
/// specifier inside a `typedef` before the alias that encloses it, and Go
/// emits a method's receiver parameter before the method. `--json-full`
/// with the default order still lists both files by position.
#[test]
fn json_full_default_order_lists_nested_declarations_by_position() {
    let scratch = scratch();
    let full = json_of(&scratch.stdout(&["--json-full", "symbols"]));
    for file in ["c.c", "nested/go.go"] {
        let index = full["indexes"]
            .as_array()
            .expect("indexes")
            .iter()
            .find(|index| index["file"] == file)
            .unwrap_or_else(|| panic!("an index for {file}"));
        let starts: Vec<u64> = index["symbols"]
            .as_array()
            .expect("symbols")
            .iter()
            .map(|record| record["source"]["declaration_span"]["start_byte"].as_u64().expect("start_byte"))
            .collect();
        assert!(starts.is_sorted(), "{file}: records by start byte ({starts:?})");
    }
}

/// A C `typedef int A, B, …;` gives every alias the whole typedef as its
/// declaration span, so the aliases tie on path, start, end and kind and
/// only the record ID orders them. Every output branch must reach it.
///
/// IDs hash the scratch directory's absolute path, so the expected order is
/// computed from the output's own IDs. Eight types make it near certain
/// (all but 1 in 8!) that the ID order differs from source order, so a
/// branch that skipped the ID tie-breaker would fail; the assertions
/// themselves never depend on which order the IDs fall in.
#[test]
fn equal_declaration_spans_fall_through_to_the_id_tie_breaker() {
    const NAMES: [&str; 8] = ["Alpha", "Beta", "Gamma", "Delta", "Epsilon", "Zeta", "Eta", "Theta"];
    let source = format!("typedef int {};\n", NAMES.join(", "));
    let scratch = Scratch::new(&[("group.c", source.as_str())]);
    let names = |records: &[&Value]| -> Vec<String> {
        records.iter().map(|record| record["identity"]["name"].as_str().expect("name").to_string()).collect()
    };
    let grouped = |json: &Value, key: &str| -> Vec<Value> {
        json[key][0]["symbols"]
            .as_array()
            .expect("symbols")
            .iter()
            .filter(|record| NAMES.contains(&record["identity"]["name"].as_str().unwrap_or_default()))
            .cloned()
            .collect()
    };

    let full = json_of(&scratch.stdout(&["--json-full", "types"]));
    let types = grouped(&full, "indexes");
    assert_eq!(types.len(), NAMES.len(), "every alias is a record");
    let tie = |record: &Value| {
        let span = &record["source"]["declaration_span"];
        (
            record["source"]["file_path"].clone(),
            span["start_byte"].clone(),
            span["end_byte"].clone(),
            record["kind"].clone(),
        )
    };
    assert!(
        types.iter().all(|record| tie(record) == tie(&types[0])),
        "the aliases tie on file, start, end and kind: {:?}",
        types.iter().map(tie).collect::<Vec<_>>()
    );
    let mut by_id: Vec<&Value> = types.iter().collect();
    by_id.sort_by(|left, right| left["id"].as_str().cmp(&right["id"].as_str()));
    let by_id = names(&by_id);

    for order in ["file", "kind"] {
        for mode in ["--json", "--json-full"] {
            let json = json_of(&scratch.stdout(&[mode, "types", "--sort", order]));
            let key = if mode == "--json" { "files" } else { "indexes" };
            assert_records_sorted(&json, key, order, mode);
            let listed = grouped(&json, key);
            assert_eq!(names(&listed.iter().collect::<Vec<_>>()), by_id, "{mode} --sort {order}: tied types by ID");
        }
        let plain = scratch.stdout(&["--plain", "types", "--sort", order]);
        let listed: Vec<String> = plain
            .lines()
            .filter_map(|line| {
                NAMES.into_iter().find(|name| line.contains(&format!(" {name} "))).map(str::to_string)
            })
            .collect();
        assert_eq!(listed, by_id, "terminal --sort {order}: tied types by ID\n{plain}");
    }
}

/// `classes` members keep source order whatever `--sort` says, in the
/// terminal sections and in JSON `members`, with the shared tie-breakers.
#[test]
fn class_members_are_in_source_order_under_every_sort() {
    let mut files = MULTI_FIXTURES.to_vec();
    files.push(("Sample.java", JAVA_FIXTURE));
    let scratch = Scratch::new(&files);
    for order in ["file", "name", "kind"] {
        let json = json_of(&scratch.stdout(&["--json", "classes", "--sort", order]));
        let mut sections = 0;
        for file in json["files"].as_array().expect("files") {
            for (container, members) in file["members"].as_object().expect("members") {
                let members = members.as_array().expect("members array");
                let mut expected = members.clone();
                expected.sort_by(|left, right| compare("file", left, right));
                assert_eq!(members, &expected, "--sort {order}: {} members of {container}", file["file"]);
                sections += usize::from(members.len() > 1);
            }
        }
        assert!(sections > 2, "several containers have more than one member ({sections})");

        // Each terminal method section lists `[line:col]` in ascending order.
        let plain = scratch.stdout(&["--plain", "classes", "--sort", order]);
        let mut section: Vec<(u64, u64)> = Vec::new();
        for line in plain.lines().chain(std::iter::once("")) {
            let trimmed = line.trim_start();
            if trimmed.starts_with("- ") && trimmed.ends_with(']') {
                let position = trimmed.rsplit(' ').next().expect("position").trim_matches(['[', ']']);
                if let Some((line, column)) = position.split_once(':') {
                    section.push((line.parse().expect("line"), column.parse().expect("column")));
                }
                continue;
            }
            assert!(section.is_sorted(), "--sort {order}: a method section out of source order\n{plain}");
            section.clear();
        }
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

/// Package sections appear whenever the scanned files span two or more
/// packages, decided before any file is analyzed, and then every package
/// gets one, `(no symbols)` included; a scan of one package is one flat
/// list. Files outside any package are grouped under the root's empty
/// heading. Every sort and grouping follows the same rule.
#[test]
fn packages_are_sectioned_whenever_the_scan_spans_two() {
    let manifest = |name: &str| format!("[package]\nname = \"{name}\"\nversion = \"0.0.0\"\n");
    let (one, two, three, four) = (manifest("p1"), manifest("p2"), manifest("p3"), manifest("p4"));

    // One package, declarations or not, is flat.
    let single = Scratch::new(&[("p2/Cargo.toml", two.as_str()), ("p2/a.rs", ""), ("p2/b.rs", "pub fn two() {}\n")]);
    assert_eq!(
        single.stdout(&["--plain", "functions"]),
        "  - public function two() p2/b.rs:1:1\n"
    );
    let root_only = Scratch::new(&[("a.rs", ""), ("top.rs", "fn root() {}\n")]);
    assert_eq!(
        root_only.stdout(&["--plain", "functions"]),
        "  - private function root() top.rs:1:1\n"
    );
    let nothing = Scratch::new(&[("p1/Cargo.toml", one.as_str()), ("p1/a.rs", "")]);
    assert_eq!(nothing.stdout(&["--plain", "functions"]), "(no symbols)\n");

    // Several packages are sectioned from the start, even when only one
    // declares anything, so no package waits for a later one.
    let mut files = vec![
        ("p1/Cargo.toml", one.as_str()),
        ("p1/a.rs", ""),
        ("p2/Cargo.toml", two.as_str()),
        ("p2/b.rs", "pub fn two() {}\n"),
        ("p3/Cargo.toml", three.as_str()),
        ("p3/c.rs", ""),
    ];
    let one_declares = Scratch::new(&files);
    for layout in [&[][..], &["--sort", "name"], &["--sort", "kind"]] {
        let mut args = vec!["--plain", "functions"];
        args.extend_from_slice(layout);
        assert_eq!(
            one_declares.stdout(&args),
            "\np1\n\n(no symbols)\n\n\
             \np2\n\n  - public function two() p2/b.rs:1:1\n\n\
             \np3\n\n(no symbols)\n\n",
            "hug {args:?}"
        );
    }
    assert_eq!(
        one_declares.stdout(&["--plain", "functions", "--group-by-file"]),
        "\np1\n\n(no symbols)\n\n\
         \np2\n\np2/b.rs\n  - public function two() [1:1]\n\n\n\
         \np3\n\n(no symbols)\n\n"
    );
    let none_declares = Scratch::new(&[
        ("p1/Cargo.toml", one.as_str()),
        ("p1/a.rs", ""),
        ("p3/Cargo.toml", three.as_str()),
        ("p3/c.rs", ""),
    ]);
    assert_eq!(
        none_declares.stdout(&["--plain", "functions"]),
        "\np1\n\n(no symbols)\n\n\np3\n\n(no symbols)\n\n"
    );

    // Root-level files are the scan root's own package, with an empty
    // heading.
    files.extend([
        ("p4/Cargo.toml", four.as_str()),
        ("p4/d.rs", "pub fn four() {}\n"),
        ("top.rs", "fn root() {}\n"),
    ]);
    let sectioned = Scratch::new(&files);
    assert_eq!(
        sectioned.stdout(&["--plain", "functions"]),
        "\n\n\n  - private function root() top.rs:1:1\n\n\
         \np1\n\n(no symbols)\n\n\
         \np2\n\n  - public function two() p2/b.rs:1:1\n\n\
         \np3\n\n(no symbols)\n\n\
         \np4\n\n  - public function four() p4/d.rs:1:1\n\n"
    );
}
