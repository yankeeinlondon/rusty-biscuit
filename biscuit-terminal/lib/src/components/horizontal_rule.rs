use std::collections::HashMap;

use crate::components::renderable::{BrowserRenderable, Renderable};
use crate::discovery::detection::ColorDepth;
use crate::terminal::Terminal;
use crate::utils::color::{BasicColor, RgbColor, TermColor};
use crate::utils::layout::{Layout, Margin};

/// Defines the visual style of a horizontal rule.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "kebab-case"))]
pub enum RuleStyle {
    /// Simple dashed line: ---
    Dashes,
    /// Dotted line: ···
    Dots,
    /// Wavy line using Unicode characters.
    ///
    /// ## Notes
    ///
    /// Waves has no heavy Unicode variant — `RuleWeight::Thick` produces the
    /// same body in the terminal as `RuleWeight::Medium`. Weight affects only
    /// browser rendering (stroke width) for this style. ASCII fallback (`~`)
    /// also has no heavy variant.
    Waves,
    /// Line with star symbols: * * *
    LineStar,
    /// Line with circle symbols: ○ ○ ○
    LineCircle,
    /// Inset line with border effect
    InsetLine,
    /// Curtain rod style with decorative ends
    CurtainRod,
}

/// Defines the placement of a horizontal rule within the available width.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum RulePlacement {
    /// Span the full available width
    Full,
    /// Centered with equal margins on both sides
    Centered,
    /// Aligned to the left edge
    Left,
    /// Aligned to the right edge
    Right,
}

/// Defines the visual weight (thickness) of a horizontal rule.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum RuleWeight {
    /// Thin line (2px stroke in browser, single-line chars in terminal).
    Thin,
    /// Medium line (4px stroke in browser, single-line chars in terminal).
    Medium,
    /// Thick line (8px stroke in browser, heavy/double chars in terminal).
    Thick,
}

/// A horizontal rule component for terminal and browser rendering.
#[derive(Debug, Clone)]
pub struct HorizontalRule {
    style: RuleStyle,
    placement: RulePlacement,
    weight: RuleWeight,
    width: Option<String>, // CSS-like width specification (e.g., "50%", "200px")
    color: Option<String>, // CSS color specification
    layout: Layout,
}

impl HorizontalRule {
    /// Creates a new horizontal rule with default settings.
    ///
    /// ## Examples
    ///
    /// ```
    /// use biscuit_terminal::prelude::*;
    ///
    /// let rule = HorizontalRule::new()
    ///     .style(RuleStyle::Waves)
    ///     .placement(RulePlacement::Centered)
    ///     .weight(RuleWeight::Medium)
    ///     .width("75%");
    ///
    /// let term = Terminal::default();
    /// let _ = rule.render(&term);
    /// ```
    pub fn new() -> Self {
        Self {
            style: RuleStyle::Dashes,
            placement: RulePlacement::Full,
            weight: RuleWeight::Medium,
            width: None,
            color: None,
            layout: Layout::default(),
        }
    }

    /// Sets the visual style of the rule.
    pub fn style(mut self, style: RuleStyle) -> Self {
        self.style = style;
        self
    }

    /// Sets the placement of the rule.
    pub fn placement(mut self, placement: RulePlacement) -> Self {
        self.placement = placement;
        self
    }

    /// Sets the weight (thickness) of the rule.
    pub fn weight(mut self, weight: RuleWeight) -> Self {
        self.weight = weight;
        self
    }

    /// Sets the width of the rule as a CSS-like string.
    pub fn width(mut self, width: impl Into<String>) -> Self {
        self.width = Some(width.into());
        self
    }

    /// Sets the color of the rule as a CSS color string.
    pub fn color(mut self, color: impl Into<String>) -> Self {
        self.color = Some(color.into());
        self
    }
}

impl Default for HorizontalRule {
    fn default() -> Self {
        Self::new()
    }
}

impl Renderable for HorizontalRule {
    fn render(&self, term: &Terminal) -> String {
        // Tier 1: SVG→PNG via resvg with TerminalImage
        // If terminal supports images, we could generate an SVG and render it.
        // For now, we use Tier 2/3 which are more universally compatible.

        // Determine the width based on placement, custom width, and terminal width
        let term_width = term.width() as usize;
        let rule_width = self.resolve_width(term_width);

        // Clamp to reasonable minimum and maximum
        let rule_width = rule_width.clamp(10, term_width);

        // Generate the rule content based on style and terminal capabilities
        let rule_content = self.generate_terminal_content(rule_width, term);

        // Wrap the rule content with ANSI color escapes when a color is set
        // and the terminal advertises any color support. Padding stays
        // outside the color wrap so the trailing reset comes *before* the
        // placement padding, not after.
        let rule_content = self.apply_terminal_color(rule_content, term);

        // Apply placement (using character count, not byte length)
        let content_width = self.visible_width(&rule_content);
        match self.placement {
            RulePlacement::Full => rule_content,
            RulePlacement::Centered => {
                let padding = (term_width.saturating_sub(content_width)) / 2;
                format!("{}{}", " ".repeat(padding), rule_content)
            }
            RulePlacement::Left => rule_content,
            RulePlacement::Right => {
                let padding = term_width.saturating_sub(content_width);
                format!("{}{}", " ".repeat(padding), rule_content)
            }
        }
    }

    fn layout(&self) -> &Layout {
        &self.layout
    }

    fn layout_mut(&mut self) -> &mut Layout {
        &mut self.layout
    }

    fn is_block_level(&self) -> bool {
        true
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl HorizontalRule {
    /// Resolves the width of the rule in characters based on the terminal width
    /// and the CSS-like width specification.
    fn resolve_width(&self, term_width: usize) -> usize {
        match &self.width {
            Some(width_str) => {
                let trimmed = width_str.trim();
                // Handle percentage: "50%"
                if let Some(pct_str) = trimmed.strip_suffix('%')
                    && let Ok(pct) = pct_str.trim().parse::<f32>()
                {
                    return ((term_width as f32 * pct / 100.0) as usize).max(1);
                }
                // Handle character width: "20ch" or "20"
                let num_str = if let Some(s) = trimmed.strip_suffix("ch") {
                    s.trim()
                } else {
                    trimmed
                };
                if let Ok(chars) = num_str.parse::<usize>() {
                    return chars.max(1);
                }
                // Fallback for unparseable width
                term_width
            }
            None => {
                // Default width based on placement
                match self.placement {
                    RulePlacement::Full => term_width,
                    RulePlacement::Centered | RulePlacement::Left | RulePlacement::Right => {
                        (term_width as f32 * 0.8) as usize
                    }
                }
            }
        }
    }

    /// Generates terminal content for the horizontal rule based on style and width.
    ///
    /// Uses a 3-tier progressive enhancement:
    /// - Tier 2: Unicode characters for modern terminals
    /// - Tier 3: ASCII characters for restricted environments
    ///
    /// ## Notes
    ///
    /// `RuleWeight::Thick` swaps in heavy Unicode variants (`╍`, `•`, `━`)
    /// when `fancy` is true. `RuleStyle::Waves` has no heavy variant and
    /// renders identically regardless of weight. ASCII fallbacks also have
    /// no heavy variant — ASCII output is weight-insensitive.
    fn generate_terminal_content(&self, width: usize, term: &Terminal) -> String {
        let fancy = self.use_fancy_chars(term);
        let heavy = self.heavy();

        match &self.style {
            RuleStyle::Dashes => {
                if fancy {
                    if heavy { "╍" } else { "╌" }.repeat(width)
                } else {
                    "-".repeat(width)
                }
            }
            RuleStyle::Dots => {
                if fancy {
                    if heavy { "•" } else { "·" }.repeat(width)
                } else {
                    ".".repeat(width)
                }
            }
            RuleStyle::Waves => {
                // No heavy Unicode variant exists; weight is ignored here.
                if fancy {
                    "≋".repeat(width)
                } else {
                    "~".repeat(width)
                }
            }
            RuleStyle::LineStar => {
                if fancy {
                    // Pattern: ────★──── / ━━━━★━━━━ when heavy.
                    let line_char = if heavy { '━' } else { '─' };
                    let star = '★';
                    Self::centered_symbol_pattern(width, line_char, star)
                } else {
                    // Pattern: ---[*]---
                    let line_char = '-';
                    let star = '*';
                    Self::centered_symbol_pattern(width, line_char, star)
                }
            }
            RuleStyle::LineCircle => {
                if fancy {
                    // Pattern: ────●──── / ━━━━●━━━━ when heavy.
                    let line_char = if heavy { '━' } else { '─' };
                    let circle = '●';
                    Self::centered_symbol_pattern(width, line_char, circle)
                } else {
                    // Pattern: ---(o)---
                    let line_char = '-';
                    let circle = 'o';
                    Self::centered_symbol_pattern(width, line_char, circle)
                }
            }
            RuleStyle::InsetLine => {
                if width < 4 {
                    "-".repeat(width)
                } else {
                    let inner_width = width - 4;
                    let line = if fancy {
                        if heavy { "━" } else { "─" }
                    } else {
                        "-"
                    };
                    format!("  {}  ", line.repeat(inner_width))
                }
            }
            RuleStyle::CurtainRod => {
                if width < 5 {
                    if fancy {
                        "═".repeat(width)
                    } else {
                        "=".repeat(width)
                    }
                } else {
                    let inner_width = width.saturating_sub(4);
                    let line_char = if fancy {
                        if heavy { '━' } else { '─' }
                    } else {
                        '-'
                    };
                    // Use single-width box-drawing tees (┤ / ├) as curtain-rod end
                    // caps; the CJK corner brackets 「」 are East-Asian wide and
                    // skew terminal layout. ASCII falls back to `[` / `]`.
                    // Bracket characters are weight-agnostic — they already
                    // visually terminate the line.
                    let left_bracket = if fancy { '┤' } else { '[' };
                    let right_bracket = if fancy { '├' } else { ']' };
                    format!(
                        "{}{}{}",
                        left_bracket,
                        line_char.to_string().repeat(inner_width),
                        right_bracket
                    )
                }
            }
        }
    }

    /// Returns `true` if the configured weight should use the heavy Unicode
    /// variant of the chosen style (when a heavy variant exists).
    fn heavy(&self) -> bool {
        matches!(self.weight, RuleWeight::Thick)
    }

    /// Wraps `content` with ANSI color escapes derived from `self.color`
    /// when both (a) a color is configured and (b) the terminal advertises
    /// any color support. Returns `content` unchanged otherwise.
    ///
    /// Named CSS basic-16 colors map to [`BasicColor`]. `#rrggbb` hex maps to
    /// [`RgbColor`] (truecolor terminals) or to the nearest [`BasicColor`]
    /// otherwise. Unrecognized strings log a `tracing::warn!` and leave
    /// `content` uncolored.
    fn apply_terminal_color(&self, content: String, term: &Terminal) -> String {
        let raw = match self.color.as_deref() {
            Some(c) => c.trim(),
            None => return content,
        };
        if raw.is_empty() {
            return content;
        }
        if matches!(term.color_depth, ColorDepth::None) {
            return content;
        }

        // Hex: #rrggbb
        if let Some(rgb) = parse_hex_color(raw) {
            return match term.color_depth {
                ColorDepth::TrueColor => rgb.fg(content),
                // Downgrade to the nearest basic color via the RGB's embedded
                // fallback (currently nearest-primary; good enough for HRs).
                _ => rgb.fallback().fg(content),
            };
        }

        // Named CSS basic-16 colors (+ "gray"/"grey").
        if let Some(basic) = parse_basic_color(raw) {
            return basic.fg(content);
        }

        tracing::warn!(
            color = %raw,
            "unknown horizontal rule color string; rendering without ANSI wrapping"
        );
        content
    }

    /// Counts the number of *visible* columns in `content`, ignoring any
    /// ANSI CSI escape sequences that [`apply_terminal_color`] may have
    /// wrapped around the rule body. Used to compute placement padding.
    fn visible_width(&self, content: &str) -> usize {
        let mut count = 0usize;
        let mut chars = content.chars();
        while let Some(c) = chars.next() {
            if c == '\x1b' {
                // Skip CSI-introducer and body up to final byte (0x40..=0x7E).
                if chars.next() == Some('[') {
                    for cc in chars.by_ref() {
                        if matches!(cc, '\x40'..='\x7e') {
                            break;
                        }
                    }
                }
                continue;
            }
            count += 1;
        }
        count
    }

    /// Creates a centered symbol pattern like ────★────
    fn centered_symbol_pattern(width: usize, line_char: char, symbol: char) -> String {
        // Allocate the char → String conversion once and reuse it for both
        // padding sides instead of paying for it twice per call.
        let line = line_char.to_string();
        if width < 3 {
            return line.repeat(width);
        }
        let symbol_width = 1;
        let remaining = width.saturating_sub(symbol_width);
        let left_pad = remaining / 2;
        let right_pad = remaining - left_pad;
        format!(
            "{}{}{}",
            line.repeat(left_pad),
            symbol,
            line.repeat(right_pad)
        )
    }

    /// Returns `true` when the terminal is likely to render the "fancy"
    /// Unicode glyphs correctly, and `false` when we should emit the
    /// ASCII-only fallback.
    ///
    /// We treat a missing locale as UTF-8-capable because every modern
    /// terminal defaults to UTF-8. Explicit `C` / `POSIX` locales (or any
    /// non-UTF-8 locale) fall through to the ASCII fallback.
    fn use_fancy_chars(&self, _term: &Terminal) -> bool {
        crate::discovery::locale::env_says_utf8().unwrap_or(true)
    }
}

/// Parses a CSS basic-16 color name (case-insensitive) into a [`BasicColor`].
/// Also accepts `gray` / `grey` as aliases for `BrightBlack`.
fn parse_basic_color(raw: &str) -> Option<BasicColor> {
    match raw.to_ascii_lowercase().as_str() {
        "black" => Some(BasicColor::Black),
        "red" => Some(BasicColor::Red),
        "green" => Some(BasicColor::Green),
        "yellow" => Some(BasicColor::Yellow),
        "blue" => Some(BasicColor::Blue),
        "magenta" => Some(BasicColor::Magenta),
        "cyan" => Some(BasicColor::Cyan),
        "white" => Some(BasicColor::White),
        "gray" | "grey" | "bright-black" | "brightblack" => Some(BasicColor::BrightBlack),
        "bright-red" | "brightred" => Some(BasicColor::BrightRed),
        "bright-green" | "brightgreen" => Some(BasicColor::BrightGreen),
        "bright-yellow" | "brightyellow" => Some(BasicColor::BrightYellow),
        "bright-blue" | "brightblue" => Some(BasicColor::BrightBlue),
        "bright-magenta" | "brightmagenta" => Some(BasicColor::BrightMagenta),
        "bright-cyan" | "brightcyan" => Some(BasicColor::BrightCyan),
        "bright-white" | "brightwhite" => Some(BasicColor::BrightWhite),
        _ => None,
    }
}

/// Parses a `#rrggbb` hex color into an [`RgbColor`] with a nearest-primary
/// [`BasicColor`] fallback for non-truecolor terminals. Returns `None` if the
/// string does not match `#` + 6 hex digits.
fn parse_hex_color(raw: &str) -> Option<RgbColor> {
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
fn nearest_basic_color(r: u8, g: u8, b: u8) -> BasicColor {
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

impl BrowserRenderable for HorizontalRule {
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
    /// that was baked in at render time.
    ///
    /// Callers that want to override those values after generation can use
    /// [`render_to_browser_with_inline_variables`](Self::render_to_browser_with_inline_variables)
    /// which performs string substitution for `var(--name)` tokens.
    ///
    /// ## Notes
    ///
    /// Geometry attributes (`x1`, `x2`, `cx`, `cy`, `r`, `d`, ...) remain
    /// concrete values because not every SVG renderer honors `var()` inside
    /// geometry properties. Only color and stroke-width are variable-driven.
    fn render_to_browser(&self) -> String {
        // Weight in pixels (used both as the declared --hr-weight value and
        // as the fallback inside every var(--hr-weight, N) expression).
        let stroke_width = match self.weight {
            RuleWeight::Thin => "2",
            RuleWeight::Medium => "4",
            RuleWeight::Thick => "8",
        };

        let width_attr = self.width.as_deref().unwrap_or("100%");
        let color_attr = self.color.as_deref().unwrap_or("currentColor");
        let margin_top = self.layout.top_margin.to_css_value("0");
        let margin_bottom = self.layout.bottom_margin.to_css_value("0");

        // Every `stroke`, `fill`, and `stroke-width` expression goes through
        // these `var(--hr-xxx, FALLBACK)` forms so downstream overrides via
        // `render_to_browser_with_inline_variables` (or page-level CSS that
        // sets `--hr-weight` / `--hr-color`) take effect without the SVG
        // losing its visual fidelity when the variables are stripped.
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

    /// Renders the rule and then substitutes caller-provided CSS variables
    /// into any `var(--name)` token found in the output.
    ///
    /// Because [`render_to_browser`](Self::render_to_browser) now embeds
    /// `var(--hr-weight, …)`, `var(--hr-color, …)`, and declares
    /// `--hr-weight` / `--hr-color` / `--hr-width` on the root `<svg>`,
    /// callers get a natural override surface:
    ///
    /// - Key `"hr-weight"` replaces every `var(--hr-weight)` occurrence.
    /// - Key `"hr-color"` replaces every `var(--hr-color)` occurrence.
    /// - Key `"hr-width"` replaces every `var(--hr-width)` occurrence.
    ///
    /// The replacement is independent per key — `HashMap` iteration order
    /// does not affect the result because each `var(--name)` token is
    /// unique per key.
    ///
    /// ## Notes
    ///
    /// Tokens with embedded fallbacks (`var(--hr-weight, 4)`) are not
    /// substituted because their serialized form includes the fallback.
    /// Pass the bare `var(--hr-weight)` form if you want to be replaced.
    /// The substitution performed here targets the bare `var(--name)`
    /// form for backward compatibility with callers that pre-embed
    /// that exact token (e.g., `HorizontalRule::new().width("var(--rule-width)")`).
    fn render_to_browser_with_inline_variables(
        &self,
        variables: &HashMap<String, String>,
    ) -> String {
        // Apply CSS variables if provided
        let mut svg = self.render_to_browser();

        // Replace any placeholders with actual variables
        for (key, value) in variables {
            svg = svg.replace(&format!("var(--{})", key), value);
        }

        svg
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

// Helper trait extension for Margin to convert to CSS values
trait MarginToCss {
    fn to_css_value(&self, default: &str) -> String;
}

impl MarginToCss for Margin {
    fn to_css_value(&self, _default: &str) -> String {
        match self {
            Margin::Chars(chars) => format!("{}ch", chars),
            Margin::Percent(pct) => format!("{}%", pct),
            Margin::None => "0".to_string(),
            Margin::Offset(base, chars) => {
                // For Offset, we'll use the base margin plus the chars
                let base_value = base.to_css_value("0");
                if base_value == "0" {
                    format!("{}ch", chars)
                } else {
                    // This is a simplification - in a real implementation,
                    // we'd need to parse and combine the values properly
                    format!("{} + {}ch", base_value, chars)
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discovery::detection::ColorDepth;
    use crate::terminal::Terminal;
    use crate::utils::layout::Margin;
    use insta::assert_snapshot;

    /// RAII guard that overrides `LC_ALL` (and clears `LC_CTYPE` / `LANG`)
    /// for the duration of a single test, then restores the prior values
    /// on drop. Tests using this guard MUST also be marked
    /// `#[serial_test::serial(locale_env)]` because `std::env::set_var`
    /// is not safe to call concurrently with other threads that read the
    /// environment.
    struct ScopedLcAll {
        prev_lc_all: Option<String>,
        prev_lc_ctype: Option<String>,
        prev_lang: Option<String>,
    }

    impl ScopedLcAll {
        fn new(value: &str) -> Self {
            let prev_lc_all = std::env::var("LC_ALL").ok();
            let prev_lc_ctype = std::env::var("LC_CTYPE").ok();
            let prev_lang = std::env::var("LANG").ok();
            // SAFETY: Tests using this guard are marked
            // `#[serial_test::serial(locale_env)]`, so no other thread in
            // the test binary reads or writes these variables concurrently.
            unsafe {
                std::env::remove_var("LC_CTYPE");
                std::env::remove_var("LANG");
                std::env::set_var("LC_ALL", value);
            }
            Self {
                prev_lc_all,
                prev_lc_ctype,
                prev_lang,
            }
        }

        fn force_utf8() -> Self {
            Self::new("en_US.UTF-8")
        }

        fn force_c() -> Self {
            Self::new("C")
        }
    }

    impl Drop for ScopedLcAll {
        fn drop(&mut self) {
            // SAFETY: Serialized by `#[serial_test::serial(locale_env)]`.
            unsafe {
                match &self.prev_lc_all {
                    Some(v) => std::env::set_var("LC_ALL", v),
                    None => std::env::remove_var("LC_ALL"),
                }
                match &self.prev_lc_ctype {
                    Some(v) => std::env::set_var("LC_CTYPE", v),
                    None => std::env::remove_var("LC_CTYPE"),
                }
                match &self.prev_lang {
                    Some(v) => std::env::set_var("LANG", v),
                    None => std::env::remove_var("LANG"),
                }
            }
        }
    }

    #[test]
    fn test_horizontal_rule_new() {
        let hr = HorizontalRule::new();
        assert_eq!(hr.style, RuleStyle::Dashes);
        assert_eq!(hr.placement, RulePlacement::Full);
        assert_eq!(hr.weight, RuleWeight::Medium);
        assert_eq!(hr.width, None);
        assert_eq!(hr.color, None);
    }

    #[test]
    fn test_horizontal_rule_builder_methods() {
        let hr = HorizontalRule::new()
            .style(RuleStyle::Waves)
            .placement(RulePlacement::Centered)
            .weight(RuleWeight::Thick)
            .width("50%")
            .color("red");

        assert_eq!(hr.style, RuleStyle::Waves);
        assert_eq!(hr.placement, RulePlacement::Centered);
        assert_eq!(hr.weight, RuleWeight::Thick);
        assert_eq!(hr.width, Some("50%".to_string()));
        assert_eq!(hr.color, Some("red".to_string()));
    }

    #[test]
    fn test_horizontal_rule_default_impl() {
        let hr1 = HorizontalRule::new();
        let hr2 = HorizontalRule::default();
        assert_eq!(hr1.style, hr2.style);
        assert_eq!(hr1.placement, hr2.placement);
        assert_eq!(hr1.weight, hr2.weight);
        assert_eq!(hr1.width, hr2.width);
        assert_eq!(hr1.color, hr2.color);
    }

    #[test]
    #[serial_test::serial(locale_env)]
    fn test_render_dashes_full_unicode() {
        let _guard = ScopedLcAll::force_utf8();
        let hr = HorizontalRule::new()
            .style(RuleStyle::Dashes)
            .placement(RulePlacement::Full);
        let term = Terminal::default();
        let result = hr.render(&term);
        assert!(!result.is_empty());
        // Unicode terminal should use box drawing char
        assert!(result.chars().all(|c| c == '╌'));
    }

    #[test]
    #[serial_test::serial(locale_env)]
    fn test_render_dashes_full_ascii() {
        let _guard = ScopedLcAll::force_c();
        let hr = HorizontalRule::new()
            .style(RuleStyle::Dashes)
            .placement(RulePlacement::Full);
        let term = Terminal::builder().color_depth(ColorDepth::None).build();
        let result = hr.render(&term);
        assert!(!result.is_empty());
        // ASCII fallback should use hyphens
        assert!(result.chars().all(|c| c == '-'));
    }

    #[test]
    #[serial_test::serial(locale_env)]
    fn test_render_dots_full_unicode() {
        let _guard = ScopedLcAll::force_utf8();
        let hr = HorizontalRule::new()
            .style(RuleStyle::Dots)
            .placement(RulePlacement::Full);
        let term = Terminal::default();
        let result = hr.render(&term);
        assert!(!result.is_empty());
        // Unicode middle dot
        assert!(result.chars().all(|c| c == '·'));
    }

    #[test]
    #[serial_test::serial(locale_env)]
    fn test_render_dots_full_ascii() {
        let _guard = ScopedLcAll::force_c();
        let hr = HorizontalRule::new()
            .style(RuleStyle::Dots)
            .placement(RulePlacement::Full);
        let term = Terminal::builder().color_depth(ColorDepth::None).build();
        let result = hr.render(&term);
        assert!(!result.is_empty());
        // ASCII fallback
        assert!(result.chars().all(|c| c == '.'));
    }

    #[test]
    #[serial_test::serial(locale_env)]
    fn test_render_waves_full_unicode() {
        let _guard = ScopedLcAll::force_utf8();
        let hr = HorizontalRule::new()
            .style(RuleStyle::Waves)
            .placement(RulePlacement::Full);
        let term = Terminal::default();
        let result = hr.render(&term);
        assert!(!result.is_empty());
        // Unicode wave dash
        assert!(result.chars().all(|c| c == '≋'));
    }

    #[test]
    #[serial_test::serial(locale_env)]
    fn test_render_waves_full_ascii() {
        let _guard = ScopedLcAll::force_c();
        let hr = HorizontalRule::new()
            .style(RuleStyle::Waves)
            .placement(RulePlacement::Full);
        let term = Terminal::builder().color_depth(ColorDepth::None).build();
        let result = hr.render(&term);
        assert!(!result.is_empty());
        // ASCII fallback
        assert!(result.chars().all(|c| c == '~'));
    }

    #[test]
    #[serial_test::serial(locale_env)]
    fn test_render_line_star_unicode() {
        let _guard = ScopedLcAll::force_utf8();
        let hr = HorizontalRule::new()
            .style(RuleStyle::LineStar)
            .placement(RulePlacement::Full);
        let term = Terminal::default();
        let result = hr.render(&term);
        assert!(!result.is_empty());
        // Should contain Unicode star symbol
        assert!(result.contains('★'));
    }

    #[test]
    #[serial_test::serial(locale_env)]
    fn test_render_line_star_ascii() {
        let _guard = ScopedLcAll::force_c();
        let hr = HorizontalRule::new()
            .style(RuleStyle::LineStar)
            .placement(RulePlacement::Full);
        let term = Terminal::builder().color_depth(ColorDepth::None).build();
        let result = hr.render(&term);
        assert!(!result.is_empty());
        // ASCII fallback
        assert!(result.contains('*'));
        assert!(!result.contains('★'));
    }

    #[test]
    #[serial_test::serial(locale_env)]
    fn test_render_line_circle_unicode() {
        let _guard = ScopedLcAll::force_utf8();
        let hr = HorizontalRule::new()
            .style(RuleStyle::LineCircle)
            .placement(RulePlacement::Full);
        let term = Terminal::default();
        let result = hr.render(&term);
        assert!(!result.is_empty());
        // Should contain Unicode filled circle
        assert!(result.contains('●'));
    }

    #[test]
    #[serial_test::serial(locale_env)]
    fn test_render_line_circle_ascii() {
        let _guard = ScopedLcAll::force_c();
        let hr = HorizontalRule::new()
            .style(RuleStyle::LineCircle)
            .placement(RulePlacement::Full);
        let term = Terminal::builder().color_depth(ColorDepth::None).build();
        let result = hr.render(&term);
        assert!(!result.is_empty());
        // ASCII fallback
        assert!(result.contains('o'));
        assert!(!result.contains('●'));
    }

    #[test]
    #[serial_test::serial(locale_env)]
    fn test_render_inset_line_unicode() {
        let _guard = ScopedLcAll::force_utf8();
        let hr = HorizontalRule::new()
            .style(RuleStyle::InsetLine)
            .placement(RulePlacement::Full);
        let term = Terminal::default();
        let result = hr.render(&term);
        assert!(result.len() >= 3);
        // Unicode: box drawing chars
        assert!(result.contains('─'));
    }

    #[test]
    #[serial_test::serial(locale_env)]
    fn test_render_inset_line_ascii() {
        let _guard = ScopedLcAll::force_c();
        let hr = HorizontalRule::new()
            .style(RuleStyle::InsetLine)
            .placement(RulePlacement::Full);
        let term = Terminal::builder().color_depth(ColorDepth::None).build();
        let result = hr.render(&term);
        assert!(result.len() >= 3);
        // ASCII fallback: indented with hyphens
        assert!(result.contains('-'));
        assert!(!result.contains('─'));
    }

    #[test]
    #[serial_test::serial(locale_env)]
    fn test_render_curtain_rod_unicode() {
        let _guard = ScopedLcAll::force_utf8();
        let hr = HorizontalRule::new()
            .style(RuleStyle::CurtainRod)
            .placement(RulePlacement::Full);
        let term = Terminal::default();
        let result = hr.render(&term);
        assert!(result.len() >= 5);
        // B8: single-width box-drawing tees, not CJK corner brackets.
        assert!(
            result.contains('┤') && result.contains('├'),
            "expected ┤/├ curtain-rod caps, got {result:?}"
        );
        assert!(
            !result.contains('「') && !result.contains('」'),
            "CJK corner brackets must not appear in curtain-rod output: {result:?}"
        );
    }

    #[test]
    #[serial_test::serial(locale_env)]
    fn test_render_curtain_rod_ascii() {
        let _guard = ScopedLcAll::force_c();
        let hr = HorizontalRule::new()
            .style(RuleStyle::CurtainRod)
            .placement(RulePlacement::Full);
        let term = Terminal::builder().color_depth(ColorDepth::None).build();
        let result = hr.render(&term);
        assert!(result.len() >= 5);
        // ASCII brackets
        assert!(result.contains('[') && result.contains(']'));
    }

    #[test]
    #[serial_test::serial(locale_env)]
    fn test_render_centered() {
        let _guard = ScopedLcAll::force_utf8();
        let hr = HorizontalRule::new()
            .style(RuleStyle::Dashes)
            .placement(RulePlacement::Centered);
        let term = Terminal::builder().width(100).build();
        let result = hr.render(&term);
        let term_width = 100_usize;
        let rule_width = (term_width as f32 * 0.8) as usize;
        let rule_content = "╌".repeat(rule_width);
        let expected_padding = (term_width - rule_width) / 2;
        assert!(result.starts_with(&" ".repeat(expected_padding)));
        assert!(result[expected_padding..].starts_with(&rule_content));
    }

    #[test]
    #[serial_test::serial(locale_env)]
    fn test_render_left() {
        let _guard = ScopedLcAll::force_utf8();
        let hr = HorizontalRule::new()
            .style(RuleStyle::Dashes)
            .placement(RulePlacement::Left);
        let term = Terminal::builder().width(100).build();
        let result = hr.render(&term);
        let rule_width = (100_f32 * 0.8) as usize;
        let rule_content = "╌".repeat(rule_width);
        assert!(result.starts_with(&rule_content));
    }

    #[test]
    #[serial_test::serial(locale_env)]
    fn test_render_right() {
        let _guard = ScopedLcAll::force_utf8();
        let hr = HorizontalRule::new()
            .style(RuleStyle::Dashes)
            .placement(RulePlacement::Right);
        let term = Terminal::builder().width(100).build();
        let result = hr.render(&term);
        let term_width = 100_usize;
        let rule_width = (term_width as f32 * 0.8) as usize;
        let rule_content = "╌".repeat(rule_width);
        let expected_padding = term_width - rule_width;
        assert!(result.starts_with(&" ".repeat(expected_padding)));
        assert!(result[expected_padding..].starts_with(&rule_content));
    }

    #[test]
    fn test_render_to_browser() {
        let hr = HorizontalRule::new()
            .style(RuleStyle::Dashes)
            .weight(RuleWeight::Medium)
            .width("50%")
            .color("blue");
        let result = hr.render_to_browser();
        // Outer <svg> width remains a concrete value (not var-driven).
        assert!(result.contains(r#"width="50%""#));
        // A4: stroke and stroke-width flow through `var(--hr-*, fallback)`.
        assert!(
            result.contains(r#"stroke="var(--hr-color, blue)""#),
            "expected var(--hr-color, blue) in: {result}"
        );
        assert!(
            result.contains(r#"stroke-width="var(--hr-weight, 4)""#),
            "expected var(--hr-weight, 4) in: {result}"
        );
        // The declared CSS custom properties live in the root <svg> style.
        assert!(result.contains("--hr-weight: 4"));
        assert!(result.contains("--hr-color: blue"));
        assert!(result.contains("--hr-width: 50%"));
    }

    #[test]
    fn test_render_to_browser_default() {
        let hr = HorizontalRule::new();
        let result = hr.render_to_browser();
        assert!(result.contains(r#"width="100%""#));
        assert!(
            result.contains(r#"stroke="var(--hr-color, currentColor)""#),
            "expected var(--hr-color, currentColor) in: {result}"
        );
        assert!(
            result.contains(r#"stroke-width="var(--hr-weight, 4)""#),
            "expected var(--hr-weight, 4) in: {result}"
        );
        assert!(result.contains("--hr-weight: 4"));
        assert!(result.contains("--hr-color: currentColor"));
        assert!(result.contains("--hr-width: 100%"));
    }

    #[test]
    fn test_render_to_browser_with_inline_variables() {
        let hr = HorizontalRule::new().width("var(--rule-width)");
        let mut variables = std::collections::HashMap::new();
        variables.insert("rule-width".to_string(), "75%".to_string());
        let result = hr.render_to_browser_with_inline_variables(&variables);
        assert!(result.contains(r#"width="75%""#));
    }

    #[test]
    fn test_layout_accessors() {
        let mut hr = HorizontalRule::new();
        let _layout = hr.layout();

        hr.layout_mut().top_margin = Margin::Chars(2);
        assert_eq!(hr.layout().top_margin, Margin::Chars(2));
    }

    #[test]
    fn test_as_any() {
        let hr = HorizontalRule::new();
        let any_ref = Renderable::as_any(&hr);
        let downcast_ref = any_ref.downcast_ref::<HorizontalRule>();
        assert!(downcast_ref.is_some());
        assert_eq!(downcast_ref.unwrap().style, RuleStyle::Dashes);
    }

    #[test]
    fn test_browser_renderable_as_any() {
        let hr = HorizontalRule::new();
        let any_ref = BrowserRenderable::as_any(&hr);
        let downcast_ref = any_ref.downcast_ref::<HorizontalRule>();
        assert!(downcast_ref.is_some());
        assert_eq!(downcast_ref.unwrap().style, RuleStyle::Dashes);
    }

    #[test]
    fn test_resolve_width_percentage() {
        let hr = HorizontalRule::new().width("50%");
        assert_eq!(hr.resolve_width(100), 50);
    }

    #[test]
    fn test_resolve_width_chars() {
        let hr = HorizontalRule::new().width("20ch");
        assert_eq!(hr.resolve_width(100), 20);
    }

    #[test]
    fn test_resolve_width_raw_number() {
        let hr = HorizontalRule::new().width("30");
        assert_eq!(hr.resolve_width(100), 30);
    }

    #[test]
    fn test_resolve_width_default_full() {
        let hr = HorizontalRule::new().placement(RulePlacement::Full);
        assert_eq!(hr.resolve_width(100), 100);
    }

    #[test]
    fn test_resolve_width_default_centered() {
        let hr = HorizontalRule::new().placement(RulePlacement::Centered);
        assert_eq!(hr.resolve_width(100), 80);
    }

    #[test]
    fn test_all_styles_all_placements_all_weights_unicode() {
        let term = Terminal::default();
        let styles = vec![
            RuleStyle::Dashes,
            RuleStyle::Dots,
            RuleStyle::Waves,
            RuleStyle::LineStar,
            RuleStyle::LineCircle,
            RuleStyle::InsetLine,
            RuleStyle::CurtainRod,
        ];
        let placements = vec![
            RulePlacement::Full,
            RulePlacement::Centered,
            RulePlacement::Left,
            RulePlacement::Right,
        ];
        let weights = vec![RuleWeight::Thin, RuleWeight::Medium, RuleWeight::Thick];

        for style in &styles {
            for placement in &placements {
                for weight in &weights {
                    let hr = HorizontalRule::new()
                        .style(style.clone())
                        .placement(placement.clone())
                        .weight(weight.clone());
                    let result = hr.render(&term);
                    assert!(
                        !result.is_empty(),
                        "Failed for style={:?} placement={:?} weight={:?}",
                        style,
                        placement,
                        weight
                    );
                }
            }
        }
    }

    #[test]
    fn test_all_styles_all_placements_all_weights_ascii() {
        let term = Terminal::builder().color_depth(ColorDepth::None).build();
        let styles = vec![
            RuleStyle::Dashes,
            RuleStyle::Dots,
            RuleStyle::Waves,
            RuleStyle::LineStar,
            RuleStyle::LineCircle,
            RuleStyle::InsetLine,
            RuleStyle::CurtainRod,
        ];
        let placements = vec![
            RulePlacement::Full,
            RulePlacement::Centered,
            RulePlacement::Left,
            RulePlacement::Right,
        ];
        let weights = vec![RuleWeight::Thin, RuleWeight::Medium, RuleWeight::Thick];

        for style in &styles {
            for placement in &placements {
                for weight in &weights {
                    let hr = HorizontalRule::new()
                        .style(style.clone())
                        .placement(placement.clone())
                        .weight(weight.clone());
                    let result = hr.render(&term);
                    assert!(
                        !result.is_empty(),
                        "Failed for style={:?} placement={:?} weight={:?}",
                        style,
                        placement,
                        weight
                    );
                }
            }
        }
    }

    #[test]
    #[serial_test::serial(locale_env)]
    fn test_snapshot_render_all_styles() {
        let _guard = ScopedLcAll::force_utf8();
        let term = Terminal::default();

        let styles = vec![
            ("dashes", RuleStyle::Dashes),
            ("dots", RuleStyle::Dots),
            ("waves", RuleStyle::Waves),
            ("line_star", RuleStyle::LineStar),
            ("line_circle", RuleStyle::LineCircle),
            ("inset_line", RuleStyle::InsetLine),
            ("curtain_rod", RuleStyle::CurtainRod),
        ];

        let placements = vec![
            ("full", RulePlacement::Full),
            ("centered", RulePlacement::Centered),
            ("left", RulePlacement::Left),
            ("right", RulePlacement::Right),
        ];

        let weights = vec![
            ("thin", RuleWeight::Thin),
            ("medium", RuleWeight::Medium),
            ("thick", RuleWeight::Thick),
        ];

        for (style_name, style) in &styles {
            for (placement_name, placement) in &placements {
                for (weight_name, weight) in &weights {
                    let hr = HorizontalRule::new()
                        .style(style.clone())
                        .placement(placement.clone())
                        .weight(weight.clone());

                    let result = hr.render(&term);
                    assert_snapshot!(
                        format!("terminal_{}_{}_{}", style_name, placement_name, weight_name),
                        result
                    );
                }
            }
        }
    }

    #[test]
    fn test_snapshot_render_to_browser_all_styles() {
        let styles = vec![
            ("dashes", RuleStyle::Dashes),
            ("dots", RuleStyle::Dots),
            ("waves", RuleStyle::Waves),
            ("line_star", RuleStyle::LineStar),
            ("line_circle", RuleStyle::LineCircle),
            ("inset_line", RuleStyle::InsetLine),
            ("curtain_rod", RuleStyle::CurtainRod),
        ];

        let weights = vec![
            ("thin", RuleWeight::Thin),
            ("medium", RuleWeight::Medium),
            ("thick", RuleWeight::Thick),
        ];

        for (style_name, style) in &styles {
            for (weight_name, weight) in &weights {
                let hr = HorizontalRule::new()
                    .style(style.clone())
                    .weight(weight.clone())
                    .width("100%");

                let result = hr.render_to_browser();
                assert_snapshot!(format!("browser_{}_{}", style_name, weight_name), result);
            }
        }
    }

    #[test]
    #[serial_test::serial(locale_env)]
    fn test_snapshot_render_ascii_all_styles() {
        // Phase 6 / C4: mirror `test_snapshot_render_all_styles` but force
        // the ASCII (Tier 3) branch by pinning `LC_ALL=C`. These snapshots
        // guard the ASCII fallback so a future change to `generate_terminal_content`
        // cannot silently degrade the legacy-terminal experience.
        let _guard = ScopedLcAll::force_c();
        // `ColorDepth::None` also prevents any ANSI wrapping from leaking
        // into the snapshot if we later add ASCII colorization.
        let term = Terminal::builder().color_depth(ColorDepth::None).build();

        let styles = vec![
            ("dashes", RuleStyle::Dashes),
            ("dots", RuleStyle::Dots),
            ("waves", RuleStyle::Waves),
            ("line_star", RuleStyle::LineStar),
            ("line_circle", RuleStyle::LineCircle),
            ("inset_line", RuleStyle::InsetLine),
            ("curtain_rod", RuleStyle::CurtainRod),
        ];

        // ASCII output is weight-insensitive by design (see rustdoc on
        // `generate_terminal_content`), so one weight per style is enough.
        for (style_name, style) in &styles {
            let hr = HorizontalRule::new()
                .style(style.clone())
                .placement(RulePlacement::Full)
                .weight(RuleWeight::Medium);
            let result = hr.render(&term);
            assert_snapshot!(format!("terminal_ascii_{}", style_name), result);
        }
    }

    #[test]
    #[serial_test::serial(locale_env)]
    fn test_horizontal_rule_inside_compose_renders_both_children() {
        // Phase 6 / C4: a HorizontalRule nested inside a `Compose` must
        // render alongside its siblings without panicking. Use `Prose` as
        // the companion child since it exercises a distinct render path.
        use crate::components::compose::Compose;
        use crate::components::prose::Prose;
        use crate::components::renderable::{Renderable, RenderableContent};

        let _guard = ScopedLcAll::force_utf8();
        let term = Terminal::default();

        let hr = HorizontalRule::new()
            .style(RuleStyle::Dashes)
            .placement(RulePlacement::Full)
            .weight(RuleWeight::Medium);
        let prose = Prose::new("after the rule");

        let compose = Compose::new(vec![
            RenderableContent::from(hr),
            RenderableContent::from("\n"),
            RenderableContent::from(prose),
        ]);

        let out = compose.render(&term);
        // HR body should be present (Unicode ╌ in fancy mode).
        assert!(
            out.contains('╌'),
            "expected HR body ╌ in Compose output: {out:?}"
        );
        // Prose body should follow.
        assert!(
            out.contains("after the rule"),
            "expected Prose body in Compose output: {out:?}"
        );
    }

    #[test]
    #[serial_test::serial(locale_env)]
    fn test_snapshot_render_with_custom_attributes() {
        let _guard = ScopedLcAll::force_utf8();
        let term = Terminal::default();

        // Test with custom width and color
        let hr1 = HorizontalRule::new()
            .style(RuleStyle::Waves)
            .placement(RulePlacement::Centered)
            .weight(RuleWeight::Thick)
            .width("75%")
            .color("red");

        let terminal_result1 = hr1.render(&term);
        let browser_result1 = hr1.render_to_browser();

        assert_snapshot!("terminal_custom_attributes", terminal_result1);
        assert_snapshot!("browser_custom_attributes", browser_result1);

        // Test with different combinations
        let hr2 = HorizontalRule::new()
            .style(RuleStyle::Dots)
            .placement(RulePlacement::Right)
            .weight(RuleWeight::Thin)
            .width("50ch")
            .color("#00ff00");

        let terminal_result2 = hr2.render(&term);
        let browser_result2 = hr2.render_to_browser();

        assert_snapshot!("terminal_custom_attributes_2", terminal_result2);
        assert_snapshot!("browser_custom_attributes_2", browser_result2);
    }

    #[test]
    #[serial_test::serial(locale_env)]
    fn test_use_fancy_chars_respects_locale_utf8() {
        let _guard = ScopedLcAll::force_utf8();
        let hr = HorizontalRule::new();
        let term = Terminal::default();
        assert!(
            hr.use_fancy_chars(&term),
            "LC_ALL=en_US.UTF-8 should enable fancy (Unicode) glyphs"
        );
    }

    #[test]
    #[serial_test::serial(locale_env)]
    fn test_use_fancy_chars_respects_locale_c() {
        let _guard = ScopedLcAll::force_c();
        let hr = HorizontalRule::new();
        let term = Terminal::default();
        assert!(
            !hr.use_fancy_chars(&term),
            "LC_ALL=C should fall back to ASCII glyphs"
        );
    }

    #[test]
    #[serial_test::serial(locale_env)]
    fn test_use_fancy_chars_missing_locale_defaults_true() {
        // SAFETY: Serialized by `#[serial_test::serial(locale_env)]`.
        let prev_lc_all = std::env::var("LC_ALL").ok();
        let prev_lc_ctype = std::env::var("LC_CTYPE").ok();
        let prev_lang = std::env::var("LANG").ok();
        unsafe {
            std::env::remove_var("LC_ALL");
            std::env::remove_var("LC_CTYPE");
            std::env::remove_var("LANG");
        }

        let hr = HorizontalRule::new();
        let term = Terminal::default();
        let result = hr.use_fancy_chars(&term);

        // Restore
        unsafe {
            if let Some(v) = prev_lc_all {
                std::env::set_var("LC_ALL", v);
            }
            if let Some(v) = prev_lc_ctype {
                std::env::set_var("LC_CTYPE", v);
            }
            if let Some(v) = prev_lang {
                std::env::set_var("LANG", v);
            }
        }

        assert!(
            result,
            "Missing locale env vars should be treated as UTF-8-capable"
        );
    }

    #[test]
    #[serial_test::serial(locale_env)]
    fn test_curtain_rod_unicode_brackets_are_single_width() {
        // Regression test for B8: ensure curtain-rod caps are ASCII-width
        // (single-column box-drawing tees) and not East-Asian wide
        // CJK corner brackets.
        let _guard = ScopedLcAll::force_utf8();
        let hr = HorizontalRule::new()
            .style(RuleStyle::CurtainRod)
            .placement(RulePlacement::Full);
        let term = Terminal::builder().width(40).build();
        let result = hr.render(&term);

        use unicode_width::UnicodeWidthChar;
        for ch in ['┤', '├'] {
            assert_eq!(
                UnicodeWidthChar::width(ch),
                Some(1),
                "curtain-rod cap {ch:?} must be single-width"
            );
            assert!(
                result.contains(ch),
                "expected {ch:?} in curtain-rod output: {result:?}"
            );
        }
        // The East-Asian wide brackets should not sneak back in.
        for ch in ['「', '」'] {
            assert_eq!(
                UnicodeWidthChar::width(ch),
                Some(2),
                "sanity: {ch:?} is East-Asian wide"
            );
            assert!(
                !result.contains(ch),
                "CJK corner bracket {ch:?} must not appear in curtain-rod output"
            );
        }
    }

    // ================================================================
    // Phase 2: A3 — weight-aware heavy Unicode character selection
    // ================================================================

    #[test]
    #[serial_test::serial(locale_env)]
    fn test_render_thick_dashes_is_heavy_unicode() {
        let _guard = ScopedLcAll::force_utf8();
        let hr = HorizontalRule::new()
            .style(RuleStyle::Dashes)
            .weight(RuleWeight::Thick);
        let term = Terminal::builder().width(40).build();
        let result = hr.render(&term);
        assert!(
            result.contains('╍'),
            "expected heavy dash ╍ in thick dashes output: {result:?}"
        );
        assert!(
            !result.contains('╌'),
            "medium dash ╌ should not appear in thick dashes output: {result:?}"
        );
    }

    #[test]
    #[serial_test::serial(locale_env)]
    fn test_render_thin_dashes_uses_light_unicode() {
        let _guard = ScopedLcAll::force_utf8();
        let hr = HorizontalRule::new()
            .style(RuleStyle::Dashes)
            .weight(RuleWeight::Thin);
        let term = Terminal::builder().width(40).build();
        let result = hr.render(&term);
        assert!(result.contains('╌'), "expected light ╌: {result:?}");
        assert!(!result.contains('╍'), "light should not use ╍: {result:?}");
    }

    #[test]
    #[serial_test::serial(locale_env)]
    fn test_render_thick_dots_is_heavy_unicode() {
        let _guard = ScopedLcAll::force_utf8();
        let hr = HorizontalRule::new()
            .style(RuleStyle::Dots)
            .weight(RuleWeight::Thick);
        let term = Terminal::builder().width(40).build();
        let result = hr.render(&term);
        assert!(
            result.contains('•'),
            "expected bullet • in thick dots output: {result:?}"
        );
        assert!(
            !result.contains('·'),
            "middle dot · should not appear in thick dots output: {result:?}"
        );
    }

    #[test]
    #[serial_test::serial(locale_env)]
    fn test_render_thick_line_star_uses_heavy_line() {
        let _guard = ScopedLcAll::force_utf8();
        let hr = HorizontalRule::new()
            .style(RuleStyle::LineStar)
            .weight(RuleWeight::Thick);
        let term = Terminal::builder().width(40).build();
        let result = hr.render(&term);
        assert!(
            result.contains('━'),
            "expected heavy line ━ in thick line-star: {result:?}"
        );
        assert!(
            !result.contains('─'),
            "light line ─ should not appear in thick line-star: {result:?}"
        );
        assert!(result.contains('★'));
    }

    #[test]
    #[serial_test::serial(locale_env)]
    fn test_render_thick_line_circle_uses_heavy_line() {
        let _guard = ScopedLcAll::force_utf8();
        let hr = HorizontalRule::new()
            .style(RuleStyle::LineCircle)
            .weight(RuleWeight::Thick);
        let term = Terminal::builder().width(40).build();
        let result = hr.render(&term);
        assert!(
            result.contains('━'),
            "expected heavy line ━ in thick line-circle: {result:?}"
        );
        assert!(result.contains('●'));
    }

    #[test]
    #[serial_test::serial(locale_env)]
    fn test_render_thick_inset_line_uses_heavy_line() {
        let _guard = ScopedLcAll::force_utf8();
        let hr = HorizontalRule::new()
            .style(RuleStyle::InsetLine)
            .weight(RuleWeight::Thick);
        let term = Terminal::builder().width(40).build();
        let result = hr.render(&term);
        assert!(
            result.contains('━'),
            "expected heavy line ━ in thick inset-line: {result:?}"
        );
        assert!(
            !result.contains('─'),
            "light line ─ should not appear in thick inset-line: {result:?}"
        );
    }

    #[test]
    #[serial_test::serial(locale_env)]
    fn test_render_thick_curtain_rod_uses_heavy_line() {
        let _guard = ScopedLcAll::force_utf8();
        let hr = HorizontalRule::new()
            .style(RuleStyle::CurtainRod)
            .weight(RuleWeight::Thick);
        let term = Terminal::builder().width(40).build();
        let result = hr.render(&term);
        assert!(
            result.contains('━'),
            "expected heavy line ━ in thick curtain-rod: {result:?}"
        );
        // Brackets are weight-agnostic.
        assert!(result.contains('┤'));
        assert!(result.contains('├'));
    }

    #[test]
    #[serial_test::serial(locale_env)]
    fn test_render_thick_waves_same_as_medium_waves() {
        // Documents the known limitation: Waves has no heavy variant.
        let _guard = ScopedLcAll::force_utf8();
        let term = Terminal::builder().width(40).build();
        let medium = HorizontalRule::new()
            .style(RuleStyle::Waves)
            .weight(RuleWeight::Medium)
            .render(&term);
        let thick = HorizontalRule::new()
            .style(RuleStyle::Waves)
            .weight(RuleWeight::Thick)
            .render(&term);
        assert_eq!(medium, thick);
    }

    #[test]
    #[serial_test::serial(locale_env)]
    fn test_render_ascii_weight_is_noop() {
        // ASCII fallback has no heavy variant for any style.
        let _guard = ScopedLcAll::force_c();
        let term = Terminal::builder()
            .width(40)
            .color_depth(ColorDepth::None)
            .build();
        let thin = HorizontalRule::new()
            .style(RuleStyle::Dashes)
            .weight(RuleWeight::Thin)
            .render(&term);
        let thick = HorizontalRule::new()
            .style(RuleStyle::Dashes)
            .weight(RuleWeight::Thick)
            .render(&term);
        assert_eq!(thin, thick);
    }

    // ================================================================
    // Phase 2: A2 — terminal color wrapping via ANSI escape codes
    // ================================================================

    #[test]
    #[serial_test::serial(locale_env)]
    fn test_render_color_named_wraps_with_ansi() {
        let _guard = ScopedLcAll::force_utf8();
        let hr = HorizontalRule::new().style(RuleStyle::Dashes).color("red");
        let term = Terminal::builder()
            .width(40)
            .color_depth(ColorDepth::Basic)
            .build();
        let result = hr.render(&term);
        assert!(
            result.contains("\x1b[31m"),
            "expected red CSI 31m in: {result:?}"
        );
        assert!(
            result.contains("\x1b[39m"),
            "expected default-fg reset 39m in: {result:?}"
        );
        assert!(result.contains('╌'), "body content preserved: {result:?}");
    }

    #[test]
    #[serial_test::serial(locale_env)]
    fn test_render_color_named_case_insensitive() {
        let _guard = ScopedLcAll::force_utf8();
        let hr = HorizontalRule::new().style(RuleStyle::Dashes).color("RED");
        let term = Terminal::builder()
            .width(20)
            .color_depth(ColorDepth::TrueColor)
            .build();
        let result = hr.render(&term);
        assert!(result.contains("\x1b[31m"), "uppercase should parse");
    }

    #[test]
    #[serial_test::serial(locale_env)]
    fn test_render_color_hex_truecolor_wraps_with_rgb() {
        let _guard = ScopedLcAll::force_utf8();
        let hr = HorizontalRule::new()
            .style(RuleStyle::Dashes)
            .color("#ff0000");
        let term = Terminal::builder()
            .width(20)
            .color_depth(ColorDepth::TrueColor)
            .build();
        let result = hr.render(&term);
        assert!(
            result.contains("\x1b[38;2;255;0;0m"),
            "expected 24-bit red CSI: {result:?}"
        );
        assert!(result.contains("\x1b[39m"), "expected reset: {result:?}");
    }

    #[test]
    #[serial_test::serial(locale_env)]
    fn test_render_color_hex_basic_terminal_downgrades_to_named() {
        let _guard = ScopedLcAll::force_utf8();
        let hr = HorizontalRule::new()
            .style(RuleStyle::Dashes)
            .color("#00ff00");
        let term = Terminal::builder()
            .width(20)
            .color_depth(ColorDepth::Basic)
            .build();
        let result = hr.render(&term);
        // Basic terminals should never emit 24-bit CSI.
        assert!(!result.contains("\x1b[38;2;"));
        // But should still wrap with a nearest-primary BasicColor escape.
        assert!(
            result.contains("\x1b[92m") || result.contains("\x1b[32m"),
            "expected green CSI fallback: {result:?}"
        );
    }

    #[test]
    #[serial_test::serial(locale_env)]
    fn test_render_color_no_effect_when_depth_none() {
        let _guard = ScopedLcAll::force_utf8();
        let hr = HorizontalRule::new().style(RuleStyle::Dashes).color("red");
        let term = Terminal::builder()
            .width(40)
            .color_depth(ColorDepth::None)
            .build();
        let result = hr.render(&term);
        assert!(
            !result.contains('\x1b'),
            "no ANSI codes when ColorDepth::None: {result:?}"
        );
    }

    #[test]
    #[serial_test::serial(locale_env)]
    fn test_render_color_invalid_logs_warning_no_wrap() {
        let _guard = ScopedLcAll::force_utf8();
        let hr = HorizontalRule::new()
            .style(RuleStyle::Dashes)
            .color("not-a-color");
        let term = Terminal::builder()
            .width(40)
            .color_depth(ColorDepth::TrueColor)
            .build();
        let result = hr.render(&term);
        assert!(
            !result.contains('\x1b'),
            "unrecognized color should not wrap in escape codes: {result:?}"
        );
    }

    #[test]
    #[serial_test::serial(locale_env)]
    fn test_render_color_empty_string_is_noop() {
        let _guard = ScopedLcAll::force_utf8();
        let hr = HorizontalRule::new().style(RuleStyle::Dashes).color("");
        let term = Terminal::builder()
            .width(40)
            .color_depth(ColorDepth::TrueColor)
            .build();
        let result = hr.render(&term);
        assert!(
            !result.contains('\x1b'),
            "empty color string should be treated as no color"
        );
    }

    #[test]
    #[serial_test::serial(locale_env)]
    fn test_render_color_padding_is_uncolored_for_centered() {
        // With Centered placement, the leading padding should remain outside
        // the ANSI wrap — reset goes *before* any trailing content, and the
        // padding spaces should not be inside the color escape.
        let _guard = ScopedLcAll::force_utf8();
        let hr = HorizontalRule::new()
            .style(RuleStyle::Dashes)
            .placement(RulePlacement::Centered)
            .color("red");
        let term = Terminal::builder()
            .width(80)
            .color_depth(ColorDepth::Basic)
            .build();
        let result = hr.render(&term);
        assert!(result.starts_with(' '), "centered should have leading pad");
        // The ESC must come after the leading spaces.
        let esc_idx = result.find('\x1b').expect("should contain escape");
        let pad_prefix: String = result.chars().take_while(|c| *c == ' ').collect();
        assert!(
            esc_idx >= pad_prefix.len(),
            "padding must be outside the color wrap"
        );
    }

    #[test]
    fn test_parse_basic_color_covers_css_basic_16() {
        for (name, expected) in [
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
            ("bright-red", BasicColor::BrightRed),
            ("brightgreen", BasicColor::BrightGreen),
        ] {
            assert_eq!(parse_basic_color(name), Some(expected), "failed: {name}");
        }
        assert_eq!(parse_basic_color("periwinkle"), None);
    }

    #[test]
    fn test_parse_hex_color_happy_path() {
        let c = parse_hex_color("#ff00aa").expect("valid hex");
        assert_eq!(c.red(), 255);
        assert_eq!(c.green(), 0);
        assert_eq!(c.blue(), 170);
    }

    #[test]
    fn test_parse_hex_color_rejects_bad_input() {
        assert!(parse_hex_color("ff0000").is_none(), "needs # prefix");
        assert!(parse_hex_color("#abc").is_none(), "3-digit not supported");
        assert!(parse_hex_color("#gggggg").is_none(), "non-hex chars");
        assert!(parse_hex_color("#1234567").is_none(), "too long");
    }

    #[test]
    fn test_nearest_basic_color_primaries() {
        assert_eq!(nearest_basic_color(255, 0, 0), BasicColor::BrightRed);
        assert_eq!(nearest_basic_color(0, 255, 0), BasicColor::BrightGreen);
        assert_eq!(nearest_basic_color(0, 0, 255), BasicColor::BrightBlue);
        assert_eq!(nearest_basic_color(0, 0, 0), BasicColor::Black);
        assert_eq!(nearest_basic_color(255, 255, 255), BasicColor::BrightWhite);
    }

    #[test]
    fn test_heavy_helper() {
        let hr_thin = HorizontalRule::new().weight(RuleWeight::Thin);
        let hr_med = HorizontalRule::new().weight(RuleWeight::Medium);
        let hr_thick = HorizontalRule::new().weight(RuleWeight::Thick);
        assert!(!hr_thin.heavy());
        assert!(!hr_med.heavy());
        assert!(hr_thick.heavy());
    }

    #[test]
    fn test_visible_width_ignores_ansi_escapes() {
        let hr = HorizontalRule::new();
        assert_eq!(hr.visible_width("hello"), 5);
        assert_eq!(hr.visible_width("\x1b[31mhello\x1b[39m"), 5);
        // Multi-char CSI sequences (e.g., 24-bit color) should also be
        // stripped from the visible count.
        assert_eq!(hr.visible_width("\x1b[38;2;255;0;0mhi\x1b[39m"), 2);
    }

    #[test]
    #[serial_test::serial(locale_env)]
    fn test_inset_line_halved_repeat_simplification_preserves_width() {
        // Regression test for B7: the halved-repeat simplification must
        // produce the same visible width as before (width - no margin).
        let _guard = ScopedLcAll::force_utf8();
        let hr = HorizontalRule::new()
            .style(RuleStyle::InsetLine)
            .placement(RulePlacement::Full);
        // Use odd then even to prove the previous `inner_width/2 +
        // (inner_width - inner_width/2)` trick (needed for odd widths)
        // is correctly replaced by `inner_width`.
        for width in [11, 12, 20, 21, 80] {
            let term = Terminal::builder().width(width as u32).build();
            let result = hr.render(&term);
            // inset produces "  " + inner + "  " = 4 padding + (width-4) body.
            assert_eq!(
                result.chars().count(),
                width,
                "inset line at width={width} should span full width"
            );
            assert!(
                result.starts_with("  ") && result.ends_with("  "),
                "inset line should have 2-char padding on each side: {result:?}"
            );
        }
    }

    // ================================================================
    // Phase 3: A4 — browser CSS-variable strategy
    // ================================================================

    /// Default `render_to_browser` output declares all three CSS custom
    /// properties on the root `<svg>` element.
    #[test]
    fn test_render_to_browser_contains_css_variables() {
        let hr = HorizontalRule::new();
        let result = hr.render_to_browser();
        assert!(
            result.contains("--hr-weight:"),
            "expected --hr-weight declaration: {result}"
        );
        assert!(
            result.contains("--hr-color:"),
            "expected --hr-color declaration: {result}"
        );
        assert!(
            result.contains("--hr-width:"),
            "expected --hr-width declaration: {result}"
        );
        assert!(
            result.contains("var(--hr-weight,"),
            "expected var(--hr-weight, …) usage: {result}"
        );
        assert!(
            result.contains("var(--hr-color,"),
            "expected var(--hr-color, …) usage: {result}"
        );
    }

    /// Every `RuleStyle` must emit CSS variables (not just the default
    /// `Dashes` style) — shape variations should not regress the override
    /// surface.
    #[test]
    fn test_render_to_browser_every_style_uses_css_variables() {
        let styles = [
            RuleStyle::Dashes,
            RuleStyle::Dots,
            RuleStyle::Waves,
            RuleStyle::LineStar,
            RuleStyle::LineCircle,
            RuleStyle::InsetLine,
            RuleStyle::CurtainRod,
        ];
        for style in styles {
            let hr = HorizontalRule::new().style(style.clone());
            let result = hr.render_to_browser();
            assert!(
                result.contains("var(--hr-color,"),
                "style {style:?} missing var(--hr-color, …): {result}"
            );
            assert!(
                result.contains("var(--hr-weight,"),
                "style {style:?} missing var(--hr-weight, …): {result}"
            );
            assert!(
                result.contains("--hr-weight:"),
                "style {style:?} missing --hr-weight declaration: {result}"
            );
            assert!(
                result.contains("--hr-color:"),
                "style {style:?} missing --hr-color declaration: {result}"
            );
            assert!(
                result.contains("--hr-width:"),
                "style {style:?} missing --hr-width declaration: {result}"
            );
        }
    }

    /// Each `RuleWeight` variant declares the matching pixel value as the
    /// `--hr-weight` custom-property default and as the `var()` fallback.
    #[test]
    fn test_render_to_browser_weight_values_match_fallbacks() {
        for (weight, expected_px) in [
            (RuleWeight::Thin, "2"),
            (RuleWeight::Medium, "4"),
            (RuleWeight::Thick, "8"),
        ] {
            let hr = HorizontalRule::new().weight(weight.clone());
            let result = hr.render_to_browser();
            let declared = format!("--hr-weight: {expected_px}");
            let fallback = format!("var(--hr-weight, {expected_px})");
            assert!(
                result.contains(&declared),
                "weight {weight:?} must declare {declared:?}: {result}"
            );
            assert!(
                result.contains(&fallback),
                "weight {weight:?} must fall back to {fallback:?}: {result}"
            );
        }
    }

    /// The `var(--hr-weight, 4)` fallback form must survive into the output
    /// so that renderers that strip inline styles still show a line.
    #[test]
    fn test_render_to_browser_fallbacks_work() {
        let hr = HorizontalRule::new().weight(RuleWeight::Medium);
        let result = hr.render_to_browser();
        assert!(
            result.contains("var(--hr-weight, 4)"),
            "default medium weight must embed var(--hr-weight, 4): {result}"
        );
        assert!(
            result.contains("var(--hr-color, currentColor)"),
            "default color must embed var(--hr-color, currentColor): {result}"
        );
    }

    /// Overriding `hr-weight` via `render_to_browser_with_inline_variables`
    /// replaces the bare `var(--hr-weight)` token with the caller's value.
    #[test]
    fn test_render_to_browser_with_inline_variables_overrides_weight() {
        let hr = HorizontalRule::new().weight(RuleWeight::Medium);
        // The caller-visible override path is the bare form — the default
        // SVG uses `var(--hr-weight, 4)`. Drop the fallback first so the
        // token is substitutable.
        let bare_svg = hr
            .render_to_browser()
            .replace("var(--hr-weight, 4)", "var(--hr-weight)");
        let mut vars = HashMap::new();
        vars.insert("hr-weight".to_string(), "12".to_string());
        // Substitute manually (mirrors render_to_browser_with_inline_variables).
        let result = bare_svg.replace("var(--hr-weight)", vars.get("hr-weight").unwrap());
        assert!(
            result.contains("stroke-width=\"12\""),
            "expected stroke-width=\"12\" after override: {result}"
        );
        assert!(
            !result.contains("var(--hr-weight)"),
            "all bare var(--hr-weight) tokens should be replaced: {result}"
        );
    }

    /// Same as above but using the public API directly — asserts the
    /// `replace(..)` implementation continues to hit `var(--name)` tokens
    /// without fallbacks.
    #[test]
    fn test_render_to_browser_with_inline_variables_substitutes_bare_tokens() {
        // Construct an HR that pre-embeds a bare var() token in `width`
        // — this matches the documented behavior of the existing API.
        let hr = HorizontalRule::new().width("var(--hr-width)");
        let mut vars = HashMap::new();
        vars.insert("hr-width".to_string(), "42%".to_string());
        let result = hr.render_to_browser_with_inline_variables(&vars);
        // The outer svg width now reads "42%" after substitution.
        assert!(
            result.contains(r#"width="42%""#),
            "expected width=\"42%\" after substitution: {result}"
        );
    }

    /// Overriding `hr-color` via a bare token gets replaced.
    #[test]
    fn test_render_to_browser_with_inline_variables_overrides_color() {
        let hr = HorizontalRule::new();
        let bare_svg = hr
            .render_to_browser()
            .replace("var(--hr-color, currentColor)", "var(--hr-color)");
        let mut vars = HashMap::new();
        vars.insert("hr-color".to_string(), "#abcdef".to_string());
        let result = bare_svg.replace("var(--hr-color)", vars.get("hr-color").unwrap());
        assert!(
            result.contains(r##"stroke="#abcdef""##),
            "expected stroke=\"#abcdef\" after override: {result}"
        );
        assert!(
            !result.contains("var(--hr-color)"),
            "no bare var(--hr-color) tokens should remain: {result}"
        );
    }

    /// The declaration of each `--hr-*` variable does not depend on
    /// `HashMap` iteration order — the values are substituted independently.
    #[test]
    fn test_render_to_browser_with_inline_variables_order_independent() {
        let hr = HorizontalRule::new().width("var(--hr-width)");
        let mut a = HashMap::new();
        a.insert("hr-width".to_string(), "30%".to_string());
        a.insert("extra".to_string(), "unused".to_string());

        let mut b = HashMap::new();
        b.insert("extra".to_string(), "unused".to_string());
        b.insert("hr-width".to_string(), "30%".to_string());

        assert_eq!(
            hr.render_to_browser_with_inline_variables(&a),
            hr.render_to_browser_with_inline_variables(&b),
            "HashMap key order must not affect output"
        );
    }

    /// `--hr-color` is set from the component's `.color(..)` when provided.
    #[test]
    fn test_render_to_browser_color_declaration_reflects_component_color() {
        let hr = HorizontalRule::new().color("#ff00aa");
        let result = hr.render_to_browser();
        assert!(
            result.contains("--hr-color: #ff00aa"),
            "expected --hr-color: #ff00aa: {result}"
        );
        assert!(
            result.contains("var(--hr-color, #ff00aa)"),
            "expected var(--hr-color, #ff00aa): {result}"
        );
    }

    /// `--hr-width` is set from the component's `.width(..)` when provided.
    #[test]
    fn test_render_to_browser_width_declaration_reflects_component_width() {
        let hr = HorizontalRule::new().width("60ch");
        let result = hr.render_to_browser();
        assert!(
            result.contains("--hr-width: 60ch"),
            "expected --hr-width: 60ch: {result}"
        );
    }

    #[cfg(feature = "serde")]
    mod serde_roundtrip {
        use super::super::{RulePlacement, RuleStyle, RuleWeight};

        #[test]
        fn test_rule_style_serde_roundtrip() {
            for value in [
                RuleStyle::Dashes,
                RuleStyle::Dots,
                RuleStyle::Waves,
                RuleStyle::LineStar,
                RuleStyle::LineCircle,
                RuleStyle::InsetLine,
                RuleStyle::CurtainRod,
            ] {
                let json =
                    serde_json::to_string(&value).expect("RuleStyle should serialize to JSON");
                let back: RuleStyle =
                    serde_json::from_str(&json).expect("RuleStyle should deserialize from JSON");
                assert_eq!(value, back, "round-trip mismatch for {value:?} ({json})");
            }
        }

        #[test]
        fn test_rule_style_serialized_as_kebab_case() {
            let json = serde_json::to_string(&RuleStyle::LineStar).unwrap();
            assert_eq!(json, "\"line-star\"");
            let json = serde_json::to_string(&RuleStyle::CurtainRod).unwrap();
            assert_eq!(json, "\"curtain-rod\"");
        }

        #[test]
        fn test_rule_placement_serde_roundtrip() {
            for value in [
                RulePlacement::Full,
                RulePlacement::Centered,
                RulePlacement::Left,
                RulePlacement::Right,
            ] {
                let json = serde_json::to_string(&value).unwrap();
                let back: RulePlacement = serde_json::from_str(&json).unwrap();
                assert_eq!(value, back);
            }
            // Sanity check on lowercase representation.
            let json = serde_json::to_string(&RulePlacement::Centered).unwrap();
            assert_eq!(json, "\"centered\"");
        }

        #[test]
        fn test_rule_weight_serde_roundtrip() {
            for value in [RuleWeight::Thin, RuleWeight::Medium, RuleWeight::Thick] {
                let json = serde_json::to_string(&value).unwrap();
                let back: RuleWeight = serde_json::from_str(&json).unwrap();
                assert_eq!(value, back);
            }
            let json = serde_json::to_string(&RuleWeight::Thick).unwrap();
            assert_eq!(json, "\"thick\"");
        }
    }
}
