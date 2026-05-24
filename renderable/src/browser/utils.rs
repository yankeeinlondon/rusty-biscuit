//! Helpers for component authors. Scope is narrow: each helper either
//! prevents a correctness bug (escaping) or removes boilerplate that
//! recurs across components.

use std::borrow::Cow;

/// Escapes the three characters that change parser state inside element
/// content: `&`, `<`, `>`.
///
/// Returns the input borrowed when no escaping is needed — the common
/// case allocates nothing.
pub fn escape_text(input: &str) -> Cow<'_, str> {
    if input.bytes().any(|b| matches!(b, b'&' | b'<' | b'>')) {
        let mut out = String::with_capacity(input.len() + 8);
        for ch in input.chars() {
            match ch {
                '&' => out.push_str("&amp;"),
                '<' => out.push_str("&lt;"),
                '>' => out.push_str("&gt;"),
                other => out.push(other),
            }
        }
        Cow::Owned(out)
    } else {
        Cow::Borrowed(input)
    }
}

/// Escapes the four characters relevant inside a double-quoted attribute
/// value: `&`, `<`, `"`, `'`. `>` is intentionally left alone.
pub fn escape_attribute(input: &str) -> Cow<'_, str> {
    if input
        .bytes()
        .any(|b| matches!(b, b'&' | b'<' | b'"' | b'\''))
    {
        let mut out = String::with_capacity(input.len() + 8);
        for ch in input.chars() {
            match ch {
                '&' => out.push_str("&amp;"),
                '<' => out.push_str("&lt;"),
                '"' => out.push_str("&quot;"),
                '\'' => out.push_str("&#39;"),
                other => out.push(other),
            }
        }
        Cow::Owned(out)
    } else {
        Cow::Borrowed(input)
    }
}

/// Joins class-name parts with a single space. `None` and parts that are
/// empty after trimming are dropped. Inner whitespace is preserved.
pub fn classes<I, S>(parts: I) -> String
where
    I: IntoIterator<Item = Option<S>>,
    S: AsRef<str>,
{
    let mut out = String::new();
    for part in parts {
        let Some(part) = part else { continue };
        let trimmed = part.as_ref().trim();
        if trimmed.is_empty() {
            continue;
        }
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(trimmed);
    }
    out
}

/// Converts an arbitrary string into a CSS-safe identifier segment.
///
/// Rules: lowercase ASCII letters/digits pass through; every other
/// character becomes `-`; consecutive `-` collapse; leading/trailing `-`
/// trim; a leading digit gets a `_` prefix; an empty result becomes `_`.
/// Unicode normalization is intentionally not performed — the rusty
/// biscuit workspace targets ASCII identifiers.
pub fn css_slug(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut last_was_dash = false;
    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            last_was_dash = false;
        } else if !last_was_dash && !out.is_empty() {
            out.push('-');
            last_was_dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    if out.is_empty() {
        return "_".to_string();
    }
    if out.starts_with(|c: char| c.is_ascii_digit()) {
        out.insert(0, '_');
    }
    out
}

/// Returns a `var(--name)` CSS variable reference. The leading `--` is
/// added by the helper; pass the bare name.
pub fn css_var(name: &str) -> String {
    format!("var(--{name})")
}

/// Returns a `var(--name, fallback)` CSS variable reference. The fallback
/// is emitted verbatim — not escaped or quoted.
pub fn css_var_with_fallback(name: &str, fallback: &str) -> String {
    format!("var(--{name}, {fallback})")
}

/// An inline-style builder. Stores declarations as an ordered `Vec` so
/// declaration order is preserved.
#[derive(Debug, Default, Clone)]
pub struct Style {
    declarations: Vec<(String, String)>,
}

impl Style {
    /// An empty style builder.
    pub fn new() -> Self {
        Style::default()
    }

    /// Set a `property: value` declaration.
    pub fn set(mut self, property: impl Into<String>, value: impl Into<String>) -> Self {
        self.declarations.push((property.into(), value.into()));
        self
    }

    /// Set a declaration only when `value` is `Some`.
    pub fn set_opt(
        mut self,
        property: impl Into<String>,
        value: Option<impl Into<String>>,
    ) -> Self {
        if let Some(value) = value {
            self.declarations.push((property.into(), value.into()));
        }
        self
    }

    /// Serializes to a `style=""` attribute value. Values are emitted
    /// raw — the rendering layer escapes on emit.
    pub fn to_css_string(&self) -> String {
        self.declarations
            .iter()
            .map(|(prop, value)| format!("{prop}: {value}"))
            .collect::<Vec<_>>()
            .join("; ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_text_handles_the_three_content_characters() {
        assert_eq!(escape_text("a & b < c > d"), "a &amp; b &lt; c &gt; d");
        assert!(matches!(escape_text("plain"), Cow::Borrowed("plain")));
    }

    #[test]
    fn escape_attribute_handles_quotes_but_not_gt() {
        assert_eq!(
            escape_attribute(r#"he said "hi" > 'bye'"#),
            "he said &quot;hi&quot; > &#39;bye&#39;"
        );
    }

    #[test]
    fn classes_filters_none_and_blank() {
        let out = classes([Some("a"), None, Some("  "), Some(" b ")]);
        assert_eq!(out, "a b");
    }

    #[test]
    fn css_slug_follows_the_rules() {
        assert_eq!(css_slug("Order ID"), "order-id");
        assert_eq!(css_slug("2024 totals"), "_2024-totals");
        assert_eq!(css_slug("---"), "_");
        assert_eq!(css_slug(css_slug("Order ID").as_str()), "order-id");
    }

    #[test]
    fn css_var_forms() {
        assert_eq!(css_var("brand-fg"), "var(--brand-fg)");
        assert_eq!(
            css_var_with_fallback("brand-border", "#ccc"),
            "var(--brand-border, #ccc)"
        );
    }

    #[test]
    fn style_builder_preserves_order_and_skips_none() {
        let style = Style::new()
            .set("color", "red")
            .set_opt("background", None::<String>)
            .set("padding", "1rem");
        assert_eq!(style.to_css_string(), "color: red; padding: 1rem");
    }
}
