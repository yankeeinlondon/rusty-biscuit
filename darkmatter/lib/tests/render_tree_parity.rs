//! Parity gate between the legacy darkmatter renderers and the render-tree
//! pipeline.
//!
//! This integration test exists to gate the eventual darkmatter migration: it
//! compares the **legacy** renderers (`Markdown::as_html` and the terminal
//! `for_terminal`) against the **render-tree** pipeline (the darkmatter fold
//! `fold_markdown_to_document`, then `render_browser_document` /
//! `render_terminal_document`).
//!
//! ## What parity means here
//!
//! The legacy renderers and the tree pipeline are **different renderers**.
//! Their output is *not* byte-identical, and that is expected — whitespace,
//! delimiter choices, wrapping, ANSI palette, and HTML structure all differ.
//! Asserting byte-equality would only test that two implementations are the
//! same implementation.
//!
//! Instead, every assertion in this file checks a **semantic invariant**.
//! Two token classes are checked:
//!
//! - **Visible-text tokens** — heading text, code-block content, list-item
//!   text, table-cell text, inline-styled words. Asserted against output with
//!   ANSI escapes / HTML tags stripped.
//! - **Attribute-bound tokens** — link/image targets and image **alt** text.
//!   These live inside HTML attributes (`href="…"`, `src="…"`, `alt="…"`), so
//!   tag-stripping would discard them; they are asserted against the *raw*
//!   (un-stripped) output instead.
//!
//! A token present in one pipeline's output but missing from the other fails
//! the test with a message naming the fixture, the token, and the pipeline
//! that dropped it. Pure formatting differences never fail the test.
//!
//! ## Observed parity classification
//!
//! The plan's parity buckets are: *acceptable formatting difference*,
//! *semantic mismatch*, *missing feature*, *bug in old renderer*, *bug in new
//! renderer*. The differences observed between the two pipelines, per
//! construct, are:
//!
//! - **Headings** — *acceptable formatting difference*. Legacy HTML emits
//!   `<h1>`..`<h6>` with slug `id` anchors; the tree browser renderer emits
//!   the same heading tags. Terminal output differs in ANSI palette and
//!   underline styling only. Heading text is preserved by both.
//! - **Paragraphs / inline styles** — *acceptable formatting difference*.
//!   Emphasis/strong/strikethrough map to `<em>`/`<strong>`/`<del>` (HTML) and
//!   ANSI SGR runs (terminal) in both pipelines; only the surrounding
//!   whitespace and wrap columns differ.
//! - **Links and images** — *acceptable formatting difference*. Both
//!   pipelines preserve the URL and the link/alt text. HTML structure
//!   (`<a href>` / `<img src>`) is equivalent; the terminal surfaces differ in
//!   presentation (legacy emits `text [url]` and an image glyph, the tree
//!   renderer emits `[text](url)`), but both keep the URL and link/alt text.
//! - **Code blocks** — *acceptable formatting difference*. Both pipelines
//!   syntax-highlight; the exact span markup and palette differ, but the code
//!   text is preserved verbatim.
//! - **Lists / task lists** — *acceptable formatting difference*. Bullet
//!   glyphs, indentation, and checkbox glyphs differ; every item's text is
//!   preserved by both.
//! - **Tables** — *acceptable formatting difference*. Legacy terminal output
//!   uses biscuit-terminal box-drawing; the tree terminal renderer reuses the
//!   same `Table` component. Cell text is preserved by both pipelines.
//! - **Raw HTML (browser)** — *acceptable formatting difference / known
//!   dialect gap*. The legacy HTML renderer passes raw HTML through; the tree
//!   browser renderer defaults to [`RawHtmlPolicy::Escape`]. This test renders
//!   the tree browser output with [`RawHtmlPolicy::Allow`] so raw-HTML
//!   fixtures compare on equal footing; the escape default is a deliberate
//!   safety choice, not a parity bug. Both pipelines preserve the raw block
//!   and inline HTML content on the browser surface.
//! - **Raw HTML (terminal)** — *bug in old renderer*. The legacy terminal
//!   renderer **silently drops raw block HTML**: a `<div>…</div>` block and
//!   its inner text never reach the terminal output, and inline HTML tags are
//!   stripped while their text survives. The tree terminal renderer is the
//!   *more faithful* of the two — it preserves the raw HTML verbatim. Because
//!   the legacy renderer loses content the tree renderer keeps, raw-block-HTML
//!   terminal parity cannot be asserted as a both-sides invariant; the
//!   `render_tree_parity_raw_html` test asserts browser parity and the
//!   terminal text the *legacy* renderer does retain. This is a pre-existing
//!   legacy-renderer gap, **not** a render-tree regression.
//!
//! No *semantic mismatch*, *missing feature*, or *bug-in-new-renderer* bucket
//! is triggered by the fixtures below: the render-tree pipeline never drops a
//! semantic token that the legacy renderer keeps. Should a future fold or
//! renderer change drop content, the invariant assertions here fail and name
//! the offending token.

use std::rc::Rc;

use biscuit_terminal::render_tree::{TerminalRenderOptions, render_terminal_document};
use darkmatter::markdown::Markdown;
use darkmatter::markdown::output::{HtmlOptions, TerminalOptions, as_html, for_terminal};
use darkmatter::markdown::render_tree::{
    TerminalCodeRenderer, fold_markdown_spanned_with_frontmatter, fold_markdown_to_document,
};
use renderable::tree::{
    BrowserRenderOptions, Diagnostic, Document, MarkdownDialect, MarkdownRenderOptions, NodeKind,
    RawHtmlPolicy, RenderNode, RenderStrictness, SourceDescriptor, render_browser_document,
    render_markdown_document,
};

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

/// A focused parity corpus covering the common Markdown constructs.
///
/// Each entry is `(name, markdown)`. Inputs are kept inline so the parity
/// expectations and the source travel together; the existing
/// `tests/fixtures/render_tree/*.md` files are exercised by the round-trip
/// test instead.
const FIXTURES: &[(&str, &str)] = &[
    (
        "headings",
        "# Top Level\n\n## Second Level\n\n### Third Level\n",
    ),
    (
        "paragraph",
        "A single paragraph of plain prose that exercises the simplest fold.\n",
    ),
    (
        "inline_styles",
        "This paragraph mixes *emphasis*, **strong**, and ~~strikethrough~~ words.\n",
    ),
    (
        "links_images",
        "A paragraph with a [Example Link](https://example.com \"Example\") and an \
         ![descriptive alt](image.png) image reference.\n",
    ),
    (
        "lists",
        "- alpha bullet\n- beta bullet\n\n1. one ordered\n2. two ordered\n",
    ),
    (
        "task_list",
        "- [x] completed task entry\n- [ ] pending task entry\n",
    ),
    (
        "code_block",
        "```rust\nfn parity_demo() {\n    println!(\"render tree\");\n}\n```\n",
    ),
    (
        "table",
        "| Fruit | Quantity |\n|:------|---------:|\n| apples | 3 |\n| pears | 12 |\n",
    ),
    (
        "blockquote",
        "> A quoted line of prose.\n>\n> A second quoted paragraph.\n",
    ),
    (
        "raw_html",
        "A paragraph then a block:\n\n<div class=\"callout\">raw block content</div>\n\n\
         Trailing paragraph with <strong>inline html</strong> too.\n",
    ),
    (
        "mark",
        "Plain text with ==highlighted phrase== inside it.\n",
    ),
    (
        "dim",
        "Plain text with \u{2304}dimmed phrase\u{2304} inside it.\n",
    ),
    (
        "hr_attributes",
        "Lead paragraph.\n\n--- { style: waves }\n\nTrailing paragraph.\n",
    ),
    ("hr_attributes_in_list", "- --- { style: waves }\n"),
    (
        "code_block_rich",
        "```rust title=\"Demo Snippet\" line-numbering=true highlight=2\n\
         fn parity_demo() {\n    println!(\"render tree\");\n}\n```\n",
    ),
    (
        "image_titled",
        "An image ![descriptive alt](photo.png \"Hover Title\") inline.\n",
    ),
    (
        "footnote",
        "Body text with a reference.[^note]\n\n[^note]: The footnote definition body.\n",
    ),
    // `pulldown-cmark` superscript / subscript only fire when the delimiters
    // flank a word boundary (`^text^` / `~text~`); mid-word forms like `x^2^`
    // parse as plain text. The fixtures use the word-boundary form.
    ("superscript", "The ^2nd^ entry wins the race.\n"),
    ("subscript", "Take the ~note~ for later.\n"),
    ("mermaid_off", "```mermaid\ngraph TD\n  A --> B\n```\n"),
];

// ---------------------------------------------------------------------------
// Text-extraction helpers
// ---------------------------------------------------------------------------

/// Strips ANSI / OSC escape sequences from terminal output.
///
/// Handles both CSI (`ESC [ … final`) and OSC (`ESC ] … BEL|ST`) sequences so
/// OSC8 hyperlink wrappers do not pollute the visible-text comparison.
fn strip_ansi(input: &str) -> String {
    let bytes: Vec<char> = input.chars().collect();
    let mut out = String::with_capacity(input.len());
    let mut idx = 0;
    while idx < bytes.len() {
        let ch = bytes[idx];
        if ch == '\u{1b}' && idx + 1 < bytes.len() {
            match bytes[idx + 1] {
                '[' => {
                    // CSI: consume up to a final byte in 0x40..=0x7e.
                    idx += 2;
                    while idx < bytes.len() && !('\u{40}'..='\u{7e}').contains(&bytes[idx]) {
                        idx += 1;
                    }
                    idx += 1;
                }
                ']' => {
                    // OSC: consume until BEL or ESC \ (ST).
                    idx += 2;
                    while idx < bytes.len() {
                        if bytes[idx] == '\u{07}' {
                            idx += 1;
                            break;
                        }
                        if bytes[idx] == '\u{1b}' && idx + 1 < bytes.len() && bytes[idx + 1] == '\\'
                        {
                            idx += 2;
                            break;
                        }
                        idx += 1;
                    }
                }
                _ => idx += 1,
            }
        } else {
            out.push(ch);
            idx += 1;
        }
    }
    out
}

/// Strips HTML tags and decodes the handful of entities the renderers emit.
fn strip_html(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut in_tag = false;
    for ch in input.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }
    out.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
}

/// Collapses all whitespace runs to single spaces for tolerant comparison.
fn normalize(input: &str) -> String {
    input.split_whitespace().collect::<Vec<_>>().join(" ")
}

// ---------------------------------------------------------------------------
// Pipeline drivers
// ---------------------------------------------------------------------------

/// Asserts a fold emitted no diagnostics, naming the fixture and fold path.
///
/// Every fixture in this suite is an **equivalence** fixture: it must fold
/// cleanly on both the plain and span-aware paths (the parser-option
/// divergences fold to *structural* nodes, not diagnostics). Surfacing fold
/// diagnostics here keeps them separately visible from render diagnostics and
/// makes a parity failure specific enough to identify a fold-vs-renderer
/// mismatch (DMTR-5): a fold-time `Unsupported` / `Lossy` / `Structural`
/// diagnostic fails at the fold boundary naming the fixture, before any
/// rendered-token comparison runs. See review-12 finding 1.
///
/// ## Panics
///
/// Panics naming the fixture, fold path, and the offending diagnostics if the
/// fold emitted any.
fn assert_fold_clean(fixture: &str, path: &str, diagnostics: &[Diagnostic]) {
    assert!(
        diagnostics.is_empty(),
        "[{fixture}/{path}] fold emitted unexpected diagnostics (fold-vs-render \
         mismatch): {diagnostics:#?}",
    );
}

/// Folds parity fixture `name` to a render-tree [`Document`], asserting the
/// fold emitted no diagnostics (see [`assert_fold_clean`]).
fn fold(name: &str, markdown: &str) -> Document {
    let source = SourceDescriptor::Virtual { name: name.into() };
    let (doc, diags) = fold_markdown_to_document(source, markdown);
    assert_fold_clean(name, "fold", &diags);
    doc
}

/// Folds parity fixture `name` to a render-tree [`Document`] through the
/// **span-aware** chain — the path that surfaces `==mark==`, `⌄dim⌄`, and
/// `--- { ... }` HR-attribute constructs. Equivalent to the path the
/// experimental `to_render_document` entry point uses.
///
/// Asserts the span-aware fold emitted no diagnostics (see
/// [`assert_fold_clean`]).
fn fold_spanned(name: &str, markdown: &str) -> Document {
    let source = SourceDescriptor::Virtual { name: name.into() };
    let md: Markdown = markdown.into();
    let (doc, diags) = fold_markdown_spanned_with_frontmatter(source, &md);
    assert_fold_clean(name, "fold_spanned", &diags);
    doc
}

/// Renders fixture text to HTML via the legacy `as_html` renderer.
fn legacy_html(markdown: &str) -> String {
    let md: Markdown = markdown.into();
    as_html(&md, HtmlOptions::default()).expect("legacy as_html must succeed")
}

/// Builds [`BrowserRenderOptions`] that mirror the production HTML entry point
/// (`render_tree_html`): the [`TerminalCodeRenderer`] hook is wired so fenced
/// code blocks reproduce darkmatter's syntax-highlighted HTML (title block,
/// line-number table, highlighted-line markup) instead of the render tree's
/// plain `<pre><code>` fallback. [`RawHtmlPolicy::Allow`] keeps raw-HTML
/// fixtures comparable to the legacy renderer (which passes raw HTML through).
fn tree_html_options() -> BrowserRenderOptions {
    BrowserRenderOptions {
        raw_html: RawHtmlPolicy::Allow,
        code_renderer: Some(Rc::new(TerminalCodeRenderer::new())),
        ..BrowserRenderOptions::default()
    }
}

/// Renders fixture text to HTML via the render-tree browser renderer.
fn tree_html(name: &str, markdown: &str) -> String {
    let doc = fold(name, markdown);
    render_browser_document(&doc, &tree_html_options())
        .expect("tree browser render must succeed")
        .output
        .render()
}

/// Renders fixture text to a terminal string via the legacy `for_terminal`.
fn legacy_terminal(markdown: &str) -> String {
    let md: Markdown = markdown.into();
    for_terminal(&md, TerminalOptions::default()).expect("legacy for_terminal must succeed")
}

/// Builds [`TerminalRenderOptions`] that mirror the production tree terminal
/// entry point (`render_tree_terminal`): the [`TerminalCodeRenderer`] hook is
/// wired in so fenced code blocks reproduce darkmatter's syntax-highlighted
/// code-block path instead of the render tree's plain-fence fallback. A
/// default-width optimistic terminal keeps the visible width repeatable.
///
/// Closes review-10 finding 2: the parity harness must exercise the same
/// code path the user-observable entry point uses, not the bare
/// `TerminalRenderOptions::default()` (which sets `code_renderer: None`).
fn tree_terminal_options() -> TerminalRenderOptions {
    // Start from the same detected-terminal default the original parity
    // harness used (so hyperlink / color behavior is unchanged) and only add
    // the code-render hook on top.
    TerminalRenderOptions::default().with_code_renderer(Rc::new(TerminalCodeRenderer::new()))
}

/// Renders fixture text to a terminal string via the render-tree renderer.
fn tree_terminal(name: &str, markdown: &str) -> String {
    let doc = fold(name, markdown);
    render_terminal_document(&doc, &tree_terminal_options())
        .expect("tree terminal render must succeed")
        .output
}

/// Renders fixture text to HTML via the render-tree browser renderer, using
/// the **span-aware** fold. Uses [`RawHtmlPolicy::Allow`] so darkmatter HTML
/// fixtures stay comparable to the legacy renderer.
fn tree_html_spanned(name: &str, markdown: &str) -> String {
    let doc = fold_spanned(name, markdown);
    render_browser_document(&doc, &tree_html_options())
        .expect("tree browser render must succeed")
        .output
        .render()
}

/// Renders fixture text to a terminal string via the render-tree renderer
/// using the **span-aware** fold.
fn tree_terminal_spanned(name: &str, markdown: &str) -> String {
    let doc = fold_spanned(name, markdown);
    render_terminal_document(&doc, &tree_terminal_options())
        .expect("tree terminal render must succeed")
        .output
}

// ---------------------------------------------------------------------------
// Invariant assertion
// ---------------------------------------------------------------------------

/// Asserts that every `token` appears in both `legacy` and `tree` plain text.
///
/// `surface` names the output surface (`"HTML"` / `"terminal"`) and `fixture`
/// names the parity fixture; both feed the failure message so a regression
/// pinpoints which construct and which pipeline dropped the token.
///
/// ## Panics
///
/// Panics naming the fixture, surface, token, and offending pipeline if a
/// token is present in one output but missing from the other — a genuine
/// semantic parity gap.
fn assert_tokens_present(
    fixture: &str,
    surface: &str,
    legacy_plain: &str,
    tree_plain: &str,
    tokens: &[&str],
) {
    let legacy = normalize(legacy_plain);
    let tree = normalize(tree_plain);
    for token in tokens {
        let needle = normalize(token);
        let in_legacy = legacy.contains(&needle);
        let in_tree = tree.contains(&needle);
        assert!(
            in_legacy || in_tree,
            "[{fixture}/{surface}] token {token:?} is absent from BOTH pipelines — \
             fixture or extraction bug",
        );
        assert!(
            in_legacy,
            "[{fixture}/{surface}] semantic parity gap: token {token:?} present in the \
             render-tree output but MISSING from the legacy renderer",
        );
        assert!(
            in_tree,
            "[{fixture}/{surface}] semantic parity gap: token {token:?} present in the \
             legacy renderer but MISSING from the render-tree pipeline",
        );
    }
}

/// Asserts that every URL `token` appears in both `legacy` and `tree` *raw*
/// output.
///
/// URL tokens are checked against un-stripped output because in HTML they live
/// inside `href`/`src` attributes that tag-stripping would discard, and on the
/// terminal surface an OSC8 hyperlink carries the URL inside the escape
/// sequence that ANSI/OSC-stripping would discard. Pass raw output here.
///
/// ## Panics
///
/// Panics naming the fixture, surface, token, and offending pipeline if a URL
/// is present in one output but missing from the other.
fn assert_urls_present(
    fixture: &str,
    surface: &str,
    legacy_raw: &str,
    tree_raw: &str,
    urls: &[&str],
) {
    for url in urls {
        let in_legacy = legacy_raw.contains(url);
        let in_tree = tree_raw.contains(url);
        assert!(
            in_legacy,
            "[{fixture}/{surface}] semantic parity gap: URL {url:?} present in the \
             render-tree output but MISSING from the legacy renderer",
        );
        assert!(
            in_tree,
            "[{fixture}/{surface}] semantic parity gap: URL {url:?} present in the \
             legacy renderer but MISSING from the render-tree pipeline",
        );
    }
}

/// Looks up a fixture's Markdown source by name.
fn fixture(name: &str) -> &'static str {
    FIXTURES
        .iter()
        .find(|(fixture_name, _)| *fixture_name == name)
        .map(|(_, markdown)| *markdown)
        .unwrap_or_else(|| panic!("unknown parity fixture {name}"))
}

/// Runs HTML and terminal parity for one fixture.
///
/// `text_tokens` are visible-text tokens checked against tag/ANSI-stripped
/// output on both surfaces. `html_url_tokens` are link/image targets and
/// image alt text checked against the *raw HTML* output (HTML keeps them
/// inside attributes).
///
/// Attribute-bound tokens are **not** checked on the terminal surface here:
/// the two terminal renderers present link/image references differently
/// (the legacy renderer drops the image `src` URL entirely, for example), so
/// terminal URL parity is asserted per-fixture where it is meaningful.
fn assert_parity(name: &str, text_tokens: &[&str], html_url_tokens: &[&str]) {
    let markdown = fixture(name);

    let legacy_html_raw = legacy_html(markdown);
    let tree_html_raw = tree_html(name, markdown);
    assert_tokens_present(
        name,
        "HTML",
        &strip_html(&legacy_html_raw),
        &strip_html(&tree_html_raw),
        text_tokens,
    );
    assert_urls_present(
        name,
        "HTML",
        &legacy_html_raw,
        &tree_html_raw,
        html_url_tokens,
    );

    let legacy_term_raw = legacy_terminal(markdown);
    let tree_term_raw = tree_terminal(name, markdown);
    assert_tokens_present(
        name,
        "terminal",
        &strip_ansi(&legacy_term_raw),
        &strip_ansi(&tree_term_raw),
        text_tokens,
    );
}

// ---------------------------------------------------------------------------
// Per-fixture parity tests
// ---------------------------------------------------------------------------

#[test]
fn render_tree_parity_headings() {
    assert_parity(
        "headings",
        &["Top Level", "Second Level", "Third Level"],
        &[],
    );
}

#[test]
fn render_tree_parity_paragraph() {
    assert_parity(
        "paragraph",
        &["A single paragraph", "exercises the simplest fold"],
        &[],
    );
}

#[test]
fn render_tree_parity_inline_styles() {
    // The styled words must survive both renderers; the SGR / tag wrappers
    // around them are formatting and are stripped before comparison.
    assert_parity(
        "inline_styles",
        &["emphasis", "strong", "strikethrough", "words"],
        &[],
    );
}

/// Link and image parity.
///
/// Visible text (`Example Link`, `image reference`) and the HTML
/// attribute-bound tokens (`href`/`src`/`alt`) are checked by [`assert_parity`].
///
/// The link `href` is additionally checked on the **terminal** surface: both
/// terminal renderers surface it, but *where* the URL lives depends on
/// hyperlink support. With OSC8 enabled (a TTY-attached, capable terminal) the
/// legacy renderer embeds the URL **inside** the OSC8 escape sequence
/// (`ESC]8;;<url>BEL`) rather than as visible `text [url]`; the tree renderer
/// surfaces it as `[text](url)`. Either way the URL is present in the *raw*
/// output, so — exactly like the HTML attribute-bound tokens — it is asserted
/// against the un-stripped terminal output. Stripping ANSI/OSC first (as the
/// visible-text checks do) would discard the URL in OSC8 mode and make this
/// assertion fail spuriously when run attached to a terminal. The image `src`
/// URL is *not* checked on the terminal — neither terminal renderer emits an
/// image's `src`; both show only the alt text. That symmetric omission is an
/// *acceptable formatting difference*.
#[test]
fn render_tree_parity_links_images() {
    assert_parity(
        "links_images",
        &["Example Link", "image reference"],
        &["https://example.com", "image.png", "descriptive alt"],
    );

    // URL parity on the terminal surface is checked against the *raw* output:
    // in OSC8 mode the legacy renderer carries the URL inside the (otherwise
    // stripped) hyperlink escape sequence. See `assert_urls_present`.
    let markdown = fixture("links_images");
    let legacy_term = legacy_terminal(markdown);
    let tree_term = tree_terminal("links_images", markdown);
    assert_urls_present(
        "links_images",
        "terminal",
        &legacy_term,
        &tree_term,
        &["https://example.com"],
    );
}

#[test]
fn render_tree_parity_lists() {
    assert_parity(
        "lists",
        &["alpha bullet", "beta bullet", "one ordered", "two ordered"],
        &[],
    );
}

#[test]
fn render_tree_parity_task_list() {
    assert_parity(
        "task_list",
        &["completed task entry", "pending task entry"],
        &[],
    );
}

#[test]
fn render_tree_parity_code_block() {
    // Code content is preserved verbatim; only highlight spans differ.
    assert_parity(
        "code_block",
        &["fn", "parity_demo", "println", "render tree"],
        &[],
    );
}

#[test]
fn render_tree_parity_table() {
    assert_parity(
        "table",
        &["Fruit", "Quantity", "apples", "pears", "12"],
        &[],
    );

    // Structural-HTML invariant for GFM tables: both pipelines must emit a
    // `<table>` element and at least one `<tr>` and `<td>`. Token-stripping
    // would let pipe-table syntax pass through as paragraph text undetected;
    // the per-tag assertions below pin that the parser-option contract
    // (DMTR-2) is honored by both pipelines — see
    // `renderable/features/2026-05-20-darkmatter-tree/parser-options.md`.
    let markdown = fixture("table");
    let legacy_html_raw = legacy_html(markdown);
    let tree_html_raw = tree_html("table", markdown);
    for tag in &["<table", "<tr", "<td"] {
        assert!(
            legacy_html_raw.contains(tag),
            "[table/HTML] legacy renderer must emit a structural {tag} element; \
             without `ENABLE_TABLES` the parser drops table events and only the \
             cell text leaks through as paragraph text",
        );
        assert!(
            tree_html_raw.contains(tag),
            "[table/HTML] render-tree renderer must emit a structural {tag} element",
        );
    }
}

#[test]
fn render_tree_parity_blockquote() {
    assert_parity(
        "blockquote",
        &["A quoted line of prose", "A second quoted paragraph"],
        &[],
    );
}

/// Raw HTML parity is asymmetric — see the module docs (*Raw HTML* buckets).
///
/// On the **browser** surface both pipelines preserve raw block and inline
/// HTML content (the tree side runs under [`RawHtmlPolicy::Allow`]), so full
/// parity is asserted there.
///
/// On the **terminal** surface the *legacy* renderer silently drops raw block
/// HTML — `raw block content` never reaches its output. That is a documented
/// legacy-renderer gap, not a render-tree regression: this test asserts only
/// the terminal text the legacy renderer *does* retain, and separately
/// confirms the render-tree pipeline keeps the content the legacy renderer
/// loses.
#[test]
fn render_tree_parity_raw_html() {
    let markdown = fixture("raw_html");

    // Browser surface: both pipelines preserve the raw HTML content.
    let legacy_html_plain = strip_html(&legacy_html(markdown));
    let tree_html_plain = strip_html(&tree_html("raw_html", markdown));
    assert_tokens_present(
        "raw_html",
        "HTML",
        &legacy_html_plain,
        &tree_html_plain,
        &["raw block content", "inline html", "Trailing paragraph"],
    );

    // Terminal surface: assert only the text the legacy renderer retains.
    let legacy_term = strip_ansi(&legacy_terminal(markdown));
    let tree_term = strip_ansi(&tree_terminal("raw_html", markdown));
    assert_tokens_present(
        "raw_html",
        "terminal",
        &legacy_term,
        &tree_term,
        // `inline html` is the inline-HTML text the legacy renderer keeps
        // (tags stripped); the surrounding paragraphs are retained too.
        &[
            "A paragraph then a block",
            "inline html",
            "Trailing paragraph",
        ],
    );

    // The legacy terminal renderer's documented gap: it drops the raw block.
    assert!(
        !normalize(&legacy_term).contains("raw block content"),
        "legacy terminal renderer is expected to drop raw block HTML; if this \
         now passes the legacy gap is fixed and the classification can change",
    );
    // The render-tree terminal renderer is the more faithful pipeline.
    assert!(
        normalize(&tree_term).contains("raw block content"),
        "render-tree terminal renderer must preserve raw block HTML content",
    );
}

/// Sanity check: every fixture folds and renders through both pipelines
/// without panicking, even ones not given a dedicated token assertion.
#[test]
fn render_tree_parity_all_fixtures_render() {
    for (name, markdown) in FIXTURES {
        let _ = legacy_html(markdown);
        let _ = tree_html(name, markdown);
        let _ = legacy_terminal(markdown);
        let _ = tree_terminal(name, markdown);
    }
}

// ---------------------------------------------------------------------------
// Darkmatter-inline parity (DMTR-3): mark / dim / HR attributes
//
// These fixtures exercise the span-aware fold, which is the path the
// experimental `to_render_document` / `render_tree_html` / `render_tree_terminal`
// entry points use. The legacy renderers (`as_html`, `for_terminal`) already
// understand the same constructs via the non-spanned `InlineStyleProcessor`
// and `RuleProcessor`, so visible-text parity is the right invariant here.
//
// Closes review-2 finding 3 in
// `renderable/features/2026-05-20-darkmatter-tree/review-2.md`.
// ---------------------------------------------------------------------------

/// Visible-text parity for `==highlighted==`: both pipelines preserve the
/// marked word on both HTML and terminal surfaces.
#[test]
fn render_tree_parity_mark_spanned() {
    let name = "mark";
    let markdown = fixture(name);
    let tokens = &["Plain text", "highlighted phrase", "inside it"];

    let legacy_html_raw = legacy_html(markdown);
    let tree_html_raw = tree_html_spanned(name, markdown);
    assert_tokens_present(
        name,
        "HTML",
        &strip_html(&legacy_html_raw),
        &strip_html(&tree_html_raw),
        tokens,
    );

    let legacy_term = legacy_terminal(markdown);
    let tree_term = tree_terminal_spanned(name, markdown);
    assert_tokens_present(
        name,
        "terminal",
        &strip_ansi(&legacy_term),
        &strip_ansi(&tree_term),
        tokens,
    );
}

/// Visible-text parity for `⌄dimmed⌄`: both pipelines preserve the dim phrase
/// on both HTML and terminal surfaces.
#[test]
fn render_tree_parity_dim_spanned() {
    let name = "dim";
    let markdown = fixture(name);
    let tokens = &["Plain text", "dimmed phrase", "inside it"];

    let legacy_html_raw = legacy_html(markdown);
    let tree_html_raw = tree_html_spanned(name, markdown);
    assert_tokens_present(
        name,
        "HTML",
        &strip_html(&legacy_html_raw),
        &strip_html(&tree_html_raw),
        tokens,
    );

    let legacy_term = legacy_terminal(markdown);
    let tree_term = tree_terminal_spanned(name, markdown);
    assert_tokens_present(
        name,
        "terminal",
        &strip_ansi(&legacy_term),
        &strip_ansi(&tree_term),
        tokens,
    );
}

/// HR-attribute parity: both pipelines render the styled rule as
/// non-literal markup (legacy renders `--- { style: waves }` as a styled
/// SVG; the tree renderer emits an `<hr>` carrying the HR-attribute hints).
/// What both pipelines must *not* do is leak the raw markdown source —
/// `--- { style: waves }` — through as visible paragraph text. The
/// surrounding paragraphs must also survive on both HTML and terminal
/// surfaces. Exact HR styling (palette, glyph) is an *acceptable formatting
/// difference* and is checked separately at Level 2.
#[test]
fn render_tree_parity_hr_attributes_spanned() {
    let name = "hr_attributes";
    let markdown = fixture(name);
    let tokens = &["Lead paragraph", "Trailing paragraph"];

    // Browser surface: the surrounding paragraphs survive, and the raw HR
    // markdown source must not appear as visible text in either pipeline.
    let legacy_html_raw = legacy_html(markdown);
    let tree_html_raw = tree_html_spanned(name, markdown);
    let legacy_html_plain = strip_html(&legacy_html_raw);
    let tree_html_plain = strip_html(&tree_html_raw);
    assert!(
        !normalize(&legacy_html_plain).contains("style: waves"),
        "[hr_attributes/HTML] legacy renderer leaked the raw HR markdown as text; \
         raw output:\n{legacy_html_raw}",
    );
    assert!(
        !normalize(&tree_html_plain).contains("style: waves"),
        "[hr_attributes/HTML] render-tree renderer leaked the raw HR markdown as text; \
         raw output:\n{tree_html_raw}",
    );
    assert_tokens_present(name, "HTML", &legacy_html_plain, &tree_html_plain, tokens);

    // Terminal surface: the surrounding paragraphs survive both pipelines,
    // and again the raw HR source must not leak through as visible text.
    let legacy_term = legacy_terminal(markdown);
    let tree_term = tree_terminal_spanned(name, markdown);
    let legacy_term_plain = strip_ansi(&legacy_term);
    let tree_term_plain = strip_ansi(&tree_term);
    assert!(
        !normalize(&legacy_term_plain).contains("style: waves"),
        "[hr_attributes/terminal] legacy renderer leaked the raw HR markdown as text",
    );
    assert!(
        !normalize(&tree_term_plain).contains("style: waves"),
        "[hr_attributes/terminal] render-tree renderer leaked the raw HR markdown as text",
    );
    assert_tokens_present(
        name,
        "terminal",
        &legacy_term_plain,
        &tree_term_plain,
        tokens,
    );
}

// ---------------------------------------------------------------------------
// HR attributes inside a list item (DMTR-5, review-11 finding 3).
//
// The span-aware design (`span-aware-processor-design.md` §"HR Attributes:
// List Item") requires the tree fold to match the legacy `RuleProcessor`, and
// mandates an explicit test rather than inferring the desired shape.
//
// **Established behavior (ledger):** the legacy `RuleProcessor` does NOT
// transform an HR-attribute paragraph inside a list item — a tight list item
// emits no `Paragraph` wrapper, so the HR-rewrite (which only fires for a
// simple buffered paragraph) never triggers. See the unit test
// `rule_processor::tests::test_horizontal_rule_inside_list_item_is_not_transformed`.
// The span-aware fold mirrors this: its `BlockExtensionProcessor` only
// rewrites when a `Start(Paragraph)` opens a buffered region, so
// `- --- { style: waves }` stays list-item text on the tree path too.
// Classified as *equivalent behavior*, not a divergence.
// ---------------------------------------------------------------------------

/// `- --- { style: waves }` must NOT fold to a `ThematicBreak`; it stays a
/// list whose item carries the literal text — matching the legacy
/// `RuleProcessor`. Both pipelines therefore surface the raw HR source as
/// visible list text (the opposite of the top-level `hr_attributes` fixture,
/// where both pipelines DO transform it).
#[test]
fn render_tree_parity_hr_attributes_in_list_item() {
    let name = "hr_attributes_in_list";
    let markdown = fixture(name);

    // Fold (Level 1): the span-aware fold — the production entry-point path —
    // must leave the HR-attribute text inside the list, not promote it to a
    // ThematicBreak.
    let doc = fold_spanned(name, markdown);
    assert!(
        find_node(&doc.root, |n| matches!(n.kind, NodeKind::ThematicBreak)).is_none(),
        "HR-attribute text inside a list item must NOT fold to a ThematicBreak \
         (must match the legacy RuleProcessor); tree:\n{:#?}",
        doc.root,
    );
    assert!(
        find_node(&doc.root, |n| matches!(n.kind, NodeKind::List { .. })).is_some(),
        "fixture must fold to a List node; tree:\n{:#?}",
        doc.root,
    );

    // Render parity: because neither pipeline transforms the rule, the raw HR
    // source survives as visible list text on both surfaces. This is the
    // matching-legacy invariant for this fixture.
    let legacy_html_plain = strip_html(&legacy_html(markdown));
    let tree_html_plain = strip_html(&tree_html_spanned(name, markdown));
    assert_tokens_present(
        name,
        "HTML",
        &legacy_html_plain,
        &tree_html_plain,
        &["style: waves"],
    );

    let legacy_term_plain = strip_ansi(&legacy_terminal(markdown));
    let tree_term_plain = strip_ansi(&tree_terminal_spanned(name, markdown));
    assert_tokens_present(
        name,
        "terminal",
        &legacy_term_plain,
        &tree_term_plain,
        &["style: waves"],
    );
}

// ---------------------------------------------------------------------------
// Rich code-block parity (DMTR-5): syntax highlighting + full info-string
// metadata parity.
//
// The render-tree entry points wire darkmatter's `TerminalCodeRenderer` for
// both terminal (review-10 finding 2) and browser/HTML (review-11 finding 2),
// and the `CodeRenderer` trait now carries the info-string `meta` (the text
// after the language token, e.g. `title="…" line-numbering=true highlight=2`).
// The renderer re-parses that directive via `parse_code_info`, so title, line
// numbering, and highlighted-line behavior reach **both** pipelines exactly as
// the legacy renderer produces them — closing the previously-accepted ledger
// divergence. The `tree_terminal` / `tree_html` helpers use the same wiring,
// so this test exercises the production-shaped path.
// ---------------------------------------------------------------------------

/// Code body, syntax highlighting, **and** the info-string metadata (title /
/// line numbers / highlight) reach both pipelines on both surfaces.
#[test]
fn render_tree_parity_code_block_rich() {
    let name = "code_block_rich";
    let markdown = fixture(name);
    let body_tokens = &["fn", "parity_demo", "println", "render tree"];

    // HTML: code body survives on both surfaces.
    let legacy_html_raw = legacy_html(markdown);
    let tree_html_raw = tree_html(name, markdown);
    assert_tokens_present(
        name,
        "HTML",
        &strip_html(&legacy_html_raw),
        &strip_html(&tree_html_raw),
        body_tokens,
    );

    // HTML: the info-string title, line-number table, and highlighted-line
    // markup now reach the tree path as well as the legacy renderer.
    for (surface, html) in [("legacy", &legacy_html_raw), ("tree", &tree_html_raw)] {
        assert!(
            html.contains("Demo Snippet"),
            "[{surface}] HTML must surface the code-block title",
        );
        assert!(
            html.contains("code-table") && html.contains("ln-gutter"),
            "[{surface}] HTML must render the line-number table",
        );
        assert!(
            html.contains("highlighted"),
            "[{surface}] HTML must mark the highlighted line",
        );
    }

    // Terminal: code body survives on both surfaces, and both pipelines
    // syntax-highlight (the tree side via the wired `TerminalCodeRenderer`).
    let legacy_term = legacy_terminal(markdown);
    let tree_term = tree_terminal(name, markdown);
    assert_tokens_present(
        name,
        "terminal",
        &strip_ansi(&legacy_term),
        &strip_ansi(&tree_term),
        body_tokens,
    );
    assert!(
        tree_term.contains('\u{1b}'),
        "tree terminal code block must be syntax-highlighted (ANSI escapes present); \
         if this fails the code renderer is not wired into the tree terminal path",
    );

    // Terminal: the info-string title now surfaces in the header row on both
    // pipelines (previously an accepted tree-path divergence).
    for (surface, term) in [("legacy", &legacy_term), ("tree", &tree_term)] {
        assert!(
            strip_ansi(term).contains("Demo Snippet"),
            "[{surface}] terminal renderer must surface the code-block title",
        );
    }

    // The info-string metadata is preserved on the folded node and re-parsed
    // by the renderer; this pins that the fold still carries it.
    let doc = fold(name, markdown);
    let code = find_node(&doc.root, |n| matches!(n.kind, NodeKind::Code { .. }))
        .expect("fixture must fold to a Code node");
    match &code.kind {
        NodeKind::Code { lang, meta, .. } => {
            assert_eq!(lang.as_deref(), Some("rust"));
            let meta = meta.as_deref().unwrap_or_default();
            assert!(
                meta.contains("title=") && meta.contains("highlight="),
                "code info-string metadata must survive on NodeKind::Code.meta; got {meta:?}",
            );
        }
        other => panic!("expected Code node, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Image alt / title / width parity (DMTR-5).
//
// `alt` and a plain `title` reach both pipelines on the browser surface.
//
// **Accepted divergence (ledger):** the tree fold captures only the image
// `url` and `title` (`Tag::Image { dest_url, title }`); it does not parse
// darkmatter's structured-title `width` extension, so a `width` attribute is
// emitted by the legacy renderer but not by the tree path. Recorded as an
// accepted experimental-path divergence.
// ---------------------------------------------------------------------------

/// Image `alt`, `src`, and plain `title` reach both browser pipelines.
#[test]
fn render_tree_parity_image_titled() {
    let name = "image_titled";
    let markdown = fixture(name);

    let legacy_html_raw = legacy_html(markdown);
    let tree_html_raw = tree_html(name, markdown);

    // Visible prose survives on both surfaces.
    assert_tokens_present(
        name,
        "HTML",
        &strip_html(&legacy_html_raw),
        &strip_html(&tree_html_raw),
        &["An image", "inline"],
    );
    // alt / src / title live inside attributes — checked against raw HTML.
    assert_urls_present(
        name,
        "HTML",
        &legacy_html_raw,
        &tree_html_raw,
        &["photo.png", "descriptive alt", "Hover Title"],
    );
    // Both renderers emit a `title=` attribute for the plain title.
    assert!(
        legacy_html_raw.contains("title=\"Hover Title\""),
        "legacy HTML must emit the image title attribute; raw:\n{legacy_html_raw}",
    );
    assert!(
        tree_html_raw.contains("title=\"Hover Title\""),
        "tree HTML must emit the image title attribute; raw:\n{tree_html_raw}",
    );
}

// ---------------------------------------------------------------------------
// Parser-option-sensitive parity (DMTR-2 / DMTR-5): footnotes, superscript,
// subscript.
//
// These constructs are **intentionally widened** on the tree path:
// `render_tree_parser_options()` enables `ENABLE_FOOTNOTES`,
// `ENABLE_SUPERSCRIPT`, and `ENABLE_SUBSCRIPT`, while the legacy renderers
// parse with only `ENABLE_TABLES | ENABLE_STRIKETHROUGH`. Parity here is
// therefore a **documented divergence**, not equivalence — see
// `parser-options.md`:
//
// 1. Legacy ignores or mangles the syntax (plain text, or strikethrough for
//    a single-tilde subscript).
// 2. The tree path folds to structural nodes
//    (`FootnoteReference`/`FootnoteDefinition`, `Span` class `sup`/`sub`).
// 3. The divergence is classified here rather than treated as a failure.
// ---------------------------------------------------------------------------

/// Footnotes: legacy emits the literal `[^note]` marker (footnotes disabled);
/// the tree path folds to `FootnoteReference` / `FootnoteDefinition` and the
/// browser renderer lowers them to anchor + definition markup.
#[test]
fn render_tree_parity_footnote_divergence() {
    let name = "footnote";
    let markdown = fixture(name);

    // Tree side: structural footnote nodes exist after the fold.
    let doc = fold(name, markdown);
    assert!(
        find_node(&doc.root, |n| matches!(
            n.kind,
            NodeKind::FootnoteReference { .. }
        ))
        .is_some(),
        "tree fold must produce a FootnoteReference node (ENABLE_FOOTNOTES)",
    );
    assert!(
        find_node(&doc.root, |n| matches!(
            n.kind,
            NodeKind::FootnoteDefinition { .. }
        ))
        .is_some(),
        "tree fold must produce a FootnoteDefinition node (ENABLE_FOOTNOTES)",
    );

    // Browser surface: the tree lowers the reference to an `#fn-` anchor and
    // the definition to a `footnote-definition` block.
    let tree_html_raw = tree_html(name, markdown);
    assert!(
        tree_html_raw.contains("#fn-note"),
        "tree HTML must link the footnote reference to its definition; raw:\n{tree_html_raw}",
    );
    assert!(
        tree_html_raw.contains("footnote-definition"),
        "tree HTML must mark the footnote definition block; raw:\n{tree_html_raw}",
    );

    // Legacy side: footnotes are disabled, so the literal marker leaks through
    // as plain text and no footnote anchor markup is produced.
    let legacy_html_raw = legacy_html(markdown);
    assert!(
        strip_html(&legacy_html_raw).contains("[^note]"),
        "legacy renderer (footnotes disabled) must emit the literal [^note] marker; \
         raw:\n{legacy_html_raw}",
    );
    assert!(
        !legacy_html_raw.contains("#fn-note"),
        "legacy renderer must NOT produce footnote anchor markup; raw:\n{legacy_html_raw}",
    );
}

/// Superscript: legacy emits the literal `^2nd^` (superscript disabled); the
/// tree path folds `^2nd^` to a `Span` class `sup`, which the browser renderer
/// lowers to `<span class="sup">` and MarkdownPlus preserves verbatim.
#[test]
fn render_tree_parity_superscript_divergence() {
    let name = "superscript";
    let markdown = fixture(name);

    // Tree side: a `sup`-class span exists after the fold.
    let doc = fold(name, markdown);
    assert!(
        find_node(&doc.root, |n| span_has_class(n, "sup")).is_some(),
        "tree fold must produce a Span with class \"sup\" (ENABLE_SUPERSCRIPT)",
    );

    // Browser surface: the class survives.
    let tree_html_raw = tree_html(name, markdown);
    assert!(
        tree_html_raw.contains("class=\"sup\""),
        "tree HTML must emit a sup-class span; raw:\n{tree_html_raw}",
    );

    // MarkdownPlus surface: the class survives as inline HTML.
    let tree_md_plus = tree_markdown_plus(name, markdown);
    assert!(
        tree_md_plus.contains("class=\"sup\""),
        "tree MarkdownPlus must preserve the sup-class span; output:\n{tree_md_plus}",
    );

    // Legacy side: superscript disabled, so the literal `^2nd^` leaks through
    // and no `<sup>` / sup-class markup is produced.
    let legacy_html_raw = legacy_html(markdown);
    assert!(
        strip_html(&legacy_html_raw).contains("^2nd^"),
        "legacy renderer (superscript disabled) must emit the literal ^2nd^; \
         raw:\n{legacy_html_raw}",
    );
    assert!(
        !legacy_html_raw.contains("class=\"sup\"") && !legacy_html_raw.contains("<sup>"),
        "legacy renderer must NOT produce superscript markup; raw:\n{legacy_html_raw}",
    );
}

/// Subscript: a single-tilde `~note~` is the parser-option-sensitive case the
/// design calls out. With `ENABLE_SUBSCRIPT` the tree path folds it to a
/// `Span` class `sub`; the legacy renderer (subscript disabled, strikethrough
/// enabled) treats the single-tilde pair as **strikethrough** instead. The
/// inner `note` survives on both surfaces; the structural treatment diverges.
#[test]
fn render_tree_parity_subscript_divergence() {
    let name = "subscript";
    let markdown = fixture(name);

    // Tree side: a `sub`-class span exists after the fold.
    let doc = fold(name, markdown);
    assert!(
        find_node(&doc.root, |n| span_has_class(n, "sub")).is_some(),
        "tree fold must produce a Span with class \"sub\" (ENABLE_SUBSCRIPT)",
    );

    // Browser surface: the class survives and the inner text is preserved.
    let tree_html_raw = tree_html(name, markdown);
    assert!(
        tree_html_raw.contains("class=\"sub\""),
        "tree HTML must emit a sub-class span; raw:\n{tree_html_raw}",
    );
    assert!(
        strip_html(&tree_html_raw).contains("note"),
        "tree HTML must preserve the subscript content; raw:\n{tree_html_raw}",
    );

    // Legacy side: subscript disabled. The single-tilde pair is consumed by
    // strikethrough (`<del>`), NOT subscript — the documented delimiter
    // divergence. The inner `note` still survives.
    let legacy_html_raw = legacy_html(markdown);
    assert!(
        strip_html(&legacy_html_raw).contains("note"),
        "legacy renderer must preserve the subscript content; raw:\n{legacy_html_raw}",
    );
    assert!(
        !legacy_html_raw.contains("class=\"sub\"") && !legacy_html_raw.contains("<sub>"),
        "legacy renderer (subscript disabled) must NOT produce subscript markup; \
         raw:\n{legacy_html_raw}",
    );
}

// ---------------------------------------------------------------------------
// MarkdownPlus target parity (DMTR-5 / DMTR-8 cutover order item 2).
//
// MarkdownPlus is the Markdown renderer's richer dialect: span classes the
// portable Markdown dialect would drop survive as inline HTML. These tests pin
// that the tree MarkdownPlus path preserves darkmatter-inline structure that
// portable Markdown cannot, so the cutover-order item-2 target has explicit
// coverage rather than only smoke tests.
// ---------------------------------------------------------------------------

/// MarkdownPlus preserves the `mark`-class span (via the span-aware fold) as
/// inline HTML, where portable Markdown would drop the class.
#[test]
fn render_tree_markdown_plus_preserves_mark_span() {
    let name = "mark";
    let markdown = fixture(name);

    let plus = tree_markdown_plus_spanned(name, markdown);
    assert!(
        plus.contains("highlighted phrase"),
        "MarkdownPlus must preserve the mark text; output:\n{plus}",
    );
    assert!(
        plus.contains("class=\"mark\""),
        "MarkdownPlus must preserve the mark-class span as inline HTML; output:\n{plus}",
    );

    // Portable Markdown drops the class but keeps the visible text.
    let portable = tree_markdown_portable_spanned(name, markdown);
    assert!(
        portable.contains("highlighted phrase"),
        "portable Markdown must keep the visible mark text; output:\n{portable}",
    );
    assert!(
        !portable.contains("class=\"mark\""),
        "portable Markdown has no span-class equivalent; output:\n{portable}",
    );
}

// ---------------------------------------------------------------------------
// Mermaid mode parity (DMTR-5): the only deterministic mode.
//
// `MermaidMode::Off` (the default for both `TerminalOptions` and `HtmlOptions`)
// renders a `mermaid` fenced block as an ordinary syntax-highlighted code
// block — no diagram rendering — so the mermaid *source* survives on both
// pipelines. This is the deterministic mode the spec asks parity to cover.
//
// **Accepted divergence (ledger):** `MermaidMode::Text` and
// `MermaidMode::Image` are NOT covered here. `Image` shells out to an external
// renderer (mermaid.ink / mmdc) and emits terminal graphics — non-deterministic
// and environment-dependent — and the experimental tree path has no Mermaid
// adapter at all (see `entry-point-shape.md` deferred list and
// `baselines.md`). Those modes are deferred until the tree renderer gains a
// Mermaid adapter.
// ---------------------------------------------------------------------------

/// In the default `MermaidMode::Off`, a `mermaid` block renders as a plain
/// highlighted code block on both pipelines, so the diagram source survives.
#[test]
fn render_tree_parity_mermaid_off_mode() {
    // `TerminalOptions::default()` / `HtmlOptions::default()` both use
    // `MermaidMode::Off`, which the parity drivers use.
    assert_parity("mermaid_off", &["graph TD", "A", "B"], &[]);
}

// ---------------------------------------------------------------------------
// Shared helpers for the structural-node parity tests above.
// ---------------------------------------------------------------------------

/// Returns the first node in the subtree (depth-first, root included) for
/// which `pred` holds.
fn find_node(node: &RenderNode, pred: impl Fn(&RenderNode) -> bool + Copy) -> Option<&RenderNode> {
    if pred(node) {
        return Some(node);
    }
    node.children()
        .iter()
        .find_map(|child| find_node(child, pred))
}

/// Returns `true` when `node` is a `Span` carrying `class`.
fn span_has_class(node: &RenderNode, class: &str) -> bool {
    matches!(node.kind, NodeKind::Span { .. }) && node.attrs.classes.iter().any(|c| c == class)
}

/// Renders fixture text to MarkdownPlus via the render-tree Markdown renderer
/// (plain fold).
fn tree_markdown_plus(name: &str, markdown: &str) -> String {
    render_markdown(fold(name, markdown), MarkdownDialect::MarkdownPlus)
}

/// Renders fixture text to MarkdownPlus via the **span-aware** fold.
fn tree_markdown_plus_spanned(name: &str, markdown: &str) -> String {
    render_markdown(fold_spanned(name, markdown), MarkdownDialect::MarkdownPlus)
}

/// Renders fixture text to portable Markdown via the **span-aware** fold.
fn tree_markdown_portable_spanned(name: &str, markdown: &str) -> String {
    render_markdown(fold_spanned(name, markdown), MarkdownDialect::Markdown)
}

/// Shared Markdown render path for the dialect helpers above.
fn render_markdown(doc: Document, dialect: MarkdownDialect) -> String {
    let opts = MarkdownRenderOptions {
        dialect,
        strictness: RenderStrictness::Warn,
        style: None,
    };
    render_markdown_document(&doc, &opts)
        .expect("tree markdown render must succeed")
        .output
}
