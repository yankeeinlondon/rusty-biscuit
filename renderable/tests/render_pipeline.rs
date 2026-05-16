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
    assert_eq!(html, "<p>hello &amp; welcome</p>");
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
    let html = page.render();
    assert!(html.starts_with("<!DOCTYPE html><html><head><meta charset=\"utf-8\">"));
    assert!(html.contains("<title>My Page</title>"));
    assert!(html.contains("<h1>Welcome</h1>"));
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
    assert!(page.render().contains("<title>Derived Title</title>"));
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
    let body = BrowserFragment::new()
        .define_as_text_fragment("content")
        .finalize();
    let mut page = HtmlPage::from(body);
    page.apply_page_options(PageOptions {
        external_stylesheet: Some(std::path::PathBuf::from("assets/page.css")),
        ..PageOptions::default()
    });
    let html = page.render();
    assert!(html.contains(r#"<link rel="stylesheet" href="assets/page.css">"#));
    assert!(!html.contains("<style>"));
}
