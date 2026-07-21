use super::*;

#[test]
fn test_compose_returns_unchanged_document() {
    let content = "# Hello\n\nWorld";
    let md: Markdown = content.into();

    let (composed, _report) = md.compose().unwrap();

    // Content should still match (cleanup may add spacing)
    assert!(composed.content().contains("Hello"));
    assert!(composed.content().contains("World"));
}

#[test]
fn test_compose_mut_modifies_in_place() {
    let content = "# Hello\n\nWorld";
    let mut md: Markdown = content.into();

    let _report = md.compose_mut().unwrap();

    assert!(md.content().contains("Hello"));
    assert!(md.content().contains("World"));
}

#[test]
fn test_compose_with_custom_options() {
    let content = "# Hello\n\nWorld";
    let md: Markdown = content.into();

    let options = ComposeOptions::new()
        .disable(ComposeOperation::Cleanup)
        .disable(ComposeOperation::Normalization);

    let (composed, report) = md.compose_with(options).unwrap();

    // With cleanup disabled, content should be unchanged
    assert_eq!(composed.content(), md.content());
    assert!(!report.cleanup_changed);
}

#[test]
fn test_compose_cleanup_stage() {
    // Content without proper spacing
    let content = "# Header\nParagraph";
    let md: Markdown = content.into();

    let options = ComposeOptions::new().only(&[ComposeOperation::Cleanup]);

    let (composed, report) = md.compose_with(options).unwrap();

    // Cleanup should add blank line between header and paragraph
    assert!(composed.content().contains("\n\n"));
    assert!(report.cleanup_changed);
}

#[test]
fn test_compose_cleanup_strips_incidental_newlines_by_default() {
    let content = "This paragraph was wrapped\nat a fixed column.";
    let md: Markdown = content.into();

    let options = ComposeOptions::new().only(&[ComposeOperation::Cleanup]);

    let (composed, report) = md.compose_with(options).unwrap();

    assert_eq!(
        composed.content(),
        "This paragraph was wrapped at a fixed column.\n"
    );
    assert!(report.cleanup_changed);
}

#[test]
fn test_compose_cleanup_can_preserve_incidental_newlines() {
    let content = "This paragraph keeps\nits single newline.\n";
    let md: Markdown = content.into();

    let options = ComposeOptions::new()
        .only(&[ComposeOperation::Cleanup])
        .with_incidental_newline_mode(crate::markdown::cleanup::IncidentalNewlineMode::Preserve);

    let (composed, report) = md.compose_with(options).unwrap();

    assert_eq!(composed.content(), content);
    assert!(!report.cleanup_changed);
}

#[test]
fn test_compose_cleanup_fixed_width_reflows_prose() {
    let content = "This paragraph starts with incidental wrapping\nand should be reflowed into lines that fit.";
    let md: Markdown = content.into();

    let options = ComposeOptions::new()
        .only(&[ComposeOperation::Cleanup])
        .with_fixed_width(40);

    let (composed, report) = md.compose_with(options).unwrap();

    assert!(report.cleanup_changed);
    assert!(composed.content().contains('\n'));
    for line in composed.content().lines() {
        assert!(
            UnicodeWidthStr::width(line) <= 40,
            "line exceeded fixed width: {line:?}"
        );
    }
}

#[test]
fn test_compose_cleanup_fixed_width_forces_strip_over_preserve() {
    // `fixed_width` must reflow canonical unwrapped prose, so it overrides
    // `Preserve` and strips the source's incidental newline before wrapping.
    let content = "Short first line.\nShort second line.\n";
    let md: Markdown = content.into();

    let options = ComposeOptions::new()
        .only(&[ComposeOperation::Cleanup])
        .with_incidental_newline_mode(crate::markdown::cleanup::IncidentalNewlineMode::Preserve)
        .with_fixed_width(80);

    let (composed, report) = md.compose_with(options).unwrap();

    // The two source lines collapse to a single line that fits within 80 columns.
    assert_eq!(composed.content(), "Short first line. Short second line.\n");
    assert!(report.cleanup_changed);
}

#[test]
fn test_compose_cleanup_list_modes_match_direct_library_cleanup() {
    use crate::markdown::cleanup::{
        cleanup_content_with_indent_preserving_incidental, reflow_to_width,
        IncidentalNewlineMode, DEFAULT_INDENT,
    };

    let source = "- Alpha beta gamma delta\n    epsilon zeta eta theta.";

    let strip_options = ComposeOptions::new().only(&[ComposeOperation::Cleanup]);
    let (stripped, _) = Markdown::from(source).compose_with(strip_options).unwrap();
    let mut direct: Markdown = source.into();
    direct.cleanup();
    assert_eq!(stripped.content(), direct.content());
    assert_eq!(stripped.content(), "- Alpha beta gamma delta epsilon zeta eta theta.\n");

    let preserve_options = ComposeOptions::new()
        .only(&[ComposeOperation::Cleanup])
        .with_incidental_newline_mode(IncidentalNewlineMode::Preserve);
    let (preserved, _) = Markdown::from(source).compose_with(preserve_options).unwrap();
    let expected_preserved = cleanup_content_with_indent_preserving_incidental(
        source,
        DEFAULT_INDENT,
    );
    assert_eq!(preserved.content(), expected_preserved);
    assert_eq!(preserved.content(), "- Alpha beta gamma delta\n    epsilon zeta eta theta.\n");

    let fixed_options = ComposeOptions::new()
        .only(&[ComposeOperation::Cleanup])
        .with_incidental_newline_mode(IncidentalNewlineMode::Preserve)
        .with_fixed_width(24);
    let (fixed, _) = Markdown::from(source).compose_with(fixed_options).unwrap();
    let expected_fixed = reflow_to_width(direct.content(), 24);
    assert_eq!(fixed.content(), expected_fixed);
    assert_eq!(
        fixed.content(),
        "- Alpha beta gamma delta\n  epsilon zeta eta\n  theta.\n"
    );
    for line in fixed.content().lines() {
        assert!(UnicodeWidthStr::width(line) <= 24, "line exceeded width: {line:?}");
    }
}

#[test]
fn test_compose_cleanup_preserves_nested_lists_inside_blockquotes() {
    use crate::markdown::cleanup::{
        cleanup_content, cleanup_content_with_indent, reflow_to_width,
    };

    let fixtures = [
        concat!(
            "> - Parent alpha beta gamma delta epsilon.\n",
            ">   - Child alpha beta gamma delta epsilon.\n"
        ),
        concat!(
            "> 1. Parent alpha beta gamma delta epsilon.\n",
            ">    1. Child alpha beta gamma delta epsilon.\n"
        ),
        concat!(
            "> - [ ] Parent alpha beta gamma delta epsilon.\n",
            ">   - [x] Child alpha beta gamma delta epsilon.\n"
        ),
    ];

    for source in fixtures {
        let default_options = ComposeOptions::new().only(&[ComposeOperation::Cleanup]);
        let (default, _) = Markdown::from(source)
            .compose_with(default_options)
            .unwrap();
        let direct_default = cleanup_content(source);
        assert_eq!(default.content(), direct_default);

        let configured_options = ComposeOptions::new()
            .only(&[ComposeOperation::Cleanup])
            .with_indent_size(2);
        let (configured, _) = Markdown::from(source)
            .compose_with(configured_options)
            .unwrap();
        assert_eq!(configured.content(), cleanup_content_with_indent(source, 2));

        let fixed_options = ComposeOptions::new()
            .only(&[ComposeOperation::Cleanup])
            .with_fixed_width(24);
        let (fixed, _) = Markdown::from(source).compose_with(fixed_options).unwrap();
        assert_eq!(fixed.content(), reflow_to_width(&direct_default, 24));
    }
}

#[test]
fn test_compose_indent_eight_matches_exact_library_cleanup() {
    use crate::markdown::cleanup::{cleanup_content_with_indent, reflow_to_width};

    let source = concat!(
        "- Parent alpha beta gamma delta epsilon.\n",
        "  - Child alpha beta gamma delta epsilon.\n",
        "    - Grandchild alpha beta gamma delta epsilon.\n",
        "\n",
        "> - [ ] Quote task parent alpha beta gamma delta.\n",
        ">   - [x] Quote task child alpha beta gamma delta epsilon.\n"
    );
    let options = ComposeOptions::new()
        .only(&[ComposeOperation::Cleanup])
        .with_indent_size(8)
        .with_fixed_width(30);
    let (composed, _) = Markdown::from(source).compose_with(options.clone()).unwrap();

    let cleaned = cleanup_content_with_indent(source, 8);
    let expected = reflow_to_width(&cleaned, 30);
    assert_eq!(composed.content(), expected);
    assert!(expected.contains("\n        -    Child"));
    assert!(expected.contains("\n                - Grandchild"));
    assert!(expected.contains("\n>         - [x] Quote task"));
    for line in expected.lines() {
        assert!(UnicodeWidthStr::width(line) <= 30, "line exceeded width: {line:?}");
    }

    let (second, _) = Markdown::from(expected.as_str()).compose_with(options).unwrap();
    assert_eq!(second.content(), expected);
}

#[test]
fn test_compose_cleanup_preserves_additional_paragraphs_inside_blockquoted_items() {
    use crate::markdown::cleanup::{
        cleanup_content_with_indent, cleanup_content_with_indent_compact,
        cleanup_content_with_indent_loose, reflow_to_width, ListSpacingMode,
    };

    let fixtures = [
        (
            "> - Parent first paragraph.\n>\n>   Second paragraph alpha beta gamma delta.\n>\n>   - Child item alpha beta.\n",
            ListSpacingMode::Normal,
            4,
        ),
        (
            "> > 1. Parent first paragraph.\n> >\n> >    Second paragraph alpha beta gamma delta.\n> >\n> >    - Child item alpha beta.\n",
            ListSpacingMode::Compact,
            2,
        ),
        (
            "> - [ ] Parent first paragraph.\n>\n>   Second café 🙂 alpha beta gamma delta.\n>\n>   1. Child item alpha beta.\n",
            ListSpacingMode::Loose,
            4,
        ),
    ];

    for (source, spacing, indent) in fixtures {
        let direct = match spacing {
            ListSpacingMode::Normal => cleanup_content_with_indent(source, indent),
            ListSpacingMode::Compact => cleanup_content_with_indent_compact(source, indent),
            ListSpacingMode::Loose => cleanup_content_with_indent_loose(source, indent),
        };
        let expected = reflow_to_width(&direct, 24);
        let options = ComposeOptions::new()
            .only(&[ComposeOperation::Cleanup])
            .with_list_spacing(spacing)
            .with_indent_size(indent)
            .with_fixed_width(24);
        let (first, _) = Markdown::from(source).compose_with(options.clone()).unwrap();
        assert_eq!(first.content(), expected);

        let (second, _) = Markdown::from(first.content()).compose_with(options).unwrap();
        assert_eq!(second.content(), expected);
    }
}

#[test]
fn test_compose_cleanup_preserves_markers_in_protected_bodies() {
    use crate::markdown::cleanup::{cleanup_content, reflow_to_width};

    let source = concat!(
        "<div>\n",
        "* literal html\n",
        "</div>\n",
        "\n",
        "::shell-block\n",
        "* literal shell\n",
        "::end-block\n",
        "\n",
        "- Actual item.\n"
    );
    let expected = reflow_to_width(&cleanup_content(source), 24);
    let options = ComposeOptions::new()
        .only(&[ComposeOperation::Cleanup])
        .with_fixed_width(24);

    let (first, _) = Markdown::from(source).compose_with(options.clone()).unwrap();
    assert_eq!(first.content(), expected);
    assert!(first.content().contains("* literal html"));
    assert!(first.content().contains("* literal shell"));
    assert!(first.content().contains("- Actual item."));

    let (second, _) = Markdown::from(first.content()).compose_with(options).unwrap();
    assert_eq!(second.content(), expected);
}

#[test]
fn test_compose_cleanup_preserves_ten_digit_prose_boundary() {
    use crate::markdown::cleanup::{cleanup_content, ListSpacingMode};

    let source = concat!(
        "123456789. nine-digit item\n",
        "\n",
        "- first unordered item\n",
        "\n",
        "1) first ordered item\n",
        "\n",
        "1234567890) ten-digit prose\n",
        "\n",
        "+ second unordered item\n",
        "\n",
        "2) second ordered item\n"
    );
    let expected = cleanup_content(source);
    let options = ComposeOptions::new()
        .only(&[ComposeOperation::Cleanup])
        .with_list_spacing(ListSpacingMode::Normal);

    let (first, _) = Markdown::from(source).compose_with(options.clone()).unwrap();
    assert_eq!(first.content(), expected);
    assert!(first.content().contains("1234567890) ten-digit prose\n\n+ second unordered"));

    let (second, _) = Markdown::from(first.content()).compose_with(options).unwrap();
    assert_eq!(second.content(), expected);
}

#[test]
fn slow_compose_cleanup_preserves_quoted_marker_looking_indented_code() {
    use crate::markdown::cleanup::{
        cleanup_content, cleanup_content_with_indent, reflow_to_width,
    };

    let fixtures = [
        "> - Parent.\n>\n>       - literal code\n>\n> - Later sibling.\n",
        "> 1. Parent.\n>\n>       1. literal code\n>\n> 2. Later sibling.\n",
        "> > - Parent.\n> >\n> >       - literal code\n> >\n> > - Later sibling.\n",
        "> > 1. Parent.\n> >\n> >       1. literal code\n> >\n> > 2. Later sibling.\n",
    ];

    for source in fixtures {
        let direct_default = cleanup_content(source);
        let (default, _) = Markdown::from(source)
            .compose_with(ComposeOptions::new().only(&[ComposeOperation::Cleanup]))
            .unwrap();
        assert_eq!(default.content(), direct_default);

        let (configured, _) = Markdown::from(source)
            .compose_with(
                ComposeOptions::new()
                    .only(&[ComposeOperation::Cleanup])
                    .with_indent_size(4),
            )
            .unwrap();
        assert_eq!(configured.content(), cleanup_content_with_indent(source, 4));

        let (fixed, _) = Markdown::from(source)
            .compose_with(
                ComposeOptions::new()
                    .only(&[ComposeOperation::Cleanup])
                    .with_fixed_width(24),
            )
            .unwrap();
        assert_eq!(fixed.content(), reflow_to_width(&direct_default, 24));
        let (fixed_second, _) = Markdown::from(fixed.content())
            .compose_with(
                ComposeOptions::new()
                    .only(&[ComposeOperation::Cleanup])
                    .with_fixed_width(24),
            )
            .unwrap();
        assert_eq!(fixed_second.content(), fixed.content());
    }
}

#[test]
fn test_compose_cleanup_fixed_width_keeps_reference_definitions_intact() {
    use crate::markdown::cleanup::reflow_to_width;

    let source = concat!(
        "- Before [label][ref] alpha beta gamma delta.\n",
        "\n",
        "[ref]: https://example.com/a/very/long/path \"A descriptive title\"\n"
    );

    let options = ComposeOptions::new()
        .only(&[ComposeOperation::Cleanup])
        .with_fixed_width(24);
    let (fixed, _) = Markdown::from(source).compose_with(options).unwrap();

    let mut direct: Markdown = source.into();
    direct.cleanup();
    assert_eq!(fixed.content(), reflow_to_width(direct.content(), 24));
    assert_eq!(
        fixed.content(),
        concat!(
            "- Before [label][ref]\n",
            "  alpha beta gamma\n",
            "  delta.\n",
            "\n",
            "[ref]: https://example.com/a/very/long/path \"A descriptive title\"\n"
        )
    );
}

#[test]
fn test_compose_normalization_stage_no_change() {
    let content = "# Hello\n\n## World";
    let md: Markdown = content.into();

    let options = ComposeOptions::new().only(&[ComposeOperation::Normalization]);

    let (_, report) = md.compose_with(options).unwrap();

    // Well-formed document, no normalization needed
    assert!(report.normalization_report.is_none());
}

#[test]
fn test_compose_preserves_frontmatter() {
    let content = "---\ntitle: Test\n---\n# Hello";
    let md: Markdown = content.into();

    let (composed, _) = md.compose().unwrap();

    let title: Option<String> = composed.fm_get("title").unwrap();
    assert_eq!(title, Some("Test".to_string()));
}

#[test]
fn test_compose_report_summary() {
    let content = "# Header\nParagraph";
    let md: Markdown = content.into();

    let (_, report) = md.compose().unwrap();

    // Should have a meaningful summary
    let summary = report.summary();
    assert!(!summary.is_empty());
}

#[test]
fn test_compose_report_has_changes() {
    let content = "# Header\nParagraph";
    let md: Markdown = content.into();

    let (_, report) = md.compose().unwrap();

    // Cleanup should have made changes
    assert!(report.has_changes());
    assert!(report.cleanup_changed);
}

#[test]
fn test_compose_stages_all_disabled() {
    let content = "# Header\nParagraph";
    let md: Markdown = content.into();

    let options = ComposeOptions::new().only(&[]);

    let (composed, report) = md.compose_with(options).unwrap();

    // No changes should be made
    assert_eq!(composed.content(), md.content());
    assert!(!report.has_changes());
}

#[test]
fn test_compose_stages_run_in_order() {
    let content = "---\nshow: false\n---\nBefore\n\n::block when=\"show\"\n\n::shell echo hidden\n\n::end-block\n\n::code ./example.rs\nAfter";
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("root.md");
    let code = dir.path().join("example.rs");
    std::fs::write(&root, content).unwrap();
    std::fs::write(&code, "fn main() {}\n").unwrap();

    let options = ComposeOptions::new()
        .with_source_file(&root)
        .with_shell_policy_root(dir.path())
        .only(&[
            ComposeOperation::PageBlocks,
            ComposeOperation::ShellExpansion,
            ComposeOperation::CodeTransclusion,
            ComposeOperation::Cleanup,
        ]);

    let (composed, report) = Markdown::try_from(root.as_path())
        .unwrap()
        .compose_with(options)
        .unwrap();

    assert!(!composed.content().contains("hidden"));
    assert!(composed.content().contains("```rs"));
    assert!(composed.content().contains("\n\n```rs"));
    assert_eq!(report.page_blocks_skipped, 1);
    assert_eq!(report.shell_expansions_applied, 0);
    assert_eq!(report.transclusions_applied, 1);
    assert!(report.cleanup_changed);
}

#[test]
fn test_compose_with_external_state() {
    let content = "# Hello";
    let md: Markdown = content.into();

    let options =
        ComposeOptions::new().with_external_state(serde_json::json!({"key": "value"}));

    // Should not fail
    let result = md.compose_with(options);
    assert!(result.is_ok());
}

#[test]
fn test_compose_options_context_captured() {
    let options = ComposeOptions::new();
    let ctx = options.context();

    // Context should have been captured
    assert!(!ctx.today().is_empty());
    assert!(!ctx.year().is_empty());
}

#[test]
fn test_compose_fail_fast_false_continues_on_warning() {
    // Document that would cause normalization warning
    // (but for now normalization doesn't fail with None target)
    let content = "# Hello";
    let md: Markdown = content.into();

    let options = ComposeOptions::new().with_fail_fast(false);

    let result = md.compose_with(options);
    assert!(result.is_ok());
}

#[test]
fn test_effective_state_available_to_stages() {
    let content = "---\nkey: value\n---\n# Hello";
    let md: Markdown = content.into();

    // External state should merge with frontmatter
    let options =
        ComposeOptions::new().with_external_state(serde_json::json!({"external": "data"}));

    let result = md.compose_with(options);
    assert!(result.is_ok());
}

// ============================================
// Replacement stage integration tests
// ============================================

#[test]
fn test_replacement_stage_with_frontmatter() {
    let content = "---\nreplace:\n  foo: bar\n---\n# Hello foo\n\nContent with foo here.";
    let md: Markdown = content.into();

    let options = ComposeOptions::new().only(&[ComposeOperation::TextReplacement]);

    let (composed, report) = md.compose_with(options).unwrap();

    assert!(composed.content().contains("Hello bar"));
    assert!(composed.content().contains("Content with bar here."));
    assert_eq!(report.replacements_applied, 2);
}

#[test]
fn test_replacement_stage_overlap_resolution() {
    // Longest key wins: "foobar" before "foo"
    let content = "---\nreplace:\n  foo: short\n  foobar: long\n---\nfoobar and foo";
    let md: Markdown = content.into();

    let options = ComposeOptions::new().only(&[ComposeOperation::TextReplacement]);

    let (composed, report) = md.compose_with(options).unwrap();

    assert_eq!(composed.content(), "long and short");
    assert_eq!(report.replacements_applied, 2);
}

#[test]
fn test_replacement_stage_non_recursive() {
    // Replacement output should NOT be re-scanned
    let content = "---\nreplace:\n  foo: foobar\n  foobar: baz\n---\nfoo";
    let md: Markdown = content.into();

    let options = ComposeOptions::new().only(&[ComposeOperation::TextReplacement]);

    let (composed, report) = md.compose_with(options).unwrap();

    // "foo" -> "foobar" but NOT -> "baz"
    assert_eq!(composed.content(), "foobar");
    assert_eq!(report.replacements_applied, 1);
}

#[test]
fn test_replacement_stage_null_value() {
    let content = "---\nreplace:\n  remove_me: null\n---\nHello remove_me world";
    let md: Markdown = content.into();

    let options = ComposeOptions::new().only(&[ComposeOperation::TextReplacement]);

    let (composed, report) = md.compose_with(options).unwrap();

    assert_eq!(composed.content(), "Hello  world");
    assert_eq!(report.replacements_applied, 1);
}

#[test]
fn test_replacement_stage_number_value() {
    let content = "---\nreplace:\n  VERSION: 42\n---\nVersion: VERSION";
    let md: Markdown = content.into();

    let options = ComposeOptions::new().only(&[ComposeOperation::TextReplacement]);

    let (composed, report) = md.compose_with(options).unwrap();

    assert_eq!(composed.content(), "Version: 42");
    assert_eq!(report.replacements_applied, 1);
}

#[test]
fn test_replacement_stage_no_replace_in_frontmatter() {
    let content = "---\ntitle: Test\n---\n# Hello foo";
    let md: Markdown = content.into();

    let options = ComposeOptions::new().only(&[ComposeOperation::TextReplacement]);

    let (composed, report) = md.compose_with(options).unwrap();

    // No changes when no replace map
    assert_eq!(composed.content(), md.content());
    assert_eq!(report.replacements_applied, 0);
}

#[test]
fn test_replacement_stage_with_external_state() {
    // External state can provide replace map
    let content = "# Hello foo";
    let md: Markdown = content.into();

    let options = ComposeOptions::new()
        .only(&[ComposeOperation::TextReplacement])
        .with_external_state(serde_json::json!({
            "replace": {"foo": "bar"}
        }));

    let (composed, report) = md.compose_with(options).unwrap();

    assert_eq!(composed.content(), "# Hello bar");
    assert_eq!(report.replacements_applied, 1);
}

#[test]
fn test_replacement_stage_frontmatter_overrides_external_with_deep_merge() {
    // External state replace map is merged as defaults; frontmatter wins on conflicts.
    let content = "---\nreplace:\n  foo: from_fm\n  baz: qux\n---\nfoo baz";
    let md: Markdown = content.into();

    let options = ComposeOptions::new()
        .only(&[ComposeOperation::TextReplacement])
        .with_external_state(serde_json::json!({
            "replace": {"foo": "from_external"}
        }));

    let (composed, report) = md.compose_with(options).unwrap();

    assert_eq!(composed.content(), "from_fm qux");
    assert_eq!(report.replacements_applied, 2);
}

#[test]
fn test_replacement_stage_report_summary() {
    let content = "---\nreplace:\n  a: b\n---\na a a";
    let md: Markdown = content.into();

    let options = ComposeOptions::new().only(&[ComposeOperation::TextReplacement]);

    let (_, report) = md.compose_with(options).unwrap();

    assert_eq!(report.replacements_applied, 3);
    let summary = report.summary();
    assert!(summary.contains("3 replacement(s)"));
}

#[test]
fn test_replacement_then_cleanup() {
    // Test that replacement runs before cleanup
    let content = "---\nreplace:\n  foo: bar\n---\n# Header\nfoo here";
    let md: Markdown = content.into();

    // Enable both replacement and cleanup
    let options = ComposeOptions::new()
        .only(&[ComposeOperation::TextReplacement, ComposeOperation::Cleanup]);

    let (composed, report) = md.compose_with(options).unwrap();

    // Replacement happened
    assert!(composed.content().contains("bar here"));
    assert_eq!(report.replacements_applied, 1);

    // Cleanup added blank line
    assert!(composed.content().contains("\n\n"));
    assert!(report.cleanup_changed);
}

// ============================================
// Interpolation stage integration tests
// ============================================

#[test]
fn test_interpolation_simple_variable() {
    let content = "---\nname: Alice\n---\n# Hello {{ name }}!";
    let md: Markdown = content.into();

    let options = ComposeOptions::new().only(&[ComposeOperation::Interpolation]);

    let (composed, report) = md.compose_with(options).unwrap();

    assert_eq!(composed.content(), "# Hello Alice!");
    assert_eq!(report.interpolations_applied, 1);
}

#[test]
fn test_interpolation_nested_variable() {
    let content = "---\nuser:\n  name: Bob\n---\nWelcome {{ user.name }}";
    let md: Markdown = content.into();

    let options = ComposeOptions::new().only(&[ComposeOperation::Interpolation]);

    let (composed, report) = md.compose_with(options).unwrap();

    assert_eq!(composed.content(), "Welcome Bob");
    assert_eq!(report.interpolations_applied, 1);
}

#[test]
fn test_interpolation_missing_variable() {
    let content = "---\ntitle: Test\n---\nHello {{ missing }}!";
    let md: Markdown = content.into();

    let options = ComposeOptions::new().only(&[ComposeOperation::Interpolation]);

    let (composed, report) = md.compose_with(options).unwrap();

    // Missing variables become empty string
    assert_eq!(composed.content(), "Hello !");
    assert_eq!(report.interpolations_applied, 1);
}

#[test]
fn test_interpolation_fallback_uses_default() {
    let content = "---\ntitle: Test\n---\nColor: {{ color || \"unknown\" }}";
    let md: Markdown = content.into();

    let options = ComposeOptions::new().only(&[ComposeOperation::Interpolation]);

    let (composed, report) = md.compose_with(options).unwrap();

    assert_eq!(composed.content(), "Color: unknown");
    assert_eq!(report.interpolations_applied, 1);
}

#[test]
fn test_interpolation_fallback_missing_variable_renders_default() {
    let content = "---\ntitle: Test\n---\nValue: {{ missing || \"default\" }}";
    let md: Markdown = content.into();

    let options = ComposeOptions::new().only(&[ComposeOperation::Interpolation]);

    let (composed, report) = md.compose_with(options).unwrap();

    assert_eq!(composed.content(), "Value: default");
    assert_eq!(report.interpolations_applied, 1);
}

#[test]
fn test_interpolation_fallback_uses_primary() {
    let content = "---\ncolor: blue\n---\nColor: {{ color || \"unknown\" }}";
    let md: Markdown = content.into();

    let options = ComposeOptions::new().only(&[ComposeOperation::Interpolation]);

    let (composed, report) = md.compose_with(options).unwrap();

    assert_eq!(composed.content(), "Color: blue");
    assert_eq!(report.interpolations_applied, 1);
}

#[test]
fn test_interpolation_ternary_true() {
    let content = "---\nactive: true\n---\nStatus: {{ active ? \"on\" : \"off\" }}";
    let md: Markdown = content.into();

    let options = ComposeOptions::new().only(&[ComposeOperation::Interpolation]);

    let (composed, report) = md.compose_with(options).unwrap();

    assert_eq!(composed.content(), "Status: on");
    assert_eq!(report.interpolations_applied, 1);
}

#[test]
fn test_interpolation_ternary_false() {
    let content = "---\nactive: false\n---\nStatus: {{ active ? \"on\" : \"off\" }}";
    let md: Markdown = content.into();

    let options = ComposeOptions::new().only(&[ComposeOperation::Interpolation]);

    let (composed, report) = md.compose_with(options).unwrap();

    assert_eq!(composed.content(), "Status: off");
    assert_eq!(report.interpolations_applied, 1);
}

#[test]
fn test_interpolation_comparison_equal() {
    let content = "---\ncount: 5\n---\n{{ count == 5 ? \"five\" : \"other\" }}";
    let md: Markdown = content.into();

    let options = ComposeOptions::new().only(&[ComposeOperation::Interpolation]);

    let (composed, report) = md.compose_with(options).unwrap();

    assert_eq!(composed.content(), "five");
    assert_eq!(report.interpolations_applied, 1);
}

#[test]
fn test_interpolation_comparison_greater_than() {
    let content = "---\ncount: 10\n---\n{{ count > 5 ? \"many\" : \"few\" }}";
    let md: Markdown = content.into();

    let options = ComposeOptions::new().only(&[ComposeOperation::Interpolation]);

    let (composed, report) = md.compose_with(options).unwrap();

    assert_eq!(composed.content(), "many");
    assert_eq!(report.interpolations_applied, 1);
}

#[test]
fn test_interpolation_multiple_expressions() {
    let content = "---\nfirst: Alice\nlast: Smith\n---\n{{ first }} {{ last }}";
    let md: Markdown = content.into();

    let options = ComposeOptions::new().only(&[ComposeOperation::Interpolation]);

    let (composed, report) = md.compose_with(options).unwrap();

    assert_eq!(composed.content(), "Alice Smith");
    assert_eq!(report.interpolations_applied, 2);
}

#[test]
fn test_interpolation_scans_inline_code_spans() {
    // Inline code spans (single backticks) are interpolated by default —
    // common templating pattern e.g. `var_{{ phase }}`.
    let content = "---\nname: Alice\n---\nHello {{ name }}! Code: `{{ name }}`";
    let md: Markdown = content.into();

    let options = ComposeOptions::new().only(&[ComposeOperation::Interpolation]);

    let (composed, report) = md.compose_with(options).unwrap();

    assert_eq!(composed.content(), "Hello Alice! Code: `Alice`");
    assert_eq!(report.interpolations_applied, 2);
}

#[test]
fn test_interpolation_code_blocks_via_option() {
    // Fenced code blocks are skipped unless explicitly opted in.
    let content = "---\nname: Alice\n---\nHello {{ name }}!\n\n```\n{{ name }}\n```";
    let md: Markdown = content.into();

    let options = ComposeOptions::new()
        .only(&[ComposeOperation::Interpolation])
        .with_interpolate_code_blocks(true);

    let (composed, report) = md.compose_with(options).unwrap();

    // Both expressions expanded when interpolate_code_blocks is enabled
    assert!(composed.content().contains("Hello Alice!"));
    assert!(composed.content().contains("```\nAlice\n```"));
    assert_eq!(report.interpolations_applied, 2);
}

#[test]
fn test_interpolation_code_blocks_via_frontmatter() {
    let content = "---\nname: Alice\ninterpolate_code_blocks: true\n---\nHello {{ name }}!\n\n```\n{{ name }}\n```";
    let md: Markdown = content.into();

    let options = ComposeOptions::new().only(&[ComposeOperation::Interpolation]);

    let (composed, report) = md.compose_with(options).unwrap();

    // Both expressions expanded when frontmatter flag is set
    assert!(composed.content().contains("Hello Alice!"));
    assert!(composed.content().contains("```\nAlice\n```"));
    assert_eq!(report.interpolations_applied, 2);
}

#[test]
fn test_interpolation_skips_fenced_code() {
    let content = "---\nname: Alice\n---\nHello {{ name }}!\n\n```\n{{ name }}\n```";
    let md: Markdown = content.into();

    let options = ComposeOptions::new().only(&[ComposeOperation::Interpolation]);

    let (composed, report) = md.compose_with(options).unwrap();

    // Only the first expression is expanded, fenced block preserved
    assert!(composed.content().contains("Hello Alice!"));
    assert!(composed.content().contains("```\n{{ name }}\n```"));
    assert_eq!(report.interpolations_applied, 1);
}

#[test]
fn test_interpolation_no_expressions() {
    let content = "---\nname: Alice\n---\n# Just plain text";
    let md: Markdown = content.into();

    let options = ComposeOptions::new().only(&[ComposeOperation::Interpolation]);

    let (composed, report) = md.compose_with(options).unwrap();

    assert_eq!(composed.content(), md.content());
    assert_eq!(report.interpolations_applied, 0);
}

#[test]
fn test_interpolation_with_external_state() {
    let content = "# Hello {{ name }}!";
    let md: Markdown = content.into();

    let options = ComposeOptions::new()
        .only(&[ComposeOperation::Interpolation])
        .with_external_state(serde_json::json!({"name": "External"}));

    let (composed, report) = md.compose_with(options).unwrap();

    assert_eq!(composed.content(), "# Hello External!");
    assert_eq!(report.interpolations_applied, 1);
}

#[test]
fn test_interpolation_frontmatter_overrides_external() {
    let content = "---\nname: Frontmatter\n---\n# Hello {{ name }}!";
    let md: Markdown = content.into();

    let options = ComposeOptions::new()
        .only(&[ComposeOperation::Interpolation])
        .with_external_state(serde_json::json!({"name": "External"}));

    let (composed, report) = md.compose_with(options).unwrap();

    // Frontmatter wins on conflict
    assert_eq!(composed.content(), "# Hello Frontmatter!");
    assert_eq!(report.interpolations_applied, 1);
}

#[test]
fn test_interpolation_chained_fallback() {
    let content = "---\nbackup: second\n---\nValue: {{ missing || backup || \"default\" }}";
    let md: Markdown = content.into();

    let options = ComposeOptions::new().only(&[ComposeOperation::Interpolation]);

    let (composed, report) = md.compose_with(options).unwrap();

    assert_eq!(composed.content(), "Value: second");
    assert_eq!(report.interpolations_applied, 1);
}

#[test]
fn test_interpolation_parse_error_preserves_original() {
    // Malformed expression should be left as-is (not fail_fast)
    let content = "---\nname: Alice\n---\nHello {{ @invalid }}!";
    let md: Markdown = content.into();

    let options = ComposeOptions::new()
        .only(&[ComposeOperation::Interpolation])
        .with_fail_fast(false);

    let (composed, report) = md.compose_with(options).unwrap();

    // Invalid expression left unchanged
    assert_eq!(composed.content(), "Hello {{ @invalid }}!");
    assert_eq!(report.interpolations_applied, 0);
}

#[test]
fn test_interpolation_parse_error_fail_fast_returns_error() {
    let content = "---\nname: Alice\n---\nHello {{ @invalid }}!";
    let md: Markdown = content.into();

    let options = ComposeOptions::new()
        .only(&[ComposeOperation::Interpolation])
        .with_fail_fast(true);

    let err = md.compose_with(options).unwrap_err();
    assert!(matches!(err, MarkdownError::Interpolation { .. }));
}

#[test]
fn test_interpolation_bare_pipe_produces_parse_error() {
    // Bare `|` in interpolation should produce a clear lexer error
    let content = "---\nname: Alice\n---\nHello {{ name | \"default\" }}!";
    let md: Markdown = content.into();

    let options = ComposeOptions::new()
        .only(&[ComposeOperation::Interpolation])
        .with_fail_fast(false);

    let (composed, report) = md.compose_with(options).unwrap();

    // Invalid expression left unchanged
    assert_eq!(composed.content(), "Hello {{ name | \"default\" }}!");
    assert_eq!(report.interpolations_applied, 0);
}

#[test]
fn test_interpolation_bare_pipe_fail_fast_error_message() {
    let content = "---\nname: Alice\n---\nHello {{ name | \"default\" }}!";
    let md: Markdown = content.into();

    let options = ComposeOptions::new()
        .only(&[ComposeOperation::Interpolation])
        .with_fail_fast(true);

    let err = md.compose_with(options).unwrap_err();
    let err_string = format!("{}", err);
    assert!(
        err_string.contains("Unexpected '|'") || err_string.contains("parse"),
        "Expected bare pipe error, got: {}",
        err_string
    );
}

#[test]
fn test_full_compose_with_interpolation() {
    // Integration test: frontmatter + interpolation + cleanup
    let content = "---\nname: Alice\ncount: 3\n---\n# Welcome {{ name }}\nYou have {{ count > 0 ? \"items\" : \"nothing\" }}";
    let md: Markdown = content.into();

    let (composed, report) = md.compose().unwrap();

    assert!(composed.content().contains("Welcome Alice"));
    assert!(composed.content().contains("You have items"));
    assert_eq!(report.interpolations_applied, 2);
    assert!(report.cleanup_changed); // Cleanup adds blank line
}

#[test]
fn test_interpolation_report_summary() {
    let content = "---\na: 1\nb: 2\n---\n{{ a }} {{ b }}";
    let md: Markdown = content.into();

    let options = ComposeOptions::new().only(&[ComposeOperation::Interpolation]);

    let (_, report) = md.compose_with(options).unwrap();

    assert_eq!(report.interpolations_applied, 2);
    let summary = report.summary();
    assert!(summary.contains("2 interpolation(s)"));
}

// ============================================
// E2E Integration tests
// ============================================

#[test]
fn test_e2e_all_stages_with_external_state() {
    // Full pipeline: replacement -> interpolation -> cleanup -> normalization
    let content = r#"---
replace:
  PLACEHOLDER: actual
name: Alice
count: 5
---
# Welcome {{ name }}
PLACEHOLDER content here.
{{ count > 3 ? "many items" : "few items" }}"#;

    let md: Markdown = content.into();

    let options = ComposeOptions::new().with_external_state(serde_json::json!({
        "extra": "external_value"
    }));

    let (composed, report) = md.compose_with(options).unwrap();

    // Replacement happened
    assert!(composed.content().contains("actual content here"));
    assert_eq!(report.replacements_applied, 1);

    // Interpolation happened
    assert!(composed.content().contains("Welcome Alice"));
    assert!(composed.content().contains("many items"));
    assert_eq!(report.interpolations_applied, 2);

    // Cleanup happened (blank line added)
    assert!(report.cleanup_changed);

    // Full summary
    let summary = report.summary();
    assert!(summary.contains("1 replacement(s)"));
    assert!(summary.contains("2 interpolation(s)"));
}

#[test]
fn test_e2e_unicode_content() {
    // Test Unicode handling in replacement and interpolation
    let content = r#"---
replace:
  ":smile:": "😊"
  ":wave:": "👋"
greeting: こんにちは
---
Hello :wave: {{ greeting }} :smile:"#;

    let md: Markdown = content.into();

    let options = ComposeOptions::new().only(&[
        ComposeOperation::TextReplacement,
        ComposeOperation::Interpolation,
    ]);

    let (composed, report) = md.compose_with(options).unwrap();

    assert_eq!(composed.content(), "Hello 👋 こんにちは 😊");
    assert_eq!(report.replacements_applied, 2);
    assert_eq!(report.interpolations_applied, 1);
}

#[test]
fn test_e2e_helper_functions() {
    let content = r#"---
items:
  - a
  - b
  - c
value: "42"
pi: 3.14159
---
Items: {{ length(items) }}
Number: {{ number(value) }}
Rounded: {{ round(pi) }}"#;

    let md: Markdown = content.into();

    let options = ComposeOptions::new().only(&[ComposeOperation::Interpolation]);

    let (composed, report) = md.compose_with(options).unwrap();

    assert!(composed.content().contains("Items: 3"));
    assert!(composed.content().contains("Number: 42"));
    assert!(composed.content().contains("Rounded: 3"));
    assert_eq!(report.interpolations_applied, 3);
}

#[test]
fn disclosure_block_markers_survive_compose() {
    let content = "::disclosure\nLicense\n::details\nBody\n::end-disclosure\n";
    let md: Markdown = content.into();

    let (composed, _report) = md.compose().unwrap();
    let text = composed.content();

    assert!(
        text.contains("::disclosure"),
        "compose must preserve ::disclosure marker: {text}"
    );
    assert!(
        text.contains("::details"),
        "compose must preserve ::details marker: {text}"
    );
    assert!(
        text.contains("::end-disclosure"),
        "compose must preserve ::end-disclosure marker: {text}"
    );
    assert!(text.contains("License"), "summary text must survive: {text}");
    assert!(text.contains("Body"), "body text must survive: {text}");
}

#[test]
fn apply_wrappers_emits_disclosure_dsl() {
    let md: Markdown = "# Test\n".into();
    let options = transclusion::BlockOptions {
        disclosure: Some("More".to_string()),
        ..Default::default()
    };

    let wrapped =
        TransclusionEngine::new(&md).apply_wrappers("# Included\n\nBody.\n".to_string(), &options);

    assert!(
        wrapped.contains("::disclosure"),
        "must emit ::disclosure opener: {wrapped}"
    );
    assert!(
        wrapped.contains("::details"),
        "must emit ::details separator: {wrapped}"
    );
    assert!(
        wrapped.contains("::end-disclosure"),
        "must emit ::end-disclosure closer: {wrapped}"
    );
    assert!(
        wrapped.contains("More"),
        "must include summary text: {wrapped}"
    );
    assert!(
        !wrapped.contains("<details>"),
        "must not emit HTML details: {wrapped}"
    );
    assert!(
        !wrapped.contains("<summary>"),
        "must not emit HTML summary: {wrapped}"
    );
}

#[test]
fn apply_wrappers_normalizes_empty_disclosure_summary_to_details() {
    let md: Markdown = "# Test\n".into();
    let options = transclusion::BlockOptions {
        disclosure: Some(String::new()),
        ..Default::default()
    };

    let wrapped =
        TransclusionEngine::new(&md).apply_wrappers("Body.\n".to_string(), &options);

    assert!(
        wrapped.contains("::disclosure\nDetails\n::details"),
        "empty summary must normalize to 'Details': {wrapped}"
    );
}

#[test]
fn apply_wrappers_normalizes_true_disclosure_summary_to_details() {
    let md: Markdown = "# Test\n".into();
    let options = transclusion::BlockOptions {
        disclosure: Some("true".to_string()),
        ..Default::default()
    };

    let wrapped =
        TransclusionEngine::new(&md).apply_wrappers("Body.\n".to_string(), &options);

    assert!(
        wrapped.contains("::disclosure\nDetails\n::details"),
        "'true' summary must normalize to 'Details': {wrapped}"
    );
}

/// A document-relative body transclusion (`::file _senior-reviewer.md`) still
/// resolves next to the prompt document when a `file_ref_fallback_dir` is
/// configured — the fallback change is scoped to caller-supplied file
/// references in expression functions and the `darkmatter-file` schema
/// validator, and must NOT leak into the transclusion resolver (which uses
/// its own document-relative `resolve_path`). Verification goal #5.
///
/// Layout: both the document dir and the fallback dir hold a same-named
/// `_senior-reviewer.md`. The transcluded content must come from the document
/// dir copy; if the fallback leaked into transclusion, the fallback content
/// would appear instead.
#[test]
fn body_file_transclusion_stays_document_relative_with_fallback() {
    let doc_dir = tempfile::tempdir().expect("tempdir");
    let fallback_dir = tempfile::tempdir().expect("tempdir");

    let root = doc_dir.path().join("review.md");
    let doc_reviewer = doc_dir.path().join("_senior-reviewer.md");
    let fallback_reviewer = fallback_dir.path().join("_senior-reviewer.md");

    std::fs::write(&root, "::file ./_senior-reviewer.md").unwrap();
    std::fs::write(&doc_reviewer, "# From Document").unwrap();
    std::fs::write(&fallback_reviewer, "# From Fallback").unwrap();

    let md = Markdown::try_from(root.as_path()).unwrap();
    let options = ComposeOptions::new()
        .with_source_file(&root)
        .with_file_ref_fallback_dir(fallback_dir.path().to_path_buf());
    let (composed, report) = md.compose_with(options).unwrap();

    assert!(
        composed.content().contains("# From Document"),
        "body `::file` transclusion must resolve from the document dir: {}",
        composed.content(),
    );
    assert!(
        !composed.content().contains("# From Fallback"),
        "transclusion must NOT leak to the fallback dir: {}",
        composed.content(),
    );
    assert_eq!(report.transclusions_applied, 1);
}
