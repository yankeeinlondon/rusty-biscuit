//! Per-bucket schema structs for the `style:` frontmatter.
//!
//! The root [`StyleFrontmatter`] is defined here in later tasks; per-bucket
//! structs live in sibling files.

pub mod common;
pub mod components;
pub mod hr;
pub mod inline;
pub mod lists;
pub mod page;

pub use common::{
    CommonStyle, ComponentBorder, ComponentEdges, ComponentEmphasis, ComponentWordWrap,
    WidthOrMode,
};
pub use components::{BlockQuoteStyle, CodeBlockStyle, DisclosureStyle, TableStyle};
pub use hr::HrStyle;
pub use inline::{HyperlinkStyle, ImageStyle};
pub use lists::{LiStyle, OlStyle, UlStyle};
pub use page::{CodeStyle, PageStyle};

use serde::Deserialize;

/// Root of the `style:` frontmatter tree. Every bucket is `Option` so a
/// sparse user document does not materialize default values across the tree.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
pub struct StyleFrontmatter {
    pub page: Option<PageStyle>,
    pub table: Option<TableStyle>,
    pub hyperlinks: Option<HyperlinkStyle>,
    pub images: Option<ImageStyle>,
    pub hr: Option<HrStyle>,
    pub ul: Option<UlStyle>,
    pub ol: Option<OlStyle>,
    pub li: Option<LiStyle>,
    #[serde(alias = "block_quote")]
    pub block_quote: Option<BlockQuoteStyle>,
    pub disclosure: Option<DisclosureStyle>,
    #[serde(alias = "code_block")]
    pub code_block: Option<CodeBlockStyle>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use renderable::layout::Length;

    #[test]
    fn fully_empty_yields_default() {
        let s: StyleFrontmatter = serde_json::from_str("{}").unwrap();
        assert_eq!(s, StyleFrontmatter::default());
    }

    #[test]
    fn only_page_set_leaves_others_none() {
        let s: StyleFrontmatter =
            serde_json::from_str(r#"{"page": {"left-margin": "2ch"}}"#).unwrap();
        assert!(s.page.is_some());
        assert!(s.table.is_none());
        assert!(s.ul.is_none());
        assert!(s.ol.is_none());
        assert!(s.li.is_none());
        assert!(s.hr.is_none());
        assert!(s.hyperlinks.is_none());
        assert!(s.images.is_none());
        assert!(s.block_quote.is_none());
        assert!(s.disclosure.is_none());
        assert!(s.code_block.is_none());

        let page = s.page.unwrap();
        assert_eq!(page.left_margin, Some(Length::Ch(2)));
    }

    #[test]
    fn block_quote_canonical_kebab_works() {
        let json = r#"{"block-quote": {"max-width": "50%"}}"#;
        let s: StyleFrontmatter = serde_json::from_str(json).unwrap();
        assert!(s.block_quote.is_some());
    }

    #[test]
    fn block_quote_snake_case_alias_works() {
        let json = r#"{"block_quote": {"max-width": "50%"}}"#;
        let s: StyleFrontmatter = serde_json::from_str(json).unwrap();
        assert!(s.block_quote.is_some());
    }

    #[test]
    fn code_block_canonical_kebab_works() {
        let json = r#"{"code-block": {"margin": "2ch"}}"#;
        let s: StyleFrontmatter = serde_json::from_str(json).unwrap();
        assert!(s.code_block.is_some());
    }

    #[test]
    fn code_block_snake_case_alias_works() {
        let json = r#"{"code_block": {"padding": "1ch"}}"#;
        let s: StyleFrontmatter = serde_json::from_str(json).unwrap();
        assert!(s.code_block.is_some());
    }
}
