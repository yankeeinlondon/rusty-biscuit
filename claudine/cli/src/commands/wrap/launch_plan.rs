//! R6/R8 — the re-entrant launch-plan builder.
//!
//! The invocation's command phase
//! (`composition::pipeline::construct_argv_and_system_prompt`) assembles the
//! provider argv and the child environment exactly once, interleaving pure
//! producers with side effects: system-prompt temp-file writes, MCP shadow-HOME
//! materialization, warning emission, an interactive `Select` for ambiguous MCP
//! tags, and `switch_process_cwd`. A retry needs the *plan* again — against a
//! refreshed document — but must not repeat any of those effects, and above all
//! must never prompt: there is no operator at a retry boundary.
//!
//! This module is that split. The invocation performs every effect once and
//! records the results into [`LaunchPlanInputs`]; [`build_launch_plan`] is a
//! pure function of `(inputs, facets)` that re-derives argv and the environment
//! overlay for whatever provider/mode/permission/MCP shape the refreshed
//! document resolves to.
//!
//! ## The verbatim shortcut
//!
//! [`LaunchPlanInputs::invocation`] records the facets the invocation itself
//! resolved *and* the argv/env it produced. When a rebuild's facets compare
//! equal, the recorded plan is returned unchanged. An unchanged document is
//! therefore byte-identical to the invocation by construction rather than by a
//! careful replay that happens to agree — which is also why landing this could
//! not disturb the existing suite: every document that moves nothing takes the
//! recorded path.
//!
//! The replay is exercised instead by
//! `tests::replay_reproduces_the_invocation_argv`, which forces the replay
//! for the invocation's *own* facets and asserts it reproduces the recorded
//! argv. That is the test that keeps the two paths honest.
//!
//! ## Ambiguous MCP tags
//!
//! `compute_session_set` takes a resolver closure. At invocation that closure
//! may prompt. Here it is a lookup into
//! [`McpRebuildInputs::ambiguity_resolutions`], the map the invocation's
//! resolutions were recorded into, and a tag absent from that map stays
//! ambiguous (and is dropped) rather than prompting. A per-attempt rebuild
//! cannot reach a terminal.
//!
//! ## Invocation intent is stored semantically, never as provider bytes
//!
//! `--output` and `--sandbox` are immutable invocation intent, but their
//! *encoding* belongs to whichever provider ends up running: Goose renders
//! `--output json` as no argv at all while Gemini needs `--output-format json`,
//! and Codex implements `--sandbox` where Goose does not. So the recorded inputs
//! hold [`OutputFormat`] and a requested-sandbox flag, and every rebuild renders
//! them through its own profile. Recording the rendered bytes instead silently
//! dropped the requested format on one provider move and leaked an unsupported
//! flag on the reverse one.
//!
//! ## The environment is a patch, not an overlay
//!
//! [`LaunchPlan::env_overlay`] is a list of [`EnvChange`]s, because a rebuild
//! frequently needs to *unset* something. The invocation records what each
//! provider-shaped environment key held before it wrote it
//! ([`LaunchPlanInputs::provider_env_baseline`]); a replay restores every one of
//! those keys it did not write again. That is what makes a retry that dropped
//! `model:` actually reach the child without `MODEL`, rather than merely
//! omitting `MODEL` from an additive overlay laid over an environment that still
//! has it.
//!
//! That baseline covers keys the provider-shaped stages *wrote*. Credential
//! sanitation is the other half: it runs earlier still, deciding which ambient
//! secrets exist in the base at all, and a key the opening profile stripped
//! leaves no trace to restore from. [`CredentialPolicyInputs`] carries the
//! unsanitized ambient values so a rebuilt provider can re-run the same
//! allow-list for itself, in both directions.
//!
//! ## System-prompt delivery
//!
//! *Resolution* — finding the file, composing its body — is
//! provider-independent and stays invocation-once. *Delivery* is
//! provider-shaped (`profile::apply_system_prompt`), so a rebuild that lands on
//! a different provider re-applies it for that provider instead of splicing the
//! old provider's flags into the new argv. Refusing the provider-move case
//! instead was tried and abandoned: Claudine composes a system prompt on
//! essentially every compose run, so refusal made the whole provider facet
//! unreachable.
//!
//! Re-application writes a temp file, whose [`SystemPromptArtifact`] must
//! outlive the child process. Those artifacts ride in
//! [`LaunchPlan::system_prompt_artifacts`] rather than being dropped when the
//! builder returns — and the caller that consumes the plan has to keep carrying
//! them, because the argv only holds a *path*. The per-attempt bundle
//! (`RebuiltLaunchIdentity`) takes ownership and holds them until the child has
//! exited.
//!
//! The *content* is an immutable invocation input under R8: a document has no
//! `system_prompt:` surface, and this builder re-delivers the already-composed
//! [`ResolvedSystemPrompt`] rather than re-reading the file it came from, so
//! rewriting that file between attempts moves nothing. See
//! `tests::rewriting_the_discovered_system_prompt_moves_no_delivered_content`.
//!
//! [`SystemPromptArtifact`]: super::system_prompt::SystemPromptArtifact

use std::collections::{HashMap, HashSet};
use std::ffi::{OsStr, OsString};
use std::path::PathBuf;

use claudine::provider::Provider;
use claudine::system_prompt::ResolvedSystemPrompt;

use super::profile::{OutputFormat, WrapperProfile, profile_for_provider};

const OPENCODE_CONFIG: &str = "OPENCODE_CONFIG_CONTENT";

/// One change a launch plan makes to the invocation's base child environment.
///
/// A set-only overlay cannot express "the refreshed document no longer wants
/// this", which is how a retry that dropped `model:` kept the opening document's
/// `MODEL`, and how a provider switch kept the old provider's
/// `OPENCODE_CONFIG_CONTENT` or shadow `HOME`. Removal is therefore part of the
/// vocabulary rather than something a caller has to infer from an absence.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum EnvChange {
    Set(OsString, OsString),
    Remove(OsString),
}

impl EnvChange {
    pub(crate) fn key(&self) -> &OsStr {
        match self {
            Self::Set(key, _) | Self::Remove(key) => key,
        }
    }
}

/// A launch plan could not be rebuilt for the refreshed document's facets.
#[derive(Debug)]
pub(crate) enum LaunchPlanError {
    /// The rebuilt provider has no runtime MCP injector, but MCP is in play.
    /// Mirrors the invocation's own refusal.
    NoMcpInjector(Provider),
    /// A producer the replay re-runs failed (model validation, MCP injection,
    /// non-interactive argv requirements).
    Producer(String),
    /// The document moved a facet on a path that recorded no replay slices.
    ReplayUnavailable,
}

impl std::fmt::Display for LaunchPlanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoMcpInjector(provider) => write!(
                f,
                "provider `{}` does not support runtime MCP injection.\n\
                 Use `claudine mcp export {} --apply` to write servers to its native config instead.",
                provider,
                provider.as_slug()
            ),
            Self::Producer(message) => write!(f, "{message}"),
            Self::ReplayUnavailable => write!(
                f,
                "the refreshed document changes the launch plan, but this invocation did not \
                 record the inputs needed to rebuild one"
            ),
        }
    }
}

impl std::error::Error for LaunchPlanError {}

/// Everything the invocation resolved once, with its side effects already
/// performed, that a per-attempt rebuild needs in order to re-derive a plan
/// without repeating them.
#[derive(Clone)]
pub(crate) struct LaunchPlanInputs {
    /// The forwarded `--` provider tail: the base every argv starts from, ahead
    /// of Claudine's own injections (`apply_entrypoint` inserts at index 0, so a
    /// tail-first base still lands after it).
    pub(crate) provider_args_tail: Vec<String>,
    /// The `--output <format>` the caller asked for, as semantic intent rather
    /// than as the argv one provider happened to render it into. The intent is
    /// invocation-fixed; its *encoding* is provider-owned (Goose renders `json`
    /// as no argv at all, Gemini as `--output-format json`), so every rebuild
    /// re-renders it through its own profile.
    pub(crate) output_format: Option<OutputFormat>,
    /// The system-prompt delivery argv `profile::apply_system_prompt` composed,
    /// and the `OPENCODE_CONFIG_CONTENT` overlay it contributed (OpenCode
    /// delivers its system prompt through the same inline config the MCP and
    /// YOLO producers write). Replayed verbatim while the provider holds still.
    pub(crate) system_prompt_args: Vec<String>,
    pub(crate) system_prompt_opencode_config: Option<String>,
    /// The resolved system prompt itself, plus what re-delivering it costs.
    ///
    /// Resolution (finding the file, composing its body) is provider-independent
    /// and happens once; *delivery* is provider-shaped, so a rebuild that lands
    /// on a different provider re-applies it for that provider rather than
    /// splicing the old provider's flags into the new argv. Claudine composes a
    /// system prompt on essentially every compose run, so refusing the
    /// provider-move case instead would have made the whole facet unreachable.
    pub(crate) system_prompt: Option<ResolvedSystemPrompt>,
    pub(crate) system_prompt_cwd: PathBuf,
    pub(crate) system_prompt_scoped_tmp: PathBuf,
    /// Whether `--sandbox` was requested. Same reasoning as
    /// [`Self::output_format`]: Codex implements it and Goose does not, so the
    /// request is invocation-fixed and the rendering is per-rebuild.
    pub(crate) sandbox_requested: bool,
    /// Whether the caller's env plan already carries a `MODEL` entry, which is
    /// the model stage's `has_model_env` input.
    pub(crate) has_model_env: bool,
    /// MCP inputs, `None` when MCP is not in play for this invocation.
    pub(crate) mcp: Option<McpRebuildInputs>,
    /// `OPENCODE_CONFIG_CONTENT` as it stood *before* the MCP, YOLO, and
    /// system-prompt folds. The replay rebuilds the chain from here in that same
    /// order, so a rebuilt permission mode or server set lands on the same base
    /// the invocation folded onto.
    pub(crate) opencode_config_base: Option<String>,
    /// The Codex `--output-last-message` sink, allocated once. Reused across
    /// attempts rather than re-allocated, so the argv a rebuild produces for an
    /// unchanged Codex run matches the recorded one.
    pub(crate) codex_last_message_path: PathBuf,
    /// What every provider-shaped environment key held *before* the invocation's
    /// provider-shaped stages wrote it; `None` means the key was absent.
    ///
    /// This is the invocation-neutral base a replay rebuilds the provider-owned
    /// half of the child environment from: a key recorded here that the rebuilt
    /// plan does not set again is restored to its pre-provider value, or removed
    /// when it had none. Without it the overlay could only ever add, so an
    /// attempt that dropped `model:` or moved provider inherited the opening
    /// document's `MODEL`, `OPENCODE_CONFIG_CONTENT`, and shadow `HOME`.
    pub(crate) provider_env_baseline: HashMap<OsString, Option<OsString>>,
    /// The unsanitized ambient credential environment plus explicit `--include`
    /// intent. See [`CredentialPolicyInputs`].
    pub(crate) credential_policy: CredentialPolicyInputs,
    /// The invocation's own facets and the plan they produced. See the module
    /// docs' verbatim shortcut.
    pub(crate) invocation: RecordedLaunch,
    /// Whether the slices above were actually recorded, i.e. whether a replay is
    /// possible at all. `false` for callers built by
    /// [`LaunchPlanInputs::recorded_only`], where a moved facet is a typed
    /// refusal rather than a plan assembled from empty slices — dropping MCP
    /// injection or a system prompt silently would be exactly the incoherence
    /// this module exists to prevent.
    pub(crate) replay_supported: bool,
}

impl LaunchPlanInputs {
    /// Inputs that carry only the recorded plan, with no replay slices.
    ///
    /// The direct provider wrappers (`claudine claude`, …) are invoked *as* a
    /// provider, which is explicit invocation intent no re-read can move, and
    /// their passthrough document carries no selection hints — so provider,
    /// mode, permission, and model are all pinned. Their *MCP* facet is not:
    /// `--mcp`/`--use` make a refreshed body's `#tag`s meaningful. Rather than
    /// replay from empty slices and silently drop MCP injection, a moved facet
    /// here is refused with [`LaunchPlanError::ReplayUnavailable`].
    pub(crate) fn recorded_only(
        facets: DocumentLaunchFacets,
        args: Vec<String>,
        // The invocation's `--output-last-message` sink, when Codex
        // structured-output capture is part of the recorded plan. Carried as
        // the real path (not a bool) so the per-attempt rebuilt bundle can
        // hand the attempt a working artifact for the recorded argv.
        codex_last_message: Option<PathBuf>,
    ) -> Self {
        let structured_codex = codex_last_message.is_some();
        Self {
            provider_args_tail: Vec::new(),
            output_format: None,
            system_prompt_args: Vec::new(),
            system_prompt_opencode_config: None,
            system_prompt: None,
            system_prompt_cwd: PathBuf::new(),
            system_prompt_scoped_tmp: PathBuf::new(),
            sandbox_requested: false,
            has_model_env: false,
            mcp: None,
            opencode_config_base: None,
            codex_last_message_path: codex_last_message.unwrap_or_default(),
            provider_env_baseline: HashMap::new(),
            credential_policy: CredentialPolicyInputs::default(),
            invocation: RecordedLaunch {
                facets,
                args,
                env_overlay: Vec::new(),
                structured_codex,
            },
            replay_supported: false,
        }
    }
}

/// The invocation-neutral inputs a rebuild re-runs credential sanitation from.
///
/// Which ambient secrets reach a child is a *profile* decision — each provider's
/// `allowed_env_keys()` — but the command phase makes it once, before the base
/// child environment exists. That base therefore records only the opening
/// profile's verdict, and a key it stripped is simply gone: no additive overlay
/// over it can readmit a credential the rebuilt provider is entitled to. Holding
/// the ambient values instead lets a replay restate the answer absolutely in
/// both directions.
///
/// ## Notes
///
/// The values here are ambient process-environment secrets. They exist so a
/// rebuild can decide admission, and reach a child only through the same
/// allow-list a direct invocation of that provider would consult.
#[derive(Clone, Default)]
pub(crate) struct CredentialPolicyInputs {
    /// Every sensitive key in the wrapper's process environment, before any
    /// provider allow-list ran.
    pub(crate) ambient: HashMap<OsString, OsString>,
    /// `--include` names. Explicit invocation intent, so it admits a key under
    /// every rebuilt provider exactly as it did at invocation.
    pub(crate) explicit_include: HashSet<String>,
}

impl CredentialPolicyInputs {
    /// The absolute sensitive-key patch one profile's allow-list produces
    /// against the ambient environment: a `Set` for every key it admits, a
    /// `Remove` for every key it does not.
    ///
    /// Absolute rather than differential because the base it lands on was
    /// sanitized for a *different* profile, and sorted by key so the patch is
    /// deterministic for the same inputs.
    fn patch_for(&self, profile: &dyn WrapperProfile) -> Vec<EnvChange> {
        let auto_include: HashSet<String> = profile
            .allowed_env_keys()
            .iter()
            .map(|key| (*key).to_string())
            .collect();
        let mut patch: Vec<EnvChange> = self
            .ambient
            .iter()
            .map(|(key, value)| {
                let name = key.to_string_lossy();
                if super::env::admits_sensitive_key(&name, &self.explicit_include, &auto_include) {
                    EnvChange::Set(key.clone(), value.clone())
                } else {
                    EnvChange::Remove(key.clone())
                }
            })
            .collect();
        patch.sort_by(|a, b| a.key().cmp(b.key()));
        patch
    }
}

/// The MCP inputs a rebuild recomputes runtime injection from.
#[derive(Clone)]
pub(crate) struct McpRebuildInputs {
    /// `--mcp-use` ids, invocation intent.
    pub(crate) explicit_use: Vec<String>,
    pub(crate) repo_root: Option<PathBuf>,
    /// The shadow HOME, materialized at invocation whenever MCP is in play —
    /// including for providers whose injector does not need one — precisely so a
    /// rebuild that lands on a shadow-HOME provider finds one already on disk
    /// instead of having to create it.
    pub(crate) shadow_home: Option<PathBuf>,
    /// `#tag` → chosen server id, resolved once at invocation. A rebuild reads
    /// this map and never prompts; an unlisted ambiguous tag is dropped.
    pub(crate) ambiguity_resolutions: HashMap<String, String>,
}

/// The invocation's own resolved facets and the plan they produced.
#[derive(Clone)]
pub(crate) struct RecordedLaunch {
    pub(crate) facets: DocumentLaunchFacets,
    pub(crate) args: Vec<String>,
    pub(crate) env_overlay: Vec<(OsString, OsString)>,
    pub(crate) structured_codex: bool,
}

/// The document-dependent half of a launch plan.
///
/// Everything here is something a refreshed document's frontmatter or body can
/// move under R6 precedence — explicit CLI intent has already been folded in by
/// the caller, so a facet that the CLI pinned simply never differs.
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct DocumentLaunchFacets {
    pub(crate) provider: Provider,
    pub(crate) non_interactive: bool,
    /// Whether `--yolo` was *requested*. Whether it applies is the profile's
    /// decision, re-made per rebuild.
    pub(crate) yolo_requested: bool,
    pub(crate) is_inline: bool,
    pub(crate) model: Option<String>,
    /// `#tag`s the document body selects MCP servers with, sorted and deduped.
    /// Empty when MCP is not in play.
    pub(crate) mcp_body_tags: Vec<String>,
}

impl DocumentLaunchFacets {
    /// The profile the facets' provider selects.
    pub(crate) fn profile(&self) -> Option<&'static dyn WrapperProfile> {
        profile_for_provider(self.provider)
    }

    /// Structured streaming is on for a provider that speaks a stream protocol,
    /// in non-interactive mode only. Mirrors the command phase's own decision.
    pub(crate) fn use_structured(&self) -> bool {
        self.profile()
            .is_some_and(|profile| profile.supports_structured_stream())
            && self.non_interactive
    }
}

/// A resolved launch plan: the provider argv and the environment overlay that
/// go on top of the invocation's base child environment.
pub(crate) struct LaunchPlan {
    pub(crate) args: Vec<String>,
    /// The patch this plan applies to the invocation's base child environment,
    /// in order. A replay that no longer wants a provider-shaped key emits a
    /// [`EnvChange::Remove`] (or a restore of its pre-provider value) for it —
    /// see [`LaunchPlanInputs::provider_env_baseline`].
    pub(crate) env_overlay: Vec<EnvChange>,
    /// Whether the provider's bypass actually applied in this mode.
    pub(crate) yolo_applied: bool,
    /// Whether Codex structured-output capture is part of this plan.
    pub(crate) structured_codex: bool,
    /// `false` when the verbatim shortcut returned the recorded invocation
    /// plan; `true` when a moved facet forced a replay. The rebuilt bundle
    /// keys its dispatch-context recompute off this: an unchanged document
    /// keeps the invocation's dispatch metadata (including selection reasons
    /// the rebuild cannot reproduce, e.g. `SequenceReview`), while a moved
    /// facet recomputes them from the refreshed facets.
    pub(crate) replayed: bool,
    /// Temp resources a re-applied system prompt created, whose paths this
    /// plan's argv and environment name. Their contents are never read here;
    /// they are moved into the per-attempt bundle so their RAII cleanup runs
    /// after the child exits, not when the plan is consumed.
    pub(crate) system_prompt_artifacts: Vec<super::system_prompt::SystemPromptArtifact>,
}

/// Re-derive the launch plan for one set of document facets.
///
/// ## Errors
///
/// Returns [`LaunchPlanError`] when the refreshed facets name a plan this
/// builder cannot assemble without re-entering the invocation pipeline — see
/// the variants. The caller surfaces these as a typed refusal rather than
/// launching a partially rebuilt bundle.
pub(crate) fn build_launch_plan(
    inputs: &LaunchPlanInputs,
    facets: &DocumentLaunchFacets,
) -> Result<LaunchPlan, LaunchPlanError> {
    if facets == &inputs.invocation.facets {
        return Ok(LaunchPlan {
            args: inputs.invocation.args.clone(),
            // Identical facets touched the base environment identically, so the
            // recorded sets are the whole patch: there is nothing to remove.
            env_overlay: inputs
                .invocation
                .env_overlay
                .iter()
                .cloned()
                .map(|(key, value)| EnvChange::Set(key, value))
                .collect(),
            // The invocation's own permission outcome, not a re-derivation:
            // identical facets cannot have produced a different one.
            yolo_applied: recorded_yolo_applied(inputs),
            structured_codex: inputs.invocation.structured_codex,
            replayed: false,
            system_prompt_artifacts: Vec::new(),
        });
    }
    replay(inputs, facets)
}

/// The invocation's achieved permission mode, read back from the `YOLO` overlay
/// it recorded rather than re-derived.
fn recorded_yolo_applied(inputs: &LaunchPlanInputs) -> bool {
    inputs
        .invocation
        .env_overlay
        .iter()
        .find(|(key, _)| key == OsStr::new("YOLO"))
        .is_some_and(|(_, value)| value == OsStr::new("true"))
}

/// Reassemble a plan from the recorded invocation-fixed slices and the
/// document-dependent producers, in the command phase's own order.
fn replay(
    inputs: &LaunchPlanInputs,
    facets: &DocumentLaunchFacets,
) -> Result<LaunchPlan, LaunchPlanError> {
    if !inputs.replay_supported {
        return Err(LaunchPlanError::ReplayUnavailable);
    }
    let invocation_provider = inputs.invocation.facets.provider;
    let provider_moved = facets.provider != invocation_provider;
    let profile = facets
        .profile()
        .ok_or(LaunchPlanError::Producer(format!(
            "provider `{}` has no wrapper profile",
            facets.provider
        )))?;

    let mut args = inputs.provider_args_tail.clone();
    let mut env_overlay: Vec<(OsString, OsString)> = Vec::new();

    // R6/AC10 — credential admission belongs to the provider that actually runs.
    // The base child environment was sanitized for the opening profile, so a
    // provider move must restate the verdict for the rebuilt one: readmit what
    // the opening profile stripped, strip what it admitted. Emitted ahead of the
    // provider-shaped sets below so those still win a key collision.
    let credential_patch = if provider_moved {
        inputs.credential_policy.patch_for(profile)
    } else {
        Vec::new()
    };

    // -- permission mode ----------------------------------------------------
    let mut yolo_applied = false;
    if facets.yolo_requested {
        let mut yolo_env: Vec<(String, String)> = Vec::new();
        let outcome = profile
            .apply_yolo_for_mode(&mut args, &mut yolo_env, !facets.non_interactive)
            .map_err(|e| LaunchPlanError::Producer(e.to_string()))?;
        yolo_applied = outcome.applied;
        for (key, value) in yolo_env {
            env_overlay.push((key.into(), value.into()));
        }
    }
    env_overlay.push((
        "YOLO".into(),
        if yolo_applied { "true" } else { "false" }.into(),
    ));

    // -- entrypoint and session mode ---------------------------------------
    profile.apply_entrypoint(&mut args, facets.non_interactive);
    if facets.non_interactive {
        profile
            .apply_non_interactive_flags(&mut args)
            .map_err(|e| LaunchPlanError::Producer(e.to_string()))?;
    }

    // -- model --------------------------------------------------------------
    let mut model_env: Vec<(String, String)> = Vec::new();
    crate::commands::exec_prep::resolve_model_and_validate(
        facets.provider,
        profile,
        &mut args,
        facets.model.as_deref(),
        facets.non_interactive,
        inputs.has_model_env,
        &mut |key, value| model_env.push((key, value)),
        // A rebuild has no operator to warn. The invocation already surfaced
        // any unsupported-model warning for this document.
        &mut |_warning| {},
    )
    .map_err(|e| LaunchPlanError::Producer(e.into_report().to_string()))?;
    for (key, value) in model_env {
        env_overlay.push((key.into(), value.into()));
    }

    // -- invocation-fixed intent, re-encoded for the rebuilt provider -------
    // The *request* is fixed; the flag shape is not. A rebuild renders it
    // through its own profile so a Goose-to-Gemini retry gains
    // `--output-format json` and the reverse does not carry Gemini's bytes to a
    // provider that rejects them. Any unsupported-format warning is dropped:
    // there is no operator at a retry boundary, and the invocation already
    // surfaced its own.
    if let Some(format) = inputs.output_format {
        let _ = profile.apply_output_format(&mut args, format);
    }

    // System-prompt delivery: replayed verbatim while the provider holds still,
    // re-applied for the rebuilt provider when it does not. The artifacts the
    // re-application creates are temp resources that must outlive the child, so
    // they ride in the plan rather than being dropped here.
    let mut artifacts = Vec::new();
    let mut system_prompt_opencode_config = inputs.system_prompt_opencode_config.clone();
    if !provider_moved {
        args.extend(inputs.system_prompt_args.iter().cloned());
    } else if let Some(ResolvedSystemPrompt::Ready(prepared)) = inputs.system_prompt.as_ref() {
        let application = profile
            .apply_system_prompt(
                prepared,
                !facets.non_interactive,
                &inputs.system_prompt_cwd,
                &inputs.system_prompt_scoped_tmp,
            )
            .map_err(|e| LaunchPlanError::Producer(e.to_string()))?;
        args.extend(application.args);
        system_prompt_opencode_config = None;
        for (key, value) in application.env {
            if key == OsStr::new(OPENCODE_CONFIG) {
                system_prompt_opencode_config = Some(value.to_string_lossy().into_owned());
            } else if key != OsStr::new("HOME") {
                // `HOME` belongs to the invocation's shadow-home plan, which the
                // base child environment already carries; a re-application must
                // not repoint it.
                env_overlay.push((key, value));
            }
        }
        artifacts = application.artifacts;
    }

    if inputs.sandbox_requested {
        let _ = profile.apply_sandbox(&mut args);
    }

    // -- MCP runtime injection ---------------------------------------------
    let mcp_env = rebuild_mcp(inputs, facets, &mut args)?;

    // -- structured output --------------------------------------------------
    let use_structured = facets.use_structured();
    if use_structured {
        profile.apply_structured_stream(&mut args);
    }
    // Ask the production predicate whether Codex structured-output capture
    // applies rather than restating which provider it singles out; the scratch
    // argv it decorates (and the fresh temp path it would allocate) is discarded
    // in favour of the invocation's recorded sink, so an unchanged Codex run
    // replays to the same argv it recorded.
    let structured_codex = {
        let mut scratch: Vec<String> = Vec::new();
        crate::commands::exec_prep::prepare_codex_structured_output(
            facets.provider,
            use_structured,
            !facets.non_interactive && facets.is_inline,
            &mut scratch,
        )
        .is_some()
    };
    if structured_codex {
        args.push("--output-last-message".to_string());
        args.push(inputs.codex_last_message_path.to_string_lossy().into_owned());
    }
    // The harness owns prompt delivery, so the captured-output shaping the
    // harness base argv carries (`runner.rs`) belongs here too.
    if !use_structured {
        profile.prepare_captured_output(&mut args);
    }

    // -- the OpenCode inline-config chain ----------------------------------
    // Folded in the command phase's order — base, MCP, YOLO, system prompt — so
    // a rebuilt permission mode or server set lands on the same base the
    // invocation folded onto.
    if let Some(config) = fold_opencode_config(
        inputs,
        facets,
        &mcp_env,
        yolo_applied,
        system_prompt_opencode_config.as_deref(),
    )? {
        env_overlay.push((OPENCODE_CONFIG.into(), config.into()));
    }
    for (key, value) in mcp_env {
        if key != OPENCODE_CONFIG {
            env_overlay.push((key.into(), value.into()));
        }
    }

    Ok(LaunchPlan {
        args,
        env_overlay: patch_with_baseline_restores(inputs, credential_patch, env_overlay),
        yolo_applied,
        structured_codex,
        replayed: true,
        system_prompt_artifacts: artifacts,
    })
}

/// Turn the sets a replay produced into the full patch against the invocation's
/// base child environment.
///
/// Every provider-shaped key the invocation wrote that this rebuild did *not*
/// write again is stale by definition, so it is returned to its pre-provider
/// value — or removed, when it had none. `MODEL` on a document that dropped
/// `model:`, `OPENCODE_CONFIG_CONTENT` on a retry that left OpenCode, and a
/// shadow `HOME` on a retry that left the provider that needed one all land
/// here.
///
/// Restores are appended after the sets and sorted by key so the patch is
/// deterministic for the same inputs — the R8 comparison reads it.
///
/// `credential_prefix` leads the patch (see [`CredentialPolicyInputs`]); a
/// baseline restore never overwrites it, because restores skip keys the patch
/// already names.
fn patch_with_baseline_restores(
    inputs: &LaunchPlanInputs,
    credential_prefix: Vec<EnvChange>,
    sets: Vec<(OsString, OsString)>,
) -> Vec<EnvChange> {
    let mut patch: Vec<EnvChange> = credential_prefix;
    patch.extend(
        sets.into_iter()
            .map(|(key, value)| EnvChange::Set(key, value)),
    );
    let mut restores: Vec<EnvChange> = inputs
        .provider_env_baseline
        .iter()
        .filter(|(key, _)| !patch.iter().any(|change| change.key() == key.as_os_str()))
        .map(|(key, prior)| match prior {
            Some(value) => EnvChange::Set(key.clone(), value.clone()),
            None => EnvChange::Remove(key.clone()),
        })
        .collect();
    restores.sort_by(|a, b| a.key().cmp(b.key()));
    patch.extend(restores);
    patch
}

/// Recompute MCP runtime injection for the refreshed document's tag set,
/// appending the injector's `extra_args` and returning its environment.
fn rebuild_mcp(
    inputs: &LaunchPlanInputs,
    facets: &DocumentLaunchFacets,
    args: &mut Vec<String>,
) -> Result<HashMap<String, String>, LaunchPlanError> {
    let Some(mcp) = inputs.mcp.as_ref() else {
        return Ok(HashMap::new());
    };
    use claudine::mcp::catalog::McpCatalogStore;
    use claudine::mcp::inject::injector_for_provider;
    use claudine::mcp::session::compute_session_set;

    let catalog = McpCatalogStore::load()
        .map_err(|e| LaunchPlanError::Producer(format!("failed to load MCP catalog: {e}")))?;
    let session = compute_session_set(
        &catalog,
        mcp.repo_root.as_deref(),
        &mcp.explicit_use,
        &facets.mcp_body_tags,
        // Never interactive: a retry has no operator. An ambiguous tag the
        // invocation did not resolve stays ambiguous and is dropped.
        |tag, _tier, _candidates| mcp.ambiguity_resolutions.get(tag).cloned(),
    )
    .map_err(|e| LaunchPlanError::Producer(format!("MCP session error: {e}")))?;

    let injector = injector_for_provider(facets.provider)
        .ok_or(LaunchPlanError::NoMcpInjector(facets.provider))?;
    if session.servers.is_empty() {
        return Ok(HashMap::new());
    }
    let mut env: HashMap<String, String> = HashMap::new();
    let result = injector
        .inject(&session.servers, &mut env, mcp.shadow_home.as_deref())
        .map_err(|e| LaunchPlanError::Producer(format!("MCP injection failed: {e}")))?;
    args.extend(result.extra_args);
    Ok(env)
}

/// Fold the `OPENCODE_CONFIG_CONTENT` chain: invocation base, then the rebuilt
/// MCP overlay, then the rebuilt YOLO permission block, then the invocation's
/// system-prompt overlay.
fn fold_opencode_config(
    inputs: &LaunchPlanInputs,
    facets: &DocumentLaunchFacets,
    mcp_env: &HashMap<String, String>,
    yolo_applied: bool,
    system_prompt_config: Option<&str>,
) -> Result<Option<String>, LaunchPlanError> {
    let mut current: Option<String> = inputs.opencode_config_base.clone();
    let mut touched = current.is_some();

    let merge = |current: &mut Option<String>, overlay: serde_json::Value| {
        let merged = claudine::opencode_config::merge_overlay(
            current.as_deref().map(OsStr::new),
            overlay,
        )
        .map_err(|e| {
            LaunchPlanError::Producer(format!("failed to merge {OPENCODE_CONFIG}: {e}"))
        })?;
        *current = Some(merged);
        Ok::<(), LaunchPlanError>(())
    };

    if let Some(raw) = mcp_env.get(OPENCODE_CONFIG) {
        let overlay = serde_json::from_str(raw).map_err(|e| {
            LaunchPlanError::Producer(format!(
                "MCP injection produced invalid {OPENCODE_CONFIG}: {e}"
            ))
        })?;
        merge(&mut current, overlay)?;
        touched = true;
    }
    // Delegate to the production overlay rather than restating its gate, so the
    // rule for when a YOLO permission block belongs in the inline config lives in
    // exactly one place. The scratch plan is a carrier for the current value.
    {
        let mut scratch = super::env::EnvPlan::default();
        if let Some(value) = current.as_ref() {
            scratch
                .env
                .insert(OPENCODE_CONFIG.into(), OsString::from(value));
        }
        super::wrapper_stages::apply_opencode_yolo_config_overlay(
            facets.provider,
            yolo_applied,
            facets.non_interactive,
            &mut scratch,
        )
        .map_err(|e| LaunchPlanError::Producer(e.to_string()))?;
        if let Some(merged) = scratch.env.get(OsStr::new(OPENCODE_CONFIG)) {
            let merged = merged.to_string_lossy().into_owned();
            if current.as_deref() != Some(merged.as_str()) {
                current = Some(merged);
                touched = true;
            }
        }
    }
    if let Some(raw) = system_prompt_config {
        let overlay = serde_json::from_str(raw).map_err(|e| {
            LaunchPlanError::Producer(format!("invalid system-prompt {OPENCODE_CONFIG}: {e}"))
        })?;
        merge(&mut current, overlay)?;
        touched = true;
    }
    Ok(touched.then_some(current).flatten())
}

#[cfg(test)]
mod tests;
