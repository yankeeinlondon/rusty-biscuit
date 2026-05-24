//! CSS property types for the stylesheet subsystem.
//!
//! This module provides the typed property system used by
//! [`CssStyle`](crate::stylesheet::CssStyle),
//! including the runtime [`CssProp`] enum and compile-time-checked subsets
//! ([`CssSizingProp`], [`CssColorProp`], etc.) that pair with their accepted
//! value types via the [`CssTypedProperty`] trait.

use std::borrow::Cow;

use crate::stylesheet::error::StylesheetError;
use crate::stylesheet::value::{
    CssColor, CssRaw, CssSizing, CssSizingMulti, CssValueKind, IntoCssValue,
};

/// Enumerates known CSS properties plus a custom fallback.
///
/// `CssProp` is the runtime, type-erased property identifier used inside a
/// [`CssStyle`](crate::stylesheet::CssStyle). Each known variant corresponds to a fixed CSS name (see
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
    /// ```rust
    /// use renderable::stylesheet::CssProp;
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
    /// - Runtime validation in [`CssStyle::try_add`](crate::stylesheet::CssStyle::try_add) (via
    ///   `ensure_valid_property_value_pair`).
    /// - The debug-time invariant check in [`CssStyle::add`](crate::stylesheet::CssStyle::add).
    /// - Property-specific parsing dispatch in `parse_value_for_prop`.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use renderable::stylesheet::{CssProp, CssValueKind};
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
    /// ```rust
    /// use renderable::stylesheet::CssProp;
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
/// Use this with [`CssStyle::add`](crate::stylesheet::CssStyle::add) to get
/// static guarantees that the value
/// type matches the property — the alternative ([`CssProp`] +
/// [`CssValue`](crate::stylesheet::CssValue) via
/// [`CssStyle::try_add`](crate::stylesheet::CssStyle::try_add)) defers checking
/// to runtime.
///
/// All variants here lower to [`CssValueKind::Sizing`]; see [`CssProp`]'s
/// property-group table for the full list.
///
/// ## Examples
///
/// ```rust
/// use renderable::stylesheet::{CssSizing, CssSizingProp, CssStyle};
///
/// let sheet = CssStyle::new()
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
/// [`CssStyle`](crate::stylesheet::CssStyle) internally.
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
/// Use with [`CssStyle::add`](crate::stylesheet::CssStyle::add) to pair these
/// with a [`CssSizingMulti`].
///
/// ## Examples
///
/// ```rust
/// use renderable::stylesheet::{
///     CssSizing, CssSizingMulti, CssSizingMultiProp, CssStyle,
/// };
///
/// let sheet = CssStyle::new().add(
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
/// ```rust
/// use renderable::stylesheet::{CssColor, CssColorProp, CssStyle};
///
/// let sheet = CssStyle::new()
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
/// ```rust
/// use renderable::stylesheet::{CssIntegerProp, CssStyle};
///
/// let sheet = CssStyle::new()
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
/// ```rust
/// use renderable::stylesheet::{CssCustomProp, CssRaw, CssStyle};
///
/// let prop = CssCustomProp::new("--brand").unwrap();
/// let value = CssRaw::new("rgb(51, 102, 153)").unwrap();
///
/// let sheet = CssStyle::new().add(prop, value);
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

/// Pairs a typed property type with the strongly-typed value it accepts.
///
/// This is the trait that makes
/// [`CssStyle::add`](crate::stylesheet::CssStyle::add) compile-time-checked: the
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
pub(crate) fn is_valid_property_name(name: &str) -> bool {
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
pub(crate) fn is_css_ident_char(ch: char, allow_underscore: bool) -> bool {
    ch.is_ascii_alphanumeric() || ch == '-' || (allow_underscore && ch == '_')
}
