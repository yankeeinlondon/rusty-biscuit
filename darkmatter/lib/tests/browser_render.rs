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
use renderable::tree::{
    BrowserRenderOptions, GraphicsMode, HintNamespace, RenderNode, render_browser_node,
};
use serial_test::serial;

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
