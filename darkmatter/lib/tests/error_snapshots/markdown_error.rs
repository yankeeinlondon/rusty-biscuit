//! `MarkdownError` variant snapshots.

use std::io;
use std::path::PathBuf;

use darkmatter::markdown::MarkdownError;
use darkmatter::markdown::compose::TransclusionError;

use crate::helpers::{assert_contains_all, render};

// NOTE: `MarkdownError::FrontmatterParse` is a struct variant carrying
// `source: YamlParseError` (alias for `serde_yaml_ng::Error`) plus the
// original `yaml: String` body so renderers can include the offending line.
// The leaf-block renderer `frontmatter_parse_block` is unit-tested directly
// in `darkmatter::markdown::errors::blocks::tests` (both the generic
// rendering smoke test and the offending-line snippet regression test) so
// we do not duplicate that coverage here.

#[test]
fn frontmatter_merge_renders_leaf_block() {
    let err = MarkdownError::FrontmatterMerge("conflict in 'title'".into());
    let out = render(&err);
    assert_contains_all(
        &out,
        &[
            "MarkdownError",
            "frontmatter merge failed",
            "conflict in 'title'",
        ],
    );
}

#[test]
fn file_load_renders_leaf_block_with_kind() {
    let err = MarkdownError::FileLoad(io::Error::new(io::ErrorKind::NotFound, "missing"));
    let out = render(&err);
    assert_contains_all(&out, &["MarkdownError", "file load failed", "NotFound"]);
}

#[test]
fn theme_load_renders_leaf_block() {
    let err = MarkdownError::ThemeLoad("unknown theme `neon`".into());
    let out = render(&err);
    assert_contains_all(
        &out,
        &["MarkdownError", "theme load failed", "unknown theme `neon`"],
    );
}

#[test]
fn ast_parse_renders_leaf_block() {
    let err = MarkdownError::AstParse("line 3: unexpected token".into());
    let out = render(&err);
    assert_contains_all(
        &out,
        &[
            "MarkdownError",
            "AST parse failed",
            "line 3: unexpected token",
        ],
    );
}

#[test]
fn invalid_line_range_renders_leaf_block() {
    let err = MarkdownError::InvalidLineRange("start > end".into());
    let out = render(&err);
    assert_contains_all(
        &out,
        &["MarkdownError", "invalid line range", "start > end"],
    );
}

#[test]
fn transform_renders_leaf_block() {
    let err = MarkdownError::Transform("pipeline stalled".into());
    let out = render(&err);
    assert_contains_all(
        &out,
        &["MarkdownError", "transform failed", "pipeline stalled"],
    );
}

#[test]
fn serialization_renders_leaf_block_with_position() {
    let json_error = serde_json::from_str::<serde_json::Value>("{ bogus }").unwrap_err();
    let err = MarkdownError::Serialization(json_error);
    let out = render(&err);
    assert_contains_all(
        &out,
        &["MarkdownError", "serialization failed", "line", "column"],
    );
}

// NOTE: `MarkdownError::UrlFetch` wraps `reqwest::Error`, which cannot be
// constructed directly without firing an actual request. The leaf-block
// renderer `url_fetch_block` is unit-tested via a live short-circuited
// request in `darkmatter::markdown::errors::blocks::tests::url_fetch_block_renders_with_reqwest_error`.

#[test]
fn transclusion_delegation_surfaces_inner_block_only() {
    let inner = TransclusionError::CycleDetected {
        chain: vec![
            (PathBuf::from("a.md"), 3),
            (PathBuf::from("b.md"), 7),
            (PathBuf::from("a.md"), 3),
        ],
    };
    let err = MarkdownError::Transclusion(Box::new(inner));
    let out = render(&err);
    assert_contains_all(
        &out,
        &[
            "TransclusionError",
            "cycle detected",
            "a.md",
            "b.md",
            ":line 3",
            ":line 7",
        ],
    );
    assert!(
        !out.contains("Caused by:"),
        "delegating variant should not duplicate the inner block under a Caused-by: caption; got:\n{out}",
    );
}
