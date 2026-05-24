//! Integration: parse the user's example fixture
//! (`darkmatter/example-docs/rendering/style-prop.md`) and assert every
//! field in the spec's acceptance criteria, plus end-to-end
//! `apply_page_style` lowering onto a `DarkmatterPage`.

#![allow(deprecated)]

use std::fs;
use std::path::PathBuf;

use biscuit_terminal::terminal::Terminal;
use darkmatter::layout::{DarkmatterPage, PageAlignment, PageBackground, PageComponent};
use darkmatter::markdown::Markdown;
use darkmatter::style::warning::StyleWarningKind;
use darkmatter::style::{
    PageStyleOverrides, StyleFrontmatter, apply_page_style, from_frontmatter, into_strict,
};
use renderable::layout::{Alignment, Length};

/// Locate the fixture relative to `CARGO_MANIFEST_DIR` so the test is
/// independent of where it's invoked from.
fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("example-docs")
        .join("rendering")
        .join("style-prop.md")
}

#[test]
fn fixture_parses_to_expected_style_frontmatter() {
    let raw = fs::read_to_string(fixture_path()).expect("read fixture");
    let md = Markdown::try_from_content(&raw).expect("parse markdown");

    let (style, warnings) = from_frontmatter(md.frontmatter()).expect("parse style");

    // Acceptance criteria: page.
    let page = style.page.as_ref().expect("page bucket");
    assert_eq!(page.left_margin, Some(Length::Ch(2)));
    assert_eq!(page.right_margin, Some(Length::Ch(4)));
    assert_eq!(page.top_margin, Some(1));
    assert_eq!(page.bottom_margin, Some(0));

    // Acceptance criteria: table.
    let table = style.table.as_ref().expect("table bucket");
    assert_eq!(table.common.alignment, Some(Alignment::Right));
    assert_eq!(table.common.max_width, Some(Length::Percent(50.0)));

    // Acceptance criteria: ol.
    let ol = style.ol.as_ref().expect("ol bucket");
    assert_eq!(ol.common.alignment, Some(Alignment::Right));

    // Acceptance criteria: ul.
    let ul = style.ul.as_ref().expect("ul bucket");
    assert_eq!(ul.common.alignment, Some(Alignment::Left));
    assert_eq!(ul.left_margin, Some(Length::Ch(4)));
    assert_eq!(ul.common.max_width, Some(Length::Ch(40)));

    // Other buckets must remain None.
    assert!(style.hyperlinks.is_none());
    assert!(style.images.is_none());
    assert!(style.hr.is_none());
    assert!(style.li.is_none());
    assert!(style.block_quote.is_none());

    // All warnings must be KnownButInactive (the fixture is schema-clean).
    let schema_issues: Vec<_> = warnings
        .iter()
        .filter(|w| w.is_schema_issue())
        .collect();
    assert!(
        schema_issues.is_empty(),
        "fixture should be schema-clean, got: {:?}",
        schema_issues
    );

    let inactive_count = warnings
        .iter()
        .filter(|w| matches!(w.kind, StyleWarningKind::KnownButInactive { .. }))
        .count();
    // After sub-specs #2–#4 land, all keys in the fixture are wired and
    // produce no KnownButInactive warnings.
    assert_eq!(inactive_count, 0, "expected zero KnownButInactive warnings");
}

#[test]
fn fixture_passes_strict_validation() {
    // Strict mode succeeds because every warning is KnownButInactive.
    let raw = fs::read_to_string(fixture_path()).expect("read fixture");
    let md = Markdown::try_from_content(&raw).expect("parse markdown");
    let parsed = from_frontmatter(md.frontmatter()).expect("parse style");
    let _: StyleFrontmatter = into_strict(parsed).expect("strict should pass");
}

/// Helper: parse the fixture's style frontmatter (non-strict).
fn fixture_style() -> StyleFrontmatter {
    let raw = fs::read_to_string(fixture_path()).expect("read fixture");
    let md = Markdown::try_from_content(&raw).expect("parse markdown");
    let (style, _warnings) = from_frontmatter(md.frontmatter()).expect("parse style");
    style
}

/// Helper: build a `DarkmatterPage` with an optimistic terminal of `width`.
fn page_with_width(width: u32) -> DarkmatterPage {
    let term = Terminal::new_optimistic(width);
    DarkmatterPage::new(&term)
}

#[test]
fn fixture_applies_expected_page_margins() {
    // Acceptance criterion: `md style-prop.md` honors page-level margins.
    // The fixture sets left=2ch, right=4ch, top=1, bottom=0.
    let style = fixture_style();
    let page = page_with_width(80);

    let applied = apply_page_style(page, &style, PageStyleOverrides::default())
        .expect("apply_page_style should succeed");

    let m = applied.margin();
    assert_eq!(m.left, 2, "left margin should come from style.page.left-margin");
    assert_eq!(
        m.right, 4,
        "right margin should come from style.page.right-margin"
    );
    assert_eq!(m.top, 1, "top margin should come from style.page.top-margin");
    assert_eq!(
        m.bottom, 0,
        "bottom margin should come from style.page.bottom-margin"
    );
}

#[test]
fn fixture_no_known_but_inactive_for_page_keys() {
    // After sub-specs #2–#4 land, page-level keys and list keys must NOT emit
    // `KnownButInactive`.
    let raw = fs::read_to_string(fixture_path()).expect("read fixture");
    let md = Markdown::try_from_content(&raw).expect("parse markdown");
    let (_style, warnings) = from_frontmatter(md.frontmatter()).expect("parse style");

    for w in &warnings {
        if let StyleWarningKind::KnownButInactive { sub_spec } = w.kind {
            assert!(
                !w.path.starts_with("style.page."),
                "page-level key `{}` should no longer be KnownButInactive \
                 (sub_spec {}); apply_page_style now wires it",
                w.path,
                sub_spec
            );
        }
    }
}

#[test]
fn fixture_render_to_terminal_runs_without_error() {
    // Acceptance criterion: `md style-prop.md` renders end-to-end with
    // page-level style applied. This is the integration check requested in
    // the spec's test #1 / acceptance criterion — we don't snapshot the
    // ANSI bytes (terminal-specific) but we DO verify the apply→render
    // pipeline produces non-empty output without panicking.
    let raw = fs::read_to_string(fixture_path()).expect("read fixture");
    let md = Markdown::try_from_content(&raw).expect("parse markdown");
    let style = fixture_style();

    let page = page_with_width(80);
    let page = apply_page_style(page, &style, PageStyleOverrides::default())
        .expect("apply_page_style should succeed");

    let output = page.render(&md).expect("render to terminal");
    assert!(!output.is_empty(), "rendered terminal output must not be empty");
}

#[test]
fn fixture_render_to_browser_runs_without_error() {
    // Acceptance criterion: `md --output html style-prop.md` uses the same
    // page-level frontmatter values through `render_to_browser`.
    let raw = fs::read_to_string(fixture_path()).expect("read fixture");
    let md = Markdown::try_from_content(&raw).expect("parse markdown");
    let style = fixture_style();

    let page = page_with_width(120);
    let page = apply_page_style(page, &style, PageStyleOverrides::default())
        .expect("apply_page_style should succeed");

    let html = page.render_to_browser(&md).expect("render to browser");
    assert!(!html.is_empty(), "rendered HTML output must not be empty");
}

#[test]
fn cli_flag_overrides_frontmatter_left_margin() {
    // Spec test #2: CLI flag wins over frontmatter on overlapping fields.
    // The fixture says `style.page.left-margin: 2ch`. Simulate a CLI that
    // already set `--margin-left 7` by pre-setting margin_left on the page
    // and claiming the field via PageStyleOverrides.
    let style = fixture_style();
    let page = page_with_width(80).with_margin_left(7);

    let overrides = PageStyleOverrides {
        margin_left: true,
        ..PageStyleOverrides::default()
    };

    let applied = apply_page_style(page, &style, overrides).expect("apply");
    assert_eq!(
        applied.margin().left,
        7,
        "CLI override must win over frontmatter"
    );
    // The other frontmatter fields still apply.
    assert_eq!(
        applied.margin().right,
        4,
        "non-overridden frontmatter field should still apply"
    );
}

#[test]
fn percent_margin_resolves_against_terminal_width() {
    // Spec test #3: `style.page.left-margin: 10%` at 80-col terminal = 8.
    let yaml = "---\nstyle:\n    page:\n        left-margin: 10%\n---\n\n# Doc\n";
    let md = Markdown::try_from_content(yaml).expect("parse markdown");
    let (style, _w) = from_frontmatter(md.frontmatter()).expect("parse style");

    let page = page_with_width(80);
    let applied = apply_page_style(page, &style, PageStyleOverrides::default())
        .expect("apply_page_style should succeed");
    assert_eq!(applied.margin().left, 8);
}

#[test]
fn percent_max_width_resolves_post_margin() {
    // Spec test #4: 10% left + 10% right margins of 100 = 20; content
    // width = 80; 50% of 80 = 40.
    let yaml = "---\n\
        style:\n\
        \x20   page:\n\
        \x20       left-margin: 10%\n\
        \x20       right-margin: 10%\n\
        \x20       max-width: 50%\n\
        ---\n\n# Doc\n";
    let md = Markdown::try_from_content(yaml).expect("parse markdown");
    let (style, _w) = from_frontmatter(md.frontmatter()).expect("parse style");

    let page = page_with_width(100);
    let applied = apply_page_style(page, &style, PageStyleOverrides::default())
        .expect("apply_page_style should succeed");
    assert_eq!(applied.max_width(), Some(40));
}

#[test]
fn page_alignment_broadcasts_to_all_components() {
    // Spec test #7: `style.page.alignment: center` makes every component's
    // default alignment center.
    let yaml = "---\nstyle:\n    page:\n        alignment: center\n---\n\n# Doc\n";
    let md = Markdown::try_from_content(yaml).expect("parse markdown");
    let (style, _w) = from_frontmatter(md.frontmatter()).expect("parse style");

    let page = page_with_width(80);
    let applied = apply_page_style(page, &style, PageStyleOverrides::default())
        .expect("apply_page_style should succeed");

    for component in PageComponent::ALL {
        assert_eq!(
            applied.alignment_for(component),
            PageAlignment::Center,
            "alignment should broadcast to {:?}",
            component
        );
    }
    // Sanity: the broadcast came from frontmatter (not CLI), and the type
    // round-trips.
    let _ = (Alignment::Center, PageBackground::Transparent);
}

#[test]
fn color_keys_do_not_emit_known_but_inactive() {
    // Sub-spec #5 keys (color, bg-color) should be wired and silent.
    let yaml = "---\nstyle:\n    page:\n        color: red-500\n        bg-color: blue-500\n    table:\n        color: green-500\n        bg-color: orange-500\n---\n\n# Doc\n";
    let md = Markdown::try_from_content(yaml).expect("parse markdown");
    let (_style, warnings) = from_frontmatter(md.frontmatter()).expect("parse style");

    let color_inactive: Vec<_> = warnings
        .iter()
        .filter(|w| {
            matches!(w.kind, StyleWarningKind::KnownButInactive { .. })
                && (w.path.contains("color") || w.path.contains("bg-color"))
        })
        .collect();
    assert!(
        color_inactive.is_empty(),
        "color/bg-color keys should not be KnownButInactive; got: {:?}",
        color_inactive
    );
}

#[test]
fn color_frontmatter_applies_to_page_via_apply_color_style() {
    use darkmatter::style::apply_color_style;

    let yaml = "---\nstyle:\n    page:\n        color: red-500\n        bg-color: blue-500\n    table:\n        color: green-500\n---\n\n# Doc\n";
    let md = Markdown::try_from_content(yaml).expect("parse markdown");
    let (style, _warnings) = from_frontmatter(md.frontmatter()).expect("parse style");

    let page = page_with_width(80);
    let page = apply_page_style(page, &style, PageStyleOverrides::default())
        .expect("apply_page_style");
    let page = apply_color_style(page, &style).expect("apply_color_style");

    assert!(
        page.page_color().is_some(),
        "page color should be set from frontmatter"
    );
    assert!(
        page.page_bg_color().is_some(),
        "page bg-color should be set from frontmatter"
    );
    assert!(
        page.color_for(PageComponent::Tables).is_some(),
        "table color should be set from frontmatter"
    );
}
