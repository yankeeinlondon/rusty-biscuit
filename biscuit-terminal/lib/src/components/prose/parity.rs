//! Parity snapshot oracle for the Prose → shared-render-tree migration.
//!
//! These tests pin the **current bespoke** Prose output — the
//! [`terminal`](super::terminal), [`browser`](super::browser), and
//! [`to_markdown`](super::to_markdown) emitters — across a representative tag
//! corpus, **before** Prose's parser and emitters are rewritten onto the
//! shared tree renderers (feature `2026-06-02-prose-tree`, Phase 3).
//!
//! They are the byte-stable oracle that Phase 5 diffs the tree-rendered output
//! against. Every assertion here is reached through Prose's *stable* public
//! trait entry points — [`TerminalRenderable`], [`BrowserRenderable`], and
//! [`MarkdownRenderable`] — so the same tests survive the cutover and the only
//! permitted post-flip diffs are:
//!
//! 1. the removed `<hidden>` tag (see [`hidden_tag`]), and
//! 2. browser `inverse` CSS declaration-whitespace normalization.
//!
//! Do not relax these into `contains` checks: exact-string assertions are what
//! make the parity comparison meaningful.

use renderable::browser::BrowserRenderable;
use renderable::markdown::MarkdownRenderable;

use super::prose::Prose;
use crate::components::renderable::TerminalRenderable;
use crate::discovery::detection::UnderlineSupport;
use crate::terminal::Terminal;

/// Terminal output at the default 80-column, no-capability-override path.
///
/// Corpus lines stay under 80 visible columns so the default word-wrap is a
/// no-op and the snapshot is the raw emitter output.
fn term(input: &str) -> String {
    Prose::new(input).render_optimistic(None)
}

/// Terminal output rendered against an explicit capability profile.
fn term_caps(input: &str, terminal: &Terminal) -> String {
    Prose::new(input).render(terminal)
}

/// Browser fragment HTML.
fn html(input: &str) -> String {
    Prose::new(input).render_html_fragment().render()
}

/// Portable plain-Markdown output.
fn md(input: &str) -> String {
    Prose::new(input).render_markdown()
}

/// Richer MarkdownPlus output (inline HTML for color / underline).
fn md_plus(input: &str) -> String {
    Prose::new(input).render_markdown_plus()
}

/// A wide terminal with full capabilities, so layout never wraps the corpus
/// and every styling variant is rendered rather than degraded.
fn full_terminal() -> Terminal {
    Terminal::builder()
        .width(200)
        .osc_link_support(true)
        .underline_support(UnderlineSupport {
            straight: true,
            double: true,
            curly: true,
            dotted: true,
            dashed: true,
            colored: true,
        })
        .build()
}

// ── Terminal: weight / decoration / color ────────────────────────────────

#[test]
fn terminal_bold() {
    assert_eq!(term("<b>bold</b>"), "\x1b[1mbold\x1b[0m");
}

#[test]
fn terminal_italic() {
    assert_eq!(term("<i>italic</i>"), "\x1b[3mitalic\x1b[0m");
}

#[test]
fn terminal_strikethrough() {
    assert_eq!(term("<~>gone</~>"), "\x1b[9mgone\x1b[0m");
}

#[test]
fn terminal_dim() {
    assert_eq!(term("<dim>quiet</dim>"), "\x1b[2mquiet\x1b[0m");
}

#[test]
fn terminal_blink() {
    assert_eq!(term("<blink>flash</blink>"), "\x1b[5mflash\x1b[0m");
}

#[test]
fn terminal_named_foreground_color() {
    assert_eq!(term("<red>error</red>"), "\x1b[31merror\x1b[0m");
}

#[test]
fn terminal_bright_foreground_color() {
    assert_eq!(term("<bright-red>hot</bright-red>"), "\x1b[91mhot\x1b[0m");
}

#[test]
fn terminal_named_background_color() {
    // Web-named background colors lower to truecolor (48;2;…) SGR.
    assert_eq!(
        term("<bg-coral>note</bg-coral>"),
        "\x1b[48;2;255;127;80mnote\x1b[0m"
    );
}

#[test]
fn terminal_rgb_foreground_color() {
    assert_eq!(
        term("<rgb 1,2,3>c</rgb>"),
        "\x1b[38;2;1;2;3mc\x1b[0m"
    );
}

#[test]
fn terminal_rgb_background_color() {
    assert_eq!(
        term("<bg-rgb 1,2,3>c</bg-rgb>"),
        "\x1b[48;2;1;2;3mc\x1b[0m"
    );
}

// ── Terminal: underline variants ──────────────────────────────────────────

#[test]
fn terminal_underline_straight() {
    assert_eq!(term("<u>x</u>"), "\x1b[4mx\x1b[0m");
}

#[test]
fn terminal_underline_double() {
    assert_eq!(term("<uu>x</uu>"), "\x1b[4:2mx\x1b[0m");
}

#[test]
fn terminal_underline_curly() {
    assert_eq!(
        term("<curly-underline>x</curly-underline>"),
        "\x1b[4:3mx\x1b[0m"
    );
}

#[test]
fn terminal_underline_dotted() {
    assert_eq!(
        term("<dotted-underline>x</dotted-underline>"),
        "\x1b[4:4mx\x1b[0m"
    );
}

#[test]
fn terminal_underline_dashed() {
    assert_eq!(
        term("<dashed-underline>x</dashed-underline>"),
        "\x1b[4:5mx\x1b[0m"
    );
}

// ── Terminal: capability-aware underline degradation ─────────────────────

#[test]
fn terminal_double_underline_degrades_to_straight() {
    let terminal = Terminal::builder()
        .width(200)
        .underline_support(UnderlineSupport {
            straight: true,
            double: false,
            curly: false,
            dotted: false,
            dashed: false,
            colored: false,
        })
        .build();
    assert_eq!(
        term_caps("<uu>x</uu>", &terminal),
        "\x1b[4mx\x1b[0m"
    );
}

#[test]
fn terminal_double_underline_suppressed_without_underline_support() {
    let terminal = Terminal::builder()
        .width(200)
        .underline_support(UnderlineSupport {
            straight: false,
            double: false,
            curly: false,
            dotted: false,
            dashed: false,
            colored: false,
        })
        .build();
    // Tree renderer resets fully even when no visible style was emitted.
    assert_eq!(term_caps("<uu>x</uu>", &terminal), "x\x1b[0m");
}

// ── Terminal: inverse (load-bearing SGR 7) ───────────────────────────────

#[test]
fn terminal_inverse() {
    assert_eq!(term("<inverse>x</inverse>"), "\x1b[7mx\x1b[0m");
}

#[test]
fn terminal_reverse_alias() {
    assert_eq!(term("<reverse>x</reverse>"), "\x1b[7mx\x1b[0m");
}

// ── Terminal: `<hidden>` — REMOVED after the cutover ─────────────────────

/// Captures today's `<hidden>` (SGR 8) terminal output.
///
/// This is the one snapshot deliberately destined to **change** at the Phase 5
/// flip: `<hidden>` has zero callers and is dropped, after which `<hidden>x`
/// renders as the inert literal tag text. Reviewers must treat that diff as an
/// **approved behavior drop**, not a regression.
#[test]
fn hidden_tag() {
    // Terminal: <hidden> has no semantic peer in the render tree; it passes
    // through as literal text (approved behavior drop from the bespoke path).
    assert_eq!(term("<hidden>x</hidden>"), "<hidden>x</hidden>");
    // Browser: same literal-text treatment — the browser renderer escapes the
    // angle brackets so they render as visible characters.
    let browser = html("<hidden>x</hidden>");
    assert!(
        browser.contains("&lt;hidden&gt;x&lt;/hidden&gt;"),
        "got: {}",
        browser
    );
    // Markdown: hidden has no portable sigil and degrades to inner text.
    assert_eq!(md("<hidden>x</hidden>"), "<hidden>x</hidden>");
}

// ── Terminal: nested style restoration ───────────────────────────────────

#[test]
fn terminal_same_layer_color_nesting_restores_parent() {
    assert_eq!(
        term("<blue>before <red>red</red> after</blue>"),
        "\x1b[34mbefore \x1b[31mred\x1b[0m\x1b[34m after\x1b[0m"
    );
}

#[test]
fn terminal_deep_three_layer_nesting() {
    assert_eq!(
        term("<b><red><i>deep</i></red></b>"),
        "\x1b[1m\x1b[31m\x1b[3m\x1b[31mdeep\x1b[0m\x1b[31m\x1b[0m\x1b[0m"
    );
}

#[test]
fn terminal_single_block_emits_one_reset() {
    assert_eq!(term("<red>hello</red>"), "\x1b[31mhello\x1b[0m");
}

// ── Terminal: links, OSC8 vs Markdown fallback ───────────────────────────

#[test]
fn terminal_link_osc8_when_supported() {
    let terminal = full_terminal();
    assert_eq!(
        term_caps(r#"<a href="https://example.com">link</a>"#, &terminal),
        "\x1b]8;;https://example.com\x1b\\link\x1b]8;;\x1b\\"
    );
}

#[test]
fn terminal_link_markdown_fallback_when_unsupported() {
    let terminal = Terminal::builder()
        .width(200)
        .osc_link_support(false)
        .build();
    assert_eq!(
        term_caps(r#"<a href="https://example.com">link</a>"#, &terminal),
        "[link](https://example.com)"
    );
}

#[test]
fn terminal_path_like_link_resolves_to_file_url() {
    let terminal = full_terminal();
    assert_eq!(
        term_caps(r#"<a href="/usr/local/bin/x">x</a>"#, &terminal),
        "\x1b]8;;file:///usr/local/bin/x\x1b\\x\x1b]8;;\x1b\\"
    );
}

// ── Terminal: unknown tags and escaped literal markup ────────────────────

#[test]
fn terminal_unknown_tag_is_literal_text() {
    assert_eq!(term("<unknown>x</unknown>"), "<unknown>x</unknown>");
}

#[test]
fn terminal_escaped_angle_brackets_are_literal() {
    assert_eq!(term(r"use \<ENV\> here"), "use <ENV> here");
}

// ── Terminal: fenced code blocks ─────────────────────────────────────────

#[test]
fn terminal_code_fence_is_dim_and_indented() {
    assert_eq!(
        term("```\ncode\n```"),
        "\x1b[2m    code\x1b[0m"
    );
}

#[test]
fn terminal_code_fence_preserves_literal_markup() {
    assert_eq!(
        term("```\n**not bold**\n```"),
        "\x1b[2m    **not bold**\x1b[0m"
    );
}

// ── Terminal: layout, margin, word wrapping ──────────────────────────────

#[test]
fn terminal_left_margin_indents() {
    use crate::utils::layout::{Length, TargetValue};
    let prose = Prose::new("hi").with_left_margin(TargetValue::universal(Length::ch(3)));
    assert_eq!(prose.render_optimistic(Some(40)), "   hi");
}

#[test]
fn terminal_word_wrap_breaks_long_line() {
    use crate::utils::wrap_policy::WordWrap;
    let prose =
        Prose::new("one two three four five").with_word_wrap(WordWrap::WrapProse(None, None));
    // At width 10, "one two" (7) fits, "three" (5) fits on its own line,
    // and "four five" (9) fits on the last line.
    assert_eq!(prose.render_optimistic(Some(10)), "one two\nthree\nfour five");
}

// ── Browser ──────────────────────────────────────────────────────────────

#[test]
fn browser_bold_is_semantic_strong() {
    assert!(html("<b>x</b>").contains("<strong>x</strong>"));
}

#[test]
fn browser_foreground_color_span() {
    assert!(
        html("<red>x</red>").contains("<span style=\"color:#800000\">x</span>"),
        "got: {}",
        html("<red>x</red>")
    );
}

#[test]
fn browser_background_color_span() {
    assert!(
        html("<bg-rgb 1,2,3>x</bg-rgb>")
            .contains("<span style=\"background-color:#010203\">x</span>"),
        "got: {}",
        html("<bg-rgb 1,2,3>x</bg-rgb>")
    );
}

#[test]
fn browser_inverse_uses_invert_filter() {
    assert!(
        html("<inverse>x</inverse>").contains("<span style=\"filter:invert(1)\">x</span>"),
        "got: {}",
        html("<inverse>x</inverse>")
    );
}

#[test]
fn browser_user_text_is_html_escaped() {
    let h = html(r"\<script\>x\</script\>");
    assert!(h.contains("&lt;script&gt;x&lt;/script&gt;"), "got: {h}");
    assert!(!h.contains("<script>"), "got: {h}");
}

#[test]
fn browser_href_is_attribute_escaped() {
    let attr = Prose::quoted_attr(r#"https://example.com/a>b?q="y""#);
    let h = html(&format!("<a href={attr}>x</a>"));
    assert!(
        h.contains(r#"<a href="https://example.com/a>b?q=&quot;y&quot;">x</a>"#),
        "got: {h}"
    );
}

#[test]
fn browser_code_block_body_is_escaped() {
    assert!(
        html("```\n</code-block><red>x</red>\n```").contains(
            "<pre><code>&lt;/code-block&gt;&lt;red&gt;x&lt;/red&gt;</code></pre>"
        ),
        "got: {}",
        html("```\n</code-block><red>x</red>\n```")
    );
}

// ── Markdown (portable) ──────────────────────────────────────────────────

#[test]
fn markdown_semantic_styles() {
    assert_eq!(md("<b>x</b>"), "**x**");
    assert_eq!(md("<i>x</i>"), "_x_");
    assert_eq!(md("<~>x</~>"), "~~x~~");
}

#[test]
fn markdown_color_degrades_to_inner_text() {
    assert_eq!(md("<red>important</red>"), "important");
}

#[test]
fn markdown_inverse_degrades_to_inner_text() {
    assert_eq!(md("<inverse>x</inverse>"), "x");
}

#[test]
fn markdown_link_basic() {
    assert_eq!(
        md(r#"<a href="https://example.com">go</a>"#),
        "[go](https://example.com)"
    );
}

#[test]
fn markdown_link_destination_escapes_parens() {
    assert_eq!(
        md(r#"<a href="https://example.com/a)b">go</a>"#),
        r"[go](https://example.com/a\)b)"
    );
}

#[test]
fn markdown_link_destination_escapes_backslash() {
    assert_eq!(
        md(r#"<a href="https://example.com/a\b">go</a>"#),
        r"[go](https://example.com/a\\b)"
    );
}

#[test]
fn markdown_link_destination_with_whitespace_uses_angle_brackets() {
    assert_eq!(
        md(r#"<a href="https://example.com/a b">go</a>"#),
        "[go](<https://example.com/a b>)"
    );
}

#[test]
fn markdown_link_description_not_double_escaped() {
    assert_eq!(
        md(r#"<a href="https://example.com">a]b</a>"#),
        "[a]b](https://example.com)"
    );
}

#[test]
fn markdown_unknown_tag_is_escaped_text() {
    assert_eq!(md("<unknown>x</unknown>"), "<unknown>x</unknown>");
}

#[test]
fn markdown_code_fence_round_trips() {
    assert_eq!(md("```rust\nlet x = 1;\n```"), "```rust\nlet x = 1;\n```");
}

// ── MarkdownPlus (inline-HTML styling) ───────────────────────────────────

#[test]
fn markdown_plus_keeps_semantic_styles() {
    assert_eq!(md_plus("<b>x</b>"), "**x**");
    assert_eq!(md_plus("<i>x</i>"), "_x_");
    assert_eq!(md_plus("<~>x</~>"), "~~x~~");
}

#[test]
fn markdown_plus_foreground_color_span() {
    assert_eq!(
        md_plus("<red>x</red>"),
        "<span style=\"color: rgb(128, 0, 0)\">x</span>"
    );
}

#[test]
fn markdown_plus_background_color_span() {
    assert_eq!(
        md_plus("<bg-rgb 1,2,3>x</bg-rgb>"),
        "<span style=\"background-color: rgb(1, 2, 3)\">x</span>"
    );
}

#[test]
fn markdown_plus_underline_span() {
    assert_eq!(
        md_plus("<u>x</u>"),
        "<span style=\"text-decoration: underline\">x</span>"
    );
}

#[test]
fn markdown_plus_inverse_degrades_to_inner_text() {
    assert_eq!(md_plus("<inverse>x</inverse>"), "x");
}

#[test]
fn markdown_plus_color_with_semantic_bold() {
    assert_eq!(
        md_plus("<red><b>x</b></red>"),
        "<span style=\"color: rgb(128, 0, 0)\">**x**</span>"
    );
}

#[test]
fn markdown_plus_html_escapes_styled_body() {
    assert_eq!(
        md_plus("<red>a & b</red>"),
        "<span style=\"color: rgb(128, 0, 0)\">a &amp; b</span>"
    );
}
