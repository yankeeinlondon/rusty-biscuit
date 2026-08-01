    use super::*;
    use serde_json::Value;

    #[test]
    fn ordinary_memory_file_is_rejected_before_body_composition() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("CLAUDE.md");
        std::fs::write(&path, "# Memory\n\n::shell rm -rf forbidden\n").unwrap();

        assert!(!wrapper_harness_frontmatter_enabled(&path).unwrap());

        let invocation = claudine::invocation_context::InvocationContext::capture_at(
            directory.path(),
        );
        let profile = super::super::profile::profile_for_provider(Provider::Claude).unwrap();
        let result = detect_wrapper_harness(
            Provider::Claude,
            profile,
            &profile::PromptSource::Inline("prompt".to_string()),
            None,
            &env::EnvPlan::default(),
            directory.path(),
            directory.path(),
            &invocation,
        )
        .unwrap();
        assert!(result.0.is_none());
        let work = invocation.work_snapshot();
        assert_eq!(work.harness_eligibility_parses, 1);
        assert_eq!(work.harness_materializations, 0);
        assert_eq!(work.compose_operations, 0);
    }

    #[test]
    fn no_prompt_skips_memory_lookup_and_harness_work() {
        let directory = tempfile::tempdir().unwrap();
        std::fs::write(directory.path().join("CLAUDE.md"), "---\nloop: {}\n---\nbody\n")
            .unwrap();
        let invocation = claudine::invocation_context::InvocationContext::capture_at(
            directory.path(),
        );
        let profile = super::super::profile::profile_for_provider(Provider::Claude).unwrap();

        let result = detect_wrapper_harness(
            Provider::Claude,
            profile,
            &profile::PromptSource::None,
            None,
            &env::EnvPlan::default(),
            directory.path(),
            directory.path(),
            &invocation,
        )
        .unwrap();

        assert!(result.0.is_none());
        let work = invocation.work_snapshot();
        assert_eq!(work.harness_eligibility_parses, 0);
        assert_eq!(work.harness_materializations, 0);
    }

    #[test]
    fn prompt_without_memory_candidate_skips_all_harness_work() {
        let directory = tempfile::tempdir().unwrap();
        let invocation = claudine::invocation_context::InvocationContext::capture_at(
            directory.path(),
        );
        let profile = super::super::profile::profile_for_provider(Provider::Claude).unwrap();

        let result = detect_wrapper_harness(
            Provider::Claude,
            profile,
            &profile::PromptSource::Inline("prompt".to_string()),
            None,
            &env::EnvPlan::default(),
            directory.path(),
            directory.path(),
            &invocation,
        )
        .unwrap();

        assert!(result.0.is_none());
        let work = invocation.work_snapshot();
        assert_eq!(work.harness_eligibility_parses, 0);
        assert_eq!(work.harness_materializations, 0);
        assert_eq!(work.compose_operations, 0);
    }

    #[test]
    fn enabled_memory_harness_uses_one_materialization_and_request_owned_compose() {
        let directory = tempfile::tempdir().unwrap();
        std::fs::write(
            directory.path().join("CLAUDE.md"),
            "---\ntimeout: 1m\n---\nMemory body.\n",
        )
        .unwrap();
        let invocation = claudine::invocation_context::InvocationContext::capture_at(
            directory.path(),
        );
        let profile = super::super::profile::profile_for_provider(Provider::Claude).unwrap();

        let result = detect_wrapper_harness(
            Provider::Claude,
            profile,
            &profile::PromptSource::Inline("prompt".to_string()),
            None,
            &env::EnvPlan::default(),
            directory.path(),
            directory.path(),
            &invocation,
        )
        .unwrap();

        assert!(result.0.is_some());
        let work = invocation.work_snapshot();
        assert_eq!(work.harness_eligibility_parses, 1);
        assert_eq!(work.harness_materializations, 1);
        assert_eq!(work.compose_operations, 1);
        assert_eq!(work.git_root_discoveries, 1);
        assert_eq!(work.topology_probes, 0);
        assert_eq!(work.ambient_fallbacks, 0);
        for group in ["os", "hardware", "gpu", "git", "repo", "file_changes"] {
            assert_eq!(
                work.runtime_evidence_captures.get(group),
                None,
                "unreferenced host group `{group}` should not be captured"
            );
        }
    }

    #[test]
    fn enabled_memory_harness_composes_requested_runtime_facets_from_invocation_evidence() {
        let directory = tempfile::tempdir().unwrap();
        std::fs::write(
            directory.path().join("CLAUDE.md"),
            "---\ntimeout: 1m\nresolved_os: \"{{ ctx.os }}\"\n---\nMemory body.\n",
        )
        .unwrap();
        let invocation = claudine::invocation_context::InvocationContext::capture_at(
            directory.path(),
        );
        let profile = super::super::profile::profile_for_provider(Provider::Claude).unwrap();

        let result = detect_wrapper_harness(
            Provider::Claude,
            profile,
            &profile::PromptSource::Inline("prompt".to_string()),
            None,
            &env::EnvPlan::default(),
            directory.path(),
            directory.path(),
            &invocation,
        )
        .unwrap();
        let (_, _, materialized, _) = result.0.expect("the timeout activates the harness");

        let resolved_os = materialized.frontmatter["resolved_os"]
            .as_str()
            .expect("frontmatter interpolation preserves a string");
        assert!(!resolved_os.is_empty());
        assert_ne!(resolved_os, "{{ ctx.os }}");
        assert_eq!(
            invocation
                .work_snapshot()
                .runtime_evidence_captures
                .get("os"),
            Some(&1)
        );
    }

    #[test]
    fn malformed_memory_frontmatter_remains_an_error() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("CLAUDE.md");
        std::fs::write(&path, "---\ninitialize: [\n---\nbody\n").unwrap();

        let error = wrapper_harness_frontmatter_enabled(&path).unwrap_err();
        assert!(error.downcast_ref::<darkmatter::markdown::MarkdownError>().is_some());
    }

    #[test]
    fn malformed_memory_frontmatter_fails_the_full_harness_detection_path() {
        let directory = tempfile::tempdir().unwrap();
        std::fs::write(
            directory.path().join("CLAUDE.md"),
            "---\ntimeout: [\n---\nbody\n",
        )
        .unwrap();
        let invocation = claudine::invocation_context::InvocationContext::capture_at(
            directory.path(),
        );
        let profile = super::super::profile::profile_for_provider(Provider::Claude).unwrap();

        let error = match detect_wrapper_harness(
            Provider::Claude,
            profile,
            &profile::PromptSource::Inline("prompt".to_string()),
            None,
            &env::EnvPlan::default(),
            directory.path(),
            directory.path(),
            &invocation,
        ) {
            Ok(_) => panic!("malformed memory frontmatter must fail detection"),
            Err(error) => error,
        };

        assert!(error.downcast_ref::<darkmatter::markdown::MarkdownError>().is_some());
        let work = invocation.work_snapshot();
        assert_eq!(work.harness_eligibility_parses, 1);
        assert_eq!(work.harness_materializations, 0);
        assert_eq!(work.compose_operations, 0);
    }

    /// Parse a `WrapperArgs` from a bare arg list via a clap probe so the
    /// `--stall-timeout` validation can be exercised without constructing the
    /// struct by hand.
    fn wrapper_args_from(extra: &[&str]) -> WrapperArgs {
        use clap::Parser;

        // `WrapperArgs` declares its own `help` field, so the auto-generated
        // `--help` must be disabled to avoid a duplicate-argument panic.
        #[derive(Debug, clap::Parser)]
        #[command(disable_help_flag = true)]
        struct Probe {
            #[command(flatten)]
            args: WrapperArgs,
        }

        let mut argv = vec!["probe"];
        argv.extend_from_slice(extra);
        Probe::try_parse_from(argv)
            .expect("probe must parse")
            .args
    }

    #[test]
    fn parse_cli_timeouts_rejects_invalid_stall_timeout() {
        let args = wrapper_args_from(&["--stall-timeout", "nope"]);
        let err = parse_cli_timeouts(&args).unwrap_err();
        assert!(
            err.to_string().contains("invalid --stall-timeout value"),
            "got: {err}"
        );
    }

    #[test]
    fn parse_cli_timeouts_accepts_zero_and_valid_stall_timeout() {
        for value in ["0s", "0.5s", "10m"] {
            let args = wrapper_args_from(&["--stall-timeout", value]);
            assert!(
                parse_cli_timeouts(&args).is_ok(),
                "--stall-timeout {value} should be accepted"
            );
        }
    }

    /// The rebuilt launch identity's MCP facet must track the switches the
    /// wrapper was actually invoked with. Pinning it to `false` is invisible
    /// while the direct-wrapper lifecycle stays empty, but it silently
    /// activates a stale compatibility path the moment retry/resume becomes
    /// reachable here — a refreshed body's `#tag`s would stop moving the facet.
    #[test]
    fn passthrough_launch_intent_mcp_tracks_wrapper_switches() {
        let cases: [(&[&str], bool); 5] = [
            (&[], false),
            (&["--mcp"], true),
            (&["--use", "context7"], true),
            (&["--use", "context7,gitnexus"], true),
            (&["--mcp", "--use", "context7"], true),
        ];

        for (switches, expected) in cases {
            let args = wrapper_args_from(switches);
            let intent = passthrough_launch_intent(
                Provider::Claude,
                Path::new("/usr/local/bin/claude"),
                true,
                &args,
                &[],
                None,
            );
            assert_eq!(
                intent.mcp_enabled, expected,
                "mcp_enabled for {switches:?} should be {expected}"
            );
        }
    }

    const KEY: &str = "OPENCODE_CONFIG_CONTENT";

    fn env_plan_with(current: Option<&str>) -> env::EnvPlan {
        let mut plan = env::EnvPlan::default();
        if let Some(raw) = current {
            plan.env
                .insert(std::ffi::OsString::from(KEY), std::ffi::OsString::from(raw));
        }
        plan
    }

    fn config_value(plan: &env::EnvPlan) -> Option<Value> {
        plan.env
            .get(std::ffi::OsStr::new(KEY))
            .and_then(|os| os.to_str())
            .map(|raw| serde_json::from_str(raw).expect("config is valid JSON"))
    }

    #[test]
    fn opencode_yolo_non_interactive_adds_three_allow_keys() {
        let mut plan = env_plan_with(None);
        apply_opencode_yolo_config_overlay(Provider::OpenCode, true, true, &mut plan).unwrap();

        let config = config_value(&plan).expect("permission overlay was written");
        let permission = config.get("permission").and_then(Value::as_object).unwrap();
        assert_eq!(permission["*"], "allow");
        assert_eq!(permission["external_directory"], "allow");
        assert_eq!(permission["doom_loop"], "allow");
    }

    #[test]
    fn opencode_yolo_interactive_adds_no_permission_key() {
        let mut plan = env_plan_with(None);
        // Interactive mirrors `YoloOutcome::not_applied`: the gate is closed.
        apply_opencode_yolo_config_overlay(Provider::OpenCode, true, false, &mut plan).unwrap();
        assert!(config_value(&plan).is_none());
    }

    #[test]
    fn opencode_non_yolo_non_interactive_leaves_env_untouched() {
        let existing = r#"{"instructions":["/tmp/sp.md"]}"#;
        let mut plan = env_plan_with(Some(existing));
        apply_opencode_yolo_config_overlay(Provider::OpenCode, false, true, &mut plan).unwrap();

        let config = config_value(&plan).unwrap();
        assert!(config.get("permission").is_none());
        assert_eq!(config["instructions"], serde_json::json!(["/tmp/sp.md"]));
    }

    #[test]
    fn non_opencode_provider_is_a_no_op() {
        let mut plan = env_plan_with(None);
        apply_opencode_yolo_config_overlay(Provider::Claude, true, true, &mut plan).unwrap();
        assert!(config_value(&plan).is_none());
    }

    #[test]
    fn yolo_merge_preserves_instructions_and_mcp() {
        // Simulate Phase 2 output: both system-prompt and MCP writers ran.
        let existing = r#"{"instructions":["/tmp/sp.md"],"mcp":{"srv":{}}}"#;
        let mut plan = env_plan_with(Some(existing));
        apply_opencode_yolo_config_overlay(Provider::OpenCode, true, true, &mut plan).unwrap();

        let config = config_value(&plan).unwrap();
        assert_eq!(config["instructions"], serde_json::json!(["/tmp/sp.md"]));
        assert_eq!(config["mcp"], serde_json::json!({ "srv": {} }));
        assert_eq!(config["permission"]["*"], "allow");
        assert_eq!(config["permission"]["external_directory"], "allow");
        assert_eq!(config["permission"]["doom_loop"], "allow");
    }

    /// Spawn-spec: a non-interactive YOLO OpenCode child carries
    /// `--dangerously-skip-permissions` on argv (from the profile) **and** the
    /// permission block in `OPENCODE_CONFIG_CONTENT` (from Stage 10.5).
    #[test]
    fn opencode_yolo_spawn_spec_has_argv_flag_and_config_permission() {
        let opencode = profile::profile_for_provider(Provider::OpenCode).unwrap();
        let mut argv = vec!["run".to_string()];
        let mut env_overrides: Vec<(String, String)> = Vec::new();
        let outcome = opencode
            .apply_yolo_for_mode(&mut argv, &mut env_overrides, false)
            .unwrap();
        assert!(outcome.applied);
        assert!(argv.iter().any(|a| a == "--dangerously-skip-permissions"));

        let mut plan = env_plan_with(None);
        apply_opencode_yolo_config_overlay(Provider::OpenCode, outcome.applied, true, &mut plan)
            .unwrap();
        let config = config_value(&plan).unwrap();
        assert_eq!(config["permission"]["external_directory"], "allow");
        assert_eq!(config["permission"]["doom_loop"], "allow");
    }

    /// Dry-run observable: `--dry-run` renders the env from `env_plan.added`
    /// (not the child `env`), so the merged permission block must appear there
    /// too — otherwise a dry-run would hide the YOLO config it is meant to show.
    #[test]
    fn opencode_yolo_overlay_is_visible_in_added_env() {
        // Simulate the system-prompt fold having already recorded an entry in
        // `added`; the overlay must replace it, not duplicate it.
        let existing = r#"{"instructions":["/tmp/sp.md"]}"#;
        let mut plan = env_plan_with(Some(existing));
        plan.added.push((KEY.to_string(), existing.to_string()));

        apply_opencode_yolo_config_overlay(Provider::OpenCode, true, true, &mut plan).unwrap();

        let entries: Vec<&(String, String)> =
            plan.added.iter().filter(|(k, _)| k == KEY).collect();
        assert_eq!(entries.len(), 1, "no duplicate OPENCODE_CONFIG_CONTENT entry");
        let shown: Value = serde_json::from_str(&entries[0].1).unwrap();
        assert_eq!(shown["instructions"], serde_json::json!(["/tmp/sp.md"]));
        assert_eq!(shown["permission"]["*"], "allow");
        assert_eq!(shown["permission"]["external_directory"], "allow");
        assert_eq!(shown["permission"]["doom_loop"], "allow");
    }
