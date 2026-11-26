//! Composition services for inline and chained document workflows.
//!
//! This module provides the shared logic for Claudine's composition features:
//! - File reference resolution via `biscuit-file::FileReference`
//! - Prompt preparation (inline frontmatter prompt and chained document)
//! - Agent/provider selection with precedence rules
//! - Composition-specific error types
//!
//! See [`claudine/docs/topics/composition.md`](../../../docs/topics/composition.md)
//! for the authoritative design — frontmatter precedence, harness
//! validations, handlers, and provider selection across `compose`,
//! `inline-compose`, and `sequence`.

use std::path::Path;

pub mod agent_message;
pub use darkmatter::lifecycle::authored_order;
pub mod closure;
pub mod completion;
pub mod coordinator;
mod error;
pub mod file_detail;
pub mod frontmatter_excerpt;
mod guardrails;
pub mod hints;
pub mod inline_prompt;
pub mod inventory;
#[cfg(test)]
pub(crate) mod interpolation_conformance;
pub(crate) mod json_util;
pub mod launch_workspace;
pub mod lifecycle;
pub use self::lifecycle::actions as lifecycle_actions;
pub use self::lifecycle::context as lifecycle_context;
pub use self::lifecycle::control as lifecycle_control;
pub use self::lifecycle::executor as lifecycle_executor;
pub mod looping;
pub mod mismatch;
pub mod preflight;
mod prepare;
pub mod provider_tail;
pub mod registration;
pub mod ownership;
mod reserved;
mod resolve;
pub mod runtime_state;
pub mod schema;
mod select;
pub mod sequence;
mod types;
pub use ownership::{
    ARGV_KEY, ArgumentOwner, ArgumentsAfterFile, CallerArgument, OwnedArguments,
    OwnershipCandidate, OwnershipError, SchemaParameters, TailMismatch, check_launch_tail,
    is_setter_name, own_arguments, owner_of_last_argument, setter_key,
};
pub use provider_tail::{ProviderTail, ProviderTailNotices, SwitchAssignment};

pub use agent_message::{agent_message_prose, agent_state_breakdown, invalid_agent_message};
pub use authored_order::AuthoredOrder;
pub use closure::{
    AgentFrontmatterRejection, BodyRejection, CLOSURE_OWNED_PROPERTIES, EncodeError, InlineArtifact,
    InlineReconciliation,
    reconcile_inline_artifact, reconcile_inline_artifact_with_evidence, restore_inline_baseline,
};
pub use completion::{
    BodyEvidence, CompletionContext, CompletionOutcome, CompletionProblem, CompletionProblemKind,
    CompletionVerdict, complete_active_document, evaluate_completion,
};
pub use coordinator::{
    ActionLocation, ActiveDocumentState, AttemptOutcome, CommandOutputPolicy, ControlBudget,
    ControlBudgetKind, DocumentIteration, DocumentOverlay, DocumentTransition,
    EvaluatedProxyRequest, HopApproval, HopRejection, InvocationInputs, InvocationInputsDraft,
    LaunchDiscovery, LedgerMut, PreparedDocument, ProviderAttempt, ProxyCommitError, ProxyHandoff,
    ProxyProvenance, ResolvedProxyTarget, RunLedger, SessionCompatibilityKey, SharedRunLedger,
    SurfacedHandoff, TransitionAbort, TransitionRecord, commit_proxy,
};
pub use darkmatter::markdown::compose::shell_expansion::{ShellCommandOrigin, ShellExpansionError};
pub use error::{
    ActionExprError, CompositionError, DroppedOptional, DroppedOptionalSource, SequenceTaskProviderProblem,
    DroppedOptionalStage, FileReferenceContext, InteractiveShape, LOOP_RATE_LIMITED_EXIT_CODE,
    LifecycleEvaluationReason, LoopExpressionCause, MarkdownLoadCause, MissingProperty,
    SequenceExpressionCause,
    SequenceLoadCause, SequenceMissingPropertiesEntry, SequenceMissingSubject, SequenceSelectionFailure, SequenceShellCause,
    ShellApprovalFailure, TextFormat,
};
pub use file_detail::{FileDetail, extract_markdown_detail, extract_yaml_sequence_detail};
pub use frontmatter_excerpt::FrontmatterExcerpt;
pub use hints::{parse_interactive_hint, parse_selection_hints_from_frontmatter};
pub use guardrails::{document_path_span, native_document_path};
pub use launch_workspace::{LaunchWorkspaceContext, PackageContext};
pub use lifecycle::runtime::{
    EvaluationMode, IterationSummarySignals, LifecycleCatchExecution, LifecycleCatchProtocol,
    LifecycleCatchProtocolExt, LifecycleCatchResult,
    LifecycleCatchState, LifecycleCatchStep, LifecycleTransitionAbort, LifecycleTransitionDecision,
    LifecycleTransitionError, LifecycleTransitionInput, decide_lifecycle_transition,
};
#[allow(deprecated)]
pub use lifecycle::{
    ClaudineRunHost, DefaultLifecycleEmitter, LifecycleConfig, LifecycleEmitter,
    LifecycleRunGuard, LifecycleRunGuardExt, LifecycleRuntimeContext, LifecycleRuntimeState, LifecycleSignal,
    claudine_registry, parse_lifecycle_config,
};
pub use lifecycle_actions::{
    CommunicationAction, CommunicationChannel, ExpressionFunctionAction, LifecycleAction,
    LifecycleActionKind, LifecycleConditional, LifecycleControlAction, LifecycleNode,
    LifecyclePath, RetryBackoff, ShellAction,
    SideEffectAction, is_known_side_effect, side_effect_signature,
};
pub use lifecycle_context::{LifecycleErrorInfo, LifecycleErrorInfoExt, LifecycleTiming};
pub use lifecycle_control::{
    ControlDispatch, MAX_PROXY_HOPS, compute_backoff_delay, control_budget_for, decide_control,
    parse_delay, proxy_handoff_allowed, proxy_path_identity, resolve_proxy_target,
};
pub use lifecycle_executor::{
    DocumentShellRunner, LifecycleEventOutcome, LifecycleExprError, ShellRunError, ShellRunner,
    StackControl, StackExecutionContext, SystemShellRunner,
};
pub use looping::{
    DEFAULT_MAX_ITERATIONS, LoopExecutionOptions, LoopExecutionResult, LoopIterationContext,
    LoopIterationOutput, LoopSeed, build_loop_seed, build_loop_seed_from_bootstrap,
    build_loop_seed_with_lifecycle, execute_loop_with_lifecycle,
};
pub use looping::{LoopAmbient, LoopExpressionLookup, evaluate_condition};
pub use looping::{
    LoopBlock, check_loop_config_keys, extract_control_variables, parse_loop_block,
    resolve_loop_config, resolve_max_iterations_from_env, resolve_pause_reset_margin_from_env,
    resolve_templated_loop_max,
};
pub use mismatch::{capture_frontmatter_yaml, is_inline_sequence_mismatch};
pub use preflight::{
    PreFlightResult, resolve_graph_shell_approvals, resolve_lifecycle_shell_approvals,
    resolve_shell_approvals,
};
pub use prepare::{
    BootstrapPreparation, BootstrapRequest, DocumentEntryReason, DocumentPreparation,
    LoopOwnership, PreparationStages, PrepareOptions, PromptSource, SchemaStage, SourceBasis,
    approve_document_shell, validate_document_shell_late_values, bind_agent_workspace, preflight_bootstrap_shell, preflight_document_shell,
    prepare_bootstrap, prepare_direct, prepare_document, prepare_document_selecting, prepare_inline, prepare_loop_seed, prepare_sequence_probe,
    read_selection_hints, replace_env_overrides, ResolvedIdentitySelector,
};
#[cfg(test)]
pub(crate) use resolve::resolve_fixture_source;
pub use resolve::{
    build_prompt_reference, build_prompt_resolution_context, capture_file_resolution_context,
    derive_request_context_for_source,
    enrich_composition_source_load_error_in_context,
    is_yaml_source, load_yaml_document, prompt_magic_fallback_roots, prompt_magic_roots,
    reload_authored_source, reload_composition_source,
    resolve_composition_source, resolve_composition_source_in_context, validate_file_permissions,
    with_prompt_magic_roots, without_formal_sequence_keys,
};
pub use runtime_state::{
    LayeredOverrides, OUTPUTS_KEY, RuntimeMutationError, RuntimeSnapshot, RuntimeState,
    layered_set_overrides, trim_transport_newline,
};
pub use schema::{
    InteractiveSchemaOptions, TriggerFindings, TriggerWarning, authored_schema_parameters, PreValidatedSchema, PropertyState, PropertyStatus,
    SchemaStatusReport, build_schema_status_report, build_schema_status_report_for_mode,
    description_suffix, drop_invalid_optionals, escape_schema_prose, launch_phase_for_mode,
    pre_validate_layered_for_mode, pre_validate_schema, pre_validate_schema_for_mode,
    prepare_direct_with_schema, prepare_direct_with_schema_selecting,
    prepare_direct_with_schema_and_prompt, prepare_inline_with_schema,
    prepare_inline_with_schema_selecting, render_optional_line,
    render_required_line, schema_status_report_prose,
};
pub use select::{
    build_candidate_set, build_installed_snapshot, build_picker_plan, prepared_environment,
    build_picker_plan_with_hints, classify_agent_resolution, detect_installed_providers,
    resolve_model, resolve_model_with_catalog, resolve_model_with_hints,
    resolve_model_with_hints_from, resolve_target_non_tty, resolve_target_non_tty_with_catalog,
    resolve_target_non_tty_with_hints, select_provider,
};
pub use sequence::preflight::{
    DiscoveredCommand, GroupExecution, PreflightAction, PreflightGraph, PreflightGroup,
    PreflightStep, PreflightTask, PreparedShellSite, PromptDocument, PromptReference,
    ShellSiteKind, SiteMode, TaskPosition, build_preflight_graph,
    build_preflight_graph_with_context, build_preflight_graph_with_invocation,
    build_preflight_graph_after_discovery, reread_preflight_graph_with_invocation, discover_sequence_inputs, InlineContainer, InputDiscoveryOptions,
    InputOwner, InputOwnerRole, InputPrompt, PendingReference, SequenceInputInventory,
    reject_non_sequence_kind, resolve_task_late_values, sequence_deferred_roots, validate_lifecycle_literals,
    providers::collect_written_keys,
    CheckFailure, CheckedTask, RootList, SequenceCheck, SequenceCheckRequest, SiteFrame, TaskSite, check_sequence_tree,
};
pub use sequence::task::{
    DEFAULT_COMMAND_TIMEOUT, PromptRunOutcome, PromptTaskRequest, PromptTaskRunner, RunawayTrip,
    RunawayTripKind, ShellCommandOutput, SystemTaskShell, TaskDiagnostic, TaskExecution,
    TaskOutcome, TaskShellError, TaskShellRunner, TaskStage, TaskStatus, UnavailablePromptRunner,
};
pub use sequence::{
    ExecutableField, ExternalTaskRef, GroupRef, OutputEntry, RuntimeMutation, SequencePlan,
    SequenceReference, SequenceSource, SequenceSourceOptions, SequenceSourceSpec, SequenceStep,
    SequenceStepOverlay, ShellSourceRunner, SourceOperator, StepExecutable, StepState, Strictness,
    build_step_overlay, resolve_sequence_plan, resolve_sequence_plan_with,
};
pub use types::{
    AgentHint, AnchoredOverlay, AgentResolutionState, AmbientVariable, CallerInputLayers, CompositionClosurePlan,
    CompositionExecutionRequest, CompositionMode, EffectiveSelectionHints, InlineClosurePlan, LaunchLog,
    InstalledProviderSnapshot, LaunchSchema, LoopAction, LoopCondition, LoopConfig, LoopLocation,
    LoopOwner, ModelHint,
    ModelResolutionReason, OnRateLimit, PickerInfluence, PreparedComposition,
    ProviderPickerOption, ProviderPickerPlan, ProviderResolutionReason, ResolutionMode,
    ResolvedCompositionSource, ResolvedExecutionTarget, ResolvedSessionInteractivity,
    SelectedProvider, SelectionReason, SequenceExecutionOptions, SequenceRunSummary,
    SequenceStepDraft, SequenceStepResult, SequenceTaskResult, SessionInteractivitySource,
    SharedApprovalCache,
};

// The variables Claudine layers onto the captured `ctx` for a run (`AGENT`,
// `MODEL`, `YOLO`, and other `env_overrides`) reach the request's environment
// through this one function, which the lifecycle executor's `set` shell values
// also use.
pub use darkmatter::lifecycle::executor::compose_request;

/// The `ComposeOptions` every Claudine composition of a document starts from.
///
/// Remote transclusion (`::file`/`::code` URL targets and `prologue`/
/// `epilogue` URLs) is switched on, as `md compose` switches it on: whether a
/// URL is fetched is decided by the request's captured `network` permission
/// (deny unless every layer allows), which also governs read-side URL
/// functions. Left off, those targets fail even under `network: allow`. A
/// caller may still turn it, or any operation, off for one preparation.
#[must_use]
pub fn compose_options_with_context(
    context: darkmatter::markdown::compose::ComposeContext,
) -> darkmatter::markdown::compose::ComposeOptions {
    darkmatter::markdown::compose::ComposeOptions::new_with_context(context).with_allow_remote_transclusion(true)
}

/// `options` with `invocation`'s Darkmatter configuration for `context`,
/// when neither the options nor their captured `ctx` supply one already (a
/// context captured outside the invocation carries no configuration capture).
///
/// ## Errors
///
/// [`ContextBuildError::Config`](darkmatter::markdown::compose::ContextBuildError::Config)
/// when the configuration does not load.
pub fn with_invocation_config(
    options: darkmatter::markdown::compose::ComposeOptions,
    invocation: Option<&crate::invocation_context::InvocationContext>,
    context: &biscuit_file::FileResolutionContext,
) -> Result<darkmatter::markdown::compose::ComposeOptions, darkmatter::markdown::compose::ContextBuildError> {
    let Some(invocation) = invocation else { return Ok(options) };
    if options.config_snapshot().is_some() || options.context().config_source().is_some() {
        return Ok(options);
    }
    invocation
        .darkmatter_config_for(context)
        .map(|snapshot| options.with_config_snapshot(snapshot))
        .map_err(|source| darkmatter::markdown::compose::ContextBuildError::Config {
            dir: context.request_cwd().to_path_buf(),
            source,
        })
}

/// Builds Darkmatter's local expression context from the request's
/// file-resolution context for the document that authored the expression.
///
/// ## Errors
///
/// Returns the [`ContextBuildError`](darkmatter::markdown::compose::ContextBuildError)
/// when `file_resolution_context` fails validation.
pub fn document_expression_resolution_context(
    source_path: &Path,
    prepared_context: Option<&darkmatter::markdown::compose::ComposeContext>,
    file_resolution_context: &biscuit_file::FileResolutionContext,
    file_ref_fallback_dir: Option<&Path>,
) -> Result<
    darkmatter::markdown::compose::expression::ResolutionContext,
    darkmatter::markdown::compose::ContextBuildError,
> {
    // The no-snapshot fallback is demand-driven, matching the executor's
    // `early_binding_context`. `ComposeContext::capture` runs the repo-wide
    // sniff scan — git, repo, file changes, languages, docs, OS, hardware, GPU
    // — and this is called once per evaluated expression argument, so an
    // eager capture here costs ~1.5s *per argument* on a large working tree.
    // Groups an expression actually reads are still captured on demand during
    // evaluation.
    let context = prepared_context.cloned().unwrap_or_else(|| {
        let base = file_ref_fallback_dir
            .or_else(|| source_path.parent())
            .unwrap_or_else(|| file_resolution_context.cwd());
        darkmatter::markdown::compose::ComposeContext::capture_for_content(base, "")
    });
    let mut options = darkmatter::markdown::compose::ComposeOptions::new_with_context(context)
        .with_source_file(source_path);
    if let Some(fallback) = file_ref_fallback_dir {
        options = options.with_file_ref_fallback_dir(fallback);
    }
    // A local-only context consults no configuration, so no request (and no
    // configuration snapshot) is built for it.
    darkmatter::lifecycle::executor::local_expression_context(options, file_resolution_context.clone())
}
