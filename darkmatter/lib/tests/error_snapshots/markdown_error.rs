//! `MarkdownError` variant snapshots.

use std::io;
use std::path::PathBuf;

use darkmatter::markdown::MarkdownError;
use darkmatter::markdown::compose::TransclusionError;
use darkmatter::markdown::schemas::ValidationProblem;

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

#[test]
fn schema_validation_missing_required_renders_block() {
    let err = MarkdownError::SchemaValidationFailed {
        path: PathBuf::from("/tmp/test/plan.md"),
        problems: vec![ValidationProblem {
            path: "".to_string(),
            property: Some("spec".to_string()),
            message: "\"spec\" is a required property".to_string(),
            line: Some(3),
            column: Some(1),
            arm_index: None,
        }],
        summary: "frontmatter did not satisfy the schema".to_string(),
        description: Some("Planner prompt".to_string()),
    };
    let out = render(&err);
    insta::assert_snapshot!(out);
}

#[test]
fn schema_validation_wrong_type_renders_block() {
    let err = MarkdownError::SchemaValidationFailed {
        path: PathBuf::from("/tmp/test/config.md"),
        problems: vec![ValidationProblem {
            path: "/count".to_string(),
            property: None,
            message: "42 is not of type \"string\"".to_string(),
            line: Some(4),
            column: Some(8),
            arm_index: None,
        }],
        summary: "frontmatter did not satisfy the schema".to_string(),
        description: None,
    };
    let out = render(&err);
    insta::assert_snapshot!(out);
}

#[test]
fn schema_validation_format_failure_renders_block() {
    let err = MarkdownError::SchemaValidationFailed {
        path: PathBuf::from("/tmp/test/doc.md"),
        problems: vec![ValidationProblem {
            path: "/cover".to_string(),
            property: None,
            message: "\"\" is not a valid \"darkmatter-file\" format".to_string(),
            line: Some(5),
            column: Some(7),
            arm_index: None,
        }],
        summary: "frontmatter did not satisfy the schema".to_string(),
        description: Some("Design document".to_string()),
    };
    let out = render(&err);
    insta::assert_snapshot!(out);
}

#[test]
fn schema_validation_preparation_failure_renders_summary() {
    // When the schema itself cannot be prepared (malformed `$schema`,
    // unresolved reference, etc.) the validation stage emits an empty
    // problem list and stuffs the underlying diagnostic into `summary`.
    // The rendered block must surface that summary so authors see the
    // actual root cause instead of just the path.
    let err = MarkdownError::SchemaValidationFailed {
        path: PathBuf::from("/tmp/test/bad-schema.md"),
        problems: vec![],
        summary: "schema could not be prepared: $schema must be a mapping or array of mappings"
            .to_string(),
        description: None,
    };
    let out = render(&err);
    insta::assert_snapshot!(out);
}

#[test]
fn schema_validation_preparation_failure_with_description_renders_summary() {
    let err = MarkdownError::SchemaValidationFailed {
        path: PathBuf::from("/tmp/test/unresolved.md"),
        problems: vec![],
        summary: "schema could not be prepared: could not resolve ./missing.yaml".to_string(),
        description: Some("Planner prompt".to_string()),
    };
    let out = render(&err);
    insta::assert_snapshot!(out);
}

#[test]
fn schema_validation_multiple_problems_renders_block() {
    let err = MarkdownError::SchemaValidationFailed {
        path: PathBuf::from("/tmp/test/multi.md"),
        problems: vec![
            ValidationProblem {
                path: "".to_string(),
                property: Some("title".to_string()),
                message: "\"title\" is a required property".to_string(),
                line: Some(2),
                column: Some(1),
                arm_index: None,
            },
            ValidationProblem {
                path: "/version".to_string(),
                property: None,
                message: "\"alpha\" is not of type \"number\"".to_string(),
                line: Some(4),
                column: Some(10),
                arm_index: None,
            },
        ],
        summary: "frontmatter did not satisfy the schema".to_string(),
        description: None,
    };
    let out = render(&err);
    insta::assert_snapshot!(out);
}
