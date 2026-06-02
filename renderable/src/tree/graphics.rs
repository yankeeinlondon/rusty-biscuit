//! Graphics helpers for the render tree.
//!
//! These functions produce target-specific rich output (SVG, etc.) from
//! render-tree nodes, letting the tree renderer emit graphics without
//! depending on downstream component crates.

/// Renders a styled horizontal rule as an SVG `<svg>` element.
///
/// The emitted SVG declares three CSS custom properties on the root
/// `<svg>` element in its inline `style` attribute:
///
/// - `--hr-weight` — stroke width in pixels (2/4/8 for thin/medium/thick)
/// - `--hr-color` — stroke/fill color (defaults to `currentColor`)
/// - `--hr-width` — the outer SVG width (e.g., `100%` or a custom value)
///
/// Shape primitives reference these via `var(--hr-weight, {fallback})`,
/// `var(--hr-color, currentColor)`, etc. so that the SVG renders
/// correctly even when the declared inline variables are stripped.
///
/// ## Arguments
///
/// - `style`: The visual style — `"dashes"`, `"dots"`, `"waves"`,
///   `"line-star"`, `"line-circle"`, `"inset-line"`, or `"curtain-rod"`.
///   Unrecognized or `None` values fall back to `"dashes"`.
/// - `weight`: The stroke weight — `"thin"`, `"medium"`, or `"thick"`.
///   Unrecognized or `None` values fall back to `"medium"`.
/// - `width`: The CSS width (e.g., `"100%"`, `"50%"`, `"20ch"`).
///   `None` falls back to `"100%"`.
/// - `color`: The CSS color (e.g., `"red"`, `"#336699"`).
///   `None` falls back to `"currentColor"`.
/// - `margin_top`: Top margin CSS value (e.g., `"0"`, `"1em"`).
/// - `margin_bottom`: Bottom margin CSS value.
///
/// ## Examples
///
/// ```
/// use renderable::tree::graphics::horizontal_rule_svg;
///
/// let svg = horizontal_rule_svg(Some("waves"), Some("thick"), Some("75%"), Some("blue"), "0", "0");
/// assert!(svg.contains(r#"width="75%""#));
/// assert!(svg.contains("--hr-weight: 8"));
/// assert!(svg.contains("--hr-color: blue"));
/// ```
pub fn horizontal_rule_svg(
    style: Option<&str>,
    weight: Option<&str>,
    width: Option<&str>,
    color: Option<&str>,
    margin_top: &str,
    margin_bottom: &str,
) -> String {
    let stroke_width = match weight {
        Some("thin") => "2",
        Some("thick") => "8",
        _ => "4", // medium or unrecognized
    };

    let width_attr = width.unwrap_or("100%");
    let color_attr = color.unwrap_or("currentColor");

    let stroke_var = format!("var(--hr-color, {})", color_attr);
    let width_var = format!("var(--hr-weight, {})", stroke_width);
    let fill_var = format!("var(--hr-color, {})", color_attr);

    let svg_content = match style {
        Some("dots") => format!(
            r#"<line x1="0" y1="50%" x2="100%" y2="50%" stroke="{}" stroke-width="{}" stroke-linecap="round" stroke-dasharray="2,6"/>"#,
            stroke_var, width_var
        ),
        Some("waves") => format!(
            r#"<path d="M0 20 Q 10 10 20 20 T 40 20 T 60 20 T 80 20 T 100 20 T 120 20 T 140 20 T 160 20 T 180 20 T 200 20" stroke="{}" stroke-width="{}" fill="none" stroke-linecap="round"/>"#,
            stroke_var, width_var
        ),
        Some("line-star") => format!(
            r#"<line x1="0" y1="50%" x2="45%" y2="50%" stroke="{}" stroke-width="{}" stroke-linecap="round"/>
  <path d="M50% 35% L52% 45% L62% 45% L54% 52% L57% 62% L50% 55% L43% 62% L46% 52% L38% 45% L48% 45% Z" fill="{}"/>
  <line x1="55%" y1="50%" x2="100%" y2="50%" stroke="{}" stroke-width="{}" stroke-linecap="round"/>"#,
            stroke_var, width_var, fill_var, stroke_var, width_var
        ),
        Some("line-circle") => format!(
            r#"<line x1="0" y1="50%" x2="45%" y2="50%" stroke="{}" stroke-width="{}" stroke-linecap="round"/>
  <circle cx="50%" cy="50%" r="8" fill="none" stroke="{}" stroke-width="{}"/>
  <line x1="55%" y1="50%" x2="100%" y2="50%" stroke="{}" stroke-width="{}" stroke-linecap="round"/>"#,
            stroke_var, width_var, stroke_var, width_var, stroke_var, width_var
        ),
        Some("inset-line") => format!(
            r#"<line x1="10%" y1="50%" x2="90%" y2="50%" stroke="{}" stroke-width="{}" stroke-linecap="round"/>"#,
            stroke_var, width_var
        ),
        Some("curtain-rod") => format!(
            r#"<line x1="5%" y1="50%" x2="95%" y2="50%" stroke="{}" stroke-width="{}" stroke-linecap="round"/>
  <circle cx="5%" cy="50%" r="4" fill="{}"/>
  <circle cx="95%" cy="50%" r="4" fill="{}"/>"#,
            stroke_var, width_var, fill_var, fill_var
        ),
        // "dashes" or unrecognized
        _ => format!(
            r#"<line x1="0" y1="50%" x2="100%" y2="50%" stroke="{}" stroke-width="{}" stroke-linecap="round" stroke-dasharray="8,4"/>"#,
            stroke_var, width_var
        ),
    };

    format!(
        r#"<svg class="darkmatter-hr" width="{width}" height="40" xmlns="http://www.w3.org/2000/svg" style="display: block; margin: {top} auto {bot} auto; --hr-weight: {weight}; --hr-color: {color}; --hr-width: {width};">
  {content}
</svg>"#,
        width = width_attr,
        top = margin_top,
        bot = margin_bottom,
        weight = stroke_width,
        color = color_attr,
        content = svg_content,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_produces_dashes_svg() {
        let svg = horizontal_rule_svg(None, None, None, None, "0", "0");
        assert!(svg.contains(r#"width="100%""#));
        assert!(svg.contains("--hr-weight: 4"));
        assert!(svg.contains("--hr-color: currentColor"));
        assert!(svg.contains("stroke-dasharray=\"8,4\""));
    }

    #[test]
    fn waves_with_thick_and_custom_color() {
        let svg = horizontal_rule_svg(Some("waves"), Some("thick"), Some("75%"), Some("blue"), "0", "0");
        assert!(svg.contains(r#"width="75%""#));
        assert!(svg.contains("--hr-weight: 8"));
        assert!(svg.contains("--hr-color: blue"));
        assert!(svg.contains(r#"d="M0 20 Q 10 10 20 20"#));
    }

    #[test]
    fn dots_with_thin() {
        let svg = horizontal_rule_svg(Some("dots"), Some("thin"), None, None, "0", "0");
        assert!(svg.contains("--hr-weight: 2"));
        assert!(svg.contains("stroke-dasharray=\"2,6\""));
    }

    #[test]
    fn line_star_produces_star_path() {
        let svg = horizontal_rule_svg(Some("line-star"), None, None, None, "0", "0");
        assert!(svg.contains("L52% 45%"));
        assert!(svg.contains("Z"));
    }

    #[test]
    fn line_circle_produces_circle() {
        let svg = horizontal_rule_svg(Some("line-circle"), None, None, None, "0", "0");
        assert!(svg.contains("<circle cx=\"50%\" cy=\"50%\" r=\"8\""));
    }

    #[test]
    fn inset_line_produces_short_line() {
        let svg = horizontal_rule_svg(Some("inset-line"), None, None, None, "0", "0");
        assert!(svg.contains("x1=\"10%\""));
        assert!(svg.contains("x2=\"90%\""));
    }

    #[test]
    fn curtain_rod_produces_circles_at_ends() {
        let svg = horizontal_rule_svg(Some("curtain-rod"), None, None, None, "0", "0");
        assert!(svg.contains("cx=\"5%\""));
        assert!(svg.contains("cx=\"95%\""));
    }

    #[test]
    fn unrecognized_style_falls_back_to_dashes() {
        let svg = horizontal_rule_svg(Some("bogus"), None, None, None, "0", "0");
        assert!(svg.contains("stroke-dasharray=\"8,4\""));
    }

    #[test]
    fn margins_are_included_in_style() {
        let svg = horizontal_rule_svg(None, None, None, None, "1em", "2em");
        assert!(svg.contains("margin: 1em auto 2em auto"));
    }

    #[test]
    fn stroke_and_fill_use_css_variables() {
        let svg = horizontal_rule_svg(Some("line-star"), None, None, Some("red"), "0", "0");
        assert!(svg.contains("stroke=\"var(--hr-color, red)\""));
        assert!(svg.contains("fill=\"var(--hr-color, red)\""));
    }
}
