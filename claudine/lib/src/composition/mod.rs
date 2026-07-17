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
pub mod schema;
mod select;
pub mod sequence;
mod types;

pub use agent_message::{agent_state_breakdown, invalid_agent_message};
pub use coordinator::{
    ActionLocation, ActiveDocumentState, AttemptOutcome, CommandOutputPolicy, ControlBudget,
    ControlBudgetKind, DocumentIteration, DocumentOverlay, DocumentTransition,
    EvaluatedProxyRequest, HopApproval, HopRejection, InvocationInputs, InvocationInputsDraft,
    LaunchDiscovery, LedgerMut, PreparedDocument, ProviderAttempt, ProxyHandoff, ProxyProvenance,
    ResolvedProxyTarget, RunLedger, TransitionAbort, TransitionRecord,
};
pub use darkmatter::markdown::compose::shell_expansion::{ShellCommandOrigin, ShellExpansionError};
pub use error::{
    CompositionError, DroppedOptional, DroppedOptionalSource, DroppedOptionalStage,
    InteractiveShape, LOOP_RATE_LIMITED_EXIT_CODE, MarkdownLoadCause, MissingProperty,
    SequenceLoadCause, SequenceMissingPropertiesStep, SequenceSelectionFailure, TextFormat,
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
    parse_delay, proxy_handoff_allowed, resolve_proxy_target,
};
pub use lifecycle_executor::{
    LifecycleEventOutcome, ShellRunner, StackControl, StackExecutionContext, SystemShellRunner,
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
pub use preflight::{PreFlightResult, resolve_shell_approvals};
pub use hints::{parse_interactive_hint, parse_selection_hints_from_frontmatter};
pub use prepare::{
    DocumentEntryReason, DocumentPreparation, LoopOwnership, PrepareOptions, PreparationStages,
    PromptSource, SourceBasis, bind_agent_workspace, prepare_direct, prepare_document,
    prepare_inline, preflight_document_shell,
};
pub use resolve::{
    enrich_composition_source_load_error, resolve_composition_source, validate_file_permissions,
};
pub use schema::{
    InteractiveSchemaOptions, PreValidatedSchema, PropertyState, PropertyStatus,
    SchemaStatusReport, build_schema_status_report, drop_invalid_optionals,
    pre_validate_schema, prepare_direct_with_schema, prepare_inline_with_schema,
};
pub use select::{
    build_candidate_set, build_installed_snapshot, build_picker_plan, build_picker_plan_with_hints,
    classify_agent_resolution, detect_installed_providers, resolve_model,
    resolve_model_with_catalog, resolve_model_with_hints,
    resolve_target_non_tty, resolve_target_non_tty_with_catalog, resolve_target_non_tty_with_hints,
    select_provider,
};
pub use sequence::{build_step_overlay, resolve_sequence_plan};
pub use types::{
    AgentHint, AgentResolutionState, AmbientVariable, CallerInputLayers, CompositionClosurePlan,
    CompositionExecutionRequest, CompositionMode, EffectiveSelectionHints, InlineClosurePlan,
    InstalledProviderSnapshot, LoopAction, LoopCondition, LoopConfig, ModelHint,
    ModelResolutionReason, OnRateLimit, OutputFormat, PickerInfluence, PreparedComposition,
    ProviderPickerOption, ProviderPickerPlan, ProviderResolutionReason, ResolutionMode,
    ResolvedCompositionSource, ResolvedExecutionTarget, ResolvedSessionInteractivity,
    SelectedProvider, SelectionReason, SessionInteractivitySource, SequenceExecutionOptions,
    SequencePlan, SequenceRunSummary, SequenceSource, SequenceStep, SequenceStepDraft,
    SequenceStepOverlay, SequenceStepResult, SharedApprovalCache,
};
