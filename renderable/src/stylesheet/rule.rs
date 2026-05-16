//! CSS rule type: a selector paired with a declaration block.
//!
//! A [`CssRule`] is the building block of a
//! [`Stylesheet`](crate::stylesheet::Stylesheet) collection.
//! It pairs a CSS selector string with a [`CssStyle`] declaration block.

use crate::stylesheet::style::CssStyle;

/// A single CSS rule: a selector paired with a declaration block.
///
/// This is the `(selector, block)` pair from Scheme A naming. Rules are
/// collected into a [`Stylesheet`](crate::stylesheet::Stylesheet) to form a
/// complete CSS stylesheet.
#[derive(Debug, Clone, PartialEq)]
pub struct CssRule {
    /// The CSS selector (e.g. `".my-class"`, `"#my-id"`, `"div > span"`).
    pub selector: String,
    /// The declaration block for this rule.
    pub style: CssStyle,
}

impl CssRule {
    /// Creates a new CSS rule.
    pub fn new(selector: impl Into<String>, style: CssStyle) -> Self {
        Self {
            selector: selector.into(),
            style,
        }
    }
}
