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

use biscuit_terminal::render_tree::{render_terminal_document, TerminalRenderOptions};
use darkmatter::markdown::output::{HtmlOptions, TerminalOptions, as_html, for_terminal};
use darkmatter::markdown::render_tree::fold_markdown_to_document;
use darkmatter::markdown::Markdown;
use renderable::tree::{
    render_browser_document, BrowserRenderOptions, Document, RawHtmlPolicy, SourceDescriptor,
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
                        if bytes[idx] == '\u{1b}'
                            && idx + 1 < bytes.len()
                            && bytes[idx + 1] == '\\'
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

/// Folds parity fixture `name` to a render-tree [`Document`].
fn fold(name: &str, markdown: &str) -> Document {
    let source = SourceDescriptor::Virtual { name: name.into() };
    let (doc, _diags) = fold_markdown_to_document(source, markdown);
    doc
}

/// Renders fixture text to HTML via the legacy `as_html` renderer.
fn legacy_html(markdown: &str) -> String {
    let md: Markdown = markdown.into();
    as_html(&md, HtmlOptions::default()).expect("legacy as_html must succeed")
}

/// Renders fixture text to HTML via the render-tree browser renderer.
///
/// Uses [`RawHtmlPolicy::Allow`] so raw-HTML fixtures compare against the
/// legacy renderer (which passes raw HTML through) on equal footing.
fn tree_html(name: &str, markdown: &str) -> String {
    let doc = fold(name, markdown);
    let opts = BrowserRenderOptions {
        raw_html: RawHtmlPolicy::Allow,
        ..BrowserRenderOptions::default()
    };
    render_browser_document(&doc, &opts)
        .expect("tree browser render must succeed")
        .output
        .render()
}

/// Renders fixture text to a terminal string via the legacy `for_terminal`.
fn legacy_terminal(markdown: &str) -> String {
    let md: Markdown = markdown.into();
    for_terminal(&md, TerminalOptions::default()).expect("legacy for_terminal must succeed")
}

/// Renders fixture text to a terminal string via the render-tree renderer.
fn tree_terminal(name: &str, markdown: &str) -> String {
    let doc = fold(name, markdown);
    render_terminal_document(&doc, &TerminalRenderOptions::default())
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
/// inside `href`/`src` attributes that tag-stripping would discard.
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
    assert_parity("headings", &["Top Level", "Second Level", "Third Level"], &[]);
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
/// terminal renderers surface it (legacy as `text [url]`, the tree renderer
/// as `[text](url)`). The image `src` URL is *not* checked on the terminal —
/// neither terminal renderer emits an image's `src`; both show only the alt
/// text. That symmetric omission is an *acceptable formatting difference*.
#[test]
fn render_tree_parity_links_images() {
    assert_parity(
        "links_images",
        &["Example Link", "image reference"],
        &["https://example.com", "image.png", "descriptive alt"],
    );

    let markdown = fixture("links_images");
    let legacy_term = strip_ansi(&legacy_terminal(markdown));
    let tree_term = strip_ansi(&tree_terminal("links_images", markdown));
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
        &["A paragraph then a block", "inline html", "Trailing paragraph"],
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
