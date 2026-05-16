//! Styled prose rendering with token (`{{bold}}`) and block-tag (`<red>…</red>`)
//! grammar.
//!
//! The module is split along grammar / styling / rendering axes:
//!
//! - [`prose`] — the public [`Prose`] struct and its builder API
//! - [`tokens`] — the recursive parser for atomic and block tokens
//! - [`styles`] — color/weight resolution tables and SGR layer state
//! - [`render`] — the [`TerminalRenderable`](crate::components::renderable::TerminalRenderable) impl

mod markdown;
#[allow(clippy::module_inception)]
mod prose;
mod render;
mod styles;
mod tokens;

pub use self::prose::{IntoProseVec, Prose};

#[cfg(test)]
mod tests {
    use super::prose::Prose;
    use super::styles::{
        BlockTagAction, StyleLayer, atomic_token_to_escape_with_term, block_tag_layer,
        block_tag_to_escape, resolve_href,
    };
    use crate::components::renderable::TerminalRenderable;
    use crate::discovery::detection::UnderlineSupport;
    use crate::terminal::Terminal;

    #[test]
    fn test_atomic_bold_token() {
        let prose = Prose::new("Hello {{bold}}world{{reset}}!");
        let result = prose.render_optimistic(None);
        assert_eq!(result, "Hello \x1b[1mworld\x1b[0m!\x1b[0m");
    }

    #[test]
    fn test_atomic_color_token() {
        let prose = Prose::new("{{red}}Error{{reset}}");
        let result = prose.render_optimistic(None);
        assert_eq!(result, "\x1b[31mError\x1b[0m\x1b[0m");
    }

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
    fn test_underline_variants_atomic() {
        // Double underline
        let prose = Prose::new("{{double-underline}}text{{reset}}");
        let result = prose.render_optimistic(None);
        assert!(result.contains("\x1b[4:2m"));

        // Curly underline
        let prose = Prose::new("{{curly-underline}}text{{reset}}");
        let result = prose.render_optimistic(None);
        assert!(result.contains("\x1b[4:3m"));

        // Dotted underline
        let prose = Prose::new("{{dotted-underline}}text{{reset}}");
        let result = prose.render_optimistic(None);
        assert!(result.contains("\x1b[4:4m"));

        // Dashed underline
        let prose = Prose::new("{{dashed-underline}}text{{reset}}");
        let result = prose.render_optimistic(None);
        assert!(result.contains("\x1b[4:5m"));
    }

    #[test]
    fn test_underline_variants_block() {
        // Double underline (full name)
        let prose = Prose::new("<double-underline>double</double-underline>");
        let result = prose.render_optimistic(None);
        assert_eq!(result, "\x1b[4:2mdouble\x1b[24m\x1b[0m");

        // Double underline (alias)
        let prose = Prose::new("<uu>double</uu>");
        let result = prose.render_optimistic(None);
        assert_eq!(result, "\x1b[4:2mdouble\x1b[24m\x1b[0m");

        // Curly underline
        let prose = Prose::new("<curly-underline>curly</curly-underline>");
        let result = prose.render_optimistic(None);
        assert_eq!(result, "\x1b[4:3mcurly\x1b[24m\x1b[0m");

        // Dotted underline
        let prose = Prose::new("<dotted-underline>dotted</dotted-underline>");
        let result = prose.render_optimistic(None);
        assert_eq!(result, "\x1b[4:4mdotted\x1b[24m\x1b[0m");

        // Dashed underline
        let prose = Prose::new("<dashed-underline>dashed</dashed-underline>");
        let result = prose.render_optimistic(None);
        assert_eq!(result, "\x1b[4:5mdashed\x1b[24m\x1b[0m");
    }

    #[test]
    fn test_block_aliases() {
        // Test that aliases produce the same output as full names
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
        // One final \x1b[0m, not two — inner recursion no longer adds its own reset
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
    fn test_background_color() {
        let prose = Prose::new("{{bg-red}}highlight{{reset}}");
        let result = prose.render_optimistic(None);
        assert_eq!(result, "\x1b[41mhighlight\x1b[0m\x1b[0m");
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
        let result = prose.render_optimistic(None);
        assert_eq!(result, "\x1b[91mbright error\x1b[39m\x1b[0m");

        let prose = Prose::new("<bright-cyan>info</bright-cyan>");
        let result = prose.render_optimistic(None);
        assert_eq!(result, "\x1b[96minfo\x1b[39m\x1b[0m");
    }

    #[test]
    fn test_web_color_block() {
        // coral is RGB(255, 127, 80)
        let prose = Prose::new("<coral>coral text</coral>");
        let result = prose.render_optimistic(None);
        assert!(result.contains("\x1b[38;2;255;127;80m"));
        assert!(result.contains("coral text"));
        assert!(result.contains("\x1b[39m"));

        // alice-blue (with hyphen) - RGB(240, 248, 255)
        let prose = Prose::new("<alice-blue>light blue</alice-blue>");
        let result = prose.render_optimistic(None);
        assert!(result.contains("\x1b[38;2;240;248;255m"));
        assert!(result.contains("light blue"));
    }

    #[test]
    fn test_tailwind_color_block() {
        // purple-500 should resolve to a purple color
        let prose = Prose::new("<purple-500>tailwind purple</purple-500>");
        let result = prose.render_optimistic(None);
        // Should have 24-bit color escape
        assert!(result.contains("\x1b[38;2;"));
        assert!(result.contains("tailwind purple"));
        assert!(result.contains("\x1b[39m"));

        // slate-500 should resolve to a gray-ish color
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
        // Regression: the parser used to greedily consume the rest of the
        // input looking for a `</root>` closing tag, then synthesize a
        // fictitious closing tag in the output. Unknown opening tags must
        // pass through as literal text — callers shouldn't have to escape
        // angle-bracketed snippets like `<root>` or `<missing.yaml>` that
        // happen to land in arbitrary content.
        let prose = Prose::new("<root> \"title\" is a required property");
        let result = prose.render_optimistic(None);
        assert_eq!(result, "<root> \"title\" is a required property");
    }

    #[test]
    fn test_unknown_tag_does_not_swallow_following_content() {
        // A sibling unknown tag followed by more text must not be merged
        // into a single fabricated tag span.
        let prose = Prose::new("<missing.yaml> not found; check <other.yaml>");
        let result = prose.render_optimistic(None);
        assert_eq!(result, "<missing.yaml> not found; check <other.yaml>");
    }

    #[test]
    fn test_unknown_tag_does_not_eat_recognized_tag_after_it() {
        // A recognized tag appearing after an unknown one must still be
        // styled. Previously the unknown tag would slurp everything
        // including the recognized `<b>...</b>` into its faux body.
        let prose = Prose::new("<root> then <b>bold</b>");
        let result = prose.render_optimistic(None);
        assert!(result.starts_with("<root> then "), "got: {result:?}");
        assert!(result.contains("\x1b[1mbold\x1b[22m"), "got: {result:?}");
        assert!(!result.contains("</root>"), "got: {result:?}");
    }

    #[test]
    fn test_rgb_tag() {
        // Test RGB color tag parsing
        let prose = Prose::new("<rgb 255,0,0>red text</rgb>");
        let result = prose.render_optimistic(None);
        assert!(
            result.contains("\x1b[38;2;255;0;0m"),
            "Expected RGB escape code, got: {:?}",
            result
        );
        assert!(result.contains("red text"));
        assert!(result.contains("\x1b[39m"));

        // Test with different RGB values
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
        let result = resolve_href("/usr/local/bin/test");
        assert_eq!(result, "file:///usr/local/bin/test");
    }

    #[test]
    fn test_resolve_href_empty() {
        assert_eq!(resolve_href(""), "");
    }

    #[test]
    fn test_resolve_href_relative_with_dot_slash() {
        // ./something should resolve relative to CWD
        let result = resolve_href("./test.txt");
        assert!(result.starts_with("file://"));
        assert!(result.contains("test.txt"));
    }

    #[test]
    fn test_resolve_href_relative_without_prefix() {
        // Something without ./ should try git-relative resolution
        let result = resolve_href("src/main.rs");
        assert!(result.starts_with("file://"));
        assert!(result.contains("src/main.rs"));
    }

    #[test]
    fn test_bg_rgb_tag() {
        let prose = Prose::new("<bg-rgb 255,0,0>red bg</bg-rgb>");
        let result = prose.render_optimistic(None);
        assert!(
            result.contains("\x1b[48;2;255;0;0m"),
            "Expected bg RGB escape code, got: {:?}",
            result
        );
        assert!(result.contains("red bg"));
        assert!(result.contains("\x1b[49m"));

        let prose = Prose::new("<bg-rgb 125,67,45>brown bg</bg-rgb>");
        let result = prose.render_optimistic(None);
        assert!(result.contains("\x1b[48;2;125;67;45m"));
        assert!(result.contains("brown bg"));
    }

    #[test]
    fn test_rgb_hex_format() {
        // Hex with # prefix
        let prose = Prose::new("<rgb #FF0000>red</rgb>");
        let result = prose.render_optimistic(None);
        assert!(result.contains("\x1b[38;2;255;0;0m"));

        // Hex without # prefix
        let prose = Prose::new("<rgb FF0000>red</rgb>");
        let result = prose.render_optimistic(None);
        assert!(result.contains("\x1b[38;2;255;0;0m"));

        // Dark red #8B0000
        let prose = Prose::new("<rgb #8B0000>dark red</rgb>");
        let result = prose.render_optimistic(None);
        assert!(result.contains("\x1b[38;2;139;0;0m"));
    }

    #[test]
    fn test_rgb_space_separated() {
        // Space-separated values
        let prose = Prose::new("<rgb 255 0 0>red</rgb>");
        let result = prose.render_optimistic(None);
        assert!(result.contains("\x1b[38;2;255;0;0m"));

        // With extra spaces
        let prose = Prose::new("<rgb  125  67  45 >brown</rgb>");
        let result = prose.render_optimistic(None);
        assert!(result.contains("\x1b[38;2;125;67;45m"));
    }

    #[test]
    fn test_bg_rgb_hex_format() {
        // Hex with # prefix
        let prose = Prose::new("<bg-rgb #00FF00>green bg</bg-rgb>");
        let result = prose.render_optimistic(None);
        assert!(result.contains("\x1b[48;2;0;255;0m"));

        // Hex without # prefix
        let prose = Prose::new("<bg-rgb 0000FF>blue bg</bg-rgb>");
        let result = prose.render_optimistic(None);
        assert!(result.contains("\x1b[48;2;0;0;255m"));
    }

    #[test]
    fn test_bg_rgb_space_separated() {
        // Space-separated values
        let prose = Prose::new("<bg-rgb 255 128 0>orange bg</bg-rgb>");
        let result = prose.render_optimistic(None);
        assert!(result.contains("\x1b[48;2;255;128;0m"));
    }

    #[test]
    fn test_bg_web_color_block() {
        // coral is RGB(255, 127, 80)
        let prose = Prose::new("<bg-coral>coral bg</bg-coral>");
        let result = prose.render_optimistic(None);
        assert!(result.contains("\x1b[48;2;255;127;80m"));
        assert!(result.contains("coral bg"));
        assert!(result.contains("\x1b[49m"));

        // alice-blue (with hyphen) - RGB(240, 248, 255)
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

    // ── Style-layer tracking tests ───────────────────────────────────

    #[test]
    fn test_same_layer_fg_nesting_restores_parent() {
        // </red> should restore blue, not reset to default
        let prose = Prose::new("<blue>before <red>red</red> after</blue>");
        let result = prose.render_optimistic(None);
        assert_eq!(
            result,
            "\x1b[34mbefore \x1b[31mred\x1b[34m after\x1b[39m\x1b[0m"
        );
    }

    #[test]
    fn test_atomic_then_block_restores_atomic() {
        // {{blue}} sets foreground, then <red> opens a new scope.
        // </red> should restore to \x1b[34m (the atomic blue).
        let prose = Prose::new("{{blue}}<red>red</red>blue");
        let result = prose.render_optimistic(None);
        assert_eq!(result, "\x1b[34m\x1b[31mred\x1b[34mblue\x1b[0m");
    }

    #[test]
    fn test_no_mid_content_resets() {
        // Only one \x1b[0m should appear, at the very end
        let prose = Prose::new("<b><i>text</i> more</b>");
        let result = prose.render_optimistic(None);
        let reset_count = result.matches("\x1b[0m").count();
        assert_eq!(
            reset_count, 1,
            "Expected exactly one \\x1b[0m, got: {:?}",
            result
        );
        assert!(result.ends_with("\x1b[0m"));
    }

    #[test]
    fn test_deep_nesting_three_layers() {
        let prose = Prose::new("<b><red><i>deep</i></red></b>");
        let result = prose.render_optimistic(None);
        // bold → red → italic → close italic (→ \x1b[23m]) → close red (→ \x1b[39m]) → close bold (→ \x1b[22m]) → final reset
        assert_eq!(
            result,
            "\x1b[1m\x1b[31m\x1b[3mdeep\x1b[23m\x1b[39m\x1b[22m\x1b[0m"
        );
    }

    #[test]
    fn test_reset_style_preserves_background() {
        // {{reset-style}} should clear everything except background
        let prose = Prose::new("{{bg-red}}{{bold}}text{{reset-style}}still bg");
        let result = prose.render_optimistic(None);
        // After reset-style: background layer should still be Some
        // The escape sequence \x1b[22;23;24;25;27;28;29;39m is emitted,
        // but background \x1b[41m stays active in the state.
        assert!(result.contains("\x1b[41m")); // bg-red open
        assert!(result.contains("\x1b[1m")); // bold
        assert!(result.contains("\x1b[22;23;24;25;27;28;29;39m")); // reset-style
        assert!(result.contains("still bg"));
    }

    #[test]
    fn test_single_block_no_extra_reset() {
        // Single block tag should produce exactly one \x1b[0m
        let prose = Prose::new("<red>hello</red>");
        let result = prose.render_optimistic(None);
        assert_eq!(result, "\x1b[31mhello\x1b[39m\x1b[0m");
    }

    #[test]
    fn test_block_tag_layer_classification() {
        assert_eq!(block_tag_layer("b"), Some(StyleLayer::FontWeight));
        assert_eq!(block_tag_layer("bold"), Some(StyleLayer::FontWeight));
        assert_eq!(block_tag_layer("dim"), Some(StyleLayer::FontWeight));
        assert_eq!(block_tag_layer("i"), Some(StyleLayer::Italic));
        assert_eq!(block_tag_layer("italic"), Some(StyleLayer::Italic));
        assert_eq!(block_tag_layer("u"), Some(StyleLayer::Underline));
        assert_eq!(block_tag_layer("red"), Some(StyleLayer::Foreground));
        assert_eq!(block_tag_layer("bright-cyan"), Some(StyleLayer::Foreground));
        assert_eq!(block_tag_layer("rgb"), Some(StyleLayer::Foreground));
        assert_eq!(block_tag_layer("bg-rgb"), Some(StyleLayer::Background));
        assert_eq!(block_tag_layer("bg-red-800"), Some(StyleLayer::Background));
        assert_eq!(block_tag_layer("a"), None);
        assert_eq!(block_tag_layer("clipboard"), None);
        // Web/tailwind colors fall through to Foreground
        assert_eq!(block_tag_layer("coral"), Some(StyleLayer::Foreground));
        assert_eq!(block_tag_layer("bg-coral"), Some(StyleLayer::Background));
    }

    #[test]
    fn test_escaped_angle_brackets() {
        let prose = Prose::new("use \\<ENV\\> here");
        let result = prose.render_optimistic(None);
        assert_eq!(result, "use <ENV> here");
    }

    #[test]
    fn test_escaped_angle_brackets_inside_block_tag() {
        let prose = Prose::new("<dim>\\<ENV\\></dim>");
        let result = prose.render_optimistic(None);
        assert!(result.contains("<ENV>"));
        assert!(result.contains("\x1b[2m")); // dim open
    }

    #[test]
    fn test_escaped_backslash() {
        let prose = Prose::new("path\\\\name");
        let result = prose.render_optimistic(None);
        assert_eq!(result, "path\\name");
    }

    #[test]
    fn test_escaped_open_brace() {
        let prose = Prose::new("\\{not a token}}");
        let result = prose.render_optimistic(None);
        assert!(result.contains("{not a token}}"));
    }

    #[test]
    fn test_backslash_before_normal_char_preserved() {
        let prose = Prose::new("hello\\nworld");
        let result = prose.render_optimistic(None);
        assert_eq!(result, "hello\\nworld");
    }

    // ── Markdown-syntax escape handling (Phase 1 of prose-plus) ─────────

    #[test]
    fn test_escaped_asterisk() {
        let prose = Prose::new("\\*literal\\*");
        let result = prose.render_optimistic(None);
        assert_eq!(result, "*literal*");
    }

    #[test]
    fn test_escaped_underscore() {
        let prose = Prose::new("\\_literal\\_");
        let result = prose.render_optimistic(None);
        assert_eq!(result, "_literal_");
    }

    #[test]
    fn test_escaped_open_bracket() {
        let prose = Prose::new("\\[not a link\\]");
        let result = prose.render_optimistic(None);
        assert_eq!(result, "[not a link]");
    }

    #[test]
    fn test_escaped_open_paren() {
        let prose = Prose::new("\\(not a url\\)");
        let result = prose.render_optimistic(None);
        assert_eq!(result, "(not a url)");
    }

    #[test]
    fn test_escaped_double_backslash_still_single() {
        let prose = Prose::new("a\\\\b");
        let result = prose.render_optimistic(None);
        assert_eq!(result, "a\\b");
    }

    // ── Capability-aware degradation (Apple Terminal etc.) ───────────

    /// `<a href>` keeps OSC8 output when the terminal supports it.
    #[test]
    fn test_osc8_link_supported_emits_osc8() {
        let term = Terminal::builder().osc_link_support(true).build();
        let prose = Prose::new("<a href=\"https://example.com\">click here</a>");
        let result = prose.parse_tokens(Some(&term));
        assert!(
            result.contains("\x1b]8;;https://example.com\x1b\\"),
            "expected OSC8 open sequence, got: {:?}",
            result
        );
        assert!(result.contains("click here"));
        assert!(result.contains("\x1b]8;;\x1b\\"));
    }

    /// `<a href>` falls back to markdown when OSC8 is unsupported.
    #[test]
    fn test_osc8_link_unsupported_emits_markdown_fallback() {
        let term = Terminal::builder().osc_link_support(false).build();
        let prose = Prose::new("<a href=\"https://example.com\">click here</a>");
        let result = prose.parse_tokens(Some(&term));
        assert!(
            result.contains("[click here](https://example.com)"),
            "expected markdown link, got: {:?}",
            result
        );
        assert!(
            !result.contains("\x1b]8;;"),
            "must not emit OSC8 escape, got: {:?}",
            result
        );
    }

    /// When OSC8 is unsupported and the description contains a literal
    /// `]`, the markdown fallback must escape it so downstream CommonMark
    /// parsers do not mis-resolve the link (e.g. `[array[0]](url)` would
    /// otherwise be parsed as `[array[`/`0]](url)`).
    #[test]
    fn link_markdown_fallback_escapes_bracket_in_description() {
        let term = Terminal::builder().osc_link_support(false).build();
        let prose = Prose::new(r#"<a href="https://example.com">array[0]</a>"#);
        let result = prose.parse_tokens(Some(&term));
        assert!(
            result.contains(r"[array[0\]](https://example.com)"),
            "expected escaped `\\]` in markdown-fallback description; got: {:?}",
            result
        );
        assert!(
            !result.contains("\x1b]8;;"),
            "must not emit OSC8 escape, got: {:?}",
            result
        );
    }

    /// `<double-underline>` degrades to a straight underline when only
    /// the straight variant is supported.
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
        assert!(
            result.contains("\x1b[4m"),
            "expected straight underline open, got: {:?}",
            result
        );
        assert!(
            result.contains("important text"),
            "missing inner text, got: {:?}",
            result
        );
        assert!(
            !result.contains("\x1b[4:2m"),
            "must not emit double underline escape, got: {:?}",
            result
        );
        assert!(
            result.contains("\x1b[24m"),
            "expected underline close, got: {:?}",
            result
        );
    }

    /// `<double-underline>` is suppressed entirely when the terminal
    /// supports neither double nor straight underlines.
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
        assert_eq!(
            result, "important text",
            "expected plain text with no escapes, got: {:?}",
            result
        );
        assert!(
            !result.contains("\x1b["),
            "must not contain any SGR escape, got: {:?}",
            result
        );
    }

    /// `<a href="">link</a>` keeps the existing no-link behavior even
    /// when the terminal supports OSC8.
    #[test]
    fn test_osc8_link_empty_href_renders_inner_only() {
        let term = Terminal::builder().osc_link_support(true).build();
        let prose = Prose::new("<a href=\"\">click here</a>");
        let result = prose.parse_tokens(Some(&term));
        assert!(result.contains("click here"));
        assert!(!result.contains("\x1b]8;;"));
        assert!(!result.contains("[click here]"));
    }

    /// `block_tag_to_escape("double-underline")` returns
    /// `BlockTagAction::Suppress` when the terminal supports neither
    /// double nor straight underline. The parser routes this variant
    /// through the inner-content-only path.
    #[test]
    fn block_tag_action_suppress_for_double_underline_no_support() {
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

        let action = block_tag_to_escape("double-underline", &[], Some(&term));
        assert!(matches!(action, Some(BlockTagAction::Suppress)));
    }

    /// `block_tag_to_escape("a")` with an empty href returns
    /// `BlockTagAction::Suppress`, regardless of OSC8 support.
    #[test]
    fn block_tag_action_suppress_for_empty_href() {
        let term = Terminal::builder().osc_link_support(true).build();
        let attrs = vec![("href".to_string(), String::new())];
        let action = block_tag_to_escape("a", &attrs, Some(&term));
        assert!(matches!(action, Some(BlockTagAction::Suppress)));
    }

    /// `block_tag_to_escape("clipboard")` always suppresses; clipboard
    /// payloads are handled outside the SGR rendering pipeline.
    #[test]
    fn block_tag_action_suppress_for_clipboard() {
        let action = block_tag_to_escape("clipboard", &[], None);
        assert!(matches!(action, Some(BlockTagAction::Suppress)));
    }

    /// The atomic-token capability-aware helper must mirror the
    /// `<double-underline>` block-tag policy:
    /// - double supported → `\x1b[4:2m`
    /// - only straight supported → `\x1b[4m`
    /// - neither supported → `None`
    #[test]
    fn atomic_token_to_escape_with_term_degrades_double_underline() {
        // 1. No terminal context → optimistic double underline.
        assert_eq!(
            atomic_token_to_escape_with_term("double-underline", None).as_deref(),
            Some("\x1b[4:2m"),
        );

        // 2. Double underline supported → `\x1b[4:2m`.
        let double_ok = Terminal::builder()
            .underline_support(UnderlineSupport {
                straight: true,
                double: true,
                curly: false,
                dotted: false,
                dashed: false,
                colored: false,
            })
            .build();
        assert_eq!(
            atomic_token_to_escape_with_term("double-underline", Some(&double_ok)).as_deref(),
            Some("\x1b[4:2m"),
        );

        // 3. Only straight supported → degrade to `\x1b[4m`.
        let straight_only = Terminal::builder()
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
            atomic_token_to_escape_with_term("double-underline", Some(&straight_only)).as_deref(),
            Some("\x1b[4m"),
        );

        // 4. Neither supported → None (parser drops the token).
        let no_underline = Terminal::builder()
            .underline_support(UnderlineSupport {
                straight: false,
                double: false,
                curly: false,
                dotted: false,
                dashed: false,
                colored: false,
            })
            .build();
        assert!(
            atomic_token_to_escape_with_term("double-underline", Some(&no_underline)).is_none()
        );

        // 5. Non-degrading tokens always pass through unchanged.
        assert_eq!(
            atomic_token_to_escape_with_term("bold", Some(&no_underline)).as_deref(),
            Some("\x1b[1m"),
        );
        assert_eq!(
            atomic_token_to_escape_with_term("red", None).as_deref(),
            Some("\x1b[31m"),
        );
    }

    /// `{{double-underline}}` rendered through `parse_tokens` must
    /// follow the same capability-aware degradation as the block tag.
    #[test]
    fn atomic_double_underline_degrades_to_straight() {
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
        let prose = Prose::new("{{double-underline}}important text{{reset}}");
        let result = prose.parse_tokens(Some(&term));
        assert!(
            result.contains("\x1b[4m"),
            "expected straight underline SGR, got: {:?}",
            result,
        );
        assert!(
            !result.contains("\x1b[4:2m"),
            "must not emit double underline SGR, got: {:?}",
            result,
        );
        assert!(
            result.contains("important text"),
            "missing inner text, got: {:?}",
            result,
        );
    }

    /// `{{double-underline}}` must be suppressed entirely when neither
    /// double nor straight underline is supported.
    #[test]
    fn atomic_double_underline_suppressed_when_no_underline_support() {
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
        let prose = Prose::new("{{double-underline}}important text");
        let result = prose.parse_tokens(Some(&term));
        assert_eq!(
            result, "important text",
            "expected plain text with no escapes, got: {:?}",
            result
        );
        assert!(
            !result.contains("\x1b["),
            "must not contain any SGR escape, got: {:?}",
            result
        );
    }

    /// Fenced code blocks render with dim color and 2-space indent.
    #[test]
    fn code_block_renders_dim_and_indented() {
        let prose = Prose::new("<code-block lang=\"yaml\">key: value\nlist:\n  - a</code-block>");
        let result = prose.parse_tokens(None);
        assert!(result.contains("\x1b[2m  key: value\x1b[0m"));
        assert!(result.contains("\x1b[2m  list:\x1b[0m"));
        assert!(result.contains("\x1b[2m    - a\x1b[0m"));
    }

    /// Prose markup inside fenced code blocks is NOT parsed.
    #[test]
    fn fenced_code_block_preserves_literal_markup() {
        let prose = Prose::new("```\n**not bold**\n```");
        let result = prose.parse_tokens(None);
        assert!(result.contains("**not bold**"), "got: {:?}", result);
        assert!(!result.contains("\x1b[1m"));
    }
}
