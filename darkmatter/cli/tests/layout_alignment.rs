mod common;

use common::layout::*;
use darkmatter::layout::PageComponent;
use renderable::layout::{Alignment, Length};

#[test]
fn layout_resolved_margin_shorthand_then_top_override() {
    // `-m 2 --mt 0`: shorthand sets all sides to 2, then --mt clears just the
    // top. The reviewer specifically called this out: precedence checks
    // should assert observable resolved behavior, not parse success.
    let page = resolved_page(&["fixture.md", "-m", "2", "--mt", "0"]);
    let m = page.page_margin();
    assert_eq!(tv_cells(&m.top), 0, "--mt 0 must override -m 2 on the top edge");
    assert_eq!(tv_cells(&m.bottom), 2, "-m 2 must apply to the bottom edge");
    assert_eq!(tv_cells(&m.left), 2, "-m 2 must apply to the left edge");
    assert_eq!(tv_cells(&m.right), 2, "-m 2 must apply to the right edge");
}

#[test]
fn layout_resolved_margin_axis_then_side() {
    // `-m 4 --mx 2 --mt 1`: shorthand 4 everywhere, then horizontal axis to 2,
    // then top to 1.
    let page = resolved_page(&["fixture.md", "-m", "4", "--mx", "2", "--mt", "1"]);
    let m = page.page_margin();
    assert_eq!(tv_cells(&m.top), 1, "--mt 1 overrides axis and shorthand on top");
    assert_eq!(tv_cells(&m.bottom), 4, "shorthand survives on bottom (no override)");
    assert_eq!(tv_cells(&m.left), 2, "--mx 2 overrides shorthand on left");
    assert_eq!(tv_cells(&m.right), 2, "--mx 2 overrides shorthand on right");
}

#[test]
fn layout_resolved_padding_axis_then_side() {
    let page = resolved_page(&["fixture.md", "--padding", "4", "--px", "2", "--pt", "1"]);
    let p = page.page_padding();
    assert_eq!(tv_cells(&p.top), 1);
    assert_eq!(tv_cells(&p.bottom), 4);
    assert_eq!(tv_cells(&p.left), 2);
    assert_eq!(tv_cells(&p.right), 2);
}

#[test]
fn layout_resolved_fill_global_then_component_specific() {
    // `--fill max=40 --fill-code-blocks max=30`: global fill applies to all
    // components, then code-block-specific fill overrides only that one.
    let page = resolved_page(&[
        "fixture.md",
        "--fill",
        "max=40",
        "--fill-code-blocks",
        "max=30",
    ]);
    assert_eq!(
        fill_for(&page, PageComponent::CodeBlocks),
        TestFill::Max(Length::ch(30)),
        "code-block-specific fill must override global"
    );
    for component in [PageComponent::Ul, PageComponent::Ol, PageComponent::Li] {
        assert_eq!(
            fill_for(&page, component),
            TestFill::Max(Length::ch(40)),
            "{:?} must still see the global fill",
            component
        );
    }
    assert_eq!(
        fill_for(&page, PageComponent::Tables),
        TestFill::Max(Length::ch(40)),
        "tables must still see the global fill"
    );
}

#[test]
fn layout_resolved_alignment_global_then_component_specific() {
    let page = resolved_page(&[
        "fixture.md",
        "--alignment",
        "center",
        "--align-code-blocks",
        "left",
    ]);
    assert_eq!(
        alignment_for(&page, PageComponent::CodeBlocks),
        Alignment::Left,
        "code-block-specific alignment must override global"
    );
    for component in [PageComponent::Ul, PageComponent::Ol, PageComponent::Li] {
        assert_eq!(
            alignment_for(&page, component),
            Alignment::Center,
            "{:?} must still see the global alignment",
            component
        );
    }
    assert_eq!(
        alignment_for(&page, PageComponent::BlockQuotes),
        Alignment::Center,
        "blockquotes must still see the global alignment"
    );
}



#[test]
fn layout_resolved_align_lists_broadcast_then_granular_override() {
    // `--align-lists right --align-ul left`: broadcast sets all three list
    // components to Right, then the granular flag overrides only Ul.
    let page = resolved_page(&[
        "fixture.md",
        "--align-lists",
        "right",
        "--align-ul",
        "left",
    ]);
    assert_eq!(
        alignment_for(&page, PageComponent::Ul),
        Alignment::Left,
        "granular --align-ul must override broadcast"
    );
    assert_eq!(
        alignment_for(&page, PageComponent::Ol),
        Alignment::Right,
        "Ol must still see the broadcast"
    );
    assert_eq!(
        alignment_for(&page, PageComponent::Li),
        Alignment::Right,
        "Li must still see the broadcast"
    );
}

#[test]
fn layout_resolved_fill_lists_broadcast_then_granular_override() {
    // `--fill-lists max=40 --fill-ol max=30`: broadcast sets all three list
    // components to Max(40), then the granular flag overrides only Ol.
    let page = resolved_page(&[
        "fixture.md",
        "--fill-lists",
        "max=40",
        "--fill-ol",
        "max=30",
    ]);
    assert_eq!(
        fill_for(&page, PageComponent::Ul),
        TestFill::Max(Length::ch(40)),
        "Ul must still see the broadcast"
    );
    assert_eq!(
        fill_for(&page, PageComponent::Ol),
        TestFill::Max(Length::ch(30)),
        "granular --fill-ol must override broadcast"
    );
    assert_eq!(
        fill_for(&page, PageComponent::Li),
        TestFill::Max(Length::ch(40)),
        "Li must still see the broadcast"
    );
}

#[test]
fn layout_resolved_max_width() {
    let page = resolved_page(&["fixture.md", "--max-width", "80"]);
    assert_eq!(page.max_width(), Some(80));
}

#[test]
fn layout_parsed_line_numbers_flag_values() {
    // `--line-numbers` (no value) defaults to true; `--line-numbers false`
    // explicitly disables. Verified against the parsed CLI struct since the
    // CLI's `render_terminal_output` applies this flag separately from the
    // layout-flag pipeline.
    assert_eq!(
        parse_cli(&["fixture.md", "--line-numbers"]).line_numbers,
        Some(true)
    );
    assert_eq!(
        parse_cli(&["fixture.md", "--line-numbers", "true"]).line_numbers,
        Some(true)
    );
    assert_eq!(
        parse_cli(&["fixture.md", "--line-numbers", "false"]).line_numbers,
        Some(false)
    );
    assert_eq!(parse_cli(&["fixture.md"]).line_numbers, None);
}

#[test]
fn layout_resolved_mt_alone_does_not_set_other_sides() {
    // `--mt 3` alone must leave other edges at default (0); no implicit
    // bleed from shorthand.
    let page = resolved_page(&["fixture.md", "--mt", "3"]);
    let m = page.page_margin();
    assert_eq!(tv_cells(&m.top), 3);
    assert_eq!(tv_cells(&m.bottom), 0);
    assert_eq!(tv_cells(&m.left), 0);
    assert_eq!(tv_cells(&m.right), 0);
}

