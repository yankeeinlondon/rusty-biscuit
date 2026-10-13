//! `hug classes` on records: which containers it summarizes, its static and
//! instance sections, its JSON `members`, and the field metadata it keeps
//! without selecting Field, Property or Constant records.

use std::collections::BTreeSet;

use serde_json::Value;

use crate::filters::DEFERRED_FILES;
use crate::golden::MULTI_FIXTURES;
use crate::{JAVA_FIXTURE, Scratch, json_of};

const CONTAINER_KINDS: [&str; 4] = ["Class", "Type", "Interface", "Trait"];

fn with_id<'a>(records: &'a [Value], id: &Value) -> &'a Value {
    records
        .iter()
        .find(|record| record["id"] == *id)
        .unwrap_or_else(|| panic!("no record with ID {id}"))
}

/// Interfaces and traits are summarized with classes and types; enums,
/// modules and other kinds are not.
#[test]
fn classes_summarize_classes_types_interfaces_and_traits() {
    let scratch = Scratch::new(MULTI_FIXTURES);
    let stdout = scratch.stdout(&["--plain", "classes"]);
    for container in [
        "interface IAuditable",
        "trait Named",
        "interface Readable",
        "interface Resettable",
        "class Ledger",
        "type Widget",
    ] {
        assert!(stdout.contains(&format!("  {container} [")), "`{container}` is summarized:\n{stdout}");
    }
    assert!(!stdout.contains(" Shape "), "an enum is not summarized:\n{stdout}");

    let json = json_of(&scratch.stdout(&["--json", "classes"]));
    for file in json["files"].as_array().expect("files") {
        let members = file["members"].as_object().expect("a classes entry has members");
        for container in file["symbols"].as_array().expect("symbols") {
            assert!(
                CONTAINER_KINDS.contains(&container["kind"].as_str().unwrap_or_default()),
                "{}: `symbols` holds containers only ({})",
                file["file"],
                container["kind"]
            );
            let id = container["id"].as_str().expect("id");
            let listed = members.get(id).and_then(Value::as_array).expect("a members entry per container");
            for member in listed {
                assert!(
                    ["Method", "Constructor"].contains(&member["kind"].as_str().unwrap_or_default()),
                    "members are methods and constructors ({})",
                    member["kind"]
                );
                assert_eq!(member["relations"]["parent"]["id"], id, "the member belongs to its container");
            }
        }
        assert_eq!(members.len(), file["symbols"].as_array().expect("symbols").len());
    }
}

/// The terminal sections keep their split: `--static-only` and
/// `--instance-only` empty the other sections, and together they are an
/// argument error.
#[test]
fn static_and_instance_sections_follow_the_member_flags() {
    let scratch = Scratch::new(&[("java.java", JAVA_FIXTURE)]);
    let all = scratch.stdout(&["--plain", "classes", "java.java"]);
    for section in ["Static Methods", "Instance Methods", "Static Fields", "Instance Fields"] {
        assert!(all.contains(section), "{section} in:\n{all}");
    }
    let static_only = scratch.stdout(&["--plain", "classes", "java.java", "--static-only"]);
    assert!(static_only.contains("Static Methods") && static_only.contains("Static Fields"));
    assert!(!static_only.contains("Instance Methods") && !static_only.contains("Instance Fields"));
    let instance_only = scratch.stdout(&["--plain", "classes", "java.java", "--instance-only"]);
    assert!(instance_only.contains("Instance Methods") && instance_only.contains("Instance Fields"));
    assert!(!instance_only.contains("Static Methods") && !instance_only.contains("Static Fields"));
    let both = scratch.hug(&["--plain", "classes", "java.java", "--static-only", "--instance-only"]);
    assert_eq!(both.status.code(), Some(2), "an argument error");
    assert!(
        String::from_utf8_lossy(&both.stderr).contains("cannot be used with"),
        "{}",
        String::from_utf8_lossy(&both.stderr)
    );
    assert!(both.stdout.is_empty());
}

/// A container's field list keeps each field's access and `static` flag
/// although the `classes` query selects no Field, Property or Constant:
/// it equals the complete index's list for every container.
#[test]
fn container_fields_keep_access_and_static_without_member_records() {
    let scratch = Scratch::new(MULTI_FIXTURES);
    let narrow = json_of(&scratch.stdout(&["--json", "classes"]));
    let full = json_of(&scratch.stdout(&["--json-full", "classes"]));
    let mut compared = 0;
    let mut with_access = 0;
    for (file, index) in narrow["files"].as_array().expect("files").iter().zip(
        full["indexes"]
            .as_array()
            .expect("indexes")
            .iter()
            .filter(|index| narrow["files"].as_array().expect("files").iter().any(|file| file["file"] == index["file"])),
    ) {
        assert_eq!(file["file"], index["file"]);
        let complete = index["symbols"].as_array().expect("symbols");
        for container in file["symbols"].as_array().expect("symbols") {
            let name = container["identity"]["name"].as_str().expect("name");
            let fields = &container["kind_data"]["data"]["fields"];
            assert_eq!(
                *fields,
                with_id(complete, &container["id"])["kind_data"]["data"]["fields"],
                "{}: `{name}` fields",
                file["file"]
            );
            for field in fields.as_array().into_iter().flatten() {
                compared += 1;
                if field.get("visibility").is_some_and(|level| level != "Unknown") || field["is_static"] == true {
                    with_access += 1;
                }
            }
        }
    }
    assert!(compared >= 8, "too few fields compared ({compared})");
    assert!(with_access >= 4, "too few fields with access or static ({with_access})");
}

/// `(name, kind)` of each member `hug --json classes` lists under the
/// container called `container`.
fn members_of(report: &Value, container: &str) -> Vec<(String, String)> {
    for file in report["files"].as_array().expect("files") {
        for record in file["symbols"].as_array().expect("symbols") {
            if record["identity"]["name"] == container {
                return file["members"][record["id"].as_str().expect("id")]
                    .as_array()
                    .expect("members")
                    .iter()
                    .map(|member| {
                        (
                            member["identity"]["name"].as_str().expect("name").to_string(),
                            member["kind"].as_str().expect("kind").to_string(),
                        )
                    })
                    .collect();
            }
        }
    }
    panic!("no `{container}` container in {report}");
}

fn pairs(expected: &[(&str, &str)]) -> Vec<(String, String)> {
    expected.iter().map(|(name, kind)| (name.to_string(), kind.to_string())).collect()
}

/// Constructors are listed with methods, in source order, in every
/// language whose constructors were deferred, and in the terminal report.
#[test]
fn class_summaries_list_constructors_with_methods() {
    let scratch = Scratch::new(DEFERRED_FILES);
    let report = json_of(&scratch.stdout(&["--json", "classes"]));
    assert_eq!(members_of(&report, "Counter"), pairs(&[("Counter", "Constructor"), ("tick", "Method")]));
    assert_eq!(members_of(&report, "Service"), pairs(&[("__construct", "Constructor")]));
    assert_eq!(members_of(&report, "C"), pairs(&[("__init__", "Constructor")]));
    assert_eq!(members_of(&report, "S"), pairs(&[("init", "Constructor")]));
    assert_eq!(members_of(&report, "Box"), pairs(&[("area", "Method")]), "fields are not members");

    let terminal = scratch.stdout(&["--plain", "classes", "Shapes.java"]);
    assert!(terminal.contains("Instance Methods (2)"), "{terminal}");
    assert!(terminal.contains("Counter(start: int)"), "{terminal}");
}

/// The field list of a Go struct and a Scala class names the same fields as
/// the standalone Field records that container owns; the list keeps each field's written type and the terminal
/// shows it.
#[test]
fn go_and_scala_field_lists_agree_with_field_records() {
    let scratch = Scratch::new(DEFERRED_FILES);
    let classes = json_of(&scratch.stdout(&["--json", "classes", "shapes.go", "model.scala"]));
    let field_records = json_of(&scratch.stdout(&["--json", "symbols", "--kind", "field", "shapes.go", "model.scala"]));
    let records: Vec<&Value> = field_records["files"]
        .as_array()
        .expect("files")
        .iter()
        .flat_map(|file| file["symbols"].as_array().expect("symbols"))
        .collect();
    let mut compared = Vec::new();
    for file in classes["files"].as_array().expect("files") {
        for container in file["symbols"].as_array().expect("symbols") {
            let name = container["identity"]["name"].as_str().expect("name").to_string();
            let listed: Vec<(String, Value)> = container["kind_data"]["data"]["fields"]
                .as_array()
                .expect("a field list")
                .iter()
                .map(|field| (field["name"].as_str().expect("field name").to_string(), field["type_text"].clone()))
                .collect();
            // Compared as sets: `X, Y int` gives both records one
            // declaration span, so `symbols` orders them by the record-ID
            // tie-breaker, while the field list keeps source order.
            let standalone: BTreeSet<String> = records
                .iter()
                .filter(|record| record["identity"]["container_name"] == container["identity"]["name"])
                .map(|record| record["identity"]["name"].as_str().expect("name").to_string())
                .collect();
            let listed_names: BTreeSet<String> = listed.iter().map(|(field, _)| field.clone()).collect();
            assert_eq!(listed_names, standalone, "`{name}`: field list and Field records");
            compared.push((name, listed));
        }
    }
    let typed = |pairs: &[(&str, Option<&str>)]| -> Vec<(String, Value)> {
        pairs.iter().map(|(name, ty)| (name.to_string(), ty.map_or(Value::Null, Value::from))).collect()
    };
    compared.sort_by(|a, b| a.0.cmp(&b.0));
    assert_eq!(
        compared,
        [
            ("Box".to_string(), typed(&[("width", Some("Int")), ("height", None)])),
            ("Point".to_string(), typed(&[("X", Some("int")), ("Y", Some("int")), ("label", Some("string"))])),
        ]
    );

    let terminal = scratch.stdout(&["--plain", "classes", "shapes.go", "model.scala"]);
    for line in ["- public width: Int", "- public height", "- public X int", "- public Y int", "- package label string"] {
        assert!(terminal.contains(line), "`{line}` in:\n{terminal}");
    }
}

/// Grouped Scala names (`val a, b: Int = 1`, `val e, f: Long`) are each a
/// Field record and an entry of their class's or trait's field list, every
/// entry keeping the shared written type, in JSON and on the terminal.
#[test]
fn scala_grouped_field_names_keep_their_shared_type() {
    let source = "class Pair {\n  val a, b: Int = 1\n  var c, d: String = \"\"\n  val e, f: Long\n}\n\n\
                  trait Shape {\n  val w, h: Double\n  var x, y: Int = 0\n}\n";
    let scratch = Scratch::new(&[("grouped.scala", source)]);
    let classes = json_of(&scratch.stdout(&["--json", "classes", "grouped.scala"]));
    let field_records = json_of(&scratch.stdout(&["--json", "symbols", "--kind", "field", "grouped.scala"]));
    let records: Vec<&Value> = field_records["files"]
        .as_array()
        .expect("files")
        .iter()
        .flat_map(|file| file["symbols"].as_array().expect("symbols"))
        .collect();
    let mut compared = Vec::new();
    for file in classes["files"].as_array().expect("files") {
        for container in file["symbols"].as_array().expect("symbols") {
            let name = container["identity"]["name"].as_str().expect("name").to_string();
            let listed: Vec<(String, Value)> = container["kind_data"]["data"]["fields"]
                .as_array()
                .expect("a field list")
                .iter()
                .map(|field| (field["name"].as_str().expect("field name").to_string(), field["type_text"].clone()))
                .collect();
            // Each grouped name has its own declaration span, so both
            // listings are in source order.
            let standalone: Vec<String> = records
                .iter()
                .filter(|record| record["identity"]["container_name"] == container["identity"]["name"])
                .map(|record| record["identity"]["name"].as_str().expect("name").to_string())
                .collect();
            let listed_names: Vec<String> = listed.iter().map(|(field, _)| field.clone()).collect();
            assert_eq!(listed_names, standalone, "`{name}`: field list and Field records");
            compared.push((name, listed));
        }
    }
    let typed = |pairs: &[(&str, &str)]| -> Vec<(String, Value)> {
        pairs.iter().map(|(name, ty)| (name.to_string(), Value::from(*ty))).collect()
    };
    compared.sort_by(|a, b| a.0.cmp(&b.0));
    assert_eq!(
        compared,
        [
            (
                "Pair".to_string(),
                typed(&[("a", "Int"), ("b", "Int"), ("c", "String"), ("d", "String"), ("e", "Long"), ("f", "Long")])
            ),
            ("Shape".to_string(), typed(&[("w", "Double"), ("h", "Double"), ("x", "Int"), ("y", "Int")])),
        ]
    );

    let terminal = scratch.stdout(&["--plain", "classes", "grouped.scala"]);
    for line in [
        "- public a: Int",
        "- public b: Int",
        "- public d: String",
        "- public f: Long",
        "- public h: Double",
        "- public y: Int",
    ] {
        assert!(terminal.contains(line), "`{line}` in:\n{terminal}");
    }
}

/// Two modules each declare `Boxed` with its own `impl`: every class
/// projection lists each `work` under its own `Boxed`, by container ID.
#[test]
fn same_named_types_in_two_modules_keep_their_own_methods() {
    let scratch = Scratch::new(&[("repeat.rs", REPEATED_OWNERS)]);

    let json = json_of(&scratch.stdout(&["classes", "--file", "repeat.rs", "--in", "*right::Boxed", "--json", "--no-cache"]));
    let file = &json["files"][0];
    let containers = file["symbols"].as_array().expect("symbols");
    assert_eq!(containers.len(), 1, "{json}");
    assert_eq!(containers[0]["identity"]["qualified_name"], "repeat.rs#right::Boxed");
    let members = file["members"][containers[0]["id"].as_str().expect("id")].as_array().expect("members of right::Boxed");
    let names: Vec<&str> = members.iter().map(|member| member["identity"]["qualified_name"].as_str().expect("name")).collect();
    assert_eq!(names, ["repeat.rs#right::Boxed::work"]);
    assert_eq!(members[0]["relations"]["container"]["id"], containers[0]["id"]);

    let lines = scratch.stdout(&["classes", "--file", "repeat.rs", "--jsonl", "--no-cache"]);
    let mut seen = 0;
    for line in lines.lines() {
        let envelope = json_of(line);
        let owner = envelope["record"]["identity"]["qualified_name"].as_str().expect("owner name");
        let members: Vec<&str> = envelope["members"]
            .as_array()
            .expect("members list")
            .iter()
            .map(|member| member["identity"]["qualified_name"].as_str().expect("member name"))
            .collect();
        assert_eq!(members, [format!("{owner}::work")], "{line}");
        seen += 1;
    }
    assert_eq!(seen, 2, "{lines}");

    let terminal = scratch.stdout(&["--plain", "classes", "--file", "repeat.rs", "--no-cache"]);
    assert_eq!(terminal.matches("work(self)").count(), 2, "{terminal}");
    for section in terminal.split("  type Boxed [").skip(1) {
        assert_eq!(section.matches("work(self)").count(), 1, "one method per Boxed:\n{terminal}");
    }
}

/// `impl other::W` beside a sole local `W`: the method belongs to the `W` in
/// `other.rs`, so no class projection lists it under the local `W`.
#[test]
fn an_impl_of_another_files_type_never_joins_a_sole_same_named_local_type() {
    let scratch = Scratch::new(&[("main.rs", EXTERNAL_OWNER), ("other.rs", "pub struct W;\n")]);

    let json = json_of(&scratch.stdout(&["classes", "--file", "main.rs", "--name", "W!", "--json", "--no-cache"]));
    let file = &json["files"][0];
    let containers = file["symbols"].as_array().expect("symbols");
    assert_eq!(containers.len(), 1, "{json}");
    assert_eq!(containers[0]["identity"]["qualified_name"], "main.rs#W");
    let members = file["members"][containers[0]["id"].as_str().expect("id")].as_array().expect("members of W");
    assert!(members.is_empty(), "local `W` lists {members:?}");

    let lines = scratch.stdout(&["classes", "--file", "main.rs", "--name", "W!", "--jsonl", "--no-cache"]);
    let envelopes: Vec<Value> = lines.lines().map(json_of).collect();
    assert_eq!(envelopes.len(), 1, "{lines}");
    assert_eq!(envelopes[0]["record"]["identity"]["qualified_name"], "main.rs#W");
    assert_eq!(envelopes[0]["members"], Value::Array(Vec::new()), "{lines}");

    let terminal = scratch.stdout(&["--plain", "classes", "--file", "main.rs", "--name", "W!", "--no-cache"]);
    assert!(terminal.contains("type W [1:1]"), "{terminal}");
    assert!(!terminal.contains("probe"), "{terminal}");
}

/// A sole local `W` and an `impl` of the `W` declared in `other.rs`.
pub const EXTERNAL_OWNER: &str = "pub struct W;\nmod other;\nimpl other::W { pub fn probe(&self) {} }\n";

pub const REPEATED_OWNERS: &str = "mod left { pub struct Boxed; impl Boxed { pub fn work(&self) {} } }\n\
                                   mod right { pub struct Boxed; impl Boxed { pub fn work(&self) {} } }\n";
