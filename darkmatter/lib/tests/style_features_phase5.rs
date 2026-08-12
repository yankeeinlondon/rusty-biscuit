//! Phase 5 integration coverage for Darkmatter's browser feature pipeline
//! (`darkmatter/features/2026-07-09-features/`).
//!
//! Every fixture is network-free and inspects strings only — no remote asset is
//! ever fetched. The full-page interactive default and its CSP/version contract
//! are pinned in `style_features_baseline.rs`; these tests cover dedup, body-only
//! wrapper injection, popover-in-body, Markdown-family neutrality, and the
//! explicit Mermaid controls (`Text` → code, `image_mode = Never` → code).
//!
//! Browser snapshots cover both output shapes (spec acceptance criterion 12):
//! the standalone full-page document (`full_page_standalone_document_snapshot`,
//! from `render_to_browser_document`) and the embeddable body-only wrapper
//! fragment (`mermaid_body_only_wrapper_snapshot`, from `render_to_browser`).
//! The wrapper is a single valid element — it never nests a
//! `<!DOCTYPE>`/`<html>`/`<head>`/`<body>` inside itself.
//!
//! The two public browser methods have a content-independent return shape:
//! `render_to_browser` is always body-only (bare body when undecorated and
//! feature-free, forced wrapper fragment otherwise), and
//! `render_to_browser_document` is always a complete standalone document. The
//! `content_independent_*` tests below pin that contract for both.

use biscuit_terminal::terminal::Terminal;
use darkmatter::layout::DarkmatterPage;
use darkmatter::markdown::Markdown;
use darkmatter::markdown::output::terminal::{MermaidMode, TerminalImageMode};

const MERMAID_DOC: &str = "```mermaid\ngraph TD; A --> B\n```\n";

/// Two Mermaid fences in one document — the dedup fixture.
const TWO_MERMAID_DOC: &str =
    "```mermaid\ngraph TD; A --> B\n```\n\n```mermaid\nsequenceDiagram\n  A->>B: hi\n```\n";

const PROMPTED_LINK_DOC: &str = "[Home](https://example.com \"prompt='go home'\")\n";

/// A code-block document with no feature request and default page layout — the
/// standalone full-page fixture.
const CODE_DOC: &str = "# Title\n\n```rust\nfn demo() {}\n```\n";

fn page(width: u32) -> DarkmatterPage {
    let term = Terminal::new_optimistic(width);
    DarkmatterPage::new(&term)
}

/// Asserts that `wrapper` is a single embeddable element: it opens with the
/// `<div class="darkmatter-page"…>` wrapper and contains **no** nested
/// document-level element (`<!DOCTYPE>`, `<html>`, `<head>`, or `<body>`). This
/// is the structural guarantee the body-only path must uphold — the wrapper
/// carries the rendered Markdown and inline assets, not an embedded document.
fn assert_body_only_wrapper(wrapper: &str) {
    assert!(
        wrapper.trim_start().starts_with(r#"<div class="darkmatter-page""#),
        "a body-only render must open with the page wrapper div, got: {wrapper}"
    );
    for forbidden in ["<!DOCTYPE", "<!doctype", "<html", "<head", "<body"] {
        assert!(
            !wrapper.contains(forbidden),
            "the body-only wrapper must not nest a `{forbidden}` element, got: {wrapper}"
        );
    }
}

// ---------------------------------------------------------------------------
// Criterion 12 — full-page (standalone) snapshot
// ---------------------------------------------------------------------------

/// Pins the standalone full-page document form: a default-layout page with no
/// requested feature returns the complete `<!DOCTYPE html>…` document from
/// `DarkmatterPage::render_to_browser_document`'s no-wrapper path, carrying the
/// design-token `:root` block and the `.code-block` panel stylesheet in
/// `<head>`. This is the "full-page" half of criterion 12's snapshot coverage
/// and guards the standalone document path against regressions.
#[test]
fn full_page_standalone_document_snapshot() {
    let md = Markdown::try_from_content(CODE_DOC).expect("parse code doc");
    let html = page(80)
        .render_to_browser_document(&md)
        .expect("browser render");

    assert!(
        html.starts_with("<!DOCTYPE html><html><head>"),
        "the no-wrapper path emits a standalone document, got: {html}"
    );
    assert!(
        !html.contains(r#"<div class="darkmatter-page""#),
        "a default-layout feature-free page adds no wrapper, got: {html}"
    );
    insta::assert_snapshot!("full_page_standalone_document", html);
}

// ---------------------------------------------------------------------------
// Criterion 12 — body-only wrapper (fragment) snapshot
// ---------------------------------------------------------------------------

/// Pins the exact bytes of the body-only Mermaid wrapper: the forced
/// `<div class="darkmatter-page" …>` open tag with its `data-darkmatter-features`
/// stamp, the embedded page-level `<style>` (design tokens + `.code-block`), the
/// injected Mermaid bootstrap `<script>` (CSP/version contract + `themeVariables`
/// palette), and the rendered `<pre class="mermaid">` body — all inside one
/// wrapper with no nested document. Strings only — no remote asset is fetched.
#[test]
fn mermaid_body_only_wrapper_snapshot() {
    let md = Markdown::try_from_content(MERMAID_DOC).expect("parse mermaid doc");
    let html = page(80).render_to_browser(&md).expect("browser render");

    assert_body_only_wrapper(&html);
    insta::assert_snapshot!("mermaid_body_only_wrapper", html);
}

/// The body-only wrapper is a valid single element: it holds the rendered
/// Markdown plus the inline feature `<style>`/`<script>` assets (emitted before
/// the body), and never nests an `<html>`/`<head>`/`<body>`/`<!DOCTYPE>`.
#[test]
fn body_only_wrapper_has_no_nested_document_and_orders_assets_before_body() {
    let md = Markdown::try_from_content(MERMAID_DOC).expect("parse mermaid doc");
    let html = page(80).render_to_browser(&md).expect("browser render");

    assert_body_only_wrapper(&html);

    // The rendered Markdown body survives inside the wrapper.
    let body_at = html
        .find(r#"<pre class="mermaid">"#)
        .expect("the rendered mermaid body must ride the wrapper");
    assert!(
        html.contains("graph TD; A --&gt; B"),
        "the diagram source survives in the body, got: {html}"
    );

    // The inline feature assets are injected *before* the body content so the
    // feature's own declarations are present when the body renders.
    let style_at = html
        .find("<style>")
        .expect("an inline feature/page style must be embedded in the wrapper");
    let script_at = html
        .find(r#"<script type="module">"#)
        .expect("the mermaid bootstrap script must be embedded in the wrapper");
    assert!(
        style_at < body_at && script_at < body_at,
        "inline `<style>`/`<script>` assets must precede the body, \
         style@{style_at} script@{script_at} body@{body_at}: {html}"
    );

    // The wrapper closes exactly once.
    assert_eq!(
        html.matches(r#"<div class="darkmatter-page""#).count(),
        1,
        "exactly one wrapper element, got: {html}"
    );
}

// ---------------------------------------------------------------------------
// Content-independent return shape of both public browser methods
// ---------------------------------------------------------------------------

/// A default-layout, feature-free document that carries a heading — the shared
/// fixture for the content-independence pair below.
const HEADING_DOC: &str = "# Heading One\n\nBody text.\n";

/// `render_to_browser` on undecorated, feature-free content is a **bare body**:
/// it carries no document scaffold (`<!DOCTYPE>`/`<html>`/`<head>`/`<body>`) and
/// no `darkmatter-page` wrapper, yet still contains the rendered Markdown. This
/// pins the body-only half of the content-independent contract — the method
/// never emits a full document regardless of content.
#[test]
fn content_independent_render_to_browser_is_bare_body() {
    let md = Markdown::try_from_content(HEADING_DOC).expect("parse heading doc");
    let html = page(80).render_to_browser(&md).expect("browser render");

    for forbidden in ["<!DOCTYPE", "<!doctype", "<html", "<head", "<body"] {
        assert!(
            !html.contains(forbidden),
            "a bare body-only render must not emit `{forbidden}`, got: {html}"
        );
    }
    assert!(
        !html.contains(r#"<div class="darkmatter-page""#),
        "a feature-free undecorated render adds no wrapper, got: {html}"
    );
    assert!(
        html.contains("Heading One"),
        "the rendered Markdown heading survives in the bare body, got: {html}"
    );
}

/// `render_to_browser_document` on the *same* undecorated, feature-free input is
/// a **complete standalone document**: it opens with `<!DOCTYPE html>` and
/// carries the head assets (`:root` design tokens, `.code-block` panel
/// stylesheet). This pins the full-document half of the content-independent
/// contract — the method always emits a scaffolded document regardless of
/// content.
#[test]
fn content_independent_render_to_browser_document_is_full_document() {
    let md = Markdown::try_from_content(HEADING_DOC).expect("parse heading doc");
    let html = page(80)
        .render_to_browser_document(&md)
        .expect("browser render");

    assert!(
        html.starts_with("<!DOCTYPE html>"),
        "the document form must open with a doctype, got: {html}"
    );
    assert!(
        html.contains("<head>") && html.contains(":root"),
        "the document form carries the head design-token assets, got: {html}"
    );
    assert!(
        html.contains("Heading One"),
        "the rendered Markdown heading survives in the document body, got: {html}"
    );
}

// ---------------------------------------------------------------------------
// Standalone document — decorated page assembles a REAL, ordered <head>
// ---------------------------------------------------------------------------

/// Splits `html` into its `<head>` and `<body>` inner content, asserting the
/// document scaffold is well-formed.
fn split_head_body(html: &str) -> (&str, &str) {
    assert!(
        html.starts_with("<!DOCTYPE html><html><head>"),
        "a standalone document must open the scaffold, got: {html}"
    );
    let head = html
        .split_once("<head>")
        .and_then(|(_, rest)| rest.split_once("</head>"))
        .map(|(head, _)| head)
        .expect("document must have a <head>…</head>");
    let body = html
        .split_once("<body>")
        .and_then(|(_, rest)| rest.split_once("</body>"))
        .map(|(body, _)| body)
        .expect("document must have a <body>…</body>");
    (head, body)
}

/// A decorated standalone document (page margins/padding/background, a page
/// `<meta>` tag, and a **remote** stylesheet) assembles a real, non-empty
/// `<head>` — not the old empty `<head></head>`. The head carries, in order, the
/// render-tree head (charset/viewport/title + design-token `:root` block +
/// `.code-block` panel stylesheet) followed by the page `<meta>` and the remote
/// `<link rel="stylesheet">`. The `<body>` holds only the `.darkmatter-page`
/// frame and rendered content — no `<meta>`, no `<link>`, and no design-token
/// `<style>` leaks into the body.
#[test]
fn decorated_standalone_document_has_ordered_head_and_wrapper_only_body() {
    use darkmatter::layout::PageBackground;
    use darkmatter::style::bespoke::{MetaTag, PageMeta, PageStylesheet};

    let md = Markdown::try_from_content(HEADING_DOC).expect("parse heading doc");
    let html = page(80)
        .with_margin(2)
        .with_padding(1)
        .with_page_background(PageBackground::Subtle)
        .with_page_meta(PageMeta {
            tags: vec![MetaTag::Name {
                name: "author".into(),
                content: "Ken".into(),
            }],
        })
        .with_stylesheet(PageStylesheet::Remote {
            href: "https://example.com/app.css".into(),
        })
        .render_to_browser_document(&md)
        .expect("browser render");

    let (head, body) = split_head_body(&html);

    assert!(!head.is_empty(), "the decorated document head must be non-empty");

    // Render-tree head first: charset/viewport/title, then the design-token
    // `:root` block and the `.code-block` panel stylesheet.
    let charset_at = head.find("<meta charset").expect("head carries charset");
    assert!(head.contains("<title>"), "head carries a title, got: {head}");
    let root_at = head.find(":root").expect("head carries the :root design tokens");
    assert!(
        head.contains(".code-block{"),
        "the `.code-block` panel stylesheet rides the head, got: {head}"
    );

    // Then the page-authored meta and remote stylesheet link.
    let meta_at = head
        .find(r#"<meta name="author" content="Ken" />"#)
        .expect("page <meta> rides the head");
    let link_at = head
        .find(r#"<link rel="stylesheet" href="https://example.com/app.css" />"#)
        .expect("remote stylesheet <link> rides the head");
    assert!(
        charset_at < root_at && root_at < meta_at && meta_at < link_at,
        "head order is render-tree head (charset→:root) then page meta then remote link, got: {head}"
    );

    // The body holds only the frame + content: no metadata, no stylesheet link,
    // and no design-token `<style>` leaked from the head.
    assert!(
        body.contains(r#"<div class="darkmatter-page""#),
        "the body holds the page frame, got: {body}"
    );
    assert!(!body.contains("<meta "), "no <meta> in the body, got: {body}");
    assert!(
        !body.contains(r#"<link rel="stylesheet""#),
        "no remote stylesheet <link> in the body, got: {body}"
    );
    assert!(
        !body.contains(":root"),
        "the design-token block must not leak into the body, got: {body}"
    );
    assert!(
        body.contains("Heading One"),
        "the rendered content rides the body, got: {body}"
    );
}

/// A feature-bearing standalone document requesting **both** Mermaid and Popover
/// places each feature's assets in the real `<head>` exactly once — the Mermaid
/// ESM bootstrap `<script type="module">` and the Popover CSS — while the
/// `<body>` holds the rendered content (the `<pre class="mermaid">` container and
/// the accessible popover markup) with no feature `<script>`/`<style>` inside it.
#[test]
fn feature_bearing_standalone_document_places_feature_assets_in_head() {
    const MERMAID_AND_POPOVER_DOC: &str = concat!(
        "```mermaid\ngraph TD; A --> B\n```\n\n",
        "[Home](https://example.com \"prompt='go home'\")\n",
    );

    let md = Markdown::try_from_content(MERMAID_AND_POPOVER_DOC).expect("parse mermaid+popover doc");
    let html = page(80)
        .render_to_browser_document(&md)
        .expect("browser render");

    let (head, body) = split_head_body(&html);

    // Both feature assets land in the head, once each.
    assert_eq!(
        head.matches(r#"<script type="module">"#).count(),
        1,
        "the Mermaid ESM bootstrap rides the head exactly once, got: {head}"
    );
    assert_eq!(
        head.matches(".dm-popover-wrapper{").count(),
        1,
        "the Popover CSS rides the head exactly once, got: {head}"
    );

    // The body holds the rendered content, not the feature assets.
    assert!(
        body.contains(r#"<pre class="mermaid">"#),
        "the interactive Mermaid container rides the body, got: {body}"
    );
    assert!(
        body.contains(r#"class="dm-popover-wrapper""#),
        "the accessible popover markup rides the body, got: {body}"
    );
    assert!(
        !body.contains("<script"),
        "no feature <script> leaks into the body, got: {body}"
    );
    assert!(
        !body.contains(".dm-popover-wrapper{"),
        "no feature <style> leaks into the body, got: {body}"
    );
}

// ---------------------------------------------------------------------------
// Criterion 1 — dedup: two mermaid blocks inject exactly one bootstrap script
// ---------------------------------------------------------------------------

#[test]
fn two_mermaid_blocks_inject_one_module_script() {
    let md = Markdown::try_from_content(TWO_MERMAID_DOC).expect("parse two-mermaid doc");
    let html = page(80).render_to_browser(&md).expect("browser render");

    assert_eq!(
        html.matches(r#"<pre class="mermaid">"#).count(),
        2,
        "both fences emit an interactive container, got: {html}"
    );
    assert_eq!(
        html.matches(r#"<script type="module">"#).count(),
        1,
        "the mermaid bootstrap is injected exactly once, got: {html}"
    );
    assert_eq!(
        // The palette rides Mermaid's `themeVariables`, so the single injected
        // bootstrap carries exactly one `themeVariables` object — and no dead
        // `--mermaid-*` custom properties nothing reads.
        html.matches("themeVariables:{").count(),
        1,
        "the mermaid bootstrap (with its themeVariables) is injected once, got: {html}"
    );
    assert!(
        !html.contains("--mermaid-"),
        "no dead --mermaid-* custom properties are emitted, got: {html}"
    );
    assert_eq!(
        html.matches(r#"data-darkmatter-features="mermaid-diagram""#).count(),
        1,
        "the wrapper carries one stable feature stamp, got: {html}"
    );
}

// ---------------------------------------------------------------------------
// Criterion 5 / 7 — body-only wrapper forced for a popover feature
// ---------------------------------------------------------------------------

#[test]
fn prompted_link_forces_wrapper_with_popover_css_in_body() {
    let md = Markdown::try_from_content(PROMPTED_LINK_DOC).expect("parse prompted link");
    let html = page(80).render_to_browser(&md).expect("browser render");

    assert!(
        html.starts_with(r#"<div class="darkmatter-page" data-darkmatter-features="popover""#),
        "a popover feature forces the wrapper stamped with its name, got: {html}"
    );
    // The wrapper is a single valid element — no nested document.
    assert_body_only_wrapper(&html);

    // The popover CSS is injected inline in the wrapper, before the body's
    // accessible markup (which carries the `class="dm-popover-wrapper"` span).
    let body_markup_at = html
        .find(r#"class="dm-popover-wrapper""#)
        .expect("popover markup present in body");
    let css_at = html
        .find(".dm-popover-wrapper{")
        .expect("popover CSS present");
    assert!(
        css_at < body_markup_at,
        "popover CSS is injected before the body, got: {html}"
    );
    assert_eq!(
        html.matches(".dm-popover-wrapper{").count(),
        1,
        "the popover CSS is injected exactly once, got: {html}"
    );
    // The accessible markup still rides the body.
    assert!(
        html.contains(r#"popover="hint""#) && html.contains("interestfor="),
        "the accessible popover markup survives in the body, got: {html}"
    );
}

/// A feature-free page keeps its prior bytes: no wrapper, no feature stamp, no
/// injected feature assets (spec acceptance criterion 2b for the browser body
/// path). `render_to_browser` returns a bare body fragment here — the full
/// standalone document is `render_to_browser_document`'s job.
#[test]
fn feature_free_render_has_no_wrapper_or_assets() {
    let md = Markdown::try_from_content("Just a paragraph.\n").expect("parse plain doc");
    let html = page(80).render_to_browser(&md).expect("browser render");

    assert!(
        !html.contains("data-darkmatter-features"),
        "no feature stamp on a feature-free page, got: {html}"
    );
    assert!(
        !html.contains(r#"<div class="darkmatter-page""#),
        "no forced wrapper on a feature-free page, got: {html}"
    );
    assert!(
        !html.contains("<script") && !html.contains(".dm-popover-wrapper{"),
        "no injected feature assets, got: {html}"
    );
}

// ---------------------------------------------------------------------------
// Criterion 2 — Markdown-family neutrality: no feature assets ever
// ---------------------------------------------------------------------------

#[test]
fn markdown_plus_mermaid_output_has_no_feature_assets() {
    let md = Markdown::try_from_content(MERMAID_DOC).expect("parse mermaid doc");
    let out = page(80)
        .render_to_markdown_plus(&md)
        .expect("markdown-plus render");

    assert!(
        out.contains("graph TD"),
        "the mermaid fence survives as a fence, got: {out}"
    );
    assert!(
        !out.contains("<script")
            && !out.contains("data-darkmatter-features")
            && !out.contains("themeVariables"),
        "MarkdownPlus output carries no injected feature assets, got: {out}"
    );
}

// ---------------------------------------------------------------------------
// Criterion 4 / task 4 — explicit Mermaid controls stay opt-ins
// ---------------------------------------------------------------------------

#[test]
fn explicit_text_mermaid_mode_renders_code_not_interactive() {
    let md = Markdown::try_from_content(MERMAID_DOC).expect("parse mermaid doc");
    let html = page(80)
        .with_mermaid_mode(MermaidMode::Text)
        .render_to_browser(&md)
        .expect("browser render");

    assert!(
        !html.contains(r#"<pre class="mermaid">"#),
        "explicit Text mode keeps Mermaid as code, got: {html}"
    );
    assert!(
        !html.contains("data-darkmatter-features") && !html.contains("<script"),
        "explicit code mode requests no interactive feature, got: {html}"
    );
    assert!(
        html.contains("graph TD"),
        "the diagram source survives as code, got: {html}"
    );
}

#[test]
fn disabled_graphics_renders_mermaid_code_with_no_feature() {
    let md = Markdown::try_from_content(MERMAID_DOC).expect("parse mermaid doc");
    let html = page(80)
        .with_image_mode(TerminalImageMode::Never)
        .render_to_browser(&md)
        .expect("browser render");

    assert!(
        !html.contains(r#"<pre class="mermaid">"#),
        "GraphicsMode::Off caps interactive Mermaid to code, got: {html}"
    );
    assert!(
        !html.contains("data-darkmatter-features") && !html.contains("<script"),
        "disabled graphics requests no interactive feature, got: {html}"
    );
}
