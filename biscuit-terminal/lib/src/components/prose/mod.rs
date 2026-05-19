//! Styled prose rendering with bracketed-tag (`<red>…</red>`) and Markdown-subset
//! grammar, rendered to Terminal, Browser, and Markdown targets.
//!
//! The module is split along grammar / styling / rendering axes:
//!
//! - [`prose`] — the public [`Prose`] struct and its builder API
//! - [`markdown`] — the Markdown-subset pre-processor
//! - [`tokens`] — the bracketed-tag parser that builds the IR
//! - [`ir`] — the target-neutral [`ProseDocument`](ir::ProseDocument) model
//! - [`styles`] — color-name lookups, href resolution, SGR layer state
//! - [`terminal`] — the IR → terminal ANSI/OSC8 emitter
//! - [`browser`] — `BrowserRenderable` for [`Prose`]
//! - [`to_markdown`] — `MarkdownRenderable` for [`Prose`]
//! - [`render`] — the [`TerminalRenderable`](crate::components::renderable::TerminalRenderable) impl

mod browser;
mod ir;
mod markdown;
#[allow(clippy::module_inception)]
mod prose;
mod render;
mod styles;
mod terminal;
mod to_markdown;
mod tokens;

pub use self::prose::{IntoProseVec, Prose};

#[cfg(test)]
mod tests {
    use super::prose::Prose;
    use super::styles::resolve_href;
    use crate::components::renderable::TerminalRenderable;
    use crate::discovery::detection::UnderlineSupport;
    use crate::terminal::Terminal;

    // ── Former atomic-token syntax is now literal text ───────────────

    #[test]
    fn atomic_tokens_render_as_literal_text() {
        // The atomic grammar has been removed: `{{token}}` is ordinary text.
        let prose = Prose::new("{{bold}}Important:{{reset}} text");
        assert_eq!(
            prose.render_optimistic(None),
            "{{bold}}Important:{{reset}} text"
        );
    }

    #[test]
    fn known_atomic_token_names_are_not_styled() {
        for token in ["{{bold}}", "{{reset}}", "{{red}}", "{{bg-blue}}"] {
            let result = Prose::new(token).render_optimistic(None);
            assert_eq!(result, token, "token {token:?} must render literally");
            assert!(!result.contains('\x1b'), "token {token:?} emitted an escape");
        }
    }

    // ── Block-tag rendering ──────────────────────────────────────────

    #[test]
    fn test_block_bold_tag() {
        let prose = Prose::new("<b>bold text</b>");
        let result = prose.render_optimistic(None);
        assert_eq!(result, "\x1b[1mbold text\x1b[22m\x1b[0m");
    }

    #[test]
    fn test_block_italic_tag() {
        let prose = Prose::new("<i>italic text</i>");
        let result = prose.render_optimistic(None);
        assert_eq!(result, "\x1b[3mitalic text\x1b[23m\x1b[0m");
    }

    #[test]
    fn test_block_underline_tag() {
        let prose = Prose::new("<u>underlined</u>");
        let result = prose.render_optimistic(None);
        assert_eq!(result, "\x1b[4munderlined\x1b[24m\x1b[0m");
    }

    #[test]
    fn test_underline_variants_block() {
        let prose = Prose::new("<double-underline>double</double-underline>");
        assert_eq!(
            prose.render_optimistic(None),
            "\x1b[4:2mdouble\x1b[24m\x1b[0m"
        );

        let prose = Prose::new("<uu>double</uu>");
        assert_eq!(
            prose.render_optimistic(None),
            "\x1b[4:2mdouble\x1b[24m\x1b[0m"
        );

        let prose = Prose::new("<curly-underline>curly</curly-underline>");
        assert_eq!(prose.render_optimistic(None), "\x1b[4:3mcurly\x1b[24m\x1b[0m");

        let prose = Prose::new("<dotted-underline>dotted</dotted-underline>");
        assert_eq!(
            prose.render_optimistic(None),
            "\x1b[4:4mdotted\x1b[24m\x1b[0m"
        );

        let prose = Prose::new("<dashed-underline>dashed</dashed-underline>");
        assert_eq!(
            prose.render_optimistic(None),
            "\x1b[4:5mdashed\x1b[24m\x1b[0m"
        );
    }

    #[test]
    fn test_block_aliases() {
        assert_eq!(
            Prose::new("<bold>x</bold>").render_optimistic(None),
            Prose::new("<b>x</b>").render_optimistic(None)
        );
        assert_eq!(
            Prose::new("<italic>x</italic>").render_optimistic(None),
            Prose::new("<i>x</i>").render_optimistic(None)
        );
        assert_eq!(
            Prose::new("<underline>x</underline>").render_optimistic(None),
            Prose::new("<u>x</u>").render_optimistic(None)
        );
        assert_eq!(
            Prose::new("<strikethrough>x</strikethrough>").render_optimistic(None),
            Prose::new("<~>x</~>").render_optimistic(None)
        );
    }

    #[test]
    fn test_nested_block_tags() {
        let prose = Prose::new("<b><i>bold italic</i></b>");
        let result = prose.render_optimistic(None);
        assert_eq!(result, "\x1b[1m\x1b[3mbold italic\x1b[23m\x1b[22m\x1b[0m");
    }

    #[test]
    fn test_nested_tags_with_unicode_before_rgb_attr_tag() {
        let prose = Prose::new("<b>Stage 0 — <rgb 64,128,255>planning</rgb></b>");
        let result = prose.render_optimistic(None);
        assert!(result.contains("Stage 0 —"));
        assert!(result.contains("planning"));
        assert!(result.contains("\x1b[38;2;64;128;255m"));
    }

    #[test]
    fn test_osc8_link() {
        let prose = Prose::new("<a href=\"https://example.com\">link</a>");
        let result = prose.render_optimistic(None);
        assert_eq!(
            result,
            "\x1b]8;;https://example.com\x1b\\link\x1b]8;;\x1b\\\x1b[0m"
        );
    }

    #[test]
    fn test_plain_text_no_reset() {
        let prose = Prose::new("Plain text with no styles");
        let result = prose.render_optimistic(None);
        assert_eq!(result, "Plain text with no styles");
    }

    #[test]
    fn test_strikethrough_block() {
        let prose = Prose::new("<~>deleted</~>");
        let result = prose.render_optimistic(None);
        assert_eq!(result, "\x1b[9mdeleted\x1b[29m\x1b[0m");
    }

    #[test]
    fn test_named_color_block() {
        let prose = Prose::new("<red>error message</red>");
        let result = prose.render_optimistic(None);
        assert_eq!(result, "\x1b[31merror message\x1b[39m\x1b[0m");
    }

    #[test]
    fn test_bright_color_block() {
        let prose = Prose::new("<bright-red>bright error</bright-red>");
        assert_eq!(
            prose.render_optimistic(None),
            "\x1b[91mbright error\x1b[39m\x1b[0m"
        );

        let prose = Prose::new("<bright-cyan>info</bright-cyan>");
        assert_eq!(prose.render_optimistic(None), "\x1b[96minfo\x1b[39m\x1b[0m");
    }

    #[test]
    fn test_web_color_block() {
        let prose = Prose::new("<coral>coral text</coral>");
        let result = prose.render_optimistic(None);
        assert!(result.contains("\x1b[38;2;255;127;80m"));
        assert!(result.contains("coral text"));
        assert!(result.contains("\x1b[39m"));

        let prose = Prose::new("<alice-blue>light blue</alice-blue>");
        let result = prose.render_optimistic(None);
        assert!(result.contains("\x1b[38;2;240;248;255m"));
        assert!(result.contains("light blue"));
    }

    #[test]
    fn test_tailwind_color_block() {
        let prose = Prose::new("<purple-500>tailwind purple</purple-500>");
        let result = prose.render_optimistic(None);
        assert!(result.contains("\x1b[38;2;"));
        assert!(result.contains("tailwind purple"));
        assert!(result.contains("\x1b[39m"));

        let prose = Prose::new("<slate-500>muted</slate-500>");
        let result = prose.render_optimistic(None);
        assert!(result.contains("\x1b[38;2;"));
        assert!(result.contains("muted"));
    }

    #[test]
    fn test_unknown_tag_preserved() {
        let prose = Prose::new("<unknown-tag>content</unknown-tag>");
        let result = prose.render_optimistic(None);
        assert!(result.contains("<unknown-tag>content</unknown-tag>"));
    }

    #[test]
    fn test_unknown_unclosed_tag_passes_through_literally() {
        let prose = Prose::new("<root> \"title\" is a required property");
        let result = prose.render_optimistic(None);
        assert_eq!(result, "<root> \"title\" is a required property");
    }

    #[test]
    fn test_unknown_tag_does_not_swallow_following_content() {
        let prose = Prose::new("<missing.yaml> not found; check <other.yaml>");
        let result = prose.render_optimistic(None);
        assert_eq!(result, "<missing.yaml> not found; check <other.yaml>");
    }

    #[test]
    fn test_unknown_tag_does_not_eat_recognized_tag_after_it() {
        let prose = Prose::new("<root> then <b>bold</b>");
        let result = prose.render_optimistic(None);
        assert!(result.starts_with("<root> then "), "got: {result:?}");
        assert!(result.contains("\x1b[1mbold\x1b[22m"), "got: {result:?}");
        assert!(!result.contains("</root>"), "got: {result:?}");
    }

    #[test]
    fn test_rgb_tag() {
        let prose = Prose::new("<rgb 255,0,0>red text</rgb>");
        let result = prose.render_optimistic(None);
        assert!(result.contains("\x1b[38;2;255;0;0m"), "got: {result:?}");
        assert!(result.contains("red text"));
        assert!(result.contains("\x1b[39m"));

        let prose = Prose::new("<rgb 125,67,45>brown text</rgb>");
        let result = prose.render_optimistic(None);
        assert!(result.contains("\x1b[38;2;125;67;45m"));
        assert!(result.contains("brown text"));
    }

    #[test]
    fn test_resolve_href_urls_unchanged() {
        assert_eq!(resolve_href("https://example.com"), "https://example.com");
        assert_eq!(resolve_href("http://example.com"), "http://example.com");
        assert_eq!(
            resolve_href("mailto:test@example.com"),
            "mailto:test@example.com"
        );
        assert_eq!(resolve_href("file:///path/to/file"), "file:///path/to/file");
    }

    #[test]
    fn test_resolve_href_absolute_path() {
        assert_eq!(resolve_href("/usr/local/bin/test"), "file:///usr/local/bin/test");
    }

    #[test]
    fn test_resolve_href_empty() {
        assert_eq!(resolve_href(""), "");
    }

    #[test]
    fn test_resolve_href_relative_with_dot_slash() {
        let result = resolve_href("./test.txt");
        assert!(result.starts_with("file://"));
        assert!(result.contains("test.txt"));
    }

    #[test]
    fn test_resolve_href_relative_without_prefix() {
        let result = resolve_href("src/main.rs");
        assert!(result.starts_with("file://"));
        assert!(result.contains("src/main.rs"));
    }

    #[test]
    fn test_bg_rgb_tag() {
        let prose = Prose::new("<bg-rgb 255,0,0>red bg</bg-rgb>");
        let result = prose.render_optimistic(None);
        assert!(result.contains("\x1b[48;2;255;0;0m"), "got: {result:?}");
        assert!(result.contains("red bg"));
        assert!(result.contains("\x1b[49m"));

        let prose = Prose::new("<bg-rgb 125,67,45>brown bg</bg-rgb>");
        let result = prose.render_optimistic(None);
        assert!(result.contains("\x1b[48;2;125;67;45m"));
        assert!(result.contains("brown bg"));
    }

    #[test]
    fn test_rgb_hex_format() {
        assert!(
            Prose::new("<rgb #FF0000>red</rgb>")
                .render_optimistic(None)
                .contains("\x1b[38;2;255;0;0m")
        );
        assert!(
            Prose::new("<rgb FF0000>red</rgb>")
                .render_optimistic(None)
                .contains("\x1b[38;2;255;0;0m")
        );
        assert!(
            Prose::new("<rgb #8B0000>dark red</rgb>")
                .render_optimistic(None)
                .contains("\x1b[38;2;139;0;0m")
        );
    }

    #[test]
    fn test_rgb_space_separated() {
        assert!(
            Prose::new("<rgb 255 0 0>red</rgb>")
                .render_optimistic(None)
                .contains("\x1b[38;2;255;0;0m")
        );
        assert!(
            Prose::new("<rgb  125  67  45 >brown</rgb>")
                .render_optimistic(None)
                .contains("\x1b[38;2;125;67;45m")
        );
    }

    #[test]
    fn test_bg_rgb_hex_format() {
        assert!(
            Prose::new("<bg-rgb #00FF00>green bg</bg-rgb>")
                .render_optimistic(None)
                .contains("\x1b[48;2;0;255;0m")
        );
        assert!(
            Prose::new("<bg-rgb 0000FF>blue bg</bg-rgb>")
                .render_optimistic(None)
                .contains("\x1b[48;2;0;0;255m")
        );
    }

    #[test]
    fn test_bg_rgb_space_separated() {
        assert!(
            Prose::new("<bg-rgb 255 128 0>orange bg</bg-rgb>")
                .render_optimistic(None)
                .contains("\x1b[48;2;255;128;0m")
        );
    }

    #[test]
    fn test_bg_web_color_block() {
        let prose = Prose::new("<bg-coral>coral bg</bg-coral>");
        let result = prose.render_optimistic(None);
        assert!(result.contains("\x1b[48;2;255;127;80m"));
        assert!(result.contains("coral bg"));
        assert!(result.contains("\x1b[49m"));

        let prose = Prose::new("<bg-alice-blue>light blue bg</bg-alice-blue>");
        let result = prose.render_optimistic(None);
        assert!(result.contains("\x1b[48;2;240;248;255m"));
        assert!(result.contains("light blue bg"));
    }

    #[test]
    fn test_bg_tailwind_color_block() {
        let prose = Prose::new("<bg-red-800>danger bg</bg-red-800>");
        let result = prose.render_optimistic(None);
        assert!(result.contains("\x1b[48;2;"));
        assert!(result.contains("danger bg"));
        assert!(result.contains("\x1b[49m"));

        let prose = Prose::new("<bg-slate-500>muted bg</bg-slate-500>");
        let result = prose.render_optimistic(None);
        assert!(result.contains("\x1b[48;2;"));
        assert!(result.contains("muted bg"));
    }

    #[test]
    fn test_osc8_link_with_absolute_path() {
        let prose = Prose::new("<a href=\"/usr/local/bin/test\">link</a>");
        let result = prose.render_optimistic(None);
        assert!(result.contains("file:///usr/local/bin/test"));
    }

    #[test]
    fn osc8_link_from_quoted_attr_preserves_escaped_delimiter() {
        // The `>` that `quoted_attr` escapes must survive the tag scan and
        // reach the OSC8 destination intact.
        let term = Terminal::builder().osc_link_support(true).build();
        let attr = Prose::quoted_attr("https://example.com/a>b");
        let result = Prose::new(&format!("<a href={attr}>x</a>")).parse_tokens(Some(&term));
        assert!(
            result.contains("\x1b]8;;https://example.com/a>b\x1b\\"),
            "got: {result:?}"
        );
    }

    #[test]
    fn osc8_link_from_quoted_attr_with_both_quote_types() {
        let term = Terminal::builder().osc_link_support(true).build();
        let value = r#"https://example.com/a>b?q='x'&r="y""#;
        let attr = Prose::quoted_attr(value);
        let result = Prose::new(&format!("<a href={attr}>x</a>")).parse_tokens(Some(&term));
        assert!(
            result.contains(&format!("\x1b]8;;{value}\x1b\\")),
            "got: {result:?}"
        );
    }

    // ── Style-layer tracking ─────────────────────────────────────────

    #[test]
    fn test_same_layer_fg_nesting_restores_parent() {
        let prose = Prose::new("<blue>before <red>red</red> after</blue>");
        let result = prose.render_optimistic(None);
        assert_eq!(
            result,
            "\x1b[34mbefore \x1b[31mred\x1b[34m after\x1b[39m\x1b[0m"
        );
    }

    #[test]
    fn test_no_mid_content_resets() {
        let prose = Prose::new("<b><i>text</i> more</b>");
        let result = prose.render_optimistic(None);
        let reset_count = result.matches("\x1b[0m").count();
        assert_eq!(reset_count, 1, "got: {result:?}");
        assert!(result.ends_with("\x1b[0m"));
    }

    #[test]
    fn test_deep_nesting_three_layers() {
        let prose = Prose::new("<b><red><i>deep</i></red></b>");
        let result = prose.render_optimistic(None);
        assert_eq!(
            result,
            "\x1b[1m\x1b[31m\x1b[3mdeep\x1b[23m\x1b[39m\x1b[22m\x1b[0m"
        );
    }

    #[test]
    fn test_single_block_no_extra_reset() {
        let prose = Prose::new("<red>hello</red>");
        let result = prose.render_optimistic(None);
        assert_eq!(result, "\x1b[31mhello\x1b[39m\x1b[0m");
    }

    // ── Escape handling ──────────────────────────────────────────────

    #[test]
    fn test_escaped_angle_brackets() {
        let prose = Prose::new("use \\<ENV\\> here");
        assert_eq!(prose.render_optimistic(None), "use <ENV> here");
    }

    #[test]
    fn test_escaped_angle_brackets_inside_block_tag() {
        let prose = Prose::new("<dim>\\<ENV\\></dim>");
        let result = prose.render_optimistic(None);
        assert!(result.contains("<ENV>"));
        assert!(result.contains("\x1b[2m"));
    }

    #[test]
    fn test_escaped_backslash() {
        let prose = Prose::new("path\\\\name");
        assert_eq!(prose.render_optimistic(None), "path\\name");
    }

    #[test]
    fn test_escaped_open_brace() {
        let prose = Prose::new("\\{not a token}}");
        assert!(prose.render_optimistic(None).contains("{not a token}}"));
    }

    #[test]
    fn test_backslash_before_normal_char_preserved() {
        let prose = Prose::new("hello\\nworld");
        assert_eq!(prose.render_optimistic(None), "hello\\nworld");
    }

    #[test]
    fn test_escaped_asterisk() {
        assert_eq!(
            Prose::new("\\*literal\\*").render_optimistic(None),
            "*literal*"
        );
    }

    #[test]
    fn test_escaped_underscore() {
        assert_eq!(
            Prose::new("\\_literal\\_").render_optimistic(None),
            "_literal_"
        );
    }

    #[test]
    fn test_escaped_open_bracket() {
        assert_eq!(
            Prose::new("\\[not a link\\]").render_optimistic(None),
            "[not a link]"
        );
    }

    #[test]
    fn test_escaped_open_paren() {
        assert_eq!(
            Prose::new("\\(not a url\\)").render_optimistic(None),
            "(not a url)"
        );
    }

    #[test]
    fn test_escaped_double_backslash_still_single() {
        assert_eq!(Prose::new("a\\\\b").render_optimistic(None), "a\\b");
    }

    // ── Capability-aware degradation ─────────────────────────────────

    #[test]
    fn test_osc8_link_supported_emits_osc8() {
        let term = Terminal::builder().osc_link_support(true).build();
        let prose = Prose::new("<a href=\"https://example.com\">click here</a>");
        let result = prose.parse_tokens(Some(&term));
        assert!(
            result.contains("\x1b]8;;https://example.com\x1b\\"),
            "got: {result:?}"
        );
        assert!(result.contains("click here"));
        assert!(result.contains("\x1b]8;;\x1b\\"));
    }

    #[test]
    fn test_osc8_link_unsupported_emits_markdown_fallback() {
        let term = Terminal::builder().osc_link_support(false).build();
        let prose = Prose::new("<a href=\"https://example.com\">click here</a>");
        let result = prose.parse_tokens(Some(&term));
        assert!(
            result.contains("[click here](https://example.com)"),
            "got: {result:?}"
        );
        assert!(!result.contains("\x1b]8;;"), "got: {result:?}");
    }

    #[test]
    fn link_markdown_fallback_escapes_bracket_in_description() {
        let term = Terminal::builder().osc_link_support(false).build();
        let prose = Prose::new(r#"<a href="https://example.com">array[0]</a>"#);
        let result = prose.parse_tokens(Some(&term));
        assert!(
            result.contains(r"[array[0\]](https://example.com)"),
            "got: {result:?}"
        );
        assert!(!result.contains("\x1b]8;;"), "got: {result:?}");
    }

    #[test]
    fn test_double_underline_degrades_to_straight_when_only_straight_supported() {
        let term = Terminal::builder()
            .underline_support(UnderlineSupport {
                straight: true,
                double: false,
                curly: false,
                dotted: false,
                dashed: false,
                colored: false,
            })
            .build();
        let prose = Prose::new("<double-underline>important text</double-underline>");
        let result = prose.parse_tokens(Some(&term));
        assert!(result.contains("\x1b[4m"), "got: {result:?}");
        assert!(result.contains("important text"));
        assert!(!result.contains("\x1b[4:2m"), "got: {result:?}");
        assert!(result.contains("\x1b[24m"), "got: {result:?}");
    }

    #[test]
    fn test_double_underline_suppressed_when_no_underline_support() {
        let term = Terminal::builder()
            .underline_support(UnderlineSupport {
                straight: false,
                double: false,
                curly: false,
                dotted: false,
                dashed: false,
                colored: false,
            })
            .build();
        let prose = Prose::new("<double-underline>important text</double-underline>");
        let result = prose.parse_tokens(Some(&term));
        assert_eq!(result, "important text", "got: {result:?}");
        assert!(!result.contains("\x1b["), "got: {result:?}");
    }

    #[test]
    fn test_osc8_link_empty_href_renders_inner_only() {
        let term = Terminal::builder().osc_link_support(true).build();
        let prose = Prose::new("<a href=\"\">click here</a>");
        let result = prose.parse_tokens(Some(&term));
        assert!(result.contains("click here"));
        assert!(!result.contains("\x1b]8;;"));
        assert!(!result.contains("[click here]"));
    }

    // ── Fenced code blocks ───────────────────────────────────────────

    #[test]
    fn code_block_renders_dim_and_indented() {
        let prose = Prose::new("<code-block lang=\"yaml\">key: value\nlist:\n  - a</code-block>");
        let result = prose.parse_tokens(None);
        assert!(result.contains("\x1b[2m  key: value\x1b[0m"));
        assert!(result.contains("\x1b[2m  list:\x1b[0m"));
        assert!(result.contains("\x1b[2m    - a\x1b[0m"));
    }

    #[test]
    fn fenced_code_block_preserves_literal_markup() {
        let prose = Prose::new("```\n**not bold**\n```");
        let result = prose.parse_tokens(None);
        assert!(result.contains("**not bold**"), "got: {result:?}");
        assert!(!result.contains("\x1b[1m"));
    }

    #[test]
    fn code_block_inside_span_restores_enclosing_style() {
        // Regression (review-5): a fenced code block nested inside an
        // active style must restore the enclosing span's SGR after its
        // hard `\x1b[0m` reset, so following sibling text stays styled.
        let prose = Prose::new("<red>before\n```\ncode\n```\nafter</red>");
        let result = prose.parse_tokens(None);
        // The code line resets, then the enclosing red foreground is
        // re-emitted before the trailing `after` text.
        assert!(
            result.contains("\x1b[2m  code\x1b[0m\x1b[31m"),
            "got: {result:?}"
        );
        let restore_idx = result
            .find("\x1b[0m\x1b[31m")
            .expect("code-block reset followed by restored red");
        let after_idx = result.find("after").expect("trailing text present");
        assert!(
            restore_idx < after_idx,
            "red must be restored before `after`; got: {result:?}"
        );
    }

    #[test]
    fn standalone_code_block_emits_no_restoration_escape() {
        // A code block with no enclosing span re-emits nothing after its
        // reset — the standalone dim output is unchanged.
        let prose = Prose::new("```\ncode\n```");
        let result = prose.parse_tokens(None);
        let dim = "\x1b[2m  code\x1b[0m";
        assert!(result.starts_with(dim), "got: {result:?}");
        // Nothing is re-applied after the code-block reset; only the
        // document's own final reset may follow.
        let tail = &result[dim.len()..];
        assert!(tail.is_empty() || tail == "\x1b[0m", "got: {result:?}");
    }

    #[test]
    fn fenced_code_block_with_closing_tag_in_body_stays_opaque() {
        // Regression: a fenced body containing the synthetic
        // `</code-block>` tag plus `<red>` markup must render verbatim as
        // dim code — no red foreground color escape may leak out.
        let prose = Prose::new("```\n</code-block><red>x</red>\n```");
        let result = prose.parse_tokens(None);
        assert!(
            result.contains("</code-block><red>x</red>"),
            "got: {result:?}"
        );
        assert!(!result.contains("\x1b[31m"), "got: {result:?}");
    }
}
