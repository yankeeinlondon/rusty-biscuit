//! Copied from S1 (`s1-comparison/src/fix.rs`), unchanged. The SVG post-processing S1's "Merman (real fonts)" column applied. It is
//! kept only because it corrects a genuine SVG paint bug: Merman's neo look
//! strokes node shapes with an `objectBoundingBox` gradient, which by the SVG
//! spec paints nothing on a zero-width or zero-height line (the class and ER
//! dividers in the dark theme).
//!
//! The column's other correction is a rasterizer option the caller applies:
//! `usvg::Options::font_family` set to the sans-serif family, for text with no
//! font set. Label clipping is deliberately not patched: it comes from text
//! measurement, which the column addresses by measuring with the real fonts.
//!
//! The edit keys on Merman's output at rev 482fcde; a Merman bump must re-check
//! it (the defect survey does, for every family).

/// Applies the gradient-stroke fix. Input without a node gradient is returned
/// unchanged.
pub fn fix_merman_svg(svg: &str) -> String {
    solid_node_stroke(svg)
}

/// Merman's default `neo` look strokes node shapes with
/// `url(#<diagram>-gradient)`, a gradient in `objectBoundingBox` units. Per the
/// SVG spec such a paint draws nothing on a zero-width or zero-height path, so
/// class compartment dividers and other straight node lines vanish, and
/// outlines fade toward the background. Every `stroke:url(#…-gradient)` becomes
/// the theme's solid node stroke, read from the stylesheet (see
/// `node_stroke_color`) rather than hard-coded.
fn solid_node_stroke(svg: &str) -> String {
    let Some(gradient_id) = node_gradient_id(svg) else { return svg.to_string() };
    let Some(color) = node_stroke_color(svg) else { return svg.to_string() };
    svg.replace(&format!("stroke:url(#{gradient_id})"), &format!("stroke:{color}"))
}

/// The id of Merman's node gradient: the `<linearGradient>` whose id is
/// `<diagram id>-gradient`.
fn node_gradient_id(svg: &str) -> Option<&str> {
    let mut rest = svg;
    while let Some(at) = rest.find("<linearGradient") {
        rest = &rest[at..];
        let tag = &rest[..rest.find('>')?];
        if let Some(id) = attribute(tag, "id")
            && id.ends_with("-gradient")
        {
            return Some(id);
        }
        rest = &rest[1..];
    }
    None
}

/// The theme's solid node stroke: the first stylesheet rule outside the `neo`
/// look (`[data-look=…]`) whose selector styles node shapes (`.node path`,
/// `.node rect`, or `.node .neo-node`; families differ in which they list)
/// and whose `stroke` is a color.
fn node_stroke_color(svg: &str) -> Option<&str> {
    let css = &svg[svg.find("<style")?..];
    let css = &css[..css.find("</style>")?];
    css.split('}').find_map(|rule| {
        let (selector, body) = rule.split_once('{')?;
        let styles_nodes = [".node path", ".node rect", ".node .neo-node"].iter().any(|s| selector.contains(s));
        if !styles_nodes || selector.contains("[data-look") {
            return None;
        }
        let value = body.split(';').find_map(|decl| decl.trim().strip_prefix("stroke:"))?.trim();
        (!value.starts_with("url(")).then_some(value)
    })
}

fn attribute<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    let key = format!(" {name}=\"");
    let start = tag.find(&key)? + key.len();
    Some(&tag[start..start + tag[start..].find('"')?])
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = concat!(
        r#"<svg id="s1"><style>#s1 .node rect,#s1 .node path{fill:#1f2020;stroke:#ccc;stroke-width:1;}"#,
        r#"#s1 [data-look="neo"].node path{stroke:url(#s1-gradient);stroke-width:1px;}</style>"#,
        r#"<linearGradient id="s1-gradient" gradientUnits="objectBoundingBox"/>"#,
        r#"<svg style="overflow:hidden"/></svg>"#,
    );

    #[test]
    fn gradient_stroke_becomes_the_stylesheet_node_stroke() {
        let fixed = solid_node_stroke(SAMPLE);
        assert!(fixed.contains(r#"[data-look="neo"].node path{stroke:#ccc;"#));
        assert!(!fixed.contains("stroke:url(#s1-gradient)"));
    }

    #[test]
    fn stroke_is_found_when_the_base_rule_omits_path() {
        let er = r#"<svg><style>#s1 .node rect,#s1 .node polygon{fill:#1f2020;stroke:#abc;}#s1 [data-look="neo"].node rect{stroke:url(#s1-gradient);}</style><linearGradient id="s1-gradient"/></svg>"#;
        assert!(solid_node_stroke(er).contains(r#"[data-look="neo"].node rect{stroke:#abc;}"#));
    }

    #[test]
    fn label_viewports_are_left_alone() {
        assert!(fix_merman_svg(SAMPLE).ends_with(r#"<svg style="overflow:hidden"/></svg>"#));
    }

    #[test]
    fn svg_without_targets_is_unchanged() {
        assert_eq!(fix_merman_svg("<svg><text>x</text></svg>"), "<svg><text>x</text></svg>");
    }
}
