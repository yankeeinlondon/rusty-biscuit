use renderable::browser::fragment::{BrowserFragment, Ready};

use crate::components::renderable::BrowserRenderable;
use crate::utils::color::{BasicColor, RgbColor};
use crate::utils::layout::{Length, TargetValue};

use super::HorizontalRule;
use super::style::{RuleStyle, RuleWeight};

/// Parses a CSS basic-16 color name (case-insensitive) into a [`BasicColor`].
/// Also accepts `gray` / `grey` as aliases for `BrightBlack`.
///
/// ## Notes
///
/// Matches without allocating — `eq_ignore_ascii_case` compares in place, so a
/// rule rendered on every paragraph break in a large document no longer pays
/// a per-call `String` allocation for the comparison.
pub(super) fn parse_basic_color(raw: &str) -> Option<BasicColor> {
    // Static table keeps the allocation amortized across matches and makes
    // extending the list a one-line change. Hyphenated and concatenated
    // bright variants are kept alongside the canonical names for
    // compatibility with existing callers.
    const PAIRS: &[(&str, BasicColor)] = &[
        ("black", BasicColor::Black),
        ("red", BasicColor::Red),
        ("green", BasicColor::Green),
        ("yellow", BasicColor::Yellow),
        ("blue", BasicColor::Blue),
        ("magenta", BasicColor::Magenta),
        ("cyan", BasicColor::Cyan),
        ("white", BasicColor::White),
        ("gray", BasicColor::BrightBlack),
        ("grey", BasicColor::BrightBlack),
        ("bright-black", BasicColor::BrightBlack),
        ("brightblack", BasicColor::BrightBlack),
        ("bright-red", BasicColor::BrightRed),
        ("brightred", BasicColor::BrightRed),
        ("bright-green", BasicColor::BrightGreen),
        ("brightgreen", BasicColor::BrightGreen),
        ("bright-yellow", BasicColor::BrightYellow),
        ("brightyellow", BasicColor::BrightYellow),
        ("bright-blue", BasicColor::BrightBlue),
        ("brightblue", BasicColor::BrightBlue),
        ("bright-magenta", BasicColor::BrightMagenta),
        ("brightmagenta", BasicColor::BrightMagenta),
        ("bright-cyan", BasicColor::BrightCyan),
        ("brightcyan", BasicColor::BrightCyan),
        ("bright-white", BasicColor::BrightWhite),
        ("brightwhite", BasicColor::BrightWhite),
    ];
    PAIRS
        .iter()
        .find(|(name, _)| raw.eq_ignore_ascii_case(name))
        .map(|(_, color)| *color)
}

/// Parses a `#rrggbb` hex color into an [`RgbColor`] with a nearest-primary
/// [`BasicColor`] fallback for non-truecolor terminals. Returns `None` if the
/// string does not match `#` + 6 hex digits.
pub(super) fn parse_hex_color(raw: &str) -> Option<RgbColor> {
    let hex = raw.strip_prefix('#')?;
    if hex.len() != 6 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    let fallback = nearest_basic_color(r, g, b);
    Some(RgbColor::new(r, g, b, fallback))
}

/// Maps an RGB triple to its nearest basic ANSI foreground color. This is a
/// coarse nearest-primary heuristic — sufficient for horizontal rule
/// fallbacks where fidelity is secondary to "picks a reasonable color".
pub(super) fn nearest_basic_color(r: u8, g: u8, b: u8) -> BasicColor {
    let max = r.max(g).max(b);
    if max < 64 {
        return BasicColor::Black;
    }
    let bright = max >= 192;
    let r_on = r >= max.saturating_sub(64);
    let g_on = g >= max.saturating_sub(64);
    let b_on = b >= max.saturating_sub(64);
    match (r_on, g_on, b_on, bright) {
        (true, true, true, true) => BasicColor::BrightWhite,
        (true, true, true, false) => BasicColor::White,
        (true, false, false, true) => BasicColor::BrightRed,
        (true, false, false, false) => BasicColor::Red,
        (false, true, false, true) => BasicColor::BrightGreen,
        (false, true, false, false) => BasicColor::Green,
        (false, false, true, true) => BasicColor::BrightBlue,
        (false, false, true, false) => BasicColor::Blue,
        (true, true, false, true) => BasicColor::BrightYellow,
        (true, true, false, false) => BasicColor::Yellow,
        (true, false, true, true) => BasicColor::BrightMagenta,
        (true, false, true, false) => BasicColor::Magenta,
        (false, true, true, true) => BasicColor::BrightCyan,
        (false, true, true, false) => BasicColor::Cyan,
        (false, false, false, _) => BasicColor::Black,
    }
}

impl HorizontalRule {
    /// Renders the rule as an SVG `<svg>` element with CSS-variable-driven
    /// styling.
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
    /// correctly even when the declared inline variables are stripped —
    /// the fallback inside each `var()` expression is the concrete value
    /// that was baked in at render time. The page declares the variables;
    /// the component emits `var(--foo)` literally.
    ///
    /// ## Notes
    ///
    /// Geometry attributes (`x1`, `x2`, `cx`, `cy`, `r`, `d`, ...) remain
    /// concrete values because not every SVG renderer honors `var()` inside
    /// geometry properties. Only color and stroke-width are variable-driven.
    pub(crate) fn render_browser_svg(&self) -> String {
        // Weight in pixels (used both as the declared --hr-weight value and
        // as the fallback inside every var(--hr-weight, N) expression).
        let stroke_width = match self.weight {
            RuleWeight::Thin => "2",
            RuleWeight::Medium => "4",
            RuleWeight::Thick => "8",
        };

        let width_attr = self.width.as_deref().unwrap_or("100%");
        let color_attr = self.color.as_deref().unwrap_or("currentColor");
        let margin_top = self.layout.margin.top.to_css_value("0");
        let margin_bottom = self.layout.margin.bottom.to_css_value("0");

        // Every `stroke`, `fill`, and `stroke-width` expression goes through
        // these `var(--hr-xxx, FALLBACK)` forms so page-level CSS that sets
        // `--hr-weight` / `--hr-color` takes effect without the SVG losing
        // its visual fidelity when the variables are stripped.
        let stroke_var = format!("var(--hr-color, {})", color_attr);
        let width_var = format!("var(--hr-weight, {})", stroke_width);
        let fill_var = format!("var(--hr-color, {})", color_attr);

        let svg_content = match &self.style {
            RuleStyle::Dashes => {
                format!(
                    r#"<line x1="0" y1="50%" x2="100%" y2="50%" stroke="{}" stroke-width="{}" stroke-linecap="round" stroke-dasharray="8,4"/>"#,
                    stroke_var, width_var
                )
            }
            RuleStyle::Dots => {
                format!(
                    r#"<line x1="0" y1="50%" x2="100%" y2="50%" stroke="{}" stroke-width="{}" stroke-linecap="round" stroke-dasharray="2,6"/>"#,
                    stroke_var, width_var
                )
            }
            RuleStyle::Waves => {
                format!(
                    r#"<path d="M0 20 Q 10 10 20 20 T 40 20 T 60 20 T 80 20 T 100 20 T 120 20 T 140 20 T 160 20 T 180 20 T 200 20" stroke="{}" stroke-width="{}" fill="none" stroke-linecap="round"/>"#,
                    stroke_var, width_var
                )
            }
            RuleStyle::LineStar => {
                format!(
                    r#"<line x1="0" y1="50%" x2="45%" y2="50%" stroke="{}" stroke-width="{}" stroke-linecap="round"/>
  <path d="M50% 35% L52% 45% L62% 45% L54% 52% L57% 62% L50% 55% L43% 62% L46% 52% L38% 45% L48% 45% Z" fill="{}"/>
  <line x1="55%" y1="50%" x2="100%" y2="50%" stroke="{}" stroke-width="{}" stroke-linecap="round"/>"#,
                    stroke_var, width_var, fill_var, stroke_var, width_var
                )
            }
            RuleStyle::LineCircle => {
                format!(
                    r#"<line x1="0" y1="50%" x2="45%" y2="50%" stroke="{}" stroke-width="{}" stroke-linecap="round"/>
  <circle cx="50%" cy="50%" r="8" fill="none" stroke="{}" stroke-width="{}"/>
  <line x1="55%" y1="50%" x2="100%" y2="50%" stroke="{}" stroke-width="{}" stroke-linecap="round"/>"#,
                    stroke_var, width_var, stroke_var, width_var, stroke_var, width_var
                )
            }
            RuleStyle::InsetLine => {
                format!(
                    r#"<line x1="10%" y1="50%" x2="90%" y2="50%" stroke="{}" stroke-width="{}" stroke-linecap="round"/>"#,
                    stroke_var, width_var
                )
            }
            RuleStyle::CurtainRod => {
                format!(
                    r#"<line x1="5%" y1="50%" x2="95%" y2="50%" stroke="{}" stroke-width="{}" stroke-linecap="round"/>
  <circle cx="5%" cy="50%" r="4" fill="{}"/>
  <circle cx="95%" cy="50%" r="4" fill="{}"/>"#,
                    stroke_var, width_var, fill_var, fill_var
                )
            }
        };

        // The outer <svg>'s `width` attribute remains a concrete value — some
        // renderers don't honor `var()` inside geometry attributes. The
        // `--hr-width` variable is still declared for downstream CSS that
        // may want to use it (e.g., authors styling the ancestor).
        format!(
            r#"<svg width="{width}" height="40" xmlns="http://www.w3.org/2000/svg" style="display: block; margin: {top} auto {bot} auto; --hr-weight: {weight}; --hr-color: {color}; --hr-width: {width};">
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
}

impl BrowserRenderable for HorizontalRule {
    /// Wraps the rule's SVG as a [`ComposableNode::RawHtml`] island.
    ///
    /// The SVG is caller-owned, already-formed markup; emitting it as a
    /// `RawHtml` node tells the renderer to pass it through verbatim
    /// rather than HTML-escaping it.
    ///
    /// [`ComposableNode::RawHtml`]: renderable::browser::fragment::ComposableNode::RawHtml
    fn render_html_fragment(&self) -> BrowserFragment<Ready> {
        BrowserFragment::new()
            .define_as_raw_html(self.render_browser_svg())
            .finalize()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

// Helper trait extension for a `TargetValue<Length>` margin to convert to a
// CSS value, resolved for the [`RenderTarget::Browser`] target.
pub(super) trait MarginToCss {
    fn to_css_value(&self, default: &str) -> String;
}

impl MarginToCss for TargetValue<Length> {
    fn to_css_value(&self, default: &str) -> String {
        use renderable::target::RenderTarget;
        match self.resolve(RenderTarget::Browser) {
            Some(Length::Zero) | None => default.to_string(),
            Some(Length::Ch(n)) => format!("{}ch", n),
            Some(Length::Percent(pct)) => format!("{}%", pct),
            Some(Length::Css(sizing)) => sizing.to_string(),
        }
    }
}
