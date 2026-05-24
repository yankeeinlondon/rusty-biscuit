//! Parse entry points for the style frontmatter.

use serde_json::Value;

use crate::style::color;
use crate::style::descriptor::{
    self, LeafType, SchemaLeaf, is_canonical_container, join_path, kebabify, leaf_for_canonical,
};
use crate::style::error::StyleParseError;
use crate::style::length::{HorizontalLengthError, parse_horizontal_typed};
use crate::style::schema::StyleFrontmatter;
use crate::style::walker;
use crate::style::warning::{StyleWarning, StyleWarningKind};

/// Highest sub-spec number whose wiring is live in the renderer.
///
/// A schema leaf whose `sub_spec` is **greater than** this constant still
/// emits a [`StyleWarningKind::KnownButInactive`] warning; leaves whose
/// `sub_spec` is `<=` this constant are considered wired and stay silent.
///
/// Advance this constant whenever a future sub-spec wires its keys.
pub const ACTIVE_STYLE_WIRING_SUB_SPEC: u8 = 2;

/// Parse a `serde_json::Value` representing the value at the `style:` key.
///
/// Returns `(StyleFrontmatter::default(), vec![])` for `Value::Null`.
///
/// ## Errors
///
/// - `StyleParseError::Structure` — wrong JSON type at a known path
///   (e.g. `top-margin: "2ch"`).
/// - `StyleParseError::InvalidLength` — string that can't be parsed as a
///   horizontal length (e.g. `"2px"`, `"-2"`).
/// - `StyleParseError::InvalidPercent` — percent value outside `0.0..=100.0`.
/// - `StyleParseError::InvalidColor` — color string that doesn't match any
///   accepted form.
/// - `StyleParseError::Serde` — fallback for serde-level failures not caught
///   by the pre-validator (currently: bad `background` enum variant, wrong
///   shape for `code`).
pub fn from_json_value(
    value: &Value,
) -> Result<(StyleFrontmatter, Vec<StyleWarning>), StyleParseError> {
    if value.is_null() {
        return Ok((StyleFrontmatter::default(), Vec::new()));
    }

    // Pass 1: collect schema-validation warnings (unknown keys, snake aliases).
    let mut warnings = walker::walk(value);

    // Pass 2a: typed pre-validation. Catches typed-leaf failures with full
    // path-aware diagnostics before serde sees them.
    pre_validate(value)?;

    // Pass 2b: typed deserialize. Serde's `alias` accepts both spellings, so
    // the value is parsed in its original form — no rewriting needed.
    let parsed: StyleFrontmatter = serde_json::from_value(value.clone())?;

    // Pass 3: emit `KnownButInactive` for every leaf that is in the schema.
    annotate_known_but_inactive(value, &mut warnings);

    Ok((parsed, warnings))
}

/// Walk the raw style value and validate every known typed leaf. Returns
/// the first typed error encountered; serde-level failures (alignment
/// strings, enums) are deferred to `serde_json::from_value` so this stays
/// focused on the four named typed variants.
fn pre_validate(value: &Value) -> Result<(), StyleParseError> {
    pre_validate_inner(value, "")
}

fn pre_validate_inner(value: &Value, canonical_path: &str) -> Result<(), StyleParseError> {
    let Value::Object(map) = value else {
        // A non-object at the root is reported by serde as a structural
        // error. We can't form a "style." path here without ambiguity, so
        // let serde handle it.
        return Ok(());
    };
    for (key, child) in map {
        let canonical_segment = kebabify(key);
        let canonical_child = join_path(canonical_path, &canonical_segment);

        if let Some(leaf) = leaf_for_canonical(&canonical_child) {
            validate_leaf(leaf, child, &canonical_child)?;
            continue;
        }
        if is_canonical_container(&canonical_child) {
            pre_validate_inner(child, &canonical_child)?;
        }
        // Unknown keys are reported by the walker; pre_validate skips them.
    }
    Ok(())
}

fn validate_leaf(
    leaf: &SchemaLeaf,
    value: &Value,
    canonical_path: &str,
) -> Result<(), StyleParseError> {
    match leaf.leaf_type {
        LeafType::HorizontalLength => validate_horizontal_length(value, canonical_path),
        LeafType::RowCount => validate_row_count(value, canonical_path),
        LeafType::Color => validate_color(value, canonical_path),
        // Alignment, BackgroundEnum, StringValue, OpaqueValue — let serde
        // handle these. Bad alignment / background variants surface as Serde
        // errors; StringValue and OpaqueValue are permissive by design.
        LeafType::Alignment
        | LeafType::BackgroundEnum
        | LeafType::StringValue
        | LeafType::OpaqueValue => Ok(()),
    }
}

fn validate_horizontal_length(value: &Value, canonical_path: &str) -> Result<(), StyleParseError> {
    match value {
        Value::Null => Ok(()),
        Value::Number(n) => {
            // u32 in range is fine; everything else is an InvalidLength.
            if n.as_u64().is_some_and(|v| v <= u32::MAX as u64) {
                Ok(())
            } else {
                Err(StyleParseError::InvalidLength {
                    path: format!("style.{}", canonical_path),
                    raw: n.to_string(),
                    reason: "integer length must fit in 0..=4294967295",
                })
            }
        }
        Value::String(s) => match parse_horizontal_typed(s) {
            Ok(_) => Ok(()),
            Err(HorizontalLengthError::PercentOutOfRange(v)) => {
                Err(StyleParseError::InvalidPercent {
                    path: format!("style.{}", canonical_path),
                    value: v,
                })
            }
            Err(other) => Err(StyleParseError::InvalidLength {
                path: format!("style.{}", canonical_path),
                raw: s.clone(),
                reason: other.as_static_reason(),
            }),
        },
        other => Err(StyleParseError::Structure {
            path: format!("style.{}", canonical_path),
            expected: "string or non-negative integer",
            actual: json_type_name(other).to_string(),
        }),
    }
}

fn validate_row_count(value: &Value, canonical_path: &str) -> Result<(), StyleParseError> {
    match value {
        Value::Null => Ok(()),
        Value::Number(n) => {
            if n.as_u64().is_some_and(|v| v <= u16::MAX as u64) {
                Ok(())
            } else {
                Err(StyleParseError::Structure {
                    path: format!("style.{}", canonical_path),
                    expected: "integer in 0..=65535",
                    actual: format!("number `{}`", n),
                })
            }
        }
        other => Err(StyleParseError::Structure {
            path: format!("style.{}", canonical_path),
            expected: "integer",
            actual: json_type_name(other).to_string(),
        }),
    }
}

fn validate_color(value: &Value, canonical_path: &str) -> Result<(), StyleParseError> {
    match value {
        Value::Null => Ok(()),
        Value::String(s) => match color::parse(s) {
            Ok(_) => Ok(()),
            Err(reason) => Err(StyleParseError::InvalidColor {
                path: format!("style.{}", canonical_path),
                raw: s.clone(),
                reason,
            }),
        },
        other => Err(StyleParseError::Structure {
            path: format!("style.{}", canonical_path),
            expected: "string",
            actual: json_type_name(other).to_string(),
        }),
    }
}

fn json_type_name(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "bool",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

/// Walk the raw style value and emit `KnownButInactive` for every leaf that
/// *is* in the schema but whose wiring sub-spec is greater than 1.
///
/// Per-segment kebabify mirrors the walker, so a document like
/// `style.block_quote.max_width` still produces a single `KnownButInactive`
/// at the canonical path `style.block-quote.max-width`.
fn annotate_known_but_inactive(value: &Value, warnings: &mut Vec<StyleWarning>) {
    annotate_inner(value, "", warnings);
}

fn annotate_inner(value: &Value, canonical_path: &str, warnings: &mut Vec<StyleWarning>) {
    let Value::Object(map) = value else {
        return;
    };
    for (key, child) in map {
        let canonical_segment = kebabify(key);
        let canonical_child = join_path(canonical_path, &canonical_segment);

        if let Some(leaf) = leaf_for_canonical(&canonical_child) {
            if leaf.sub_spec > ACTIVE_STYLE_WIRING_SUB_SPEC {
                warnings.push(StyleWarning::new(
                    format!("style.{}", leaf.canonical),
                    StyleWarningKind::KnownButInactive {
                        sub_spec: leaf.sub_spec,
                    },
                ));
            }
            // Leaves don't recurse.
            continue;
        }

        // Containers and unknowns: recurse if it's a container, ignore
        // unknown (walker already reported them).
        if is_canonical_container(&canonical_child) && child.is_object() {
            annotate_inner(child, &canonical_child, warnings);
        }
    }
}

use crate::markdown::Frontmatter;

/// Parse the `style:` value from a `Frontmatter`. Returns
/// `(StyleFrontmatter::default(), vec![])` when no `style:` key is present.
///
/// ## Errors
///
/// Propagates any `StyleParseError` returned by [`from_json_value`].
pub fn from_frontmatter(
    fm: &Frontmatter,
) -> Result<(StyleFrontmatter, Vec<StyleWarning>), StyleParseError> {
    match fm.as_map().get("style") {
        Some(value) => from_json_value(value),
        None => Ok((StyleFrontmatter::default(), Vec::new())),
    }
}

/// Promote schema-validation warnings (`UnknownKey`, `Deprecated`) to errors.
///
/// `KnownButInactive` warnings are deliberately ignored so a strict caller
/// does not fail on a forward-compatible document.
///
/// ## Errors
///
/// Returns `StyleParseError::Strict` when any `UnknownKey` or `Deprecated`
/// warnings are present in the parsed result.
pub fn into_strict(
    parsed: (StyleFrontmatter, Vec<StyleWarning>),
) -> Result<StyleFrontmatter, StyleParseError> {
    let (style, warnings) = parsed;
    let schema: Vec<StyleWarning> = warnings
        .into_iter()
        .filter(|w| w.is_schema_issue())
        .collect();
    if schema.is_empty() {
        Ok(style)
    } else {
        Err(StyleParseError::Strict { warnings: schema })
    }
}

// Re-export the SCHEMA so test modules that already imported `descriptor::SCHEMA`
// through this file continue to compile. Anything else from `descriptor` is
// imported directly at the top of this file.
pub use descriptor::SCHEMA;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::Frontmatter;
    use renderable::layout::{Alignment, Length};
    use serde_json::json;

    #[test]
    fn null_yields_default() {
        let (s, w) = from_json_value(&Value::Null).unwrap();
        assert_eq!(s, StyleFrontmatter::default());
        assert!(w.is_empty());
    }

    #[test]
    fn empty_object_yields_default() {
        let (s, w) = from_json_value(&json!({})).unwrap();
        assert_eq!(s, StyleFrontmatter::default());
        assert!(w.is_empty());
    }

    #[test]
    fn page_left_margin_parses() {
        let (s, w) = from_json_value(&json!({"page": {"left-margin": "2ch"}})).unwrap();
        let schema_warnings: Vec<_> = w.iter().filter(|w| w.is_schema_issue()).collect();
        assert!(schema_warnings.is_empty());
        assert_eq!(s.page.unwrap().left_margin, Some(Length::Ch(2)));
    }

    #[test]
    fn unknown_key_produces_warning_but_parse_succeeds() {
        let (s, w) = from_json_value(&json!({"page": {"lft-margin": "2ch"}})).unwrap();
        assert_eq!(w.len(), 1);
        assert_eq!(w[0].path, "style.page.lft-margin");
        assert!(s.page.is_some());
    }

    #[test]
    fn deprecated_alias_produces_warning_but_parse_succeeds() {
        let (s, w) = from_json_value(&json!({"page": {"left_margin": "2ch"}})).unwrap();
        let schema_warnings: Vec<_> = w.iter().filter(|w| w.is_schema_issue()).collect();
        assert_eq!(schema_warnings.len(), 1);
        assert!(matches!(
            schema_warnings[0].kind,
            StyleWarningKind::Deprecated { .. }
        ));
        assert_eq!(s.page.unwrap().left_margin, Some(Length::Ch(2)));
    }

    #[test]
    fn invalid_length_returns_typed_variant() {
        let err = from_json_value(&json!({"page": {"left-margin": "2px"}})).unwrap_err();
        match err {
            StyleParseError::InvalidLength { path, raw, reason } => {
                assert_eq!(path, "style.page.left-margin");
                assert_eq!(raw, "2px");
                assert!(reason.contains("unsupported unit"), "got: {}", reason);
            }
            other => panic!("expected InvalidLength, got {:?}", other),
        }
    }

    #[test]
    fn invalid_percent_returns_typed_variant() {
        let err = from_json_value(&json!({"page": {"max-width": "150%"}})).unwrap_err();
        match err {
            StyleParseError::InvalidPercent { path, value } => {
                assert_eq!(path, "style.page.max-width");
                assert_eq!(value, 150.0);
            }
            other => panic!("expected InvalidPercent, got {:?}", other),
        }
    }

    #[test]
    fn invalid_color_returns_typed_variant() {
        let err = from_json_value(&json!({"page": {"color": "puce"}})).unwrap_err();
        match err {
            StyleParseError::InvalidColor { path, raw, reason } => {
                assert_eq!(path, "style.page.color");
                assert_eq!(raw, "puce");
                assert!(!reason.is_empty());
            }
            other => panic!("expected InvalidColor, got {:?}", other),
        }
    }

    #[test]
    fn vertical_margin_string_returns_structure() {
        // Spec test #4: `top-margin: "2ch"` must surface as Structure with
        // the canonical path.
        let err = from_json_value(&json!({"page": {"top-margin": "2ch"}})).unwrap_err();
        match err {
            StyleParseError::Structure {
                path,
                expected,
                actual,
            } => {
                assert_eq!(path, "style.page.top-margin");
                assert_eq!(expected, "integer");
                assert_eq!(actual, "string");
            }
            other => panic!("expected Structure, got {:?}", other),
        }
    }

    #[test]
    fn vertical_margin_integer_succeeds() {
        let (s, _) = from_json_value(&json!({"page": {"top-margin": 1}})).unwrap();
        assert_eq!(s.page.unwrap().top_margin, Some(1));
    }

    #[test]
    fn non_object_color_returns_structure() {
        let err = from_json_value(&json!({"page": {"color": 42}})).unwrap_err();
        match err {
            StyleParseError::Structure {
                path,
                expected,
                actual,
            } => {
                assert_eq!(path, "style.page.color");
                assert_eq!(expected, "string");
                assert_eq!(actual, "number");
            }
            other => panic!("expected Structure, got {:?}", other),
        }
    }

    #[test]
    fn typed_error_uses_canonical_path_even_when_user_wrote_alias() {
        // The user spelled `max_width` (deprecated alias); the typed error's
        // path is the canonical kebab form.
        let err = from_json_value(&json!({"page": {"max_width": "150%"}})).unwrap_err();
        match err {
            StyleParseError::InvalidPercent { path, value } => {
                assert_eq!(path, "style.page.max-width");
                assert_eq!(value, 150.0);
            }
            other => panic!("expected InvalidPercent, got {:?}", other),
        }
    }

    #[test]
    fn typed_error_path_handles_nested_local_style() {
        let err = from_json_value(&json!({
            "hyperlinks": {"local-style": {"max-width": "150%"}}
        }))
        .unwrap_err();
        match err {
            StyleParseError::InvalidPercent { path, .. } => {
                assert_eq!(path, "style.hyperlinks.local-style.max-width");
            }
            other => panic!("expected InvalidPercent, got {:?}", other),
        }
    }

    #[test]
    fn page_keys_suppress_known_but_inactive() {
        // Sub-spec #2 wires page-level keys, so they no longer emit
        // `KnownButInactive` warnings.
        let (_, w) = from_json_value(&json!({
            "page": {"left-margin": "2ch"}
        }))
        .unwrap();
        let inactive: Vec<_> = w
            .iter()
            .filter(|w| matches!(w.kind, StyleWarningKind::KnownButInactive { .. }))
            .collect();
        assert!(inactive.is_empty(), "page leaves should be wired: {:?}", w);
    }

    #[test]
    fn deprecated_alias_for_wired_page_key_still_warns_only_deprecated() {
        let (_, w) = from_json_value(&json!({
            "page": {"left_margin": "2ch"}
        }))
        .unwrap();
        let inactive: Vec<_> = w
            .iter()
            .filter(|w| matches!(w.kind, StyleWarningKind::KnownButInactive { .. }))
            .collect();
        assert!(
            inactive.is_empty(),
            "wired page leaves should not emit inactive"
        );

        let deprecated: Vec<_> = w
            .iter()
            .filter(|w| matches!(w.kind, StyleWarningKind::Deprecated { .. }))
            .collect();
        assert_eq!(deprecated.len(), 1);
        assert_eq!(deprecated[0].path, "style.page.left_margin");
    }

    #[test]
    fn future_phase_key_still_emits_known_but_inactive() {
        // `table.alignment` is wired in sub-spec #3 — currently inactive.
        let (_, w) = from_json_value(&json!({
            "table": {"alignment": "right"}
        }))
        .unwrap();
        let inactive: Vec<_> = w
            .iter()
            .filter(|w| matches!(w.kind, StyleWarningKind::KnownButInactive { .. }))
            .collect();
        assert_eq!(inactive.len(), 1);
        assert_eq!(inactive[0].path, "style.table.alignment");
        assert!(matches!(
            inactive[0].kind,
            StyleWarningKind::KnownButInactive { sub_spec: 3 }
        ));
    }

    #[test]
    fn nested_alias_emits_two_deprecated_and_one_known_but_inactive() {
        // Spec test #8 + Finding 1 regression: `block_quote.max_width` must
        // emit exactly two `Deprecated` warnings plus a single
        // `KnownButInactive` at the canonical path.
        let (_, w) = from_json_value(&json!({
            "block_quote": {"max_width": "50%"}
        }))
        .unwrap();
        let deprecated: Vec<_> = w
            .iter()
            .filter(|w| matches!(w.kind, StyleWarningKind::Deprecated { .. }))
            .collect();
        let inactive: Vec<_> = w
            .iter()
            .filter(|w| matches!(w.kind, StyleWarningKind::KnownButInactive { .. }))
            .collect();
        assert_eq!(deprecated.len(), 2, "got: {:?}", deprecated);
        assert_eq!(inactive.len(), 1);
        assert_eq!(inactive[0].path, "style.block-quote.max-width");
    }

    #[test]
    fn test_doc_all_known_but_inactive() {
        let v = json!({
            "page":  {"left-margin": "2ch", "right-margin": "4ch",
                       "top-margin": 1, "bottom-margin": 0},
            "table": {"alignment": "right", "max-width": "50%"},
            "ol":    {"alignment": "right"},
            "ul":    {"alignment": "left", "left-margin": "4ch", "max-width": "40"}
        });
        let (_, w) = from_json_value(&v).unwrap();
        let schema: Vec<_> = w.iter().filter(|w| w.is_schema_issue()).collect();
        assert!(
            schema.is_empty(),
            "should not produce schema warnings: {:?}",
            schema
        );
        let inactive: Vec<_> = w
            .iter()
            .filter(|w| matches!(w.kind, StyleWarningKind::KnownButInactive { .. }))
            .collect();
        // Sub-spec #2 wires the 4 page leaves; the remaining 2 table + 1 ol
        // + 3 ul = 6 are still future-phase.
        assert_eq!(inactive.len(), 6, "got {:?}", inactive);
    }

    #[test]
    fn matches_test_doc_acceptance_criteria() {
        let v = json!({
            "page": {
                "left-margin": "2ch",
                "right-margin": "4ch",
                "top-margin": 1,
                "bottom-margin": 0
            },
            "table": {
                "alignment": "right",
                "max-width": "50%"
            },
            "ol": {"alignment": "right"},
            "ul": {
                "alignment": "left",
                "left-margin": "4ch",
                "max-width": "40"
            }
        });

        let (s, _w) = from_json_value(&v).unwrap();
        let p = s.page.expect("page");
        assert_eq!(p.left_margin, Some(Length::Ch(2)));
        assert_eq!(p.right_margin, Some(Length::Ch(4)));
        assert_eq!(p.top_margin, Some(1));
        assert_eq!(p.bottom_margin, Some(0));

        let t = s.table.expect("table");
        assert_eq!(t.common.alignment, Some(Alignment::Right));
        assert_eq!(t.common.max_width, Some(Length::Percent(50.0)));

        let ol = s.ol.expect("ol");
        assert_eq!(ol.common.alignment, Some(Alignment::Right));

        let ul = s.ul.expect("ul");
        assert_eq!(ul.common.alignment, Some(Alignment::Left));
        assert_eq!(ul.left_margin, Some(Length::Ch(4)));
        assert_eq!(ul.common.max_width, Some(Length::Ch(40)));
    }

    #[test]
    fn from_frontmatter_no_style_key_yields_default() {
        let fm = Frontmatter::new();
        let (s, w) = from_frontmatter(&fm).unwrap();
        assert_eq!(s, StyleFrontmatter::default());
        assert!(w.is_empty());
    }

    #[test]
    fn from_frontmatter_with_style_key() {
        let mut fm = Frontmatter::new();
        fm.insert("style", json!({"page": {"left-margin": "2ch"}}))
            .unwrap();
        let (s, _w) = from_frontmatter(&fm).unwrap();
        assert!(s.page.is_some());
    }

    #[test]
    fn into_strict_passes_clean_parse() {
        let parsed = from_json_value(&json!({"page": {"left-margin": "2ch"}})).unwrap();
        let s = into_strict(parsed).unwrap();
        assert!(s.page.is_some());
    }

    #[test]
    fn into_strict_fails_on_unknown_key() {
        let parsed = from_json_value(&json!({"page": {"lft-margin": "2ch"}})).unwrap();
        match into_strict(parsed) {
            Err(StyleParseError::Strict { warnings }) => {
                assert_eq!(warnings.len(), 1);
                assert_eq!(warnings[0].path, "style.page.lft-margin");
            }
            other => panic!("expected Strict error, got {:?}", other),
        }
    }

    #[test]
    fn into_strict_fails_on_deprecated_alias() {
        let parsed = from_json_value(&json!({"page": {"left_margin": "2ch"}})).unwrap();
        assert!(matches!(
            into_strict(parsed),
            Err(StyleParseError::Strict { .. })
        ));
    }

    #[test]
    fn into_strict_ignores_known_but_inactive() {
        let parsed = from_json_value(&json!({
            "table": {"alignment": "right", "max-width": "50%"}
        }))
        .unwrap();
        assert!(into_strict(parsed).is_ok());
    }
}
