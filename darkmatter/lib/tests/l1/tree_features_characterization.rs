//! Pre-change characterization of the behaviors the *Tree Features* cutover
//! (`renderable/features/_completed/2026-06-06-tree-features/`) re-architected.
//!
//! Phase 1 of that plan captures the current observable output so every later
//! phase's snapshot or output change can be classified as **intended** or
//! **regressive**. These cases deliberately exercise the production paths that
//! later phases rewrite:
//!
//! - component color opacity, which today survives only via the
//!   `darkmatter.style` hint + `apply_style_merges` post-render `rgba(...)`
//!   rewrite (deleted in Phase 7, replaced by `PaintColor` in Phase 3/6);
//! - the `transparent` / `currentColor` / `inherit` CSS keyword degradation;
//! - structured-link directive lowering to `<a>` attributes, which today runs
//!   through `inject_link_image_attributes` sentinel-class rewriting (replaced
//!   by typed browser attrs in Phase 5/6);
//! - per-node inline CSS winning property-by-property over the frontmatter
//!   `local-image-style` overlay;
//! - width-dependent hyperlink-label / image-placeholder layout and
//!   right/center list-item body placement, today driven by the decorate pass
//!   and `darkmatter.li` hints (replaced by `TextLayoutHints` in Phase 4/5).
//!
//! These are intentionally string-output snapshots of the **current** behavior,
//! not a forward contract. When a later phase changes one, review the diff and
//! re-accept it with a rationale — exactly as the cutover-reference corpus is
//! handled.
//!
//! ## Phase 8 review: re-baselined snapshots (all accepted as intended)
//!
//! The cutover changed seven of these snapshots. Each diff is the intended
//! outcome of the typed-tree architecture, reviewed and accepted individually:
//!
//! - **alpha / component color** (`char_component_color_opaque`,
//!   `char_component_color_half_opacity`, `…_transparent`, `…_current`,
//!   `…_inherit`): colors now lower through the shared `PaintColor` → `CssColor`
//!   helper, so an opaque color emits `rgb(r, g, b)` (not `#rrggbb`) and a
//!   half-opacity color emits a **single** `rgba(r, g, b, a)` declaration. The
//!   pre-cutover double declaration (`background-color:#fb2c36;background-color:
//!   rgba(…)`) and its `class=""` opacity sentinel are gone — the alpha now
//!   rides `PaintColor` straight to one direct declaration. The alpha literal is
//!   the exact `u8` normalization (`128/255 = 0.5019608`), the canonical stored
//!   form per the spec's `Opacity` design.
//! - **structured link attributes** (`char_structured_link_attributes`): the
//!   inline style is the validated `CssStyle` canonical form (`color:blue`), and
//!   `target` / `data-*` / `title` are typed browser attrs emitted in
//!   deterministic order — replacing the sentinel-class + post-render
//!   opening-tag rewrite.
//! - **local image inline CSS precedence** (`char_local_image_styling`): the
//!   per-image and frontmatter CSS merge into **one** validated `inline_style`
//!   attribute (was two `style=` attributes), and the image `alt` is the real
//!   source text (`alt="A"`) — the terminal placeholder's trailing-space padding
//!   no longer leaks into the browser alt, honoring "source content is never
//!   mutated during construction."
//!
//! At Phase 8 the terminal characterization snapshots (`char_hyperlink_*`,
//! `char_list_item_*`) and `char_terminal_drops_component_alpha` were unchanged:
//! width-dependent text layout and alpha degradation already matched the typed
//! `TextLayoutHints` / `PaintColor` resolution.
//!
//! ## Tree-closeout review: re-baselined terminal snapshots (accepted)
//!
//! Two later tree-closeout fixes deliberately decoupled page *capabilities* and
//! *content width* from component-policy presence, keying both on page-frame
//! geometry alone (`DarkmatterPage::render`, review-2 finding 2 / review-5
//! finding 1):
//!
//! - `fix(darkmatter): gate page-frame width cap on frame geometry` — a page
//!   carrying only component `style:` (no margin/padding/max-width) resolves its
//!   content box at the **ambient** width, not the captured frame width.
//! - `fix(darkmatter): reuse owned render tree across page probe and fold` —
//!   `optimistic_capabilities` (TrueColor + OSC8) is now selected by frame
//!   geometry alone, never by a matched component policy.
//!
//! These re-baselined three of the terminal snapshots, all accepted as intended:
//!
//! - **`char_hyperlink_exact_width` / `char_hyperlink_max_width`**: the
//!   component-only `hyperlinks.*` page has no frame geometry, so it no longer
//!   promotes the document to OSC8. The padded/truncated label is unchanged
//!   (`TextLayoutHints` still apply); only the link emission degrades to the
//!   non-OSC8 `[label](url)` markdown fallback the ambient terminal advertises.
//! - **`char_list_item_right_alignment` / `char_list_item_center_alignment`**:
//!   the component-only `li.*` page resolves at the ambient content width rather
//!   than the captured frame width, so right/center placement pads against the
//!   wider ambient field (right padding stays exactly twice the center padding).
//!
//! `char_terminal_drops_component_alpha` is still unchanged — both its renders
//! shift identically, so the equality it asserts holds.
//!
//! ## Known stale snapshots (do not "fix" here)
//!
//! `cutover_reference.rs` currently has five red browser snapshots
//! (`reference_block_quote_width_and_left`, `reference_centered_table`,
//! `reference_list_left_margin`, `reference_page_background_pronounced`,
//! `reference_table_max_width`). They expect the page wrapper's default
//! horizontal margins as `0ch` while the current centering implementation emits
//! `auto`. Per the plan these are deferred to Phase 8 review (accept `auto`
//! margins only with an explicit CSS rationale); they are *not* in scope for
//! Phase 1 characterization and must not be re-baselined blindly.

use biscuit_terminal::terminal::Terminal;
use darkmatter::layout::DarkmatterPage;
use darkmatter::markdown::Markdown;
use darkmatter::markdown::output::HtmlOptions;
use darkmatter::style::{
    BespokeStyleOverrides, ComponentStyleOverrides, HrStyleOverrides, ListStyleOverrides,
    PageStyleOverrides, apply_bespoke_style, apply_color_style, apply_component_style,
    apply_hr_style, apply_list_style, apply_page_style, from_frontmatter,
};

// ---------------------------------------------------------------------------
// Helpers (mirrors cutover_reference.rs so both corpora build identically)
// ---------------------------------------------------------------------------

/// Build a Markdown document whose frontmatter carries the given `style:` block.
fn with_style(style_yaml: &str, content: &str) -> Markdown {
    let full = format!("---\nstyle:\n{}---\n\n{}", indent(style_yaml, 4), content);
    Markdown::try_from_content(&full).expect("parse markdown with style frontmatter")
}

fn indent(text: &str, spaces: usize) -> String {
    let prefix = " ".repeat(spaces);
    text.lines()
        .map(|line| {
            if line.trim().is_empty() {
                line.to_string()
            } else {
                format!("{}{}", prefix, line)
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

/// Apply the full `style:` pipeline (page → component → list → hr → color →
/// bespoke) with no CLI overrides.
fn apply_all_styles(page: DarkmatterPage, md: &Markdown) -> DarkmatterPage {
    let (style, _warnings) = from_frontmatter(md.frontmatter()).expect("parse style");

    let page = apply_page_style(page, &style, PageStyleOverrides::default())
        .expect("apply_page_style");
    let page = apply_component_style(page, &style, ComponentStyleOverrides::default())
        .expect("apply_component_style");
    let page = apply_list_style(page, &style, ListStyleOverrides::default())
        .expect("apply_list_style");
    let page = apply_hr_style(page, &style, HrStyleOverrides::default())
        .expect("apply_hr_style");
    let page = apply_color_style(page, &style).expect("apply_color_style");
    apply_bespoke_style(page, &style, BespokeStyleOverrides::default(), None)
        .expect("apply_bespoke_style")
}

/// Render `md` to terminal through a `DarkmatterPage` at the given width.
fn render_terminal(md: &Markdown, width: u32) -> String {
    let term = Terminal::new_optimistic(width);
    let page = DarkmatterPage::new(&term);
    let page = apply_all_styles(page, md);
    page.render(md).expect("terminal render")
}

/// Render `md` to browser HTML through a `DarkmatterPage`, then keep only the
/// `<body>` content so the snapshot is the component markup we are
/// characterizing rather than the large `:root` design-token preamble.
fn render_browser_body(md: &Markdown) -> String {
    let term = Terminal::new_optimistic(80);
    let page = DarkmatterPage::new(&term);
    let page = apply_all_styles(page, md);
    let html = page.render_to_browser(md).expect("browser render");
    body_of(&html)
}

/// Render the raw markdown link/image directives to HTML via the direct
/// `Markdown::as_html` path (no `style:` frontmatter needed), keeping only the
/// `<body>` content.
fn render_as_html_body(md: &Markdown) -> String {
    let html = md.as_html(HtmlOptions::default()).expect("as_html");
    body_of(&html)
}

/// Extract the inner `<body>…</body>` markup, falling back to the whole string
/// when no `<body>` is present.
fn body_of(html: &str) -> String {
    match (html.find("<body>"), html.find("</body>")) {
        (Some(start), Some(end)) => html[start + "<body>".len()..end].to_string(),
        _ => html.to_string(),
    }
}

// ---------------------------------------------------------------------------
// 1. Component colors: opaque and `/50` opacity
// ---------------------------------------------------------------------------

#[test]
fn char_component_color_opaque() {
    let md = with_style(
        "block-quote:\n  bg-color: red-500\n",
        "> A quoted paragraph.\n",
    );
    insta::assert_snapshot!("char_component_color_opaque_browser", render_browser_body(&md));
}

#[test]
fn char_component_color_half_opacity() {
    // `/50` opacity today survives to the browser only through the
    // `darkmatter.style` hint + post-render `rgba(...)` merge.
    let md = with_style(
        "block-quote:\n  bg-color: red-500/50\n",
        "> A quoted paragraph.\n",
    );
    let body = render_browser_body(&md);
    assert!(
        body.contains("rgba("),
        "half-opacity component bg-color must lower to rgba(...) today. body:\n{body}"
    );
    insta::assert_snapshot!("char_component_color_half_opacity_browser", body);
}

// ---------------------------------------------------------------------------
// 2. Tailwind keyword colors with opacity: transparent / current / inherit
// ---------------------------------------------------------------------------

#[test]
fn char_component_color_transparent_with_opacity() {
    let md = with_style(
        "block-quote:\n  color: transparent/50\n",
        "> A quoted paragraph.\n",
    );
    insta::assert_snapshot!(
        "char_component_color_transparent_browser",
        render_browser_body(&md)
    );
}

#[test]
fn char_component_color_current_with_opacity() {
    let md = with_style(
        "block-quote:\n  color: current/50\n",
        "> A quoted paragraph.\n",
    );
    insta::assert_snapshot!(
        "char_component_color_current_browser",
        render_browser_body(&md)
    );
}

#[test]
fn char_component_color_inherit_with_opacity() {
    let md = with_style(
        "block-quote:\n  color: inherit/50\n",
        "> A quoted paragraph.\n",
    );
    insta::assert_snapshot!(
        "char_component_color_inherit_browser",
        render_browser_body(&md)
    );
}

// ---------------------------------------------------------------------------
// 3. Structured link directives -> <a> attributes
// ---------------------------------------------------------------------------

#[test]
fn char_structured_link_attributes() {
    // Directive in the title position: class, inline style, target, plain title,
    // a `prompt` (consumed into the Phase-4 popover structure), and a custom
    // data-* attribute.
    let md = Markdown::try_from_content(
        "[Click](https://example.com \"class='btn' style='color: blue;' target='_blank' title='Go' prompt='Run it' data-id='42'\")\n",
    )
    .expect("parse structured link");
    let body = render_as_html_body(&md);
    assert!(body.contains(r#"class="btn""#), "class lost. body:\n{body}");
    assert!(body.contains(r#"target="_blank""#), "target lost. body:\n{body}");
    // Phase 4: the prompt lowers to the accessible popover markup rather than a
    // `data-prompt` transport attribute.
    assert!(
        !body.contains("data-prompt="),
        "internal prompt transport leaked. body:\n{body}"
    );
    assert!(
        body.contains(r#"popover="hint""#) && body.contains("Run it"),
        "prompt lost from popover markup. body:\n{body}"
    );
    assert!(body.contains(r#"data-id="42""#), "custom data-* lost. body:\n{body}");
    insta::assert_snapshot!("char_structured_link_attributes_html", body);
}

// ---------------------------------------------------------------------------
// 4. Local image styling: frontmatter overlay + component color
// ---------------------------------------------------------------------------

#[test]
fn char_local_image_styling() {
    // Frontmatter `images.local-style` sets color + width on a local image while
    // a per-image `style='color: blue;'` title directive overrides the color.
    // The per-image and frontmatter CSS merge into ONE validated `inline_style`
    // with per-node precedence (blue wins over red), the `width` survives, the
    // raw directive does not leak as a literal `title`, and `alt` is the real
    // source text (review-1, finding 2).
    let md = with_style(
        "images:\n  local-style:\n    color: red-500\n    width: 40\n",
        "![A](./local.png \"style='color: blue;'\")\n",
    );
    let body = render_browser_body(&md);
    assert!(
        body.contains("color:blue"),
        "per-node blue must win over frontmatter red. body:\n{body}"
    );
    assert!(
        !body.contains("color:rgb(251, 44, 54)"),
        "frontmatter red must be overridden by the per-node declaration. body:\n{body}"
    );
    assert!(
        !body.contains("title="),
        "raw `style='...'` directive must not leak as an HTML title. body:\n{body}"
    );
    assert_eq!(
        body.matches("style=").count(),
        1,
        "exactly one merged inline style attribute expected. body:\n{body}"
    );
    insta::assert_snapshot!("char_local_image_inline_css_precedence_browser", body);
}

// ---------------------------------------------------------------------------
// 5. Hyperlink-label width vs max-width (terminal)
// ---------------------------------------------------------------------------

#[test]
fn char_hyperlink_exact_width_pads() {
    // `width` establishes an exact field width: the short label is padded.
    let md = with_style(
        "hyperlinks:\n  width: 20\n",
        "[HI](https://example.com)\n",
    );
    insta::assert_snapshot!(
        "char_hyperlink_exact_width_terminal",
        render_terminal(&md, 60)
    );
}

#[test]
fn char_hyperlink_max_width_truncates() {
    // `max-width` only truncates content that exceeds the cap.
    let md = with_style(
        "hyperlinks:\n  max-width: 8\n",
        "[A very long hyperlink label](https://example.com)\n",
    );
    insta::assert_snapshot!(
        "char_hyperlink_max_width_terminal",
        render_terminal(&md, 60)
    );
}

#[test]
fn char_hyperlink_exact_width_truncates_long_label() {
    // Regression (review-1, finding 1): an exact `width` is an exact field, so a
    // label longer than the field is truncated with an ellipsis — matching the
    // retired `apply_inline_text_layout` contract. Before the fix the typed
    // hint carried `TextOverflow::Preserve`, so the long label overflowed the
    // five-column field unchanged.
    let md = with_style(
        "hyperlinks:\n  width: 5\n",
        "[A very long hyperlink label](https://example.com)\n",
    );
    let out = render_terminal(&md, 60);
    assert!(
        out.contains('…'),
        "exact-width long label must be truncated with an ellipsis. out:\n{out}"
    );
    assert!(
        !out.contains("hyperlink label"),
        "the overflowing tail of the label must be removed. out:\n{out}"
    );
}

#[test]
fn char_image_alt_exact_width_truncates_long_alt() {
    // Regression (review-1, finding 1): a local image's exact `width` truncates
    // a long alt-text placeholder rather than letting it overflow the field.
    //
    // Regression (review-2): the exact `width` governs the *complete* visible
    // placeholder (block prefix + brackets), not the bare alt run. A six-column
    // field must yield a six-column visible placeholder — framing included.
    use biscuit_terminal::utils::block_constraint::visible_width;
    use biscuit_terminal::utils::escape_codes::strip_escape_codes;

    let md = with_style(
        "images:\n  local-style:\n    width: 6\n",
        "![A very long image alt text](./local.png)\n",
    );
    let out = render_terminal(&md, 60);
    assert!(
        out.contains('…'),
        "exact-width long image alt must be truncated with an ellipsis. out:\n{out}"
    );
    assert!(
        !out.contains("image alt text"),
        "the overflowing tail of the alt text must be removed. out:\n{out}"
    );
    let placeholder_line = out
        .lines()
        .find(|l| l.contains('…'))
        .expect("placeholder line present in output");
    let visible = strip_escape_codes(placeholder_line);
    assert_eq!(
        visible_width(visible.trim_end()),
        6,
        "the complete visible placeholder must fill exactly the 6-column field, \
         framing included. line: {visible:?}"
    );
}

// ---------------------------------------------------------------------------
// 6. List-item body placement: right and center
// ---------------------------------------------------------------------------

#[test]
fn char_list_item_right_alignment() {
    let md = with_style(
        "li:\n  alignment: right\n  max-width: 40\n",
        "- Item one\n- Item two\n",
    );
    insta::assert_snapshot!(
        "char_list_item_right_alignment_terminal",
        render_terminal(&md, 60)
    );
}

#[test]
fn char_list_item_center_alignment() {
    let md = with_style(
        "li:\n  alignment: center\n  max-width: 40\n",
        "- Item one\n- Item two\n",
    );
    insta::assert_snapshot!(
        "char_list_item_center_alignment_terminal",
        render_terminal(&md, 60)
    );
}

// ---------------------------------------------------------------------------
// 7. Terminal degrades alpha to the underlying opaque color
// ---------------------------------------------------------------------------

#[test]
fn char_terminal_drops_component_alpha() {
    // The same half-opacity bg-color that the browser lowers to rgba(...) is
    // rendered by the terminal using the underlying opaque color. Capture the
    // current terminal output so the documented degradation is pinned.
    let md = with_style(
        "block-quote:\n  bg-color: red-500/50\n",
        "> A quoted paragraph.\n",
    );
    let opaque = with_style(
        "block-quote:\n  bg-color: red-500\n",
        "> A quoted paragraph.\n",
    );
    assert_eq!(
        render_terminal(&md, 60),
        render_terminal(&opaque, 60),
        "terminal must render the half-opacity bg identically to the opaque bg"
    );
}
