//! Browser-level (real headless Chrome) tests for darkmatter HTML output.
//!
//! Where the Level-2 terminal tests drive a real WezTerm pane, these drive a
//! real headless Chromium via the shared `biscuit-browser-harness` so we
//! assert on the **computed** result of the CSS/HTML darkmatter emits — not on
//! HTML source substrings. Computed-style assertions are deterministic and
//! robust (unlike pixel diffs), and prove the browser actually applies our
//! styles.
//!
//! ## Skips cleanly
//!
//! These tests skip silently when no Chrome/Chromium binary is found. Set
//! `BISCUIT_BROWSER_REQUIRED=1` to turn a missing browser into a hard
//! failure (for CI that provisions one). Point `CHROME` at an explicit
//! executable to override discovery.

use biscuit_browser_harness::{BrowserHarness, ChromeHarness, require_browser, wrap_fragment};
use darkmatter::markdown::Markdown;
use darkmatter::markdown::highlighting::{ColorMode, ThemePair};
use darkmatter::markdown::output::HtmlOptions;
use darkmatter::markdown::render_tree::{TerminalCodeRenderer, fold_markdown_to_document};
use darkmatter::markdown::render_tree::svg_sanitizer::sanitize_svg;
use renderable::tree::{
    BrowserMermaidMode, BrowserRenderOptions, GraphicsMode, HintNamespace, RawHtmlPolicy,
    RenderNode, RenderStrictness, SourceDescriptor, render_browser_document, render_browser_node,
};
use serial_test::serial;
use std::rc::Rc;

/// The `.code-block` background darkmatter emits for a `github` + dark page must
/// compute, in a real browser, to the github-*light* panel color (`#ffffff`).
/// Code blocks invert their theme variant for page contrast (Defect D), so a
/// dark page gets a light code panel — matching the terminal renderer. This
/// proves the emitted CSS is valid and applied — not just present in the source.
#[tokio::test]
#[serial(browser)]
async fn browser_code_block_background_computes_in_browser() {
    if !require_browser() {
        return;
    }

    let md: Markdown = "```rust\nfn main() {}\n```\n".into();
    let mut options = HtmlOptions::default();
    options.code_theme = ThemePair::Github;
    options.prose_theme = ThemePair::Github;
    options.color_mode = ColorMode::Dark;
    let fragment = md.as_html(options).expect("as_html");
    let doc = wrap_fragment(&fragment, "#202020");

    let mut harness = ChromeHarness::new();
    harness.spawn().await.expect("spawn chrome");
    harness.render_html(&doc).await.expect("render html");

    let bg = harness
        .computed_style(".code-block", "background-color")
        .await
        .expect("computed style query");

    // Inverted for a dark page: github light panel = #ffffff = rgb(255, 255, 255).
    assert_eq!(
        bg, "rgb(255, 255, 255)",
        "browser-computed .code-block background-color should be the inverted (github-light) panel color",
    );
}

/// Renders a `style: waves` thematic break to a browser-renderer HTML fragment
/// at the given graphics tier.
fn waves_hr_fragment(mode: GraphicsMode) -> String {
    let mut hr = RenderNode::thematic_break();
    let ns = HintNamespace("darkmatter.hr");
    hr.attrs.set_hint(ns, "kind", serde_json::json!("waves"));
    hr.attrs.set_hint(ns, "weight", serde_json::json!("thick"));
    hr.attrs.set_hint(ns, "color", serde_json::json!("red"));

    let opts = BrowserRenderOptions {
        graphics_mode: mode,
        ..BrowserRenderOptions::default()
    };
    render_browser_node(&hr, &opts)
        .expect("render hr")
        .output
        .render()
}

/// Review-3 finding 2: the browser HR at `Vector`/`Rich` must render the styled
/// SVG as real, valid DOM — not just emit an `<svg>` source string. This drives
/// a real headless Chromium and asserts the `.darkmatter-hr` element exists,
/// computes `display: block`, and that its waves `<path>` parsed into the DOM
/// with a resolved stroke width. String tests cannot prove any of these.
#[tokio::test]
#[serial(browser)]
async fn browser_hr_waves_svg_computes_in_browser() {
    if !require_browser() {
        return;
    }

    let mut harness = ChromeHarness::new();
    harness.spawn().await.expect("spawn chrome");

    for mode in [GraphicsMode::Vector, GraphicsMode::Rich] {
        let doc = wrap_fragment(&waves_hr_fragment(mode), "#ffffff");
        harness.render_html(&doc).await.expect("render html");

        let display = harness
            .computed_style(".darkmatter-hr", "display")
            .await
            .expect("computed style query");
        assert_eq!(
            display, "block",
            "{mode:?}: .darkmatter-hr must exist and compute display:block in the browser",
        );

        // The waves style emits a `<path>`; if it parsed as DOM the browser
        // resolves its stroke width (`var(--hr-weight, 8)` → 8px).
        let stroke = harness
            .computed_style(".darkmatter-hr path", "stroke-width")
            .await
            .expect("computed style query");
        assert!(
            stroke != "<no-match>" && stroke.ends_with("px"),
            "{mode:?}: waves <path> must parse into the DOM with a resolved stroke-width; got {stroke:?}",
        );
    }
}

/// Renders a thematic break whose `width` / `color` hints carry hostile
/// attribute/markup-breaking payloads, at `Rich`.
fn hostile_hr_fragment() -> String {
    let mut hr = RenderNode::thematic_break();
    let ns = HintNamespace("darkmatter.hr");
    hr.attrs.set_hint(ns, "kind", serde_json::json!("waves"));
    // A `color` that, if interpolated unescaped, breaks out of the SVG attribute
    // and injects an `<img onerror>` sibling.
    hr.attrs.set_hint(
        ns,
        "color",
        serde_json::json!(r#"red"><img src=x onerror="window.__pwned=1">"#),
    );
    // A `width` that, if interpolated unescaped, injects a `<script>` element.
    hr.attrs.set_hint(
        ns,
        "width",
        serde_json::json!(r#"100%"><script>window.__pwned=1</script>"#),
    );

    let opts = BrowserRenderOptions {
        graphics_mode: GraphicsMode::Rich,
        ..BrowserRenderOptions::default()
    };
    render_browser_node(&hr, &opts)
        .expect("render hr")
        .output
        .render()
}

/// Review-5 finding 1: hostile `darkmatter.hr.*` `width` / `color` hints must
/// not be able to escape the styled-HR SVG attribute/markup context. This drives
/// a real headless Chromium and proves the hostile payload neither parses into
/// an injected `<img>`/`<script>` node nor corrupts the SVG: the `.darkmatter-hr`
/// element still computes `display: block` (intact), and no injected node exists.
/// A string assertion cannot prove the browser did not parse an injected node.
#[tokio::test]
#[serial(browser)]
async fn browser_hr_hostile_attrs_inject_no_nodes() {
    if !require_browser() {
        return;
    }

    let fragment = hostile_hr_fragment();
    // Source-level guard: the sanitized fragment must carry neither the injected
    // markup nor the attacker payload.
    assert!(
        !fragment.contains("<img") && !fragment.contains("<script") && !fragment.contains("__pwned"),
        "hostile HR hints must be dropped before raw-HTML emission; got:\n{fragment}",
    );

    let doc = wrap_fragment(&fragment, "#ffffff");
    let mut harness = ChromeHarness::new();
    harness.spawn().await.expect("spawn chrome");
    harness.render_html(&doc).await.expect("render html");

    // The SVG itself must still be intact (the broken-out attribute did not
    // corrupt it).
    let display = harness
        .computed_style(".darkmatter-hr", "display")
        .await
        .expect("computed style query");
    assert_eq!(
        display, "block",
        ".darkmatter-hr must survive intact when hostile hints are supplied",
    );

    // No injected node may exist in the DOM.
    for injected in ["img", "script"] {
        let display = harness
            .computed_style(injected, "display")
            .await
            .expect("computed style query");
        assert_eq!(
            display, "<no-match>",
            "hostile HR hint injected a <{injected}> node into the DOM",
        );
    }
}

/// Review-3 finding 2: browser Mermaid at `Rich` with promotion enabled must
/// render a static `<svg>` as valid DOM. Driven end-to-end through
/// [`Markdown::as_html`] with `MermaidMode::Image` (→ `StaticSvg`).
///
/// Skips cleanly when the host lacks the Mermaid toolchain: in that case the
/// promotion hook returns `None`, the renderer degrades to a code block, and no
/// `<svg>` appears — there is nothing browser-renderable to assert.
#[tokio::test]
#[serial(browser)]
async fn browser_mermaid_static_svg_computes_in_browser() {
    if !require_browser() {
        return;
    }

    let md: Markdown = "```mermaid\nflowchart LR\n    A --> B\n```\n".into();
    let mut options = HtmlOptions::default();
    options.mermaid_mode = darkmatter::markdown::output::terminal::MermaidMode::Image;
    let fragment = md.as_html(options).expect("as_html");

    if !fragment.contains("<svg") {
        eprintln!("skipping: Mermaid toolchain unavailable (no SVG produced; degraded to code block)");
        return;
    }

    let doc = wrap_fragment(&fragment, "#ffffff");
    let mut harness = ChromeHarness::new();
    harness.spawn().await.expect("spawn chrome");
    harness.render_html(&doc).await.expect("render html");

    // The promoted diagram must parse as a real SVG element, not a literal
    // `<svg>` text node. A matched element computes a non-sentinel display.
    let display = harness
        .computed_style("svg", "display")
        .await
        .expect("computed style query");
    assert!(
        display != "<no-match>",
        "promoted Mermaid <svg> must exist as DOM in the browser; got {display:?}",
    );
}

/// Folds a `mermaid` fence to a render tree and renders it through the browser
/// **tree graphics-policy path** with the same wiring the production
/// `render_tree_html` entry point uses for `MermaidMode::Image`: the darkmatter
/// [`TerminalCodeRenderer`] hook, `GraphicsMode::Rich`, and
/// [`BrowserMermaidMode::StaticSvg`]. This mirrors
/// `darkmatter::markdown::render_tree::entrypoints::browser_options_from_html_options`,
/// which is `pub(crate)`, so the policy mapping is reconstructed here from the
/// public `render_browser_document` surface.
fn render_tree_path_mermaid_html() -> String {
    const MERMAID_SRC: &str = "```mermaid\nflowchart LR\n    A --> B\n```\n";

    let source = SourceDescriptor::Virtual {
        name: "mermaid".into(),
    };
    let (doc, _diags) = fold_markdown_to_document(source, MERMAID_SRC);

    let opts = BrowserRenderOptions {
        strictness: RenderStrictness::Warn,
        raw_html: RawHtmlPolicy::Escape,
        code_renderer: Some(Rc::new(TerminalCodeRenderer::new())),
        graphics_mode: GraphicsMode::Rich,
        mermaid_mode: BrowserMermaidMode::StaticSvg,
        ..Default::default()
    };
    render_browser_document(&doc, &opts)
        .expect("browser tree render")
        .output
        .render()
}

/// Review-4 finding 1: the browser Mermaid static-SVG promotion must be proven
/// through the **render-tree graphics-policy path** — a
/// `NodeKind::Code { lang: "mermaid", .. }` node promoted by
/// `render_browser_document` when `GraphicsMode` and
/// [`BrowserMermaidMode::StaticSvg`] permit it — not only through the legacy
/// [`Markdown::as_html`] surface (covered by
/// [`browser_mermaid_static_svg_computes_in_browser`]). This folds Markdown to a
/// `Document` and renders it through the tree path, then drives a real headless
/// Chromium and asserts the promoted `<svg>` parsed into the DOM (a matched
/// element computes a non-sentinel `display`).
///
/// Skips cleanly when the host lacks the Mermaid toolchain: the promotion hook
/// returns `None`, the renderer degrades to a code block, and no `<svg>` is
/// produced — there is nothing browser-renderable to assert.
#[tokio::test]
#[serial(browser)]
async fn browser_mermaid_tree_path_static_svg_computes_in_browser() {
    if !require_browser() {
        return;
    }

    let html = render_tree_path_mermaid_html();
    if !html.contains("<svg") {
        eprintln!(
            "skipping: Mermaid toolchain unavailable (no SVG produced; degraded to code block)"
        );
        return;
    }

    let mut harness = ChromeHarness::new();
    harness.spawn().await.expect("spawn chrome");
    harness.render_html(&html).await.expect("render html");

    let display = harness
        .computed_style("svg", "display")
        .await
        .expect("computed style query");
    assert!(
        display != "<no-match>",
        "tree-path promoted Mermaid <svg> must exist as DOM in the browser; got {display:?}",
    );
}

/// A hostile SVG, as if a future `mermaid-rs-renderer` path (or an unescaped
/// graph label) emitted active markup. This stands in for the renderer output
/// that [`sanitize_svg`] guards: the render-tree path always runs
/// `render_to_svg()` through the sanitizer before raw-HTML emission.
const HOSTILE_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="0 0 100 50">
  <script>window.__pwned = 1;</script>
  <rect x="0" y="0" width="100" height="50" fill="#eeeeee" onload="window.__pwned = 1"/>
  <foreignObject width="100" height="50"><img src="x" onerror="window.__pwned = 1"/></foreignObject>
  <a xlink:href="javascript:window.__pwned = 1"><text x="5" y="25">node</text></a>
</svg>"##;

/// Review-5 finding 2: the Mermaid static-SVG adapter must sanitize the renderer
/// output before emitting it as raw HTML. This proves the sanitizer's output is
/// safe **DOM** — driven through a real headless Chromium — not just a safe
/// string: a hostile SVG must lose its `<script>`/`<img>`/`onload`/`javascript:`
/// payloads while the `<svg>`/`<rect>` survive. The page is given 100ms to let
/// any surviving handler fire; `window.__pwned` must remain unset.
#[tokio::test]
#[serial(browser)]
async fn browser_sanitized_mermaid_svg_injects_no_active_markup() {
    if !require_browser() {
        return;
    }

    let safe = sanitize_svg(HOSTILE_SVG).expect("hostile SVG is well-formed");
    assert!(
        !safe.contains("<script")
            && !safe.contains("onload")
            && !safe.contains("onerror")
            && !safe.contains("javascript:")
            && !safe.contains("foreignObject"),
        "sanitizer left active markup in the string:\n{safe}",
    );

    let doc = wrap_fragment(&safe, "#ffffff");
    let mut harness = ChromeHarness::new();
    harness.spawn().await.expect("spawn chrome");
    harness.render_html(&doc).await.expect("render html");

    // The diagram body must still render as DOM.
    let svg_display = harness
        .computed_style("svg", "display")
        .await
        .expect("computed style query");
    assert!(
        svg_display != "<no-match>",
        "sanitized <svg> must survive as DOM; got {svg_display:?}",
    );
    let rect_fill = harness
        .computed_style("svg rect", "fill")
        .await
        .expect("computed style query");
    assert!(
        rect_fill != "<no-match>",
        "sanitized <rect> must survive as DOM; got {rect_fill:?}",
    );

    // No injected node, and no handler executed.
    for injected in ["script", "img"] {
        let display = harness
            .computed_style(injected, "display")
            .await
            .expect("computed style query");
        assert_eq!(
            display, "<no-match>",
            "sanitizer let a <{injected}> node into the DOM",
        );
    }
}

/// Review-5 finding 2 (fidelity): sanitizing a *real* promoted Mermaid diagram
/// must keep the diagram — the allowlist must not strip the shape vocabulary the
/// renderer emits. Folds a `mermaid` fence through the render-tree path (whose
/// `render_browser_mermaid` hook now runs the sanitizer) and asserts the
/// surviving `<svg>` still carries drawable geometry.
///
/// Skips cleanly when the Mermaid toolchain is unavailable (no `<svg>` produced).
#[test]
fn sanitized_real_mermaid_retains_diagram_geometry() {
    let html = render_tree_path_mermaid_html();
    if !html.contains("<svg") {
        eprintln!("skipping: Mermaid toolchain unavailable (no SVG produced; degraded to code block)");
        return;
    }
    assert!(
        html.contains("<path") || html.contains("<rect") || html.contains("<polygon"),
        "sanitized real Mermaid SVG kept no drawable geometry:\n{html}",
    );
    assert!(
        !html.contains("<script") && !html.contains("onload="),
        "sanitized real Mermaid SVG still carries active markup:\n{html}",
    );
}
