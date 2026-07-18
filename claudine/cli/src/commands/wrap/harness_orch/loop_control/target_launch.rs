//! R6/R8 — document-dependent launch state is rebuilt from the document that is
//! about to run.
//!
//! Launch decisions must be recomputed from the frontmatter of the document the
//! attempt will actually execute, not inherited from whatever produced the
//! previous attempt. Two callers need that:
//!
//! - **Proxy adoption (R6).** A hand-off adopts a new active document, so its
//!   launch identity comes from the *target's* own frontmatter plus the
//!   immutable invocation intent, not from the router that pointed at it.
//! - **Every retry/resume fresh-read boundary (R8).** Those boundaries
//!   re-materialize the active document from disk. If the refreshed frontmatter
//!   resolves to a different launch identity, the attempt must launch with the
//!   refreshed one — and, for `resume`, that difference is exactly what the
//!   session-compatibility key has to see in order to refuse mixing a live
//!   session with a newly prepared launch plan.
//!
//! Both paths share [`rebuild_launch_env`]; only proxy adoption additionally
//! needs the early-binding context, so [`rebuild_target_launch`] wraps it.
//!
//! ## Scope
//!
//! The rebuild recomputes **launch identity** — provider/model selection
//! (honoring explicit CLI precedence) projected into the `AGENT`/`MODEL`/`YOLO`
//! environment — and propagates it to the provider child's environment (via the
//! materialized prompt's `env_overrides`, which
//! [`super::super::build_harness_launch`] overlays onto the base env). Proxy
//! adoption also installs the rebuilt early-binding context on the guard, so
//! `env.MODEL`/`ctx.model` in the target's stacks resolve to its own identity.
//!
//! ## This is the in-harness fallback, not the composition-command path
//!
//! [`rebuild_target_launch`] runs only when a handoff has **no** run ledger to
//! surface to — see `super::surface_or_adopt_terminal_proxy`, which adopts
//! in-harness only in that case. Every composition command (`compose`,
//! `inline-compose`, `sequence`) carries a ledger, so its handoffs surface to the
//! command-owned coordinator instead, and the target is re-prepared as a fresh
//! document through `commands::compose::prep::prepare_and_run_active_document` —
//! which re-enters the production selection/MCP/argv pipeline and therefore
//! rebuilds profile/binary sub-selection, the argv entrypoint, MCP runtime
//! injection, child CWD, and system-prompt delivery from the target's own
//! frontmatter.
//!
//! This env-only overlay is consequently a **fallback** scope, and its limits are
//! limits of that fallback alone: it does not re-select profile/binary, argv
//! entrypoint, or MCP runtime injection, so on this path those stay as the
//! invocation resolved them. They diverge only when a proxy changes the provider
//! itself; explicit CLI provider selection — the common case on this path — pins
//! it.
//!
//! [`rebuild_launch_env`] is the shared half and *is* on the hot path for every
//! retry/resume fresh read regardless of route, which is what lets `model` move
//! across a canonical refresh and makes the AC15 refusal reachable. The other
//! compatibility facets are argv-derived or CLI-resolved and cannot move across a
//! same-document refresh on either path — see `super::super::session_key`.

use std::path::Path;

use claudine::model_catalog::ModelCatalogService;
use claudine::provider::Provider;

use super::super::MaterializedHarnessPrompt;

/// The recomputed launch identity for a freshly adopted proxy target.
pub(super) struct TargetLaunchRebuild {
    /// `AGENT`/`MODEL`/`YOLO` overlays derived from the target's own resolved
    /// provider/model. Applied to every subsequent attempt's child environment
    /// so a real provider process for the target launches with the target's
    /// configuration.
    pub(super) env_overrides: Vec<(String, String)>,
    /// The target's early-binding context, seeded with the rebuilt identity, for
    /// installation on the lifecycle guard via
    /// [`claudine::composition::LifecycleRunGuard::set_proxy_prepared_context`].
    pub(super) prepared_context: darkmatter::markdown::compose::ComposeContext,
}

/// Recompute the `AGENT`/`MODEL`/`YOLO` launch-identity overlays from a
/// prepared document's own selection hints.
///
/// This is the whole of the rebuild that the per-attempt fresh-read boundary
/// needs; proxy adoption calls it through [`rebuild_target_launch`], which adds
/// the early-binding context. Splitting them keeps the retry/resume path off the
/// context capture it has no consumer for.
///
/// `cli_model` is the explicit `--model`, which stays authoritative over any
/// frontmatter `model:`. The shape mirrors `install_agent_env_for_composition`
/// exactly — `AGENT` always, `MODEL` only when a model resolved, `YOLO` always —
/// because that identity is what a direct invocation of the same document at the
/// same moment would install, and equality with it is the contract.
pub(super) fn rebuild_launch_env(
    provider: Provider,
    cli_model: Option<&str>,
    yolo: bool,
    repo_root: Option<&Path>,
    document: &MaterializedHarnessPrompt,
) -> Vec<(String, String)> {
    let selection_config =
        crate::commands::wrap::composition::load_selection_config_for_repo(repo_root);
    let catalog = match &selection_config {
        Some(cfg) => ModelCatalogService::with_overrides(cfg.model_overrides.clone()),
        None => ModelCatalogService::new(),
    };
    let (model, _model_reason) = claudine::composition::resolve_model_with_hints(
        provider,
        &document.selection_hints,
        cli_model,
        Some(&catalog),
    );

    let mut env_overrides: Vec<(String, String)> = Vec::new();
    env_overrides.push(("AGENT".to_string(), provider.as_slug().to_string()));
    if let Some(model) = &model {
        env_overrides.push(("MODEL".to_string(), model.clone()));
    }
    env_overrides.push(("YOLO".to_string(), yolo.to_string()));
    env_overrides
}

/// Recompute a proxied target's launch identity from its own frontmatter.
///
/// `provider` is the run's resolved provider (pinned by an explicit CLI flag in
/// the common case). `cli_model` is the explicit `--model`, which stays
/// authoritative over any target frontmatter `model:`. The model is re-resolved
/// against the target's own [`EffectiveSelectionHints`] with the same catalog
/// validation a direct invocation performs, so a target that pins its own valid
/// `model:` resolves it, and an invalid one falls back exactly as it would
/// directly.
pub(super) fn rebuild_target_launch(
    provider: Provider,
    cli_model: Option<&str>,
    yolo: bool,
    repo_root: Option<&Path>,
    launch_area: &Path,
    target: &MaterializedHarnessPrompt,
) -> TargetLaunchRebuild {
    let env_overrides = rebuild_launch_env(provider, cli_model, yolo, repo_root, target);

    // Rebuild the early-binding context the same way the pipeline builds the
    // router's lifecycle context (see `construct_lifecycle_runtime`): capture at
    // the launch area against the target's own frontmatter+body, then overlay the
    // rebuilt identity so `env.AGENT`/`env.MODEL`/`env.YOLO` resolve to it.
    let fm_json = serde_json::to_string(&target.frontmatter).unwrap_or_default();
    let scan = format!("{fm_json}\n{}", target.prompt);
    let mut prepared_context =
        darkmatter::markdown::compose::ComposeContext::capture_for_content(launch_area, &scan);
    for (key, value) in &env_overrides {
        prepared_context.env_mut().insert(key.clone(), value.clone());
    }

    TargetLaunchRebuild {
        env_overrides,
        prepared_context,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use claudine::composition::{EffectiveSelectionHints, ModelHint};

    /// Build a materialized prompt carrying a target's `model:` hint plus a
    /// matching effective-frontmatter object, as canonical preparation would.
    fn target_with_model(model: Option<&str>) -> MaterializedHarnessPrompt {
        let (selection_hints, frontmatter) = match model {
            Some(model) => (
                EffectiveSelectionHints {
                    model: Some(ModelHint::Single(model.to_string())),
                    ..EffectiveSelectionHints::default()
                },
                serde_json::json!({ "model": model }),
            ),
            None => (
                EffectiveSelectionHints::default(),
                serde_json::json!({ "title": "no model" }),
            ),
        };
        MaterializedHarnessPrompt {
            live_frontmatter: MaterializedHarnessPrompt::live_cell_from(&frontmatter),
            frontmatter,
            prompt: "body".to_string(),
            env_overrides: Vec::new(),
            selection_hints,
            inline_closure_plan: None,
            lifecycle: None,
        }
    }

    fn value_of<'a>(overrides: &'a [(String, String)], key: &str) -> Option<&'a str> {
        overrides
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }

    /// A target that pins its own `model:` resolves it into the rebuilt launch
    /// identity — the `AGENT`/`MODEL`/`YOLO` env AND the early-binding context
    /// that lifecycle `env.*` reads. This is the launch facet the L2 pinned-model
    /// equivalence test observes through the target's `success` stack.
    #[test]
    fn rebuild_projects_target_model_into_env_and_context() {
        // A namespaced local-runner id is catalog-valid by construction, so this
        // is independent of the host's live model listings.
        let target = target_with_model(Some("llamacpp/probe-model-x"));
        let rebuild = rebuild_target_launch(
            Provider::Goose,
            None,
            false,
            None,
            Path::new("."),
            &target,
        );

        assert_eq!(value_of(&rebuild.env_overrides, "AGENT"), Some("goose"));
        assert_eq!(
            value_of(&rebuild.env_overrides, "MODEL"),
            Some("llamacpp/probe-model-x"),
            "the target's own pinned model must reach the launch env",
        );
        assert_eq!(value_of(&rebuild.env_overrides, "YOLO"), Some("false"));
        assert_eq!(
            rebuild.prepared_context.env().get("MODEL").map(String::as_str),
            Some("llamacpp/probe-model-x"),
            "the lifecycle early-binding context must carry the target's model",
        );
    }

    /// Explicit `--model` is immutable invocation intent: it stays authoritative
    /// over the target's frontmatter `model:` exactly as it would for a direct
    /// invocation under the same CLI arguments.
    #[test]
    fn rebuild_keeps_explicit_cli_model_authoritative() {
        let target = target_with_model(Some("llamacpp/probe-model-x"));
        let rebuild = rebuild_target_launch(
            Provider::Goose,
            Some("llamacpp/cli-pinned"),
            true,
            None,
            Path::new("."),
            &target,
        );

        assert_eq!(
            value_of(&rebuild.env_overrides, "MODEL"),
            Some("llamacpp/cli-pinned"),
            "explicit --model must win over the target frontmatter model",
        );
        assert_eq!(value_of(&rebuild.env_overrides, "YOLO"), Some("true"));
    }

    /// A target with no `model:` produces no `MODEL` overlay (matching
    /// `install_agent_env_for_composition`), so the child env falls through to
    /// the provider default rather than a stale router model.
    #[test]
    fn rebuild_omits_model_when_target_pins_none() {
        let target = target_with_model(None);
        let rebuild = rebuild_target_launch(
            Provider::Goose,
            None,
            false,
            None,
            Path::new("."),
            &target,
        );

        assert_eq!(value_of(&rebuild.env_overrides, "AGENT"), Some("goose"));
        assert_eq!(
            value_of(&rebuild.env_overrides, "MODEL"),
            None,
            "no frontmatter model means no MODEL overlay",
        );
    }
}
