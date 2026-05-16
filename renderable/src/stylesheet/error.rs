//! Errors returned when parsing or validating CSS declarations.
//!
//! This is the unified error type for every fallible operation in the
//! stylesheet module: parsing a stylesheet from a string, constructing a value
//! via a `try_*` constructor, or adding a dynamic property/value pair through
//! [`CssStyle::try_add`].

use thiserror::Error;

use crate::stylesheet::value::CssValueKind;

/// Errors returned when parsing or validating CSS declarations.
///
/// This is the unified error type for every fallible operation in this module:
/// parsing a stylesheet from a string, constructing a value via a `try_*`
/// constructor, or adding a dynamic property/value pair through
/// [`CssStyle::try_add`].
///
/// Each variant carries the offending input so callers can show the user
/// exactly what went wrong.
///
/// ## Examples
///
/// ```rust
/// use renderable::stylesheet::{CssStyle, StylesheetError};
///
/// let err = CssStyle::try_from("z-index: 1px;").unwrap_err();
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
    /// Raised by [`CssStyle::try_add`] and string parsing when, for example,
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
