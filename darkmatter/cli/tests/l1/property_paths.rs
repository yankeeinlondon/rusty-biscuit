//! Property paths across commands: what one command writes another reads, and
//! the commands agree on how a value and a path are read.
//!
//! Each command's own behavior is in `get_set_rm` and `compose_setters`. These
//! tests cover what only shows when commands are run together through the real
//! `md`: exit status, persistence, and failure with no mutation.

use crate::common;

use common::CliProcessFixture;
use serde_json::{Value, json};

const NESTED: &str = "---\n# about the file\nauthor:\n  # about the name\n  name: Ann # kept\n  email: a@x\nitems:\n  - name: a\n  - name: b\nnothing: null\n---\n# Body\n\n  keeps   its spacing\n";

fn with_document(name: &str, document: &str) -> CliProcessFixture {
    let fixture = CliProcessFixture::named(name);
    fixture.write_file(fixture.cwd().join("doc.md"), document);
    fixture
}

fn read_document(fixture: &CliProcessFixture) -> String {
    std::fs::read_to_string(fixture.cwd().join("doc.md")).expect("the document")
}

fn run(fixture: &CliProcessFixture, args: &[&str]) -> (Option<i32>, String, String) {
    let output = fixture.command().args(args).output().expect("md runs");
    (output.status.code(), String::from_utf8_lossy(&output.stdout).into_owned(), String::from_utf8_lossy(&output.stderr).into_owned())
}

fn succeed(fixture: &CliProcessFixture, args: &[&str]) -> String {
    let (code, stdout, stderr) = run(fixture, args);
    assert_eq!(code, Some(0), "{args:?}\n{stdout}\n{stderr}");
    stdout
}

fn get(fixture: &CliProcessFixture, path: &str) -> Value {
    serde_json::from_str(&succeed(fixture, &["get", "doc.md", path, "--output", "json"])).unwrap()
}

/// `md get`'s JSON value at `path` in a document given on stdin.
fn get_from_text(fixture: &CliProcessFixture, document: &str, path: &str) -> Value {
    let output = fixture.command().args(["get", "-", path, "--output", "json"]).write_stdin(document).output().expect("md runs");
    assert!(output.status.success(), "{output:?}");
    serde_json::from_slice(&output.stdout).unwrap()
}

/// A mutation that must exit with status 1 and leave the file's bytes alone.
fn refused(fixture: &CliProcessFixture, args: &[&str]) -> String {
    let before = read_document(fixture);
    let (code, stdout, stderr) = run(fixture, args);
    assert_eq!(code, Some(1), "{args:?}\n{stdout}\n{stderr}");
    assert_eq!(read_document(fixture), before, "{args:?} must write nothing");
    stderr
}

/// What `md set` writes and what `md compose` overrides, read back the same way.
fn set_and_compose_values(fixture: &CliProcessFixture, path: &str, value: &str) -> (Value, Value) {
    let edited = succeed(fixture, &["set", "doc.md", path, value, "--output", "markdown"]);
    let composed = succeed(fixture, &["compose", "doc.md", &format!("{path}={value}"), "--frontmatter"]);
    (get_from_text(fixture, &edited, path), get_from_text(fixture, &composed, path))
}

/// One test per row, because each runs four processes.
macro_rules! both_commands_read {
    ($($name:ident: $schema:literal, $path:literal, $value:literal => $expected:expr;)+) => {
        $(
            #[test]
            fn $name() {
                let fixture = with_document(stringify!($name), concat!("---\n", $schema, "title: T\n---\nBody\n"));
                let (set, compose) = set_and_compose_values(&fixture, $path, $value);
                assert_eq!(set, $expected, "md set");
                assert_eq!(compose, $expected, "md compose");
            }
        )+
    };
}

both_commands_read! {
    a_json5_object_is_an_object: "", "x", "{a:1}" => json!({"a": 1});
    a_yaml_flow_object_is_an_object: "", "x", "{ max-width: 50 }" => json!({"max-width": 50});
    an_undeclared_number_is_a_number: "", "count", "5" => json!(5);
    a_declared_number_is_a_number: "$schema:\n  count: number\n", "count", "5" => json!(5);
    a_declared_string_keeps_its_spelling: "$schema:\n  version: string\n", "version", "1.10" => json!("1.10");
    a_quoted_string_decodes_once: "", "x", "\"5\"" => json!("5");
    an_empty_value_is_the_empty_string: "$schema:\n  x: string\n", "x", "" => json!("");
    a_rejected_syntax_stays_text: "", "x", "{unclosed" => json!("{unclosed");
    a_nested_value_reads_through_its_type: "$schema:\n  a:\n    v: string\n", "a.v", "1.10" => json!("1.10");
}

#[test]
fn an_edit_survives_every_later_command_and_what_was_not_named_survives_the_edits() {
    let fixture = with_document("paths-cycle", NESTED);

    succeed(&fixture, &["set", "doc.md", "author.name", "Alice", "--save"]);
    assert_eq!(get(&fixture, "author.name"), json!("Alice"));
    assert_eq!(get(&fixture, "author.email"), json!("a@x"));

    succeed(&fixture, &["set", "doc.md", "items.1.name", "z", "--save"]);
    succeed(&fixture, &["set", "doc.md", r"dotted\.key", "literal", "--save"]);
    succeed(&fixture, &["set", "doc.md", "created.deep.value", "{a:1}", "--save"]);
    assert_eq!(get(&fixture, "items"), json!([{"name": "a"}, {"name": "z"}]));
    assert_eq!(get(&fixture, r"dotted\.key"), json!("literal"));
    assert_eq!(get(&fixture, "dotted.key"), json!(""), "a dot always separates");
    assert_eq!(get(&fixture, "created.deep.value.a"), json!(1));

    let composed = succeed(&fixture, &["compose", "doc.md", "author.email=b@y", "--frontmatter"]);
    assert_eq!(get_from_text(&fixture, &composed, "author"), json!({"name": "Alice", "email": "b@y"}));
    assert_eq!(get(&fixture, "author.email"), json!("a@x"), "composing changes nothing on disk");

    succeed(&fixture, &["rm", "doc.md", "created.deep.value", r"dotted\.key", "items.0"]);
    assert_eq!(get(&fixture, "created"), json!({"deep": {}}), "the parent stays");
    assert_eq!(get(&fixture, "items"), json!([{"name": "z"}]));

    let text = read_document(&fixture);
    for kept in ["# about the file\n", "  # about the name\n", "name: Alice # kept\n", "# Body\n\n  keeps   its spacing\n", "nothing: null\n"] {
        assert!(text.contains(kept), "`{kept}` in:\n{text}");
    }
}

#[test]
fn whitespace_and_unicode_in_a_key_are_kept_by_every_command() {
    let fixture = with_document("paths-spelling", "---\ntitle: T\n---\nBody\n");
    for path in ["é t.clé", " padded .  inner  ", r"C:\\.name", r"C:\dir"] {
        succeed(&fixture, &["set", "doc.md", path, "v", "--save"]);
        assert_eq!(get(&fixture, path), json!("v"), "{path}");
    }
    assert_eq!(get(&fixture, "é t"), json!({"clé": "v"}));
    assert_eq!(get(&fixture, " padded "), json!({"  inner  ": "v"}), "nothing is trimmed");
    assert_eq!(get(&fixture, "padded"), json!(""));
    assert_eq!(get(&fixture, r"C:\\"), json!({"name": "v"}), "the key ending in a backslash");

    succeed(&fixture, &["rm", "doc.md", "é t.clé", " padded .  inner  ", r"C:\\.name", r"C:\dir"]);
    assert_eq!(get(&fixture, "é t"), json!({}));
    assert_eq!(get(&fixture, r"C:\dir"), json!(""));
}

/// The spec's out-of-range table: `get` calls it missing, and a mutation names the
/// array's length and changes nothing. The spellings that are not indexes behave
/// the same way, and a huge number neither wraps nor panics.
#[test]
fn an_index_that_does_not_exist_is_the_same_case_for_every_command() {
    let fixture = with_document("paths-range", NESTED);
    for path in ["items.2", "items.01", "items.-1", "items.-", "items.1234567890123456789012345678901234567890"] {
        assert_eq!(get(&fixture, path), json!(""), "{path}");
        let stderr = refused(&fixture, &["set", "doc.md", path, "x", "--save"]);
        assert!(stderr.contains("2 element"), "set {path}: {stderr}");
        refused(&fixture, &["rm", "doc.md", path]);
        let setter = format!("{path}=x");
        // `items.-1` is not a setter's key, so it reads as a file that does not exist.
        let stderr = refused(&fixture, &["compose", "doc.md", &setter]);
        if path.ends_with("-1") || path.ends_with('-') {
            assert!(stderr.contains(&setter) || stderr.contains("path"), "compose {path}: {stderr}");
        } else {
            assert!(stderr.contains("2 element"), "compose {path}: {stderr}");
        }
    }
}

#[test]
fn a_null_or_scalar_parent_refuses_every_mutation_and_can_be_replaced_by_name() {
    let fixture = with_document("paths-parents", NESTED);
    for (parent, child, kind) in [("nothing", "nothing.deep", "null"), ("author.name", "author.name.deep", "text")] {
        assert_eq!(get(&fixture, child), json!(""), "{child}");
        let stderr = refused(&fixture, &["set", "doc.md", child, "1", "--save"]);
        assert!(stderr.contains(parent) && stderr.contains(kind), "{stderr}");
        refused(&fixture, &["rm", "doc.md", child]);
        let stderr = refused(&fixture, &["compose", "doc.md", &format!("{child}=1")]);
        assert!(stderr.contains(parent) && stderr.contains(kind), "{stderr}");
    }
    succeed(&fixture, &["set", "doc.md", "nothing", "{deep:1}", "--save"]);
    assert_eq!(get(&fixture, "nothing.deep"), json!(1));
}

/// The value being walked decides what `0` means, so both spellings reach the
/// declared `count` and only its type rejects `abc`. The report names the place as
/// it was written, and a command-line value has no authored line to claim.
#[test]
fn a_rejected_value_names_its_place_whether_a_number_was_an_index_or_a_key() {
    let fixture = with_document("paths-diagnostics", "---\n$schema: ./types.yaml\nitems:\n  - count: 1\nbyid:\n  \"0\":\n    count: 1\n---\nBody\n");
    fixture.write_file(
        fixture.cwd().join("types.yaml"),
        "kind: schema\ntypes:\n  item:\n    count: number\n$schema:\n  items: item[]\n  byid:\n    \"<string>\":\n      count: number\n",
    );
    let failure = |args: &[&str]| -> Value {
        let before = read_document(&fixture);
        let (code, stdout, stderr) = run(&fixture, args);
        assert_eq!(code, Some(1), "{args:?}\n{stdout}\n{stderr}");
        assert_eq!(read_document(&fixture), before, "{args:?} must write nothing");
        serde_json::from_str(&stdout).unwrap()
    };
    for path in ["items.0.count", "byid.0.count"] {
        let set = failure(&["set", "doc.md", path, "abc", "--save", "--output", "json"]);
        assert_eq!(set["property"], path);
        let setter = format!("{path}=abc");
        let compose = failure(&["compose", "doc.md", &setter, "--output", "json"]);
        let message = compose["diagnostics"][0]["message"].as_str().unwrap();
        assert!(message.contains(&setter) && message.contains(&format!("`{path}`")), "{message}");
        for report in [&set, &compose] {
            let diagnostic = &report["diagnostics"][0];
            assert_eq!(diagnostic["code"], "dm.schema.type_mismatch", "{report:#}");
            assert_eq!(diagnostic["source"]["line"], Value::Null, "{report:#}");
        }
        succeed(&fixture, &["set", "doc.md", path, "5", "--save"]);
        assert_eq!(get(&fixture, path), json!(5), "the control: the type accepts a number at this place");
    }
}

#[test]
fn a_nested_value_prints_in_every_output_format() {
    let fixture = with_document("paths-formats", NESTED);
    for (format, expected) in [("json", "\"Ann\"\n"), ("yaml", "Ann\n"), ("plain", "Ann\n")] {
        assert_eq!(succeed(&fixture, &["get", "doc.md", "author.name", "--output", format]), expected, "{format}");
    }
    let table = succeed(&fixture, &["get", "doc.md", "author", "--output", "toml"]);
    assert!(table.contains("name = \"Ann\"") && table.contains("email = \"a@x\""), "{table}");
}

/// One grammar: a CLI handler that splits a path on `.` would read `a\.b` wrongly
/// while every other test here kept passing for ordinary keys.
#[test]
fn only_property_path_splits_a_path_in_the_commands() {
    for (file, source) in [
        ("commands/frontmatter.rs", include_str!("../../src/commands/frontmatter.rs")),
        ("args/path_completion.rs", include_str!("../../src/args/path_completion.rs")),
        ("commands/compose.rs", include_str!("../../src/commands/compose.rs")),
    ] {
        for needle in ["split('.')", "split(\".\")", "split_once('.')", "rsplit('.')", "rsplit_once('.')"] {
            assert!(!source.contains(needle), "{file} parses a path with `{needle}`");
        }
    }
}
