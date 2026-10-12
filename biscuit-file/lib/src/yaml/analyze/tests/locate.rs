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
fn test_locate_anchored_value_reports_anchor() {
    let source = "release: &ver 1.20\n";
    let located = locate_yaml_value(source, &[key("release")]).expect("must locate");
    // The span still covers the property prefix.
    assert_eq!(&source[located.span.clone()], "&ver 1.20");
    assert!(located.plain);
    assert_eq!(located.properties.anchor.clone().map(|span| &source[span]), Some("&ver"));
    assert_eq!(located.properties.tag, None);
    assert_eq!(located.properties.alias, None);
    assert!(!located.properties.is_empty());
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

#[test]
fn test_locate_plain_value_has_no_properties() {
    let source = "release: 1.20\n";
    let located = locate_yaml_value(source, &[key("release")]).expect("must locate");
    assert!(located.properties.is_empty());
}

#[test]
fn test_locate_tagged_value_reports_tag() {
    let source = "release: !!str 1.20\n";
    let located = locate_yaml_value(source, &[key("release")]).expect("must locate");
    assert_eq!(&source[located.span.clone()], "!!str 1.20");
    assert_eq!(located.properties.tag.clone().map(|span| &source[span]), Some("!!str"));
    assert_eq!(located.properties.anchor, None);
    assert_eq!(located.properties.alias, None);
}

#[test]
fn test_locate_anchor_and_tag_in_either_order() {
    for source in ["release: &ver !!str 1.20\n", "release: !!str &ver 1.20\n"] {
        let located = locate_yaml_value(source, &[key("release")]).expect("must locate");
        assert_eq!(located.properties.anchor.clone().map(|span| &source[span]), Some("&ver"));
        assert_eq!(located.properties.tag.clone().map(|span| &source[span]), Some("!!str"));
        assert_eq!(located.properties.alias, None);
    }
}

#[test]
fn test_locate_alias_value_reports_alias() {
    let source = "base: &b 1\nrelease: *b\n";
    let located = locate_yaml_value(source, &[key("release")]).expect("must locate");
    assert_eq!(&source[located.span.clone()], "*b");
    assert_eq!(located.properties.alias.clone().map(|span| &source[span]), Some("*b"));
    assert_eq!(located.properties.anchor, None);
    assert_eq!(located.properties.tag, None);
}

#[test]
fn test_locate_alias_sequence_entry_reports_alias() {
    let source = "base: &b 1\nlist:\n  - *b\n";
    let located = locate_yaml_value(source, &[key("list"), index(0)]).expect("must locate");
    assert_eq!(located.properties.alias.clone().map(|span| &source[span]), Some("*b"));
}

#[test]
fn test_locate_interior_indicator_bytes_are_not_properties() {
    let source = "expr: 2 * 3 & !x\nquoted: \"&not-an-anchor\"\n";
    let expr = locate_yaml_value(source, &[key("expr")]).expect("must locate");
    assert!(expr.properties.is_empty());
    let quoted = locate_yaml_value(source, &[key("quoted")]).expect("must locate");
    assert!(quoted.properties.is_empty());
}

#[test]
fn test_locate_multi_line_plain_mapping_value_returns_none() {
    let source = "a: one\n  two\nb: 2\n";
    assert!(locate_yaml_value(source, &[key("a")]).is_none());
    // The single-line sibling is unaffected.
    let located = locate_yaml_value(source, &[key("b")]).expect("must locate");
    assert_eq!(&source[located.span], "2");
}

#[test]
fn test_locate_multi_line_plain_value_across_blank_line_returns_none() {
    let source = "a: one\n\n  two\n";
    assert!(locate_yaml_value(source, &[key("a")]).is_none());
}

#[test]
fn test_locate_multi_line_plain_sequence_entry_returns_none() {
    let source = "p:\n  - V(3mo,\n    x)\n  - single\n";
    assert!(locate_yaml_value(source, &[key("p"), index(0)]).is_none());
    let located = locate_yaml_value(source, &[key("p"), index(1)]).expect("must locate");
    assert_eq!(&source[located.span], "single");
}

#[test]
fn test_locate_multi_line_double_quoted_value_returns_none() {
    let source = "a: \"one\n  two\"\n";
    assert!(locate_yaml_value(source, &[key("a")]).is_none());
}

#[test]
fn test_locate_multi_line_single_quoted_value_returns_none() {
    let source = "a: 'one\n  two'\n";
    assert!(locate_yaml_value(source, &[key("a")]).is_none());
}

#[test]
fn test_locate_multi_line_flow_value_returns_none() {
    let source = "ports: [80,\n  443]\n";
    assert!(locate_yaml_value(source, &[key("ports")]).is_none());
}

#[test]
fn test_locate_single_line_quoted_value_followed_by_sibling_still_locates() {
    let source = "a: \"one\"\nb: 'two'\n";
    let first = locate_yaml_value(source, &[key("a")]).expect("must locate");
    assert_eq!(&source[first.span], "\"one\"");
    let second = locate_yaml_value(source, &[key("b")]).expect("must locate");
    assert_eq!(&source[second.span], "'two'");
}

#[test]
fn test_locate_sequence_mapping_value_followed_by_sibling_key_still_locates() {
    // `action` sits deeper than the dash but at the key column of `rule`:
    // a sibling key, not a continuation of `x`.
    let source = "items:\n  - rule: x\n    action: a\n";
    let located =
        locate_yaml_value(source, &[key("items"), index(0), key("rule")]).expect("must locate");
    assert_eq!(&source[located.span], "x");
}

#[test]
fn test_locate_value_followed_by_comment_line_still_locates() {
    let source = "a: one\n  # note\nb: 2\n";
    let located = locate_yaml_value(source, &[key("a")]).expect("must locate");
    assert_eq!(&source[located.span], "one");
}

#[test]
fn test_locate_zero_indent_sequence_entry_under_key() {
    let source = "content_policy:\n- V(3mo)\n- V(1y)\nother: 1\n";
    let located =
        locate_yaml_value(source, &[key("content_policy"), index(1)]).expect("must locate");
    assert_eq!(&source[located.span], "V(1y)");
    assert!(locate_yaml_value(source, &[index(0)]).is_none());
    let other = locate_yaml_value(source, &[key("other")]).expect("following key at root");
    assert_eq!(&source[other.span], "1");
}

#[test]
fn test_locate_zero_indent_sequence_of_mappings_under_key() {
    let source = "p:\n- rule: V(3mo)\n  action: a\n";
    let rule = locate_yaml_value(source, &[key("p"), index(0), key("rule")]).expect("rule");
    assert_eq!(&source[rule.span], "V(3mo)");
    let action = locate_yaml_value(source, &[key("p"), index(0), key("action")]).expect("action");
    assert_eq!(&source[action.span], "a");
    let key_span = locate_yaml_key(source, &[key("p"), index(0), key("action")]).expect("key");
    assert_eq!(&source[key_span], "action");
}
