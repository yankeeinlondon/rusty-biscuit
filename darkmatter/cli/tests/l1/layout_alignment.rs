//! Page spacing, fill, and alignment setters resolved onto the page `md`
//! builds, in process: shorthands against sides, a broadcast object against a
//! later bucket setter, and the list group named bucket by bucket.

use crate::common;

use common::layout::*;
use common::style::{COMPONENT_BUCKETS, every_component, prepared, style_object};
use darkmatter::layout::PageComponent;
use renderable::layout::{Alignment, Length};

const LISTS: [&str; 3] = ["ul", "ol", "li"];

#[test]
fn layout_resolved_margin_shorthand_then_top_override() {
    // `style.margin=2 style.mt=0`: the shorthand sets all sides to 2 and the
    // side clears just the top, in either order within one layer.
    for args in [["fixture.md", "style.margin=2", "style.mt=0"], ["fixture.md", "style.mt=0", "style.margin=2"]] {
        let page = resolved_page(&args);
        let m = page.page_margin();
        assert_eq!(tv_cells(&m.top), 0, "{args:?}: style.mt=0 must override style.margin=2 on the top edge");
        assert_eq!(tv_cells(&m.bottom), 2, "{args:?}: style.margin=2 must apply to the bottom edge");
        assert_eq!(tv_cells(&m.left), 2, "{args:?}: style.margin=2 must apply to the left edge");
        assert_eq!(tv_cells(&m.right), 2, "{args:?}: style.margin=2 must apply to the right edge");
    }
}

#[test]
fn layout_resolved_margin_axis_then_side() {
    // Shorthand 4 everywhere, then the horizontal axis to 2, then the top to 1.
    let page = resolved_page(&["fixture.md", "style.margin=4", "style.mx=2", "style.mt=1"]);
    let m = page.page_margin();
    assert_eq!(tv_cells(&m.top), 1, "style.mt=1 overrides axis and shorthand on top");
    assert_eq!(tv_cells(&m.bottom), 4, "shorthand survives on bottom (no override)");
    assert_eq!(tv_cells(&m.left), 2, "style.mx=2 overrides shorthand on left");
    assert_eq!(tv_cells(&m.right), 2, "style.mx=2 overrides shorthand on right");
}

#[test]
fn layout_resolved_page_shorthands_under_page_match_the_top_level() {
    let top = resolved_page(&["fixture.md", "style.margin=4", "style.mx=2", "style.mt=1"]);
    let page = resolved_page(&["fixture.md", "style.page.margin=4", "style.page.mx=2", "style.page.mt=1"]);
    assert_eq!(page.page_margin(), top.page_margin());
}

#[test]
fn layout_resolved_padding_axis_then_side() {
    let page = resolved_page(&["fixture.md", "style.padding=4", "style.px=2", "style.pt=1"]);
    let p = page.page_padding();
    assert_eq!(tv_cells(&p.top), 1);
    assert_eq!(tv_cells(&p.bottom), 4);
    assert_eq!(tv_cells(&p.left), 2);
    assert_eq!(tv_cells(&p.right), 2);
}

#[test]
fn layout_resolved_page_side_keys_set_one_edge_each() {
    let page = resolved_page(&[
        "fixture.md",
        "style.page.margin-top=1",
        "style.page.margin-bottom=2",
        "style.page.margin-left=3",
        "style.page.margin-right=4",
        "style.page.padding-top=5",
        "style.page.padding-bottom=6",
        "style.page.padding-left=7",
        "style.page.padding-right=8",
    ]);
    let (m, p) = (page.page_margin(), page.page_padding());
    assert_eq!([&m.top, &m.bottom, &m.left, &m.right].into_iter().map(tv_cells).collect::<Vec<_>>(), [1, 2, 3, 4]);
    assert_eq!([&p.top, &p.bottom, &p.left, &p.right].into_iter().map(tv_cells).collect::<Vec<_>>(), [5, 6, 7, 8]);
}

#[test]
fn layout_resolved_fill_broadcast_then_component_specific() {
    // The old `--fill max=40 --fill-code-blocks max=30`: every bucket named in
    // one object, then a later setter for the code block alone.
    let broadcast = every_component(r#"{ "max-width": 40 }"#);
    let page = resolved_page(&["fixture.md", &broadcast, "style.code-block.max-width=30"]);
    assert_eq!(
        fill_for(&page, PageComponent::CodeBlocks),
        TestFill::Max(Length::ch(30)),
        "the code-block setter must override the broadcast"
    );
    // A hyperlink showed no change from any fill mode, and a rule keeps its
    // width in its own style (`layout_fill` checks the drawn rule).
    let others = COMPONENT_BUCKETS.into_iter().map(|(component, _)| component);
    let skipped = |component: &PageComponent| {
        matches!(component, PageComponent::CodeBlocks | PageComponent::Hyperlinks | PageComponent::Hr)
    };
    for component in others.filter(|component| !skipped(component)) {
        assert_eq!(fill_for(&page, component), TestFill::Max(Length::ch(40)), "{component:?} must still see the broadcast");
    }
}

#[test]
fn layout_resolved_alignment_global_then_component_specific() {
    let page = resolved_page(&["fixture.md", "style.alignment=center", "style.code-block.alignment=left"]);
    assert_eq!(
        alignment_for(&page, PageComponent::CodeBlocks),
        Alignment::Left,
        "the code-block alignment must override the broadcast"
    );
    for (component, _) in COMPONENT_BUCKETS.into_iter().filter(|(component, _)| *component != PageComponent::CodeBlocks) {
        assert_eq!(alignment_for(&page, component), Alignment::Center, "{component:?} must still see the broadcast");
    }
}

#[test]
fn layout_resolved_page_alignment_is_the_same_broadcast() {
    let page = resolved_page(&["fixture.md", "style.page.alignment=right"]);
    for (component, _) in COMPONENT_BUCKETS {
        assert_eq!(alignment_for(&page, component), Alignment::Right, "{component:?}");
    }
}

#[test]
fn layout_resolved_align_lists_group_then_granular_override() {
    // The old `--align-lists right --align-ul left`: the group names `ul`, `ol`,
    // and `li`, then a later setter overrides only `ul`.
    let group = style_object(&LISTS, r#"{ alignment: "right" }"#);
    let page = resolved_page(&["fixture.md", &group, "style.ul.alignment=left"]);
    assert_eq!(alignment_for(&page, PageComponent::Ul), Alignment::Left, "the ul setter must override the group");
    assert_eq!(alignment_for(&page, PageComponent::Ol), Alignment::Right, "Ol must still see the group");
    assert_eq!(alignment_for(&page, PageComponent::Li), Alignment::Right, "Li must still see the group");
    assert_eq!(alignment_for(&page, PageComponent::Tables), Alignment::default(), "the group names lists only");
}

#[test]
fn layout_resolved_fill_lists_group_then_granular_override() {
    // The old `--fill-lists max=40 --fill-ol max=30`.
    let group = style_object(&LISTS, r#"{ "max-width": 40 }"#);
    let page = resolved_page(&["fixture.md", &group, "style.ol.max-width=30"]);
    assert_eq!(fill_for(&page, PageComponent::Ul), TestFill::Max(Length::ch(40)), "Ul must still see the group");
    assert_eq!(fill_for(&page, PageComponent::Ol), TestFill::Max(Length::ch(30)), "the ol setter must override the group");
    assert_eq!(fill_for(&page, PageComponent::Li), TestFill::Max(Length::ch(40)), "Li must still see the group");
    assert_eq!(fill_for(&page, PageComponent::Tables), TestFill::Full, "the group names lists only");
}

#[test]
fn layout_resolved_max_width() {
    for setter in ["style.max-width=80", "style.page.max-width=80"] {
        let page = resolved_page(&["fixture.md", setter]);
        assert_eq!(page.max_width(), Some(80), "{setter}");
    }
}

#[test]
fn layout_prepared_line_numbers_setter_values() {
    // `style.code-block.line-numbers` takes an explicit boolean, and an absent
    // setter leaves the document's (here: none) in place.
    let line_numbers = |args: &[&str]| prepared("# Doc\n", args).1.style().code_block.as_ref().and_then(|code| code.line_numbers);
    assert_eq!(line_numbers(&["fixture.md", "style.code-block.line-numbers=true"]), Some(true));
    assert_eq!(line_numbers(&["fixture.md", "style.code-block.line-numbers=false"]), Some(false));
    assert_eq!(line_numbers(&["fixture.md"]), None);

    assert!(resolved_page(&["fixture.md", "style.code-block.line-numbers=true"]).line_numbers());
    assert!(!resolved_page(&["fixture.md", "style.code-block.line-numbers=false"]).line_numbers());
}

#[test]
fn layout_resolved_mt_alone_does_not_set_other_sides() {
    // `style.mt=3` alone must leave other edges at default (0); no implicit
    // bleed from shorthand.
    let page = resolved_page(&["fixture.md", "style.mt=3"]);
    let m = page.page_margin();
    assert_eq!(tv_cells(&m.top), 3);
    assert_eq!(tv_cells(&m.bottom), 0);
    assert_eq!(tv_cells(&m.left), 0);
    assert_eq!(tv_cells(&m.right), 0);
}
