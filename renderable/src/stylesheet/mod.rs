//! Type-safe CSS stylesheet declarations.
//!
//! This module provides a strongly-typed builder for CSS declarations that is
//! independent of any single render target. A [`CssStyle`] is a list of
//! `property: value` pairs that can be emitted as:
//!
//! - Plain CSS text (via [`CssStyle::to_css`] or [`Display`](std::fmt::Display))
//! - Pretty-printed JSON (via [`CssStyle::to_json`])
//! - JSON5 with unquoted keys and trailing commas (via [`CssStyle::to_json5`])
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
//!    its accepted value type, allowing [`CssStyle::add`] to be compile-time
//!    checked: passing a `CssColor` to a `CssSizingProp` is a type error.
//!
//! For dynamic input (e.g. parsing from a string), [`CssStyle::try_add`] and
//! [`CssStyle::try_from`] perform runtime validation and return
//! [`StylesheetError`] on failure.
//!
//! ## Examples
//!
//! Building a style with compile-time checked typed properties:
//!
//! ```rust
//! use renderable::stylesheet::{
//!     CssColor, CssColorProp, CssIntegerProp, CssSizing, CssSizingProp, CssStyle,
//! };
//!
//! let style = CssStyle::new()
//!     .add(CssSizingProp::Width, CssSizing::px(320.0))
//!     .add(CssColorProp::Color, CssColor::rgb(0x33, 0x66, 0x99))
//!     .add(CssIntegerProp::ZIndex, 10);
//!
//! assert_eq!(
//!     style.to_css(),
//!     "width: 320px;\ncolor: rgb(51, 102, 153);\nz-index: 10;"
//! );
//! ```
//!
//! Parsing from a CSS-like string:
//!
//! ```rust
//! use renderable::stylesheet::CssStyle;
//!
//! let style = CssStyle::try_from("color: #336699; margin: 8px 16px;").unwrap();
//! assert_eq!(style.len(), 2);
//! ```

pub mod error;
pub mod prop;
pub mod rule;
pub mod sheet;
pub mod style;
pub mod value;

pub use error::StylesheetError;
pub use prop::{
    CssColorProp, CssCustomProp, CssIntegerProp, CssProp, CssSizingMultiProp, CssSizingProp,
    CssTypedProperty,
};
pub use rule::CssRule;
pub use sheet::Stylesheet;
pub use style::CssStyle;
pub use value::{
    CssColor, CssRaw, CssSizing, CssSizingMulti, CssUnit, CssValue, CssValueKind, IntoCssValue,
};
