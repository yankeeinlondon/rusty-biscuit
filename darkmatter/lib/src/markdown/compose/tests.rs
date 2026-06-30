//! Integration tests for the compose pipeline public API and stages.
//!
//! Extracted from `compose/mod.rs` (Phase 6: slim mod.rs to a thin facade).

#[allow(unused_imports)]
use super::HeadingLevel;
use super::*;
use super::super::types::MarkdownError;
use super::transclusion::TransclusionEngine;
use biscuit_terminal::utils::UnicodeWidthStr;

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
fn test_stage2_file_transclusion_relevels_to_parent_heading() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("root.md");
    let child = dir.path().join("child.md");

    std::fs::write(&root, "## Parent\n\n::file ./child.md").unwrap();
    std::fs::write(&child, "# Child\n\nBody").unwrap();

    let md = Markdown::try_from(root.as_path()).unwrap();
    let options = ComposeOptions::new().with_source_file(root);
    let (composed, report) = md.compose_with(options).unwrap();

    assert!(composed.content().contains("### Child"));
    assert!(composed.content().contains("Body"));
    assert_eq!(report.transclusions_applied, 1);
}

#[test]
fn test_stage2_nested_transclusion_counts_recursive_includes() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("root.md");
    let a = dir.path().join("a.md");
    let b = dir.path().join("b.md");

    std::fs::write(&root, "::file ./a.md").unwrap();
    std::fs::write(&a, "::file ./b.md").unwrap();
    std::fs::write(&b, "# Leaf").unwrap();

    let md = Markdown::try_from(root.as_path()).unwrap();
    let options = ComposeOptions::new().with_source_file(root);
    let (composed, report) = md.compose_with(options).unwrap();

    assert!(composed.content().contains("# Leaf"));
    assert_eq!(report.transclusions_applied, 2);
    assert!(report.max_transclusion_depth >= 2);
}

#[test]
fn test_stage2_duplicate_sibling_includes_are_not_treated_as_cycles() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("root.md");
    let child = dir.path().join("child.md");

    std::fs::write(&root, "::file ./child.md\n\n::file ./child.md").unwrap();
    std::fs::write(&child, "# Child").unwrap();

    let md = Markdown::try_from(root.as_path()).unwrap();
    let options = ComposeOptions::new().with_source_file(&root);
    let (composed, report) = md.compose_with(options).unwrap();

    assert_eq!(composed.content().matches("# Child").count(), 2);
    assert_eq!(report.transclusions_applied, 2);
}

#[test]
fn test_stage2_diamond_dependency_graph_is_allowed() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("root.md");
    let left = dir.path().join("left.md");
    let right = dir.path().join("right.md");
    let shared = dir.path().join("shared.md");

    std::fs::write(&root, "::file ./left.md\n\n::file ./right.md").unwrap();
    std::fs::write(&left, "## Left\n\n::file ./shared.md").unwrap();
    std::fs::write(&right, "## Right\n\n::file ./shared.md").unwrap();
    std::fs::write(&shared, "### Shared").unwrap();

    let md = Markdown::try_from(root.as_path()).unwrap();
    let options = ComposeOptions::new().with_source_file(&root);
    let (composed, report) = md.compose_with(options).unwrap();

    assert_eq!(composed.content().matches("### Shared").count(), 2);
    assert_eq!(report.transclusions_applied, 4);
}

#[test]
fn test_stage2_cycle_detection_fails() {
    let dir = tempfile::tempdir().unwrap();
    let a = dir.path().join("a.md");
    let b = dir.path().join("b.md");

    std::fs::write(&a, "::file ./b.md").unwrap();
    std::fs::write(&b, "::file ./a.md").unwrap();

    let md = Markdown::try_from(a.as_path()).unwrap();
    let options = ComposeOptions::new().with_source_file(a);
    let err = md.compose_with(options).unwrap_err();

    assert!(matches!(
        err,
        MarkdownError::Transclusion(ref inner)
            if matches!(inner.as_ref(), transclusion::TransclusionError::CycleDetected { .. })
    ));
}

#[test]
fn test_stage2_code_transclusion_wraps_fenced_block() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("root.md");
    let code = dir.path().join("main.rs");

    std::fs::write(&root, "## Code\n\n::code ./main.rs").unwrap();
    std::fs::write(&code, "fn main() {\n    println!(\"hi\");\n}\n").unwrap();

    let md = Markdown::try_from(root.as_path()).unwrap();
    let options = ComposeOptions::new().with_source_file(root);
    let (composed, report) = md.compose_with(options).unwrap();

    assert!(composed.content().contains("```rs"));
    assert!(composed.content().contains("fn main()"));
    assert_eq!(report.transclusions_applied, 1);
}

#[test]
fn test_stage2_code_transclusion_uses_fallback_language_for_unknown_extension() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("root.md");
    let code = dir.path().join("sample.weird");

    std::fs::write(&root, "::code ./sample.weird").unwrap();
    std::fs::write(&code, "hello").unwrap();

    let md = Markdown::try_from(root.as_path()).unwrap();
    let options = ComposeOptions::new().with_source_file(root);
    let (composed, report) = md.compose_with(options).unwrap();

    assert!(composed.content().contains("```txt"));
    assert!(composed.content().contains("hello"));
    assert_eq!(report.transclusions_applied, 1);
}

#[test]
fn test_stage2_repeated_code_includes_are_allowed() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("root.md");
    let code = dir.path().join("main.rs");

    std::fs::write(&root, "::code ./main.rs\n\n::code ./main.rs").unwrap();
    std::fs::write(&code, "fn repeated() {}\n").unwrap();

    let md = Markdown::try_from(root.as_path()).unwrap();
    let options = ComposeOptions::new().with_source_file(&root);
    let (composed, report) = md.compose_with(options).unwrap();

    assert_eq!(composed.content().matches("fn repeated() {}").count(), 2);
    assert_eq!(report.transclusions_applied, 2);
}

#[test]
fn test_stage2_when_false_skips_directive() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("root.md");
    let child = dir.path().join("child.md");

    std::fs::write(
        &root,
        "---\nenabled: false\n---\n::file ./child.md when=\"enabled\"",
    )
    .unwrap();
    std::fs::write(&child, "# Child").unwrap();

    let md = Markdown::try_from(root.as_path()).unwrap();
    let options = ComposeOptions::new().with_source_file(root);
    let (composed, report) = md.compose_with(options).unwrap();

    assert!(!composed.content().contains("Child"));
    assert_eq!(report.transclusions_skipped, 1);
}

#[test]
fn test_stage2_frontmatter_prologue_epilogue() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("root.md");
    let intro = dir.path().join("intro.md");
    let outro = dir.path().join("outro.md");

    std::fs::write(
        &root,
        "---\nprologue: ./intro.md\nepilogue: [\"./outro.md\"]\n---\nBody",
    )
    .unwrap();
    std::fs::write(&intro, "Intro").unwrap();
    std::fs::write(&outro, "Outro").unwrap();

    let md = Markdown::try_from(root.as_path()).unwrap();
    let options = ComposeOptions::new().with_source_file(root);
    let (composed, report) = md.compose_with(options).unwrap();

    assert!(composed.content().starts_with("Intro"));
    assert!(composed.content().contains("Body"));
    assert!(composed.content().trim_end_matches('\n').ends_with("Outro"));
    assert_eq!(report.transclusions_applied, 2);
}

#[test]
fn test_stage2_same_file_can_be_used_in_prologue_and_body() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("root.md");
    let shared = dir.path().join("shared.md");

    std::fs::write(&root, "---\nprologue: ./shared.md\n---\n::file ./shared.md").unwrap();
    std::fs::write(&shared, "## Shared").unwrap();

    let md = Markdown::try_from(root.as_path()).unwrap();
    let options = ComposeOptions::new().with_source_file(&root);
    let (composed, report) = md.compose_with(options).unwrap();

    assert_eq!(composed.content().matches("## Shared").count(), 2);
    assert_eq!(report.transclusions_applied, 2);
}

#[test]
fn test_stage2_missing_source_context_for_relative_path() {
    let md: Markdown = "::file ./child.md".into();
    let err = md.compose().unwrap_err();
    assert!(matches!(
        err,
        MarkdownError::Transclusion(ref inner)
            if matches!(
                inner.as_ref(),
                transclusion::TransclusionError::MissingSourceContext { .. }
            )
    ));
}

#[test]
fn test_toc_linking_fail_fast_false_becomes_warning() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("root.md");
    std::fs::write(&root, "::toc-linking ./missing.md").unwrap();

    let md = Markdown::try_from(root.as_path()).unwrap();
    let options = ComposeOptions::new()
        .with_source_file(&root)
        .with_fail_fast(false);
    let (composed, report) = md.compose_with(options).unwrap();

    assert_eq!(composed.content().trim_end(), "::toc-linking ./missing.md");
    assert_eq!(report.toc_links_generated, 0);
    assert!(
        report
            .warnings
            .iter()
            .any(|warning| warning.message.contains("File not found"))
    );
}

#[test]
fn test_toc_linking_fail_fast_true_returns_error() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("root.md");
    std::fs::write(&root, "::toc-linking ./missing.md").unwrap();

    let md = Markdown::try_from(root.as_path()).unwrap();
    let options = ComposeOptions::new()
        .with_source_file(&root)
        .with_fail_fast(true);
    let err = md.compose_with(options).unwrap_err();

    assert!(matches!(err, MarkdownError::TocLinking(_)));
}

#[test]
fn test_stage2_h6_overflow_converts_to_bold_text() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("root.md");
    let child = dir.path().join("child.md");

    std::fs::write(&root, "###### Root\n\n::file ./child.md").unwrap();
    std::fs::write(&child, "## Child\n\n### Deep").unwrap();

    let md = Markdown::try_from(root.as_path()).unwrap();
    let options = ComposeOptions::new().with_source_file(root);
    let (composed, report) = md.compose_with(options).unwrap();

    assert!(composed.content().contains("###### Child"));
    assert!(composed.content().contains("**Deep**"));
    assert!(
        report
            .warnings
            .iter()
            .any(|warning| warning.message.contains("Heading overflow"))
    );
}

#[test]
fn test_stage2_consecutive_file_directives_separated_by_blank_line() {
    // Regression test: when two ::file directives are consecutive, the second
    // file's content must not be absorbed into the last block element (e.g., a
    // list) of the first file.
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("root.md");
    let one = dir.path().join("one.md");
    let two = dir.path().join("two.md");

    std::fs::write(&root, "::file ./one.md\n\n::file ./two.md").unwrap();
    std::fs::write(&one, "- Item A\n- Item B").unwrap();
    std::fs::write(&two, "## Section Two\n\nParagraph.").unwrap();

    let md = Markdown::try_from(root.as_path()).unwrap();
    let options = ComposeOptions::new().with_source_file(root);
    let (composed, report) = md.compose_with(options).unwrap();

    // Two transclusions should have occurred
    assert_eq!(report.transclusions_applied, 2);
    // The heading from two.md must exist as a proper heading, not inside a list
    assert!(
        composed.content().contains("\n## Section Two\n")
            || composed.content().contains("\n## Section Two"),
        "Second file's heading should not be absorbed into first file's list, got:\n{}",
        composed.content()
    );
}

#[test]
fn test_stage2_frontmatter_inline_string_prologue() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("root.md");
    std::fs::write(
        &root,
        "---\nprologue: \"**Draft** document\"\n---\nBody content.",
    )
    .unwrap();

    let md = Markdown::try_from(root.as_path()).unwrap();
    let options = ComposeOptions::new().with_source_file(root);
    let (composed, report) = md.compose_with(options).unwrap();

    assert!(composed.content().starts_with("**Draft** document"));
    assert!(composed.content().contains("Body content."));
    assert_eq!(report.transclusions_applied, 0); // inline string is not a transclusion
}

#[test]
fn test_stage2_frontmatter_inline_string_epilogue() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("root.md");
    std::fs::write(&root, "---\nepilogue: \"End of document.\"\n---\nBody.").unwrap();

    let md = Markdown::try_from(root.as_path()).unwrap();
    let options = ComposeOptions::new().with_source_file(root);
    let (composed, report) = md.compose_with(options).unwrap();

    assert!(
        composed
            .content()
            .trim_end_matches('\n')
            .ends_with("End of document.")
    );
    assert_eq!(report.transclusions_applied, 0);
}

#[test]
fn test_stage2_frontmatter_mixed_file_and_inline() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("root.md");
    let intro = dir.path().join("intro.md");

    std::fs::write(
        &root,
        "---\nprologue: [\"./intro.md\", \"Inline note.\"]\n---\nBody.",
    )
    .unwrap();
    std::fs::write(&intro, "File intro.").unwrap();

    let md = Markdown::try_from(root.as_path()).unwrap();
    let options = ComposeOptions::new().with_source_file(root);
    let (composed, report) = md.compose_with(options).unwrap();

    let content = composed.content();
    assert!(content.starts_with("File intro."));
    assert!(content.contains("Inline note."));
    assert!(content.contains("Body."));
    assert_eq!(report.transclusions_applied, 1); // only the file counts
}

#[test]
fn test_stage2_frontmatter_bare_filename_is_treated_as_file_reference() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("root.md");
    let intro = dir.path().join("intro.md");

    std::fs::write(&root, "---\nprologue: intro.md\n---\nBody.").unwrap();
    std::fs::write(&intro, "Intro text.").unwrap();

    let md = Markdown::try_from(root.as_path()).unwrap();
    let options = ComposeOptions::new().with_source_file(root);
    let (composed, report) = md.compose_with(options).unwrap();

    let content = composed.content();
    assert!(content.starts_with("Intro text."));
    assert!(content.contains("Body."));
    assert_eq!(report.transclusions_applied, 1);
}

#[test]
fn test_stage2_parent_frontmatter_propagates_to_child_interpolation() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("root.md");
    let child = dir.path().join("child.md");

    std::fs::write(&root, "---\nauthor: Alice\n---\n::file ./child.md").unwrap();
    std::fs::write(&child, "Written by {{ author }}.").unwrap();

    let md = Markdown::try_from(root.as_path()).unwrap();
    let options = ComposeOptions::new().with_source_file(root);
    let (composed, _report) = md.compose_with(options).unwrap();

    assert!(composed.content().contains("Written by Alice."));
}

#[test]
fn test_stage2_parent_replace_map_propagates_to_child() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("root.md");
    let child = dir.path().join("child.md");

    std::fs::write(
        &root,
        "---\nreplace:\n  PLACEHOLDER: actual\n---\n::file ./child.md",
    )
    .unwrap();
    std::fs::write(&child, "Content with PLACEHOLDER here.").unwrap();

    let md = Markdown::try_from(root.as_path()).unwrap();
    let options = ComposeOptions::new().with_source_file(root);
    let (composed, _report) = md.compose_with(options).unwrap();

    assert!(composed.content().contains("Content with actual here."));
}

#[test]
fn test_stage2_replace_parent_wins_inverts_precedence() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("root.md");
    let child = dir.path().join("child.md");

    std::fs::write(
        &root,
        "---\nreplace:\n  TOKEN: parent\n---\n::file ./child.md replace=true",
    )
    .unwrap();
    std::fs::write(&child, "---\nreplace:\n  TOKEN: child\n---\nTOKEN").unwrap();

    let md = Markdown::try_from(root.as_path()).unwrap();
    let options = ComposeOptions::new().with_source_file(root);
    let (composed, _report) = md.compose_with(options).unwrap();

    assert_eq!(composed.content().trim(), "parent");
}

#[test]
fn test_stage2_replace_one_off_does_not_propagate_to_grandchildren() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("root.md");
    let child = dir.path().join("child.md");
    let grand = dir.path().join("grand.md");

    std::fs::write(
        &root,
        "---\nreplace:\n  A: root\n---\n::file ./child.md replace={\"ONE\":\"oneoff\"}",
    )
    .unwrap();
    std::fs::write(&child, "Child: ONE A\n::file ./grand.md").unwrap();
    std::fs::write(&grand, "Grand: ONE A").unwrap();

    let md = Markdown::try_from(root.as_path()).unwrap();
    let options = ComposeOptions::new().with_source_file(root);
    let (composed, _report) = md.compose_with(options).unwrap();

    let content = composed.content();
    assert!(content.contains("Child: oneoff root"));
    assert!(content.contains("Grand: ONE root"));
}

#[test]
fn test_stage2_prologue_epilogue_do_not_propagate_to_children() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("root.md");
    let child = dir.path().join("child.md");

    std::fs::write(
        &root,
        "---\nepilogue: \"Root epilogue.\"\n---\n::file ./child.md",
    )
    .unwrap();
    std::fs::write(&child, "Child body.").unwrap();

    let md = Markdown::try_from(root.as_path()).unwrap();
    let options = ComposeOptions::new().with_source_file(root);
    let (composed, _report) = md.compose_with(options).unwrap();

    let content = composed.content();
    // "Root epilogue." should appear exactly once — at the end of root, not within child
    assert_eq!(content.matches("Root epilogue.").count(), 1);
    assert!(content.trim_end_matches('\n').ends_with("Root epilogue."));
}

#[test]
fn test_stage2_inline_epilogue_with_markdown_links() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("root.md");

    // Epilogue containing markdown link syntax should be treated as inline
    // content, not as a file reference. Previously, the `/` in `](./...)
    // caused `is_file_like_reference` to misidentify it as a path.
    std::fs::write(
        &root,
        "---\nepilogue: \"---\\n\\n- No [animals](./animals.md) were hurt\"\n---\nBody.",
    )
    .unwrap();

    let md = Markdown::try_from(root.as_path()).unwrap();
    let options = ComposeOptions::new().with_source_file(root);
    let (composed, _report) = md.compose_with(options).unwrap();

    let content = composed.content();
    assert!(
        content.contains("[animals](./animals.md)"),
        "Inline epilogue with markdown links should be preserved, got:\n{}",
        content
    );
}

#[test]
fn test_stage2_exclude_removes_section() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("root.md");
    let child = dir.path().join("child.md");

    std::fs::write(&root, "::file ./child.md exclude=\"## Remove Me\"").unwrap();
    std::fs::write(
        &child,
        "## Keep\n\nKept body.\n\n## Remove Me\n\nRemoved body.\n\n## Also Keep\n\nAlso kept.",
    )
    .unwrap();

    let md = Markdown::try_from(root.as_path()).unwrap();
    let options = ComposeOptions::new().with_source_file(root);
    let (composed, report) = md.compose_with(options).unwrap();

    assert!(composed.content().contains("## Keep"));
    assert!(composed.content().contains("Kept body."));
    assert!(!composed.content().contains("Remove Me"));
    assert!(!composed.content().contains("Removed body."));
    assert!(composed.content().contains("## Also Keep"));
    assert_eq!(report.transclusions_applied, 1);
}

#[test]
fn test_stage2_exclude_wildcard() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("root.md");
    let child = dir.path().join("child.md");

    std::fs::write(&root, "::file ./child.md exclude=\"## Remove*\"").unwrap();
    std::fs::write(
        &child,
        "## Keep\n\nKept.\n\n## Remove This\n\nGone.\n\n## Also Keep\n\nStays.",
    )
    .unwrap();

    let md = Markdown::try_from(root.as_path()).unwrap();
    let options = ComposeOptions::new().with_source_file(root);
    let (composed, _) = md.compose_with(options).unwrap();

    assert!(composed.content().contains("## Keep"));
    assert!(!composed.content().contains("Remove This"));
    assert!(composed.content().contains("## Also Keep"));
}

#[test]
fn test_stage2_exclude_prelude() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("root.md");
    let child = dir.path().join("child.md");

    std::fs::write(&root, "::file ./child.md exclude=\"!prelude\"").unwrap();
    std::fs::write(&child, "Prelude text here.\n\n## Heading\n\nBody.").unwrap();

    let md = Markdown::try_from(root.as_path()).unwrap();
    let options = ComposeOptions::new().with_source_file(root);
    let (composed, _) = md.compose_with(options).unwrap();

    assert!(!composed.content().contains("Prelude text"));
    assert!(composed.content().contains("## Heading"));
    assert!(composed.content().contains("Body."));
}

#[test]
fn test_stage2_multiple_excludes() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("root.md");
    let child = dir.path().join("child.md");

    std::fs::write(&root, "::file ./child.md exclude=\"## A\" exclude=\"## C\"").unwrap();
    std::fs::write(
        &child,
        "## A\n\nA body.\n\n## B\n\nB body.\n\n## C\n\nC body.",
    )
    .unwrap();

    let md = Markdown::try_from(root.as_path()).unwrap();
    let options = ComposeOptions::new().with_source_file(root);
    let (composed, _) = md.compose_with(options).unwrap();

    assert!(!composed.content().contains("## A"));
    assert!(composed.content().contains("## B"));
    assert!(!composed.content().contains("## C"));
}

#[test]
fn test_stage2_quotation_wrapper_does_not_absorb_following_content() {
    // Regression: wrap_quotation consumed trailing \n\n, causing the
    // next paragraph to become a lazy continuation of the blockquote.
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("root.md");
    let child = dir.path().join("child.md");

    std::fs::write(
        &root,
        "## Section\n\n::file ./child.md quotation=\"Source\"\n\nFollowing paragraph.",
    )
    .unwrap();
    std::fs::write(&child, "Quoted content here.").unwrap();

    let md = Markdown::try_from(root.as_path()).unwrap();
    let options = ComposeOptions::new().with_source_file(root);
    let (composed, report) = md.compose_with(options).unwrap();

    assert_eq!(report.transclusions_applied, 1);
    // The "Following paragraph" must NOT be inside the blockquote
    let content = composed.content();
    assert!(
        content.contains("\n\nFollowing paragraph."),
        "Following content should be separated from blockquote by blank line, got:\n{}",
        content
    );
    // Verify blockquote is present
    assert!(content.contains("> Quoted content here."));
    assert!(content.contains("> — Source"));
}

// ============================================
// Conditional transclusion tests
// ============================================

#[test]
fn test_stage2_when_env_match_includes_directive() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("root.md");
    let child = dir.path().join("child.md");

    std::fs::write(
        &root,
        "## Section\n\n::file ./child.md when=\"env.AGENT == 'claude'\"",
    )
    .unwrap();
    std::fs::write(&child, "Claude content.").unwrap();

    let md = Markdown::try_from(root.as_path()).unwrap();
    let mut ctx = ComposeContext::capture();
    ctx.env_mut()
        .insert("AGENT".to_string(), "claude".to_string());
    let options = ComposeOptions::new()
        .with_source_file(root)
        .with_context(ctx);
    let (composed, report) = md.compose_with(options).unwrap();

    assert!(composed.content().contains("Claude content."));
    assert_eq!(report.transclusions_applied, 1);
    assert_eq!(report.transclusions_skipped, 0);
}

#[test]
fn test_stage2_when_env_mismatch_skips_directive() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("root.md");
    let child = dir.path().join("child.md");

    std::fs::write(
        &root,
        "## Section\n\n::file ./child.md when=\"env.AGENT == 'claude'\"",
    )
    .unwrap();
    std::fs::write(&child, "Claude content.").unwrap();

    let md = Markdown::try_from(root.as_path()).unwrap();
    let mut ctx = ComposeContext::capture();
    ctx.env_mut()
        .insert("AGENT".to_string(), "opencode".to_string());
    let options = ComposeOptions::new()
        .with_source_file(root)
        .with_context(ctx);
    let (composed, report) = md.compose_with(options).unwrap();

    assert!(!composed.content().contains("Claude content."));
    assert_eq!(report.transclusions_applied, 0);
    assert_eq!(report.transclusions_skipped, 1);
}

#[test]
fn test_stage2_when_env_unset_skips_equality() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("root.md");
    let child = dir.path().join("child.md");

    std::fs::write(
        &root,
        "## Section\n\n::file ./child.md when=\"env.AGENT == 'claude'\"",
    )
    .unwrap();
    std::fs::write(&child, "Claude content.").unwrap();

    let md = Markdown::try_from(root.as_path()).unwrap();
    // Use a fixed context with no AGENT env var
    let ctx = ComposeContext::fixed_for_testing();
    let options = ComposeOptions::new()
        .with_source_file(root)
        .with_context(ctx);
    let (composed, report) = md.compose_with(options).unwrap();

    assert!(!composed.content().contains("Claude content."));
    assert_eq!(report.transclusions_skipped, 1);
}

#[test]
fn test_stage2_mutual_exclusion_conditions() {
    // Three directives with mutually exclusive conditions:
    //   AGENT == 'claude'
    //   AGENT == 'opencode'
    //   !env.AGENT (unset)
    // Only one should match at any time.
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("root.md");
    let cc = dir.path().join("cc.md");
    let oc = dir.path().join("oc.md");
    let default = dir.path().join("default.md");

    std::fs::write(
        &root,
        "## Section\n\n\
         ::file ./cc.md when=\"env.AGENT == 'claude'\"\n\
         ::file ./oc.md when=\"env.AGENT == 'opencode'\"\n\
         ::file ./default.md when=\"!env.AGENT\"",
    )
    .unwrap();
    std::fs::write(&cc, "CC only.").unwrap();
    std::fs::write(&oc, "OC only.").unwrap();
    std::fs::write(&default, "Default only.").unwrap();

    // Test 1: AGENT=claude → only cc.md included
    let md = Markdown::try_from(root.as_path()).unwrap();
    let mut ctx = ComposeContext::capture();
    ctx.env_mut()
        .insert("AGENT".to_string(), "claude".to_string());
    let opts = ComposeOptions::new()
        .with_source_file(&root)
        .with_context(ctx);
    let (out, report) = md.compose_with(opts).unwrap();
    assert!(out.content().contains("CC only."), "Expected CC content");
    assert!(!out.content().contains("OC only."), "Should not contain OC");
    assert!(
        !out.content().contains("Default only."),
        "Should not contain default"
    );
    assert_eq!(report.transclusions_applied, 1);
    assert_eq!(report.transclusions_skipped, 2);

    // Test 2: AGENT=opencode → only oc.md included
    let md = Markdown::try_from(root.as_path()).unwrap();
    let mut ctx = ComposeContext::capture();
    ctx.env_mut()
        .insert("AGENT".to_string(), "opencode".to_string());
    let opts = ComposeOptions::new()
        .with_source_file(&root)
        .with_context(ctx);
    let (out, report) = md.compose_with(opts).unwrap();
    assert!(!out.content().contains("CC only."));
    assert!(out.content().contains("OC only."), "Expected OC content");
    assert!(!out.content().contains("Default only."));
    assert_eq!(report.transclusions_applied, 1);
    assert_eq!(report.transclusions_skipped, 2);

    // Test 3: AGENT not set → only default.md included
    let md = Markdown::try_from(root.as_path()).unwrap();
    let ctx = ComposeContext::fixed_for_testing();
    let opts = ComposeOptions::new()
        .with_source_file(&root)
        .with_context(ctx);
    let (out, report) = md.compose_with(opts).unwrap();
    assert!(!out.content().contains("CC only."));
    assert!(!out.content().contains("OC only."));
    assert!(
        out.content().contains("Default only."),
        "Expected default content"
    );
    assert_eq!(report.transclusions_applied, 1);
    assert_eq!(report.transclusions_skipped, 2);
}

// ============================================
// Re-leveling tests
// ============================================

#[test]
fn test_stage2_relevel_h1_child_under_h3_parent() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("root.md");
    let child = dir.path().join("child.md");

    std::fs::write(
        &root,
        "# Title\n\n## Overview\n\n### Details\n\n::file ./child.md",
    )
    .unwrap();
    std::fs::write(&child, "# Child Title\n\n## Child Sub\n\nBody.").unwrap();

    let md = Markdown::try_from(root.as_path()).unwrap();
    let options = ComposeOptions::new().with_source_file(root);
    let (composed, _) = md.compose_with(options).unwrap();

    // Parent heading before directive is H3, so child should be re-leveled:
    // H1 → H4, H2 → H5
    assert!(
        composed.content().contains("#### Child Title"),
        "H1 should become H4, got:\n{}",
        composed.content()
    );
    assert!(
        composed.content().contains("##### Child Sub"),
        "H2 should become H5, got:\n{}",
        composed.content()
    );
}

// ── Page block integration tests ────────────────────────────────────

#[test]
fn page_block_true_preserves_content_through_pipeline() {
    let content = "---\nflag: true\n---\n\nbefore\n\n::block when=\"flag\"\n\nkept content\n\n::end-block\n\nafter\n";
    let md: Markdown = content.into();

    let options = ComposeOptions::new().only(&[ComposeOperation::PageBlocks]);

    let (composed, report) = md.compose_with(options).unwrap();
    assert!(
        composed.content().contains("kept content"),
        "True block body should be preserved, got:\n{}",
        composed.content()
    );
    assert!(
        composed.content().contains("before"),
        "Content before block should be preserved"
    );
    assert!(
        composed.content().contains("after"),
        "Content after block should be preserved"
    );
    assert_eq!(report.page_blocks_rendered, 1);
}

#[test]
fn page_block_false_removes_content_through_pipeline() {
    let content = "---\nflag: false\n---\n\nbefore\n\n::block when=\"flag\"\n\nremoved\n\n::end-block\n\nafter\n";
    let md: Markdown = content.into();

    let options = ComposeOptions::new().only(&[ComposeOperation::PageBlocks]);

    let (composed, report) = md.compose_with(options).unwrap();
    assert!(
        !composed.content().contains("removed"),
        "False block body should be removed, got:\n{}",
        composed.content()
    );
    assert!(composed.content().contains("before"));
    assert!(composed.content().contains("after"));
    assert_eq!(report.page_blocks_skipped, 1);
}

#[test]
fn page_block_condition_can_read_frontmatter_from_file() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("root.md");
    let review = dir.path().join("review.md");
    std::fs::write(&review, "---\nready: true\n---\n# Review\n").unwrap();

    let content = concat!(
        "---\nreview_path: review.md\n---\n\n",
        "::block when=\"frontmatter(review_path, 'ready') == true\"\n\n",
        "ready block\n\n",
        "::end-block\n",
    );
    std::fs::write(&root, content).unwrap();
    let md: Markdown = content.into();

    let options = ComposeOptions::new()
        .only(&[ComposeOperation::PageBlocks])
        .with_source_file(&root);

    let (composed, report) = md.compose_with(options).unwrap();
    assert!(
        composed.content().contains("ready block"),
        "page block should evaluate filesystem expression functions, got:\n{}",
        composed.content()
    );
    assert_eq!(report.page_blocks_rendered, 1);
}

#[test]
fn page_block_coexists_with_interpolation() {
    let content =
        "---\nshow: true\n---\n\n::block when=\"show\"\n\nShown: {{show}}\n\n::end-block\n";
    let md: Markdown = content.into();

    let options = ComposeOptions::new().only(&[
        ComposeOperation::Interpolation,
        ComposeOperation::PageBlocks,
    ]);

    let (composed, report) = md.compose_with(options).unwrap();
    assert!(
        composed.content().contains("Shown: true"),
        "Page blocks and interpolation should both apply, got:\n{}",
        composed.content()
    );
    assert_eq!(report.page_blocks_rendered, 1);
    assert!(report.interpolations_applied > 0);
}

#[test]
fn page_block_report_and_warnings_populated() {
    let content = "---\na: true\nb: false\n---\n\n::block when=\"a\" unknown=\"x\"\n\nA\n\n::end-block\n\n::block when=\"b\"\n\nB\n\n::end-block\n";
    let md: Markdown = content.into();

    let options = ComposeOptions::new().only(&[ComposeOperation::PageBlocks]);

    let (_, report) = md.compose_with(options).unwrap();
    assert_eq!(report.page_blocks_rendered, 1);
    assert_eq!(report.page_blocks_skipped, 1);
    assert!(
        report
            .warnings
            .iter()
            .any(|w| w.message.contains("unknown")),
        "Should warn about unknown option"
    );
}

#[test]
fn page_block_toggle_disabled_leaves_directives_as_text() {
    let content = "::block when=\"x\"\nbody\n::end-block\n";
    let md: Markdown = content.into();

    let options = ComposeOptions::new().only(&[]);

    let (composed, report) = md.compose_with(options).unwrap();
    assert!(
        composed.content().contains("::block"),
        "With page_blocks disabled, directives should be left as text"
    );
    assert_eq!(report.page_blocks_rendered, 0);
    assert_eq!(report.page_blocks_skipped, 0);
}

#[test]
fn perf_disabled_produces_no_report() {
    let content = "# Test\nSome content";
    let md: Markdown = content.into();

    let options = ComposeOptions::new(); // perf_enabled defaults to false
    let (_, report) = md.compose_with(options).unwrap();
    assert!(report.perf.is_none(), "Perf should be None when disabled");
}

#[test]
fn perf_enabled_produces_report() {
    let content = "# Test\nSome content";
    let md: Markdown = content.into();

    let options = ComposeOptions::new().with_perf(true);
    let (_, report) = md.compose_with(options).unwrap();
    assert!(
        report.perf.is_some(),
        "Perf should be populated when enabled"
    );

    let perf = report.perf.unwrap();
    assert!(perf.total > std::time::Duration::ZERO);
    assert!(!perf.metrics.is_empty(), "Should have at least one metric");

    // Verify expected stages are present
    let stages: Vec<_> = perf.metrics.iter().map(|m| m.stage).collect();
    assert!(stages.contains(&ComposeStage::EffectiveStateBuild));
    assert!(stages.contains(&ComposeStage::Cleanup));
}

#[test]
fn perf_enabled_with_interpolation() {
    let content = "---\nname: World\n---\nHello {{ name }}!";
    let md: Markdown = content.into();

    let options = ComposeOptions::new().with_perf(true);
    let (composed, report) = md.compose_with(options).unwrap();

    assert!(composed.content().contains("Hello World!"));
    let perf = report.perf.unwrap();
    let interp = perf
        .metrics
        .iter()
        .find(|m| m.stage == ComposeStage::Interpolation)
        .unwrap();
    assert_eq!(interp.calls, 1);
}

// ── Frontmatter Interpolation Integration Tests ─────────────────

#[test]
fn test_frontmatter_interpolation_spec_example() {
    let content = "---\nbase: /path/to/something\nspec: \"{{base}}/spec.md\"\nplan: \"{{base}}/plan.md\"\n---\nThe spec is located at: {{spec}}\nThe plan is located at: {{plan}}";
    let md: Markdown = content.into();
    let (composed, report) = md
        .compose_with(ComposeOptions::new().only(&[
            ComposeOperation::FrontmatterInterpolation,
            ComposeOperation::Interpolation,
        ]))
        .unwrap();

    assert_eq!(report.frontmatter_interpolations_applied, 2);
    assert!(
        composed
            .content()
            .contains("The spec is located at: /path/to/something/spec.md")
    );
    assert!(
        composed
            .content()
            .contains("The plan is located at: /path/to/something/plan.md")
    );
}

#[test]
fn test_frontmatter_interpolation_with_set_overrides() {
    let content = "---\nbase: /original\nspec: \"{{base}}/spec.md\"\n---\nSpec: {{spec}}";
    let md: Markdown = content.into();
    let (composed, report) = md
        .compose_with(
            ComposeOptions::new()
                .only(&[
                    ComposeOperation::FrontmatterInterpolation,
                    ComposeOperation::Interpolation,
                ])
                .with_set_overrides(serde_json::json!({"base": "/override"})),
        )
        .unwrap();

    assert_eq!(report.frontmatter_interpolations_applied, 1);
    assert!(composed.content().contains("Spec: /override/spec.md"));
}

#[test]
fn test_frontmatter_interpolation_arrays_and_objects() {
    let content = "---\nbase: /root\npaths:\n  - \"{{base}}/a\"\n  - \"{{base}}/b\"\nmeta:\n  home: \"{{base}}/home\"\n---\n";
    let md: Markdown = content.into();
    let (_, report) = md
        .compose_with(ComposeOptions::new().only(&[ComposeOperation::FrontmatterInterpolation]))
        .unwrap();

    assert!(report.frontmatter_interpolations_applied >= 3);
}

#[test]
fn test_frontmatter_interpolation_disabled() {
    let content = "---\nbase: /path\nspec: \"{{base}}/spec.md\"\n---\n{{spec}}";
    let md: Markdown = content.into();
    let (composed, report) = md
        .compose_with(
            ComposeOptions::new()
                .disable(ComposeOperation::FrontmatterInterpolation)
                .only(&[ComposeOperation::Interpolation]),
        )
        .unwrap();

    assert_eq!(report.frontmatter_interpolations_applied, 0);
    // body interpolation resolves {{spec}} to {{base}}/spec.md and then
    // recursively resolves {{base}} in the same pass.
    assert!(composed.content().contains("/path/spec.md"));
}

#[test]
fn test_frontmatter_interpolation_body_still_skips_fenced_code() {
    // Inline code spans interpolate, but fenced blocks remain untouched
    // unless `interpolate_code_blocks` is set.
    let content =
        "---\nname: World\n---\nHello {{ name }}! Code: `{{ name }}`\n\n```\n{{ name }}\n```";
    let md: Markdown = content.into();
    let (composed, _) = md
        .compose_with(ComposeOptions::new().only(&[
            ComposeOperation::FrontmatterInterpolation,
            ComposeOperation::Interpolation,
        ]))
        .unwrap();

    assert!(composed.content().contains("Hello World!"));
    assert!(composed.content().contains("Code: `World`"));
    assert!(composed.content().contains("```\n{{ name }}\n```"));
}

#[test]
fn test_frontmatter_interpolation_report_counted_separately() {
    let content = "---\nbase: /path\nspec: \"{{base}}/spec.md\"\n---\nHello {{ spec }}!";
    let md: Markdown = content.into();
    let (_, report) = md
        .compose_with(ComposeOptions::new().only(&[
            ComposeOperation::FrontmatterInterpolation,
            ComposeOperation::Interpolation,
        ]))
        .unwrap();

    assert_eq!(report.frontmatter_interpolations_applied, 1);
    assert_eq!(report.interpolations_applied, 1);
}

#[test]
fn test_frontmatter_interpolation_summary() {
    let mut report = ComposeReport::new();
    report.frontmatter_interpolations_applied = 2;
    let summary = report.summary();
    assert!(summary.contains("2 frontmatter interpolation(s)"));
}

#[test]
fn test_frontmatter_interpolation_report_merge() {
    let mut r1 = ComposeReport::new();
    r1.frontmatter_interpolations_applied = 3;
    let mut r2 = ComposeReport::new();
    r2.frontmatter_interpolations_applied = 5;
    r1.merge(r2);
    assert_eq!(r1.frontmatter_interpolations_applied, 8);
}

// ── DM1: exclude-keys integration tests ───────────────────────────

#[test]
fn dm1_excluded_key_survives_raw_through_compose() {
    let content = "---\n\
        base: /root\n\
        failure:\n\
        \x20 message: \"{{err.msg}}\"\n\
        ---\n\
        body\n";
    let md: Markdown = content.into();
    let (composed, report) = md
        .compose_with(
            ComposeOptions::new()
                .only(&[ComposeOperation::FrontmatterInterpolation])
                .with_exclude_keys(["failure"]),
        )
        .unwrap();

    // The excluded key keeps its raw `{{err.msg}}` span.
    assert_eq!(
        composed.frontmatter().as_map().get("failure").unwrap().get("message"),
        Some(&serde_json::json!("{{err.msg}}")),
        "excluded key must survive raw through compose"
    );
    // The deferred-key metadata is surfaced in the report.
    assert!(
        report.deferred_frontmatter_keys.contains("failure"),
        "report must list 'failure' as deferred"
    );
}

#[test]
fn dm1_non_excluded_key_resolves_through_compose() {
    let content = "---\n\
        base: /root\n\
        summary: \"{{base}}/summary\"\n\
        failure:\n\
        \x20 message: \"{{err.msg}}\"\n\
        ---\n\
        body\n";
    let md: Markdown = content.into();
    let (composed, report) = md
        .compose_with(
            ComposeOptions::new()
                .only(&[ComposeOperation::FrontmatterInterpolation])
                .with_exclude_keys(["failure"]),
        )
        .unwrap();

    // Non-excluded key resolves normally.
    assert_eq!(
        composed.frontmatter().as_map().get("summary"),
        Some(&serde_json::json!("/root/summary"))
    );
    // Excluded key stays raw.
    assert_eq!(
        composed.frontmatter().as_map().get("failure").unwrap().get("message"),
        Some(&serde_json::json!("{{err.msg}}"))
    );
    assert_eq!(report.frontmatter_interpolations_applied, 1);
}

#[test]
fn dm1_empty_exclude_set_is_byte_identical_to_default() {
    let content = "---\nbase: /root\nspec: \"{{base}}/spec.md\"\n---\n{{spec}}";
    let md1: Markdown = content.into();
    let md2: Markdown = content.into();

    let (composed_default, _) = md1
        .compose_with(ComposeOptions::new().only(&[
            ComposeOperation::FrontmatterInterpolation,
            ComposeOperation::Interpolation,
        ]))
        .unwrap();

    let (composed_excluded, report) = md2
        .compose_with(
            ComposeOptions::new()
                .only(&[
                    ComposeOperation::FrontmatterInterpolation,
                    ComposeOperation::Interpolation,
                ])
                .with_exclude_keys(std::iter::empty::<&str>()),
        )
        .unwrap();

    assert_eq!(
        composed_default.content(),
        composed_excluded.content(),
        "empty exclude set must be byte-identical to default"
    );
    assert!(
        report.deferred_frontmatter_keys.is_empty(),
        "no keys deferred with empty exclude set"
    );
}

#[test]
fn dm1a_composed_key_referencing_deferred_fails_through_compose() {
    let content = "---\n\
        summary: \"{{ failure.message }}\"\n\
        failure:\n\
        \x20 message: \"{{err.msg}}\"\n\
        ---\n\
        body\n";
    let md: Markdown = content.into();
    let result = md.compose_with(
        ComposeOptions::new()
            .only(&[ComposeOperation::FrontmatterInterpolation])
            .with_exclude_keys(["failure"]),
    );

    let err = result.unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("summary"), "error names the composed key: {msg}");
    assert!(
        msg.contains("failure"),
        "error names the deferred key: {msg}"
    );
}

// ── DM2: subtree compose integration tests ──────────────────────────

#[test]
fn dm2_subtree_resolves_injected_eager_and_lazy_globals() {
    use super::subtree::{InjectedGlobal, SubtreeStrictness, compose_subtree};
    use crate::markdown::compose::EffectiveStateBuilder;
    use std::collections::HashMap;

    let fm: HashMap<String, serde_json::Value> =
        [("phase".to_string(), serde_json::json!(2))].into();
    let state = EffectiveStateBuilder::new()
        .with_frontmatter(fm)
        .build()
        .unwrap();

    let mut globals = HashMap::new();
    globals.insert(
        "err".to_string(),
        InjectedGlobal::eager(serde_json::json!({"msg": "disk full"})),
    );
    globals.insert(
        "current".to_string(),
        InjectedGlobal::lazy(|| serde_json::json!({"ctx": {"today": "2026-06-24"}})),
    );

    let result = compose_subtree(
        &serde_json::json!("phase {{phase}} failed: {{err.msg}} on {{current.ctx.today}}"),
        &state,
        globals,
        SubtreeStrictness::Lenient,
    )
    .unwrap();

    assert_eq!(
        result,
        serde_json::json!("phase 2 failed: disk full on 2026-06-24")
    );
}

#[test]
fn dm2_subtree_layered_seed_state_still_resolves() {
    use super::subtree::{SubtreeStrictness, compose_subtree};
    use crate::markdown::compose::EffectiveStateBuilder;
    use std::collections::HashMap;

    let fm: HashMap<String, serde_json::Value> = [
        ("phase".to_string(), serde_json::json!(3)),
        (
            "config".to_string(),
            serde_json::json!({"artifact": {"path": "/tmp/out"}}),
        ),
    ]
    .into();
    let state = EffectiveStateBuilder::new()
        .with_frontmatter(fm)
        .build()
        .unwrap();

    let result = compose_subtree(
        &serde_json::json!("artifact={{config.artifact.path}} phase={{phase}}"),
        &state,
        HashMap::new(),
        SubtreeStrictness::Lenient,
    )
    .unwrap();
    assert_eq!(result, serde_json::json!("artifact=/tmp/out phase=3"));
}

#[test]
fn dm2_subtree_parity_with_main_compose_whole_value() {
    // A whole-value single `{{ expr }}` yields the same typed Value in subtree
    // compose as main compose's frontmatter interpolation does.
    use super::subtree::{SubtreeStrictness, compose_subtree};
    use crate::markdown::compose::EffectiveStateBuilder;
    use std::collections::HashMap;

    let fm: HashMap<String, serde_json::Value> =
        [("count".to_string(), serde_json::json!(5))].into();
    let state = EffectiveStateBuilder::new()
        .with_frontmatter(fm)
        .build()
        .unwrap();

    // Whole-value span: typed Number result, not a string.
    let result = compose_subtree(
        &serde_json::json!("{{count}}"),
        &state,
        HashMap::new(),
        SubtreeStrictness::Strict,
    )
    .unwrap();
    assert_eq!(result, serde_json::json!(5));
}

#[test]
fn dm2_subtree_parity_with_main_compose_mixed_string() {
    use super::subtree::{SubtreeStrictness, compose_subtree};
    use crate::markdown::compose::EffectiveStateBuilder;
    use std::collections::HashMap;

    let fm: HashMap<String, serde_json::Value> =
        [("count".to_string(), serde_json::json!(5))].into();
    let state = EffectiveStateBuilder::new()
        .with_frontmatter(fm)
        .build()
        .unwrap();

    let result = compose_subtree(
        &serde_json::json!("count={{count}}"),
        &state,
        HashMap::new(),
        SubtreeStrictness::Strict,
    )
    .unwrap();
    assert_eq!(result, serde_json::json!("count=5"));
}

#[test]
fn dm2_subtree_lazy_global_only_evaluated_when_referenced() {
    use super::subtree::{InjectedGlobal, SubtreeStrictness, compose_subtree};
    use crate::markdown::compose::EffectiveStateBuilder;
    use std::collections::HashMap;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    let state = EffectiveStateBuilder::new().build().unwrap();
    let count = Arc::new(AtomicUsize::new(0));
    let count_for_closure = count.clone();
    let mut globals = HashMap::new();
    globals.insert(
        "current".to_string(),
        InjectedGlobal::lazy(move || {
            count_for_closure.fetch_add(1, Ordering::SeqCst);
            serde_json::json!({"phase": 1})
        }),
    );

    // String does NOT reference `current`: closure must not run.
    let result = compose_subtree(
        &serde_json::json!("no reference"),
        &state,
        globals,
        SubtreeStrictness::Lenient,
    )
    .unwrap();
    assert_eq!(result, serde_json::json!("no reference"));
    assert_eq!(count.load(Ordering::SeqCst), 0);
}

#[test]
fn dm2_subtree_lazy_global_evaluated_at_most_once() {
    use super::subtree::{InjectedGlobal, SubtreeStrictness, compose_subtree};
    use crate::markdown::compose::EffectiveStateBuilder;
    use std::collections::HashMap;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    let state = EffectiveStateBuilder::new().build().unwrap();
    let count = Arc::new(AtomicUsize::new(0));
    let count_for_closure = count.clone();
    let mut globals = HashMap::new();
    globals.insert(
        "current".to_string(),
        InjectedGlobal::lazy(move || {
            count_for_closure.fetch_add(1, Ordering::SeqCst);
            serde_json::json!({"phase": 7})
        }),
    );

    // Two references to `current.phase`: closure runs at most once.
    let result = compose_subtree(
        &serde_json::json!("{{current.phase}} then {{current.phase}}"),
        &state,
        globals,
        SubtreeStrictness::Lenient,
    )
    .unwrap();
    assert_eq!(result, serde_json::json!("7 then 7"));
    assert_eq!(count.load(Ordering::SeqCst), 1);
}

#[test]
fn dm2_subtree_strict_rejects_unknown_root() {
    use super::subtree::{SubtreeStrictness, compose_subtree};
    use crate::markdown::compose::EffectiveStateBuilder;
    use std::collections::HashMap;

    let fm: HashMap<String, serde_json::Value> =
        [("phase".to_string(), serde_json::json!(2))].into();
    let state = EffectiveStateBuilder::new()
        .with_frontmatter(fm)
        .build()
        .unwrap();

    let err = compose_subtree(
        &serde_json::json!("{{spec_fil}}"),
        &state,
        HashMap::new(),
        SubtreeStrictness::Strict,
    )
    .unwrap_err()
    .to_string();

    assert!(err.contains("unknown root"), "error: {err}");
    assert!(err.contains("spec_fil"), "error names the typo: {err}");
}

#[test]
fn dm2_subtree_strict_known_but_empty_renders_empty() {
    use super::subtree::{SubtreeStrictness, compose_subtree};
    use crate::markdown::compose::EffectiveStateBuilder;
    use std::collections::HashMap;

    let fm: HashMap<String, serde_json::Value> =
        [("spec_file".to_string(), serde_json::Value::Null)].into();
    let state = EffectiveStateBuilder::new()
        .with_frontmatter(fm)
        .build()
        .unwrap();

    // `spec_file` is a known root that resolves to null: renders empty.
    let result = compose_subtree(
        &serde_json::json!("spec={{spec_file}}"),
        &state,
        HashMap::new(),
        SubtreeStrictness::Strict,
    )
    .unwrap();
    assert_eq!(result, serde_json::json!("spec="));
}

#[test]
fn dm2_subtree_strict_rejects_malformed_span() {
    use super::subtree::{SubtreeStrictness, compose_subtree};
    use crate::markdown::compose::EffectiveStateBuilder;
    use std::collections::HashMap;

    let state = EffectiveStateBuilder::new().build().unwrap();

    let err = compose_subtree(
        &serde_json::json!("{{ > broken }}"),
        &state,
        HashMap::new(),
        SubtreeStrictness::Strict,
    )
    .unwrap_err()
    .to_string();

    assert!(err.contains("failed to parse"), "error: {err}");
}

#[test]
fn dm2_subtree_strict_rejects_unknown_function() {
    use super::subtree::{SubtreeStrictness, compose_subtree};
    use crate::markdown::compose::EffectiveStateBuilder;
    use std::collections::HashMap;

    let fm: HashMap<String, serde_json::Value> =
        [("phase".to_string(), serde_json::json!(2))].into();
    let state = EffectiveStateBuilder::new()
        .with_frontmatter(fm)
        .build()
        .unwrap();

    let err = compose_subtree(
        &serde_json::json!("{{ bogus_fn(phase) }}"),
        &state,
        HashMap::new(),
        SubtreeStrictness::Strict,
    )
    .unwrap_err()
    .to_string();

    assert!(
        err.contains("Unknown function") || err.to_lowercase().contains("bogus_fn"),
        "error names the unknown function: {err}"
    );
}

#[test]
fn dm2_subtree_strict_rejects_unknown_root_in_function_argument() {
    // The strict root check walks the AST, so a typo buried in a function
    // argument also fails.
    use super::subtree::{SubtreeStrictness, compose_subtree};
    use crate::markdown::compose::EffectiveStateBuilder;
    use std::collections::HashMap;

    let fm: HashMap<String, serde_json::Value> =
        [("phase".to_string(), serde_json::json!(2))].into();
    let state = EffectiveStateBuilder::new()
        .with_frontmatter(fm)
        .build()
        .unwrap();

    let err = compose_subtree(
        &serde_json::json!("{{ parent_dir(typo_var) }}"),
        &state,
        HashMap::new(),
        SubtreeStrictness::Strict,
    )
    .unwrap_err()
    .to_string();

    assert!(err.contains("unknown root"), "error: {err}");
    assert!(err.contains("typo_var"), "error names the typo: {err}");
}

// ── Nested external state regression tests ────────────────────────

#[test]
fn test_frontmatter_interpolation_nested_external_state() {
    // External state has nested keys; frontmatter references them.
    let content = "---\nmeta:\n  author: Local\nspec: \"{{meta.base}}/spec.md\"\n---\n{{spec}}";
    let md: Markdown = content.into();
    let (composed, report) = md
        .compose_with(
            ComposeOptions::new()
                .with_external_state(serde_json::json!({
                    "meta": {"base": "/root", "author": "Parent"}
                }))
                .only(&[
                    ComposeOperation::FrontmatterInterpolation,
                    ComposeOperation::Interpolation,
                ]),
        )
        .unwrap();

    // meta.base from external state should be deep-merged in
    assert!(
        composed.content().contains("/root/spec.md"),
        "Expected /root/spec.md but got: {}",
        composed.content()
    );
    // frontmatter author should win over external
    assert_eq!(
        composed
            .frontmatter()
            .as_map()
            .get("meta")
            .and_then(|v| v.get("author")),
        Some(&serde_json::json!("Local"))
    );
    assert!(report.frontmatter_interpolations_applied >= 1);
}

#[test]
fn test_external_state_deep_merge_preserves_frontmatter_values() {
    // Both frontmatter and external have nested objects; frontmatter wins on conflict.
    let content =
        "---\nconfig:\n  theme: dark\n---\ntheme={{config.theme}} lang={{config.lang}}";
    let md: Markdown = content.into();
    let (composed, _) = md
        .compose_with(
            ComposeOptions::new()
                .with_external_state(serde_json::json!({
                    "config": {"theme": "light", "lang": "en"}
                }))
                .only(&[ComposeOperation::Interpolation]),
        )
        .unwrap();

    assert!(
        composed.content().contains("theme=dark"),
        "Frontmatter should win: {}",
        composed.content()
    );
    assert!(
        composed.content().contains("lang=en"),
        "External nested key should fill in: {}",
        composed.content()
    );
}

// ── Child document frontmatter from parent state ──────────────────

#[test]
fn test_child_frontmatter_interpolation_from_parent_state() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("root.md");
    let child = dir.path().join("child.md");

    std::fs::write(&root, "---\nbase: /docs\n---\n::file ./child.md").unwrap();
    std::fs::write(
        &child,
        "---\nspec: \"{{base}}/spec.md\"\n---\nSpec: {{spec}}",
    )
    .unwrap();

    let md = Markdown::try_from(root.as_path()).unwrap();
    let options = ComposeOptions::new().with_source_file(root);
    let (composed, _) = md.compose_with(options).unwrap();

    assert!(
        composed.content().contains("Spec: /docs/spec.md"),
        "Child should derive frontmatter from parent state: {}",
        composed.content()
    );
}

// ── Interpolated prologue/epilogue paths ──────────────────────────

#[test]
fn test_interpolated_prologue_path() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("root.md");
    let intro = dir.path().join("intro.md");

    std::fs::write(
        &root,
        "---\nparts: .\nprologue: \"{{parts}}/intro.md\"\n---\nBody",
    )
    .unwrap();
    std::fs::write(&intro, "Prologue content").unwrap();

    let md = Markdown::try_from(root.as_path()).unwrap();
    let options = ComposeOptions::new().with_source_file(root);
    let (composed, report) = md.compose_with(options).unwrap();

    assert!(
        composed.content().contains("Prologue content"),
        "Interpolated prologue path should resolve: {}",
        composed.content()
    );
    assert!(report.frontmatter_interpolations_applied >= 1);
}

#[test]
fn test_interpolated_epilogue_path() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("root.md");
    let outro = dir.path().join("outro.md");

    std::fs::write(
        &root,
        "---\nparts: .\nepilogue: \"{{parts}}/outro.md\"\n---\nBody",
    )
    .unwrap();
    std::fs::write(&outro, "Epilogue content").unwrap();

    let md = Markdown::try_from(root.as_path()).unwrap();
    let options = ComposeOptions::new().with_source_file(root);
    let (composed, report) = md.compose_with(options).unwrap();

    assert!(
        composed.content().contains("Epilogue content"),
        "Interpolated epilogue path should resolve: {}",
        composed.content()
    );
    assert!(report.frontmatter_interpolations_applied >= 1);
}

// ── Page blocks consuming interpolated frontmatter values ─────────

#[test]
fn test_page_block_uses_interpolated_frontmatter() {
    // Frontmatter interpolation produces a value that page blocks consume.
    let content = "---\nbase: show\nflag: \"{{base}}\"\n---\n\n::block when=\"flag\"\n\nVisible\n\n::end-block\n";
    let md: Markdown = content.into();

    let options = ComposeOptions::new().only(&[
        ComposeOperation::FrontmatterInterpolation,
        ComposeOperation::PageBlocks,
    ]);
    let (composed, report) = md.compose_with(options).unwrap();

    assert!(
        composed.content().contains("Visible"),
        "Page block should see interpolated frontmatter value: {}",
        composed.content()
    );
    assert!(report.frontmatter_interpolations_applied >= 1);
    assert!(report.page_blocks_rendered >= 1);
}

#[test]
fn test_page_block_false_from_interpolated_frontmatter() {
    let content = "---\nbase: \"\"\nflag: \"{{base}}\"\n---\n\n::block when=\"flag\"\n\nHidden\n\n::end-block\n\nAfter\n";
    let md: Markdown = content.into();

    let options = ComposeOptions::new().only(&[
        ComposeOperation::FrontmatterInterpolation,
        ComposeOperation::PageBlocks,
    ]);
    let (composed, _) = md.compose_with(options).unwrap();

    assert!(
        !composed.content().contains("Hidden"),
        "Page block with falsy interpolated value should be removed: {}",
        composed.content()
    );
    assert!(composed.content().contains("After"));
}

mod frontmatter_shell_expansion_integration {
    use super::*;
    use crate::markdown::compose::shell_expansion::types::{
        ShellApprovalDecision, ShellApprovalHandler, ShellApprovalRequest, ShellExpansionError,
        ShellExpansionOptions,
    };
    use std::sync::Arc;
    use std::time::Duration;
    use tempfile::TempDir;

    struct MockApproval;
    impl ShellApprovalHandler for MockApproval {
        fn approve(
            &self,
            _req: ShellApprovalRequest,
        ) -> Result<ShellApprovalDecision, ShellExpansionError> {
            Ok(ShellApprovalDecision::AllowOnce)
        }
    }

    #[test]
    fn frontmatter_shell_output_visible_to_body_interpolation() {
        let temp_dir = TempDir::new().unwrap();
        let content = "---\ngreeting: \"$(echo hello)\"\n---\nMessage: {{greeting}}\n";
        let md: Markdown = content.into();

        let options = ComposeOptions::new()
            .only(&[
                ComposeOperation::FrontmatterInterpolation,
                ComposeOperation::FrontmatterShellExpansion,
                ComposeOperation::Interpolation,
            ])
            .with_shell(ShellExpansionOptions {
                policy_root: Some(temp_dir.path().to_path_buf()),
                approval_handler: Some(Arc::new(MockApproval)),
                ..Default::default()
            });

        let (composed, report) = md.compose_with(options).unwrap();
        assert!(
            composed.content().contains("Message: hello"),
            "Expected 'Message: hello' in:\n{}",
            composed.content()
        );
        assert_eq!(report.frontmatter_shell_expansions_applied, 1);
    }

    #[test]
    fn frontmatter_interpolation_feeds_into_shell_expansion() {
        let temp_dir = TempDir::new().unwrap();
        let content = "---\nfile: README.md\ndir: \"$(dirname {{file}})\"\n---\nDir: {{dir}}\n";
        let md: Markdown = content.into();

        let options = ComposeOptions::new()
            .only(&[
                ComposeOperation::FrontmatterInterpolation,
                ComposeOperation::FrontmatterShellExpansion,
                ComposeOperation::Interpolation,
            ])
            .with_shell(ShellExpansionOptions {
                policy_root: Some(temp_dir.path().to_path_buf()),
                approval_handler: Some(Arc::new(MockApproval)),
                ..Default::default()
            });

        let (composed, report) = md.compose_with(options).unwrap();
        // dirname README.md returns "."
        assert!(
            composed.content().contains("Dir: ."),
            "Expected 'Dir: .' in:\n{}",
            composed.content()
        );
        assert_eq!(report.frontmatter_shell_expansions_applied, 1);
    }

    #[test]
    fn body_and_frontmatter_shell_coexist() {
        let temp_dir = TempDir::new().unwrap();
        let content =
            "---\nfm_val: \"$(echo from-frontmatter)\"\n---\n::shell echo from-body\n";
        let md: Markdown = content.into();

        let options = ComposeOptions::new()
            .only(&[
                ComposeOperation::FrontmatterShellExpansion,
                ComposeOperation::ShellExpansion,
            ])
            .with_shell(ShellExpansionOptions {
                policy_root: Some(temp_dir.path().to_path_buf()),
                approval_handler: Some(Arc::new(MockApproval)),
                ..Default::default()
            });

        let (composed, report) = md.compose_with(options).unwrap();
        assert_eq!(report.frontmatter_shell_expansions_applied, 1);
        assert_eq!(report.shell_expansions_applied, 1);
        assert!(composed.content().contains("from-body"));
    }

    #[test]
    fn frontmatter_shell_with_no_candidates_is_noop() {
        let content = "---\ntitle: Hello\n---\nBody text\n";
        let md: Markdown = content.into();

        let options =
            ComposeOptions::new().only(&[ComposeOperation::FrontmatterShellExpansion]);

        let (composed, report) = md.compose_with(options).unwrap();
        assert_eq!(report.frontmatter_shell_expansions_applied, 0);
        assert!(composed.content().contains("Body text"));
    }

    #[test]
    fn frontmatter_shell_timeout_empty_emits_warning() {
        let temp_dir = TempDir::new().unwrap();
        let content = "---\nval: \"$(sleep 1)\"\n---\nValue: {{val}}\n";
        let md: Markdown = content.into();

        let options = ComposeOptions::new()
            .only(&[
                ComposeOperation::FrontmatterShellExpansion,
                ComposeOperation::Interpolation,
            ])
            .with_shell(ShellExpansionOptions {
                timeout: Duration::from_millis(100),
                timeout_behavior: super::ShellTimeoutBehavior::EmptyString,
                policy_root: Some(temp_dir.path().to_path_buf()),
                approval_handler: Some(Arc::new(MockApproval)),
                ..Default::default()
            });

        let (composed, report) = md.compose_with(options).unwrap();
        assert!(composed.content().contains("Value: "));
        assert_eq!(report.frontmatter_shell_expansions_applied, 1);
        assert_eq!(report.warnings.len(), 1);
        assert!(report.warnings[0].message.contains("timed out"));
    }

    #[test]
    fn frontmatter_shell_rejects_interpolated_executable() {
        let content = "---\ncmd_name: echo\nval: \"$({{cmd_name}} hello)\"\n---\n";
        let md: Markdown = content.into();

        let options = ComposeOptions::new().only(&[
            ComposeOperation::FrontmatterInterpolation,
            ComposeOperation::FrontmatterShellExpansion,
        ]);

        let err = md.compose_with(options).unwrap_err();
        assert!(
            err.to_string()
                .contains("Frontmatter shell executable may not come from interpolation")
        );
    }

    #[test]
    fn frontmatter_shell_rejects_pipe_in_command() {
        let temp_dir = TempDir::new().unwrap();
        let content = "---\nval: \"$(echo a | cat)\"\n---\n";
        let md: Markdown = content.into();

        let options = ComposeOptions::new()
            .only(&[ComposeOperation::FrontmatterShellExpansion])
            .with_shell(ShellExpansionOptions {
                policy_root: Some(temp_dir.path().to_path_buf()),
                approval_handler: Some(Arc::new(MockApproval)),
                ..Default::default()
            });

        let err = md.compose_with(options).unwrap_err();
        assert!(
            err.to_string().contains("pipes") || err.to_string().contains("Shell pipes"),
            "Expected shell pipe rejection, got: {}",
            err
        );
    }

    #[test]
    fn frontmatter_shell_or_chain_works() {
        let temp_dir = TempDir::new().unwrap();
        let content = "---\nval: \"$(false || echo fallback)\"\n---\n";
        let md: Markdown = content.into();

        let options = ComposeOptions::new()
            .only(&[ComposeOperation::FrontmatterShellExpansion])
            .with_shell(ShellExpansionOptions {
                policy_root: Some(temp_dir.path().to_path_buf()),
                approval_handler: Some(Arc::new(MockApproval)),
                ..Default::default()
            });

        let (composed, _report) = md.compose_with(options).unwrap();
        assert_eq!(
            composed.frontmatter().as_map().get("val"),
            Some(&serde_json::json!("fallback"))
        );
    }

    #[test]
    fn ternary_motivating_workflow_true_branch_through_full_pipeline() {
        // Review finding 4: exercise the motivating spec_file workflow
        // through the full compose pipeline so frontmatter interpolation,
        // pre-interpolation snapshot capture, and frontmatter shell
        // expansion are all wired together. With `has_spec: true` the
        // then-branch wins and produces the basename of the spec path.
        let temp_dir = TempDir::new().unwrap();
        let content = concat!(
            "---\n",
            "has_spec: true\n",
            "spec: /tmp/example-spec.md\n",
            "spec_file: \"$({{has_spec}} ? basename {{spec}} : '')\"\n",
            "---\n",
            "Spec: {{spec_file}}\n",
        );
        let md: Markdown = content.into();

        let options = ComposeOptions::new()
            .only(&[
                ComposeOperation::FrontmatterInterpolation,
                ComposeOperation::FrontmatterShellExpansion,
                ComposeOperation::Interpolation,
            ])
            .with_shell(ShellExpansionOptions {
                policy_root: Some(temp_dir.path().to_path_buf()),
                approval_handler: Some(Arc::new(MockApproval)),
                ..Default::default()
            });

        let (composed, report) = md.compose_with(options).unwrap();
        assert_eq!(report.frontmatter_shell_expansions_applied, 1);
        assert_eq!(
            composed.frontmatter().as_map().get("spec_file"),
            Some(&serde_json::json!("example-spec.md"))
        );
        assert!(
            composed.content().contains("Spec: example-spec.md"),
            "Expected body to interpolate spec_file, got:\n{}",
            composed.content()
        );
    }

    #[test]
    fn ternary_motivating_workflow_false_branch_through_full_pipeline() {
        // Counterpart to the true-branch test: with `has_spec: false`
        // the else-branch (`''`) wins, short-circuiting to an empty
        // string without invoking the shell.
        let temp_dir = TempDir::new().unwrap();
        let content = concat!(
            "---\n",
            "has_spec: false\n",
            "spec: /tmp/example-spec.md\n",
            "spec_file: \"$({{has_spec}} ? basename {{spec}} : '')\"\n",
            "---\n",
            "Spec: {{spec_file}}\n",
        );
        let md: Markdown = content.into();

        let options = ComposeOptions::new()
            .only(&[
                ComposeOperation::FrontmatterInterpolation,
                ComposeOperation::FrontmatterShellExpansion,
                ComposeOperation::Interpolation,
            ])
            .with_shell(ShellExpansionOptions {
                policy_root: Some(temp_dir.path().to_path_buf()),
                approval_handler: Some(Arc::new(MockApproval)),
                ..Default::default()
            });

        let (composed, report) = md.compose_with(options).unwrap();
        assert_eq!(report.frontmatter_shell_expansions_applied, 1);
        assert_eq!(
            composed.frontmatter().as_map().get("spec_file"),
            Some(&serde_json::json!(""))
        );
        assert!(
            composed.content().contains("Spec: "),
            "Expected body to render with empty spec_file, got:\n{}",
            composed.content()
        );
    }

    #[test]
    fn frontmatter_false_flows_to_shell_else_branch_in_pipeline() {
        // Compose level: a whole-value `{{raw_false}}` keeps `has_spec` a
        // real boolean `false` (type-preserving interpolation). Embedded
        // into the `$(...)` shell value it stringifies to `false`, and the
        // shell branch resolves to the empty string.
        let temp_dir = TempDir::new().unwrap();
        let content = concat!(
            "---\n",
            "raw_false: false\n",
            "has_spec: \"{{raw_false}}\"\n",
            "spec_file: \"$({{has_spec}} ? echo present : '')\"\n",
            "---\n",
        );
        let md: Markdown = content.into();

        let options = ComposeOptions::new()
            .only(&[
                ComposeOperation::FrontmatterInterpolation,
                ComposeOperation::FrontmatterShellExpansion,
            ])
            .with_shell(ShellExpansionOptions {
                policy_root: Some(temp_dir.path().to_path_buf()),
                approval_handler: Some(Arc::new(MockApproval)),
                ..Default::default()
            });

        let (composed, _report) = md.compose_with(options).unwrap();
        // has_spec is preserved as a real boolean `false` (whole-value
        // interpolation), and the shell branch resolves to empty.
        assert_eq!(
            composed.frontmatter().as_map().get("has_spec"),
            Some(&serde_json::json!(false))
        );
        assert_eq!(
            composed.frontmatter().as_map().get("spec_file"),
            Some(&serde_json::json!(""))
        );
    }
}

mod infix_logic_conditions {
    use super::*;

    fn compose_with_page_blocks(content: &str) -> (String, ComposeReport) {
        let md: Markdown = content.into();
        let options = ComposeOptions::new().only(&[ComposeOperation::PageBlocks]);
        let (composed, report) = md.compose_with(options).unwrap();
        (composed.content().to_string(), report)
    }

    #[test]
    fn page_block_with_infix_and_true() {
        let content =
            "---\na: true\nb: true\n---\n::block when=\"a && b\"\ninside\n::end-block\n";
        let (output, report) = compose_with_page_blocks(content);
        assert!(output.contains("inside"));
        assert_eq!(report.page_blocks_rendered, 1);
        assert_eq!(report.page_blocks_skipped, 0);
    }

    #[test]
    fn page_block_with_infix_and_false() {
        let content =
            "---\na: true\nb: false\n---\n::block when=\"a && b\"\ninside\n::end-block\n";
        let (output, report) = compose_with_page_blocks(content);
        assert!(!output.contains("inside"));
        assert_eq!(report.page_blocks_rendered, 0);
        assert_eq!(report.page_blocks_skipped, 1);
    }

    #[test]
    fn page_block_with_infix_or_one_true() {
        let content =
            "---\na: false\nb: true\n---\n::block when=\"a || b\"\ninside\n::end-block\n";
        let (output, report) = compose_with_page_blocks(content);
        assert!(output.contains("inside"));
        assert_eq!(report.page_blocks_rendered, 1);
    }

    #[test]
    fn page_block_with_infix_or_both_false() {
        let content =
            "---\na: false\nb: false\n---\n::block when=\"a || b\"\ninside\n::end-block\n";
        let (output, _report) = compose_with_page_blocks(content);
        assert!(!output.contains("inside"));
    }

    #[test]
    fn page_block_with_grouped_precedence() {
        // (a || b) && c — grouping overrides default precedence
        let content = "---\na: false\nb: true\nc: true\n---\n::block when=\"(a || b) && c\"\ninside\n::end-block\n";
        let (output, _report) = compose_with_page_blocks(content);
        assert!(output.contains("inside"));

        let content_false = "---\na: false\nb: true\nc: false\n---\n::block when=\"(a || b) && c\"\ninside\n::end-block\n";
        let (output, _report) = compose_with_page_blocks(content_false);
        assert!(!output.contains("inside"));
    }

    #[test]
    fn page_block_with_chained_or() {
        // Chained `||` in condition mode evaluates as logical OR
        let content = "---\na: false\nb: false\nc: true\n---\n::block when=\"a || b || c\"\ninside\n::end-block\n";
        let (output, _report) = compose_with_page_blocks(content);
        assert!(output.contains("inside"));
    }

    #[test]
    fn transclusion_directive_with_mixed_infix_logic() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root.md");
        let child = dir.path().join("child.md");

        // Child loaded only if (enabled || fallback) && !skip
        std::fs::write(&child, "child body").unwrap();
        std::fs::write(
            &root,
            "---\nenabled: true\nskip: false\n---\nbefore\n\n::file child.md when=\"enabled && !skip\"\n\nafter\n",
        )
        .unwrap();

        let options = ComposeOptions::new()
            .with_source_file(&root)
            .only(&[ComposeOperation::BlockTransclusion]);

        let (composed, _) = Markdown::try_from(root.as_path())
            .unwrap()
            .compose_with(options)
            .unwrap();
        assert!(composed.content().contains("child body"));
    }

    #[test]
    fn transclusion_skipped_when_infix_condition_false() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root.md");
        let child = dir.path().join("child.md");

        std::fs::write(&child, "child body").unwrap();
        std::fs::write(
            &root,
            "---\nenabled: true\nskip: true\n---\nbefore\n\n::file child.md when=\"enabled && !skip\"\n\nafter\n",
        )
        .unwrap();

        let options = ComposeOptions::new()
            .with_source_file(&root)
            .only(&[ComposeOperation::BlockTransclusion]);

        let (composed, _) = Markdown::try_from(root.as_path())
            .unwrap()
            .compose_with(options)
            .unwrap();
        assert!(!composed.content().contains("child body"));
    }

    #[test]
    fn page_block_with_bare_pipe_fails_parse() {
        // Bare `|` in condition expressions should produce a parse error
        let content = "---\na: true\n---\n::block when=\"a | b\"\ninside\n::end-block\n";
        let md: Markdown = content.into();
        let options = ComposeOptions::new().only(&[ComposeOperation::PageBlocks]);
        let err = md.compose_with(options).unwrap_err();

        let err_string = format!("{}", err);
        assert!(
            err_string.contains("Unexpected '|'") || err_string.contains("logical OR"),
            "Expected bare pipe error in condition, got: {}",
            err_string
        );
    }
}

// ============================================
// Schema Validation integration tests
// ============================================

mod schema_validation_integration {
    use super::*;

    #[test]
    fn schema_validation_fails_fast_before_shell_expansion() {
        // Document matching the shape of the failing planner prompt:
        // spec is empty, and dir uses shell expansion that would fail
        // if spec stays empty.
        let content = "---\n$schema:\n  spec: 'file(required)'\nspec: \"\"\ndir: \"$(dirname '{{ spec }}')\"\n---\nBody\n";
        let md: Markdown = content.into();

        let options = ComposeOptions::new().only(&[
            ComposeOperation::FrontmatterInterpolation,
            ComposeOperation::FrontmatterShellExpansion,
            ComposeOperation::Interpolation,
        ]);

        let err = md.compose_with(options).unwrap_err();
        let err_string = format!("{err}");
        assert!(
            err_string.contains("Schema validation failed"),
            "Expected schema validation error, got: {err_string}"
        );
        assert!(
            !err_string.contains("dirname"),
            "Shell expansion should not have run, got: {err_string}"
        );

        // The error variant itself should name the failing property.
        match err {
            MarkdownError::SchemaValidationFailed { problems, .. } => {
                assert!(
                    problems.iter().any(|p| {
                        p.property.as_deref() == Some("spec") || p.path == "/spec"
                    }),
                    "Error should mention the spec property, got: {problems:?}"
                );
            }
            other => panic!("Expected SchemaValidationFailed, got {other:?}"),
        }
    }

    #[test]
    fn schema_violation_on_shell_value_reported_when_shell_expansion_disabled() {
        // A `$(...)` frontmatter value violates the schema, but
        // FrontmatterShellExpansion is NOT in the enabled set. Because no
        // later stage will expand or re-validate `spec`, the violation must
        // surface here rather than being deferred and silently accepted.
        let content =
            "---\n$schema:\n  spec: 'number(required)'\nspec: \"$(echo 1)\"\n---\nBody\n";
        let md: Markdown = content.into();

        let options = ComposeOptions::new().only(&[ComposeOperation::Interpolation]);

        let err = md.compose_with(options).unwrap_err();
        match err {
            MarkdownError::SchemaValidationFailed { problems, .. } => {
                assert!(
                    problems
                        .iter()
                        .any(|p| p.property.as_deref() == Some("spec") || p.path == "/spec"),
                    "Error should mention the spec property, got: {problems:?}"
                );
            }
            other => panic!("Expected SchemaValidationFailed, got {other:?}"),
        }
    }

    #[test]
    fn schema_validation_reports_zero_shell_replacements() {
        let content = "---\n$schema:\n  spec: 'file(required)'\nspec: \"\"\ndir: \"$(dirname '{{ spec }}')\"\n---\nBody\n";
        let md: Markdown = content.into();

        // Even with fail_fast=false, schema validation is a hard error.
        let options = ComposeOptions::new()
            .only(&[
                ComposeOperation::FrontmatterInterpolation,
                ComposeOperation::FrontmatterShellExpansion,
                ComposeOperation::Interpolation,
            ])
            .with_fail_fast(false);

        let err = md.compose_with(options).unwrap_err();
        match err {
            MarkdownError::SchemaValidationFailed { .. } => {}
            other => panic!("Expected SchemaValidationFailed, got {other:?}"),
        }
    }

    #[test]
    fn coercion_write_back_flows_to_composed_frontmatter() {
        // `has_spec` derives from a ternary, resolves to the string "true"
        // during frontmatter interpolation, and is coerced to a real JSON
        // bool by schema validation. The composed frontmatter must hold the
        // bool, not the string.
        let content = "---\n$schema:\n  spec: string(required)\n  has_spec: boolean\nspec: design.md\nhas_spec: \"{{spec ? true : false}}\"\n---\nBody\n";
        let md: Markdown = content.into();

        let options = ComposeOptions::new().only(&[
            ComposeOperation::FrontmatterInterpolation,
            ComposeOperation::Interpolation,
        ]);

        let (composed, _report) = md.compose_with(options).unwrap();
        assert_eq!(
            composed.frontmatter().as_map().get("has_spec"),
            Some(&serde_json::json!(true))
        );
    }

    #[test]
    fn implement_md_three_arm_union_ternaries_coerce_and_defer_shell() {
        // Faithful reproduction of the original failing `claudine compose
        // prompts/implement.md spec=… --claude` invocation: a 3-arm root
        // union where every arm types the `has_*` trio as strict `boolean`,
        // computed `has_*` ternaries that render into quoted scalars
        // ("true"/"false"), and a `$(...)`-bearing `dir`. A `spec=` value is
        // supplied via --set, so arm 2 (`spec: string(required)`) validates
        // post-coercion. Before this feature the strict `boolean` arms
        // rejected the "false"/"true" strings; now they coerce.
        //
        // Frontmatter shell expansion is left disabled to keep the test
        // hermetic (no real `dirname` invocation). `dir` is typed `string`,
        // so its literal `$(...)` value is already a valid string: coercion
        // skips it and validation raises no type problem, so it survives
        // untouched into the composed output as a deferred shell expression.
        let content = "---\n\
            $schema:\n\
            \x20 - review: string(required)\n\
            \x20   spec: string\n\
            \x20   iteration: number\n\
            \x20   has_plan: boolean\n\
            \x20   has_spec: boolean\n\
            \x20   has_review: boolean\n\
            \x20 - spec: string(required)\n\
            \x20   has_plan: boolean\n\
            \x20   has_spec: boolean\n\
            \x20   has_review: boolean\n\
            \x20 - plan: string(required)\n\
            \x20   spec: string\n\
            \x20   iteration: number\n\
            \x20   has_plan: boolean\n\
            \x20   has_spec: boolean\n\
            \x20   has_review: boolean\n\
            has_spec: \"{{spec ? true : false}}\"\n\
            has_plan: \"{{plan ? true : false}}\"\n\
            has_review: \"{{review ? true : false}}\"\n\
            dir: \"$(dirname '{{spec || plan}}')\"\n\
            ---\nBody\n";
        let md: Markdown = content.into();

        // `spec=` provided via --set; no `plan`/`review` → second arm wins.
        let options = ComposeOptions::new()
            .with_set_overrides(serde_json::json!({ "spec": "features/plan.md" }))
            .only(&[
                ComposeOperation::FrontmatterInterpolation,
                ComposeOperation::Interpolation,
            ]);

        let (composed, _report) = md
            .compose_with(options)
            .expect("compose should succeed once the has_* strings coerce");

        let fm = composed.frontmatter();
        let map = fm.as_map();
        // The motivating fix: the ternary-derived strings become real bools.
        assert_eq!(map.get("has_spec"), Some(&serde_json::json!(true)));
        assert_eq!(map.get("has_plan"), Some(&serde_json::json!(false)));
        assert_eq!(map.get("has_review"), Some(&serde_json::json!(false)));
        // The `$(...)` `dir` value is deferred: coercion skips it pre-shell,
        // and the unresolved interpolation/shell template never errored.
        let dir = map.get("dir").and_then(serde_json::Value::as_str).unwrap();
        assert!(
            dir.contains("$(") && dir.contains("dirname"),
            "dir should remain a deferred shell expression, got: {dir}"
        );
    }

    #[test]
    fn parent_set_overlay_satisfies_child_schema() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root.md");
        let child = dir.path().join("child.md");

        // Child has a schema requiring child_input
        std::fs::write(
            &child,
            "---\n$schema:\n  child_input: 'string(required)'\n---\nChild body\n",
        )
        .unwrap();

        // Parent transcludes child with set.child_input="ok"
        std::fs::write(
            &root,
            "# Parent\n\n::file ./child.md set.child_input=\"ok\"\n",
        )
        .unwrap();

        let md = Markdown::try_from(root.as_path()).unwrap();
        let options = ComposeOptions::new().with_source_file(root);
        let (composed, report) = md.compose_with(options).unwrap();

        assert!(composed.content().contains("Child body"));
        assert_eq!(report.transclusions_applied, 1);
    }

    #[test]
    fn parent_set_overlay_missing_child_schema_fails() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root.md");
        let child = dir.path().join("child.md");

        // Child has a schema requiring child_input
        std::fs::write(
            &child,
            "---\n$schema:\n  child_input: 'string(required)'\n---\nChild body\n",
        )
        .unwrap();

        // Parent transcludes child WITHOUT the set overlay
        std::fs::write(&root, "# Parent\n\n::file ./child.md\n").unwrap();

        let md = Markdown::try_from(root.as_path()).unwrap();
        // fail_fast=true so the schema validation error propagates rather
        // than being downgraded to a transclusion warning.
        let options = ComposeOptions::new()
            .with_source_file(root)
            .with_fail_fast(true);
        let err = md.compose_with(options).unwrap_err();

        match err {
            MarkdownError::SchemaValidationFailed { problems, .. } => {
                assert!(
                    problems.iter().any(|p| p.property.as_deref() == Some("child_input")),
                    "Expected problem on child_input, got: {problems:?}"
                );
            }
            other => panic!("Expected SchemaValidationFailed, got {other:?}"),
        }
    }

    /// Different baseline schemas must not share cache entries for the same
    /// transcluded child. Compose the same parent+child three times against
    /// a shared persistent cache:
    ///
    /// 1. baseline A → cold cache, child is computed and written to the
    ///    persistent store (`persistent_hits == 0`, `persistent_writes >= 1`).
    /// 2. baseline A again → cache is warm and the child compose entry is
    ///    reused (`persistent_hits >= 1`).
    /// 3. baseline B → baseline differs, so the persistent cache key
    ///    differs; the child must be recomputed rather than reuse the
    ///    baseline-A entry (`persistent_hits == 0` again).
    ///
    /// This proves `options_hash` includes `baseline_schema` in a way that
    /// actually invalidates the persistent cache — guarding against the
    /// "stale success keyed without baseline" regression.
    #[test]
    fn baseline_cache_does_not_reuse_across_distinct_baselines() {
        use crate::markdown::compose::CacheAccessMode;
        use crate::markdown::schemas::{
            Constraint, PropertyAtom, PropertyDef, SchemaShape, SimplifiedSchema,
            SimplifiedType, TypeExpr,
        };
        use indexmap::IndexMap;

        fn baseline_required(prop: &str) -> SimplifiedSchema {
            let mut properties = IndexMap::new();
            properties.insert(
                prop.into(),
                PropertyDef::Single(PropertyAtom {
                    ty: TypeExpr::Primitive(SimplifiedType::String),
                    is_array: false,
                    constraints: vec![Constraint::Required],
                    array_constraints: vec![],
                    description: None,
                }),
            );
            SimplifiedSchema::Single(SchemaShape { properties })
        }

        let dir = tempfile::tempdir().unwrap();
        let cache_root = dir.path().join("cache");
        let root = dir.path().join("root.md");
        let child = dir.path().join("child.md");

        // Parent supplies both `alpha` and `beta` so it (and its effective
        // state inherited by the child) satisfies either baseline under
        // test. Cache invalidation is the contract we care about here, not
        // the validation outcome.
        std::fs::write(&child, "---\nalpha: ok\nbeta: ok\n---\nChild body\n").unwrap();
        std::fs::write(
            &root,
            "---\nalpha: ok\nbeta: ok\n---\n# Parent\n\n::file ./child.md\n",
        )
        .unwrap();

        let mk_options = |baseline_prop: &str| {
            ComposeOptions::new()
                .with_source_file(&root)
                .with_baseline_schema(baseline_required(baseline_prop))
                .with_cache_access_mode(CacheAccessMode::ReadWrite)
                .with_cache_root(&cache_root)
                .with_cache_namespace("baseline_cache_regression")
                .with_fail_fast(true)
        };

        // ── Run 1: cold cache under baseline A ─────────────────────
        let md1 = Markdown::try_from(root.as_path()).unwrap();
        let (_, report1) = md1
            .compose_with(mk_options("alpha"))
            .expect("run 1 (baseline alpha, cold cache) should succeed");
        let stats1 = report1
            .cache_stats
            .expect("expected cache stats with cache enabled");
        assert_eq!(
            stats1.persistent_hits, 0,
            "run 1 should have a cold persistent cache, got {stats1:?}"
        );
        assert!(
            stats1.persistent_writes >= 1,
            "run 1 must write the child compose to the persistent cache, got {stats1:?}"
        );

        // ── Run 2: same baseline A → cache should be warm ──────────
        let md2 = Markdown::try_from(root.as_path()).unwrap();
        let (_, report2) = md2
            .compose_with(mk_options("alpha"))
            .expect("run 2 (baseline alpha, warm cache) should succeed");
        let stats2 = report2
            .cache_stats
            .expect("expected cache stats with cache enabled");
        assert!(
            stats2.persistent_hits >= 1,
            "run 2 must reuse the warmed persistent entry, got {stats2:?}",
        );

        // ── Run 3: baseline B → distinct key, must not reuse run 1 ─
        let md3 = Markdown::try_from(root.as_path()).unwrap();
        let (_, report3) = md3
            .compose_with(mk_options("beta"))
            .expect("run 3 (baseline beta) should succeed");
        let stats3 = report3
            .cache_stats
            .expect("expected cache stats with cache enabled");
        assert_eq!(
            stats3.persistent_hits, 0,
            "run 3 must NOT reuse the baseline-A entry — options_hash must include \
             baseline_schema. got {stats3:?}",
        );
        assert!(
            stats3.persistent_writes >= 1,
            "run 3 must compute and write a fresh entry under the new baseline, got {stats3:?}"
        );
    }

    /// The launch-area anchor (`file_ref_fallback_dir`) changes read-side file
    /// resolution, so two runs that differ only in their anchor must not share a
    /// persistent cache entry.
    ///
    /// The body interpolates `{{ file_exists("anchored.md") }}`. The document
    /// dir never holds `anchored.md`, so the outcome depends entirely on the
    /// fallback dir: run 1's fallback has the file (resolves `true`), run 2's
    /// fallback does not (`false`). If `options_hash` ignored
    /// `file_ref_fallback_dir`, run 2 would reuse run 1's cached `true` —
    /// returning a stale, wrong-launch-area result.
    #[test]
    fn cache_does_not_reuse_across_distinct_file_ref_fallback_dirs() {
        use crate::markdown::compose::CacheAccessMode;

        let dir = tempfile::tempdir().unwrap();
        let cache_root = dir.path().join("cache");
        let doc_dir = dir.path().join("doc");
        std::fs::create_dir_all(&doc_dir).unwrap();
        let root = doc_dir.join("root.md");
        std::fs::write(&root, "anchored exists: {{ file_exists(\"anchored.md\") }}\n").unwrap();

        // Run 1's launch area HAS anchored.md; run 2's does NOT.
        let fallback_present = dir.path().join("launch-present");
        let fallback_absent = dir.path().join("launch-absent");
        std::fs::create_dir_all(&fallback_present).unwrap();
        std::fs::create_dir_all(&fallback_absent).unwrap();
        std::fs::write(fallback_present.join("anchored.md"), "# Anchored\n").unwrap();

        let mk_options = |fallback: &std::path::Path| {
            ComposeOptions::new()
                .with_source_file(&root)
                .with_file_ref_fallback_dir(fallback.to_path_buf())
                .with_cache_access_mode(CacheAccessMode::ReadWrite)
                .with_cache_root(&cache_root)
                .with_cache_namespace("file_ref_fallback_cache_regression")
                .with_fail_fast(true)
        };

        // ── Run 1: cold cache, fallback HAS anchored.md → true ─────
        let md1 = Markdown::try_from(root.as_path()).unwrap();
        let (composed1, report1) = md1
            .compose_with(mk_options(&fallback_present))
            .expect("run 1 (present fallback, cold cache) should succeed");
        assert!(
            composed1.content().contains("anchored exists: true"),
            "run 1 must resolve file_exists as true from its launch area: {}",
            composed1.content(),
        );
        let stats1 = report1
            .cache_stats
            .expect("expected cache stats with cache enabled");
        assert_eq!(
            stats1.persistent_hits, 0,
            "run 1 should have a cold persistent cache, got {stats1:?}"
        );

        // ── Run 2: different launch area, fallback LACKS the file ──
        let md2 = Markdown::try_from(root.as_path()).unwrap();
        let (composed2, _report2) = md2
            .compose_with(mk_options(&fallback_absent))
            .expect("run 2 (absent fallback) should succeed");
        assert!(
            composed2.content().contains("anchored exists: false"),
            "run 2 must reflect its OWN launch area (file absent) — options_hash \
             must include file_ref_fallback_dir, not reuse run 1's cached output: {}",
            composed2.content(),
        );
    }
}

mod remote_transclusion_tests {
    use super::*;
    use crate::markdown::compose::remote::RemoteReadConfig;
    use std::collections::HashSet;
    use wiremock::{Mock, MockServer, ResponseTemplate};
    use wiremock::matchers::{method, path};

    async fn compose_with_remote(
        content: &str,
        source_file: &std::path::Path,
        _server: &MockServer,
        allowed_hosts: Vec<String>,
    ) -> MarkdownResult<(Markdown, ComposeReport)> {
        let md: Markdown = content.into();
        let config = RemoteReadConfig {
            allowed_hosts,
            ..Default::default()
        };
        let options = ComposeOptions::new()
            .with_source_file(source_file)
            .with_allow_remote_transclusion(true)
            .with_remote_read_config(config)
            .disable(ComposeOperation::Cleanup)
            .disable(ComposeOperation::Normalization);
        md.compose_with(options)
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn remote_file_transclusion_inserts_fetched_body() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/remote.md"))
            .respond_with(ResponseTemplate::new(200).set_body_string("# Remote\n\nHello from remote"))
            .mount(&server)
            .await;

        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root.md");
        let remote_url = format!("{}/remote.md", server.uri());
        let content = format!("# Local\n\n::file {remote_url}\n");
        std::fs::write(&root, &content).unwrap();

        let (composed, report) =
            compose_with_remote(&content, &root, &server, vec!["127.0.0.1".into()])
                .await
                .unwrap();

        let text = composed.content();
        assert!(text.contains("Hello from remote"), "content: {text}");
        assert!(text.contains("# Local"), "content: {text}");
        assert_eq!(report.transclusions_applied, 1);
        let rf = report.remote_fetch_stats.unwrap();
        assert_eq!(rf.fetched, 1);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn remote_code_transclusion_inserts_fetched_code() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/snippet.rs"))
            .respond_with(ResponseTemplate::new(200).set_body_string("fn main() {}"))
            .mount(&server)
            .await;

        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root.md");
        let remote_url = format!("{}/snippet.rs", server.uri());
        let content = format!("# Doc\n\n::code {remote_url}\n");
        std::fs::write(&root, &content).unwrap();

        let (composed, report) =
            compose_with_remote(&content, &root, &server, vec!["127.0.0.1".into()])
                .await
                .unwrap();

        let text = composed.content();
        assert!(text.contains("fn main()"), "content: {text}");
        assert!(text.contains("```rs"), "content: {text}");
        assert_eq!(report.transclusions_applied, 1);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn remote_transclusion_denied_by_default() {
        let server = MockServer::start().await;

        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root.md");
        let remote_url = format!("{}/blocked.md", server.uri());
        let content = format!("# Doc\n\n::file {remote_url}\n");
        std::fs::write(&root, &content).unwrap();

        let md: Markdown = content.clone().into();
        let options = ComposeOptions::new()
            .with_source_file(&root)
            .with_allow_remote_transclusion(true)
            .disable(ComposeOperation::Cleanup)
            .disable(ComposeOperation::Normalization);
        let result = md.compose_with(options);
        assert!(
            result.is_err(),
            "Expected error because no allowed hosts configured"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn remote_file_duplicate_consumers_one_fetch() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/shared.md"))
            .respond_with(ResponseTemplate::new(200).set_body_string("shared content"))
            .mount(&server)
            .await;

        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root.md");
        let remote_url = format!("{}/shared.md", server.uri());
        // Two line-start directives referencing the same URL: directives
        // are line-oriented, so each must begin its own line.
        let content = format!(
            "# Doc\n\n::file {remote_url}\n\n::file {remote_url}\n"
        );
        std::fs::write(&root, &content).unwrap();

        let (composed, report) =
            compose_with_remote(&content, &root, &server, vec!["127.0.0.1".into()])
                .await
                .unwrap();

        let text = composed.content();
        let count = text.matches("shared content").count();
        assert_eq!(count, 2, "Should appear twice (once per directive)");
        assert_eq!(report.transclusions_applied, 2);
        let rf = report.remote_fetch_stats.unwrap();
        assert_eq!(rf.fetched, 1, "Only one actual network fetch should occur");
    }

    /// Mounts a single document and composes a body that reads it through
    /// the given quoted URL expression, returning the composed text.
    ///
    /// The URL argument is quoted because the interpolation expression
    /// parser only accepts a string literal there; the unquoted
    /// `frontmatter(https://…)` form does not tokenize.
    async fn compose_expr_against_doc(doc_body: &str, expr_template: &str) -> String {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/doc.md"))
            .respond_with(ResponseTemplate::new(200).set_body_string(doc_body.to_string()))
            .mount(&server)
            .await;
        let url = format!("{}/doc.md", server.uri());
        let body = expr_template.replace("{URL}", &url);

        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root.md");
        std::fs::write(&root, &body).unwrap();

        let (composed, _) =
            compose_with_remote(&body, &root, &server, vec!["127.0.0.1".into()])
                .await
                .unwrap();
        composed.content().to_string()
    }

    /// The `frontmatter()` read-side **function** is remote-capable when
    /// called from the **body** surface: body interpolation carries the
    /// run's remote-fetch runtime, so reading another document's frontmatter
    /// property over HTTP(S) succeeds. (Decision B restricts only the
    /// frontmatter *surface*, not this function — see the loud-failure test
    /// `frontmatter_value_remote_url_fails_loudly` below.)
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn remote_frontmatter_function_in_body_reads_url() {
        let text = compose_expr_against_doc(
            "---\ntitle: Remote Title\nstatus: draft\n---\n# H1\n\nBody\n",
            "S: {{ frontmatter(\"{URL}\", \"status\") }}\n",
        )
        .await;
        assert_eq!(text, "S: draft\n");
    }

    /// Decision B: the frontmatter *surface* is local-filesystem only. A
    /// remote URL argument to a read-side function written in a frontmatter
    /// value must fail loudly rather than performing a network read — even
    /// when the run otherwise allows the host for body/transclusion fetches.
    /// The value here is a whole-value `{{ … }}`, which is executable state,
    /// so the evaluation error aborts composition regardless of fail_fast —
    /// the loud failure can never leave the raw template unsubstituted.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn frontmatter_value_remote_url_fails_loudly() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/doc.md"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string("---\nstatus: draft\n---\n# H1\n"),
            )
            .mount(&server)
            .await;
        let url = format!("{}/doc.md", server.uri());

        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root.md");
        // The expression lives in a frontmatter value, not the body.
        let content = format!(
            "---\nstatus: '{{{{ frontmatter(\"{url}\", \"status\") }}}}'\n---\n# H1\n"
        );
        std::fs::write(&root, &content).unwrap();

        let err = compose_with_remote(&content, &root, &server, vec!["127.0.0.1".into()])
            .await
            .expect_err("a whole-value remote-URL read in frontmatter must abort composition");

        // The loud failure names the key, the local-only diagnostic, and the URL.
        let msg = err.to_string();
        assert!(msg.contains("status"), "error must name the key, got: {msg}");
        assert!(
            msg.contains("remote reads are not enabled"),
            "expected a local-only frontmatter diagnostic, got: {msg}"
        );
        assert!(msg.contains(&url), "error must name the URL, got: {msg}");
    }

    /// Decision B also applies to `file_exists()`: a remote URL argument in
    /// a whole-value `{{ … }}` frontmatter value must abort composition rather
    /// than silently reporting the URL as absent or leaking the raw template.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn frontmatter_file_exists_remote_url_fails_loudly() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/doc.md"))
            .respond_with(ResponseTemplate::new(200).set_body_string("# Present\n"))
            .mount(&server)
            .await;
        let url = format!("{}/doc.md", server.uri());

        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root.md");
        let content = format!(
            "---\npresent: '{{{{ file_exists(\"{url}\") }}}}'\n---\n# H1\n"
        );
        std::fs::write(&root, &content).unwrap();

        let err = compose_with_remote(&content, &root, &server, vec!["127.0.0.1".into()])
            .await
            .expect_err("a whole-value remote-URL file_exists in frontmatter must abort composition");

        let msg = err.to_string();
        // The receiving key is captured as structured scope, not Display prose.
        let MarkdownError::Interpolation { key, .. } = &err else {
            panic!("expected Interpolation error, got: {err:?}");
        };
        assert_eq!(key.as_deref(), Some("present"), "error must capture the key");
        assert!(
            msg.contains("local-only"),
            "expected a local-only frontmatter diagnostic for file_exists, got: {msg}"
        );
        assert!(msg.contains(&url), "error must name the URL, got: {msg}");
    }

    /// Decision B covers the `$()` shell-ternary condition surface too: a
    /// read-side function call in the condition is evaluated with the
    /// local-only frontmatter context, so a remote URL argument errors
    /// before any branch is selected or executed.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn frontmatter_shell_ternary_remote_url_condition_fails_loudly() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/doc.md"))
            .respond_with(ResponseTemplate::new(200).set_body_string("# Present\n"))
            .mount(&server)
            .await;
        let url = format!("{}/doc.md", server.uri());

        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root.md");
        let content = format!(
            "---\nresult: $(file_exists(\"{url}\") ? echo yes : echo no)\n---\n# H1\n"
        );
        std::fs::write(&root, &content).unwrap();

        let mut approved = HashSet::new();
        approved.insert("echo yes".to_string());
        approved.insert("echo no".to_string());

        let md: Markdown = content.into();
        let config = RemoteReadConfig {
            allowed_hosts: vec!["127.0.0.1".into()],
            ..Default::default()
        };
        let options = ComposeOptions::new()
            .with_source_file(&root)
            .with_allow_remote_transclusion(true)
            .with_remote_read_config(config)
            .with_pre_approved_commands(approved)
            .disable(ComposeOperation::Cleanup)
            .disable(ComposeOperation::Normalization);
        let result = md.compose_with(options);

        assert!(
            result.is_err(),
            "expected compose to fail because the ternary condition uses a remote URL in local-only frontmatter"
        );
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("local-only") && err.contains(&url),
            "expected helpful local-only diagnostic, got: {err}"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn remote_markdown_title_expression_reads_url() {
        let text = compose_expr_against_doc(
            "---\ntitle: Remote Title\n---\n# H1\n\nBody\n",
            "T: {{ markdown_title(\"{URL}\") }}\n",
        )
        .await;
        assert_eq!(text, "T: Remote Title\n");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn remote_markdown_body_empty_expression_reads_url() {
        let text = compose_expr_against_doc(
            "---\ntitle: Empty Body\n---\n",
            "E: {{ markdown_body_empty(\"{URL}\") }}\n",
        )
        .await;
        assert_eq!(text, "E: true\n");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn remote_validate_schema_expression_reads_url() {
        // A document without `$schema` always validates as `true`.
        let text = compose_expr_against_doc(
            "---\ntitle: No Schema\n---\n# H1\n",
            "V: {{ validate_schema(\"{URL}\") }}\n",
        )
        .await;
        assert_eq!(text, "V: true\n");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn remote_file_exists_expression_reads_url() {
        let text = compose_expr_against_doc(
            "# Present\n",
            "X: {{ file_exists(\"{URL}\") }}\n",
        )
        .await;
        assert_eq!(text, "X: true\n");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn file_exists_false_for_unallowed_remote_host() {
        // Remote reads enabled but no allowed hosts: the URL is denied and
        // never fetched, so `file_exists` reads it as non-existent rather
        // than erroring out of composition.
        let server = MockServer::start().await;
        let url = format!("{}/blocked.md", server.uri());

        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root.md");
        let body = format!("X: {{{{ file_exists(\"{url}\") }}}}\n");
        std::fs::write(&root, &body).unwrap();

        let (composed, _) = compose_with_remote(&body, &root, &server, vec![])
            .await
            .unwrap();
        assert_eq!(composed.content(), "X: false\n");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn interpolation_only_discovers_and_reads_remote_expression_url() {
        // Review 7: a library caller following the documented API enables
        // remote expression reads via `with_remote_read_config` alone — an
        // allowed host is sufficient. No `with_allow_remote_transclusion`
        // call is required, since expression URL reads are a read-side
        // capability independent of block transclusion.
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/doc.md"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string("---\ntitle: Interp Only\n---\n# H1\n"),
            )
            .mount(&server)
            .await;
        let url = format!("{}/doc.md", server.uri());

        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root.md");
        let body = format!("T: {{{{ markdown_title(\"{url}\") }}}}\n");
        std::fs::write(&root, &body).unwrap();

        let config = RemoteReadConfig {
            allowed_hosts: vec!["127.0.0.1".into()],
            ..Default::default()
        };
        let options = ComposeOptions::new()
            .with_source_file(&root)
            .with_remote_read_config(config)
            .only(&[ComposeOperation::Interpolation]);
        let md: Markdown = body.clone().into();
        let (composed, report) = md.compose_with(options).unwrap();

        assert_eq!(composed.content(), "T: Interp Only\n");
        let rf = report.remote_fetch_stats.unwrap();
        assert_eq!(rf.fetched, 1, "URL must be fetched without BlockTransclusion");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn local_markdown_title_expression_reads_relative_file() {
        // The same resolution-context wiring resolves local relative paths
        // against the source document's directory.
        let server = MockServer::start().await;
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("other.md"),
            "---\ntitle: Local Title\n---\n# H1\n",
        )
        .unwrap();
        let root = dir.path().join("root.md");
        let body = "T: {{ markdown_title(\"./other.md\") }}\n";
        std::fs::write(&root, body).unwrap();

        let (composed, _) = compose_with_remote(body, &root, &server, vec![])
            .await
            .unwrap();
        assert_eq!(composed.content(), "T: Local Title\n");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn nested_remote_reference_is_discovered_and_fetched() {
        let server = MockServer::start().await;
        // The parent document itself transcludes a further remote child.
        let child_url = format!("{}/child.md", server.uri());
        Mock::given(method("GET"))
            .and(path("/parent.md"))
            .respond_with(ResponseTemplate::new(200).set_body_string(format!(
                "# Parent\n\n::file {child_url}\n"
            )))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/child.md"))
            .respond_with(
                ResponseTemplate::new(200).set_body_string("nested child body"),
            )
            .mount(&server)
            .await;

        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root.md");
        let parent_url = format!("{}/parent.md", server.uri());
        let content = format!("# Local\n\n::file {parent_url}\n");
        std::fs::write(&root, &content).unwrap();

        let (composed, report) =
            compose_with_remote(&content, &root, &server, vec!["127.0.0.1".into()])
                .await
                .unwrap();

        let text = composed.content();
        assert!(text.contains("nested child body"), "content: {text}");
        let rf = report.remote_fetch_stats.unwrap();
        assert_eq!(rf.fetched, 2, "Both parent and nested child fetched once");
    }

    /// Recursive preflight graph reuse for **remote** children (review-6
    /// Medium): the preflight walk discovers root → remote child → remote
    /// grandchild, and reusing the graph must thread each remote child its own
    /// sub-node so the grandchild URL resolution is reused too — the remote
    /// analogue of `preflight_graph_reuse_recurses_to_grandchild`. The
    /// graph-seeded compose must match the no-graph baseline and still reach the
    /// grandchild body.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn preflight_graph_reuse_recurses_to_remote_grandchild() {
        use crate::markdown::compose::preflight::PreflightResolvedTarget;

        let server = MockServer::start().await;
        let grandchild_url = format!("{}/grandchild.md", server.uri());
        let child_url = format!("{}/child.md", server.uri());

        Mock::given(method("GET"))
            .and(path("/child.md"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(format!("# Child\n\n::file {grandchild_url}\n")),
            )
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/grandchild.md"))
            .respond_with(ResponseTemplate::new(200).set_body_string("GRANDCHILD-CONTENT-MARKER"))
            .mount(&server)
            .await;

        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root.md");
        let content = format!("# Local\n\n::file {child_url}\n");
        std::fs::write(&root, &content).unwrap();

        let md: Markdown = content.as_str().into();
        let config = RemoteReadConfig {
            allowed_hosts: vec!["127.0.0.1".into()],
            ..Default::default()
        };
        let base_options = ComposeOptions::new()
            .with_source_file(&root)
            .with_allow_remote_transclusion(true)
            .with_remote_read_config(config)
            .disable(ComposeOperation::Cleanup)
            .disable(ComposeOperation::Normalization);

        // The walked graph is recursive across the URL edges: the root's single
        // edge points at the remote child, whose own single edge points at the
        // remote grandchild.
        let preflight = md.compose_preflight(&base_options).unwrap();
        assert_eq!(
            preflight.preflight_graph.edges.len(),
            1,
            "root should have one remote edge: {:?}",
            preflight.preflight_graph
        );
        let child_node = &preflight.preflight_graph.edges[0].child;
        assert_eq!(
            child_node.edges.len(),
            1,
            "remote child node should carry one grandchild edge: {child_node:?}"
        );
        match &child_node.edges[0].resolved_target {
            PreflightResolvedTarget::Url(u) => assert_eq!(
                u.as_str(),
                grandchild_url,
                "child node's edge should carry the resolved grandchild URL"
            ),
            other => panic!("grandchild edge should resolve to a Url target, got {other:?}"),
        }

        // Compose WITHOUT the preflight graph — baseline output.
        let (baseline, _) = md.compose_with(base_options.clone()).unwrap();
        assert!(
            baseline.content().contains("GRANDCHILD-CONTENT-MARKER"),
            "baseline did not transclude remote grandchild: {}",
            baseline.content()
        );

        // Compose WITH the preflight graph — must reach the grandchild via the
        // threaded remote sub-node and stay byte-identical to the baseline.
        let with_graph_options =
            base_options.with_preflight_graph(preflight.preflight_graph.clone());
        let (with_graph, _) = md.compose_with(with_graph_options).unwrap();
        assert!(
            with_graph.content().contains("GRANDCHILD-CONTENT-MARKER"),
            "graph-seeded compose did not transclude remote grandchild: {}",
            with_graph.content()
        );
        assert_eq!(
            with_graph.content(),
            baseline.content(),
            "recursive remote preflight-graph reuse must produce byte-identical output to the baseline"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn interpolated_directive_creates_fetchable_remote_file() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/late.md"))
            .respond_with(ResponseTemplate::new(200).set_body_string("late remote body"))
            .mount(&server)
            .await;

        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root.md");
        let remote_url = format!("{}/late.md", server.uri());
        // The directive's URL only materializes after interpolation expands
        // `{{ remote_ref }}`, so the eager pre-scan never sees it. It must be
        // registered when prepared, or point-of-use fails "not registered".
        let content = format!(
            "---\nremote_ref: \"{remote_url}\"\n---\n# Local\n\n::file {{{{ remote_ref }}}}\n"
        );
        std::fs::write(&root, &content).unwrap();

        let (composed, report) =
            compose_with_remote(&content, &root, &server, vec!["127.0.0.1".into()])
                .await
                .unwrap();

        let text = composed.content();
        assert!(text.contains("late remote body"), "content: {text}");
        let rf = report.remote_fetch_stats.unwrap();
        assert_eq!(rf.fetched, 1);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn interpolated_directive_creates_fetchable_remote_code() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/late.rs"))
            .respond_with(ResponseTemplate::new(200).set_body_string("fn main() {}"))
            .mount(&server)
            .await;

        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root.md");
        let remote_url = format!("{}/late.rs", server.uri());
        let content = format!(
            "---\nremote_ref: \"{remote_url}\"\n---\n# Doc\n\n::code {{{{ remote_ref }}}}\n"
        );
        std::fs::write(&root, &content).unwrap();

        let (composed, report) =
            compose_with_remote(&content, &root, &server, vec!["127.0.0.1".into()])
                .await
                .unwrap();

        let text = composed.content();
        assert!(text.contains("fn main()"), "content: {text}");
        assert!(text.contains("```rs"), "content: {text}");
        let rf = report.remote_fetch_stats.unwrap();
        assert_eq!(rf.fetched, 1);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn cached_local_child_revalidates_nested_remote_under_refresh() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use wiremock::{Request, Respond};

        // Serves "remote v1" on the first request and "remote v2" on every
        // later request, so a missed revalidation surfaces as stale output.
        struct Versioned {
            hits: Arc<AtomicUsize>,
        }
        impl Respond for Versioned {
            fn respond(&self, _req: &Request) -> ResponseTemplate {
                let n = self.hits.fetch_add(1, Ordering::SeqCst);
                let body = if n == 0 { "remote v1" } else { "remote v2" };
                ResponseTemplate::new(200).set_body_string(body)
            }
        }

        let server = MockServer::start().await;
        let hits = Arc::new(AtomicUsize::new(0));
        Mock::given(method("GET"))
            .and(path("/remote.md"))
            .respond_with(Versioned {
                hits: Arc::clone(&hits),
            })
            .mount(&server)
            .await;

        let dir = tempfile::tempdir().unwrap();
        let cache_root = dir.path().join("cache");
        let root = dir.path().join("root.md");
        let child = dir.path().join("child.md");
        let remote_url = format!("{}/remote.md", server.uri());
        std::fs::write(&root, "# Root\n\n::file ./child.md\n").unwrap();
        std::fs::write(&child, format!("# Child\n\n::file {remote_url}\n")).unwrap();

        let mk_options = |refresh: bool| {
            let config = RemoteReadConfig {
                allowed_hosts: vec!["127.0.0.1".into()],
                refresh,
                ..Default::default()
            };
            ComposeOptions::new()
                .with_source_file(&root)
                .with_allow_remote_transclusion(true)
                .with_remote_read_config(config)
                .with_cache_root(&cache_root)
                .disable(ComposeOperation::Cleanup)
                .disable(ComposeOperation::Normalization)
        };

        // Run 1: populate the local-child and remote caches; remote → v1.
        let md1 = Markdown::try_from(root.as_path()).unwrap();
        let (c1, _) = md1.compose_with(mk_options(false)).unwrap();
        assert!(
            c1.content().contains("remote v1"),
            "run 1 should embed the original remote body: {}",
            c1.content()
        );

        // Run 2: the remote body has changed and `--remote-refresh` forces a
        // revalidation. The cached local child must NOT be accepted against
        // the stale remote manifest.
        let md2 = Markdown::try_from(root.as_path()).unwrap();
        let (c2, _) = md2.compose_with(mk_options(true)).unwrap();
        assert!(
            c2.content().contains("remote v2"),
            "cached local child must revalidate its nested remote URL under \
             --remote-refresh; got: {}",
            c2.content()
        );
        assert!(
            !c2.content().contains("remote v1"),
            "stale remote body served from cache: {}",
            c2.content()
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn remote_prologue_inserts_fetched_body() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/intro.md"))
            .respond_with(ResponseTemplate::new(200).set_body_string("# Intro\n\nFrom prologue"))
            .mount(&server)
            .await;

        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root.md");
        let remote_url = format!("{}/intro.md", server.uri());
        let content = format!("---\nprologue: {remote_url}\n---\n# Local\n\nBody.\n");
        std::fs::write(&root, &content).unwrap();

        let (composed, report) =
            compose_with_remote(&content, &root, &server, vec!["127.0.0.1".into()])
                .await
                .unwrap();

        let text = composed.content();
        assert!(text.contains("From prologue"), "content: {text}");
        assert!(text.contains("# Local"), "content: {text}");
        assert_eq!(report.transclusions_applied, 1);
        let rf = report.remote_fetch_stats.unwrap();
        assert_eq!(rf.fetched, 1);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn remote_epilogue_inserts_fetched_body() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/outro.md"))
            .respond_with(ResponseTemplate::new(200).set_body_string("# Outro\n\nFrom epilogue"))
            .mount(&server)
            .await;

        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root.md");
        let remote_url = format!("{}/outro.md", server.uri());
        let content = format!("---\nepilogue: [\"{remote_url}\"]\n---\n# Local\n\nBody.\n");
        std::fs::write(&root, &content).unwrap();

        let (composed, report) =
            compose_with_remote(&content, &root, &server, vec!["127.0.0.1".into()])
                .await
                .unwrap();

        let text = composed.content();
        assert!(text.contains("From epilogue"), "content: {text}");
        assert!(text.contains("# Local"), "content: {text}");
        assert_eq!(report.transclusions_applied, 1);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn remote_prologue_denied_by_policy() {
        // Remote transclusion is enabled but the host is not allowlisted, so
        // the registered fetch fails by policy and surfaces an error rather
        // than a bogus "URL was not registered" message.
        let server = MockServer::start().await;
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root.md");
        let remote_url = format!("{}/intro.md", server.uri());
        let content = format!("---\nprologue: {remote_url}\n---\n# Local\n\nBody.\n");
        std::fs::write(&root, &content).unwrap();

        let md: Markdown = content.clone().into();
        let options = ComposeOptions::new()
            .with_source_file(&root)
            .with_allow_remote_transclusion(true)
            .disable(ComposeOperation::Cleanup)
            .disable(ComposeOperation::Normalization);
        let result = md.compose_with(options);
        assert!(
            result.is_err(),
            "Expected error because no allowed hosts configured"
        );
    }
}

mod file_links_compose {
    use super::*;

    #[test]
    fn glob_mode_replaces_directive_with_linked_tree() {
        let dir = tempfile::tempdir().unwrap();
        // Fake repo so boundary resolves to temp dir, not CWD.
        std::fs::create_dir(dir.path().join(".git")).unwrap();
        let docs = dir.path().join("docs");
        std::fs::create_dir(&docs).unwrap();
        std::fs::write(docs.join("a.md"), "# A\n").unwrap();
        std::fs::write(docs.join("b.txt"), "B\n").unwrap();

        let root = dir.path().join("root.md");
        std::fs::write(&root, "# Root\n\n::file-links docs/*\n\nFooter.\n").unwrap();

        let md = Markdown::try_from(root.as_path()).unwrap();
        let options = ComposeOptions::new()
            .with_source_file(&root)
            .disable(ComposeOperation::Cleanup)
            .disable(ComposeOperation::Normalization);
        let (composed, report) = md.compose_with(options).unwrap();

        let text = composed.content();
        assert!(text.contains("a.md"), "content: {text}");
        assert!(text.contains("b.txt"), "content: {text}");
        assert!(!text.contains("::file-links"), "directive should be replaced: {text}");
        assert_eq!(report.transclusions_applied, 1);
    }

    /// End-to-end: composing a `::file-links` directive through the full
    /// default pipeline embeds the FileSystem render subtree, and rendering
    /// the composed document to a terminal reproduces the styled tree —
    /// OSC8 hyperlinks and dim styling survive the round-trip, and the
    /// embedding marker never leaks into rendered output.
    #[test]
    fn embedded_subtree_round_trips_through_compose_then_terminal_render() {
        use crate::markdown::highlighting::{ColorMode, ThemePair};
        use crate::markdown::output::terminal::{
            ColorDepth, DimMode, HyperlinkMode, ItalicMode, MermaidMode, TerminalImageMode,
            TerminalOptions,
        };

        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join(".git")).unwrap();
        let docs = dir.path().join("docs").join("topics");
        std::fs::create_dir_all(&docs).unwrap();
        std::fs::write(docs.join("alpha.md"), "# Alpha\n").unwrap();
        std::fs::write(docs.join("beta.md"), "# Beta\n").unwrap();

        let root = dir.path().join("root.md");
        std::fs::write(&root, "# Root\n\n::file-links docs/topics/*.md\n").unwrap();

        let md = Markdown::try_from(root.as_path()).unwrap();
        // Full default pipeline (cleanup + normalization enabled) proves the
        // embedded block survives end to end.
        let (composed, _report) = md
            .compose_with(ComposeOptions::new().with_source_file(&root))
            .unwrap();
        assert!(
            composed.content().contains("bt:render-tree"),
            "composed doc should carry the embedded subtree: {}",
            composed.content()
        );

        let options = TerminalOptions {
            code_theme: ThemePair::OneHalf,
            prose_theme: ThemePair::OneHalf,
            color_mode: ColorMode::Dark,
            include_line_numbers: false,
            color_depth: Some(ColorDepth::TrueColor),
            image_mode: TerminalImageMode::Never,
            base_path: None,
            italic_mode: ItalicMode::Always,
            dim_mode: DimMode::Always,
            max_width: Some(100),
            mermaid_mode: MermaidMode::Off,
            hyperlink_mode: HyperlinkMode::Always,
            hr_defaults: None,
            code_block_mode: crate::markdown::highlighting::CodeBlockMode::default(),
        };
        let output = composed.as_terminal(options).unwrap();

        assert!(
            !output.contains("bt:render-tree"),
            "embedding marker leaked into rendered output: {output:?}"
        );
        assert!(output.contains("alpha.md"), "missing file: {output:?}");
        assert!(output.contains("beta.md"), "missing file: {output:?}");
        assert!(
            output.contains("\x1b]8;;"),
            "OSC8 hyperlink did not survive: {output:?}"
        );
        assert!(
            output.contains("file://"),
            "file:// link target did not survive: {output:?}"
        );
        assert!(
            output.contains("\x1b[2m"),
            "dim styling (dimmed root prefix) did not survive: {output:?}"
        );
    }

    /// In-process companion to the Level 2 presentation test: the full
    /// `::file-links` contract (extension glyphs, repository icon, italic
    /// dotfile, dimmed gitignored entry, bold target) is produced by
    /// composing then rendering to a terminal string. This runs in the L1
    /// suite, so the bytes the real-terminal test asserts on are verified
    /// without a WezTerm pane.
    #[test]
    fn rich_fixture_renders_full_presentation_contract() {
        use crate::markdown::output::terminal::{
            ColorDepth, DimMode, HyperlinkMode, ItalicMode, MermaidMode, TerminalImageMode,
            TerminalOptions,
        };

        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join(".git")).unwrap();
        let topics = dir.path().join("docs").join("topics");
        std::fs::create_dir_all(&topics).unwrap();
        std::fs::write(topics.join("alpha.md"), "# Alpha\n").unwrap();
        std::fs::write(topics.join("notes.txt"), "notes\n").unwrap();
        std::fs::write(topics.join("report.pdf"), "pdf\n").unwrap();
        std::fs::write(topics.join("sheet.xlsx"), "xls\n").unwrap();
        std::fs::write(topics.join("memo.docx"), "doc\n").unwrap();
        std::fs::write(topics.join(".hidden.md"), "# Hidden\n").unwrap();
        std::fs::write(topics.join(".gitignore"), "ignored.md\n").unwrap();
        std::fs::write(topics.join("ignored.md"), "# Ignored\n").unwrap();

        let root = dir.path().join("root.md");
        std::fs::write(&root, "# Root\n\n::file-links --dir docs/topics\n").unwrap();

        let md = Markdown::try_from(root.as_path()).unwrap();
        let (composed, _report) = md
            .compose_with(ComposeOptions::new().with_source_file(&root))
            .unwrap();

        let options = TerminalOptions {
            color_mode: crate::markdown::highlighting::ColorMode::Dark,
            color_depth: Some(ColorDepth::TrueColor),
            image_mode: TerminalImageMode::Never,
            italic_mode: ItalicMode::Always,
            dim_mode: DimMode::Always,
            hyperlink_mode: HyperlinkMode::Always,
            mermaid_mode: MermaidMode::Off,
            max_width: Some(100),
            ..TerminalOptions::default()
        };
        let output = composed.as_terminal(options).unwrap();

        // Extension-specific Unicode glyphs.
        for glyph in ["📝", "📕", "📗", "📘"] {
            assert!(output.contains(glyph), "missing glyph {glyph:?}: {output:?}");
        }
        // Repository icon, never the ordinary folder icon (no subdirs).
        assert!(output.contains("📦"), "missing repository icon: {output:?}");
        assert!(!output.contains("📂"), "unexpected folder icon: {output:?}");
        // Dotfile italic, gitignored dim, target bold.
        assert!(output.contains("\x1b[3m"), "missing italic dotfile: {output:?}");
        assert!(output.contains("\x1b[2m"), "missing dim entry: {output:?}");
        assert!(output.contains("\x1b[1m"), "missing bold target: {output:?}");
        // The gitignored document is present but dim, the dotfile present.
        assert!(output.contains("ignored.md"), "missing ignored.md: {output:?}");
        assert!(output.contains(".hidden.md"), "missing .hidden.md: {output:?}");
        assert!(
            !output.contains(".gitignore"),
            ".gitignore should not be a tree entry: {output:?}"
        );
    }

    #[test]
    fn dir_mode_with_depth_zero_lists_top_level_only() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join(".git")).unwrap();
        let docs = dir.path().join("docs");
        std::fs::create_dir(&docs).unwrap();
        std::fs::write(docs.join("top.md"), "# Top\n").unwrap();
        let sub = docs.join("sub");
        std::fs::create_dir(&sub).unwrap();
        std::fs::write(sub.join("nested.md"), "# Nested\n").unwrap();

        let root = dir.path().join("root.md");
        std::fs::write(&root, "# Root\n\n::file-links --dir docs\n\n").unwrap();

        let md = Markdown::try_from(root.as_path()).unwrap();
        let options = ComposeOptions::new()
            .with_source_file(&root)
            .disable(ComposeOperation::Cleanup)
            .disable(ComposeOperation::Normalization);
        let (composed, report) = md.compose_with(options).unwrap();

        let text = composed.content();
        assert!(text.contains("top.md"), "content: {text}");
        assert!(
            !text.contains("nested.md"),
            "depth 0 should not recurse: {text}"
        );
        assert_eq!(report.transclusions_applied, 1);
    }

    #[test]
    fn dir_mode_with_depth_recovers_nested_files() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join(".git")).unwrap();
        let docs = dir.path().join("docs");
        std::fs::create_dir(&docs).unwrap();
        let sub = docs.join("sub");
        std::fs::create_dir(&sub).unwrap();
        std::fs::write(sub.join("nested.md"), "# Nested\n").unwrap();

        let root = dir.path().join("root.md");
        std::fs::write(&root, "# Root\n\n::file-links --dir docs --depth 2\n\n").unwrap();

        let md = Markdown::try_from(root.as_path()).unwrap();
        let options = ComposeOptions::new()
            .with_source_file(&root)
            .disable(ComposeOperation::Cleanup)
            .disable(ComposeOperation::Normalization);
        let (composed, report) = md.compose_with(options).unwrap();

        let text = composed.content();
        assert!(text.contains("nested.md"), "content: {text}");
        assert_eq!(report.transclusions_applied, 1);
    }

    #[test]
    fn self_exclusion_skips_source_document() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join(".git")).unwrap();
        let root = dir.path().join("root.md");
        std::fs::write(&root, "# Root\n\n::file-links *.md\n\n").unwrap();

        let md = Markdown::try_from(root.as_path()).unwrap();
        let options = ComposeOptions::new()
            .with_source_file(&root)
            .disable(ComposeOperation::Cleanup)
            .disable(ComposeOperation::Normalization);
        let (composed, _report) = md.compose_with(options).unwrap();

        let text = composed.content();
        assert!(
            !text.contains("root.md"),
            "source doc should be excluded: {text}"
        );
    }

    #[test]
    fn strict_empty_result_inserts_notice() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root.md");
        std::fs::write(&root, "# Root\n\n::file-links *.nonexistent\n\n").unwrap();

        let md = Markdown::try_from(root.as_path()).unwrap();
        let options = ComposeOptions::new()
            .with_source_file(&root)
            .with_fail_fast(true)
            .disable(ComposeOperation::Cleanup)
            .disable(ComposeOperation::Normalization);
        let (composed, report) = md.compose_with(options).unwrap();

        let text = composed.content();
        assert!(text.contains("No matching files"), "content: {text}");
        assert_eq!(report.transclusions_skipped, 1);
    }

    #[test]
    fn permissive_empty_result_removes_directive_with_warning() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root.md");
        std::fs::write(&root, "# Root\n\n::file-links *.nonexistent\n\n").unwrap();

        let md = Markdown::try_from(root.as_path()).unwrap();
        let options = ComposeOptions::new()
            .with_source_file(&root)
            .with_fail_fast(false)
            .disable(ComposeOperation::Cleanup)
            .disable(ComposeOperation::Normalization);
        let (composed, report) = md.compose_with(options).unwrap();

        let text = composed.content();
        assert!(
            !text.contains("::file-links"),
            "directive should be removed: {text}"
        );
        assert!(!text.contains("No matching files"), "content: {text}");
        assert_eq!(report.transclusions_skipped, 1);
        assert!(
            report.warnings.iter().any(|w| w.message.contains("No matching files")),
            "expected warning: {:?}",
            report.warnings
        );
    }

    #[test]
    fn dir_target_regular_file_errors_strict() {
        // `--dir` pointed at a regular file is a syntax error, not an empty
        // directory: strict mode fails the compose with a clear diagnostic.
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join(".git")).unwrap();
        std::fs::write(dir.path().join("report.pdf"), "pdf\n").unwrap();
        let root = dir.path().join("root.md");
        std::fs::write(&root, "# Root\n\n::file-links --dir report.pdf\n\n").unwrap();

        let md = Markdown::try_from(root.as_path()).unwrap();
        let options = ComposeOptions::new()
            .with_source_file(&root)
            .with_fail_fast(true)
            .disable(ComposeOperation::Cleanup)
            .disable(ComposeOperation::Normalization);
        let err = md.compose_with(options).unwrap_err();
        assert!(
            err.to_string().contains("not a directory"),
            "expected a not-a-directory error, got: {err}"
        );
    }

    #[test]
    fn dir_target_regular_file_warns_permissive() {
        // Permissive mode removes the directive and records the real
        // not-a-directory diagnostic instead of an empty/misleading warning.
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join(".git")).unwrap();
        std::fs::write(dir.path().join("report.pdf"), "pdf\n").unwrap();
        let root = dir.path().join("root.md");
        std::fs::write(&root, "# Root\n\n::file-links --dir report.pdf\n\n").unwrap();

        let md = Markdown::try_from(root.as_path()).unwrap();
        let options = ComposeOptions::new()
            .with_source_file(&root)
            .with_ignore_invalid_references(Some(true))
            .disable(ComposeOperation::Cleanup)
            .disable(ComposeOperation::Normalization);
        let (composed, report) = md.compose_with(options).unwrap();

        let text = composed.content();
        assert!(
            !text.contains("::file-links"),
            "directive should be removed: {text}"
        );
        assert_eq!(report.transclusions_skipped, 1);
        assert!(
            report
                .warnings
                .iter()
                .any(|w| w.message.contains("not a directory")),
            "expected not-a-directory warning: {:?}",
            report.warnings
        );
    }

    #[test]
    fn operation_disabling_leaves_directive_intact() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root.md");
        std::fs::write(&root, "# Root\n\n::file-links *.md\n\n").unwrap();

        let md = Markdown::try_from(root.as_path()).unwrap();
        let options = ComposeOptions::new()
            .with_source_file(&root)
            .disable(ComposeOperation::FileLinks)
            .disable(ComposeOperation::Cleanup)
            .disable(ComposeOperation::Normalization);
        let (composed, _report) = md.compose_with(options).unwrap();

        let text = composed.content();
        assert!(text.contains("::file-links"), "directive should remain: {text}");
    }

    #[test]
    fn indented_directive_preserves_container_nesting() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join(".git")).unwrap();
        let docs = dir.path().join("docs");
        std::fs::create_dir(&docs).unwrap();
        std::fs::write(docs.join("a.md"), "# A\n").unwrap();

        let root = dir.path().join("root.md");
        std::fs::write(&root, "# Root\n\n- Item\n  ::file-links docs/*\n\n").unwrap();

        let md = Markdown::try_from(root.as_path()).unwrap();
        let options = ComposeOptions::new()
            .with_source_file(&root)
            .disable(ComposeOperation::Cleanup)
            .disable(ComposeOperation::Normalization);
        let (composed, _report) = md.compose_with(options).unwrap();

        let text = composed.content();
        // Each output line should be indented to match the list item
        for line in text.lines().skip(3) {
            if line.contains("a.md") {
                assert!(
                    line.starts_with("  "),
                    "expected indent, got: {line}"
                );
            }
        }
    }

    #[test]
    fn malformed_directive_in_strict_mode_fails() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root.md");
        std::fs::write(&root, "# Root\n\n::file-links\n\n").unwrap();

        let md = Markdown::try_from(root.as_path()).unwrap();
        let options = ComposeOptions::new()
            .with_source_file(&root)
            .with_fail_fast(true)
            .disable(ComposeOperation::Cleanup)
            .disable(ComposeOperation::Normalization);
        let result = md.compose_with(options);
        assert!(result.is_err(), "expected parse error in strict mode");
    }

    #[test]
    fn out_of_bound_path_is_ignored() {
        let dir = tempfile::tempdir().unwrap();
        let docs = dir.path().join("docs");
        std::fs::create_dir(&docs).unwrap();
        std::fs::write(docs.join("a.md"), "# A\n").unwrap();

        let root = dir.path().join("root.md");
        std::fs::write(&root, "# Root\n\n::file-links ../*\n\n").unwrap();

        let md = Markdown::try_from(root.as_path()).unwrap();
        let options = ComposeOptions::new()
            .with_source_file(&root)
            .disable(ComposeOperation::Cleanup)
            .disable(ComposeOperation::Normalization);
        let (composed, report) = md.compose_with(options).unwrap();

        let text = composed.content();
        assert!(
            !text.contains(".."),
            "out-of-bound paths should be excluded: {text}"
        );
        assert_eq!(report.transclusions_skipped, 1);
    }

    #[test]
    fn mixed_case_extensions_are_included() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join(".git")).unwrap();
        let docs = dir.path().join("docs");
        std::fs::create_dir(&docs).unwrap();
        std::fs::write(docs.join("lower.md"), "# L\n").unwrap();
        std::fs::write(docs.join("UPPER.MD"), "# U\n").unwrap();
        std::fs::write(docs.join("MiXeD.Txt"), "# M\n").unwrap();
        std::fs::write(docs.join("binary.exe"), "binary\n").unwrap();

        let root = dir.path().join("root.md");
        std::fs::write(&root, "# Root\n\n::file-links docs/*\n\n").unwrap();

        let md = Markdown::try_from(root.as_path()).unwrap();
        let options = ComposeOptions::new()
            .with_source_file(&root)
            .disable(ComposeOperation::Cleanup)
            .disable(ComposeOperation::Normalization);
        let (composed, _report) = md.compose_with(options).unwrap();

        let text = composed.content();
        assert!(text.contains("lower.md"), "content: {text}");
        assert!(text.contains("UPPER.MD"), "content: {text}");
        assert!(text.contains("MiXeD.Txt"), "content: {text}");
        assert!(
            !text.contains("binary.exe"),
            "unsupported extension should be excluded: {text}"
        );
    }

    #[test]
    fn multiple_directives_produce_deterministic_ordering() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join(".git")).unwrap();
        let docs = dir.path().join("docs");
        std::fs::create_dir(&docs).unwrap();
        std::fs::write(docs.join("a.md"), "# A\n").unwrap();
        let other = dir.path().join("other");
        std::fs::create_dir(&other).unwrap();
        std::fs::write(other.join("b.txt"), "B\n").unwrap();

        let root = dir.path().join("root.md");
        std::fs::write(
            &root,
            "# Root\n\n::file-links docs/*\n\n::file-links other/*\n\n",
        )
        .unwrap();

        let md = Markdown::try_from(root.as_path()).unwrap();
        let options = ComposeOptions::new()
            .with_source_file(&root)
            .disable(ComposeOperation::Cleanup)
            .disable(ComposeOperation::Normalization);
        let (composed, report) = md.compose_with(options).unwrap();

        let text = composed.content();
        // First directive's content should appear before second
        let pos_a = text.find("a.md").expect("a.md present");
        let pos_b = text.find("b.txt").expect("b.txt present");
        assert!(pos_a < pos_b, "deterministic order violated: {text}");
        assert_eq!(report.transclusions_applied, 2);
    }
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
