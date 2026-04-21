//! `MarkdownError` variant snapshots.

use std::io;

use darkmatter::markdown::MarkdownError;
use darkmatter::markdown::compose::TransclusionError;

use crate::helpers::{assert_contains_all, render};

// NOTE: `MarkdownError::FrontmatterParse` wraps `biscuit_file::YamlParseError`
// (an alias for `serde_yaml_ng::Error`). Constructing that error type directly
// from darkmatter's test code would require adding `serde_yaml_ng` as a
// dev-dependency just for one fixture. Since the rendering path for this
// variant is covered by the inline tests in `darkmatter::markdown::errors`,
// we skip it here to keep the integration-test dependency surface narrow.

#[test]
fn frontmatter_merge_renders_leaf_block() {
    let err = MarkdownError::FrontmatterMerge("conflict in 'title'".into());
    let out = render(&err);
    assert_contains_all(
        &out,
        &["MarkdownError", "frontmatter merge failed", "conflict in 'title'"],
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
        &["MarkdownError", "AST parse failed", "line 3: unexpected token"],
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
        &[
            "MarkdownError",
            "serialization failed",
            "line",
            "column",
        ],
    );
}

// NOTE: `MarkdownError::UrlFetch` wraps `reqwest::Error`, which cannot be
// constructed directly without firing an actual request — the unit tests in
// `darkmatter::markdown::errors::blocks` exercise that renderer instead.

#[test]
fn transclusion_delegation_surfaces_inner_block_only() {
    let inner = TransclusionError::CycleDetected {
        chain: vec!["a.md".into(), "b.md".into(), "a.md".into()],
    };
    let err = MarkdownError::Transclusion(inner);
    let out = render(&err);
    assert_contains_all(
        &out,
        &["TransclusionError", "cycle detected", "a.md", "b.md"],
    );
    assert!(
        !out.contains("Caused by:"),
        "delegating variant should not duplicate the inner block under a Caused-by: caption; got:\n{out}",
    );
}
