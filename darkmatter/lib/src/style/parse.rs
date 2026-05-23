//! Parse entry points for the style frontmatter.

use serde_json::Value;

use crate::style::descriptor::SCHEMA;
use crate::style::error::StyleParseError;
use crate::style::schema::StyleFrontmatter;
use crate::style::walker;
use crate::style::warning::{StyleWarning, StyleWarningKind};

/// Parse a `serde_json::Value` representing the value at the `style:` key.
///
/// Returns `(StyleFrontmatter::default(), vec![])` for `Value::Null`.
///
/// ## Errors
///
/// `StyleParseError::Serde` on any typed-deserialization failure
/// (structure/length/color/alignment value errors).
pub fn from_json_value(
    value: &Value,
) -> Result<(StyleFrontmatter, Vec<StyleWarning>), StyleParseError> {
    if value.is_null() {
        return Ok((StyleFrontmatter::default(), Vec::new()));
    }

    // Pass 1: collect schema-validation warnings.
    let mut warnings = walker::walk(value);

    // Pass 2: typed deserialize. Serde's `alias` accepts both spellings, so
    // the value is parsed in its original form — no rewriting needed.
    let parsed: StyleFrontmatter = serde_json::from_value(value.clone())?;

    // Pass 3: emit `KnownButInactive` for every leaf that is in the schema.
    annotate_known_but_inactive(value, &mut warnings);

    Ok((parsed, warnings))
}

/// Walk the raw style value a second time and emit `KnownButInactive` for
/// every leaf that *is* in the schema but whose wiring sub-spec is greater
/// than 1. We re-walk the raw value (rather than the typed
/// `StyleFrontmatter`) because the typed walk would require visitor code per
/// bucket — the raw walk is one function shared by every leaf.
fn annotate_known_but_inactive(value: &Value, warnings: &mut Vec<StyleWarning>) {
    annotate_inner(value, "", warnings);
}

fn annotate_inner(value: &Value, path: &str, warnings: &mut Vec<StyleWarning>) {
    let Value::Object(map) = value else {
        return;
    };
    for (key, child) in map {
        let child_path = if path.is_empty() {
            key.clone()
        } else {
            format!("{}.{}", path, key)
        };

        if let Some(leaf) = SCHEMA
            .iter()
            .find(|l| l.canonical == child_path || l.alias == Some(child_path.as_str()))
        {
            // Use the canonical path in the warning regardless of which
            // spelling the user wrote — the wiring is tracked against
            // canonical paths.
            warnings.push(StyleWarning::new(
                format!("style.{}", leaf.canonical),
                StyleWarningKind::KnownButInactive {
                    sub_spec: leaf.sub_spec,
                },
            ));
            // Leaves don't recurse.
            continue;
        }

        // Containers and unknowns: recurse if it's a container, ignore
        // unknown (pass 1 already reported them).
        if child.is_object() {
            annotate_inner(child, &child_path, warnings);
        }
    }
}

use crate::markdown::Frontmatter;

/// Parse the `style:` value from a `Frontmatter`. Returns
/// `(StyleFrontmatter::default(), vec![])` when no `style:` key is present.
///
/// ## Errors
///
/// Propagates any `StyleParseError` returned by `from_json_value`.
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
    let schema: Vec<StyleWarning> =
        warnings.into_iter().filter(|w| w.is_schema_issue()).collect();
    if schema.is_empty() {
        Ok(style)
    } else {
        Err(StyleParseError::Strict { warnings: schema })
    }
}

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
        // Pass 3 emits KnownButInactive for the known leaf; no schema issues.
        let schema_warnings: Vec<_> = w.iter().filter(|w| w.is_schema_issue()).collect();
        assert!(schema_warnings.is_empty());
        assert_eq!(s.page.unwrap().left_margin, Some(Length::Ch(2)));
    }

    #[test]
    fn unknown_key_produces_warning_but_parse_succeeds() {
        let (s, w) = from_json_value(&json!({"page": {"lft-margin": "2ch"}})).unwrap();
        // The unknown key is dropped by serde; warning is recorded.
        assert_eq!(w.len(), 1);
        assert_eq!(w[0].path, "style.page.lft-margin");
        assert!(s.page.is_some()); // page bucket still materialized (empty).
    }

    #[test]
    fn deprecated_alias_produces_warning_but_parse_succeeds() {
        let (s, w) = from_json_value(&json!({"page": {"left_margin": "2ch"}})).unwrap();
        // Pass 1 emits Deprecated; pass 3 emits KnownButInactive — 2 total.
        // Filter to schema-validation warnings to assert on the Deprecated one.
        let schema_warnings: Vec<_> = w.iter().filter(|w| w.is_schema_issue()).collect();
        assert_eq!(schema_warnings.len(), 1);
        assert!(matches!(
            schema_warnings[0].kind,
            crate::style::warning::StyleWarningKind::Deprecated { .. }
        ));
        // Value still parsed because of serde alias.
        assert_eq!(s.page.unwrap().left_margin, Some(Length::Ch(2)));
    }

    #[test]
    fn type_error_short_circuits() {
        let err = from_json_value(&json!({"page": {"left-margin": "2px"}})).unwrap_err();
        // The unknown unit should surface as a Serde error.
        let msg = err.to_string();
        assert!(msg.contains("unsupported unit"));
    }

    #[test]
    fn known_but_inactive_per_field() {
        let (_, w) = from_json_value(&json!({
            "page": {"left-margin": "2ch"}
        }))
        .unwrap();
        let inactive: Vec<_> = w
            .iter()
            .filter(|w| matches!(w.kind, crate::style::warning::StyleWarningKind::KnownButInactive { .. }))
            .collect();
        assert_eq!(inactive.len(), 1);
        assert_eq!(inactive[0].path, "style.page.left-margin");
        assert!(matches!(
            inactive[0].kind,
            crate::style::warning::StyleWarningKind::KnownButInactive { sub_spec: 2 }
        ));
    }

    #[test]
    fn deprecated_alias_uses_canonical_in_known_but_inactive() {
        let (_, w) = from_json_value(&json!({
            "page": {"left_margin": "2ch"}
        }))
        .unwrap();
        let inactive: Vec<_> = w
            .iter()
            .filter(|w| matches!(w.kind, crate::style::warning::StyleWarningKind::KnownButInactive { .. }))
            .collect();
        assert_eq!(inactive.len(), 1);
        // KnownButInactive uses the canonical name even when the user wrote
        // the alias.
        assert_eq!(inactive[0].path, "style.page.left-margin");
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
        assert!(schema.is_empty(), "should not produce schema warnings: {:?}", schema);
        let inactive: Vec<_> = w
            .iter()
            .filter(|w| matches!(w.kind, crate::style::warning::StyleWarningKind::KnownButInactive { .. }))
            .collect();
        // 4 page + 2 table + 1 ol + 3 ul = 10 leaves.
        assert_eq!(inactive.len(), 10, "got {:?}", inactive);
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
        fm.insert("style", json!({"page": {"left-margin": "2ch"}})).unwrap();
        let (s, _w) = from_frontmatter(&fm).unwrap();
        assert!(s.page.is_some());
    }

    #[test]
    fn into_strict_passes_clean_parse() {
        let parsed = from_json_value(&json!({"page": {"left-margin": "2ch"}})).unwrap();
        // Only KnownButInactive warnings; strict should succeed.
        let s = into_strict(parsed).unwrap();
        assert!(s.page.is_some());
    }

    #[test]
    fn into_strict_fails_on_unknown_key() {
        let parsed =
            from_json_value(&json!({"page": {"lft-margin": "2ch"}})).unwrap();
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
        let parsed =
            from_json_value(&json!({"page": {"left_margin": "2ch"}})).unwrap();
        assert!(matches!(
            into_strict(parsed),
            Err(StyleParseError::Strict { .. })
        ));
    }

    #[test]
    fn into_strict_ignores_known_but_inactive() {
        // Document fully valid; every key emits KnownButInactive but strict
        // must still succeed.
        let parsed = from_json_value(&json!({
            "table": {"alignment": "right", "max-width": "50%"}
        }))
        .unwrap();
        assert!(into_strict(parsed).is_ok());
    }
}
