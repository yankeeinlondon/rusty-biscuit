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

// Whitebox: these tests wire the deprecated `TerminalCodeRenderer` adapter
// directly to exercise the render-tree code path.
#![allow(deprecated)]

use biscuit_browser_harness::{
    BrowserHarness, ChromeHarness, InputStep, KeyStroke, require_browser, wrap_fragment,
};
use darkmatter::mermaid::{
    MERMAID_CDN_FALLBACK_ORIGIN, MERMAID_CDN_PRIMARY_ORIGIN, MERMAID_VERSION,
};
use darkmatter::markdown::Markdown;
use darkmatter::markdown::highlighting::{CodeBlockMode, ColorMode, ThemePair};
use darkmatter::markdown::output::HtmlOptions;
use darkmatter::markdown::render_tree::{TerminalCodeRenderer, fold_markdown_to_document};
use darkmatter::markdown::render_tree::svg_sanitizer::sanitize_svg;
use renderable::tree::{
    BrowserMermaidMode, BrowserRenderOptions, GraphicsMode, HrAlignment, HrKind, HrWeight,
    RawHtmlPolicy, RenderNode, RenderStrictness, SourceDescriptor, ThematicBreakAttrs,
    render_browser_document, render_browser_node,
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
    harness.shutdown().await;
}

/// Renders a `style: waves` thematic break to a browser-renderer HTML fragment
/// at the given graphics tier.
fn waves_hr_fragment(mode: GraphicsMode) -> String {
    let mut hr = RenderNode::thematic_break();
    hr.attrs.set_thematic_break(&ThematicBreakAttrs {
        kind: Some(HrKind::Waves),
        weight: Some(HrWeight::Thick),
        color: Some("red".into()),
        ..Default::default()
    });

    let opts = BrowserRenderOptions {
        graphics_mode: mode,
        ..BrowserRenderOptions::default()
    };
    render_browser_node(&hr, &opts)
        .expect("render hr")
        .output
        .render()
}

/// Renders a dashed thematic break at the given alignment and a narrow authored
/// width (`50%`) to a `Rich`-tier browser fragment, used to prove the SVG honors
/// horizontal placement rather than always centering (review-1 finding 1).
fn aligned_hr_fragment(alignment: HrAlignment) -> String {
    let mut hr = RenderNode::thematic_break();
    hr.attrs.set_thematic_break(&ThematicBreakAttrs {
        alignment: Some(alignment),
        width: Some("50%".into()),
        ..Default::default()
    });

    let opts = BrowserRenderOptions {
        graphics_mode: GraphicsMode::Rich,
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
    harness.shutdown().await;
}

/// Renders an aligned narrow rule and returns its browser-resolved
/// `(margin-left, margin-right, width-ratio)`. The width ratio is the rule's
/// used pixel width divided by its containing block (`body`) width, so a
/// stretched-to-full rule resolves to ~1.0 and a narrow `50%` rule to ~0.5.
async fn hr_geometry(harness: &mut ChromeHarness, alignment: HrAlignment) -> (f64, f64, f64) {
    let doc = wrap_fragment(&aligned_hr_fragment(alignment), "#ffffff");
    harness.render_html(&doc).await.expect("render html");
    let left = px(&harness
        .computed_style(".darkmatter-hr", "margin-left")
        .await
        .expect("margin-left"));
    let right = px(&harness
        .computed_style(".darkmatter-hr", "margin-right")
        .await
        .expect("margin-right"));
    let rule_width = px(&harness
        .computed_style(".darkmatter-hr", "width")
        .await
        .expect("width"));
    let block_width = px(&harness
        .computed_style("body", "width")
        .await
        .expect("body width"));
    assert!(block_width > 0.0, "containing block must have a positive used width");
    (left, right, rule_width / block_width)
}

/// Review-1 finding 1: a non-full authored width (`50%`) must honor the
/// `alignment` attribute instead of always centering. Review-2 finding 1: a
/// `full` rule must stretch across the whole width even though `50%` is
/// authored. This drives a real headless Chromium and asserts both the
/// browser-resolved left/right margins (`auto` margins resolve to the remaining
/// space, so horizontal placement is observable) and the used width relative to
/// the containing block: `left`/`center`/`right` stay narrow (~50%) while `full`
/// fills it (~100%).
#[tokio::test]
#[serial(browser)]
async fn browser_hr_alignment_positions_narrow_rule() {
    if !require_browser() {
        return;
    }

    let mut harness = ChromeHarness::new();
    harness.spawn().await.expect("spawn chrome");

    // Left-anchored: no left margin, the slack collapses to the right; the rule
    // honors the authored 50% width.
    let (l, r, w) = hr_geometry(&mut harness, HrAlignment::Left).await;
    assert!(
        l < 1.0 && r > 1.0,
        "left alignment must anchor left (ml≈0, mr>0); got ml={l}, mr={r}",
    );
    assert!((w - 0.5).abs() < 0.02, "left rule must stay narrow (~50%); got width ratio {w}");

    // Right-anchored: the mirror image.
    let (l, r, w) = hr_geometry(&mut harness, HrAlignment::Right).await;
    assert!(
        r < 1.0 && l > 1.0,
        "right alignment must anchor right (mr≈0, ml>0); got ml={l}, mr={r}",
    );
    assert!((w - 0.5).abs() < 0.02, "right rule must stay narrow (~50%); got width ratio {w}");

    // Centered: equal slack on both sides.
    let (l, r, w) = hr_geometry(&mut harness, HrAlignment::Center).await;
    assert!(
        l > 1.0 && r > 1.0 && (l - r).abs() < 1.0,
        "center alignment must split the slack evenly; got ml={l}, mr={r}",
    );
    assert!((w - 0.5).abs() < 0.02, "center rule must stay narrow (~50%); got width ratio {w}");

    // Full: zero horizontal margin and stretched to the whole containing block,
    // overriding the authored 50% width (review-2 finding 1).
    let (l, r, w) = hr_geometry(&mut harness, HrAlignment::Full).await;
    assert!(
        l < 1.0 && r < 1.0,
        "full alignment must use zero horizontal margin; got ml={l}, mr={r}",
    );
    assert!(
        (w - 1.0).abs() < 0.02,
        "full rule must fill the containing block (~100%), not the authored 50%; got width ratio {w}",
    );
    harness.shutdown().await;
}

/// Renders a thematic break whose `width` / `color` hints carry hostile
/// attribute/markup-breaking payloads, at `Rich`.
fn hostile_hr_fragment() -> String {
    let mut hr = RenderNode::thematic_break();
    hr.attrs.set_thematic_break(&ThematicBreakAttrs {
        kind: Some(HrKind::Waves),
        // A `color` that, if interpolated unescaped, breaks out of the SVG
        // attribute and injects an `<img onerror>` sibling.
        color: Some(r#"red"><img src=x onerror="window.__pwned=1">"#.into()),
        // A `width` that, if interpolated unescaped, injects a `<script>`.
        width: Some(r#"100%"><script>window.__pwned=1</script>"#.into()),
        ..Default::default()
    });

    let opts = BrowserRenderOptions {
        graphics_mode: GraphicsMode::Rich,
        ..BrowserRenderOptions::default()
    };
    render_browser_node(&hr, &opts)
        .expect("render hr")
        .output
        .render()
}

/// Review-5 finding 1: hostile thematic-break `width` / `color` values must
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
    harness.shutdown().await;
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
    harness.shutdown().await;
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
        .expect("browser tree render")
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
    harness.shutdown().await;
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
    harness.shutdown().await;
}

/// A hostile SVG that smuggles external references through CSS surfaces rather
/// than active markup: an `@import`, a `style="...url(...)"`, and a
/// `filter="url(...)"`, alongside a local `url(#…)` fragment that must survive.
/// This stands in for the renderer output that [`sanitize_svg`] must scrub of
/// off-document fetches before raw-HTML emission.
const HOSTILE_CSS_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 50">
  <style>@import url(https://attacker.example/x.css); rect { fill: url(https://attacker.example/p.svg#x) }</style>
  <defs><linearGradient id="grad"><stop offset="0" stop-color="#00ff00"/></linearGradient></defs>
  <rect x="0" y="0" width="100" height="50" style="fill:url(https://attacker.example/p.svg#x)" filter="url(https://attacker.example/filter.svg#f)"/>
  <rect x="10" y="10" width="20" height="20" fill="url(#grad)"/>
</svg>"##;

/// Review-6 finding: the Mermaid static-SVG sanitizer must also scrub CSS
/// external-reference surfaces — `@import`, `style="...url(...)"`, and
/// `filter="url(...)"` — not just active markup. This proves, through a real
/// headless Chromium, that no `attacker.example` reference reaches the applied
/// DOM while the diagram geometry and a local `url(#grad)` fill survive.
#[tokio::test]
#[serial(browser)]
async fn browser_sanitized_mermaid_svg_strips_external_css_references() {
    if !require_browser() {
        return;
    }

    let safe = sanitize_svg(HOSTILE_CSS_SVG).expect("hostile SVG is well-formed");
    assert!(
        !safe.contains("attacker.example") && !safe.contains("@import"),
        "sanitizer left an external CSS reference in the string:\n{safe}",
    );
    // The benign local-fragment fill must remain for diagram fidelity.
    assert!(
        safe.contains("url(#grad)"),
        "sanitizer dropped the local fragment fill:\n{safe}",
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

    // The hostile `filter="url(https://attacker.example/...)"` must not apply:
    // with the attribute stripped, the rect's computed filter is `none`.
    let rect_filter = harness
        .computed_style("svg rect", "filter")
        .await
        .expect("computed style query");
    assert!(
        !rect_filter.contains("attacker"),
        "external filter reference reached the DOM; got {rect_filter:?}",
    );
    harness.shutdown().await;
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

// ---------------------------------------------------------------------------
// Review-1 findings 1 & 3: component layout / style is browser-observable, so
// it must be verified through a real browser (computed style), not just HTML
// source substrings. These drive the decorated `DarkmatterPage::render_to_browser`
// path — the one that lowers `style:` per-component layout and colors.
// ---------------------------------------------------------------------------

/// Builds full-page HTML from `style:` frontmatter through the decorated browser
/// path, wrapped for the harness.
fn style_page_doc(width: u32, style_yaml: &str, body: &str) -> String {
    use biscuit_terminal::terminal::Terminal;
    use darkmatter::layout::DarkmatterPage;
    use darkmatter::style::{
        ComponentStyleOverrides, HrStyleOverrides, ListStyleOverrides, PageStyleOverrides,
        apply_color_style, apply_component_style, apply_hr_style, apply_list_style,
        apply_page_style, from_frontmatter,
    };

    let indented: String = style_yaml
        .lines()
        .map(|l| {
            if l.trim().is_empty() {
                l.to_string()
            } else {
                format!("    {l}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    let full = format!("---\nstyle:\n{indented}\n---\n\n{body}");
    let md = Markdown::try_from_content(&full).expect("parse markdown with style frontmatter");
    let (style, _warnings) = from_frontmatter(md.frontmatter()).expect("parse style");

    let term = Terminal::new_optimistic(width);
    let page = DarkmatterPage::new(&term);
    let page = apply_page_style(page, &style, PageStyleOverrides::default()).unwrap();
    let page = apply_component_style(page, &style, ComponentStyleOverrides::default()).unwrap();
    let page = apply_list_style(page, &style, ListStyleOverrides::default()).unwrap();
    let page = apply_color_style(page, &style).unwrap();
    let page = apply_hr_style(page, &style, HrStyleOverrides::default()).unwrap();
    let body_html = page.render_to_browser(&md).expect("browser render");
    wrap_fragment(&body_html, "#ffffff")
}

/// Parses a `"<n>px"` computed length to `f64` (`0.0` when not px-shaped).
fn px(value: &str) -> f64 {
    value.trim_end_matches("px").parse().unwrap_or(0.0)
}

const TABLE_MD: &str = "| a | b |\n|---|---|\n| 1 | 2 |\n";

/// Reads the table's *used* pixel width as a ratio of its containing block
/// (`.darkmatter-page`) — the observable geometry a percentage width is supposed
/// to control. Chrome resolves a percentage `width` to a px used value, so this
/// reads applied layout rather than a serialized declaration.
async fn used_table_ratio(harness: &mut ChromeHarness) -> f64 {
    let table = harness
        .computed_style("table", "width")
        .await
        .expect("computed style query");
    let page = harness
        .computed_style(".darkmatter-page", "width")
        .await
        .expect("computed style query");
    assert!(
        table.ends_with("px") && page.ends_with("px"),
        "expected px used widths from the browser; got table {table}, page {page}",
    );
    px(&table) / px(&page)
}

/// A component `bg-color` with opacity must compute, in a real browser, to the
/// `rgba(...)` form — the regression the render tree's `Color`-typed `Style`
/// cannot represent and the browser entry point splices back in.
#[tokio::test]
#[serial(browser)]
async fn browser_component_blockquote_bg_opacity_computes_rgba() {
    if !require_browser() {
        return;
    }
    let doc = style_page_doc(120, "block-quote:\n  bg-color: '#ff000080'", "> Quote\n");

    let mut harness = ChromeHarness::new();
    harness.spawn().await.expect("spawn chrome");
    harness.render_html(&doc).await.expect("render html");

    let bg = harness
        .computed_style("blockquote", "background-color")
        .await
        .expect("computed style query");
    assert_eq!(
        bg, "rgba(255, 0, 0, 0.5)",
        "component bg-color opacity must compute to rgba in a real browser; got {bg}",
    );
    harness.shutdown().await;
}

/// A component `color` must compute to the declared `rgb(...)` in a real browser.
#[tokio::test]
#[serial(browser)]
async fn browser_component_table_color_computes_rgb() {
    if !require_browser() {
        return;
    }
    let doc = style_page_doc(120, "table:\n  color: '#ff0000'", TABLE_MD);

    let mut harness = ChromeHarness::new();
    harness.spawn().await.expect("spawn chrome");
    harness.render_html(&doc).await.expect("render html");

    let color = harness
        .computed_style("table", "color")
        .await
        .expect("computed style query");
    assert_eq!(
        color, "rgb(255, 0, 0)",
        "component color must compute to rgb in a real browser; got {color}",
    );
    harness.shutdown().await;
}

/// A percentage page `max-width` must be accepted by the browser and resolved
/// against the viewport (computed to a non-zero px used value), proving the
/// frame retains the authored `Length` rather than a pre-resolved cell count.
#[tokio::test]
#[serial(browser)]
async fn browser_page_max_width_percent_computes_against_viewport() {
    if !require_browser() {
        return;
    }
    let doc = style_page_doc(120, "page:\n  max-width: 50%", "Hello world\n");

    let mut harness = ChromeHarness::new();
    harness.spawn().await.expect("spawn chrome");
    harness.render_html(&doc).await.expect("render html");

    let max_width = harness
        .computed_style(".darkmatter-page", "max-width")
        .await
        .expect("computed style query");
    assert!(
        max_width != "none" && max_width != "<no-match>" && (max_width.ends_with("px") || max_width.ends_with('%')),
        "browser must accept and compute percentage max-width; got {max_width}",
    );
    if max_width.ends_with("px") {
        assert!(px(&max_width) > 0.0, "resolved max-width must be positive; got {max_width}");
    }
    harness.shutdown().await;
}

/// Review-2 finding (High) — "add a browser-tier computed-style test for a
/// percentage-width table or blockquote, asserting its used width relative to
/// its containing block" — tightened by Review-3 finding 1: the assertion must
/// verify *used geometry*, not a serialized declaration.
///
/// The earlier attempt used `max-width: 50%`, which Chrome reports verbatim as
/// the literal `50%` from `getComputedStyle` — proving only that the percent
/// round tripped. A table cannot be made to *bind* a percentage `max-width`
/// either: the component lowers to `white-space: nowrap`, so its min-content
/// equals its max-content and `max-width` can never shrink it, while an explicit
/// `width` alongside `max-width` is rejected as a configuration conflict. So a
/// percentage **`width`** — exactly what Review-2 named — is the observable
/// vehicle: the browser resolves it to a px used value against the containing
/// block.
///
/// This drives a real headless Chromium and asserts the table's used pixel width
/// is ~50% of its containing block (`.darkmatter-page`) at **two different**
/// container sizes. A value pre-resolved to a fixed cell count would track only
/// one container; holding at 50% across both proves the authored `Length` was
/// carried onto the node and resolved live against the containing block.
#[tokio::test]
#[serial(browser)]
async fn browser_component_table_width_percent_resolves_against_container() {
    if !require_browser() {
        return;
    }

    let mut harness = ChromeHarness::new();
    harness.spawn().await.expect("spawn chrome");

    // Two distinct containing-block widths (set via the page frame). A live
    // percentage tracks each; a pre-resolved fixed width could match at most one.
    for page_max in ["40ch", "80ch"] {
        let style = format!("page:\n  max-width: {page_max}\ntable:\n  width: 50%");
        let doc = style_page_doc(120, &style, TABLE_MD);
        harness.render_html(&doc).await.expect("render html");
        let ratio = used_table_ratio(&mut harness).await;
        assert!(
            (ratio - 0.5).abs() < 0.02,
            "percentage component width must resolve to ~50% of its containing \
             block (page max-width {page_max}); got ratio {ratio}",
        );
    }
    harness.shutdown().await;
}

/// A centered table (`alignment: center` + `max-width`) must compute equal,
/// non-zero auto margins in a real browser.
#[tokio::test]
#[serial(browser)]
async fn browser_table_center_alignment_computes_equal_margins() {
    if !require_browser() {
        return;
    }
    let doc = style_page_doc(120, "table:\n  alignment: center\n  max-width: 20ch", TABLE_MD);

    let mut harness = ChromeHarness::new();
    harness.spawn().await.expect("spawn chrome");
    harness.render_html(&doc).await.expect("render html");

    let left = harness
        .computed_style("table", "margin-left")
        .await
        .expect("computed style query");
    let right = harness
        .computed_style("table", "margin-right")
        .await
        .expect("computed style query");
    assert_eq!(left, right, "centered table must have equal auto margins; got {left} / {right}");
    assert!(px(&left) > 0.0, "centered table margins must be non-zero; got {left}");
    harness.shutdown().await;
}

/// Review-4 finding (High) — "Browser page max-width centering". A page
/// `max-width` with the default (zero) side margins must center the page frame
/// (`.darkmatter-page`) in the viewport. Earlier coverage checked only the
/// `max-width` *declaration*; this asserts the *used geometry*: equal, non-zero
/// left/right offsets and a positive used max-width in a real browser.
#[tokio::test]
#[serial(browser)]
async fn browser_page_max_width_centers_frame() {
    if !require_browser() {
        return;
    }
    let doc = style_page_doc(120, "page:\n  max-width: 40ch", "Hello world\n");

    let mut harness = ChromeHarness::new();
    harness.spawn().await.expect("spawn chrome");
    harness.render_html(&doc).await.expect("render html");

    // The wrapper's used side margins (auto-resolved to px) are its offsets from
    // the body/viewport edges. Equal + non-zero proves the frame is centered.
    let left = harness
        .computed_style(".darkmatter-page", "margin-left")
        .await
        .expect("computed style query");
    let right = harness
        .computed_style(".darkmatter-page", "margin-right")
        .await
        .expect("computed style query");
    assert_eq!(
        left, right,
        "max-width page frame must center via equal auto side offsets; got {left} / {right}",
    );
    assert!(px(&left) > 0.0, "centered page frame side offsets must be non-zero; got {left}");

    let max_width = harness
        .computed_style(".darkmatter-page", "max-width")
        .await
        .expect("computed style query");
    assert!(
        max_width.ends_with("px") && px(&max_width) > 0.0,
        "page max-width must resolve to a positive used px width; got {max_width}",
    );
    harness.shutdown().await;
}

/// Review-1 finding 2: a per-image `style='color: blue;'` title directive must
/// win, in a real browser, over the frontmatter `images.local-style.color`
/// default — the merged `inline_style` resolves to blue, not the frontmatter
/// red. The raw directive must not survive as a literal `title`.
#[tokio::test]
#[serial(browser)]
async fn browser_local_image_per_node_css_overrides_frontmatter() {
    if !require_browser() {
        return;
    }
    let doc = style_page_doc(
        120,
        "images:\n  local-style:\n    color: red-500",
        "![A](./local.png \"style='color: blue;'\")\n",
    );

    let mut harness = ChromeHarness::new();
    harness.spawn().await.expect("spawn chrome");
    harness.render_html(&doc).await.expect("render html");

    let color = harness
        .computed_style("img", "color")
        .await
        .expect("computed style query");
    assert_eq!(
        color, "rgb(0, 0, 255)",
        "per-node blue must win over frontmatter red in a real browser; got {color}",
    );

    let title = harness
        .computed_style("img[title]", "display")
        .await
        .expect("computed style query");
    assert_eq!(
        title, "<no-match>",
        "raw `style='...'` directive must not survive as an HTML title attribute",
    );
    harness.shutdown().await;
}

/// Review-1 finding 3: a page-level `color` must inherit, in a real browser, to
/// descendant text — proving the foreground rides the root node (rendered as a
/// wrapping div) rather than being copied onto each component. A component with
/// no color of its own computes the inherited page color.
#[tokio::test]
#[serial(browser)]
async fn browser_page_color_inherits_to_descendants() {
    if !require_browser() {
        return;
    }
    let doc = style_page_doc(120, "page:\n  color: red-500", "A paragraph of text.\n\n# Heading\n");

    let mut harness = ChromeHarness::new();
    harness.spawn().await.expect("spawn chrome");
    harness.render_html(&doc).await.expect("render html");

    // red-500 = rgb(251, 44, 54). Both the paragraph and the heading inherit it
    // from the root wrapper — neither carries its own color declaration.
    for selector in ["p", "h1"] {
        let color = harness
            .computed_style(selector, "color")
            .await
            .expect("computed style query");
        assert_eq!(
            color, "rgb(251, 44, 54)",
            "{selector} must inherit the page foreground in a real browser; got {color}",
        );
    }
    harness.shutdown().await;
}

/// Review-1 finding 3: with only a page color set, a component (table) must not
/// carry a copied color — it computes the inherited page color, and removing the
/// per-component copy is observable as a single inherited value.
#[tokio::test]
#[serial(browser)]
async fn browser_page_color_not_copied_onto_component() {
    if !require_browser() {
        return;
    }
    let doc = style_page_doc(120, "page:\n  color: red-500", TABLE_MD);

    let mut harness = ChromeHarness::new();
    harness.spawn().await.expect("spawn chrome");
    harness.render_html(&doc).await.expect("render html");

    let color = harness
        .computed_style("table", "color")
        .await
        .expect("computed style query");
    assert_eq!(
        color, "rgb(251, 44, 54)",
        "table must inherit the page color (not a copied per-component value); got {color}",
    );
    harness.shutdown().await;
}

/// Review-2: a translucent page background must be painted **only** by the page
/// frame. Links and images must not carry a copied page background — otherwise
/// each one composites the same translucent paint again over the frame. In a
/// real browser the `.darkmatter-page` frame computes the translucent rgba while
/// the `<a>` and `<img>` compute fully-transparent backgrounds.
#[tokio::test]
#[serial(browser)]
async fn browser_translucent_page_background_painted_only_by_frame() {
    if !require_browser() {
        return;
    }
    let doc = style_page_doc(
        120,
        "page:\n  bg-color: blue-500/50",
        "[label](https://example.com)\n\n![A](./local.png)\n",
    );

    let mut harness = ChromeHarness::new();
    harness.spawn().await.expect("spawn chrome");
    harness.render_html(&doc).await.expect("render html");

    // The frame paints the translucent page background once (alpha < 1).
    let frame_bg = harness
        .computed_style(".darkmatter-page", "background-color")
        .await
        .expect("computed style query");
    assert!(
        frame_bg.starts_with("rgba(") && frame_bg != "rgba(0, 0, 0, 0)",
        "the page frame must paint the translucent background; got {frame_bg}",
    );

    // The link and image must be fully transparent — the page background is not
    // copied onto them, so it is composited exactly once.
    for selector in ["a", "img"] {
        let bg = harness
            .computed_style(selector, "background-color")
            .await
            .expect("computed style query");
        assert_eq!(
            bg, "rgba(0, 0, 0, 0)",
            "{selector} must not carry a copied page background; got {bg}",
        );
    }
    harness.shutdown().await;
}

/// A list `left-margin` must compute to a non-zero px margin in a real browser.
#[tokio::test]
#[serial(browser)]
async fn browser_list_left_margin_computes() {
    if !require_browser() {
        return;
    }
    let doc = style_page_doc(120, "ul:\n  left-margin: 4ch", "- item one\n- item two\n");

    let mut harness = ChromeHarness::new();
    harness.spawn().await.expect("spawn chrome");
    harness.render_html(&doc).await.expect("render html");

    let margin_left = harness
        .computed_style("ul", "margin-left")
        .await
        .expect("computed style query");
    assert!(
        margin_left.ends_with("px") && px(&margin_left) > 0.0,
        "list left-margin must compute to a non-zero px margin; got {margin_left}",
    );
    harness.shutdown().await;
}

// ---------------------------------------------------------------------------
// Review-2 finding 2: the browser-visible code-block theme/mode is
// user-observable styling — the `.code-block` panel background and the syntax
// `<span>` colors must compute to the page-resolved theme/mode in an actual
// browser. The L1 string-luminance checks in `page.rs` do not exercise the
// parser/cascade/computed-style path, so these drive
// `DarkmatterPage::render_to_browser` through a real headless Chromium.
// ---------------------------------------------------------------------------

/// Parses a `"rgb(r, g, b)"` / `"rgba(r, g, b, a)"` computed color into a
/// perceptual luminance in `0.0..=1.0`. Returns `None` for any other shape.
fn rgb_luminance(value: &str) -> Option<f32> {
    let trimmed = value.trim();
    let inner = trimmed
        .strip_prefix("rgb(")
        .or_else(|| trimmed.strip_prefix("rgba("))?
        .strip_suffix(')')?;
    let mut parts = inner.split(',').map(str::trim);
    let r: f32 = parts.next()?.parse().ok()?;
    let g: f32 = parts.next()?.parse().ok()?;
    let b: f32 = parts.next()?.parse().ok()?;
    Some((0.2126 * r + 0.7152 * g + 0.0722 * b) / 255.0)
}

/// Renders a fenced code block through `DarkmatterPage::render_to_browser_document`
/// with a captured terminal `mode` (and optional `CodeBlockMode`), loads it in
/// the browser, and returns the computed `(.code-block background-color, first
/// .code-block span color)`. The `.code-block` panel stylesheet lives in the
/// document `<head>`, so this reads the full standalone document form (the
/// body-only `render_to_browser` fragment omits it). The `github` paired theme
/// is pinned so the result is deterministic across hosts and unaffected by
/// ambient `THEME` env.
async fn page_code_block_computed_styles(
    harness: &mut ChromeHarness,
    mode: ColorMode,
    code_block_mode: Option<CodeBlockMode>,
) -> (String, String) {
    use biscuit_terminal::terminal::Terminal;
    use darkmatter::layout::DarkmatterPage;

    let md: Markdown = "```rust\nfn main() {}\n```\n".into();
    let mut term = Terminal::new_optimistic(80);
    term.color_mode = mode;
    let mut page = DarkmatterPage::new(&term).with_code_theme("github");
    if let Some(cbm) = code_block_mode {
        page = page.with_code_block_mode(cbm);
    }
    let doc = page
        .render_to_browser_document(&md)
        .expect("render_to_browser_document");
    harness.render_html(&doc).await.expect("render html");

    let bg = harness
        .computed_style(".code-block", "background-color")
        .await
        .expect("computed style query");
    let color = harness
        .computed_style(".code-block span", "color")
        .await
        .expect("computed style query");
    (bg, color)
}

/// Review-2 finding 2: `DarkmatterPage::render_to_browser_document` must resolve
/// the code panel's theme variant against the *captured terminal mode* in a real
/// browser. A dark terminal inverts (default) to a light panel and a light
/// terminal to a dark panel; both the `.code-block` computed `background-color`
/// and a representative syntax `<span>`'s computed `color` must follow that
/// page-resolved mode — not a fixed default theme.
#[tokio::test]
#[serial(browser)]
async fn browser_page_code_block_theme_follows_captured_terminal_mode() {
    if !require_browser() {
        return;
    }
    let mut harness = ChromeHarness::new();
    harness.spawn().await.expect("spawn chrome");

    let (dark_bg, dark_span) =
        page_code_block_computed_styles(&mut harness, ColorMode::Dark, None).await;
    let (light_bg, light_span) =
        page_code_block_computed_styles(&mut harness, ColorMode::Light, None).await;

    let dark_bg_lum =
        rgb_luminance(&dark_bg).unwrap_or_else(|| panic!("unparseable dark panel bg {dark_bg:?}"));
    let light_bg_lum = rgb_luminance(&light_bg)
        .unwrap_or_else(|| panic!("unparseable light panel bg {light_bg:?}"));

    // Dark page -> inverse -> light panel; light page -> inverse -> dark panel.
    assert!(
        dark_bg_lum > 0.6,
        "a dark terminal must invert to a light code panel; got {dark_bg:?} (lum {dark_bg_lum:.3})",
    );
    assert!(
        light_bg_lum < 0.4,
        "a light terminal must invert to a dark code panel; got {light_bg:?} (lum {light_bg_lum:.3})",
    );
    assert!(
        (dark_bg_lum - light_bg_lum).abs() > 0.4,
        "captured-mode panels must be well-separated; dark {dark_bg_lum:.3}, light {light_bg_lum:.3}",
    );

    // The syntax markup color must follow the page-resolved mode too (the L1
    // bug: markup painted a fixed default theme while the panel inverted).
    assert_ne!(
        dark_span, light_span,
        "syntax span color must follow the captured terminal mode; dark {dark_span:?} vs light {light_span:?}",
    );
    harness.shutdown().await;
}

/// Review-2 finding 2: `CodeBlockMode::Same` vs `Inverse` must be
/// browser-observable through `DarkmatterPage::render_to_browser_document`. On a dark
/// page, `Inverse` (default) computes a light panel while `Same` keeps a dark
/// panel, so the `.code-block` computed `background-color` must differ and
/// `Inverse` must be the lighter of the two.
#[tokio::test]
#[serial(browser)]
async fn browser_page_code_block_mode_same_vs_inverse_computes() {
    if !require_browser() {
        return;
    }
    let mut harness = ChromeHarness::new();
    harness.spawn().await.expect("spawn chrome");

    let (inverse_bg, _) =
        page_code_block_computed_styles(&mut harness, ColorMode::Dark, Some(CodeBlockMode::Inverse))
            .await;
    let (same_bg, _) =
        page_code_block_computed_styles(&mut harness, ColorMode::Dark, Some(CodeBlockMode::Same))
            .await;

    let inverse_lum = rgb_luminance(&inverse_bg)
        .unwrap_or_else(|| panic!("unparseable inverse panel bg {inverse_bg:?}"));
    let same_lum =
        rgb_luminance(&same_bg).unwrap_or_else(|| panic!("unparseable same panel bg {same_bg:?}"));

    assert!(
        inverse_lum > 0.6,
        "Inverse on a dark page must compute a light panel; got {inverse_bg:?} (lum {inverse_lum:.3})",
    );
    assert!(
        same_lum < 0.4,
        "Same on a dark page must compute a dark panel; got {same_bg:?} (lum {same_lum:.3})",
    );
    assert_ne!(
        inverse_bg, same_bg,
        "Inverse and Same must compute different code-panel backgrounds",
    );
    harness.shutdown().await;
}

/// The body-only contract of `DarkmatterPage::render_to_browser`, observed in a
/// live DOM: a feature-free, undecorated fragment embedded in a host document
/// introduces no nested document scaffold. The host keeps exactly one
/// `<html>`/`<head>`/`<body>`, no stray `<style>` from a leaked document
/// `<head>` reaches the DOM, and the rendered Markdown heading rides the host
/// body. A regression that returned a full `<!DOCTYPE html>` document here would
/// splice a second `<head>`/`<style>` into the page, which this probe catches.
/// The standalone-document form is `render_to_browser_document`'s job and is
/// pinned separately.
#[tokio::test]
#[serial(browser)]
async fn browser_feature_free_fragment_has_no_nested_document_scaffold() {
    if !require_browser() {
        return;
    }
    use biscuit_terminal::terminal::Terminal;
    use darkmatter::layout::DarkmatterPage;

    let md: Markdown = "# Heading One\n\nBody text.\n".into();
    let term = Terminal::new_optimistic(80);
    let fragment = DarkmatterPage::new(&term)
        .render_to_browser(&md)
        .expect("render_to_browser");
    let doc = wrap_fragment(&fragment, "#ffffff");

    let mut harness = ChromeHarness::new();
    harness.spawn().await.expect("spawn chrome");
    harness.render_html(&doc).await.expect("render html");

    let probe = "(() => {\
        const htmls = document.getElementsByTagName('html').length;\
        const heads = document.getElementsByTagName('head').length;\
        const bodies = document.getElementsByTagName('body').length;\
        const styles = document.querySelectorAll('style').length;\
        const h1 = document.querySelector('h1');\
        return `htmls=${htmls};heads=${heads};bodies=${bodies};styles=${styles};heading=${h1 ? h1.textContent : ''}`;\
    })()";
    let result = harness.evaluate(probe).await.expect("evaluate scaffold probe");
    harness.shutdown().await;

    assert!(!result.starts_with("err="), "scaffold probe failed: {result}");
    let kv = parse_kv(&result);
    assert_eq!(
        kv.get("htmls").map(String::as_str),
        Some("1"),
        "exactly one <html> in the host DOM; got {result}",
    );
    assert_eq!(
        kv.get("heads").map(String::as_str),
        Some("1"),
        "exactly one <head>; got {result}",
    );
    assert_eq!(
        kv.get("bodies").map(String::as_str),
        Some("1"),
        "exactly one <body>; got {result}",
    );
    assert_eq!(
        kv.get("styles").map(String::as_str),
        Some("0"),
        "a body-only fragment leaks no document <style> into the DOM; got {result}",
    );
    assert_eq!(
        kv.get("heading").map(String::as_str),
        Some("Heading One"),
        "the rendered heading rides the host body; got {result}",
    );
}

/// The standalone-document contract of `DarkmatterPage::render_to_browser_document`
/// for a decorated, feature-bearing page, observed in a live DOM: page metadata,
/// feature styles, and feature scripts are children of `document.head`, while the
/// `.darkmatter-page` frame (and the rendered content it wraps) is a child of
/// `document.body`. This is the head-fix regression guard — before it, the
/// decorated path emitted an empty `<head></head>` and buried the metadata /
/// styles / scripts inside the body wrapper. The document is loaded directly
/// (not `wrap_fragment`) because it is already a complete `<!DOCTYPE html>`
/// document.
#[tokio::test]
#[serial(browser)]
async fn browser_decorated_standalone_document_head_body_placement() {
    if !require_browser() {
        return;
    }
    use biscuit_terminal::terminal::Terminal;
    use darkmatter::layout::DarkmatterPage;
    use darkmatter::style::bespoke::{MetaTag, PageMeta};

    // Mermaid (a head `<script type="module">` feature) + a prompted link (the
    // Popover CSS feature) + a page `<meta>` + page margins — a decorated,
    // feature-bearing page that exercises every head slot.
    let md: Markdown = concat!(
        "```mermaid\ngraph TD; A --> B\n```\n\n",
        "[Home](https://example.com \"prompt='go home'\")\n",
    )
    .into();
    let term = Terminal::new_optimistic(80);
    let doc = DarkmatterPage::new(&term)
        .with_margin(2)
        .with_page_meta(PageMeta {
            tags: vec![MetaTag::Name {
                name: "author".into(),
                content: "Ken".into(),
            }],
        })
        .render_to_browser_document(&md)
        .expect("render_to_browser_document");

    let mut harness = ChromeHarness::new();
    harness.spawn().await.expect("spawn chrome");
    harness.render_html(&doc).await.expect("render html");

    let probe = "(() => {\
        const dp = document.querySelector('.darkmatter-page');\
        if (!dp) return 'err=no-darkmatter-page';\
        const dpParent = dp.parentElement ? dp.parentElement.tagName : 'none';\
        const headModuleScripts = document.head.querySelectorAll('script[type=\"module\"]').length;\
        const bodyScripts = document.body.querySelectorAll('script').length;\
        const headMeta = document.head.querySelectorAll('meta[name=\"author\"]').length;\
        const bodyMeta = document.body.querySelectorAll('meta').length;\
        const headPopover = [...document.head.querySelectorAll('style')].some(s => s.textContent.includes('.dm-popover-wrapper{')) ? 1 : 0;\
        const bodyStyles = document.body.querySelectorAll('style').length;\
        const mermaidInBody = dp.querySelector('pre.mermaid') ? 1 : 0;\
        const popoverInBody = dp.querySelector('.dm-popover-wrapper') ? 1 : 0;\
        return `dpParent=${dpParent};headModuleScripts=${headModuleScripts};bodyScripts=${bodyScripts};headMeta=${headMeta};bodyMeta=${bodyMeta};headPopover=${headPopover};bodyStyles=${bodyStyles};mermaidInBody=${mermaidInBody};popoverInBody=${popoverInBody}`;\
    })()";
    let result = harness.evaluate(probe).await.expect("evaluate placement probe");
    harness.shutdown().await;

    assert!(!result.starts_with("err="), "placement probe failed: {result}");
    let kv = parse_kv(&result);
    assert_eq!(
        kv.get("dpParent").map(String::as_str),
        Some("BODY"),
        "the .darkmatter-page frame is a direct child of <body>; got {result}",
    );
    assert_eq!(
        kv.get("headModuleScripts").map(String::as_str),
        Some("1"),
        "the Mermaid ESM bootstrap is a child of <head>; got {result}",
    );
    assert_eq!(
        kv.get("bodyScripts").map(String::as_str),
        Some("0"),
        "no feature <script> lives in <body>; got {result}",
    );
    assert_eq!(
        kv.get("headMeta").map(String::as_str),
        Some("1"),
        "the page <meta name=author> is a child of <head>; got {result}",
    );
    assert_eq!(
        kv.get("bodyMeta").map(String::as_str),
        Some("0"),
        "no <meta> lives in <body>; got {result}",
    );
    assert_eq!(
        kv.get("headPopover").map(String::as_str),
        Some("1"),
        "the Popover CSS <style> is a child of <head>; got {result}",
    );
    assert_eq!(
        kv.get("bodyStyles").map(String::as_str),
        Some("0"),
        "no feature/design-token <style> lives in <body>; got {result}",
    );
    assert_eq!(
        kv.get("mermaidInBody").map(String::as_str),
        Some("1"),
        "the rendered Mermaid container rides the body frame; got {result}",
    );
    assert_eq!(
        kv.get("popoverInBody").map(String::as_str),
        Some("1"),
        "the rendered popover markup rides the body frame; got {result}",
    );
}

// ---------------------------------------------------------------------------
// Review-5 finding 1: browser disclosure behavior was only verified at Level 1
// (HTML-source substrings). The spec requires native `<details>`/`<summary>`
// with NO JavaScript, where the body is revealed by the browser's own
// click-to-open behavior. These drive a real headless Chromium and assert the
// parsed DOM toggles: the body is unrendered while `details.open === false` and
// rendered after the summary is clicked and `details.open === true`. The whole
// interaction runs in one `evaluate` because each `computed_style` call opens a
// fresh page and could not observe the click.
// ---------------------------------------------------------------------------

/// Parses a `"key=value;key=value"` payload (the shape the disclosure probe
/// scripts return) into a lookup map.
fn parse_kv(payload: &str) -> std::collections::HashMap<String, String> {
    payload
        .split(';')
        .filter_map(|pair| pair.split_once('='))
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

/// Review-5 finding 1: a disclosure rendered to HTML must parse into a native
/// `<details>`/`<summary>` DOM, carry no `<script>`, hide the body while closed,
/// and reveal it when the summary is clicked — proven against a real browser,
/// not an HTML-source substring.
#[tokio::test]
#[serial(browser)]
async fn browser_disclosure_click_reveals_body() {
    if !require_browser() {
        return;
    }

    let md: Markdown =
        "::disclosure\nSummary text\n::details\nBody paragraph.\n::end-disclosure\n".into();
    let fragment = md.as_html(HtmlOptions::default()).expect("as_html");
    let doc = wrap_fragment(&fragment, "#ffffff");

    let mut harness = ChromeHarness::new();
    harness.spawn().await.expect("spawn chrome");
    harness.render_html(&doc).await.expect("render html");

    // `checkVisibility()` reports the browser's own rendered-visibility verdict:
    // a closed `<details>` hides its non-summary content (Chrome wraps it in a
    // `content-visibility: hidden` `::details-content`), so the body computes
    // not-visible while closed and visible once the summary is clicked open. (A
    // bounding-box height check is unreliable here: Chrome still lays the body
    // out at its intrinsic size even while it is visually hidden.)
    let probe = "(() => {\
        const d = document.querySelector('details');\
        if (!d) return 'err=no-details';\
        const summary = d.querySelector('summary');\
        if (!summary) return 'err=no-summary';\
        const body = d.querySelector('p');\
        if (!body) return 'err=no-body';\
        const scripts = document.querySelectorAll('script').length;\
        const closedOpen = d.open;\
        const closedVis = body.checkVisibility();\
        summary.click();\
        const openedOpen = d.open;\
        const openedVis = body.checkVisibility();\
        return `scripts=${scripts};closedOpen=${closedOpen};closedVis=${closedVis};openedOpen=${openedOpen};openedVis=${openedVis}`;\
    })()";
    let result = harness.evaluate(probe).await.expect("evaluate disclosure probe");
    assert!(
        !result.starts_with("err="),
        "disclosure DOM probe failed: {result}",
    );
    let kv = parse_kv(&result);

    assert_eq!(
        kv.get("scripts").map(String::as_str),
        Some("0"),
        "native disclosure must include no <script>; got {result}",
    );
    assert_eq!(
        kv.get("closedOpen").map(String::as_str),
        Some("false"),
        "disclosure must start closed; got {result}",
    );
    assert_eq!(
        kv.get("closedVis").map(String::as_str),
        Some("false"),
        "body must be hidden while closed; got {result}",
    );
    assert_eq!(
        kv.get("openedOpen").map(String::as_str),
        Some("true"),
        "clicking the summary must open the disclosure; got {result}",
    );
    assert_eq!(
        kv.get("openedVis").map(String::as_str),
        Some("true"),
        "body must be visible once opened; got {result}",
    );

    harness.shutdown().await;
}

/// Review-5 finding 1 (nested): a disclosure nested in another's body must parse
/// as a second `<details>`, stay unrendered until the outer opens, then toggle
/// independently when its own summary is clicked.
#[tokio::test]
#[serial(browser)]
async fn browser_nested_disclosure_toggles_independently() {
    if !require_browser() {
        return;
    }

    let md: Markdown = "::disclosure\nOuter\n::details\nOuter body.\n\n::disclosure\nInner\n::details\nInner body.\n::end-disclosure\n\n::end-disclosure\n".into();
    let fragment = md.as_html(HtmlOptions::default()).expect("as_html");
    let doc = wrap_fragment(&fragment, "#ffffff");

    let mut harness = ChromeHarness::new();
    harness.spawn().await.expect("spawn chrome");
    harness.render_html(&doc).await.expect("render html");

    let probe = "(() => {\
        const all = document.querySelectorAll('details');\
        if (all.length < 2) return `err=count-${all.length}`;\
        const outer = all[0], inner = all[1];\
        const innerVisClosed = inner.checkVisibility();\
        outer.querySelector('summary').click();\
        const innerVisOuterOpen = inner.checkVisibility();\
        inner.querySelector('summary').click();\
        return `count=${all.length};innerVisClosed=${innerVisClosed};innerVisOuterOpen=${innerVisOuterOpen};innerOpen=${inner.open}`;\
    })()";
    let result = harness.evaluate(probe).await.expect("evaluate nested probe");
    assert!(!result.starts_with("err="), "nested DOM probe failed: {result}");
    let kv = parse_kv(&result);

    assert_eq!(
        kv.get("count").map(String::as_str),
        Some("2"),
        "nested disclosures must parse as two <details>; got {result}",
    );
    assert_eq!(
        kv.get("innerVisClosed").map(String::as_str),
        Some("false"),
        "inner disclosure must be hidden while the outer is closed; got {result}",
    );
    assert_eq!(
        kv.get("innerVisOuterOpen").map(String::as_str),
        Some("true"),
        "inner disclosure must become visible once the outer opens; got {result}",
    );
    assert_eq!(
        kv.get("innerOpen").map(String::as_str),
        Some("true"),
        "clicking the inner summary must open the inner disclosure; got {result}",
    );

    harness.shutdown().await;
}

/// Phase 4 (Popover): a prompted link's CSS-only enhancement must actually work
/// in a real browser. Proven against computed styles / live focus rather than
/// HTML-source substrings:
///
/// - the prompt's computed `display` is `block` — our author rule overrides the
///   popover-supporting UA `[popover]{display:none}` rule, so the `:hover` /
///   `:focus-within` fallback is never defeated (the key cross-browser point);
/// - the prompt is `visibility:hidden` by default and becomes `visible` when the
///   anchor is keyboard-focused (`:focus-within` — keyboard reachable, no JS);
/// - the anchor keeps its real `href` and the `aria-describedby` association
///   names the prompt element's `id`.
#[tokio::test]
#[serial(browser)]
async fn browser_prompted_link_popover_reveals_on_focus() {
    if !require_browser() {
        return;
    }

    let md: Markdown =
        "[Click](https://example.com \"prompt='Extra detail'\")\n".into();
    let doc = md.as_html(HtmlOptions::default()).expect("as_html");

    let mut harness = ChromeHarness::new();
    harness.spawn().await.expect("spawn chrome");
    harness.render_html(&doc).await.expect("render html");

    // Everything runs in one page load: fresh `page()` calls do not share focus
    // state, so the focus + re-query must happen inside a single evaluate.
    let probe = "(() => {\
        const p = document.querySelector('.dm-popover-prompt');\
        const a = document.querySelector('.dm-popover-wrapper a');\
        if (!p || !a) return 'err=missing';\
        const cs = getComputedStyle(p);\
        const display = cs.display;\
        const visDefault = cs.visibility;\
        a.focus();\
        const visFocus = getComputedStyle(p).visibility;\
        return `display=${display};visDefault=${visDefault};visFocus=${visFocus};` +\
            `href=${a.getAttribute('href')};describedby=${a.getAttribute('aria-describedby')};promptId=${p.id}`;\
    })()";
    let result = harness.evaluate(probe).await.expect("evaluate popover probe");
    assert!(!result.starts_with("err="), "popover DOM probe failed: {result}");
    let kv = parse_kv(&result);

    assert_eq!(
        kv.get("display").map(String::as_str),
        Some("block"),
        "the prompt must override the UA [popover] display:none; got {result}",
    );
    assert_eq!(
        kv.get("visDefault").map(String::as_str),
        Some("hidden"),
        "the prompt must be hidden by default; got {result}",
    );
    assert_eq!(
        kv.get("visFocus").map(String::as_str),
        Some("visible"),
        "keyboard focus must reveal the prompt via :focus-within; got {result}",
    );
    assert_eq!(
        kv.get("href").map(String::as_str),
        Some("https://example.com"),
        "the anchor must keep its real href; got {result}",
    );
    assert_eq!(
        kv.get("describedby"),
        kv.get("promptId"),
        "aria-describedby must name the prompt element id; got {result}",
    );

    harness.shutdown().await;
}

/// The single-quoted popover fixture (`prompt='…'` title). `{PROMPT}` is
/// substituted so a test can supply a short or a deliberately long prompt.
fn popover_doc(prompt: &str) -> Markdown {
    format!("[Click](https://example.com \"prompt='{prompt}'\")\n").into()
}

/// A JS snippet that moves the popover wrapper into a `position:fixed` box
/// pinned to `{EDGE}` (`right`/`left`), focuses the anchor to reveal the
/// prompt, forces layout, and reports the prompt's viewport-space geometry.
/// Placing the trigger hard against a viewport edge is what exposes the
/// right-edge overflow the CSS fix addresses.
fn edge_geometry_probe(edge: &str) -> String {
    format!(
        "(() => {{\
            const w = document.querySelector('.dm-popover-wrapper');\
            const a = w && w.querySelector('a');\
            const p = document.querySelector('.dm-popover-prompt');\
            if (!w || !a || !p) return 'err=missing';\
            const box = document.createElement('div');\
            box.style.cssText = 'position:fixed;top:2px;margin:0;{edge}:2px';\
            w.parentNode.insertBefore(box, w);\
            box.appendChild(w);\
            a.focus();\
            void p.getBoundingClientRect();\
            const r = p.getBoundingClientRect();\
            const supports = CSS.supports('position-try-fallbacks', 'flip-inline');\
            return `left=${{r.left}};right=${{r.right}};width=${{r.width}};`+\
                `iw=${{window.innerWidth}};supports=${{supports}}`;\
        }})()"
    )
}

/// Review-1 finding (High): the popover's `left:0; width:max-content` rule
/// overflows the viewport when the trigger sits near the RIGHT edge. This
/// validates the CSS fix — with the trigger pinned 2px from the right edge and
/// the prompt revealed, the prompt's bounding box must stay on-screen
/// (`right <= innerWidth`). CSS anchor positioning (`flip-inline`) is what keeps
/// it on-screen; the probe reports `supports` so a regression that loses anchor
/// positioning is legible in the failure message.
#[tokio::test]
#[serial(browser)]
async fn browser_popover_stays_within_viewport_right_edge() {
    if !require_browser() {
        return;
    }
    let doc = popover_doc("Extra detail").as_html(HtmlOptions::default()).expect("as_html");

    let mut harness = ChromeHarness::new();
    harness.spawn().await.expect("spawn chrome");
    harness.render_html(&doc).await.expect("render html");

    let result = harness
        .evaluate(&edge_geometry_probe("right"))
        .await
        .expect("evaluate right-edge probe");
    assert!(!result.starts_with("err="), "right-edge probe failed: {result}");
    let kv = parse_kv(&result);
    let right: f64 = kv["right"].parse().expect("right is a number");
    let iw: f64 = kv["iw"].parse().expect("iw is a number");
    assert_eq!(kv.get("supports").map(String::as_str), Some("true"), "anchor positioning must be active: {result}");
    assert!(
        right <= iw + 0.5,
        "popover near the right edge must stay on-screen: right={right} > innerWidth={iw} ({result})",
    );

    harness.shutdown().await;
}

/// The mirror of the right-edge test: with the trigger pinned to the LEFT edge
/// the prompt must not spill off the left (`left >= 0`) and must still fit
/// (`right <= innerWidth`).
#[tokio::test]
#[serial(browser)]
async fn browser_popover_stays_within_viewport_left_edge() {
    if !require_browser() {
        return;
    }
    let doc = popover_doc("Extra detail").as_html(HtmlOptions::default()).expect("as_html");

    let mut harness = ChromeHarness::new();
    harness.spawn().await.expect("spawn chrome");
    harness.render_html(&doc).await.expect("render html");

    let result = harness
        .evaluate(&edge_geometry_probe("left"))
        .await
        .expect("evaluate left-edge probe");
    assert!(!result.starts_with("err="), "left-edge probe failed: {result}");
    let kv = parse_kv(&result);
    let left: f64 = kv["left"].parse().expect("left is a number");
    let right: f64 = kv["right"].parse().expect("right is a number");
    let iw: f64 = kv["iw"].parse().expect("iw is a number");
    assert!(left >= -0.5, "popover near the left edge must not spill left: left={left} ({result})");
    assert!(right <= iw + 0.5, "popover near the left edge must stay on-screen: right={right} > iw={iw} ({result})");

    harness.shutdown().await;
}

/// A long prompt must WRAP inside the capped `max-width` panel rather than
/// overflow it or the viewport. Asserts the panel is no wider than the 20rem
/// cap, has no horizontal overflow, wraps onto multiple lines, and stays
/// on-screen.
#[tokio::test]
#[serial(browser)]
async fn browser_popover_long_prompt_wraps_within_viewport() {
    if !require_browser() {
        return;
    }
    let long = "This is a deliberately long prompt that must wrap onto several lines \
                instead of overflowing the popover panel or spilling past the right edge \
                of the viewport when rendered in a real browser window.";
    let doc = popover_doc(long).as_html(HtmlOptions::default()).expect("as_html");

    let mut harness = ChromeHarness::new();
    harness.spawn().await.expect("spawn chrome");
    harness.render_html(&doc).await.expect("render html");

    let probe = "(() => {\
        const a = document.querySelector('.dm-popover-wrapper a');\
        const p = document.querySelector('.dm-popover-prompt');\
        if (!a || !p) return 'err=missing';\
        a.focus();\
        void p.getBoundingClientRect();\
        const r = p.getBoundingClientRect();\
        const lh = parseFloat(getComputedStyle(p).lineHeight) || 16;\
        const lines = Math.round(p.scrollHeight / lh);\
        return `width=${r.width};right=${r.right};iw=${window.innerWidth};`+\
            `overflow=${p.scrollWidth - p.clientWidth};lines=${lines}`;\
    })()";
    let result = harness.evaluate(probe).await.expect("evaluate long-prompt probe");
    assert!(!result.starts_with("err="), "long-prompt probe failed: {result}");
    let kv = parse_kv(&result);
    let width: f64 = kv["width"].parse().expect("width is a number");
    let right: f64 = kv["right"].parse().expect("right is a number");
    let iw: f64 = kv["iw"].parse().expect("iw is a number");
    let overflow: f64 = kv["overflow"].parse().expect("overflow is a number");
    let lines: i64 = kv["lines"].parse().expect("lines is a number");
    // 20rem == 320px content cap; the border-box adds padding (.5rem each side)
    // + 1px border, so ~338px. The point is that the cap engaged — far below the
    // ~1100px an unwrapped single line would need.
    assert!(width <= 340.5, "panel must not exceed the 20rem cap plus box model: width={width} ({result})");
    assert!(right <= iw + 0.5, "wrapped panel must stay on-screen: right={right} > iw={iw} ({result})");
    assert!(overflow <= 1.0, "prompt text must wrap, not overflow horizontally: overflow={overflow} ({result})");
    assert!(lines >= 2, "a long prompt must wrap onto multiple lines: lines={lines} ({result})");

    harness.shutdown().await;
}

/// Token-less embeds track the OS theme: with no page-level `--color-*` tokens
/// defined, the popover's fallback colors must differ between an emulated dark
/// and light `prefers-color-scheme`. Proven through CDP media emulation on a
/// single page (`evaluate_with_media`), which the per-call fresh-page
/// `evaluate` cannot express.
#[tokio::test]
#[serial(browser)]
async fn browser_popover_color_modes_differ() {
    if !require_browser() {
        return;
    }
    let doc = popover_doc("Extra detail").as_html(HtmlOptions::default()).expect("as_html");

    let mut harness = ChromeHarness::new();
    harness.spawn().await.expect("spawn chrome");
    harness.render_html(&doc).await.expect("render html");

    let probe = "(() => {\
        const p = document.querySelector('.dm-popover-prompt');\
        if (!p) return 'err=missing';\
        const cs = getComputedStyle(p);\
        return `bg=${cs.backgroundColor};fg=${cs.color}`;\
    })()";
    let dark = harness
        .evaluate_with_media(&[("prefers-color-scheme", "dark")], probe)
        .await
        .expect("evaluate dark");
    let light = harness
        .evaluate_with_media(&[("prefers-color-scheme", "light")], probe)
        .await
        .expect("evaluate light");
    assert!(!dark.starts_with("err="), "dark probe failed: {dark}");
    assert!(!light.starts_with("err="), "light probe failed: {light}");
    assert_ne!(dark, light, "dark and light popover colors must differ (dark={dark}, light={light})");

    harness.shutdown().await;
}

/// `prefers-reduced-motion: reduce` must suppress the popover's opacity
/// transition. Under emulation the computed `transition-duration` collapses to
/// `0s`, while the default retains a non-zero duration.
#[tokio::test]
#[serial(browser)]
async fn browser_popover_reduced_motion_suppresses_transition() {
    if !require_browser() {
        return;
    }
    let doc = popover_doc("Extra detail").as_html(HtmlOptions::default()).expect("as_html");

    let mut harness = ChromeHarness::new();
    harness.spawn().await.expect("spawn chrome");
    harness.render_html(&doc).await.expect("render html");

    let probe = "(() => {\
        const p = document.querySelector('.dm-popover-prompt');\
        if (!p) return 'err=missing';\
        return getComputedStyle(p).transitionDuration;\
    })()";
    let reduced = harness
        .evaluate_with_media(&[("prefers-reduced-motion", "reduce")], probe)
        .await
        .expect("evaluate reduced");
    let normal = harness
        .evaluate_with_media(&[("prefers-reduced-motion", "no-preference")], probe)
        .await
        .expect("evaluate normal");
    assert_eq!(reduced, "0s", "reduced motion must suppress the transition, got {reduced}");
    assert_ne!(normal, "0s", "the default keeps a non-zero transition, got {normal}");

    harness.shutdown().await;
}

// ---------------------------------------------------------------------------
// Browser-tier popover interaction tests (real CDP keyboard/pointer input).
//
// Review-1 finding (High): the existing popover coverage calls `a.focus()` from
// JS, which proves the CSS result but not that the *user actions* work. These
// tests drive Tab / Shift-Tab / Enter and pointer hover through headless
// Chromium's real input pipeline — CDP `Input.dispatch{Key,Mouse}Event` via the
// harness's `drive` — which exercises Chromium's own focus traversal and
// default-action handling.
//
// CDP input is injected directly into the renderer and never enters the host OS
// input path. These therefore use the same Browser-tier gate as every other
// `browser_*` test in this file (`if !require_browser() { return; }`) and remain
// headless, isolated from the developer's windows, keyboard focus, and pointer.
// ---------------------------------------------------------------------------

/// Tab must move focus to the anchor (keyboard reachability) and that focus
/// must reveal the prompt via `:focus-within` — the user action the old
/// `a.focus()` shortcut never exercised.
#[tokio::test]
#[serial(browser)]
async fn browser_popover_tab_reaches_anchor() {
    if !require_browser() {
        return;
    }
    let doc = popover_doc("Extra detail").as_html(HtmlOptions::default()).expect("as_html");

    let mut harness = ChromeHarness::new();
    harness.spawn().await.expect("spawn chrome");
    harness.render_html(&doc).await.expect("render html");

    let result = harness
        .drive(&[
            InputStep::Key(KeyStroke::TAB),
            InputStep::Eval(
                "(() => {\
                    const a = document.querySelector('.dm-popover-wrapper a');\
                    const p = document.querySelector('.dm-popover-prompt');\
                    return `active=${document.activeElement === a};`+\
                        `vis=${getComputedStyle(p).visibility}`;\
                })()",
            ),
        ])
        .await
        .expect("drive tab probe");
    let kv = parse_kv(&result);
    assert_eq!(kv.get("active").map(String::as_str), Some("true"), "Tab must focus the anchor: {result}");
    assert_eq!(kv.get("vis").map(String::as_str), Some("visible"), "focusing the anchor must reveal the prompt: {result}");

    harness.shutdown().await;
}

/// Shift+Tab must move focus AWAY from the anchor, and losing focus must
/// re-hide the prompt.
#[tokio::test]
#[serial(browser)]
async fn browser_popover_shift_tab_leaves_anchor() {
    if !require_browser() {
        return;
    }
    let doc = popover_doc("Extra detail").as_html(HtmlOptions::default()).expect("as_html");

    let mut harness = ChromeHarness::new();
    harness.spawn().await.expect("spawn chrome");
    harness.render_html(&doc).await.expect("render html");

    let result = harness
        .drive(&[
            InputStep::Key(KeyStroke::TAB),
            InputStep::Key(KeyStroke::SHIFT_TAB),
            InputStep::Eval(
                "(() => {\
                    const a = document.querySelector('.dm-popover-wrapper a');\
                    const p = document.querySelector('.dm-popover-prompt');\
                    return `active=${document.activeElement === a};`+\
                        `vis=${getComputedStyle(p).visibility}`;\
                })()",
            ),
        ])
        .await
        .expect("drive shift-tab probe");
    let kv = parse_kv(&result);
    assert_eq!(kv.get("active").map(String::as_str), Some("false"), "Shift+Tab must move focus off the anchor: {result}");
    assert_eq!(kv.get("vis").map(String::as_str), Some("hidden"), "losing focus must re-hide the prompt: {result}");

    harness.shutdown().await;
}

/// Enter on the focused anchor must trigger ordinary link activation with the
/// real `href` preserved (the popover enhancement must not swallow navigation).
/// A capturing click listener records the resolved href and prevents an actual
/// (network) navigation.
#[tokio::test]
#[serial(browser)]
async fn browser_popover_enter_activates_link() {
    if !require_browser() {
        return;
    }
    let doc = popover_doc("Extra detail").as_html(HtmlOptions::default()).expect("as_html");

    let mut harness = ChromeHarness::new();
    harness.spawn().await.expect("spawn chrome");
    harness.render_html(&doc).await.expect("render html");

    let result = harness
        .drive(&[
            InputStep::Eval(
                "(() => {\
                    const a = document.querySelector('.dm-popover-wrapper a');\
                    a.addEventListener('click', (e) => { e.preventDefault(); window.__nav = a.href; });\
                    return 'ready';\
                })()",
            ),
            InputStep::Key(KeyStroke::TAB),
            InputStep::Key(KeyStroke::ENTER),
            InputStep::Eval("(() => `nav=${window.__nav || ''}`)()"),
        ])
        .await
        .expect("drive enter probe");
    let kv = parse_kv(&result);
    assert_eq!(
        kv.get("nav").map(String::as_str),
        Some("https://example.com/"),
        "Enter must activate the link with its real href: {result}",
    );

    harness.shutdown().await;
}

/// Pointer hover over the anchor must reveal the prompt via `:hover` — driven
/// by a real CDP `mouseMoved`, not a synthetic JS event.
#[tokio::test]
#[serial(browser)]
async fn browser_popover_pointer_hover_reveals_prompt() {
    if !require_browser() {
        return;
    }
    let doc = popover_doc("Extra detail").as_html(HtmlOptions::default()).expect("as_html");

    let mut harness = ChromeHarness::new();
    harness.spawn().await.expect("spawn chrome");
    harness.render_html(&doc).await.expect("render html");

    let result = harness
        .drive(&[
            InputStep::Eval(
                "(() => { window.__before = getComputedStyle(\
                    document.querySelector('.dm-popover-prompt')).visibility; return 'ready'; })()",
            ),
            InputStep::Hover(".dm-popover-wrapper a"),
            InputStep::Eval(
                "(() => `before=${window.__before};after=${getComputedStyle(\
                    document.querySelector('.dm-popover-prompt')).visibility}`)()",
            ),
        ])
        .await
        .expect("drive hover probe");
    let kv = parse_kv(&result);
    assert_eq!(kv.get("before").map(String::as_str), Some("hidden"), "prompt must start hidden: {result}");
    assert_eq!(kv.get("after").map(String::as_str), Some("visible"), "pointer hover must reveal the prompt: {result}");

    harness.shutdown().await;
}

// ---------------------------------------------------------------------------
// Review-1 finding (High): interactive Mermaid and body placement had no
// real-browser verification. `full_page_..._mermaid_defaults_to_interactive`
// (in `style_features_baseline.rs`) is a synchronous L1 string test that never
// launches Chrome and only proves the emitted *markup* — it cannot observe
// Mermaid load the module, replace the `<pre>` source with an SVG, fall back
// from the primary CDN to the fallback, leave readable source when both fail,
// dedup two blocks in the live DOM, place the body wrapper, or apply the theme.
//
// These Browser-tier tests exercise all six through a real headless Chromium.
// They stay NETWORK-FREE without touching production output: the rendered
// fragment already emits `import('https://cdn.jsdelivr.net/…mermaid@<VER>…')`
// with an unpkg fallback, so the page is wrapped in a shell whose
// `<script type="importmap">` maps those two *exact* absolute URLs to local
// `data:text/javascript` stub ES modules. The bootstrap's dynamic imports then
// resolve to the stubs — no CDN request is ever made. A head-level classic
// script captures `console.error` so the total-failure path is observable.
//
// Stub-module contract (matches the bootstrap in `darkmatter/lib/src/mermaid/
// feature.rs`): the module's *default export* is a `mermaid` object with
// `initialize(config)` — which records `config.theme`/`config.themeVariables`
// on `window` — and `run({ querySelector })` — which replaces each matched
// `.mermaid` element's content with an `<svg>`. A "throwing" stub throws at
// module-evaluation time so the corresponding `import()` rejects.
// ---------------------------------------------------------------------------

/// One Mermaid fence — the single-diagram fixture.
const MERMAID_ONE_DOC: &str = "```mermaid\ngraph TD; A --> B\n```\n";

/// Two Mermaid fences in one document — the dedup fixture (one bootstrap must
/// run; both diagrams must render).
const MERMAID_TWO_DOC: &str =
    "```mermaid\ngraph TD; A --> B\n```\n\n```mermaid\nsequenceDiagram\n  A->>B: hi\n```\n";

/// A working Mermaid stub module. `__SOURCE__` is replaced with a label
/// (`PRIMARY`/`FALLBACK`) so a test can prove *which* CDN specifier served the
/// render. `initialize` records the theme inputs; `run` swaps each `.mermaid`
/// element's content for an `<svg>` colored from the received `background`.
const WORKING_STUB_TEMPLATE: &str = r#"
const mermaid = {
  initialize(cfg) {
    window.__dmInit = (window.__dmInit || 0) + 1;
    window.__dmTheme = cfg.theme;
    window.__dmVars = cfg.themeVariables || {};
    window.__dmSource = '__SOURCE__';
  },
  run(opts) {
    window.__dmRun = (window.__dmRun || 0) + 1;
    const els = document.querySelectorAll(opts.querySelector);
    window.__dmRunEls = els.length;
    const bg = (window.__dmVars && window.__dmVars.background) || '#000000';
    els.forEach(function (el) {
      el.setAttribute('data-processed', 'true');
      el.innerHTML = '<svg xmlns="http://www.w3.org/2000/svg" data-stub="1"><rect width="10" height="10" fill="' + bg + '"></rect></svg>';
    });
    return Promise.resolve();
  }
};
export default mermaid;
"#;

/// A stub that throws at module-evaluation time, so `import()` of it rejects —
/// standing in for a blocked/failed CDN fetch.
const THROWING_STUB: &str = "throw new Error('darkmatter-test: mermaid stub import failure');";

/// A live-DOM probe run inside a single page load. It waits (bounded) for the
/// Mermaid bootstrap's async work to settle — either an SVG appears or a
/// `console.error` is captured — then reports the observable outcome as a
/// `key=value;…` payload (parsed by [`parse_kv`]). No asserted field contains a
/// `;`/`=`, so the diagram source is reported only as the boolean `hasSource`.
const MERMAID_DOM_PROBE: &str = r#"(async () => {
  const deadline = Date.now() + 4000;
  const first = document.querySelector('.mermaid');
  if (!first) return 'mermaid=0;svg=0';
  while (Date.now() < deadline
      && !document.querySelector('.mermaid svg')
      && (window.__dmConsoleErrors || []).length === 0) {
    await new Promise(function (r) { setTimeout(r, 25); });
  }
  const mermaid = document.querySelectorAll('.mermaid').length;
  const svg = document.querySelectorAll('.mermaid svg').length;
  const scripts = document.querySelectorAll('script[type="module"]').length;
  const errs = (window.__dmConsoleErrors || []).length;
  const source = window.__dmSource || '';
  const vars = window.__dmVars || {};
  const bg = vars.background || '';
  const hasSource = (first.textContent || '').indexOf('graph TD') >= 0;
  return 'mermaid=' + mermaid + ';svg=' + svg + ';init=' + (window.__dmInit || 0)
    + ';run=' + (window.__dmRun || 0) + ';runEls=' + (window.__dmRunEls || 0)
    + ';source=' + source + ';scripts=' + scripts + ';errs=' + errs
    + ';bg=' + bg + ';hasSource=' + hasSource;
})()"#;

/// A synchronous probe for the body-only wrapper placement: reports the
/// `.darkmatter-page` element's parent tag and how many nested document
/// elements (`html`/`head`/`body`) it wrongly contains.
const MERMAID_WRAPPER_PROBE: &str = r#"(() => {
  const page = document.querySelector('.darkmatter-page');
  if (!page) return 'parent=MISSING;nested=0';
  const parent = page.parentElement ? page.parentElement.tagName : 'NULL';
  const nested = page.querySelectorAll('html, head, body').length;
  return 'parent=' + parent + ';nested=' + nested;
})()"#;

/// Percent-encodes `source` into a `data:text/javascript` URL usable as an
/// import-map target. Everything outside the URL "unreserved" set is
/// `%`-escaped, so the result is a valid URL and safe to embed verbatim in the
/// import map's JSON (no `"`/`\` survive to break the string).
fn data_module(source: &str) -> String {
    let mut out = String::from("data:text/javascript,");
    for byte in source.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char);
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

/// Builds a working Mermaid stub module labeled with its serving CDN.
fn working_mermaid_stub(source_label: &str) -> String {
    WORKING_STUB_TEMPLATE.replace("__SOURCE__", source_label)
}

/// Renders a Mermaid document through `DarkmatterPage::render_to_browser` at a
/// captured terminal color mode, returning the body-only wrapper fragment (the
/// interactive default: a `.darkmatter-page` div carrying the injected bootstrap
/// `<script type="module">` and the `<pre class="mermaid">` body).
fn mermaid_body_fragment(color_mode: ColorMode, doc: &str) -> String {
    use biscuit_terminal::terminal::Terminal;
    use darkmatter::layout::DarkmatterPage;

    let mut term = Terminal::new_optimistic(80);
    term.color_mode = color_mode;
    let page = DarkmatterPage::new(&term);
    let md = Markdown::try_from_content(doc).expect("parse mermaid doc");
    page.render_to_browser(&md).expect("render_to_browser")
}

/// Wraps a rendered Mermaid `fragment` in a standalone document that redirects
/// the bootstrap's two exact CDN imports to local stub modules via an import
/// map, keeping the whole test network-free. The `primary_module` /
/// `fallback_module` sources become `data:` URL ES modules mapped to the
/// jsDelivr and unpkg specifiers the bootstrap imports. A head-level classic
/// script captures `console.error` into `window.__dmConsoleErrors` before the
/// deferred module bootstrap can run.
fn wrap_mermaid_stub_page(
    fragment: &str,
    primary_module: &str,
    fallback_module: &str,
) -> String {
    // Must byte-match the specifiers emitted by `mermaid_bootstrap` in
    // `darkmatter/lib/src/mermaid/feature.rs`; an import-map URL key only
    // redirects an exactly-equal `import()` specifier.
    let primary_url =
        format!("{MERMAID_CDN_PRIMARY_ORIGIN}/npm/mermaid@{MERMAID_VERSION}/dist/mermaid.esm.min.mjs");
    let fallback_url =
        format!("{MERMAID_CDN_FALLBACK_ORIGIN}/mermaid@{MERMAID_VERSION}/dist/mermaid.esm.min.mjs");
    let primary_data = data_module(primary_module);
    let fallback_data = data_module(fallback_module);
    format!(
        "<!doctype html><html><head><meta charset=\"utf-8\">\
<script>window.__dmConsoleErrors=[];var __e=console.error.bind(console);\
console.error=function(){{window.__dmConsoleErrors.push(\
Array.prototype.map.call(arguments,String).join(' '));__e.apply(console,arguments);}};</script>\
<script type=\"importmap\">{{\"imports\":{{\
\"{primary_url}\":\"{primary_data}\",\
\"{fallback_url}\":\"{fallback_data}\"}}}}</script>\
</head><body style=\"margin:0;background:#fff;\">{fragment}</body></html>"
    )
}

/// (1) Successful interactive rendering: the bootstrap loads the (stubbed)
/// primary Mermaid module and `run` replaces the `<pre class="mermaid">` source
/// with a live `<svg>` in the DOM — `initialize`/`run` each fire once and no
/// `console.error` is emitted. This is the load-and-render path the L1 string
/// test can never observe.
#[tokio::test]
#[serial(browser)]
async fn browser_mermaid_interactive_renders_svg_in_live_dom() {
    if !require_browser() {
        return;
    }

    let fragment = mermaid_body_fragment(ColorMode::Dark, MERMAID_ONE_DOC);
    let doc = wrap_mermaid_stub_page(
        &fragment,
        &working_mermaid_stub("PRIMARY"),
        &working_mermaid_stub("PRIMARY"),
    );

    let mut harness = ChromeHarness::new();
    harness.spawn().await.expect("spawn chrome");
    harness.render_html(&doc).await.expect("render html");
    let result = harness.evaluate(MERMAID_DOM_PROBE).await.expect("mermaid probe");
    harness.shutdown().await;

    let kv = parse_kv(&result);
    assert_eq!(
        kv.get("svg").map(String::as_str),
        Some("1"),
        "Mermaid must replace the <pre class=\"mermaid\"> source with one live SVG; got {result}",
    );
    assert_eq!(
        kv.get("init").map(String::as_str),
        Some("1"),
        "mermaid.initialize must run exactly once; got {result}",
    );
    assert_eq!(
        kv.get("run").map(String::as_str),
        Some("1"),
        "mermaid.run must run exactly once; got {result}",
    );
    assert_eq!(
        kv.get("source").map(String::as_str),
        Some("PRIMARY"),
        "the primary (jsDelivr) specifier must serve the render; got {result}",
    );
    assert_eq!(
        kv.get("errs").map(String::as_str),
        Some("0"),
        "no console.error may be emitted on success; got {result}",
    );
}

/// (2) Primary failure + fallback success: the jsDelivr import throws, the
/// unpkg import succeeds, and the diagram still renders — proving the
/// try/catch fallback in the bootstrap actually works at runtime. The recorded
/// `source=FALLBACK` shows the second specifier served it, and no total-failure
/// `console.error` is emitted.
#[tokio::test]
#[serial(browser)]
async fn browser_mermaid_primary_failure_falls_back_to_unpkg() {
    if !require_browser() {
        return;
    }

    let fragment = mermaid_body_fragment(ColorMode::Dark, MERMAID_ONE_DOC);
    let doc = wrap_mermaid_stub_page(
        &fragment,
        THROWING_STUB,
        &working_mermaid_stub("FALLBACK"),
    );

    let mut harness = ChromeHarness::new();
    harness.spawn().await.expect("spawn chrome");
    harness.render_html(&doc).await.expect("render html");
    let result = harness.evaluate(MERMAID_DOM_PROBE).await.expect("mermaid probe");
    harness.shutdown().await;

    let kv = parse_kv(&result);
    assert_eq!(
        kv.get("svg").map(String::as_str),
        Some("1"),
        "the diagram must still render after the primary import fails; got {result}",
    );
    assert_eq!(
        kv.get("source").map(String::as_str),
        Some("FALLBACK"),
        "the fallback (unpkg) specifier must serve the render; got {result}",
    );
    assert_eq!(
        kv.get("errs").map(String::as_str),
        Some("0"),
        "a recovered fallback must not emit the total-failure console.error; got {result}",
    );
}

/// (3) Total failure with readable source: both imports throw, so no SVG is
/// produced, the escaped diagram source stays visible in the `<pre>`, and the
/// bootstrap emits its single `console.error`. Proven against the live DOM (an
/// SVG never appears; `hasSource` stays true) plus the captured console error.
#[tokio::test]
#[serial(browser)]
async fn browser_mermaid_total_failure_keeps_readable_source() {
    if !require_browser() {
        return;
    }

    let fragment = mermaid_body_fragment(ColorMode::Dark, MERMAID_ONE_DOC);
    let doc = wrap_mermaid_stub_page(&fragment, THROWING_STUB, THROWING_STUB);

    let mut harness = ChromeHarness::new();
    harness.spawn().await.expect("spawn chrome");
    harness.render_html(&doc).await.expect("render html");
    let result = harness.evaluate(MERMAID_DOM_PROBE).await.expect("mermaid probe");
    harness.shutdown().await;

    let kv = parse_kv(&result);
    assert_eq!(
        kv.get("svg").map(String::as_str),
        Some("0"),
        "no SVG may be produced when both imports fail; got {result}",
    );
    assert_eq!(
        kv.get("init").map(String::as_str),
        Some("0"),
        "initialize must not run when no module loaded; got {result}",
    );
    assert_eq!(
        kv.get("hasSource").map(String::as_str),
        Some("true"),
        "the escaped diagram source must remain readable in the <pre>; got {result}",
    );
    let errs: u32 = kv
        .get("errs")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    assert!(
        errs >= 1,
        "a total load failure must emit a console.error; got {result}",
    );
}

/// (4) Deduplication in the live DOM: two Mermaid fences emit two
/// `<pre class="mermaid">` containers but exactly one injected bootstrap; the
/// single `run` processes both, so both diagrams render as SVG and `initialize`
/// still fires once. The L1 dedup test counts source substrings; this proves the
/// deduped bootstrap actually drives *both* diagrams in a real browser.
#[tokio::test]
#[serial(browser)]
async fn browser_mermaid_dedup_renders_both_diagrams_once() {
    if !require_browser() {
        return;
    }

    let fragment = mermaid_body_fragment(ColorMode::Dark, MERMAID_TWO_DOC);
    let doc = wrap_mermaid_stub_page(
        &fragment,
        &working_mermaid_stub("PRIMARY"),
        &working_mermaid_stub("PRIMARY"),
    );

    let mut harness = ChromeHarness::new();
    harness.spawn().await.expect("spawn chrome");
    harness.render_html(&doc).await.expect("render html");
    let result = harness.evaluate(MERMAID_DOM_PROBE).await.expect("mermaid probe");
    harness.shutdown().await;

    let kv = parse_kv(&result);
    assert_eq!(
        kv.get("mermaid").map(String::as_str),
        Some("2"),
        "both fences must parse into two .mermaid containers; got {result}",
    );
    assert_eq!(
        kv.get("scripts").map(String::as_str),
        Some("1"),
        "exactly one bootstrap module script may be injected; got {result}",
    );
    assert_eq!(
        kv.get("init").map(String::as_str),
        Some("1"),
        "the single deduped bootstrap must initialize once; got {result}",
    );
    assert_eq!(
        kv.get("runEls").map(String::as_str),
        Some("2"),
        "the single run must process both diagrams; got {result}",
    );
    assert_eq!(
        kv.get("svg").map(String::as_str),
        Some("2"),
        "both diagrams must render as live SVG; got {result}",
    );
}

/// (5) Valid wrapper placement (the DOM assertion Review-1 Finding 1 asked for):
/// the body-only `<div class="darkmatter-page">` wrapper must be a real direct
/// child of `<body>` and must nest NO `html`/`head`/`body` element. The L1 tests
/// assert this on the source string; this parses the fragment into a real DOM
/// and checks the wrapper's `parentElement` and descendant document elements.
#[tokio::test]
#[serial(browser)]
async fn browser_mermaid_body_wrapper_is_direct_child_of_body() {
    if !require_browser() {
        return;
    }

    let fragment = mermaid_body_fragment(ColorMode::Dark, MERMAID_ONE_DOC);
    let doc = wrap_mermaid_stub_page(
        &fragment,
        &working_mermaid_stub("PRIMARY"),
        &working_mermaid_stub("PRIMARY"),
    );

    let mut harness = ChromeHarness::new();
    harness.spawn().await.expect("spawn chrome");
    harness.render_html(&doc).await.expect("render html");
    let result = harness
        .evaluate(MERMAID_WRAPPER_PROBE)
        .await
        .expect("wrapper probe");
    harness.shutdown().await;

    let kv = parse_kv(&result);
    assert_eq!(
        kv.get("parent").map(String::as_str),
        Some("BODY"),
        "the .darkmatter-page wrapper must be a direct child of <body>; got {result}",
    );
    assert_eq!(
        kv.get("nested").map(String::as_str),
        Some("0"),
        "the wrapper must not nest any html/head/body element; got {result}",
    );
}

/// (6) Theme application: the resolved `themeVariables` palette must actually
/// reach `mermaid.initialize`, and a light page and a dark page must produce
/// *different* applied theme inputs. The stub records the received
/// `themeVariables.background`; rendering the same diagram in each mode must
/// yield two distinct, non-empty backgrounds — proving the palette is a live
/// input to Mermaid, not dead markup.
#[tokio::test]
#[serial(browser)]
async fn browser_mermaid_theme_variables_differ_by_color_mode() {
    if !require_browser() {
        return;
    }

    let mut harness = ChromeHarness::new();
    harness.spawn().await.expect("spawn chrome");

    let dark_doc = wrap_mermaid_stub_page(
        &mermaid_body_fragment(ColorMode::Dark, MERMAID_ONE_DOC),
        &working_mermaid_stub("PRIMARY"),
        &working_mermaid_stub("PRIMARY"),
    );
    harness.render_html(&dark_doc).await.expect("render dark");
    let dark = parse_kv(&harness.evaluate(MERMAID_DOM_PROBE).await.expect("dark probe"));

    let light_doc = wrap_mermaid_stub_page(
        &mermaid_body_fragment(ColorMode::Light, MERMAID_ONE_DOC),
        &working_mermaid_stub("PRIMARY"),
        &working_mermaid_stub("PRIMARY"),
    );
    harness.render_html(&light_doc).await.expect("render light");
    let light = parse_kv(&harness.evaluate(MERMAID_DOM_PROBE).await.expect("light probe"));

    harness.shutdown().await;

    let dark_bg = dark.get("bg").map(String::as_str).unwrap_or("");
    let light_bg = light.get("bg").map(String::as_str).unwrap_or("");
    assert!(
        !dark_bg.is_empty() && !light_bg.is_empty(),
        "the themeVariables background must reach mermaid.initialize in both modes; \
         dark={dark_bg:?} light={light_bg:?}",
    );
    assert_ne!(
        dark_bg, light_bg,
        "light and dark pages must apply different Mermaid theme inputs; \
         dark={dark_bg:?} light={light_bg:?}",
    );
}

// ---------------------------------------------------------------------------
// Review finding (High): the stub Mermaid tests above prove the bootstrap can
// drive an API shape the TEST defines — they never execute the REAL pinned
// Mermaid 11.6.0, so they cannot prove that real Mermaid exports that shape,
// accepts the resolver's `themeVariables`, parses the diagram, or produces a
// correctly themed SVG (spec acceptance criteria 3 and 11).
//
// This test runs the REAL pinned engine, network-free. Mermaid 11.6.0's
// `dist/mermaid.esm.min.mjs` is vendored (with its complete `chunks/` import
// closure) under `tests/fixtures/mermaid/<MERMAID_VERSION>/dist/` and served by
// a loopback-only (`127.0.0.1:0`) static server. The page's import map
// redirects the two EXACT CDN specifiers the bootstrap emits to that server, so
// the bootstrap's real `import()` resolves to the vendored engine — no external
// egress ever occurs. See `tests/fixtures/mermaid/README.md` for the vendoring
// and its `regen.mjs` regeneration script.
//
// Gating: `require_browser()` AND the vendored entry's presence. A checkout
// without the fixtures skips cleanly (an explanatory line, green suite).
// ---------------------------------------------------------------------------

/// Absolute path to the vendored Mermaid `dist/` tree for the pinned version.
fn mermaid_fixture_dist() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/mermaid")
        .join(MERMAID_VERSION)
        .join("dist")
}

/// The first `\r\n\r\n` (end of HTTP request headers) in `buf`, if present.
fn find_crlfcrlf(buf: &[u8]) -> Option<usize> {
    buf.windows(4).position(|w| w == b"\r\n\r\n")
}

/// Maps a request target to a file in the vendored `dist/` tree, accepting both
/// the jsDelivr (`/npm/mermaid@<V>/dist/…`) and unpkg (`/mermaid@<V>/dist/…`)
/// URL shapes — plus the relative `chunks/…` requests the browser derives from
/// the module's own URL (which keep whichever prefix served the entry). Returns
/// `None` for anything outside the tree or containing `..`.
fn resolve_mermaid_fixture(dist: &std::path::Path, target: &str) -> Option<std::path::PathBuf> {
    let path = target.split(['?', '#']).next().unwrap_or(target);
    let rest = path
        .strip_prefix(&format!("/npm/mermaid@{MERMAID_VERSION}/dist/"))
        .or_else(|| path.strip_prefix(&format!("/mermaid@{MERMAID_VERSION}/dist/")))?;
    if rest.is_empty() || rest.contains("..") {
        return None;
    }
    Some(dist.join(rest))
}

/// The `Content-Type` for a served fixture. `.mjs`/`.js` MUST be
/// `text/javascript`: a browser refuses to execute a module served with any
/// other MIME type, which would silently break the import.
fn mermaid_content_type(path: &std::path::Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()) {
        Some("mjs" | "js") => "text/javascript",
        Some("json" | "map") => "application/json",
        Some("css") => "text/css",
        _ => "application/octet-stream",
    }
}

/// Builds a minimal HTTP/1.1 response. `Access-Control-Allow-Origin: *` lets the
/// `file://` test page (a `null` origin) fetch the cross-origin ES module.
fn mermaid_http_response(status: u16, content_type: &str, body: &[u8]) -> Vec<u8> {
    let reason = if status == 200 { "OK" } else { "Not Found" };
    let mut out = format!(
        "HTTP/1.1 {status} {reason}\r\n\
Content-Type: {content_type}\r\n\
Content-Length: {len}\r\n\
Access-Control-Allow-Origin: *\r\n\
Cache-Control: no-store\r\n\
Connection: keep-alive\r\n\r\n",
        len = body.len(),
    )
    .into_bytes();
    out.extend_from_slice(body);
    out
}

/// Serves keep-alive GET requests for one connection until it closes. Mermaid's
/// entry statically and dynamically imports many `chunks/` files, so a
/// connection carries several sequential requests.
async fn serve_mermaid_conn(mut socket: tokio::net::TcpStream, dist: std::path::PathBuf) {
    use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};

    let mut buf: Vec<u8> = Vec::new();
    let mut chunk = [0u8; 8192];
    loop {
        let header_end = loop {
            if let Some(pos) = find_crlfcrlf(&buf) {
                break pos + 4;
            }
            match socket.read(&mut chunk).await {
                Ok(0) => return,
                Ok(n) => buf.extend_from_slice(&chunk[..n]),
                Err(_) => return,
            }
        };
        let head = String::from_utf8_lossy(&buf[..header_end]);
        let target = head
            .lines()
            .next()
            .and_then(|line| line.split_whitespace().nth(1))
            .unwrap_or("");
        let response = match resolve_mermaid_fixture(&dist, target) {
            Some(path) => match tokio::fs::read(&path).await {
                Ok(body) => mermaid_http_response(200, mermaid_content_type(&path), &body),
                Err(_) => mermaid_http_response(404, "text/plain", b"not found"),
            },
            None => mermaid_http_response(404, "text/plain", b"not found"),
        };
        if socket.write_all(&response).await.is_err() {
            return;
        }
        // GET carries no body; drop this request and await the next on the
        // same keep-alive connection.
        buf.drain(..header_end);
    }
}

/// Binds a loopback static server rooted at the vendored `dist` tree and returns
/// its port plus the accept-loop task handle (aborted by the caller at teardown).
async fn spawn_mermaid_loopback_server(
    dist: std::path::PathBuf,
) -> (u16, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .expect("bind loopback mermaid server");
    let port = listener.local_addr().expect("loopback addr").port();
    let handle = tokio::spawn(async move {
        while let Ok((socket, _)) = listener.accept().await {
            let root = dist.clone();
            tokio::spawn(async move { serve_mermaid_conn(socket, root).await });
        }
    });
    (port, handle)
}

/// Wraps a rendered Mermaid `fragment` in a standalone document whose import map
/// redirects the bootstrap's two EXACT CDN specifiers to the loopback `base_url`
/// (jsDelivr shape → `/npm/…`, unpkg shape → `/…`). The URLs are built from the
/// same `MERMAID_CDN_*`/`MERMAID_VERSION` constants the bootstrap uses, so the
/// import-map keys stay byte-identical to the emitted `import()` specifiers (an
/// import-map key only redirects an exactly-equal specifier). A head-level
/// classic script captures `console.error` for diagnostics.
fn wrap_mermaid_loopback_page(fragment: &str, base_url: &str) -> String {
    let primary_url = format!(
        "{MERMAID_CDN_PRIMARY_ORIGIN}/npm/mermaid@{MERMAID_VERSION}/dist/mermaid.esm.min.mjs"
    );
    let fallback_url =
        format!("{MERMAID_CDN_FALLBACK_ORIGIN}/mermaid@{MERMAID_VERSION}/dist/mermaid.esm.min.mjs");
    let primary_local =
        format!("{base_url}/npm/mermaid@{MERMAID_VERSION}/dist/mermaid.esm.min.mjs");
    let fallback_local = format!("{base_url}/mermaid@{MERMAID_VERSION}/dist/mermaid.esm.min.mjs");
    format!(
        "<!doctype html><html><head><meta charset=\"utf-8\">\
<script>window.__dmConsoleErrors=[];var __e=console.error.bind(console);\
console.error=function(){{window.__dmConsoleErrors.push(\
Array.prototype.map.call(arguments,String).join(' '));__e.apply(console,arguments);}};</script>\
<script type=\"importmap\">{{\"imports\":{{\
\"{primary_url}\":\"{primary_local}\",\
\"{fallback_url}\":\"{fallback_local}\"}}}}</script>\
</head><body style=\"margin:0;background:#fff;\">{fragment}</body></html>"
    )
}

/// A live-DOM probe that waits (bounded) for the REAL Mermaid engine to replace
/// the `<pre class="mermaid">` source with an SVG, then reports engine-identity
/// and computed-theme signals as a `key=value;…` payload (parsed by
/// [`parse_kv`]). `fill`/`stroke`/`labelColor` are `getComputedStyle` results
/// (`rgb(…)` — no `;`/`=`, so kv-safe) proving the palette reached the engine.
const MERMAID_REAL_PROBE: &str = r#"(async () => {
  const deadline = Date.now() + 6000;
  const done = function () {
    const s = document.querySelector('.mermaid svg');
    return s && s.querySelectorAll('g.node').length > 0;
  };
  while (Date.now() < deadline
      && !done()
      && (window.__dmConsoleErrors || []).length === 0) {
    await new Promise(function (r) { setTimeout(r, 25); });
  }
  const errs = (window.__dmConsoleErrors || []).length;
  const svg = document.querySelector('.mermaid svg');
  if (!svg) return 'svg=0;errs=' + errs;
  const role = svg.getAttribute('aria-roledescription') || '';
  const stub = document.querySelectorAll('[data-stub]').length;
  const nodes = svg.querySelectorAll('g.node').length;
  const edges = svg.querySelectorAll('.edgePaths path, path.flowchart-link').length;
  const nodeEl = svg.querySelector('g.node');
  const shape = nodeEl ? nodeEl.querySelector('rect, polygon, path, circle, ellipse') : null;
  const cs = shape ? getComputedStyle(shape) : null;
  const fill = cs ? cs.fill : '';
  const stroke = cs ? cs.stroke : '';
  const labelEl = svg.querySelector('.nodeLabel, g.node text');
  const labelColor = labelEl ? getComputedStyle(labelEl).color : '';
  return 'svg=1;errs=' + errs + ';role=' + role + ';stub=' + stub
    + ';nodes=' + nodes + ';edges=' + edges
    + ';fill=' + fill + ';stroke=' + stroke + ';labelColor=' + labelColor;
})()"#;

/// The REAL pinned Mermaid 11.6.0 engine, served over loopback, must render a
/// genuine (non-stub) flowchart SVG and honor the resolved `themeVariables`:
/// rendering the same diagram under the light and dark palettes must yield a
/// DIFFERENT computed node fill. This is the proof the stub tests cannot give —
/// that real Mermaid exports the bootstrap's API shape, parses the diagram, and
/// applies the palette (criteria 3/11). Runtime is loopback-only; no external
/// network is touched.
#[tokio::test]
#[serial(browser)]
async fn browser_mermaid_real_engine_renders_and_themes() {
    if !require_browser() {
        return;
    }
    let dist = mermaid_fixture_dist();
    if !dist.join("mermaid.esm.min.mjs").is_file() {
        eprintln!(
            "skipping browser_mermaid_real_engine_renders_and_themes: vendored Mermaid \
             {MERMAID_VERSION} fixtures absent at {} — run \
             `node darkmatter/lib/tests/fixtures/mermaid/regen.mjs`",
            dist.display(),
        );
        return;
    }

    let (port, server) = spawn_mermaid_loopback_server(dist).await;
    let base = format!("http://127.0.0.1:{port}");

    let mut harness = ChromeHarness::new();
    harness.spawn().await.expect("spawn chrome");

    let dark_doc =
        wrap_mermaid_loopback_page(&mermaid_body_fragment(ColorMode::Dark, MERMAID_ONE_DOC), &base);
    harness.render_html(&dark_doc).await.expect("render dark");
    let dark_raw = harness.evaluate(MERMAID_REAL_PROBE).await.expect("dark probe");
    let dark = parse_kv(&dark_raw);

    let light_doc = wrap_mermaid_loopback_page(
        &mermaid_body_fragment(ColorMode::Light, MERMAID_ONE_DOC),
        &base,
    );
    harness.render_html(&light_doc).await.expect("render light");
    let light_raw = harness.evaluate(MERMAID_REAL_PROBE).await.expect("light probe");
    let light = parse_kv(&light_raw);

    harness.shutdown().await;
    server.abort();

    // A real SVG appears in both modes.
    assert_eq!(
        dark.get("svg").map(String::as_str),
        Some("1"),
        "real Mermaid must render an SVG (dark); got {dark_raw}",
    );
    assert_eq!(
        light.get("svg").map(String::as_str),
        Some("1"),
        "real Mermaid must render an SVG (light); got {light_raw}",
    );

    // It is the REAL engine, not a stub: flowchart role present, no stub marker.
    for (mode, kv, raw) in [("dark", &dark, &dark_raw), ("light", &light, &light_raw)] {
        assert!(
            kv.get("role").map(String::as_str).unwrap_or("").contains("flowchart"),
            "the {mode} SVG must carry Mermaid's flowchart aria-roledescription \
             (real engine output); got {raw}",
        );
        assert_eq!(
            kv.get("stub").map(String::as_str),
            Some("0"),
            "the {mode} SVG must NOT be the handwritten stub (data-stub); got {raw}",
        );
        assert!(
            kv.get("nodes").and_then(|v| v.parse::<u32>().ok()).unwrap_or(0) >= 2,
            "the {mode} flowchart must render both nodes as real Mermaid <g class=node>; got {raw}",
        );
        assert_eq!(
            kv.get("errs").map(String::as_str),
            Some("0"),
            "the real engine must load and render without a console.error ({mode}); got {raw}",
        );
    }

    // The resolved themeVariables reached the engine: the same diagram under the
    // light vs dark palette must compute a DIFFERENT node fill. `mainBkg`
    // resolves to `#1e1e1e` (dark) vs `#ececff` (light), so the base-theme node
    // fill must differ — a fact only the real engine, honoring themeVariables,
    // can produce.
    let dark_fill = dark.get("fill").map(String::as_str).unwrap_or("");
    let light_fill = light.get("fill").map(String::as_str).unwrap_or("");
    assert!(
        !dark_fill.is_empty() && !light_fill.is_empty(),
        "a node fill must compute in both modes; dark={dark_fill:?} light={light_fill:?} \
         (dark={dark_raw}) (light={light_raw})",
    );
    assert_ne!(
        dark_fill, light_fill,
        "real Mermaid must apply the resolved themeVariables — light and dark node fills \
         must differ; dark={dark_fill:?} light={light_fill:?}",
    );
}
