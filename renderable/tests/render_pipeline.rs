use renderable::browser::PageOptions;
use renderable::browser::fragment::BrowserFragment;
use renderable::html::HtmlPage;
use renderable::html::tag::BlockTag;

#[test]
fn fragment_renders_block_with_text_child() {
    let fragment = BrowserFragment::new()
        .define_as_block_tag(BlockTag::P, "intro")
        .add_child(renderable::browser::fragment::ComposableNode::TextFragment(
            "hello & welcome".to_string(),
        ))
        .finalize();
    let html = fragment.render();
    assert_eq!(html, r#"<p class="intro">hello &amp; welcome</p>"#);
}

#[test]
fn raw_html_is_not_escaped() {
    let fragment = BrowserFragment::new()
        .define_as_raw_html("<svg><rect/></svg>")
        .finalize();
    assert_eq!(fragment.render(), "<svg><rect/></svg>");
}

#[test]
fn page_render_emits_doctype_charset_and_title() {
    let body = BrowserFragment::new()
        .define_as_block_tag(BlockTag::H1, "heading")
        .add_child(renderable::browser::fragment::ComposableNode::TextFragment(
            "Welcome".to_string(),
        ))
        .finalize();
    let mut page = HtmlPage::from(body);
    page.set_title("My Page");
    let html = page.render().expect("render");
    assert!(html.starts_with("<!DOCTYPE html><html><head><meta charset=\"utf-8\">"));
    assert!(html.contains("<title>My Page</title>"));
    assert!(html.contains(r#"<h1 class="heading">Welcome</h1>"#));
}

#[test]
fn title_falls_back_to_first_h1() {
    let body = BrowserFragment::new()
        .define_as_block_tag(BlockTag::H1, "heading")
        .add_child(renderable::browser::fragment::ComposableNode::TextFragment(
            "Derived Title".to_string(),
        ))
        .finalize();
    let page = HtmlPage::from(body);
    assert!(page.render().expect("render").contains("<title>Derived Title</title>"));
}

#[test]
fn page_emits_root_block_with_semantic_tokens() {
    let body = BrowserFragment::new()
        .define_as_text_fragment("content")
        .finalize();
    let page = HtmlPage::from(body);
    let css = page.stylesheet();
    assert!(css.contains("--color-bg:"));
    assert!(css.contains("--space-2:"));
    assert!(css.contains("--font-mono:"));
}

#[test]
fn page_options_override_a_css_variable() {
    let body = BrowserFragment::new()
        .define_as_text_fragment("content")
        .finalize();
    let mut page = HtmlPage::from(body);
    page.apply_page_options(PageOptions {
        css_variables: Some(vec![("color-bg".to_string(), "#000000".to_string())]),
        ..PageOptions::default()
    });
    assert!(page.stylesheet().contains("--color-bg: #000000;"));
}

#[test]
fn external_stylesheet_emits_a_link_not_inline_style() {
    use renderable::browser::RelativeAssetPath;

    let body = BrowserFragment::new()
        .define_as_text_fragment("content")
        .finalize();
    let mut page = HtmlPage::from(body);
    page.apply_page_options(PageOptions {
        external_stylesheet: Some(RelativeAssetPath::new("assets/page.css").unwrap()),
        ..PageOptions::default()
    });
    let html = page.render().expect("render");
    assert!(html.contains(r#"<link rel="stylesheet" href="assets/page.css">"#));
    assert!(!html.contains("<style>"));
}

#[test]
fn relative_asset_path_rejects_absolute_paths() {
    use renderable::browser::RelativeAssetPath;

    assert!(RelativeAssetPath::new("/etc/page.css").is_err());
    assert!(RelativeAssetPath::new("/var/app.js").is_err());
    assert!(RelativeAssetPath::new("assets/page.css").is_ok());
}

#[test]
fn nested_component_aux_state_rolls_up_to_page() {
    use renderable::browser::ComponentStylesheet;
    use renderable::browser::feature::PageFeature;
    use renderable::microdata::MicrodataKey;
    use renderable::stylesheet::CssStyle;

    let child = BrowserFragment::new()
        .define_as_block_tag(BlockTag::Span, "child")
        .with_stylesheet(ComponentStylesheet::new("child").add("label", CssStyle::new()))
        .add_metadata_keypair(MicrodataKey::Description, "child description")
        .add_feature(PageFeature::Popover)
        .finalize();

    let parent = BrowserFragment::new()
        .define_as_block_tag(BlockTag::Div, "parent")
        .add_component(child)
        .finalize();

    let page = HtmlPage::from(parent);

    // Stylesheet rollup: the nested component's scoped rule appears.
    assert!(page.stylesheet().contains(".child .label"));
    // Metadata rollup: the nested description reaches the page <head>.
    assert!(page.render().expect("render").contains("child description"));
    // Feature rollup through the nested component.
    assert!(page.features().contains(&PageFeature::Popover));
}

/// Regression guard for the Phase 4 rollup-hygiene refactor: `HtmlPage::render`
/// now collects the fragment tree once and shares that single walk across the
/// metadata, link, and stylesheet rollups. This proves every composition
/// channel still reaches the rendered page after that change.
///
/// Dependency-link rollup runs through the same shared traversal but is not
/// asserted here: `LinkTag` has no public constructor, so neither page links
/// nor fragment dependency links can be populated from an integration test.
#[test]
fn html_page_render_rolls_up_every_composition_channel() {
    use renderable::browser::ComponentStylesheet;
    use renderable::browser::feature::PageFeature;
    use renderable::microdata::MicrodataKey;
    use renderable::stylesheet::{CssRule, CssStyle, Stylesheet};

    // A nested component carries its own metadata, a scoped stylesheet, and a
    // page feature, all of which must bubble up through the parent fragment.
    let child = BrowserFragment::new()
        .define_as_block_tag(BlockTag::Span, "child")
        .with_stylesheet(ComponentStylesheet::new("child").add("label", CssStyle::new()))
        .add_metadata_keypair(MicrodataKey::Description, "child description")
        .add_feature(PageFeature::Popover)
        .finalize();
    let parent = BrowserFragment::new()
        .define_as_block_tag(BlockTag::Div, "parent")
        .add_component(child)
        .finalize();

    let mut page = HtmlPage::from(parent);
    page.set_title("Rollup Page");
    page.add_script_block("console.log('rollup');");

    let mut sheet = Stylesheet::new();
    sheet.push(CssRule::new(".page-rule", CssStyle::new()));
    page.apply_page_options(PageOptions {
        stylesheet: Some(sheet),
        css_variables: Some(vec![("color-bg".to_string(), "#101010".to_string())]),
        ..PageOptions::default()
    });

    let html = page.render().expect("render");

    // Page metadata (title) reached <head>.
    assert!(html.contains("<title>Rollup Page</title>"), "{html}");
    // Nested-component metadata reached <head>.
    assert!(html.contains("child description"), "{html}");
    // Page stylesheet rule rolled into the inline <style>.
    assert!(html.contains(".page-rule"), "{html}");
    // Nested component's scoped stylesheet rolled in.
    assert!(html.contains(".child .label"), "{html}");
    // CSS-variable override applied to the :root block.
    assert!(html.contains("--color-bg: #101010;"), "{html}");
    // Page-level script block rolled into the inline <script>.
    assert!(html.contains("console.log('rollup');"), "{html}");
    // The composed component body is present in <body>.
    assert!(html.contains(r#"<div class="parent">"#), "{html}");

    // Page-feature rollup is observed through the dedicated accessor, which
    // shares the same recursive fragment walk.
    assert!(
        page.features().contains(&PageFeature::Popover),
        "page feature rollup"
    );
}

#[test]
fn block_emits_base_class() {
    use renderable::browser::fragment::ComposableNode;

    let fragment = BrowserFragment::new()
        .define_as_block_tag(BlockTag::Div, "widget")
        .add_child(ComposableNode::TextFragment("hi".to_string()))
        .finalize();
    assert_eq!(fragment.render(), r#"<div class="widget">hi</div>"#);
}

#[test]
fn base_class_merges_with_explicit_class() {
    use renderable::html::attribute::ClassDefinition;
    use renderable::html::tag::HtmlAttribute;

    let fragment = BrowserFragment::new()
        .define_as_block_tag(BlockTag::Div, "widget")
        .add_attribute(HtmlAttribute::Class(ClassDefinition::new("active large")))
        .finalize();
    assert_eq!(
        fragment.render(),
        r#"<div class="widget active large"></div>"#
    );
}

#[test]
fn empty_base_class_yields_no_class_attribute() {
    let fragment = BrowserFragment::new()
        .define_as_block_tag(BlockTag::Div, "")
        .finalize();
    assert_eq!(fragment.render(), "<div></div>");
}

#[test]
fn id_and_style_are_rendered() {
    use renderable::html::attribute::DomId;
    use renderable::html::tag::HtmlAttribute;
    use renderable::stylesheet::{CssIntegerProp, CssSizing, CssSizingProp, CssStyle};

    // A two-declaration style exercises the `\n` → space collapse.
    let style = CssStyle::new()
        .add(CssSizingProp::Width, CssSizing::px(80.0))
        .add(CssIntegerProp::ZIndex, 5);
    let fragment = BrowserFragment::new()
        .define_as_block_tag(BlockTag::Span, "")
        .add_attribute(HtmlAttribute::Id(DomId::new("main")))
        .add_attribute(HtmlAttribute::Style(style))
        .finalize();
    let html = fragment.render();
    assert!(html.contains(r#"id="main""#), "{html}");
    assert!(
        html.contains(r#"style="width: 80px; z-index: 5;""#),
        "{html}"
    );
}

#[test]
fn boolean_attribute_present_when_true_absent_when_false() {
    use renderable::html::tag::{HtmlAttribute, VoidTag};

    let on = BrowserFragment::new()
        .define_as_void_tag(VoidTag::Input)
        .add_attribute(HtmlAttribute::Disabled(true))
        .finalize();
    assert_eq!(on.render(), "<input disabled>");

    let off = BrowserFragment::new()
        .define_as_void_tag(VoidTag::Input)
        .add_attribute(HtmlAttribute::Disabled(false))
        .finalize();
    assert_eq!(off.render(), "<input>");
}

#[test]
fn role_aria_and_data_attributes_are_rendered() {
    use renderable::html::attribute::HtmlDataAttribute;
    use renderable::html::attribute::aria::{AriaAttribute, AriaRole};
    use renderable::html::tag::HtmlAttribute;

    let fragment = BrowserFragment::new()
        .define_as_block_tag(BlockTag::Div, "")
        .add_attribute(HtmlAttribute::Role(AriaRole::AlertDialog))
        .add_attribute(HtmlAttribute::Aria(AriaAttribute::Expanded(true)))
        .add_attribute(HtmlAttribute::Data(
            HtmlDataAttribute::new("count"),
            "3".to_string(),
        ))
        .finalize();
    let html = fragment.render();
    assert!(html.contains(r#"role="alertdialog""#), "{html}");
    assert!(html.contains(r#"aria-expanded="true""#), "{html}");
    assert!(html.contains(r#"data-count="3""#), "{html}");
}

#[test]
fn attribute_values_are_escaped() {
    use renderable::html::tag::HtmlAttribute;

    let fragment = BrowserFragment::new()
        .define_as_block_tag(BlockTag::Div, "")
        .add_attribute(HtmlAttribute::Title(r#"a "quote" < & b"#.to_string()))
        .finalize();
    let html = fragment.render();
    assert!(
        html.contains(r#"title="a &quot;quote&quot; &lt; &amp; b""#),
        "{html}"
    );
}

#[test]
fn external_code_emits_a_script_src() {
    use renderable::browser::RelativeAssetPath;

    let body = BrowserFragment::new()
        .define_as_text_fragment("content")
        .finalize();
    let mut page = HtmlPage::from(body);
    page.apply_page_options(PageOptions {
        external_code: Some(RelativeAssetPath::new("assets/app.js").unwrap()),
        ..PageOptions::default()
    });
    let html = page.render().expect("render");
    assert!(
        html.contains(r#"<script src="assets/app.js" defer></script>"#),
        "{html}"
    );
}

#[test]
fn component_metadata_is_first_write_wins_and_page_overrides() {
    use renderable::microdata::MicrodataKey;

    let first = BrowserFragment::new()
        .define_as_text_fragment("a")
        .add_metadata_keypair(MicrodataKey::Description, "first description")
        .finalize();
    let second = BrowserFragment::new()
        .define_as_text_fragment("b")
        .add_metadata_keypair(MicrodataKey::Description, "second description")
        .finalize();
    let mut page = HtmlPage::from_fragments(vec![first, second]);

    // The earlier component wins a component-vs-component conflict.
    let html = page.render().expect("render");
    assert!(html.contains("first description"), "{html}");
    assert!(!html.contains("second description"), "{html}");

    // Page-level metadata overrides any component value.
    page.add_metadata(MicrodataKey::Description, "page description");
    let html = page.render().expect("render");
    assert!(html.contains("page description"), "{html}");
    assert!(!html.contains("first description"), "{html}");
}

#[test]
fn page_stylesheet_ships_palette_and_semantic_references() {
    let body = BrowserFragment::new()
        .define_as_text_fragment("x")
        .finalize();
    let css = HtmlPage::from(body).stylesheet();
    // Palette layer is present.
    assert!(css.contains("--color-blue-500:"), "{css}");
    // Semantic color tokens reference the palette, not raw hex.
    assert!(
        css.contains("--color-accent: var(--color-blue-500);"),
        "{css}"
    );
}

#[test]
fn render_html_page_returns_a_page_with_options_applied() {
    use renderable::browser::fragment::{BrowserFragment, Ready};
    use renderable::browser::{BrowserRenderable, PageOptions, RelativeAssetPath};
    use std::any::Any;

    #[derive(Debug)]
    struct Dummy;
    impl BrowserRenderable for Dummy {
        fn render_html_fragment(&self) -> BrowserFragment<Ready> {
            BrowserFragment::new()
                .define_as_raw_html("<p>x</p>")
                .finalize()
        }
        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    // No options: a plain page.
    let page = Dummy.render_html_page(None);
    assert!(page.render().expect("render").contains("<p>x</p>"));

    // Valid options apply; the operation is infallible because
    // RelativeAssetPath already guaranteed the path is relative.
    let page = Dummy.render_html_page(Some(PageOptions {
        external_stylesheet: Some(RelativeAssetPath::new("assets/page.css").unwrap()),
        ..PageOptions::default()
    }));
    assert!(page.render().expect("render").contains(r#"href="assets/page.css""#));
}

#[test]
fn composable_node_component_constructs_directly() {
    use renderable::browser::fragment::ComposableNode;

    let child = BrowserFragment::new()
        .define_as_text_fragment("hi")
        .finalize();
    let parent = BrowserFragment::new()
        .define_as_block_tag(BlockTag::Div, "wrap")
        .add_child(ComposableNode::Component(Box::new(child)))
        .finalize();
    assert!(parent.render().contains("hi"));
}
