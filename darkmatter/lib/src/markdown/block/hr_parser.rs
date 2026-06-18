use crate::markdown::inline::{HorizontalRuleAttrs, InlineEvent};
use crate::style::warning::{StyleWarning, StyleWarningKind};
use pulldown_cmark::{Event, Tag, TagEnd};

/// Result of [`parse_hr_attribute_block`] — the parsed attribute values plus
/// any [`StyleWarning`]s the caller should surface (deprecation, etc.).
///
/// `tracing::warn!` side effects (malformed YAML, unknown keys, non-scalar
/// values) are emitted directly during parsing and are intentionally not
/// carried in this struct; callers can observe them through a tracing
/// subscriber.
#[derive(Debug, Default)]
pub(crate) struct HrAttributeParseResult {
    pub(crate) attrs: HorizontalRuleAttrs,
    pub(crate) warnings: Vec<StyleWarning>,
}

/// Single source of truth for HR attribute parsing — used by the span-aware
/// fold's [`try_parse_hr_attrs`] wrapper, the render-tree block-extension
/// processor, and the [`scan_inline_hr_warnings`] preflight.
///
/// `attribute_str` is the content between `{` and `}` in an HR-attribute
/// directive such as `--- { kind: waves }`. The body is parsed as a YAML
/// flow mapping; on parse failure the legacy comma splitter runs as a
/// graceful fallback so previously-accepted-but-malformed inputs keep
/// working.
///
/// ## Notes
///
/// - Unknown keys are dropped with a `tracing::warn!`.
/// - Non-scalar values are dropped with a `tracing::warn!`.
/// - Presence of `style` (the deprecated key) records a deprecation
///   [`StyleWarning`] in [`HrAttributeParseResult::warnings`] in addition to
///   surfacing the value on [`HorizontalRuleAttrs::legacy_style`].
pub(crate) fn parse_hr_attribute_block(attribute_str: &str) -> HrAttributeParseResult {
    let attrs = parse_attrs(attribute_str);

    let mut warnings = Vec::new();
    if attrs.legacy_style.is_some() {
        warnings.push(StyleWarning::new(
            "hr.inline.style",
            StyleWarningKind::Deprecated {
                replacement: "hr.inline.kind".into(),
            },
        ));
    }

    HrAttributeParseResult { attrs, warnings }
}

/// Parses a darkmatter HR-attribute paragraph body — `---|***|___` followed by
/// an optional `{ ... }` attribute block — into [`HorizontalRuleAttrs`].
///
/// Returns `None` when `body` is not a recognized HR-attribute paragraph. The
/// span-aware fold uses this to detect and rewrite HR-attribute paragraphs
/// without re-implementing the matcher.
#[must_use]
pub fn try_parse_hr_attrs(body: &str) -> Option<HorizontalRuleAttrs> {
    // The matcher requires an attribute block (`{ ... }`); a bare `---`
    // already arrives from pulldown-cmark as `Event::Rule`, so this helper
    // intentionally returns `None` for it.
    let (_, attribute_str) = matches_horizontal_rule_pattern(body)?;
    Some(parse_hr_attribute_block(&attribute_str).attrs)
}

/// Matches `body` against the HR-attribute paragraph pattern, returning
/// `(marker, attribute-string)` when the body is a single-line directive.
///
/// The pattern requires three or more identical characters from `-`, `_`, or
/// `*`, optional whitespace, then a `{ ... }` attribute block. A bare `---`
/// is intentionally not recognized here — pulldown-cmark already emits it as
/// `Event::Rule`.
pub(crate) fn matches_horizontal_rule_pattern(text: &str) -> Option<(String, String)> {
    let trimmed = text.trim();
    if trimmed.len() < 3 {
        return None;
    }
    let first_char = trimmed.chars().next()?;
    if !['-', '_', '*'].contains(&first_char) {
        return None;
    }
    let mut marker_end = 0;
    for (i, ch) in trimmed.char_indices() {
        if ch != first_char {
            marker_end = i;
            break;
        }
    }
    if marker_end == 0 {
        marker_end = trimmed.len();
    }
    if marker_end < 3 {
        return None;
    }
    let after_markers = trimmed[marker_end..].trim_start();
    if !after_markers.starts_with('{') || !after_markers.ends_with('}') {
        return None;
    }
    let attributes = after_markers[1..after_markers.len() - 1].trim();
    let marker_str = trimmed[..marker_end].to_string();
    Some((marker_str, attributes.to_string()))
}

/// Parses the attribute string as a YAML flow mapping, falling back to the
/// legacy comma splitter on parse failure. The YAML path correctly handles
/// quoted values with embedded commas or colons (e.g. `color: "rgb(255,0,0)"`).
fn parse_attrs(attribute_str: &str) -> HorizontalRuleAttrs {
    if attribute_str.trim().is_empty() {
        return HorizontalRuleAttrs::default();
    }

    let yaml_src = format!("{{ {attribute_str} }}");
    match serde_yaml_ng::from_str::<serde_yaml_ng::Value>(&yaml_src) {
        Ok(serde_yaml_ng::Value::Mapping(map)) => attrs_from_mapping(&map),
        Ok(other) => {
            tracing::warn!(
                kind = ?std::mem::discriminant(&other),
                "horizontal rule attributes did not parse as a YAML mapping; using legacy splitter"
            );
            parse_attributes_legacy(attribute_str)
        }
        Err(err) => {
            tracing::warn!(
                error = %err,
                input = %attribute_str,
                "malformed horizontal rule attributes; using legacy splitter"
            );
            parse_attributes_legacy(attribute_str)
        }
    }
}

/// Builds [`HorizontalRuleAttrs`] from a YAML flow mapping, coercing scalars
/// to strings. Unknown keys and non-scalar values are dropped with a
/// `tracing::warn!`.
fn attrs_from_mapping(map: &serde_yaml_ng::Mapping) -> HorizontalRuleAttrs {
    let mut attrs = HorizontalRuleAttrs::default();

    for (yaml_key, yaml_value) in map {
        let Some(key) = yaml_value_as_string(yaml_key) else {
            tracing::warn!(
                key = ?yaml_key,
                "non-scalar key in horizontal rule attributes; ignoring"
            );
            continue;
        };

        let Some(value) = yaml_value_as_string(yaml_value) else {
            tracing::warn!(
                key = %key,
                value = ?yaml_value,
                "non-scalar value in horizontal rule attribute; ignoring"
            );
            continue;
        };

        match key.as_str() {
            "kind" => attrs.kind = Some(value),
            "style" => attrs.legacy_style = Some(value),
            "alignment" => attrs.alignment = Some(value),
            "weight" => attrs.weight = Some(value),
            "width" => attrs.width = Some(value),
            "color" => attrs.color = Some(value),
            other => {
                tracing::warn!(
                    key = %other,
                    value = %value,
                    "unknown horizontal rule attribute; ignoring"
                );
            }
        }
    }

    attrs
}

/// Coerces a YAML scalar to a Rust `String`, returning `None` for non-scalar
/// shapes (sequences, mappings, null, etc.).
fn yaml_value_as_string(value: &serde_yaml_ng::Value) -> Option<String> {
    match value {
        serde_yaml_ng::Value::String(s) => Some(s.clone()),
        serde_yaml_ng::Value::Number(n) => Some(n.to_string()),
        serde_yaml_ng::Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

/// Legacy ad-hoc splitter kept as a graceful fallback when the YAML parser
/// rejects the input. Mirrors the previous behavior so
/// malformed-but-previously-accepted attribute strings continue to parse
/// instead of silently losing all fields.
fn parse_attributes_legacy(attribute_str: &str) -> HorizontalRuleAttrs {
    let mut attrs = HorizontalRuleAttrs::default();

    if attribute_str.is_empty() {
        return attrs;
    }

    for pair in attribute_str.split(',') {
        let pair = pair.trim();
        if pair.is_empty() {
            continue;
        }

        let parts: Vec<&str> = pair.splitn(2, ':').collect();
        if parts.len() != 2 {
            continue;
        }

        let key = parts[0].trim();
        let value = parts[1].trim();

        let clean_value = if value.len() >= 2
            && ((value.starts_with('"') && value.ends_with('"'))
                || (value.starts_with('\'') && value.ends_with('\'')))
        {
            value[1..value.len() - 1].to_string()
        } else {
            value.to_string()
        };

        match key {
            "kind" => attrs.kind = Some(clean_value),
            "style" => attrs.legacy_style = Some(clean_value),
            "alignment" => attrs.alignment = Some(clean_value),
            "weight" => attrs.weight = Some(clean_value),
            "width" => attrs.width = Some(clean_value),
            "color" => attrs.color = Some(clean_value),
            other => {
                tracing::warn!(
                    key = %other,
                    value = %clean_value,
                    "unknown horizontal rule attribute; ignoring"
                );
            }
        }
    }

    attrs
}

/// Scan markdown content for inline horizontal-rule deprecation warnings.
///
/// Runs the inline event pipeline ([`pulldown_cmark::Parser`] →
/// [`InlineStyleProcessor`]) and checks each simple single-text paragraph for
/// the HR-attribute pattern, returning any deprecation warnings emitted for
/// legacy inline `style` attributes (`--- { style: waves }`).
///
/// This is a preflight helper for `--strict-style`: callers can check the
/// returned warnings before rendering and promote them to errors when strict
/// mode is enabled.
///
/// [`InlineStyleProcessor`]: crate::markdown::inline::InlineStyleProcessor
pub fn scan_inline_hr_warnings(content: &str) -> Vec<StyleWarning> {
    let preprocessed = crate::markdown::inline::preprocess_escaped_markers(content);
    let parser = pulldown_cmark::Parser::new_ext(
        &preprocessed,
        pulldown_cmark::Options::ENABLE_TABLES | pulldown_cmark::Options::ENABLE_STRIKETHROUGH,
    );
    let inline_events = crate::markdown::inline::InlineStyleProcessor::new(parser);

    let mut warnings = Vec::new();
    let mut paragraph_buffer: Vec<InlineEvent<'_>> = Vec::new();
    let mut in_paragraph = false;
    let mut paragraph_is_simple = true;

    for event in inline_events {
        match event {
            InlineEvent::Standard(Event::Start(Tag::Paragraph)) => {
                in_paragraph = true;
                paragraph_is_simple = true;
                paragraph_buffer.clear();
            }
            InlineEvent::Standard(Event::End(TagEnd::Paragraph)) if in_paragraph => {
                in_paragraph = false;
                if paragraph_is_simple
                    && paragraph_buffer.len() == 1
                    && let InlineEvent::Standard(Event::Text(text)) = &paragraph_buffer[0]
                    && let Some((_, attribute_str)) = matches_horizontal_rule_pattern(text)
                {
                    warnings.extend(parse_hr_attribute_block(&attribute_str).warnings);
                }
                paragraph_buffer.clear();
            }
            InlineEvent::Standard(Event::Text(_)) if in_paragraph => {
                paragraph_buffer.push(event);
            }
            other if in_paragraph => {
                paragraph_is_simple = false;
                paragraph_buffer.push(other);
            }
            _ => {}
        }
    }

    warnings
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::inline::HorizontalRuleAttrs;

    /// Parses an HR-attribute directive body (`--- { ... }`) into
    /// [`HorizontalRuleAttrs`], panicking if the body is not recognized as an
    /// HR-attribute paragraph. Targets the kept [`try_parse_hr_attrs`] parser
    /// directly (the same single source of truth the fold uses).
    fn hr_attrs(body: &str) -> HorizontalRuleAttrs {
        try_parse_hr_attrs(body).expect("expected an HR-attribute directive")
    }

    #[test]
    fn test_simple_horizontal_rule() {
        let attrs = hr_attrs("--- { style: waves }");
        assert_eq!(attrs.legacy_style, Some("waves".to_string()));
        assert_eq!(attrs.kind, None);
        assert_eq!(attrs.alignment, None);
        assert_eq!(attrs.weight, None);
        assert_eq!(attrs.width, None);
        assert_eq!(attrs.color, None);
    }

    #[test]
    fn test_horizontal_rule_with_multiple_attributes() {
        let attrs = hr_attrs("--- { style: dots, alignment: centered, weight: thick }");
        assert_eq!(attrs.legacy_style, Some("dots".to_string()));
        assert_eq!(attrs.kind, None);
        assert_eq!(attrs.alignment, Some("centered".to_string()));
        assert_eq!(attrs.weight, Some("thick".to_string()));
        assert_eq!(attrs.width, None);
        assert_eq!(attrs.color, None);
    }

    #[test]
    fn test_horizontal_rule_with_quoted_values() {
        let attrs = hr_attrs("--- { width: \"50%\", color: \"#ff0000\" }");
        assert_eq!(attrs.width, Some("50%".to_string()));
        assert_eq!(attrs.color, Some("#ff0000".to_string()));
    }

    #[test]
    fn test_horizontal_rule_with_single_quotes() {
        let attrs = hr_attrs("--- { width: '75%', color: '#00ff00' }");
        assert_eq!(attrs.width, Some("75%".to_string()));
        assert_eq!(attrs.color, Some("#00ff00".to_string()));
    }

    #[test]
    fn test_insufficient_markers() {
        // Fewer than three markers is not an HR-attribute directive.
        assert!(try_parse_hr_attrs("-- { style: waves }").is_none());
    }

    #[test]
    fn test_malformed_attributes() {
        // Missing the marker prefix entirely: not an HR-attribute directive.
        assert!(try_parse_hr_attrs("regular paragraph with { style waves }").is_none());
    }

    #[test]
    fn test_bare_rule_is_not_an_attribute_directive() {
        // A bare `---` arrives from pulldown-cmark as `Event::Rule`, so the
        // matcher intentionally does not recognize it.
        assert!(try_parse_hr_attrs("---").is_none());
    }

    #[test]
    fn test_different_marker_types() {
        assert!(try_parse_hr_attrs("*** { style: dashes }").is_some());
        assert!(try_parse_hr_attrs("___ { style: dots }").is_some());
    }

    #[test]
    fn test_attributes_with_spaces() {
        let attrs = hr_attrs("--- { style: line star, alignment: left }");
        assert_eq!(attrs.legacy_style, Some("line star".to_string()));
        assert_eq!(attrs.kind, None);
        assert_eq!(attrs.alignment, Some("left".to_string()));
    }

    #[test]
    fn test_empty_attributes() {
        let attrs = hr_attrs("--- { }");
        assert_eq!(attrs.kind, None);
        assert_eq!(attrs.legacy_style, None);
        assert_eq!(attrs.alignment, None);
        assert_eq!(attrs.weight, None);
        assert_eq!(attrs.width, None);
        assert_eq!(attrs.color, None);
    }

    #[test]
    fn test_whitespace_handling() {
        let attrs = hr_attrs("   ---   {   style:   waves   }   ");
        assert_eq!(attrs.legacy_style, Some("waves".to_string()));
        assert_eq!(attrs.kind, None);
    }

    #[test]
    fn test_horizontal_rule_attrs_default() {
        let attrs = HorizontalRuleAttrs::default();
        assert_eq!(attrs.kind, None);
        assert_eq!(attrs.legacy_style, None);
        assert_eq!(attrs.alignment, None);
        assert_eq!(attrs.weight, None);
        assert_eq!(attrs.width, None);
        assert_eq!(attrs.color, None);
    }

    #[test]
    fn test_horizontal_rule_attrs_clone() {
        let attrs1 = HorizontalRuleAttrs {
            kind: Some("test".to_string()),
            legacy_style: None,
            alignment: Some("centered".to_string()),
            weight: Some("medium".to_string()),
            width: Some("50%".to_string()),
            color: Some("red".to_string()),
        };
        let attrs2 = attrs1.clone();
        assert_eq!(attrs1.kind, attrs2.kind);
        assert_eq!(attrs1.legacy_style, attrs2.legacy_style);
        assert_eq!(attrs1.alignment, attrs2.alignment);
        assert_eq!(attrs1.weight, attrs2.weight);
        assert_eq!(attrs1.width, attrs2.width);
        assert_eq!(attrs1.color, attrs2.color);
    }

    #[test]
    fn test_horizontal_rule_attrs_partial() {
        let attrs = HorizontalRuleAttrs {
            kind: None,
            legacy_style: Some("waves".to_string()),
            alignment: None,
            weight: Some("thick".to_string()),
            width: None,
            color: Some("blue".to_string()),
        };
        assert_eq!(attrs.kind, None);
        assert_eq!(attrs.legacy_style, Some("waves".to_string()));
        assert_eq!(attrs.alignment, None);
        assert_eq!(attrs.weight, Some("thick".to_string()));
        assert_eq!(attrs.width, None);
        assert_eq!(attrs.color, Some("blue".to_string()));
    }

    // Phase 2 / Sub-spec #6 — inline `kind` is canonical; `style` is deprecated.

    #[test]
    fn test_inline_kind_parses_without_warning() {
        let result = parse_hr_attribute_block("kind: waves");
        assert_eq!(result.attrs.kind, Some("waves".to_string()));
        assert_eq!(result.attrs.legacy_style, None);
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn test_inline_legacy_style_emits_deprecation_warning() {
        let result = parse_hr_attribute_block("style: waves");
        assert_eq!(result.attrs.legacy_style, Some("waves".to_string()));
        assert_eq!(result.attrs.kind, None);
        assert_eq!(result.warnings.len(), 1);
        assert_eq!(result.warnings[0].path, "hr.inline.style");
        assert!(
            matches!(
                &result.warnings[0].kind,
                StyleWarningKind::Deprecated { replacement } if replacement == "hr.inline.kind"
            ),
            "expected Deprecated warning for inline style, got {:?}",
            result.warnings[0].kind
        );
    }

    #[test]
    fn test_inline_kind_beats_legacy_style() {
        let result = parse_hr_attribute_block("kind: dots, style: waves");
        assert_eq!(result.attrs.kind, Some("dots".to_string()));
        assert_eq!(result.attrs.legacy_style, Some("waves".to_string()));
        // Legacy key is still present, so deprecation warning fires even when
        // canonical wins.
        assert_eq!(result.warnings.len(), 1);
    }

    // ----- scan_inline_hr_warnings preflight -----

    #[test]
    fn scan_inline_hr_warnings_empty_for_clean_doc() {
        let warnings = scan_inline_hr_warnings("# Hello\n\n---\n\nSome text.");
        assert!(warnings.is_empty());
    }

    #[test]
    fn scan_inline_hr_warnings_detects_legacy_style() {
        let warnings = scan_inline_hr_warnings("--- { style: waves }");
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].path, "hr.inline.style");
        assert!(
            matches!(
                &warnings[0].kind,
                StyleWarningKind::Deprecated { replacement } if replacement == "hr.inline.kind"
            )
        );
    }

    #[test]
    fn scan_inline_hr_warnings_detects_multiple_legacy_rules() {
        let warnings = scan_inline_hr_warnings(
            "--- { style: waves }\n\nSome text.\n\n--- { style: dots }\n",
        );
        assert_eq!(warnings.len(), 2);
    }

    #[test]
    fn scan_inline_hr_warnings_empty_for_canonical_kind() {
        let warnings = scan_inline_hr_warnings("--- { kind: waves }");
        assert!(warnings.is_empty());
    }

    // Phase 5 B1: validation — unknown enum values are captured verbatim so the
    // downstream builder can warn + fall back to defaults. Unknown keys are
    // dropped with a warn (see `build_rule`); we assert the resulting attrs.

    #[test]
    fn test_parse_attributes_unknown_style_is_captured_raw() {
        // Unknown enum values are retained on the attrs struct so the
        // builder layer (`build_rule`) can emit a warning and fall back.
        let attrs = hr_attrs("--- { style: bogus }");
        assert_eq!(attrs.legacy_style, Some("bogus".to_string()));
        assert_eq!(attrs.kind, None);
    }

    #[test]
    fn test_parse_attributes_unknown_alignment_is_captured_raw() {
        let attrs = hr_attrs("--- { alignment: diagonal }");
        assert_eq!(attrs.alignment, Some("diagonal".to_string()));
    }

    #[test]
    fn test_parse_attributes_unknown_weight_is_captured_raw() {
        let attrs = hr_attrs("--- { weight: zzz }");
        assert_eq!(attrs.weight, Some("zzz".to_string()));
    }

    #[test]
    fn test_parse_attributes_unknown_key_is_dropped() {
        // Unknown keys are dropped with a warn; the resulting attrs struct
        // has all fields `None`.
        let attrs = hr_attrs("--- { margin: 4 }");
        assert_eq!(attrs.kind, None);
        assert_eq!(attrs.legacy_style, None);
        assert_eq!(attrs.alignment, None);
        assert_eq!(attrs.weight, None);
        assert_eq!(attrs.width, None);
        assert_eq!(attrs.color, None);
    }

    #[test]
    #[tracing_test::traced_test]
    fn test_parse_attributes_unknown_key_emits_warning() {
        // With tracing-test's subscriber in scope, the warn! call inside
        // `parse_attributes` becomes observable via `logs_contain`.
        let _ = parse_hr_attribute_block("totally_bogus_key: 42");
        assert!(logs_contain("unknown horizontal rule attribute"));
        assert!(logs_contain("totally_bogus_key"));
    }

    // =====================================================================
    // Phase 6 / E2 — YAML flow-mapping parsing handles quoted separators
    // =====================================================================

    #[test]
    fn test_parse_attributes_quoted_color_with_embedded_comma() {
        // Quoted values must preserve embedded commas. The hand-rolled
        // splitter used to break on these; `serde_yaml_ng` handles them
        // correctly.
        let attrs = hr_attrs("--- { color: \"rgb(255, 0, 0)\" }");
        assert_eq!(attrs.color, Some("rgb(255, 0, 0)".to_string()));
    }

    #[test]
    fn test_parse_attributes_quoted_value_with_embedded_colon() {
        // Quoted values must preserve embedded colons. The key `prefix` is
        // unknown, so it is dropped — but the test still exercises the
        // quoted-colon code path because the YAML parser has to succeed
        // before the drop-with-warn happens.
        let attrs = hr_attrs("--- { prefix: \"a:b\" }");
        // Unknown key is dropped with a warn.
        assert_eq!(attrs.kind, None);
        assert_eq!(attrs.legacy_style, None);
        assert_eq!(attrs.color, None);
    }

    #[test]
    fn test_parse_attributes_malformed_yaml_falls_back_gracefully() {
        // `{ style: }` is not valid YAML (missing value). The parser must
        // not panic; instead it falls back to the legacy splitter (which
        // also cannot parse this) and produces default attrs.
        let attrs = hr_attrs("--- { style: }");
        assert_eq!(attrs.kind, None);
        assert_eq!(attrs.legacy_style, None);
        assert_eq!(attrs.alignment, None);
        assert_eq!(attrs.weight, None);
        assert_eq!(attrs.width, None);
        assert_eq!(attrs.color, None);
    }

    #[test]
    fn test_parse_attributes_yaml_number_scalar_coerced_to_string() {
        // Numeric YAML scalars must round-trip as strings so downstream
        // string-typed fields receive a sensible value.
        let attrs = hr_attrs("--- { width: 50 }");
        assert_eq!(attrs.width, Some("50".to_string()));
    }

    // =====================================================================
    // Phase 6 / C1 — matcher edge cases (mixed markers)
    // =====================================================================

    #[test]
    fn test_mixed_marker_dash_star_dash_is_not_transformed() {
        // `-*-` mixes marker characters; the matcher requires three or more
        // of the *same* character, so this is not an HR-attribute directive.
        assert!(try_parse_hr_attrs("-*- { style: dots }").is_none());
    }

    #[test]
    fn test_mixed_marker_dash_underscore_dash_is_not_transformed() {
        // `-_-` is another mixed-marker case; same invariant applies.
        assert!(try_parse_hr_attrs("-_- { style: dots }").is_none());
    }
}
