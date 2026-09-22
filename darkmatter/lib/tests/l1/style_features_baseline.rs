//! Phase 1 baseline freeze for the Style Features implementation
//! (`darkmatter/features/2026-07-09-features/`).
//!
//! These pin Darkmatter behavior so every later phase can classify an output
//! change as intended or regressive:
//!
//! - Darkmatter's body-only browser fragment path
//!   (`DarkmatterPage::render_to_browser`) renders a Mermaid fence as
//!   **Interactive** by default (Phase 5): the body carries `<pre class="mermaid">`
//!   and reports a feature request; the outer page resolves it through
//!   `DarkmatterFeatureResolver` and injects one module bootstrap into the
//!   forced page wrapper. Mermaid emits no CSS; its palette rides the script's
//!   `themeVariables`. (`render_to_browser_document` is the companion standalone
//!   `<!DOCTYPE html>` document path the CLI uses for HTML output.)
//! - a feature-free, undecorated `render_to_browser` returns a body-only
//!   fragment — the bare body, with no `.darkmatter-page` wrapper and no feature
//!   assets — and a requested feature forces the wrapper only when present
//!   (Phase 5);
//! - the terminal Mermaid default keeps the fence as code — no network or
//!   subprocess I/O.
//!
//! The prompted-link fixtures below were flipped to their Phase-4 "after" state:
//! the production tree path now emits the accessible wrapper/anchor/prompt
//! structure with `interestfor` / `aria-describedby`, a `popover="hint"` prompt,
//! injected Popover CSS, and document-unique ids.
//!
//! All fixtures are network-free and deterministic.

use biscuit_terminal::terminal::Terminal;
use darkmatter::layout::DarkmatterPage;
use darkmatter::markdown::Markdown;
use darkmatter::markdown::output::HtmlOptions;

// ---------------------------------------------------------------------------
// Deterministic fixtures
// ---------------------------------------------------------------------------

const MERMAID_DOC: &str = "```mermaid\ngraph TD; A --> B\n```\n";

/// Two prompted links, identical target and prompt — the fixture behind the
/// Phase 4 "repeated identical links get unique IDs" criterion.
const TWO_PROMPTED_LINKS_DOC: &str = concat!(
    "[Home](https://example.com \"prompt='go home'\")\n\n",
    "[Home](https://example.com \"prompt='go home'\")\n",
);

fn page(width: u32) -> DarkmatterPage {
    let term = Terminal::new_optimistic(width);
    DarkmatterPage::new(&term)
}

// ---------------------------------------------------------------------------
// Full-page browser Mermaid default (Phase 5): Interactive
// ---------------------------------------------------------------------------

/// `DarkmatterPage::render_to_browser` renders a Mermaid fence as **Interactive**
/// by default: the body carries the escaped `<pre class="mermaid">` container and
/// reports a feature request. The outer page resolves that request and the
/// forced page wrapper carries one module bootstrap that names both CDN origins
/// with an exact (non-floating) version. Mermaid emits no CSS; the bootstrap
/// supplies its palette through `themeVariables`.
///
/// This is a Level-1 *string* assertion on the emitted markup (it never launches
/// a browser); the live-DOM behavior — module load, SVG replacement, CDN
/// fallback, theme application — is proven by the Browser-tier `browser_mermaid_*`
/// tests in `browser_render.rs`. The name deliberately avoids the `browser_`
/// substring so the nextest `test(/browser_/)` selector routes it to the L1
/// `just test` suite, not `just test-browser`.
#[test]
fn full_page_mermaid_default_markup_is_interactive() {
    let md = Markdown::try_from_content(MERMAID_DOC).expect("parse mermaid doc");
    let html = page(80).render_to_browser(&md).expect("browser render");

    assert!(
        html.contains(r#"<pre class="mermaid">"#),
        "the default full-page path emits the interactive container, got: {html}"
    );
    assert!(
        html.contains("graph TD"),
        "the diagram source survives (escaped) inside the container, got: {html}"
    );
    assert!(
        html.contains(r#"data-darkmatter-features="mermaid-diagram""#),
        "the wrapper is forced and stamped with the feature name, got: {html}"
    );
    assert!(
        html.contains(r#"<script type="module">"#),
        "the ESM bootstrap is injected, got: {html}"
    );
    assert_eq!(
        html.matches(r#"<script type="module">"#).count(),
        1,
        "exactly one bootstrap script is injected, got: {html}"
    );
    assert!(
        html.contains("cdn.jsdelivr.net") && html.contains("unpkg.com"),
        "both CDN origins are named, got: {html}"
    );
    assert!(
        html.contains("mermaid@11.6.0") && !html.contains("mermaid@11/"),
        "the pinned version is exact, not a floating major tag, got: {html}"
    );
}

// ---------------------------------------------------------------------------
// Feature-free page neutrality
// ---------------------------------------------------------------------------

/// A zero-configuration, feature-free `render_to_browser` returns a body-only
/// fragment — the bare body, with no `.darkmatter-page` wrapper and no feature
/// assets. A requested feature forces the wrapper into existence only when it
/// resolves to inline assets.
#[test]
fn bare_body_render_has_no_wrapper_and_no_feature_assets() {
    let md = Markdown::try_from_content("A plain paragraph.\n").expect("parse plain doc");
    let html = page(80).render_to_browser(&md).expect("browser render");

    assert!(
        !html.contains(r#"<div class="darkmatter-page""#),
        "a zero-config render must not wrap in the page div, got: {html}"
    );
    assert!(
        !html.contains("data-darkmatter-features"),
        "a feature-free render has no feature-debug attribute, got: {html}"
    );
    assert!(
        !html.contains("<script") && !html.contains(r#"<pre class="mermaid">"#),
        "a feature-free page injects no feature assets, got: {html}"
    );
}

// ---------------------------------------------------------------------------
// Prompted-link tree path (Phase 4)
// ---------------------------------------------------------------------------

/// A `prompt='…'` structured link lowers to the accessible wrapper/anchor/prompt
/// structure in the production tree path: the internal `data-prompt` transport is
/// consumed (never emitted), the anchor keeps its real `href` and gains
/// `interestfor` / `aria-describedby`, the prompt is a `popover="hint"` element
/// with escaped content, and the shared Popover CSS is injected once.
#[test]
fn prompted_link_tree_path_emits_accessible_popover() {
    let md = Markdown::try_from_content(
        "[Click](https://example.com \"prompt='hi <b> & bye'\")\n",
    )
    .expect("parse prompted link");
    let html = md.as_html(HtmlOptions::default()).expect("as_html");

    assert!(
        !html.contains("data-prompt="),
        "the internal transport attribute must not be emitted, got: {html}"
    );
    assert!(
        html.contains(r#"<span class="dm-popover-wrapper">"#),
        "the accessible wrapper is emitted, got: {html}"
    );
    assert!(
        html.contains(r#"href="https://example.com""#),
        "the real href is preserved, got: {html}"
    );
    assert!(
        html.contains("interestfor=") && html.contains("aria-describedby="),
        "the anchor carries the popover association, got: {html}"
    );
    assert!(
        html.contains(r#"popover="hint""#),
        "the prompt is a popover element, got: {html}"
    );
    assert!(
        html.contains("hi &lt;b&gt; &amp; bye") && !html.contains("hi <b>"),
        "prompt content is escaped, not raw, got: {html}"
    );
    assert_eq!(
        html.matches(".dm-popover-wrapper{").count(),
        1,
        "the Popover CSS is injected exactly once, got: {html}"
    );
}

/// Two identical prompted links get distinct, document-unique popover ids so the
/// `aria-describedby` / popover association never collides (Phase 4 unique-ID
/// requirement).
#[test]
fn repeated_prompted_links_get_unique_ids() {
    let md = Markdown::try_from_content(TWO_PROMPTED_LINKS_DOC).expect("parse two links");
    let html = md.as_html(HtmlOptions::default()).expect("as_html");

    assert_eq!(
        html.matches(r#"popover="hint""#).count(),
        2,
        "both prompted links emit a popover, got: {html}"
    );
    assert!(
        html.contains(r#"id="dm-popover-https-example-com""#)
            && html.contains(r#"id="dm-popover-https-example-com-1""#),
        "the two identical links get distinct ids, got: {html}"
    );
    assert_eq!(
        html.matches(".dm-popover-wrapper{").count(),
        1,
        "the Popover CSS is still injected only once, got: {html}"
    );
}

// ---------------------------------------------------------------------------
// Terminal Mermaid default (no network / subprocess)
// ---------------------------------------------------------------------------

/// The terminal default keeps a Mermaid fence as code — the diagram source is
/// rendered verbatim rather than rasterized, and no browser assets leak in. This
/// pins the side-effect-free default the spec preserves.
#[test]
fn terminal_mermaid_defaults_to_code() {
    let md = Markdown::try_from_content(MERMAID_DOC).expect("parse mermaid doc");
    let out = page(80).render(&md).expect("terminal render");

    assert!(
        out.contains("graph TD"),
        "the diagram source must render as code on the terminal, got: {out}"
    );
    assert!(
        !out.contains("<script") && !out.contains(r#"class="mermaid""#),
        "terminal output must carry no browser mermaid assets, got: {out}"
    );
}
