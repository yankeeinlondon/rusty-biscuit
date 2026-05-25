//! CSS stylesheet collection.
//!
//! A [`Stylesheet`] is an ordered collection of [`CssRule`] pairs
//! (selector + declaration block). It replaces the former `HtmlStyleSheet`
//! placeholder from `browser/mod.rs`.

use crate::stylesheet::rule::CssRule;

/// A collection of CSS rulesets keyed by **selector** in declaration order.
///
/// Each entry is a [`CssRule`] containing a `(selector, style)` pair. The
/// order is preserved because CSS cascade resolves ties in source order, and
/// silently dropping duplicate selectors would break the override model that
/// lets later rules win.
///
/// ## Notes
///
/// - Selectors are stored as strings rather than a typed selector model;
///   the rendering layer is responsible for emitting them verbatim.
/// - For class-scoped collections produced by component stylesheets,
///   selectors are descendant selectors like `.simple-table .col-string`.
/// - Page assembly may dedup or merge entries with identical selectors,
///   but this type does not enforce it.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Stylesheet {
    rules: Vec<CssRule>,
}

impl Stylesheet {
    /// Creates an empty stylesheet.
    pub fn new() -> Self {
        Self::default()
    }

    /// Append a [`CssRule`] entry, preserving order.
    pub fn push(&mut self, rule: CssRule) -> &mut Self {
        self.rules.push(rule);
        self
    }

    /// Returns the rules as a slice.
    pub fn entries(&self) -> &[CssRule] {
        &self.rules
    }

    /// Returns the number of rules.
    pub fn len(&self) -> usize {
        self.rules.len()
    }

    /// Returns `true` when the stylesheet has no rules.
    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stylesheet::style::CssStyle;

    #[test]
    fn stylesheet_collection_holds_css_rules() {
        let mut sheet = Stylesheet::new();
        sheet.push(CssRule::new(".a", CssStyle::new()));
        sheet.push(CssRule::new(".b", CssStyle::new()));
        let entries = sheet.entries();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].selector, ".a");
        assert_eq!(entries[1].selector, ".b");
    }
}
