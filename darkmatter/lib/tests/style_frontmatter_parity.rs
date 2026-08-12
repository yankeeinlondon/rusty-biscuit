use biscuit_terminal::terminal::Terminal;
use darkmatter::layout::{ComponentPolicy, DarkmatterPage, PageBackground, PageComponent};
use darkmatter::markdown::Markdown;
use darkmatter::style::schema::hr::{HrAlignment, HrKind, HrWeight};
use darkmatter::style::schema::{CommonStyle, WidthOrMode};
use darkmatter::style::{
    BespokeStyleOverrides, ComponentStyleOverrides, DisclosureStyleOverrides, HrStyleOverrides,
    ListStyleOverrides, PageStyleOverrides, apply_bespoke_style, apply_color_style,
    apply_component_style, apply_disclosure_style, apply_hr_style, apply_list_style,
    apply_page_style, from_frontmatter, StyleParseError,
};
use renderable::layout::{Alignment, Edges, Length, TargetValue, Width};
use renderable::style::{
    Border, BorderSides, PaintColor, TextEmphasis, UnderlineStyle,
};
use renderable::wrap_policy::WordWrap;

const WIDTH: u32 = 80;

fn with_style(style_yaml: &str, body: &str) -> Markdown {
    let full = format!("---\nstyle:\n{}---\n\n{}", indent(style_yaml, 4), body);
    Markdown::try_from_content(&full).expect("parse markdown with style frontmatter")
}

fn without_frontmatter(body: &str) -> Markdown {
    Markdown::try_from_content(body).expect("parse markdown body")
}

fn indent(text: &str, spaces: usize) -> String {
    let prefix = " ".repeat(spaces);
    text.lines()
        .map(|line| {
            if line.trim().is_empty() {
                line.to_string()
            } else {
                format!("{}{}", prefix, line)
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

fn page() -> DarkmatterPage {
    let term = Terminal::new_optimistic(WIDTH);
    DarkmatterPage::new(&term)
}

fn frontmatter_page(md: &Markdown) -> DarkmatterPage {
    let (style, _warnings) = from_frontmatter(md.frontmatter()).expect("parse style frontmatter");
    let page = apply_page_style(page(), &style, PageStyleOverrides::default())
        .expect("apply page style");
    let page = apply_component_style(page, &style, ComponentStyleOverrides::default())
        .expect("apply component style");
    let page = apply_list_style(page, &style, ListStyleOverrides::default())
        .expect("apply list style");
    let page = apply_hr_style(page, &style, HrStyleOverrides::default())
        .expect("apply hr style");
    let page = apply_disclosure_style(page, &style, DisclosureStyleOverrides::default())
        .expect("apply disclosure style");
    let page = apply_color_style(page, &style).expect("apply color style");
    apply_bespoke_style(page, &style, BespokeStyleOverrides::default(), None)
        .expect("apply bespoke style")
}

fn assert_render_parity(
    style_yaml: &str,
    body: &str,
    manual_page: DarkmatterPage,
    expected_markdown_plus: &str,
) {
    let styled_md = with_style(style_yaml, body);
    let plain_md = without_frontmatter(body);
    let styled_page = frontmatter_page(&styled_md);

    let styled_terminal = styled_page.render(&styled_md).expect("styled terminal render");
    let manual_terminal = manual_page.render(&plain_md).expect("manual terminal render");
    assert_eq!(styled_terminal, manual_terminal, "terminal output differs");

    let styled_html = styled_page
        .render_to_browser(&styled_md)
        .expect("styled browser render");
    let manual_html = manual_page
        .render_to_browser(&plain_md)
        .expect("manual browser render");
    assert_eq!(styled_html, manual_html, "browser HTML differs");

    let styled_body_md = Markdown::new(styled_md.content());
    let styled_markdown = styled_page
        .render_to_markdown_plus(&styled_body_md)
        .expect("styled MarkdownPlus render");
    let manual_markdown = manual_page
        .render_to_markdown_plus(&plain_md)
        .expect("manual MarkdownPlus render");
    assert_eq!(styled_markdown, manual_markdown, "MarkdownPlus differs");
    assert_eq!(
        styled_markdown, expected_markdown_plus,
        "MarkdownPlus should degrade to structural Markdown without style frontmatter"
    );
}

fn paint(raw: &str) -> PaintColor {
    darkmatter::style::color::parse(raw)
        .expect("valid style color")
        .to_paint_color()
}

fn style_color(raw: &str) -> darkmatter::style::color::StyleColor {
    darkmatter::style::color::parse(raw).expect("valid style color")
}

fn fixed_width(ch: u32) -> Width {
    Width::Fixed(TargetValue::universal(Length::ch(ch)))
}

fn width_policy(width: Width) -> ComponentPolicy {
    let mut policy = ComponentPolicy::default();
    policy.layout.width = width;
    policy
}

fn table_policy() -> ComponentPolicy {
    let mut policy = ComponentPolicy::default();
    policy.layout.alignment = Alignment::Center;
    policy.layout.width = fixed_width(32);
    policy.color = Some(paint("red-500"));
    policy
}

fn block_quote_policy() -> ComponentPolicy {
    let mut policy = ComponentPolicy::default();
    policy.layout.margin = Edges::all(Length::ch(1));
    policy.layout.padding = Edges::all(Length::ch(1));
    policy.border = Some(Border {
        sides: BorderSides::All,
        ..Border::default()
    });
    policy.emphasis = Some(TextEmphasis {
        italic: true,
        underline: Some(UnderlineStyle::Straight),
        ..TextEmphasis::default()
    });
    policy.bg_color = Some(paint("zinc-100"));
    policy
}

fn list_policy() -> ComponentPolicy {
    let mut policy = ComponentPolicy::default();
    policy.layout.margin.left = TargetValue::universal(Length::ch(4));
    policy.layout.max_width = Some(TargetValue::universal(Length::ch(24)));
    policy.word_wrap = Some(WordWrap::Truncate(None));
    policy
}

fn code_block_policy() -> ComponentPolicy {
    let mut policy = ComponentPolicy::default();
    policy.layout.width = fixed_width(28);
    policy.layout.padding = Edges::all(Length::ch(1));
    policy.bg_color = Some(paint("slate-900"));
    policy.word_wrap = Some(WordWrap::WrapProse(Some(8), None));
    policy
}

fn ordered_list_policy() -> ComponentPolicy {
    let mut policy = ComponentPolicy::default();
    policy.layout.width = fixed_width(42);
    policy.layout.alignment = Alignment::Center;
    policy.layout.padding = Edges::all(Length::ch(1));
    policy.color = Some(paint("green-600"));
    policy
}

fn list_item_policy() -> ComponentPolicy {
    let mut policy = ComponentPolicy::default();
    policy.layout.max_width = Some(TargetValue::universal(Length::ch(26)));
    policy.emphasis = Some(TextEmphasis {
        bold: true,
        ..TextEmphasis::default()
    });
    policy
}

fn image_policy() -> ComponentPolicy {
    let mut policy = ComponentPolicy::default();
    policy.layout.width = fixed_width(24);
    policy.layout.alignment = Alignment::Center;
    policy.layout.margin = Edges::all(Length::ch(1));
    policy.color = Some(paint("purple-600"));
    policy
}

fn disclosure_policy() -> ComponentPolicy {
    let mut policy = ComponentPolicy::default();
    policy.layout.width = fixed_width(34);
    policy.layout.alignment = Alignment::Right;
    policy.layout.padding = Edges::all(Length::ch(1));
    policy.border = Some(Border {
        sides: BorderSides::All,
        ..Border::default()
    });
    policy.color = Some(paint("cyan-700"));
    policy
}

#[test]
fn table_style_frontmatter_matches_manual_component_policy() {
    let body = "| Name | Value |\n|:-----|------:|\n| Alpha | 42 |\n";
    let style = "\
table:
  alignment: center
  width: 32ch
  color: red-500
";
    let manual_page = page().with_component_policy(PageComponent::Tables, table_policy());

    assert_render_parity(
        style,
        body,
        manual_page,
        "| Name | Value |\n| :-- | --: |\n| Alpha | 42 |",
    );
}

#[test]
fn width_auto_frontmatter_matches_manual_component_policy() {
    let body = "| Name | Value |\n|:-----|------:|\n| Alpha | 42 |\n";
    let style = "\
table:
  width: auto
";
    let manual_page =
        page().with_component_policy(PageComponent::Tables, width_policy(Width::Auto));

    assert_render_parity(
        style,
        body,
        manual_page,
        "| Name | Value |\n| :-- | --: |\n| Alpha | 42 |",
    );
}

#[test]
fn width_fit_content_frontmatter_matches_manual_component_policy() {
    let body = "> Compact quote.\n";
    let style = "\
block-quote:
  width: fit-content
";
    let manual_page =
        page().with_component_policy(PageComponent::BlockQuotes, width_policy(Width::FitContent));

    assert_render_parity(style, body, manual_page, body.trim_end());
}

#[test]
fn invalid_width_mode_string_still_reports_path_aware_length_error() {
    let md = with_style(
        "\
table:
  width: max-content
",
        "| Name | Value |\n|:-----|------:|\n| Alpha | 42 |\n",
    );

    let err = from_frontmatter(md.frontmatter()).expect_err("invalid width string should fail");
    assert!(
        matches!(
            err,
            StyleParseError::InvalidLength { ref path, ref raw, .. }
                if path == "style.table.width" && raw == "max-content"
        ),
        "unexpected error: {err:?}"
    );
}

#[test]
fn width_mode_keywords_do_not_apply_to_max_width() {
    let md = with_style(
        "\
table:
  max-width: auto
",
        "| Name | Value |\n|:-----|------:|\n| Alpha | 42 |\n",
    );

    let err = from_frontmatter(md.frontmatter()).expect_err("max-width mode should fail");
    assert!(
        matches!(
            err,
            StyleParseError::InvalidLength { ref path, ref raw, .. }
                if path == "style.table.max-width" && raw == "auto"
        ),
        "unexpected error: {err:?}"
    );
}

#[test]
fn block_quote_style_frontmatter_matches_manual_component_policy() {
    let body = "> A quoted paragraph with enough words to exercise the styled block quote path.\n";
    let style = "\
block-quote:
  margin: 1ch
  padding: 1ch
  border: true
  emphasis:
    italic: true
    underline: straight
  bg-color: zinc-100
";
    let manual_page =
        page().with_component_policy(PageComponent::BlockQuotes, block_quote_policy());

    assert_render_parity(style, body, manual_page, body.trim_end());
}

#[test]
fn list_style_frontmatter_matches_manual_component_policy() {
    let body = "- A long unordered list item whose text is intentionally wide enough to trigger list wrapping policy.\n- Short item.\n";
    let style = "\
ul:
  left-margin: 4ch
  max-width: 24ch
  word-wrap: truncate
";
    let manual_page = page().with_component_policy(PageComponent::Ul, list_policy());

    assert_render_parity(style, body, manual_page, body.trim_end());
}

#[test]
fn code_block_style_frontmatter_matches_manual_component_policy() {
    let body = "```rust\nfn main() {}\n```\n";
    let style = "\
code-block:
  width: 28ch
  padding: 1ch
  bg-color: slate-900
  word-wrap: wrap
";
    let manual_page =
        page().with_component_policy(PageComponent::CodeBlocks, code_block_policy());

    assert_render_parity(style, body, manual_page, body.trim_end());
}

#[test]
fn ordered_list_and_list_item_frontmatter_match_manual_component_policy() {
    let body = "1. First ordered item with enough words to exercise the list item width policy.\n2. Second item.\n";
    let style = "\
ol:
  width: 42ch
  alignment: center
  padding: 1ch
  color: green-600
li:
  max-width: 26ch
  emphasis:
    bold: true
";
    let manual_page = page()
        .with_component_policy(PageComponent::Ol, ordered_list_policy())
        .with_component_policy(PageComponent::Li, list_item_policy());

    assert_render_parity(style, body, manual_page, body.trim_end());
}

#[test]
fn image_style_frontmatter_matches_manual_component_and_local_policy() {
    let body = "![Diagram alt text](./diagram.png)\n";
    let style = "\
images:
  width: 24ch
  alignment: center
  margin: 1ch
  color: purple-600
  local-style:
    width: 18ch
    color: rose-600
";
    let local_style = CommonStyle {
        width: Some(WidthOrMode::Width(fixed_width(18))),
        color: Some(style_color("rose-600")),
        ..CommonStyle::default()
    };
    let manual_page = page()
        .with_component_policy(PageComponent::Images, image_policy())
        .with_local_image_style(local_style);

    assert_render_parity(style, body, manual_page, body.trim_end());
}

#[test]
fn hyperlink_style_frontmatter_matches_manual_inline_policy() {
    let body = "[A long link label](https://example.com)\n\n[Local link](./local.md)\n";
    let style = "\
hyperlinks:
  width: 18ch
  color: blue-600
  local-style:
    max-width: 10ch
    bg-color: yellow-100
";
    let hyperlink_style = CommonStyle {
        width: Some(WidthOrMode::Width(fixed_width(18))),
        color: Some(style_color("blue-600")),
        ..CommonStyle::default()
    };
    let local_hyperlink_style = CommonStyle {
        width: Some(WidthOrMode::Width(fixed_width(18))),
        max_width: Some(Length::ch(10)),
        color: Some(style_color("blue-600")),
        bg_color: Some(style_color("yellow-100")),
        ..CommonStyle::default()
    };
    let manual_page = page()
        .with_hyperlink_style(hyperlink_style)
        .with_local_hyperlink_style(local_hyperlink_style);

    assert_render_parity(style, body, manual_page, body.trim_end());
}

#[test]
fn disclosure_style_frontmatter_matches_manual_component_policy() {
    let body = "::disclosure\nSummary\n::details\nHidden body text.\n::end-disclosure\n";
    let style = "\
disclosure:
  width: 34ch
  alignment: right
  padding: 1ch
  border: true
  color: cyan-700
";
    let manual_page =
        page().with_component_policy(PageComponent::Disclosure, disclosure_policy());

    assert_render_parity(
        style,
        body,
        manual_page,
        "<details><summary>Summary</summary>\n\n\nHidden body text.\n\n</details>",
    );
}

#[test]
fn hr_style_frontmatter_matches_manual_page_defaults() {
    let body = "Before\n\n---\n\nAfter\n";
    let style = "\
hr:
  width: 20ch
  alignment: center
  kind: waves
  weight: medium
  color: blue-500
";
    let manual_page = page()
        .with_hr_width("20ch")
        .with_hr_alignment(HrAlignment::Center)
        .with_hr_kind(HrKind::Waves)
        .with_hr_weight(HrWeight::Medium)
        .with_component_color(PageComponent::Hr, paint("blue-500"));

    assert_render_parity(style, body, manual_page, "Before\n\n---\n\nAfter");
}

#[test]
fn page_style_frontmatter_matches_manual_page_policy() {
    let body = "# Page\n\nPage-level color and background policy.\n";
    let style = "\
page:
  color: slate-900
  bg-color: amber-50
  background: subtle
";
    let manual_page = page()
        .with_page_color(paint("slate-900"))
        .with_page_bg_color(paint("amber-50"))
        .with_page_background(PageBackground::Subtle);

    assert_render_parity(style, body, manual_page, body.trim_end());
}
