//! `StylesheetError` variant snapshots.

use darkmatter::render::stylesheet::{CssValueKind, StylesheetError};

use crate::helpers::{assert_contains_all, render};

#[test]
fn invalid_declaration_shows_input() {
    let err = StylesheetError::InvalidDeclaration {
        declaration: "color".into(),
    };
    let out = render(&err);
    assert_contains_all(
        &out,
        &[
            "StylesheetError",
            "invalid declaration",
            "color",
            "property: value",
        ],
    );
}

#[test]
fn invalid_property_name_shows_name() {
    let err = StylesheetError::InvalidPropertyName {
        name: "-moz@broken".into(),
    };
    let out = render(&err);
    assert_contains_all(
        &out,
        &["StylesheetError", "invalid property name", "-moz@broken"],
    );
}

#[test]
fn property_value_type_mismatch_shows_expected_actual_and_example() {
    let err = StylesheetError::PropertyValueTypeMismatch {
        property: "font-size".into(),
        expected: CssValueKind::Sizing,
        actual: CssValueKind::Integer,
        value: "40".into(),
    };
    let out = render(&err);
    assert_contains_all(
        &out,
        &[
            "StylesheetError",
            "property/value type mismatch",
            "font-size",
            "sizing",
            "integer",
            "40",
            "16px",
        ],
    );
}

#[test]
fn invalid_sizing_hints_at_accepted_tokens() {
    let err = StylesheetError::InvalidSizing {
        value: "abc".into(),
    };
    let out = render(&err);
    assert_contains_all(
        &out,
        &["StylesheetError", "invalid sizing value", "abc", "42px"],
    );
}

#[test]
fn invalid_multi_sizing_shows_token_count_hint() {
    let err = StylesheetError::InvalidSizingMulti {
        value: "1px 2px 3px 4px 5px".into(),
    };
    let out = render(&err);
    assert_contains_all(
        &out,
        &[
            "StylesheetError",
            "invalid multi-sizing value",
            "1px 2px 3px 4px 5px",
            "1 to 4",
        ],
    );
}

#[test]
fn invalid_color_lists_accepted_formats() {
    let err = StylesheetError::InvalidColor {
        value: "notacolor".into(),
    };
    let out = render(&err);
    assert_contains_all(
        &out,
        &[
            "StylesheetError",
            "invalid color value",
            "notacolor",
            "#rrggbb",
            "rgb(",
        ],
    );
}

#[test]
fn invalid_integer_hints_at_whole_number() {
    let err = StylesheetError::InvalidInteger {
        value: "1.5".into(),
    };
    let out = render(&err);
    assert_contains_all(
        &out,
        &["StylesheetError", "invalid integer value", "1.5", "-3"],
    );
}

#[test]
fn invalid_raw_value_hints_at_semicolons() {
    let err = StylesheetError::InvalidRawValue {
        value: "1px;".into(),
    };
    let out = render(&err);
    assert_contains_all(&out, &["StylesheetError", "invalid raw value", "1px;"]);
}

#[test]
fn invalid_custom_property_hints_at_dashes() {
    let err = StylesheetError::InvalidCustomProperty { name: "foo".into() };
    let out = render(&err);
    assert_contains_all(
        &out,
        &["StylesheetError", "invalid custom property", "foo", "--"],
    );
}
