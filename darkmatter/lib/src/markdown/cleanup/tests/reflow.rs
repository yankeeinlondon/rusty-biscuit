use super::*;
use biscuit_terminal::utils::UnicodeWidthStr;
use pulldown_cmark::TagEnd;

fn structural_fingerprint(content: &str) -> Vec<String> {
    let mut fingerprint = Vec::new();
    let mut list_depth = 0usize;
    let mut item_depth = 0usize;
    let mut blockquote_depth = 0usize;
    let mut explicit_paragraph_depth = 0usize;
    let mut implicit_paragraph = false;

    macro_rules! close_implicit_paragraph {
        () => {
            if implicit_paragraph {
                fingerprint.push(format!("end-paragraph:{list_depth}:{blockquote_depth}"));
                implicit_paragraph = false;
            }
        };
    }

    macro_rules! open_implicit_paragraph {
        () => {
            if item_depth > 0 && explicit_paragraph_depth == 0 && !implicit_paragraph {
                fingerprint.push(format!("start-paragraph:{list_depth}:{blockquote_depth}"));
                implicit_paragraph = true;
            }
        };
    }

    for event in Parser::new_ext(content, cleanup_parser_options()) {
        match event {
            Event::Start(Tag::List(start)) => {
                close_implicit_paragraph!();
                list_depth += 1;
                let kind = if start.is_some() { "ordered" } else { "unordered" };
                fingerprint.push(format!("start-list:{kind}:{list_depth}"));
            }
            Event::End(TagEnd::List(ordered)) => {
                close_implicit_paragraph!();
                let kind = if ordered { "ordered" } else { "unordered" };
                fingerprint.push(format!("end-list:{kind}:{list_depth}"));
                list_depth = list_depth.saturating_sub(1);
            }
            Event::Start(Tag::Item) => {
                item_depth += 1;
                fingerprint.push(format!("start-item:{list_depth}"));
            }
            Event::End(TagEnd::Item) => {
                close_implicit_paragraph!();
                fingerprint.push(format!("end-item:{list_depth}"));
                item_depth = item_depth.saturating_sub(1);
            }
            Event::Start(Tag::Paragraph) => {
                close_implicit_paragraph!();
                fingerprint.push(format!("start-paragraph:{list_depth}:{blockquote_depth}"));
                explicit_paragraph_depth += 1;
            }
            Event::End(TagEnd::Paragraph) => {
                fingerprint.push(format!("end-paragraph:{list_depth}:{blockquote_depth}"));
                explicit_paragraph_depth = explicit_paragraph_depth.saturating_sub(1);
            }
            Event::Start(Tag::BlockQuote(_)) => {
                close_implicit_paragraph!();
                blockquote_depth += 1;
                fingerprint.push(format!("start-blockquote:{blockquote_depth}"));
            }
            Event::End(TagEnd::BlockQuote(_)) => {
                close_implicit_paragraph!();
                fingerprint.push(format!("end-blockquote:{blockquote_depth}"));
                blockquote_depth = blockquote_depth.saturating_sub(1);
            }
            Event::Start(Tag::CodeBlock(kind)) => {
                close_implicit_paragraph!();
                fingerprint.push(format!("start-code:{kind:?}:{list_depth}"));
            }
            Event::End(TagEnd::CodeBlock) => {
                fingerprint.push(format!("end-code:{list_depth}"));
            }
            Event::Start(Tag::Table(_)) => {
                close_implicit_paragraph!();
                fingerprint.push(format!("start-table:{list_depth}"));
            }
            Event::End(TagEnd::Table) => fingerprint.push(format!("end-table:{list_depth}")),
            Event::Start(Tag::HtmlBlock) => {
                close_implicit_paragraph!();
                fingerprint.push(format!("start-html:{list_depth}"));
            }
            Event::End(TagEnd::HtmlBlock) => fingerprint.push(format!("end-html:{list_depth}")),
            Event::Text(_)
            | Event::Code(_)
            | Event::InlineHtml(_)
            | Event::InlineMath(_)
            | Event::DisplayMath(_)
            | Event::FootnoteReference(_)
            | Event::TaskListMarker(_) => open_implicit_paragraph!(),
            Event::Start(
                Tag::Emphasis
                | Tag::Strong
                | Tag::Strikethrough
                | Tag::Link { .. }
                | Tag::Image { .. },
            ) => open_implicit_paragraph!(),
            _ => {}
        }
    }
    fingerprint
}

fn assert_structure_preserved(source: &str, output: &str) {
    assert_eq!(
        structural_fingerprint(output),
        structural_fingerprint(source),
        "cleanup changed semantic block structure\nsource:\n{source}\noutput:\n{output}"
    );
}

fn assert_lines_within_width(content: &str, width: usize) {
    for line in content.lines() {
        assert!(
            UnicodeWidthStr::width(line) <= width,
            "line exceeded width {width}: {line:?}\n{content}"
        );
    }
}

    // ==================== Incidental Newline Tests ====================

    #[test]
    fn strip_incidental_newlines_drops_after_trailing_whitespace() {
        let content = "Wrapped line \ncontinues here";
        let stripped = strip_incidental_newlines(content);
        assert_eq!(stripped, "Wrapped line continues here");
    }

    #[test]
    fn strip_incidental_newlines_replaces_with_space_without_trailing_whitespace() {
        let content = "Wrapped line\ncontinues here";
        let stripped = strip_incidental_newlines(content);
        assert_eq!(stripped, "Wrapped line continues here");
    }

    #[test]
    fn strip_incidental_newlines_preserves_blank_lines() {
        let content = "First paragraph\n\nSecond paragraph\n\n\nThird paragraph";
        let stripped = strip_incidental_newlines(content);
        assert_eq!(stripped, content);
    }

    #[test]
    fn strip_incidental_newlines_preserves_fenced_code_verbatim() {
        let content = "Before\n```rust\nlet value = 1;\nlet other = 2;\n```\nAfter";
        let stripped = strip_incidental_newlines(content);
        assert_eq!(
            stripped,
            "Before\n```rust\nlet value = 1;\nlet other = 2;\n```\nAfter"
        );
    }

    #[test]
    fn strip_incidental_newlines_preserves_indented_code_verbatim() {
        let content = "Before\n\n    let value = 1;\n    let other = 2;\n\nAfter";
        let stripped = strip_incidental_newlines(content);
        assert_eq!(stripped, content);
    }

    #[test]
    fn strip_incidental_newlines_preserves_newline_inside_inline_code_span() {
        let content = "Before `code\nspan` after";
        let stripped = strip_incidental_newlines(content);
        assert_eq!(stripped, content);
    }

    #[test]
    fn strip_incidental_newlines_preserves_html_block_verbatim() {
        let content = "<div>\n<span>One</span>\n<span>Two</span>\n</div>\n\nAfter";
        let stripped = strip_incidental_newlines(content);
        assert_eq!(stripped, content);
    }

    #[test]
    fn strip_incidental_newlines_preserves_blockquote_prefix() {
        let content = "> Wrapped quote\n> continues here\n\nAfter";
        let stripped = strip_incidental_newlines(content);
        assert_eq!(stripped, "> Wrapped quote continues here\n\nAfter");
    }

    #[test]
    fn strip_incidental_newlines_preserves_table_rows() {
        let content = "| A | B |\n|---|---|\n| 1 | 2 |\n\nAfter";
        let stripped = strip_incidental_newlines(content);
        assert_eq!(stripped, content);
    }

    #[test]
    fn strip_incidental_newlines_preserves_list_marker() {
        let content = "- Wrapped item\ncontinues here\n- Second item";
        let stripped = strip_incidental_newlines(content);
        assert_eq!(stripped, "- Wrapped item continues here\n- Second item");
    }

    #[test]
    fn strip_incidental_newlines_collapses_explicit_list_continuation_without_layout_spaces() {
        let content = "- Ratified design: `claudine/features/2026-07-12-rendezvous-dashboard/spec.md`\n    (see the \"Decisions\" section, especially the implementation stamps).";
        let stripped = strip_incidental_newlines(content);

        assert_eq!(
            stripped,
            "- Ratified design: `claudine/features/2026-07-12-rendezvous-dashboard/spec.md` (see the \"Decisions\" section, especially the implementation stamps)."
        );
        assert_eq!(strip_incidental_newlines(&stripped), stripped);
    }

    #[test]
    fn strip_incidental_newlines_collapses_unordered_indent_variants() {
        let cases = [
            ("- Alpha\n  beta", "- Alpha beta"),
            ("* Alpha\n    beta", "* Alpha beta"),
            ("+ Alpha\n        beta", "+ Alpha beta"),
        ];

        for (content, expected) in cases {
            assert_eq!(strip_incidental_newlines(content), expected, "{content:?}");
        }
    }

    #[test]
    fn strip_incidental_newlines_collapses_ordered_marker_variants() {
        let cases = [
            ("1. Alpha\n   beta", "1. Alpha beta"),
            ("1. Alpha\n    beta", "1. Alpha beta"),
            ("10. Alpha\n    beta", "10. Alpha beta"),
            ("10) Alpha\n    beta", "10) Alpha beta"),
        ];

        for (content, expected) in cases {
            assert_eq!(strip_incidental_newlines(content), expected, "{content:?}");
        }
    }

    #[test]
    fn strip_incidental_newlines_collapses_task_item_continuations() {
        let cases = [
            ("- [ ] Alpha\n      beta", "- [ ] Alpha beta"),
            ("- [x] Alpha\n      beta", "- [x] Alpha beta"),
        ];

        for (content, expected) in cases {
            assert_eq!(strip_incidental_newlines(content), expected, "{content:?}");
        }
    }

    #[test]
    fn strip_incidental_newlines_collapses_nested_items_independently() {
        let content = "- Parent alpha\n    parent beta\n    - Child alpha\n      child beta\n- Sibling";
        let stripped = strip_incidental_newlines(content);

        assert_eq!(
            stripped,
            "- Parent alpha parent beta\n    - Child alpha child beta\n- Sibling"
        );
    }

    #[test]
    fn strip_incidental_newlines_removes_composite_blockquote_prefixes() {
        let content = "> - Alpha\n>   beta\n\n> > 10. Gamma\n> >     delta";
        let stripped = strip_incidental_newlines(content);

        assert_eq!(stripped, "> - Alpha beta\n\n> > 10. Gamma delta");
    }

    #[test]
    fn strip_incidental_newlines_preserves_second_paragraph_container() {
        let content = "- First paragraph.\n\n    Second alpha\n    second beta";
        let stripped = strip_incidental_newlines(content);

        assert_eq!(
            stripped,
            "- First paragraph.\n\n    Second alpha second beta"
        );
    }

    #[test]
    fn strip_incidental_newlines_preserves_list_hard_breaks() {
        let cases = [
            "- First line ends here.  \n  Second line remains.",
            "- First line ends here.\\\n  Second line remains.",
        ];

        for content in cases {
            assert_eq!(strip_incidental_newlines(content), content, "{content:?}");
        }
    }

    #[test]
    fn strip_incidental_newlines_preserves_list_child_blocks() {
        let cases = [
            "- Before\n\n    ```text\n    fenced one\n    fenced two\n    ```",
            "- Before\n\n    ```text\n    unclosed fenced one\n    unclosed fenced two",
            "- Before\n\n        indented one\n        indented two",
            "- Before\n\n    | A | B |\n    |---|---|\n    | 1 | 2 |",
            "- Before\n\n    <div>\n    html one\n    html two\n    </div>",
            "- Before\n\n    ::shell-block\n    echo one\n    echo two\n    ::end-block",
            "- Before\n\n    ::shell-block\n    echo unclosed one\n    echo unclosed two",
        ];

        for content in cases {
            assert_eq!(strip_incidental_newlines(content), content, "{content:?}");
        }
    }

    #[test]
    fn cleanup_preserves_list_semantic_structure_while_collapsing_soft_breaks() {
        let fixtures = [
            (
                "- Parent alpha\n    parent beta\n    - Child alpha\n      child beta\n- Sibling",
                "- Parent alpha parent beta\n    - Child alpha child beta\n\n- Sibling\n",
            ),
            (
                "- First paragraph.\n\n    Second alpha\n    second beta",
                "- First paragraph.\n    \n    Second alpha second beta\n",
            ),
            ("> - Alpha\n>   beta", "> - Alpha beta\n"),
            (
                "- Before\n\n    ```text\n    fenced one\n    fenced two\n    ```",
                "- Before\n    \n  ```text\n  fenced one\n  fenced two\n  ```\n",
            ),
        ];

        for (source, expected) in fixtures {
            let output = cleanup_content(source);
            assert_eq!(output, expected, "{source:?}");
            assert_structure_preserved(source, &output);
        }
    }

    #[test]
    fn cleanup_list_modes_share_soft_break_policy_without_changing_spacing_policy() {
        let source = "- Alpha\n    beta\n- Gamma\n    delta";

        assert_eq!(cleanup_content(source), "- Alpha beta\n- Gamma delta\n");
        assert_eq!(cleanup_content_compact(source), "- Alpha beta\n- Gamma delta\n");
        assert_eq!(cleanup_content_loose(source), "- Alpha beta\n\n- Gamma delta\n");
        assert_eq!(
            cleanup_content_with_indent_preserving_incidental(source, DEFAULT_INDENT),
            "- Alpha\n    beta\n\n- Gamma\n    delta\n"
        );
    }

    #[test]
    fn strip_incidental_newlines_normalizes_list_line_endings_identically() {
        let lf = "- Alpha\n    beta\n\nAfter";
        let expected = "- Alpha beta\n\nAfter";

        assert_eq!(strip_incidental_newlines(lf), expected);
        assert_eq!(strip_incidental_newlines(&lf.replace('\n', "\r\n")), expected);
        assert_eq!(strip_incidental_newlines(&lf.replace('\n', "\r")), expected);
    }

    #[test]
    fn strip_incidental_newlines_keeps_unicode_separator_parity_inside_lists() {
        let cases = [
            ("\u{6F22}", "\u{5B57}", "\u{6F22}\u{5B57}"),
            ("\u{0E20}\u{0E32}", "\u{0E44}\u{0E17}", "\u{0E20}\u{0E32}\u{0E44}\u{0E17}"),
            ("\u{D55C}", "\u{AE00}", "\u{D55C} \u{AE00}"),
            ("\u{1F600}", "\u{1F601}", "\u{1F600} \u{1F601}"),
            ("\u{6F22}\u{3002}", "\u{5B57}", "\u{6F22}\u{3002}\u{5B57}"),
            ("foo\u{200B}", "bar", "foo\u{200B}bar"),
        ];

        for (previous, next, expected) in cases {
            let outside = format!("{previous}\n{next}");
            let inside = format!("- {previous}\n  {next}");
            assert_eq!(strip_incidental_newlines(&outside), expected);
            assert_eq!(
                strip_incidental_newlines(&inside),
                format!("- {expected}")
            );
        }
    }

    #[test]
    fn strip_incidental_newlines_preserves_transclusion_directives() {
        let content = "::file ./README.md\n::code ./src/main.rs\n::shell echo hi\n::disclosure Details\n::details\nWrapped\ntext\n::end-disclosure\n";
        let stripped = strip_incidental_newlines(content);
        assert_eq!(
            stripped,
            "::file ./README.md\n::code ./src/main.rs\n::shell echo hi\n::disclosure Details\n::details\nWrapped text\n::end-disclosure\n"
        );
    }

    #[test]
    fn strip_incidental_newlines_preserves_shell_block_bodies() {
        let content = "::shell-block\necho a\necho b\n::end-block\n\nAfter";
        let stripped = strip_incidental_newlines(content);
        assert_eq!(stripped, content);
    }

    #[test]
    fn strip_incidental_newlines_preserves_blockquoted_shell_block_bodies() {
        let content = "> ::shell-block\n> echo a\n> echo b\n> ::end-block\n\nAfter";
        let stripped = strip_incidental_newlines(content);
        assert_eq!(stripped, content);
    }

    #[test]
    fn strip_incidental_newlines_treats_crlf_and_lf_as_newlines() {
        let content = "One\r\ntwo\nthree\r\n\nFour";
        let stripped = strip_incidental_newlines(content);
        assert_eq!(stripped, "One two three\n\nFour");
    }

    // ---- Structural safety: hard line breaks and setext headings ----

    #[test]
    fn strip_incidental_newlines_preserves_two_space_hard_break() {
        let content = "Roses are red  \nviolets are blue";
        let stripped = strip_incidental_newlines(content);
        assert_eq!(stripped, content);
    }

    #[test]
    fn strip_incidental_newlines_preserves_trailing_backslash_hard_break() {
        let content = "Roses are red\\\nviolets are blue";
        let stripped = strip_incidental_newlines(content);
        assert_eq!(stripped, content);
    }

    #[test]
    fn strip_incidental_newlines_collapses_single_trailing_space_not_hard_break() {
        // A single trailing space is incidental, not a hard break: the newline drops.
        let content = "Roses are red \nviolets are blue";
        let stripped = strip_incidental_newlines(content);
        assert_eq!(stripped, "Roses are red violets are blue");
    }

    #[test]
    fn strip_incidental_newlines_preserves_setext_h1_underline() {
        let content = "Heading\n===\nbody continues";
        let stripped = strip_incidental_newlines(content);
        assert_eq!(stripped, content);
    }

    #[test]
    fn strip_incidental_newlines_preserves_setext_h2_underline() {
        let content = "Heading\n---\nbody continues";
        let stripped = strip_incidental_newlines(content);
        assert_eq!(stripped, content);
    }

    // ---- Join separator by Unicode Script ----

    #[test]
    fn strip_incidental_newlines_joins_han_without_separator() {
        let content = "\u{6F22}\n\u{5B57}";
        let stripped = strip_incidental_newlines(content);
        assert_eq!(stripped, "\u{6F22}\u{5B57}");
    }

    #[test]
    fn strip_incidental_newlines_joins_thai_without_separator() {
        let content = "\u{0E20}\u{0E32}\u{0E29}\u{0E32}\n\u{0E44}\u{0E17}\u{0E22}";
        let stripped = strip_incidental_newlines(content);
        assert_eq!(stripped, "\u{0E20}\u{0E32}\u{0E29}\u{0E32}\u{0E44}\u{0E17}\u{0E22}");
    }

    #[test]
    fn strip_incidental_newlines_keeps_space_between_hangul() {
        // Hangul is space-delimited (excluded from the spaceless set).
        let content = "\u{D55C}\n\u{AE00}";
        let stripped = strip_incidental_newlines(content);
        assert_eq!(stripped, "\u{D55C} \u{AE00}");
    }

    #[test]
    fn strip_incidental_newlines_joins_han_to_latin_without_separator() {
        // Script transition: neutral reconstruction, never a pangu space.
        let content = "\u{6F22}\ntext";
        let stripped = strip_incidental_newlines(content);
        assert_eq!(stripped, "\u{6F22}text");
    }

    #[test]
    fn strip_incidental_newlines_keeps_space_between_emoji() {
        // Emoji are not letters, so they are not mis-joined as CJK.
        let content = "\u{1F600}\n\u{1F601}";
        let stripped = strip_incidental_newlines(content);
        assert_eq!(stripped, "\u{1F600} \u{1F601}");
    }

    #[test]
    fn strip_incidental_newlines_joins_after_cjk_punctuation() {
        // A line ending in `。` joined with a following ideograph: no separator.
        let content = "\u{6F22}\u{3002}\n\u{5B57}";
        let stripped = strip_incidental_newlines(content);
        assert_eq!(stripped, "\u{6F22}\u{3002}\u{5B57}");
    }

    #[test]
    fn strip_incidental_newlines_no_separator_around_zwsp() {
        let trailing = "foo\u{200B}\nbar";
        assert_eq!(strip_incidental_newlines(trailing), "foo\u{200B}bar");

        let leading = "foo\n\u{200B}bar";
        assert_eq!(strip_incidental_newlines(leading), "foo\u{200B}bar");
    }

    // ==================== Fixed-Width Reflow Tests ====================

    #[test]
    fn reflow_to_width_wraps_ascii_paragraphs_at_common_widths() {
        let content = "This paragraph has enough words to wrap cleanly at several common editor widths without needing to split any individual word.";

        for width in [20, 40, 80, 100] {
            let reflowed = cleanup_to_fixed_width(content, width);
            assert!(
                reflowed
                    .lines()
                    .all(|line| UnicodeWidthStr::width(line) <= width),
                "line exceeded width {width}:\n{reflowed}"
            );
        }
    }

    #[test]
    fn reflow_to_width_uses_unicode_display_columns() {
        let content = "café naïve résumé words";
        let reflowed = cleanup_to_fixed_width(content, 12);

        assert_eq!(reflowed, "café naïve\nrésumé words");
        assert!(reflowed.lines().all(|line| UnicodeWidthStr::width(line) <= 12));
    }

    #[test]
    fn reflow_to_width_keeps_long_words_on_one_line() {
        let content = "supercalifragilisticexpialidocious";
        let reflowed = cleanup_to_fixed_width(content, 10);

        assert_eq!(reflowed, content);
    }

    #[test]
    fn reflow_to_width_keeps_han_run_atomic_and_overflows() {
        // A contiguous Han run is a single whitespace-free token. v1 treats it
        // like a long URL: never split between ideographs, allowed to overflow.
        let content = "\u{6F22}".repeat(10);
        let width = 8;
        assert!(UnicodeWidthStr::width(content.as_str()) > width);

        let reflowed = cleanup_to_fixed_width(&content, width);

        assert_eq!(reflowed, content);
        assert!(!reflowed.contains('\n'));
    }

    #[test]
    fn reflow_to_width_keeps_thai_run_atomic_and_overflows() {
        // Thai is spaceless but non-Han; the atomic-overflow contract holds for
        // the whole curated spaceless set, not just CJK ideographs.
        let content = "\u{0E2A}\u{0E27}\u{0E31}\u{0E2A}\u{0E14}\u{0E35}\u{0E0A}\u{0E32}\u{0E27}\u{0E42}\u{0E25}\u{0E01}";
        let width = 6;
        assert!(UnicodeWidthStr::width(content) > width);

        let reflowed = cleanup_to_fixed_width(content, width);

        assert_eq!(reflowed, content);
        assert!(!reflowed.contains('\n'));
    }

    #[test]
    fn reflow_to_width_handles_empty_document() {
        assert_eq!(cleanup_to_fixed_width("", 80), "");
    }

    #[test]
    fn reflow_to_width_preserves_code_only_document() {
        let content = "```rust\nlet value = \"a very long line that stays untouched\";\n```\n";
        let reflowed = cleanup_to_fixed_width(content, 19);

        assert_eq!(reflowed, content);
    }

    #[test]
    fn reflow_to_width_preserves_mixed_list_marker_alignment() {
        let content = "- short marker has enough text to wrap around the target width\n1. ordered marker has enough text to wrap around the target width\n- [ ] task marker has enough text to wrap around the target width";
        let reflowed = cleanup_to_fixed_width(content, 28);

        assert_eq!(
            reflowed,
            "- short marker has enough\n  text to wrap around the\n  target width\n1. ordered marker has enough\n   text to wrap around the\n   target width\n- [ ] task marker has enough\n      text to wrap around\n      the target width"
        );
    }

    #[test]
    fn reflow_to_width_unwraps_complete_list_paragraph_before_wrapping() {
        let content = "- Alpha beta gamma delta\n    epsilon zeta eta theta.";
        let reflowed = cleanup_to_fixed_width(content, 24);

        assert_eq!(
            reflowed,
            "- Alpha beta gamma delta\n  epsilon zeta eta\n  theta."
        );
        assert_lines_within_width(&reflowed, 24);
    }

    #[test]
    fn reflow_to_width_derives_ordered_prefix_width_per_item() {
        let content = "9. Alpha beta gamma delta epsilon.\n10. Alpha beta gamma delta epsilon.";
        let reflowed = cleanup_to_fixed_width(content, 19);

        assert_eq!(
            reflowed,
            "9. Alpha beta gamma\n   delta epsilon.\n10. Alpha beta\n    gamma delta\n    epsilon."
        );
        assert_lines_within_width(&reflowed, 19);
    }

    #[test]
    fn reflow_to_width_aligns_checked_and_unchecked_task_items() {
        let content = "- [ ] Alpha beta gamma delta epsilon.\n- [x] Alpha beta gamma delta epsilon.";
        let reflowed = cleanup_to_fixed_width(content, 24);

        assert_eq!(
            reflowed,
            "- [ ] Alpha beta gamma\n      delta epsilon.\n- [x] Alpha beta gamma\n      delta epsilon."
        );
        assert_lines_within_width(&reflowed, 24);
    }

    #[test]
    fn reflow_to_width_uses_configured_nested_list_indentation() {
        let cases = [
            (
                2,
                "- Parent\n    - Alpha beta gamma delta epsilon.",
                "- Parent\n  - Alpha beta gamma\n    delta epsilon.\n",
            ),
            (
                4,
                "- Parent\n  - Child\n    - Alpha beta gamma delta epsilon.",
                "- Parent\n    - Child\n        - Alpha beta\n          gamma delta\n          epsilon.\n",
            ),
        ];

        for (indent, content, expected) in cases {
            let normalized = cleanup_content_with_indent(content, indent);
            let reflowed = reflow_to_width(&normalized, 22);

            assert_eq!(reflowed, expected, "indent {indent}, normalized {normalized:?}");
            assert_lines_within_width(&reflowed, 22);
        }
    }

    #[test]
    fn reflow_to_width_composes_blockquote_list_prefix_families() {
        let cases = [
            (
                "> - Alpha beta gamma delta epsilon.",
                "> - Alpha beta gamma\n>   delta epsilon.",
            ),
            (
                "> 10. Alpha beta gamma delta epsilon.",
                "> 10. Alpha beta gamma\n>     delta epsilon.",
            ),
            (
                "> - [x] Alpha beta gamma delta epsilon.",
                "> - [x] Alpha beta gamma\n>       delta epsilon.",
            ),
            (
                "> > - Alpha beta gamma delta epsilon.",
                "> > - Alpha beta gamma\n> >   delta epsilon.",
            ),
        ];

        for (content, expected) in cases {
            let reflowed = cleanup_to_fixed_width(content, 24);
            assert_eq!(reflowed, expected, "{content:?}");
            assert_lines_within_width(&reflowed, 24);
        }
    }

    #[test]
    fn reflow_to_width_keeps_first_and_subsequent_item_paragraphs_independent() {
        let content = "- First alpha beta gamma delta.\n\n    Second alpha beta gamma delta epsilon.";
        let reflowed = reflow_to_width(content, 24);

        assert_eq!(
            reflowed,
            "- First alpha beta gamma\n  delta.\n\n    Second alpha beta\n    gamma delta epsilon."
        );
        assert_lines_within_width(&reflowed, 24);
    }

    #[test]
    fn reflow_to_width_preserves_list_hard_breaks_and_container_prefixes() {
        let cases = [
            (
                "- Alpha beta gamma delta.  \n  Epsilon zeta eta theta iota.",
                "- Alpha beta gamma\n  delta.  \n  Epsilon zeta eta\n  theta iota.",
            ),
            (
                "- Alpha beta gamma delta.\\\n  Epsilon zeta eta theta iota.",
                "- Alpha beta gamma\n  delta.\\\n  Epsilon zeta eta\n  theta iota.",
            ),
        ];

        for (content, expected) in cases {
            let reflowed = reflow_to_width(content, 20);
            assert_eq!(reflowed, expected, "{content:?}");
            assert_lines_within_width(&reflowed, 20);
        }
    }

    #[test]
    fn reflow_to_width_keeps_long_token_intact_after_tight_prefix() {
        let content = "> - [ ] supercalifragilisticexpialidocious tail";
        let reflowed = reflow_to_width(content, 12);

        assert_eq!(
            reflowed,
            "> - [ ] supercalifragilisticexpialidocious\n>       tail"
        );
        assert!(UnicodeWidthStr::width(reflowed.lines().next().unwrap()) > 12);
        assert!(UnicodeWidthStr::width(reflowed.lines().nth(1).unwrap()) <= 12);
    }

    #[test]
    fn reflow_to_width_measures_wide_unicode_with_list_prefix() {
        let content = "- 漢字 alpha 漢字 beta";
        let reflowed = reflow_to_width(content, 12);

        assert_eq!(reflowed, "- 漢字 alpha\n  漢字 beta");
        assert_lines_within_width(&reflowed, 12);
    }

    #[test]
    fn reflow_to_width_preserves_list_child_blocks_byte_for_byte() {
        let cases = [
            "- Before\n\n    ```text\n    fenced one stays long\n    fenced two stays long\n    ```",
            "- Before\n\n        indented one stays long\n        indented two stays long",
            "- Before\n\n    | A | B |\n    |---|---|\n    | a long cell | another long cell |",
            "- Before\n\n    <div>\n    html one stays long\n    html two stays long\n    </div>",
            "- Before\n\n    ::shell-block\n    echo one stays long\n    echo two stays long\n    ::end-block",
        ];

        for content in cases {
            assert_eq!(reflow_to_width(content, 12), content, "{content:?}");
        }
    }

    #[test]
    fn cleanup_to_fixed_width_is_idempotent_for_nested_composite_lists() {
        let content = "> - Parent alpha beta gamma delta epsilon.\n>     - Child alpha beta gamma delta epsilon.";
        let once = cleanup_to_fixed_width(content, 24);
        let twice = cleanup_to_fixed_width(&once, 24);

        assert_eq!(
            once,
            "> - Parent alpha beta\n>   gamma delta epsilon.\n>     - Child alpha beta\n>       gamma delta\n>       epsilon."
        );
        assert_eq!(twice, once);
        assert_lines_within_width(&twice, 24);
    }

    #[test]
    fn reflow_to_width_preserves_table_rows() {
        let content = "| A | B |\n|---|---|\n| a very long cell | another long cell |\n";
        let reflowed = cleanup_to_fixed_width(content, 12);

        assert_eq!(reflowed, content);
    }

    #[test]
    fn reflow_to_width_keeps_blockquote_prefixes() {
        let content = "> This quote has enough text to wrap while preserving the quote prefix";
        let reflowed = cleanup_to_fixed_width(content, 24);

        assert_eq!(
            reflowed,
            "> This quote has enough\n> text to wrap while\n> preserving the quote\n> prefix"
        );
    }

    #[test]
    fn reflow_to_width_keeps_composite_blockquote_list_prefixes() {
        let content = "> - Alpha beta gamma delta epsilon zeta.";
        let reflowed = cleanup_to_fixed_width(content, 24);

        assert_eq!(
            reflowed,
            "> - Alpha beta gamma\n>   delta epsilon zeta."
        );
        assert_lines_within_width(&reflowed, 24);
    }

    #[test]
    fn reflow_to_width_preserves_transclusion_directives() {
        let content = "::file ./a/path/that/should/not/be/wrapped.md\n::code ./src/main.rs\n";
        let reflowed = cleanup_to_fixed_width(content, 12);

        assert_eq!(reflowed, content);
    }

    #[test]
    fn reflow_to_width_preserves_shell_block_bodies() {
        let content = "::shell-block\necho a\necho b\n::end-block\n";
        let reflowed = cleanup_to_fixed_width(content, 8);

        assert_eq!(reflowed, content);
    }

    #[test]
    fn reflow_to_width_preserves_html_blocks() {
        let content = "<div class=\"options\">\n<span>Very long HTML content stays untouched.</span>\n</div>\n";
        let reflowed = cleanup_to_fixed_width(content, 12);

        assert_eq!(reflowed, content);
    }

    #[test]
    #[should_panic(expected = "fixed-width cleanup requires a width greater than 0")]
    fn cleanup_to_fixed_width_rejects_zero_width() {
        let _ = cleanup_to_fixed_width("content", 0);
    }
