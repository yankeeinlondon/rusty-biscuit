//! Shared, target-neutral text-styling leaf primitives.
//!
//! [`TextEmphasis`] is the single source of truth for text weight and
//! decoration intent — bold, dim, italic, underline, strikethrough, and
//! blink. It is reused by `biscuit-terminal`'s `Prose` styling and is
//! intended to back the render-tree `Style` primitive, so terminal SGR
//! emission and browser CSS / semantic-HTML emission are written once here
//! and never drift between targets.
//!
//! Capability-aware degradation (e.g. downgrading a double underline on a
//! terminal that lacks it) is deliberately *not* done here — that decision
//! belongs to the target emitter that knows the terminal's capabilities.
//!
//! Alongside the leaf primitives, this module also defines [`Style`] — the
//! appearance primitive a component declares and the tree renderers apply,
//! the sibling of [`Layout`](crate::layout::Layout). `Style` re-homes the
//! foreground/background color, text emphasis, border, and fill concepts
//! that were previously expressed as bespoke component fields.

use serde::{Deserialize, Serialize};

use crate::color::{Color, ColorMode};
use crate::layout::{Length, TargetValue};

/// The underline variant a styled region requests.
///
/// This records intent only. Capability-aware degradation is the target
/// emitter's responsibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnderlineStyle {
    /// A straight single underline.
    Straight,
    /// A double underline.
    Double,
    /// A curly (wavy) underline.
    Curly,
    /// A dotted underline.
    Dotted,
    /// A dashed underline.
    Dashed,
}

impl UnderlineStyle {
    /// The opening SGR escape for this underline variant.
    ///
    /// This is the *un-degraded* code; a terminal emitter may substitute a
    /// simpler variant when the terminal lacks support for it.
    pub fn sgr_open(self) -> &'static str {
        match self {
            Self::Straight => "\x1b[4m",
            Self::Double => "\x1b[4:2m",
            Self::Curly => "\x1b[4:3m",
            Self::Dotted => "\x1b[4:4m",
            Self::Dashed => "\x1b[4:5m",
        }
    }

    /// The CSS `text-decoration` declaration(s) for this underline variant.
    pub fn css_declaration(self) -> &'static str {
        match self {
            Self::Straight => "text-decoration: underline",
            Self::Double => "text-decoration: underline; text-decoration-style: double",
            Self::Curly => "text-decoration: underline; text-decoration-style: wavy",
            Self::Dotted => "text-decoration: underline; text-decoration-style: dotted",
            Self::Dashed => "text-decoration: underline; text-decoration-style: dashed",
        }
    }
}

/// An independent SGR attribute group within [`TextEmphasis`].
///
/// Each layer maps to one SGR attribute group, so a terminal emitter can
/// restore a parent span's value per layer instead of issuing a blanket
/// reset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EmphasisLayer {
    /// Bold / dim — the SGR font-weight group.
    Weight,
    /// Italic.
    Italic,
    /// Underline (any variant).
    Underline,
    /// Strikethrough.
    Strikethrough,
    /// Blink.
    Blink,
}

impl EmphasisLayer {
    /// The SGR code that clears this layer back to the terminal default.
    pub fn sgr_reset(self) -> &'static str {
        match self {
            Self::Weight => "\x1b[22m",
            Self::Italic => "\x1b[23m",
            Self::Underline => "\x1b[24m",
            Self::Strikethrough => "\x1b[29m",
            Self::Blink => "\x1b[25m",
        }
    }
}

/// Target-neutral text-emphasis leaf: weight and decoration intent.
///
/// Shared across render targets — `biscuit-terminal`'s `Prose` embeds it,
/// and the render-tree `Style` primitive is intended to as well. Terminal
/// SGR emission ([`sgr_ops`](Self::sgr_ops)) and browser semantic-HTML / CSS
/// emission ([`html_wrappers`](Self::html_wrappers)) are defined here so the
/// two targets never drift apart.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct TextEmphasis {
    /// Bold weight.
    pub bold: bool,
    /// Dim weight.
    pub dim: bool,
    /// Italic.
    pub italic: bool,
    /// Strikethrough.
    pub strikethrough: bool,
    /// Blink.
    pub blink: bool,
    /// Underline variant, if any.
    pub underline: Option<UnderlineStyle>,
}

impl TextEmphasis {
    /// `true` when no emphasis attribute is set.
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }

    /// The non-underline SGR open codes for this emphasis, in nesting order.
    ///
    /// Underline is intentionally excluded: its opening escape depends on a
    /// capability-aware degradation decision that only the terminal emitter
    /// can make. Use the [`underline`](Self::underline) field together with
    /// [`UnderlineStyle::sgr_open`] for the underline layer.
    pub fn sgr_ops(&self) -> Vec<(EmphasisLayer, &'static str)> {
        let mut ops = Vec::new();
        if self.bold {
            ops.push((EmphasisLayer::Weight, "\x1b[1m"));
        }
        if self.dim {
            ops.push((EmphasisLayer::Weight, "\x1b[2m"));
        }
        if self.italic {
            ops.push((EmphasisLayer::Italic, "\x1b[3m"));
        }
        if self.blink {
            ops.push((EmphasisLayer::Blink, "\x1b[5m"));
        }
        if self.strikethrough {
            ops.push((EmphasisLayer::Strikethrough, "\x1b[9m"));
        }
        ops
    }

    /// Returns this emphasis merged with an inherited `parent` emphasis.
    ///
    /// Each boolean attribute is `true` when set on either side, so a
    /// parent's bold or italic cascades onto a child. The `underline`
    /// variant falls back to the parent's when this side has none — a child
    /// that declares its own underline variant keeps it.
    pub fn inherited_from(&self, parent: &TextEmphasis) -> TextEmphasis {
        TextEmphasis {
            bold: self.bold || parent.bold,
            dim: self.dim || parent.dim,
            italic: self.italic || parent.italic,
            strikethrough: self.strikethrough || parent.strikethrough,
            blink: self.blink || parent.blink,
            underline: self.underline.or(parent.underline),
        }
    }

    /// The HTML wrapper pairs (`open`, `close`) for this emphasis, in nesting
    /// order.
    ///
    /// Semantic styles use semantic HTML (`<strong>`, `<em>`, `<s>`);
    /// presentational styles use `<span style="…">`.
    pub fn html_wrappers(&self) -> Vec<(String, &'static str)> {
        let mut wrappers: Vec<(String, &'static str)> = Vec::new();
        if self.bold {
            wrappers.push(("<strong>".to_string(), "</strong>"));
        }
        if self.italic {
            wrappers.push(("<em>".to_string(), "</em>"));
        }
        if self.strikethrough {
            wrappers.push(("<s>".to_string(), "</s>"));
        }
        if let Some(underline) = self.underline {
            wrappers.push((
                format!("<span style=\"{}\">", underline.css_declaration()),
                "</span>",
            ));
        }
        if self.dim {
            wrappers.push(("<span style=\"opacity: 0.6\">".to_string(), "</span>"));
        }
        if self.blink {
            wrappers.push((
                "<span style=\"text-decoration: blink\">".to_string(),
                "</span>",
            ));
        }
        wrappers
    }
}

/// A style value that adapts to the terminal / page background color mode.
///
/// Color-bearing `Style` fields wrap their value in `PerMode` so a component
/// can author one universal value, or distinct light- and dark-mode values,
/// without the component branching on the resolved [`ColorMode`].
///
/// Composes with [`TargetValue`] for the common
/// `TargetValue<PerMode<Color>>` shape: `TargetValue` selects per render
/// target, `PerMode` then adapts to light/dark within a target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PerMode<T> {
    /// One value used for every background mode.
    Universal(T),
    /// Distinct values for light and dark backgrounds.
    Adaptive {
        /// The value used on a light background.
        light: T,
        /// The value used on a dark background.
        dark: T,
    },
}

impl<T> PerMode<T> {
    /// A value used for every background mode.
    pub fn universal(value: T) -> PerMode<T> {
        PerMode::Universal(value)
    }

    /// Distinct values for light and dark backgrounds.
    pub fn adaptive(light: T, dark: T) -> PerMode<T> {
        PerMode::Adaptive { light, dark }
    }

    /// Resolve the value for `mode`.
    ///
    /// `Universal` always resolves to its single value. `Adaptive` resolves
    /// to `light` for [`ColorMode::Light`] and to `dark` for both
    /// [`ColorMode::Dark`] and [`ColorMode::Unknown`] — `Unknown` defaults to
    /// the dark treatment, matching `ColorMode`'s own default.
    pub fn resolve(&self, mode: ColorMode) -> &T {
        match self {
            PerMode::Universal(value) => value,
            PerMode::Adaptive { light, dark } => match mode {
                ColorMode::Light => light,
                ColorMode::Dark | ColorMode::Unknown => dark,
            },
        }
    }
}

/// The visual weight (thickness) of a [`Border`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BorderWeight {
    /// A thin border — single-weight box-drawing on the terminal.
    #[default]
    Thin,
    /// A medium-weight border.
    Medium,
    /// A thick border — heavy box-drawing on the terminal.
    Thick,
}

/// The line treatment of a [`Border`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BorderLineStyle {
    /// An unbroken line.
    #[default]
    Solid,
    /// A dashed line.
    Dashed,
    /// A dotted line.
    Dotted,
    /// A double line.
    Double,
}

/// Which sides of a component a [`Border`] is drawn on.
///
/// `All` and `None` are the aggregate choices; `Sides` addresses each edge
/// individually so an author can enable, for example, only the `left` side
/// without introducing a separate `LeftBorder` type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BorderSides {
    /// All four sides.
    #[default]
    All,
    /// No sides — the border is effectively disabled.
    None,
    /// Per-side selection.
    Sides {
        /// Draw the top edge.
        top: bool,
        /// Draw the right edge.
        right: bool,
        /// Draw the bottom edge.
        bottom: bool,
        /// Draw the left edge.
        left: bool,
    },
}

/// A component's border appearance.
///
/// One aggregate entity: uniform `weight` / `line_style` / `color` defaults
/// plus side-specific addressing through [`BorderSides`]. A border does not
/// inherit through the render tree.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Border {
    /// Border color, if any. `None` uses the target's default line color.
    pub color: Option<TargetValue<PerMode<Color>>>,
    /// Visual weight.
    pub weight: BorderWeight,
    /// Line treatment.
    pub line_style: BorderLineStyle,
    /// Which sides are drawn.
    pub sides: BorderSides,
    /// Optional corner radius.
    pub radius: Option<TargetValue<Length>>,
}

/// How intensely a [`Fill`] paints its band relative to the background.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FillIntensity {
    /// No paint — the band is left transparent.
    Transparent,
    /// A faint tint.
    #[default]
    Subtle,
    /// A strong, clearly visible band.
    Pronounced,
}

/// How wide a band a [`Fill`] paints across the available width.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FillBand {
    /// Paint the full available width.
    #[default]
    Full,
    /// Paint the content band, leaving the component's padding unpainted.
    Padded,
    /// Paint an indented band, inset from both edges.
    Indented,
}

/// How a component paints its band of available width.
///
/// `Fill` models painted-band *behavior* — how and whether the width is
/// painted — and is intentionally separate from [`Style::background`], which
/// is plain adaptive color. A fill does not inherit through the render tree.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Fill {
    /// Fill color, if any. `None` derives a tint from the background.
    pub color: Option<TargetValue<PerMode<Color>>>,
    /// Paint intensity.
    pub intensity: FillIntensity,
    /// The band painted across the available width.
    pub band: FillBand,
    /// Optional inset applied to the painted band.
    pub inset: Option<TargetValue<Length>>,
}

/// How a component paints itself and its named sub-parts.
///
/// `Style` is the appearance sibling of [`Layout`](crate::layout::Layout):
/// `Layout` decides *where the box sits*, `Style` decides *what the box
/// looks like*. Components declare a `Style`; the tree renderers apply it.
/// A component never hand-writes ANSI or CSS.
///
/// `Style` may attach to block nodes and inline `Span` nodes. Only the text
/// appearance fields — `color` and `emphasis` — inherit through render-tree
/// traversal; `background`, `border`, and `fill` are box-painting properties
/// that stay explicit on the node that paints them.
///
/// ## Serialized shape
///
/// `Style` rides on `NodeAttrs` and serializes with the render tree. A node
/// with an accent foreground that adapts to light/dark, a universal subtle
/// background, bold-italic emphasis, and a thin left border:
///
/// ```json
/// {
///   "color": {
///     "universal": {
///       "adaptive": {
///         "light": { "Tailwind": "Blue700" },
///         "dark": { "Tailwind": "Blue300" }
///       }
///     }
///   },
///   "background": {
///     "universal": { "universal": { "Tailwind": "Slate100" } }
///   },
///   "emphasis": {
///     "bold": true,
///     "dim": false,
///     "italic": true,
///     "strikethrough": false,
///     "blink": false,
///     "underline": null
///   },
///   "border": {
///     "color": null,
///     "weight": "thin",
///     "line_style": "solid",
///     "sides": { "sides": { "top": false, "right": false,
///                           "bottom": false, "left": true } },
///     "radius": null
///   },
///   "fill": null
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Style {
    /// Foreground (text) color. Inherits through the render tree.
    pub color: Option<TargetValue<PerMode<Color>>>,
    /// Background color of the component's box. Does not inherit.
    pub background: Option<TargetValue<PerMode<Color>>>,
    /// Text emphasis — the shared [`TextEmphasis`] leaf. Inherits through the
    /// render tree.
    ///
    /// For *inline* content prefer the `Strong` / `Emphasis` / `Delete` node
    /// kinds; these flags exist for block-component slot styling and because
    /// the leaf is shared.
    pub emphasis: TextEmphasis,
    /// Border appearance, if any. Does not inherit.
    pub border: Option<Border>,
    /// Fill — how the component paints its band of available width. Does not
    /// inherit.
    pub fill: Option<Fill>,
}

impl Style {
    /// `true` when no appearance attribute is set.
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }

    /// Computes the effective `Style` for a child node given its `parent`.
    ///
    /// Only the text-appearance fields inherit, matching Spec B D6: a `None`
    /// [`color`](Self::color) falls back to the parent's color, and
    /// [`emphasis`](Self::emphasis) is the union of the parent's and this
    /// style's emphasis via [`TextEmphasis::inherited_from`]. The
    /// box-painting fields — [`background`](Self::background),
    /// [`border`](Self::border), and [`fill`](Self::fill) — never inherit
    /// and are taken from `self` unchanged.
    #[must_use]
    pub fn inherited_from(&self, parent: &Style) -> Style {
        Style {
            color: self.color.clone().or_else(|| parent.color.clone()),
            background: self.background.clone(),
            emphasis: self.emphasis.inherited_from(&parent.emphasis),
            border: self.border.clone(),
            fill: self.fill.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::Tailwind;

    #[test]
    fn default_is_empty() {
        assert!(TextEmphasis::default().is_empty());
    }

    #[test]
    fn bold_is_not_empty() {
        let e = TextEmphasis {
            bold: true,
            ..Default::default()
        };
        assert!(!e.is_empty());
    }

    #[test]
    fn sgr_ops_excludes_underline() {
        let e = TextEmphasis {
            bold: true,
            underline: Some(UnderlineStyle::Double),
            ..Default::default()
        };
        let ops = e.sgr_ops();
        assert_eq!(ops, vec![(EmphasisLayer::Weight, "\x1b[1m")]);
    }

    #[test]
    fn html_wrappers_use_semantic_tags() {
        let e = TextEmphasis {
            bold: true,
            italic: true,
            ..Default::default()
        };
        let wrappers = e.html_wrappers();
        assert_eq!(wrappers[0].0, "<strong>");
        assert_eq!(wrappers[1].0, "<em>");
    }

    #[test]
    fn underline_css_for_curly_is_wavy() {
        assert_eq!(
            UnderlineStyle::Curly.css_declaration(),
            "text-decoration: underline; text-decoration-style: wavy"
        );
    }

    #[test]
    fn underline_sgr_open_matches_variant() {
        assert_eq!(UnderlineStyle::Straight.sgr_open(), "\x1b[4m");
        assert_eq!(UnderlineStyle::Double.sgr_open(), "\x1b[4:2m");
    }

    #[test]
    fn text_emphasis_serde_roundtrip() {
        let e = TextEmphasis {
            bold: true,
            italic: true,
            underline: Some(UnderlineStyle::Curly),
            ..Default::default()
        };
        let json = serde_json::to_string(&e).unwrap();
        let back: TextEmphasis = serde_json::from_str(&json).unwrap();
        assert_eq!(e, back);
    }

    #[test]
    fn per_mode_universal_resolves_for_every_mode() {
        let value = PerMode::universal(Color::Tailwind(Tailwind::Blue500));
        for mode in [ColorMode::Light, ColorMode::Dark, ColorMode::Unknown] {
            assert_eq!(value.resolve(mode), &Color::Tailwind(Tailwind::Blue500));
        }
    }

    #[test]
    fn per_mode_adaptive_resolves_per_mode() {
        let value = PerMode::adaptive(
            Color::Tailwind(Tailwind::Blue700),
            Color::Tailwind(Tailwind::Blue300),
        );
        assert_eq!(
            value.resolve(ColorMode::Light),
            &Color::Tailwind(Tailwind::Blue700)
        );
        assert_eq!(
            value.resolve(ColorMode::Dark),
            &Color::Tailwind(Tailwind::Blue300)
        );
        // Unknown defaults to the dark treatment.
        assert_eq!(
            value.resolve(ColorMode::Unknown),
            &Color::Tailwind(Tailwind::Blue300)
        );
    }

    #[test]
    fn per_mode_serde_roundtrip() {
        let value = PerMode::adaptive(
            Color::Tailwind(Tailwind::Slate100),
            Color::Tailwind(Tailwind::Slate900),
        );
        let json = serde_json::to_string(&value).unwrap();
        let back: PerMode<Color> = serde_json::from_str(&json).unwrap();
        assert_eq!(value, back);
    }

    #[test]
    fn border_serde_roundtrip() {
        let border = Border {
            color: Some(TargetValue::universal(PerMode::universal(
                Color::Tailwind(Tailwind::Indigo500),
            ))),
            weight: BorderWeight::Thick,
            line_style: BorderLineStyle::Double,
            sides: BorderSides::Sides {
                top: false,
                right: false,
                bottom: false,
                left: true,
            },
            radius: Some(TargetValue::universal(Length::ch(1))),
        };
        let json = serde_json::to_string(&border).unwrap();
        let back: Border = serde_json::from_str(&json).unwrap();
        assert_eq!(border, back);
    }

    #[test]
    fn fill_serde_roundtrip() {
        let fill = Fill {
            color: Some(TargetValue::universal(PerMode::adaptive(
                Color::Tailwind(Tailwind::Slate100),
                Color::Tailwind(Tailwind::Slate800),
            ))),
            intensity: FillIntensity::Pronounced,
            band: FillBand::Padded,
            inset: Some(TargetValue::universal(Length::ch(2))),
        };
        let json = serde_json::to_string(&fill).unwrap();
        let back: Fill = serde_json::from_str(&json).unwrap();
        assert_eq!(fill, back);
    }

    #[test]
    fn style_default_is_empty() {
        assert!(Style::default().is_empty());
    }

    #[test]
    fn style_serde_roundtrip() {
        let style = Style {
            color: Some(TargetValue::universal(PerMode::adaptive(
                Color::Tailwind(Tailwind::Blue700),
                Color::Tailwind(Tailwind::Blue300),
            ))),
            background: Some(TargetValue::universal(PerMode::universal(
                Color::Tailwind(Tailwind::Slate100),
            ))),
            emphasis: TextEmphasis {
                bold: true,
                italic: true,
                ..Default::default()
            },
            border: Some(Border {
                sides: BorderSides::Sides {
                    top: false,
                    right: false,
                    bottom: false,
                    left: true,
                },
                ..Border::default()
            }),
            fill: None,
        };
        let json = serde_json::to_string(&style).unwrap();
        let back: Style = serde_json::from_str(&json).unwrap();
        assert_eq!(style, back);
    }

    #[test]
    fn emphasis_inheritance_unions_flags() {
        let parent = TextEmphasis {
            bold: true,
            underline: Some(UnderlineStyle::Straight),
            ..Default::default()
        };
        let child = TextEmphasis {
            italic: true,
            ..Default::default()
        };
        let merged = child.inherited_from(&parent);
        // Parent's bold cascades, child keeps its italic.
        assert!(merged.bold && merged.italic);
        // Child has no underline, so the parent's variant is inherited.
        assert_eq!(merged.underline, Some(UnderlineStyle::Straight));
    }

    #[test]
    fn emphasis_inheritance_child_underline_wins() {
        let parent = TextEmphasis {
            underline: Some(UnderlineStyle::Straight),
            ..Default::default()
        };
        let child = TextEmphasis {
            underline: Some(UnderlineStyle::Curly),
            ..Default::default()
        };
        assert_eq!(
            child.inherited_from(&parent).underline,
            Some(UnderlineStyle::Curly)
        );
    }

    #[test]
    fn style_inheritance_color_falls_back_to_parent() {
        let parent = Style {
            color: Some(TargetValue::universal(PerMode::universal(
                Color::Tailwind(Tailwind::Blue700),
            ))),
            ..Style::default()
        };
        // A child with no color of its own inherits the parent's.
        let child = Style::default();
        assert_eq!(child.inherited_from(&parent).color, parent.color);

        // A child with its own color keeps it.
        let own = Style {
            color: Some(TargetValue::universal(PerMode::universal(
                Color::Tailwind(Tailwind::Red500),
            ))),
            ..Style::default()
        };
        assert_eq!(own.inherited_from(&parent).color, own.color);
    }

    #[test]
    fn style_inheritance_box_painting_does_not_inherit() {
        let parent = Style {
            background: Some(TargetValue::universal(PerMode::universal(
                Color::Tailwind(Tailwind::Slate100),
            ))),
            border: Some(Border::default()),
            fill: Some(Fill::default()),
            ..Style::default()
        };
        // The child declares no box-painting properties; none are inherited.
        let merged = Style::default().inherited_from(&parent);
        assert!(merged.background.is_none());
        assert!(merged.border.is_none());
        assert!(merged.fill.is_none());
    }

    #[test]
    fn style_documented_json_shape_roundtrips() {
        let json = r#"{
            "color": {
              "universal": {
                "adaptive": {
                  "light": { "Tailwind": "Blue700" },
                  "dark": { "Tailwind": "Blue300" }
                }
              }
            },
            "background": {
              "universal": { "universal": { "Tailwind": "Slate100" } }
            },
            "emphasis": {
              "bold": true,
              "dim": false,
              "italic": true,
              "strikethrough": false,
              "blink": false,
              "underline": null
            },
            "border": {
              "color": null,
              "weight": "thin",
              "line_style": "solid",
              "sides": { "sides": { "top": false, "right": false,
                                    "bottom": false, "left": true } },
              "radius": null
            },
            "fill": null
        }"#;
        let style: Style = serde_json::from_str(json).unwrap();
        let reencoded = serde_json::to_string(&style).unwrap();
        let back: Style = serde_json::from_str(&reencoded).unwrap();
        assert_eq!(style, back);
        assert!(style.emphasis.bold && style.emphasis.italic);
    }
}
