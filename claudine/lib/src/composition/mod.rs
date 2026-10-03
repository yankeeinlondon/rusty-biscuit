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
pub mod authored_order;
pub mod closure;
pub mod completion;
pub mod coordinator;
mod error;
pub mod file_detail;
pub mod frontmatter_excerpt;
mod guardrails;
pub mod hints;
pub mod inline_prompt;
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

pub use agent_message::{agent_state_breakdown, invalid_agent_message};
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
    ActionExprError, CompositionError, DroppedOptional, DroppedOptionalSource,
    DroppedOptionalStage, FileReferenceContext, InteractiveShape, LOOP_RATE_LIMITED_EXIT_CODE,
    LifecycleEvaluationReason, LoopExpressionCause, MarkdownLoadCause, MissingProperty,
    SequenceExpressionCause,
    SequenceLoadCause, SequenceMissingPropertiesStep, SequenceSelectionFailure, SequenceShellCause,
    ShellApprovalFailure, TextFormat,
};
pub use file_detail::{FileDetail, extract_markdown_detail, extract_yaml_sequence_detail};
pub use frontmatter_excerpt::FrontmatterExcerpt;
pub use hints::{parse_interactive_hint, parse_selection_hints_from_frontmatter};
pub use guardrails::{document_path_span, native_document_path};
pub use launch_workspace::{LaunchWorkspaceContext, PackageContext};
pub use lifecycle::runtime::{
    IterationSummarySignals, LifecycleCatchExecution, LifecycleCatchProtocol, LifecycleCatchResult,
    LifecycleCatchState, LifecycleCatchStep, LifecycleTransitionAbort, LifecycleTransitionDecision,
    LifecycleTransitionError, LifecycleTransitionInput, decide_lifecycle_transition,
};
#[allow(deprecated)]
pub use lifecycle::{
    DefaultLifecycleEmitter, LIFECYCLE_EVENT_KEYS, LifecycleConfig, LifecycleEmitter,
    LifecycleNotification, LifecycleRunGuard, LifecycleRuntimeContext, LifecycleRuntimeState,
    LifecycleSignal, parse_lifecycle_config,
};
pub use lifecycle_actions::{
    CommunicationAction, CommunicationChannel, ExpressionFunctionAction, LifecycleAction,
    LifecycleActionKind, LifecycleControlAction, LifecycleStackItem, RetryBackoff, ShellAction,
    SideEffectAction, is_known_side_effect, side_effect_signature,
};
pub use lifecycle_context::{LifecycleErrorInfo, LifecycleTiming};
pub use lifecycle_control::{
    ControlDispatch, MAX_PROXY_HOPS, compute_backoff_delay, control_budget_for, decide_control,
    parse_delay, proxy_handoff_allowed, proxy_path_identity, resolve_proxy_target,
};
pub use lifecycle_executor::{
    LifecycleEventOutcome, LifecycleExprError, ShellRunError, ShellRunner, StackControl,
    StackExecutionContext, SystemShellRunner,
};
pub use looping::{
    DEFAULT_MAX_ITERATIONS, LoopExecutionOptions, LoopExecutionResult, LoopIterationContext,
    LoopIterationOutput, LoopSeed, build_loop_seed, build_loop_seed_from_bootstrap,
    build_loop_seed_with_lifecycle, execute_loop_with_lifecycle,
};
pub use looping::{LoopAmbient, LoopExpressionLookup, evaluate_condition};
pub use looping::{
    extract_control_variables, resolve_fail_fast_from_env, resolve_loop_config,
    resolve_max_iterations_from_env, resolve_pause_reset_margin_from_env,
};
pub use mismatch::{capture_frontmatter_yaml, is_inline_sequence_mismatch};
pub use preflight::{
    PreFlightResult, resolve_graph_shell_approvals, resolve_lifecycle_shell_approvals,
    resolve_shell_approvals,
};
pub use prepare::{
    BootstrapPreparation, BootstrapRequest, DocumentEntryReason, DocumentPreparation,
    LoopOwnership, PreparationStages, PrepareOptions, PromptSource, SchemaStage, SourceBasis,
    approve_document_shell, bind_agent_workspace, preflight_bootstrap_shell, preflight_document_shell,
    prepare_bootstrap, prepare_direct, prepare_document, prepare_inline,
};
#[cfg(test)]
pub(crate) use resolve::resolve_fixture_source;
pub use resolve::{
    build_prompt_reference, build_prompt_resolution_context, capture_file_resolution_context,
    derive_request_context_for_source,
    enrich_composition_source_load_error_in_context,
    is_yaml_source, load_yaml_document, prompt_magic_fallback_roots, prompt_magic_roots,
    reload_composition_source,
    resolve_composition_source, resolve_composition_source_in_context, validate_file_permissions,
    with_prompt_magic_roots, without_formal_sequence_keys,
};
pub use runtime_state::{
    LayeredOverrides, OUTPUTS_KEY, RuntimeMutationError, RuntimeSnapshot, RuntimeState,
    layered_set_overrides, trim_transport_newline,
};
pub use schema::{
    InteractiveSchemaOptions, authored_schema_parameters, PreValidatedSchema, PropertyState, PropertyStatus,
    SchemaStatusReport, build_schema_status_report, build_schema_status_report_for_mode,
    description_suffix, drop_invalid_optionals, escape_schema_prose, launch_phase_for_mode,
    pre_validate_layered_for_mode, pre_validate_schema, pre_validate_schema_for_mode,
    prepare_direct_with_schema,
    prepare_direct_with_schema_and_prompt, prepare_inline_with_schema, render_optional_line,
    render_required_line, schema_status_report_prose,
};
pub use select::{
    ambient_env_lookup, build_candidate_set, build_installed_snapshot, build_picker_plan,
    build_picker_plan_with_hints, classify_agent_resolution, detect_installed_providers,
    resolve_model, resolve_model_with_catalog, resolve_model_with_hints,
    resolve_model_with_hints_from, resolve_target_non_tty, resolve_target_non_tty_with_catalog,
    resolve_target_non_tty_with_hints, select_provider,
};
pub use sequence::preflight::{
    DiscoveredCommand, GroupExecution, PreflightAction, PreflightGraph, PreflightGroup,
    PreflightStep, PreflightTask, PromptDocument, build_preflight_graph,
    build_preflight_graph_with_context,
    build_preflight_graph_with_invocation, reject_non_sequence_kind,
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
    AgentHint, AgentResolutionState, AmbientVariable, CallerInputLayers, CompositionClosurePlan,
    CompositionExecutionRequest, CompositionMode, EffectiveSelectionHints, InlineClosurePlan,
    InstalledProviderSnapshot, LaunchSchema, LoopAction, LoopCondition, LoopConfig, ModelHint,
    ModelResolutionReason, OnRateLimit, OutputFormat, PickerInfluence, PreparedComposition,
    ProviderPickerOption, ProviderPickerPlan, ProviderResolutionReason, ResolutionMode,
    ResolvedCompositionSource, ResolvedExecutionTarget, ResolvedSessionInteractivity,
    SelectedProvider, SelectionReason, SequenceExecutionOptions, SequenceRunSummary,
    SequenceStepDraft, SequenceStepResult, SequenceTaskResult, SessionInteractivitySource,
    SharedApprovalCache,
};

/// Pairs `options` with the request's `file_resolution_context`.
///
/// Darkmatter aligns the options' `ctx.*` environment with the context's, so
/// the variables Claudine layers onto the captured `ctx` for a run (`AGENT`,
/// `MODEL`, `YOLO`, and other `env_overrides`) are added to the context's
/// environment first. An expression, `env.*`, and a `{{VAR}}` file reference
/// then read one environment, and none of the run's variables is lost.
///
/// ## Errors
///
/// Returns the [`ContextBuildError`](darkmatter::markdown::compose::ContextBuildError)
/// when `file_resolution_context` fails validation.
pub fn compose_request(
    options: darkmatter::markdown::compose::ComposeOptions,
    file_resolution_context: biscuit_file::FileResolutionContext,
) -> Result<darkmatter::markdown::compose::ComposeRequest, darkmatter::markdown::compose::ContextBuildError>
{
    let layered = options.context().env();
    let context = if layered
        .iter()
        .all(|(key, value)| file_resolution_context.env().get(key) == Some(value))
    {
        file_resolution_context
    } else {
        let mut env = file_resolution_context.env().clone();
        env.extend(layered.iter().map(|(key, value)| (key.clone(), value.clone())));
        file_resolution_context.with_env(env)
    };
    darkmatter::markdown::compose::ComposeRequest::with_context(options, context)
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
    let request = compose_request(options, file_resolution_context.clone())?;
    Ok(request.local_expression_resolution_context())
}
