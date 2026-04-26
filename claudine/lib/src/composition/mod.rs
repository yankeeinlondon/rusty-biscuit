//! Composition services for inline and chained document workflows.
//!
//! This module provides the shared logic for Claudine's composition features:
//! - File reference resolution via `biscuit-file::FileReference`
//! - Prompt preparation (inline frontmatter prompt and chained document)
//! - Agent/provider selection with precedence rules
//! - Composition-specific error types

pub mod closure;
mod error;
mod guardrails;
pub mod lifecycle;
pub mod preflight;
mod prepare;
mod resolve;
mod select;
pub mod sequence;
mod types;

pub use darkmatter::markdown::compose::shell_expansion::{ShellCommandOrigin, ShellExpansionError};
pub use error::{CompositionError, SequenceSelectionFailure};
#[allow(deprecated)]
pub use lifecycle::{
    DefaultLifecycleEmitter, LifecycleConfig, LifecycleEmitter, LifecycleNotification,
    LifecycleRunGuard, LifecycleRuntimeContext, LifecycleRuntimeState, LifecycleSignal,
    emit_lifecycle_signal, parse_lifecycle_config,
};
pub use preflight::{PreFlightResult, resolve_shell_approvals};
pub use prepare::{PrepareOptions, prepare_direct, prepare_inline};
pub use resolve::{resolve_composition_source, validate_file_permissions};
pub use select::{build_candidate_set, select_provider};
pub use sequence::{build_step_overlay, resolve_sequence_plan};
pub use types::{
    AgentHint, CompositionClosurePlan, CompositionExecutionRequest, CompositionMode,
    EffectiveSelectionHints, InlineClosurePlan, InstalledProviderSnapshot, ModelHint,
    ModelResolutionReason, OutputFormat, PickerInfluence, PreparedComposition,
    ProviderPickerOption, ProviderPickerPlan, ProviderResolutionReason, ResolutionMode,
    ResolvedCompositionSource, ResolvedExecutionTarget, SelectedProvider, SelectionReason,
    SequenceExecutionOptions, SequencePlan, SequenceRunSummary, SequenceSource,
    SequenceStep, SequenceStepDraft, SequenceStepOverlay, SequenceStepResult,
    SharedApprovalCache,
};
