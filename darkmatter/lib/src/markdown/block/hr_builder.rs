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
    HorizontalRule, RulePlacement, RuleStyle, RuleWeight,
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

/// Allowed values for the `placement` attribute.
pub(crate) const ALLOWED_PLACEMENTS: &[&str] = &["full", "centered", "left", "right"];

/// Allowed values for the `weight` attribute.
pub(crate) const ALLOWED_WEIGHTS: &[&str] = &["thin", "medium", "thick"];

/// Builds a [`HorizontalRule`] from a parsed [`HorizontalRuleAttrs`], applying
/// each attribute through the appropriate builder method.
///
/// Unknown enum values for `style`, `placement`, or `weight` keep the rule's
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
///     placement: Some("centered".into()),
///     weight: Some("thick".into()),
///     width: Some("50%".into()),
///     color: Some("red".into()),
/// };
/// let _rule = build_rule(&attrs);
/// ```
pub(crate) fn build_rule(attrs: &HorizontalRuleAttrs) -> HorizontalRule {
    let mut rule = HorizontalRule::new();

    if let Some(raw) = attrs.style.as_deref() {
        rule = map_style(rule, raw);
    }
    if let Some(raw) = attrs.placement.as_deref() {
        rule = map_placement(rule, raw);
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

fn map_placement(rule: HorizontalRule, raw: &str) -> HorizontalRule {
    match raw {
        "full" => rule.placement(RulePlacement::Full),
        "centered" => rule.placement(RulePlacement::Centered),
        "left" => rule.placement(RulePlacement::Left),
        "right" => rule.placement(RulePlacement::Right),
        other => {
            tracing::warn!(
                attribute = "placement",
                value = %other,
                allowed = ?ALLOWED_PLACEMENTS,
                "unknown horizontal rule placement; falling back to default"
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
    use biscuit_terminal::components::renderable::{BrowserRenderable, Renderable};
    use biscuit_terminal::terminal::Terminal;

    #[test]
    fn build_rule_with_all_attrs_applies_them() {
        let attrs = HorizontalRuleAttrs {
            style: Some("waves".into()),
            placement: Some("centered".into()),
            weight: Some("thick".into()),
            width: Some("50%".into()),
            color: Some("red".into()),
        };

        let rule = build_rule(&attrs);
        let term = Terminal::builder().width(40).build();
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
            placement: None,
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
    fn build_rule_unknown_placement_and_weight_fall_back() {
        let attrs = HorizontalRuleAttrs {
            placement: Some("diagonal".into()),
            weight: Some("ultra".into()),
            ..Default::default()
        };
        let rule = build_rule(&attrs);
        let out = rule.render(&Terminal::builder().width(20).build());
        assert!(!out.is_empty());
    }
}
