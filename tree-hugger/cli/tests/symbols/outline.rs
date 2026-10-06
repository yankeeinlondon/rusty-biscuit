//! Declared bases on `--tree` type rows: `extends A; implements B` from the
//! records' relations, in every signature mode, and the same facts in the
//! JSON formats.

use serde_json::Value;

use crate::{Scratch, json_of, stdout_of};

/// Supertraits (in the header), impl-derived `implements` (not in the
/// header, one of them also written in a bound), a method that keeps its
/// trait relation, and an impl on `&Local` that attaches to no type.
const RUST: &str = r#"pub trait Base {}
pub trait Shown: Base + std::fmt::Debug {}
pub struct W<T: std::fmt::Display>(T);
impl<T: std::fmt::Display> std::fmt::Display for W<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "w")
    }
}
impl Base for W<u8> {}
pub struct Local;
impl Base for &Local {
    fn by_ref(&self) {}
}
"#;

/// Both lists in the header, generic arguments kept as written.
const TYPESCRIPT: &str = "interface Shape {}
class Base<T> {}
class Square extends Base<number> implements Shape, Comparable<Square> {
    area(): number { return 1; }
}
";

/// Interface embeds: `extends` read from the body, never in the header.
const GO: &str = "package p\n\ntype Base interface{ M() }\n\ntype Reader interface {\n\tBase\n\tfmt.Stringer\n}\n";

fn scratch() -> Scratch {
    Scratch::new(&[("lib.rs", RUST), ("shapes.ts", TYPESCRIPT), ("reader.go", GO)])
}

fn rows(scratch: &Scratch, args: &[&str]) -> Vec<String> {
    scratch.stdout(args).lines().map(str::to_string).collect()
}

fn assert_rows(rows: &[String], expected: &[&str], context: &str) {
    for row in expected {
        assert!(rows.iter().any(|actual| actual == row), "{context}: expected row {row:?} in\n{}", rows.join("\n"));
    }
}

#[test]
fn type_rows_show_their_declared_bases_in_source_order() {
    let scratch = scratch();
    let expected = [
        "  public trait Base [1:1]",
        "  public trait Shown extends Base, std::fmt::Debug [2:1]",
        "  public struct W { 0: T } implements std::fmt::Display, Base [3:1]",
        // An impl on `&Local` attaches to no type: nothing is added.
        "  public struct Local [10:1]",
        "  public type Reader extends Base, fmt.Stringer [5:1]",
        "  class Square extends Base<number>; implements Shape, Comparable<Square> [3:1]",
    ];
    // `types` lists no members: a container with no displayed children
    // still shows its bases.
    let types = rows(&scratch, &["--plain", "types", "--tree", "--no-cache"]);
    assert_rows(&types, &expected, "types --tree");
    let symbols = rows(&scratch, &["--plain", "symbols", "--tree", "--no-cache"]);
    assert_rows(&symbols, &expected, "symbols --tree");

    // A Rust method keeps its trait relation in the record, but its row is
    // not an inheritance heading; nor is the textual `[in &Local]` heading.
    assert_rows(
        &symbols,
        &[
            "    method fmt(self, f: &mut std::fmt::Formatter) -> std::fmt::Result [5:5]",
            "  [in &Local]",
            "    public method by_ref(self) [12:5]",
            "    public method area(): number [4:5]",
        ],
        "symbols --tree",
    );
    let with_bases = symbols.iter().filter(|row| row.contains(" extends ") || row.contains(" implements ")).count();
    assert_eq!(with_bases, 4, "only the four type rows with bases:\n{}", symbols.join("\n"));
}

#[test]
fn functions_tree_adds_no_type_row_and_selectors_add_no_container_or_target() {
    let scratch = scratch();
    let functions = rows(&scratch, &["--plain", "functions", "--tree", "--no-cache"]);
    assert_eq!(
        functions,
        [
            "lib.rs",
            "  [in type W]",
            "    method fmt(self, f: &mut std::fmt::Formatter) -> std::fmt::Result [5:5]",
            // A reference self type names no local type, so its heading
            // carries no kind.
            "  [in &Local]",
            "    public method by_ref(self) [12:5]",
            "",
            "shapes.ts",
            "  [in class Square]",
            "    public method area(): number [4:5]",
            "",
        ],
        "{}",
        functions.join("\n")
    );

    // `Base` and `Shape` are named on the row, never added as rows.
    let selected = rows(&scratch, &["--plain", "types", "--tree", "--name", "Square", "--no-cache"]);
    assert_eq!(
        selected,
        ["shapes.ts", "  class Square extends Base<number>; implements Shape, Comparable<Square> [3:1]", ""],
        "{}",
        selected.join("\n")
    );
}

/// `full` shows the header as written: a list the header writes is not
/// repeated, and a list read from elsewhere (Rust impls, Go interface
/// bodies) still follows it, even when its text also appears in a bound.
/// `none` keeps the bases.
#[test]
fn signature_modes_keep_bases_without_repeating_the_header() {
    let scratch = scratch();
    let full = rows(&scratch, &["--plain", "types", "--tree", "--signatures", "full", "--no-cache"]);
    assert_rows(
        &full,
        &[
            "  pub trait Shown: Base + std::fmt::Debug [2:1]",
            "  pub struct W<T: std::fmt::Display> implements std::fmt::Display, Base [3:1]",
            "  type Reader interface extends Base, fmt.Stringer [5:1]",
            "  class Square extends Base<number> implements Shape, Comparable<Square> [3:1]",
        ],
        "--signatures full",
    );
    let none = rows(&scratch, &["--plain", "types", "--tree", "--signatures", "none", "--no-cache"]);
    assert_rows(
        &none,
        &[
            "  public trait Shown extends Base, std::fmt::Debug [2:1]",
            "  public struct W implements std::fmt::Display, Base [3:1]",
            "  public type Reader extends Base, fmt.Stringer [5:1]",
            "  class Square extends Base<number>; implements Shape, Comparable<Square> [3:1]",
        ],
        "--signatures none",
    );
}

#[test]
fn bases_follow_sorting_plain_output_and_wrapping() {
    let scratch = scratch();
    let plain = scratch.stdout(&["--plain", "types", "--tree", "shapes.ts", "--no-cache"]);
    assert_eq!(scratch.stdout(&["types", "--tree", "--format", "plain", "shapes.ts", "--no-cache"]), plain);
    let by_name = scratch.stdout(&["--plain", "types", "--tree", "--sort", "name", "shapes.ts", "--no-cache"]);
    assert_eq!(
        by_name.lines().collect::<Vec<_>>(),
        [
            "shapes.ts",
            "  class Base [2:1]",
            "  interface Shape [1:1]",
            "  class Square extends Base<number>; implements Shape, Comparable<Square> [3:1]",
            "",
        ],
        "{by_name}"
    );

    // Styled and wrapped: the clause wraps under the row's hanging indent,
    // and the `<…>` in a written base is text, never Prose markup.
    let args = ["types", "--tree", "shapes.ts", "--no-cache"];
    let styled = stdout_of(scratch.hug_with_env(&args, &[("CLICOLOR_FORCE", "1"), ("COLUMNS", "40")]), &args);
    let visible = biscuit_terminal::prelude::strip_escape_codes(biscuit_terminal::prelude::strip_osc8_links(&styled));
    assert!(visible.lines().all(|row| row.chars().count() <= 40), "{visible}");
    let joined = visible.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(
        joined.contains("class Square extends Base<number>; implements Shape, Comparable<Square> [3:1]"),
        "{visible}"
    );
    assert!(visible.lines().count() > plain.lines().count(), "the Square row wraps at 40 columns:\n{visible}");
}

/// Counts and statistics are unchanged by relations: no new category.
#[test]
fn counts_and_stats_gain_no_relation_category() {
    let scratch = scratch();
    let records = json_records(&json_of(&scratch.stdout(&["--json", "types", "--no-cache"])));
    assert_eq!(scratch.stdout(&["types", "--count", "--no-cache"]), format!("{}\n", records.len()));
    for args in [&["types", "--stats", "--no-cache"][..], &["--json", "types", "--stats", "--no-cache"]] {
        let stats = scratch.stdout(args);
        assert!(!stats.contains("extends") && !stats.contains("implements"), "{args:?}: {stats}");
    }
}

/// `--json`, `--jsonl` and `--json-full` carry the relations the outline
/// shows, and the complete index carries `metrics.lines`.
#[test]
fn json_formats_carry_the_outline_relations_and_body_lines() {
    let scratch = scratch();
    let json = json_records(&json_of(&scratch.stdout(&["--json", "symbols", "--no-cache"])));
    let jsonl: Vec<Value> = scratch
        .stdout(&["symbols", "--jsonl", "--no-cache"])
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).expect("line")["record"].clone())
        .collect();
    let full_report = json_of(&scratch.stdout(&["--json-full", "symbols", "--no-cache"]));
    let full: Vec<Value> = full_report["indexes"]
        .as_array()
        .expect("indexes")
        .iter()
        .flat_map(|index| index["symbols"].as_array().expect("symbols").iter().cloned())
        .collect();
    let tree = rows(&scratch, &["--plain", "symbols", "--tree", "--no-cache"]);

    for (name, extends, implements) in [
        ("Shown", &["Base", "std::fmt::Debug"][..], &[][..]),
        ("W", &[], &["std::fmt::Display", "Base"]),
        ("Reader", &["Base", "fmt.Stringer"], &[]),
        ("Square", &["Base<number>"], &["Shape", "Comparable<Square>"]),
    ] {
        let relations = |records: &[Value]| -> (Vec<String>, Vec<String>) {
            let record = one(records, name);
            (written(&record["relations"]["extends"]), written(&record["relations"]["implements"]))
        };
        let expected = (to_strings(extends), to_strings(implements));
        assert_eq!(relations(&json), expected, "{name}: --json");
        assert_eq!(relations(&jsonl), expected, "{name}: --jsonl");
        assert_eq!(relations(&full), expected, "{name}: --json-full");

        let mut clauses = Vec::new();
        if !extends.is_empty() {
            clauses.push(format!("extends {}", extends.join(", ")));
        }
        if !implements.is_empty() {
            clauses.push(format!("implements {}", implements.join(", ")));
        }
        let clause = format!(" {} [", clauses.join("; "));
        assert!(
            tree.iter().any(|row| row.contains(&format!(" {name}")) && row.contains(&clause)),
            "{name}: the outline row shows {clause:?}:\n{}",
            tree.join("\n")
        );
    }

    // The method keeps its trait relation in every format.
    for records in [&json, &jsonl, &full] {
        assert_eq!(written(&one(records, "fmt")["relations"]["implements"]), ["std::fmt::Display"]);
    }

    // `metrics.lines` counts the body in the complete index; a listing does
    // not request metrics, so its records carry no facet at all.
    assert_eq!(one(&full, "fmt")["metrics"]["lines"], 3);
    assert_eq!(one(&full, "by_ref")["metrics"]["lines"], 1);
    assert_eq!(one(&full, "Square")["metrics"]["lines"], Value::Null, "a type has no body-line count");
    assert!(json.iter().chain(&jsonl).all(|record| record.get("metrics").is_none()));
}

fn json_records(report: &Value) -> Vec<Value> {
    report["files"]
        .as_array()
        .expect("files")
        .iter()
        .flat_map(|file| file["symbols"].as_array().expect("symbols").iter().cloned())
        .collect()
}

fn one<'a>(records: &'a [Value], name: &str) -> &'a Value {
    let found: Vec<&Value> = records.iter().filter(|record| record["identity"]["name"] == name).collect();
    assert_eq!(found.len(), 1, "one `{name}` record");
    found[0]
}

/// The written names of a relation list; an absent list is empty.
fn written(list: &Value) -> Vec<String> {
    list.as_array()
        .map(|entries| entries.iter().map(|entry| entry["name"].as_str().expect("name").to_string()).collect())
        .unwrap_or_default()
}

fn to_strings(names: &[&str]) -> Vec<String> {
    names.iter().map(|name| (*name).to_string()).collect()
}

#[test]
fn mixed_bases_and_swift_extension_conformances_survive_signature_modes_and_cache() {
    let swift = "protocol P {}\nprotocol Q {}\nstruct Box: P {}\nextension Box: Q {}\nclass Unknown: External {}\n";
    let scratch = Scratch::new(&[("mixed.swift", swift)]);
    for mode in ["none", "short", "full"] {
        let args = ["--plain", "types", "--tree", "--signatures", mode, "mixed.swift"];
        let cold = scratch.stdout(&args);
        assert_eq!(scratch.stdout(&args), cold, "{mode}: warm output");
        let owner = cold.lines().find(|row| row.contains(" Box")).expect("Box row");
        if mode == "full" {
            assert!(owner.contains("Box: P implements Q"), "{owner}");
            assert!(!owner.contains("implements P"), "header conformance must not repeat: {owner}");
        } else {
            assert!(owner.contains("implements P, Q"), "{mode}: {owner}");
        }
        let unknown = cold.lines().find(|row| row.contains(" Unknown")).expect("Unknown row");
        if mode == "full" {
            assert!(unknown.contains("Unknown: External"), "{unknown}");
            assert!(!unknown.contains("unclassified bases"), "header base must not repeat: {unknown}");
        } else {
            assert!(unknown.contains("unclassified bases External"), "{unknown}");
        }
    }
    scratch.stdout(&["--json", "types", "mixed.swift"]);
    let output = scratch.hug(&["--verbose", "--json", "types", "mixed.swift"]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("1 hits") && stderr.contains("parsed 0 files"), "{stderr}");
    let json = json_of(&stdout_of(output, &["--json", "types"]));
    let box_record = json["files"][0]["symbols"].as_array().expect("symbols").iter()
        .find(|record| record["identity"]["name"] == "Box").expect("Box");
    assert_eq!(written(&box_record["relations"]["implements"]), ["P", "Q"]);
    let source = &box_record["relations"]["implements"][1]["source_span"];
    assert_eq!(source["start"]["line"], 4, "extension evidence survives the CLI cache");
}
