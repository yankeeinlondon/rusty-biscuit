//! `hug classes` on records: which containers it summarizes, its static and
//! instance sections, its JSON `members`, and the field metadata it keeps
//! without selecting Field, Property or Constant records.

use serde_json::Value;

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
