//! The staged boot a live document takes when it authors `initialize`.
//!
//! `initialize` may create the very files the document's body includes, so no
//! body dependency may be dereferenced before it runs. The command coordinator
//! (`compose/prep.rs`) therefore boots such a document in order:
//!
//! 1. the **bootstrap read** — frontmatter and lifecycle surface only
//!    ([`bootstrap_document`]);
//! 2. reject initialization shell actions and bootstrap shell expansion;
//! 3. `initialize`, through the pipeline's ordinary initialize route
//!    ([`route_staged_initialize`]);
//! 4. the **stabilized reread** — a fresh read of the document `initialize`
//!    left on disk, with the caller's layers, overlay, and epoch reapplied
//!    ([`reread_and_audit`]);
//! 5. the full body audit and canonical preparation.
//!
//! Only then is a `PreparedComposition` built and handed to the pipeline. A
//! document without `initialize` keeps its eager preparation.

// `CompositionError` is intentionally large (it carries frontmatter excerpts);
// the composition execution path returns it and opts out of the
// `result_large_err` lint the same way (see `harness_orch/prompt.rs`).
#![allow(clippy::result_large_err)]

use std::collections::{BTreeMap, HashSet};
use std::io::IsTerminal;
use std::path::Path;
use std::time::Instant;

use biscuit_terminal::terminal::Terminal;
use claudine::composition::lifecycle::{
    DefaultLifecycleEmitter, LifecycleConfig, LifecycleRunGuard, LifecycleSignal,
};
use claudine::composition::lifecycle_executor::{StackExecutionContext, SystemShellRunner};
use claudine::composition::{
    BootstrapPreparation, BootstrapRequest, CompositionError, CompositionMode,
    DocumentEntryReason, DocumentTransition, LifecycleErrorInfo, PrepareOptions,
    ResolvedCompositionSource, SharedRunLedger, SurfacedHandoff,
};
use claudine::invocation_context::{DocumentEpoch, InvocationContext};
use claudine::provider::Provider;
use color_eyre::eyre::Result;
use darkmatter::effects::EffectEngine;
use darkmatter::markdown::compose::{ComposeContext, ContextRequirements};

use super::pipeline::{
    CompositionPhaseResult, InitializeCatchSurface, commit_initialize_proxy, route_initialize,
};
use super::preflight::PreflightBlockedOutcome;

/// Whether a document authors `initialize` — the one predicate that selects the
/// staged boot, and the deferral of its schema verdict to the stabilized reread.
///
/// R4 puts `initialize` before body discovery and schema validation precisely
/// so a document can create an included file or add or repair a schema
/// property from its own bootstrap. A document that declares no `initialize`
/// has nothing to wait for and is read and judged eagerly.
///
/// Keyed on the *authored* key, not on the parsed lifecycle: a malformed or
/// empty `initialize:` must still stage, because the lifecycle parse that
/// rejects it belongs to the bootstrap read, and something downstream owes the
/// verdict.
pub(crate) fn authors_initialize(source: &ResolvedCompositionSource) -> bool {
    source
        .markdown
        .frontmatter()
        .as_map()
        .contains_key("initialize")
}

/// Take a shell-free bootstrap read. The empty approval set is extended only
/// by the stabilized reread after initialization.
///
/// ## Errors
///
/// A forbidden frontmatter expansion or `initialize` shell action, or a frontmatter-side
/// preparation failure. None of these is routed through the document's own
/// catch stacks: they happen before its lifecycle is installed.
pub(crate) fn bootstrap_document(
    source: &ResolvedCompositionSource,
    mode: CompositionMode,
    entry: DocumentEntryReason,
    mut options: PrepareOptions,
    approval_options: &claudine::harness::ShellApprovalOptions,
) -> Result<(BootstrapPreparation, HashSet<String>), CompositionError> {
    let approved =
        claudine::composition::preflight_bootstrap_shell(source, mode, &options, approval_options)?;
    options.pre_approved_commands = Some(approved.clone());
    let bootstrap = claudine::composition::prepare_bootstrap(BootstrapRequest {
        entry,
        mode,
        source,
        options,
    })?;
    Ok((bootstrap, approved))
}

/// The lifecycle bindings a staged document's `initialize` runs against.
pub(crate) struct StagedLifecycleRuntime {
    pub settings: claudine::events::GlobalSettings,
    pub messaging: claudine::messaging::RuntimeMessagingSettings,
    pub effect_engine: EffectEngine,
    /// The document epoch's snapshot, extended for the groups the effective
    /// frontmatter names, with the target identity overrides applied.
    pub context: ComposeContext,
}

impl StagedLifecycleRuntime {
    pub(crate) fn new(
        bootstrap: &BootstrapPreparation,
        env_overrides: &BTreeMap<String, String>,
        repo_root: Option<&Path>,
        mutation_fallback: &Path,
    ) -> Self {
        let (settings, messaging) = lifecycle_settings(&bootstrap.lifecycle, repo_root);
        let effect_engine = EffectEngine::builder()
            .mutation_root(repo_root.unwrap_or(mutation_fallback))
            .auto_rehash(false)
            .build();
        let mut context = bootstrap.compose_context.clone();
        if let Some(epoch) = bootstrap.document_epoch.as_ref() {
            epoch.record_prepared_context_consumer(
                claudine::invocation_context::PreparedContextConsumer::Lifecycle,
            );
            let scan = serde_json::to_string(&bootstrap.effective_frontmatter).unwrap_or_default();
            epoch.extend_launch_context(&mut context, &ContextRequirements::for_content(&scan));
        }
        for (key, value) in env_overrides {
            context.env_mut().insert(key.clone(), value.clone());
        }
        Self {
            settings,
            messaging,
            effect_engine,
            context,
        }
    }
}

/// The user/repo lifecycle settings, loaded only when the document configures
/// lifecycle notifications at all.
pub(crate) fn lifecycle_settings(
    lifecycle: &LifecycleConfig,
    repo_root: Option<&Path>,
) -> (
    claudine::events::GlobalSettings,
    claudine::messaging::RuntimeMessagingSettings,
) {
    let unconfigured = || {
        (
            claudine::events::GlobalSettings::default(),
            claudine::messaging::RuntimeMessagingSettings {
                user: None,
                repo: None,
            },
        )
    };
    if lifecycle.is_empty() {
        return unconfigured();
    }
    match claudine::dispatch::loader::load_claudine_config(None, repo_root) {
        Ok(config) => (
            claudine::dispatch::loader::bridge_tts_settings(&config),
            claudine::dispatch::loader::bridge_messaging_settings(&config),
        ),
        Err(_) => unconfigured(),
    }
}

/// Where a staged document runs its `initialize` and catches its failures.
pub(crate) struct StagedDocument<'a> {
    pub bootstrap: &'a BootstrapPreparation,
    pub settings: &'a claudine::events::GlobalSettings,
    pub messaging: &'a claudine::messaging::RuntimeMessagingSettings,
    pub effect_engine: &'a EffectEngine,
    pub context: &'a ComposeContext,
    pub emitter: &'a DefaultLifecycleEmitter,
    pub term: &'a Terminal,
    pub repo_root: Option<&'a Path>,
    pub launch_area: &'a Path,
    pub document_start: Instant,
}

impl StagedDocument<'_> {
    fn catch_surface<'s>(
        &'s self,
        frontmatter: &'s serde_json::Map<String, serde_json::Value>,
    ) -> InitializeCatchSurface<'s> {
        InitializeCatchSurface {
            effect_engine: self.effect_engine,
            emitter: self.emitter,
            settings: self.settings,
            messaging: self.messaging,
            term: self.term,
            source_path: &self.bootstrap.resolved_path,
            repo_root: self.repo_root,
            launch_area: self.launch_area,
            context: self.context,
            file_resolution_context: self.bootstrap.input_layers.file_resolution_context.as_ref(),
            frontmatter,
            document_start: self.document_start,
        }
    }
}

/// What a staged document's `initialize` decided.
pub(crate) enum StagedInitialize {
    /// Continue to the stabilized reread.
    Proceed,
    /// A clean `skip`: the run ends successfully with no further events.
    Skipped,
    /// A proxy, already committed against the invocation ledger.
    Handoff(SurfacedHandoff),
}

/// Emit a staged document's `initialize` through the pipeline's initialize
/// route and commit a proxy it selects.
///
/// `initialize` runs with the process CWD at `child_cwd`, where the pipeline
/// route runs it, and the launch area is restored afterwards so the stabilized
/// reread is anchored exactly like the bootstrap read.
///
/// ## Errors
///
/// An `initialize` `error`, evaluation error, setup-phase recovery refusal, or a
/// refused proxy hop, each surfaced with the pipeline route's semantics.
pub(crate) fn route_staged_initialize(
    guard: &mut LifecycleRunGuard<'_>,
    document: &StagedDocument<'_>,
    child_cwd: &Path,
    provider: Provider,
    ledger: &SharedRunLedger,
    invocation: Option<&InvocationContext>,
) -> Result<StagedInitialize> {
    let empty = serde_json::Map::new();
    let frontmatter = document
        .bootstrap
        .effective_frontmatter
        .as_object()
        .unwrap_or(&empty);
    let surface = document.catch_surface(frontmatter);
    let current = claudine::composition::lifecycle_context::LifecycleCurrent::capture_at_event(
        document.launch_area,
    );
    let timing = claudine::composition::lifecycle_context::LifecycleTiming::from_instants(
        document.document_start,
        None,
        Instant::now(),
    );
    let init_ctx = StackExecutionContext {
        signal: LifecycleSignal::Initialize,
        frontmatter,
        live_frontmatter: None,
        runtime_state: None,
        err: None,
        timing: Some(&timing),
        current: Some(&current),
        group: None,
        base_dir: surface.source_path.parent().or(document.repo_root),
        ctx_base_dir: Some(document.launch_area),
        prepared_context: Some(surface.context),
        file_resolution_context: surface.file_resolution_context,
        effect_engine: surface.effect_engine,
        shell_runner: &SystemShellRunner,
        emitter: document.emitter,
        term: document.term,
        source_path: surface.source_path,
        repo_root: document.repo_root,
        messaging: surface.messaging,
        settings: surface.settings,
    };

    super::switch_process_cwd(child_cwd)?;
    let routed = route_initialize(
        guard,
        &init_ctx,
        surface.source_path,
        provider,
        document.term,
    );
    let _ = std::env::set_current_dir(document.launch_area);
    match routed {
        CompositionPhaseResult::Proceed(DocumentTransition::Proxy(request)) => Ok(
            StagedInitialize::Handoff(commit_initialize_proxy(
                guard, ledger, request, invocation, &surface,
            )?),
        ),
        CompositionPhaseResult::Proceed(_) => Ok(StagedInitialize::Proceed),
        CompositionPhaseResult::Completed(_) => Ok(StagedInitialize::Skipped),
        CompositionPhaseResult::Blocked(error) | CompositionPhaseResult::Failed(error) => {
            Err(error)
        }
    }
}

/// The stabilized reread: read the document `initialize` left on disk, reapply
/// the overlay, and run the full body audit against it.
///
/// `options` are the options the bootstrap read was assembled from — caller
/// layers, file-resolution context, epoch, and snapshot — so nothing is
/// recaptured. The retained snapshot is only extended for context groups the
/// rewritten document newly names. Approvals accumulate: `approved` (from the
/// bootstrap) plus every command the body audit approves.
///
/// ## Errors
///
/// A load failure or a body shell denial. A body dependency that is still
/// missing is not an error here; canonical preparation reports it with its
/// typed compose diagnostic.
pub(crate) fn reread_and_audit(
    path: &Path,
    original_ref: &str,
    overlay: &indexmap::IndexMap<String, serde_json::Value>,
    mut options: PrepareOptions,
    approved: HashSet<String>,
    approval_options: &claudine::harness::ShellApprovalOptions,
) -> Result<(ResolvedCompositionSource, PrepareOptions), CompositionError> {
    let source = crate::commands::wrap::load_overlaid_document(path, original_ref, overlay)?;
    extend_epoch_context(&mut options, &source);
    // This read owns the verdict the bootstrap withheld.
    options.defer_schema_verdict = false;
    options.pre_approved_commands = Some(approved);
    let body_approved =
        claudine::composition::preflight_document_shell(&source, &options, approval_options)?;
    if let Some(set) = options.pre_approved_commands.as_mut() {
        set.extend(body_approved);
    }
    Ok((source, options))
}

fn extend_epoch_context(options: &mut PrepareOptions, source: &ResolvedCompositionSource) {
    let requirements = ContextRequirements::for_document(&source.markdown);
    let epoch: Option<&DocumentEpoch> = options.document_epoch.as_ref();
    if let (Some(context), Some(epoch)) = (options.prepared_context.as_mut(), epoch) {
        epoch.extend_launch_context(context, &requirements);
        for (key, value) in &options.env_overrides {
            context.env_mut().insert(key.clone(), value.clone());
        }
    }
}

/// Route a stabilized-reread failure through the staged document's own
/// `blocked`/`finalize`, exactly once, and return the diagnostic to surface.
///
/// From `initialize` on the document owns the run, so a body dependency that
/// is still missing, a body shell denial, or a schema verdict is an ordinary
/// blocked attempt. A catch stack's evaluation error outranks the cause.
pub(crate) fn route_stabilized_failure(
    guard: &mut LifecycleRunGuard<'_>,
    document: &StagedDocument<'_>,
    error: CompositionError,
) -> CompositionError {
    let empty = serde_json::Map::new();
    let frontmatter = document
        .bootstrap
        .effective_frontmatter
        .as_object()
        .unwrap_or(&empty);
    let info = LifecycleErrorInfo::from_composition_error(&error);
    match document.catch_surface(frontmatter).route_blocked(guard, info) {
        PreflightBlockedOutcome::EvaluationError(evaluation) => evaluation,
        PreflightBlockedOutcome::Control(_) => {
            let stderr_is_tty =
                std::io::stderr().is_terminal() || std::env::var_os("FORCE_COLOR").is_some();
            let text = std::fs::read_to_string(&document.bootstrap.resolved_path)
                .unwrap_or_default();
            error.enrich_frontmatter_text(&text, stderr_is_tty)
        }
    }
}
