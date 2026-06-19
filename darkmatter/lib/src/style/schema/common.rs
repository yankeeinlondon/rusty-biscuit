//! `CommonStyle` — the five mutations shared by every component bucket
//! (`width`, `max-width`, `alignment`, `color`, `bg-color`).

use renderable::layout::{Alignment, Length};
use renderable::stylesheet::CssSizing;
use serde::Deserialize;

use crate::style::alignment::deserialize_optional_alignment;
use crate::style::color::{StyleColor, deserialize_optional_color};
use crate::style::length::deserialize_optional_length;

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
pub struct CommonStyle {
    #[serde(deserialize_with = "deserialize_optional_length")]
    pub width: Option<Length>,
    #[serde(deserialize_with = "deserialize_optional_length", alias = "max_width")]
    pub max_width: Option<Length>,
    #[serde(deserialize_with = "deserialize_optional_alignment")]
    pub alignment: Option<Alignment>,
    #[serde(deserialize_with = "deserialize_optional_color")]
    pub color: Option<StyleColor>,
    #[serde(deserialize_with = "deserialize_optional_color", alias = "bg_color")]
    pub bg_color: Option<StyleColor>,
}

impl CommonStyle {
    /// Convert this `CommonStyle` into a CSS declaration overlay.
    ///
    /// Handles `width`, `max-width`, `text-align`, `color`, and
    /// `background-color`. Returns `None` when no style fields are set.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use renderable::layout::{Alignment, Length};
    /// use darkmatter::style::schema::CommonStyle;
    ///
    /// let style = CommonStyle {
    ///     alignment: Some(Alignment::Center),
    ///     ..CommonStyle::default()
    /// };
    /// let css = style.to_css_overlay().expect("has alignment");
    /// assert!(css.to_css().contains("text-align: center"));
    /// ```
    pub fn to_css_overlay(&self) -> Option<renderable::stylesheet::CssStyle> {
        use renderable::stylesheet::{
            CssColor, CssColorProp, CssProp, CssRaw, CssSizingProp, CssStyle, CssValue,
        };

        let mut css = CssStyle::new();
        let mut has_any = false;

        if let Some(width) = &self.width
            && let Some(sizing) = length_to_css_sizing(width)
        {
            css = css.add(CssSizingProp::Width, sizing);
            has_any = true;
        }

        if let Some(max_width) = &self.max_width
            && let Some(sizing) = length_to_css_sizing(max_width)
        {
            css = css.add(CssSizingProp::MaxWidth, sizing);
            has_any = true;
        }

        if let Some(alignment) = self.alignment {
            let value = match alignment {
                Alignment::Left => "left",
                Alignment::Center => "center",
                Alignment::Right => "right",
            };
            css = css
                .try_add(
                    CssProp::Other("text-align".into()),
                    CssValue::Raw(CssRaw::new(value).expect("static value is valid")),
                )
                .expect("static value is valid");
            has_any = true;
        }

        if let Some(color) = &self.color
            && let Some(css_str) = crate::style::color::lower_to_css(color)
            && let Ok(css_color) = CssColor::try_from(css_str.as_str())
        {
            css = css.add(CssColorProp::Color, css_color);
            has_any = true;
        }

        if let Some(bg_color) = &self.bg_color
            && let Some(css_str) = crate::style::color::lower_to_css(bg_color)
            && let Ok(css_color) = CssColor::try_from(css_str.as_str())
        {
            css = css.add(CssColorProp::BackgroundColor, css_color);
            has_any = true;
        }

        if has_any { Some(css) } else { None }
    }
}

/// Lower a [`Length`] to a [`CssSizing`] for CSS declaration output.
fn length_to_css_sizing(length: &Length) -> Option<CssSizing> {
    use renderable::stylesheet::CssUnit;
    match length {
        Length::Zero => Some(CssSizing::Zero),
        Length::Ch(n) => Some(CssSizing::dimension(*n as f32, CssUnit::Ch)),
        Length::Percent(p) => Some(CssSizing::percent(*p)),
        Length::Css(sizing) => Some(sizing.clone()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_object_yields_default() {
        let c: CommonStyle = serde_json::from_str("{}").unwrap();
        assert_eq!(c, CommonStyle::default());
    }

    #[test]
    fn parses_max_width_percent() {
        let c: CommonStyle = serde_json::from_str(r#"{"max-width": "50%"}"#).unwrap();
        assert_eq!(c.max_width, Some(Length::Percent(50.0)));
    }

    #[test]
    fn parses_alignment_centered() {
        let c: CommonStyle = serde_json::from_str(r#"{"alignment": "centered"}"#).unwrap();
        assert_eq!(c.alignment, Some(Alignment::Center));
    }

    #[test]
    fn snake_case_max_width_alias_accepted() {
        // serde `alias` accepts the snake_case form. The Deprecated warning
        // is emitted by the canonicalization walker (Task 17), not by serde.
        let c: CommonStyle = serde_json::from_str(r#"{"max_width": "50%"}"#).unwrap();
        assert_eq!(c.max_width, Some(Length::Percent(50.0)));
    }

    // ---------- to_css_overlay ----------

    use renderable::color::{Color, Tailwind};
    
    use crate::style::color::StyleColor;

    #[test]
    fn to_css_overlay_empty_returns_none() {
        let style = CommonStyle::default();
        assert!(style.to_css_overlay().is_none());
    }

    #[test]
    fn to_css_overlay_width() {
        let style = CommonStyle {
            width: Some(Length::Ch(40)),
            ..CommonStyle::default()
        };
        let css = style.to_css_overlay().unwrap();
        assert!(css.to_css().contains("width: 40ch"));
    }

    #[test]
    fn to_css_overlay_max_width_percent() {
        let style = CommonStyle {
            max_width: Some(Length::Percent(50.0)),
            ..CommonStyle::default()
        };
        let css = style.to_css_overlay().unwrap();
        assert!(css.to_css().contains("max-width: 50%"));
    }

    #[test]
    fn to_css_overlay_alignment_center() {
        let style = CommonStyle {
            alignment: Some(Alignment::Center),
            ..CommonStyle::default()
        };
        let css = style.to_css_overlay().unwrap();
        assert!(css.to_css().contains("text-align: center"));
    }

    #[test]
    fn to_css_overlay_alignment_left() {
        let style = CommonStyle {
            alignment: Some(Alignment::Left),
            ..CommonStyle::default()
        };
        let css = style.to_css_overlay().unwrap();
        assert!(css.to_css().contains("text-align: left"));
    }

    #[test]
    fn to_css_overlay_color() {
        let style = CommonStyle {
            color: Some(StyleColor {
                color: Color::Tailwind(Tailwind::Red500),
                opacity: None,
            }),
            ..CommonStyle::default()
        };
        let css = style.to_css_overlay().unwrap();
        assert!(css.to_css().contains("color: rgb("));
    }

    #[test]
    fn to_css_overlay_bg_color() {
        let style = CommonStyle {
            bg_color: Some(StyleColor {
                color: Color::Tailwind(Tailwind::Blue500),
                opacity: None,
            }),
            ..CommonStyle::default()
        };
        let css = style.to_css_overlay().unwrap();
        assert!(css.to_css().contains("background-color: rgb("));
    }

    #[test]
    fn to_css_overlay_multiple_properties() {
        let style = CommonStyle {
            width: Some(Length::Ch(40)),
            alignment: Some(Alignment::Right),
            color: Some(StyleColor {
                color: Color::Tailwind(Tailwind::Green500),
                opacity: None,
            }),
            ..CommonStyle::default()
        };
        let css = style.to_css_overlay().unwrap();
        let text = css.to_css();
        assert!(text.contains("width: 40ch"));
        assert!(text.contains("text-align: right"));
        assert!(text.contains("color: rgb("));
    }

    #[test]
    fn to_css_overlay_css_length_passthrough() {
        use renderable::stylesheet::CssSizing;
        let style = CommonStyle {
            width: Some(Length::Css(CssSizing::px(320.0))),
            ..CommonStyle::default()
        };
        let css = style.to_css_overlay().unwrap();
        assert!(css.to_css().contains("width: 320px"));
    }
}
