//! Tests for compose pipeline types moved out of the former `types.rs`.
//! Items now live in `pipeline::operations`, `context::{options,runtime,report}`,
//! and `perf`; these behavior tests exercise them through the compose facade.

    use super::*;
use super::pipeline::operations::COMPOSE_OPERATION_DESCRIPTORS;
    use std::path::PathBuf;
    use std::time::Duration;

    #[test]
    fn compose_source_display_covers_all_variants() {
        assert_eq!(ComposeSource::Unknown.display(), "<stdin>");
        assert_eq!(
            ComposeSource::File(PathBuf::from("/tmp/doc.md")).display(),
            "/tmp/doc.md"
        );
        let url = url::Url::parse("https://example.com/doc.md").unwrap();
        assert_eq!(
            ComposeSource::Url(url).display(),
            "https://example.com/doc.md"
        );
    }

    #[test]
    fn test_compose_options_new_captures_context() {
        let options = ComposeOptions::new();
        let ctx = options.context();

        // Context should have captured current date
        assert!(!ctx.today().is_empty());
        assert!(!ctx.year().is_empty());
        assert!(!ctx.day().is_empty());
    }

    #[test]
    fn test_compose_options_default_stages() {
        let options = ComposeOptions::new();

        assert!(options.is_enabled(ComposeOperation::FrontmatterInterpolation));
        assert!(options.is_enabled(ComposeOperation::TextReplacement));
        assert!(options.is_enabled(ComposeOperation::Interpolation));
        assert!(options.is_enabled(ComposeOperation::Cleanup));
        assert!(options.is_enabled(ComposeOperation::Normalization));
        assert!(options.is_enabled(ComposeOperation::BlockTransclusion));
        assert!(options.is_enabled(ComposeOperation::FrontmatterTransclusion));
    }

    #[test]
    fn test_transclusion_options_defaults() {
        let options = ComposeOptions::new();
        assert_eq!(options.max_transclusion_depth, 16);
        assert!(matches!(options.source, ComposeSource::Unknown));
        assert_eq!(options.code_fallback_language, "txt");
    }

    #[test]
    fn test_compose_options_builder_pattern() {
        let mut options = ComposeOptions::new()
            .disable(ComposeOperation::Cleanup)
            .only(&[
                ComposeOperation::BlockTransclusion,
                ComposeOperation::CodeTransclusion,
            ])
            .with_fail_fast(true)
            .with_external_state(serde_json::json!({"key": "value"}));

        options.max_transclusion_depth = 8;

        assert!(!options.is_enabled(ComposeOperation::Cleanup));
        assert!(options.is_enabled(ComposeOperation::BlockTransclusion));
        assert!(!options.is_enabled(ComposeOperation::FrontmatterTransclusion));
        assert_eq!(options.max_transclusion_depth, 8);
        assert!(options.fail_fast);
        assert!(options.external_state.is_some());
    }

    #[test]
    fn test_compose_options_with_context() {
        let fixed_ctx = ComposeContext::fixed_for_testing();
        let options = ComposeOptions::new().with_context(fixed_ctx.clone());

        assert_eq!(options.context().today(), "2024-06-15");
        assert_eq!(options.context().year(), "2024");
    }

    #[test]
    fn test_compose_options_enable_disable() {
        let mut options = ComposeOptions::new();

        // All operations enabled by default
        assert!(options.is_enabled(ComposeOperation::TextReplacement));
        assert!(options.is_enabled(ComposeOperation::Cleanup));

        // Disable cleanup
        options = options.disable(ComposeOperation::Cleanup);
        assert!(!options.is_enabled(ComposeOperation::Cleanup));
        assert!(options.is_enabled(ComposeOperation::TextReplacement));
    }

    #[test]
    fn test_compose_operation_default_order_exact() {
        assert_eq!(
            ComposeOperation::default_order(),
            &[
                ComposeOperation::FrontmatterInterpolation,
                ComposeOperation::FrontmatterShellExpansion,
                ComposeOperation::TextReplacement,
                ComposeOperation::PageBlocks,
                ComposeOperation::Interpolation,
                ComposeOperation::ShellExpansion,
                ComposeOperation::ShellBlocks,
                ComposeOperation::LinkResolve,
                ComposeOperation::BlockTransclusion,
                ComposeOperation::FrontmatterTransclusion,
                ComposeOperation::CodeTransclusion,
                ComposeOperation::TocLinking,
                ComposeOperation::FileLinks,
                ComposeOperation::Cleanup,
                ComposeOperation::Normalization,
                ComposeOperation::LinkNormalization,
            ]
        );
    }

    #[test]
    fn test_compose_operation_phase_mapping_is_complete() {
        let expectations = [
            (
                ComposeOperation::FrontmatterInterpolation,
                ComposePhase::InlinePre,
            ),
            (
                ComposeOperation::FrontmatterShellExpansion,
                ComposePhase::InlinePre,
            ),
            (ComposeOperation::TextReplacement, ComposePhase::InlinePre),
            (ComposeOperation::PageBlocks, ComposePhase::InlinePre),
            (ComposeOperation::Interpolation, ComposePhase::InlinePre),
            (ComposeOperation::ShellExpansion, ComposePhase::InlinePre),
            (ComposeOperation::ShellBlocks, ComposePhase::InlinePre),
            (
                ComposeOperation::BlockTransclusion,
                ComposePhase::Transclusion,
            ),
            (
                ComposeOperation::FrontmatterTransclusion,
                ComposePhase::Transclusion,
            ),
            (
                ComposeOperation::CodeTransclusion,
                ComposePhase::Transclusion,
            ),
            (ComposeOperation::TocLinking, ComposePhase::Transclusion),
            (ComposeOperation::FileLinks, ComposePhase::Transclusion),
            (ComposeOperation::Cleanup, ComposePhase::InlinePost),
            (ComposeOperation::Normalization, ComposePhase::InlinePost),
            (ComposeOperation::LinkResolve, ComposePhase::InlinePre),
            (
                ComposeOperation::LinkNormalization,
                ComposePhase::Finalization,
            ),
        ];

        for (operation, expected_phase) in expectations {
            assert_eq!(operation.phase(), expected_phase, "{operation:?}");
        }
    }

    #[test]
    fn test_compose_operation_all_is_complete_and_ordered() {
        let all = ComposeOperation::all();
        let collected = all.iter().collect::<Vec<_>>();
        assert_eq!(collected, ComposeOperation::default_order());
        assert_eq!(collected.len(), ComposeOperation::COUNT);
    }

    #[test]
    fn test_compose_options_only() {
        let options = ComposeOptions::new().only(&[
            ComposeOperation::TextReplacement,
            ComposeOperation::Interpolation,
        ]);

        assert!(options.is_enabled(ComposeOperation::TextReplacement));
        assert!(options.is_enabled(ComposeOperation::Interpolation));
        assert!(!options.is_enabled(ComposeOperation::Cleanup));
        assert!(!options.is_enabled(ComposeOperation::Normalization));
        assert!(!options.is_enabled(ComposeOperation::BlockTransclusion));
    }

    #[test]
    fn test_compose_options_only_empty() {
        let options = ComposeOptions::new().only(&[]);

        assert!(!options.is_enabled(ComposeOperation::TextReplacement));
        assert!(!options.is_enabled(ComposeOperation::Cleanup));
        assert!(!options.is_enabled(ComposeOperation::BlockTransclusion));
    }

    #[test]
    fn test_compose_options_flat_builders() {
        let handler: std::sync::Arc<dyn super::shell_expansion::ShellApprovalHandler> =
            std::sync::Arc::new(TestApprovalHandler);
        let options = ComposeOptions::new()
            .with_shell_timeout(std::time::Duration::from_secs(3))
            .with_shell_policy_root("/tmp/policy")
            .with_shell_working_directory("/tmp/work")
            .with_shell_approval_handler(handler)
            .with_allow_remote_transclusion(true)
            .with_allow_local_markdown(false)
            .with_allow_local_code(false)
            .with_max_transclusion_depth(4)
            .with_ignore_invalid_references(Some(true))
            .with_resolve_repo_root(false)
            .with_code_fallback_language("md");

        assert_eq!(options.shell_timeout, std::time::Duration::from_secs(3));
        assert_eq!(
            options.shell_policy_root,
            Some(PathBuf::from("/tmp/policy"))
        );
        assert_eq!(
            options.shell_working_directory,
            Some(PathBuf::from("/tmp/work"))
        );
        assert!(options.shell_approval_handler.is_some());
        assert!(options.allow_remote_transclusion);
        assert!(!options.allow_local_markdown);
        assert!(!options.allow_local_code);
        assert_eq!(options.max_transclusion_depth, 4);
        assert_eq!(options.ignore_invalid_references, Some(true));
        assert!(!options.resolve_repo_root);
        assert_eq!(options.code_fallback_language, "md");
    }

    #[test]
    fn test_compose_context_capture() {
        let ctx = ComposeContext::capture();

        // Should have reasonable values
        assert!(ctx.year().parse::<i32>().is_ok());
        assert!(ctx.month().len() == 2);
        assert!(!ctx.today().is_empty());
        assert!(!ctx.yesterday().is_empty());
        assert!(!ctx.tomorrow().is_empty());
    }

    #[test]
    fn test_compose_context_fixed_for_testing() {
        let ctx = ComposeContext::fixed_for_testing();

        assert_eq!(ctx.today(), "2024-06-15");
        assert_eq!(ctx.yesterday(), "2024-06-14");
        assert_eq!(ctx.tomorrow(), "2024-06-16");
        assert_eq!(ctx.day(), "Saturday");
        assert_eq!(ctx.year(), "2024");
        assert_eq!(ctx.month(), "06");
    }

    #[test]
    fn test_compose_report_new() {
        let report = ComposeReport::new();

        assert_eq!(report.replacements_applied, 0);
        assert_eq!(report.interpolations_applied, 0);
        assert!(!report.cleanup_changed);
        assert!(report.normalization_report.is_none());
        assert_eq!(report.transclusions_applied, 0);
        assert_eq!(report.transclusions_skipped, 0);
        assert_eq!(report.max_transclusion_depth, 0);
        assert!(report.warnings.is_empty());
    }

    #[test]
    fn test_compose_report_has_changes() {
        let mut report = ComposeReport::new();
        assert!(!report.has_changes());

        report.replacements_applied = 1;
        assert!(report.has_changes());
    }

    #[test]
    fn test_compose_report_summary() {
        let mut report = ComposeReport::new();
        assert_eq!(report.summary(), "No changes made");

        report.replacements_applied = 2;
        report.interpolations_applied = 3;
        report.cleanup_changed = true;
        report.transclusions_applied = 1;
        report.transclusions_skipped = 1;

        let summary = report.summary();
        assert!(summary.contains("2 replacement(s)"));
        assert!(summary.contains("3 interpolation(s)"));
        assert!(summary.contains("cleanup applied"));
        assert!(summary.contains("1 transclusion(s)"));
        assert!(summary.contains("1 transclusion(s) skipped"));
    }

    #[test]
    fn test_compose_warning() {
        let warning = ComposeWarning::new("interpolation", "Missing variable: foo");
        assert_eq!(warning.stage, "interpolation");
        assert_eq!(warning.message, "Missing variable: foo");
        assert!(warning.line_number.is_none());

        let warning_with_line = warning.at_line(42);
        assert_eq!(warning_with_line.line_number, Some(42));
    }

    #[test]
    fn test_compose_report_add_warning() {
        let mut report = ComposeReport::new();
        report.add_warning(ComposeWarning::new("test", "test warning"));

        assert_eq!(report.warnings.len(), 1);
        assert_eq!(report.warnings[0].message, "test warning");
    }

    struct TestApprovalHandler;

    impl super::shell_expansion::ShellApprovalHandler for TestApprovalHandler {
        fn approve(
            &self,
            _request: super::shell_expansion::ShellApprovalRequest,
        ) -> Result<
            super::shell_expansion::ShellApprovalDecision,
            super::ShellExpansionError,
        > {
            Ok(super::shell_expansion::ShellApprovalDecision::Deny)
        }
    }

    #[test]
    fn with_magic_path_accumulates_entries() {
        use biscuit_file::PathPosition;

        let options = ComposeOptions::new()
            .with_magic_path("/project/.claudine", PathPosition::Start)
            .with_magic_path("/home/user/.claudine", PathPosition::Start)
            .with_magic_path("/fallback", PathPosition::End);

        assert_eq!(options.magic_paths.len(), 3);
        assert_eq!(
            options.magic_paths[0].0,
            PathBuf::from("/project/.claudine")
        );
        assert_eq!(options.magic_paths[0].1, PathPosition::Start);
        assert_eq!(
            options.magic_paths[1].0,
            PathBuf::from("/home/user/.claudine")
        );
        assert_eq!(options.magic_paths[1].1, PathPosition::Start);
        assert_eq!(options.magic_paths[2].0, PathBuf::from("/fallback"));
        assert_eq!(options.magic_paths[2].1, PathPosition::End);
    }

    #[test]
    fn magic_paths_appear_in_transclusion_options() {
        use biscuit_file::PathPosition;

        let options = ComposeOptions::new().with_magic_path("/custom/root", PathPosition::Start);

        let transclusion = options.transclusion_options();
        assert_eq!(transclusion.magic_paths.len(), 1);
        assert_eq!(transclusion.magic_paths[0].0, PathBuf::from("/custom/root"));
        assert_eq!(transclusion.magic_paths[0].1, PathPosition::Start);
    }

    #[test]
    fn magic_paths_default_empty() {
        let options = ComposeOptions::new();
        assert!(options.magic_paths.is_empty());

        let transclusion = options.transclusion_options();
        assert!(transclusion.magic_paths.is_empty());
    }

    #[test]
    fn perf_disabled_by_default() {
        let options = ComposeOptions::new();
        assert!(!options.perf_enabled);
    }

    #[test]
    fn with_perf_enables_collection() {
        let options = ComposeOptions::new().with_perf(true);
        assert!(options.perf_enabled);
    }

    #[test]
    fn compose_report_perf_none_by_default() {
        let report = ComposeReport::new();
        assert!(report.perf.is_none());
    }

    #[test]
    fn compose_perf_report_merge_sums_durations() {
        let mut parent = ComposePerfReport {
            total: Duration::from_millis(10),
            metrics: vec![
                ComposePerfMetric {
                    stage: ComposeStage::Cleanup,
                    elapsed: Duration::from_millis(3),
                    calls: 1,
                },
                ComposePerfMetric {
                    stage: ComposeStage::Interpolation,
                    elapsed: Duration::from_millis(5),
                    calls: 2,
                },
            ],
            ..Default::default()
        };
        let child = ComposePerfReport {
            total: Duration::from_millis(7),
            metrics: vec![
                ComposePerfMetric {
                    stage: ComposeStage::Cleanup,
                    elapsed: Duration::from_millis(2),
                    calls: 1,
                },
                ComposePerfMetric {
                    stage: ComposeStage::Normalization,
                    elapsed: Duration::from_millis(4),
                    calls: 1,
                },
            ],
            ..Default::default()
        };

        parent.merge(&child);

        assert_eq!(parent.total, Duration::from_millis(17));

        let cleanup = parent
            .metrics
            .iter()
            .find(|m| m.stage == ComposeStage::Cleanup)
            .unwrap();
        assert_eq!(cleanup.elapsed, Duration::from_millis(5));
        assert_eq!(cleanup.calls, 2);

        let interp = parent
            .metrics
            .iter()
            .find(|m| m.stage == ComposeStage::Interpolation)
            .unwrap();
        assert_eq!(interp.elapsed, Duration::from_millis(5));
        assert_eq!(interp.calls, 2);

        let norm = parent
            .metrics
            .iter()
            .find(|m| m.stage == ComposeStage::Normalization)
            .unwrap();
        assert_eq!(norm.elapsed, Duration::from_millis(4));
        assert_eq!(norm.calls, 1);
    }

    #[test]
    fn compose_report_merge_with_perf_none_and_some() {
        let mut report_a = ComposeReport::new();
        assert!(report_a.perf.is_none());

        let mut report_b = ComposeReport::new();
        report_b.perf = Some(ComposePerfReport {
            total: Duration::from_millis(5),
            metrics: vec![ComposePerfMetric {
                stage: ComposeStage::Cleanup,
                elapsed: Duration::from_millis(5),
                calls: 1,
            }],
            ..Default::default()
        });

        report_a.merge(report_b);
        assert!(report_a.perf.is_some());
        assert_eq!(
            report_a.perf.as_ref().unwrap().total,
            Duration::from_millis(5)
        );
    }

    #[test]
    fn compose_report_merge_with_both_perf() {
        let mut report_a = ComposeReport::new();
        report_a.perf = Some(ComposePerfReport {
            total: Duration::from_millis(10),
            metrics: vec![ComposePerfMetric {
                stage: ComposeStage::Cleanup,
                elapsed: Duration::from_millis(3),
                calls: 1,
            }],
            ..Default::default()
        });

        let mut report_b = ComposeReport::new();
        report_b.perf = Some(ComposePerfReport {
            total: Duration::from_millis(7),
            metrics: vec![ComposePerfMetric {
                stage: ComposeStage::Cleanup,
                elapsed: Duration::from_millis(2),
                calls: 1,
            }],
            ..Default::default()
        });

        report_a.merge(report_b);
        let perf = report_a.perf.as_ref().unwrap();
        assert_eq!(perf.total, Duration::from_millis(17));
        assert_eq!(perf.metrics[0].elapsed, Duration::from_millis(5));
        assert_eq!(perf.metrics[0].calls, 2);
    }

    #[test]
    fn compose_options_default_timeout_behavior_is_error() {
        let options = ComposeOptions::new();
        assert_eq!(options.shell_timeout_behavior, ShellTimeoutBehavior::Error);
    }

    #[test]
    fn with_shell_timeout_behavior_sets_value() {
        let options =
            ComposeOptions::new().with_shell_timeout_behavior(ShellTimeoutBehavior::EmptyString);
        assert_eq!(
            options.shell_timeout_behavior,
            ShellTimeoutBehavior::EmptyString
        );
    }

    #[test]
    fn with_allow_shell_timeout_sets_empty_string() {
        let options = ComposeOptions::new().with_allow_shell_timeout(true);
        assert_eq!(
            options.shell_timeout_behavior,
            ShellTimeoutBehavior::EmptyString
        );
    }

    #[test]
    fn with_allow_shell_timeout_false_keeps_error() {
        let options = ComposeOptions::new().with_allow_shell_timeout(false);
        assert_eq!(options.shell_timeout_behavior, ShellTimeoutBehavior::Error);
    }

    #[test]
    fn frontmatter_shell_expansion_is_inline_pre() {
        assert_eq!(
            ComposeOperation::FrontmatterShellExpansion.phase(),
            ComposePhase::InlinePre
        );
    }

    #[test]
    fn frontmatter_shell_expansion_follows_interpolation_in_default_order() {
        let order = ComposeOperation::default_order();
        let fm_interp_pos = order
            .iter()
            .position(|op| *op == ComposeOperation::FrontmatterInterpolation)
            .unwrap();
        let fm_shell_pos = order
            .iter()
            .position(|op| *op == ComposeOperation::FrontmatterShellExpansion)
            .unwrap();
        assert_eq!(fm_shell_pos, fm_interp_pos + 1);
    }

    #[test]
    fn compose_report_tracks_frontmatter_shell_expansions() {
        let report = ComposeReport::new();
        assert_eq!(report.frontmatter_shell_expansions_applied, 0);
    }

    #[test]
    fn compose_stage_phase_mapping() {
        assert_eq!(ComposeStage::ShellExpansion.phase(), ComposePhase::InlinePre);
        assert_eq!(
            ComposeStage::TransclusionApply.phase(),
            ComposePhase::Transclusion
        );
        assert_eq!(ComposeStage::Cleanup.phase(), ComposePhase::InlinePost);
        assert_eq!(
            ComposeStage::LinkNormalization.phase(),
            ComposePhase::Finalization
        );
    }

    #[test]
    fn redact_shell_command_masks_bearer_token() {
        let out = redact_shell_command("curl -H 'Authorization: Bearer abc123def456'");
        assert!(out.contains("Bearer ***"), "got: {out}");
        assert!(!out.contains("abc123def456"), "got: {out}");
    }

    #[test]
    fn redact_shell_command_masks_token_flag() {
        let out = redact_shell_command("gh --token=ghp_secretvalue123 repo list");
        assert!(out.contains("--token=***"), "got: {out}");
        assert!(!out.contains("ghp_secretvalue123"), "got: {out}");
    }

    #[test]
    fn redact_shell_command_masks_space_separated_flag() {
        let out = redact_shell_command("tool --password hunter2value list");
        assert!(out.contains("--password ***"), "got: {out}");
        assert!(!out.contains("hunter2value"), "got: {out}");
    }

    #[test]
    fn redact_shell_command_masks_url_credentials() {
        let out = redact_shell_command("git clone https://user:secretpw@example.com/repo.git");
        assert!(out.contains("https://***@example.com"), "got: {out}");
        assert!(!out.contains("secretpw"), "got: {out}");
    }

    #[test]
    fn redact_shell_command_masks_query_secret() {
        let out = redact_shell_command("curl 'https://api.test/v1?access_token=qsValue9876xyz'");
        assert!(out.contains("access_token=***"), "got: {out}");
        assert!(!out.contains("qsValue9876xyz"), "got: {out}");
    }

    #[test]
    fn redact_shell_command_masks_long_jwt_blob() {
        // A JWT-like opaque blob (letters + digits, >= 40 chars, no slash).
        let jwt = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.payload1234567890abcdef";
        let out = redact_shell_command(&format!("echo {jwt}"));
        assert!(out.contains("***"), "got: {out}");
        assert!(!out.contains(jwt), "got: {out}");
    }

    #[test]
    fn redact_shell_command_normalizes_whitespace() {
        let out = redact_shell_command("echo\t hello\n\n   world");
        assert_eq!(out, "echo hello world");
    }

    #[test]
    fn redact_shell_command_length_caps() {
        let raw = "echo ".to_string() + &"a".repeat(200);
        let out = redact_shell_command(&raw);
        // 80 chars + the ellipsis.
        assert_eq!(out.chars().count(), 81, "got: {out}");
        assert!(out.ends_with('…'), "got: {out}");
    }

    #[test]
    fn default_order_has_sixteen_operations() {
        assert_eq!(ComposeOperation::default_order().len(), 16);
    }

    #[test]
    fn link_resolve_is_last_inline_pre_operation() {
        let order = ComposeOperation::default_order();
        let inline_pre = order
            .iter()
            .copied()
            .filter(|op| op.phase() == ComposePhase::InlinePre)
            .collect::<Vec<_>>();
        assert_eq!(
            inline_pre.last().copied(),
            Some(ComposeOperation::LinkResolve)
        );
    }

    #[test]
    fn link_normalization_is_sole_finalization_operation() {
        let finalization = ComposeOperation::default_order()
            .iter()
            .copied()
            .filter(|op| op.phase() == ComposePhase::Finalization)
            .collect::<Vec<_>>();
        assert_eq!(finalization, vec![ComposeOperation::LinkNormalization]);
    }

    #[test]
    fn finalization_phase_displays_lowercase() {
        assert_eq!(format!("{}", ComposePhase::Finalization), "finalization");
    }

    #[test]
    fn env_path_whitelist_default_is_empty_with_known_fallbacks() {
        let options = ComposeOptions::new();
        assert!(options.env_path_whitelist.is_empty());
        assert_eq!(
            options.effective_env_path_whitelist(),
            vec!["PROJECT_ROOT".to_string(), "DOCS_BASE".to_string()]
        );
    }

    #[test]
    fn with_env_path_whitelist_overrides_default() {
        let options = ComposeOptions::new()
            .with_env_path_whitelist(vec!["MY_VAR".to_string(), "OTHER".to_string()]);
        assert_eq!(
            options.effective_env_path_whitelist(),
            vec!["MY_VAR".to_string(), "OTHER".to_string()]
        );
    }

    #[test]
    fn remote_read_config_defaults_to_deny_all() {
        let options = ComposeOptions::new();
        let config = options.remote_read_config();
        assert!(config.allowed_hosts.is_empty());
        assert_eq!(
            config.remote_concurrency,
            crate::markdown::compose::remote::DEFAULT_REMOTE_CONCURRENCY
        );
        assert_eq!(config.freshness_mode, RemoteFreshnessMode::Fallback);
    }

    #[test]
    fn with_allowed_host_adds_to_allowlist() {
        let options = ComposeOptions::new()
            .with_allowed_host("example.com")
            .with_allowed_host("cdn.example.com");
        let config = options.remote_read_config();
        assert!(config.is_host_allowed("example.com"));
        assert!(config.is_host_allowed("cdn.example.com"));
        assert!(!config.is_host_allowed("other.com"));
    }

    #[test]
    fn with_remote_concurrency_sets_value() {
        let options = ComposeOptions::new().with_remote_concurrency(8);
        assert_eq!(options.remote_read_config().remote_concurrency, 8);
    }

    #[test]
    fn with_remote_ttl_sets_duration() {
        let options = ComposeOptions::new().with_remote_ttl(Some(Duration::from_secs(300)));
        assert_eq!(
            options.remote_read_config().remote_ttl,
            Some(Duration::from_secs(300))
        );
    }

    #[test]
    fn with_remote_refresh_sets_flag() {
        let options = ComposeOptions::new().with_remote_refresh(true);
        assert!(options.remote_read_config().refresh);
    }

    #[test]
    fn with_remote_freshness_mode_sets_mode() {
        let options =
            ComposeOptions::new().with_remote_freshness_mode(RemoteFreshnessMode::Optimistic);
        assert_eq!(
            options.remote_read_config().freshness_mode,
            RemoteFreshnessMode::Optimistic
        );
    }

    #[test]
    fn with_remote_read_config_replaces_entire_config() {
        let custom = RemoteReadConfig {
            allowed_hosts: vec!["custom.host".to_string()],
            remote_concurrency: 2,
            remote_ttl: Some(Duration::from_secs(60)),
            refresh: true,
            freshness_mode: RemoteFreshnessMode::Fallback,
        };
        let options = ComposeOptions::new().with_remote_read_config(custom);
        let config = options.remote_read_config();
        assert!(config.is_host_allowed("custom.host"));
        assert_eq!(config.remote_concurrency, 2);
        assert_eq!(config.remote_ttl, Some(Duration::from_secs(60)));
        assert!(config.refresh);
        assert_eq!(config.freshness_mode, RemoteFreshnessMode::Fallback);
    }

    #[test]
    fn compose_operation_descriptors_cover_all_variants() {
        assert_eq!(COMPOSE_OPERATION_DESCRIPTORS.len(), ComposeOperation::COUNT);

        let mut seen_indices = std::collections::HashSet::new();
        for descriptor in COMPOSE_OPERATION_DESCRIPTORS.iter() {
            assert!(
                seen_indices.insert(descriptor.index),
                "duplicate descriptor index {} for {:?}",
                descriptor.index,
                descriptor.operation
            );
            assert_eq!(
                descriptor.index,
                descriptor.operation.index(),
                "descriptor index mismatch for {:?}",
                descriptor.operation
            );
        }

        assert_eq!(seen_indices.len(), ComposeOperation::COUNT);
        for expected in 0..ComposeOperation::COUNT {
            assert!(
                seen_indices.contains(&expected),
                "missing descriptor index {expected}"
            );
        }

        // Every enum variant must appear exactly once in the descriptor table.
        let descriptor_ops: std::collections::HashSet<_> = COMPOSE_OPERATION_DESCRIPTORS
            .iter()
            .map(|d| d.operation)
            .collect();
        assert_eq!(descriptor_ops.len(), ComposeOperation::COUNT);
        for operation in ComposeOperation::default_order() {
            assert!(
                descriptor_ops.contains(operation),
                "missing descriptor for {:?}",
                operation
            );
        }
    }

    #[test]
    fn compose_operation_default_order_matches_descriptor_enabled_order() {
        let expected: Vec<_> = COMPOSE_OPERATION_DESCRIPTORS
            .iter()
            .filter(|d| d.default_enabled)
            .map(|d| d.operation)
            .collect();
        assert_eq!(
            ComposeOperation::default_order(),
            expected.as_slice(),
            "default_order() must equal descriptors filtered by default_enabled"
        );
    }

    #[test]
    fn compose_operation_perf_mapping_is_exhaustive_and_consistent() {
        for operation in ComposeOperation::default_order() {
            let descriptor = operation.descriptor();
            match operation.phase() {
                ComposePhase::InlinePre | ComposePhase::InlinePost | ComposePhase::Finalization => {
                    assert!(
                        descriptor.perf_kind.is_some(),
                        "{:?} in phase {:?} must have an operation-level perf metric",
                        operation,
                        operation.phase()
                    );
                }
                ComposePhase::Transclusion => {
                    assert!(
                        descriptor.perf_kind.is_none(),
                        "{:?} is a transclusion operation and must use transclusion sub-stage \
                         perf metrics (parse/prepare/resolve/apply) rather than an operation-level metric",
                        operation
                    );
                }
            }
        }
    }

    #[test]
    fn compose_operation_label_returns_descriptor_label() {
        assert_eq!(
            ComposeOperation::FrontmatterInterpolation.label(),
            "frontmatter interpolation"
        );
        assert_eq!(ComposeOperation::BlockTransclusion.label(), "block transclusion");
        assert_eq!(ComposeOperation::LinkNormalization.label(), "link normalization");
    }
