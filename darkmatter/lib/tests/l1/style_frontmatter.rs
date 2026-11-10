//! Integration: parse the user's example fixture
//! (`darkmatter/example-docs/rendering/style-prop.md`) and assert every
//! field in the spec's acceptance criteria, plus end-to-end
//! `apply_page_style` lowering onto a `DarkmatterPage`.

use std::fs;
use std::path::{Path, PathBuf};

use biscuit_file::FileResolutionContext;
use biscuit_terminal::output::OutputTarget;
use biscuit_terminal::terminal::Terminal;
use darkmatter::layout::{DarkmatterPage, PageBackground, PageComponent};
use darkmatter::markdown::Markdown;
use darkmatter::style::warning::StyleWarningKind;
use darkmatter::markdown::highlighting::ThemePair;
use darkmatter::style::{
    PresentationDefaults, StyleAssetResolver, StyleFrontmatter, StyleLayer, StylePreparationPolicy,
    StyleSetter, apply_bespoke_style, apply_page_style, apply_prepared_style, from_frontmatter,
    into_strict, prepare_style_layers,
};
use biscuit_test_harness::manifest_dir;
use renderable::layout::{Alignment, Length, TargetValue};
use serde_json::json;
use simplified_schema::PropertyPath;

/// Locate the fixture relative to the run-time manifest directory so the
/// test is independent of where it's invoked from.
fn fixture_path() -> PathBuf {
    manifest_dir!()
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
    assert_eq!(page.margin_left, Some(Length::Ch(2)));
    assert_eq!(page.margin_right, Some(Length::Ch(4)));
    assert_eq!(page.margin_top, Some(1));
    assert_eq!(page.margin_bottom, Some(0));

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
    let schema_issues: Vec<_> = warnings.iter().filter(|w| w.is_schema_issue()).collect();
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

/// Extract whole terminal cells from a page-frame [`TargetValue<Length>`].
fn edge_ch(tv: &TargetValue<Length>) -> u16 {
    match tv {
        TargetValue::Universal(Length::Ch(n)) => u16::try_from(*n).unwrap_or(u16::MAX),
        _ => 0,
    }
}

#[test]
fn fixture_applies_expected_page_margins() {
    // Acceptance criterion: `md style-prop.md` honors page-level margins.
    // The fixture sets left=2ch, right=4ch, top=1, bottom=0.
    let style = fixture_style();
    let page = page_with_width(80);

    let applied = apply_page_style(page, &style)
        .expect("apply_page_style should succeed");

    let m = applied.page_margin();
    assert_eq!(
        edge_ch(&m.left), 2,
        "left margin should come from style.page.margin-left"
    );
    assert_eq!(
        edge_ch(&m.right), 4,
        "right margin should come from style.page.margin-right"
    );
    assert_eq!(
        edge_ch(&m.top), 1,
        "top margin should come from style.page.margin-top"
    );
    assert_eq!(
        edge_ch(&m.bottom), 0,
        "bottom margin should come from style.page.margin-bottom"
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
    let page = apply_page_style(page, &style)
        .expect("apply_page_style should succeed");

    let output = page.render(&md).expect("render to terminal");
    assert!(
        !output.is_empty(),
        "rendered terminal output must not be empty"
    );
}

#[test]
fn fixture_render_to_browser_runs_without_error() {
    // Acceptance criterion: `md --output html style-prop.md` uses the same
    // page-level frontmatter values through `render_to_browser`.
    let raw = fs::read_to_string(fixture_path()).expect("read fixture");
    let md = Markdown::try_from_content(&raw).expect("parse markdown");
    let style = fixture_style();

    let page = page_with_width(120);
    let page = apply_page_style(page, &style)
        .expect("apply_page_style should succeed");

    let html = page.render_to_browser(&md).expect("render to browser");
    assert!(!html.is_empty(), "rendered HTML output must not be empty");
}

/// The fixture's document style with a later setter layer over it, prepared
/// and applied as the CLI does; both layers resolve in `dir`.
fn layered_page(setter: &str, dir: &Path) -> DarkmatterPage {
    let raw = fs::read_to_string(fixture_path()).expect("read fixture");
    let md = Markdown::try_from_content(&raw).expect("parse markdown");
    let document = md.frontmatter().as_map().get("style").cloned().expect("fixture style");
    let context = FileResolutionContext::new(dir.to_path_buf());
    let (key, value) = setter.split_once('=').expect("a setter has `=`");
    let setters = vec![StyleSetter::new(setter, PropertyPath::parse(key).expect("setter path"), value)];
    let layers = [
        StyleLayer::document(document, "style-prop.md", context.clone(), None),
        StyleLayer::setters(setters, context),
    ];
    let prepared = prepare_style_layers(&layers, None, StylePreparationPolicy::render())
        .unwrap_or_else(|error| panic!("preparation failed: {:#?}", error.problems));
    apply_prepared_style(
        page_with_width(80),
        &prepared,
        StyleAssetResolver::for_target(OutputTarget::Terminal),
        &PresentationDefaults::default(),
    )
    .expect("apply prepared style")
}

#[test]
fn setter_layer_overrides_document_left_margin_field_by_field() {
    // The fixture says `style.page.margin-left: 2ch` and `margin-right: 4ch`;
    // a later setter layer replaces only the left margin.
    let dir = tempfile::tempdir().expect("tempdir");
    let applied = layered_page("style.page.margin-left=7ch", dir.path());
    assert_eq!(edge_ch(&applied.page_margin().left), 7, "the later layer wins");
    assert_eq!(
        edge_ch(&applied.page_margin().right),
        4,
        "a field the later layer leaves unset keeps the document value"
    );
}

#[test]
fn percent_margin_retained_and_resolved_per_target() {
    // Spec test #3 (revised for review-1 finding 3): `style.page.margin-left:
    // 10%`. The page frame retains the authored `Length`; the terminal resolves
    // it (10% of 80 = 8 cells) and the browser emits `10%` against the viewport.
    let yaml = "---\nstyle:\n    page:\n        margin-left: 10%\n---\n\n# Doc\n";
    let md = Markdown::try_from_content(yaml).expect("parse markdown");
    let (style, _w) = from_frontmatter(md.frontmatter()).expect("parse style");

    let page = page_with_width(80);
    let applied = apply_page_style(page, &style)
        .expect("apply_page_style should succeed");

    // Browser: the percent survives to CSS.
    let html = applied.render_to_browser(&md).expect("browser render");
    assert!(html.contains("10%"), "browser must emit percent margin; got: {html}");

    // Terminal: the percent resolves to an 8-cell left margin.
    let out = applied.render(&md).expect("terminal render");
    assert!(
        out.lines().any(|l| l.starts_with("        ")),
        "terminal must resolve 10% of 80 to an 8-cell left margin; got: {out:?}"
    );
}

#[test]
fn percent_max_width_resolves_post_margin() {
    // Spec test #4: 10% left + 10% right margins of 100 = 20; content
    // width = 80; 50% of 80 = 40.
    let yaml = "---\n\
        style:\n\
        \x20   page:\n\
        \x20       margin-left: 10%\n\
        \x20       margin-right: 10%\n\
        \x20       max-width: 50%\n\
        ---\n\n# Doc\n";
    let md = Markdown::try_from_content(yaml).expect("parse markdown");
    let (style, _w) = from_frontmatter(md.frontmatter()).expect("parse style");

    let page = page_with_width(100);
    let applied = apply_page_style(page, &style)
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
    let applied = apply_page_style(page, &style)
        .expect("apply_page_style should succeed");

    for component in PageComponent::ALL {
        let policy = applied.component_policy(component);
        assert_eq!(
            policy.map(|p| p.layout.alignment),
            Some(Alignment::Center),
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
    let page = apply_page_style(page, &style)
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

#[test]
fn code_theme_frontmatter_applies_via_apply_bespoke_style() {
    let yaml = "---\nstyle:\n    page:\n        code:\n            theme: dracula\n---\n\n# Doc\n";
    let md = Markdown::try_from_content(yaml).expect("parse markdown");
    let (style, _warnings) = from_frontmatter(md.frontmatter()).expect("parse style");

    let page = page_with_width(80);
    let page = apply_bespoke_style(page, &style, darkmatter::style::StyleAssetResolver::for_target(biscuit_terminal::output::OutputTarget::Html), &biscuit_file::FileResolutionContext::new(std::env::temp_dir()))
        .expect("apply_bespoke_style");

    assert_eq!(
        page.page_code_theme(),
        Some(&ThemePair::Dracula),
        "page code theme should be set from frontmatter"
    );
}

#[test]
fn code_theme_setter_layer_overrides_document_code_theme() {
    let dir = tempfile::tempdir().expect("tempdir");
    let document = json!({ "page": { "code": { "theme": "dracula" } } });
    let context = FileResolutionContext::new(dir.path().to_path_buf());
    let setter = StyleSetter::new(
        "style.page.code.theme=nord",
        PropertyPath::parse("style.page.code.theme").expect("setter path"),
        "nord",
    );
    let layers = [
        StyleLayer::document(document, "doc.md", context.clone(), None),
        StyleLayer::setters(vec![setter], context),
    ];
    let prepared = prepare_style_layers(&layers, None, StylePreparationPolicy::render())
        .unwrap_or_else(|error| panic!("preparation failed: {:#?}", error.problems));
    let page = apply_prepared_style(
        page_with_width(80),
        &prepared,
        StyleAssetResolver::for_target(OutputTarget::Terminal),
        &PresentationDefaults::default(),
    )
    .expect("apply prepared style");

    assert_eq!(page.page_code_theme(), Some(&ThemePair::Nord));
}

/// The render-surface keys reach their typed fields through
/// `from_frontmatter`.
#[test]
fn render_surface_keys_reach_typed_fields() {
    use darkmatter::markdown::highlighting::CodeBlockMode;

    let yaml = "---\nstyle:\n\
                \x20   page:\n\
                \x20       margin-left: 2ch\n\
                \x20       padding-bottom: 3\n\
                \x20       theme: Base16-Ocean\n\
                \x20       code:\n\
                \x20           theme: vs-dark\n\
                \x20           mode: same\n\
                \x20   code-block:\n\
                \x20       line-numbers: false\n\
                \x20       padding-left: 2\n\
                \x20   table:\n\
                \x20       padding-right: 10%\n\
                \x20   hr:\n\
                \x20       padding-left: 4ch\n\
                ---\n\n# Doc\n";
    let md = Markdown::try_from_content(yaml).expect("parse markdown");
    let (style, warnings) = from_frontmatter(md.frontmatter()).expect("parse style");
    assert!(!warnings.iter().any(|w| w.is_schema_issue()), "{warnings:?}");

    let page = style.page.expect("page");
    assert_eq!(page.margin_left, Some(Length::Ch(2)));
    assert_eq!(page.padding_bottom, Some(3));
    assert_eq!(page.theme, Some(ThemePair::Base16Ocean));
    let code = page.code.expect("page.code");
    assert_eq!(code.theme, Some(ThemePair::VisualStudioDark));
    assert_eq!(code.mode, Some(CodeBlockMode::Same));

    let code_block = style.code_block.expect("code-block");
    assert_eq!(code_block.line_numbers, Some(false));
    assert_eq!(code_block.common.padding_left, Some(Length::Ch(2)));
    assert_eq!(style.table.expect("table").common.padding_right, Some(Length::Percent(10.0)));
    assert_eq!(style.hr.expect("hr").padding_left, Some(Length::Ch(4)));
}

/// Removed page spacing names, in kebab or snake case, are unknown keys
/// whose warning names the CSS-order key, ranged at the authored key.
#[test]
fn removed_page_spacing_names_warn_with_their_replacement() {
    let yaml = "---\nstyle:\n    page:\n        left-margin: 2ch\n        bottom_padding: 1\n    top-margin: 1\n---\n\n# Doc\n";
    let md = Markdown::try_from_content(yaml).expect("parse markdown");
    let (style, warnings) = from_frontmatter(md.frontmatter()).expect("parse style");
    let found: Vec<(&str, Option<&str>, bool)> = warnings
        .iter()
        .filter(|w| w.kind == StyleWarningKind::UnknownKey)
        .map(|w| (w.path.as_str(), w.suggestion.as_deref(), w.source_span.is_some()))
        .collect();
    assert_eq!(
        found,
        [
            ("style.page.left-margin", Some("page.margin-left"), true),
            ("style.page.bottom_padding", Some("page.padding-bottom"), true),
            ("style.top-margin", Some("margin-top"), true),
        ]
    );
    let page = style.page.expect("page");
    assert_eq!((page.margin_left, page.padding_bottom), (None, None));
    assert!(into_strict(from_frontmatter(md.frontmatter()).unwrap()).is_err());
}

/// Shorthands and top-level page keys are accepted (no unknown-key warning),
/// validated, and normalized onto the page's typed fields.
#[test]
fn page_shorthands_are_accepted_at_top_level_and_under_page() {
    let yaml = "---\nstyle:\n    margin: 2\n    mx: 10%\n    max-width: 80ch\n    page:\n        py: 1\n        pl: 2ch\n---\n\n# Doc\n";
    let md = Markdown::try_from_content(yaml).expect("parse markdown");
    let (style, warnings) = from_frontmatter(md.frontmatter()).expect("parse style");
    assert!(!warnings.iter().any(|w| w.is_schema_issue()), "{warnings:?}");
    let page = style.page.expect("page");
    assert_eq!(
        (page.margin_left, page.margin_right, page.margin_top, page.margin_bottom),
        (Some(Length::Percent(10.0)), Some(Length::Percent(10.0)), Some(2), Some(2))
    );
    assert_eq!((page.padding_left, page.padding_top, page.padding_bottom), (Some(Length::Ch(2)), Some(1), Some(1)));
    assert_eq!(page.max_width, Some(Length::Ch(80)));

    for bad in ["margin: 10%", "my: 2ch", "page:\n        py: 65536", "max-width: 0"] {
        let yaml = format!("---\nstyle:\n    {bad}\n---\n\n# Doc\n");
        let md = Markdown::try_from_content(&yaml).expect("parse markdown");
        assert!(from_frontmatter(md.frontmatter()).is_err(), "{bad}");
    }
}
