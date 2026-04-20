//! Type-safe CSS stylesheet declarations.
//!
//! `Stylesheet` provides a strongly-typed builder for common CSS properties while
//! still allowing custom properties through [`CssProp::Other`] and
//! [`CssCustomProp`].

use std::borrow::Cow;
use std::fmt::{self, Display};

use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::Renderable;
use biscuit_terminal::terminal::Terminal;
use serde_json::{Map, Value};
use thiserror::Error;

/// Errors returned when parsing or validating CSS declarations.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum StylesheetError {
    /// A declaration did not contain a valid `property: value` pair.
    #[error("invalid declaration `{declaration}`; expected `property: value`")]
    InvalidDeclaration { declaration: String },

    /// A property name failed basic CSS identifier validation.
    #[error("invalid CSS property name `{name}`")]
    InvalidPropertyName { name: String },

    /// A value's type did not match the expected category for the property.
    #[error("property `{property}` expects `{expected}` values, but got `{actual}` from `{value}`")]
    PropertyValueTypeMismatch {
        /// Property name.
        property: String,
        /// Expected value category.
        expected: CssValueKind,
        /// Actual value category.
        actual: CssValueKind,
        /// Original value string.
        value: String,
    },

    /// A single sizing value was invalid.
    #[error("invalid CSS sizing value `{value}`")]
    InvalidSizing { value: String },

    /// A multi-sizing value was invalid.
    #[error("invalid CSS multi-sizing value `{value}`; expected 1 to 4 sizing tokens")]
    InvalidSizingMulti { value: String },

    /// A color value was invalid.
    #[error("invalid CSS color value `{value}`")]
    InvalidColor { value: String },

    /// An integer value was invalid.
    #[error("invalid integer value `{value}`")]
    InvalidInteger { value: String },

    /// A raw CSS value was invalid.
    #[error("invalid raw CSS value `{value}`")]
    InvalidRawValue { value: String },

    /// A custom property name was invalid.
    #[error("invalid custom CSS property `{name}`")]
    InvalidCustomProperty { name: String },
}

impl biscuit_terminal::errors::BlockError for StylesheetError {
    fn status_block(
        &self,
        _term: &biscuit_terminal::terminal::Terminal,
    ) -> biscuit_terminal::components::status_block::StatusBlock {
        use biscuit_terminal::components::status::StatusState;
        use biscuit_terminal::components::status_block::StatusBlock;
        use biscuit_terminal::errors::{ErrorHeader, StatusBlockExt};

        match self {
            Self::InvalidDeclaration { declaration } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("StylesheetError", "invalid declaration"))
                .body(format!(
                    "<dim>Declaration:</dim> <cyan>{declaration}</cyan>"
                ))
                .hint("Each declaration must be of the form <cyan>property: value;</cyan>."),

            Self::InvalidPropertyName { name } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new(
                    "StylesheetError",
                    "invalid property name",
                ))
                .body(format!("<dim>Property:</dim> <cyan>{name}</cyan>"))
                .hint(
                    "CSS property names must start with a letter (or `--` for custom properties) and contain only letters, digits, and hyphens.",
                ),

            Self::PropertyValueTypeMismatch {
                property,
                expected,
                actual,
                value,
            } => {
                let example = example_for_kind(*expected);
                StatusBlock::new(StatusState::Error)
                    .error_header(ErrorHeader::new(
                        "StylesheetError",
                        "property/value type mismatch",
                    ))
                    .body(format!(
                        "<dim>Property:</dim> <cyan>{property}</cyan>\n<dim>Expected:</dim> <b>{expected}</b>\n<dim>Actual:</dim> {actual}\n<dim>Value:</dim> <cyan>{value}</cyan>"
                    ))
                    .hint(format!(
                        "Use a <cyan>{expected}</cyan> value (e.g., <cyan>{example}</cyan>)."
                    ))
            }

            Self::InvalidSizing { value } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("StylesheetError", "invalid sizing value"))
                .body(format!("<dim>Value:</dim> <cyan>{value}</cyan>"))
                .hint(
                    "Accepted sizing tokens: <cyan>0</cyan>, <cyan>42px</cyan>, <cyan>1.5rem</cyan>, <cyan>50%</cyan>, <cyan>auto</cyan>, <cyan>min-content</cyan>, or a <cyan>calc(...)</cyan> expression.",
                ),

            Self::InvalidSizingMulti { value } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new(
                    "StylesheetError",
                    "invalid multi-sizing value",
                ))
                .body(format!("<dim>Value:</dim> <cyan>{value}</cyan>"))
                .hint(
                    "Use 1 to 4 sizing tokens separated by spaces, e.g. <cyan>8px 16px</cyan> or <cyan>4px 8px 12px 16px</cyan>.",
                ),

            Self::InvalidColor { value } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("StylesheetError", "invalid color value"))
                .body(format!("<dim>Value:</dim> <cyan>{value}</cyan>"))
                .hint(
                    "Accepted color tokens: <cyan>#rgb</cyan>, <cyan>#rrggbb</cyan>, <cyan>rgb(r,g,b)</cyan>, <cyan>rgba(r,g,b,a)</cyan>, a named color, or one of <cyan>transparent</cyan> / <cyan>currentColor</cyan>.",
                ),

            Self::InvalidInteger { value } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("StylesheetError", "invalid integer value"))
                .body(format!("<dim>Value:</dim> <cyan>{value}</cyan>"))
                .hint("Use a whole number, e.g. <cyan>0</cyan>, <cyan>1</cyan>, or <cyan>-3</cyan>."),

            Self::InvalidRawValue { value } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("StylesheetError", "invalid raw value"))
                .body(format!("<dim>Value:</dim> <cyan>{value}</cyan>"))
                .hint(
                    "Raw CSS values must not be empty and must not contain <cyan>;</cyan>.",
                ),

            Self::InvalidCustomProperty { name } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new(
                    "StylesheetError",
                    "invalid custom property",
                ))
                .body(format!("<dim>Property:</dim> <cyan>{name}</cyan>"))
                .hint(
                    "Custom properties must start with <cyan>--</cyan> followed by a non-empty identifier (letters, digits, hyphens, underscores).",
                ),
        }
    }
}

fn example_for_kind(kind: CssValueKind) -> &'static str {
    match kind {
        CssValueKind::Sizing => "12px",
        CssValueKind::SizingMulti => "8px 16px",
        CssValueKind::Color => "#336699",
        CssValueKind::Integer => "40",
        CssValueKind::Raw => "var(--token)",
    }
}

/// High-level category for a CSS property's accepted value type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CssValueKind {
    /// A single sizing token, such as `12px` or `auto`.
    Sizing,
    /// A multi-sizing token list of 1..=4 sizing values.
    SizingMulti,
    /// A color token.
    Color,
    /// Integer values.
    Integer,
    /// Freeform raw values.
    Raw,
}

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

/// Enumerates known CSS properties plus a custom fallback.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CssProp {
    /// `width`
    Width,
    /// `height`
    Height,
    /// `min-width`
    MinWidth,
    /// `min-height`
    MinHeight,
    /// `max-width`
    MaxWidth,
    /// `max-height`
    MaxHeight,
    /// `top`
    Top,
    /// `right`
    Right,
    /// `bottom`
    Bottom,
    /// `left`
    Left,
    /// `margin-top`
    TopMargin,
    /// `margin-right`
    RightMargin,
    /// `margin-bottom`
    BottomMargin,
    /// `margin-left`
    LeftMargin,
    /// `padding-top`
    TopPadding,
    /// `padding-right`
    RightPadding,
    /// `padding-bottom`
    BottomPadding,
    /// `padding-left`
    LeftPadding,
    /// `gap`
    Gap,
    /// `column-gap`
    ColumnGap,
    /// `row-gap`
    RowGap,
    /// `font-size`
    FontSize,
    /// `border-width`
    BorderWidth,
    /// `border-top-width`
    BorderTopWidth,
    /// `border-right-width`
    BorderRightWidth,
    /// `border-bottom-width`
    BorderBottomWidth,
    /// `border-left-width`
    BorderLeftWidth,

    /// `margin`
    Margin,
    /// `padding`
    Padding,
    /// `border-radius`
    BorderRadius,

    /// `color`
    Color,
    /// `background-color`
    BackgroundColor,
    /// `border-color`
    BorderColor,
    /// `outline-color`
    OutlineColor,
    /// `text-decoration-color`
    TextDecorationColor,

    /// `z-index`
    ZIndex,
    /// `order`
    Order,
    /// `flex-grow`
    FlexGrow,
    /// `flex-shrink`
    FlexShrink,

    /// Any non-enumerated property.
    Other(String),
}

impl CssProp {
    /// Returns the CSS property name.
    pub fn css_name(&self) -> Cow<'_, str> {
        match self {
            Self::Width => Cow::Borrowed("width"),
            Self::Height => Cow::Borrowed("height"),
            Self::MinWidth => Cow::Borrowed("min-width"),
            Self::MinHeight => Cow::Borrowed("min-height"),
            Self::MaxWidth => Cow::Borrowed("max-width"),
            Self::MaxHeight => Cow::Borrowed("max-height"),
            Self::Top => Cow::Borrowed("top"),
            Self::Right => Cow::Borrowed("right"),
            Self::Bottom => Cow::Borrowed("bottom"),
            Self::Left => Cow::Borrowed("left"),
            Self::TopMargin => Cow::Borrowed("margin-top"),
            Self::RightMargin => Cow::Borrowed("margin-right"),
            Self::BottomMargin => Cow::Borrowed("margin-bottom"),
            Self::LeftMargin => Cow::Borrowed("margin-left"),
            Self::TopPadding => Cow::Borrowed("padding-top"),
            Self::RightPadding => Cow::Borrowed("padding-right"),
            Self::BottomPadding => Cow::Borrowed("padding-bottom"),
            Self::LeftPadding => Cow::Borrowed("padding-left"),
            Self::Gap => Cow::Borrowed("gap"),
            Self::ColumnGap => Cow::Borrowed("column-gap"),
            Self::RowGap => Cow::Borrowed("row-gap"),
            Self::FontSize => Cow::Borrowed("font-size"),
            Self::BorderWidth => Cow::Borrowed("border-width"),
            Self::BorderTopWidth => Cow::Borrowed("border-top-width"),
            Self::BorderRightWidth => Cow::Borrowed("border-right-width"),
            Self::BorderBottomWidth => Cow::Borrowed("border-bottom-width"),
            Self::BorderLeftWidth => Cow::Borrowed("border-left-width"),
            Self::Margin => Cow::Borrowed("margin"),
            Self::Padding => Cow::Borrowed("padding"),
            Self::BorderRadius => Cow::Borrowed("border-radius"),
            Self::Color => Cow::Borrowed("color"),
            Self::BackgroundColor => Cow::Borrowed("background-color"),
            Self::BorderColor => Cow::Borrowed("border-color"),
            Self::OutlineColor => Cow::Borrowed("outline-color"),
            Self::TextDecorationColor => Cow::Borrowed("text-decoration-color"),
            Self::ZIndex => Cow::Borrowed("z-index"),
            Self::Order => Cow::Borrowed("order"),
            Self::FlexGrow => Cow::Borrowed("flex-grow"),
            Self::FlexShrink => Cow::Borrowed("flex-shrink"),
            Self::Other(name) => Cow::Borrowed(name.as_str()),
        }
    }

    /// Returns the expected value kind for this property, if known.
    pub fn expected_kind(&self) -> Option<CssValueKind> {
        match self {
            Self::Width
            | Self::Height
            | Self::MinWidth
            | Self::MinHeight
            | Self::MaxWidth
            | Self::MaxHeight
            | Self::Top
            | Self::Right
            | Self::Bottom
            | Self::Left
            | Self::TopMargin
            | Self::RightMargin
            | Self::BottomMargin
            | Self::LeftMargin
            | Self::TopPadding
            | Self::RightPadding
            | Self::BottomPadding
            | Self::LeftPadding
            | Self::Gap
            | Self::ColumnGap
            | Self::RowGap
            | Self::FontSize
            | Self::BorderWidth
            | Self::BorderTopWidth
            | Self::BorderRightWidth
            | Self::BorderBottomWidth
            | Self::BorderLeftWidth => Some(CssValueKind::Sizing),
            Self::Margin | Self::Padding | Self::BorderRadius => Some(CssValueKind::SizingMulti),
            Self::Color
            | Self::BackgroundColor
            | Self::BorderColor
            | Self::OutlineColor
            | Self::TextDecorationColor => Some(CssValueKind::Color),
            Self::ZIndex | Self::Order | Self::FlexGrow | Self::FlexShrink => {
                Some(CssValueKind::Integer)
            }
            Self::Other(_) => None,
        }
    }

    /// Parses a property name to a known property or `Other(...)`.
    pub fn from_css_name(name: &str) -> Result<Self, StylesheetError> {
        let normalized = name.trim();
        if normalized.is_empty() || !is_valid_property_name(normalized) {
            return Err(StylesheetError::InvalidPropertyName {
                name: name.to_string(),
            });
        }

        let lower = normalized.to_ascii_lowercase();
        let prop = match lower.as_str() {
            "width" => Self::Width,
            "height" => Self::Height,
            "min-width" => Self::MinWidth,
            "min-height" => Self::MinHeight,
            "max-width" => Self::MaxWidth,
            "max-height" => Self::MaxHeight,
            "top" => Self::Top,
            "right" => Self::Right,
            "bottom" => Self::Bottom,
            "left" => Self::Left,
            "margin-top" => Self::TopMargin,
            "margin-right" => Self::RightMargin,
            "margin-bottom" => Self::BottomMargin,
            "margin-left" => Self::LeftMargin,
            "padding-top" => Self::TopPadding,
            "padding-right" => Self::RightPadding,
            "padding-bottom" => Self::BottomPadding,
            "padding-left" => Self::LeftPadding,
            "gap" => Self::Gap,
            "column-gap" => Self::ColumnGap,
            "row-gap" => Self::RowGap,
            "font-size" => Self::FontSize,
            "border-width" => Self::BorderWidth,
            "border-top-width" => Self::BorderTopWidth,
            "border-right-width" => Self::BorderRightWidth,
            "border-bottom-width" => Self::BorderBottomWidth,
            "border-left-width" => Self::BorderLeftWidth,
            "margin" => Self::Margin,
            "padding" => Self::Padding,
            "border-radius" => Self::BorderRadius,
            "color" => Self::Color,
            "background-color" => Self::BackgroundColor,
            "border-color" => Self::BorderColor,
            "outline-color" => Self::OutlineColor,
            "text-decoration-color" => Self::TextDecorationColor,
            "z-index" => Self::ZIndex,
            "order" => Self::Order,
            "flex-grow" => Self::FlexGrow,
            "flex-shrink" => Self::FlexShrink,
            _ => Self::Other(normalized.to_string()),
        };

        Ok(prop)
    }
}

/// Typed property set for singular sizing properties.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CssSizingProp {
    /// `width`
    Width,
    /// `height`
    Height,
    /// `min-width`
    MinWidth,
    /// `min-height`
    MinHeight,
    /// `max-width`
    MaxWidth,
    /// `max-height`
    MaxHeight,
    /// `top`
    Top,
    /// `right`
    Right,
    /// `bottom`
    Bottom,
    /// `left`
    Left,
    /// `margin-top`
    TopMargin,
    /// `margin-right`
    RightMargin,
    /// `margin-bottom`
    BottomMargin,
    /// `margin-left`
    LeftMargin,
    /// `padding-top`
    TopPadding,
    /// `padding-right`
    RightPadding,
    /// `padding-bottom`
    BottomPadding,
    /// `padding-left`
    LeftPadding,
    /// `gap`
    Gap,
    /// `column-gap`
    ColumnGap,
    /// `row-gap`
    RowGap,
    /// `font-size`
    FontSize,
    /// `border-width`
    BorderWidth,
    /// `border-top-width`
    BorderTopWidth,
    /// `border-right-width`
    BorderRightWidth,
    /// `border-bottom-width`
    BorderBottomWidth,
    /// `border-left-width`
    BorderLeftWidth,
}

impl From<CssSizingProp> for CssProp {
    fn from(value: CssSizingProp) -> Self {
        match value {
            CssSizingProp::Width => Self::Width,
            CssSizingProp::Height => Self::Height,
            CssSizingProp::MinWidth => Self::MinWidth,
            CssSizingProp::MinHeight => Self::MinHeight,
            CssSizingProp::MaxWidth => Self::MaxWidth,
            CssSizingProp::MaxHeight => Self::MaxHeight,
            CssSizingProp::Top => Self::Top,
            CssSizingProp::Right => Self::Right,
            CssSizingProp::Bottom => Self::Bottom,
            CssSizingProp::Left => Self::Left,
            CssSizingProp::TopMargin => Self::TopMargin,
            CssSizingProp::RightMargin => Self::RightMargin,
            CssSizingProp::BottomMargin => Self::BottomMargin,
            CssSizingProp::LeftMargin => Self::LeftMargin,
            CssSizingProp::TopPadding => Self::TopPadding,
            CssSizingProp::RightPadding => Self::RightPadding,
            CssSizingProp::BottomPadding => Self::BottomPadding,
            CssSizingProp::LeftPadding => Self::LeftPadding,
            CssSizingProp::Gap => Self::Gap,
            CssSizingProp::ColumnGap => Self::ColumnGap,
            CssSizingProp::RowGap => Self::RowGap,
            CssSizingProp::FontSize => Self::FontSize,
            CssSizingProp::BorderWidth => Self::BorderWidth,
            CssSizingProp::BorderTopWidth => Self::BorderTopWidth,
            CssSizingProp::BorderRightWidth => Self::BorderRightWidth,
            CssSizingProp::BorderBottomWidth => Self::BorderBottomWidth,
            CssSizingProp::BorderLeftWidth => Self::BorderLeftWidth,
        }
    }
}

/// Typed property set for 1..=4 value sizing properties.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CssSizingMultiProp {
    /// `margin`
    Margin,
    /// `padding`
    Padding,
    /// `border-radius`
    BorderRadius,
}

impl From<CssSizingMultiProp> for CssProp {
    fn from(value: CssSizingMultiProp) -> Self {
        match value {
            CssSizingMultiProp::Margin => Self::Margin,
            CssSizingMultiProp::Padding => Self::Padding,
            CssSizingMultiProp::BorderRadius => Self::BorderRadius,
        }
    }
}

/// Typed property set for color values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CssColorProp {
    /// `color`
    Color,
    /// `background-color`
    BackgroundColor,
    /// `border-color`
    BorderColor,
    /// `outline-color`
    OutlineColor,
    /// `text-decoration-color`
    TextDecorationColor,
}

impl From<CssColorProp> for CssProp {
    fn from(value: CssColorProp) -> Self {
        match value {
            CssColorProp::Color => Self::Color,
            CssColorProp::BackgroundColor => Self::BackgroundColor,
            CssColorProp::BorderColor => Self::BorderColor,
            CssColorProp::OutlineColor => Self::OutlineColor,
            CssColorProp::TextDecorationColor => Self::TextDecorationColor,
        }
    }
}

/// Typed property set for integer values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CssIntegerProp {
    /// `z-index`
    ZIndex,
    /// `order`
    Order,
    /// `flex-grow`
    FlexGrow,
    /// `flex-shrink`
    FlexShrink,
}

impl From<CssIntegerProp> for CssProp {
    fn from(value: CssIntegerProp) -> Self {
        match value {
            CssIntegerProp::ZIndex => Self::ZIndex,
            CssIntegerProp::Order => Self::Order,
            CssIntegerProp::FlexGrow => Self::FlexGrow,
            CssIntegerProp::FlexShrink => Self::FlexShrink,
        }
    }
}

/// Typed wrapper for custom CSS properties.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CssCustomProp(String);

impl CssCustomProp {
    /// Creates a custom CSS property wrapper.
    pub fn new(name: impl Into<String>) -> Result<Self, StylesheetError> {
        let name = name.into();
        if !is_valid_property_name(&name) {
            return Err(StylesheetError::InvalidCustomProperty { name });
        }
        Ok(Self(name))
    }

    /// Returns the underlying property name.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<&str> for CssCustomProp {
    type Error = StylesheetError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<String> for CssCustomProp {
    type Error = StylesheetError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<CssCustomProp> for CssProp {
    fn from(value: CssCustomProp) -> Self {
        Self::Other(value.0)
    }
}

/// Unit options for dimension-based sizing values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CssUnit {
    /// Pixels (`px`).
    Px,
    /// Root em (`rem`).
    Rem,
    /// Em (`em`).
    Em,
    /// Viewport width (`vw`).
    Vw,
    /// Viewport height (`vh`).
    Vh,
    /// Viewport minimum (`vmin`).
    Vmin,
    /// Viewport maximum (`vmax`).
    Vmax,
    /// Character width (`ch`).
    Ch,
    /// X-height (`ex`).
    Ex,
    /// Points (`pt`).
    Pt,
    /// Picas (`pc`).
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
    /// Returns the unit string.
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

/// A single sizing value.
#[derive(Debug, Clone, PartialEq)]
pub enum CssSizing {
    /// A number with unit.
    Dimension {
        /// Numeric value.
        value: f32,
        /// Unit suffix.
        unit: CssUnit,
    },
    /// `0` shorthand.
    Zero,
    /// `auto`
    Auto,
    /// `min-content`
    MinContent,
    /// `max-content`
    MaxContent,
    /// `fit-content`
    FitContent,
    /// `inherit`
    Inherit,
    /// `initial`
    Initial,
    /// `unset`
    Unset,
    /// Function-like expression such as `calc(...)`.
    Expression(String),
}

impl CssSizing {
    /// Creates a dimension value.
    pub fn dimension(value: f32, unit: CssUnit) -> Self {
        if value == 0.0 {
            return Self::Zero;
        }

        Self::Dimension { value, unit }
    }

    /// Creates a `px` sizing.
    pub fn px(value: f32) -> Self {
        Self::dimension(value, CssUnit::Px)
    }

    /// Creates a `rem` sizing.
    pub fn rem(value: f32) -> Self {
        Self::dimension(value, CssUnit::Rem)
    }

    /// Creates an `em` sizing.
    pub fn em(value: f32) -> Self {
        Self::dimension(value, CssUnit::Em)
    }

    /// Creates a `%` sizing.
    pub fn percent(value: f32) -> Self {
        Self::dimension(value, CssUnit::Percent)
    }

    /// Creates a function-like expression value.
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

impl TryFrom<String> for CssSizing {
    type Error = StylesheetError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::try_from(value.as_str())
    }
}

/// A sizing shorthand that can contain 1 to 4 sizing values.
#[derive(Debug, Clone, PartialEq)]
pub enum CssSizingMulti {
    /// One value.
    One(CssSizing),
    /// Two values.
    Two(CssSizing, CssSizing),
    /// Three values.
    Three(CssSizing, CssSizing, CssSizing),
    /// Four values.
    Four(CssSizing, CssSizing, CssSizing, CssSizing),
}

impl CssSizingMulti {
    /// Builds from a vector of 1..=4 sizing values.
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

    fn as_slice(&self) -> Vec<&CssSizing> {
        match self {
            Self::One(a) => vec![a],
            Self::Two(a, b) => vec![a, b],
            Self::Three(a, b, c) => vec![a, b, c],
            Self::Four(a, b, c, d) => vec![a, b, c, d],
        }
    }
}

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

impl From<CssSizing> for CssSizingMulti {
    fn from(value: CssSizing) -> Self {
        Self::One(value)
    }
}

impl From<(CssSizing, CssSizing)> for CssSizingMulti {
    fn from(value: (CssSizing, CssSizing)) -> Self {
        Self::Two(value.0, value.1)
    }
}

impl From<(CssSizing, CssSizing, CssSizing)> for CssSizingMulti {
    fn from(value: (CssSizing, CssSizing, CssSizing)) -> Self {
        Self::Three(value.0, value.1, value.2)
    }
}

impl From<(CssSizing, CssSizing, CssSizing, CssSizing)> for CssSizingMulti {
    fn from(value: (CssSizing, CssSizing, CssSizing, CssSizing)) -> Self {
        Self::Four(value.0, value.1, value.2, value.3)
    }
}

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

impl TryFrom<String> for CssSizingMulti {
    type Error = StylesheetError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::try_from(value.as_str())
    }
}

/// A validated color token.
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
    pub fn hex(value: impl Into<String>) -> Result<Self, StylesheetError> {
        let value = value.into();
        let trimmed = value.trim();
        if !is_hex_color(trimmed) {
            return Err(StylesheetError::InvalidColor { value });
        }
        Ok(Self::Hex(trimmed.to_ascii_lowercase()))
    }

    /// Creates an RGB color.
    pub fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self::Rgb(r, g, b)
    }

    /// Creates an RGBA color.
    pub fn rgba(r: u8, g: u8, b: u8, a: f32) -> Result<Self, StylesheetError> {
        if !(0.0..=1.0).contains(&a) || !a.is_finite() {
            return Err(StylesheetError::InvalidColor {
                value: format!("rgba({r}, {g}, {b}, {a})"),
            });
        }
        Ok(Self::Rgba(r, g, b, a))
    }

    /// Creates a validated named color.
    pub fn named(value: impl Into<String>) -> Result<Self, StylesheetError> {
        let value = value.into();
        let trimmed = value.trim();
        if !is_named_color(trimmed) {
            return Err(StylesheetError::InvalidColor { value });
        }
        Ok(Self::Named(trimmed.to_ascii_lowercase()))
    }

    /// Creates a function-like color expression.
    pub fn expression(value: impl Into<String>) -> Result<Self, StylesheetError> {
        let value = value.into();
        let trimmed = value.trim();

        if trimmed.is_empty() || trimmed.contains(';') || !is_css_function_call(trimmed) {
            return Err(StylesheetError::InvalidColor { value });
        }

        Ok(Self::Expression(trimmed.to_string()))
    }
}

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

impl TryFrom<String> for CssColor {
    type Error = StylesheetError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::try_from(value.as_str())
    }
}

/// Validated raw CSS value for custom properties.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CssRaw(String);

impl CssRaw {
    /// Creates a validated raw CSS value.
    pub fn new(value: impl Into<String>) -> Result<Self, StylesheetError> {
        let value = value.into();
        let trimmed = value.trim();
        if trimmed.is_empty() || trimmed.contains(';') {
            return Err(StylesheetError::InvalidRawValue { value });
        }
        Ok(Self(trimmed.to_string()))
    }

    /// Returns the raw value string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for CssRaw {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<&str> for CssRaw {
    type Error = StylesheetError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<String> for CssRaw {
    type Error = StylesheetError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

/// Stored CSS value representation.
#[derive(Debug, Clone, PartialEq)]
pub enum CssValue {
    /// Single sizing value.
    Sizing(CssSizing),
    /// Multi sizing value.
    SizingMulti(CssSizingMulti),
    /// Color value.
    Color(CssColor),
    /// Integer value.
    Integer(i32),
    /// Raw value.
    Raw(CssRaw),
}

impl CssValue {
    /// Returns the coarse value kind.
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

/// Converts typed values into [`CssValue`].
pub trait IntoCssValue {
    /// Converts to the internal representation.
    fn into_css_value(self) -> CssValue;
}

impl IntoCssValue for CssSizing {
    fn into_css_value(self) -> CssValue {
        CssValue::Sizing(self)
    }
}

impl IntoCssValue for CssSizingMulti {
    fn into_css_value(self) -> CssValue {
        CssValue::SizingMulti(self)
    }
}

impl IntoCssValue for CssColor {
    fn into_css_value(self) -> CssValue {
        CssValue::Color(self)
    }
}

impl IntoCssValue for i32 {
    fn into_css_value(self) -> CssValue {
        CssValue::Integer(self)
    }
}

impl IntoCssValue for CssRaw {
    fn into_css_value(self) -> CssValue {
        CssValue::Raw(self)
    }
}

impl IntoCssValue for CssValue {
    fn into_css_value(self) -> CssValue {
        self
    }
}

/// Trait mapping typed properties to their accepted value type.
pub trait CssTypedProperty {
    /// The allowed value type for this property.
    type Value: IntoCssValue;

    /// Converts the typed property to a generic [`CssProp`].
    fn into_css_prop(self) -> CssProp;
}

impl CssTypedProperty for CssSizingProp {
    type Value = CssSizing;

    fn into_css_prop(self) -> CssProp {
        self.into()
    }
}

impl CssTypedProperty for CssSizingMultiProp {
    type Value = CssSizingMulti;

    fn into_css_prop(self) -> CssProp {
        self.into()
    }
}

impl CssTypedProperty for CssColorProp {
    type Value = CssColor;

    fn into_css_prop(self) -> CssProp {
        self.into()
    }
}

impl CssTypedProperty for CssIntegerProp {
    type Value = i32;

    fn into_css_prop(self) -> CssProp {
        self.into()
    }
}

impl CssTypedProperty for CssCustomProp {
    type Value = CssRaw;

    fn into_css_prop(self) -> CssProp {
        self.into()
    }
}

#[derive(Debug, Clone, PartialEq)]
struct CssDeclaration {
    prop: CssProp,
    value: CssValue,
}

/// Type-safe CSS declaration container.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Stylesheet {
    declarations: Vec<CssDeclaration>,
}

impl Stylesheet {
    /// Creates an empty stylesheet.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the number of declarations.
    pub fn len(&self) -> usize {
        self.declarations.len()
    }

    /// Returns true when the stylesheet has no declarations.
    pub fn is_empty(&self) -> bool {
        self.declarations.is_empty()
    }

    /// Adds a typed property/value declaration and returns `self`.
    ///
    /// This method is compile-time constrained by [`CssTypedProperty`], so
    /// property groups only accept their matching value type.
    pub fn add<T, U>(mut self, prop: T, value: U) -> Self
    where
        T: CssTypedProperty<Value = U>,
        U: IntoCssValue,
    {
        let prop = prop.into_css_prop();
        let value = value.into_css_value();

        debug_assert!(
            is_value_allowed_for_prop(&prop, &value),
            "typed property/value pair should always be valid"
        );

        self.insert_or_replace(prop, value);
        self
    }

    /// Adds a dynamic property/value declaration with runtime validation.
    pub fn try_add(mut self, prop: CssProp, value: CssValue) -> Result<Self, StylesheetError> {
        ensure_valid_property_value_pair(&prop, &value)?;
        self.insert_or_replace(prop, value);
        Ok(self)
    }

    /// Renders the stylesheet as CSS declarations.
    pub fn to_css(&self) -> String {
        self.declarations
            .iter()
            .map(|decl| {
                let name = decl.prop.css_name();
                format!("{name}: {};", decl.value)
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Renders the stylesheet with ANSI coloring and writes it to stdout.
    pub fn to_terminal(&self, terminal: &Terminal) {
        let output = self.to_terminal_string(terminal);
        if !output.is_empty() {
            println!("{output}");
        }
    }

    /// Renders the stylesheet for terminal output with optional ANSI styling.
    pub fn to_terminal_string(&self, terminal: &Terminal) -> String {
        if self.is_empty() {
            return String::new();
        }

        let colorize = terminal.is_tty;

        self.declarations
            .iter()
            .map(|decl| {
                let name = decl.prop.css_name();
                let value_text = decl.value.to_string();

                if !colorize {
                    return format!("{name}: {value_text};");
                }

                let styled_name =
                    prose_style_text(terminal, "<bold><rgb 97,175,239>{text}</rgb></bold>", &name);
                let styled_colon = Prose::new("<rgb 160,160,160>:</rgb>").render(terminal);
                let styled_value = match decl.value.kind() {
                    CssValueKind::Sizing | CssValueKind::SizingMulti => {
                        prose_style_text(terminal, "<rgb 86,182,194>{text}</rgb>", &value_text)
                    }
                    CssValueKind::Color => {
                        prose_style_text(terminal, "<rgb 152,195,121>{text}</rgb>", &value_text)
                    }
                    CssValueKind::Integer => {
                        prose_style_text(terminal, "<rgb 229,192,123>{text}</rgb>", &value_text)
                    }
                    CssValueKind::Raw => {
                        prose_style_text(terminal, "<rgb 220,220,220>{text}</rgb>", &value_text)
                    }
                };
                let styled_semicolon = Prose::new("<rgb 160,160,160>;</rgb>").render(terminal);

                format!("{styled_name}{styled_colon} {styled_value}{styled_semicolon}")
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Renders declarations as pretty JSON.
    ///
    /// Integer declarations are emitted as JSON numbers; all other categories are
    /// emitted as strings.
    pub fn to_json(&self) -> String {
        let mut map = Map::new();
        for decl in &self.declarations {
            map.insert(
                decl.prop.css_name().into_owned(),
                json_value_for_css_value(&decl.value),
            );
        }

        match serde_json::to_string_pretty(&Value::Object(map)) {
            Ok(json) => json,
            Err(_) => "{}".to_string(),
        }
    }

    /// Renders declarations as JSON5.
    ///
    /// Output uses trailing commas and unquoted keys when they are valid JSON5
    /// identifiers. Values follow the same typing rules as [`Stylesheet::to_json`].
    pub fn to_json5(&self) -> String {
        if self.declarations.is_empty() {
            return "{}".to_string();
        }

        let mut lines = Vec::with_capacity(self.declarations.len());
        for decl in &self.declarations {
            let key = json5_key(&decl.prop.css_name());
            let value = json5_value(&decl.value);
            lines.push(format!("  {key}: {value},"));
        }

        format!("{{\n{}\n}}", lines.join("\n"))
    }

    fn insert_or_replace(&mut self, prop: CssProp, value: CssValue) {
        if let Some(existing) = self.declarations.iter_mut().find(|decl| decl.prop == prop) {
            existing.value = value;
            return;
        }

        self.declarations.push(CssDeclaration { prop, value });
    }
}

impl Display for Stylesheet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_css())
    }
}

impl TryFrom<&str> for Stylesheet {
    type Error = StylesheetError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let mut stylesheet = Stylesheet::new();
        let content = strip_surrounding_braces(value.trim());

        for declaration in split_declarations(content) {
            let declaration = declaration.trim();
            if declaration.is_empty() {
                continue;
            }

            let (prop_text, value_text) =
                declaration
                    .split_once(':')
                    .ok_or_else(|| StylesheetError::InvalidDeclaration {
                        declaration: declaration.to_string(),
                    })?;

            let prop = CssProp::from_css_name(prop_text)?;
            let value = parse_value_for_prop(&prop, value_text.trim())?;
            stylesheet = stylesheet.try_add(prop, value)?;
        }

        Ok(stylesheet)
    }
}

impl TryFrom<&String> for Stylesheet {
    type Error = StylesheetError;

    fn try_from(value: &String) -> Result<Self, Self::Error> {
        Self::try_from(value.as_str())
    }
}

impl TryFrom<String> for Stylesheet {
    type Error = StylesheetError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::try_from(value.as_str())
    }
}

fn parse_value_for_prop(prop: &CssProp, value: &str) -> Result<CssValue, StylesheetError> {
    match prop.expected_kind() {
        Some(CssValueKind::Sizing) => Ok(CssSizing::try_from(value)?.into_css_value()),
        Some(CssValueKind::SizingMulti) => Ok(CssSizingMulti::try_from(value)?.into_css_value()),
        Some(CssValueKind::Color) => Ok(CssColor::try_from(value)?.into_css_value()),
        Some(CssValueKind::Integer) => {
            let parsed =
                value
                    .trim()
                    .parse::<i32>()
                    .map_err(|_| StylesheetError::InvalidInteger {
                        value: value.to_string(),
                    })?;
            Ok(parsed.into_css_value())
        }
        Some(CssValueKind::Raw) | None => Ok(CssRaw::new(value)?.into_css_value()),
    }
}

fn ensure_valid_property_value_pair(
    prop: &CssProp,
    value: &CssValue,
) -> Result<(), StylesheetError> {
    if let Some(expected_kind) = prop.expected_kind() {
        let actual_kind = value.kind();
        if expected_kind != actual_kind {
            return Err(StylesheetError::PropertyValueTypeMismatch {
                property: prop.css_name().into_owned(),
                expected: expected_kind,
                actual: actual_kind,
                value: value.to_string(),
            });
        }
    }

    Ok(())
}

fn is_value_allowed_for_prop(prop: &CssProp, value: &CssValue) -> bool {
    match prop.expected_kind() {
        Some(expected) => expected == value.kind(),
        None => true,
    }
}

fn json_value_for_css_value(value: &CssValue) -> Value {
    match value {
        CssValue::Integer(number) => Value::from(*number),
        _ => Value::from(value.to_string()),
    }
}

fn json5_key(key: &str) -> String {
    if is_json5_identifier(key) {
        key.to_string()
    } else {
        json_quote(key)
    }
}

fn json5_value(value: &CssValue) -> String {
    match value {
        CssValue::Integer(number) => number.to_string(),
        _ => json_quote(&value.to_string()),
    }
}

fn json_quote(value: &str) -> String {
    match serde_json::to_string(value) {
        Ok(serialized) => serialized,
        Err(_) => {
            let escaped = value.replace('\\', "\\\\").replace('"', "\\\"");
            format!("\"{escaped}\"")
        }
    }
}

fn is_json5_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };

    if !(first.is_ascii_alphabetic() || first == '_' || first == '$') {
        return false;
    }

    chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '$')
}

fn is_valid_property_name(name: &str) -> bool {
    if let Some(rest) = name.strip_prefix("--") {
        return !rest.is_empty() && rest.chars().all(|ch| is_css_ident_char(ch, true));
    }

    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };

    if !(first.is_ascii_alphabetic() || first == '-') {
        return false;
    }

    if first == '-' {
        let Some(second) = chars.next() else {
            return false;
        };
        if !second.is_ascii_alphabetic() {
            return false;
        }
        return chars.all(|ch| is_css_ident_char(ch, false));
    }

    chars.all(|ch| is_css_ident_char(ch, false))
}

fn is_css_ident_char(ch: char, allow_underscore: bool) -> bool {
    ch.is_ascii_alphanumeric() || ch == '-' || (allow_underscore && ch == '_')
}

fn is_zero_number(value: &str) -> bool {
    match value.parse::<f32>() {
        Ok(number) => number == 0.0,
        Err(_) => false,
    }
}

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

fn is_hex_color(value: &str) -> bool {
    let Some(hex) = value.strip_prefix('#') else {
        return false;
    };

    matches!(hex.len(), 3 | 4 | 6 | 8) && hex.chars().all(|ch| ch.is_ascii_hexdigit())
}

fn is_named_color(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphabetic() || ch == '-')
}

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

fn parse_u8_channel(value: &str, original: &str) -> Result<u8, StylesheetError> {
    value
        .parse::<u8>()
        .map_err(|_| StylesheetError::InvalidColor {
            value: original.to_string(),
        })
}

fn split_declarations(input: &str) -> Vec<String> {
    let mut declarations = Vec::new();
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
            ';' if depth == 0 => {
                let trimmed = current.trim();
                if !trimmed.is_empty() {
                    declarations.push(trimmed.to_string());
                }
                current.clear();
            }
            _ => current.push(ch),
        }
    }

    let trimmed = current.trim();
    if !trimmed.is_empty() {
        declarations.push(trimmed.to_string());
    }

    declarations
}

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

fn strip_surrounding_braces(input: &str) -> &str {
    let trimmed = input.trim();
    if trimmed.starts_with('{') && trimmed.ends_with('}') && trimmed.len() >= 2 {
        &trimmed[1..trimmed.len() - 1]
    } else {
        trimmed
    }
}

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

fn prose_style_text(term: &Terminal, template: &str, text: &str) -> String {
    let placeholder = unique_placeholder(text);
    let source = template.replace("{text}", &placeholder);
    let rendered = Prose::new(source).render(term);
    rendered.replace(&placeholder, text)
}

fn unique_placeholder(text: &str) -> String {
    let mut idx = 0usize;
    loop {
        let candidate = format!("__DM_STYLESHEET_TEXT_{idx}__");
        if !text.contains(&candidate) {
            return candidate;
        }
        idx += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typed_add_builds_valid_stylesheet() {
        let stylesheet = Stylesheet::new()
            .add(CssSizingProp::TopMargin, CssSizing::px(12.0))
            .add(
                CssSizingMultiProp::Margin,
                CssSizingMulti::from((CssSizing::px(8.0), CssSizing::px(16.0))),
            )
            .add(
                CssColorProp::Color,
                CssColor::hex("#336699").expect("hex color should be valid"),
            )
            .add(CssIntegerProp::ZIndex, 40);

        assert_eq!(
            stylesheet.to_css(),
            "margin-top: 12px;\nmargin: 8px 16px;\ncolor: #336699;\nz-index: 40;"
        );
    }

    #[test]
    fn parse_from_css_string_validates_by_property_category() {
        let stylesheet = Stylesheet::try_from(
            "margin-top: 8px; margin: 8px 12px; color: rgb(255, 0, 10); z-index: 7;",
        )
        .expect("stylesheet should parse");

        assert_eq!(stylesheet.len(), 4);
        assert!(stylesheet.to_css().contains("margin-top: 8px;"));
        assert!(stylesheet.to_css().contains("margin: 8px 12px;"));
        assert!(stylesheet.to_css().contains("color: rgb(255, 0, 10);"));
        assert!(stylesheet.to_css().contains("z-index: 7;"));
    }

    #[test]
    fn parse_rejects_invalid_integer_property_value() {
        let err = Stylesheet::try_from("z-index: 1px;").expect_err("z-index should reject sizing");
        assert!(matches!(err, StylesheetError::InvalidInteger { .. }));
    }

    #[test]
    fn custom_property_supports_raw_values() {
        let custom = CssCustomProp::new("--chip-shadow").expect("custom property should be valid");
        let raw = CssRaw::new("0 0 0 1px rgba(0, 0, 0, 0.1)").expect("raw value should be valid");

        let stylesheet = Stylesheet::new().add(custom, raw);
        assert_eq!(
            stylesheet.to_css(),
            "--chip-shadow: 0 0 0 1px rgba(0, 0, 0, 0.1);"
        );
    }

    #[test]
    fn to_json_preserves_integer_type() {
        let stylesheet = Stylesheet::new()
            .add(CssSizingProp::Width, CssSizing::rem(42.0))
            .add(CssIntegerProp::ZIndex, 9);

        let json = stylesheet.to_json();
        assert!(json.contains("\"width\": \"42rem\""));
        assert!(json.contains("\"z-index\": 9"));
    }

    #[test]
    fn to_json5_uses_object_syntax_with_trailing_commas() {
        let stylesheet = Stylesheet::new()
            .add(CssSizingProp::Width, CssSizing::px(320.0))
            .add(CssIntegerProp::Order, 2);

        let json5 = stylesheet.to_json5();
        assert!(json5.starts_with("{\n"));
        assert!(json5.contains("width: \"320px\","));
        assert!(json5.contains("order: 2,"));
        assert!(json5.ends_with("\n}"));
    }

    #[test]
    fn terminal_rendering_contains_ansi_when_tty() {
        let stylesheet = Stylesheet::new().add(CssColorProp::Color, CssColor::rgb(1, 2, 3));
        let terminal = Terminal::new_tty();

        let rendered = stylesheet.to_terminal_string(&terminal);
        assert!(rendered.contains("\u{1b}["));
        assert!(rendered.contains("color"));
        assert!(rendered.contains("rgb(1, 2, 3)"));
    }

    #[test]
    fn parsing_allows_wrapped_braces() {
        let stylesheet = Stylesheet::try_from("{ color: #fff; z-index: 2; }")
            .expect("stylesheet with braces should parse");
        assert_eq!(stylesheet.len(), 2);
        assert!(stylesheet.to_css().contains("color: #fff;"));
    }
}
