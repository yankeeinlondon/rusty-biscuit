use super::*;

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

fn frontmatter_file_reference_error(
    error: &MarkdownError,
) -> &biscuit_file::FileReferenceError {
    match error {
        MarkdownError::Transclusion(inner) => match inner.as_ref() {
            transclusion::TransclusionError::FileReference(source) => source,
            other => panic!("expected a file-reference transclusion error, got: {other:?}"),
        },
        other => panic!("expected a transclusion error, got: {other:?}"),
    }
}

#[test]
fn test_stage2_frontmatter_reference_parse_errors_match_preflight_and_execution() {
    for property in ["prologue", "epilogue"] {
        for reference in ["@//escape.md", "%@//escape.md", "~alice/secret.md"] {
            let dir = tempfile::tempdir().unwrap();
            let root = dir.path().join("root.md");
            std::fs::write(
                &root,
                format!("---\n{property}: {reference:?}\n---\nBody"),
            )
            .unwrap();

            let md = Markdown::try_from(root.as_path()).unwrap();
            let options = ComposeOptions::new()
                .with_source_file(&root)
                .with_ignore_invalid_references(Some(true));
            let preflight_error = md.compose_preflight(&options).unwrap_err();
            let execution_error = md.compose_with(options).unwrap_err();
            let preflight_source = frontmatter_file_reference_error(&preflight_error);
            let execution_source = frontmatter_file_reference_error(&execution_error);

            assert_eq!(preflight_source.to_string(), execution_source.to_string());
            if reference.starts_with('~') {
                assert!(matches!(
                    preflight_source,
                    biscuit_file::FileReferenceError::UnsupportedUserHome(_)
                ));
                assert!(matches!(
                    execution_source,
                    biscuit_file::FileReferenceError::UnsupportedUserHome(_)
                ));
            } else {
                assert!(matches!(
                    preflight_source,
                    biscuit_file::FileReferenceError::InvalidSyntax(_)
                ));
                assert!(matches!(
                    execution_source,
                    biscuit_file::FileReferenceError::InvalidSyntax(_)
                ));
            }
        }
    }
}

#[test]
fn test_stage2_frontmatter_inline_content_matches_preflight_and_execution() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("root.md");
    std::fs::write(
        &root,
        "---\nprologue: Inline preface text\nepilogue: Inline closing text\n---\nBody",
    )
    .unwrap();

    let md = Markdown::try_from(root.as_path()).unwrap();
    let options = ComposeOptions::new().with_source_file(&root);
    let preflight = md.compose_preflight(&options).unwrap();
    assert!(preflight.preflight_graph.edges.is_empty());

    let (composed, report) = md.compose_with(options).unwrap();
    assert!(composed.content().starts_with("Inline preface text"));
    assert!(
        composed
            .content()
            .trim_end_matches('\n')
            .ends_with("Inline closing text")
    );
    assert_eq!(report.transclusions_applied, 0);
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
    async fn mixed_case_http_file_and_code_targets_execute_as_remote_transclusions() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/remote.md"))
            .respond_with(ResponseTemplate::new(200).set_body_string("remote markdown"))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/snippet.rs"))
            .respond_with(ResponseTemplate::new(200).set_body_string("fn mixed_case() {}"))
            .mount(&server)
            .await;

        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root.md");
        let remote_base = server.uri().replacen("http://", "hTtP://", 1);
        let content = format!(
            "::file {remote_base}/remote.md\n\n::code {remote_base}/snippet.rs\n"
        );
        std::fs::write(&root, &content).unwrap();

        let (composed, report) =
            compose_with_remote(&content, &root, &server, vec!["127.0.0.1".into()])
                .await
                .unwrap();

        assert!(composed.content().contains("remote markdown"));
        assert!(composed.content().contains("fn mixed_case()"));
        assert_eq!(report.transclusions_applied, 2);
        assert_eq!(report.remote_fetch_stats.unwrap().fetched, 2);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn mixed_case_http_target_is_denied_by_remote_host_policy() {
        let server = MockServer::start().await;
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root.md");
        let remote_url = format!(
            "{}/blocked.md",
            server.uri().replacen("http://", "HtTp://", 1)
        );
        let content = format!("::file {remote_url}\n");
        std::fs::write(&root, &content).unwrap();

        let err = compose_with_remote(&content, &root, &server, Vec::new())
            .await
            .expect_err("mixed-case HTTP must remain subject to the deny-all policy");

        assert!(
            err.to_string().contains("denied") || err.to_string().contains("not allowed"),
            "expected remote policy denial, got: {err}"
        );
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
