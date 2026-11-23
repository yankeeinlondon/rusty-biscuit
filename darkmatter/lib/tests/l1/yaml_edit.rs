//! The plain-YAML editor, `darkmatter::yaml_edit::edit_yaml_paths`, through its
//! public API: every result is compared byte for byte, and every refusal by
//! kind, property, and source position.

use darkmatter::yaml_edit::{YamlEdit, YamlEditError, YamlEditErrorKind, edit_yaml_paths};
use serde_json::{Value, json};
use simplified_schema::PropertyPath;

fn path(text: &str) -> PropertyPath {
    PropertyPath::parse(text).expect("a valid path")
}

fn set(target: &str, value: Value) -> YamlEdit {
    YamlEdit::Set {
        path: path(target),
        value,
    }
}

fn remove(target: &str) -> YamlEdit {
    YamlEdit::Remove { path: path(target) }
}

fn edited(source: &str, edits: &[YamlEdit]) -> String {
    edit_yaml_paths(source, edits).unwrap_or_else(|error| panic!("refused: {error:?}"))
}

fn refused(source: &str, edits: &[YamlEdit]) -> YamlEditError {
    match edit_yaml_paths(source, edits) {
        Ok(text) => panic!("expected a refusal, got {text:?}"),
        Err(error) => error,
    }
}

fn assert_refusal(error: &YamlEditError, kind: YamlEditErrorKind, property: Option<&str>, line: usize, column: usize) {
    assert_eq!(error.kind, kind, "{error:?}");
    assert_eq!(error.path.as_ref().map(ToString::to_string).as_deref(), property, "{error:?}");
    assert_eq!((error.location.line, error.location.column), (line, column), "{error:?}");
}

/// The parsed values, as a reader of the file would see them.
fn values(text: &str) -> Value {
    serde_yaml_ng::from_str(text).expect("the text loads")
}

#[test]
fn an_empty_document_gains_its_first_key() {
    assert_eq!(edited("", &[set("name", json!("x"))]), "name: x\n");
}

#[test]
fn a_comment_only_document_keeps_its_comments_above_the_new_key() {
    let source = "# settings for the tool\n\n";
    assert_eq!(
        edited(source, &[set("a.b", json!(1))]),
        "# settings for the tool\n\na:\n    b: 1\n"
    );
}

#[test]
fn a_whitespace_only_document_gains_its_first_key() {
    assert_eq!(edited("\n", &[set("a", json!(true))]), "\na: true\n");
}

#[test]
fn missing_parents_are_created_inside_the_nearest_mapping() {
    let source = "server:\n  port: 80 # http\n";
    assert_eq!(
        edited(source, &[set("server.tls.cert", json!("c.pem"))]),
        "server:\n  port: 80 # http\n  tls:\n    cert: c.pem\n"
    );
}

#[test]
fn a_null_parent_becomes_a_mapping() {
    let source = "# who may do what\npermissions:\nname: x\n";
    assert_eq!(
        edited(source, &[set("permissions.network", json!("deny"))]),
        "# who may do what\npermissions:\n    network: deny\nname: x\n"
    );
}

#[test]
fn a_nested_value_is_replaced_in_place() {
    let source = "server:\n  port: 80 # http\n  host: 'a'\n";
    assert_eq!(
        edited(source, &[set("server.port", json!(8080))]),
        "server:\n  port: 8080 # http\n  host: 'a'\n"
    );
}

#[test]
fn a_nested_value_is_removed_with_its_line() {
    let source = "server:\n  port: 80 # http\n  host: 'a'\n";
    assert_eq!(edited(source, &[remove("server.host")]), "server:\n  port: 80 # http\n");
}

#[test]
fn a_list_item_is_replaced_in_place() {
    let source = "list:\n  - a # first\n  - \"b\"\n";
    assert_eq!(edited(source, &[set("list.1", json!("c"))]), "list:\n  - a # first\n  - c\n");
}

#[test]
fn setting_the_index_past_the_last_item_appends() {
    let source = "list:\n  - a # first\n  - \"b\"\nafter: 1\n";
    assert_eq!(
        edited(source, &[set("list.2", json!("c"))]),
        "list:\n  - a # first\n  - \"b\"\n  - c\nafter: 1\n"
    );
}

#[test]
fn an_appended_mapping_item_is_indented_under_its_dash() {
    let source = "rules:\n- name: a\n";
    assert_eq!(
        edited(source, &[set("rules.1", json!({"name": "b", "on": true}))]),
        "rules:\n- name: a\n- name: b\n  on: true\n"
    );
}

#[test]
fn two_appends_in_one_call_land_in_order() {
    let source = "list:\n  - a\n";
    assert_eq!(
        edited(source, &[set("list.1", json!("b")), set("list.2", json!("c"))]),
        "list:\n  - a\n  - b\n  - c\n"
    );
}

#[test]
fn a_list_is_replaced_as_a_whole() {
    let source = "list:\n  - a # gone with the list\n  - b\nkeep: 1 # kept\n";
    assert_eq!(
        edited(source, &[set("list", json!(["x", "y"]))]),
        "list:\n  - x\n  - y\nkeep: 1 # kept\n"
    );
}

#[test]
fn explicit_empty_collections_are_written_as_values() {
    let source = "list:\n  - a\nmap:\n  k: v\n";
    assert_eq!(
        edited(source, &[set("list", json!([])), set("map", json!({})), set("new", json!([]))]),
        "list: []\nmap: {}\nnew: []\n"
    );
}

#[test]
fn an_empty_flow_sequence_becomes_a_block_on_its_first_append() {
    let source = "allow: [] # none yet\n";
    assert_eq!(edited(source, &[set("allow.0", json!("ls"))]), "allow: # none yet\n    - ls\n");
}

#[test]
fn an_empty_flow_mapping_becomes_a_block_on_its_first_key() {
    assert_eq!(edited("opts: {}\n", &[set("opts.a", json!(1))]), "opts:\n    a: 1\n");
}

const COMMENTED: &str = "\
# Host configuration
name: 'tool'   # single quotes stay
zeta: \"z\"
alpha: 1

permissions:
  # shell commands the user approved
  shell-commands:
    - prefix ls   # listing
    - exact ps -e
  network: deny
trailer: |
  kept
  as is
";

#[test]
fn comments_quotes_and_order_outside_the_edit_are_kept_byte_for_byte() {
    let text = edited(COMMENTED, &[set("permissions.shell-commands.2", json!("exact pwd"))]);
    assert_eq!(
        text,
        COMMENTED.replace("    - exact ps -e\n", "    - exact ps -e\n    - exact pwd\n")
    );
}

#[test]
fn a_byte_order_mark_stays_first() {
    assert_eq!(edited("\u{feff}a: 1\n", &[set("b", json!(2))]), "\u{feff}a: 1\nb: 2\n");
}

#[test]
fn crlf_line_endings_are_used_for_new_lines() {
    let source = "a: 1\r\nlist:\r\n  - x\r\n";
    assert_eq!(
        edited(source, &[set("list.1", json!("y")), set("c", json!(3))]),
        "a: 1\r\nlist:\r\n  - x\r\n  - y\r\nc: 3\r\n"
    );
}

#[test]
fn a_missing_final_line_break_stays_missing() {
    assert_eq!(edited("a: 1\nb: 2", &[set("c", json!(3))]), "a: 1\nb: 2\nc: 3");
    assert_eq!(edited("a: 1\nb: 2", &[set("b", json!(5))]), "a: 1\nb: 5");
    assert_eq!(edited("a: 1\r\nb: 2", &[set("c", json!(3))]), "a: 1\r\nb: 2\r\nc: 3");
    assert_eq!(edited("# only a comment", &[set("a", json!(1))]), "# only a comment\na: 1");
}

#[test]
fn a_leading_document_marker_is_kept() {
    assert_eq!(edited("---\na: 1\n", &[set("b", json!(2))]), "---\na: 1\nb: 2\n");
}

#[test]
fn sets_and_removals_apply_together_in_one_call() {
    let source = "a: 1 # one\nb:\n  - x\n  - y\n  - z\nc: 3\n";
    let edits = [set("a", json!(10)), remove("b.0"), remove("b.2"), set("d.e", json!("f"))];
    assert_eq!(edited(source, &edits), "a: 10 # one\nb:\n  - y\nc: 3\nd:\n    e: f\n");
}

#[test]
fn removing_the_last_child_leaves_an_empty_collection() {
    assert_eq!(edited("a:\n  b: 1\nc: 2\n", &[remove("a.b")]), "a: {}\nc: 2\n");
    assert_eq!(edited("l:\n  - x\nc: 2\n", &[remove("l.0")]), "l: []\nc: 2\n");
}

#[test]
fn removing_the_last_top_level_key_leaves_the_comments_and_an_empty_root() {
    assert_eq!(edited("# about\na: 1\n", &[remove("a")]), "# about\n{}\n");
    assert_eq!(edited("a: 1", &[remove("a")]), "{}");
    assert_eq!(edited("a: 1\r\n", &[remove("a")]), "{}\r\n");
}

#[test]
fn an_empty_flow_root_becomes_a_block_on_its_first_key() {
    assert_eq!(edited("{}\n", &[set("a.b", json!(1))]), "a:\n    b: 1\n");
    assert_eq!(edited("# keep\n{ }  # defaults\n", &[set("a", json!(1))]), "# keep\n# defaults\na: 1\n");
    assert_eq!(edited("{}", &[set("a", json!(1))]), "a: 1");
}

#[test]
fn an_edit_through_an_alias_is_refused_at_the_alias() {
    let source = "base: &b\n  x: 1\nother: *b\n";
    let error = refused(source, &[set("other.x", json!(2))]);
    assert_refusal(&error, YamlEditErrorKind::Alias, Some("other.x"), 3, 1);
}

#[test]
fn an_edit_inside_an_anchored_node_is_refused_at_the_anchor() {
    let source = "keep: 0\nbase: &b\n  x: 1\nother: *b\n";
    let error = refused(source, &[set("base.x", json!(2))]);
    assert_refusal(&error, YamlEditErrorKind::Anchor, Some("base.x"), 2, 1);
}

#[test]
fn an_edit_of_a_tagged_node_is_refused_at_the_tag() {
    let source = "a: 1\nn: !!str 5\n";
    let error = refused(source, &[set("n", json!("6"))]);
    assert_refusal(&error, YamlEditErrorKind::Tag, Some("n"), 2, 1);
}

#[test]
fn an_edit_inside_a_flow_mapping_is_refused() {
    let source = "name: x\nopts: {a: 1}\n";
    let error = refused(source, &[set("opts.b", json!(2))]);
    assert_refusal(&error, YamlEditErrorKind::FlowCollection, Some("opts.b"), 2, 1);
}

#[test]
fn an_edit_inside_a_flow_sequence_is_refused() {
    let source = "name: x\nl: [a, b]\n";
    let error = refused(source, &[set("l.2", json!("c"))]);
    assert_refusal(&error, YamlEditErrorKind::FlowCollection, Some("l.2"), 2, 1);
    let error = refused(source, &[remove("l.0")]);
    assert_refusal(&error, YamlEditErrorKind::FlowCollection, Some("l.0"), 2, 1);
}

#[test]
fn a_flow_collection_may_still_be_replaced_whole() {
    assert_eq!(edited("l: [a, b] # c\n", &[set("l", json!("x"))]), "l: x # c\n");
}

#[test]
fn a_flow_root_is_refused() {
    let error = refused("# c\n{a: 1}\n", &[set("b", json!(2))]);
    assert_refusal(&error, YamlEditErrorKind::FlowCollection, None, 2, 1);
}

#[test]
fn malformed_yaml_is_refused_where_the_parser_stopped() {
    let error = refused("a: 1\nb: [1, 2\nc: 3\n", &[set("a", json!(2))]);
    assert_eq!(error.kind, YamlEditErrorKind::Parse, "{error:?}");
    assert_eq!(error.path, None);
    // The parser's own position: the line where the unclosed `[` is noticed.
    assert_eq!((error.location.line, error.location.column), (3, 2), "{error:?}");
    assert!(error.to_string().starts_with("line 3, column 2: the YAML could not be parsed"));
}

#[test]
fn a_location_counts_the_byte_order_mark_in_its_offset() {
    let error = refused("\u{feff}a: 1\nl: [x]\n", &[set("l.1", json!("y"))]);
    assert_refusal(&error, YamlEditErrorKind::FlowCollection, Some("l.1"), 2, 1);
    assert_eq!(error.location.offset, "\u{feff}a: 1\n".len());
}

/// R4-F2.7: a column counts characters, so `é` and `🧪` before the problem
/// each move it by one; the offset stays a byte offset.
#[test]
fn a_location_column_counts_characters() {
    let source = "a: 1\ncafé🧪: x: y\n";
    let error = refused(source, &[set("a", json!(2))]);
    assert_eq!(error.kind, YamlEditErrorKind::Parse, "{error:?}");
    assert_eq!((error.location.line, error.location.column), (2, 9), "{error:?}");
    assert_eq!(error.location.offset, source.find(": y").unwrap(), "{error:?}");
}

#[test]
fn a_root_that_is_not_a_mapping_is_refused() {
    let error = refused("# list\n- a\n", &[set("b", json!(2))]);
    assert_refusal(&error, YamlEditErrorKind::NotAMapping, None, 2, 1);
}

#[test]
fn an_index_beyond_the_end_is_refused_at_the_list() {
    let error = refused("x: 0\nl:\n  - a\n", &[set("l.5", json!("b"))]);
    assert_refusal(&error, YamlEditErrorKind::InvalidPath, Some("l.5"), 2, 1);
}

#[test]
fn removing_a_missing_value_is_refused() {
    let error = refused("a: 1\n", &[remove("b")]);
    assert_eq!(error.kind, YamlEditErrorKind::InvalidPath, "{error:?}");
    assert_eq!(error.path, Some(path("b")));
}

#[test]
fn repeated_read_write_read_round_trips_keep_every_other_byte() {
    let first = edited(COMMENTED, &[set("permissions.file-read", json!(["./*.yaml"]))]);
    let mut expected = values(COMMENTED);
    expected["permissions"]["file-read"] = json!(["./*.yaml"]);
    assert_eq!(values(&first), expected);
    assert_eq!(first, COMMENTED.replace("  network: deny\n", "  network: deny\n  file-read:\n    - ./*.yaml\n"));

    let second = edited(&first, &[set("permissions.file-read.1", json!("~/notes/*.md"))]);
    expected["permissions"]["file-read"] = json!(["./*.yaml", "~/notes/*.md"]);
    assert_eq!(values(&second), expected);

    let third = edited(&second, &[remove("permissions.file-read")]);
    assert_eq!(third, COMMENTED, "removing the added list restores the original bytes");
    assert_eq!(values(&third), values(COMMENTED));
}

#[test]
fn an_indented_root_is_refused_rather_than_rewritten() {
    let error = refused("  a: 1\n  b: 2\n", &[set("c", json!(3))]);
    assert_refusal(&error, YamlEditErrorKind::UnsupportedForm, Some("c"), 1, 3);
}

mod indentation_of_new_values {
    //! Where a new collection's indentation comes from: the touched block,
    //! then the nearest authored block of its kind, then the caller's
    //! preference.

    use super::{edited, path, set};
    use biscuit_file::Indentation;
    use darkmatter::yaml_edit::{YamlEdit, edit_yaml_paths_with_indentation};
    use serde_json::{Value, json};
    use std::num::NonZeroU8;

    fn spaces(width: u8) -> Indentation {
        Indentation::spaces(NonZeroU8::new(width).unwrap())
    }

    fn edited_with(source: &str, edits: &[YamlEdit], fallback: u8) -> String {
        edit_yaml_paths_with_indentation(source, edits, spaces(fallback)).unwrap_or_else(|error| panic!("refused: {error:?}"))
    }

    fn values(text: &str) -> Value {
        biscuit_file::serde_yaml_ng::from_str(text).unwrap()
    }

    #[test]
    fn a_file_with_no_blocks_takes_the_preference_for_every_level() {
        let text = edited_with("name: x\n", &[set("permissions.file-read", json!(["./*.txt", "./*.md"]))], 2);
        assert_eq!(text, "name: x\npermissions:\n  file-read:\n    - ./*.txt\n    - ./*.md\n");
        assert_eq!(values(&text)["permissions"]["file-read"], json!(["./*.txt", "./*.md"]));
        let text = edited("name: x\n", &[set("permissions.file-read", json!(["./*.txt"]))]);
        assert_eq!(text, "name: x\npermissions:\n    file-read:\n        - ./*.txt\n", "the default is four spaces");
    }

    #[test]
    fn a_new_block_copies_the_nearest_authored_block_of_its_kind_over_the_preference() {
        let source = "a:\n  x:\n    y: 1\nb:\n      p:\n            q: 1\n";
        let text = edited_with(source, &[set("b.r.s", json!(1))], 4);
        assert_eq!(text, format!("{source}      r:\n            s: 1\n"), "the sibling inside `b` wins");
        let text = edited_with(source, &[set("a.z.w", json!(1))], 4);
        assert_eq!(text, "a:\n  x:\n    y: 1\n  z:\n    w: 1\nb:\n      p:\n            q: 1\n");
        let text = edited_with("l:\n   - a\n", &[set("m.k", json!(1)), set("n", json!(["v"]))], 2);
        assert_eq!(text, "l:\n   - a\nm:\n  k: 1\nn:\n   - v\n", "a mapping has no sequence example; a sequence has one");
    }

    #[test]
    fn a_replaced_block_keeps_its_offset_and_an_indentless_list_stays_indentless() {
        let source = "list:\n- a\nmap:\n      k: v\n";
        let text = edited_with(source, &[set("list.1", json!("b")), set("map.n", json!({"x": 1}))], 2);
        assert_eq!(text, "list:\n- a\n- b\nmap:\n      k: v\n      n:\n            x: 1\n");
        let text = edited_with(source, &[set("list", json!(["c", "d"]))], 2);
        assert_eq!(text, "list:\n- c\n- d\nmap:\n      k: v\n");
        let text = edited_with("only:\n- a\n", &[set("more", json!(["b"]))], 3);
        assert_eq!(text, "only:\n- a\nmore:\n   - b\n", "an indentless list is not an example for a new one");
    }

    #[test]
    fn multiline_scalar_content_and_flow_collections_are_not_examples() {
        let source = "note: |\n        deep text\nflow: {a: [1]}\nitem:\n- name: x\n  age: 1\n";
        let text = edited_with(source, &[set("new.k", json!(1))], 3);
        assert_eq!(text, format!("{source}new:\n   k: 1\n"));
    }

    #[test]
    fn crlf_lines_and_an_empty_flow_collection_take_the_layout() {
        let source = "x:\r\n   y: 1\r\nopts: {}\r\n";
        let text = edited_with(source, &[set("opts.a.b", json!(1))], 2);
        assert_eq!(text, "x:\r\n   y: 1\r\nopts:\r\n   a:\r\n      b: 1\r\n");
    }

    #[test]
    fn a_multiline_value_is_written_at_the_chosen_level() {
        let text = edited_with("a:\n   b: 1\n", &[set("a.c", json!("one\ntwo\n"))], 2);
        assert_eq!(text, "a:\n   b: 1\n   c: |\n      one\n      two\n");
        assert_eq!(values(&text)["a"]["c"], json!("one\ntwo\n"));
    }

    #[test]
    fn edits_that_already_hold_return_the_source_bytes() {
        let source = "# kept\na: 'x'   # quoted\nl:\n- 1\n";
        assert_eq!(edited(source, &[set("a", json!("x")), set("l", json!([1]))]), source);
        assert_eq!(edited(source, &[]), source);
        assert_eq!(path("a").to_string(), "a");
    }
}
