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

#[test]
fn csharp_record_class_targets_render_as_extends_cold_and_warm() {
    let csharp = "record ReviewBase;\nrecord ReviewChild : ReviewBase;\nrecord ReviewGeneric<T>;\n\
                  record ReviewGenericChild : ReviewGeneric<int>;\nstruct ReviewStruct {}\n\
                  class ReviewStructChild : ReviewStruct {}\n";
    let scratch = Scratch::new(&[("records.cs", csharp)]);
    let args = ["--plain", "types", "--tree", "records.cs"];
    let cold = rows(&scratch, &args);
    let expected = [
        "  internal record ReviewChild extends ReviewBase [2:1]",
        "  internal record ReviewGenericChild extends ReviewGeneric<int> [4:1]",
        // A struct is never a superclass; its category stays unknown.
        "  internal class ReviewStructChild unclassified bases ReviewStruct [6:1]",
    ];
    assert_rows(&cold, &expected, "cold types --tree");
    assert_eq!(rows(&scratch, &args), cold, "warm output");
}

/// A Rust impl beside a finished block that declares a same-named enum and
/// trait: the block's declarations are out of view, so the impl belongs to
/// the file-level struct and implements the file-level trait. Checked on a
/// member-only listing (container kind, trait ID), after widening that
/// entry to a containment tree, and on a genuine warm hit of the full index.
const SIBLING_BLOCK_OWNER: &str = "pub struct ReviewSelf;\npub trait ReviewTrait {}\nfn f() {\n    \
                                   { enum ReviewSelf {} trait ReviewTrait {} }\n    \
                                   impl ReviewTrait for ReviewSelf { fn probe(&self) {} }\n}\n";

fn by_qualified<'a>(records: &'a [Value], qualified_name: &str) -> &'a Value {
    let found: Vec<&Value> = records.iter().filter(|record| record["identity"]["qualified_name"] == qualified_name).collect();
    assert_eq!(found.len(), 1, "one `{qualified_name}` record");
    found[0]
}

#[test]
fn rust_impl_beside_a_sibling_blocks_types_belongs_to_the_file_level_struct() {
    let scratch = Scratch::new(&[("owner.rs", SIBLING_BLOCK_OWNER)]);

    // Member-only: declarations of functions alone, cached by kind.
    let functions = json_records(&json_of(&scratch.stdout(&["--json", "functions", "--all-scopes", "owner.rs"])));
    let probe = one(&functions, "probe");
    assert_eq!(probe["identity"]["container_kind"], "Type", "the struct's kind, never the block's enum's");
    let trait_target = &probe["relations"]["implements"][0];
    assert_eq!(trait_target["name"], "ReviewTrait");
    assert_eq!(trait_target["qualified_name"], "ReviewTrait", "the file-level trait");

    // Widened from the function-only entry to a containment tree.
    let output = scratch.hug(&["--verbose", "--plain", "symbols", "--tree", "--all-scopes", "owner.rs"]);
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    assert!(stderr.contains("1 partial hits"), "the declarations entry is widened: {stderr}");
    let tree: Vec<String> = stdout_of(output, &["symbols", "--tree"]).lines().map(str::to_string).collect();
    let owner = tree.iter().position(|row| row == "  public struct ReviewSelf implements ReviewTrait [1:1]")
        .unwrap_or_else(|| panic!("struct row in\n{}", tree.join("\n")));
    assert_eq!(tree[owner + 1], "    public method probe(self) [5:39]", "{}", tree.join("\n"));
    let enum_row = tree.iter().position(|row| row.trim_start().starts_with("enum ReviewSelf")).expect("enum row");
    assert!(
        tree.get(enum_row + 1).is_none_or(|next| !next.contains("probe")),
        "the block's enum owns nothing: {}",
        tree.join("\n")
    );

    // A genuine warm hit of the complete index keeps the same identities.
    scratch.stdout(&["--json-full", "symbols", "owner.rs"]);
    let output = scratch.hug(&["--verbose", "--json-full", "symbols", "owner.rs"]);
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    assert!(stderr.contains("1 hits") && stderr.contains("parsed 0 files"), "{stderr}");
    let full = json_of(&stdout_of(output, &["--json-full", "symbols"]));
    let records: Vec<Value> = full["indexes"][0]["symbols"].as_array().expect("symbols").clone();
    let structure = by_qualified(&records, "owner.rs#ReviewSelf");
    let file_trait = by_qualified(&records, "owner.rs#ReviewTrait");
    let block_enum = by_qualified(&records, "owner.rs#f::ReviewSelf");
    let probe = one(&records, "probe");
    assert_eq!(structure["relations"]["implements"][0]["id"], file_trait["id"], "struct conformance");
    assert_eq!(probe["relations"]["implements"][0]["id"], file_trait["id"], "member's trait");
    assert_eq!(probe["relations"]["parent"]["id"], structure["id"], "member's parent");
    assert_eq!(probe["relations"]["container"]["id"], structure["id"], "member's container");
    assert_eq!(probe["identity"]["container_kind"], "Type");
    let members: Vec<&Value> = structure["relations"]["members"].as_array().expect("members").iter().collect();
    assert!(members.iter().any(|member| member["id"] == probe["id"]), "the struct lists its member: {members:?}");
    assert!(block_enum["relations"].get("implements").is_none(), "the block's enum conforms to nothing");
    assert!(block_enum["relations"].get("members").is_none(), "the block's enum owns nothing");
}

/// Impls inside a function whose self type, trait or both are paths through
/// that function's local module: each path names the module's own
/// declaration, so the module's struct gains the file-level trait, the
/// function's struct gains the module's trait, and the same-named file-level
/// struct and trait gain nothing.
const LOCAL_MODULE_OWNER: &str = "pub struct ReviewSelf;\npub trait ReviewTrait {}\nfn f() {\n    \
    mod reviewHolder { pub struct ReviewSelf; pub trait ReviewTrait {} pub struct ReviewOther; }\n    \
    struct ReviewLocal;\n    \
    impl ReviewTrait for reviewHolder::ReviewSelf { fn probe_self(&self) {} }\n    \
    impl reviewHolder::ReviewTrait for ReviewLocal { fn probe_trait(&self) {} }\n    \
    impl reviewHolder::ReviewTrait for reviewHolder::ReviewOther { fn probe_both(&self) {} }\n}\n";

/// `(method, owner's in-file qualified name, trait's in-file qualified name)`.
const LOCAL_MODULE_IMPLS: &[(&str, &str, &str)] = &[
    ("probe_self", "f::reviewHolder::ReviewSelf", "ReviewTrait"),
    ("probe_trait", "f::ReviewLocal", "f::reviewHolder::ReviewTrait"),
    ("probe_both", "f::reviewHolder::ReviewOther", "f::reviewHolder::ReviewTrait"),
];

/// The local-module impls on a member-only listing (container kind, trait
/// ID), after widening that entry to a containment tree, and on a genuine
/// warm hit of the full index (owners, conformance, membership).
#[test]
fn rust_impls_through_a_local_module_belong_to_its_declarations() {
    let scratch = Scratch::new(&[("owner.rs", LOCAL_MODULE_OWNER)]);

    // Member-only: functions alone, without Containment.
    let functions = json_records(&json_of(&scratch.stdout(&["--json", "functions", "--all-scopes", "owner.rs"])));
    for &(method, _, trait_name) in LOCAL_MODULE_IMPLS {
        let probe = one(&functions, method);
        assert_eq!(probe["identity"]["container_kind"], "Type", "`{method}`: the local struct's kind");
        let target = &probe["relations"]["implements"][0];
        assert_eq!(target["qualified_name"], trait_name, "`{method}`: {target}");
        assert!(target["id"].is_string(), "`{method}`: trait ID: {target}");
    }

    // Widened from the function-only entry to a containment tree.
    let output = scratch.hug(&["--verbose", "--plain", "symbols", "--tree", "--all-scopes", "owner.rs"]);
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    assert!(stderr.contains("1 partial hits"), "the declarations entry is widened: {stderr}");
    let tree: Vec<String> = stdout_of(output, &["symbols", "--tree"]).lines().map(str::to_string).collect();
    for (owner, member) in [
        ("    struct ReviewSelf implements ReviewTrait [4:24]", "      public method probe_self(self) [6:53]"),
        ("  struct ReviewLocal implements reviewHolder::ReviewTrait [5:5]", "    public method probe_trait(self) [7:54]"),
        ("    struct ReviewOther implements reviewHolder::ReviewTrait [4:72]", "      public method probe_both(self) [8:68]"),
    ] {
        let row = tree.iter().position(|row| row == owner).unwrap_or_else(|| panic!("{owner:?} in\n{}", tree.join("\n")));
        assert_eq!(tree[row + 1], member, "{}", tree.join("\n"));
    }
    assert_rows(&tree, &["  public struct ReviewSelf [1:1]", "  public trait ReviewTrait [2:1]"], "file-level items gain nothing");

    // A genuine warm hit of the complete index keeps the same identities.
    scratch.stdout(&["--json-full", "symbols", "owner.rs"]);
    let output = scratch.hug(&["--verbose", "--json-full", "symbols", "owner.rs"]);
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    assert!(stderr.contains("1 hits") && stderr.contains("parsed 0 files"), "{stderr}");
    let full = json_of(&stdout_of(output, &["--json-full", "symbols"]));
    let records: Vec<Value> = full["indexes"][0]["symbols"].as_array().expect("symbols").clone();
    for &(method, owner, trait_name) in LOCAL_MODULE_IMPLS {
        let owner = by_qualified(&records, &format!("owner.rs#{owner}"));
        let trait_record = by_qualified(&records, &format!("owner.rs#{trait_name}"));
        let probe = one(&records, method);
        assert_eq!(owner["relations"]["implements"][0]["id"], trait_record["id"], "`{method}`: owner conformance");
        assert_eq!(probe["relations"]["implements"][0]["id"], trait_record["id"], "`{method}`: member's trait");
        assert_eq!(probe["relations"]["parent"]["id"], owner["id"], "`{method}`: parent");
        assert_eq!(probe["relations"]["container"]["id"], owner["id"], "`{method}`: container");
        assert_eq!(probe["identity"]["container_kind"], "Type", "`{method}`");
        let members = owner["relations"]["members"].as_array().expect("members");
        assert!(members.iter().any(|member| member["id"] == probe["id"]), "`{method}`: listed by its owner");
    }
    for file_level in ["owner.rs#ReviewSelf", "owner.rs#ReviewTrait"] {
        let record = by_qualified(&records, file_level);
        assert!(record["relations"].get("implements").is_none(), "`{file_level}` conforms to nothing");
        assert!(record["relations"].get("members").is_none(), "`{file_level}` owns nothing");
    }
}

/// A Swift extension of a generic owner sees the owner's type parameter:
/// a nested class whose base spells it keeps the base as written, on a
/// cold run and on a genuine warm hit, while the unparameterized control
/// resolves to the file-level class.
#[test]
fn swift_extension_of_a_generic_owner_keeps_its_parameter_cold_and_warm() {
    let swift = "class ReviewBase {}\nstruct ReviewBox<ReviewBase> {}\nextension ReviewBox {\n    class ReviewChild: ReviewBase {}\n}\n\
                 struct ReviewPlain {}\nextension ReviewPlain {\n    class ReviewChild: ReviewBase {}\n}\n";
    let scratch = Scratch::new(&[("box.swift", swift)]);
    let check = |records: &[Value], context: &str| {
        let base = by_qualified(records, "box.swift#ReviewBase");
        let boxed = by_qualified(records, "box.swift#ReviewBox.ReviewChild");
        let target = &boxed["relations"]["unclassified_bases"][0];
        assert_eq!(target["name"], "ReviewBase", "{context}");
        assert!(target.get("id").is_none() && target.get("qualified_name").is_none(), "{context}: {target}");
        assert!(boxed["relations"].get("extends").is_none(), "{context}");
        let plain = by_qualified(records, "box.swift#ReviewPlain.ReviewChild");
        assert_eq!(plain["relations"]["extends"][0]["id"], base["id"], "{context}");
    };
    let cold = json_records(&json_of(&scratch.stdout(&["--json", "types", "box.swift"])));
    check(&cold, "cold");
    let output = scratch.hug(&["--verbose", "--json", "types", "box.swift"]);
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    assert!(stderr.contains("1 hits") && stderr.contains("parsed 0 files"), "{stderr}");
    check(&json_records(&json_of(&stdout_of(output, &["--json", "types"]))), "warm");
}

/// A Swift extension of a nested file-level type beside a closure's
/// same-named owner: the method sits under the file-level nested type,
/// which shows the conformance, never under an unresolved `[in …]` heading;
/// a genuine warm hit of the complete index keeps the parent, container
/// and membership IDs.
#[test]
fn swift_extension_member_beside_a_closure_shadow_belongs_to_the_file_level_type() {
    let swift = "struct ReviewHolder { struct ReviewInner {} }\n\
                 let reviewClosure = { struct ReviewHolder { struct ReviewInner {} } }\n\
                 protocol ReviewP {}\n\
                 extension ReviewHolder.ReviewInner: ReviewP { func reviewMethod() {} }\n";
    let scratch = Scratch::new(&[("shadow.swift", swift)]);
    let tree = rows(&scratch, &["--plain", "symbols", "--tree", "shadow.swift"]);
    let owner = "    internal struct ReviewInner implements ReviewP [1:23]";
    let row = tree.iter().position(|row| row == owner).unwrap_or_else(|| panic!("{owner:?} in\n{}", tree.join("\n")));
    assert_eq!(tree[row + 1], "      internal method reviewMethod() [4:47]", "{}", tree.join("\n"));
    assert!(
        !tree.iter().any(|row| row.contains("[in type ReviewHolder.ReviewInner]")),
        "no unresolved owner heading:\n{}",
        tree.join("\n")
    );

    scratch.stdout(&["--json-full", "symbols", "shadow.swift"]);
    let output = scratch.hug(&["--verbose", "--json-full", "symbols", "shadow.swift"]);
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    assert!(stderr.contains("1 hits") && stderr.contains("parsed 0 files"), "{stderr}");
    let full = json_of(&stdout_of(output, &["--json-full", "symbols"]));
    let records: Vec<Value> = full["indexes"][0]["symbols"].as_array().expect("symbols").clone();
    let owner = records
        .iter()
        .find(|record| {
            record["identity"]["qualified_name"] == "shadow.swift#ReviewHolder.ReviewInner"
                && record["source"]["declaration_span"]["start"]["line"] == 1
        })
        .expect("file-level ReviewInner");
    let method = by_qualified(&records, "shadow.swift#ReviewHolder.ReviewInner.reviewMethod");
    assert_eq!(method["relations"]["parent"]["id"], owner["id"], "parent");
    assert_eq!(method["relations"]["container"]["id"], owner["id"], "container");
    let members = owner["relations"]["members"].as_array().expect("members");
    assert!(members.iter().any(|member| member["id"] == method["id"]), "listed by its owner");
    let protocol = by_qualified(&records, "shadow.swift#ReviewP");
    assert_eq!(owner["relations"]["implements"][0]["id"], protocol["id"], "conformance");
}

/// A Swift extension of a nested owner whose intermediate name two
/// file-level types share proves no owner: the method sits under an
/// `[in …]` heading named as written, the nested type neither lists it nor
/// takes the extension's conformance, cold and on a genuine warm hit.
#[test]
fn swift_extension_member_of_an_ambiguous_owner_stays_under_its_written_owner() {
    let swift = "protocol ReviewP {}\n\
                 struct ReviewHolder { struct ReviewBase {} }\n\
                 struct ReviewHolder {}\n\
                 extension ReviewHolder.ReviewBase: ReviewP { func reviewProbe() {} }\n";
    let scratch = Scratch::new(&[("ambiguous.swift", swift)]);
    let tree = rows(&scratch, &["--plain", "symbols", "--tree", "ambiguous.swift"]);
    let owner = "    internal struct ReviewBase [2:23]";
    assert!(tree.iter().any(|row| row == owner), "{owner:?} without a conformance in\n{}", tree.join("\n"));
    let heading = tree
        .iter()
        .position(|row| row.trim() == "[in type ReviewHolder.ReviewBase]")
        .unwrap_or_else(|| panic!("an unresolved owner heading in\n{}", tree.join("\n")));
    assert_eq!(tree[heading + 1].trim(), "internal method reviewProbe() [4:46]", "{}", tree.join("\n"));

    scratch.stdout(&["--json-full", "symbols", "ambiguous.swift"]);
    let output = scratch.hug(&["--verbose", "--json-full", "symbols", "ambiguous.swift"]);
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    assert!(stderr.contains("1 hits") && stderr.contains("parsed 0 files"), "{stderr}");
    let full = json_of(&stdout_of(output, &["--json-full", "symbols"]));
    let records: Vec<Value> = full["indexes"][0]["symbols"].as_array().expect("symbols").clone();
    let method = by_qualified(&records, "ambiguous.swift#ReviewHolder.ReviewBase.reviewProbe");
    assert!(method["relations"]["parent"].is_null(), "parent: {}", method["relations"]["parent"]);
    assert_eq!(method["relations"]["container"]["name"], "ReviewHolder.ReviewBase", "textual container");
    assert!(method["relations"]["container"]["id"].is_null(), "container ID");
    let base = by_qualified(&records, "ambiguous.swift#ReviewHolder.ReviewBase");
    assert!(base["relations"]["members"].as_array().is_none_or(Vec::is_empty), "members: {}", base["relations"]["members"]);
    assert!(base["relations"]["implements"].as_array().is_none_or(Vec::is_empty), "conformance");
}

/// A Python class member rebound by a class-body import names no
/// same-file declaration: the base keeps its text with no ID or qualified
/// name, cold and on a genuine warm hit, while the unedited holder's base
/// keeps the nested class's ID.
#[test]
fn python_member_rebound_by_an_import_keeps_only_its_text_cold_and_warm() {
    let python = "class ReviewHolder:\n    class ReviewBase:\n        pass\n    from builtins import object as ReviewBase\n\
                  class ReviewChild(ReviewHolder.ReviewBase):\n    pass\n\
                  class ReviewPlain:\n    class ReviewBase:\n        pass\n\
                  class ReviewPlainChild(ReviewPlain.ReviewBase):\n    pass\n";
    let scratch = Scratch::new(&[("probe.py", python)]);
    let check = |records: &[Value], context: &str| {
        let child = by_qualified(records, "probe.py#ReviewChild");
        let target = &child["relations"]["extends"][0];
        assert_eq!(target["name"], "ReviewHolder.ReviewBase", "{context}");
        assert!(target.get("id").is_none() && target.get("qualified_name").is_none(), "{context}: {target}");
        let plain = by_qualified(records, "probe.py#ReviewPlainChild");
        let base = by_qualified(records, "probe.py#ReviewPlain.ReviewBase");
        assert_eq!(plain["relations"]["extends"][0]["id"], base["id"], "{context}: control");
    };
    check(&json_records(&json_of(&scratch.stdout(&["--json", "types", "--all-scopes", "probe.py"]))), "cold");
    let output = scratch.hug(&["--verbose", "--json", "types", "--all-scopes", "probe.py"]);
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    assert!(stderr.contains("1 hits") && stderr.contains("parsed 0 files"), "{stderr}");
    check(&json_records(&json_of(&stdout_of(output, &["--json", "types"]))), "warm");
}

/// Nested extensions keep enclosing parameters textual and retain raw-looking
/// bases in outlines, without disturbing containment or actual warm hits.
#[test]
fn swift_nested_generic_extensions_keep_textual_bases_and_conformance_cold_and_warm() {
    let source = "class ReviewBase {}\nprotocol ReviewP {}\nstruct ReviewOuter<ReviewBase, ReviewP, Int> { struct ReviewInner {} }\nextension ReviewOuter.ReviewInner: ReviewP {\n    class ReviewChild: ReviewBase {}\n    enum ReviewRaw: Int { case a }\n}\n";
    let scratch = Scratch::new(&[("nested.swift", source)]);
    let check = |records: &[Value]| {
        let owner = by_qualified(records, "nested.swift#ReviewOuter.ReviewInner");
        let conformance = &owner["relations"]["implements"][0];
        assert_eq!(conformance["name"], "ReviewP");
        assert!(conformance.get("id").is_none() && conformance.get("qualified_name").is_none());
        for (name, base) in [("ReviewChild", "ReviewBase"), ("ReviewRaw", "Int")] {
            let record = by_qualified(records, &format!("nested.swift#ReviewOuter.ReviewInner.{name}"));
            let target = &record["relations"]["unclassified_bases"][0];
            assert_eq!(target["name"], base);
            assert!(target.get("id").is_none() && target.get("qualified_name").is_none());
            assert!(record["relations"].get("extends").is_none());
            assert_eq!(record["relations"]["parent"]["id"], owner["id"]);
            assert!(record["type_info"].get("generic_params").is_none());
        }
    };
    let cold = json_of(&scratch.stdout(&["--json-full", "symbols", "nested.swift"]));
    check(cold["indexes"][0]["symbols"].as_array().expect("symbols"));
    for mode in ["none", "short", "full"] {
        let output = scratch.hug(&["--verbose", "--plain", "types", "--tree", "--signatures", mode, "nested.swift"]);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("1 hits") && stderr.contains("parsed 0 files"), "{stderr}");
        let outline = stdout_of(output, &["types", "--tree"]);
        assert!(outline.contains("ReviewInner implements ReviewP"), "{mode}: {outline}");
        if mode != "full" {
            assert!(outline.contains("ReviewChild unclassified bases ReviewBase"), "{outline}");
            assert!(outline.contains("ReviewRaw unclassified bases Int"), "{outline}");
        } else {
            assert!(outline.contains("ReviewChild: ReviewBase") && outline.contains("ReviewRaw: Int"), "{outline}");
        }
        assert!(!outline.contains("extends ReviewBase"), "{outline}");
    }
    let output = scratch.hug(&["--verbose", "--json-full", "symbols", "nested.swift"]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("1 hits") && stderr.contains("parsed 0 files"), "{stderr}");
    let warm = json_of(&stdout_of(output, &["--json-full", "symbols"]));
    check(warm["indexes"][0]["symbols"].as_array().expect("symbols"));
}
