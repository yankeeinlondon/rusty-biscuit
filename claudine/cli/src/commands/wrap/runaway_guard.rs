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
//! Resolution is best-effort: a missing or unreadable config never aborts a
//! run, it just falls back to the built-in defaults (guards enabled with
//! conservative thresholds, empty exit-expression set). Validation of
//! `exit_expressions` already happened at config-load (Phase 3), so any
//! residual compile error here defaults to an empty set rather than killing
//! the run.

use claudine::dispatch::loader;
use claudine::events::EnvironmentContext;
use claudine::provider::Provider;
use claudine::runaway::{
    extract_frontmatter_exit_expressions, resolve_exit_expressions, resolve_guard_settings,
    CaptureVolumeCap, CompiledExitExpressions, ContentDetector, DetectorConfig, GuardSettings,
    ScopeSelector,
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

/// Resolve the runaway guards for a `(provider, model)` run.
///
/// `frontmatter` is the composed document frontmatter (composition paths)
/// or `None` (direct wrappers, which have no frontmatter layer).
pub(crate) fn resolve_runaway_guards(
    provider: Provider,
    model: Option<&str>,
    env_context: &EnvironmentContext,
    frontmatter: Option<&Value>,
) -> ResolvedRunawayGuards {
    let repo_root = env_context.git.as_ref().map(|g| g.repo_root.as_path());

    // User layer — load the user config only (no repo merge); default on
    // absence so a host with no `~/.claudine/config.json` still gets the
    // built-in guards.
    let user = loader::load_claudine_config(None, None).unwrap_or_default();

    // Repo layer — the repo-scoped override, when a repo root is known.
    let repo = repo_root.and_then(|root| {
        loader::load_repo_override_config(&root.join(".claudine/config.json"))
            .ok()
            .flatten()
    });

    // Frontmatter layer — exit-expressions + guard-settings declared inline
    // on the prompt document.
    let fm_map = frontmatter.and_then(Value::as_object);
    let fm_exit = fm_map.and_then(|m| extract_frontmatter_exit_expressions(m).ok().flatten());
    let fm_guards = fm_map.and_then(extract_frontmatter_guard_settings);

    let resolved_entries = resolve_exit_expressions(
        user.exit_expressions.as_ref(),
        repo.as_ref().and_then(|r| r.exit_expressions.as_ref()),
        fm_exit.as_ref(),
    );
    let guards = resolve_guard_settings(
        &user.guard_settings,
        repo.as_ref().and_then(|r| r.guard_settings.as_ref()),
        fm_guards.as_ref(),
    );

    // Filter the resolved exit-expression set to the entries whose scope
    // matches this run's `(provider, model)`. An unknown model (`None`) is
    // treated as `""`: global and agent scopes still match, but
    // agent/model scopes do not (conservative — never a wrongful match).
    let model = model.unwrap_or("");
    let inputs: Vec<_> = resolved_entries
        .iter()
        .filter(|entry| scope_matches(entry.scope.as_deref(), provider, model))
        .map(|entry| entry.to_input())
        .collect();

    // Compile once before streaming (never per line). Validation ran at
    // config-load, so default to an empty set on the (unexpected) error.
    let compiled = CompiledExitExpressions::compile(&inputs).unwrap_or_else(|error| {
        tracing::warn!(%provider, "exit-expression compile failed at wiring; disabling set: {error}");
        CompiledExitExpressions::empty()
    });

    let detector_config = DetectorConfig {
        repetition_enabled: guards.repetition.enabled,
        max_repeats: guards.repetition.max_repeats,
        max_cycle_length: guards.repetition.max_cycle_length,
        volume_enabled: guards.volume.enabled,
        max_lines: guards.volume.max_lines,
        max_bytes: guards.volume.max_bytes,
    };

    let capture_volume_cap = CaptureVolumeCap::new(
        guards.volume.enabled,
        guards.volume.max_lines,
        guards.volume.max_bytes,
    );

    // Skip detector construction when there is nothing to detect: no
    // patterns, repetition off, volume off. Behavior is then identical to
    // pre-guard claudine on the streaming path.
    let detector = if compiled.is_empty()
        && !detector_config.repetition_enabled
        && !detector_config.volume_enabled
    {
        None
    } else {
        Some(ContentDetector::new(detector_config, compiled))
    };

    ResolvedRunawayGuards {
        detector,
        capture_volume_cap,
    }
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
/// knobs. A malformed value is dropped (returns `None`) rather than failing
/// the run — the lower layers still apply.
fn extract_frontmatter_guard_settings(map: &Map<String, Value>) -> Option<GuardSettings> {
    let value = map.get("guard_settings")?;
    serde_json::from_value(value.clone()).ok()
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
        let gs = extract_frontmatter_guard_settings(&map).expect("present");
        assert!(!gs.repetition.enabled);
        // Unset sub-fields keep their defaults.
        assert!(gs.volume.enabled);
    }

    #[test]
    fn extract_frontmatter_guard_settings_absent_is_none() {
        assert!(extract_frontmatter_guard_settings(&Map::new()).is_none());
    }
}
