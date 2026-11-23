//! Markdown frontmatter extraction through the public loaders
//! (`Markdown::try_from_content` and `Markdown::try_from(&Path)`): a valid
//! fence keeps an omitted property absent and loads `k:` and `k: null` as
//! the same `null`; malformed YAML and a duplicate key inside the fences are
//! located load errors; and every body line after the closing fence keeps
//! its text (the loader rejoins lines with `\n` and drops the final line
//! ending). Body grammar is not under test here.

use darkmatter::markdown::{Markdown, MarkdownError};
use serde_json::{Value, json};

/// A body that holds the lines a fence scanner could misread.
const BODY: &str = "# Title\n\n  indented line\n\n---\n\nafter a thematic break: `---`\n";

#[test]
fn an_omitted_property_is_absent_and_an_empty_or_null_one_is_null() {
    let document = Markdown::try_from_content(format!("---\ntitle: T\nempty:\nexplicit: null\n---\n{BODY}"))
        .expect("the frontmatter loads");
    let map = document.frontmatter().as_map();
    assert_eq!(map.keys().map(String::as_str).collect::<Vec<_>>(), ["title", "empty", "explicit"]);
    assert_eq!(map["empty"], Value::Null);
    assert_eq!(map["explicit"], Value::Null);
    assert!(!map.contains_key("omitted"));
    assert_eq!(map["title"], json!("T"));
}

#[test]
fn the_body_after_the_closing_fence_is_unchanged() {
    let document = Markdown::try_from_content(format!("---\ntitle: T\n---\n{BODY}")).expect("the frontmatter loads");
    assert_eq!(document.content().trim_end_matches('\n'), BODY.trim_end_matches('\n'));
    // An empty block is still a fence: its body is everything after it.
    let empty = Markdown::try_from_content(format!("---\n---\n{BODY}")).expect("an empty block loads");
    assert!(empty.frontmatter().is_empty());
    assert_eq!(empty.content().trim_end_matches('\n'), BODY.trim_end_matches('\n'));
}

/// The YAML error of a document whose frontmatter does not load, through the
/// path loader, with the error's file.
fn load_error(name: &str, frontmatter: &str) -> (String, usize, usize, String) {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join(name);
    let text = format!("---\n{frontmatter}---\n{BODY}");
    std::fs::write(&path, &text).expect("write document");
    let error = Markdown::try_from(path.as_path()).expect_err("the frontmatter does not load");
    let rendered = error.to_string();
    let MarkdownError::FrontmatterParse { source, .. } = error else {
        panic!("a frontmatter load error: {rendered}");
    };
    let location = source.location().expect("the YAML error is located");
    assert_eq!(std::fs::read_to_string(&path).expect("read document"), text, "loading never edits the source");
    (rendered, location.line(), location.column(), source.to_string())
}

#[test]
fn malformed_yaml_inside_the_fences_is_a_located_load_error() {
    let (rendered, line, column, message) = load_error("broken.md", "title: T\nitems: [a, b\n");
    assert!(rendered.contains("broken.md"), "the error names the file: {rendered}");
    assert_eq!((line, column), (3, 1), "located where the unclosed list ends: {message}");
}

/// The frontmatter loader locates a duplicate key at the mapping that holds
/// it, line and column inside the block; the message names the key.
#[test]
fn a_duplicate_key_inside_the_fences_is_a_located_load_error() {
    let (rendered, line, column, message) = load_error("twice.md", "status: a\ntitle: T\nstatus: b\n");
    assert!(rendered.contains("twice.md"), "the error names the file: {rendered}");
    assert!(message.contains("duplicate entry with key \"status\""), "{message}");
    assert_eq!((line, column), (1, 1), "located at the holding mapping: {message}");

    let (_, line, column, message) = load_error("nested.md", "title: T\nouter:\n  status: a\n  status: b\n");
    assert!(message.contains("duplicate entry with key \"status\""), "{message}");
    assert_eq!((line, column), (3, 3), "located at the nested mapping: {message}");
}

