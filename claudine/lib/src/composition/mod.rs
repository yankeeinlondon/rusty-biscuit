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
mod error;
pub mod frontmatter_excerpt;
mod guardrails;
pub mod launch_workspace;
pub mod lifecycle;
mod lifecycle_actions;
pub mod lifecycle_context;
pub mod lifecycle_control;
pub mod lifecycle_executor;
pub mod loop_actions;
pub mod loop_config;
pub mod loop_engine;
pub mod loop_expression;
pub mod mismatch;
pub mod preflight;
mod prepare;
mod resolve;
pub mod schema_validation;
mod select;
pub mod sequence;
mod types;

pub use agent_message::{agent_state_breakdown, invalid_agent_message};
pub use darkmatter::markdown::compose::shell_expansion::{ShellCommandOrigin, ShellExpansionError};
pub use error::{
    CompositionError, DroppedOptional, DroppedOptionalSource, DroppedOptionalStage,
    InteractiveShape, LOOP_RATE_LIMITED_EXIT_CODE, MissingProperty,
    SequenceMissingPropertiesStep, SequenceSelectionFailure, TextFormat,
};
pub use frontmatter_excerpt::FrontmatterExcerpt;
pub use launch_workspace::{LaunchWorkspaceContext, PackageContext};
#[allow(deprecated)]
pub use lifecycle::{
    DefaultLifecycleEmitter, LifecycleConfig, LifecycleEmitter, LifecycleNotification,
    LifecycleRunGuard, LifecycleRuntimeContext, LifecycleRuntimeState, LifecycleSignal,
    emit_lifecycle_signal, parse_lifecycle_config,
};
pub use lifecycle_actions::{
    CommunicationAction, CommunicationChannel, ExpressionFunctionAction, LifecycleAction,
    LifecycleActionKind, LifecycleControlAction, LifecycleStackItem, RetryBackoff, ShellAction,
    SideEffectAction, is_known_side_effect, side_effect_signature,
};
pub use lifecycle_context::{
    LifecycleCurrent, LifecycleErrorInfo, LifecycleLookup, LifecycleTiming,
};
pub use lifecycle_control::{
    ControlDispatch, MAX_PROXY_HOPS, compute_backoff_delay, control_budget_for, decide_control,
    parse_delay, proxy_handoff_allowed, resolve_proxy_target,
};
pub use lifecycle_executor::{
    LifecycleEventOutcome, ShellRunner, StackControl, StackExecutionContext, SystemShellRunner,
};
pub use loop_config::{
    extract_control_variables, resolve_fail_fast_from_env, resolve_loop_config,
    resolve_max_iterations_from_env, resolve_pause_reset_margin_from_env,
};
pub use loop_engine::{
    DEFAULT_MAX_ITERATIONS, LoopExecutionOptions, LoopExecutionResult, LoopIterationContext,
    LoopIterationOutput, LoopSeed, build_loop_seed, build_loop_seed_with_lifecycle, execute_loop,
    execute_loop_with_config, execute_loop_with_lifecycle,
};
pub use loop_expression::{LoopAmbient, LoopExpressionLookup, evaluate_condition};
pub use mismatch::{capture_frontmatter_yaml, is_inline_sequence_mismatch};
pub use preflight::{PreFlightResult, resolve_shell_approvals};
pub use prepare::{
    PrepareOptions, bind_agent_workspace, parse_interactive_hint,
    parse_selection_hints_from_frontmatter, prepare_direct, prepare_inline,
};
pub use resolve::{resolve_composition_source, validate_file_permissions};
pub use schema_validation::{
    InteractiveSchemaOptions, PreValidatedSchema, PropertyState, PropertyStatus,
    SchemaStatusReport, build_schema_status_report, drop_invalid_optionals,
    pre_validate_schema, prepare_direct_with_schema, prepare_inline_with_schema,
};
pub use select::{
    build_candidate_set, build_installed_snapshot, build_picker_plan, build_picker_plan_with_hints,
    classify_agent_resolution, resolve_model, resolve_model_with_catalog, resolve_model_with_hints,
    resolve_target_non_tty, resolve_target_non_tty_with_catalog, resolve_target_non_tty_with_hints,
    select_provider,
};
pub use sequence::{build_step_overlay, resolve_sequence_plan};
pub use types::{
    AgentHint, AgentResolutionState, AmbientVariable, CompositionClosurePlan,
    CompositionExecutionRequest, CompositionMode, EffectiveSelectionHints, InlineClosurePlan,
    InstalledProviderSnapshot, LoopAction, LoopCondition, LoopConfig, ModelHint,
    ModelResolutionReason, OnRateLimit, OutputFormat, PickerInfluence, PreparedComposition,
    ProviderPickerOption, ProviderPickerPlan, ProviderResolutionReason, ResolutionMode,
    ResolvedCompositionSource, ResolvedExecutionTarget, ResolvedSessionInteractivity,
    SelectedProvider, SelectionReason, SessionInteractivitySource, SequenceExecutionOptions,
    SequencePlan, SequenceRunSummary, SequenceSource, SequenceStep, SequenceStepDraft,
    SequenceStepOverlay, SequenceStepResult, SharedApprovalCache,
};
