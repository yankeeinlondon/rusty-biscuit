//! Shared builder for `HorizontalRule` components from
//! [`HorizontalRuleAttrs`](crate::markdown::inline::HorizontalRuleAttrs).
//!
//! Both the terminal and HTML renderers translate the string-keyed
//! [`HorizontalRuleAttrs`] produced by
//! [`RuleProcessor`](crate::markdown::block::RuleProcessor) into a
//! [`HorizontalRule`] builder. Keeping that translation in one place prevents
//! the two renderers from drifting and gives a single site for validation
//! warnings on unknown attribute values (B1).
//!
//! Unknown enum values (e.g., `style: dashse`) fall through to the
//! [`HorizontalRule`] defaults and emit a `tracing::warn!` rather than
//! returning an error — the renderer still produces output, but the problem
//! becomes visible with `RUST_LOG=darkmatter=warn`.

use crate::markdown::inline::HorizontalRuleAttrs;
use biscuit_terminal::components::horizontal_rule::{
    HorizontalRule, RuleAlignment, RuleStyle, RuleWeight,
};

/// Allowed values for the `style` attribute.
pub(crate) const ALLOWED_STYLES: &[&str] = &[
    "dashes",
    "dots",
    "waves",
    "line-star",
    "line-circle",
    "inset-line",
    "curtain-rod",
];

/// Allowed values for the `alignment` attribute.
pub(crate) const ALLOWED_ALIGNMENTS: &[&str] = &["full", "centered", "left", "right"];

/// Allowed values for the `weight` attribute.
pub(crate) const ALLOWED_WEIGHTS: &[&str] = &["thin", "medium", "thick"];

/// Builds a [`HorizontalRule`] from a parsed [`HorizontalRuleAttrs`], applying
/// each attribute through the appropriate builder method.
///
/// Unknown enum values for `style`, `alignment`, or `weight` keep the rule's
/// default and emit a `tracing::warn!` with the raw string. Unknown attribute
/// keys are never seen here — they are dropped (with a warning) by
/// [`RuleProcessor::parse_attributes`](crate::markdown::block::RuleProcessor)
/// before reaching this helper.
///
/// ## Examples
///
/// ```ignore
/// use darkmatter::markdown::block::build_rule;
/// use darkmatter::markdown::inline::HorizontalRuleAttrs;
///
/// let attrs = HorizontalRuleAttrs {
///     style: Some("waves".into()),
///     alignment: Some("centered".into()),
///     weight: Some("thick".into()),
///     width: Some("50%".into()),
///     color: Some("red".into()),
/// };
/// let _rule = build_rule(&attrs);
/// ```
#[cfg(test)]
pub(crate) fn build_rule(attrs: &HorizontalRuleAttrs) -> HorizontalRule {
    build_rule_with_defaults(None, attrs)
}

pub(crate) fn build_rule_with_defaults(
    defaults: Option<&HorizontalRuleAttrs>,
    attrs: &HorizontalRuleAttrs,
) -> HorizontalRule {
    let attrs = merge_attrs(defaults, attrs);
    let mut rule = HorizontalRule::new();

    if let Some(raw) = attrs.style.as_deref() {
        rule = map_style(rule, raw);
    }
    if let Some(raw) = attrs.alignment.as_deref() {
        rule = map_alignment(rule, raw);
    }
    if let Some(raw) = attrs.weight.as_deref() {
        rule = map_weight(rule, raw);
    }
    if let Some(width) = attrs.width.as_deref() {
        rule = rule.width(width.to_string());
    }
    if let Some(color) = attrs.color.as_deref() {
        rule = rule.color(color.to_string());
    }

    rule
}

/// Resolves the `hr:` block from the document's frontmatter into a
/// [`HorizontalRuleAttrs`] suitable for feeding
/// [`build_rule_with_defaults`].
///
/// The frontmatter backing store is `serde_json::Value`, so a direct
/// `frontmatter.get::<HorizontalRuleAttrs>("hr")` call fails fast on any
/// non-string scalar (e.g., `hr: { width: 50 }`), dropping every sibling
/// key. To keep authoring ergonomic and to match the attribute-block
/// path's coercion (see `RuleProcessor::yaml_value_as_string`), this
/// helper walks the `hr` mapping entry-by-entry and coerces numbers and
/// bools to strings via [`json_scalar_as_string`].
///
/// ## Notes
///
/// - Non-mapping `hr` values (e.g., `hr: 42`, `hr: "foo"`) emit a
///   `tracing::warn!` and return `None`.
/// - Unknown keys emit a `tracing::warn!` mirroring the attribute-block
///   path's message and are dropped from the result.
/// - Non-scalar values (arrays, objects, `null`) for recognized keys emit
///   a `tracing::warn!` and are skipped — remaining sibling keys still
///   apply.
pub(crate) fn hr_defaults_from_frontmatter(
    md: &crate::markdown::Markdown,
) -> Option<HorizontalRuleAttrs> {
    let value = md.frontmatter().as_map().get("hr")?;
    let map = match value {
        serde_json::Value::Object(map) => map,
        other => {
            tracing::warn!(
                value = ?other,
                "non-mapping `hr` frontmatter; using horizontal rule component defaults"
            );
            return None;
        }
    };

    let mut attrs = HorizontalRuleAttrs::default();
    for (key, entry) in map {
        let Some(value) = json_scalar_as_string(entry) else {
            tracing::warn!(
                key = %key,
                value = ?entry,
                "non-scalar value in `hr` frontmatter; ignoring"
            );
            continue;
        };

        match key.as_str() {
            "style" => attrs.style = Some(value),
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

    Some(attrs)
}

/// Coerces a JSON scalar to a Rust `String`, returning `None` for
/// non-scalar shapes (arrays, objects, null). Mirrors
/// [`RuleProcessor::yaml_value_as_string`](crate::markdown::block::RuleProcessor)
/// on the attribute-block path so both sites agree on which values are
/// considered string-coercible.
fn json_scalar_as_string(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::String(s) => Some(s.clone()),
        serde_json::Value::Number(n) => Some(n.to_string()),
        serde_json::Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

fn merge_attrs(
    defaults: Option<&HorizontalRuleAttrs>,
    attrs: &HorizontalRuleAttrs,
) -> HorizontalRuleAttrs {
    let mut merged = defaults.cloned().unwrap_or_default();
    if attrs.style.is_some() {
        merged.style = attrs.style.clone();
    }
    if attrs.alignment.is_some() {
        merged.alignment = attrs.alignment.clone();
    }
    if attrs.weight.is_some() {
        merged.weight = attrs.weight.clone();
    }
    if attrs.width.is_some() {
        merged.width = attrs.width.clone();
    }
    if attrs.color.is_some() {
        merged.color = attrs.color.clone();
    }
    merged
}

fn map_style(rule: HorizontalRule, raw: &str) -> HorizontalRule {
    match raw {
        "dashes" => rule.style(RuleStyle::Dashes),
        "dots" => rule.style(RuleStyle::Dots),
        "waves" => rule.style(RuleStyle::Waves),
        "line-star" => rule.style(RuleStyle::LineStar),
        "line-circle" => rule.style(RuleStyle::LineCircle),
        "inset-line" => rule.style(RuleStyle::InsetLine),
        "curtain-rod" => rule.style(RuleStyle::CurtainRod),
        other => {
            tracing::warn!(
                attribute = "style",
                value = %other,
                allowed = ?ALLOWED_STYLES,
                "unknown horizontal rule style; falling back to default"
            );
            rule
        }
    }
}

fn map_alignment(rule: HorizontalRule, raw: &str) -> HorizontalRule {
    match raw {
        "full" => rule.alignment(RuleAlignment::Full),
        "centered" => rule.alignment(RuleAlignment::Centered),
        "left" => rule.alignment(RuleAlignment::Left),
        "right" => rule.alignment(RuleAlignment::Right),
        other => {
            tracing::warn!(
                attribute = "alignment",
                value = %other,
                allowed = ?ALLOWED_ALIGNMENTS,
                "unknown horizontal rule alignment; falling back to default"
            );
            rule
        }
    }
}

fn map_weight(rule: HorizontalRule, raw: &str) -> HorizontalRule {
    match raw {
        "thin" => rule.weight(RuleWeight::Thin),
        "medium" => rule.weight(RuleWeight::Medium),
        "thick" => rule.weight(RuleWeight::Thick),
        other => {
            tracing::warn!(
                attribute = "weight",
                value = %other,
                allowed = ?ALLOWED_WEIGHTS,
                "unknown horizontal rule weight; falling back to default"
            );
            rule
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use biscuit_terminal::components::renderable::{BrowserRenderable, TerminalRenderable};
    use biscuit_terminal::discovery::detection::ImageSupport;
    use biscuit_terminal::terminal::Terminal;

    // --------------------------------------------------------------
    // B3 / C2 — `hr_defaults_from_frontmatter` must coerce non-string
    // scalars and preserve siblings; non-mapping `hr` values warn and
    // return `None`.
    // --------------------------------------------------------------

    fn make_md_with_hr_frontmatter(value: serde_json::Value) -> crate::markdown::Markdown {
        let mut fm = crate::markdown::Frontmatter::new();
        fm.as_map_mut().insert("hr".to_string(), value);
        crate::markdown::Markdown::with_frontmatter(fm, "---\n".to_string())
    }

    #[test]
    fn hr_defaults_from_frontmatter_numeric_width_preserves_siblings() {
        let md = make_md_with_hr_frontmatter(serde_json::json!({
            "style": "dots",
            "width": 20,
            "color": "red",
        }));
        let attrs = hr_defaults_from_frontmatter(&md).expect("expected Some attrs");
        assert_eq!(attrs.style.as_deref(), Some("dots"));
        assert_eq!(
            attrs.width.as_deref(),
            Some("20"),
            "numeric width must be coerced to string"
        );
        assert_eq!(attrs.color.as_deref(), Some("red"));
    }

    #[test]
    fn hr_defaults_from_frontmatter_bool_value_preserves_siblings() {
        let md = make_md_with_hr_frontmatter(serde_json::json!({
            "style": "waves",
            "alignment": true,
            "color": "blue",
        }));
        let attrs = hr_defaults_from_frontmatter(&md).expect("expected Some attrs");
        // `true` coerces to the string "true" (which is not a recognized
        // alignment value), but siblings still apply.
        assert_eq!(attrs.style.as_deref(), Some("waves"));
        assert_eq!(attrs.alignment.as_deref(), Some("true"));
        assert_eq!(attrs.color.as_deref(), Some("blue"));
    }

    #[test]
    #[tracing_test::traced_test]
    fn hr_defaults_from_frontmatter_non_mapping_warns_and_returns_none() {
        let md = make_md_with_hr_frontmatter(serde_json::json!(42));
        let attrs = hr_defaults_from_frontmatter(&md);
        assert!(attrs.is_none(), "non-mapping hr value must yield None");
        assert!(
            logs_contain("non-mapping"),
            "expected a warn log mentioning non-mapping hr frontmatter"
        );
    }

    #[test]
    #[tracing_test::traced_test]
    fn hr_defaults_from_frontmatter_unknown_key_warns_and_drops_it() {
        let md = make_md_with_hr_frontmatter(serde_json::json!({
            "style": "dashes",
            "bogus": "value",
        }));
        let attrs = hr_defaults_from_frontmatter(&md).expect("expected Some attrs");
        assert_eq!(attrs.style.as_deref(), Some("dashes"));
        assert!(
            logs_contain("unknown horizontal rule attribute"),
            "expected a warn log for unknown `bogus` key"
        );
    }

    #[test]
    fn hr_defaults_from_frontmatter_missing_key_returns_none() {
        // No `hr` at all — `None` without a warn.
        let md = crate::markdown::Markdown::with_frontmatter(
            crate::markdown::Frontmatter::new(),
            "---\n".to_string(),
        );
        assert!(hr_defaults_from_frontmatter(&md).is_none());
    }

    #[test]
    fn build_rule_with_all_attrs_applies_them() {
        let attrs = HorizontalRuleAttrs {
            style: Some("waves".into()),
            alignment: Some("centered".into()),
            weight: Some("thick".into()),
            width: Some("50%".into()),
            color: Some("red".into()),
        };

        let rule = build_rule(&attrs);
        assert_eq!(rule.rule_style(), &RuleStyle::Waves);
        assert_eq!(rule.rule_alignment(), &RuleAlignment::Centered);
        assert_eq!(rule.rule_weight(), &RuleWeight::Thick);
        assert_eq!(rule.rule_width(), Some("50%"));
        assert_eq!(rule.rule_color(), Some("red"));

        let term = Terminal::builder()
            .width(40)
            .image_support(ImageSupport::None)
            .build();
        let out = rule.render(&term);
        // Waves uses ≋ in Unicode mode, ~ otherwise.
        assert!(out.contains('≋') || out.contains('~'));
    }

    #[test]
    fn build_rule_produces_consistent_browser_output_for_both_renderers() {
        // Both the terminal and HTML code paths call `build_rule` with the
        // same attrs — assert the same builder produces matching browser
        // SVG for any given attr set.
        let attrs = HorizontalRuleAttrs {
            style: Some("dashes".into()),
            alignment: None,
            weight: Some("thin".into()),
            width: Some("33%".into()),
            color: Some("blue".into()),
        };

        let svg_a = build_rule(&attrs).render_to_browser();
        let svg_b = build_rule(&attrs).render_to_browser();
        assert_eq!(svg_a, svg_b);
        assert!(svg_a.contains("--hr-weight: 2"));
        assert!(svg_a.contains("--hr-color: blue"));
        assert!(svg_a.contains("--hr-width: 33%"));
    }

    #[test]
    fn build_rule_unknown_style_falls_back_to_default() {
        let attrs = HorizontalRuleAttrs {
            style: Some("bogus".into()),
            ..Default::default()
        };
        // Must not panic; default style (Dashes) renders successfully.
        let rule = build_rule(&attrs);
        let out = rule.render(&Terminal::builder().width(20).build());
        assert!(!out.is_empty());
    }

    #[test]
    fn build_rule_unknown_alignment_and_weight_fall_back() {
        let attrs = HorizontalRuleAttrs {
            alignment: Some("diagonal".into()),
            weight: Some("ultra".into()),
            ..Default::default()
        };
        let rule = build_rule(&attrs);
        let out = rule.render(&Terminal::builder().width(20).build());
        assert!(!out.is_empty());
    }
}
