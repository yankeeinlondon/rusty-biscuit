//! Phase 5 integration coverage for Darkmatter's browser feature pipeline
//! (`darkmatter/features/2026-07-09-features/`).
//!
//! Every fixture is network-free and inspects strings only — no remote asset is
//! ever fetched. The full-page interactive default and its CSP/version contract
//! are pinned in `style_features_baseline.rs`; these tests cover dedup, body-only
//! wrapper injection, popover-in-body, Markdown-family neutrality, and the
//! explicit Mermaid controls (`Text` → code, `image_mode = Never` → code).

use biscuit_terminal::terminal::Terminal;
use darkmatter::layout::DarkmatterPage;
use darkmatter::markdown::Markdown;
use darkmatter::markdown::output::terminal::{MermaidMode, TerminalImageMode};

const MERMAID_DOC: &str = "```mermaid\ngraph TD; A --> B\n```\n";

/// Two Mermaid fences in one document — the dedup fixture.
const TWO_MERMAID_DOC: &str =
    "```mermaid\ngraph TD; A --> B\n```\n\n```mermaid\nsequenceDiagram\n  A->>B: hi\n```\n";

const PROMPTED_LINK_DOC: &str = "[Home](https://example.com \"prompt='go home'\")\n";

fn page(width: u32) -> DarkmatterPage {
    let term = Terminal::new_optimistic(width);
    DarkmatterPage::new(&term)
}

/// The wrapper prefix Darkmatter injects **before** the embedded document body:
/// the `<div class="darkmatter-page" …>` open tag plus the resolved inline
/// feature `<style>`/`<script>`. Excludes the large design-token `:root`
/// preamble (which lives in the embedded document `<head>`), so the snapshot is
/// exactly the feature-injection surface.
fn feature_wrapper_prefix(html: &str) -> &str {
    let end = html.find("<!DOCTYPE html>").expect("embedded document body");
    &html[..end]
}

// ---------------------------------------------------------------------------
// Snapshot — the exact injected mermaid feature assets (CSP/version contract)
// ---------------------------------------------------------------------------

/// Pins the exact bytes of the injected Mermaid feature assets: the forced
/// wrapper + `data-darkmatter-features` stamp, the one CSS variable block
/// (light + dark), and the one module bootstrap naming both CDN origins at the
/// exact pinned version. Strings only — no remote asset is fetched.
#[test]
fn mermaid_feature_assets_snapshot() {
    let md = Markdown::try_from_content(MERMAID_DOC).expect("parse mermaid doc");
    let html = page(80).render_to_browser(&md).expect("browser render");
    insta::assert_snapshot!("mermaid_feature_assets", feature_wrapper_prefix(&html));
}

// ---------------------------------------------------------------------------
// Criterion 1 — dedup: two mermaid blocks inject exactly one CSS + one script
// ---------------------------------------------------------------------------

#[test]
fn two_mermaid_blocks_inject_one_css_and_one_module_script() {
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
        html.matches("--mermaid-primary-color").count(),
        // The variable appears once in `:root` and once in the dark override —
        // both inside the single injected CSS block.
        2,
        "the mermaid CSS block is injected exactly once (root + dark), got: {html}"
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
    // The popover CSS is injected inline in the wrapper (before the body's
    // `<!DOCTYPE html>`), not in the embedded document `<head>`.
    let doctype_at = html.find("<!DOCTYPE html>").expect("document body present");
    let css_at = html
        .find(".dm-popover-wrapper{")
        .expect("popover CSS present");
    assert!(
        css_at < doctype_at,
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
/// injected assets (spec acceptance criterion 2b for the browser body path).
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
            && !out.contains("--mermaid-primary-color"),
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
