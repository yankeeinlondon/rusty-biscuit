//! Component buckets built on `CommonStyle`: `table`, `block-quote`, and
//! `disclosure` add nothing; `code-block` adds `line-numbers`.

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
/// code blocks. The code theme and mode are page defaults under
/// `page.code`.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
pub struct CodeBlockStyle {
    #[serde(flatten)]
    pub common: CommonStyle,
    /// Whether code blocks show a line-number gutter; an explicit `false`
    /// is kept, distinct from unset.
    pub line_numbers: Option<bool>,
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

    #[test]
    fn code_block_keeps_explicit_false_line_numbers() {
        let cb: CodeBlockStyle = serde_json::from_str(r#"{"line-numbers": false}"#).unwrap();
        assert_eq!(cb.line_numbers, Some(false));
        let unset: CodeBlockStyle = serde_json::from_str("{}").unwrap();
        assert_eq!(unset.line_numbers, None);
        assert!(serde_json::from_str::<CodeBlockStyle>(r#"{"line-numbers": "yes"}"#).is_err());
    }

    #[test]
    fn table_parses_horizontal_padding_sides() {
        let t: TableStyle = serde_json::from_str(r#"{"padding": 0, "padding-left": 4, "padding-right": "2ch"}"#).unwrap();
        assert_eq!(t.common.padding_left, Some(Length::Ch(4)));
        assert_eq!(t.common.padding_right, Some(Length::Ch(2)));
        assert!(t.common.padding.is_some());
    }
}
