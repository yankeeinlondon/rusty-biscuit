//! CSS value types for the stylesheet subsystem.
//!
//! This module provides the strongly-typed value system used by [`CssStyle`].
//! Values are categorized by [`CssValueKind`] and include dimensional sizing,
//! colors, integers, and raw CSS expressions.

use std::fmt::{self, Display};

use crate::stylesheet::error::StylesheetError;

/// High-level category for a CSS property's accepted value type.
///
/// `CssValueKind` is the coarse classification used by the type system to
/// pair properties with values. Each [`CssValue`] variant maps to exactly one
/// kind (see [`CssValue::kind`]), and each known [`CssProp`] reports its
/// expected kind via [`CssProp::expected_kind`].
///
/// The kinds are intentionally narrow — `Sizing` and `SizingMulti` are
/// distinguished because shorthand properties (`margin`, `padding`,
/// `border-radius`) accept 1–4 values, while their individual sub-properties
/// (`margin-top`, etc.) accept exactly one.
///
/// ## Examples
///
/// ```rust
/// use renderable::stylesheet::{CssProp, CssValueKind};
///
/// assert_eq!(CssProp::Width.expected_kind(), Some(CssValueKind::Sizing));
/// assert_eq!(CssProp::Margin.expected_kind(), Some(CssValueKind::SizingMulti));
/// assert_eq!(CssProp::ZIndex.expected_kind(), Some(CssValueKind::Integer));
/// assert_eq!(CssProp::Other("--foo".into()).expected_kind(), None);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CssValueKind {
    /// A single sizing token, such as `12px`, `50%`, `auto`, or `calc(...)`.
    Sizing,
    /// A 1–4 element list of sizing tokens used by shorthand properties.
    SizingMulti,
    /// A color token: hex, `rgb()`/`rgba()`, named color, or keyword.
    Color,
    /// A signed [`i32`] value (used by `z-index`, `order`, `flex-grow`, etc.).
    Integer,
    /// Any freeform string — the fallback for custom properties and other
    /// values that don't fit a structured category.
    Raw,
}

/// Formats the kind using the kebab-case names used in error messages
/// (`sizing`, `sizing-multi`, `color`, `integer`, `raw`).
impl Display for CssValueKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sizing => write!(f, "sizing"),
            Self::SizingMulti => write!(f, "sizing-multi"),
            Self::Color => write!(f, "color"),
            Self::Integer => write!(f, "integer"),
            Self::Raw => write!(f, "raw"),
        }
    }
}

/// Unit suffix for a dimensional [`CssSizing::Dimension`] value.
///
/// Mirrors the standard CSS length and percentage units. `Percent` is grouped
/// with the lengths for convenience since it shares the same numeric grammar.
///
/// ## Categories
///
/// - **Absolute lengths**: `Px`, `Pt`, `Pc`, `Cm`, `Mm`, `In`
/// - **Font-relative**: `Em`, `Rem`, `Ex`, `Ch`
/// - **Viewport-relative**: `Vw`, `Vh`, `Vmin`, `Vmax`
/// - **Percent**: `Percent` (`%`)
///
/// ## Examples
///
/// ```rust
/// use renderable::stylesheet::{CssSizing, CssUnit};
///
/// let size = CssSizing::dimension(2.0, CssUnit::Rem);
/// assert_eq!(size.to_string(), "2rem");
/// assert_eq!(CssUnit::Percent.as_str(), "%");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CssUnit {
    /// Pixels (`px`) — absolute length.
    Px,
    /// Root em (`rem`) — relative to root font size.
    Rem,
    /// Em (`em`) — relative to the current element font size.
    Em,
    /// Viewport width percent (`vw`).
    Vw,
    /// Viewport height percent (`vh`).
    Vh,
    /// Smaller of [`Vw`](Self::Vw) and [`Vh`](Self::Vh) (`vmin`).
    Vmin,
    /// Larger of [`Vw`](Self::Vw) and [`Vh`](Self::Vh) (`vmax`).
    Vmax,
    /// Character advance unit (`ch`).
    Ch,
    /// X-height (`ex`).
    Ex,
    /// Points (`pt`) — 1/72in.
    Pt,
    /// Picas (`pc`) — 12pt.
    Pc,
    /// Centimeters (`cm`).
    Cm,
    /// Millimeters (`mm`).
    Mm,
    /// Inches (`in`).
    In,
    /// Percent (`%`).
    Percent,
}

impl CssUnit {
    /// Returns the unit suffix as it appears in CSS output (e.g. `"px"`,
    /// `"rem"`, `"%"`).
    ///
    /// Returns a `&'static str` since every variant maps to a fixed literal.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Px => "px",
            Self::Rem => "rem",
            Self::Em => "em",
            Self::Vw => "vw",
            Self::Vh => "vh",
            Self::Vmin => "vmin",
            Self::Vmax => "vmax",
            Self::Ch => "ch",
            Self::Ex => "ex",
            Self::Pt => "pt",
            Self::Pc => "pc",
            Self::Cm => "cm",
            Self::Mm => "mm",
            Self::In => "in",
            Self::Percent => "%",
        }
    }
}

/// A single sizing value — the [`CssValueKind::Sizing`] payload.
///
/// Covers everything that may legally appear on the right of a `width:`,
/// `font-size:`, or other single-sizing declaration: dimensional values,
/// keyword sizings, and validated CSS function expressions.
///
/// ## Variants
///
/// - **`Dimension { value, unit }`** — a finite number paired with a [`CssUnit`].
///   Constructed via [`CssSizing::dimension`] or the unit-specific shortcuts
///   ([`px`](Self::px), [`rem`](Self::rem), [`em`](Self::em),
///   [`percent`](Self::percent)).
/// - **`Zero`** — the unit-less `0` literal. Any dimensional value whose
///   numeric component is exactly `0.0` is normalized to this variant by
///   [`CssSizing::dimension`].
/// - **Sizing keywords** — `Auto`, `MinContent`, `MaxContent`, `FitContent`.
/// - **Global keywords** — `Inherit`, `Initial`, `Unset`.
/// - **`Expression(String)`** — a CSS function call such as `calc(100% - 16px)`
///   or `var(--gap)`. Constructed via [`CssSizing::expression`] which validates
///   the function-call shape.
///
/// ## Examples
///
/// ```rust
/// use renderable::stylesheet::{CssSizing, CssUnit};
///
/// assert_eq!(CssSizing::px(0.0), CssSizing::Zero);
/// assert_eq!(CssSizing::rem(1.5).to_string(), "1.5rem");
/// assert_eq!(CssSizing::Auto.to_string(), "auto");
///
/// let calc = CssSizing::expression("calc(100% - 16px)").unwrap();
/// assert_eq!(calc.to_string(), "calc(100% - 16px)");
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum CssSizing {
    /// A finite number paired with a unit (e.g. `12px`, `1.5rem`, `50%`).
    Dimension {
        /// Numeric coefficient. Must be finite; `0.0` is normalized to
        /// [`CssSizing::Zero`] by the constructors.
        value: f32,
        /// Unit suffix attached to `value` during rendering.
        unit: CssUnit,
    },
    /// The `0` literal — emitted unit-less.
    Zero,
    /// The `auto` keyword.
    Auto,
    /// The `min-content` keyword.
    MinContent,
    /// The `max-content` keyword.
    MaxContent,
    /// The `fit-content` keyword.
    FitContent,
    /// The `inherit` keyword.
    Inherit,
    /// The `initial` keyword.
    Initial,
    /// The `unset` keyword.
    Unset,
    /// A CSS function call such as `calc(...)`, `clamp(...)`, or `var(...)`.
    ///
    /// The string is stored verbatim; only the outer shape is validated on
    /// construction (must end with `)`, must contain a function name + `(`).
    Expression(String),
}

impl CssSizing {
    /// Creates a dimensional sizing.
    ///
    /// A `value` of exactly `0.0` is normalized to [`CssSizing::Zero`]
    /// regardless of `unit`, so `dimension(0.0, CssUnit::Px)` and
    /// `dimension(0.0, CssUnit::Rem)` are equal. This matches the CSS
    /// convention that `0` is unitless.
    ///
    /// ## Notes
    ///
    /// This constructor does not validate that `value` is finite; passing
    /// `NaN` or infinity produces a `Dimension` whose [`Display`] output is
    /// implementation-defined. Prefer the [`TryFrom<&str>`] path when input
    /// originates from user-supplied strings.
    pub fn dimension(value: f32, unit: CssUnit) -> Self {
        if value == 0.0 {
            return Self::Zero;
        }

        Self::Dimension { value, unit }
    }

    /// Shortcut for [`CssSizing::dimension`] with [`CssUnit::Px`].
    pub fn px(value: f32) -> Self {
        Self::dimension(value, CssUnit::Px)
    }

    /// Shortcut for [`CssSizing::dimension`] with [`CssUnit::Rem`].
    pub fn rem(value: f32) -> Self {
        Self::dimension(value, CssUnit::Rem)
    }

    /// Shortcut for [`CssSizing::dimension`] with [`CssUnit::Em`].
    pub fn em(value: f32) -> Self {
        Self::dimension(value, CssUnit::Em)
    }

    /// Shortcut for [`CssSizing::dimension`] with [`CssUnit::Percent`].
    pub fn percent(value: f32) -> Self {
        Self::dimension(value, CssUnit::Percent)
    }

    /// Creates a validated [`CssSizing::Expression`] from a CSS function-call
    /// string such as `"calc(100% - 16px)"`.
    ///
    /// ## Errors
    ///
    /// Returns [`StylesheetError::InvalidSizing`] when the trimmed input:
    /// - is empty,
    /// - contains a `;`,
    /// - or fails `is_css_function_call` (no `(`, doesn't end with `)`,
    ///   or the head before `(` contains non-identifier characters).
    pub fn expression(value: impl Into<String>) -> Result<Self, StylesheetError> {
        let value = value.into();
        let trimmed = value.trim();

        if trimmed.is_empty() || trimmed.contains(';') {
            return Err(StylesheetError::InvalidSizing { value });
        }

        if !is_css_function_call(trimmed) {
            return Err(StylesheetError::InvalidSizing { value });
        }

        Ok(Self::Expression(trimmed.to_string()))
    }
}

/// Formats a sizing value as it would appear in CSS output.
///
/// `Dimension` uses `format_css_number` to suppress trailing zeros and
/// avoid scientific notation. Keyword variants emit their canonical lowercase
/// form. `Expression` emits its stored string verbatim.
impl Display for CssSizing {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Dimension { value, unit } => {
                write!(f, "{}{}", format_css_number(*value), unit.as_str())
            }
            Self::Zero => write!(f, "0"),
            Self::Auto => write!(f, "auto"),
            Self::MinContent => write!(f, "min-content"),
            Self::MaxContent => write!(f, "max-content"),
            Self::FitContent => write!(f, "fit-content"),
            Self::Inherit => write!(f, "inherit"),
            Self::Initial => write!(f, "initial"),
            Self::Unset => write!(f, "unset"),
            Self::Expression(expr) => write!(f, "{expr}"),
        }
    }
}

/// Parses a [`CssSizing`] from its CSS string representation.
///
/// Recognition order (first match wins):
///
/// 1. The empty string → error.
/// 2. Sizing/global keywords: `auto`, `min-content`, `max-content`,
///    `fit-content`, `inherit`, `initial`, `unset` (case-insensitive).
/// 3. A unit-less number that equals zero → [`CssSizing::Zero`].
/// 4. A function-call shape (per `is_css_function_call`) →
///    [`CssSizing::Expression`].
/// 5. A `<number><unit>` token where `<unit>` is one of the [`CssUnit`]
///    suffixes (longest suffix wins to disambiguate `vmin`/`vmax` from
///    `vw`/`vh`). The numeric part must parse to a finite [`f32`].
///
/// ## Errors
///
/// Returns [`StylesheetError::InvalidSizing`] when no rule matches, the
/// numeric portion fails to parse, or the parsed number is non-finite.
impl TryFrom<&str> for CssSizing {
    type Error = StylesheetError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(StylesheetError::InvalidSizing {
                value: value.to_string(),
            });
        }

        let lower = trimmed.to_ascii_lowercase();
        let keyword = match lower.as_str() {
            "auto" => Some(Self::Auto),
            "min-content" => Some(Self::MinContent),
            "max-content" => Some(Self::MaxContent),
            "fit-content" => Some(Self::FitContent),
            "inherit" => Some(Self::Inherit),
            "initial" => Some(Self::Initial),
            "unset" => Some(Self::Unset),
            _ => None,
        };

        if let Some(found) = keyword {
            return Ok(found);
        }

        if is_zero_number(trimmed) {
            return Ok(Self::Zero);
        }

        if is_css_function_call(trimmed) {
            return Ok(Self::Expression(trimmed.to_string()));
        }

        for (suffix, unit) in [
            ("vmin", CssUnit::Vmin),
            ("vmax", CssUnit::Vmax),
            ("rem", CssUnit::Rem),
            ("px", CssUnit::Px),
            ("em", CssUnit::Em),
            ("vw", CssUnit::Vw),
            ("vh", CssUnit::Vh),
            ("ch", CssUnit::Ch),
            ("ex", CssUnit::Ex),
            ("pt", CssUnit::Pt),
            ("pc", CssUnit::Pc),
            ("cm", CssUnit::Cm),
            ("mm", CssUnit::Mm),
            ("in", CssUnit::In),
            ("%", CssUnit::Percent),
        ] {
            if let Some(number) = lower.strip_suffix(suffix) {
                let parsed =
                    number
                        .trim()
                        .parse::<f32>()
                        .map_err(|_| StylesheetError::InvalidSizing {
                            value: value.to_string(),
                        })?;

                if !parsed.is_finite() {
                    return Err(StylesheetError::InvalidSizing {
                        value: value.to_string(),
                    });
                }

                return Ok(Self::dimension(parsed, unit));
            }
        }

        Err(StylesheetError::InvalidSizing {
            value: value.to_string(),
        })
    }
}

/// Owned-string convenience wrapper over [`TryFrom<&str> for CssSizing`].
impl TryFrom<String> for CssSizing {
    type Error = StylesheetError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::try_from(value.as_str())
    }
}

/// A 1–4 element list of [`CssSizing`] tokens — the
/// [`CssValueKind::SizingMulti`] payload.
///
/// Models the CSS shorthand grammar for `margin`, `padding`, and
/// `border-radius`, where:
///
/// - 1 value applies to all sides.
/// - 2 values apply as `vertical horizontal`.
/// - 3 values apply as `top horizontal bottom`.
/// - 4 values apply as `top right bottom left`.
///
/// The enum is fixed-arity rather than a `Vec` so that the legal cardinalities
/// are encoded in the type. Constructors that may yield other counts (e.g.
/// [`CssSizingMulti::from_vec`] or [`TryFrom<&str>`](#impl-TryFrom<%26str>-for-CssSizingMulti))
/// fall back to [`StylesheetError::InvalidSizingMulti`].
///
/// ## Examples
///
/// ```rust
/// use renderable::stylesheet::{CssSizing, CssSizingMulti};
///
/// // From a tuple
/// let pad = CssSizingMulti::from((CssSizing::px(8.0), CssSizing::px(16.0)));
/// assert_eq!(pad.to_string(), "8px 16px");
///
/// // From parsed CSS
/// let radius = CssSizingMulti::try_from("4px 8px 12px 16px").unwrap();
/// assert!(matches!(radius, CssSizingMulti::Four(..)));
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum CssSizingMulti {
    /// Single value — applies to all four sides.
    One(CssSizing),
    /// Two values — `vertical horizontal`.
    Two(CssSizing, CssSizing),
    /// Three values — `top horizontal bottom`.
    Three(CssSizing, CssSizing, CssSizing),
    /// Four values — `top right bottom left`.
    Four(CssSizing, CssSizing, CssSizing, CssSizing),
}

impl CssSizingMulti {
    /// Builds a multi-sizing from a vector of 1..=4 [`CssSizing`] values.
    ///
    /// ## Errors
    ///
    /// Returns [`StylesheetError::InvalidSizingMulti`] when `values.len()` is
    /// zero or greater than four. The error's `value` field contains the
    /// space-joined string form of the rejected inputs for diagnostics.
    pub fn from_vec(values: Vec<CssSizing>) -> Result<Self, StylesheetError> {
        match values.len() {
            1 => Ok(Self::One(values[0].clone())),
            2 => Ok(Self::Two(values[0].clone(), values[1].clone())),
            3 => Ok(Self::Three(
                values[0].clone(),
                values[1].clone(),
                values[2].clone(),
            )),
            4 => Ok(Self::Four(
                values[0].clone(),
                values[1].clone(),
                values[2].clone(),
                values[3].clone(),
            )),
            _ => Err(StylesheetError::InvalidSizingMulti {
                value: values
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(" "),
            }),
        }
    }

    /// Returns the underlying tokens as a `Vec<&CssSizing>` in declaration
    /// order. Used internally by [`Display`] and the JSON helpers.
    fn as_slice(&self) -> Vec<&CssSizing> {
        match self {
            Self::One(a) => vec![a],
            Self::Two(a, b) => vec![a, b],
            Self::Three(a, b, c) => vec![a, b, c],
            Self::Four(a, b, c, d) => vec![a, b, c, d],
        }
    }
}

/// Joins the underlying tokens with single spaces in declaration order.
impl Display for CssSizingMulti {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let rendered = self
            .as_slice()
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(" ");
        write!(f, "{rendered}")
    }
}

/// Lifts a single [`CssSizing`] into [`CssSizingMulti::One`].
impl From<CssSizing> for CssSizingMulti {
    fn from(value: CssSizing) -> Self {
        Self::One(value)
    }
}

/// Builds a [`CssSizingMulti::Two`] from a `(vertical, horizontal)` tuple.
impl From<(CssSizing, CssSizing)> for CssSizingMulti {
    fn from(value: (CssSizing, CssSizing)) -> Self {
        Self::Two(value.0, value.1)
    }
}

/// Builds a [`CssSizingMulti::Three`] from a `(top, horizontal, bottom)` tuple.
impl From<(CssSizing, CssSizing, CssSizing)> for CssSizingMulti {
    fn from(value: (CssSizing, CssSizing, CssSizing)) -> Self {
        Self::Three(value.0, value.1, value.2)
    }
}

/// Builds a [`CssSizingMulti::Four`] from a `(top, right, bottom, left)` tuple.
impl From<(CssSizing, CssSizing, CssSizing, CssSizing)> for CssSizingMulti {
    fn from(value: (CssSizing, CssSizing, CssSizing, CssSizing)) -> Self {
        Self::Four(value.0, value.1, value.2, value.3)
    }
}

/// Parses a multi-sizing from a CSS string like `"8px 16px"`.
///
/// Splits on top-level whitespace using `split_whitespace_outside_parens`
/// so that parenthesized expressions (e.g. `calc(...)`, `var(...)`) are kept
/// intact. Each token is then parsed via [`CssSizing::try_from`].
///
/// ## Errors
///
/// Returns [`StylesheetError::InvalidSizingMulti`] when:
/// - The token count is outside `1..=4`.
/// - Any individual token fails to parse as a [`CssSizing`].
impl TryFrom<&str> for CssSizingMulti {
    type Error = StylesheetError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let tokens = split_whitespace_outside_parens(value);
        if !(1..=4).contains(&tokens.len()) {
            return Err(StylesheetError::InvalidSizingMulti {
                value: value.to_string(),
            });
        }

        let parsed = tokens
            .iter()
            .map(|token| CssSizing::try_from(token.as_str()))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| StylesheetError::InvalidSizingMulti {
                value: value.to_string(),
            })?;

        Self::from_vec(parsed)
    }
}

/// Owned-string convenience wrapper over [`TryFrom<&str> for CssSizingMulti`].
impl TryFrom<String> for CssSizingMulti {
    type Error = StylesheetError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::try_from(value.as_str())
    }
}

/// A validated color token — the [`CssValueKind::Color`] payload.
///
/// Covers the four common CSS color forms plus the standard color keywords
/// and arbitrary function-like expressions (`var(...)`, `color-mix(...)`,
/// etc.). All forms are validated at construction time so a [`CssColor`]
/// always renders to legal CSS via [`Display`].
///
/// ## Variants
///
/// - **`Hex(String)`** — `#rgb`, `#rgba`, `#rrggbb`, or `#rrggbbaa`. The
///   stored string includes the leading `#` and is lowercased.
/// - **`Rgb(r, g, b)`** — three [`u8`] channels, rendered as
///   `rgb(r, g, b)` with explicit comma separators.
/// - **`Rgba(r, g, b, a)`** — three [`u8`] channels and an alpha in
///   `0.0..=1.0`. Rendered as `rgba(r, g, b, a)`.
/// - **`Named(String)`** — a CSS named color (any alphabetic-plus-hyphen
///   identifier; no list lookup is performed beyond syntax).
/// - **Keywords** — `Transparent`, `CurrentColor`, `Inherit`, `Initial`, `Unset`.
/// - **`Expression(String)`** — a function-call shape like `var(--brand)`.
#[derive(Debug, Clone, PartialEq)]
pub enum CssColor {
    /// Hex color (for example `#fff` or `#ff00aa`).
    Hex(String),
    /// RGB function.
    Rgb(u8, u8, u8),
    /// RGBA function.
    Rgba(u8, u8, u8, f32),
    /// Named color (for example `red`).
    Named(String),
    /// `transparent` keyword.
    Transparent,
    /// `currentColor` keyword.
    CurrentColor,
    /// `inherit` keyword.
    Inherit,
    /// `initial` keyword.
    Initial,
    /// `unset` keyword.
    Unset,
    /// Function-like expression such as `var(--brand)`.
    Expression(String),
}

impl CssColor {
    /// Creates a validated hex color.
    ///
    /// The input must start with `#` and have a hex body of 3, 4, 6, or 8
    /// digits. The result is lowercased and stored with the leading `#`.
    ///
    /// ## Errors
    ///
    /// Returns [`StylesheetError::InvalidColor`] when the input does not pass
    /// `is_hex_color`.
    pub fn hex(value: impl Into<String>) -> Result<Self, StylesheetError> {
        let value = value.into();
        let trimmed = value.trim();
        if !is_hex_color(trimmed) {
            return Err(StylesheetError::InvalidColor { value });
        }
        Ok(Self::Hex(trimmed.to_ascii_lowercase()))
    }

    /// Creates an opaque RGB color from three [`u8`] channels.
    ///
    /// Infallible: every `(u8, u8, u8)` triple is a valid RGB color.
    pub fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self::Rgb(r, g, b)
    }

    /// Creates an RGBA color from three [`u8`] channels and an alpha.
    ///
    /// ## Errors
    ///
    /// Returns [`StylesheetError::InvalidColor`] when `a` is not finite or
    /// lies outside the `0.0..=1.0` interval.
    pub fn rgba(r: u8, g: u8, b: u8, a: f32) -> Result<Self, StylesheetError> {
        if !(0.0..=1.0).contains(&a) || !a.is_finite() {
            return Err(StylesheetError::InvalidColor {
                value: format!("rgba({r}, {g}, {b}, {a})"),
            });
        }
        Ok(Self::Rgba(r, g, b, a))
    }

    /// Creates a named color (e.g. `red`, `rebeccapurple`).
    ///
    /// Validation is purely syntactic — the input is accepted if it is
    /// non-empty and composed only of ASCII letters and `-`. The name is
    /// lowercased for storage. This module deliberately does not maintain
    /// the CSS named-color list, so any syntactically valid identifier is
    /// accepted; the browser is responsible for ultimate interpretation.
    ///
    /// ## Errors
    ///
    /// Returns [`StylesheetError::InvalidColor`] when the input is empty or
    /// contains characters other than ASCII letters and `-`.
    pub fn named(value: impl Into<String>) -> Result<Self, StylesheetError> {
        let value = value.into();
        let trimmed = value.trim();
        if !is_named_color(trimmed) {
            return Err(StylesheetError::InvalidColor { value });
        }
        Ok(Self::Named(trimmed.to_ascii_lowercase()))
    }

    /// Creates a color from a CSS function-call expression such as
    /// `var(--brand)` or `color-mix(in srgb, red, blue)`.
    ///
    /// ## Errors
    ///
    /// Returns [`StylesheetError::InvalidColor`] when the trimmed input is
    /// empty, contains `;`, or does not pass `is_css_function_call`.
    pub fn expression(value: impl Into<String>) -> Result<Self, StylesheetError> {
        let value = value.into();
        let trimmed = value.trim();

        if trimmed.is_empty() || trimmed.contains(';') || !is_css_function_call(trimmed) {
            return Err(StylesheetError::InvalidColor { value });
        }

        Ok(Self::Expression(trimmed.to_string()))
    }
}

/// Formats a color in canonical CSS notation.
///
/// `Rgb` and `Rgba` use comma-separated channels with a single space after
/// each comma. Alpha is rendered via `format_css_number` to strip trailing
/// zeros. `Hex`, `Named`, and `Expression` emit their stored string as-is.
impl Display for CssColor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Hex(value) => write!(f, "{value}"),
            Self::Rgb(r, g, b) => write!(f, "rgb({r}, {g}, {b})"),
            Self::Rgba(r, g, b, a) => write!(f, "rgba({r}, {g}, {b}, {})", format_css_number(*a)),
            Self::Named(name) => write!(f, "{name}"),
            Self::Transparent => write!(f, "transparent"),
            Self::CurrentColor => write!(f, "currentColor"),
            Self::Inherit => write!(f, "inherit"),
            Self::Initial => write!(f, "initial"),
            Self::Unset => write!(f, "unset"),
            Self::Expression(expr) => write!(f, "{expr}"),
        }
    }
}

/// Parses a [`CssColor`] from its CSS string representation.
///
/// Recognition order (first match wins):
///
/// 1. Empty input → error.
/// 2. Keywords: `transparent`, `currentColor`, `inherit`, `initial`, `unset`
///    (case-insensitive).
/// 3. Hex literal beginning with `#` (per `is_hex_color`).
/// 4. `rgb(...)` / `rgba(...)` (per `parse_rgb_like`). Both comma- and
///    space-separated channel lists are accepted for `rgb`; `rgba` also
///    accepts a `/` between channels and alpha.
/// 5. Other function-call shapes → [`CssColor::Expression`].
/// 6. A bare identifier of ASCII letters and `-` → [`CssColor::Named`].
///
/// ## Errors
///
/// Returns [`StylesheetError::InvalidColor`] when no rule matches or when a
/// recognized form contains malformed channels (e.g. an alpha outside `[0,1]`
/// or a non-numeric channel).
impl TryFrom<&str> for CssColor {
    type Error = StylesheetError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(StylesheetError::InvalidColor {
                value: value.to_string(),
            });
        }

        let lower = trimmed.to_ascii_lowercase();
        let keyword = match lower.as_str() {
            "transparent" => Some(Self::Transparent),
            "currentcolor" => Some(Self::CurrentColor),
            "inherit" => Some(Self::Inherit),
            "initial" => Some(Self::Initial),
            "unset" => Some(Self::Unset),
            _ => None,
        };

        if let Some(found) = keyword {
            return Ok(found);
        }

        if is_hex_color(trimmed) {
            return Ok(Self::Hex(trimmed.to_ascii_lowercase()));
        }

        if let Some(rgb) = parse_rgb_like(trimmed)? {
            return Ok(rgb);
        }

        if is_css_function_call(trimmed) {
            return Ok(Self::Expression(trimmed.to_string()));
        }

        if is_named_color(trimmed) {
            return Ok(Self::Named(trimmed.to_ascii_lowercase()));
        }

        Err(StylesheetError::InvalidColor {
            value: value.to_string(),
        })
    }
}

/// Owned-string convenience wrapper over [`TryFrom<&str> for CssColor`].
impl TryFrom<String> for CssColor {
    type Error = StylesheetError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::try_from(value.as_str())
    }
}

/// Validated raw CSS value — the [`CssValueKind::Raw`] payload.
///
/// Used for declarations whose value type is not structurally classified
/// — primarily custom properties paired via [`CssCustomProp`]. The wrapper
/// trims whitespace and rejects values containing `;` so a `CssRaw` is always
/// safe to embed in a `property: value;` declaration without breaking
/// downstream parsers.
///
/// ## Examples
///
/// ```rust
/// use renderable::stylesheet::CssRaw;
///
/// let raw = CssRaw::new("0 0 0 1px rgba(0, 0, 0, 0.1)").unwrap();
/// assert_eq!(raw.as_str(), "0 0 0 1px rgba(0, 0, 0, 0.1)");
///
/// assert!(CssRaw::new("foo; drop table").is_err());
/// assert!(CssRaw::new("   ").is_err());
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CssRaw(String);

impl CssRaw {
    /// Creates a validated raw CSS value.
    ///
    /// The input is trimmed before storage.
    ///
    /// ## Errors
    ///
    /// Returns [`StylesheetError::InvalidRawValue`] when the trimmed input is
    /// empty or contains `;`.
    pub fn new(value: impl Into<String>) -> Result<Self, StylesheetError> {
        let value = value.into();
        let trimmed = value.trim();
        if trimmed.is_empty() || trimmed.contains(';') {
            return Err(StylesheetError::InvalidRawValue { value });
        }
        Ok(Self(trimmed.to_string()))
    }

    /// Returns the validated, trimmed value as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Renders the stored value verbatim.
impl Display for CssRaw {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Validating conversion from a string slice; see [`CssRaw::new`].
impl TryFrom<&str> for CssRaw {
    type Error = StylesheetError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

/// Validating conversion from an owned [`String`]; see [`CssRaw::new`].
impl TryFrom<String> for CssRaw {
    type Error = StylesheetError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

/// A type-erased CSS value — the runtime representation stored inside a
/// [`CssStyle`].
///
/// Each variant carries a structurally-validated payload and maps 1:1 to a
/// [`CssValueKind`]:
///
/// | Variant         | Kind                              |
/// |-----------------|-----------------------------------|
/// | `Sizing`        | [`CssValueKind::Sizing`]          |
/// | `SizingMulti`   | [`CssValueKind::SizingMulti`]     |
/// | `Color`         | [`CssValueKind::Color`]           |
/// | `Integer`       | [`CssValueKind::Integer`]         |
/// | `Raw`           | [`CssValueKind::Raw`]             |
///
/// Use the [`IntoCssValue`] trait to construct a `CssValue` from one of the
/// strongly-typed value types; this is what [`CssStyle::add`] uses
/// internally.
#[derive(Debug, Clone, PartialEq)]
pub enum CssValue {
    /// A single sizing token.
    Sizing(CssSizing),
    /// A 1–4 token sizing shorthand.
    SizingMulti(CssSizingMulti),
    /// A color token.
    Color(CssColor),
    /// A signed integer.
    Integer(i32),
    /// A freeform validated string (typically for custom properties).
    Raw(CssRaw),
}

impl CssValue {
    /// Returns the coarse [`CssValueKind`] for this value.
    ///
    /// Used by `ensure_valid_property_value_pair` and the debug check in
    /// [`CssStyle::add`] to verify that a value matches its property's
    /// [`CssProp::expected_kind`].
    pub fn kind(&self) -> CssValueKind {
        match self {
            Self::Sizing(_) => CssValueKind::Sizing,
            Self::SizingMulti(_) => CssValueKind::SizingMulti,
            Self::Color(_) => CssValueKind::Color,
            Self::Integer(_) => CssValueKind::Integer,
            Self::Raw(_) => CssValueKind::Raw,
        }
    }
}

/// Delegates to the inner payload's [`Display`] impl. The output is the
/// canonical CSS form (e.g. `"12px"`, `"8px 16px"`, `"rgb(1, 2, 3)"`, `"42"`).
impl Display for CssValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sizing(value) => write!(f, "{value}"),
            Self::SizingMulti(value) => write!(f, "{value}"),
            Self::Color(value) => write!(f, "{value}"),
            Self::Integer(value) => write!(f, "{value}"),
            Self::Raw(value) => write!(f, "{value}"),
        }
    }
}

/// Lifts a strongly-typed value (e.g. [`CssSizing`], [`CssColor`], [`i32`])
/// into the runtime [`CssValue`] enum.
///
/// Implemented by every type that may appear on the right-hand side of a
/// [`CssStyle::add`] call, plus a passthrough impl for [`CssValue`] itself.
///
/// This trait exists to make [`CssStyle::add`] generic without requiring
/// callers to manually wrap values:
///
/// ```ignore
/// // Both compile because `CssSizing: IntoCssValue` and `i32: IntoCssValue`.
/// sheet.add(CssSizingProp::Width, CssSizing::px(100.0));
/// sheet.add(CssIntegerProp::ZIndex, 10);
/// ```
pub trait IntoCssValue {
    /// Wraps `self` in the appropriate [`CssValue`] variant.
    fn into_css_value(self) -> CssValue;
}

/// Wraps as [`CssValue::Sizing`].
impl IntoCssValue for CssSizing {
    fn into_css_value(self) -> CssValue {
        CssValue::Sizing(self)
    }
}

/// Wraps as [`CssValue::SizingMulti`].
impl IntoCssValue for CssSizingMulti {
    fn into_css_value(self) -> CssValue {
        CssValue::SizingMulti(self)
    }
}

/// Wraps as [`CssValue::Color`].
impl IntoCssValue for CssColor {
    fn into_css_value(self) -> CssValue {
        CssValue::Color(self)
    }
}

/// Wraps as [`CssValue::Integer`].
impl IntoCssValue for i32 {
    fn into_css_value(self) -> CssValue {
        CssValue::Integer(self)
    }
}

/// Wraps as [`CssValue::Raw`].
impl IntoCssValue for CssRaw {
    fn into_css_value(self) -> CssValue {
        CssValue::Raw(self)
    }
}

/// Identity impl — used when a [`CssValue`] is already in hand (e.g. dynamic
/// parsing paths).
impl IntoCssValue for CssValue {
    fn into_css_value(self) -> CssValue {
        self
    }
}

/// Formats an [`f32`] for CSS output.
///
/// - Exact zero → `"0"`.
/// - Integral values → no decimal point (e.g. `42.0_f32` → `"42"`).
/// - Fractional values → trailing zeros and any orphan `.` are stripped
///   (e.g. `1.50` → `"1.5"`).
fn format_css_number(value: f32) -> String {
    if value == 0.0 {
        return "0".to_string();
    }

    if (value.fract() - 0.0).abs() < f32::EPSILON {
        return (value as i64).to_string();
    }

    let mut text = format!("{value}");
    if text.contains('.') {
        while text.ends_with('0') {
            text.pop();
        }
        if text.ends_with('.') {
            text.pop();
        }
    }

    text
}

/// Returns `true` when `value` parses as a number equal to `0.0` (e.g.
/// `"0"`, `"0.0"`, `"-0"`). Used to short-circuit unit-less zero detection in
/// [`TryFrom<&str> for CssSizing`].
fn is_zero_number(value: &str) -> bool {
    match value.parse::<f32>() {
        Ok(number) => number == 0.0,
        Err(_) => false,
    }
}

/// Returns `true` when `value` has the shape of a CSS function call:
/// `name(...)` where `name` is non-empty and composed of ASCII alphanumerics
/// or `-`, and the value ends with `)`. The argument body is not validated.
fn is_css_function_call(value: &str) -> bool {
    let Some(open_idx) = value.find('(') else {
        return false;
    };

    value.ends_with(')')
        && open_idx > 0
        && value
            .chars()
            .take(open_idx)
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-')
}

/// Returns `true` when `value` is a `#`-prefixed hex color with a body of
/// 3, 4, 6, or 8 hex digits.
fn is_hex_color(value: &str) -> bool {
    let Some(hex) = value.strip_prefix('#') else {
        return false;
    };

    matches!(hex.len(), 3 | 4 | 6 | 8) && hex.chars().all(|ch| ch.is_ascii_hexdigit())
}

/// Returns `true` when `value` is a non-empty string of ASCII letters and
/// `-`. Names are not checked against the CSS named-color list.
fn is_named_color(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphabetic() || ch == '-')
}

/// Parses `rgb(...)` and `rgba(...)` color forms.
///
/// Returns `Ok(Some(_))` when `value` matches one of those forms, `Ok(None)`
/// when it does not (so the caller can fall through to other recognizers).
/// Accepts both comma- and whitespace-separated channels for `rgb`, plus
/// `/`-separated alpha syntax for `rgba`. Channels must parse as [`u8`];
/// alpha must be a finite [`f32`] in `0.0..=1.0`.
fn parse_rgb_like(value: &str) -> Result<Option<CssColor>, StylesheetError> {
    let lower = value.to_ascii_lowercase();

    if let Some(inner) = lower.strip_prefix("rgb(").and_then(|v| v.strip_suffix(')')) {
        let channels =
            parse_channel_list(inner, 3).ok_or_else(|| StylesheetError::InvalidColor {
                value: value.to_string(),
            })?;

        return Ok(Some(CssColor::Rgb(channels[0], channels[1], channels[2])));
    }

    if let Some(inner) = lower
        .strip_prefix("rgba(")
        .and_then(|v| v.strip_suffix(')'))
    {
        let normalized = inner.replace('/', ",");
        let parts = normalized
            .split(',')
            .map(str::trim)
            .filter(|p| !p.is_empty())
            .collect::<Vec<_>>();

        if parts.len() != 4 {
            return Err(StylesheetError::InvalidColor {
                value: value.to_string(),
            });
        }

        let r = parse_u8_channel(parts[0], value)?;
        let g = parse_u8_channel(parts[1], value)?;
        let b = parse_u8_channel(parts[2], value)?;
        let alpha = parts[3]
            .parse::<f32>()
            .map_err(|_| StylesheetError::InvalidColor {
                value: value.to_string(),
            })?;

        if !(0.0..=1.0).contains(&alpha) || !alpha.is_finite() {
            return Err(StylesheetError::InvalidColor {
                value: value.to_string(),
            });
        }

        return Ok(Some(CssColor::Rgba(r, g, b, alpha)));
    }

    Ok(None)
}

/// Parses the inner argument list of an `rgb(...)` call into `expected`
/// [`u8`] channels. Accepts either comma- or whitespace-separated tokens.
/// Returns `None` if the token count is wrong or any channel fails to parse.
fn parse_channel_list(inner: &str, expected: usize) -> Option<Vec<u8>> {
    let parts = if inner.contains(',') {
        inner
            .split(',')
            .map(str::trim)
            .filter(|p| !p.is_empty())
            .collect::<Vec<_>>()
    } else {
        inner
            .split_whitespace()
            .filter(|p| !p.is_empty())
            .collect::<Vec<_>>()
    };

    if parts.len() != expected {
        return None;
    }

    parts
        .iter()
        .map(|part| part.parse::<u8>().ok())
        .collect::<Option<Vec<_>>>()
}

/// Parses a single RGB channel as [`u8`], attributing any failure to the
/// `original` color string in the resulting [`StylesheetError::InvalidColor`].
fn parse_u8_channel(value: &str, original: &str) -> Result<u8, StylesheetError> {
    value
        .parse::<u8>()
        .map_err(|_| StylesheetError::InvalidColor {
            value: original.to_string(),
        })
}

/// Splits `input` on whitespace at parenthesis depth zero.
///
/// Keeps parenthesized expressions intact, so `"8px calc(100% - 4px)"`
/// produces two tokens: `"8px"` and `"calc(100% - 4px)"`. Empty tokens are
/// skipped. Used by [`TryFrom<&str> for CssSizingMulti`] to split shorthand
/// values without breaking embedded function calls.
fn split_whitespace_outside_parens(input: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut depth = 0usize;

    for ch in input.chars() {
        match ch {
            '(' => {
                depth += 1;
                current.push(ch);
            }
            ')' => {
                depth = depth.saturating_sub(1);
                current.push(ch);
            }
            _ if ch.is_whitespace() && depth == 0 => {
                let trimmed = current.trim();
                if !trimmed.is_empty() {
                    tokens.push(trimmed.to_string());
                }
                current.clear();
            }
            _ => current.push(ch),
        }
    }

    let trimmed = current.trim();
    if !trimmed.is_empty() {
        tokens.push(trimmed.to_string());
    }

    tokens
}

/// Removes a single layer of `{ ... }` wrapping if present, otherwise returns
/// the trimmed input unchanged. Lets [`TryFrom<&str> for CssStyle`] accept
/// both bare declaration lists and brace-wrapped blocks.
pub(crate) fn strip_surrounding_braces(input: &str) -> &str {
    let trimmed = input.trim();
    if trimmed.starts_with('{') && trimmed.ends_with('}') && trimmed.len() >= 2 {
        &trimmed[1..trimmed.len() - 1]
    } else {
        trimmed
    }
}
