//! Resolve the runaway-output guards for a single wrapped run (Phase 6).
//!
//! Bridges the configuration layer ([`claudine::runaway::config`]) and the
//! pure detector ([`claudine::runaway::detector`]) at the CLI wiring point:
//! it loads the user/repo/frontmatter `exit_expressions` + `guard_settings`,
//! resolves them through the three-layer pipeline, filters the
//! exit-expression set to the entries whose `scope` matches the run's
//! `(provider, model)`, compiles that set once, and hands back a
//! [`ContentDetector`] for the streaming path plus a [`CaptureVolumeCap`]
//! for the capture path.
//!
//! Resolution fails closed on a declared-but-broken safety rule: the
//! contract is **absent = built-in defaults; present-but-invalid = abort**.
//! A genuinely-absent layer (no `~/.claudine/config.json`, a repo with no
//! `.claudine/config.json`, or a document without frontmatter
//! `exit_expressions`/`guard_settings`) still falls back to the built-in
//! defaults (guards enabled with conservative thresholds, empty
//! exit-expression set). But a layer that is *present* yet unreadable,
//! malformed, or declares an unknown `scope` agent / invalid regex aborts
//! the run rather than silently reverting to weaker guards — otherwise a
//! typo'd safety rule leaves the user believing a guard is active while
//! Claudine proceeds with the built-in defaults.

use claudine::dispatch::loader;
use claudine::error::{ClaudineError, ConfigCause, Result};
use claudine::events::EnvironmentContext;
use claudine::provider::Provider;
use claudine::runaway::{
    extract_frontmatter_exit_expressions, resolve_exit_expressions, resolve_guard_settings,
    validate_exit_expressions, CaptureVolumeCap, CompiledExitExpressions, ContentDetector,
    DetectorConfig, ExitExpressionEntry, GuardSettings, ScopeSelector,
};
use serde_json::{Map, Value};

/// The resolved runaway guards for one run.
///
/// `detector` is `None` when every guard is disabled and the in-scope
/// exit-expression set is empty (the streaming path then skips detector
/// construction entirely — zero per-line overhead for opted-out runs). The
/// `capture_volume_cap` is always present (it carries its own `enabled`
/// flag) so the capture path can arm it unconditionally.
pub(crate) struct ResolvedRunawayGuards {
    pub(crate) detector: Option<ContentDetector>,
    pub(crate) capture_volume_cap: CaptureVolumeCap,
}

/// The validated, layer-merged guard config for one run, *before* scope
/// filtering against a `(provider, model)`.
///
/// This is the expensive half of guard resolution: it has already run the
/// user/repo/frontmatter I/O and the fail-closed validation, so it is
/// produced exactly once per run. The cheap, re-runnable half is
/// [`ResolvedGuardInputs::compile_for_model`], which filters this entry set
/// to a given model and compiles it. The streaming path keeps a copy so it
/// can re-scope when a provider reports its actual model via `SessionStart`
/// (the model supplied on the CLI / in frontmatter is only a launch-time
/// hint and is often absent).
pub(crate) struct ResolvedGuardInputs {
    provider: Provider,
    entries: Vec<ExitExpressionEntry>,
    guards: GuardSettings,
}

impl ResolvedGuardInputs {
    /// Build guard inputs from already-resolved parts, bypassing the
    /// I/O + validation phase. Test-only: production code resolves inputs
    /// through [`resolve_guard_inputs`] so the fail-closed contract is
    /// always exercised.
    #[cfg(test)]
    pub(crate) fn from_parts(
        provider: Provider,
        entries: Vec<ExitExpressionEntry>,
        guards: GuardSettings,
    ) -> Self {
        Self {
            provider,
            entries,
            guards,
        }
    }

    /// The provider this config was resolved for.
    pub(crate) fn provider(&self) -> Provider {
        self.provider
    }

    /// Whether any resolved exit-expression entry could match this provider
    /// under *some* model — i.e. a global, agent-only, or agent/model scope
    /// targeting this provider.
    ///
    /// Used to decide whether a run that armed *no* launch-time detector (all
    /// other guards off, the only exit expression out of scope for the
    /// launch-time model) should still wire a trip channel, in case a
    /// `SessionStart` model brings such an entry into scope.
    pub(crate) fn has_provider_scoped_entries(&self) -> bool {
        self.entries.iter().any(|entry| match entry.scope.as_deref() {
            None => true,
            Some(scope) => ScopeSelector::parse(scope)
                .map(|selector| match selector {
                    ScopeSelector::Global => true,
                    ScopeSelector::Agent(p) | ScopeSelector::AgentModel(p, _) => p == self.provider,
                })
                .unwrap_or(false),
        })
    }

    /// Filter the resolved exit-expression set to the entries whose scope
    /// matches `(provider, model)`, compile that set, and assemble the
    /// detector + capture cap for the run.
    ///
    /// An unknown model (`None`) is treated as `""`: global and agent
    /// scopes still match, but agent/model scopes do not (conservative —
    /// never a wrongful match).
    ///
    /// ## Errors
    ///
    /// Returns [`ClaudineError`] only if the in-scope set fails to compile.
    /// Validation already rejected unknown scopes / invalid regexes when
    /// the inputs were resolved, so a compile error here is unexpected — but
    /// it still aborts rather than silently disabling the declared set
    /// (fail-closed, consistent with the resolve phase).
    pub(crate) fn compile_for_model(&self, model: Option<&str>) -> Result<ResolvedRunawayGuards> {
        let model = model.unwrap_or("");
        let inputs: Vec<_> = self
            .entries
            .iter()
            .filter(|entry| scope_matches(entry.scope.as_deref(), self.provider, model))
            .map(|entry| entry.to_input())
            .collect();

        // Compile once per (re)scope (never per line). Validation already
        // rejected unknown scopes / invalid regexes when the inputs were
        // resolved, so a compile error here is unexpected — but it still
        // aborts rather than silently disabling the declared set.
        let compiled = CompiledExitExpressions::compile(&inputs)?;

        let detector_config = DetectorConfig {
            repetition_enabled: self.guards.repetition.enabled,
            max_repeats: self.guards.repetition.max_repeats,
            max_cycle_length: self.guards.repetition.max_cycle_length,
            volume_enabled: self.guards.volume.enabled,
            max_lines: self.guards.volume.max_lines,
            max_bytes: self.guards.volume.max_bytes,
        };

        let capture_volume_cap = CaptureVolumeCap::new(
            self.guards.volume.enabled,
            self.guards.volume.max_lines,
            self.guards.volume.max_bytes,
        );

        // Skip detector construction when there is nothing to detect: no
        // patterns, repetition off, volume off. Behavior is then identical
        // to pre-guard claudine on the streaming path.
        let detector = if compiled.is_empty()
            && !detector_config.repetition_enabled
            && !detector_config.volume_enabled
        {
            None
        } else {
            Some(ContentDetector::new(detector_config, compiled))
        };

        Ok(ResolvedRunawayGuards {
            detector,
            capture_volume_cap,
        })
    }

    /// Filter + compile the in-scope exit-expression set for a model,
    /// returning the compiled patterns alone (without rebuilding a whole
    /// detector).
    ///
    /// The streaming sink uses this to *re-scope* an already-running
    /// detector when a provider reports its actual model via `SessionStart`:
    /// it swaps the compiled set in place with
    /// [`ContentDetector::set_exit_expressions`], preserving the detector's
    /// volume / repetition state.
    ///
    /// ## Errors
    ///
    /// Same fail-closed contract as [`Self::compile_for_model`].
    pub(crate) fn compile_patterns_for_model(
        &self,
        model: Option<&str>,
    ) -> Result<CompiledExitExpressions> {
        let model = model.unwrap_or("");
        let inputs: Vec<_> = self
            .entries
            .iter()
            .filter(|entry| scope_matches(entry.scope.as_deref(), self.provider, model))
            .map(|entry| entry.to_input())
            .collect();
        CompiledExitExpressions::compile(&inputs)
    }
}

/// Resolve the runaway guards for a `(provider, model)` run.
///
/// Convenience wrapper over the two-phase pipeline: [`resolve_guard_inputs`]
/// (the expensive I/O + validation half) followed by
/// [`ResolvedGuardInputs::compile_for_model`] (the cheap scope-filter +
/// compile half) for the launch-time `model` hint.
///
/// `frontmatter` is the composed document frontmatter (composition paths)
/// or `None` (direct wrappers, which have no frontmatter layer).
///
/// ## Errors
///
/// Returns [`ClaudineError`] when any *present* config layer is broken: a
/// repo `.claudine/config.json` that exists but is unreadable/malformed, a
/// repo or frontmatter `exit_expressions` whose `scope` references an
/// unknown agent or whose regex is invalid, malformed frontmatter
/// `guard_settings`, or an in-scope set that fails to compile. A
/// genuinely-absent layer falls back to the built-in defaults without
/// error.
///
/// Production wrappers call the two-phase API directly (resolve the inputs,
/// then compile for a model) so they can also re-scope on the
/// provider-reported model. This single-shot convenience wrapper is retained
/// as the end-to-end fail-closed entry point exercised by the tests.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn resolve_runaway_guards(
    provider: Provider,
    model: Option<&str>,
    env_context: &EnvironmentContext,
    frontmatter: Option<&Value>,
) -> Result<ResolvedRunawayGuards> {
    let inputs = resolve_guard_inputs(provider, env_context, frontmatter)?;
    inputs.compile_for_model(model)
}

/// Phase 1 of guard resolution: load + validate the user/repo/frontmatter
/// guard config, fail closed on any present-but-broken layer, and return the
/// layer-merged [`ResolvedGuardInputs`] *before* scope filtering.
///
/// The model is deliberately not an argument here: this phase is
/// model-independent, so the result can be (re-)filtered for any model the
/// run later turns out to use (e.g. a provider-reported `SessionStart`
/// model) without repeating the I/O or validation.
///
/// ## Errors
///
/// Same present-but-broken contract as [`resolve_runaway_guards`], minus the
/// in-scope compile (which belongs to the per-model phase). A
/// genuinely-absent layer falls back to the built-in defaults without error.
pub(crate) fn resolve_guard_inputs(
    provider: Provider,
    env_context: &EnvironmentContext,
    frontmatter: Option<&Value>,
) -> Result<ResolvedGuardInputs> {
    let repo_root = env_context.git.as_ref().map(|g| g.repo_root.as_path());

    // User layer — load the user config only (no repo merge). A genuinely
    // absent `~/.claudine/config.json` is not an error (fall back to the
    // built-in guards), but a present-but-broken one aborts:
    // `load_claudine_config` already runs `ClaudineConfig::validate()`, so
    // an unknown scope agent or invalid regex here fails the run.
    let user = match loader::load_claudine_config(None, None) {
        Ok(config) => config,
        Err(ClaudineError::ConfigNotFound(_)) => Default::default(),
        Err(error) => return Err(error),
    };

    // Repo layer — the repo-scoped override, when a repo root is known. A
    // present-but-unreadable/malformed file propagates; an absent file is
    // `Ok(None)`. `load_repo_override_config` does not run config
    // validation, so the repo's `exit_expressions` are validated explicitly
    // below.
    //
    // Skip the repo layer when its path resolves to the same file as the
    // user config (e.g. HOME points at the repo root): that file is a
    // `ClaudineConfig`, not a `RepoOverrideConfig`, and re-reading it as the
    // wrong type would spuriously fail. Mirrors the same-file guard in
    // `load_claudine_config`'s repo merge.
    let repo = match repo_root {
        Some(root) => {
            let repo_path = root.join(".claudine/config.json");
            let same_file = repo_path
                .canonicalize()
                .ok()
                .zip(loader::user_config_path().canonicalize().ok())
                .is_some_and(|(repo, user)| repo == user);
            if same_file {
                None
            } else {
                loader::load_repo_override_config(&repo_path)?
            }
        }
        None => None,
    };
    if let Some(exit) = repo.as_ref().and_then(|r| r.exit_expressions.as_ref()) {
        validate_exit_expressions(exit.rules())?;
    }

    // Frontmatter layer — exit-expressions + guard-settings declared inline
    // on the prompt document. A malformed declaration aborts; an absent one
    // is `None`.
    let fm_map = frontmatter.and_then(Value::as_object);
    let fm_exit = match fm_map {
        Some(m) => extract_frontmatter_exit_expressions(m)?,
        None => None,
    };
    if let Some(exit) = fm_exit.as_ref() {
        validate_exit_expressions(exit.rules())?;
    }
    let fm_guards = match fm_map {
        Some(m) => extract_frontmatter_guard_settings(m)?,
        None => None,
    };

    let entries = resolve_exit_expressions(
        user.exit_expressions.as_ref(),
        repo.as_ref().and_then(|r| r.exit_expressions.as_ref()),
        fm_exit.as_ref(),
    );
    let guards = resolve_guard_settings(
        &user.guard_settings,
        repo.as_ref().and_then(|r| r.guard_settings.as_ref()),
        fm_guards.as_ref(),
    );

    Ok(ResolvedGuardInputs {
        provider,
        entries,
        guards,
    })
}

/// Whether a (possibly absent) `scope` string matches `(provider, model)`.
///
/// A `None`/empty scope is the global wildcard. An invalid scope (unknown
/// agent) never matches — config-load validation already rejects those, so
/// this is just defense-in-depth.
fn scope_matches(scope: Option<&str>, provider: Provider, model: &str) -> bool {
    match scope {
        None => true,
        Some(scope) => ScopeSelector::parse(scope)
            .map(|selector| selector.matches(provider, model))
            .unwrap_or(false),
    }
}

/// Extract a frontmatter-scoped `guard_settings` object, when present.
///
/// Mirrors [`extract_frontmatter_exit_expressions`] for the scalar guard
/// knobs.
///
/// ## Errors
///
/// Returns [`ClaudineError::ConfigValidation`] when `guard_settings` is
/// present but does not deserialize cleanly. A present-but-malformed value
/// aborts the run rather than being silently dropped — otherwise a typo'd
/// kill-switch would leave the lower layers' guards in force while the
/// author believes their override took effect. An absent key returns
/// `Ok(None)`.
fn extract_frontmatter_guard_settings(map: &Map<String, Value>) -> Result<Option<GuardSettings>> {
    let Some(value) = map.get("guard_settings") else {
        return Ok(None);
    };
    let parsed = serde_json::from_value(value.clone()).map_err(|error| {
        ClaudineError::ConfigValidationWithCause {
            message: format!("frontmatter `guard_settings` is invalid: {error}"),
            source: ConfigCause::Json(error),
        }
    })?;
    Ok(Some(parsed))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scope_matches_global_when_absent() {
        assert!(scope_matches(None, Provider::OpenCode, "any-model"));
    }

    #[test]
    fn scope_matches_agent_only() {
        assert!(scope_matches(Some("opencode"), Provider::OpenCode, "k2"));
        assert!(!scope_matches(Some("opencode"), Provider::Claude, "k2"));
    }

    #[test]
    fn scope_matches_agent_and_model_exactly() {
        assert!(scope_matches(
            Some("opencode/kimi-for-coding/k2p7"),
            Provider::OpenCode,
            "kimi-for-coding/k2p7",
        ));
        // Unknown model (`""`) never matches an agent/model scope.
        assert!(!scope_matches(
            Some("opencode/kimi-for-coding/k2p7"),
            Provider::OpenCode,
            "",
        ));
    }

    #[test]
    fn scope_unknown_agent_never_matches() {
        assert!(!scope_matches(Some("nonsense/k2"), Provider::OpenCode, "k2"));
    }

    #[test]
    fn extract_frontmatter_guard_settings_reads_partial_object() {
        let mut map = Map::new();
        map.insert(
            "guard_settings".to_string(),
            serde_json::json!({ "repetition": { "enabled": false } }),
        );
        let gs = extract_frontmatter_guard_settings(&map)
            .expect("valid")
            .expect("present");
        assert!(!gs.repetition.enabled);
        // Unset sub-fields keep their defaults.
        assert!(gs.volume.enabled);
    }

    #[test]
    fn extract_frontmatter_guard_settings_absent_is_none() {
        assert!(extract_frontmatter_guard_settings(&Map::new())
            .expect("absent is ok")
            .is_none());
    }

    #[test]
    fn extract_frontmatter_guard_settings_malformed_errors() {
        // Present but malformed (unknown field) must abort, not silently drop.
        let mut map = Map::new();
        map.insert(
            "guard_settings".to_string(),
            serde_json::json!({ "posture": "strict" }),
        );
        assert!(extract_frontmatter_guard_settings(&map).is_err());
    }

    // -------------------------------------------------------------------------
    // resolve_runaway_guards — fail-closed on present-but-invalid config
    // -------------------------------------------------------------------------

    use claudine::events::{EnvironmentContext, GitContext};

    /// An `EnvironmentContext` whose git repo root is `repo_root`, so the
    /// resolver loads `<repo_root>/.claudine/config.json` as the repo layer.
    /// `GitContext` has one required field (`repo_root`); the rest fill from
    /// their serde defaults.
    fn env_with_repo(repo_root: &std::path::Path) -> EnvironmentContext {
        let git: GitContext = serde_json::from_value(serde_json::json!({
            "repo_root": repo_root,
        }))
        .unwrap();
        EnvironmentContext {
            git: Some(git),
            ..Default::default()
        }
    }

    fn write_repo_config(repo_root: &std::path::Path, body: &str) {
        let dir = repo_root.join(".claudine");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("config.json"), body).unwrap();
    }

    /// Point `HOME` at an empty dir so the user layer is genuinely absent
    /// (`ConfigNotFound` → built-in defaults) and the test exercises only
    /// the repo / frontmatter layer regardless of the host's real
    /// `~/.claudine/config.json`. Caller must hold the guard for the test's
    /// lifetime and be `#[serial]`.
    fn isolate_empty_home() -> (tempfile::TempDir, test_toolkit::EnvGuard) {
        let home = tempfile::tempdir().unwrap();
        let guard = test_toolkit::EnvGuard::set_safe("HOME", home.path().to_str().unwrap());
        (home, guard)
    }

    #[test]
    #[serial_test::serial]
    fn resolve_repo_unknown_scope_agent_aborts() {
        let (_home, _guard) = isolate_empty_home();
        let tmp = tempfile::tempdir().unwrap();
        write_repo_config(
            tmp.path(),
            r#"{ "exit_expressions": [{ "pattern": "x", "scope": "nonsense/k2" }] }"#,
        );
        let env = env_with_repo(tmp.path());
        let result = resolve_runaway_guards(Provider::OpenCode, Some("k2"), &env, None);
        assert!(result.is_err(), "unknown scope agent must abort");
    }

    #[test]
    #[serial_test::serial]
    fn resolve_repo_invalid_regex_aborts() {
        let (_home, _guard) = isolate_empty_home();
        let tmp = tempfile::tempdir().unwrap();
        write_repo_config(
            tmp.path(),
            r#"{ "exit_expressions": [{ "pattern": "[", "kind": "regex" }] }"#,
        );
        let env = env_with_repo(tmp.path());
        let result = resolve_runaway_guards(Provider::OpenCode, Some("k2"), &env, None);
        assert!(result.is_err(), "invalid regex must abort");
    }

    #[test]
    #[serial_test::serial]
    fn resolve_repo_unreadable_config_aborts() {
        let (_home, _guard) = isolate_empty_home();
        let tmp = tempfile::tempdir().unwrap();
        // Present but not valid JSON5 → load_repo_override_config errors.
        write_repo_config(tmp.path(), "{ not valid json ");
        let env = env_with_repo(tmp.path());
        let result = resolve_runaway_guards(Provider::OpenCode, Some("k2"), &env, None);
        assert!(result.is_err(), "malformed repo config must abort");
    }

    #[test]
    #[serial_test::serial]
    fn resolve_frontmatter_unknown_scope_agent_aborts() {
        let (_home, _guard) = isolate_empty_home();
        let env = EnvironmentContext::default();
        let frontmatter = serde_json::json!({
            "exit_expressions": [{ "pattern": "x", "scope": "nonsense/k2" }]
        });
        let result =
            resolve_runaway_guards(Provider::OpenCode, Some("k2"), &env, Some(&frontmatter));
        assert!(result.is_err(), "unknown frontmatter scope must abort");
    }

    #[test]
    #[serial_test::serial]
    fn resolve_frontmatter_invalid_regex_aborts() {
        let (_home, _guard) = isolate_empty_home();
        let env = EnvironmentContext::default();
        let frontmatter = serde_json::json!({
            "exit_expressions": [{ "pattern": "[", "kind": "regex" }]
        });
        let result =
            resolve_runaway_guards(Provider::OpenCode, Some("k2"), &env, Some(&frontmatter));
        assert!(result.is_err(), "invalid frontmatter regex must abort");
    }

    #[test]
    #[serial_test::serial]
    fn resolve_frontmatter_malformed_guard_settings_aborts() {
        let (_home, _guard) = isolate_empty_home();
        let env = EnvironmentContext::default();
        let frontmatter = serde_json::json!({
            "guard_settings": { "posture": "strict" }
        });
        let result =
            resolve_runaway_guards(Provider::OpenCode, Some("k2"), &env, Some(&frontmatter));
        assert!(result.is_err(), "malformed guard_settings must abort");
    }

    #[test]
    #[serial_test::serial]
    fn resolve_absent_config_falls_back_to_builtin_defaults() {
        // No repo (no git context), no frontmatter. The user layer points at
        // a non-existent home so `load_claudine_config` returns
        // `ConfigNotFound` → built-in defaults, not an error.
        let (_home, _guard) = isolate_empty_home();
        let env = EnvironmentContext::default();
        let resolved = resolve_runaway_guards(Provider::OpenCode, Some("k2"), &env, None)
            .expect("absent config must succeed with built-in defaults");
        // Built-in defaults keep both guards enabled, so a detector is armed.
        assert!(resolved.detector.is_some());
        assert!(resolved.capture_volume_cap.enabled);
    }

    // -------------------------------------------------------------------------
    // Two-phase re-scope — an agent/model-scoped entry only compiles into the
    // pattern set once the matching model is supplied (Review 2 High #2).
    // -------------------------------------------------------------------------

    /// Guard inputs carrying a single agent/model-scoped exit expression
    /// declared in repo config. Built once, then filtered for different
    /// models to prove the cheap re-scope phase.
    fn agent_model_scoped_inputs(tmp: &std::path::Path) -> ResolvedGuardInputs {
        write_repo_config(
            tmp,
            r#"{ "exit_expressions": [{ "pattern": "k2-only-stop", "scope": "opencode/k2p7" }] }"#,
        );
        let env = env_with_repo(tmp);
        resolve_guard_inputs(Provider::OpenCode, &env, None).expect("inputs resolve")
    }

    #[test]
    #[serial_test::serial]
    fn compile_without_model_excludes_agent_model_scope() {
        let (_home, _guard) = isolate_empty_home();
        let tmp = tempfile::tempdir().unwrap();
        let inputs = agent_model_scoped_inputs(tmp.path());

        // No launch-time model hint: the agent/model scope cannot match yet,
        // so the compiled set is empty.
        let patterns = inputs.compile_patterns_for_model(None).expect("compiles");
        assert!(
            patterns.matches_line("k2-only-stop").is_none(),
            "agent/model scope must be inactive without a matching model"
        );
    }

    #[test]
    #[serial_test::serial]
    fn compile_with_matching_model_includes_agent_model_scope() {
        let (_home, _guard) = isolate_empty_home();
        let tmp = tempfile::tempdir().unwrap();
        let inputs = agent_model_scoped_inputs(tmp.path());

        // The provider-reported model brings the scoped entry into scope.
        let patterns = inputs
            .compile_patterns_for_model(Some("k2p7"))
            .expect("compiles");
        assert!(
            patterns.matches_line("k2-only-stop").is_some(),
            "agent/model scope must be active for the matching model"
        );
    }

    #[test]
    #[serial_test::serial]
    fn compile_with_non_matching_model_excludes_agent_model_scope() {
        let (_home, _guard) = isolate_empty_home();
        let tmp = tempfile::tempdir().unwrap();
        let inputs = agent_model_scoped_inputs(tmp.path());

        // A different model never matches the agent/model scope.
        let patterns = inputs
            .compile_patterns_for_model(Some("some-other-model"))
            .expect("compiles");
        assert!(
            patterns.matches_line("k2-only-stop").is_none(),
            "agent/model scope must stay inactive for a non-matching model"
        );
    }
}
