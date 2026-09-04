use std::collections::HashMap;

use chrono::Utc;

use super::*;
use crate::config::claudine_config::{ClaudineConfig, TtsValue};
use crate::events::{EnvironmentContext, GitContext, HardwareContext, OsContext, RepoContext};
use crate::provider::Provider;
use std::path::PathBuf;

fn meta() -> EventMeta {
    EventMeta {
        provider: Provider::Claude,
        event: crate::events::AgenticEvent::BeforeTool,
        timestamp: Utc::now(),
        session_id: Some("test-session".to_string()),
        cwd: Some("/tmp".to_string()),
        tool_name: Some("Bash".to_string()),
        tool_input: None,
        tool_response: None,
        error: None,
        prompt: None,
        agent_type: None,
        notification_type: None,
        notification_message: None,
        agent_pid: None,
        extra: HashMap::new(),
        env: EnvironmentContext::default(),
    }
}

fn claudine_config_with_tts(tts: TtsValue) -> ClaudineConfig {
    ClaudineConfig {
        tts,
        messenger: None,
        logging: true,
        protect: Default::default(),
        actions: HashMap::new(),
        matchers: HashMap::new(),
        preferred_agent: Some(Provider::Claude),
        canonical_provider: None,
        models: HashMap::new(),
        default_sounds: Default::default(),
        prompt_for_missing: true,
        harvest_unmatched: false,
        exit_expressions: None,
        guard_settings: Default::default(),
    }
}

fn make_meta_for_when_tests() -> EventMeta {
    let mut m = meta();
    m.tool_name = Some("Bash".to_string());
    m
}

fn meta_with_full_env() -> EventMeta {
    let mut m = make_meta_for_when_tests();
    m.env.os = OsContext {
        os_type: "macos".to_string(),
        name: "macOS".to_string(),
        version: "15.3".to_string(),
        kernel: "Darwin 25.3.0".to_string(),
        hostname: "test-host".to_string(),
        linux_family: None,
        package_managers: vec!["brew".to_string()],
    };
    m.env.hardware = HardwareContext {
        arch: "aarch64".to_string(),
        cpu: "Apple M4 Max".to_string(),
        cores: 16,
        memory_bytes: 68_719_476_736,
        memory_available_bytes: 34_359_738_368,
    };
    m.env.git = Some(GitContext {
        repo_root: PathBuf::from("/tmp/project"),
        branch: Some("main".to_string()),
        is_dirty: true,
        staged_count: 0,
        unstaged_count: 0,
        untracked_count: 0,
        head_sha: Some("abc123def".to_string()),
        head_message: Some("feat: add feature".to_string()),
        user_name: None,
        user_email: None,
        remote_name: Some("origin".to_string()),
        remote_url: None,
        hosting_provider: Some("github".to_string()),
        repo_name: Some("rusty-biscuit".to_string()),
        repo_org: Some("anthropics".to_string()),
    });
    m.env.repo = Some(RepoContext {
        is_monorepo: true,
        monorepo_standard: Some("cargo-workspace".to_string()),
        monorepo_orchestrators: vec!["nx".to_string()],
        monorepo_tool: Some("cargo-workspace".to_string()),
        root: PathBuf::from("/tmp/project"),
        packages: vec!["lib".to_string(), "cli".to_string()],
    });
    m.env.primary_language = Some("Rust".to_string());
    m
}

#[tokio::test]
async fn message_action_skipped_when_no_route() {
    let actions = vec![HookAction::Message {
        message: "test notification".to_string(),
        image: None,
        when: None,
    }];

    let messaging = crate::messaging::RuntimeMessagingSettings::default();

    let result = execute_actions(
        &actions,
        None,
        &meta(),
        DispatchConfig::Canonical(&ClaudineConfig::default()),
        &messaging,
        false,
        None,
    )
    .await
    .unwrap();

    assert!(result.is_none());
}

#[tokio::test]
async fn message_action_does_not_block() {
    let actions = vec![
        HookAction::Message {
            message: "notify".to_string(),
            image: None,
            when: None,
        },
        HookAction::Report {
            handler: None,
            when: None,
        },
    ];

    let messaging = crate::messaging::RuntimeMessagingSettings::default();

    let result = execute_actions(
        &actions,
        None,
        &meta(),
        DispatchConfig::Canonical(&ClaudineConfig::default()),
        &messaging,
        true,
        None,
    )
    .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn execute_actions_runs_bash_action_with_canonical_config() {
    let config = claudine_config_with_tts(TtsValue::Boolean(false));
    let messaging = RuntimeMessagingSettings::default();

    let actions = vec![
        HookAction::Bash {
            command: "echo".to_string(),
            params: "hello".to_string(),
            when: None,
        },
        HookAction::Report {
            handler: None,
            when: None,
        },
    ];

    let result = execute_actions(
        &actions,
        None,
        &meta(),
        DispatchConfig::Canonical(&config),
        &messaging,
        true,
        None,
    )
    .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn execute_actions_message_action_with_canonical_config() {
    let config = claudine_config_with_tts(TtsValue::Boolean(false));
    let messaging = RuntimeMessagingSettings::default();

    let actions = vec![HookAction::Message {
        message: "test".to_string(),
        image: None,
        when: None,
    }];

    let result = execute_actions(
        &actions,
        None,
        &meta(),
        DispatchConfig::Canonical(&config),
        &messaging,
        false,
        None,
    )
    .await
    .unwrap();

    assert!(result.is_none());
}

#[cfg(unix)]
#[tokio::test]
#[serial_test::serial]
async fn audio_actions_publish_in_order_and_return_before_worker_execution() {
    use fs4::fs_std::FileExt as _;
    use std::fs::{self, OpenOptions};
    use std::os::unix::fs::PermissionsExt as _;
    use std::time::{Duration, Instant};

    let temp = tempfile::tempdir().unwrap();
    let spool = temp.path().join("spool");
    let bin = temp.path().join("bin");
    fs::create_dir(&bin).unwrap();
    let espeak = bin.join("espeak");
    fs::write(&espeak, "#!/bin/sh\n/bin/sleep 5\n").unwrap();
    let mut permissions = fs::metadata(&espeak).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&espeak, permissions).unwrap();
    let path = std::env::join_paths(
        std::iter::once(bin.clone()).chain(std::env::split_paths(
            &std::env::var_os("PATH").unwrap_or_default(),
        )),
    )
    .unwrap();
    let _path = test_toolkit::EnvGuard::set_safe("PATH", path);
    let _spool = test_toolkit::EnvGuard::set_safe("PLAYA_SPOOL_DIR", &spool);
    let _dry_run = test_toolkit::EnvGuard::remove_safe("PLAYA_DRY_RUN");
    assert_eq!(biscuit_speaks::run_if_worker().await, None);

    fs::create_dir(&spool).unwrap();
    fs::set_permissions(&spool, fs::Permissions::from_mode(0o700)).unwrap();
    let worker = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(spool.join("worker.lock"))
        .unwrap();
    worker.lock_exclusive().unwrap();

    let config = claudine_config_with_tts(TtsValue::Config(
        crate::config::tts::TtsConfigSettings {
            provider: "espeak".to_string(),
            voice: None,
            gender: crate::config::tts::Gender::Female,
        },
    ));
    let actions = vec![
        HookAction::Speak {
            message: "Phase 1 of the plan in the claudine package area, was implemented successfully".to_string(),
            voice: None,
            gender: None,
            when: None,
        },
        HookAction::SoundEffect {
            effect: "doorbell-2".to_string(),
            volume: 0.5,
            speed: 1.25,
            when: None,
        },
    ];
    let start = Instant::now();
    let result = execute_actions(
        &actions,
        None,
        &meta(),
        DispatchConfig::Canonical(&config),
        &RuntimeMessagingSettings::default(),
        false,
        None,
    )
    .await
    .unwrap();

    assert!(result.is_none());
    assert!(
        start.elapsed() < Duration::from_secs(4),
        "dispatch must return after durable publication"
    );
    let snapshot = playa::detached::snapshot().unwrap();
    assert_eq!(
        snapshot
            .pending
            .iter()
            .map(|job| (job.sequence, job.source_kind))
            .collect::<Vec<_>>(),
        vec![
            (1, playa::detached::JournalSourceKind::Command),
            (2, playa::detached::JournalSourceKind::File),
        ]
    );
}

#[tokio::test]
#[serial_test::serial]
#[tracing_test::traced_test]
async fn sound_effect_action_warns_once_when_handoff_fails() {
    let temp = tempfile::tempdir().unwrap();
    let not_a_directory = temp.path().join("not-a-directory");
    std::fs::write(&not_a_directory, b"file").unwrap();
    let _spool = test_toolkit::EnvGuard::set_safe("PLAYA_SPOOL_DIR", &not_a_directory);
    let _dry_run = test_toolkit::EnvGuard::remove_safe("PLAYA_DRY_RUN");
    assert_eq!(biscuit_speaks::run_if_worker().await, None);

    execute_sound_effect("doorbell-2", 1.0, 1.0);

    logs_assert(|logs| {
        let warnings = logs
            .iter()
            .filter(|line| line.contains("Sound effect handoff failed"))
            .count();
        assert_eq!(warnings, 1, "expected one handoff warning, got: {logs:?}");
        Ok(())
    });
}

#[cfg(unix)]
#[tokio::test]
#[serial_test::serial]
#[tracing_test::traced_test]
async fn speak_action_warns_once_when_handoff_fails() {
    use std::fs;
    use std::os::unix::fs::PermissionsExt as _;

    let temp = tempfile::tempdir().unwrap();
    let bin = temp.path().join("bin");
    fs::create_dir(&bin).unwrap();
    let espeak = bin.join("espeak");
    fs::write(&espeak, "#!/bin/sh\nexit 0\n").unwrap();
    let mut permissions = fs::metadata(&espeak).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&espeak, permissions).unwrap();
    let path = std::env::join_paths(
        std::iter::once(bin).chain(std::env::split_paths(
            &std::env::var_os("PATH").unwrap_or_default(),
        )),
    )
    .unwrap();
    let not_a_directory = temp.path().join("not-a-directory");
    fs::write(&not_a_directory, b"file").unwrap();
    let _path = test_toolkit::EnvGuard::set_safe("PATH", path);
    let _spool = test_toolkit::EnvGuard::set_safe("PLAYA_SPOOL_DIR", &not_a_directory);
    let _dry_run = test_toolkit::EnvGuard::remove_safe("PLAYA_DRY_RUN");
    assert_eq!(biscuit_speaks::run_if_worker().await, None);

    let config = claudine_config_with_tts(TtsValue::Config(
        crate::config::tts::TtsConfigSettings {
            provider: "espeak".to_string(),
            voice: None,
            gender: crate::config::tts::Gender::Female,
        },
    ));
    let actions = vec![HookAction::Speak {
        message: "Phase 1 of the plan in the claudine package area, was implemented successfully".to_string(),
        voice: None,
        gender: None,
        when: None,
    }];
    execute_actions(
        &actions,
        None,
        &meta(),
        DispatchConfig::Canonical(&config),
        &RuntimeMessagingSettings::default(),
        false,
        None,
    )
    .await
    .unwrap();

    logs_assert(|logs| {
        let warnings = logs
            .iter()
            .filter(|line| line.contains("TTS handoff failed"))
            .count();
        assert_eq!(warnings, 1, "expected one handoff warning, got: {logs:?}");
        Ok(())
    });
}

#[tokio::test]
async fn blocking_call_command_failure_fails_closed() {
    let config = claudine_config_with_tts(TtsValue::Boolean(false));
    let messaging = RuntimeMessagingSettings::default();
    let actions = vec![HookAction::Call {
        command: "__claudine_missing_command__".to_string(),
        args: None,
        timeout_ms: Some(25),
        mapper: None,
        when: None,
    }];

    let result = execute_actions(
        &actions,
        None,
        &meta(),
        DispatchConfig::Canonical(&config),
        &messaging,
        true,
        None,
    )
    .await
    .unwrap()
    .expect("blocking call failure must synthesize a deny response");

    assert_eq!(result.decision, Some(HookDecision::Deny));
    let reason = result.reason.expect("deny response should explain failure");
    assert!(reason.contains("call action command failed"));
    assert!(reason.contains("__claudine_missing_command__"));
}

#[cfg(unix)]
#[tokio::test]
async fn blocking_call_without_actionable_response_falls_through() {
    let config = claudine_config_with_tts(TtsValue::Boolean(false));
    let messaging = RuntimeMessagingSettings::default();
    let actions = vec![HookAction::Call {
        command: "sh".to_string(),
        args: Some(vec!["-c".to_string(), "exit 1".to_string()]),
        timeout_ms: Some(250),
        mapper: None,
        when: None,
    }];

    let result = execute_actions(
        &actions,
        None,
        &meta(),
        DispatchConfig::Canonical(&config),
        &messaging,
        true,
        None,
    )
    .await
    .unwrap();

    assert!(
        result.is_none(),
        "exit-code mapper status 1 should fall through instead of producing an implicit allow"
    );
}

// =========================================================================
// `when` condition tests (Phase 3 of leverage-dm-parser)
// =========================================================================

#[tokio::test]
async fn when_condition_true_executes_action_and_can_block() {
    let config = claudine_config_with_tts(TtsValue::Boolean(false));
    let messaging = RuntimeMessagingSettings::default();
    let actions = vec![HookAction::Call {
        command: "__claudine_missing_when_true__".to_string(),
        args: None,
        timeout_ms: Some(50),
        mapper: None,
        when: Some("tool_name == 'Bash'".to_string()),
    }];

    let result = execute_actions(
        &actions,
        None,
        &make_meta_for_when_tests(),
        DispatchConfig::Canonical(&config),
        &messaging,
        true,
        None,
    )
    .await
    .unwrap()
    .expect("call action with truthy when should still run and synthesize a deny");

    assert_eq!(result.decision, Some(HookDecision::Deny));
}

#[tokio::test]
async fn when_condition_false_skips_call_action_and_no_blocking_response() {
    let config = claudine_config_with_tts(TtsValue::Boolean(false));
    let messaging = RuntimeMessagingSettings::default();
    let actions = vec![HookAction::Call {
        command: "__claudine_missing_when_false__".to_string(),
        args: None,
        timeout_ms: Some(50),
        mapper: None,
        when: Some("tool_name == 'Read'".to_string()),
    }];

    let result = execute_actions(
        &actions,
        None,
        &make_meta_for_when_tests(),
        DispatchConfig::Canonical(&config),
        &messaging,
        true,
        None,
    )
    .await
    .unwrap();

    assert!(
        result.is_none(),
        "skipped Call action must not produce a blocking response",
    );
}

#[tokio::test]
async fn when_invalid_expression_skips_action_non_fatally() {
    let config = claudine_config_with_tts(TtsValue::Boolean(false));
    let messaging = RuntimeMessagingSettings::default();
    let actions = vec![HookAction::Call {
        command: "__claudine_missing_when_invalid__".to_string(),
        args: None,
        timeout_ms: Some(50),
        mapper: None,
        when: Some("&& this is not a valid condition".to_string()),
    }];

    let result = execute_actions(
        &actions,
        None,
        &make_meta_for_when_tests(),
        DispatchConfig::Canonical(&config),
        &messaging,
        true,
        None,
    )
    .await;

    let outcome = result.expect("invalid `when` expression must not error the runner");
    assert!(
        outcome.is_none(),
        "invalid `when` should skip the action without producing a blocking response",
    );
}

#[tokio::test]
async fn when_skipped_call_does_not_replace_prior_selected_response() {
    // First action is a failing Call producing a deny; second action
    // is a Call that would normally produce Continue but is skipped
    // by a falsy `when` and therefore must not overwrite the deny.
    let config = claudine_config_with_tts(TtsValue::Boolean(false));
    let messaging = RuntimeMessagingSettings::default();
    let actions = vec![
        HookAction::Call {
            command: "__claudine_when_first__".to_string(),
            args: None,
            timeout_ms: Some(50),
            mapper: None,
            when: None,
        },
        HookAction::Call {
            command: "echo".to_string(),
            args: Some(vec!["allow".to_string()]),
            timeout_ms: Some(500),
            mapper: None,
            when: Some("tool_name == 'Read'".to_string()),
        },
    ];

    let result = execute_actions(
        &actions,
        None,
        &make_meta_for_when_tests(),
        DispatchConfig::Canonical(&config),
        &messaging,
        true,
        None,
    )
    .await
    .unwrap()
    .expect("first call should produce a blocking deny");

    assert_eq!(result.decision, Some(HookDecision::Deny));
}

#[tokio::test]
async fn when_env_var_resolves_via_env_namespace() {
    let config = claudine_config_with_tts(TtsValue::Boolean(false));
    let messaging = RuntimeMessagingSettings::default();

    let key = "CLAUDINE_DISPATCH_WHEN_ENV_VAR_PRESENT";
    // SAFETY: tests that touch process env should run serially within a
    // module; the var is unique per test scope and removed below.
    unsafe {
        std::env::set_var(key, "yes");
    }

    let actions = vec![HookAction::Call {
        command: "__claudine_missing_when_env__".to_string(),
        args: None,
        timeout_ms: Some(50),
        mapper: None,
        when: Some(format!("env.{key} == 'yes'")),
    }];

    let result = execute_actions(
        &actions,
        None,
        &make_meta_for_when_tests(),
        DispatchConfig::Canonical(&config),
        &messaging,
        true,
        None,
    )
    .await
    .unwrap();

    unsafe {
        std::env::remove_var(key);
    }

    let response = result.expect("env-backed condition should be truthy and let the call run");
    assert_eq!(response.decision, Some(HookDecision::Deny));
}

#[tokio::test]
async fn when_missing_env_var_is_falsy_and_skips_action() {
    let config = claudine_config_with_tts(TtsValue::Boolean(false));
    let messaging = RuntimeMessagingSettings::default();

    unsafe {
        std::env::remove_var("CLAUDINE_DISPATCH_WHEN_ENV_VAR_ABSENT");
    }

    let actions = vec![HookAction::Call {
        command: "__claudine_missing_when_env_absent__".to_string(),
        args: None,
        timeout_ms: Some(50),
        mapper: None,
        when: Some("env.CLAUDINE_DISPATCH_WHEN_ENV_VAR_ABSENT".to_string()),
    }];

    let result = execute_actions(
        &actions,
        None,
        &make_meta_for_when_tests(),
        DispatchConfig::Canonical(&config),
        &messaging,
        true,
        None,
    )
    .await
    .unwrap();

    assert!(
        result.is_none(),
        "missing env var should evaluate falsy and skip the call action",
    );
}

#[tokio::test]
async fn when_ctx_fields_do_not_require_precomputed_event_metadata() {
    // `ctx.*` paths are resolved lazily by Darkmatter's shortcut
    // lookup. The condition should evaluate without panicking even
    // though Claudine's EventMeta does not precompute these fields.
    let config = claudine_config_with_tts(TtsValue::Boolean(false));
    let messaging = RuntimeMessagingSettings::default();
    let actions = vec![HookAction::Call {
        command: "__claudine_missing_when_ctx_weak__".to_string(),
        args: None,
        timeout_ms: Some(50),
        mapper: None,
        when: Some("ctx.today != ''".to_string()),
    }];

    let result = execute_actions(
        &actions,
        None,
        &make_meta_for_when_tests(),
        DispatchConfig::Canonical(&config),
        &messaging,
        true,
        None,
    )
    .await;

    let response = result
        .expect("ctx.* condition should evaluate without erroring the runner")
        .expect("ctx.today != '' should be truthy and let the call fire");
    assert_eq!(response.decision, Some(HookDecision::Deny));
}

#[tokio::test]
async fn when_ctx_today_resolves() {
    // Regression test: ctx.today should resolve truthy through the
    // EventMetaConditionLookup composite and allow the action to run.
    let config = claudine_config_with_tts(TtsValue::Boolean(false));
    let messaging = RuntimeMessagingSettings::default();
    let actions = vec![HookAction::Call {
        command: "__claudine_missing_when_ctx_today__".to_string(),
        args: None,
        timeout_ms: Some(50),
        mapper: None,
        when: Some("ctx.today != ''".to_string()),
    }];

    let result = execute_actions(
        &actions,
        None,
        &make_meta_for_when_tests(),
        DispatchConfig::Canonical(&config),
        &messaging,
        true,
        None,
    )
    .await
    .unwrap()
    .expect("ctx.today != '' should be truthy and let the call fire");

    assert_eq!(result.decision, Some(HookDecision::Deny));
}

// =========================================================================
// `when` evaluation tests via EventMetaConditionLookup
// =========================================================================

#[tokio::test]
async fn when_git_branch_matches_main_resolves_truthy() {
    let config = claudine_config_with_tts(TtsValue::Boolean(false));
    let messaging = RuntimeMessagingSettings::default();
    let actions = vec![HookAction::Call {
        command: "__claudine_missing_when_git_branch__".to_string(),
        args: None,
        timeout_ms: Some(50),
        mapper: None,
        when: Some("git.branch == 'main'".to_string()),
    }];

    let result = execute_actions(
        &actions,
        None,
        &meta_with_full_env(),
        DispatchConfig::Canonical(&config),
        &messaging,
        true,
        None,
    )
    .await
    .unwrap()
    .expect("git.branch == 'main' should resolve truthy and let the call fire");

    assert_eq!(result.decision, Some(HookDecision::Deny));
}

#[tokio::test]
async fn when_git_is_dirty_resolves_as_boolean() {
    let config = claudine_config_with_tts(TtsValue::Boolean(false));
    let messaging = RuntimeMessagingSettings::default();
    let mut meta = meta_with_full_env();
    if let Some(git) = meta.env.git.as_mut() {
        git.is_dirty = false;
    }

    let actions = vec![HookAction::Call {
        command: "__claudine_missing_when_not_dirty__".to_string(),
        args: None,
        timeout_ms: Some(50),
        mapper: None,
        when: Some("!git.is_dirty".to_string()),
    }];

    let result = execute_actions(
        &actions,
        None,
        &meta,
        DispatchConfig::Canonical(&config),
        &messaging,
        true,
        None,
    )
    .await
    .unwrap()
    .expect("!git.is_dirty should resolve truthy when is_dirty is false");

    assert_eq!(result.decision, Some(HookDecision::Deny));
}

#[tokio::test]
async fn when_hardware_cores_numeric_comparison() {
    // Pins that `hardware.cores` is exposed as a JSON Number, not a
    // string — otherwise `> 8` would fail to evaluate.
    let config = claudine_config_with_tts(TtsValue::Boolean(false));
    let messaging = RuntimeMessagingSettings::default();
    let actions = vec![HookAction::Call {
        command: "__claudine_missing_when_cores__".to_string(),
        args: None,
        timeout_ms: Some(50),
        mapper: None,
        when: Some("hardware.cores > 8".to_string()),
    }];

    let result = execute_actions(
        &actions,
        None,
        &meta_with_full_env(),
        DispatchConfig::Canonical(&config),
        &messaging,
        true,
        None,
    )
    .await
    .unwrap()
    .expect("hardware.cores > 8 should resolve truthy with cores=16");

    assert_eq!(result.decision, Some(HookDecision::Deny));
}

#[tokio::test]
async fn when_project_language_matches() {
    let config = claudine_config_with_tts(TtsValue::Boolean(false));
    let messaging = RuntimeMessagingSettings::default();
    let actions = vec![HookAction::Call {
        command: "__claudine_missing_when_project_language__".to_string(),
        args: None,
        timeout_ms: Some(50),
        mapper: None,
        when: Some("project.language == 'Rust'".to_string()),
    }];

    let result = execute_actions(
        &actions,
        None,
        &meta_with_full_env(),
        DispatchConfig::Canonical(&config),
        &messaging,
        true,
        None,
    )
    .await
    .unwrap()
    .expect("project.language == 'Rust' should resolve truthy");

    assert_eq!(result.decision, Some(HookDecision::Deny));
}

#[tokio::test]
async fn when_nested_tool_input_path() {
    // `tool_input` is a top-level field on EventMeta and resolves
    // directly through EventMetaExpressionLookup. This test guards
    // against regression in the expression evaluation path.
    let config = claudine_config_with_tts(TtsValue::Boolean(false));
    let messaging = RuntimeMessagingSettings::default();
    let mut meta = make_meta_for_when_tests();
    meta.tool_input = Some(serde_json::json!({"command": "npm test"}));

    let actions = vec![HookAction::Call {
        command: "__claudine_missing_when_tool_input__".to_string(),
        args: None,
        timeout_ms: Some(50),
        mapper: None,
        when: Some("tool_input.command == 'npm test'".to_string()),
    }];

    let result = execute_actions(
        &actions,
        None,
        &meta,
        DispatchConfig::Canonical(&config),
        &messaging,
        true,
        None,
    )
    .await
    .unwrap()
    .expect("nested tool_input.command should resolve truthy");

    assert_eq!(result.decision, Some(HookDecision::Deny));
}

#[tokio::test]
async fn when_extra_dot_path_resolves() {
    let config = claudine_config_with_tts(TtsValue::Boolean(false));
    let messaging = RuntimeMessagingSettings::default();
    let mut meta = make_meta_for_when_tests();
    meta.extra
        .insert("attempt".to_string(), serde_json::json!(3));

    let actions = vec![HookAction::Call {
        command: "__claudine_missing_when_extra_attempt__".to_string(),
        args: None,
        timeout_ms: Some(50),
        mapper: None,
        when: Some("extra.attempt > 1".to_string()),
    }];

    let result = execute_actions(
        &actions,
        None,
        &meta,
        DispatchConfig::Canonical(&config),
        &messaging,
        true,
        None,
    )
    .await
    .unwrap()
    .expect("extra.attempt > 1 should resolve truthy with attempt=3");

    assert_eq!(result.decision, Some(HookDecision::Deny));
}

#[tokio::test]
async fn when_tool_response_path_resolves() {
    let config = claudine_config_with_tts(TtsValue::Boolean(false));
    let messaging = RuntimeMessagingSettings::default();
    let mut meta = make_meta_for_when_tests();
    meta.tool_response = Some(serde_json::json!({"exit_code": 0}));

    let actions = vec![HookAction::Call {
        command: "__claudine_missing_when_tool_response__".to_string(),
        args: None,
        timeout_ms: Some(50),
        mapper: None,
        when: Some("tool_response.exit_code == 0".to_string()),
    }];

    let result = execute_actions(
        &actions,
        None,
        &meta,
        DispatchConfig::Canonical(&config),
        &messaging,
        true,
        None,
    )
    .await
    .unwrap()
    .expect("tool_response.exit_code == 0 should resolve truthy and let the call fire");

    assert_eq!(result.decision, Some(HookDecision::Deny));
}

#[tokio::test]
async fn when_env_fallback_syntax_works() {
    let config = claudine_config_with_tts(TtsValue::Boolean(false));
    let messaging = RuntimeMessagingSettings::default();

    let actions = vec![HookAction::Call {
        command: "__claudine_missing_when_fallback__".to_string(),
        args: None,
        timeout_ms: Some(50),
        mapper: None,
        when: Some("env.CLAUDINE_TEST_MISSING || 'default' == 'default'".to_string()),
    }];

    let result = execute_actions(
        &actions,
        None,
        &make_meta_for_when_tests(),
        DispatchConfig::Canonical(&config),
        &messaging,
        true,
        None,
    )
    .await
    .unwrap()
    .expect("env fallback syntax should evaluate truthy and let the call fire");

    assert_eq!(result.decision, Some(HookDecision::Deny));
}

#[tokio::test]
async fn when_ctx_year_resolves() {
    // Regression test: ctx.year should resolve truthy through the
    // EventMetaConditionLookup composite and allow the action to run.
    let config = claudine_config_with_tts(TtsValue::Boolean(false));
    let messaging = RuntimeMessagingSettings::default();
    let actions = vec![HookAction::Call {
        command: "__claudine_missing_when_ctx_year__".to_string(),
        args: None,
        timeout_ms: Some(50),
        mapper: None,
        when: Some("ctx.year != ''".to_string()),
    }];

    let result = execute_actions(
        &actions,
        None,
        &make_meta_for_when_tests(),
        DispatchConfig::Canonical(&config),
        &messaging,
        true,
        None,
    )
    .await
    .unwrap()
    .expect("ctx.year != '' should be truthy and let the call fire");

    assert_eq!(result.decision, Some(HookDecision::Deny));
}

#[tokio::test]
async fn when_missing_git_block_is_falsy() {
    // When `meta.env.git` is None, `git.branch` resolves to Null
    // through EventMetaExpressionLookup and the condition is falsy.
    let config = claudine_config_with_tts(TtsValue::Boolean(false));
    let messaging = RuntimeMessagingSettings::default();
    let mut meta = make_meta_for_when_tests();
    meta.env.git = None;

    let actions = vec![HookAction::Call {
        command: "__claudine_missing_when_no_git__".to_string(),
        args: None,
        timeout_ms: Some(50),
        mapper: None,
        when: Some("git.branch == 'main'".to_string()),
    }];

    let result = execute_actions(
        &actions,
        None,
        &meta,
        DispatchConfig::Canonical(&config),
        &messaging,
        true,
        None,
    )
    .await
    .unwrap();

    assert!(
        result.is_none(),
        "missing git block must be falsy and skip the call action",
    );
}
