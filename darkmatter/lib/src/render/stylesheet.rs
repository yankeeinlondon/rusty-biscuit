//! Type-safe CSS stylesheet declarations.
//!
//! This module provides a strongly-typed builder for CSS declarations that is
//! independent of any single render target. A [`Stylesheet`] is a list of
//! `property: value` pairs that can be emitted as:
//!
//! - Plain CSS text (via [`Stylesheet::to_css`] or [`Display`])
//! - ANSI-styled terminal output (via [`Stylesheet::to_terminal_string`])
//! - Pretty-printed JSON (via [`Stylesheet::to_json`])
//! - JSON5 with unquoted keys and trailing commas (via [`Stylesheet::to_json5`])
//!
//! ## Design
//!
//! The type system is layered:
//!
//! 1. **Properties** are enumerated in [`CssProp`] (the runtime type) plus four
//!    typed subsets — [`CssSizingProp`], [`CssSizingMultiProp`], [`CssColorProp`],
//!    [`CssIntegerProp`] — and a custom-property newtype [`CssCustomProp`].
//! 2. **Values** are enumerated in [`CssValue`] with five categories matching
//!    [`CssValueKind`]: `Sizing`, `SizingMulti`, `Color`, `Integer`, `Raw`.
//! 3. **The [`CssTypedProperty`] trait** pairs each typed property subset with
//!    its accepted value type, allowing [`Stylesheet::add`] to be compile-time
//!    checked: passing a `CssColor` to a `CssSizingProp` is a type error.
//!
//! For dynamic input (e.g. parsing from a string), [`Stylesheet::try_add`] and
//! [`Stylesheet::try_from`] perform runtime validation and return
//! [`StylesheetError`] on failure.
//!
//! ## Examples
//!
//! Building a stylesheet with compile-time checked typed properties:
//!
//! ```ignore
//! use darkmatter::render::stylesheet::{
//!     CssColor, CssColorProp, CssIntegerProp, CssSizing, CssSizingProp, Stylesheet,
//! };
//!
//! let sheet = Stylesheet::new()
//!     .add(CssSizingProp::Width, CssSizing::px(320.0))
//!     .add(CssColorProp::Color, CssColor::rgb(0x33, 0x66, 0x99))
//!     .add(CssIntegerProp::ZIndex, 10);
//!
//! assert_eq!(
//!     sheet.to_css(),
//!     "width: 320px;\ncolor: rgb(51, 102, 153);\nz-index: 10;"
//! );
//! ```
//!
//! Parsing from a CSS-like string:
//!
//! ```ignore
//! use darkmatter::render::stylesheet::Stylesheet;
//!
//! let sheet = Stylesheet::try_from("color: #336699; margin: 8px 16px;").unwrap();
//! assert_eq!(sheet.len(), 2);
//! ```

use std::borrow::Cow;
use std::fmt::{self, Display};

use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::Renderable;
use biscuit_terminal::terminal::Terminal;
use serde_json::{Map, Value};
use thiserror::Error;

/// Errors returned when parsing or validating CSS declarations.
///
/// This is the unified error type for every fallible operation in this module:
/// parsing a stylesheet from a string, constructing a value via a `try_*`
/// constructor, or adding a dynamic property/value pair through
/// [`Stylesheet::try_add`].
///
/// Each variant carries the offending input so callers (and the rendered
/// [`biscuit_terminal::components::status_block::StatusBlock`] produced via the
/// [`biscuit_terminal::errors::BlockError`] impl) can show the user exactly what
/// went wrong.
///
/// ## Examples
///
/// ```ignore
/// use darkmatter::render::stylesheet::{Stylesheet, StylesheetError};
///
/// let err = Stylesheet::try_from("z-index: 1px;").unwrap_err();
/// assert!(matches!(err, StylesheetError::InvalidInteger { .. }));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum StylesheetError {
    /// A declaration did not contain a valid `property: value` pair.
    ///
    /// Returned when a declaration is missing the `:` separator, e.g. when
    /// input like `"color #fff"` is parsed instead of `"color: #fff"`.
    #[error("invalid declaration `{declaration}`; expected `property: value`")]
    InvalidDeclaration {
        /// The raw declaration text that failed to split on `:`.
        declaration: String,
    },

    /// A property name failed basic CSS identifier validation.
    ///
    /// CSS property names must begin with an ASCII letter (or `--` for custom
    /// properties) and contain only letters, digits, and hyphens. See
    /// `is_valid_property_name` for the exact rules.
    #[error("invalid CSS property name `{name}`")]
    InvalidPropertyName {
        /// The rejected property name.
        name: String,
    },

    /// A value's type did not match the expected category for the property.
    ///
    /// Raised by [`Stylesheet::try_add`] and string parsing when, for example,
    /// a [`CssValueKind::Color`] is supplied for a property whose
    /// [`CssProp::expected_kind`] is [`CssValueKind::Sizing`].
    #[error("property `{property}` expects `{expected}` values, but got `{actual}` from `{value}`")]
    PropertyValueTypeMismatch {
        /// CSS name of the property (e.g. `"width"`).
        property: String,
        /// The value category the property requires.
        expected: CssValueKind,
        /// The value category that was actually supplied.
        actual: CssValueKind,
        /// The original value string as it appeared in the input.
        value: String,
    },

    /// A single sizing value was invalid.
    ///
    /// Returned by [`CssSizing::try_from`] when the input is empty, contains
    /// `;`, has an unrecognized unit suffix, is a non-finite number, or fails
    /// the function-call shape check for `Expression` variants.
    #[error("invalid CSS sizing value `{value}`")]
    InvalidSizing {
        /// The rejected sizing input.
        value: String,
    },

    /// A multi-sizing value was invalid.
    ///
    /// Multi-sizing shorthand (`margin`, `padding`, `border-radius`) accepts 1
    /// to 4 whitespace-separated sizing tokens. This is raised when the token
    /// count is outside that range or any individual token fails parsing.
    #[error("invalid CSS multi-sizing value `{value}`; expected 1 to 4 sizing tokens")]
    InvalidSizingMulti {
        /// The rejected multi-sizing input.
        value: String,
    },

    /// A color value was invalid.
    ///
    /// Raised when the input is not a recognized hex, `rgb()`/`rgba()`, named
    /// color, function-like expression, or color keyword
    /// (`transparent`, `currentColor`, `inherit`, `initial`, `unset`).
    #[error("invalid CSS color value `{value}`")]
    InvalidColor {
        /// The rejected color input.
        value: String,
    },

    /// An integer value was invalid.
    ///
    /// Returned when an integer-typed property (e.g. `z-index`, `order`,
    /// `flex-grow`, `flex-shrink`) receives a value that cannot be parsed as
    /// [`i32`].
    #[error("invalid integer value `{value}`")]
    InvalidInteger {
        /// The rejected integer input.
        value: String,
    },

    /// A raw CSS value was invalid.
    ///
    /// Raw values may be any non-empty string that does not contain `;`. This
    /// variant is raised when the input is empty, only whitespace, or contains
    /// a semicolon.
    #[error("invalid raw CSS value `{value}`")]
    InvalidRawValue {
        /// The rejected raw value input.
        value: String,
    },

    /// A custom property name was invalid.
    ///
    /// Custom properties must start with `--` followed by a non-empty
    /// identifier composed of letters, digits, hyphens, or underscores.
    #[error("invalid custom CSS property `{name}`")]
    InvalidCustomProperty {
        /// The rejected custom-property name.
        name: String,
    },
}

/// Renders [`StylesheetError`] into a styled
/// [`biscuit_terminal::components::status_block::StatusBlock`] for CLI display.
///
/// Each variant maps to a `StatusState::Error` block with three parts:
///
/// - **Header** — `"StylesheetError"` plus a short subtitle.
/// - **Body** — the offending input rendered with `<dim>` labels and `<cyan>` values.
/// - **Hint** — a one-line fix suggestion, often including a worked example.
///
/// For [`StylesheetError::PropertyValueTypeMismatch`] the hint is contextual:
/// it tries to resolve the property name back to a [`CssProp`] and looks up a
/// canonical example value via `examples_for_property`; otherwise it falls
/// back to a generic example from `example_for_kind`.
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
                let example = CssProp::from_css_name(property)
                    .ok()
                    .map(|prop| examples_for_property(&prop))
                    .unwrap_or(example_for_kind(*expected));
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

/// Returns a canonical example string for the given value kind.
///
/// Used by [`StylesheetError`]'s status block rendering to suggest a valid
/// value when a category mismatch occurs. The returned slice is short and
/// representative — it is **not** an exhaustive description of the kind's
/// accepted syntax.
fn example_for_kind(kind: CssValueKind) -> &'static str {
    match kind {
        CssValueKind::Sizing => "12px",
        CssValueKind::SizingMulti => "8px 16px",
        CssValueKind::Color => "#336699",
        CssValueKind::Integer => "40",
        CssValueKind::Raw => "var(--token)",
    }
}

/// Returns a property-specific example value used in error messages.
///
/// Falls back to `example_for_kind` for properties without a hand-tuned
/// example, and to `"var(--token)"` when the property has no known
/// [`CssValueKind`] (i.e. [`CssProp::Other`]).
fn examples_for_property(prop: &CssProp) -> &'static str {
    match prop {
        CssProp::FontSize => "16px",
        CssProp::Width | CssProp::MinWidth | CssProp::MaxWidth => "320px",
        CssProp::Height | CssProp::MinHeight | CssProp::MaxHeight => "240px",
        CssProp::Margin | CssProp::Padding => "8px 16px",
        CssProp::BorderRadius => "12px 12px 0 0",
        CssProp::Color
        | CssProp::BackgroundColor
        | CssProp::BorderColor
        | CssProp::OutlineColor
        | CssProp::TextDecorationColor => "#336699",
        CssProp::ZIndex => "10",
        CssProp::Order | CssProp::FlexGrow | CssProp::FlexShrink => "1",
        CssProp::Other(_) => prop
            .expected_kind()
            .map(example_for_kind)
            .unwrap_or("var(--token)"),
        _ => prop
            .expected_kind()
            .map(example_for_kind)
            .unwrap_or("var(--token)"),
    }
}

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
/// ```ignore
/// use darkmatter::render::stylesheet::{CssProp, CssValueKind};
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

/// Enumerates known CSS properties plus a custom fallback.
///
/// `CssProp` is the runtime, type-erased property identifier used inside a
/// [`Stylesheet`]. Each known variant corresponds to a fixed CSS name (see
/// [`CssProp::css_name`]) and reports its expected value category via
/// [`CssProp::expected_kind`].
///
/// For compile-time-checked construction, prefer one of the typed subsets
/// — [`CssSizingProp`], [`CssSizingMultiProp`], [`CssColorProp`],
/// [`CssIntegerProp`], or [`CssCustomProp`] — which all convert into `CssProp`
/// via [`From`]. Properties that don't match any known variant are represented
/// by [`CssProp::Other`].
///
/// ## Property groups
///
/// The variants are organized by expected value kind:
///
/// | Group              | Variants                                                                                                                                                                  | Kind         |
/// |--------------------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------------|--------------|
/// | Sizing             | `Width`, `Height`, `MinWidth`/`MaxWidth`/`MinHeight`/`MaxHeight`, `Top`/`Right`/`Bottom`/`Left`, individual margin/padding/border-width sides, `Gap`, `ColumnGap`, `RowGap`, `FontSize` | `Sizing`     |
/// | Sizing shorthand   | `Margin`, `Padding`, `BorderRadius`                                                                                                                                       | `SizingMulti`|
/// | Color              | `Color`, `BackgroundColor`, `BorderColor`, `OutlineColor`, `TextDecorationColor`                                                                                          | `Color`      |
/// | Integer            | `ZIndex`, `Order`, `FlexGrow`, `FlexShrink`                                                                                                                               | `Integer`    |
/// | Custom / fallback  | `Other(String)`                                                                                                                                                           | unknown      |
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
    /// Returns the canonical CSS property name as it appears in stylesheets.
    ///
    /// Known variants return a borrowed `&'static str`; [`CssProp::Other`]
    /// borrows from the underlying owned [`String`]. The return type is
    /// [`Cow`] to avoid allocating for the common case.
    ///
    /// ## Examples
    ///
    /// ```ignore
    /// use darkmatter::render::stylesheet::CssProp;
    ///
    /// assert_eq!(CssProp::Width.css_name(), "width");
    /// assert_eq!(CssProp::TopMargin.css_name(), "margin-top");
    /// assert_eq!(CssProp::Other("--brand".into()).css_name(), "--brand");
    /// ```
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

    /// Returns the [`CssValueKind`] this property accepts, if known.
    ///
    /// Returns `None` for [`CssProp::Other`] (custom or non-enumerated
    /// properties), since the accepted value type is unknown — such values
    /// are treated as [`CssValueKind::Raw`] when parsed.
    ///
    /// This drives:
    /// - Runtime validation in [`Stylesheet::try_add`] (via
    ///   `ensure_valid_property_value_pair`).
    /// - The debug-time invariant check in [`Stylesheet::add`].
    /// - Property-specific parsing dispatch in `parse_value_for_prop`.
    ///
    /// ## Examples
    ///
    /// ```ignore
    /// use darkmatter::render::stylesheet::{CssProp, CssValueKind};
    ///
    /// assert_eq!(CssProp::FontSize.expected_kind(), Some(CssValueKind::Sizing));
    /// assert_eq!(CssProp::Padding.expected_kind(), Some(CssValueKind::SizingMulti));
    /// assert_eq!(CssProp::BackgroundColor.expected_kind(), Some(CssValueKind::Color));
    /// assert_eq!(CssProp::Order.expected_kind(), Some(CssValueKind::Integer));
    /// assert_eq!(CssProp::Other("--x".into()).expected_kind(), None);
    /// ```
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

    /// Parses a property name into a [`CssProp`].
    ///
    /// The input is trimmed and matched case-insensitively against the
    /// canonical CSS names. Unknown but syntactically valid names are
    /// preserved verbatim (case sensitive) in [`CssProp::Other`].
    ///
    /// ## Returns
    ///
    /// The matching enumerated variant, or [`CssProp::Other`] holding the
    /// trimmed original-case name if no known variant matches.
    ///
    /// ## Errors
    ///
    /// Returns [`StylesheetError::InvalidPropertyName`] when:
    /// - The trimmed input is empty.
    /// - The name fails `is_valid_property_name` (does not match the CSS
    ///   identifier grammar described on that helper).
    ///
    /// ## Examples
    ///
    /// ```ignore
    /// use darkmatter::render::stylesheet::CssProp;
    ///
    /// assert_eq!(CssProp::from_css_name("Width").unwrap(), CssProp::Width);
    /// assert!(matches!(
    ///     CssProp::from_css_name("--brand").unwrap(),
    ///     CssProp::Other(_)
    /// ));
    /// assert!(CssProp::from_css_name(" ").is_err());
    /// ```
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

/// Compile-time-checked subset of [`CssProp`] for properties that accept a
/// single [`CssSizing`] value.
///
/// Use this with [`Stylesheet::add`] to get static guarantees that the value
/// type matches the property — the alternative ([`CssProp`] + [`CssValue`] via
/// [`Stylesheet::try_add`]) defers checking to runtime.
///
/// All variants here lower to [`CssValueKind::Sizing`]; see [`CssProp`]'s
/// property-group table for the full list.
///
/// ## Examples
///
/// ```ignore
/// use darkmatter::render::stylesheet::{CssSizing, CssSizingProp, Stylesheet};
///
/// let sheet = Stylesheet::new()
///     .add(CssSizingProp::Width, CssSizing::px(320.0))
///     .add(CssSizingProp::TopMargin, CssSizing::rem(1.5));
/// ```
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

/// Lifts a typed sizing property into the runtime [`CssProp`] enum used by
/// [`Stylesheet`] internally.
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

/// Compile-time-checked subset of [`CssProp`] for shorthand sizing
/// properties that accept 1–4 [`CssSizing`] values.
///
/// Use with [`Stylesheet::add`] to pair these with a [`CssSizingMulti`].
///
/// ## Examples
///
/// ```ignore
/// use darkmatter::render::stylesheet::{
///     CssSizing, CssSizingMulti, CssSizingMultiProp, Stylesheet,
/// };
///
/// let sheet = Stylesheet::new().add(
///     CssSizingMultiProp::Padding,
///     CssSizingMulti::from((CssSizing::px(8.0), CssSizing::px(16.0))),
/// );
/// assert_eq!(sheet.to_css(), "padding: 8px 16px;");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CssSizingMultiProp {
    /// `margin`
    Margin,
    /// `padding`
    Padding,
    /// `border-radius`
    BorderRadius,
}

/// Lifts a typed shorthand-sizing property into the runtime [`CssProp`] enum.
impl From<CssSizingMultiProp> for CssProp {
    fn from(value: CssSizingMultiProp) -> Self {
        match value {
            CssSizingMultiProp::Margin => Self::Margin,
            CssSizingMultiProp::Padding => Self::Padding,
            CssSizingMultiProp::BorderRadius => Self::BorderRadius,
        }
    }
}

/// Compile-time-checked subset of [`CssProp`] for properties that accept a
/// [`CssColor`] value.
///
/// ## Examples
///
/// ```ignore
/// use darkmatter::render::stylesheet::{CssColor, CssColorProp, Stylesheet};
///
/// let sheet = Stylesheet::new()
///     .add(CssColorProp::Color, CssColor::hex("#336699").unwrap())
///     .add(CssColorProp::BackgroundColor, CssColor::rgb(255, 255, 255));
/// ```
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

/// Lifts a typed color property into the runtime [`CssProp`] enum.
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

/// Compile-time-checked subset of [`CssProp`] for properties that accept a
/// signed integer ([`i32`]) value.
///
/// ## Examples
///
/// ```ignore
/// use darkmatter::render::stylesheet::{CssIntegerProp, Stylesheet};
///
/// let sheet = Stylesheet::new()
///     .add(CssIntegerProp::ZIndex, 10)
///     .add(CssIntegerProp::Order, -1);
/// ```
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

/// Lifts a typed integer property into the runtime [`CssProp`] enum.
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

/// Validated newtype wrapper around a custom CSS property name.
///
/// Custom properties (the CSS variable mechanism) and any other property not
/// covered by the enumerated [`CssProp`] variants flow through this wrapper.
/// Construction is fallible: the name must satisfy
/// `is_valid_property_name` — i.e. begin with `--` followed by an identifier
/// (letters, digits, hyphens, underscores) or follow the standard CSS
/// identifier rules.
///
/// When paired with a value via [`CssTypedProperty`] the accepted type is
/// [`CssRaw`] — custom properties have no built-in value classification.
///
/// ## Examples
///
/// ```ignore
/// use darkmatter::render::stylesheet::{CssCustomProp, CssRaw, Stylesheet};
///
/// let prop = CssCustomProp::new("--brand").unwrap();
/// let value = CssRaw::new("rgb(51, 102, 153)").unwrap();
///
/// let sheet = Stylesheet::new().add(prop, value);
/// assert_eq!(sheet.to_css(), "--brand: rgb(51, 102, 153);");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CssCustomProp(String);

impl CssCustomProp {
    /// Creates a validated custom CSS property wrapper.
    ///
    /// ## Errors
    ///
    /// Returns [`StylesheetError::InvalidCustomProperty`] when the supplied
    /// name fails `is_valid_property_name`. Despite the variant name, this
    /// constructor accepts *any* valid CSS property name, not only custom
    /// (`--`-prefixed) ones — but the typical caller passes a `--prefixed`
    /// identifier.
    pub fn new(name: impl Into<String>) -> Result<Self, StylesheetError> {
        let name = name.into();
        if !is_valid_property_name(&name) {
            return Err(StylesheetError::InvalidCustomProperty { name });
        }
        Ok(Self(name))
    }

    /// Returns the wrapped property name as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Validating conversion from a string slice.
///
/// Equivalent to [`CssCustomProp::new`] — see its docs for the validation
/// rules and error cases.
impl TryFrom<&str> for CssCustomProp {
    type Error = StylesheetError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

/// Validating conversion from an owned [`String`]; see [`CssCustomProp::new`].
impl TryFrom<String> for CssCustomProp {
    type Error = StylesheetError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

/// Lifts a custom property into the runtime [`CssProp`] enum as
/// [`CssProp::Other`].
impl From<CssCustomProp> for CssProp {
    fn from(value: CssCustomProp) -> Self {
        Self::Other(value.0)
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
/// ```ignore
/// use darkmatter::render::stylesheet::{CssSizing, CssUnit};
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
/// ```ignore
/// use darkmatter::render::stylesheet::{CssSizing, CssUnit};
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
/// ```ignore
/// use darkmatter::render::stylesheet::{CssSizing, CssSizingMulti};
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
/// ```ignore
/// use darkmatter::render::stylesheet::CssRaw;
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
/// [`Stylesheet`].
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
/// strongly-typed value types; this is what [`Stylesheet::add`] uses
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
    /// [`Stylesheet::add`] to verify that a value matches its property's
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
/// [`Stylesheet::add`] call, plus a passthrough impl for [`CssValue`] itself.
///
/// This trait exists to make [`Stylesheet::add`] generic without requiring
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

/// Pairs a typed property type with the strongly-typed value it accepts.
///
/// This is the trait that makes [`Stylesheet::add`] compile-time-checked: the
/// signature
///
/// ```ignore
/// pub fn add<T, U>(self, prop: T, value: U) -> Self
/// where
///     T: CssTypedProperty<Value = U>,
///     U: IntoCssValue,
/// { /* ... */ }
/// ```
///
/// constrains `U` to be exactly the `Value` associated type of `T`. The
/// implementations below define the legal pairings:
///
/// | Typed property         | Associated value type |
/// |------------------------|-----------------------|
/// | [`CssSizingProp`]      | [`CssSizing`]         |
/// | [`CssSizingMultiProp`] | [`CssSizingMulti`]    |
/// | [`CssColorProp`]       | [`CssColor`]          |
/// | [`CssIntegerProp`]     | [`i32`]               |
/// | [`CssCustomProp`]      | [`CssRaw`]            |
pub trait CssTypedProperty {
    /// The strongly-typed value that may be paired with this property.
    type Value: IntoCssValue;

    /// Converts the typed property into the runtime [`CssProp`] enum.
    fn into_css_prop(self) -> CssProp;
}

/// Pairs [`CssSizingProp`] with [`CssSizing`].
impl CssTypedProperty for CssSizingProp {
    type Value = CssSizing;

    fn into_css_prop(self) -> CssProp {
        self.into()
    }
}

/// Pairs [`CssSizingMultiProp`] with [`CssSizingMulti`].
impl CssTypedProperty for CssSizingMultiProp {
    type Value = CssSizingMulti;

    fn into_css_prop(self) -> CssProp {
        self.into()
    }
}

/// Pairs [`CssColorProp`] with [`CssColor`].
impl CssTypedProperty for CssColorProp {
    type Value = CssColor;

    fn into_css_prop(self) -> CssProp {
        self.into()
    }
}

/// Pairs [`CssIntegerProp`] with [`i32`].
impl CssTypedProperty for CssIntegerProp {
    type Value = i32;

    fn into_css_prop(self) -> CssProp {
        self.into()
    }
}

/// Pairs [`CssCustomProp`] with [`CssRaw`]. Custom properties have no
/// known value classification, so they accept any validated raw value.
impl CssTypedProperty for CssCustomProp {
    type Value = CssRaw;

    fn into_css_prop(self) -> CssProp {
        self.into()
    }
}

/// Internal storage unit for [`Stylesheet`]: a single
/// `property: value` pair preserved in insertion order.
#[derive(Debug, Clone, PartialEq)]
struct CssDeclaration {
    prop: CssProp,
    value: CssValue,
}

/// Type-safe CSS declaration container.
///
/// A `Stylesheet` is an ordered list of `property: value` declarations.
/// Insertion preserves order; re-adding an existing property replaces the
/// previous value in place without changing position. There is no concept of
/// selectors or rules — this is a flat declaration block, suitable for use as
/// an inline style attribute or as the body of a single rule.
///
/// ## Construction modes
///
/// - **Typed (preferred)** — chain [`Stylesheet::add`] calls with a typed
///   property and its matching value type. Mistypes are compile-time errors.
/// - **Dynamic** — use [`Stylesheet::try_add`] with [`CssProp`] + [`CssValue`]
///   for input whose property/value pairing isn't known at compile time.
/// - **Parsing** — [`Stylesheet::try_from`] accepts a CSS-like declaration
///   block (optionally wrapped in `{ }`).
///
/// ## Rendering targets
///
/// | Method                                | Output                              |
/// |---------------------------------------|-------------------------------------|
/// | [`to_css`](Self::to_css) / [`Display`] | Newline-joined `property: value;` declarations |
/// | [`to_terminal_string`](Self::to_terminal_string) | ANSI-styled declarations for a CLI |
/// | [`to_terminal`](Self::to_terminal)    | Prints styled output to stdout      |
/// | [`to_json`](Self::to_json)            | Pretty JSON object (integers preserved) |
/// | [`to_json5`](Self::to_json5)          | JSON5 with unquoted keys + trailing commas |
///
/// ## Examples
///
/// ```ignore
/// use darkmatter::render::stylesheet::{CssColor, CssColorProp, CssSizing, CssSizingProp, Stylesheet};
///
/// let sheet = Stylesheet::new()
///     .add(CssSizingProp::Width, CssSizing::px(320.0))
///     .add(CssColorProp::Color, CssColor::hex("#336699").unwrap());
///
/// assert_eq!(sheet.len(), 2);
/// assert!(sheet.to_css().contains("width: 320px"));
/// ```
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Stylesheet {
    declarations: Vec<CssDeclaration>,
}

impl Stylesheet {
    /// Creates an empty stylesheet with no declarations.
    ///
    /// Equivalent to [`Stylesheet::default`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the number of declarations currently stored.
    pub fn len(&self) -> usize {
        self.declarations.len()
    }

    /// Returns `true` when the stylesheet has no declarations.
    pub fn is_empty(&self) -> bool {
        self.declarations.is_empty()
    }

    /// Adds (or replaces) a typed property/value declaration and returns
    /// `self` for chaining.
    ///
    /// The signature is constrained by [`CssTypedProperty`]: each typed
    /// property subset (e.g. [`CssSizingProp`], [`CssColorProp`]) only
    /// accepts its matching value type. Trying to pair, say, a
    /// [`CssSizingProp`] with a [`CssColor`] is a compile-time error.
    ///
    /// If a declaration for the same [`CssProp`] already exists, its value is
    /// updated in place (insertion order is preserved). New properties are
    /// appended.
    ///
    /// ## Panics
    ///
    /// Panics in debug builds via `debug_assert!` if the value's kind does
    /// not match the property's [`CssProp::expected_kind`]. In release
    /// builds the assertion is elided; the type system already prevents the
    /// mismatch for typed properties, so this assertion exists only as
    /// defense-in-depth against bugs in [`CssTypedProperty`] impls.
    ///
    /// ## Examples
    ///
    /// ```ignore
    /// use darkmatter::render::stylesheet::{CssIntegerProp, Stylesheet};
    ///
    /// let sheet = Stylesheet::new()
    ///     .add(CssIntegerProp::ZIndex, 1)
    ///     .add(CssIntegerProp::ZIndex, 10); // replaces in place
    /// assert_eq!(sheet.to_css(), "z-index: 10;");
    /// ```
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

    /// Adds (or replaces) a dynamic property/value declaration with runtime
    /// validation.
    ///
    /// Use this when working with already-erased [`CssProp`] / [`CssValue`]
    /// values — typically when parsing user input or operating on data of
    /// unknown shape. For statically known pairings, [`Stylesheet::add`] is
    /// stricter and infallible.
    ///
    /// Replacement semantics are identical to [`Stylesheet::add`].
    ///
    /// ## Errors
    ///
    /// Returns [`StylesheetError::PropertyValueTypeMismatch`] when the
    /// property has a known [`CssProp::expected_kind`] that disagrees with
    /// the value's [`CssValue::kind`]. Properties whose `expected_kind`
    /// is `None` (i.e. [`CssProp::Other`]) accept any value.
    pub fn try_add(mut self, prop: CssProp, value: CssValue) -> Result<Self, StylesheetError> {
        ensure_valid_property_value_pair(&prop, &value)?;
        self.insert_or_replace(prop, value);
        Ok(self)
    }

    /// Renders the stylesheet as plain CSS declarations.
    ///
    /// Each declaration is emitted on its own line in `property: value;`
    /// form, joined by `\n` (no leading or trailing whitespace). Returns an
    /// empty string when the stylesheet has no declarations.
    ///
    /// ## Examples
    ///
    /// ```ignore
    /// use darkmatter::render::stylesheet::{CssIntegerProp, CssSizing, CssSizingProp, Stylesheet};
    ///
    /// let sheet = Stylesheet::new()
    ///     .add(CssSizingProp::Width, CssSizing::px(80.0))
    ///     .add(CssIntegerProp::ZIndex, 5);
    /// assert_eq!(sheet.to_css(), "width: 80px;\nz-index: 5;");
    /// ```
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

    /// Renders the stylesheet with ANSI styling and writes it to stdout.
    ///
    /// Equivalent to calling [`Stylesheet::to_terminal_string`] and printing
    /// the result via `println!`. When the stylesheet is empty, nothing is
    /// printed (no trailing newline either).
    ///
    /// ## Notes
    ///
    /// This is a convenience wrapper for CLI tooling. Library code that
    /// needs to capture or compose the rendered output should call
    /// [`Stylesheet::to_terminal_string`] directly.
    pub fn to_terminal(&self, terminal: &Terminal) {
        let output = self.to_terminal_string(terminal);
        if !output.is_empty() {
            println!("{output}");
        }
    }

    /// Renders the stylesheet for terminal output, with ANSI styling only
    /// when the target is a TTY.
    ///
    /// When `terminal.is_tty` is `false`, the output matches
    /// [`Stylesheet::to_css`] verbatim. When it is `true`, each declaration
    /// is colorized by category:
    ///
    /// | Element            | Style                          |
    /// |--------------------|--------------------------------|
    /// | Property name      | bold blue (`rgb 97,175,239`)   |
    /// | `:` and `;`        | gray (`rgb 160,160,160`)       |
    /// | Sizing values      | teal (`rgb 86,182,194`)        |
    /// | Color values       | green (`rgb 152,195,121`)      |
    /// | Integer values     | amber (`rgb 229,192,123`)      |
    /// | Raw values         | light gray (`rgb 220,220,220`) |
    ///
    /// Values are routed through `prose_style_text` with a placeholder so
    /// that user content is never interpreted as Prose markup.
    ///
    /// ## Returns
    ///
    /// The styled (or plain) declaration block, joined by `\n`. Returns an
    /// empty string when the stylesheet has no declarations.
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

    /// Renders declarations as pretty-printed JSON.
    ///
    /// The output is a JSON object whose keys are CSS property names and
    /// whose values are:
    ///
    /// - JSON numbers for [`CssValue::Integer`] declarations.
    /// - JSON strings (the [`Display`] form of the value) for everything else.
    ///
    /// Declaration order is preserved by [`serde_json::Map`] when the
    /// `preserve_order` feature is enabled in `serde_json`; otherwise key
    /// order follows that implementation's default. Returns `"{}"` for an
    /// empty stylesheet (and as a fallback if serialization fails — which
    /// is unreachable in practice since all values stringify cleanly).
    ///
    /// ## Examples
    ///
    /// ```ignore
    /// use darkmatter::render::stylesheet::{CssIntegerProp, CssSizing, CssSizingProp, Stylesheet};
    ///
    /// let sheet = Stylesheet::new()
    ///     .add(CssSizingProp::Width, CssSizing::rem(42.0))
    ///     .add(CssIntegerProp::ZIndex, 9);
    /// let json = sheet.to_json();
    /// assert!(json.contains("\"width\": \"42rem\""));
    /// assert!(json.contains("\"z-index\": 9"));
    /// ```
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
    /// JSON5 differs from JSON in two ways here:
    ///
    /// - Keys are emitted **unquoted** when they pass `is_json5_identifier`
    ///   (e.g. `width`, `zIndex`). Keys containing hyphens or other
    ///   characters that disqualify them as identifiers are JSON-quoted
    ///   (e.g. `"z-index"`).
    /// - Each declaration ends with a **trailing comma** (legal in JSON5).
    ///
    /// Value typing matches [`Stylesheet::to_json`]: integers are emitted as
    /// JSON numbers, everything else is emitted as a quoted string.
    ///
    /// Returns `"{}"` for an empty stylesheet.
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

    /// Inserts a new declaration or, if a declaration with the same
    /// [`CssProp`] already exists, replaces its value in place.
    ///
    /// Preserves insertion order: replacing a value keeps the declaration's
    /// position; only genuinely new properties append to the end.
    fn insert_or_replace(&mut self, prop: CssProp, value: CssValue) {
        if let Some(existing) = self.declarations.iter_mut().find(|decl| decl.prop == prop) {
            existing.value = value;
            return;
        }

        self.declarations.push(CssDeclaration { prop, value });
    }
}

/// Delegates to [`Stylesheet::to_css`]. Use `{stylesheet}` in format strings
/// to emit the plain CSS form.
impl Display for Stylesheet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_css())
    }
}

/// Parses a stylesheet from a CSS-like declaration block.
///
/// Accepted shapes:
///
/// - A bare list of declarations: `"color: red; margin: 8px 16px;"`
/// - The same list wrapped in braces: `"{ color: red; margin: 8px 16px; }"`
///
/// Trailing semicolons are optional; declarations are separated by top-level
/// `;` (parenthesized content is preserved so `calc(...)` and `rgba(...)`
/// arguments are not split). Empty declarations between `;` characters are
/// skipped.
///
/// Each declaration is split on the first `:`. The property name is parsed
/// via [`CssProp::from_css_name`]; the value is parsed via
/// `parse_value_for_prop`, which dispatches based on the property's
/// [`CssProp::expected_kind`]. Properties with no known kind (i.e.
/// [`CssProp::Other`]) accept any non-empty raw value not containing `;`.
///
/// ## Errors
///
/// Bubbles up the first [`StylesheetError`] returned by any of:
///
/// - [`CssProp::from_css_name`] (invalid property name)
/// - the per-kind value parser (`CssSizing::try_from`, `CssColor::try_from`,
///   `i32::parse`, `CssRaw::new`, ...)
/// - [`Stylesheet::try_add`] (type mismatch, though this can only fire when
///   `parse_value_for_prop` returns a value of the wrong kind, which is
///   unreachable in normal use).
///
/// Returns [`StylesheetError::InvalidDeclaration`] when a declaration does
/// not contain `:`.
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

/// Borrowed-string convenience wrapper over [`TryFrom<&str> for Stylesheet`].
impl TryFrom<&String> for Stylesheet {
    type Error = StylesheetError;

    fn try_from(value: &String) -> Result<Self, Self::Error> {
        Self::try_from(value.as_str())
    }
}

/// Owned-string convenience wrapper over [`TryFrom<&str> for Stylesheet`].
impl TryFrom<String> for Stylesheet {
    type Error = StylesheetError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::try_from(value.as_str())
    }
}

/// Parses a value string into the [`CssValue`] variant expected by `prop`.
///
/// Dispatches on [`CssProp::expected_kind`]:
/// - `Sizing` → [`CssSizing::try_from`]
/// - `SizingMulti` → [`CssSizingMulti::try_from`]
/// - `Color` → [`CssColor::try_from`]
/// - `Integer` → [`i32`]'s [`std::str::FromStr`] impl (wrapped in [`StylesheetError::InvalidInteger`])
/// - `Raw` or `None` (unknown property) → [`CssRaw::new`]
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

/// Validates that a value's [`CssValueKind`] matches its property's expected
/// kind, returning [`StylesheetError::PropertyValueTypeMismatch`] on conflict.
///
/// Properties with no known kind (i.e. [`CssProp::Other`]) accept any value.
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

/// Boolean form of `ensure_valid_property_value_pair` used by the
/// debug-time assertion in [`Stylesheet::add`].
fn is_value_allowed_for_prop(prop: &CssProp, value: &CssValue) -> bool {
    match prop.expected_kind() {
        Some(expected) => expected == value.kind(),
        None => true,
    }
}

/// Maps a [`CssValue`] to a [`serde_json::Value`] for [`Stylesheet::to_json`].
/// Integers become JSON numbers; all other variants stringify via [`Display`].
fn json_value_for_css_value(value: &CssValue) -> Value {
    match value {
        CssValue::Integer(number) => Value::from(*number),
        _ => Value::from(value.to_string()),
    }
}

/// Emits a JSON5 object key: unquoted if it qualifies as an identifier per
/// `is_json5_identifier`, otherwise JSON-quoted.
fn json5_key(key: &str) -> String {
    if is_json5_identifier(key) {
        key.to_string()
    } else {
        json_quote(key)
    }
}

/// Emits a JSON5 value: bare number for [`CssValue::Integer`], quoted string
/// for everything else.
fn json5_value(value: &CssValue) -> String {
    match value {
        CssValue::Integer(number) => number.to_string(),
        _ => json_quote(&value.to_string()),
    }
}

/// JSON-escapes `value` and surrounds it with `"..."`.
///
/// Uses [`serde_json`] for proper Unicode escaping. The fallback branch
/// (manual escape of `\` and `"`) is unreachable in practice but provides a
/// safe default if `serde_json` ever rejects the input.
fn json_quote(value: &str) -> String {
    match serde_json::to_string(value) {
        Ok(serialized) => serialized,
        Err(_) => {
            let escaped = value.replace('\\', "\\\\").replace('"', "\\\"");
            format!("\"{escaped}\"")
        }
    }
}

/// Returns `true` when `value` matches the JSON5 identifier grammar:
/// starts with `_`, `$`, or an ASCII letter, followed by any number of
/// `_`/`$`/ASCII alphanumerics. CSS property names with hyphens (`z-index`,
/// `margin-top`) return `false` and must be quoted in JSON5 output.
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

/// Validates a CSS property name against a simplified CSS identifier grammar.
///
/// Two shapes are accepted:
///
/// 1. **Custom property**: starts with `--`, followed by a non-empty body of
///    ASCII alphanumerics, `-`, or `_`.
/// 2. **Standard property**: starts with an ASCII letter (or a single `-`
///    followed by an ASCII letter, for vendor prefixes), followed by ASCII
///    alphanumerics or `-` (no `_`).
///
/// Used by [`CssProp::from_css_name`] and [`CssCustomProp::new`].
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

/// One-character CSS identifier predicate. Underscores are only legal inside
/// custom (`--`-prefixed) property bodies.
fn is_css_ident_char(ch: char, allow_underscore: bool) -> bool {
    ch.is_ascii_alphanumeric() || ch == '-' || (allow_underscore && ch == '_')
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

/// Splits a CSS declaration block on top-level `;` characters.
///
/// Tracks parenthesis depth so that `;` inside `calc(...)`, `rgba(...)`, etc.
/// are not treated as declaration separators (CSS does not actually allow
/// `;` inside function arguments, but this defensive behavior keeps malformed
/// input from producing spurious extra tokens). Empty segments are skipped.
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
/// the trimmed input unchanged. Lets [`TryFrom<&str> for Stylesheet`] accept
/// both bare declaration lists and brace-wrapped blocks.
fn strip_surrounding_braces(input: &str) -> &str {
    let trimmed = input.trim();
    if trimmed.starts_with('{') && trimmed.ends_with('}') && trimmed.len() >= 2 {
        &trimmed[1..trimmed.len() - 1]
    } else {
        trimmed
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

/// Renders `text` inside a [`Prose`] template without letting Prose interpret
/// any of `text`'s characters as inline markdown markup.
///
/// `template` must contain a literal `{text}` token. The token is replaced
/// with a unique placeholder, Prose renders the resulting source through the
/// terminal-aware styler, and the placeholder is swapped back to the original
/// `text` in the final string. This keeps CSS values like `_my-token_` or
/// `**important**` from being mangled by Prose's markdown subset.
fn prose_style_text(term: &Terminal, template: &str, text: &str) -> String {
    let placeholder = unique_placeholder(text);
    let source = template.replace("{text}", &placeholder);
    let rendered = Prose::new(source).render(term);
    rendered.replace(&placeholder, text)
}

/// Generates a placeholder string guaranteed not to appear in `text`.
///
/// Avoids characters [`Prose`] interprets as inline markdown (notably `_`,
/// which pairs as italics/bold). The placeholder must survive
/// [`Prose::render`] unchanged so the caller can swap it for the original
/// `text` afterwards. Numeric collisions are avoided by incrementing `idx`
/// until the candidate is absent from `text`.
fn unique_placeholder(text: &str) -> String {
    let mut idx = 0usize;
    loop {
        let candidate = format!("XDMSTYLESHEETTEXT{idx}XEND");
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
