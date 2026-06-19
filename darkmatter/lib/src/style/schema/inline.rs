//! Inline buckets: `hyperlinks`, `images`. Both carry an optional
//! `local-style` override applied to file-local references.

use serde::Deserialize;

use super::common::CommonStyle;

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
pub struct HyperlinkStyle {
    #[serde(flatten)]
    pub common: CommonStyle,
    #[serde(alias = "local_style")]
    pub local_style: Option<Box<CommonStyle>>,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
pub struct ImageStyle {
    #[serde(flatten)]
    pub common: CommonStyle,
    #[serde(alias = "local_style")]
    pub local_style: Option<Box<CommonStyle>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use renderable::layout::Alignment;

    #[test]
    fn hyperlinks_with_local_style() {
        let json = r#"{
            "alignment": "left",
            "local-style": {"alignment": "right"}
        }"#;
        let h: HyperlinkStyle = serde_json::from_str(json).unwrap();
        assert_eq!(h.common.alignment, Some(Alignment::Left));
        let local = h.local_style.expect("local_style should be Some");
        assert_eq!(local.alignment, Some(Alignment::Right));
    }

    #[test]
    fn snake_case_local_style_alias_accepted() {
        let json = r#"{"local_style": {"alignment": "right"}}"#;
        let h: HyperlinkStyle = serde_json::from_str(json).unwrap();
        assert!(h.local_style.is_some());
    }

    #[test]
    fn images_local_style_independent() {
        let json = r#"{"local-style": {"max-width": "50%"}}"#;
        let img: ImageStyle = serde_json::from_str(json).unwrap();
        let local = img.local_style.expect("local_style should be Some");
        assert!(local.max_width.is_some());
    }
}
