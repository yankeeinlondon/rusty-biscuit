//! The report formats and views of the declaration commands: JSON Lines,
//! `--count`, `--stats` (terminal and JSON), `--format table|plain`,
//! `--tree`, `--signatures`, interactive wrapping, literal source text, and
//! the rule that exactly one output mode is active.

use serde_json::Value;
use tree_hugger::SymbolRecord;

use crate::{Scratch, json_of, stdout_of};

const RUST: &str = r#"/// A widget.
pub struct Widget {
    pub size: u32,
}

impl Widget {
    pub fn new(size: u32) -> Self {
        Self { size }
    }
}

impl Widget {
    pub fn grow(&mut self, by: u32) {
        self.size += by;
    }
}

impl Elsewhere {
    pub fn visit(&self) {}
}

pub struct Wide { pub a: u8, pub b: u8, pub c: u8, pub d: u8, pub e: u8 }

pub struct Four { pub a: u8, pub b: u8, pub c: u8, pub d: u8 }

#[deprecated]
pub fn old() {}

pub fn zeta() {}

pub fn alpha() {}

#[cfg(test)]
mod tests {
    #[test]
    fn checks() {}
}
"#;

const GO: &str = "package shapes\n\ntype Square struct {\n\tSide int\n}\n\nfunc (s Square) Area() int {\n\treturn s.Side * s.Side\n}\n";

const JAVA: &str = "class Printer {\n    void print(int value) {}\n\n    void print(String value) {}\n}\n";

const LUA: &str = "local function helper()\nend\n";

const BASH: &str = "build() {\n  echo hi\n}\n";

/// Rust, Go, Java, Lua and Bash: impl blocks, a type declared elsewhere, a
/// Go receiver, Java overloads, and two languages without doc detection.
fn scratch() -> Scratch {
    Scratch::new(&[
        ("lib.rs", RUST),
        ("shapes.go", GO),
        ("Printer.java", JAVA),
        ("tool.lua", LUA),
        ("run.sh", BASH),
    ])
}

/// A file `hug` cannot read as UTF-8, so its analysis fails.
pub(crate) fn add_bad_file(scratch: &Scratch, name: &str) {
    std::fs::write(scratch.repo.path().join(name), b"fn ok() {}\n\xff\xfe\n").expect("write bad file");
}

/// What a terminal shows of `output`: OSC 8 links reduced to their text and
/// every other escape removed.
fn visible(output: &str) -> String {
    biscuit_terminal::prelude::strip_escape_codes(biscuit_terminal::prelude::strip_osc8_links(output))
}

fn assert_argument_error(scratch: &Scratch, args: &[&str], needle: &str) {
    let output = scratch.hug(args);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(2), "hug {args:?} is an argument error: {stderr}");
    assert!(stderr.contains(needle), "hug {args:?}: `{needle}` in {stderr}");
    assert!(output.stdout.is_empty(), "hug {args:?} prints nothing on stdout");
}

/// Every record of a `--json` report, in report order.
fn json_records(report: &Value) -> Vec<Value> {
    report["files"]
        .as_array()
        .expect("files")
        .iter()
        .flat_map(|file| file["symbols"].as_array().expect("symbols").iter().cloned())
        .collect()
}

fn lines_of(stdout: &str) -> Vec<Value> {
    stdout
        .lines()
        .map(|line| serde_json::from_str(line).unwrap_or_else(|error| panic!("invalid line ({error}): {line}")))
        .collect()
}

fn names(records: &[Value]) -> Vec<String> {
    records
        .iter()
        .map(|record| record["identity"]["name"].as_str().expect("name").to_string())
        .collect()
}

#[test]
fn json_lines_are_self_describing_envelopes_that_round_trip() {
    let scratch = scratch();
    let stdout = scratch.stdout(&["symbols", "--jsonl"]);
    let report = json_of(&scratch.stdout(&["--json", "symbols"]));

    // Each line is the `--json` file entry's context plus one record, in the
    // spec's field order, with no source blobs.
    let mut from_lines = Vec::new();
    for line in stdout.lines() {
        assert!(
            line.starts_with(r#"{"report_schema_version":1,"schema_version":{"major":5,"minor":2},"file":""#),
            "{line}"
        );
        let order = ["\"file\":", "\"language\":", "\"completed_passes\":", "\"completed_capabilities\":", "\"record\":"];
        let positions: Vec<usize> = order.iter().map(|key| line.find(key).expect(key)).collect();
        assert!(positions.windows(2).all(|pair| pair[0] < pair[1]), "field order: {line}");
        assert!(!line.contains("declaration_text") && !line.contains("\"members\""), "{line}");

        let value: Value = serde_json::from_str(line).expect("line");
        let file = report["files"]
            .as_array()
            .expect("files")
            .iter()
            .find(|entry| entry["file"] == value["file"])
            .expect("the line's file is in the --json report");
        for key in ["language", "completed_passes", "completed_capabilities"] {
            assert_eq!(value[key], file[key], "{key} repeats the file's value");
        }

        // read → write → read: the record is a `SymbolRecord` and survives
        // a round trip unchanged.
        let record: SymbolRecord = serde_json::from_value(value["record"].clone()).expect("a SymbolRecord");
        let written = serde_json::to_value(&record).expect("serialize");
        assert_eq!(written, value["record"]);
        let again: SymbolRecord = serde_json::from_value(written.clone()).expect("read again");
        assert_eq!(serde_json::to_value(&again).expect("serialize"), written);
        from_lines.push(value["record"].clone());
    }
    assert_eq!(from_lines, json_records(&report), "the same records in the same order as --json");
}

#[test]
fn json_lines_class_lines_carry_their_members_as_a_list() {
    let scratch = scratch();
    let lines = lines_of(&scratch.stdout(&["classes", "--jsonl"]));
    let report = json_of(&scratch.stdout(&["--json", "classes"]));
    let containers = json_records(&report);
    assert_eq!(lines.len(), containers.len(), "one line per selected container");
    for (line, container) in lines.iter().zip(&containers) {
        assert_eq!(line["record"], *container);
        let id = container["id"].as_str().expect("id");
        let file = report["files"]
            .as_array()
            .expect("files")
            .iter()
            .find(|entry| entry["file"] == line["file"])
            .expect("file entry");
        assert_eq!(line["members"], file["members"][id], "the --json map entry, as a list");
        assert!(line["members"].is_array());
    }
    let widget = lines.iter().find(|line| line["record"]["identity"]["name"] == "Widget").expect("Widget");
    assert_eq!(names(widget["members"].as_array().expect("members")), ["new", "grow"]);
}

#[test]
fn json_lines_keep_files_in_path_order_and_sort_within_each() {
    let scratch = Scratch::new(&[
        ("b/y.rs", "pub fn beta() {}\npub struct Alpha;\npub fn aardvark() {}\n"),
        ("a/x.rs", "pub fn zulu() {}\npub const MID: u8 = 1;\npub fn able() {}\n"),
    ]);
    let file_of = |lines: &[Value]| -> Vec<String> {
        lines.iter().map(|line| line["file"].as_str().expect("file").to_string()).collect()
    };
    let records = |lines: &[Value]| -> Vec<Value> { lines.iter().map(|line| line["record"].clone()).collect() };

    let by_file = lines_of(&scratch.stdout(&["symbols", "--jsonl"]));
    assert_eq!(file_of(&by_file), ["a/x.rs", "a/x.rs", "a/x.rs", "b/y.rs", "b/y.rs", "b/y.rs"]);
    assert_eq!(names(&records(&by_file)), ["zulu", "MID", "able", "beta", "Alpha", "aardvark"]);

    let by_name = lines_of(&scratch.stdout(&["symbols", "--jsonl", "--sort", "name"]));
    assert_eq!(file_of(&by_name), file_of(&by_file), "files stay in path order");
    assert_eq!(names(&records(&by_name)), ["MID", "able", "zulu", "Alpha", "aardvark", "beta"]);

    let by_kind = lines_of(&scratch.stdout(&["symbols", "--jsonl", "--sort", "kind"]));
    assert_eq!(file_of(&by_kind), file_of(&by_file));
    assert_eq!(names(&records(&by_kind)), ["zulu", "able", "MID", "Alpha", "beta", "aardvark"]);

    // The same per-file order as `--json --sort`.
    for order in ["name", "kind"] {
        let json = json_of(&scratch.stdout(&["--json", "symbols", "--sort", order]));
        let lines = lines_of(&scratch.stdout(&["symbols", "--jsonl", "--sort", order]));
        assert_eq!(records(&lines), json_records(&json), "--sort {order}");
    }
}

#[test]
fn json_lines_report_failures_after_partial_output_and_nothing_for_an_empty_selection() {
    let scratch = Scratch::new(&[("a.rs", "pub fn first() {}\n")]);
    add_bad_file(&scratch, "bad.rs");
    let output = scratch.hug(&["symbols", "--jsonl"]);
    assert_eq!(output.status.code(), Some(1), "a failed file fails the run");
    assert!(String::from_utf8_lossy(&output.stderr).contains("bad.rs"));
    let stdout = String::from_utf8(output.stdout).expect("utf-8");
    let lines = lines_of(&stdout);
    assert_eq!(lines.len(), 1, "the good file's line is still written: {stdout}");
    assert_eq!(lines[0]["file"], "a.rs");

    let scratch = Scratch::new(&[("a.rs", "pub fn first() {}\n")]);
    let empty = scratch.stdout(&["symbols", "--jsonl", "--name", "nothing_matches_this"]);
    assert_eq!(empty, "", "an empty selection is zero lines");
}

/// `--json` and `--jsonl` compute the test and deprecation flags with no
/// filter asking for them, so `false` means "not set".
#[test]
fn json_and_jsonl_records_carry_computed_flags_without_a_filter() {
    let scratch = scratch();
    let report = json_of(&scratch.stdout(&["--json", "symbols", "lib.rs"]));
    let lines = lines_of(&scratch.stdout(&["symbols", "--jsonl", "lib.rs"]));
    let file = &report["files"][0];
    assert!(file["completed_capabilities"].as_array().expect("capabilities").contains(&Value::from("flags")));
    for records in [json_records(&report), lines.iter().map(|line| line["record"].clone()).collect()] {
        let flag = |name: &str, flag: &str| -> bool {
            records
                .iter()
                .find(|record| record["identity"]["name"] == name)
                .unwrap_or_else(|| panic!("{name}"))["visibility"][flag]
                .as_bool()
                .expect("a flag")
        };
        assert!(flag("old", "is_deprecated") && !flag("zeta", "is_deprecated"));
        assert!(flag("tests", "is_test") && !flag("alpha", "is_test"));
    }
    for line in &lines {
        assert!(line["completed_capabilities"].as_array().expect("capabilities").contains(&Value::from("flags")));
    }
}

#[test]
fn stats_json_envelope_counts_doc_coverage_and_reports_failures_last() {
    let scratch = scratch();
    let stdout = scratch.stdout(&["--json", "symbols", "--stats"]);
    assert!(stdout.starts_with(r#"{"report_schema_version":1,"root_dir":"#), "{stdout}");
    assert!(stdout.ends_with(",\"errors\":[]}\n"), "errors comes last: {stdout}");
    let stats = json_of(&stdout);
    assert!(stats.get("schema_version").is_none(), "no record schema version");
    let coverage = &stats["stats"]["doc_coverage"];
    // Lua and Bash have no doc-comment detection: one declaration each.
    assert_eq!(coverage["unavailable"], 2, "{coverage}");
    assert_eq!(stats["stats"]["by_language"]["Lua"], 1);
    assert_eq!(stats["stats"]["by_language"]["Bash"], 1);
    let documented = coverage["documented"].as_u64().expect("documented");
    let total = coverage["total"].as_u64().expect("total");
    assert_eq!(documented, 1, "only Widget has a doc comment");
    assert_eq!(coverage["ratio"].as_f64(), Some(documented as f64 / total as f64));
    assert_eq!(stats["stats"]["synthetic_exports"], 0);

    // Zero matches: empty maps and a null ratio.
    let empty = json_of(&scratch.stdout(&["--json", "symbols", "--stats", "--name", "nothing_matches_this"]));
    assert_eq!(empty["stats"]["by_kind"], serde_json::json!({}));
    assert_eq!(empty["stats"]["doc_coverage"]["total"], 0);
    assert!(empty["stats"]["doc_coverage"]["ratio"].is_null());
    // Only Lua: every record's docs are unknown, so the ratio is null too.
    let lua = json_of(&scratch.stdout(&["--json", "symbols", "--stats", "tool.lua"]));
    assert_eq!(lua["stats"]["doc_coverage"]["unavailable"], 1);
    assert!(lua["stats"]["doc_coverage"]["ratio"].is_null());

    add_bad_file(&scratch, "bad.rs");
    let output = scratch.hug(&["--json", "symbols", "--stats"]);
    assert_eq!(output.status.code(), Some(1), "a failed file fails the run");
    assert!(String::from_utf8_lossy(&output.stderr).contains("bad.rs"));
    let stdout = String::from_utf8(output.stdout).expect("utf-8");
    let failed = json_of(&stdout);
    assert_eq!(failed["errors"][0]["file"], "bad.rs");
    assert!(stdout.trim_end().ends_with("}]}"), "errors is the last member: {stdout}");
    assert_eq!(failed["stats"]["by_kind"], stats["stats"]["by_kind"], "the good files are still counted");
}

/// Every filter applies before counting: for each selection, `--count`,
/// the `--stats` totals and the `--json` records agree.
#[test]
fn count_and_stats_apply_every_filter() {
    let scratch = scratch();
    let selections: [&[&str]; 14] = [
        &[],
        &["--kind", "function"],
        &["--scope", "member"],
        &["--all-scopes"],
        &["--visibility", "public"],
        &["--exported"],
        &["--in", "Widget"],
        &["--exclude-tests"],
        &["--tests-only"],
        &["--undocumented"],
        &["--deprecated"],
        &["--min-params", "1"],
        &["--name", "print"],
        &["lib.rs"],
    ];
    for selection in selections {
        let with = |mode: &[&'static str]| -> Vec<&'static str> {
            let mut args = vec!["symbols"];
            args.extend_from_slice(selection);
            args.extend_from_slice(mode);
            args
        };
        let mut json_args = vec!["--json"];
        json_args.extend(with(&[]));
        let records = json_records(&json_of(&scratch.stdout(&json_args)));
        let count = scratch.stdout(&with(&["--count"]));
        assert_eq!(count, format!("{}\n", records.len()), "--count {selection:?}");

        let mut stats_args = vec!["--json"];
        stats_args.extend(with(&["--stats"]));
        let stats = json_of(&scratch.stdout(&stats_args));
        for map in ["by_kind", "by_visibility", "by_language"] {
            let total: u64 = stats["stats"][map].as_object().expect(map).values().map(|n| n.as_u64().expect("count")).sum();
            assert_eq!(total as usize, records.len(), "{map} {selection:?}");
        }
        for record in &records {
            let kind = record["kind"].as_str().expect("kind");
            assert!(stats["stats"]["by_kind"][kind].as_u64().is_some_and(|n| n > 0), "{kind} {selection:?}");
        }
        if selection == ["--undocumented"] {
            assert_eq!(stats["stats"]["doc_coverage"]["documented"], 0);
            assert_eq!(stats["stats"]["doc_coverage"]["unavailable"], 0, "unknown docs never match --undocumented");
        }
    }
}

/// For each deferred kind and the commands that list it, `--count`, the
/// `--stats` totals, the `--json` records and the `--jsonl` envelopes report
/// the same records, and the table and plain views show each of them.
#[test]
fn reports_consume_the_same_deferred_kind_records() {
    let scratch = Scratch::new(crate::filters::DEFERRED_FILES);
    let selections: [(&[&str], &str, usize); 7] = [
        (&["functions", "--kind", "constructor"], "Constructor", 4),
        (&["symbols", "--kind", "constructor"], "Constructor", 4),
        (&["symbols", "--kind", "enum_variant"], "EnumVariant", 7),
        (&["types", "--kind", "type_alias"], "TypeAlias", 3),
        (&["symbols", "--kind", "field"], "Field", 6),
        // Go's package clause is a namespace too.
        (&["symbols", "--kind", "namespace"], "Namespace", 3),
        (&["symbols", "--kind", "namespace", "--scope", "member"], "Namespace", 0),
    ];
    for (selection, kind, expected) in selections {
        let with = |mode: &[&'static str]| -> Vec<&'static str> {
            let mut args = selection.to_vec();
            args.extend_from_slice(mode);
            args
        };
        let mut json_args = vec!["--json"];
        json_args.extend(with(&[]));
        let records = json_records(&json_of(&scratch.stdout(&json_args)));
        assert_eq!(records.len(), expected, "--json {selection:?}");
        assert!(records.iter().all(|record| record["kind"] == kind), "--json {selection:?}");

        assert_eq!(scratch.stdout(&with(&["--count"])), format!("{expected}\n"), "--count {selection:?}");

        let mut stats_args = vec!["--json"];
        stats_args.extend(with(&["--stats"]));
        let stats = json_of(&scratch.stdout(&stats_args));
        let by_kind = if expected == 0 { serde_json::json!({}) } else { serde_json::json!({ kind: expected }) };
        assert_eq!(stats["stats"]["by_kind"], by_kind, "--stats {selection:?}");

        let envelopes = lines_of(&scratch.stdout(&with(&["--jsonl"])));
        let ids = |records: Vec<&Value>| -> Vec<Value> { records.into_iter().map(|record| record["id"].clone()).collect() };
        assert_eq!(
            ids(envelopes.iter().map(|envelope| &envelope["record"]).collect()),
            ids(records.iter().collect()),
            "--jsonl {selection:?}"
        );

        for format in ["table", "plain"] {
            let view = visible(&scratch.stdout(&with(&["--format", format])));
            for name in names(&records) {
                assert!(view.contains(&name), "--format {format} {selection:?}: `{name}` in\n{view}");
            }
        }
    }
}

#[test]
fn classes_count_and_stats_count_containers_only() {
    let scratch = scratch();
    let report = json_of(&scratch.stdout(&["--json", "classes"]));
    let containers = json_records(&report);
    assert_eq!(names(&containers), ["Printer", "Widget", "Wide", "Four", "Square"]);
    assert_eq!(scratch.stdout(&["classes", "--count"]), "5\n");
    let stats = json_of(&scratch.stdout(&["--json", "classes", "--stats"]));
    assert_eq!(stats["stats"]["by_kind"], serde_json::json!({"Class": 1, "Type": 4}), "no member rows");
    assert_eq!(scratch.stdout(&["classes", "--count", "--name", "nothing_matches_this"]), "0\n");
}

#[test]
fn count_is_one_integer_line_even_when_a_file_fails() {
    let scratch = scratch();
    assert_eq!(scratch.stdout(&["functions", "--count", "--name", "nothing_matches_this"]), "0\n");
    let count = scratch.stdout(&["functions", "--count"]);
    add_bad_file(&scratch, "bad.rs");
    let output = scratch.hug(&["functions", "--count"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("bad.rs"));
    assert_eq!(String::from_utf8(output.stdout).expect("utf-8"), count, "the good files are counted");
}

#[test]
fn stats_terminal_tables_compose_with_formats() {
    let scratch = scratch();
    let default = scratch.stdout(&["symbols", "--stats"]);
    for heading in ["Kind", "Visibility", "Language"] {
        assert!(default.lines().any(|line| line.contains(&format!("│ {heading} "))), "{heading} table: {default}");
    }
    assert!(default.contains("Doc coverage: 1 of"), "{default}");
    assert!(default.contains("; 2 unavailable"), "{default}");
    assert_eq!(scratch.stdout(&["symbols", "--stats", "--format", "table"]), default, "redundant but accepted");

    // Forced color: styled tables by default, none under `--format plain`.
    let forced = [("CLICOLOR_FORCE", "1"), ("COLUMNS", "200")];
    let styled = stdout_of(scratch.hug_with_env(&["symbols", "--stats"], &forced), &["symbols", "--stats"]);
    let plain = stdout_of(
        scratch.hug_with_env(&["symbols", "--stats", "--format", "plain"], &forced),
        &["symbols", "--stats", "--format", "plain"],
    );
    assert!(!plain.contains('\u{1b}'), "--format plain has no escapes: {plain:?}");
    assert_eq!(visible(&styled), plain, "the same tables and line");

    // Narrower than every table: each keeps its natural width (the terminal
    // wraps it) rather than being replaced by a "could not be rendered" notice.
    let narrow = [("CLICOLOR_FORCE", "1"), ("COLUMNS", "12")];
    let narrow = stdout_of(scratch.hug_with_env(&["symbols", "--stats"], &narrow), &["symbols", "--stats"]);
    assert_eq!(visible(&narrow), plain, "natural-width tables at COLUMNS=12");
}

#[test]
fn exactly_one_output_mode_with_the_stats_exception() {
    let scratch = scratch();
    for (args, needle) in [
        (&["--json", "symbols", "--jsonl"][..], "different output modes"),
        (&["--json", "symbols", "--count"], "different output modes"),
        (&["--json-full", "symbols", "--json"], "different output modes"),
        (&["symbols", "--jsonl", "--count"], "different output modes"),
        (&["symbols", "--count", "--format", "plain"], "different output modes"),
        (&["--json", "symbols", "--format", "table"], "different output modes"),
        (&["symbols", "--stats", "--jsonl"], "`--stats` cannot be combined with `--jsonl`"),
        (&["symbols", "--stats", "--count"], "`--stats` cannot be combined with `--count`"),
        (&["--json-full", "symbols", "--stats"], "`--stats` cannot be combined with `--json-full`"),
        (&["--json", "symbols", "--stats", "--format", "plain"], "different output modes"),
        (&["symbols", "--stats", "--sort", "name"], "`--sort`"),
        (&["symbols", "--stats", "--sort", "file"], "`--sort`"),
        (&["symbols", "--stats", "--signatures", "full"], "`--signatures`"),
        (&["symbols", "--stats", "--tree"], "`--tree`"),
        (&["classes", "--static-only", "--instance-only"], "cannot be used with"),
    ] {
        assert_argument_error(&scratch, args, needle);
    }
    // `imports` aggregates nothing.
    assert_argument_error(&scratch, &["imports", "--stats"], "--stats");

    for args in [
        &["--json", "symbols", "--stats"][..],
        &["symbols", "--stats", "--format", "plain"],
        &["symbols", "--stats", "--format", "table"],
        &["--plain", "symbols", "--stats"],
        &["--plain", "symbols", "--jsonl"],
        &["symbols", "--format", "table", "--signatures", "none"],
    ] {
        assert_eq!(scratch.hug(args).status.code(), Some(0), "hug {args:?} is accepted");
    }
}

#[test]
fn tree_nests_members_under_their_container() {
    let scratch = scratch();
    let tree = scratch.stdout(&["--plain", "symbols", "--tree", "lib.rs", "shapes.go", "Printer.java"]);
    let expected_rows = [
        "Printer.java",
        "  package class Printer [1:1]",
        "    package method print(value: int) -> void [2:5]",
        "    package method print(value: String) -> void [4:5]",
        "lib.rs",
        "  public struct Widget { size: u32 } [2:1]",
        "    public field size [3:9]",
        // Two `impl` blocks, grouped under the type by container ID.
        "    public method new(size: u32) -> Self [7:5]",
        "    public method grow(self, by: u32) [13:5]",
        // Declared in another file: a marked heading, not a record.
        "  [in Elsewhere]",
        "    public method visit(self) [19:5]",
        "shapes.go",
        "  public type Square { Side int } [3:1]",
        // A Go receiver method joins its type.
        "    public method Area() int [7:1]",
    ];
    let mut from = 0;
    let rows: Vec<&str> = tree.lines().collect();
    for expected in expected_rows {
        let offset = rows[from..]
            .iter()
            .position(|row| *row == expected)
            .unwrap_or_else(|| panic!("expected {expected:?} after row {from}:\n{tree}"));
        from += offset + 1;
    }
    // The flat listing keeps the container suffix the outline replaces.
    let flat = scratch.stdout(&["--plain", "symbols", "lib.rs"]);
    assert!(flat.contains("method new(size: u32) -> Self [in type Widget]"), "{flat}");
    assert!(!tree.contains("[in type Widget]"), "{tree}");

    // `functions` selects no type: each method sits under a contextual
    // heading, which is never a record.
    let functions = scratch.stdout(&["--plain", "functions", "--tree", "lib.rs", "shapes.go"]);
    assert!(functions.lines().any(|row| row == "  [in type Widget]"), "{functions}");
    assert!(functions.lines().any(|row| row == "  [in type Square]"), "{functions}");
    assert_eq!(functions.matches("[in type Widget]").count(), 1, "one heading for both impl blocks");
    let count = scratch.stdout(&["functions", "--count", "lib.rs", "shapes.go"]);
    let listed = functions.lines().filter(|row| row.contains("method ") || row.contains("function ")).count();
    assert_eq!(count, format!("{listed}\n"), "headings are not counted");

    let empty = scratch.stdout(&["--plain", "symbols", "--tree", "--name", "nothing_matches_this"]);
    assert_eq!(empty, "(no symbols)\n");
}

/// Each `work` sits under its own `Boxed`, not under the first `Boxed`.
#[test]
fn tree_nests_methods_under_their_own_same_named_type() {
    let scratch = Scratch::new(&[("repeat.rs", crate::classes::REPEATED_OWNERS)]);
    let tree = scratch.stdout(&["symbols", "--file", "repeat.rs", "--tree", "--format", "plain", "--no-cache"]);
    let rows: Vec<&str> = tree.trim_end().lines().collect();
    assert_eq!(
        rows,
        [
            "repeat.rs",
            "  private module left [1:1]",
            "    public struct Boxed [1:12]",
            "      public method work(self) [1:43]",
            "  private module right [2:1]",
            "    public struct Boxed [2:13]",
            "      public method work(self) [2:44]",
        ],
        "{tree}"
    );
}

/// A method of `other::W` stays out of the sole local `W`, under a
/// contextual heading that names its qualified owner.
#[test]
fn tree_keeps_an_impl_of_another_files_type_out_of_a_same_named_local_type() {
    let scratch = Scratch::new(&[("main.rs", crate::classes::EXTERNAL_OWNER), ("other.rs", "pub struct W;\n")]);
    let tree = scratch.stdout(&["symbols", "--file", "main.rs", "--tree", "--format", "plain", "--no-cache"]);
    let rows: Vec<&str> = tree.trim_end().lines().collect();
    assert_eq!(
        rows,
        [
            "main.rs",
            "  public struct W [1:1]",
            "  private module other [2:1]",
            "  [in other::W]",
            "    public method probe(self) [3:17]",
        ],
        "{tree}"
    );
}

/// Members of two different external `W`s get one heading each.
#[test]
fn tree_heads_each_external_owner_separately() {
    let scratch = Scratch::new(&[("ext.rs", "impl x::W { fn p(&self) {} }\nimpl y::W { fn q(&self) {} }\n")]);
    let tree = scratch.stdout(&["symbols", "--file", "ext.rs", "--tree", "--format", "plain", "--no-cache"]);
    let rows: Vec<&str> = tree.trim_end().lines().collect();
    assert_eq!(
        rows,
        ["ext.rs", "  [in x::W]", "    private method p(self) [1:13]", "  [in y::W]", "    private method q(self) [2:13]"],
        "{tree}"
    );
}

#[test]
fn tree_combines_with_sort_grouping_and_formats() {
    let scratch = scratch();
    let tree = scratch.stdout(&["--plain", "symbols", "--tree", "lib.rs"]);

    // `--sort name`: containers and, separately, each container's members.
    let sorted = scratch.stdout(&["--plain", "symbols", "--tree", "--sort", "name", "lib.rs"]);
    let top: Vec<&str> = sorted
        .lines()
        .filter(|row| row.starts_with("  ") && !row.starts_with("    "))
        .collect();
    // A contextual heading sorts by its container's name.
    assert_eq!(
        top,
        [
            "  [in Elsewhere]",
            "  public struct Four { a: u8, b: u8, c: u8, d: u8 } [24:1]",
            "  public struct Wide { a: u8, b: u8, c: u8, d: u8, … +1 } [22:1]",
            "  public struct Widget { size: u32 } [2:1]",
            "  public function alpha() [31:1]",
            "  public function old() [27:1]",
            "  private module tests [34:1]",
            "  public function zeta() [29:1]",
        ],
        "{sorted}"
    );
    let widget_members: Vec<&str> = sorted
        .lines()
        .skip_while(|row| !row.contains("struct Widget"))
        .skip(1)
        .take_while(|row| row.starts_with("    "))
        .map(str::trim)
        .collect();
    assert_eq!(
        widget_members,
        ["public method grow(self, by: u32) [13:5]", "public method new(size: u32) -> Self [7:5]", "public field size [3:9]"],
        "{sorted}"
    );

    assert_eq!(scratch.stdout(&["--plain", "symbols", "--tree", "--group-by-file", "lib.rs"]), tree);
    assert_eq!(scratch.stdout(&["symbols", "--tree", "--format", "plain", "lib.rs"]), tree);
    let by_module = scratch.stdout(&["--plain", "symbols", "--tree", "--group-by-module"]);
    assert!(by_module.starts_with("(root)\nPrinter.java\n"), "{by_module}");

    for (args, needle) in [
        (&["symbols", "--tree", "--format", "table"][..], "`--tree` cannot be combined with `--format table`"),
        (&["--json", "symbols", "--tree"], "`--tree`"),
        (&["--json-full", "symbols", "--tree"], "`--tree`"),
        (&["symbols", "--tree", "--jsonl"], "`--tree`"),
        (&["symbols", "--tree", "--count"], "`--tree`"),
        (&["classes", "--tree"], "`classes` already nests members"),
    ] {
        assert_argument_error(&scratch, args, needle);
    }
}

#[test]
fn signature_modes_show_full_short_or_no_signatures() {
    let scratch = scratch();
    let short = scratch.stdout(&["--plain", "types", "lib.rs"]);
    assert!(short.contains("struct Wide { a: u8, b: u8, c: u8, d: u8, … +1 } lib.rs:22:1"), "{short}");
    assert!(short.contains("struct Four { a: u8, b: u8, c: u8, d: u8 } lib.rs:24:1"), "four entries stay whole");
    assert_eq!(scratch.stdout(&["--plain", "types", "lib.rs", "--signatures", "short"]), short, "the default");

    let full = scratch.stdout(&["--plain", "symbols", "lib.rs", "--signatures", "full"]);
    assert!(full.contains("  - pub fn grow(&mut self, by: u32) [in type Widget] lib.rs:13:5"), "{full}");
    assert!(full.contains("  - pub struct Wide lib.rs:22:1"), "{full}");

    let none = scratch.stdout(&["--plain", "symbols", "lib.rs", "--signatures", "none"]);
    assert!(none.contains("  - public method grow [in type Widget] lib.rs:13:5"), "{none}");
    assert!(none.contains("  - public struct Wide lib.rs:22:1"), "{none}");

    let table = scratch.stdout(&["symbols", "lib.rs", "--format", "table", "--signatures", "none"]);
    assert!(table.contains("│ Name ") && !table.contains("Signature"), "no Signature column: {table}");
    let full_table = scratch.stdout(&["symbols", "lib.rs", "--format", "table", "--signatures", "full"]);
    assert!(full_table.contains("│ pub fn grow(&mut self, by: u32) "), "{full_table}");
}

#[test]
fn table_rows_carry_location_kind_visibility_name_and_signature() {
    let scratch = scratch();
    let table = scratch.stdout(&["functions", "--format", "table", "lib.rs", "shapes.go"]);
    let cells = |row: &str| -> Vec<String> {
        row.trim().trim_matches('│').split('│').map(|cell| cell.trim().to_string()).collect()
    };
    let rows: Vec<&str> = table.lines().filter(|row| row.starts_with('│')).collect();
    assert_eq!(cells(rows[0]), ["Location", "Kind", "Visibility", "Name", "Signature"]);
    assert!(rows.iter().any(|row| cells(row) == ["lib.rs:13", "method", "public", "grow", "(self, by: u32)"]), "{table}");
    assert!(rows.iter().any(|row| cells(row) == ["shapes.go:7", "method", "public", "Area", "() int"]), "{table}");
    // One table per file, so the report streams.
    assert_eq!(table.matches("│ Location ").count(), 2, "{table}");

    // `classes`: each container followed by its displayed members.
    let classes = scratch.stdout(&["classes", "--format", "table", "lib.rs"]);
    let names: Vec<String> = classes.lines().filter(|row| row.starts_with('│')).skip(1).map(|row| cells(row)[3].clone()).collect();
    assert_eq!(names, ["Widget", "new", "grow", "Wide", "Four"]);

    let empty = scratch.stdout(&["functions", "--format", "table", "--name", "nothing_matches_this"]);
    assert_eq!(empty, "(no symbols)\n");
}

/// The table's Location link names the declaring file when `hug` runs below
/// the scan root: the record's root-relative path is resolved against the
/// root, never the working directory (which once linked `src/src/lib.rs`).
#[test]
fn table_location_links_resolve_against_the_scan_root() {
    let scratch = Scratch::new(&[
        ("Cargo.toml", "[package]\nname = \"linked\"\nversion = \"0.1.0\"\nedition = \"2024\"\n"),
        ("src/lib.rs", "pub fn first() {}\n\npub fn second(by: u32) -> u32 {\n    by\n}\n"),
    ]);
    let links = |format: &[&str]| -> Vec<String> {
        let mut args = vec!["functions"];
        args.extend_from_slice(format);
        args.push("lib.rs");
        let output = scratch
            .process(&args)
            .current_dir(scratch.repo.path().join("src"))
            .env_remove("NO_COLOR")
            .env("CLICOLOR_FORCE", "1")
            .env("COLUMNS", "10000")
            .output()
            .expect("run hug");
        stdout_of(output, &args)
            .split("\x1b]8;;")
            .skip(1)
            .filter_map(|tail| tail.split(['\x1b', '\x07']).next())
            .filter(|url| url.starts_with("file://"))
            .map(str::to_string)
            .collect()
    };
    let table = links(&["--format", "table"]);
    let listing = links(&[]);
    assert_eq!(table.len(), 2, "one link per row: {table:?}");
    assert!(table.iter().all(|url| url.ends_with("/src/lib.rs#L1") || url.ends_with("/src/lib.rs#L3")), "{table:?}");
    assert!(table.iter().all(|url| !url.contains("/src/src/")), "{table:?}");
    // The listing links from the analyzed file's own path: the same targets.
    let listing_targets: Vec<&str> = listing.iter().filter(|url| url.contains("#L")).map(String::as_str).collect();
    assert_eq!(table, listing_targets);
}

/// Interactive output wraps at the terminal width (`COLUMNS` injects it)
/// with the list's hanging indent; plain and redirected output never wraps.
#[test]
fn interactive_output_wraps_at_the_injected_width() {
    let scratch = Scratch::new(&[(
        "wide.py",
        "def summarize_quarterly_revenue(regional_breakdown: dict[str, float], currency_table: dict[str, float]) -> float:\n    return 0.0\n",
    )]);
    let args = ["functions"];
    let wrapped = stdout_of(scratch.hug_with_env(&args, &[("CLICOLOR_FORCE", "1"), ("COLUMNS", "40")]), &args);
    let wrapped = visible(&wrapped);
    let rows: Vec<&str> = wrapped.lines().collect();
    assert!(rows.len() >= 3, "wrapped onto several rows: {wrapped}");
    for row in &rows {
        assert!(row.chars().count() <= 40, "a row wider than COLUMNS=40: {row:?}");
    }
    assert!(rows[0].starts_with("  - "), "{wrapped}");
    for row in &rows[1..] {
        assert!(row.starts_with("    ") && !row.trim_start().starts_with("- "), "a hanging continuation: {row:?}");
    }
    let squeeze = |text: &str| -> String { text.chars().filter(|c| !c.is_whitespace() && *c != '-').collect() };
    let plain = scratch.stdout(&["--plain", "functions"]);
    assert_eq!(plain.lines().count(), 1, "--plain never wraps: {plain}");
    assert_eq!(squeeze(&wrapped), squeeze(&plain), "the same text, only broken into rows");

    // Neither `--format plain` nor `--plain` wraps or styles, whatever the
    // width; redirected output (no forced color) neither.
    for args in [&["functions", "--format", "plain"][..], &["--plain", "functions"]] {
        let output = stdout_of(scratch.hug_with_env(args, &[("CLICOLOR_FORCE", "1"), ("COLUMNS", "40")]), args);
        assert_eq!(output, plain, "hug {args:?}");
    }
    let redirected = stdout_of(scratch.hug_with_env(&args, &[("COLUMNS", "40")]), &args);
    assert_eq!(redirected, plain, "redirected output never wraps");

    // The tree wraps under its own indent, too.
    let tree_args = ["functions", "--tree"];
    let tree = stdout_of(scratch.hug_with_env(&tree_args, &[("CLICOLOR_FORCE", "1"), ("COLUMNS", "40")]), &tree_args);
    let tree = visible(&tree);
    assert!(tree.lines().all(|row| row.chars().count() <= 40), "{tree}");
    assert!(tree.lines().count() >= 4, "{tree}");
}

/// Source text is shown, never interpreted: a control sequence in a
/// signature or doc comment is escaped, and Prose markup prints as written.
#[test]
fn control_sequences_and_markup_in_source_are_shown_literally() {
    let source = "def paint(color=\"\u{1b}[31m<red>**x**</red>\u{1b}]8;;http://evil\u{7}\"):\n    \"\"\"Doc \u{1b}[2J _it_ [a](b).\"\"\"\n    pass\n";
    let scratch = Scratch::new(&[("paint.py", source)]);
    let forced = [("CLICOLOR_FORCE", "1"), ("COLUMNS", "500")];
    for args in [
        &["functions", "--comments"][..],
        &["functions", "--signatures", "full"],
        &["functions", "--tree", "--comments"],
        &["functions", "--format", "table"],
        &["functions", "--format", "table", "--signatures", "full"],
    ] {
        let output = stdout_of(scratch.hug_with_env(args, &forced), args);
        assert!(output.contains('\u{1b}'), "styled output: {args:?}");
        assert!(!output.contains("\u{1b}[31m"), "hug {args:?} passed a color sequence through: {output:?}");
        assert!(!output.contains("\u{1b}]8;;http://evil"), "hug {args:?} passed a link through: {output:?}");
        assert!(!output.contains("\u{1b}[2J"), "hug {args:?} passed a clear-screen through: {output:?}");
        let visible = visible(&output);
        assert!(visible.contains("\\u{1b}[31m<red>**x**</red>\\u{1b}]8;;http://evil\\u{7}"), "hug {args:?}: {visible}");
        if args.contains(&"--comments") {
            assert!(visible.contains("Doc \\u{1b}[2J _it_ [a](b)."), "hug {args:?}: {visible}");
        }
    }
    // Plain output escapes the same way.
    let plain = scratch.stdout(&["--plain", "functions"]);
    assert!(!plain.contains('\u{1b}') && plain.contains("\\u{1b}[31m"), "{plain:?}");
}

/// `metrics.cyclomatic` reaches the reports that already request metrics,
/// and no other: `--json-full` carries the key on every record, a number
/// for a measured callable and `null` (never `0`) where it is unavailable,
/// identical cold, warm and uncached; `--json` and `--jsonl` listings
/// compute no metrics unless a selector reads them (`--undocumented`,
/// `--min-params`), and then carry the same values; `--stats` adds no
/// complexity statistic.
#[test]
fn cyclomatic_is_reported_only_where_metrics_already_are() {
    let scratch = Scratch::new(&[
        ("walks.rs", "fn branch(x: bool) -> u8 { if x && !x { 1 } else { 2 } }\ntrait T { fn bodyless(&self); }\n"),
        ("walks.js", "function coalesce(a) { return a ?? (a || 1); }\n"),
        ("walks.go", "package p\n\nfunc branch(a bool) int {\n\tif a {\n\t\treturn 1\n\t}\n\treturn 0\n}\n"),
    ]);
    let uncached = json_of(&scratch.stdout(&["--json-full", "functions", "--no-cache"]));
    let cold = json_of(&scratch.stdout(&["--json-full", "functions"]));
    let warm = json_of(&scratch.stdout(&["--json-full", "functions"]));
    assert_eq!(cold, uncached, "cold run");
    assert_eq!(warm, uncached, "warm run");
    let cyclomatic = |file: &str, name: &str| -> Value {
        let index = uncached["indexes"]
            .as_array()
            .expect("indexes")
            .iter()
            .find(|index| index["file"] == file)
            .unwrap_or_else(|| panic!("no index for {file}"));
        let record = index["symbols"]
            .as_array()
            .expect("symbols")
            .iter()
            .find(|record| record["identity"]["name"] == name)
            .unwrap_or_else(|| panic!("no `{name}` in {file}"));
        record["metrics"]
            .get("cyclomatic")
            .unwrap_or_else(|| panic!("`{name}` in {file} has no cyclomatic key"))
            .clone()
    };
    assert_eq!(cyclomatic("walks.rs", "branch"), Value::from(3));
    assert_eq!(cyclomatic("walks.rs", "bodyless"), Value::Null, "a bodyless declaration");
    assert_eq!(cyclomatic("walks.js", "coalesce"), Value::from(3));
    assert_eq!(cyclomatic("walks.go", "branch"), Value::Null, "a language without counting rules");

    let listing = json_of(&scratch.stdout(&["--json", "functions"]));
    let lines = lines_of(&scratch.stdout(&["functions", "--jsonl"]));
    assert_eq!(lines.len(), 4, "every function is listed");
    let capability_lists = listing["files"]
        .as_array()
        .expect("files")
        .iter()
        .chain(lines.iter())
        .map(|entry| &entry["completed_capabilities"]);
    for capabilities in capability_lists {
        assert!(!capabilities.as_array().expect("capabilities").contains(&Value::from("metrics")), "{capabilities}");
    }
    let records = json_records(&listing).into_iter().chain(lines.iter().map(|line| line["record"].clone()));
    for record in records {
        assert!(record.get("metrics").is_none(), "a listing record carries metrics: {record}");
    }

    let selected = json_of(&scratch.stdout(&["--json", "functions", "--undocumented"]));
    let selected_lines = lines_of(&scratch.stdout(&["functions", "--jsonl", "--min-params", "1"]));
    let selected_records: Vec<Value> = json_records(&selected)
        .into_iter()
        .chain(selected_lines.iter().map(|line| line["record"].clone()))
        .collect();
    assert_eq!(selected_records.len(), 4 + 3, "every undocumented function, every one with a parameter");
    for record in &selected_records {
        let file = record["source"]["file_path"].as_str().expect("file_path");
        let name = record["identity"]["name"].as_str().expect("name");
        let value = record["metrics"].get("cyclomatic").unwrap_or_else(|| panic!("`{name}` in {file}: no key: {record}"));
        assert_eq!(*value, cyclomatic(file, name), "`{name}` in {file}: {record}");
    }

    for args in [&["--json", "functions", "--stats"][..], &["functions", "--stats"]] {
        let stats = scratch.stdout(args);
        assert!(!stats.contains("cyclomatic") && !stats.contains("complexity"), "{args:?}: {stats}");
    }
}
