//! Convert parsed CLI arguments into a neutral [`CliStyleClaims`] value.
//!
//! This is the single CLI-side location that knows about clap wrappers
//! (`PageBackgroundArg`, `PageAlignmentArg`, `CliFill`). It lowers them into
//! library/layout types so the precedence and override logic can live in
//! `darkmatter::style::cli_claims`.

use crate::args::{Cli, CliFill};
use darkmatter::style::{CliStyleClaims, FillClaim};

/// Build a [`CliStyleClaims`] from the parsed top-level CLI flags.
///
/// This function intentionally ignores subcommand fields; only the flags that
/// affect page layout and style rendering are relevant.
pub fn cli_style_claims(cli: &Cli) -> CliStyleClaims {
    CliStyleClaims {
        margin: cli.margin,
        margin_x: cli.mx,
        margin_y: cli.my,
        margin_top: cli.mt,
        margin_bottom: cli.mb,
        margin_left: cli.ml,
        margin_right: cli.mr,

        padding: cli.padding,
        padding_x: cli.px,
        padding_y: cli.py,
        padding_top: cli.pt,
        padding_bottom: cli.pb,
        padding_left: cli.pl,
        padding_right: cli.pr,

        page_background: cli.page_bg.map(Into::into),
        page_bg_color: cli.page_bg_color,
        max_width: cli.max_width,

        alignment: cli.alignment.map(Into::into),
        align_images: cli.align_images.map(Into::into),
        align_lists: cli.align_lists.map(Into::into),
        align_ul: cli.align_ul.map(Into::into),
        align_ol: cli.align_ol.map(Into::into),
        align_li: cli.align_li.map(Into::into),
        align_block_quotes: cli.align_block_quotes.map(Into::into),
        align_tables: cli.align_tables.map(Into::into),
        align_code_blocks: cli.align_code_blocks.map(Into::into),

        fill: cli.fill.as_ref().map(fill_claim_from_cli),
        fill_images: cli.fill_images.as_ref().map(fill_claim_from_cli),
        fill_lists: cli.fill_lists.as_ref().map(fill_claim_from_cli),
        fill_ul: cli.fill_ul.as_ref().map(fill_claim_from_cli),
        fill_ol: cli.fill_ol.as_ref().map(fill_claim_from_cli),
        fill_li: cli.fill_li.as_ref().map(fill_claim_from_cli),
        fill_block_quotes: cli.fill_block_quotes.as_ref().map(fill_claim_from_cli),
        fill_tables: cli.fill_tables.as_ref().map(fill_claim_from_cli),
        fill_code_blocks: cli.fill_code_blocks.as_ref().map(fill_claim_from_cli),

        code_theme: cli.code_theme,
    }
}

fn fill_claim_from_cli(fill: &CliFill) -> FillClaim {
    match fill {
        CliFill::Full => FillClaim::Full,
        CliFill::Pad(length) => FillClaim::Pad(length.clone()),
        CliFill::Indent(length) => FillClaim::Indent(length.clone()),
        CliFill::Max(length) => FillClaim::Max(length.clone()),
        CliFill::Explicit(length) => FillClaim::Explicit(length.clone()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;
    use darkmatter::layout::PageBackground;
    use renderable::layout::{Alignment, Length};

    fn cli_from(args: &[&str]) -> Cli {
        Cli::try_parse_from(args).expect("CLI should parse")
    }

    #[test]
    fn no_layout_flags_produces_empty_claims() {
        let cli = cli_from(&["md", "doc.md"]);
        let claims = cli_style_claims(&cli);
        assert_eq!(claims, CliStyleClaims::default());
    }

    #[test]
    fn margin_and_padding_flags_are_captured() {
        let cli = cli_from(&[
            "md",
            "doc.md",
            "--margin",
            "4",
            "--mx",
            "2",
            "--padding",
            "3",
            "--py",
            "1",
        ]);
        let claims = cli_style_claims(&cli);
        assert_eq!(claims.margin, Some(4));
        assert_eq!(claims.margin_x, Some(2));
        assert_eq!(claims.padding, Some(3));
        assert_eq!(claims.padding_y, Some(1));
    }

    #[test]
    fn page_background_and_color_are_captured() {
        let cli = cli_from(&["md", "doc.md", "--page-bg", "subtle", "--page-bg-color", "#1e1e23"]);
        let claims = cli_style_claims(&cli);
        assert_eq!(claims.page_background, Some(PageBackground::Subtle));
        assert!(claims.page_bg_color.is_some());
    }

    #[test]
    fn alignment_flags_are_captured() {
        let cli = cli_from(&[
            "md",
            "doc.md",
            "--alignment",
            "center",
            "--align-images",
            "left",
            "--align-ul",
            "right",
        ]);
        let claims = cli_style_claims(&cli);
        assert_eq!(claims.alignment, Some(Alignment::Center));
        assert_eq!(claims.align_images, Some(Alignment::Left));
        assert_eq!(claims.align_ul, Some(Alignment::Right));
    }

    #[test]
    fn fill_flags_are_captured() {
        let cli = cli_from(&[
            "md",
            "doc.md",
            "--fill",
            "pad=4",
            "--fill-tables",
            "max=60",
            "--fill-code-blocks",
            "explicit=40",
        ]);
        let claims = cli_style_claims(&cli);
        assert_eq!(claims.fill, Some(FillClaim::Pad(Length::ch(4))));
        assert_eq!(claims.fill_tables, Some(FillClaim::Max(Length::ch(60))));
        assert_eq!(
            claims.fill_code_blocks,
            Some(FillClaim::Explicit(Length::ch(40)))
        );
    }

    #[test]
    fn code_theme_flag_is_captured() {
        let cli = cli_from(&["md", "doc.md", "--code-theme", "dracula"]);
        let claims = cli_style_claims(&cli);
        assert!(claims.code_theme.is_some());
    }
}
