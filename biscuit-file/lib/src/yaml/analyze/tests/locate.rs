//! Tests for the public path → authored-span lookup (`locate_yaml_value`):
//! nested mappings/sequences, comments, quoted and flow values, CRLF,
//! multibyte text, and the conservative `None` boundaries.

use super::super::{YamlPathSegment, locate_yaml_key, locate_yaml_value};

fn key(name: &str) -> YamlPathSegment {
    YamlPathSegment::Key(name.to_string())
}

fn index(i: usize) -> YamlPathSegment {
    YamlPathSegment::Index(i)
}

#[test]
fn test_locate_top_level_mapping_value() {
    let source = "release: 1.20\n";
    let located = locate_yaml_value(source, &[key("release")]).expect("must locate");
    assert_eq!(located.span, 9..13);
    assert_eq!(&source[located.span], "1.20");
    assert_eq!(located.key_span, Some(0..7));
    assert!(located.plain);
}

#[test]
fn test_locate_nested_mapping_value() {
    let source = "style:\n  page:\n    width: 40ch\n";
    let located = locate_yaml_value(source, &[key("style"), key("page"), key("width")])
        .expect("must locate nested");
    assert_eq!(&source[located.span.clone()], "40ch");
    assert_eq!(&source[located.key_span.clone().unwrap()], "width");
    assert!(located.plain);
}

#[test]
fn test_locate_sequence_entry() {
    let source = "tags:\n  - alpha\n  - 42\n";
    let located = locate_yaml_value(source, &[key("tags"), index(1)]).expect("must locate");
    assert_eq!(&source[located.span.clone()], "42");
    assert_eq!(located.key_span, None);
    assert!(located.plain);
}

#[test]
fn test_locate_sequence_of_mappings() {
    let source = "items:\n  - name: a\n  - name: b\n";
    let located =
        locate_yaml_value(source, &[key("items"), index(1), key("name")]).expect("must locate");
    assert_eq!(&source[located.span.clone()], "b");
    assert!(located.plain);
}

#[test]
fn test_locate_value_with_trailing_comment_excludes_comment() {
    let source = "release: 1.20 # pinned\n";
    let located = locate_yaml_value(source, &[key("release")]).expect("must locate");
    assert_eq!(&source[located.span.clone()], "1.20");
}

#[test]
fn test_locate_quoted_value_is_not_plain() {
    let source = "release: \"1.20\"\n";
    let located = locate_yaml_value(source, &[key("release")]).expect("must locate");
    assert_eq!(&source[located.span.clone()], "\"1.20\"");
    assert!(!located.plain);
}

#[test]
fn test_locate_flow_value_is_not_plain() {
    let source = "ports: [80, 443]\n";
    let located = locate_yaml_value(source, &[key("ports")]).expect("must locate");
    assert_eq!(&source[located.span.clone()], "[80, 443]");
    assert!(!located.plain);
}

#[test]
fn test_locate_inside_flow_collection_returns_none() {
    let source = "ports: [80, 443]\n";
    assert!(locate_yaml_value(source, &[key("ports"), index(0)]).is_none());
}

#[test]
fn test_locate_crlf_spans() {
    let source = "a: 1\r\nb: 22\r\n";
    let located = locate_yaml_value(source, &[key("b")]).expect("must locate");
    assert_eq!(&source[located.span.clone()], "22");
    assert_eq!(located.span, 9..11);
}

#[test]
fn test_locate_multibyte_spans() {
    let source = "clé: café\nrelease: 1.20\n";
    let located = locate_yaml_value(source, &[key("release")]).expect("must locate");
    assert_eq!(&source[located.span.clone()], "1.20");
    let accented = locate_yaml_value(source, &[key("clé")]).expect("must locate");
    assert_eq!(&source[accented.span.clone()], "café");
}

#[test]
fn test_locate_unknown_path_returns_none() {
    let source = "release: 1.20\n";
    assert!(locate_yaml_value(source, &[key("missing")]).is_none());
    assert!(locate_yaml_value(source, &[key("release"), key("nested")]).is_none());
    assert!(locate_yaml_value(source, &[index(0)]).is_none());
}

#[test]
fn test_locate_empty_value_returns_none() {
    let source = "parent:\n  child: 1\n";
    // `parent` has no inline value; only leaf values are locatable.
    assert!(locate_yaml_value(source, &[key("parent")]).is_none());
}

#[test]
fn test_locate_block_scalar_header_is_value_text() {
    let source = "notes: |\n  line one\n  line two\n";
    let located = locate_yaml_value(source, &[key("notes")]).expect("header locates");
    assert_eq!(&source[located.span.clone()], "|");
}

#[test]
fn test_locate_deeply_nested_sequence_index() {
    let source = "a:\n  b:\n    - x\n    - y\n";
    let located =
        locate_yaml_value(source, &[key("a"), key("b"), index(1)]).expect("must locate");
    assert_eq!(&source[located.span.clone()], "y");
}

#[test]
fn test_locate_anchored_value_still_locates() {
    let source = "release: &ver 1.20\n";
    let located = locate_yaml_value(source, &[key("release")]).expect("must locate");
    // The authored value text includes the anchor token; callers requiring a
    // single-token plain scalar must reject this shape.
    assert_eq!(&source[located.span.clone()], "&ver 1.20");
    assert!(located.plain);
}

#[test]
fn test_locate_key_top_level() {
    let source = "release: 1.20\n";
    let span = locate_yaml_key(source, &[key("release")]).expect("must locate");
    assert_eq!(&source[span], "release");
}

#[test]
fn test_locate_key_nested() {
    let source = "style:\n  page:\n    width: 40ch\n";
    let span =
        locate_yaml_key(source, &[key("style"), key("page"), key("width")]).expect("must locate");
    assert_eq!(&source[span], "width");
}

#[test]
fn test_locate_key_in_sequence_mapping() {
    let source = "items:\n  - name: a\n  - name: b\n";
    let span =
        locate_yaml_key(source, &[key("items"), index(1), key("name")]).expect("must locate");
    assert_eq!(&source[span], "name");
}

#[test]
fn test_locate_key_unknown_returns_none() {
    let source = "release: 1.20\n";
    assert!(locate_yaml_key(source, &[]).is_none());
    assert!(locate_yaml_key(source, &[key("missing")]).is_none());
    assert!(locate_yaml_key(source, &[key("release"), key("nested")]).is_none());
}
