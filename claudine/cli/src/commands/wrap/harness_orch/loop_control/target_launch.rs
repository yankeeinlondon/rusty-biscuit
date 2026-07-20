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
//! Both paths share [`rebuild_launch_identity`]; [`rebuild_target_launch`] adds
//! the early-binding context proxy adoption needs.
//!
//! ## One rebuild per attempt
//!
//! [`rebuild_launch_identity`] runs **once** per attempt, at the fresh-read
//! boundary that produces the document, and its result is paired with that
//! document (`AttemptDocument`). The attempt previously rebuilt twice — an
//! env-only projection for the child environment and a second, independently
//! computed one for the compatibility key — which meant two values that were
//! only equal by convention. Now the child-environment overlay and the key are
//! both derived from the one bundle, so the key cannot describe a different
//! environment than the child receives.
//!
//! That one bundle is also what the child is spawned with — argv, profile, and
//! MCP injection included. See "The launch this rebuild reaches" below.
//!
//! ## Scope
//!
//! [`rebuild_launch_identity`] recomputes every launch property a resume cannot
//! renegotiate that a document can move: provider (frontmatter `agent:` under
//! explicit-CLI precedence), the profile and binary that provider selects, the
//! resume protocol that profile supports, model (frontmatter `model:` under
//! explicit `--model`), interactivity (frontmatter `interactive:`), the
//! permission mode `--yolo` actually achieves in that mode for that provider,
//! the structured-output mode the pair implies, and the MCP tag set lexed from
//! the refreshed body. It is the input both sides of the R8 session-compatibility
//! comparison are derived from — see [`super::super::session_key`].
//!
//! ### Why a deterministic function of the document is the right contract
//!
//! The compatibility key is only ever compared **key against key across two
//! attempts of the same loop**, both computed by this code. It is never compared
//! against a value the invocation pipeline resolved. So the rebuild has to be a
//! deterministic function of (refreshed document, immutable invocation intent) —
//! it does not have to reproduce the invocation pipeline bit for bit. An
//! unchanged document therefore yields an identical key and cannot produce a
//! false refusal, while any document mutation that moves a facet is named.
//!
//! ### Facets deliberately outside the rebuild
//!
//! R8 defines exactly two of the key's facets as *immutable invocation inputs*:
//! no same-document resume can move them, so the rebuild has nothing to
//! recompute and the key reads the invocation's own value.
//!
//! - **Workspace/child CWD** is resolved from the process's launch directory
//!   before any document is read. The complete set of document surfaces over
//!   launch identity is `agent:`/`model:`/`interactive:` — none names a
//!   directory — and the resolver deliberately prefers the launch repository
//!   over the document's own, so even a proxy into a sibling clone leaves the
//!   child where it started (`composition::prep_context::tests::
//!   prep_context_launch_workspace_split_contract_unit`).
//! - **System-prompt *content*** is composed once, at invocation, and captured.
//!   A document has no `system_prompt:` surface, and rewriting the discovered
//!   `system-prompt.md` between attempts moves neither delivery path — proven by
//!   `launch_plan::tests::rewriting_the_discovered_system_prompt_moves_no_delivered_content`.
//!   Its *delivery* is not in this bucket: it is rebuilt per provider (see
//!   [`super::super::super::launch_plan`]), so it moves exactly when the
//!   provider facet moves, which already refuses.
//!
//! ## The launch this rebuild reaches
//!
//! The rebuilt bundle is the launch. It carries the provider, profile, and
//! binary the harness spawns, the provider argv, the session mode and
//! structured-output shape, the permission mode, and the MCP runtime injection —
//! and the compatibility key is computed from that same value. It also owns
//! every provider-dependent *execution adapter* the attempt runs with: the
//! stdout/stderr noise-prefix policy and structured-stderr suppression of the
//! rebuilt profile, the Codex `--output-last-message` artifact (present iff the
//! rebuilt plan captures through it), and the composition dispatch context with
//! its provider/model selection metadata. There is no invocation-fixed launch
//! state left for the key — or the attempt's own execution — to disagree with.
//!
//! Argv and MCP injection come from [`super::super::super::launch_plan`], a
//! re-entrant builder the invocation feeds once with the results of every side
//! effect it performed (temp files, shadow HOME, ambiguity resolutions). A
//! rebuild whose facets match the invocation's gets that recorded plan back
//! verbatim, so an unchanged document is byte-identical to the invocation by
//! construction; a rebuild that moved a facet replays the producers around the
//! recorded invocation-fixed slices.
//!
//! ### Proxy adoption — the target's full bundle is built above the harness
//!
//! [`rebuild_target_launch`] runs for a target the **command coordinator**
//! already re-prepared. Every route that reaches it carries a run ledger, so the
//! handoff surfaced up and the target was rebuilt as a fresh document through
//! `commands::compose::prep::prepare_and_run_active_document` — which re-enters
//! the production selection/MCP/argv pipeline from the target's own frontmatter.
//! What is rebuilt here is the remainder: the lifecycle early-binding context and
//! the `AGENT`/`MODEL`/`YOLO` env for the target's staged bootstrap.
//!
//! A handoff with no ledger to surface to has no such coordinator, and is
//! therefore **refused** rather than adopted against a borrowed bundle — see
//! `super::surface_or_adopt_terminal_proxy`. There is no reduced launch path.
//!
//! ### Retry and resume, which differ on purpose
//!
//! At a same-document fresh-read boundary there is no coordinator re-entry, so
//! this rebuild is the whole story — and the two verbs want opposite things
//! from it.
//!
//! A **resume** reuses a live provider session the old plan opened, so a moved
//! facet is a genuine conflict: it is **refused** with the facets named, rather
//! than run against a plan the session was not opened with.
//!
//! A **retry** opens a *fresh* session, so there is nothing to conflict with. It
//! simply launches under the rebuilt plan: a document that changes `agent:`,
//! `interactive:`, permission mode, or its MCP `#tag` set before a retry gets a
//! child spawned from the refreshed plan, not the invocation's. That is what
//! R8's "one coherent prepared document" asks for, and what review-8 finding 2
//! reported missing.
//!
//! Only where a path recorded no replay inputs (the direct provider wrappers,
//! whose facets are pinned by invocation intent anyway) does a moved facet
//! refuse instead — assembling a plan from empty slices would silently drop the
//! MCP injection or system prompt the invocation performed.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use claudine::composition::{
    AgentHint, InstalledProviderSnapshot, ModelResolutionReason, ProviderResolutionReason,
};
use claudine::model_catalog::ModelCatalogService;
use claudine::provider::Provider;

use super::super::super::launch_plan;
use super::super::super::profile::{WrapperProfile, profile_for_provider};

use super::super::MaterializedHarnessPrompt;

/// The immutable invocation intent every per-attempt launch rebuild resolves
/// the refreshed document against.
///
/// "Immutable" is the R8 sense: these are the caller's own decisions, which stay
/// authoritative at every document, retry, and resume of the invocation. Only
/// the document half of the rebuild moves.
#[derive(Clone)]
pub(crate) struct LaunchRebuildIntent {
    /// Explicit `--claude` / `--codex` / … — authoritative over any frontmatter
    /// `agent:`, exactly as it is for a direct invocation.
    pub(crate) explicit_provider: Option<Provider>,
    /// The provider the invocation resolved, used when neither an explicit flag
    /// nor the refreshed document's `agent:` names one.
    pub(crate) fallback_provider: Provider,
    /// The binary path the invocation resolved for [`Self::fallback_provider`].
    /// Reused verbatim when the rebuild lands on that same provider, so an
    /// unchanged document reproduces the invocation's own binary identity.
    pub(crate) fallback_binary: PathBuf,
    /// Providers detected as installed at invocation, used to pick from a
    /// list-valued `agent:` hint and to resolve a rebuilt provider's binary.
    pub(crate) installed_snapshot: Option<InstalledProviderSnapshot>,
    /// Interactivity for a refreshed document that declares no `interactive:`.
    pub(crate) default_non_interactive: bool,
    /// Whether `--yolo` was requested. Whether it is *achieved* depends on the
    /// rebuilt provider and mode, which is what the permission facet records.
    pub(crate) cli_yolo: bool,
    /// `inline-compose` prepares Codex structured output even for an
    /// interactive session; mirrors `prepare_codex_structured_output`'s
    /// `inline_interactive` input.
    pub(crate) is_inline: bool,
    /// Whether MCP is in play at all (`--mcp` or a non-empty `--mcp-use`). With
    /// MCP off no document tag participates in the launch, so none may move the
    /// MCP facet either.
    pub(crate) mcp_enabled: bool,
    /// Why the invocation selected [`Self::fallback_provider`]. Reused when a
    /// replayed rebuild falls back to that provider (no explicit flag, no
    /// frontmatter `agent:`), because the rebuild cannot re-derive
    /// picker/favorite provenance from the document alone.
    pub(crate) fallback_provider_reason: ProviderResolutionReason,
    /// The invocation's composition dispatch context. An unchanged document
    /// keeps it verbatim; a rebuild that moved a facet overwrites its
    /// provider/model selection entries from the refreshed facets. Empty for
    /// the direct wrapper passthrough, which publishes no composition
    /// metadata — and whose moved facets refuse before a recompute anyway.
    pub(crate) dispatch_context: HashMap<String, serde_json::Value>,
    /// The invocation-fixed half of the re-entrant launch-plan builder: every
    /// effect the command phase performed once, recorded so a per-attempt
    /// rebuild can re-derive argv and the environment overlay without repeating
    /// any of them. See [`crate::commands::wrap::launch_plan`].
    pub(crate) launch_plan_inputs: crate::commands::wrap::launch_plan::LaunchPlanInputs,
}

/// The launch properties a resume cannot renegotiate, recomputed from one
/// refreshed read of the active document.
///
/// Built by [`rebuild_launch_identity`]. Both sides of the R8 compatibility
/// comparison are derived from this, so a document mutation that moves any field
/// here is a named incompatible facet on the next resume.
#[derive(Clone)]
pub(crate) struct RebuiltLaunchIdentity {
    pub(crate) provider: Provider,
    pub(crate) profile: &'static dyn WrapperProfile,
    pub(crate) binary_path: PathBuf,
    pub(crate) non_interactive: bool,
    /// Whether the provider's bypass mechanism actually takes effect in this
    /// mode — not merely whether `--yolo` was asked for.
    pub(crate) yolo: bool,
    pub(crate) use_structured: bool,
    /// The Codex `--output-last-message` artifact, present iff the rebuilt
    /// plan carries Codex structured-output capture. Wraps the same recorded
    /// sink path the rebuilt argv names, so a retry that lands on Codex can
    /// recover the final message from the file its argv points at — and a
    /// retry that leaves Codex drops the artifact with the flag.
    pub(crate) codex_output: Option<crate::commands::wrap::policy::StructuredCodexOutput>,
    /// MCP tags lexed from the refreshed body, sorted and deduped. Empty when
    /// MCP is not enabled for the invocation.
    pub(crate) mcp_tags: Vec<String>,
    /// Stdout noise prefixes for the rebuilt profile in the rebuilt session
    /// mode (empty when interactive, matching the invocation's own gate).
    pub(crate) stdout_noise: &'static [&'static str],
    /// Stderr noise prefixes, same derivation as [`Self::stdout_noise`].
    pub(crate) stderr_noise: &'static [&'static str],
    /// The rebuilt profile's structured-stderr suppression policy.
    pub(crate) suppress_stderr_on_success: bool,
    /// The dispatch context this attempt publishes with its provider events.
    /// The invocation's context verbatim for an unchanged document; the
    /// provider/model selection entries are recomputed from the refreshed
    /// facets when a facet moved.
    pub(crate) dispatch_context: HashMap<String, serde_json::Value>,
    /// `AGENT`/`MODEL`/`YOLO` overlays for the provider child's environment.
    pub(crate) env_overrides: Vec<(String, String)>,
    /// The provider argv this attempt launches with, rebuilt from the refreshed
    /// document. Identical to the invocation's own argv whenever the document
    /// moved no launch facet — see [`crate::commands::wrap::launch_plan`].
    pub(crate) args: Vec<String>,
    /// The environment patch the rebuilt plan contributes (MCP runtime
    /// injection, the OpenCode inline config, the permission mode, and removals
    /// for provider-shaped keys the refreshed document no longer wants).
    /// Applied beneath [`Self::env_overrides`], which stays authoritative for
    /// `AGENT`/`MODEL`/`YOLO`.
    pub(crate) launch_env: Vec<launch_plan::EnvChange>,
}

/// Recompute the full launch identity from one refreshed read of a document.
///
/// See the module docs for why this is a deterministic function of the document
/// rather than a replay of the invocation pipeline.
pub(crate) fn rebuild_launch_identity(
    intent: &LaunchRebuildIntent,
    cli_model: Option<&str>,
    repo_root: Option<&Path>,
    document: &MaterializedHarnessPrompt,
    source_path: Option<&Path>,
) -> Result<RebuiltLaunchIdentity, crate::commands::wrap::launch_plan::LaunchPlanError> {
    let snapshot = intent.installed_snapshot.as_ref();
    let provider = intent
        .explicit_provider
        .or_else(|| provider_from_hints(&document.selection_hints, snapshot))
        .unwrap_or(intent.fallback_provider);
    let profile = profile_for_provider(provider)
        .or_else(|| profile_for_provider(intent.fallback_provider))
        .expect("the invocation's own provider always has a wrapper profile");
    let binary_path = resolve_binary_for(provider, profile, intent);

    // Frontmatter `interactive:` is the only document surface over session mode;
    // absent (or explicitly null) it keeps whatever the invocation resolved.
    let non_interactive = document
        .selection_hints
        .interactive
        .map_or(intent.default_non_interactive, |interactive| !interactive);

    // Lexed from the document on disk, not from `document.prompt`: a resume
    // attempt deliberately substitutes the follow-up message for the composed
    // body (R8), so the prompt field carries the follow-up rather than the text
    // whose `#tag`s select servers. Reading the source is also what keeps the
    // signal stable for an unchanged document — same bytes, same tags.
    let mcp_tags = match (intent.mcp_enabled, source_path) {
        (true, Some(path)) => {
            let body = std::fs::read_to_string(path).unwrap_or_default();
            let (_cleaned, mut tags) = claudine::mcp::session::lex_tags(&body);
            tags.sort();
            tags.dedup();
            tags
        }
        _ => Vec::new(),
    };

    let (model, model_reason) = resolve_launch_model(provider, cli_model, repo_root, document);

    // The single re-entrant rebuild. `--yolo` is a request; whether it *applies*
    // is the profile's decision in this mode, and the plan is what makes it —
    // there is no second scratch derivation to disagree with the argv that
    // actually ships. Same for the structured-output shape.
    let facets = launch_plan::DocumentLaunchFacets {
        provider,
        non_interactive,
        yolo_requested: intent.cli_yolo,
        is_inline: intent.is_inline,
        model: model.clone(),
        mcp_body_tags: mcp_tags.clone(),
    };
    let use_structured = facets.use_structured();
    let plan = launch_plan::build_launch_plan(&intent.launch_plan_inputs, &facets)?;

    let env_overrides =
        launch_env_overrides(provider, model.as_deref(), plan.yolo_applied);

    // `env_overrides` can only *set*, and it is applied over a base child
    // environment the invocation stamped its own resolved `MODEL` into. A
    // refreshed document that resolves none therefore has to say so explicitly,
    // or the child inherits the opening model (review-9 finding 2).
    //
    // Gated on the invocation having resolved one: with no model at either end,
    // whatever `MODEL` the child environment carries is the caller's own ambient
    // value — invocation-fixed, and not this rebuild's to delete.
    let mut launch_env = plan.env_overlay;
    let invocation_resolved_a_model = intent
        .launch_plan_inputs
        .invocation
        .facets
        .model
        .is_some();
    if invocation_resolved_a_model && model.is_none() {
        launch_env.push(launch_plan::EnvChange::Remove("MODEL".into()));
    }

    // The provider-dependent execution adapters, derived from the REBUILT
    // profile and mode: the attempt that spawns under this bundle also filters,
    // suppresses, and recovers output with this bundle's policies — never the
    // opening provider's (review-9 finding 1).
    //
    // Interactive sessions get empty noise filters. Beyond being useless for an
    // inherited TTY, a non-empty stderr filter makes `exec::run_child` pipe
    // stderr, which flips `isolate_process_group` on and leaves an interactive
    // TUI in a background pgroup — it then hangs on SIGTTIN reading the TTY.
    let stdout_noise: &'static [&'static str] = if non_interactive {
        profile.stdout_noise_prefixes()
    } else {
        &[]
    };
    let stderr_noise: &'static [&'static str] = if non_interactive {
        profile.stderr_noise_prefixes()
    } else {
        &[]
    };
    let codex_output = plan.structured_codex.then(|| {
        crate::commands::wrap::policy::StructuredCodexOutput {
            // The recorded sink is the path the plan's argv names (verbatim and
            // replay alike), so artifact and argv cannot disagree.
            last_message_path: intent.launch_plan_inputs.codex_last_message_path.clone(),
        }
    });
    let dispatch_context = if plan.replayed {
        let provider_reason = if intent.explicit_provider.is_some() {
            ProviderResolutionReason::ExplicitFlag
        } else {
            match document.selection_hints.agent.as_ref() {
                Some(AgentHint::Single(_)) => ProviderResolutionReason::FrontmatterSingle,
                Some(AgentHint::List(_)) => ProviderResolutionReason::FrontmatterList,
                None => intent.fallback_provider_reason,
            }
        };
        let mut context = intent.dispatch_context.clone();
        context.insert(
            "provider_selection_reason".into(),
            serde_json::Value::String(format!("{provider_reason:?}")),
        );
        context.insert(
            "resolved_model".into(),
            serde_json::Value::String(model.clone().unwrap_or_default()),
        );
        context.insert(
            "model_selection_reason".into(),
            serde_json::Value::String(format!("{model_reason:?}")),
        );
        context
    } else {
        // Verbatim: the invocation's own metadata, including selection reasons
        // this rebuild could not reproduce (picker, favorite, sequence review).
        intent.dispatch_context.clone()
    };

    Ok(RebuiltLaunchIdentity {
        provider,
        profile,
        binary_path,
        non_interactive,
        yolo: plan.yolo_applied,
        use_structured,
        codex_output,
        mcp_tags,
        stdout_noise,
        stderr_noise,
        suppress_stderr_on_success: profile.suppress_structured_stderr_on_success(),
        dispatch_context,
        env_overrides,
        args: plan.args,
        launch_env,
    })
}

/// Pick the provider a frontmatter `agent:` hint names.
///
/// Mirrors `composition::select`'s non-TTY agent resolution: a list prefers its
/// first runnable entry and otherwise falls back to its first entry, so a hint
/// that names only uninstalled providers still registers as a *changed* identity
/// rather than silently collapsing onto the previous one.
fn provider_from_hints(
    hints: &claudine::composition::EffectiveSelectionHints,
    snapshot: Option<&InstalledProviderSnapshot>,
) -> Option<Provider> {
    match hints.agent.as_ref()? {
        AgentHint::Single(provider) => Some(*provider),
        AgentHint::List(providers) => providers
            .iter()
            .copied()
            .find(|p| snapshot.is_none_or(|s| s.runnable.contains(p)))
            .or_else(|| providers.first().copied()),
    }
}

/// The binary the rebuilt provider launches.
///
/// Landing back on the invocation's own provider reuses its already-resolved
/// path verbatim — an unchanged document must reproduce the invocation's binary
/// identity exactly, not re-derive it and risk a `which` result that drifted.
/// A rebuilt *different* provider resolves through the snapshot, falling back to
/// the profile's bare binary name so the facet still moves when that provider is
/// not installed.
fn resolve_binary_for(
    provider: Provider,
    profile: &'static dyn WrapperProfile,
    intent: &LaunchRebuildIntent,
) -> PathBuf {
    if provider == intent.fallback_provider {
        return intent.fallback_binary.clone();
    }
    crate::commands::wrap::resolve_binary_path_direct(profile, intent.installed_snapshot.as_ref())
        .unwrap_or_else(|_| PathBuf::from(profile.binary()))
}

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

/// Resolve the model the refreshed document launches with, under R6 precedence:
/// explicit `--model` beats the document's own `model:`, validated against the
/// same catalog a direct invocation uses.
///
/// One answer serves both the launch plan's argv and the `MODEL` environment, so
/// the two cannot describe different models.
fn resolve_launch_model(
    provider: Provider,
    cli_model: Option<&str>,
    repo_root: Option<&Path>,
    document: &MaterializedHarnessPrompt,
) -> (Option<String>, ModelResolutionReason) {
    let selection_config =
        crate::commands::wrap::composition::load_selection_config_for_repo(repo_root);
    let catalog = match &selection_config {
        Some(cfg) => ModelCatalogService::with_overrides(cfg.model_overrides.clone()),
        None => ModelCatalogService::new(),
    };
    claudine::composition::resolve_model_with_hints(
        provider,
        &document.selection_hints,
        cli_model,
        Some(&catalog),
    )
}

/// Project a resolved provider/model/yolo triple into the `AGENT`/`MODEL`/`YOLO`
/// child environment.
fn launch_env_overrides(
    provider: Provider,
    model: Option<&str>,
    yolo: bool,
) -> Vec<(String, String)> {
    let mut env_overrides: Vec<(String, String)> = Vec::new();
    env_overrides.push(("AGENT".to_string(), provider.as_slug().to_string()));
    if let Some(model) = model {
        env_overrides.push(("MODEL".to_string(), model.to_string()));
    }
    env_overrides.push(("YOLO".to_string(), yolo.to_string()));
    env_overrides
}

/// Recompute a proxied target's launch identity from its own frontmatter.
///
/// `cli_model` is the explicit `--model`, which stays authoritative over any
/// target frontmatter `model:`. The model is re-resolved against the target's
/// own [`EffectiveSelectionHints`] with the same catalog validation a direct
/// invocation performs, so a target that pins its own valid `model:` resolves
/// it, and an invalid one falls back exactly as it would directly.
pub(super) fn rebuild_target_launch(
    intent: &LaunchRebuildIntent,
    cli_model: Option<&str>,
    repo_root: Option<&Path>,
    launch_area: &Path,
    target: &MaterializedHarnessPrompt,
) -> Result<TargetLaunchRebuild, crate::commands::wrap::launch_plan::LaunchPlanError> {
    // Passes no source path: the environment overlay has no MCP component, and
    // only the compatibility key reads [`RebuiltLaunchIdentity::mcp_tags`].
    let env_overrides =
        rebuild_launch_identity(intent, cli_model, repo_root, target, None)?.env_overrides;

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

    Ok(TargetLaunchRebuild {
        env_overrides,
        prepared_context,
    })
}

#[cfg(test)]
mod tests;
