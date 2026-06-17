//! Parse inline `key=value` style tokens from a `::disclosure` opener line.
//!
//! Recognized keys mirror the `style.disclosure.*` bucket: `width`,
//! `max-width` (and `max_width`), `alignment`, `color`, `bg-color` (and
//! `bg_color`). Tokens that do not match a recognized key/value pair are
//! treated as the start of the summary text.

use renderable::layout::{Layout, TargetValue, Width};
use renderable::tree::DisclosureStyleHints;

use crate::style::color::StyleColor;
use crate::style::schema::CommonStyle;

/// Lower a parsed [`CommonStyle`] into renderable [`DisclosureStyleHints`].
pub fn common_style_to_disclosure_hints(style: &CommonStyle) -> DisclosureStyleHints {
    let mut layout = Layout::default();
    let mut changed = false;

    if let Some(width) = style.width.as_ref() {
        layout.width = Width::Fixed(TargetValue::universal(width.clone()));
        changed = true;
    }

    if let Some(max_width) = style.max_width.as_ref() {
        layout.max_width = Some(TargetValue::universal(max_width.clone()));
        changed = true;
    }

    if let Some(alignment) = style.alignment {
        layout.alignment = alignment;
        changed = true;
    }

    let color = style.color.as_ref().map(StyleColor::to_paint_color);
    let bg_color = style.bg_color.as_ref().map(StyleColor::to_paint_color);

    DisclosureStyleHints {
        layout: if changed { Some(layout) } else { None },
        color,
        bg_color,
    }
}

/// Parse the text immediately following the `::disclosure` keyword.
///
/// Returns the parsed inline style and any remaining text that should be
/// treated as summary content. Whitespace-separated `key=value` tokens are
/// consumed from the left until a token is not a recognized style pair; that
/// token and everything after it becomes the summary remainder.
pub fn parse_disclosure_opener_style(rest: &str) -> (Option<CommonStyle>, Option<String>) {
    let mut style = CommonStyle::default();
    let tokens: Vec<&str> = rest.split_whitespace().collect();
    let mut consumed = 0usize;

    for token in &tokens {
        if let Some((key, value)) = token.split_once('=') {
            let key = key.trim();
            let value = value.trim();
            if value.is_empty() {
                break;
            }
            if try_apply_style_token(key, value, &mut style) {
                consumed += 1;
                continue;
            }
        }
        break;
    }

    let summary = if consumed == tokens.len() {
        None
    } else {
        let remainder = tokens[consumed..].join(" ");
        if remainder.is_empty() {
            None
        } else {
            Some(remainder)
        }
    };

    if style == CommonStyle::default() {
        (None, summary)
    } else {
        (Some(style), summary)
    }
}

fn try_apply_style_token(key: &str, value: &str, style: &mut CommonStyle) -> bool {
    match key {
        "width" => {
            if let Ok(len) = crate::style::length::parse_horizontal_typed(value) {
                style.width = Some(len);
                true
            } else {
                false
            }
        }
        "max-width" | "max_width" => {
            if let Ok(len) = crate::style::length::parse_horizontal_typed(value) {
                style.max_width = Some(len);
                true
            } else {
                false
            }
        }
        "alignment" => {
            if let Ok(align) = crate::style::alignment::parse(value) {
                style.alignment = Some(align);
                true
            } else {
                false
            }
        }
        "color" => {
            if let Ok(color) = crate::style::color::parse(value) {
                style.color = Some(color);
                true
            } else {
                false
            }
        }
        "bg-color" | "bg_color" => {
            if let Ok(color) = crate::style::color::parse(value) {
                style.bg_color = Some(color);
                true
            } else {
                false
            }
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use renderable::layout::{Alignment, Length};

    #[test]
    fn empty_rest_yields_no_style_no_summary() {
        let (style, summary) = parse_disclosure_opener_style("");
        assert!(style.is_none());
        assert!(summary.is_none());
    }

    #[test]
    fn plain_summary_text_yields_no_style() {
        let (style, summary) = parse_disclosure_opener_style("License Agreement");
        assert!(style.is_none());
        assert_eq!(summary.as_deref(), Some("License Agreement"));
    }

    #[test]
    fn single_param_parses() {
        let (style, summary) = parse_disclosure_opener_style("max-width=50ch");
        let style = style.expect("style parsed");
        assert_eq!(style.max_width, Some(Length::Ch(50)));
        assert!(summary.is_none());
    }

    #[test]
    fn multiple_params_and_summary() {
        let (style, summary) = parse_disclosure_opener_style("max-width=50ch color=red-500 License");
        let style = style.expect("style parsed");
        assert_eq!(style.max_width, Some(Length::Ch(50)));
        assert!(style.color.is_some());
        assert_eq!(summary.as_deref(), Some("License"));
    }

    #[test]
    fn snake_case_keys_accepted() {
        let (style, _) = parse_disclosure_opener_style("max_width=50ch bg_color=blue-500");
        let style = style.expect("style parsed");
        assert_eq!(style.max_width, Some(Length::Ch(50)));
        assert!(style.bg_color.is_some());
    }

    #[test]
    fn invalid_value_treated_as_summary() {
        let (style, summary) = parse_disclosure_opener_style("max-width=not-a-length Summary");
        assert!(style.is_none());
        assert_eq!(summary.as_deref(), Some("max-width=not-a-length Summary"));
    }

    #[test]
    fn alignment_value_parses() {
        let (style, _) = parse_disclosure_opener_style("alignment=center");
        let style = style.expect("style parsed");
        assert_eq!(style.alignment, Some(Alignment::Center));
    }
}
