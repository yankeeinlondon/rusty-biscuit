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
//! All three paths share [`rebuild_launch_identity`]; [`rebuild_launch_env`]
//! projects its `AGENT`/`MODEL`/`YOLO` half, and [`rebuild_target_launch`] adds
//! the early-binding context proxy adoption needs.
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
//! - **Workspace/child CWD** has no frontmatter surface at all: it comes from
//!   the launch workspace and `--repo`. Nothing a document can write moves it,
//!   so the key reads the invocation's `child_cwd` directly and the facet stays
//!   projection-only.
//! - **System-prompt content** is composed once, at invocation, into the
//!   provider-native argv (`profile::apply_system_prompt`). A document has no
//!   `system_prompt:` surface, and rewriting the `--append-system-prompt` file
//!   after the fact changes nothing, because the argv already carries the
//!   composed text (or a temp file written at invocation). Re-composing it per
//!   attempt would be a launch-pipeline re-entry, not a document rebuild, so the
//!   facet stays projection-only.
//!
//! ## The launch this rebuild reaches, and the one it does not
//!
//! The rebuilt identity feeds the compatibility key on every route. It also
//! feeds the child environment via the materialized prompt's `env_overrides`
//! (which [`super::super::build_harness_launch`] overlays onto the base env).
//!
//! It does **not** rebuild the provider argv, MCP runtime injection, or the
//! profile the harness actually spawns: those are assembled once by the
//! invocation pipeline. Two different callers live with that, and they live
//! with it for two different reasons.
//!
//! ### Proxy adoption — the target's full bundle is built above the harness
//!
//! [`rebuild_target_launch`] runs for a target the **command coordinator**
//! already re-prepared. Every route that reaches it carries a run ledger, so the
//! handoff surfaced up and the target was rebuilt as a fresh document through
//! `commands::compose::prep::prepare_and_run_active_document` — which re-enters
//! the production selection/MCP/argv pipeline and rebuilds profile/binary
//! sub-selection, the argv entrypoint, MCP runtime injection, child CWD, and
//! system-prompt delivery from the target's own frontmatter. What is rebuilt
//! here is the remainder: the lifecycle early-binding context and the
//! `AGENT`/`MODEL`/`YOLO` env for the target's staged bootstrap.
//!
//! A handoff with no ledger to surface to has no such coordinator, and is
//! therefore **refused** rather than adopted against a borrowed bundle — see
//! `super::surface_or_adopt_terminal_proxy`. There is no reduced launch path.
//!
//! ### Retry/resume refresh — bounded, and in the safe direction
//!
//! At a same-document fresh-read boundary there is no coordinator re-entry at
//! all. A resume whose refresh moved provider, interactivity, permission mode,
//! or structured-output mode is **refused** rather than run against a launch
//! plan the session was not opened with, which is what R8 asks for. A *retry*
//! on that path still launches under the invocation's argv; that residue is
//! tracked against AC15 rather than here.

use std::path::{Path, PathBuf};

use claudine::composition::{AgentHint, InstalledProviderSnapshot};
use claudine::model_catalog::ModelCatalogService;
use claudine::provider::Provider;

use super::super::super::profile::{WrapperProfile, profile_for_provider};

use super::super::MaterializedHarnessPrompt;

/// The immutable invocation intent every per-attempt launch rebuild resolves
/// the refreshed document against.
///
/// "Immutable" is the R8 sense: these are the caller's own decisions, which stay
/// authoritative at every document, retry, and resume of the invocation. Only
/// the document half of the rebuild moves.
#[derive(Debug, Clone)]
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
}

/// The launch properties a resume cannot renegotiate, recomputed from one
/// refreshed read of the active document.
///
/// Built by [`rebuild_launch_identity`]. Both sides of the R8 compatibility
/// comparison are derived from this, so a document mutation that moves any field
/// here is a named incompatible facet on the next resume.
pub(crate) struct RebuiltLaunchIdentity {
    pub(crate) provider: Provider,
    pub(crate) profile: &'static dyn WrapperProfile,
    pub(crate) binary_path: PathBuf,
    pub(crate) non_interactive: bool,
    /// Whether the provider's bypass mechanism actually takes effect in this
    /// mode — not merely whether `--yolo` was asked for.
    pub(crate) yolo: bool,
    pub(crate) use_structured: bool,
    pub(crate) structured_codex: bool,
    /// MCP tags lexed from the refreshed body, sorted and deduped. Empty when
    /// MCP is not enabled for the invocation.
    pub(crate) mcp_tags: Vec<String>,
    /// `AGENT`/`MODEL`/`YOLO` overlays for the provider child's environment.
    pub(crate) env_overrides: Vec<(String, String)>,
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
) -> RebuiltLaunchIdentity {
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

    // Mirrors the command phase: `--yolo` is a request, and the provider decides
    // whether its bypass applies in this mode. A provider whose bypass is
    // non-interactive-only silently declines in an interactive session, which is
    // a genuine change of permission mode.
    let yolo = intent.cli_yolo && {
        let mut scratch_args: Vec<String> = Vec::new();
        let mut scratch_env: Vec<(String, String)> = Vec::new();
        profile
            .apply_yolo_for_mode(&mut scratch_args, &mut scratch_env, !non_interactive)
            .map(|outcome| outcome.applied)
            .unwrap_or(false)
    };

    // Mirrors the command phase's structured-streaming decision, then asks the
    // production Codex predicate itself rather than restating which provider it
    // singles out — the scratch argv it would decorate is discarded.
    let use_structured = profile.supports_structured_stream() && non_interactive;
    let structured_codex = {
        let mut scratch_args: Vec<String> = Vec::new();
        crate::commands::exec_prep::prepare_codex_structured_output(
            provider,
            use_structured,
            !non_interactive && intent.is_inline,
            &mut scratch_args,
        )
        .is_some()
    };

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

    let env_overrides = launch_env_overrides(provider, cli_model, yolo, repo_root, document);

    RebuiltLaunchIdentity {
        provider,
        profile,
        binary_path,
        non_interactive,
        yolo,
        use_structured,
        structured_codex,
        mcp_tags,
        env_overrides,
    }
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

/// The `AGENT`/`MODEL`/`YOLO` half of [`rebuild_launch_identity`], for the
/// callers that only need to overlay the child environment.
///
/// Passes no source path because the environment overlay has no MCP component;
/// only the compatibility key reads [`RebuiltLaunchIdentity::mcp_tags`].
pub(super) fn rebuild_launch_env(
    intent: &LaunchRebuildIntent,
    cli_model: Option<&str>,
    repo_root: Option<&Path>,
    document: &MaterializedHarnessPrompt,
) -> Vec<(String, String)> {
    rebuild_launch_identity(intent, cli_model, repo_root, document, None).env_overrides
}

/// Project a resolved provider/model/yolo triple into the `AGENT`/`MODEL`/`YOLO`
/// child environment.
fn launch_env_overrides(
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
) -> TargetLaunchRebuild {
    let env_overrides = rebuild_launch_env(intent, cli_model, repo_root, target);

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

    /// The invocation intent an unchanged run resolves against: Goose,
    /// non-interactive, nothing explicit, MCP in play so a body tag counts.
    fn intent() -> LaunchRebuildIntent {
        LaunchRebuildIntent {
            explicit_provider: None,
            fallback_provider: Provider::Goose,
            fallback_binary: PathBuf::from("/usr/bin/goose"),
            installed_snapshot: None,
            default_non_interactive: true,
            cli_yolo: false,
            is_inline: false,
            mcp_enabled: true,
        }
    }

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
        document(selection_hints, frontmatter, "body")
    }

    fn document(
        selection_hints: EffectiveSelectionHints,
        frontmatter: serde_json::Value,
        prompt: &str,
    ) -> MaterializedHarnessPrompt {
        MaterializedHarnessPrompt {
            live_frontmatter: MaterializedHarnessPrompt::live_cell_from(&frontmatter),
            frontmatter,
            prompt: prompt.to_string(),
            env_overrides: Vec::new(),
            selection_hints,
            inline_closure_plan: None,
            lifecycle: None,
        }
    }

    fn with_hints(hints: EffectiveSelectionHints) -> MaterializedHarnessPrompt {
        document(hints, serde_json::json!({ "title": "t" }), "body")
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
        let rebuild = rebuild_target_launch(&intent(), None, None, Path::new("."), &target);

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
            &intent(),
            Some("llamacpp/cli-pinned"),
            None,
            Path::new("."),
            &target,
        );

        assert_eq!(
            value_of(&rebuild.env_overrides, "MODEL"),
            Some("llamacpp/cli-pinned"),
            "explicit --model must win over the target frontmatter model",
        );
    }

    /// A target with no `model:` produces no `MODEL` overlay (matching
    /// `install_agent_env_for_composition`), so the child env falls through to
    /// the provider default rather than a stale router model.
    #[test]
    fn rebuild_omits_model_when_target_pins_none() {
        let target = target_with_model(None);
        let rebuild = rebuild_target_launch(&intent(), None, None, Path::new("."), &target);

        assert_eq!(value_of(&rebuild.env_overrides, "AGENT"), Some("goose"));
        assert_eq!(
            value_of(&rebuild.env_overrides, "MODEL"),
            None,
            "no frontmatter model means no MODEL overlay",
        );
    }

    /// The property the whole R8 comparison rests on: the rebuild is a pure
    /// function of (document, intent), so an unchanged document cannot produce a
    /// false refusal no matter how many attempts run.
    #[test]
    fn an_unchanged_document_rebuilds_to_an_identical_identity() {
        let doc = target_with_model(Some("llamacpp/probe-model-x"));
        let first = rebuild_launch_identity(&intent(), None, None, &doc, None);
        let second = rebuild_launch_identity(&intent(), None, None, &doc, None);

        assert_eq!(first.provider, second.provider);
        assert_eq!(first.binary_path, second.binary_path);
        assert_eq!(first.non_interactive, second.non_interactive);
        assert_eq!(first.yolo, second.yolo);
        assert_eq!(first.use_structured, second.use_structured);
        assert_eq!(first.structured_codex, second.structured_codex);
        assert_eq!(first.mcp_tags, second.mcp_tags);
        assert_eq!(first.env_overrides, second.env_overrides);
    }

    /// Frontmatter `agent:` moves the provider — and with it the binary and the
    /// resume protocol the compatibility key reads.
    #[test]
    fn frontmatter_agent_moves_the_provider_and_its_binary() {
        let base = rebuild_launch_identity(&intent(), None, None, &with_hints(
            EffectiveSelectionHints::default(),
        ), None);
        let switched = rebuild_launch_identity(&intent(), None, None, &with_hints(
            EffectiveSelectionHints {
                agent: Some(AgentHint::Single(Provider::Claude)),
                ..EffectiveSelectionHints::default()
            },
        ), None);

        assert_eq!(base.provider, Provider::Goose);
        assert_eq!(switched.provider, Provider::Claude);
        assert_ne!(
            base.binary_path, switched.binary_path,
            "a provider switch must move the binary the key records",
        );
    }

    /// Explicit CLI provider selection is immutable invocation intent: a
    /// frontmatter `agent:` cannot move it, so it can never refuse a resume.
    #[test]
    fn an_explicit_cli_provider_pins_the_rebuilt_provider() {
        let pinned = LaunchRebuildIntent {
            explicit_provider: Some(Provider::Goose),
            ..intent()
        };
        let rebuilt = rebuild_launch_identity(&pinned, None, None, &with_hints(
            EffectiveSelectionHints {
                agent: Some(AgentHint::Single(Provider::Claude)),
                ..EffectiveSelectionHints::default()
            },
        ), None);

        assert_eq!(rebuilt.provider, Provider::Goose);
    }

    /// Frontmatter `interactive:` moves the session mode, and the
    /// structured-output mode that mode implies moves with it.
    #[test]
    fn frontmatter_interactive_moves_the_mode_and_structured_output() {
        // OpenCode has a stream protocol, so structured streaming is on for it in
        // non-interactive mode; Goose has none and would hide the flip.
        let streaming = LaunchRebuildIntent {
            fallback_provider: Provider::OpenCode,
            ..intent()
        };
        let base = rebuild_launch_identity(&streaming, None, None, &with_hints(
            EffectiveSelectionHints::default(),
        ), None);
        let interactive = rebuild_launch_identity(&streaming, None, None, &with_hints(
            EffectiveSelectionHints {
                interactive: Some(true),
                ..EffectiveSelectionHints::default()
            },
        ), None);

        assert!(base.non_interactive);
        assert!(!interactive.non_interactive);
        assert!(
            base.use_structured && !interactive.use_structured,
            "structured streaming is non-interactive only, so the mode flip must move it",
        );
    }

    /// `--yolo` records the permission mode the provider actually achieves, not
    /// the one that was asked for: a provider whose bypass is non-interactive
    /// only declines in an interactive session, which is a real mode change.
    #[test]
    fn the_permission_mode_records_what_yolo_achieved_not_what_was_asked() {
        // OpenCode's bypass is non-interactive only; Goose sets an env var that
        // applies in either mode and would hide the difference.
        let requested = LaunchRebuildIntent {
            cli_yolo: true,
            fallback_provider: Provider::OpenCode,
            ..intent()
        };
        let non_interactive = rebuild_launch_identity(&requested, None, None, &with_hints(
            EffectiveSelectionHints::default(),
        ), None);
        let interactive = rebuild_launch_identity(&requested, None, None, &with_hints(
            EffectiveSelectionHints {
                interactive: Some(true),
                ..EffectiveSelectionHints::default()
            },
        ), None);

        assert!(non_interactive.yolo);
        assert!(
            !interactive.yolo,
            "OpenCode bypass is non-interactive only, so an interactive refresh drops it",
        );
    }

    /// MCP `#tag`s lexed from the document **on disk** participate in the
    /// identity — but only when MCP is in play for the invocation at all.
    ///
    /// Reading the source rather than `document.prompt` is what makes the signal
    /// survive a resume, whose prompt field carries the follow-up message.
    #[test]
    fn body_mcp_tags_are_lexed_from_disk_and_only_when_mcp_is_enabled() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("doc.md");
        std::fs::write(&source, "---\ntitle: t\n---\nwork on #calendar today\n").unwrap();
        // The prompt field deliberately carries a resume follow-up, proving the
        // tags do not come from it.
        let doc = document(
            EffectiveSelectionHints::default(),
            serde_json::json!({ "title": "t" }),
            "please finish the resumed work",
        );

        let enabled =
            rebuild_launch_identity(&intent(), None, None, &doc, Some(source.as_path()));
        assert_eq!(enabled.mcp_tags, vec!["calendar".to_string()]);

        let off = LaunchRebuildIntent {
            mcp_enabled: false,
            ..intent()
        };
        assert!(
            rebuild_launch_identity(&off, None, None, &doc, Some(source.as_path()))
                .mcp_tags
                .is_empty(),
            "with MCP off no body tag participates in the launch, so none may refuse a resume",
        );
    }
}
