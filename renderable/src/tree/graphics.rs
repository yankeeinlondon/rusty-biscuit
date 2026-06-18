//! Graphics helpers for the render tree.
//!
//! These functions produce target-specific rich output (SVG, etc.) from
//! render-tree nodes, letting the tree renderer emit graphics without
//! depending on downstream component crates.

use crate::tree::{HrAlignment, HrKind, HrWeight};

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
/// - `kind`: The visual rule kind. `None` falls back to dashes.
/// - `weight`: The stroke weight. `None` falls back to medium.
/// - `alignment`: Horizontal placement. The non-vertical CSS margins are set
///   from this so a non-full `width` anchors left/right/center rather than
///   always centering. `None` and `Center` both center; `Left`/`Right` anchor
///   to that edge; `Full` uses zero horizontal margin **and** stretches the
///   rule to the whole width, overriding the authored `width` (the
///   [`HrAlignment::Full`] contract, matching the terminal `RuleAlignment::Full`
///   mapping).
/// - `width`: The CSS width (e.g., `"100%"`, `"50%"`, `"20ch"`).
///   `None` — or any value that is not a safe CSS length/percentage — falls
///   back to `"100%"`. Ignored when `alignment` is `Full`, which always
///   resolves to `"100%"`.
/// - `color`: The CSS color (e.g., `"red"`, `"#336699"`).
///   `None` — or any value that is not a safe CSS color — falls back to
///   `"currentColor"`.
/// - `margin_top`: Top margin CSS value (e.g., `"0"`, `"1em"`).
/// - `margin_bottom`: Bottom margin CSS value.
///
/// ## Safety
///
/// The emitted SVG is passed through unescaped (the browser renderer treats it
/// as raw HTML). `width` and `color` can originate from untrusted document
/// thematic-break styling, so they are whitelist-validated by
/// [`is_safe_css_dimension`] / [`is_safe_css_color`] before interpolation. The
/// accepted character sets contain none of `"`, `'`, `<`, `>`, or `;`, so a
/// validated value cannot break out of the SVG `width` attribute, the inline
/// `style` declaration, or the `var(--hr-color, …)` fallback context. Anything
/// that fails validation is replaced with the safe default rather than emitted.
/// `kind`, `weight`, and `alignment` are closed enums and are never
/// interpolated from caller text.
///
/// ## Examples
///
/// ```
/// use renderable::tree::graphics::horizontal_rule_svg;
/// use renderable::tree::{HrAlignment, HrKind, HrWeight};
///
/// let svg = horizontal_rule_svg(
///     Some(HrKind::Waves), Some(HrWeight::Thick), Some(HrAlignment::Left),
///     Some("75%"), Some("blue"), "0", "0",
/// );
/// assert!(svg.contains(r#"width="75%""#));
/// assert!(svg.contains("--hr-weight: 8"));
/// assert!(svg.contains("--hr-color: blue"));
/// // Left alignment anchors the rule: zero left margin, auto right.
/// assert!(svg.contains("margin: 0 auto 0 0"));
/// ```
pub fn horizontal_rule_svg(
    kind: Option<HrKind>,
    weight: Option<HrWeight>,
    alignment: Option<HrAlignment>,
    width: Option<&str>,
    color: Option<&str>,
    margin_top: &str,
    margin_bottom: &str,
) -> String {
    let stroke_width = match weight {
        Some(HrWeight::Thin) => "2",
        Some(HrWeight::Thick) => "8",
        _ => "4", // medium or unset
    };

    // Horizontal margins realize the authored alignment: `auto` on a side
    // pushes the rule away from it. `None`/`Center` keep the long-standing
    // centered default; `Left`/`Right` anchor; `Full` uses no horizontal
    // margin so the rule fills from the leading edge.
    let (margin_left, margin_right) = match alignment {
        Some(HrAlignment::Left) => ("0", "auto"),
        Some(HrAlignment::Right) => ("auto", "0"),
        Some(HrAlignment::Full) => ("0", "0"),
        Some(HrAlignment::Center) | None => ("auto", "auto"),
    };

    // `width` / `color` may be untrusted document hints; only emit them when
    // they pass the conservative CSS whitelist, otherwise fall back to the safe
    // default. See this function's `Safety` section. `Full` alignment stretches
    // the rule across the whole width regardless of the authored value, so it
    // always resolves to `100%` (the `HrAlignment::Full` cross-target contract).
    let width_attr = match alignment {
        Some(HrAlignment::Full) => "100%",
        _ => width.filter(|w| is_safe_css_dimension(w)).unwrap_or("100%"),
    };
    let color_attr = color
        .filter(|c| is_safe_css_color(c))
        .unwrap_or("currentColor");

    let stroke_var = format!("var(--hr-color, {})", color_attr);
    let width_var = format!("var(--hr-weight, {})", stroke_width);
    let fill_var = format!("var(--hr-color, {})", color_attr);

    let svg_content = match kind {
        Some(HrKind::Dots) => format!(
            r#"<line x1="0" y1="50%" x2="100%" y2="50%" stroke="{}" stroke-width="{}" stroke-linecap="round" stroke-dasharray="2,6"/>"#,
            stroke_var, width_var
        ),
        Some(HrKind::Waves) => format!(
            r#"<path d="M0 20 Q 10 10 20 20 T 40 20 T 60 20 T 80 20 T 100 20 T 120 20 T 140 20 T 160 20 T 180 20 T 200 20" stroke="{}" stroke-width="{}" fill="none" stroke-linecap="round"/>"#,
            stroke_var, width_var
        ),
        Some(HrKind::LineStar) => format!(
            r#"<line x1="0" y1="50%" x2="45%" y2="50%" stroke="{}" stroke-width="{}" stroke-linecap="round"/>
  <path d="M50% 35% L52% 45% L62% 45% L54% 52% L57% 62% L50% 55% L43% 62% L46% 52% L38% 45% L48% 45% Z" fill="{}"/>
  <line x1="55%" y1="50%" x2="100%" y2="50%" stroke="{}" stroke-width="{}" stroke-linecap="round"/>"#,
            stroke_var, width_var, fill_var, stroke_var, width_var
        ),
        Some(HrKind::LineCircle) => format!(
            r#"<line x1="0" y1="50%" x2="45%" y2="50%" stroke="{}" stroke-width="{}" stroke-linecap="round"/>
  <circle cx="50%" cy="50%" r="8" fill="none" stroke="{}" stroke-width="{}"/>
  <line x1="55%" y1="50%" x2="100%" y2="50%" stroke="{}" stroke-width="{}" stroke-linecap="round"/>"#,
            stroke_var, width_var, stroke_var, width_var, stroke_var, width_var
        ),
        Some(HrKind::InsetLine) => format!(
            r#"<line x1="10%" y1="50%" x2="90%" y2="50%" stroke="{}" stroke-width="{}" stroke-linecap="round"/>"#,
            stroke_var, width_var
        ),
        Some(HrKind::CurtainRod) => format!(
            r#"<line x1="5%" y1="50%" x2="95%" y2="50%" stroke="{}" stroke-width="{}" stroke-linecap="round"/>
  <circle cx="5%" cy="50%" r="4" fill="{}"/>
  <circle cx="95%" cy="50%" r="4" fill="{}"/>"#,
            stroke_var, width_var, fill_var, fill_var
        ),
        // Dashes or unset.
        _ => format!(
            r#"<line x1="0" y1="50%" x2="100%" y2="50%" stroke="{}" stroke-width="{}" stroke-linecap="round" stroke-dasharray="8,4"/>"#,
            stroke_var, width_var
        ),
    };

    format!(
        r#"<svg class="darkmatter-hr" width="{width}" height="40" xmlns="http://www.w3.org/2000/svg" style="display: block; margin: {top} {right} {bot} {left}; --hr-weight: {weight}; --hr-color: {color}; --hr-width: {width};">
  {content}
</svg>"#,
        width = width_attr,
        top = margin_top,
        right = margin_right,
        bot = margin_bottom,
        left = margin_left,
        weight = stroke_width,
        color = color_attr,
        content = svg_content,
    )
}

/// Returns `true` when `value` is a safe CSS `<length-percentage>`: an optional
/// integer/decimal number followed by an optional known unit (`%`, `px`, `em`,
/// …). The accepted character set is digits, a single `.`, and ASCII-letter
/// units — no quote, angle-bracket, or `;` can appear, so a value that passes
/// is safe to interpolate into both an HTML attribute and a CSS declaration.
pub fn is_safe_css_dimension(value: &str) -> bool {
    let value = value.trim();
    let bytes = value.as_bytes();
    let mut i = 0;
    let mut saw_digit = false;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
        saw_digit = true;
    }
    if i < bytes.len() && bytes[i] == b'.' {
        i += 1;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
            saw_digit = true;
        }
    }
    if !saw_digit {
        return false;
    }
    matches!(
        &value[i..],
        "" | "%"
            | "px"
            | "em"
            | "rem"
            | "ch"
            | "ex"
            | "vw"
            | "vh"
            | "vmin"
            | "vmax"
            | "cm"
            | "mm"
            | "in"
            | "pt"
            | "pc"
    )
}

/// Returns `true` when `value` is a safe CSS `<color>`: a hex literal
/// (`#rgb` / `#rgba` / `#rrggbb` / `#rrggbbaa`), an all-letter keyword
/// (`red`, `currentColor`, `transparent`, …), or an `rgb()`/`rgba()`/`hsl()`/
/// `hsla()` function whose interior is restricted to digits, `.`, `,`, `%`,
/// `/`, and spaces. None of the accepted forms can contain a quote,
/// angle-bracket, or `;`, so a value that passes cannot break out of the SVG
/// attribute, the inline `style`, or the `var(--hr-color, …)` fallback.
pub fn is_safe_css_color(value: &str) -> bool {
    let value = value.trim();
    if value.is_empty() {
        return false;
    }
    if let Some(hex) = value.strip_prefix('#') {
        return matches!(hex.len(), 3 | 4 | 6 | 8) && hex.bytes().all(|b| b.is_ascii_hexdigit());
    }
    if value.bytes().all(|b| b.is_ascii_alphabetic()) {
        return true;
    }
    for func in ["rgba", "rgb", "hsla", "hsl"] {
        if let Some(rest) = value.strip_prefix(func)
            && let Some(inner) = rest.strip_prefix('(').and_then(|s| s.strip_suffix(')'))
        {
            return !inner.is_empty()
                && inner
                    .bytes()
                    .all(|b| b.is_ascii_digit() || matches!(b, b'.' | b',' | b'%' | b' ' | b'/'));
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_produces_dashes_svg() {
        let svg = horizontal_rule_svg(None, None, None, None, None, "0", "0");
        assert!(svg.contains(r#"width="100%""#));
        assert!(svg.contains("--hr-weight: 4"));
        assert!(svg.contains("--hr-color: currentColor"));
        assert!(svg.contains("stroke-dasharray=\"8,4\""));
    }

    #[test]
    fn waves_with_thick_and_custom_color() {
        let svg = horizontal_rule_svg(
            Some(HrKind::Waves),
            Some(HrWeight::Thick),
            None,
            Some("75%"),
            Some("blue"),
            "0",
            "0",
        );
        assert!(svg.contains(r#"width="75%""#));
        assert!(svg.contains("--hr-weight: 8"));
        assert!(svg.contains("--hr-color: blue"));
        assert!(svg.contains(r#"d="M0 20 Q 10 10 20 20"#));
    }

    #[test]
    fn dots_with_thin() {
        let svg =
            horizontal_rule_svg(Some(HrKind::Dots), Some(HrWeight::Thin), None, None, None, "0", "0");
        assert!(svg.contains("--hr-weight: 2"));
        assert!(svg.contains("stroke-dasharray=\"2,6\""));
    }

    #[test]
    fn line_star_produces_star_path() {
        let svg = horizontal_rule_svg(Some(HrKind::LineStar), None, None, None, None, "0", "0");
        assert!(svg.contains("L52% 45%"));
        assert!(svg.contains("Z"));
    }

    #[test]
    fn line_circle_produces_circle() {
        let svg = horizontal_rule_svg(Some(HrKind::LineCircle), None, None, None, None, "0", "0");
        assert!(svg.contains("<circle cx=\"50%\" cy=\"50%\" r=\"8\""));
    }

    #[test]
    fn inset_line_produces_short_line() {
        let svg = horizontal_rule_svg(Some(HrKind::InsetLine), None, None, None, None, "0", "0");
        assert!(svg.contains("x1=\"10%\""));
        assert!(svg.contains("x2=\"90%\""));
    }

    #[test]
    fn curtain_rod_produces_circles_at_ends() {
        let svg = horizontal_rule_svg(Some(HrKind::CurtainRod), None, None, None, None, "0", "0");
        assert!(svg.contains("cx=\"5%\""));
        assert!(svg.contains("cx=\"95%\""));
    }

    #[test]
    fn none_kind_falls_back_to_dashes() {
        let svg = horizontal_rule_svg(None, None, None, None, None, "0", "0");
        assert!(svg.contains("stroke-dasharray=\"8,4\""));
    }

    #[test]
    fn margins_are_included_in_style() {
        let svg = horizontal_rule_svg(None, None, None, None, None, "1em", "2em");
        assert!(svg.contains("margin: 1em auto 2em auto"));
    }

    /// Alignment drives the horizontal margins so a non-full rule anchors
    /// instead of always centering (review-1 finding 1).
    #[test]
    fn alignment_controls_horizontal_margins() {
        let at = |a| horizontal_rule_svg(None, None, a, Some("50%"), None, "0", "0");
        // None and Center both center.
        assert!(at(None).contains("margin: 0 auto 0 auto"));
        assert!(at(Some(HrAlignment::Center)).contains("margin: 0 auto 0 auto"));
        // Left anchors to the leading edge, Right to the trailing edge.
        assert!(at(Some(HrAlignment::Left)).contains("margin: 0 auto 0 0"));
        assert!(at(Some(HrAlignment::Right)).contains("margin: 0 0 0 auto"));
        // Full uses zero horizontal margin.
        assert!(at(Some(HrAlignment::Full)).contains("margin: 0 0 0 0"));
    }

    /// `Full` stretches the rule across the whole width: it overrides the
    /// authored width with `100%` so a `full` rule is full-width, not a
    /// left-anchored partial rule (review-2 finding 1).
    #[test]
    fn full_alignment_overrides_authored_width_to_full() {
        // A narrow authored width that any other alignment would honor.
        let full = horizontal_rule_svg(None, None, Some(HrAlignment::Full), Some("50%"), None, "0", "0");
        assert!(full.contains(r#"width="100%""#), "full must force 100% width: {full}");
        assert!(full.contains("--hr-width: 100%"), "full must force the --hr-width var: {full}");
        assert!(full.contains("margin: 0 0 0 0"), "full keeps zero horizontal margin: {full}");

        // The same narrow width is preserved for a non-full alignment.
        let left = horizontal_rule_svg(None, None, Some(HrAlignment::Left), Some("50%"), None, "0", "0");
        assert!(left.contains(r#"width="50%""#), "non-full alignment keeps the authored width: {left}");
    }

    #[test]
    fn stroke_and_fill_use_css_variables() {
        let svg =
            horizontal_rule_svg(Some(HrKind::LineStar), None, None, None, Some("red"), "0", "0");
        assert!(svg.contains("stroke=\"var(--hr-color, red)\""));
        assert!(svg.contains("fill=\"var(--hr-color, red)\""));
    }

    #[test]
    fn safe_css_dimension_accepts_lengths_and_percentages() {
        for ok in ["100%", "50%", "20ch", "300px", "1.5rem", "0", "12.25vw"] {
            assert!(is_safe_css_dimension(ok), "{ok} should be accepted");
        }
    }

    #[test]
    fn safe_css_dimension_rejects_hostile_values() {
        for bad in [
            "\"><script>alert(1)</script>",
            "100%\" onload=\"alert(1)",
            "100%;background:url(x)",
            "calc(100% - 1px)",
            "url(x)",
            "",
            "px",
            "100zz",
        ] {
            assert!(!is_safe_css_dimension(bad), "{bad:?} should be rejected");
        }
    }

    #[test]
    fn safe_css_color_accepts_valid_colors() {
        for ok in [
            "red",
            "currentColor",
            "transparent",
            "#fff",
            "#336699",
            "#11223344",
            "rgb(1,2,3)",
            "rgba(1, 2, 3, 0.5)",
            "hsl(120, 50%, 50%)",
        ] {
            assert!(is_safe_css_color(ok), "{ok} should be accepted");
        }
    }

    #[test]
    fn safe_css_color_rejects_hostile_values() {
        for bad in [
            "red\"><script>alert(1)</script>",
            "red; } body { display:none",
            "rgb(1,2,3);x:y",
            "url(javascript:alert(1))",
            "#xyz",
            "#1234567",
            "expression(alert(1))",
            "",
        ] {
            assert!(!is_safe_css_color(bad), "{bad:?} should be rejected");
        }
    }

    /// A hostile `width` / `color` must never reach the emitted SVG: the helper
    /// falls back to the safe defaults, so the raw-HTML string carries no
    /// attacker-controlled quote, angle bracket, or extra declaration.
    #[test]
    fn hostile_width_and_color_fall_back_to_safe_defaults() {
        let svg = horizontal_rule_svg(
            Some(HrKind::Waves),
            Some(HrWeight::Thick),
            None,
            Some("100%\"><script>alert(1)</script>"),
            Some("red\" onload=\"alert(1)"),
            "0",
            "0",
        );
        assert!(!svg.contains("<script"), "no injected script: {svg}");
        assert!(!svg.contains("onload"), "no injected handler: {svg}");
        assert!(!svg.contains("alert(1)"), "no injected payload: {svg}");
        // The single emitted `<svg …>` opening tag is the only `<` before the
        // shape primitive; the attacker's `<` never appears.
        assert!(svg.contains(r#"width="100%""#), "fell back to 100%: {svg}");
        assert!(
            svg.contains("--hr-color: currentColor"),
            "fell back to currentColor: {svg}"
        );
    }
}
