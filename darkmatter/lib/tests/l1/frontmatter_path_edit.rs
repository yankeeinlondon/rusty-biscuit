//! The nested frontmatter text editor, through its public functions, with the
//! expected values coming from the same pure edits the commands make.

use darkmatter::markdown::hash::{
    PathEdit, PathEditRefusal, edit_frontmatter_paths, edited_document_text,
};
use darkmatter::markdown::{FrontmatterMap, Markdown};
use serde_json::{Value, json};
use simplified_schema::{PropertyPath, RemovalPlan, set_at};

fn path(text: &str) -> PropertyPath {
    PropertyPath::parse(text).expect("a valid path")
}

fn values(source: &str) -> FrontmatterMap {
    Markdown::try_from_content(source)
        .expect("the source loads")
        .frontmatter()
        .as_map()
        .clone()
}

/// `md set`'s edit of `source`.
fn set(source: &str, target: &str, value: Value) -> Result<String, PathEditRefusal> {
    let target = path(target);
    let mut expected = values(source);
    set_at(&mut expected, &target, value).expect("the pure edit applies");
    let steps = target.resolve_map(&expected).steps().to_vec();
    edit_frontmatter_paths(source, &expected, &[PathEdit::Set(steps)])
}

/// `md rm`'s edit of `source`.
fn rm(source: &str, targets: &[&str]) -> Result<String, PathEditRefusal> {
    let targets: Vec<PropertyPath> = targets.iter().map(|target| path(target)).collect();
    let original = values(source);
    let plan = RemovalPlan::new(&original, &targets).expect("the plan resolves");
    let mut expected = original.clone();
    plan.apply(&mut expected);
    let edits: Vec<PathEdit> = plan.targets().map(PathEdit::Remove).collect();
    edit_frontmatter_paths(source, &expected, &edits)
}

/// The document as the commands hold it after a pure `set`.
fn edited_by_set(source: &str, target: &str, value: Value) -> Markdown {
    let mut document = Markdown::try_from_content(source).expect("the source loads");
    set_at(document.frontmatter_mut().as_map_mut(), &path(target), value).expect("the pure edit applies");
    document
}

fn steps_of<'p>(target: &'p PropertyPath, document: &'p Markdown) -> Vec<simplified_schema::completion::PathStep<'p>> {
    target.resolve_map(document.frontmatter().as_map()).steps().to_vec()
}

fn assert_unsupported(result: Result<String, PathEditRefusal>) {
    assert!(
        matches!(result, Err(PathEditRefusal::UnsupportedForm { .. })),
        "expected an unsupported form, got {result:?}"
    );
}

fn assert_would_change(result: Result<String, PathEditRefusal>, named: &str) {
    match result {
        Err(PathEditRefusal::WouldChangeUnnamed { reason }) => {
            assert!(reason.contains(named), "`{named}` not in: {reason}");
        }
        other => panic!("expected a refusal naming `{named}`, got {other:?}"),
    }
}

const SHARED_PARENT: &str = "\
---
# about the author
author:   # who wrote this
    # the display name
    name: 'Alice'   # keep me
    # how to reach them
    email: \"a@example.com\"
    since: 0x10
title: Notes
---
# Body

  Kept   exactly.
";

#[test]
fn control_an_unedited_document_is_returned_unchanged() {
    let text = edit_frontmatter_paths(SHARED_PARENT, &values(SHARED_PARENT), &[]).unwrap();
    assert_eq!(text, SHARED_PARENT);
}

#[test]
fn a_nested_edit_keeps_comments_and_spelling_in_the_shared_parent() {
    let text = set(SHARED_PARENT, "author.email", json!("b@example.com")).unwrap();
    assert_eq!(text, SHARED_PARENT.replace("\"a@example.com\"", "b@example.com"));
}

#[test]
fn a_replaced_scalar_keeps_the_comment_after_it() {
    let text = set(SHARED_PARENT, "author.name", json!("Bob")).unwrap();
    assert_eq!(text, SHARED_PARENT.replace("'Alice'", "Bob"));
}

#[test]
fn a_value_that_would_read_as_another_type_stays_a_string() {
    let source = "---\nmeta:\n  version: 1\n  other: 2\n---\n";
    let text = set(source, "meta.version", json!("1.10")).unwrap();
    assert_eq!(text, "---\nmeta:\n  version: '1.10'\n  other: 2\n---\n");
    assert_eq!(values(&text)["meta"]["version"], json!("1.10"));
}

#[test]
fn a_new_key_appends_at_its_parents_indent() {
    let text = set(SHARED_PARENT, "author.url", json!("x.dev")).unwrap();
    assert_eq!(text, SHARED_PARENT.replace("    since: 0x10\n", "    since: 0x10\n    url: x.dev\n"));
}

#[test]
fn missing_parents_are_created_inside_the_nearest_block_mapping() {
    let text = set(SHARED_PARENT, "author.social.site.url", json!("x.dev")).unwrap();
    assert_eq!(
        text,
        SHARED_PARENT.replace(
            "    since: 0x10\n",
            "    since: 0x10\n    social:\n      site:\n        url: x.dev\n"
        )
    );
}

#[test]
fn a_new_top_level_key_goes_at_the_end_of_the_frontmatter() {
    let source = "---\na: 1\n# closing note\n---\nBody\n";
    let text = set(source, "b.c", json!(2)).unwrap();
    assert_eq!(text, "---\na: 1\n# closing note\nb:\n  c: 2\n---\nBody\n");
}

#[test]
fn an_empty_block_or_absent_frontmatter_gains_the_key() {
    assert_eq!(set("---\n---\nBody\n", "a.b", json!(1)).unwrap(), "---\na:\n  b: 1\n---\nBody\n");
    assert_eq!(set("Body\n", "a.b", json!(1)).unwrap(), "---\na:\n  b: 1\n---\nBody\n");
    assert_eq!(set("# only a comment\n", "a", json!(1)).unwrap(), "---\na: 1\n---\n# only a comment\n");
    assert_eq!(set("---\n# note\n---\nBody\n", "a", json!(1)).unwrap(), "---\n# note\na: 1\n---\nBody\n");
}

#[test]
fn keys_that_need_quoting_are_found_and_written_as_keys() {
    let source = "---\n'a.b': 1\n\"\": 2\n---\n";
    assert_eq!(set(source, r"a\.b", json!(3)).unwrap(), "---\n'a.b': 3\n\"\": 2\n---\n");
    assert_eq!(set(source, "", json!(3)).unwrap(), "---\n'a.b': 1\n\"\": 3\n---\n");
    let added = set(source, r"c\.d", json!(3)).unwrap();
    assert_eq!(added, "---\n'a.b': 1\n\"\": 2\nc.d: 3\n---\n");
    assert_eq!(values(&added)["c.d"], json!(3));
    let empty_key = set("---\na: 1\n---\n", "", json!(2)).unwrap();
    assert_eq!(values(&empty_key)[""], json!(2));
}

const SEQUENCES: &str = "\
---
tags:
  - one   # first
  - 'two'
  - three
people:
- name: Ann   # lead
  age: 30
- name: Bo
  nested:
    - x
after: 1
---
Body
";

#[test]
fn an_item_in_a_block_sequence_is_replaced_in_place() {
    let text = set(SEQUENCES, "tags.0", json!("uno")).unwrap();
    assert_eq!(text, SEQUENCES.replace("- one   # first", "- uno   # first"));
    let text = set(SEQUENCES, "tags.1", json!(2)).unwrap();
    assert_eq!(text, SEQUENCES.replace("- 'two'", "- 2"));
}

#[test]
fn a_key_inside_a_sequence_item_is_replaced_and_appended_at_the_items_indent() {
    let text = set(SEQUENCES, "people.0.name", json!("Cy")).unwrap();
    assert_eq!(text, SEQUENCES.replace("name: Ann", "name: Cy"));
    let text = set(SEQUENCES, "people.0.role", json!("dev")).unwrap();
    assert_eq!(text, SEQUENCES.replace("  age: 30\n", "  age: 30\n  role: dev\n"));
    let text = set(SEQUENCES, "people.1.nested.0", json!("y")).unwrap();
    assert_eq!(text, SEQUENCES.replace("    - x\n", "    - y\n"));
}

#[test]
fn a_whole_item_can_become_a_mapping_or_a_scalar() {
    let text = set(SEQUENCES, "tags.2", json!({"k": 1, "l": [2]})).unwrap();
    assert_eq!(text, SEQUENCES.replace("  - three\n", "  - k: 1\n    l:\n    - 2\n"));
    let text = set(SEQUENCES, "people.0", json!("gone")).unwrap();
    assert_eq!(text, SEQUENCES.replace("- name: Ann   # lead\n  age: 30\n", "- gone\n"));
}

#[test]
fn an_item_with_no_value_or_on_its_own_line_can_be_replaced() {
    let source = "---\nitems:\n  -\n  - # note\n  -\n    k: 1\n---\n";
    assert_eq!(set(source, "items.0", json!("a")).unwrap(), "---\nitems:\n  - a\n  - # note\n  -\n    k: 1\n---\n");
    assert_eq!(set(source, "items.1", json!("a")).unwrap(), "---\nitems:\n  -\n  - a # note\n  -\n    k: 1\n---\n");
    assert_eq!(set(source, "items.2.k", json!(2)).unwrap(), "---\nitems:\n  -\n  - # note\n  -\n    k: 2\n---\n");
}

#[test]
fn a_scalar_becomes_a_container_and_keeps_its_comment() {
    let source = "---\nauthor: Ken   # why\nnext: 1\n---\n";
    let text = set(source, "author", json!({"name": "Bob"})).unwrap();
    assert_eq!(text, "---\nauthor:   # why\n  name: Bob\nnext: 1\n---\n");
}

#[test]
fn a_container_becomes_a_scalar_and_keeps_the_key_lines_comment() {
    let source = "---\nauthor: # why\n  name: Bob\n  age: 3\nnext: 1\n---\n";
    assert_eq!(set(source, "author", json!("Ken")).unwrap(), "---\nauthor: Ken # why\nnext: 1\n---\n");
    assert_eq!(set(source, "author", json!({})).unwrap(), "---\nauthor: {} # why\nnext: 1\n---\n");
}

#[test]
fn a_replaced_container_keeps_its_childrens_column() {
    let source = "---\na:\n      x: 1\nb:\n- 1\n- 2\nc: 3\n---\n";
    assert_eq!(set(source, "a", json!({"y": 2})).unwrap(), "---\na:\n      y: 2\nb:\n- 1\n- 2\nc: 3\n---\n");
    assert_eq!(set(source, "b", json!([9])).unwrap(), "---\na:\n      x: 1\nb:\n- 9\nc: 3\n---\n");
}

#[test]
fn an_empty_value_is_filled_in_after_its_colon() {
    let source = "---\na:   # note\nb:\nc: 1\n---\n";
    assert_eq!(set(source, "a", json!(1)).unwrap(), "---\na: 1   # note\nb:\nc: 1\n---\n");
    assert_eq!(set(source, "b", json!({"k": 1})).unwrap(), "---\na:   # note\nb:\n  k: 1\nc: 1\n---\n");
}

#[test]
fn multi_line_scalars_are_replaced_whole() {
    let source = "---\nblock: |\n  one\n  two\nplain: first\n  second\nquoted: \"a\n  b\"   # note\nnext: 1\n---\n";
    assert_eq!(
        set(source, "block", json!("x")).unwrap(),
        "---\nblock: x\nplain: first\n  second\nquoted: \"a\n  b\"   # note\nnext: 1\n---\n"
    );
    assert_eq!(
        set(source, "plain", json!("x")).unwrap(),
        "---\nblock: |\n  one\n  two\nplain: x\nquoted: \"a\n  b\"   # note\nnext: 1\n---\n"
    );
    assert_eq!(
        set(source, "quoted", json!("x")).unwrap(),
        "---\nblock: |\n  one\n  two\nplain: first\n  second\nquoted: x   # note\nnext: 1\n---\n"
    );
}

#[test]
fn a_multi_line_value_is_written_inside_its_parent() {
    let source = "---\nm:\n  text: old\n  next: 1\n---\n";
    let text = set(source, "m.text", json!("line one\nline two")).unwrap();
    assert_eq!(text, "---\nm:\n  text: |-\n    line one\n    line two\n  next: 1\n---\n");
    assert_eq!(values(&text)["m"]["text"], json!("line one\nline two"));
}

#[test]
fn removing_a_node_takes_its_own_lines_and_leaves_its_neighbors() {
    let text = rm(SHARED_PARENT, &["author.email"]).unwrap();
    assert_eq!(text, SHARED_PARENT.replace("    email: \"a@example.com\"\n", ""));
    let text = rm(SHARED_PARENT, &["author"]).unwrap();
    assert!(!text.contains("name:"), "the node's lines go: {text}");
    assert!(text.contains("# about the author\n"), "comments outside it stay: {text}");
    assert!(text.ends_with("title: Notes\n---\n# Body\n\n  Kept   exactly.\n"), "{text}");
    let text = rm(SEQUENCES, &["tags.1"]).unwrap();
    assert_eq!(text, SEQUENCES.replace("  - 'two'\n", ""));
    let text = rm(SEQUENCES, &["people.0"]).unwrap();
    assert_eq!(text, SEQUENCES.replace("- name: Ann   # lead\n  age: 30\n", ""));
}

#[test]
fn removing_the_last_child_leaves_an_empty_parent() {
    let source = "---\nstyle:\n  # page rules\n  page:   # why\n    # width\n    max-width: 1   # note\n  other: 2\n---\n";
    assert_eq!(
        rm(source, &["style.page.max-width"]).unwrap(),
        "---\nstyle:\n  # page rules\n  page: {}   # why\n    # width\n  other: 2\n---\n"
    );
    let source = "---\nitems:\n  - a\nnext: 1\n---\n";
    assert_eq!(rm(source, &["items.0"]).unwrap(), "---\nitems: []\nnext: 1\n---\n");
    let text = rm("---\na:\n  x: 1\n  y: 2\n---\n", &["a.x", "a.y"]).unwrap();
    assert_eq!(text, "---\na: {}\n---\n");
    assert_eq!(values(&text)["a"], json!({}));
}

#[test]
fn removing_the_only_top_level_key_leaves_an_empty_block() {
    assert_eq!(rm("---\na: 1\n---\nBody\n", &["a"]).unwrap(), "---\n---\nBody\n");
    assert_eq!(rm("---\n# note\na: 1\n---\nBody\n", &["a"]).unwrap(), "---\n# note\n---\nBody\n");
}

#[test]
fn removals_use_original_indexes_in_either_order() {
    let source = "---\nitems: \n  - a\n  - b # keep\n  - c\n  - d\n---\n";
    let expected = "---\nitems: \n  - b # keep\n  - d\n---\n";
    assert_eq!(rm(source, &["items.0", "items.2"]).unwrap(), expected);
    assert_eq!(rm(source, &["items.2", "items.0"]).unwrap(), expected);
}

#[test]
fn removing_a_key_that_shares_a_line_with_a_dash_keeps_the_dash() {
    let source = "---\npeople:\n- name: Ann\n  age: 30\n- only: 1\n---\n";
    assert_eq!(rm(source, &["people.0.name"]).unwrap(), "---\npeople:\n- age: 30\n- only: 1\n---\n");
    assert_eq!(rm(source, &["people.1.only"]).unwrap(), "---\npeople:\n- name: Ann\n  age: 30\n- {}\n---\n");
}

#[test]
fn a_sequence_nested_in_a_sequence_is_edited_on_its_own_lines_only() {
    let source = "---\ngrid:\n  -\n    - x\n    - y\n  -\n    - z\n---\n";
    assert_eq!(set(source, "grid.0.1", json!("w")).unwrap(), "---\ngrid:\n  -\n    - x\n    - w\n  -\n    - z\n---\n");
    assert_eq!(rm(source, &["grid.0.0"]).unwrap(), "---\ngrid:\n  -\n    - y\n  -\n    - z\n---\n");
    assert_eq!(rm(source, &["grid.1.0"]).unwrap(), "---\ngrid:\n  -\n    - x\n    - y\n  -\n    []\n---\n");
    assert_unsupported(set("---\ngrid:\n- - x\n  - y\n---\n", "grid.0.0", json!("w")));
}

#[test]
fn an_item_whose_dash_is_on_its_own_line_is_removed_with_it() {
    let source = "---\nitems:\n  -\n    # note\n    k: 1\n  - b\n---\n";
    let text = rm(source, &["items.1"]).unwrap();
    assert_eq!(text, "---\nitems:\n  -\n    # note\n    k: 1\n---\n");
    let text = rm(source, &["items.0"]).unwrap();
    assert_eq!(text, "---\nitems:\n  - b\n---\n");
}

const CRLF: &str = "---\r\nauthor:\r\n  name: Alice # c\r\n  items:\r\n    - a\r\n    - b\r\nnext: 1\r\n---\r\nBody\r\n";

#[test]
fn a_crlf_document_keeps_its_line_endings() {
    assert_eq!(set(CRLF, "author.name", json!("Bob")).unwrap(), CRLF.replace("Alice", "Bob"));
    assert_eq!(
        set(CRLF, "author.email", json!("e")).unwrap(),
        CRLF.replace("    - b\r\n", "    - b\r\n  email: e\r\n")
    );
    assert_eq!(
        set(CRLF, "author.name", json!({"first": "A", "last": "B"})).unwrap(),
        CRLF.replace("  name: Alice # c\r\n", "  name: # c\r\n    first: A\r\n    last: B\r\n")
    );
    assert_eq!(set(CRLF, "new.key", json!(1)).unwrap(), CRLF.replace("next: 1\r\n", "next: 1\r\nnew:\r\n  key: 1\r\n"));
    assert_eq!(rm(CRLF, &["author.items.0"]).unwrap(), CRLF.replace("    - a\r\n", ""));
    assert_eq!(
        rm(CRLF, &["author.items.0", "author.items.1"]).unwrap(),
        CRLF.replace("  items:\r\n    - a\r\n    - b\r\n", "  items: []\r\n")
    );
    assert_eq!(
        set(CRLF, "author.name", json!("one\ntwo")).unwrap(),
        CRLF.replace("name: Alice # c", "name: |- # c\r\n    one\r\n    two")
    );
}

#[test]
fn a_crlf_document_with_no_frontmatter_gets_a_crlf_block() {
    assert_eq!(set("Body\r\nMore\r\n", "a", json!(1)).unwrap(), "---\r\na: 1\r\n---\r\nBody\r\nMore\r\n");
}

const FLOW: &str = "---\n# kept only in place\nauthor: {name: Alice, email: e}\ntags: [a, b]   # note\nnext: 1\n---\n\n  Body  \n";

#[test]
fn walking_into_a_flow_collection_is_refused_and_falls_back() {
    assert_unsupported(set(FLOW, "author.name", json!("Bob")));
    assert_unsupported(set(FLOW, "tags.0", json!("z")));
    assert_unsupported(rm(FLOW, &["author.email"]));

    let document = edited_by_set(FLOW, "author.name", json!("Bob"));
    let target = path("author.name");
    let text = edited_document_text(&document, &[PathEdit::Set(steps_of(&target, &document))]).unwrap();
    assert_eq!(values(&text)["author"], json!({"name": "Bob", "email": "e"}));
    assert_eq!(values(&text)["tags"], json!(["a", "b"]));
    assert!(text.ends_with("---\n\n  Body  \n"), "the body stays: {text:?}");
    assert!(!text.contains("# kept only in place"), "this path re-serializes: {text:?}");
}

#[test]
fn the_fallback_keeps_the_fences_and_body_bytes_of_a_crlf_document() {
    let source = "---\r\nauthor: {name: Alice}\r\n---\r\nBody  \r\n\r\nMore\r\n";
    let document = edited_by_set(source, "author.name", json!("Bob"));
    let target = path("author.name");
    let text = edited_document_text(&document, &[PathEdit::Set(steps_of(&target, &document))]).unwrap();
    assert_eq!(text, "---\r\nauthor:\r\n  name: Bob\r\n---\r\nBody  \r\n\r\nMore\r\n");
}

#[test]
fn a_comment_stays_on_the_first_line_of_a_value_that_grows() {
    let source = "---\na: x   # one\nb:   # two\nc: # three\n  k: 1\nd:\n  - y # four\n  - # five\n---\n";
    let long = json!("p\nq");
    assert_eq!(set(source, "a", long.clone()).unwrap(), source.replace("a: x   # one", "a: |-   # one\n  p\n  q"));
    assert_eq!(set(source, "b", long.clone()).unwrap(), source.replace("b:   # two", "b: |-   # two\n  p\n  q"));
    assert_eq!(set(source, "c", long.clone()).unwrap(), source.replace("c: # three\n  k: 1", "c: |- # three\n  p\n  q"));
    assert_eq!(set(source, "d.0", long.clone()).unwrap(), source.replace("- y # four", "- |- # four\n      p\n      q"));
    assert_eq!(set(source, "d.1", long).unwrap(), source.replace("- # five", "- |- # five\n      p\n      q"));
}

#[test]
fn a_flow_collection_named_whole_is_replaced_in_place() {
    assert_eq!(set(FLOW, "tags", json!("t")).unwrap(), FLOW.replace("[a, b]", "t"));
    let text = rm(FLOW, &["tags"]).unwrap();
    assert_eq!(text, FLOW.replace("tags: [a, b]   # note\n", ""));
    let multi = "---\ntags: [\n  a,\n  b\n  ]\nnext: 1\n---\n";
    assert_eq!(set(multi, "tags", json!(1)).unwrap(), "---\ntags: 1\nnext: 1\n---\n");
    // A closing bracket at column 0 reads as a top-level line, which the
    // top-level scanner refuses.
    assert_unsupported(set("---\ntags: [\n  a\n]\n---\n", "tags", json!(1)));
}

#[test]
fn an_empty_flow_mapping_takes_a_new_key_as_a_block() {
    let source = "---\nstyle: {}   # why\npage:\n  rules: {}\n---\n";
    assert_eq!(
        set(source, "style.width", json!(1)).unwrap(),
        "---\nstyle:   # why\n  width: 1\npage:\n  rules: {}\n---\n"
    );
    assert_eq!(
        set(source, "page.rules.a.b", json!(1)).unwrap(),
        "---\nstyle: {}   # why\npage:\n  rules:\n    a:\n      b: 1\n---\n"
    );
}

#[test]
fn an_edit_an_alias_reads_is_refused_with_no_fallback() {
    let source = "---\nbase: &b\n  x: 1\nother: *b\n---\n";
    let attempt = set(source, "base.x", json!(2));
    assert_would_change(attempt, "other");

    let document = edited_by_set(source, "base.x", json!(2));
    let target = path("base.x");
    let result = edited_document_text(&document, &[PathEdit::Set(steps_of(&target, &document))]);
    assert_would_change(result, "other");
}

#[test]
fn an_anchored_node_with_no_alias_reading_the_change_can_be_edited() {
    let source = "---\nbase: &b # shared\n  x: 1\n  y: 2\nother: *b\nlone: &l\n  z: 1\n---\n";
    assert_eq!(
        set(source, "lone.z", json!(2)).unwrap(),
        "---\nbase: &b # shared\n  x: 1\n  y: 2\nother: *b\nlone: &l\n  z: 2\n---\n"
    );
}

#[test]
fn removing_an_anchor_an_alias_needs_is_refused_with_no_fallback() {
    let source = "---\na: &x 1\nb: *x\nc: 2\n---\n";
    assert_would_change(rm(source, &["a"]), "anchor");
    assert_would_change(set(source, "a", json!(5)), "anchor");
    assert_would_change(rm(source, &["a", "c"]), "anchor");

    let targets = [path("a")];
    let mut document = Markdown::try_from_content(source).unwrap();
    let plan = RemovalPlan::new(document.frontmatter().as_map(), &targets).unwrap();
    plan.apply(document.frontmatter_mut().as_map_mut());
    let edits: Vec<PathEdit> = plan.targets().map(PathEdit::Remove).collect();
    assert_would_change(edited_document_text(&document, &edits), "anchor");
}

#[test]
fn an_anchor_redeclared_earlier_cannot_silently_redirect_an_alias() {
    let source = "---\none: &n 1\ntwo: &n 2\nreader: *n\n---\n";
    assert_would_change(set(source, "two", json!(9)), "reader");
    assert_would_change(rm(source, &["two"]), "reader");
}

#[test]
fn replacing_an_alias_itself_changes_only_that_value() {
    let source = "---\na: &x 1\nb: *x\nc: *x\n---\n";
    assert_eq!(set(source, "b", json!(2)).unwrap(), "---\na: &x 1\nb: 2\nc: *x\n---\n");
}

#[test]
fn walking_through_an_alias_or_tag_is_refused_and_serialization_keeps_the_values() {
    let source = "---\nbase: &b\n  x: 1\nother: *b\n---\nBody\n";
    assert_unsupported(set(source, "other.x", json!(2)));
    let document = edited_by_set(source, "other.x", json!(2));
    let target = path("other.x");
    let text = edited_document_text(&document, &[PathEdit::Set(steps_of(&target, &document))]).unwrap();
    assert_eq!(values(&text)["base"], json!({"x": 1}));
    assert_eq!(values(&text)["other"], json!({"x": 2}));

    assert_unsupported(set("---\na: !!map\n  x: 1\n---\n", "a.x", json!(2)));
}

#[test]
fn an_indented_root_is_refused_and_serialization_writes_it() {
    let source = "---\n  a:\n    b: 1 # gone\n  c: 2\n---\nBody\n";
    assert_unsupported(set(source, "a.b", json!(2)));
    assert_unsupported(rm(source, &["c"]));
    let document = edited_by_set(source, "a.b", json!(2));
    let target = path("a.b");
    let text = edited_document_text(&document, &[PathEdit::Set(steps_of(&target, &document))]).unwrap();
    assert_eq!(text, "---\na:\n  b: 2\nc: 2\n---\nBody\n");
}

#[test]
fn a_bare_template_beside_the_edit_is_kept() {
    let source = "---\ntitle: {{ name }}\nmeta:\n  a: 1 # c\n---\nBody\n";
    assert_eq!(set(source, "meta.a", json!(2)).unwrap(), source.replace("a: 1", "a: 2"));
}

#[test]
fn frontmatter_only_the_lenient_loader_reads_falls_back() {
    let source = "---\ntitle: \"$(dirname \"{{ path }}\")\"\nmeta:\n  a: 1\n---\nBody\n";
    assert_unsupported(set(source, "meta.a", json!(2)));
    let document = edited_by_set(source, "meta.a", json!(2));
    let target = path("meta.a");
    let text = edited_document_text(&document, &[PathEdit::Set(steps_of(&target, &document))]).unwrap();
    assert_eq!(values(&text)["meta"]["a"], json!(2));
    assert_eq!(values(&text)["title"], values(source)["title"]);
}

#[test]
fn an_expected_value_the_text_does_not_hold_is_refused() {
    let source = "---\na: 1\nb: 2\n---\n";
    let mut expected = values(source);
    expected.insert("a".to_string(), json!(5));
    expected.insert("b".to_string(), json!(6));
    let target = path("a");
    let steps = target.resolve_map(&expected).steps().to_vec();
    assert_would_change(edit_frontmatter_paths(source, &expected, &[PathEdit::Set(steps)]), "b");
}

#[test]
fn a_scalar_keeping_its_trailing_blank_lines_is_not_cut_short() {
    let source = "---\na:\n  t: |+\n    x\n\nb: 1\n---\n";
    let text = set(source, "a.new", json!(1)).unwrap();
    assert_eq!(values(&text)["a"], json!({"t": "x\n\n", "new": 1}));
    assert_eq!(values(&text)["b"], json!(1));

    let text = set(source, "b", json!(2)).unwrap();
    assert_eq!(text, source.replace("b: 1", "b: 2"));
    let text = rm(source, &["a.t"]).unwrap();
    assert_eq!(values(&text)["a"], json!({}));
    assert_eq!(values(&text)["b"], json!(1));
    let text = set(source, "a.t", json!("y")).unwrap();
    assert_eq!(values(&text)["a"], json!({"t": "y"}));
    assert_eq!(values(&text)["b"], json!(1));
}

#[test]
fn a_lone_cr_document_is_edited_exactly_or_refused() {
    let source = "---\ra:\r  b: 1\r  c: 2\rd: 3\r---\rBody\r";
    for result in [
        set(source, "a.b", json!(9)),
        set(source, "a.new", json!(9)),
        set(source, "d", json!(9)),
        rm(source, &["a.b"]),
    ] {
        match result {
            Err(PathEditRefusal::UnsupportedForm { .. }) => {}
            Ok(text) => assert!(text.ends_with("---\rBody\r"), "{text:?}"),
            other => panic!("{other:?}"),
        }
    }
    let document = edited_by_set(source, "a.b", json!(9));
    let target = path("a.b");
    let text = edited_document_text(&document, &[PathEdit::Set(steps_of(&target, &document))]).unwrap();
    assert_eq!(values(&text)["a"], json!({"b": 9, "c": 2}));
    assert!(text.ends_with("---\rBody\r"), "{text:?}");
}

#[test]
fn unusual_key_spellings_are_matched_by_their_decoded_text() {
    let source = "---\nouter :\n  \"in ner\"  : 1\n  'it''s': 2\n  a:b: 3\n  caf\u{e9}: \u{1f600} # \u{e9}\nnext: 1\n---\n";
    assert_eq!(set(source, "outer.in ner", json!(9)).unwrap(), source.replace("\"in ner\"  : 1", "\"in ner\"  : 9"));
    assert_eq!(set(source, "outer.it's", json!(9)).unwrap(), source.replace("'it''s': 2", "'it''s': 9"));
    assert_eq!(set(source, "outer.a:b", json!(9)).unwrap(), source.replace("a:b: 3", "a:b: 9"));
    assert_eq!(set(source, "outer.caf\u{e9}", json!("\u{e9}")).unwrap(), source.replace("\u{1f600} #", "\u{e9} #"));
    assert_eq!(rm(source, &["outer.caf\u{e9}"]).unwrap(), source.replace("  caf\u{e9}: \u{1f600} # \u{e9}\n", ""));
    let added = set(source, "outer.\u{e9}t\u{e9}", json!(1)).unwrap();
    assert_eq!(values(&added)["outer"]["\u{e9}t\u{e9}"], json!(1));
    assert_eq!(added, source.replace("# \u{e9}\n", "# \u{e9}\n  \u{e9}t\u{e9}: 1\n"));
}

#[test]
fn a_merge_key_is_an_ordinary_key_beside_the_edit() {
    let source = "---\nbase: &b\n  x: 1\nchild:\n  <<: *b\n  y: 2 # c\n---\n";
    assert_eq!(set(source, "child.y", json!(3)).unwrap(), source.replace("y: 2", "y: 3"));
    assert_unsupported(set(source, "child.<<.x", json!(3)));
}

#[test]
fn text_that_only_looks_like_structure_is_not_mistaken_for_it() {
    let source = "---\nnote: |\n  key: not a key\n  - not an item\nurl: http://x.dev/#frag # real\nquoted: \"a: b # c\" # real\nnext: 1\n---\n";
    assert_eq!(set(source, "next", json!(2)).unwrap(), source.replace("next: 1", "next: 2"));
    assert_eq!(set(source, "url", json!("y")).unwrap(), source.replace("http://x.dev/#frag", "y"));
    assert_eq!(set(source, "quoted", json!("y")).unwrap(), source.replace("\"a: b # c\"", "y"));
    assert_eq!(set(source, "note", json!("y")).unwrap(), source.replace("|\n  key: not a key\n  - not an item", "y"));
    let text = rm(source, &["note"]).unwrap();
    assert!(text.starts_with("---\nurl:"), "{text}");
}

#[test]
fn a_document_built_in_memory_is_serialized() {
    let mut document = Markdown::new("Body\n".to_string());
    set_at(document.frontmatter_mut().as_map_mut(), &path("a.b"), json!(1)).unwrap();
    assert_eq!(edited_document_text(&document, &[]).unwrap(), "---\na:\n  b: 1\n---\nBody\n");
}

#[test]
fn only_the_named_node_changes_across_a_read_write_read_round_trip() {
    let mut text = SHARED_PARENT.to_string();
    let edits: [(&str, Value); 4] = [
        ("author.name", json!("Bob")),
        ("author.social.site", json!("x.dev")),
        ("title", json!("1.10")),
        ("author.social.tags", json!(["a", "b: c", ""])),
    ];
    let mut expected = values(&text);
    for (target, value) in edits {
        let target = path(target);
        set_at(&mut expected, &target, value).unwrap();
        let steps = target.resolve_map(&expected).steps().to_vec();
        text = edit_frontmatter_paths(&text, &expected, &[PathEdit::Set(steps)]).unwrap();
        assert_eq!(values(&text), expected);
    }
    let again = edit_frontmatter_paths(&text, &expected, &[]).unwrap();
    assert_eq!(again, text, "a repeated read changes nothing");

    for comment in ["# about the author", "# who wrote this", "# the display name", "# keep me", "# how to reach them"] {
        assert!(text.contains(comment), "lost `{comment}`: {text}");
    }
    assert!(text.contains("since: 0x10\n"), "a sibling keeps its spelling: {text}");
    assert!(text.ends_with("---\n# Body\n\n  Kept   exactly.\n"), "the body stays: {text}");

    let removed = rm(&text, &["author.social.tags.1", "author.social.site", "title"]).unwrap();
    let reloaded = values(&removed);
    assert_eq!(reloaded["author"]["social"], json!({"tags": ["a", ""]}));
    assert!(!reloaded.contains_key("title"));
    assert_eq!(rm(&removed, &["author.social.tags.0", "author.social.tags.1"]).map(|text| values(&text)["author"]["social"].clone()).unwrap(), json!({"tags": []}));
}
