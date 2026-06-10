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
