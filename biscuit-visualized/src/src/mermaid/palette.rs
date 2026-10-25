//! Diagram palettes: the colors a Mermaid diagram is styled with.
//!
//! A [`Palette`] is backend-neutral: a background, a foreground, and six
//! accent colors, plus the surface, border, and line colors derived from
//! them. The renderer turns it into one compiled Merman theme per diagram
//! family (see `docs/merman.md`, "Themes").
//!
//! Palettes come from two places:
//!
//! - **Fixed palettes**, one per [`MermaidTheme`], starting from Mermaid.js
//!   12.2.0's theme of the same name and then put through the accent and
//!   contrast passes below. They are used for output that is not for a
//!   terminal and whenever terminal colors are unknown.
//! - **Terminal palettes**, from the terminal's detected background,
//!   foreground, and ANSI colors ([`TerminalColors`]): ANSI 1–6 become the
//!   accents; a slot the terminal did not report gets an accent computed from
//!   the background instead.
//!
//! Every palette is finished by the same passes, so whatever its source:
//!
//! - text meets 4.5:1 against the background and every fill a design puts
//!   text on (surfaces and accent tints), whatever its size (D45);
//! - accents and borders meet 3:1 against the background.

use super::color::{GRAPHIC_CONTRAST, Rgb, TEXT_CONTRAST, most_readable, readable_tint, with_contrast};
use super::config::MermaidTheme;

/// How strongly an accent tints the fill behind node text, at most.
const TINT_STRENGTH: f64 = 0.22;

/// How strongly an accent tints a chart fill (pie slices), at most: strong
/// enough to tell neighbors apart, weaker where the text would not read.
const CHART_TINT_STRENGTH: f64 = 0.6;

/// The colors a Mermaid diagram is styled with.
///
/// Construct one with [`Palette::fixed`], [`Palette::new`], or
/// [`Palette::for_terminal`]; every constructor applies the contrast passes,
/// so a palette's text color is always readable on its fills.
///
/// ## Examples
///
/// ```rust
/// use biscuit_visualized::mermaid::{MermaidDiagram, MermaidTheme, Palette, Rgb};
///
/// let tokyo = Palette::new(
///     Rgb::from_hex("#1a1b26").unwrap(),
///     Rgb::from_hex("#c0caf5").unwrap(),
///     ["#f7768e", "#9ece6a", "#e0af68", "#7aa2f7", "#bb9af7", "#7dcfff"]
///         .map(|hex| Rgb::from_hex(hex).unwrap()),
/// );
/// assert!(tokyo.text().contrast(tokyo.background()) >= 4.5);
///
/// let diagram = MermaidDiagram::new("flowchart LR\n    A --> B").with_palette(tokyo);
/// assert_eq!(Palette::fixed(MermaidTheme::Dark).background(), Rgb::from_hex("#333").unwrap());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Palette {
    background: Rgb,
    text: Rgb,
    surface: Rgb,
    surface_alt: Rgb,
    border: Rgb,
    line: Rgb,
    accents: [Rgb; 6],
    tints: [Rgb; 6],
}

/// The colors detected from a terminal, as `biscuit-terminal` reports them;
/// `None` for a color the terminal did not report.
///
/// [`TerminalColors::detect`] reads them (feature `terminal`); tests build one
/// by hand to pin a terminal.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TerminalColors {
    pub background: Option<Rgb>,
    pub foreground: Option<Rgb>,
    /// ANSI slots 0–15.
    pub ansi: [Option<Rgb>; 16],
}

/// Roles that set a fixed palette apart from one derived from a background
/// and foreground: Mermaid.js's own surface, border, and line colors.
struct Roles {
    surface: Rgb,
    border: Rgb,
    line: Rgb,
}

impl Palette {
    /// A palette from a background, a foreground, and six accents (in ANSI
    /// order: red, green, yellow, blue, magenta, cyan). Surfaces, borders, and
    /// lines are mixed from the foreground over the background.
    pub fn new(background: Rgb, foreground: Rgb, accents: [Rgb; 6]) -> Self {
        let roles = Roles {
            surface: foreground.over(background, 0.08),
            border: foreground.over(background, 0.35),
            line: foreground.over(background, 0.6),
        };
        Self::finish(background, foreground, roles, accents)
    }

    /// The fixed palette for `theme`, starting from Mermaid.js 12.2.0's theme
    /// of the same name: its background, text, node, border, and line colors,
    /// and accents taken from its gitGraph branch colors.
    pub fn fixed(theme: MermaidTheme) -> Self {
        let hex = |text: &str| Rgb::from_hex(text).unwrap_or(Rgb::BLACK);
        let hsl = |hue: f64, saturation: f64, lightness: f64| Rgb::from_hsl(hue, saturation / 100.0, lightness / 100.0);
        match theme {
            MermaidTheme::Dark => Self::finish(
                hex("#333"),
                hex("#ccc"),
                Roles { surface: hex("#1f2020"), border: hex("#ccc"), line: hex("#d3d3d3") },
                // git4, git7, git6, git2, git1, git3
                [
                    hsl(0.0, 83.3, 43.5),
                    hsl(106.2, 84.4, 35.1),
                    hsl(35.1, 98.7, 40.2),
                    hsl(194.4, 16.6, 49.6),
                    hsl(321.6, 65.6, 38.2),
                    hsl(23.1, 49.1, 40.8),
                ],
            ),
            MermaidTheme::Default => Self::finish(
                hex("#ffffff"),
                hex("#333"),
                Roles { surface: hex("#ececff"), border: hex("#9370db"), line: hex("#333333") },
                // git7, git2, git1, git0, git6, git4
                [
                    hsl(0.0, 100.0, 46.3),
                    hsl(80.0, 100.0, 46.3),
                    hsl(60.0, 100.0, 43.5),
                    hsl(240.0, 100.0, 46.3),
                    hsl(300.0, 100.0, 46.3),
                    hsl(180.0, 100.0, 46.3),
                ],
            ),
            MermaidTheme::Forest => Self::finish(
                hex("#ffffff"),
                hex("#000000"),
                Roles { surface: hex("#cde498"), border: hex("#13540c"), line: hex("#000000") },
                // git4, git0, git3, git7, git5, git6
                [
                    hsl(18.2, 58.5, 49.5),
                    hsl(78.2, 58.5, 49.5),
                    hsl(48.2, 58.5, 49.5),
                    hsl(198.2, 58.5, 49.5),
                    hsl(348.2, 58.5, 49.5),
                    hsl(138.2, 58.5, 49.5),
                ],
            ),
            MermaidTheme::Neutral => Self::finish(
                hex("#ffffff"),
                hex("#000000"),
                Roles { surface: hex("#eeeeee"), border: hex("#999999"), line: hex("#666666") },
                // git1 and git3, and grays between them, kept apart from the
                // border (git4's #999 would meet it after the contrast pass)
                ["#555555", "#777777", "#444444", "#666666", "#888888", "#333333"].map(hex),
            ),
        }
    }

    /// The palette for a terminal (R-P3–R-P5):
    ///
    /// - background, foreground, and ANSI colors known: accents are ANSI 1–6;
    /// - an ANSI slot unknown: that accent is computed from the background;
    /// - no foreground: text is black or white, whichever reads on the
    ///   background;
    /// - no background: the fixed palette for the color mode
    ///   ([`MermaidTheme::for_color_mode`] of `is_dark`).
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use biscuit_visualized::mermaid::{MermaidTheme, Palette, Rgb, TerminalColors};
    ///
    /// let unknown = TerminalColors::default();
    /// assert_eq!(Palette::for_terminal(&unknown, true), Palette::fixed(MermaidTheme::Dark));
    ///
    /// let background_only = TerminalColors {
    ///     background: Rgb::from_hex("#002b36"),
    ///     ..TerminalColors::default()
    /// };
    /// assert_eq!(Palette::for_terminal(&background_only, true).background(), Rgb::new(0, 0x2b, 0x36));
    /// ```
    pub fn for_terminal(colors: &TerminalColors, is_dark: bool) -> Self {
        let Some(background) = colors.background else {
            return Self::fixed(MermaidTheme::for_color_mode(is_dark));
        };
        let foreground = colors
            .foreground
            .unwrap_or_else(|| most_readable(background, &[Rgb::BLACK, Rgb::WHITE]));
        let derived = background_accents(background);
        let accents = std::array::from_fn(|slot| colors.ansi[slot + 1].unwrap_or(derived[slot]));
        Self::new(background, foreground, accents)
    }

    /// The contrast passes every constructor ends with.
    fn finish(background: Rgb, foreground: Rgb, roles: Roles, accents: [Rgb; 6]) -> Self {
        let text = readable_text(foreground, background);
        let accents = accents.map(|accent| with_contrast(accent, background, GRAPHIC_CONTRAST));
        let surface = readable_fill(roles.surface, background, text);
        let surface_alt = readable_tint(text, background, text, 0.14);
        Self {
            background,
            text,
            surface,
            surface_alt,
            border: with_contrast(roles.border, background, GRAPHIC_CONTRAST),
            line: with_contrast(roles.line, background, GRAPHIC_CONTRAST),
            tints: accents.map(|accent| readable_tint(accent, background, text, TINT_STRENGTH)),
            accents,
        }
    }

    /// The color painted behind the diagram for `Background::Theme`.
    pub fn background(&self) -> Rgb {
        self.background
    }

    /// The text color, readable (4.5:1) on the background and on every
    /// surface and tint of this palette.
    pub fn text(&self) -> Rgb {
        self.text
    }

    /// The fill of nodes that take no accent (actors, entities, classes
    /// before their accent).
    pub fn surface(&self) -> Rgb {
        self.surface
    }

    /// The fill of groups and secondary boxes (clusters, activations, loop
    /// labels).
    pub fn surface_alt(&self) -> Rgb {
        self.surface_alt
    }

    /// Node and lifeline outlines.
    pub fn border(&self) -> Rgb {
        self.border
    }

    /// Edges, messages, and markers.
    pub fn line(&self) -> Rgb {
        self.line
    }

    /// The six rotating accents, in ANSI order (red, green, yellow, blue,
    /// magenta, cyan), each at least 3:1 against the background.
    pub fn accents(&self) -> [Rgb; 6] {
        self.accents
    }

    /// Each accent mixed over the background as a node fill, at most 22%
    /// strong and weaker where the text would otherwise drop below 4.5:1.
    pub fn tints(&self) -> [Rgb; 6] {
        self.tints
    }

    /// Each accent mixed over the background as a chart fill (pie slices), at
    /// most 60% strong and weaker where the text would drop below 4.5:1.
    pub fn chart_tints(&self) -> [Rgb; 6] {
        self.accents
            .map(|accent| readable_tint(accent, self.background, self.text, CHART_TINT_STRENGTH))
    }

    /// The readable label color on `fill`: this palette's text or
    /// background, or black or white.
    pub fn text_on(&self, fill: Rgb) -> Rgb {
        let preferred = [self.text, self.background];
        let best = most_readable(fill, &preferred);
        if best.contrast(fill) >= TEXT_CONTRAST {
            best
        } else {
            most_readable(fill, &[Rgb::BLACK, Rgb::WHITE])
        }
    }

    /// Whether the background is dark.
    pub fn is_dark(&self) -> bool {
        self.background.is_dark()
    }
}

#[cfg(feature = "terminal")]
impl TerminalColors {
    /// The attached terminal's colors, or all unknown for output that is not
    /// for a terminal: stdout not a terminal, or `CI` set (R-P5, R-P7).
    /// Otherwise `biscuit-terminal`'s per-process detection (OSC 10/11 and the
    /// OSC 4 palette, each sent at most once and only where it already
    /// probes).
    pub fn detect() -> Self {
        use std::io::IsTerminal;

        if !std::io::stdout().is_terminal() || std::env::var_os("CI").is_some() {
            return Self::default();
        }
        let rgb = |value: biscuit_terminal::discovery::osc_queries::RgbValue| {
            Rgb::new(value.r, value.g, value.b)
        };
        let ansi = biscuit_terminal::discovery::osc_queries::ansi_palette();
        Self {
            background: biscuit_terminal::discovery::osc_queries::bg_color().map(rgb),
            foreground: biscuit_terminal::discovery::osc_queries::text_color().map(rgb),
            ansi: ansi.map(|slot| slot.map(rgb)),
        }
    }
}

/// `foreground` moved to 4.5:1 against `background` when it falls short,
/// and black or white when even that cannot reach it.
fn readable_text(foreground: Rgb, background: Rgb) -> Rgb {
    let adjusted = with_contrast(foreground, background, TEXT_CONTRAST);
    if adjusted.contrast(background) >= TEXT_CONTRAST {
        adjusted
    } else {
        most_readable(background, &[Rgb::BLACK, Rgb::WHITE])
    }
}

/// `fill` when `text` reads on it, else `fill` moved toward `background` until
/// it does.
fn readable_fill(fill: Rgb, background: Rgb, text: Rgb) -> Rgb {
    if text.contrast(fill) >= TEXT_CONTRAST {
        fill
    } else {
        readable_tint(fill, background, text, 1.0)
    }
}

/// Six accents in ANSI hue order, computed from the background alone (R-P4):
/// light on a dark background, darker on a light one, each at 3:1.
fn background_accents(background: Rgb) -> [Rgb; 6] {
    let (hue, saturation, _) = background.to_hsl();
    let lightness = if background.is_dark() { 0.68 } else { 0.38 };
    // A tinted background pulls the accents slightly toward its own hue.
    let shift = if saturation > 0.15 { (hue - 180.0) * 0.05 } else { 0.0 };
    [0.0, 120.0, 55.0, 220.0, 300.0, 185.0]
        .map(|base| Rgb::from_hsl(base + shift, 0.65, lightness))
        .map(|accent| with_contrast(accent, background, GRAPHIC_CONTRAST))
}
