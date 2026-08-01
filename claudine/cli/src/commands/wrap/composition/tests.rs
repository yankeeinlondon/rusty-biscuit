//! Composition orchestration unit tests: launch-workspace resolution, repo
//! guard, model catalog refresh gating, selection config loading, TTY
//! resolution state classification, picker scoping, and interactive-timeout
//! conflict detection.

use super::*;

/// W0 regression: when the executor is given a precomputed
/// `prep_launch_workspace`, [`select_launch_workspace`] must
/// short-circuit before reaching the legacy
/// `env::resolve_launch_workspace_context` walk. The fallback
/// counter is process-global so we reset it under `serial_test` to
/// observe a clean baseline.
#[test]
#[serial_test::serial]
fn select_launch_workspace_uses_prep_without_calling_fallback() {
    use std::path::PathBuf;
    reset_launch_workspace_fallbacks_for_tests();

    let prep = env::LaunchWorkspaceContext {
        launch_cwd: PathBuf::from("/tmp/launch"),
        repo_root: Some(PathBuf::from("/tmp/launch")),
        child_cwd: PathBuf::from("/tmp/launch"),
        package_context: None,
        warnings: Vec::new(),
    };

    let result = select_launch_workspace(Some(&prep), Path::new("/tmp/launch"), None);
    assert_eq!(result.launch_cwd, prep.launch_cwd);
    assert_eq!(
        launch_workspace_fallback_count_for_tests(),
        0,
        "providing prep_launch_workspace must not call the fallback walker"
    );
}

/// W0 contract: the fallback path is still reachable for legacy
/// callers that don't thread a `CompositionPrepContext` (e.g. a
/// hand-built library invocation). Calling without `prep` must
/// increment the counter exactly once so a future refactor that
/// adds a hidden second walk would fail this assertion.
#[test]
#[serial_test::serial]
fn select_launch_workspace_falls_back_once_when_prep_missing() {
    reset_launch_workspace_fallbacks_for_tests();
    // Point the fallback walker at an empty tempdir rather than the real
    // current dir: the counter increments before the walk, so the
    // contract holds, while avoiding an expensive repo scan of the whole
    // monorepo worktree.
    let cwd = tempfile::tempdir().unwrap();

    let _ = select_launch_workspace(None, cwd.path(), None);

    assert_eq!(
        launch_workspace_fallback_count_for_tests(),
        1,
        "missing prep_launch_workspace must call the fallback walker exactly once"
    );
}

#[test]
fn enforce_repo_launch_detection_passes_when_no_error() {
    // Successful prep-time sniff scan: the executor should proceed
    // regardless of whether `--repo` is set.
    assert!(enforce_repo_launch_detection(false, None).is_ok());
    assert!(enforce_repo_launch_detection(true, None).is_ok());
}

/// The captured prep-time failure, projected exactly as
/// `CompositionPrepContext` projects it.
fn captured_launch_detection_failure() -> claudine::diagnostics::DiagnosticSnapshot {
    claudine::diagnostics::DiagnosticSnapshot::from_diagnostic(
        &claudine::error::ClaudineError::LaunchContextDetection(Box::new(
            sniff::SniffError::NotARepository(std::path::PathBuf::from("/no/such/repo")),
        )),
    )
}

#[test]
fn enforce_repo_launch_detection_passes_when_repo_off() {
    // Sniff failed during prep but `--repo` is not set, so the
    // best-effort default is acceptable and the executor proceeds.
    assert!(enforce_repo_launch_detection(false, Some(&captured_launch_detection_failure())).is_ok());
}

#[test]
fn enforce_repo_launch_detection_fails_when_repo_and_sniff_failed() {
    // The legacy contract: `--repo` requires startup repo detection,
    // so a captured prep-time sniff failure must abort the run with
    // a hard error that surfaces the original sniff message.
    let captured = captured_launch_detection_failure();
    let result = enforce_repo_launch_detection(true, Some(&captured));
    let err = result.expect_err("--repo + prep sniff failure must error");
    let message = err.to_string();
    assert!(
        message.contains("--repo requires startup repo detection"),
        "expected --repo guard message, got: {message}"
    );
    assert!(
        message.contains("/no/such/repo"),
        "expected captured sniff error in message, got: {message}"
    );
}

/// The regression this guard-plus-restoration exists for: the abort used to be
/// an `eyre!` built from `snapshot.message`, so a `when:` clause and the
/// rendered block saw a bare string. Asserting only on message substrings
/// cannot tell the two apart — these assertions can.
#[test]
fn the_repo_abort_surfaces_the_captured_diagnostic_identity() {
    let captured = captured_launch_detection_failure();
    let err = enforce_repo_launch_detection(true, Some(&captured))
        .expect_err("--repo + prep sniff failure must error");

    let root: &(dyn std::error::Error + 'static) = err.as_ref();
    let selected = claudine::diagnostics::select_effective_diagnostic(root)
        .expect("the abort must be a selectable diagnostic, not an opaque report");
    let diagnostic = selected
        .diagnostic()
        .expect("the selection must carry Claudine facets");

    assert_eq!(diagnostic.code(), captured.code);
    assert_eq!(diagnostic.code(), "internal.bug");
    assert_eq!(diagnostic.category().as_str(), captured.category);
    assert_eq!(diagnostic.disposition().as_str(), captured.disposition);
    assert_eq!(diagnostic.origin().as_str(), captured.origin);
    assert_eq!(diagnostic.detail(), captured.detail);
    assert_eq!(
        diagnostic.detail()["message"],
        serde_json::json!("Not a git repository: /no/such/repo"),
        "the catalog-declared detail field must survive the restoration"
    );

    // Rendering resolves through the same selection, so the block carries the
    // captured message rather than a generic `Error:` line.
    let rendered = selected.block_error().report_block_error_optimistic(Some(120));
    assert!(
        rendered.contains("/no/such/repo"),
        "expected the captured sniff text in the rendered block, got: {rendered}"
    );
}

#[test]
fn load_selection_config_returns_both_favorite_and_overrides() {
    use claudine::config::claudine_config::{
        DetailedModelOverride, ModelOverrideMode, ProviderModelOverride,
    };
    use std::collections::HashMap;

    let dir = tempfile::tempdir().unwrap();
    let home = dir.path();
    let claudine_dir = home.join(".claudine");
    std::fs::create_dir_all(&claudine_dir).unwrap();

    use claudine::config::claudine_config::ClaudineConfig;

    let config = ClaudineConfig {
        preferred_agent: Some(Provider::Codex),
        models: {
            let mut m = HashMap::new();
            m.insert(
                Provider::Codex,
                ProviderModelOverride::Detailed(DetailedModelOverride {
                    mode: ModelOverrideMode::Add,
                    values: vec!["gpt-5".into()],
                }),
            );
            m
        },
        ..ClaudineConfig::default()
    };
    let config_path = claudine_dir.join("config.json");
    claudine::dispatch::loader::save_claudine_config(&config, &config_path).unwrap();

    let result = selection::load_selection_config_from_paths(Some(&config_path), None);

    let cfg = result.expect("should load config");
    assert_eq!(cfg.favorite, Some(Provider::Codex));
    assert!(cfg.model_overrides.contains_key(&Provider::Codex));
    assert_eq!(cfg.model_overrides[&Provider::Codex].values(), &["gpt-5"]);
}

#[test]
fn load_selection_config_handles_missing_config() {
    let dir = tempfile::tempdir().unwrap();
    let nonexistent = dir.path().join("no-such-config.json");
    let result =
        claudine::dispatch::loader::load_claudine_config(Some(&nonexistent), None);
    assert!(
        result.is_err(),
        "expected error for missing config file"
    );
}

#[test]
fn catalog_initialized_with_config_overrides() {
    use claudine::config::claudine_config::{
        DetailedModelOverride, ModelOverrideMode, ProviderModelOverride,
    };
    use std::collections::HashMap;

    let mut overrides = HashMap::new();
    overrides.insert(
        Provider::Codex,
        ProviderModelOverride::Detailed(DetailedModelOverride {
            mode: ModelOverrideMode::Add,
            values: vec!["gpt-5".into()],
        }),
    );

    let config = SelectionConfig {
        favorite: Some(Provider::Codex),
        model_overrides: overrides,
    };

    let catalog =
        claudine::model_catalog::ModelCatalogService::with_overrides(config.model_overrides);

    // Baseline (expected-offering) model should still be valid (additive mode)
    assert!(catalog.is_valid(Provider::Codex, "gpt-5.5"));
    // Override model should also be valid
    assert!(catalog.is_valid(Provider::Codex, "gpt-5"));
    // Non-overridden provider should use its baseline
    assert!(catalog.is_valid(Provider::Claude, "claude-opus-4-8"));
}

/// RAII guard that restores the prior value of an env var on drop. Used
/// to keep the resolve_timeouts tests hermetic.
struct EnvGuard {
    key: &'static str,
    prior: Option<String>,
}

impl EnvGuard {
    fn set(key: &'static str, value: &str) -> Self {
        let prior = std::env::var(key).ok();
        unsafe {
            std::env::set_var(key, value);
        }
        Self { key, prior }
    }

    fn clear(key: &'static str) -> Self {
        let prior = std::env::var(key).ok();
        unsafe {
            std::env::remove_var(key);
        }
        Self { key, prior }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        unsafe {
            match &self.prior {
                Some(v) => std::env::set_var(self.key, v),
                None => std::env::remove_var(self.key),
            }
        }
    }
}

#[test]
#[serial_test::serial]
fn resolve_timeouts_cli_wins_over_frontmatter_env_and_default() {
    let _g1 = EnvGuard::set("CLAUDINE_TIMEOUT", "1h");
    let _g2 = EnvGuard::set("CLAUDINE_STEP_TIMEOUT", "5m");

    let cfg = resolve_timeouts(
        Some("60s".into()),
        Some(std::time::Duration::from_secs(120)),
        Some("45s".into()),
        Some(std::time::Duration::from_secs(90)),
    );
    assert_eq!(cfg.timeout, Some(std::time::Duration::from_secs(60)));
    assert_eq!(cfg.step_timeout, Some(std::time::Duration::from_secs(45)));
}

#[test]
#[serial_test::serial]
fn resolve_timeouts_frontmatter_wins_over_env_and_default() {
    let _g1 = EnvGuard::set("CLAUDINE_TIMEOUT", "1h");
    let _g2 = EnvGuard::set("CLAUDINE_STEP_TIMEOUT", "5m");

    let cfg = resolve_timeouts(
        None,
        Some(std::time::Duration::from_secs(7200)),
        None,
        Some(std::time::Duration::from_secs(900)),
    );
    assert_eq!(cfg.timeout, Some(std::time::Duration::from_secs(7200)));
    assert_eq!(cfg.step_timeout, Some(std::time::Duration::from_secs(900)));
}

#[test]
#[serial_test::serial]
fn resolve_timeouts_env_wins_over_built_in_default() {
    let _g1 = EnvGuard::set("CLAUDINE_TIMEOUT", "1h");
    let _g2 = EnvGuard::set("CLAUDINE_STEP_TIMEOUT", "10m");

    let cfg = resolve_timeouts(None, None, None, None);
    assert_eq!(cfg.timeout, Some(std::time::Duration::from_secs(3600)));
    assert_eq!(cfg.step_timeout, Some(std::time::Duration::from_secs(600)));
}

#[test]
#[serial_test::serial]
fn resolve_timeouts_built_in_default_used_when_nothing_set() {
    let _g1 = EnvGuard::clear("CLAUDINE_TIMEOUT");
    let _g2 = EnvGuard::clear("CLAUDINE_STEP_TIMEOUT");

    let cfg = resolve_timeouts(None, None, None, None);
    // No CLI/frontmatter/env: timeout has no built-in default (None);
    // step_timeout falls back to 30m.
    assert_eq!(cfg.timeout, None);
    assert_eq!(
        cfg.step_timeout,
        Some(std::time::Duration::from_secs(30 * 60))
    );
}

#[test]
#[serial_test::serial]
fn resolve_timeouts_zero_env_disables_rule() {
    let _g1 = EnvGuard::set("CLAUDINE_TIMEOUT", "0s");
    let _g2 = EnvGuard::set("CLAUDINE_STEP_TIMEOUT", "0s");

    let cfg = resolve_timeouts(None, None, None, None);
    assert_eq!(cfg.timeout, None);
    assert_eq!(cfg.step_timeout, None);
}

#[test]
#[serial_test::serial]
fn resolve_timeouts_invalid_env_falls_back_to_built_in() {
    let _g1 = EnvGuard::set("CLAUDINE_TIMEOUT", "garbage");
    let _g2 = EnvGuard::set("CLAUDINE_STEP_TIMEOUT", "also garbage");

    let cfg = resolve_timeouts(None, None, None, None);
    assert_eq!(cfg.timeout, None);
    assert_eq!(
        cfg.step_timeout,
        Some(std::time::Duration::from_secs(30 * 60))
    );
}

#[test]
#[serial_test::serial]
fn resolve_timeouts_accepts_duration_strings_cli() {
    let _g1 = EnvGuard::clear("CLAUDINE_TIMEOUT");
    let _g2 = EnvGuard::clear("CLAUDINE_STEP_TIMEOUT");

    let cfg = resolve_timeouts(Some("2h".into()), None, Some("5m".into()), None);
    assert_eq!(cfg.timeout, Some(std::time::Duration::from_secs(7200)));
    assert_eq!(cfg.step_timeout, Some(std::time::Duration::from_secs(300)));
}

#[test]
#[serial_test::serial]
fn resolve_timeouts_cli_zero_rejected() {
    let _g1 = EnvGuard::clear("CLAUDINE_TIMEOUT");
    let _g2 = EnvGuard::clear("CLAUDINE_STEP_TIMEOUT");

    // parse_timeout rejects 0s, so CLI layer falls through to next
    // precedence (which is None here), resulting in built-in defaults.
    let cfg = resolve_timeouts(Some("0s".into()), None, Some("0s".into()), None);
    assert_eq!(cfg.timeout, None);
    assert_eq!(
        cfg.step_timeout,
        Some(std::time::Duration::from_secs(30 * 60))
    );
}

#[test]
#[serial_test::serial]
fn resolve_timeouts_accepts_hour_and_minute_cli() {
    let _g1 = EnvGuard::clear("CLAUDINE_TIMEOUT");
    let _g2 = EnvGuard::clear("CLAUDINE_STEP_TIMEOUT");

    let cfg = resolve_timeouts(Some("2h".into()), None, Some("30m".into()), None);
    assert_eq!(cfg.timeout, Some(std::time::Duration::from_secs(7200)));
    assert_eq!(cfg.step_timeout, Some(std::time::Duration::from_secs(1800)));
}

#[test]
#[serial_test::serial]
fn resolve_timeouts_cli_duration_string_parsed() {
    let _g1 = EnvGuard::clear("CLAUDINE_TIMEOUT");
    let _g2 = EnvGuard::clear("CLAUDINE_STEP_TIMEOUT");

    let cfg = resolve_timeouts(Some("2h".into()), None, Some("5m".into()), None);
    assert_eq!(cfg.timeout, Some(std::time::Duration::from_secs(7200)));
    assert_eq!(cfg.step_timeout, Some(std::time::Duration::from_secs(300)));
}

#[test]
#[serial_test::serial]
fn resolve_timeouts_rejects_bare_seconds_cli() {
    let _g1 = EnvGuard::set("CLAUDINE_TIMEOUT", "1h");
    let _g2 = EnvGuard::set("CLAUDINE_STEP_TIMEOUT", "5m");

    // Bare seconds like "60" are rejected by parse_timeout, so CLI
    // falls through to env / frontmatter / built-in.
    let cfg = resolve_timeouts(Some("60".into()), None, Some("45".into()), None);
    assert_eq!(cfg.timeout, Some(std::time::Duration::from_secs(3600)));
    assert_eq!(cfg.step_timeout, Some(std::time::Duration::from_secs(300)));
}

// -- Dynamic refresh gating tests (Phase 2) ---------------------------

fn make_hints_with_model(model: &str) -> claudine::composition::EffectiveSelectionHints {
    use claudine::composition::ModelHint;
    claudine::composition::EffectiveSelectionHints {
        agent: None,
        model: Some(ModelHint::Single(model.into())),
        ..Default::default()
    }
}

#[test]
#[serial_test::serial]
fn opencode_model_env_skips_refresh_for_frontmatter_model() {
    let _g = EnvGuard::set("OPENCODE_MODEL", "fast");
    let _g2 = EnvGuard::clear("MODEL");

    let tmp = tempfile::tempdir().unwrap();
    let catalog =
        claudine::model_catalog::ModelCatalogService::with_cache_dir(tmp.path().to_path_buf());
    let hints = make_hints_with_model("slow");

    // Probe resolution without catalog tells us the model comes from env
    let (_, probe_reason) =
        claudine::composition::resolve_model_with_hints(Provider::OpenCode, &hints, None, None);
    assert!(matches!(
        probe_reason,
        ModelResolutionReason::ProviderEnv("OPENCODE_MODEL")
    ));

    refresh_for_model_validation(&catalog, Provider::OpenCode, &hints, Some(&probe_reason));

    // No opencode models subprocess should have been attempted
    assert_eq!(catalog.shell_command_fetch_attempts(), 0);
}

#[test]
#[serial_test::serial]
fn generic_model_env_skips_refresh_for_frontmatter_model() {
    let _g = EnvGuard::set("MODEL", "fast");
    let _g2 = EnvGuard::clear("OPENCODE_MODEL");

    let tmp = tempfile::tempdir().unwrap();
    let catalog =
        claudine::model_catalog::ModelCatalogService::with_cache_dir(tmp.path().to_path_buf());
    let hints = make_hints_with_model("slow");

    let (_, probe_reason) =
        claudine::composition::resolve_model_with_hints(Provider::OpenCode, &hints, None, None);
    assert!(matches!(probe_reason, ModelResolutionReason::GenericEnv));

    refresh_for_model_validation(&catalog, Provider::OpenCode, &hints, Some(&probe_reason));

    assert_eq!(catalog.shell_command_fetch_attempts(), 0);
}

#[test]
#[serial_test::serial]
fn provider_specific_model_env_skips_refresh_for_frontmatter_model() {
    let _g = EnvGuard::set("CLAUDE_MODEL", "claude-3-7-sonnet-20250219");
    let _g2 = EnvGuard::clear("MODEL");

    let tmp = tempfile::tempdir().unwrap();
    let catalog =
        claudine::model_catalog::ModelCatalogService::with_cache_dir(tmp.path().to_path_buf());
    let hints = make_hints_with_model("slow");

    let (_, probe_reason) =
        claudine::composition::resolve_model_with_hints(Provider::Claude, &hints, None, None);
    assert!(matches!(
        probe_reason,
        ModelResolutionReason::ProviderEnv("CLAUDE_MODEL")
    ));

    refresh_for_model_validation(&catalog, Provider::Claude, &hints, Some(&probe_reason));

    // Claude is a static provider; refresh writes to cache. With env
    // override the refresh should be skipped, so no cache file.
    let cache_file = tmp.path().join("claude.json");
    assert!(!cache_file.exists(), "refresh should have been skipped");
}

#[test]
#[serial_test::serial]
fn frontmatter_model_without_env_override_refreshes_dynamic_provider() {
    let _g1 = EnvGuard::clear("OPENCODE_MODEL");
    let _g2 = EnvGuard::clear("MODEL");

    let tmp = tempfile::tempdir().unwrap();
    let catalog =
        claudine::model_catalog::ModelCatalogService::with_cache_dir(tmp.path().to_path_buf());
    let hints = make_hints_with_model("slow");

    let (_, probe_reason) =
        claudine::composition::resolve_model_with_hints(Provider::OpenCode, &hints, None, None);
    assert!(matches!(
        probe_reason,
        ModelResolutionReason::FrontmatterSingle | ModelResolutionReason::ProviderDefault
    ));

    refresh_for_model_validation(&catalog, Provider::OpenCode, &hints, Some(&probe_reason));

    // The refresh detaches to a background thread even on a cold cache
    // (validation is baseline-fed and never waits on the subprocess), so
    // poll for the attempt instead of asserting synchronously. The
    // attempt itself fails gracefully since opencode is not on PATH, but
    // the counter still increments.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while catalog.shell_command_fetch_attempts() == 0 {
        assert!(
            std::time::Instant::now() < deadline,
            "background refresh never attempted the shell-command fetch"
        );
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert_eq!(catalog.shell_command_fetch_attempts(), 1);
}

// ------------------------------------------------------------------------
// Live agent resolution TTY gate (Phase 3)
// ------------------------------------------------------------------------

fn make_empty_hints() -> claudine::composition::EffectiveSelectionHints {
    claudine::composition::EffectiveSelectionHints::default()
}

fn make_snapshot(runnable: Vec<Provider>) -> claudine::composition::InstalledProviderSnapshot {
    claudine::composition::InstalledProviderSnapshot {
        runnable: runnable.clone(),
        excluded: std::collections::BTreeSet::new(),
        all_installed: runnable,
        binary_paths: std::collections::BTreeMap::new(),
    }
}

fn assert_agent_resolution_failed(
    result: &Result<ResolvedExecutionTarget>,
    expected_state: claudine::composition::AgentResolutionState,
) {
    let err = result
        .as_ref()
        .expect_err("expected AgentResolutionFailed error");
    let composition_err = err
        .downcast_ref::<claudine::composition::CompositionError>()
        .expect("error should downcast to CompositionError");
    match composition_err {
        claudine::composition::CompositionError::AgentResolutionFailed { state, .. } => {
            assert_eq!(*state, expected_state, "unexpected resolution state");
        }
        other => panic!("expected AgentResolutionFailed, got {other:?}"),
    }
}

#[test]
fn live_selected_auto_selects_regardless_of_tty() {
    let tmp = tempfile::tempdir().unwrap();
    let catalog =
        claudine::model_catalog::ModelCatalogService::with_cache_dir(tmp.path().to_path_buf());
    let hints = make_empty_hints();
    let snapshot = make_snapshot(vec![Provider::Claude]);
    let state = claudine::composition::AgentResolutionState::Selected {
        provider: Provider::Claude,
    };
    let result = resolve_live_target_with_tty(
        state,
        &hints,
        &snapshot,
        None,
        None,
        &catalog,
        Path::new("/tmp/doc.md"),
        false,
    );
    let target = result.expect("should auto-select");
    assert_eq!(target.provider, Provider::Claude);
    assert!(matches!(
        target.provider_reason,
        claudine::composition::ProviderResolutionReason::FrontmatterSingle
    ));
}

#[test]
fn live_list_one_installed_auto_selects_regardless_of_tty() {
    let tmp = tempfile::tempdir().unwrap();
    let catalog =
        claudine::model_catalog::ModelCatalogService::with_cache_dir(tmp.path().to_path_buf());
    let hints = claudine::composition::EffectiveSelectionHints {
        agent: Some(claudine::composition::AgentHint::List(vec![
            Provider::Claude,
            Provider::Gemini,
        ])),
        ..Default::default()
    };
    let snapshot = make_snapshot(vec![Provider::Claude]);
    let state = claudine::composition::AgentResolutionState::ListOneInstalled {
        selected: Provider::Claude,
        not_installed: vec![Provider::Gemini],
        invalid: Vec::new(),
    };
    let result = resolve_live_target_with_tty(
        state,
        &hints,
        &snapshot,
        None,
        None,
        &catalog,
        Path::new("/tmp/doc.md"),
        false,
    );
    let target = result.expect("should auto-select from list");
    assert_eq!(target.provider, Provider::Claude);
    assert!(matches!(
        target.provider_reason,
        claudine::composition::ProviderResolutionReason::FrontmatterList
    ));
}

#[test]
fn live_no_agent_aborts_when_not_tty() {
    let tmp = tempfile::tempdir().unwrap();
    let catalog =
        claudine::model_catalog::ModelCatalogService::with_cache_dir(tmp.path().to_path_buf());
    let hints = make_empty_hints();
    let snapshot = make_snapshot(vec![]);
    let state = claudine::composition::AgentResolutionState::NoAgent;
    let result = resolve_live_target_with_tty(
        state.clone(),
        &hints,
        &snapshot,
        None,
        None,
        &catalog,
        Path::new("/tmp/doc.md"),
        false,
    );
    assert_agent_resolution_failed(&result, state);
}

#[test]
fn live_single_invalid_aborts_when_not_tty() {
    let tmp = tempfile::tempdir().unwrap();
    let catalog =
        claudine::model_catalog::ModelCatalogService::with_cache_dir(tmp.path().to_path_buf());
    let hints = claudine::composition::EffectiveSelectionHints {
        agent_invalid: vec!["nope".into()],
        ..Default::default()
    };
    let snapshot = make_snapshot(vec![]);
    let state = claudine::composition::AgentResolutionState::SingleInvalid {
        hint: "nope".into(),
    };
    let result = resolve_live_target_with_tty(
        state.clone(),
        &hints,
        &snapshot,
        None,
        None,
        &catalog,
        Path::new("/tmp/doc.md"),
        false,
    );
    assert_agent_resolution_failed(&result, state);
}

#[test]
fn live_single_not_installed_aborts_when_not_tty() {
    let tmp = tempfile::tempdir().unwrap();
    let catalog =
        claudine::model_catalog::ModelCatalogService::with_cache_dir(tmp.path().to_path_buf());
    let hints = claudine::composition::EffectiveSelectionHints {
        agent: Some(claudine::composition::AgentHint::Single(Provider::Gemini)),
        ..Default::default()
    };
    let snapshot = make_snapshot(vec![]);
    let state = claudine::composition::AgentResolutionState::SingleNotInstalled {
        provider: Provider::Gemini,
    };
    let result = resolve_live_target_with_tty(
        state.clone(),
        &hints,
        &snapshot,
        None,
        None,
        &catalog,
        Path::new("/tmp/doc.md"),
        false,
    );
    assert_agent_resolution_failed(&result, state);
}

#[test]
fn live_list_multiple_installed_aborts_when_not_tty() {
    let tmp = tempfile::tempdir().unwrap();
    let catalog =
        claudine::model_catalog::ModelCatalogService::with_cache_dir(tmp.path().to_path_buf());
    let hints = claudine::composition::EffectiveSelectionHints {
        agent: Some(claudine::composition::AgentHint::List(vec![
            Provider::Claude,
            Provider::Gemini,
        ])),
        ..Default::default()
    };
    let snapshot = make_snapshot(vec![Provider::Claude, Provider::Gemini]);
    let state = claudine::composition::AgentResolutionState::ListMultipleInstalled {
        installed: vec![Provider::Claude, Provider::Gemini],
        not_installed: Vec::new(),
        invalid: Vec::new(),
    };
    let result = resolve_live_target_with_tty(
        state.clone(),
        &hints,
        &snapshot,
        None,
        None,
        &catalog,
        Path::new("/tmp/doc.md"),
        false,
    );
    assert_agent_resolution_failed(&result, state);
}

#[test]
fn live_zero_installed_list_aborts_when_not_tty() {
    let tmp = tempfile::tempdir().unwrap();
    let catalog =
        claudine::model_catalog::ModelCatalogService::with_cache_dir(tmp.path().to_path_buf());
    let hints = claudine::composition::EffectiveSelectionHints {
        agent: Some(claudine::composition::AgentHint::List(vec![
            Provider::Claude,
            Provider::Gemini,
        ])),
        ..Default::default()
    };
    let snapshot = make_snapshot(vec![]);
    let state = claudine::composition::AgentResolutionState::ZeroInstalledList {
        not_installed: vec![Provider::Claude, Provider::Gemini],
        invalid: Vec::new(),
    };
    let result = resolve_live_target_with_tty(
        state.clone(),
        &hints,
        &snapshot,
        None,
        None,
        &catalog,
        Path::new("/tmp/doc.md"),
        false,
    );
    assert_agent_resolution_failed(&result, state);
}

// -- Picker scope per state (pure planner helper) ------------------------

fn list_hint(providers: Vec<Provider>) -> claudine::composition::EffectiveSelectionHints {
    claudine::composition::EffectiveSelectionHints {
        agent: Some(claudine::composition::AgentHint::List(providers)),
        agent_was_list: true,
        ..Default::default()
    }
}

#[test]
fn picker_scope_list_multiple_installed_is_scoped_to_suggested() {
    // The picker for a multi-installed list must offer ONLY the suggested
    // installed providers, even when more agents are installed on the host.
    let hints = list_hint(vec![Provider::Claude, Provider::Gemini]);
    let snapshot = make_snapshot(vec![Provider::Claude, Provider::Gemini, Provider::Codex]);
    let state = AgentResolutionState::ListMultipleInstalled {
        installed: vec![Provider::Claude, Provider::Gemini],
        not_installed: Vec::new(),
        invalid: Vec::new(),
    };
    let plan = scoped_picker_plan_for_state(&state, &hints, &snapshot, None).unwrap();
    let providers: Vec<Provider> = plan.options.iter().map(|o| o.provider).collect();
    assert_eq!(providers, vec![Provider::Claude, Provider::Gemini]);
    assert!(
        !providers.contains(&Provider::Codex),
        "installed-but-unsuggested Codex must not appear in the scoped picker"
    );
}

#[test]
fn picker_scope_zero_installed_list_offers_all_installed() {
    // The zero-installed-list state scopes to ALL installed agents, since
    // none of the suggestions are installed.
    let hints = list_hint(vec![Provider::Gemini, Provider::Goose]);
    let snapshot = make_snapshot(vec![Provider::Claude, Provider::Codex]);
    let state = AgentResolutionState::ZeroInstalledList {
        not_installed: vec![Provider::Gemini, Provider::Goose],
        invalid: Vec::new(),
    };
    let plan = scoped_picker_plan_for_state(&state, &hints, &snapshot, None).unwrap();
    let providers: Vec<Provider> = plan.options.iter().map(|o| o.provider).collect();
    assert!(providers.contains(&Provider::Claude));
    assert!(providers.contains(&Provider::Codex));
    assert_eq!(providers.len(), 2);
}

#[test]
fn picker_scope_no_agent_and_invalid_offer_all_installed() {
    let snapshot = make_snapshot(vec![Provider::Claude, Provider::Codex]);
    for state in [
        AgentResolutionState::NoAgent,
        AgentResolutionState::SingleInvalid {
            hint: "nope".into(),
        },
        AgentResolutionState::SingleNotInstalled {
            provider: Provider::Gemini,
        },
    ] {
        assert!(
            picker_scope_for_state(&state).is_none(),
            "state {state:?} must offer all installed agents (no scope)"
        );
        let plan =
            scoped_picker_plan_for_state(&state, &make_empty_hints(), &snapshot, None).unwrap();
        assert_eq!(
            plan.options.len(),
            2,
            "state {state:?} should offer both installed providers"
        );
    }
}

// -- Pre-prompt message text (shared with dry-run / no-TTY) ---------------

#[test]
fn agent_prompt_message_single_invalid_is_imperative_with_link() {
    let state = AgentResolutionState::SingleInvalid {
        hint: "totally-bogus".into(),
    };
    let msg = agent_prompt_message(&state, Path::new("/tmp/doc.md"))
        .expect("single-invalid has a pre-prompt message");
    assert!(msg.contains("<red><b>Invalid Agent:</b></red>"), "got: {msg}");
    assert!(msg.contains("totally-bogus"), "got: {msg}");
    assert!(msg.contains("/tmp/doc.md"), "got: {msg}");
    // The TTY pre-prompt and the no-TTY abort body share this exact text.
    let href = crate::cli_utils::file_url(Path::new("/tmp/doc.md"))
        .expect("test path should convert to a file URL");
    assert!(
        msg.starts_with(&invalid_agent_message(
            "totally-bogus",
            &format!("<a href=\"{href}\">/tmp/doc.md</a>")
        )),
        "got: {msg}"
    );
}

#[test]
fn agent_prompt_message_zero_installed_matches_breakdown() {
    let state = AgentResolutionState::ZeroInstalledList {
        not_installed: vec![Provider::Gemini],
        invalid: vec!["bad".into()],
    };
    let msg = agent_prompt_message(&state, Path::new("/tmp/doc.md"))
        .expect("zero-installed-list has a pre-prompt message");
    assert_eq!(msg, agent_state_breakdown(&state));
}

#[test]
fn agent_prompt_message_is_none_for_picker_only_states() {
    for state in [
        AgentResolutionState::NoAgent,
        AgentResolutionState::SingleNotInstalled {
            provider: Provider::Gemini,
        },
        AgentResolutionState::ListMultipleInstalled {
            installed: vec![Provider::Claude, Provider::Codex],
            not_installed: Vec::new(),
            invalid: Vec::new(),
        },
    ] {
        assert!(
            agent_prompt_message(&state, Path::new("/tmp/doc.md")).is_none(),
            "state {state:?} should not show a pre-prompt message"
        );
    }
}

// -- Interactive timeout conflict (Phase 3) -------------------------------

#[test]
fn timeout_conflict_message_names_source_and_flag() {
    assert_eq!(
        format_interactive_timeout_conflict(
            SessionInteractivitySource::Frontmatter,
            "--timeout"
        ),
        "interactive mode (from frontmatter) cannot be used with --timeout"
    );
    assert_eq!(
        format_interactive_timeout_conflict(
            SessionInteractivitySource::InteractiveFlag,
            "--timeout"
        ),
        "interactive mode (from --interactive) cannot be used with --timeout"
    );
    assert_eq!(
        format_interactive_timeout_conflict(
            SessionInteractivitySource::Default,
            "--timeout"
        ),
        "interactive mode (from default) cannot be used with --timeout"
    );
    // The step-silence flag is named distinctly so a `--step-timeout`
    // conflict does not mis-report as `--timeout`.
    assert_eq!(
        format_interactive_timeout_conflict(
            SessionInteractivitySource::Frontmatter,
            "--step-timeout"
        ),
        "interactive mode (from frontmatter) cannot be used with --step-timeout"
    );
}

/// The conflict check resolves both timeouts against CLI + frontmatter +
/// env sources, excluding the built-in `step_timeout` default. This mirrors
/// the executor guard's source resolution so the unit test catches a
/// regression that drops the `step_timeout` (or frontmatter) source.
fn explicit_timeouts_for(
    cli_timeout: Option<&str>,
    cli_step_timeout: Option<&str>,
    fm: &serde_json::Value,
) -> (Option<std::time::Duration>, Option<std::time::Duration>) {
    let sp = std::path::Path::new("<test>");
    let timeout = resolve_single_timeout(TimeoutResolutionInput {
        cli: cli_timeout.map(str::to_string),
        frontmatter: frontmatter_timeout_duration(fm, "timeout", sp),
        env_var: "CLAUDINE_TIMEOUT_TEST_UNSET",
        built_in: None,
    });
    let step_timeout = resolve_single_timeout(TimeoutResolutionInput {
        cli: cli_step_timeout.map(str::to_string),
        frontmatter: frontmatter_timeout_duration(fm, "step_timeout", sp),
        env_var: "CLAUDINE_STEP_TIMEOUT_TEST_UNSET",
        built_in: None,
    });
    (timeout, step_timeout)
}

#[test]
fn step_timeout_alone_is_an_explicit_conflict_source() {
    // `--step-timeout` with no `--timeout` must still register as an
    // explicit timeout, so a resolved-interactive session rejects it.
    let empty = serde_json::json!({});
    let (timeout, step_timeout) = explicit_timeouts_for(None, Some("30s"), &empty);
    assert!(timeout.is_none(), "no wall-clock timeout was requested");
    assert!(
        step_timeout.is_some(),
        "an explicit --step-timeout must count as a conflict source"
    );
}

#[test]
fn frontmatter_timeout_is_an_explicit_conflict_source() {
    // A composed-frontmatter `timeout` (the harness wall-clock key) must
    // register as an explicit timeout even without any CLI flag.
    let fm = serde_json::json!({ "timeout": "5m" });
    let (timeout, step_timeout) = explicit_timeouts_for(None, None, &fm);
    assert_eq!(timeout, Some(std::time::Duration::from_secs(300)));
    assert!(step_timeout.is_none());
}

#[test]
fn no_explicit_timeout_when_all_sources_empty() {
    // With no CLI flags, no frontmatter timeouts, and the built-in
    // step_timeout default excluded, an interactive session has nothing to
    // conflict with.
    let empty = serde_json::json!({});
    let (timeout, step_timeout) = explicit_timeouts_for(None, None, &empty);
    assert!(timeout.is_none() && step_timeout.is_none());
}

// -- Inline interactive unsupported source (Phase 3) ----------------------

#[test]
fn inline_interactive_unsupported_names_frontmatter_source() {
    let err = CompositionError::InlineInteractiveUnsupported {
        provider: Provider::Claude.to_string(),
        source_kind: SessionInteractivitySource::Frontmatter,
    };
    let msg = err.to_string();
    assert!(msg.contains("Claude"), "{msg}");
    assert!(msg.contains("frontmatter"), "{msg}");
}

#[test]
fn inline_interactive_unsupported_names_flag_source() {
    let err = CompositionError::InlineInteractiveUnsupported {
        provider: Provider::Claude.to_string(),
        source_kind: SessionInteractivitySource::InteractiveFlag,
    };
    let msg = err.to_string();
    assert!(msg.contains("Claude"), "{msg}");
    assert!(msg.contains("--interactive"), "{msg}");
}

// -- emit_preflight_blocked_and_finalize L1 unit tests ---------------------

/// Recording emitter that captures the `signal` + `text` of every `stderr`
/// action, mirroring the pattern in `claudine/lib/.../lifecycle.rs`'s test
/// module. Used to prove [`emit_preflight_blocked_and_finalize`] fires both
/// the `blocked` and `finalize` events through the stack-aware runner (i.e.
/// top-level communication AND typed stack), not just the legacy top-level
/// surface.
struct PreflightRecordingEmitter {
    stderr_calls: std::sync::Mutex<Vec<(claudine::composition::LifecycleSignal, String)>>,
}

impl PreflightRecordingEmitter {
    fn new() -> Self {
        Self {
            stderr_calls: std::sync::Mutex::new(Vec::new()),
        }
    }

    fn stderr_calls(&self) -> Vec<(claudine::composition::LifecycleSignal, String)> {
        self.stderr_calls.lock().unwrap().clone()
    }
}

impl claudine::composition::LifecycleEmitter for PreflightRecordingEmitter {
    fn emit_stderr(
        &self,
        signal: claudine::composition::LifecycleSignal,
        text: &str,
        _term: &Terminal,
    ) {
        self.stderr_calls
            .lock()
            .unwrap()
            .push((signal, text.to_string()));
    }

    fn emit_message(
        &self,
        _text: &str,
        _source_path: &Path,
        _repo_root: Option<&Path>,
        _messaging: &claudine::messaging::RuntimeMessagingSettings,
    ) {
    }

    fn emit_speech(&self, _text: &str, _tts_config: biscuit_speaks::TtsConfig) {}

    fn emit_effect(&self, _name: &str) {}

    fn emit_notification(&self, _title: &str) {}
}

/// The helper fires BOTH `blocked` and `finalize`, each with its top-level
/// communication AND its typed stack. The doc under test carries a
/// top-level `stderr` line plus a stack `stderr` action on each event,
/// so a successful invocation records exactly four stderr emissions in the
/// order: blocked top-level, blocked stack, finalize top-level, finalize
/// stack. This is the L1 invariant for review-3 Finding 1.
#[test]
fn emit_preflight_blocked_and_finalize_runs_top_level_and_stack_for_both_events() {
    use claudine::composition::{
        LifecycleErrorInfo, LifecycleRunGuard, LifecycleRuntimeContext, LifecycleSignal,
        parse_lifecycle_config,
    };
    use serde_json::json;

    let fm = json!({
        "blocked": {
            "stderr": "blocked-toplevel",
            "stack": [{"action": {"stderr": "blocked-stack"}}]
        },
        "finalize": {
            "stderr": "finalize-toplevel",
            "stack": [{"action": {"stderr": "finalize-stack"}}]
        }
    });
    let config = parse_lifecycle_config(&fm, Path::new("/tmp/test.md")).unwrap();
    let settings = claudine::events::GlobalSettings::default();
    let messaging = claudine::messaging::RuntimeMessagingSettings {
        user: None,
        repo: None,
    };
    let term = Terminal::default();
    let ctx = LifecycleRuntimeContext {
        settings: &settings,
        messaging: &messaging,
        term: &term,
        source_path: Path::new("/tmp/test.md"),
        repo_root: None,
        launch_area: None,
        context: None,
    };
    let emitter = PreflightRecordingEmitter::new();
    let effect_engine = darkmatter::effects::EffectEngine::builder()
        .mutation_root(std::env::temp_dir())
        .auto_rehash(false)
        .build();
    let frontmatter = serde_json::Map::new();
    let err_info = LifecycleErrorInfo::from_action_failure(
        "harness_plan",
        "harness plan parse failed: bogus".to_string(),
    );

    let mut guard = LifecycleRunGuard::new(&config, &ctx, &emitter);
    let outcome = emit_preflight_blocked_and_finalize(
        &mut guard,
        &effect_engine,
        &emitter,
        &settings,
        &messaging,
        &term,
        Path::new("/tmp/test.md"),
        None,
        None,
        None,
        None,
        &frontmatter,
        std::time::Instant::now(),
        err_info,
    );
    assert!(
        matches!(outcome, PreflightBlockedOutcome::Control(None)),
        "no evaluation error and no flow-control action → Control(None); got {outcome:?}"
    );

    let calls = emitter.stderr_calls();
    let signals: Vec<LifecycleSignal> = calls.iter().map(|(s, _)| *s).collect();
    assert_eq!(
        signals,
        vec![
            LifecycleSignal::Blocked,
            LifecycleSignal::Blocked,
            LifecycleSignal::Finalize,
            LifecycleSignal::Finalize,
        ],
        "emit_preflight_blocked_and_finalize must fire blocked (top-level + stack) \
         then finalize (top-level + stack); got {calls:?}"
    );
    let texts: Vec<&str> = calls.iter().map(|(_, t)| t.as_str()).collect();
    assert_eq!(
        texts,
        vec![
            "blocked-toplevel",
            "blocked-stack",
            "finalize-toplevel",
            "finalize-stack",
        ],
        "top-level + stack texts must appear in order per event; got {calls:?}"
    );

    // Guard state: blocked is the recorded terminal, finalize fired exactly once.
    assert_eq!(guard.terminal_signal(), Some(LifecycleSignal::Blocked));
    assert!(guard.finalize_emitted());
    // Drop the guard after the explicit terminal + finalize: must NOT add a
    // fifth emission (the explicit `terminal_emitted = true` from
    // `execute_event(Blocked)` suppresses the Drop safety-net).
    drop(guard);
    assert_eq!(
        emitter.stderr_calls().len(),
        4,
        "Drop safety-net must NOT double-emit after explicit blocked+finalize"
    );
}

/// The `err` global carried into the blocked stack must surface the
/// preflight failure message so a user-authored `blocked.stack` can reference
/// `{{ err.msg }}` meaningfully. Verifies the err_info contract: the helper
/// threads the failure description into the stack evaluation context.
#[test]
fn emit_preflight_blocked_and_finalize_propagates_err_msg_into_blocked_stack() {
    use claudine::composition::{
        LifecycleErrorInfo, LifecycleRunGuard, LifecycleRuntimeContext, LifecycleSignal,
        parse_lifecycle_config,
    };
    use serde_json::json;

    // `err` reaches a message via `{{ … }}` interpolation (resolved at render
    // time against the runtime stack context), not a bare expression arg — a
    // single-parameter action body is literal text.
    let fm = json!({
        "blocked": {
            "stack": [{"action": {"stderr": "{{err.msg}}"}}]
        },
        "finalize": {
            "stack": [{"action": {"stderr": "finalize"}}]
        }
    });
    let config = parse_lifecycle_config(&fm, Path::new("/tmp/test.md")).unwrap();
    let settings = claudine::events::GlobalSettings::default();
    let messaging = claudine::messaging::RuntimeMessagingSettings {
        user: None,
        repo: None,
    };
    let term = Terminal::default();
    let ctx = LifecycleRuntimeContext {
        settings: &settings,
        messaging: &messaging,
        term: &term,
        source_path: Path::new("/tmp/test.md"),
        repo_root: None,
        launch_area: None,
        context: None,
    };
    let emitter = PreflightRecordingEmitter::new();
    let effect_engine = darkmatter::effects::EffectEngine::builder()
        .mutation_root(std::env::temp_dir())
        .auto_rehash(false)
        .build();
    let frontmatter = serde_json::Map::new();

    let mut guard = LifecycleRunGuard::new(&config, &ctx, &emitter);
    let outcome = emit_preflight_blocked_and_finalize(
        &mut guard,
        &effect_engine,
        &emitter,
        &settings,
        &messaging,
        &term,
        Path::new("/tmp/test.md"),
        None,
        None,
        None,
        None,
        &frontmatter,
        std::time::Instant::now(),
        LifecycleErrorInfo::from_action_failure(
            "shell_approval",
            "shell command 'rm' is blacklisted".to_string(),
        ),
    );
    assert!(
        matches!(outcome, PreflightBlockedOutcome::Control(None)),
        "no evaluation error and no flow-control action → Control(None); got {outcome:?}"
    );
    drop(guard);

    let calls = emitter.stderr_calls();
    // Only the blocked stack ran a stderr action; the err.msg text must
    // appear verbatim, proving the err global propagated into the stack
    // expression evaluation.
    let blocked_stack_texts: Vec<&str> = calls
        .iter()
        .filter(|(s, _)| *s == LifecycleSignal::Blocked)
        .map(|(_, t)| t.as_str())
        .collect();
    assert!(
        blocked_stack_texts
            .iter()
            .any(|t| t.contains("shell command 'rm' is blacklisted")),
        "blocked.stack must see err.msg; got {calls:?}"
    );
}

/// A late-binding evaluation error raised by the `blocked` stack (here a
/// `when:` guard referencing an undefined root under DM2 strict mode) takes
/// precedence over the original pre-flight failure: the helper surfaces the
/// typed `LifecycleEvaluationError` for the `blocked` event and runs
/// `finalize` carrying the evaluation error as `err` (proven by the
/// finalize stack seeing the evaluation error's message, not the original
/// pre-flight message).
#[test]
fn emit_preflight_blocked_and_finalize_surfaces_blocked_evaluation_error() {
    use claudine::composition::{
        LifecycleErrorInfo, LifecycleRunGuard, LifecycleRuntimeContext, parse_lifecycle_config,
    };
    use serde_json::json;

    let dir = tempfile::tempdir().unwrap();
    let source_path = dir.path().join("test.md");
    let log_path = dir.path().join("events.log");
    // The `blocked` stack's `when:` references an undefined root, so it
    // *raises* at event time rather than evaluating cleanly to false. The
    // `failure` and `finalize` stacks each append a marker so we can prove
    // both events fired carrying the evaluation error as `err`.
    let fm = json!({
        "blocked": {
            "stack": [{"when": "missing_root == true", "action": {"stderr": "unreachable"}}]
        },
        "failure": {
            "stack": [{"when": "err", "action": {"append_line": ["events.log", "failure-saw-err"]}}]
        },
        "finalize": {
            "stack": [{"when": "err", "action": {"append_line": ["events.log", "{{err.msg}}"]}}]
        }
    });
    let config = parse_lifecycle_config(&fm, &source_path).unwrap();
    let settings = claudine::events::GlobalSettings::default();
    let messaging = claudine::messaging::RuntimeMessagingSettings {
        user: None,
        repo: None,
    };
    let term = Terminal::default();
    let ctx = LifecycleRuntimeContext {
        settings: &settings,
        messaging: &messaging,
        term: &term,
        source_path: &source_path,
        repo_root: Some(dir.path()),
        launch_area: None,
        context: None,
    };
    let emitter = PreflightRecordingEmitter::new();
    let effect_engine = darkmatter::effects::EffectEngine::builder()
        .mutation_root(dir.path())
        .auto_rehash(false)
        .build();
    let frontmatter = serde_json::Map::new();
    let err_info = LifecycleErrorInfo::from_action_failure(
        "harness_plan",
        "original-preflight-failure-message".to_string(),
    );

    let mut guard = LifecycleRunGuard::new(&config, &ctx, &emitter);
    let outcome = emit_preflight_blocked_and_finalize(
        &mut guard,
        &effect_engine,
        &emitter,
        &settings,
        &messaging,
        &term,
        &source_path,
        Some(dir.path()),
        Some(dir.path()),
        None,
        None,
        &frontmatter,
        std::time::Instant::now(),
        err_info,
    );
    drop(guard);

    let ce = match outcome {
        PreflightBlockedOutcome::EvaluationError(ce) => ce,
        other => panic!("expected EvaluationError, got {other:?}"),
    };
    let msg = ce.to_string();
    assert!(
        msg.contains("lifecycle"),
        "error must mention lifecycle; got: {msg}"
    );
    assert!(
        msg.contains("evaluation error"),
        "error must mention evaluation error; got: {msg}"
    );
    assert!(
        msg.contains("`blocked`"),
        "error must name the blocked event; got: {msg}"
    );

    // The failure stack ran and saw `err` (the evaluation error), proving the
    // helper routed through `failure` — not just `finalize`. Without the
    // `redesignate_terminal_to_failure` fix, `execute_event(Failure)` would
    // be a no-op (the terminal slot was already taken by Blocked) and this
    // marker would be absent.
    let log = std::fs::read_to_string(&log_path).unwrap();
    assert!(
        log.contains("failure-saw-err"),
        "failure stack must have fired with the evaluation error as err; got: {log}"
    );

    // The finalize stack also ran and saw the *evaluation error* as `err` — its
    // message references `missing_root`, which only the raised `when:` guard
    // produces, not the original pre-flight `err_info` ("original-preflight-
    // failure-message"). This is the load-bearing assertion: without the
    // fix, finalize would run with the original pre-flight error instead.
    assert!(
        log.contains("missing_root"),
        "finalize must have run with the evaluation error as err (msg should mention \
         missing_root); got: {log}"
    );
    assert!(
        !log.contains("original-preflight-failure-message"),
        "finalize must NOT have run with the original pre-flight err; got: {log}"
    );
}

/// Precedence: a `blocked.when` raise followed by a catch `finalize.when`
/// raise must surface the `finalize` raise — the latest lifecycle crash —
/// not the original `blocked` raise. Previously the blocked raise hid the
/// finalize raise behind it because the catch path discarded the finalize
/// outcome.
#[test]
fn emit_preflight_blocked_and_finalize_blocked_raise_then_finalize_raise_surfaces_finalize() {
    use claudine::composition::{
        LifecycleErrorInfo, LifecycleRunGuard, LifecycleRuntimeContext, parse_lifecycle_config,
    };
    use serde_json::json;

    let dir = tempfile::tempdir().unwrap();
    let source_path = dir.path().join("test.md");
    let log_path = dir.path().join("events.log");
    // The `blocked` stack's `when:` raises (undefined root), triggering the
    // catch path (failure + finalize). The `failure` stack is clean so we
    // isolate the precedence to blocked-raise vs finalize-raise. The
    // `finalize` stack's `when:` also raises, so the surfaced error must name
    // `finalize`.
    let fm = json!({
        "blocked": {
            "stack": [{"when": "missing_root == true", "action": {"stderr": "unreachable"}}]
        },
        "failure": {
            "stack": [{"when": "err", "action": {"append_line": ["events.log", "failure-ran"]}}]
        },
        "finalize": {
            "stack": [{"when": "also_missing == true", "action": {"stderr": "unreachable"}}]
        }
    });
    let config = parse_lifecycle_config(&fm, &source_path).unwrap();
    let settings = claudine::events::GlobalSettings::default();
    let messaging = claudine::messaging::RuntimeMessagingSettings {
        user: None,
        repo: None,
    };
    let term = Terminal::default();
    let ctx = LifecycleRuntimeContext {
        settings: &settings,
        messaging: &messaging,
        term: &term,
        source_path: &source_path,
        repo_root: Some(dir.path()),
        launch_area: None,
        context: None,
    };
    let emitter = PreflightRecordingEmitter::new();
    let effect_engine = darkmatter::effects::EffectEngine::builder()
        .mutation_root(dir.path())
        .auto_rehash(false)
        .build();
    let frontmatter = serde_json::Map::new();
    let err_info = LifecycleErrorInfo::from_action_failure(
        "harness_plan",
        "original-preflight-failure-message".to_string(),
    );

    let mut guard = LifecycleRunGuard::new(&config, &ctx, &emitter);
    let outcome = emit_preflight_blocked_and_finalize(
        &mut guard,
        &effect_engine,
        &emitter,
        &settings,
        &messaging,
        &term,
        &source_path,
        Some(dir.path()),
        Some(dir.path()),
        None,
        None,
        &frontmatter,
        std::time::Instant::now(),
        err_info,
    );
    drop(guard);

    let ce = match outcome {
        PreflightBlockedOutcome::EvaluationError(ce) => ce,
        other => panic!("expected EvaluationError, got {other:?}"),
    };
    let msg = ce.to_string();
    assert!(
        msg.contains("`finalize`"),
        "error must name the finalize event (latest crash); got: {msg}"
    );
    assert!(
        !msg.contains("`blocked`"),
        "error must NOT name the blocked event (hidden behind finalize raise); got: {msg}"
    );

    // The catch path ran `failure` (no raise authored) carrying the blocked
    // evaluation error as `err`.
    let log = std::fs::read_to_string(&log_path).unwrap();
    assert!(
        log.contains("failure-ran"),
        "failure stack ran with the blocked evaluation error as err: {log}"
    );
}

/// A late-binding evaluation error raised by the `finalize` stack itself
/// surfaces as the typed `LifecycleEvaluationError` for the `finalize` event
/// without re-entering `finalize` (the re-entry guard from Decision #3).
/// `finalize` runs exactly once: the raise halts the run; the helper does
/// not loop back into `finalize`.
#[test]
fn emit_preflight_blocked_and_finalize_surfaces_finalize_evaluation_error_without_reentry() {
    use claudine::composition::{
        LifecycleErrorInfo, LifecycleRunGuard, LifecycleRuntimeContext, parse_lifecycle_config,
    };
    use serde_json::json;

    let dir = tempfile::tempdir().unwrap();
    let source_path = dir.path().join("test.md");
    let log_path = dir.path().join("events.log");
    // The blocked stack is clean (appends a marker so we can prove it ran);
    // the finalize stack's `when:` raises, so `finalize` runs exactly once
    // and the helper returns the typed error without re-entering finalize.
    let fm = json!({
        "blocked": {
            "stack": [{"action": {"append_line": ["events.log", "blocked-ran"]}}]
        },
        "finalize": {
            "stack": [{"when": "missing_root == true", "action": {"stderr": "unreachable"}}]
        }
    });
    let config = parse_lifecycle_config(&fm, &source_path).unwrap();
    let settings = claudine::events::GlobalSettings::default();
    let messaging = claudine::messaging::RuntimeMessagingSettings {
        user: None,
        repo: None,
    };
    let term = Terminal::default();
    let ctx = LifecycleRuntimeContext {
        settings: &settings,
        messaging: &messaging,
        term: &term,
        source_path: &source_path,
        repo_root: Some(dir.path()),
        launch_area: None,
        context: None,
    };
    let emitter = PreflightRecordingEmitter::new();
    let effect_engine = darkmatter::effects::EffectEngine::builder()
        .mutation_root(dir.path())
        .auto_rehash(false)
        .build();
    let frontmatter = serde_json::Map::new();
    let err_info =
        LifecycleErrorInfo::from_action_failure("harness_plan", "preflight-failure".to_string());

    let mut guard = LifecycleRunGuard::new(&config, &ctx, &emitter);
    let outcome = emit_preflight_blocked_and_finalize(
        &mut guard,
        &effect_engine,
        &emitter,
        &settings,
        &messaging,
        &term,
        &source_path,
        Some(dir.path()),
        Some(dir.path()),
        None,
        None,
        &frontmatter,
        std::time::Instant::now(),
        err_info,
    );
    drop(guard);

    let ce = match outcome {
        PreflightBlockedOutcome::EvaluationError(ce) => ce,
        other => panic!("expected EvaluationError, got {other:?}"),
    };
    let msg = ce.to_string();
    assert!(
        msg.contains("`finalize`"),
        "error must name the finalize event; got: {msg}"
    );

    // The blocked stack ran (its marker is in the log); finalize raised and
    // did not re-enter, so the blocked marker appears exactly once. The
    // finalize stack's `stderr` action never ran (its `when:` raised before
    // the action could execute), proving the re-entry guard held.
    let log = std::fs::read_to_string(&log_path).unwrap();
    assert_eq!(
        log.lines().filter(|l| *l == "blocked-ran").count(),
        1,
        "blocked stack ran exactly once; got: {log}"
    );
}

/// The happy path: when neither the `blocked` nor the `finalize` stack
/// raises an evaluation error, the helper returns the blocked stack's
/// flow-control action (here `None`) unchanged. Verifies the fix does not
/// regress the no-evaluation-error path.
#[test]
fn emit_preflight_blocked_and_finalize_returns_control_when_no_evaluation_error() {
    use claudine::composition::{
        LifecycleErrorInfo, LifecycleRunGuard, LifecycleRuntimeContext, parse_lifecycle_config,
    };
    use serde_json::json;

    let dir = tempfile::tempdir().unwrap();
    let source_path = dir.path().join("test.md");
    let fm = json!({
        "blocked": {
            "stderr": "blocked",
            "stack": [{"action": {"stderr": "blocked-stack"}}]
        },
        "finalize": {
            "stderr": "finalize",
            "stack": [{"action": {"stderr": "finalize-stack"}}]
        }
    });
    let config = parse_lifecycle_config(&fm, &source_path).unwrap();
    let settings = claudine::events::GlobalSettings::default();
    let messaging = claudine::messaging::RuntimeMessagingSettings {
        user: None,
        repo: None,
    };
    let term = Terminal::default();
    let ctx = LifecycleRuntimeContext {
        settings: &settings,
        messaging: &messaging,
        term: &term,
        source_path: &source_path,
        repo_root: Some(dir.path()),
        launch_area: None,
        context: None,
    };
    let emitter = PreflightRecordingEmitter::new();
    let effect_engine = darkmatter::effects::EffectEngine::builder()
        .mutation_root(dir.path())
        .auto_rehash(false)
        .build();
    let frontmatter = serde_json::Map::new();
    let err_info =
        LifecycleErrorInfo::from_action_failure("harness_plan", "preflight-failure".to_string());

    let mut guard = LifecycleRunGuard::new(&config, &ctx, &emitter);
    let outcome = emit_preflight_blocked_and_finalize(
        &mut guard,
        &effect_engine,
        &emitter,
        &settings,
        &messaging,
        &term,
        &source_path,
        Some(dir.path()),
        Some(dir.path()),
        None,
        None,
        &frontmatter,
        std::time::Instant::now(),
        err_info,
    );
    drop(guard);

    assert!(
        matches!(outcome, PreflightBlockedOutcome::Control(None)),
        "no evaluation error and no flow-control action → Control(None); got {outcome:?}"
    );
}

// --- Composition OpenCode YOLO / OPENCODE_CONFIG_CONTENT assembly -----------
//
// These exercise the composition env-assembly *sequence* (system-prompt
// `instructions` → MCP fold → YOLO overlay) the executor runs, using the same
// helpers the executor calls. They are the composition-path counterparts to the
// direct wrapper's Stage 10.5 coverage in `wrapper_stages.rs`.

mod opencode_yolo_assembly {
    use super::*;
    use crate::commands::wrap::wrapper_mcp::merge_injected_env_into_plan;
    use crate::commands::wrap::wrapper_stages::apply_opencode_yolo_config_overlay;
    use serde_json::Value;
    use std::collections::HashMap;

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
    fn non_interactive_yolo_adds_permission_block() {
        let mut plan = env::EnvPlan::default();
        apply_opencode_yolo_config_overlay(Provider::OpenCode, true, true, &mut plan).unwrap();

        let config = config_value(&plan).expect("permission overlay was written");
        let permission = config.get("permission").and_then(Value::as_object).unwrap();
        assert_eq!(permission["*"], "allow");
        assert_eq!(permission["external_directory"], "allow");
        assert_eq!(permission["doom_loop"], "allow");
    }

    #[test]
    fn non_yolo_run_adds_no_permission_key() {
        // Mirrors the composition gate passing `effective_yolo == false`.
        let mut plan = env::EnvPlan::default();
        apply_opencode_yolo_config_overlay(Provider::OpenCode, false, true, &mut plan).unwrap();
        assert!(config_value(&plan).is_none());
    }

    /// Full composition assembly: system-prompt `instructions`, then the MCP
    /// fold, then the YOLO overlay. The final config must preserve all three —
    /// the regression guard for the MCP fold clobbering an existing value.
    #[test]
    fn instructions_mcp_and_yolo_all_survive_assembly() {
        // Stage: system-prompt writer contributed `instructions`.
        let mut plan = env_plan_with(Some(r#"{"instructions":["/tmp/sp.md"]}"#));

        // Stage: MCP fold injects an `mcp` block (must merge, not clobber).
        let injected =
            HashMap::from([(KEY.to_string(), r#"{"mcp":{"srv":{}}}"#.to_string())]);
        merge_injected_env_into_plan(injected, &mut plan).unwrap();

        // Stage: YOLO overlay, effective + non-interactive.
        apply_opencode_yolo_config_overlay(Provider::OpenCode, true, true, &mut plan).unwrap();

        let config = config_value(&plan).unwrap();
        assert_eq!(config["instructions"], serde_json::json!(["/tmp/sp.md"]));
        assert_eq!(config["mcp"], serde_json::json!({ "srv": {} }));
        assert_eq!(config["permission"]["*"], "allow");
        assert_eq!(config["permission"]["external_directory"], "allow");
        assert_eq!(config["permission"]["doom_loop"], "allow");
    }

    /// Regression for the compose-path clobber the `real_` subagent test
    /// surfaced live: in `execute_composition_request_inner_with_guard` the YOLO
    /// `permission` overlay is applied (mod.rs:826) **before** the system-prompt
    /// writer contributes `instructions` (mod.rs:~981). A discovered
    /// `system-prompt.md` made the system-prompt writer run on an ordinary
    /// `compose --opencode --yolo` even with no `--asp`, and a raw
    /// `env_plan.env.insert` there dropped the whole `permission` block — silently
    /// re-opening the subagent `external_directory` hang. The fix routes the
    /// system-prompt `OPENCODE_CONFIG_CONTENT` through the same merge helper, so
    /// `instructions` and `permission` must coexist regardless of which writer
    /// runs last.
    #[test]
    fn system_prompt_instructions_folded_after_yolo_keep_permission() {
        // Stage: YOLO overlay runs first (matches mod.rs:826 ordering).
        let mut plan = env::EnvPlan::default();
        apply_opencode_yolo_config_overlay(Provider::OpenCode, true, true, &mut plan).unwrap();

        // Stage: the system-prompt writer contributes `instructions` LAST,
        // folded through the shared merge helper (the fixed behavior) rather
        // than a raw clobbering insert.
        let injected = HashMap::from([(
            KEY.to_string(),
            r#"{"instructions":["/tmp/sp.md"]}"#.to_string(),
        )]);
        merge_injected_env_into_plan(injected, &mut plan).unwrap();

        let config = config_value(&plan).expect("merged config present");
        assert_eq!(config["instructions"], serde_json::json!(["/tmp/sp.md"]));
        assert_eq!(config["permission"]["*"], "allow");
        assert_eq!(config["permission"]["external_directory"], "allow");
        assert_eq!(config["permission"]["doom_loop"], "allow");
    }

    /// A user-supplied object-valued OPENCODE_CONFIG_CONTENT is merged through
    /// the whole composition sequence, never replaced.
    #[test]
    fn user_supplied_object_value_is_merged_not_replaced() {
        let mut plan = env_plan_with(Some(r#"{"theme":"dark"}"#));

        let injected =
            HashMap::from([(KEY.to_string(), r#"{"mcp":{"srv":{}}}"#.to_string())]);
        merge_injected_env_into_plan(injected, &mut plan).unwrap();
        apply_opencode_yolo_config_overlay(Provider::OpenCode, true, true, &mut plan).unwrap();

        let config = config_value(&plan).unwrap();
        assert_eq!(config["theme"], "dark");
        assert_eq!(config["mcp"], serde_json::json!({ "srv": {} }));
        assert_eq!(config["permission"]["*"], "allow");
    }
}

mod projected_rate_limit_bridge {
    use claudine::stream::summary::RateLimitInfo;

    use super::IterationSummarySignals;

    fn parser_signals() -> IterationSummarySignals {
        IterationSummarySignals {
            rate_limit: Some(RateLimitInfo {
                is_throttled: Some(true),
                retry_after_ms: None,
                message: Some("rendered by parser".into()),
                reset_at: None,
            }),
            ..Default::default()
        }
    }

    /// No projection → the parser value survives untouched (the migration
    /// bridge's fallback).
    #[test]
    fn none_projection_keeps_parser_value() {
        let mut signals = parser_signals();
        signals.apply_projected_rate_limit(None);
        assert_eq!(
            signals.rate_limit.unwrap().message.as_deref(),
            Some("rendered by parser")
        );
    }

    /// Projected fields win where present; parser fields fill the gaps —
    /// notably the parser's rendered message when the projection carries
    /// no raw provider message.
    #[test]
    fn projected_fields_win_and_parser_fills_gaps() {
        let mut signals = parser_signals();
        signals.apply_projected_rate_limit(Some(RateLimitInfo {
            is_throttled: Some(true),
            retry_after_ms: Some(5_000),
            message: None,
            reset_at: None,
        }));
        let merged = signals.rate_limit.unwrap();
        assert_eq!(merged.is_throttled, Some(true));
        assert_eq!(merged.retry_after_ms, Some(5_000));
        assert_eq!(merged.message.as_deref(), Some("rendered by parser"));
    }

    /// A projection with no parser counterpart installs itself wholesale.
    #[test]
    fn projection_without_parser_value_installs_itself() {
        let mut signals = IterationSummarySignals::default();
        signals.apply_projected_rate_limit(Some(RateLimitInfo {
            is_throttled: Some(true),
            retry_after_ms: None,
            message: Some("raw provider message".into()),
            reset_at: None,
        }));
        let merged = signals.rate_limit.unwrap();
        assert_eq!(merged.is_throttled, Some(true));
        assert_eq!(merged.message.as_deref(), Some("raw provider message"));
    }
}

// The former `proxy_target_declares_loop_follows_the_adopted_target` test was
// removed with the pipeline peek it exercised: every committed `initialize`
// proxy on a live run now surfaces up to the composition coordinator regardless
// of whether the target loops, so there is no pre-commit loop peek to test. Loop
// ownership is decided from the fully prepared target after re-entry (R7),
// covered by the harness-orchestration and L2 equivalence suites.
