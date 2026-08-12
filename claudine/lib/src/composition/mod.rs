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
pub mod closure;
pub mod coordinator;
mod error;
pub mod file_detail;
pub mod frontmatter_excerpt;
mod guardrails;
pub mod hints;
#[cfg(test)]
mod interpolation_conformance;
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
mod resolve;
mod reserved;
pub mod runtime_state;
pub mod schema;
mod select;
pub mod sequence;
mod types;

pub use agent_message::{agent_state_breakdown, invalid_agent_message};
pub use coordinator::{
    ActionLocation, ActiveDocumentState, AttemptOutcome, CommandOutputPolicy, ControlBudget,
    ControlBudgetKind, DocumentIteration, DocumentOverlay, DocumentTransition,
    EvaluatedProxyRequest, HopApproval, HopRejection, InvocationInputs, InvocationInputsDraft,
    LaunchDiscovery, LedgerMut, PreparedDocument, ProviderAttempt, ProxyCommitError, ProxyHandoff,
    ProxyProvenance, ResolvedProxyTarget, RunLedger, SessionCompatibilityKey, SharedRunLedger,
    SurfacedHandoff, TransitionAbort, TransitionRecord, commit_proxy, commit_proxy_in_context,
};
pub use darkmatter::markdown::compose::shell_expansion::{ShellCommandOrigin, ShellExpansionError};
pub use error::{
    ActionExprError, CompositionError, DroppedOptional, DroppedOptionalSource, DroppedOptionalStage,
    FileReferenceContext, InteractiveShape, LOOP_RATE_LIMITED_EXIT_CODE, LoopExpressionCause,
    MarkdownLoadCause, MissingProperty,
    SequenceExpressionCause, SequenceLoadCause, SequenceMissingPropertiesStep,
    SequenceSelectionFailure, SequenceShellCause, ShellApprovalFailure, TextFormat,
};
pub use file_detail::{FileDetail, extract_markdown_detail, extract_yaml_sequence_detail};
pub use frontmatter_excerpt::FrontmatterExcerpt;
pub use launch_workspace::{LaunchWorkspaceContext, PackageContext};
#[allow(deprecated)]
pub use lifecycle::{
    DefaultLifecycleEmitter, LIFECYCLE_EVENT_KEYS, LifecycleConfig, LifecycleEmitter,
    LifecycleNotification, LifecycleRunGuard, LifecycleRuntimeContext, LifecycleRuntimeState,
    LifecycleSignal, emit_lifecycle_signal, parse_lifecycle_config,
};
pub use lifecycle_actions::{
    CommunicationAction, CommunicationChannel, ExpressionFunctionAction, LifecycleAction,
    LifecycleActionKind, LifecycleControlAction, LifecycleStackItem, RetryBackoff, ShellAction,
    SideEffectAction, is_known_side_effect, side_effect_signature,
};
pub use lifecycle_context::{
    LifecycleCurrent, LifecycleErrorInfo, LifecycleTiming, lifecycle_injected_globals,
};
pub use lifecycle_control::{
    ControlDispatch, MAX_PROXY_HOPS, compute_backoff_delay, control_budget_for, decide_control,
    parse_delay, proxy_handoff_allowed, proxy_path_identity, resolve_proxy_target,
    resolve_proxy_target_in_context,
};
pub use lifecycle_executor::{
    LifecycleEventOutcome, LifecycleExprError, ShellRunError, ShellRunner, StackControl,
    StackExecutionContext, SystemShellRunner,
};
pub use lifecycle::runtime::{
    IterationSummarySignals, LifecycleTransitionAbort, LifecycleTransitionDecision,
    LifecycleTransitionError, LifecycleTransitionInput, LifecycleCatchExecution,
    LifecycleCatchProtocol, LifecycleCatchResult, LifecycleCatchState, LifecycleCatchStep,
    decide_lifecycle_transition,
};
pub use looping::{
    extract_control_variables, resolve_fail_fast_from_env, resolve_loop_config,
    resolve_max_iterations_from_env, resolve_pause_reset_margin_from_env,
};
pub use looping::{
    DEFAULT_MAX_ITERATIONS, LoopExecutionOptions, LoopExecutionResult, LoopIterationContext,
    LoopIterationOutput, LoopSeed, build_loop_seed, build_loop_seed_with_lifecycle, execute_loop,
    execute_loop_with_config, execute_loop_with_lifecycle,
};
pub use looping::{LoopAmbient, LoopExpressionLookup, evaluate_condition};
pub use mismatch::{capture_frontmatter_yaml, is_inline_sequence_mismatch};
pub use runtime_state::{
    OUTPUTS_KEY, RuntimeMutationError, RuntimeSnapshot, RuntimeState, layered_set_overrides,
    trim_transport_newline, with_initialized_outputs,
};
pub use preflight::{
    PreFlightResult, resolve_graph_shell_approvals, resolve_lifecycle_shell_approvals,
    resolve_shell_approvals,
};
pub use hints::{parse_interactive_hint, parse_selection_hints_from_frontmatter};
pub use prepare::{
    DocumentEntryReason, DocumentPreparation, LoopOwnership, PrepareOptions, PreparationStages,
    PromptSource, SchemaStage, SourceBasis, bind_agent_workspace, prepare_direct,
    prepare_document,
    prepare_inline, preflight_document_shell,
};
pub use resolve::{
    build_prompt_reference, capture_file_resolution_context, derive_request_context_for_source,
    enrich_composition_source_load_error, enrich_composition_source_load_error_in_context,
    is_yaml_source,
    load_yaml_document, prompt_magic_roots, reload_composition_source,
    resolve_composition_source, resolve_composition_source_in_context,
    validate_file_permissions, without_formal_sequence_keys,
};
pub use schema::{
    InteractiveSchemaOptions, PreValidatedSchema, PropertyState, PropertyStatus,
    SchemaStatusReport, build_schema_status_report, drop_invalid_optionals,
    pre_validate_schema, prepare_direct_with_schema, prepare_direct_with_schema_and_prompt,
    prepare_inline_with_schema,
};
pub use select::{
    build_candidate_set, build_installed_snapshot, build_picker_plan, build_picker_plan_with_hints,
    classify_agent_resolution, detect_installed_providers, resolve_model,
    resolve_model_with_catalog, resolve_model_with_hints,
    resolve_target_non_tty, resolve_target_non_tty_with_catalog, resolve_target_non_tty_with_hints,
    select_provider,
};
pub use sequence::{
    ExecutableField, ExternalTaskRef, GroupRef, OutputEntry, RuntimeMutation, SequencePlan,
    SequenceReference, SequenceSource, SequenceSourceOptions, SequenceSourceSpec, SequenceStep,
    SequenceStepOverlay, ShellSourceRunner, SourceOperator, StepExecutable, StepState, Strictness,
    build_step_overlay, resolve_sequence_plan, resolve_sequence_plan_with,
};
pub use sequence::preflight::{
    DiscoveredCommand, GroupExecution, PreflightAction, PreflightGraph, PreflightGroup,
    PreflightStep, PreflightTask, PromptDocument, build_preflight_graph,
    build_preflight_graph_with_context, build_preflight_graph_with_context_and_resolution,
    build_preflight_graph_with_invocation, reject_non_sequence_kind,
};
pub use sequence::task::{
    DEFAULT_COMMAND_TIMEOUT, PromptRunOutcome, PromptTaskRequest, PromptTaskRunner, RunawayTrip,
    RunawayTripKind, ShellCommandOutput, SystemTaskShell, TaskDiagnostic, TaskExecution,
    TaskOutcome, TaskShellError, TaskShellRunner, TaskStage, TaskStatus, UnavailablePromptRunner,
};
pub use types::{
    AgentHint, AgentResolutionState, AmbientVariable, CallerInputLayers, CompositionClosurePlan,
    CompositionExecutionRequest, CompositionMode, EffectiveSelectionHints, InlineClosurePlan,
    InstalledProviderSnapshot, LoopAction, LoopCondition, LoopConfig, ModelHint,
    ModelResolutionReason, OnRateLimit, OutputFormat, PickerInfluence, PreparedComposition,
    ProviderPickerOption, ProviderPickerPlan, ProviderResolutionReason, ResolutionMode,
    ResolvedCompositionSource, ResolvedExecutionTarget, ResolvedSessionInteractivity,
    SelectedProvider, SelectionReason, SessionInteractivitySource, SequenceExecutionOptions,
    SequenceRunSummary, SequenceStepDraft, SequenceStepResult, SequenceTaskResult,
    SharedApprovalCache,
};

/// Builds Darkmatter's local expression context from the request snapshot for
/// the document that authored the expression.
pub fn document_expression_resolution_context(
    source_path: &Path,
    prepared_context: Option<&darkmatter::markdown::compose::ComposeContext>,
    file_resolution_context: Option<&biscuit_file::FileResolutionContext>,
    file_ref_fallback_dir: Option<&Path>,
) -> darkmatter::markdown::compose::expression::ResolutionContext {
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
            .unwrap_or_else(|| Path::new("."));
        capture_compatibility_context_for_content(base, "")
    });
    let mut options = darkmatter::markdown::compose::ComposeOptions::new_with_context(context)
        .with_source_file(source_path);
    if let Some(snapshot) = file_resolution_context {
        options = options.with_file_resolution_context(snapshot.clone());
    }
    if let Some(fallback) = file_ref_fallback_dir {
        options = options.with_file_ref_fallback_dir(fallback);
    }
    options.local_expression_resolution_context()
}

/// Capture demand-driven context for a caller that has no invocation snapshot.
///
/// Canonical CLI routes must use `InvocationContext::capture_launch_context`.
/// This compatibility seam preserves the library APIs that may be called
/// independently of a Claudine invocation.
#[doc(hidden)]
pub fn capture_compatibility_context_for_content(
    base_dir: &Path,
    content: &str,
) -> darkmatter::markdown::compose::ComposeContext {
    darkmatter::markdown::compose::ComposeContext::capture_for_content(base_dir, content)
}

/// Capture document requirements for a caller that has no invocation snapshot.
///
/// As with [`capture_compatibility_context_for_content`], this is not a
/// canonical CLI preparation owner.
#[doc(hidden)]
pub fn capture_compatibility_context_for_document(
    base_dir: &Path,
    document: &darkmatter::markdown::Markdown,
) -> darkmatter::markdown::compose::ComposeContext {
    darkmatter::markdown::compose::ComposeContext::capture_for_document(base_dir, document)
}
