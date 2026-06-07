//! Styling options applied to an icon, and the local `<svg>` assembler.

use crate::body::IconBody;
use crate::error::{IconError, Result};

/// Accumulated presentation options. All default to "unset"; an unset option
/// means the corresponding SVG attribute is omitted (Iconify defaults apply).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Style {
    /// CSS color applied via the `color` style (drives `currentColor`).
    pub color: Option<String>,
    /// SVG width attribute (default `1em` when `None`).
    pub width: Option<String>,
    /// SVG height attribute (default `1em` when `None`).
    pub height: Option<String>,
    /// Horizontal, vertical, or both-axis flip.
    pub flip: Option<Flip>,
    /// 90/180/270 degree rotation.
    pub rotate: Option<Rotate>,
    /// When true, emit a transparent bounding-box rect spanning the viewBox.
    pub view_box: bool,
}

/// Allowed flip values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flip {
    /// Flip horizontally.
    Horizontal,
    /// Flip vertically.
    Vertical,
    /// Flip both axes.
    Both,
}

impl TryFrom<&str> for Flip {
    type Error = IconError;

    fn try_from(value: &str) -> Result<Self> {
        match value {
            "horizontal" => Ok(Flip::Horizontal),
            "vertical" => Ok(Flip::Vertical),
            "both" => Ok(Flip::Both),
            _ => Err(IconError::InvalidIdentifier(format!("invalid flip: {value}"))),
        }
    }
}

/// Allowed rotation values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rotate {
    /// 90 degrees clockwise.
    R90,
    /// 180 degrees.
    R180,
    /// 270 degrees clockwise.
    R270,
}

impl TryFrom<&str> for Rotate {
    type Error = IconError;

    fn try_from(value: &str) -> Result<Self> {
        match value {
            "90" => Ok(Rotate::R90),
            "180" => Ok(Rotate::R180),
            "270" => Ok(Rotate::R270),
            _ => Err(IconError::InvalidIdentifier(format!("invalid rotate: {value}"))),
        }
    }
}

impl Style {
    /// Builds the SVG `transform` value for the configured flip/rotate, if any.
    ///
    /// Returns `None` when neither flip nor rotate is set.
    fn transform(&self, body: &IconBody) -> Option<String> {
        let (w, h) = (f64::from(body.width), f64::from(body.height));
        let mut parts: Vec<String> = Vec::new();
        match self.flip {
            Some(Flip::Horizontal) => parts.push(format!("translate({w} 0) scale(-1 1)")),
            Some(Flip::Vertical) => parts.push(format!("translate(0 {h}) scale(1 -1)")),
            Some(Flip::Both) => parts.push(format!("translate({w} {h}) scale(-1 -1)")),
            None => {}
        }
        match self.rotate {
            Some(Rotate::R90) => {
                // Rotate around the original center, then translate so the
                // swapped viewBox (0 0 h w) is filled without clipping.
                parts.push(format!("translate({h} {w}) rotate(90) translate(-{w} -{h})", w = w / 2.0, h = h / 2.0));
            }
            Some(Rotate::R180) => parts.push(format!("rotate(180 {} {})", w / 2.0, h / 2.0)),
            Some(Rotate::R270) => {
                parts.push(format!("translate({h} {w}) rotate(270) translate(-{w} -{h})", w = w / 2.0, h = h / 2.0));
            }
            None => {}
        }
        if parts.is_empty() { None } else { Some(parts.join(" ")) }
    }

    /// Assembles a complete `<svg>` string from an icon body and this style.
    #[must_use]
    pub fn assemble(&self, body: &IconBody) -> String {
        let (w, h) = (body.width, body.height);
        let (vw, vh) = match self.rotate {
            Some(Rotate::R90) | Some(Rotate::R270) => (h, w),
            _ => (w, h),
        };
        let width = self.width.as_deref().unwrap_or("1em");
        let height = self.height.as_deref().unwrap_or("1em");
        let color_style = self
            .color
            .as_deref()
            .map(|c| format!(" style=\"color: {}\"", escape_xml_attr(c)))
            .unwrap_or_default();

        let inner = match self.transform(body) {
            Some(t) => format!("<g transform=\"{}\">{}</g>", escape_xml_attr(&t), body.body),
            None => body.body.clone(),
        };
        let view_rect = if self.view_box {
            format!(
                "<rect width=\"{}\" height=\"{}\" fill=\"none\"/>",
                vw, vh
            )
        } else {
            String::new()
        };

        format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" \
             viewBox=\"0 0 {vw} {vh}\"{color_style}>{view_rect}{inner}</svg>",
            escape_xml_attr(width),
            escape_xml_attr(height),
        )
    }
}

/// Escapes a string so it is safe inside a double-quoted XML attribute.
fn escape_xml_attr(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            c => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn body() -> IconBody {
        IconBody::new("<path d=\"M0 0\"/>", 24, 24)
    }

    fn wide_body() -> IconBody {
        IconBody::new("<path d=\"M0 0\"/>", 32, 16)
    }

    #[test]
    fn defaults_emit_1em_and_viewbox() {
        let svg = Style::default().assemble(&body());
        assert!(svg.contains("width=\"1em\""));
        assert!(svg.contains("height=\"1em\""));
        assert!(svg.contains("viewBox=\"0 0 24 24\""));
        assert!(svg.starts_with("<svg"));
        assert!(svg.ends_with("</svg>"));
    }

    #[test]
    fn explicit_size_and_color_applied() {
        let style = Style {
            width: Some("32".into()),
            height: Some("32".into()),
            color: Some("#d97706".into()),
            ..Style::default()
        };
        let svg = style.assemble(&body());
        assert!(svg.contains("width=\"32\""));
        assert!(svg.contains("style=\"color: #d97706\""));
    }

    #[test]
    fn rotate_wraps_body_in_transform_group() {
        let style = Style { rotate: Some(Rotate::R90), ..Style::default() };
        let svg = style.assemble(&body());
        assert!(svg.contains("<g transform="));
        assert!(svg.contains("rotate(90)"));
        assert!(svg.contains("viewBox=\"0 0 24 24\""));
    }

    #[test]
    fn non_square_rotate_swaps_viewbox() {
        let style = Style { rotate: Some(Rotate::R90), ..Style::default() };
        let svg = style.assemble(&wide_body());
        assert!(svg.contains("viewBox=\"0 0 16 32\""));
    }

    #[test]
    fn flip_horizontal_emits_scale() {
        let style = Style { flip: Some(Flip::Horizontal), ..Style::default() };
        let svg = style.assemble(&body());
        assert!(svg.contains("translate(24 0) scale(-1 1)"));
    }

    #[test]
    fn view_box_flag_emits_transparent_rect() {
        let style = Style { view_box: true, ..Style::default() };
        let svg = style.assemble(&body());
        assert!(svg.contains("<rect width=\"24\" height=\"24\" fill=\"none\"/>"));
    }

    #[test]
    fn malicious_color_is_escaped() {
        let style = Style { color: Some("red\" onload=alert(1)".into()), ..Style::default() };
        let svg = style.assemble(&body());
        assert!(!svg.contains("onload="));
        assert!(svg.contains("&quot;"));
    }

    #[test]
    fn invalid_flip_is_rejected() {
        assert!(Flip::try_from("diagonal").is_err());
    }

    #[test]
    fn invalid_rotate_is_rejected() {
        assert!(Rotate::try_from("45").is_err());
    }
}
