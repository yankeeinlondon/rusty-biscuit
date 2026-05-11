//! `MermaidThemeError` variant snapshots.

use darkmatter::mermaid::MermaidThemeError;

use crate::helpers::{assert_contains_all, render};

#[test]
fn invalid_json_surfaces_line_column_and_snippet() {
    let parse_error = serde_json::from_str::<serde_json::Value>("{ bogus }").unwrap_err();
    let err = MermaidThemeError::invalid_json("{ bogus }", parse_error);
    let out = render(&err);
    assert_contains_all(
        &out,
        &[
            "MermaidThemeError",
            "invalid JSON",
            "line",
            "column",
            "Snippet:",
            "{ bogus }",
        ],
    );
}

#[test]
fn invalid_color_lists_accepted_formats() {
    let err = MermaidThemeError::InvalidColor {
        field: "background".into(),
        value: "notacolor".into(),
    };
    let out = render(&err);
    assert_contains_all(
        &out,
        &[
            "MermaidThemeError",
            "invalid color",
            "background",
            "notacolor",
            "#rgb",
        ],
    );
}
