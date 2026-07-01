//! Component buckets whose schema is exactly `CommonStyle` (no bespoke
//! fields): `table`, `block-quote`, `disclosure`, `code-block`.

use serde::Deserialize;

use super::common::CommonStyle;

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
pub struct TableStyle {
    #[serde(flatten)]
    pub common: CommonStyle,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
pub struct BlockQuoteStyle {
    #[serde(flatten)]
    pub common: CommonStyle,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
pub struct DisclosureStyle {
    #[serde(flatten)]
    pub common: CommonStyle,
}

/// `code-block` bucket carries `CommonStyle` layout/appearance for fenced
/// code blocks. The code theme remains under `page.code.theme` for
/// compatibility.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
pub struct CodeBlockStyle {
    #[serde(flatten)]
    pub common: CommonStyle,
}

#[cfg(test)]
mod tests {
    use super::*;
    use renderable::layout::{Alignment, Length};

    #[test]
    fn table_parses_alignment_and_max_width() {
        let json = r#"{"alignment": "right", "max-width": "50%"}"#;
        let t: TableStyle = serde_json::from_str(json).unwrap();
        assert_eq!(t.common.alignment, Some(Alignment::Right));
        assert_eq!(t.common.max_width, Some(Length::Percent(50.0)));
    }

    #[test]
    fn block_quote_parses_color() {
        let json = r#"{"color": "red-500"}"#;
        let bq: BlockQuoteStyle = serde_json::from_str(json).unwrap();
        assert!(bq.common.color.is_some());
    }

    #[test]
    fn code_block_parses_margin_and_emphasis() {
        let json = r#"{"margin": "2ch", "emphasis": {"bold": true}}"#;
        let cb: CodeBlockStyle = serde_json::from_str(json).unwrap();
        assert!(cb.common.margin.is_some());
        assert!(cb.common.emphasis.as_ref().unwrap().to_text_emphasis().bold);
    }
}
