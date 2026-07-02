//! `CommonStyle` — the shared surface exposed by every component bucket.
//!
//! Covers the original five keys (`width`, `max-width`, `alignment`, `color`,
//! `bg-color`) plus the Phase 5 surface (`margin`, `padding`, `border`,
//! `emphasis`, `word-wrap`) and the width-mode syntax (`width: auto`,
//! `width: fit-content`).

use renderable::layout::{Alignment, Edges, Length, TargetValue, Width};
use renderable::style::{
    Border, BorderLineStyle, BorderSides, BorderWeight, TextEmphasis, UnderlineStyle,
};
use renderable::stylesheet::CssSizing;
use renderable::wrap_policy::WordWrap;
use serde::{Deserialize, Deserializer, de};

use crate::style::alignment::deserialize_optional_alignment;
use crate::style::color::{StyleColor, deserialize_optional_color};
use crate::style::length::deserialize_optional_length;

/// A `width` value that is either a length or a width-mode keyword.
///
/// Accepted forms:
/// - `"auto"` → `Width::Auto`
/// - `"fit-content"` or `"fit_content"` → `Width::FitContent`
/// - `"40ch"`, `"50%"`, `40` → `Width::Fixed(TargetValue::universal(...))`
#[derive(Debug, Clone, PartialEq)]
pub enum WidthOrMode {
    Width(Width),
}

/// Extract the fixed [`Length`] from a [`WidthOrMode`], if any.
///
/// `Auto` and `FitContent` return `None`; `Fixed(len)` returns the universal
/// length when present.
pub fn width_or_mode_fixed_length(width: &WidthOrMode) -> Option<&Length> {
    match width.as_width() {
        Width::Fixed(renderable::layout::TargetValue::Universal(len)) => Some(len),
        _ => None,
    }
}

impl WidthOrMode {
    pub fn as_width(&self) -> &Width {
        match self {
            WidthOrMode::Width(w) => w,
        }
    }
}

impl Default for WidthOrMode {
    fn default() -> Self {
        WidthOrMode::Width(Width::Auto)
    }
}

impl<'de> Deserialize<'de> for WidthOrMode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value: serde_json::Value = Deserialize::deserialize(deserializer)?;
        match value {
            serde_json::Value::String(s) => match s.trim().to_ascii_lowercase().as_str() {
                "auto" => Ok(WidthOrMode::Width(Width::Auto)),
                "fit-content" | "fit_content" => Ok(WidthOrMode::Width(Width::FitContent)),
                _ => {
                    let len = crate::style::length::parse_horizontal(&s).map_err(|reason| {
                        de::Error::custom(format!(
                            "width must be 'auto', 'fit-content', or a length (got {s}: {reason})"
                        ))
                    })?;
                    Ok(WidthOrMode::Width(Width::Fixed(TargetValue::universal(len))))
                }
            },
            serde_json::Value::Number(n) => {
                let u = n.as_u64().and_then(|v| u32::try_from(v).ok()).ok_or_else(|| {
                    de::Error::custom("width integer must be a non-negative u32")
                })?;
                Ok(WidthOrMode::Width(Width::Fixed(TargetValue::universal(
                    Length::Ch(u),
                ))))
            }
            other => Err(de::Error::custom(format!(
                "width must be a string or non-negative integer (got {})",
                json_type_name(&other)
            ))),
        }
    }
}

fn json_type_name(v: &serde_json::Value) -> &'static str {
    match v {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "bool",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::String(_) => "string",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::Object(_) => "object",
    }
}

fn deserialize_optional_width_or_mode<'de, D>(de: D) -> Result<Option<WidthOrMode>, D::Error>
where
    D: Deserializer<'de>,
{
    let value: Option<serde_json::Value> = Option::deserialize(de)?;
    match value {
        None | Some(serde_json::Value::Null) => Ok(None),
        Some(v) => {
            let width = WidthOrMode::deserialize(v).map_err(|e| {
                de::Error::custom(format!("invalid width value: {e}"))
            })?;
            Ok(Some(width))
        }
    }
}

/// A margin or padding shorthand: a single length applied to all four sides.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ComponentEdges(pub Edges);

impl ComponentEdges {
    pub fn as_edges(&self) -> &Edges {
        &self.0
    }
}

impl<'de> Deserialize<'de> for ComponentEdges {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value: serde_json::Value = Deserialize::deserialize(deserializer)?;
        match value {
            serde_json::Value::String(s) => {
                let len = crate::style::length::parse_horizontal(&s).map_err(|reason| {
                    de::Error::custom(format!(
                        "margin/padding shorthand must be a length (got {s}: {reason})"
                    ))
                })?;
                Ok(ComponentEdges(Edges::all(len)))
            }
            serde_json::Value::Number(n) => {
                let u = n.as_u64().and_then(|v| u32::try_from(v).ok()).ok_or_else(|| {
                    de::Error::custom("margin/padding shorthand integer must be a non-negative u32")
                })?;
                Ok(ComponentEdges(Edges::all(Length::Ch(u))))
            }
            other => Err(de::Error::custom(format!(
                "margin/padding shorthand must be a length string or non-negative integer (got {})",
                json_type_name(&other)
            ))),
        }
    }
}

fn deserialize_optional_component_edges<'de, D>(de: D) -> Result<Option<ComponentEdges>, D::Error>
where
    D: Deserializer<'de>,
{
    let value: Option<serde_json::Value> = Option::deserialize(de)?;
    match value {
        None | Some(serde_json::Value::Null) => Ok(None),
        Some(v) => {
            let edges = ComponentEdges::deserialize(v).map_err(|e| {
                de::Error::custom(format!("invalid margin/padding value: {e}"))
            })?;
            Ok(Some(edges))
        }
    }
}

/// Simplified border description accepted from frontmatter.
///
/// Accepted forms:
/// - `true` / `false` — enable/disable a thin solid border on all sides
/// - `"thin"` / `"medium"` / `"thick"` — set weight on all sides
/// - object `{ left: true, right: false, ... }` with optional `weight`,
///   `color`, `style`, `radius`
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ComponentBorder(pub Border);

impl ComponentBorder {
    pub fn as_border(&self) -> &Border {
        &self.0
    }
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
struct ComponentBorderObject {
    pub left: Option<bool>,
    pub right: Option<bool>,
    pub top: Option<bool>,
    pub bottom: Option<bool>,
    pub weight: Option<BorderWeight>,
    #[serde(deserialize_with = "deserialize_optional_color")]
    pub color: Option<StyleColor>,
    #[serde(rename = "style")]
    pub line_style: Option<BorderLineStyle>,
    #[serde(deserialize_with = "deserialize_optional_length")]
    pub radius: Option<Length>,
}

impl<'de> Deserialize<'de> for ComponentBorder {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value: serde_json::Value = Deserialize::deserialize(deserializer)?;
        match value {
            serde_json::Value::Bool(true) => Ok(ComponentBorder(Border {
                sides: BorderSides::All,
                ..Border::default()
            })),
            serde_json::Value::Bool(false) => Ok(ComponentBorder(Border {
                sides: BorderSides::None,
                ..Border::default()
            })),
            serde_json::Value::String(s) => {
                let weight = match s.trim().to_ascii_lowercase().as_str() {
                    "thin" => BorderWeight::Thin,
                    "medium" => BorderWeight::Medium,
                    "thick" => BorderWeight::Thick,
                    "none" => {
                        return Ok(ComponentBorder(Border {
                            sides: BorderSides::None,
                            ..Border::default()
                        }))
                    }
                    _ => {
                        return Err(de::Error::custom(format!(
                            "border weight must be 'thin', 'medium', 'thick', or 'none' (got {s})"
                        )))
                    }
                };
                Ok(ComponentBorder(Border {
                    weight,
                    sides: BorderSides::All,
                    ..Border::default()
                }))
            }
            serde_json::Value::Object(_) => {
                let obj: ComponentBorderObject =
                    ComponentBorderObject::deserialize(value).map_err(|e| {
                        de::Error::custom(format!("invalid border object: {e}"))
                    })?;

                let sides = if obj.left.is_none()
                    && obj.right.is_none()
                    && obj.top.is_none()
                    && obj.bottom.is_none()
                {
                    BorderSides::All
                } else {
                    BorderSides::Sides {
                        top: obj.top.unwrap_or(false),
                        right: obj.right.unwrap_or(false),
                        bottom: obj.bottom.unwrap_or(false),
                        left: obj.left.unwrap_or(false),
                    }
                };

                let color = obj.color.as_ref().map(|c| {
                    TargetValue::universal(renderable::style::PerMode::universal(c.to_paint_color()))
                });
                let radius = obj.radius.as_ref().map(|r| TargetValue::universal(r.clone()));

                Ok(ComponentBorder(Border {
                    color,
                    weight: obj.weight.unwrap_or_default(),
                    line_style: obj.line_style.unwrap_or_default(),
                    sides,
                    radius,
                }))
            }
            other => Err(de::Error::custom(format!(
                "border must be a bool, string, or object (got {})",
                json_type_name(&other)
            ))),
        }
    }
}

fn deserialize_optional_component_border<'de, D>(de: D) -> Result<Option<ComponentBorder>, D::Error>
where
    D: Deserializer<'de>,
{
    let value: Option<serde_json::Value> = Option::deserialize(de)?;
    match value {
        None | Some(serde_json::Value::Null) => Ok(None),
        Some(v) => {
            let border = ComponentBorder::deserialize(v).map_err(|e| {
                de::Error::custom(format!("invalid border value: {e}"))
            })?;
            Ok(Some(border))
        }
    }
}

/// Simplified emphasis description accepted from frontmatter.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
pub struct ComponentEmphasis {
    pub bold: Option<bool>,
    pub dim: Option<bool>,
    pub italic: Option<bool>,
    pub strikethrough: Option<bool>,
    pub underline: Option<ComponentUnderline>,
    pub inverse: Option<bool>,
    pub blink: Option<bool>,
}

impl ComponentEmphasis {
    pub fn to_text_emphasis(&self) -> TextEmphasis {
        TextEmphasis {
            bold: self.bold.unwrap_or(false),
            dim: self.dim.unwrap_or(false),
            italic: self.italic.unwrap_or(false),
            strikethrough: self.strikethrough.unwrap_or(false),
            blink: self.blink.unwrap_or(false),
            inverse: self.inverse.unwrap_or(false),
            underline: self.underline.map(|u| u.into()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ComponentUnderline {
    Straight,
    Double,
    Curly,
    Dotted,
    Dashed,
}

impl From<ComponentUnderline> for UnderlineStyle {
    fn from(value: ComponentUnderline) -> Self {
        match value {
            ComponentUnderline::Straight => UnderlineStyle::Straight,
            ComponentUnderline::Double => UnderlineStyle::Double,
            ComponentUnderline::Curly => UnderlineStyle::Curly,
            ComponentUnderline::Dotted => UnderlineStyle::Dotted,
            ComponentUnderline::Dashed => UnderlineStyle::Dashed,
        }
    }
}

/// Simplified word-wrap description accepted from frontmatter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ComponentWordWrap {
    None,
    Wrap,
    Truncate,
}

impl ComponentWordWrap {
    pub fn to_word_wrap(&self) -> WordWrap {
        match self {
            ComponentWordWrap::None => WordWrap::None,
            ComponentWordWrap::Wrap => WordWrap::WrapProse(Some(8), None),
            ComponentWordWrap::Truncate => WordWrap::Truncate(None),
        }
    }
}

fn deserialize_optional_word_wrap<'de, D>(de: D) -> Result<Option<ComponentWordWrap>, D::Error>
where
    D: Deserializer<'de>,
{
    let value: Option<serde_json::Value> = Option::deserialize(de)?;
    match value {
        None | Some(serde_json::Value::Null) => Ok(None),
        Some(serde_json::Value::String(s)) => match s.trim().to_ascii_lowercase().as_str() {
            "none" => Ok(Some(ComponentWordWrap::None)),
            "wrap" | "wrap-prose" | "wrap_prose" => Ok(Some(ComponentWordWrap::Wrap)),
            "truncate" => Ok(Some(ComponentWordWrap::Truncate)),
            _ => Err(de::Error::custom(format!(
                "word-wrap must be 'none', 'wrap', or 'truncate' (got {s})"
            ))),
        },
        Some(other) => Err(de::Error::custom(format!(
            "word-wrap must be a string (got {})",
            json_type_name(&other)
        ))),
    }
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
pub struct CommonStyle {
    #[serde(deserialize_with = "deserialize_optional_width_or_mode")]
    pub width: Option<WidthOrMode>,
    #[serde(deserialize_with = "deserialize_optional_length", alias = "max_width")]
    pub max_width: Option<Length>,
    #[serde(deserialize_with = "deserialize_optional_alignment")]
    pub alignment: Option<Alignment>,
    #[serde(deserialize_with = "deserialize_optional_color")]
    pub color: Option<StyleColor>,
    #[serde(deserialize_with = "deserialize_optional_color", alias = "bg_color")]
    pub bg_color: Option<StyleColor>,
    #[serde(deserialize_with = "deserialize_optional_component_edges")]
    pub margin: Option<ComponentEdges>,
    #[serde(deserialize_with = "deserialize_optional_component_edges")]
    pub padding: Option<ComponentEdges>,
    #[serde(deserialize_with = "deserialize_optional_component_border")]
    pub border: Option<ComponentBorder>,
    pub emphasis: Option<ComponentEmphasis>,
    #[serde(deserialize_with = "deserialize_optional_word_wrap", alias = "word_wrap")]
    pub word_wrap: Option<ComponentWordWrap>,
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

        if let Some(width) = &self.width {
            match width.as_width() {
                Width::Auto => {
                    css = css.add(CssSizingProp::Width, renderable::stylesheet::CssSizing::Auto);
                    has_any = true;
                }
                Width::FitContent => {
                    css = css.add(
                        CssSizingProp::Width,
                        renderable::stylesheet::CssSizing::FitContent,
                    );
                    has_any = true;
                }
                Width::Fixed(tv) => {
                    if let renderable::layout::TargetValue::Universal(len) = tv
                        && let Some(sizing) = length_to_css_sizing(len)
                    {
                        css = css.add(CssSizingProp::Width, sizing);
                        has_any = true;
                    }
                }
            }
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
            width: Some(WidthOrMode::Width(Width::Fixed(TargetValue::universal(
                Length::Ch(40),
            )))),
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
            width: Some(WidthOrMode::Width(Width::Fixed(TargetValue::universal(
                Length::Ch(40),
            )))),
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
            width: Some(WidthOrMode::Width(Width::Fixed(TargetValue::universal(
                Length::Css(CssSizing::px(320.0)),
            )))),
            ..CommonStyle::default()
        };
        let css = style.to_css_overlay().unwrap();
        assert!(css.to_css().contains("width: 320px"));
    }

    #[test]
    fn parses_width_mode_auto() {
        let c: CommonStyle = serde_json::from_str(r#"{"width": "auto"}"#).unwrap();
        assert_eq!(
            c.width,
            Some(WidthOrMode::Width(Width::Auto))
        );
    }

    #[test]
    fn parses_width_mode_fit_content() {
        let c: CommonStyle = serde_json::from_str(r#"{"width": "fit-content"}"#).unwrap();
        assert_eq!(
            c.width,
            Some(WidthOrMode::Width(Width::FitContent))
        );
    }

    #[test]
    fn parses_width_length_still_works() {
        let c: CommonStyle = serde_json::from_str(r#"{"width": "40ch"}"#).unwrap();
        assert_eq!(
            c.width,
            Some(WidthOrMode::Width(Width::Fixed(TargetValue::universal(Length::Ch(40)))))
        );
    }

    #[test]
    fn parses_margin_shorthand() {
        let c: CommonStyle = serde_json::from_str(r#"{"margin": "2ch"}"#).unwrap();
        let edges = c.margin.expect("margin").0;
        assert_eq!(edges, Edges::all(Length::Ch(2)));
    }

    #[test]
    fn parses_padding_shorthand() {
        let c: CommonStyle = serde_json::from_str(r#"{"padding": 1}"#).unwrap();
        let edges = c.padding.expect("padding").0;
        assert_eq!(edges, Edges::all(Length::Ch(1)));
    }

    #[test]
    fn parses_border_bool() {
        let c: CommonStyle = serde_json::from_str(r#"{"border": true}"#).unwrap();
        let border = c.border.expect("border").0;
        assert_eq!(border.sides, BorderSides::All);
        assert_eq!(border.weight, BorderWeight::Thin);
    }

    #[test]
    fn parses_border_object_with_left_side() {
        let c: CommonStyle = serde_json::from_str(r#"{"border": {"left": true, "weight": "thick"}}"#).unwrap();
        let border = c.border.expect("border").0;
        assert_eq!(border.weight, BorderWeight::Thick);
        assert_eq!(
            border.sides,
            BorderSides::Sides {
                top: false,
                right: false,
                bottom: false,
                left: true,
            }
        );
    }

    #[test]
    fn parses_emphasis() {
        let c: CommonStyle = serde_json::from_str(r#"{"emphasis": {"bold": true, "italic": true}}"#).unwrap();
        let emphasis = c.emphasis.expect("emphasis").to_text_emphasis();
        assert!(emphasis.bold);
        assert!(emphasis.italic);
    }

    #[test]
    fn parses_word_wrap() {
        let c: CommonStyle = serde_json::from_str(r#"{"word-wrap": "wrap"}"#).unwrap();
        assert_eq!(c.word_wrap, Some(ComponentWordWrap::Wrap));
    }

    /// Validation guard (spec item 8): an unsupported `word-wrap` value is
    /// rejected at deserialize time, never silently dropped.
    #[test]
    fn rejects_unknown_word_wrap() {
        let err = serde_json::from_str::<CommonStyle>(r#"{"word-wrap": "diagonal"}"#).unwrap_err();
        assert!(
            err.to_string().contains("word-wrap must be"),
            "unexpected error: {err}"
        );
    }
}
