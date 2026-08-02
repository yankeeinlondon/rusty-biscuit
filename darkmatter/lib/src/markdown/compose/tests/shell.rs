use super::*;

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

        let options = context_free_options()
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

        let options = context_free_options()
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

        let options = context_free_options()
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
            context_free_options().only(&[ComposeOperation::FrontmatterShellExpansion]);

        let (composed, report) = md.compose_with(options).unwrap();
        assert_eq!(report.frontmatter_shell_expansions_applied, 0);
        assert!(composed.content().contains("Body text"));
    }

    #[test]
    fn frontmatter_shell_timeout_empty_emits_warning() {
        let temp_dir = TempDir::new().unwrap();
        let content = "---\nval: \"$(sleep 1)\"\n---\nValue: {{val}}\n";
        let md: Markdown = content.into();

        let options = context_free_options()
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

        let options = context_free_options().only(&[
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

        let options = context_free_options()
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

        let options = context_free_options()
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

        let options = context_free_options()
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

        let options = context_free_options()
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

        let options = context_free_options()
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
        let options = context_free_options().only(&[ComposeOperation::PageBlocks]);
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

        let options = context_free_options()
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

        let options = context_free_options()
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
        let options = context_free_options().only(&[ComposeOperation::PageBlocks]);
        let err = md.compose_with(options).unwrap_err();

        let err_string = format!("{}", err);
        assert!(
            err_string.contains("Unexpected '|'") || err_string.contains("logical OR"),
            "Expected bare pipe error in condition, got: {}",
            err_string
        );
    }
}


