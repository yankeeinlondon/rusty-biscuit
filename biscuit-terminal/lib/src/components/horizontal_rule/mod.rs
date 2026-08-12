//! Horizontal rule component split into focused submodules.
//!
//! - [`style`] — `RuleStyle`, `RuleAlignment`, `RuleWeight`
//! - [`browser`] — `impl BrowserRenderable for HorizontalRule`,
//!   the `MarginToCss` extension trait, and CSS color parsing helpers
//!
//! Public items are re-exported here so external consumers can import them
//! from `biscuit_terminal::components::horizontal_rule` directly.

mod browser;
pub mod style;

pub use style::{RuleAlignment, RuleStyle, RuleWeight};

use std::borrow::Cow;
#[cfg(feature = "image")]
use std::io::Cursor;

use crate::components::renderable::TerminalRenderable;
#[cfg(feature = "image")]
use crate::components::terminal_image::TerminalImage;
use crate::discovery::detection::{ColorDepth, ColorMode, ImageSupport};
use crate::terminal::Terminal;
use crate::utils::color::TermColor;
use crate::utils::layout::{Layout, LayoutTerminalExt};

use self::browser::{parse_basic_color, parse_hex_color};
use renderable::tree::{
    HrAlignment, HrKind, HrWeight, RenderNode, ThematicBreakAttrs, TreeRenderable,
};

/// Default cell width (in pixels) used by `render_image_tier` and
/// `resolve_width`'s `"NNpx"` branch when the terminal does not advertise a
/// concrete `cell_size`. Matches typical monospace fonts at common DPI.
pub(crate) const DEFAULT_CELL_WIDTH: u32 = 8;

/// Default cell height (in pixels) used by `render_image_tier` when the
/// terminal does not advertise a concrete `cell_size`. Paired with
/// [`DEFAULT_CELL_WIDTH`] for an assumed 8×16 cell.
pub(crate) const DEFAULT_CELL_HEIGHT: u32 = 16;

/// A horizontal rule component for terminal and browser rendering.
///
/// ## Layout & Style Contract
///
/// `HorizontalRule` is a bespoke component (spec C5/D5) with two render
/// tiers: the **image tier** (Kitty graphics protocol, active on capable
/// TTYs) and the **text tier** (Unicode or ASCII glyphs). The drawn glyph
/// / image core is irreducible — no render-tree `NodeKind` can represent
/// either — so the component emits those bytes directly. The structural
/// `ThematicBreak` semantics (spec C9) are preserved on every target.
///
/// The honored subset (spec C5: minimum bar) is the outer placement,
/// applied to both tiers:
///
/// | Property | Status | Rationale |
/// |----------|--------|-----------|
/// | `Layout::margin` | **Honored** | Outer placement is target-agnostic; the rule is positioned within the available width after margins. |
/// | `Layout::alignment` | **Honored** | `RuleAlignment::Centered` / `Left` / `Right` already mirror this contract on the text tier; `Layout::alignment` is reserved for the outer block box. |
/// | `Layout::max_width` | **Honored** | Caps the rule's outer width. |
/// | `Layout::width` | **Honored** (via the rule's own `width()` builder) | The HR has its own CSS-like width specification (`"50%"` / `"20ch"` / `"200px"`) that drives `resolve_width`. Setting `Layout::width` itself is honored on the same outer box. |
/// | `Layout::padding` | **N/A** | Padding cannot paint the glyph/image core; there is no padding box. |
/// | `Layout::word_wrap` | **N/A** | A horizontal rule cannot wrap. |
/// | `Style::color` / `emphasis` | **N/A** | The HR has its own `color()` builder (CSS color string) routed through `apply_terminal_color`. `Style::color` / `Style::emphasis` are not lowered onto the glyph core. |
/// | `Style::background` | **N/A** | A block background would have to paint cells *around* the glyph/image bytes; the protocol/glyph owns the cells it covers. |
/// | `Style::border` | **N/A** | Box-drawing glyphs cannot frame an HR glyph/image without disrupting the rule itself. |
///
/// Parity for the honored subset is pinned in `horizontal_rule_parity.rs`.
#[derive(Debug, Clone, PartialEq)]
pub struct HorizontalRule {
    style: RuleStyle,
    alignment: RuleAlignment,
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
    ///     .alignment(RuleAlignment::Centered)
    ///     .weight(RuleWeight::Medium)
    ///     .width("75%");
    ///
    /// let term = Terminal::default();
    /// let _ = rule.render(&term);
    /// ```
    pub fn new() -> Self {
        Self {
            style: RuleStyle::Dashes,
            alignment: RuleAlignment::Full,
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

    /// Sets the alignment of the rule.
    pub fn alignment(mut self, alignment: RuleAlignment) -> Self {
        self.alignment = alignment;
        self
    }

    /// Returns the resolved alignment.
    pub fn rule_alignment(&self) -> &RuleAlignment {
        &self.alignment
    }

    /// Returns the configured style.
    pub fn rule_style(&self) -> &RuleStyle {
        &self.style
    }

    /// Returns the configured weight.
    pub fn rule_weight(&self) -> &RuleWeight {
        &self.weight
    }

    /// Returns the configured width string, if any.
    pub fn rule_width(&self) -> Option<&str> {
        self.width.as_deref()
    }

    /// Returns the configured color string, if any.
    pub fn rule_color(&self) -> Option<&str> {
        self.color.as_deref()
    }

    /// Sets the layout.
    pub fn with_layout(mut self, layout: Layout) -> Self {
        self.layout = layout;
        self
    }

    /// Sets the weight (thickness) of the rule.
    pub fn weight(mut self, weight: RuleWeight) -> Self {
        self.weight = weight;
        self
    }

    /// Sets the width of the rule as a CSS-like string.
    ///
    /// Accepts `"NN%"`, `"NNch"`, `"NNpx"`, or a bare `"NN"`. Any other
    /// non-empty string is treated as unrecognized at render time and
    /// falls back to the terminal's full width with a `tracing::warn!`.
    ///
    /// The minimum effective width is 1 column; values smaller than 1
    /// (e.g., `"0%"`, `"0px"`) are coerced to 1. The maximum is the
    /// terminal's column count at render time — wider values are clamped
    /// down.
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

impl TerminalRenderable for HorizontalRule {
    fn render(&self, term: &Terminal) -> String {
        // Mirror the render-tree block contract (`render_with_layout`): the
        // horizontal margins and `max_width` narrow the box the rule is drawn
        // into, then the drawn block is placed within the remaining slack.
        // The tree path applies its own layout around `render_*_tier`, so this
        // wrapper lives on `render` alone and never double-applies.
        let term_width = term.width();
        let inner_width = self.layout.content_width(term_width);
        let inner = if inner_width == term_width {
            Cow::Borrowed(term)
        } else {
            let mut narrowed = term.clone();
            narrowed.fixed_width = Some(inner_width);
            Cow::Owned(narrowed)
        };

        // Tier 1: SVG -> PNG -> Kitty graphics protocol. Any capability or
        // rasterization failure falls through to the text tiers below.
        let content = match self.render_image_tier(&inner) {
            Some(image) => image,
            None => self.render_text_tier(&inner),
        };

        self.layout.apply_block_layout(&content, term_width)
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

impl TreeRenderable for HorizontalRule {
    /// Projects the rule into a [`NodeKind::ThematicBreak`] node carrying the
    /// component's [`Layout`] and typed [`ThematicBreakAttrs`].
    ///
    /// The structural `ThematicBreak` semantics (spec C9) are preserved on
    /// every target. The terminal renderer reconstructs a `HorizontalRule`
    /// from the typed attrs, so the bespoke glyph/image core survives a
    /// tree round-trip; Browser lowers to `<hr>` and Markdown to `---`.
    ///
    /// [`NodeKind::ThematicBreak`]: renderable::tree::NodeKind::ThematicBreak
    fn render_tree(&self) -> RenderNode {
        let mut node = RenderNode::thematic_break();
        node.attrs.set_thematic_break(&ThematicBreakAttrs {
            kind: Some(match self.style {
                RuleStyle::Dashes => HrKind::Dashes,
                RuleStyle::Dots => HrKind::Dots,
                RuleStyle::Waves => HrKind::Waves,
                RuleStyle::LineStar => HrKind::LineStar,
                RuleStyle::LineCircle => HrKind::LineCircle,
                RuleStyle::InsetLine => HrKind::InsetLine,
                RuleStyle::CurtainRod => HrKind::CurtainRod,
            }),
            alignment: Some(match self.alignment {
                RuleAlignment::Full => HrAlignment::Full,
                RuleAlignment::Centered => HrAlignment::Center,
                RuleAlignment::Left => HrAlignment::Left,
                RuleAlignment::Right => HrAlignment::Right,
            }),
            weight: Some(match self.weight {
                RuleWeight::Thin => HrWeight::Thin,
                RuleWeight::Medium => HrWeight::Medium,
                RuleWeight::Thick => HrWeight::Thick,
            }),
            width: self.width.clone(),
            color: self.color.clone(),
        });
        node.attrs.set_layout(&self.layout);
        node
    }
}

impl HorizontalRule {
    /// Renders the text tier (Unicode or ASCII glyphs) without attempting
    /// the image tier.
    ///
    /// Used by the render-tree renderer when [`GraphicsMode::Off`] suppresses
    /// graphics, or when the image tier is unavailable or fails.
    pub(crate) fn render_text_tier(&self, term: &Terminal) -> String {
        let term_width = term.width() as usize;
        let rule_width = self.resolve_width(term_width);

        // Floor at 1 column so explicit `width: 1`/`1%` still renders one
        // visible glyph; ceiling at `term_width` so no rule exceeds the
        // terminal.
        let rule_width = rule_width.min(term_width).max(1);

        // Generate the rule content based on style and terminal capabilities
        let rule_content = self.generate_terminal_content(rule_width, term);

        // Wrap the rule content with ANSI color escapes when a color is set
        // and the terminal advertises any color support. Padding stays
        // outside the color wrap so the trailing reset comes *before* the
        // alignment padding, not after.
        let rule_content = self.apply_terminal_color(rule_content, term);

        // Apply alignment (using character count, not byte length)
        let content_width = self.visible_width(&rule_content);
        match self.alignment {
            RuleAlignment::Full => rule_content,
            RuleAlignment::Centered => {
                let padding = (term_width.saturating_sub(content_width)) / 2;
                format!("{}{}", " ".repeat(padding), rule_content)
            }
            RuleAlignment::Left => rule_content,
            RuleAlignment::Right => {
                let padding = term_width.saturating_sub(content_width);
                format!("{}{}", " ".repeat(padding), rule_content)
            }
        }
    }

    /// Resolves the width of the rule in characters based on the terminal
    /// width and the CSS-like width specification.
    ///
    /// ## Supported Width Forms
    ///
    /// - `"NN%"` — percentage of `term_width` (e.g., `"50%"`)
    /// - `"NNch"` — explicit character count (e.g., `"20ch"`)
    /// - `"NNpx"` — pixel count, converted to columns using
    ///   [`DEFAULT_CELL_WIDTH`] (currently 8 px/cell). The resulting column
    ///   count is clamped to `[1, term_width]`.
    /// - Bare `"NN"` — character count
    ///
    /// When `self.width` is `None`, the function returns a style-appropriate
    /// default (`term_width` for `Full`; 80% of it otherwise) silently.
    ///
    /// ## Notes
    ///
    /// Any other non-empty string (e.g., `"10em"`, `"50vw"`, `"garbage"`)
    /// emits a `tracing::warn!` and falls back to `term_width`. The `None`
    /// path does not warn — that is the documented default.
    ///
    /// The `"NNpx"` path assumes 8 pixels per cell; `resolve_width` takes a
    /// plain `term_width: usize` and does not consult `Terminal::cell_size()`
    /// directly. Callers that need exact pixel sizing should prefer `ch` or
    /// `%` widths.
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
                // Handle character width: "20ch"
                if let Some(ch_str) = trimmed.strip_suffix("ch")
                    && let Ok(chars) = ch_str.trim().parse::<usize>()
                {
                    return chars.max(1);
                }
                // Handle pixel width: "200px" — convert to columns using the
                // default cell width. Clamped to the terminal's column count.
                if let Some(px_str) = trimmed.strip_suffix("px")
                    && let Ok(pixels) = px_str.trim().parse::<u32>()
                {
                    let cell_width = DEFAULT_CELL_WIDTH.max(1) as usize;
                    let cols = (pixels as usize / cell_width).max(1);
                    return cols.min(term_width.max(1));
                }
                // Bare number: "20"
                if let Ok(chars) = trimmed.parse::<usize>() {
                    return chars.max(1);
                }
                // Unrecognized — warn and fall back to the full terminal
                // width so that bad authoring is visible via
                // `RUST_LOG=biscuit_terminal=warn` without rendering an
                // empty or misleading rule.
                tracing::warn!(
                    width = %width_str,
                    "unrecognized horizontal rule width; falling back to terminal width"
                );
                term_width
            }
            None => {
                // Default width based on alignment
                match self.alignment {
                    RuleAlignment::Full => term_width,
                    RuleAlignment::Centered | RuleAlignment::Left | RuleAlignment::Right => {
                        (term_width as f32 * 0.8) as usize
                    }
                }
            }
        }
    }

    /// Tier 1 image rendering: rasterize the HR's SVG to a PNG and emit it
    /// through biscuit-terminal's Kitty-protocol image emitter.
    ///
    /// ## Activation Guard
    ///
    /// Tier 1 activates only when the terminal satisfies both:
    ///
    /// - `term.is_tty == true`
    /// - `term.image_support` is one of [`ImageSupport::Kitty`] or
    ///   [`ImageSupport::ITerm`]
    ///
    /// Both branches currently route through
    /// [`TerminalImage::render_kitty_cells`]. Modern iTerm2 (and the other
    /// iTerm-compatible emulators biscuit-terminal identifies) accept the
    /// Kitty graphics protocol for cell-sized inline images, so a single
    /// emitter is correct here; the iTerm2 `ESC ]1337;File=…` protocol
    /// serves image-file upload workflows where aspect-preserving scaling
    /// takes arguments this path does not produce. Adding a dedicated
    /// iTerm branch remains a future option if visual parity ever drifts.
    ///
    /// ## Fallback Contract
    ///
    /// Returns `None` when the guard fails or when SVG rasterization fails
    /// (in which case a `tracing::warn!` is logged). On `None` the caller
    /// falls through to Tier 2 (Unicode) or Tier 3 (ASCII) via
    /// `generate_terminal_content`.
    ///
    /// ## Cell Size
    ///
    /// When `term.cell_size()` is `None`, the tier assumes
    /// [`DEFAULT_CELL_WIDTH`]×[`DEFAULT_CELL_HEIGHT`] (8×16 px) per cell.
    /// This matches typical monospace fonts at common DPI; oversized fonts
    /// render a proportionally thicker-looking rule.
    pub(crate) fn render_image_tier(&self, term: &Terminal) -> Option<String> {
        if !term.is_tty
            || !matches!(
                term.image_support,
                ImageSupport::Kitty | ImageSupport::ITerm
            )
        {
            return None;
        }

        let term_width = term.width() as usize;
        let rule_width = self.resolve_width(term_width).min(term_width).max(1);
        let height_cells = match self.weight {
            RuleWeight::Thin => 1,
            RuleWeight::Medium => 2,
            RuleWeight::Thick => 3,
        };
        let cell = term.cell_size();
        let cell_width = cell.map(|c| c.width.max(1)).unwrap_or(DEFAULT_CELL_WIDTH);
        let cell_height = cell.map(|c| c.height.max(1)).unwrap_or(DEFAULT_CELL_HEIGHT);
        let pixel_width = (rule_width as u32).saturating_mul(cell_width).max(1);
        let pixel_height = height_cells * cell_height;

        let default_color = match term.color_mode() {
            ColorMode::Light => "black",
            ColorMode::Dark | ColorMode::Unknown => "white",
        };
        let svg = self.render_image_svg(pixel_width, pixel_height, default_color);
        self.render_image_tier_from_svg(term, &svg, rule_width, height_cells)
    }

    /// SVG-injection seam used by [`render_image_tier`] and the test suite.
    /// Rasterizes `svg`, emits the Kitty-graphics cells, and decorates the
    /// output with cursor save/restore + downward motion so callers can
    /// splice the escape sequence into a larger render without overlapping
    /// subsequent content. Returns `None` (and logs a `tracing::warn!`)
    /// when rasterization fails.
    pub(crate) fn render_image_tier_from_svg(
        &self,
        term: &Terminal,
        svg: &str,
        rule_width: usize,
        height_cells: u32,
    ) -> Option<String> {
        let term_width = term.width() as usize;
        #[cfg(feature = "image")]
        {
            let png = match rasterize_svg_to_png(svg.as_bytes()) {
                Ok(png) => png,
                Err(err) => {
                    tracing::warn!(
                        error = %err,
                        "horizontal rule image rendering failed; falling back to text"
                    );
                    return None;
                }
            };

            // Both Kitty-protocol terminals and modern iTerm2 accept
            // `render_kitty_cells` for cell-sized inline images; see the rustdoc
            // note on `render_image_tier` for the reasoning behind the unified
            // emitter.
            let image =
                TerminalImage::default().render_kitty_cells(&png, rule_width as u32, height_cells);
            let content_width = rule_width;
            let x_offset = match self.alignment {
                RuleAlignment::Full | RuleAlignment::Left => 0,
                RuleAlignment::Centered => term_width.saturating_sub(content_width) / 2,
                RuleAlignment::Right => term_width.saturating_sub(content_width),
            };
            let prefix = if x_offset > 0 {
                format!("\x1b[{}C", x_offset)
            } else {
                String::new()
            };

            Some(format!(
                "\x1b[s{}{}\x1b[u\x1b[{}B\r",
                prefix, image, height_cells
            ))
        }
        #[cfg(not(feature = "image"))]
        {
            let _ = (term_width, svg, rule_width, height_cells);
            None
        }
    }

    fn render_image_svg(&self, pixel_width: u32, pixel_height: u32, default_color: &str) -> String {
        let stroke_width = match self.weight {
            RuleWeight::Thin => 2,
            RuleWeight::Medium => 4,
            RuleWeight::Thick => 8,
        };
        let color = self.color.as_deref().unwrap_or(default_color);
        let y = pixel_height as f32 / 2.0;
        let width = pixel_width as f32;

        let content = match self.style {
            RuleStyle::Dashes => format!(
                r#"<line x1="0" y1="{y}" x2="{width}" y2="{y}" stroke="{color}" stroke-width="{stroke_width}" stroke-linecap="round" stroke-dasharray="8,4"/>"#
            ),
            RuleStyle::Dots => format!(
                r#"<line x1="0" y1="{y}" x2="{width}" y2="{y}" stroke="{color}" stroke-width="{stroke_width}" stroke-linecap="round" stroke-dasharray="2,6"/>"#
            ),
            RuleStyle::Waves => {
                let mut path = format!("M0 {y}");
                let mut x = 10.0;
                while x <= width + 20.0 {
                    path.push_str(&format!(" Q {} {} {} {}", x, y - 8.0, x + 10.0, y));
                    x += 20.0;
                }
                format!(
                    r#"<path d="{path}" stroke="{color}" stroke-width="{stroke_width}" fill="none" stroke-linecap="round"/>"#
                )
            }
            RuleStyle::LineStar => format!(
                r#"<line x1="0" y1="{y}" x2="{left}" y2="{y}" stroke="{color}" stroke-width="{stroke_width}" stroke-linecap="round"/>
  <path d="M{cx} {top} L{r1} {mid1} L{r2} {mid1} L{r3} {mid2} L{r4} {bot} L{cx} {low} L{l4} {bot} L{l3} {mid2} L{l2} {mid1} L{l1} {mid1} Z" fill="{color}"/>
  <line x1="{right}" y1="{y}" x2="{width}" y2="{y}" stroke="{color}" stroke-width="{stroke_width}" stroke-linecap="round"/>"#,
                left = width * 0.45,
                right = width * 0.55,
                cx = width * 0.5,
                top = y - 12.0,
                mid1 = y - 4.0,
                mid2 = y + 3.0,
                bot = y + 12.0,
                low = y + 6.0,
                r1 = width * 0.52,
                r2 = width * 0.58,
                r3 = width * 0.53,
                r4 = width * 0.55,
                l1 = width * 0.48,
                l2 = width * 0.42,
                l3 = width * 0.47,
                l4 = width * 0.45,
            ),
            RuleStyle::LineCircle => format!(
                r#"<line x1="0" y1="{y}" x2="{left}" y2="{y}" stroke="{color}" stroke-width="{stroke_width}" stroke-linecap="round"/>
  <circle cx="{cx}" cy="{y}" r="{r}" fill="none" stroke="{color}" stroke-width="{stroke_width}"/>
  <line x1="{right}" y1="{y}" x2="{width}" y2="{y}" stroke="{color}" stroke-width="{stroke_width}" stroke-linecap="round"/>"#,
                left = width * 0.45,
                right = width * 0.55,
                cx = width * 0.5,
                r = stroke_width * 2,
            ),
            RuleStyle::InsetLine => format!(
                r#"<line x1="{left}" y1="{y}" x2="{right}" y2="{y}" stroke="{color}" stroke-width="{stroke_width}" stroke-linecap="round"/>"#,
                left = width * 0.1,
                right = width * 0.9,
            ),
            RuleStyle::CurtainRod => format!(
                r#"<line x1="{left}" y1="{y}" x2="{right}" y2="{y}" stroke="{color}" stroke-width="{stroke_width}" stroke-linecap="round"/>
  <circle cx="{left}" cy="{y}" r="4" fill="{color}"/>
  <circle cx="{right}" cy="{y}" r="4" fill="{color}"/>"#,
                left = width * 0.05,
                right = width * 0.95,
            ),
        };

        format!(
            r#"<svg width="{pixel_width}" height="{pixel_height}" viewBox="0 0 {pixel_width} {pixel_height}" xmlns="http://www.w3.org/2000/svg">
  {content}
</svg>"#
        )
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
            "unknown horizontal rule color string; browser rendering will pass \
             the value through to the SVG, but the terminal output is uncolored. \
             Recognized: CSS basic-16 names, `gray`/`grey`, and `#rrggbb` hex."
        );
        content
    }

    /// Counts the number of *visible* columns in `content`, ignoring any
    /// ANSI CSI escape sequences that [`apply_terminal_color`] may have
    /// wrapped around the rule body. Used to compute alignment padding.
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
    /// Consults [`Terminal::supports_unicode`], which is a capability flag
    /// populated from [`crate::discovery::locale::env_says_utf8`] during
    /// detection (and with a `true` fallback for unknown environments —
    /// every modern terminal defaults to UTF-8). Callers that want to force
    /// ASCII fallbacks can build a `Terminal` with
    /// `.supports_unicode(false)`.
    fn use_fancy_chars(&self, term: &Terminal) -> bool {
        term.supports_unicode
    }
}

#[cfg(feature = "image")]
pub(crate) fn rasterize_svg_to_png(svg_data: &[u8]) -> Result<Vec<u8>, String> {
    let tree = resvg::usvg::Tree::from_data(svg_data, &resvg::usvg::Options::default())
        .map_err(|err| format!("SVG parse error: {err}"))?;
    let size = tree.size();
    let width = size.width() as u32;
    let height = size.height() as u32;
    let mut pixmap = resvg::tiny_skia::Pixmap::new(width, height)
        .ok_or_else(|| format!("failed to create {width}x{height} pixmap"))?;
    resvg::render(
        &tree,
        resvg::usvg::Transform::default(),
        &mut pixmap.as_mut(),
    );

    let mut rgba = pixmap.data().to_vec();
    for chunk in rgba.chunks_exact_mut(4) {
        let a = chunk[3] as u16;
        if a > 0 && a < 255 {
            chunk[0] = ((chunk[0] as u16 * 255 + a / 2) / a).min(255) as u8;
            chunk[1] = ((chunk[1] as u16 * 255 + a / 2) / a).min(255) as u8;
            chunk[2] = ((chunk[2] as u16 * 255 + a / 2) / a).min(255) as u8;
        }
    }

    let image = image::RgbaImage::from_raw(width, height, rgba)
        .map(image::DynamicImage::ImageRgba8)
        .ok_or_else(|| "failed to create image from SVG rasterization".to_string())?;
    let mut buffer = Cursor::new(Vec::new());
    image
        .write_to(&mut buffer, image::ImageFormat::Png)
        .map_err(|err| format!("PNG encoding failed: {err}"))?;
    Ok(buffer.into_inner())
}

#[cfg(test)]
mod tests {
    use super::browser::{MarginToCss, nearest_basic_color, parse_basic_color, parse_hex_color};
    use super::*;
    use crate::components::renderable::BrowserRenderable;
    use crate::discovery::detection::ColorDepth;
    use crate::terminal::Terminal;
    use crate::utils::color::BasicColor;
    use crate::utils::layout::{Length, TargetValue};
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

    fn text_terminal() -> Terminal {
        text_terminal_with_width(80)
    }

    fn text_terminal_with_width(width: u32) -> Terminal {
        text_terminal_with_width_and_color_depth(width, ColorDepth::TrueColor)
    }

    fn text_terminal_with_color_depth(color_depth: ColorDepth) -> Terminal {
        text_terminal_with_width_and_color_depth(80, color_depth)
    }

    fn text_terminal_with_width_and_color_depth(width: u32, color_depth: ColorDepth) -> Terminal {
        let mut term = Terminal::new_optimistic(width);
        term.is_tty = false;
        term.image_support = ImageSupport::None;
        term.color_depth = color_depth;
        term.supports_unicode = crate::discovery::locale::env_says_utf8().unwrap_or(true);
        term
    }

    fn text_terminal_with_width_and_unicode(width: u32, supports_unicode: bool) -> Terminal {
        let mut term = text_terminal_with_width(width);
        term.supports_unicode = supports_unicode;
        term
    }

    #[test]
    fn test_horizontal_rule_new() {
        let hr = HorizontalRule::new();
        assert_eq!(hr.style, RuleStyle::Dashes);
        assert_eq!(hr.alignment, RuleAlignment::Full);
        assert_eq!(hr.weight, RuleWeight::Medium);
        assert_eq!(hr.width, None);
        assert_eq!(hr.color, None);
    }

    #[test]
    fn test_horizontal_rule_builder_methods() {
        let hr = HorizontalRule::new()
            .style(RuleStyle::Waves)
            .alignment(RuleAlignment::Centered)
            .weight(RuleWeight::Thick)
            .width("50%")
            .color("red");

        assert_eq!(hr.style, RuleStyle::Waves);
        assert_eq!(hr.alignment, RuleAlignment::Centered);
        assert_eq!(hr.weight, RuleWeight::Thick);
        assert_eq!(hr.width, Some("50%".to_string()));
        assert_eq!(hr.color, Some("red".to_string()));
    }

    #[test]
    fn test_horizontal_rule_default_impl() {
        let hr1 = HorizontalRule::new();
        let hr2 = HorizontalRule::default();
        assert_eq!(hr1.style, hr2.style);
        assert_eq!(hr1.alignment, hr2.alignment);
        assert_eq!(hr1.weight, hr2.weight);
        assert_eq!(hr1.width, hr2.width);
        assert_eq!(hr1.color, hr2.color);
    }

    #[test]
    #[cfg(feature = "image")]
    fn test_render_uses_kitty_image_tier_when_supported() {
        let hr = HorizontalRule::new()
            .style(RuleStyle::Waves)
            .alignment(RuleAlignment::Centered)
            .width("50%")
            .color("red");
        let term = Terminal::builder()
            .width(80)
            .is_tty(true)
            .image_support(ImageSupport::Kitty)
            .build();

        let result = hr.render(&term);
        assert!(result.contains("\x1b_G"), "{result:?}");
    }

    #[test]
    fn test_image_tier_svg_uses_visible_default_color() {
        // When no explicit color is set, the image-tier SVG must not use
        // `currentColor` — resvg resolves it to black, making the rule
        // invisible on dark terminal backgrounds.
        let hr = HorizontalRule::new().style(RuleStyle::Dashes);
        let svg = hr.render_image_svg(100, 32, "white");
        assert!(
            !svg.contains("currentColor"),
            "SVG should not use currentColor: {svg}"
        );
        assert!(
            svg.contains("white"),
            "SVG should use a visible default color: {svg}"
        );
    }

    #[test]
    fn test_image_tier_svg_respects_explicit_color() {
        let hr = HorizontalRule::new().style(RuleStyle::Dashes).color("red");
        let svg = hr.render_image_svg(100, 32, "white");
        assert!(
            svg.contains("red"),
            "SVG should use the explicitly set color: {svg}"
        );
        assert!(
            !svg.contains("white"),
            "SVG should not use the default when color is explicit: {svg}"
        );
    }

    #[test]
    #[serial_test::serial(locale_env)]
    fn test_image_tier_unavailable_falls_back_to_unicode() {
        let _guard = ScopedLcAll::force_utf8();
        let hr = HorizontalRule::new().style(RuleStyle::Dashes).color("red");
        let term = text_terminal_with_width(20);

        let result = hr.render(&term);
        assert!(!result.contains("\x1b_G"), "{result:?}");
        assert!(result.contains('╌'), "{result:?}");
    }

    #[test]
    #[serial_test::serial(locale_env)]
    fn test_text_terminal_helper_disables_image_tier() {
        let _guard = ScopedLcAll::force_utf8();
        let hr = HorizontalRule::new()
            .style(RuleStyle::Waves)
            .alignment(RuleAlignment::Centered)
            .width("50%");
        let term = text_terminal();

        let result = hr.render(&term);
        assert!(
            !result.contains("\x1b_G"),
            "text helper must disable Kitty image output: {result:?}"
        );
        assert!(
            !result.contains("\x1b]1337;File="),
            "text helper must disable iTerm image output: {result:?}"
        );
        assert!(
            result.contains('≋'),
            "text helper should force the Unicode fallback baseline: {result:?}"
        );
    }

    #[test]
    #[serial_test::serial(locale_env)]
    fn test_render_dashes_full_unicode() {
        let _guard = ScopedLcAll::force_utf8();
        let hr = HorizontalRule::new()
            .style(RuleStyle::Dashes)
            .alignment(RuleAlignment::Full);
        let term = text_terminal();
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
            .alignment(RuleAlignment::Full);
        let term = text_terminal_with_color_depth(ColorDepth::None);
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
            .alignment(RuleAlignment::Full);
        let term = text_terminal();
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
            .alignment(RuleAlignment::Full);
        let term = text_terminal_with_color_depth(ColorDepth::None);
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
            .alignment(RuleAlignment::Full);
        let term = text_terminal();
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
            .alignment(RuleAlignment::Full);
        let term = text_terminal_with_color_depth(ColorDepth::None);
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
            .alignment(RuleAlignment::Full);
        let term = text_terminal();
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
            .alignment(RuleAlignment::Full);
        let term = text_terminal_with_color_depth(ColorDepth::None);
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
            .alignment(RuleAlignment::Full);
        let term = text_terminal();
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
            .alignment(RuleAlignment::Full);
        let term = text_terminal_with_color_depth(ColorDepth::None);
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
            .alignment(RuleAlignment::Full);
        let term = text_terminal();
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
            .alignment(RuleAlignment::Full);
        let term = text_terminal_with_color_depth(ColorDepth::None);
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
            .alignment(RuleAlignment::Full);
        let term = text_terminal();
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
            .alignment(RuleAlignment::Full);
        let term = text_terminal_with_color_depth(ColorDepth::None);
        let result = hr.render(&term);
        assert!(result.len() >= 5);
        // ASCII brackets
        assert!(result.contains('[') && result.contains(']'));
    }

    // ================================================================
    // Phase 3: C4 — hard assertion for CurtainRod + Thick + Right
    //
    // The 36-tuple sweep `test_all_styles_all_alignments_all_weights_unicode`
    // only asserts `!result.is_empty()`, so a regression that (for example)
    // dropped the right bracket or swapped the heavy glyph for medium would
    // not be caught at unit-test granularity — the snapshot sweep would
    // update silently under `cargo insta accept`. These two tests fail fast
    // on the specific "hard" combination the review called out.
    // ================================================================

    /// C4 (Unicode path): CurtainRod + Thick + Right produces the expected
    /// byte pattern on a 100-column terminal with `width("40")`.
    ///
    /// - Right alignment → 62 leading spaces (100 − visible_width = 38 → 62).
    /// - First non-space char is `┤` (left cap).
    /// - Last char is `├` (right cap).
    /// - Body between caps is entirely `━` (heavy line, proving `Thick`).
    ///
    /// No color is set, so no ANSI escapes appear in `result`; the
    /// assertions below can inspect the raw bytes.
    #[test]
    #[serial_test::serial(locale_env)]
    fn test_render_curtain_rod_thick_right_has_brackets_and_heavy_line() {
        let _guard = ScopedLcAll::force_utf8();
        let hr = HorizontalRule::new()
            .style(RuleStyle::CurtainRod)
            .weight(RuleWeight::Thick)
            .alignment(RuleAlignment::Right)
            .width("40");
        // `is_tty=false` keeps us out of Tier 1; TrueColor exercises the
        // same code path as production TTYs without inviting the image tier.
        let term = text_terminal_with_width_and_color_depth(100, ColorDepth::TrueColor);
        let result = hr.render(&term);

        // No color is configured → output is pure Unicode, no ANSI escapes.
        assert!(
            !result.contains('\x1b'),
            "no color was configured, output must be ANSI-free: {result:?}"
        );

        // CurtainRod body: `┤` + `━`.repeat(inner_width) + `├` where
        // `inner_width = 40 - 4 = 36`. Visible width = 1 + 36 + 1 = 38.
        let expected_body: String = format!("┤{}├", "━".repeat(36));
        let expected_leading_spaces = " ".repeat(62);
        let expected = format!("{expected_leading_spaces}{expected_body}");
        assert_eq!(
            result, expected,
            "CurtainRod+Thick+Right must render exactly 62 leading spaces, ┤, 36 heavy bars, ├: {result:?}"
        );

        // Cross-check individual invariants in case `expected` ever drifts.
        assert!(
            result.starts_with(&expected_leading_spaces),
            "expected 62 leading spaces for width=40 on term_width=100 (right-aligned): {result:?}"
        );
        // `┤` is 3 UTF-8 bytes; so is `├` — strip them via char indexing.
        let non_space: String = result.chars().skip_while(|c| *c == ' ').collect();
        assert!(
            non_space.starts_with('┤'),
            "first non-space char must be ┤ (left cap): {non_space:?}"
        );
        assert!(
            non_space.ends_with('├'),
            "last char must be ├ (right cap): {non_space:?}"
        );
        let inner: String = non_space
            .chars()
            .skip(1)
            .take_while(|c| *c == '━')
            .collect();
        assert_eq!(
            inner.chars().count(),
            36,
            "heavy body between caps must be exactly 36 `━` glyphs (Thick variant): {non_space:?}"
        );
        // Visible width = 38 (1 + 36 + 1) — the CurtainRod reserves 4 cells
        // for the bracket pair and inter-cap padding in its width math.
        assert_eq!(
            hr.visible_width(&non_space),
            38,
            "visible width of the rule body must be 38: {non_space:?}"
        );
    }

    /// C4 (ASCII path): the same boundary combination under a `C` locale
    /// falls through to ASCII and must use `[`/`]` and `-`.
    #[test]
    #[serial_test::serial(locale_env)]
    fn test_render_curtain_rod_thick_right_ascii_fallback() {
        let _guard = ScopedLcAll::force_c();
        let hr = HorizontalRule::new()
            .style(RuleStyle::CurtainRod)
            .weight(RuleWeight::Thick)
            .alignment(RuleAlignment::Right)
            .width("40");
        let term = text_terminal_with_width_and_color_depth(100, ColorDepth::None);
        let result = hr.render(&term);

        assert!(
            !result.contains('\x1b'),
            "no color was configured, output must be ANSI-free: {result:?}"
        );

        // ASCII body: `[` + `-`.repeat(36) + `]`. Visible width = 38.
        let expected_body = format!("[{}]", "-".repeat(36));
        let expected_leading_spaces = " ".repeat(62);
        let expected = format!("{expected_leading_spaces}{expected_body}");
        assert_eq!(
            result, expected,
            "CurtainRod+Thick+Right ASCII fallback must use `[` / `-` / `]`: {result:?}"
        );

        // ASCII fallback is weight-insensitive by design — this assertion
        // is the canary that catches a regression that tried to "promote"
        // ASCII to some heavy variant.
        assert!(
            !result.contains('━') && !result.contains('┤') && !result.contains('├'),
            "ASCII fallback must not leak Unicode glyphs: {result:?}"
        );
    }

    #[test]
    #[serial_test::serial(locale_env)]
    fn test_render_centered() {
        let _guard = ScopedLcAll::force_utf8();
        let hr = HorizontalRule::new()
            .style(RuleStyle::Dashes)
            .alignment(RuleAlignment::Centered);
        let term = text_terminal_with_width(100);
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
            .alignment(RuleAlignment::Left);
        let term = text_terminal_with_width(100);
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
            .alignment(RuleAlignment::Right);
        let term = text_terminal_with_width(100);
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
        let result = hr.render_browser_svg();
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
        let result = hr.render_browser_svg();
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
    fn test_layout_accessors() {
        let mut hr = HorizontalRule::new();
        let _layout = hr.layout();

        hr.layout_mut().margin.top = TargetValue::universal(Length::ch(2));
        assert_eq!(
            hr.layout().margin.top,
            TargetValue::universal(Length::ch(2))
        );
    }

    #[test]
    fn test_as_any() {
        let hr = HorizontalRule::new();
        let any_ref = TerminalRenderable::as_any(&hr);
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
        let hr = HorizontalRule::new().alignment(RuleAlignment::Full);
        assert_eq!(hr.resolve_width(100), 100);
    }

    #[test]
    fn test_resolve_width_default_centered() {
        let hr = HorizontalRule::new().alignment(RuleAlignment::Centered);
        assert_eq!(hr.resolve_width(100), 80);
    }

    // ================================================================
    // Phase 2: B1 / C1 — invalid width warn+fallback
    // ================================================================

    #[test]
    #[tracing_test::traced_test]
    fn test_resolve_width_invalid_warns_and_falls_back() {
        let hr = HorizontalRule::new().width("garbage");
        let resolved = hr.resolve_width(100);
        assert_eq!(resolved, 100, "unparseable width should return term_width");
        assert!(
            logs_contain("unrecognized horizontal rule width"),
            "expected a warn log for unrecognized width"
        );
    }

    #[test]
    #[tracing_test::traced_test]
    fn test_resolve_width_invalid_em_warns() {
        let hr = HorizontalRule::new().width("10em");
        let resolved = hr.resolve_width(100);
        assert_eq!(resolved, 100);
        assert!(
            logs_contain("unrecognized horizontal rule width"),
            "expected warn for `em` widths (unsupported)"
        );
    }

    #[test]
    #[tracing_test::traced_test]
    fn test_resolve_width_invalid_vw_warns() {
        let hr = HorizontalRule::new().width("50vw");
        let resolved = hr.resolve_width(100);
        assert_eq!(resolved, 100);
        assert!(
            logs_contain("unrecognized horizontal rule width"),
            "expected warn for `vw` widths (unsupported)"
        );
    }

    #[test]
    #[tracing_test::traced_test]
    fn test_resolve_width_none_is_silent() {
        // The `None` branch is the documented default; it must not warn.
        let hr = HorizontalRule::new();
        let _ = hr.resolve_width(100);
        assert!(
            !logs_contain("unrecognized horizontal rule width"),
            "no warn should be emitted when width is unset"
        );
    }

    // ================================================================
    // Phase 2: B2 — "NNNpx" width parsing
    // ================================================================

    #[test]
    fn test_resolve_width_px_parses() {
        let hr = HorizontalRule::new().width("160px");
        // 160 / DEFAULT_CELL_WIDTH (8) == 20 columns.
        assert_eq!(hr.resolve_width(100), 20);
    }

    #[test]
    fn test_resolve_width_px_clamps_to_term_width() {
        let hr = HorizontalRule::new().width("4000px");
        // 4000 / 8 = 500 cols, clamped down to term_width.
        assert_eq!(hr.resolve_width(100), 100);
    }

    #[test]
    fn test_resolve_width_px_zero_is_one() {
        let hr = HorizontalRule::new().width("0px");
        assert_eq!(hr.resolve_width(100), 1, "0px should floor to 1 column");
    }

    #[test]
    #[tracing_test::traced_test]
    fn test_resolve_width_px_malformed_warns() {
        let hr = HorizontalRule::new().width("abcpx");
        let resolved = hr.resolve_width(100);
        assert_eq!(resolved, 100);
        assert!(
            logs_contain("unrecognized horizontal rule width"),
            "malformed px value should warn"
        );
    }

    // ================================================================
    // Phase 2: B4 — minimum-width boundary coverage
    // ================================================================

    #[test]
    #[serial_test::serial(locale_env)]
    fn test_render_width_one_percent_yields_one_column() {
        // Previously clamped to 10; now honors the explicit 1-column intent.
        let _guard = ScopedLcAll::force_utf8();
        let hr = HorizontalRule::new()
            .style(RuleStyle::Dashes)
            .alignment(RuleAlignment::Full)
            .width("1%");
        let term = text_terminal_with_width(100);
        let result = hr.render(&term);
        assert_eq!(
            hr.visible_width(&result),
            1,
            "1% of 100 cols should render exactly 1 visible column: {result:?}"
        );
    }

    #[test]
    #[serial_test::serial(locale_env)]
    fn test_render_width_three_yields_three_columns() {
        let _guard = ScopedLcAll::force_utf8();
        let hr = HorizontalRule::new()
            .style(RuleStyle::Dashes)
            .alignment(RuleAlignment::Left)
            .width("3");
        let term = text_terminal_with_width(100);
        let result = hr.render(&term);
        assert_eq!(
            hr.visible_width(&result),
            3,
            "width:3 should render 3 columns with no hidden bump: {result:?}"
        );
    }

    #[test]
    #[serial_test::serial(locale_env)]
    fn test_render_width_five_percent_yields_five_columns() {
        let _guard = ScopedLcAll::force_utf8();
        let hr = HorizontalRule::new()
            .style(RuleStyle::Dashes)
            .alignment(RuleAlignment::Full)
            .width("5%");
        let term = text_terminal_with_width(100);
        let result = hr.render(&term);
        assert_eq!(
            hr.visible_width(&result),
            5,
            "5% of 100 cols should render 5 columns (previously clamped to 10): {result:?}"
        );
    }

    // ================================================================
    // Phase 2: B5 — Tier 1 extends to ImageSupport::ITerm
    // ================================================================

    #[test]
    #[cfg(feature = "image")]
    fn test_render_uses_image_tier_when_iterm() {
        let hr = HorizontalRule::new()
            .style(RuleStyle::Dashes)
            .alignment(RuleAlignment::Full)
            .width("40");
        let term = Terminal::builder()
            .width(80)
            .is_tty(true)
            .image_support(ImageSupport::ITerm)
            .build();

        let result = hr.render(&term);
        // Kitty-protocol graphics escape; see `render_image_tier` rustdoc
        // for why iTerm routes through the same emitter today.
        assert!(
            result.contains("\x1b_G"),
            "ImageSupport::ITerm should trigger Tier 1 image rendering: {result:?}"
        );
    }

    #[test]
    fn test_render_does_not_use_image_tier_when_unsupported() {
        let hr = HorizontalRule::new().style(RuleStyle::Dashes).width("40");
        let term = Terminal::builder()
            .width(80)
            .is_tty(true)
            .image_support(ImageSupport::None)
            .build();

        let result = hr.render(&term);
        assert!(
            !result.contains("\x1b_G"),
            "ImageSupport::None must not emit image escapes: {result:?}"
        );
    }

    // ================================================================
    // Phase 2: C3 — rasterization-failure fallback
    // ================================================================

    #[test]
    #[cfg(feature = "image")]
    fn test_rasterize_svg_to_png_fails_on_garbage() {
        let result = rasterize_svg_to_png(b"<not-an-svg>");
        assert!(
            result.is_err(),
            "garbage input must return Err from rasterize_svg_to_png"
        );
    }

    #[test]
    #[cfg(feature = "image")]
    #[tracing_test::traced_test]
    fn test_render_image_tier_falls_through_on_rasterization_failure() {
        // Feed `render_image_tier_from_svg` deliberately broken SVG bytes
        // to exercise the warn+None fallback without stubbing the whole
        // `render_image_tier` guard chain. The public contract is:
        //   - return None
        //   - log "horizontal rule image rendering failed"
        let hr = HorizontalRule::new();
        let term = Terminal::builder()
            .width(80)
            .is_tty(true)
            .image_support(ImageSupport::Kitty)
            .build();
        let result = hr.render_image_tier_from_svg(&term, "<not-an-svg>", 10, 1);
        assert!(
            result.is_none(),
            "rasterization failure must return None so the caller falls back to text"
        );
        assert!(
            logs_contain("horizontal rule image rendering failed"),
            "expected a warn log on rasterization failure"
        );
    }

    #[test]
    fn test_all_styles_all_alignments_all_weights_unicode() {
        let term = text_terminal();
        let styles = vec![
            RuleStyle::Dashes,
            RuleStyle::Dots,
            RuleStyle::Waves,
            RuleStyle::LineStar,
            RuleStyle::LineCircle,
            RuleStyle::InsetLine,
            RuleStyle::CurtainRod,
        ];
        let alignments = vec![
            RuleAlignment::Full,
            RuleAlignment::Centered,
            RuleAlignment::Left,
            RuleAlignment::Right,
        ];
        let weights = vec![RuleWeight::Thin, RuleWeight::Medium, RuleWeight::Thick];

        for style in &styles {
            for alignment in &alignments {
                for weight in &weights {
                    let hr = HorizontalRule::new()
                        .style(style.clone())
                        .alignment(alignment.clone())
                        .weight(weight.clone());
                    let result = hr.render(&term);
                    assert!(
                        !result.is_empty(),
                        "Failed for style={:?} alignment={:?} weight={:?}",
                        style,
                        alignment,
                        weight
                    );
                }
            }
        }
    }

    #[test]
    fn test_all_styles_all_alignments_all_weights_ascii() {
        let term = text_terminal_with_color_depth(ColorDepth::None);
        let styles = vec![
            RuleStyle::Dashes,
            RuleStyle::Dots,
            RuleStyle::Waves,
            RuleStyle::LineStar,
            RuleStyle::LineCircle,
            RuleStyle::InsetLine,
            RuleStyle::CurtainRod,
        ];
        let alignments = vec![
            RuleAlignment::Full,
            RuleAlignment::Centered,
            RuleAlignment::Left,
            RuleAlignment::Right,
        ];
        let weights = vec![RuleWeight::Thin, RuleWeight::Medium, RuleWeight::Thick];

        for style in &styles {
            for alignment in &alignments {
                for weight in &weights {
                    let hr = HorizontalRule::new()
                        .style(style.clone())
                        .alignment(alignment.clone())
                        .weight(weight.clone());
                    let result = hr.render(&term);
                    assert!(
                        !result.is_empty(),
                        "Failed for style={:?} alignment={:?} weight={:?}",
                        style,
                        alignment,
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
        let term = text_terminal();

        let styles = vec![
            ("dashes", RuleStyle::Dashes),
            ("dots", RuleStyle::Dots),
            ("waves", RuleStyle::Waves),
            ("line_star", RuleStyle::LineStar),
            ("line_circle", RuleStyle::LineCircle),
            ("inset_line", RuleStyle::InsetLine),
            ("curtain_rod", RuleStyle::CurtainRod),
        ];

        let alignments = vec![
            ("full", RuleAlignment::Full),
            ("centered", RuleAlignment::Centered),
            ("left", RuleAlignment::Left),
            ("right", RuleAlignment::Right),
        ];

        let weights = vec![
            ("thin", RuleWeight::Thin),
            ("medium", RuleWeight::Medium),
            ("thick", RuleWeight::Thick),
        ];

        for (style_name, style) in &styles {
            for (alignment_name, alignment) in &alignments {
                for (weight_name, weight) in &weights {
                    let hr = HorizontalRule::new()
                        .style(style.clone())
                        .alignment(alignment.clone())
                        .weight(weight.clone());

                    let result = hr.render(&term);
                    assert_snapshot!(
                        format!("terminal_{}_{}_{}", style_name, alignment_name, weight_name),
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

                let result = hr.render_browser_svg();
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
        let term = text_terminal_with_color_depth(ColorDepth::None);

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
                .alignment(RuleAlignment::Full)
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
        use crate::components::renderable::{RenderableTerminalContent, TerminalRenderable};

        let _guard = ScopedLcAll::force_utf8();
        let term = text_terminal();

        let hr = HorizontalRule::new()
            .style(RuleStyle::Dashes)
            .alignment(RuleAlignment::Full)
            .weight(RuleWeight::Medium);
        let prose = Prose::new("after the rule");

        let compose = Compose::new(vec![
            RenderableTerminalContent::from(hr),
            RenderableTerminalContent::from("\n"),
            RenderableTerminalContent::from(prose),
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
        let term = text_terminal();

        // Test with custom width and color
        let hr1 = HorizontalRule::new()
            .style(RuleStyle::Waves)
            .alignment(RuleAlignment::Centered)
            .weight(RuleWeight::Thick)
            .width("75%")
            .color("red");

        let terminal_result1 = hr1.render(&term);
        let browser_result1 = hr1.render_browser_svg();

        assert_snapshot!("terminal_custom_attributes", terminal_result1);
        assert_snapshot!("browser_custom_attributes", browser_result1);

        // Test with different combinations
        let hr2 = HorizontalRule::new()
            .style(RuleStyle::Dots)
            .alignment(RuleAlignment::Right)
            .weight(RuleWeight::Thin)
            .width("50ch")
            .color("#00ff00");

        let terminal_result2 = hr2.render(&term);
        let browser_result2 = hr2.render_browser_svg();

        assert_snapshot!("terminal_custom_attributes_2", terminal_result2);
        assert_snapshot!("browser_custom_attributes_2", browser_result2);
    }

    #[test]
    #[serial_test::serial(locale_env)]
    fn test_use_fancy_chars_respects_locale_utf8() {
        let _guard = ScopedLcAll::force_utf8();
        let hr = HorizontalRule::new();
        let term = text_terminal();
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
        let term = text_terminal();
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
        let term = text_terminal();
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
            .alignment(RuleAlignment::Full);
        let term = text_terminal_with_width(40);
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
        let term = text_terminal_with_width(40);
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
        let term = text_terminal_with_width(40);
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
        let term = text_terminal_with_width(40);
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
        let term = text_terminal_with_width(40);
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
        let term = text_terminal_with_width(40);
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
        let term = text_terminal_with_width(40);
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
        let term = text_terminal_with_width(40);
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
        let term = text_terminal_with_width(40);
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
        let term = text_terminal_with_width_and_color_depth(40, ColorDepth::None);
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
        let term = text_terminal_with_width_and_color_depth(40, ColorDepth::Basic);
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
        let term = text_terminal_with_width_and_color_depth(20, ColorDepth::TrueColor);
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
        let term = text_terminal_with_width_and_color_depth(20, ColorDepth::TrueColor);
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
        let term = text_terminal_with_width_and_color_depth(20, ColorDepth::Basic);
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
        let term = text_terminal_with_width_and_color_depth(40, ColorDepth::None);
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
        let term = text_terminal_with_width_and_color_depth(40, ColorDepth::TrueColor);
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
        let term = text_terminal_with_width_and_color_depth(40, ColorDepth::TrueColor);
        let result = hr.render(&term);
        assert!(
            !result.contains('\x1b'),
            "empty color string should be treated as no color"
        );
    }

    #[test]
    #[serial_test::serial(locale_env)]
    fn test_render_color_padding_is_uncolored_for_centered() {
        // With Centered alignment, the leading padding should remain outside
        // the ANSI wrap — reset goes *before* any trailing content, and the
        // padding spaces should not be inside the color escape.
        let _guard = ScopedLcAll::force_utf8();
        let hr = HorizontalRule::new()
            .style(RuleStyle::Dashes)
            .alignment(RuleAlignment::Centered)
            .color("red");
        let term = text_terminal_with_width_and_color_depth(80, ColorDepth::Basic);
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
            .alignment(RuleAlignment::Full);
        // Use odd then even to prove the previous `inner_width/2 +
        // (inner_width - inner_width/2)` trick (needed for odd widths)
        // is correctly replaced by `inner_width`.
        for width in [11, 12, 20, 21, 80] {
            let term = text_terminal_with_width(width as u32);
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

    /// Default browser SVG output declares all three CSS custom
    /// properties on the root `<svg>` element.
    #[test]
    fn test_render_to_browser_contains_css_variables() {
        let hr = HorizontalRule::new();
        let result = hr.render_browser_svg();
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
            let result = hr.render_browser_svg();
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
            let result = hr.render_browser_svg();
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
        let result = hr.render_browser_svg();
        assert!(
            result.contains("var(--hr-weight, 4)"),
            "default medium weight must embed var(--hr-weight, 4): {result}"
        );
        assert!(
            result.contains("var(--hr-color, currentColor)"),
            "default color must embed var(--hr-color, currentColor): {result}"
        );
    }

    /// `--hr-color` is set from the component's `.color(..)` when provided.
    #[test]
    fn test_render_to_browser_color_declaration_reflects_component_color() {
        let hr = HorizontalRule::new().color("#ff00aa");
        let result = hr.render_browser_svg();
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
        let result = hr.render_browser_svg();
        assert!(
            result.contains("--hr-width: 60ch"),
            "expected --hr-width: 60ch: {result}"
        );
    }

    // ================================================================
    // Phase 4: D1 — PartialEq derive on HorizontalRule
    // ================================================================

    /// Two rules built with the same builder calls compare equal.
    #[test]
    fn test_horizontal_rule_partial_eq_identity() {
        let a = HorizontalRule::new()
            .style(RuleStyle::Waves)
            .alignment(RuleAlignment::Centered)
            .weight(RuleWeight::Thick)
            .width("75%")
            .color("red");
        let b = HorizontalRule::new()
            .style(RuleStyle::Waves)
            .alignment(RuleAlignment::Centered)
            .weight(RuleWeight::Thick)
            .width("75%")
            .color("red");
        assert_eq!(a, b);
    }

    /// Two rules that differ only in style must not compare equal.
    #[test]
    fn test_horizontal_rule_partial_eq_differs_on_style() {
        let a = HorizontalRule::new().style(RuleStyle::Dashes);
        let b = HorizontalRule::new().style(RuleStyle::Waves);
        assert_ne!(a, b);
    }

    // ================================================================
    // Phase 4: D2 — `supports_unicode` terminal capability
    // ================================================================

    /// When `supports_unicode` is `false`, the rule renders ASCII even if
    /// `env_says_utf8()` would otherwise return `true`.
    #[test]
    #[serial_test::serial(locale_env)]
    fn test_use_fancy_chars_honors_terminal_capability() {
        // Force UTF-8 so the locale fallback would pick Unicode, then flip
        // the explicit capability off to prove the capability dominates.
        let _guard = ScopedLcAll::force_utf8();
        let hr = HorizontalRule::new()
            .style(RuleStyle::Dashes)
            .alignment(RuleAlignment::Full)
            .width("10");
        let term = text_terminal_with_width_and_unicode(20, false);
        let result = hr.render(&term);
        assert!(
            result.chars().all(|c| c == '-'),
            "supports_unicode=false must emit ASCII dashes: {result:?}"
        );
    }

    // ================================================================
    // Phase 4: D4 — margin CSS lowering for the browser target
    // ================================================================

    /// A percent margin lowers to a `%` CSS value.
    #[test]
    fn test_margin_percent_emits_percent() {
        let margin = TargetValue::universal(Length::percent(2.0).unwrap());
        assert_eq!(margin.to_css_value("0"), "2%");
    }

    /// A zero margin lowers to the supplied default.
    #[test]
    fn test_margin_zero_emits_default() {
        let margin: TargetValue<Length> = TargetValue::universal(Length::Zero);
        assert_eq!(margin.to_css_value("0"), "0");
    }

    /// A cell margin lowers to a `ch` CSS value.
    #[test]
    fn test_margin_chars_emits_ch() {
        let margin = TargetValue::universal(Length::ch(3));
        assert_eq!(margin.to_css_value("0"), "3ch");
    }

    /// A percent margin composed into a full browser render emits legal
    /// CSS inside the outer `<svg>`'s inline style.
    #[test]
    fn test_render_to_browser_percent_margin() {
        use crate::utils::layout::Layout;
        let layout = Layout {
            margin: crate::utils::layout::Edges {
                top: TargetValue::universal(Length::percent(2.0).unwrap()),
                ..crate::utils::layout::Edges::default()
            },
            ..Layout::default()
        };
        let hr = HorizontalRule::new().with_layout(layout);
        let result = hr.render_browser_svg();
        assert!(
            result.contains("margin: 2% auto"),
            "expected 2% margin in browser output: {result}"
        );
    }

    // ================================================================
    // Phase 4: D6 — color drop asymmetry warn message
    // ================================================================

    /// When `apply_terminal_color` sees an unrecognized color name, the warn
    /// log must explain that the browser SVG still receives the raw value.
    #[test]
    #[tracing_test::traced_test]
    fn test_apply_terminal_color_unknown_name_warns_with_asymmetry_note() {
        let hr = HorizontalRule::new().color("tomato");
        let term = text_terminal_with_width_and_color_depth(40, ColorDepth::TrueColor);
        let _ = hr.apply_terminal_color("body".to_string(), &term);
        assert!(
            logs_contain("browser rendering will pass"),
            "warn log must mention the browser-vs-terminal asymmetry"
        );
    }

    // ================================================================
    // Phase 4: D7 — case-insensitive color matching without allocation
    // ================================================================

    /// `parse_basic_color` continues to match mixed-case inputs after the
    /// refactor that removed the per-call `to_ascii_lowercase` allocation.
    #[test]
    fn test_parse_basic_color_case_insensitive() {
        assert_eq!(parse_basic_color("BLACK"), Some(BasicColor::Black));
        assert_eq!(parse_basic_color("Red"), Some(BasicColor::Red));
        assert_eq!(parse_basic_color("bright-red"), Some(BasicColor::BrightRed));
        assert_eq!(parse_basic_color("BrightRed"), Some(BasicColor::BrightRed));
        assert_eq!(parse_basic_color("Gray"), Some(BasicColor::BrightBlack));
        assert_eq!(parse_basic_color("grey"), Some(BasicColor::BrightBlack));
        assert_eq!(parse_basic_color("unknown-color-xyz"), None);
    }

    #[cfg(feature = "serde")]
    mod serde_roundtrip {
        use super::super::{RuleAlignment, RuleStyle, RuleWeight};

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
        fn test_rule_alignment_serde_roundtrip() {
            for value in [
                RuleAlignment::Full,
                RuleAlignment::Centered,
                RuleAlignment::Left,
                RuleAlignment::Right,
            ] {
                let json = serde_json::to_string(&value).unwrap();
                let back: RuleAlignment = serde_json::from_str(&json).unwrap();
                assert_eq!(value, back);
            }
            // Sanity check on lowercase representation.
            let json = serde_json::to_string(&RuleAlignment::Centered).unwrap();
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

        // ================================================================
        // Phase 2 parity: renderable::tree::graphics::horizontal_rule_svg
        // must match HorizontalRule::render_browser_svg byte-for-byte.
        // ================================================================

        #[test]
        fn parity_default_dashes() {
            let rule = HorizontalRule::new();
            let expected = rule.render_browser_svg();
            let actual = renderable::tree::graphics::horizontal_rule_svg(
                None, None, None, None, "0", "0",
            );
            assert_eq!(
                actual, expected,
                "default dashed SVG must match between renderable and biscuit-terminal"
            );
        }

        #[test]
        fn parity_waves_thick_blue() {
            let rule = HorizontalRule::new()
                .style(RuleStyle::Waves)
                .weight(RuleWeight::Thick)
                .color("blue");
            let expected = rule.render_browser_svg();
            let actual = renderable::tree::graphics::horizontal_rule_svg(
                Some("waves"),
                Some("thick"),
                None,
                Some("blue"),
                "0",
                "0",
            );
            assert_eq!(
                actual, expected,
                "waves+thick+blue SVG must match between renderable and biscuit-terminal"
            );
        }

        #[test]
        fn parity_dots_thin_custom_width() {
            let rule = HorizontalRule::new()
                .style(RuleStyle::Dots)
                .weight(RuleWeight::Thin)
                .width("50%");
            let expected = rule.render_browser_svg();
            let actual = renderable::tree::graphics::horizontal_rule_svg(
                Some("dots"),
                Some("thin"),
                Some("50%"),
                None,
                "0",
                "0",
            );
            assert_eq!(
                actual, expected,
                "dots+thin+50% SVG must match between renderable and biscuit-terminal"
            );
        }

        #[test]
        fn parity_line_star() {
            let rule = HorizontalRule::new().style(RuleStyle::LineStar);
            let expected = rule.render_browser_svg();
            let actual = renderable::tree::graphics::horizontal_rule_svg(
                Some("line-star"),
                None,
                None,
                None,
                "0",
                "0",
            );
            assert_eq!(
                actual, expected,
                "line-star SVG must match between renderable and biscuit-terminal"
            );
        }

        #[test]
        fn parity_line_circle() {
            let rule = HorizontalRule::new().style(RuleStyle::LineCircle);
            let expected = rule.render_browser_svg();
            let actual = renderable::tree::graphics::horizontal_rule_svg(
                Some("line-circle"),
                None,
                None,
                None,
                "0",
                "0",
            );
            assert_eq!(
                actual, expected,
                "line-circle SVG must match between renderable and biscuit-terminal"
            );
        }

        #[test]
        fn parity_inset_line() {
            let rule = HorizontalRule::new().style(RuleStyle::InsetLine);
            let expected = rule.render_browser_svg();
            let actual = renderable::tree::graphics::horizontal_rule_svg(
                Some("inset-line"),
                None,
                None,
                None,
                "0",
                "0",
            );
            assert_eq!(
                actual, expected,
                "inset-line SVG must match between renderable and biscuit-terminal"
            );
        }

        #[test]
        fn parity_curtain_rod() {
            let rule = HorizontalRule::new().style(RuleStyle::CurtainRod);
            let expected = rule.render_browser_svg();
            let actual = renderable::tree::graphics::horizontal_rule_svg(
                Some("curtain-rod"),
                None,
                None,
                None,
                "0",
                "0",
            );
            assert_eq!(
                actual, expected,
                "curtain-rod SVG must match between renderable and biscuit-terminal"
            );
        }

        #[test]
        fn parity_with_custom_margins() {
            let rule = HorizontalRule::new()
                .with_layout(crate::utils::layout::Layout {
                    margin: crate::utils::layout::Edges {
                        top: renderable::layout::TargetValue::universal(
                            renderable::layout::Length::ch(2),
                        ),
                        right: renderable::layout::TargetValue::universal(
                            renderable::layout::Length::Zero,
                        ),
                        bottom: renderable::layout::TargetValue::universal(
                            renderable::layout::Length::ch(1),
                        ),
                        left: renderable::layout::TargetValue::universal(
                            renderable::layout::Length::Zero,
                        ),
                    },
                    ..crate::utils::layout::Layout::default()
                });
            let expected = rule.render_browser_svg();
            let actual = renderable::tree::graphics::horizontal_rule_svg(
                None, None, None, None, "2ch", "1ch",
            );
            assert_eq!(
                actual, expected,
                "SVG with custom margins must match between renderable and biscuit-terminal"
            );
        }

        #[test]
        fn render_text_tier_matches_render_when_image_unavailable() {
            let _guard = ScopedLcAll::force_utf8();
            let hr = HorizontalRule::new()
                .style(RuleStyle::Waves)
                .alignment(RuleAlignment::Centered)
                .width("50%");
            let term = text_terminal_with_width(80);

            // text_terminal disables the image tier, so render() falls through
            // to the text tier automatically.
            let via_render = hr.render(&term);
            let via_text_tier = hr.render_text_tier(&term);

            assert_eq!(
                via_render, via_text_tier,
                "render_text_tier must match render() when image tier is unavailable"
            );
        }
    }
}
