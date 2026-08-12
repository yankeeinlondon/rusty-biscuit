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
        layout.width = match width.as_width() {
            Width::Auto => Width::Auto,
            Width::FitContent => Width::FitContent,
            Width::Fixed(tv) => Width::Fixed(tv.clone()),
        };
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
    let emphasis = style.emphasis.as_ref().map(|e| e.to_text_emphasis());
    let border = style.border.as_ref().map(|b| b.as_border().clone());
    let word_wrap = style.word_wrap.map(|w| w.to_word_wrap());

    DisclosureStyleHints {
        layout: if changed { Some(layout) } else { None },
        color,
        bg_color,
        emphasis,
        border,
        word_wrap,
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

/// Returns the byte spans of the leading recognized `key=value` style tokens in
/// a `::disclosure` opener remainder, in source order.
///
/// Uses the same left-to-right consumption rule as
/// [`parse_disclosure_opener_style`]: whitespace-separated `key=value` tokens
/// are consumed until the first token that is not a recognized style pair.
/// Everything after the last returned span is summary text. Spans are byte
/// offsets into `rest`. This is the span-aware companion used by the read-only
/// disclosure scanner; it shares [`try_apply_style_token`] so the set of
/// recognized tokens can never drift from the render path.
pub fn disclosure_style_token_spans(rest: &str) -> Vec<std::ops::Range<usize>> {
    let mut spans = Vec::new();
    let mut style = CommonStyle::default();
    let bytes = rest.as_bytes();
    let mut idx = 0usize;

    loop {
        while idx < bytes.len() && bytes[idx].is_ascii_whitespace() {
            idx += 1;
        }
        if idx >= bytes.len() {
            break;
        }
        let start = idx;
        while idx < bytes.len() && !bytes[idx].is_ascii_whitespace() {
            idx += 1;
        }
        let end = idx;
        let token = &rest[start..end];

        let recognized = token
            .split_once('=')
            .map(|(key, value)| (key.trim(), value.trim()))
            .filter(|(_, value)| !value.is_empty())
            .is_some_and(|(key, value)| try_apply_style_token(key, value, &mut style));

        if recognized {
            spans.push(start..end);
        } else {
            break;
        }
    }

    spans
}

fn try_apply_style_token(key: &str, value: &str, style: &mut CommonStyle) -> bool {
    match key {
        "width" => {
            if let Ok(len) = crate::style::length::parse_horizontal_typed(value) {
                style.width = Some(crate::style::schema::common::WidthOrMode::Width(
                    Width::Fixed(TargetValue::universal(len)),
                ));
                true
            } else if matches!(value.trim().to_ascii_lowercase().as_str(), "auto" | "fit-content" | "fit_content") {
                let mode = match value.trim().to_ascii_lowercase().as_str() {
                    "auto" => Width::Auto,
                    "fit-content" | "fit_content" => Width::FitContent,
                    _ => unreachable!(),
                };
                style.width = Some(crate::style::schema::common::WidthOrMode::Width(mode));
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
        "margin" => {
            if let Ok(len) = crate::style::length::parse_horizontal_typed(value) {
                style.margin = Some(crate::style::schema::common::ComponentEdges(
                    renderable::layout::Edges::all(len),
                ));
                true
            } else {
                false
            }
        }
        "padding" => {
            if let Ok(len) = crate::style::length::parse_horizontal_typed(value) {
                style.padding = Some(crate::style::schema::common::ComponentEdges(
                    renderable::layout::Edges::all(len),
                ));
                true
            } else {
                false
            }
        }
        "word-wrap" | "word_wrap" => {
            let wrap = match value.trim().to_ascii_lowercase().as_str() {
                "none" => crate::style::schema::common::ComponentWordWrap::None,
                "wrap" | "wrap-prose" | "wrap_prose" => {
                    crate::style::schema::common::ComponentWordWrap::Wrap
                }
                "truncate" => crate::style::schema::common::ComponentWordWrap::Truncate,
                _ => return false,
            };
            style.word_wrap = Some(wrap);
            true
        }
        "border" => {
            let border = match value.trim().to_ascii_lowercase().as_str() {
                "true" | "thin" => crate::style::schema::common::ComponentBorder(
                    renderable::style::Border {
                        sides: renderable::style::BorderSides::All,
                        ..Default::default()
                    },
                ),
                "false" | "none" => crate::style::schema::common::ComponentBorder(
                    renderable::style::Border {
                        sides: renderable::style::BorderSides::None,
                        ..Default::default()
                    },
                ),
                "medium" | "thick" => {
                    let weight = match value.trim().to_ascii_lowercase().as_str() {
                        "medium" => renderable::style::BorderWeight::Medium,
                        "thick" => renderable::style::BorderWeight::Thick,
                        _ => unreachable!(),
                    };
                    crate::style::schema::common::ComponentBorder(renderable::style::Border {
                        weight,
                        sides: renderable::style::BorderSides::All,
                        ..Default::default()
                    })
                }
                _ => return false,
            };
            style.border = Some(border);
            true
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
