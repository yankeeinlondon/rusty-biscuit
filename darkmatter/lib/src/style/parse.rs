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
use crate::style::warning::{StyleSpan, StyleWarning, StyleWarningKind};

/// Highest sub-spec number whose wiring is live in the renderer.
///
/// A schema leaf whose `sub_spec` is **greater than** this constant still
/// emits a [`StyleWarningKind::KnownButInactive`] warning; leaves whose
/// `sub_spec` is `<=` this constant are considered wired and stay silent.
///
/// Advance this constant whenever a future sub-spec wires its keys.
pub const ACTIVE_STYLE_WIRING_SUB_SPEC: u8 = 8;

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

    // Pass 4: detect deprecated typed-enum spellings that serde's `alias`
    // attribute accepts silently. Currently this covers
    // `style.hr.alignment: centered`, which is accepted as an alias for
    // `center` but must surface a `Deprecated` warning so `--strict-style`
    // rejects it.
    scan_deprecated_enum_aliases(value, &mut warnings);

    Ok((parsed, warnings))
}

/// Detect typed-enum aliases that serde's `#[serde(alias = ...)]` accepts
/// without surfacing the spelling used by the document.
///
/// Today this covers a single case: `style.hr.alignment: "centered"` is
/// accepted as an alias for `"center"`, but Design Decision #10 of sub-spec #6
/// requires a `Deprecated { replacement: "center" }` warning so strict mode
/// can reject the legacy spelling. The schema walker only handles
/// snake-case → kebab-case aliasing, so this special case lives here.
fn scan_deprecated_enum_aliases(value: &Value, warnings: &mut Vec<StyleWarning>) {
    let Value::Object(root) = value else {
        return;
    };
    let Some(Value::Object(hr)) = root.get("hr") else {
        return;
    };
    if let Some(Value::String(s)) = hr.get("alignment")
        && s == "centered"
    {
        warnings.push(StyleWarning::new(
            "style.hr.alignment",
            StyleWarningKind::Deprecated {
                replacement: "center".into(),
            },
        ));
    }
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
        LeafType::WidthOrMode => validate_width_or_mode(value, canonical_path),
        LeafType::RowCount => validate_row_count(value, canonical_path),
        LeafType::Color => validate_color(value, canonical_path),
        // Alignment, BackgroundEnum, StringValue, OpaqueValue — let serde
        // handle these. Bad alignment / background variants surface as Serde
        // errors; StringValue and OpaqueValue are permissive by design.
        LeafType::Alignment
        | LeafType::BackgroundEnum
        | LeafType::StringValue
        | LeafType::OpaqueValue
        | LeafType::HrKind
        | LeafType::HrWeight
        | LeafType::HrAlignment
        | LeafType::CompoundStyle
        | LeafType::WordWrap => Ok(()),
    }
}

fn validate_width_or_mode(value: &Value, canonical_path: &str) -> Result<(), StyleParseError> {
    match value {
        Value::Null => Ok(()),
        Value::String(s) => match s.trim().to_ascii_lowercase().as_str() {
            "auto" | "fit-content" | "fit_content" => Ok(()),
            _ => validate_horizontal_length(value, canonical_path),
        },
        _ => validate_horizontal_length(value, canonical_path),
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
        Some(value) => {
            let (style, mut warnings) = from_json_value(value)?;
            // When the original frontmatter text is available, range each
            // warning at the key it flags (R-5 Priority 5). Line/column are
            // relative to the raw YAML block (line 1 = first YAML line), so a
            // consumer with the block's source offset (DMLS) can project them.
            if let Some(raw) = fm.raw_source() {
                let positions = build_yaml_position_map(raw);
                for warning in &mut warnings {
                    if let Some(span) = positions.get(&warning.path) {
                        warning.source_span = Some(span.clone());
                    }
                }
            }
            Ok((style, warnings))
        }
        None => Ok((StyleFrontmatter::default(), Vec::new())),
    }
}

/// Maps each dotted YAML key path to the source span of its key token, over
/// the raw frontmatter text (R-4 item 6).
///
/// Keys are the full dotted path using the spellings the author wrote (e.g.
/// `style.page.left_margin`), so a [`StyleWarning::path`](StyleWarning) is a
/// direct lookup. Coordinates are 1-based and relative to `yaml` (line 1 = the
/// first line of `yaml`); `length` is the key token's length in characters.
///
/// ## Notes
///
/// The scan understands block mappings only. Keys inside a flow mapping
/// (`page: { left-margin: 2ch }`) are not indexed, so a warning under one
/// keeps `source_span = None` — a graceful degradation, never a wrong span.
pub fn build_yaml_position_map(yaml: &str) -> indexmap::IndexMap<String, StyleSpan> {
    let mut out = indexmap::IndexMap::new();
    // Ancestor keys with their indentation, innermost last.
    let mut stack: Vec<(usize, String)> = Vec::new();

    for (idx, line) in yaml.lines().enumerate() {
        let indent = line.chars().take_while(|c| *c == ' ').count();
        let content = &line[line.len() - line.trim_start().len()..];
        // Blank lines, comments, and sequence items carry no mapping key.
        if content.is_empty() || content.starts_with('#') || content.starts_with('-') {
            continue;
        }
        let Some(colon) = content.find(':') else {
            continue;
        };
        let raw_key = content[..colon].trim();
        if raw_key.is_empty() {
            continue;
        }
        let key = strip_yaml_quotes(raw_key);

        // Drop ancestors at the same or deeper indentation before nesting.
        while matches!(stack.last(), Some((top, _)) if *top >= indent) {
            stack.pop();
        }

        let dotted = if stack.is_empty() {
            key.to_string()
        } else {
            let mut path: String = stack
                .iter()
                .map(|(_, k)| k.as_str())
                .collect::<Vec<_>>()
                .join(".");
            path.push('.');
            path.push_str(key);
            path
        };

        out.insert(
            dotted,
            StyleSpan {
                line: idx as u32 + 1,
                column: indent as u32 + 1,
                length: raw_key.chars().count() as u32,
            },
        );
        stack.push((indent, key.to_string()));
    }

    out
}

/// Strips a single pair of matching surrounding `"` or `'` quotes.
fn strip_yaml_quotes(raw: &str) -> &str {
    for quote in ['"', '\''] {
        if raw.len() >= 2 && raw.starts_with(quote) && raw.ends_with(quote) {
            return &raw[1..raw.len() - 1];
        }
    }
    raw
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
    fn sub_spec_7_keys_no_longer_emit_known_but_inactive() {
        // After sub-spec #7 lands, all its keys are wired and silent.
        let cases = [
            json!({"hyperlinks": {"local-style": {"color": "red-500"}}}),
            json!({"page": {"stylesheet": "https://example.com/style.css"}}),
            json!({"page": {"meta": {"description": "A page"}}}),
            json!({"page": {"code": {"theme": "dracula"}}}),
            json!({"hyperlinks": {"width": "40ch"}}),
            json!({"images": {"local-style": {"color": "blue-500"}}}),
        ];
        for v in &cases {
            let (_, w) = from_json_value(v).unwrap();
            let inactive: Vec<_> = w
                .iter()
                .filter(|w| matches!(w.kind, StyleWarningKind::KnownButInactive { .. }))
                .collect();
            assert!(
                inactive.is_empty(),
                "sub-spec #7 key in {:?} should not emit KnownButInactive; got: {:?}",
                v,
                inactive,
            );
        }
    }

    #[test]
    fn sub_spec_3_keys_no_longer_emit_known_but_inactive() {
        // Phase 4 of sub-spec #3: table/images/block-quote width, max-width,
        // and alignment are now wired. They must not emit KnownButInactive.
        let cases = [
            json!({"table": {"width": "40ch"}}),
            json!({"table": {"max-width": "50%"}}),
            json!({"table": {"alignment": "right"}}),
            json!({"images": {"width": "40ch"}}),
            json!({"images": {"max-width": "50%"}}),
            json!({"images": {"alignment": "center"}}),
            json!({"block-quote": {"width": "40ch"}}),
            json!({"block-quote": {"max-width": "50%"}}),
            json!({"block-quote": {"alignment": "left"}}),
        ];
        for v in &cases {
            let (_, w) = from_json_value(v).unwrap();
            let inactive: Vec<_> = w
                .iter()
                .filter(|w| matches!(w.kind, StyleWarningKind::KnownButInactive { .. }))
                .collect();
            assert!(
                inactive.is_empty(),
                "sub-spec #3 key in {:?} should not emit KnownButInactive; got: {:?}",
                v,
                inactive,
            );
        }
    }

    #[test]
    fn sub_spec_6_keys_no_longer_emit_known_but_inactive() {
        // Phase 3 of sub-spec #6: HR keys are now wired. They must not emit
        // KnownButInactive.
        let cases = [
            json!({"hr": {"color": "red-500"}}),
            json!({"hr": {"bg-color": "blue-200"}}),
            json!({"hr": {"kind": "waves"}}),
            json!({"hr": {"weight": "thick"}}),
            json!({"hr": {"width": "50%"}}),
            json!({"hr": {"max-width": "20ch"}}),
            json!({"hr": {"alignment": "center"}}),
        ];
        for v in &cases {
            let (_, w) = from_json_value(v).unwrap();
            let inactive: Vec<_> = w
                .iter()
                .filter(|w| matches!(w.kind, StyleWarningKind::KnownButInactive { .. }))
                .collect();
            assert!(
                inactive.is_empty(),
                "sub-spec #6 key in {:?} should not emit KnownButInactive; got: {:?}",
                v,
                inactive,
            );
        }
    }

    #[test]
    fn no_known_but_inactive_for_any_valid_v1_key() {
        // After sub-spec #7, every valid v1 schema key is wired.
        let v = json!({
            "page": {
                "left-margin": "2ch", "right-margin": "4ch",
                "top-margin": 1, "bottom-margin": 0,
                "max-width": "80%", "alignment": "center",
                "background": "subtle",
                "stylesheet": "https://example.com/style.css",
                "meta": { "description": "Test" },
                "code": { "theme": "dracula" }
            },
            "table": { "alignment": "right", "max-width": "50%", "color": "red-500" },
            "images": { "width": "40ch", "alignment": "center", "local-style": { "color": "blue-500" } },
            "block-quote": { "max-width": "60ch", "alignment": "left" },
            "ul": { "alignment": "left", "left-margin": "4ch", "max-width": "40" },
            "ol": { "alignment": "right" },
            "li": { "alignment": "center" },
            "hr": { "kind": "waves", "weight": "thick", "alignment": "center", "color": "slate-400" },
            "hyperlinks": { "color": "cyan-400", "local-style": { "color": "blue-300" } }
        });
        let (_, w) = from_json_value(&v).unwrap();
        let inactive: Vec<_> = w
            .iter()
            .filter(|w| matches!(w.kind, StyleWarningKind::KnownButInactive { .. }))
            .collect();
        assert!(inactive.is_empty(), "expected zero KnownButInactive warnings; got: {:?}", inactive);
    }

    #[test]
    fn nested_alias_emits_two_deprecated_warnings() {
        // Spec test #8 + Finding 1 regression: `block_quote.max_width` must
        // emit exactly two `Deprecated` warnings (one per snake-cased
        // segment). After sub-spec #3 phase 4 it no longer emits
        // `KnownButInactive` because `block-quote.max-width` is now wired.
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
        assert!(
            inactive.is_empty(),
            "block-quote.max-width is wired in sub-spec #3; got: {:?}",
            inactive
        );
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
        // Sub-specs #2, #3, and #4 wire the page leaves, the table
        // alignment/max-width leaves, and all list leaves. No inactive
        // warnings remain for this document.
        assert_eq!(inactive.len(), 0, "got {:?}", inactive);
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

    #[test]
    fn hr_alignment_centered_emits_deprecated_warning() {
        // `style.hr.alignment: centered` is accepted as an alias for
        // `center` (serde alias). The typed value must still parse to
        // `HrAlignment::Center`, but a `Deprecated` warning must be
        // emitted so `--strict-style` rejects the legacy spelling.
        let (s, w) = from_json_value(&json!({"hr": {"alignment": "centered"}})).unwrap();
        let hr = s.hr.expect("hr should be populated");
        assert_eq!(
            hr.alignment,
            Some(crate::style::schema::hr::HrAlignment::Center)
        );

        let deprecated: Vec<_> = w
            .iter()
            .filter(|w| matches!(w.kind, StyleWarningKind::Deprecated { .. }))
            .collect();
        assert_eq!(
            deprecated.len(),
            1,
            "expected exactly one Deprecated warning, got {:?}",
            w
        );
        assert_eq!(deprecated[0].path, "style.hr.alignment");
        match &deprecated[0].kind {
            StyleWarningKind::Deprecated { replacement } => {
                assert_eq!(replacement, "center");
            }
            other => panic!("expected Deprecated, got {:?}", other),
        }
    }

    #[test]
    fn hr_alignment_center_emits_no_warning() {
        // The canonical spelling must produce zero warnings.
        let (s, w) = from_json_value(&json!({"hr": {"alignment": "center"}})).unwrap();
        let hr = s.hr.expect("hr should be populated");
        assert_eq!(
            hr.alignment,
            Some(crate::style::schema::hr::HrAlignment::Center)
        );
        assert!(
            w.is_empty(),
            "canonical `center` must not produce warnings, got {:?}",
            w
        );
    }

    #[test]
    fn into_strict_fails_on_hr_alignment_centered_alias() {
        let parsed = from_json_value(&json!({"hr": {"alignment": "centered"}})).unwrap();
        match into_strict(parsed) {
            Err(StyleParseError::Strict { warnings }) => {
                assert!(
                    warnings
                        .iter()
                        .any(|w| w.path == "style.hr.alignment"
                            && matches!(
                                &w.kind,
                                StyleWarningKind::Deprecated { replacement } if replacement == "center"
                            )),
                    "expected style.hr.alignment deprecation with replacement=center, got {:?}",
                    warnings
                );
            }
            other => panic!("expected Strict error, got {:?}", other),
        }
    }

    #[test]
    fn active_wiring_warnings_for_list_keys() {
        let v = json!({
            "ul": {
                "width": "40ch",
                "max-width": "50%",
                "alignment": "left",
                "left-margin": "4ch",
                "color": "red-500",
                "bg-color": "blue-500"
            },
            "ol": {
                "width": "40ch",
                "max-width": "50%",
                "alignment": "right",
                "color": "red-500",
                "bg-color": "blue-500"
            },
            "li": {
                "width": "40ch",
                "max-width": "50%",
                "alignment": "center",
                "color": "red-500",
                "bg-color": "blue-500"
            }
        });
        let (_parsed, warnings) = from_json_value(&v).unwrap();

        let inactive: Vec<&StyleWarning> = warnings
            .iter()
            .filter(|w| matches!(w.kind, StyleWarningKind::KnownButInactive { .. }))
            .collect();

        // All list keys including color/bg-color (sub_spec 4 and 5) should NOT
        // produce inactive warnings.
        for path in [
            "style.ul.width",
            "style.ul.max-width",
            "style.ul.alignment",
            "style.ul.left-margin",
            "style.ul.color",
            "style.ul.bg-color",
            "style.ol.width",
            "style.ol.max-width",
            "style.ol.alignment",
            "style.ol.color",
            "style.ol.bg-color",
            "style.li.width",
            "style.li.max-width",
            "style.li.alignment",
            "style.li.color",
            "style.li.bg-color",
        ] {
            assert!(
                !inactive.iter().any(|w| w.path == path),
                "wired list key `{}` should not produce KnownButInactive, got: {:?}",
                path,
                inactive
            );
        }
    }

    #[test]
    fn into_strict_passes_clean_disclosure_style() {
        let parsed = from_json_value(
            &json!({
                "disclosure": {
                    "width": "40ch",
                    "max-width": "50%",
                    "alignment": "left",
                    "color": "red-500",
                    "bg-color": "blue-500"
                }
            }),
        )
        .unwrap();
        let s = into_strict(parsed).unwrap();
        assert!(s.disclosure.is_some());
    }

    #[test]
    fn into_strict_fails_on_unknown_disclosure_key() {
        let parsed = from_json_value(&json!({"disclosure": {"unknown_key": "x"}})).unwrap();
        match into_strict(parsed) {
            Err(StyleParseError::Strict { warnings }) => {
                assert_eq!(warnings.len(), 1);
                assert_eq!(warnings[0].path, "style.disclosure.unknown_key");
            }
            other => panic!("expected Strict error, got {:?}", other),
        }
    }

    #[test]
    fn into_strict_fails_on_disclosure_snake_case_aliases() {
        let parsed = from_json_value(
            &json!({
                "disclosure": {
                    "max_width": "50%",
                    "bg_color": "blue-500"
                }
            }),
        )
        .unwrap();
        match into_strict(parsed) {
            Err(StyleParseError::Strict { warnings }) => {
                assert!(
                    warnings.iter().any(|w| w.path == "style.disclosure.max_width"),
                    "expected style.disclosure.max_width deprecation, got {:?}",
                    warnings
                );
                assert!(
                    warnings.iter().any(|w| w.path == "style.disclosure.bg_color"),
                    "expected style.disclosure.bg_color deprecation, got {:?}",
                    warnings
                );
            }
            other => panic!("expected Strict error, got {:?}", other),
        }
    }

    // ── R-4 item 6 / R-5 Priority 5: nested YAML position map + spans ─────

    #[test]
    fn position_map_indexes_nested_keys() {
        let yaml = "title: Post\nstyle:\n  page:\n    left_margin: 2ch\n  table:\n    max-width: 50%\n";
        let map = build_yaml_position_map(yaml);
        // Top-level.
        assert_eq!(
            map.get("title"),
            Some(&StyleSpan { line: 1, column: 1, length: 5 })
        );
        // Container and deep leaf carry indent-aware columns.
        assert_eq!(
            map.get("style.page"),
            Some(&StyleSpan { line: 3, column: 3, length: 4 })
        );
        assert_eq!(
            map.get("style.page.left_margin"),
            Some(&StyleSpan { line: 4, column: 5, length: 11 })
        );
        assert_eq!(
            map.get("style.table.max-width"),
            Some(&StyleSpan { line: 6, column: 5, length: 9 })
        );
    }

    #[test]
    fn position_map_skips_comments_and_sequence_items() {
        let yaml = "# a comment\nlist:\n  - one\n  - two\nkey: value\n";
        let map = build_yaml_position_map(yaml);
        assert!(map.contains_key("list"));
        assert!(map.contains_key("key"));
        // Sequence items are not mapping keys.
        assert!(!map.keys().any(|k| k.contains("one") || k.contains("two")));
    }

    #[test]
    fn from_frontmatter_populates_warning_span_from_raw_source() {
        // A snake-cased leaf under `style.page` is Deprecated; with the raw
        // frontmatter text available its warning is ranged at the key.
        let raw = "style:\n  page:\n    left_margin: 2ch\n";
        let mut map = crate::markdown::FrontmatterMap::new();
        map.insert(
            "style".to_string(),
            json!({ "page": { "left_margin": "2ch" } }),
        );
        let fm = crate::markdown::Frontmatter::from_map_with_source(map, raw.to_string());
        let (_style, warnings) = from_frontmatter(&fm).unwrap();
        let warning = warnings
            .iter()
            .find(|w| w.path == "style.page.left_margin")
            .expect("deprecated left_margin warning");
        assert_eq!(
            warning.source_span,
            Some(StyleSpan { line: 3, column: 5, length: 11 }),
        );
    }

    #[test]
    fn from_frontmatter_without_raw_source_leaves_span_none() {
        // Programmatic frontmatter has no raw text — the span stays `None`.
        let mut fm = Frontmatter::new();
        fm.insert("style", json!({ "page": { "left_margin": "2ch" } }))
            .unwrap();
        let (_style, warnings) = from_frontmatter(&fm).unwrap();
        assert!(warnings.iter().all(|w| w.source_span.is_none()));
    }
}
