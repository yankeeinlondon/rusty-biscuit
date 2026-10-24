//! Lifecycle stacks: the typed action model authored in document frontmatter
//! and the parser that builds it.
//!
//! A document's lifecycle fires through eight engine slots
//! ([`LifecycleSignal`]); the host's [`registry::EventRegistry`] names the
//! events that use them (Darkmatter's `composed` is the `Start` slot, and a
//! host may call it `start`). Each event is a stack: an ordered action list, or one dictionary of verbs that expands
//! into that list in canonical order (see [`actions::LifecycleNode`]). The
//! `loop:` block's lifecycle concerns are loose verb keys or a `gate:` list;
//! its iteration controls ([`LOOP_ITERATION_KEYS`]) belong to the host.
//!
//! This module parses, statically validates, and executes. A host supplies what
//! the grammar leaves open through [`LifecycleParseOptions`] — its
//! [`registry::EventRegistry`] (which keys are lifecycle stacks, under which
//! names), the sound-effect catalog, and the profile's policy for verbs it
//! does not know — and runs one event's parsed stack through
//! [`executor::StackExecutor`].
//!
//! The runtime's host-neutral pieces live here too: the error envelope behind
//! the `err` and `timing` globals ([`envelope`], [`context`]), cancellation
//! tokens ([`cancellation`]), and the service seams a host implements —
//! communication ([`LifecycleEffects`]) and error classification
//! ([`DiagnosticCatalog`]), gathered on [`LifecycleServices`], plus the
//! executor's policy seam ([`executor::LifecycleHost`]) and shell route
//! ([`executor::ShellRunner`]).
//!
//! A run is driven either by [`run_guard::LifecycleRunGuard`] (event
//! accounting, the one terminal slot, the shell lock) over a host's
//! [`run_guard::LifecycleRunHost`], or request by request through
//! [`run::Run`], which returns each unit of work for the host to perform. Both
//! keep [`run_guard::RunAccounting`] and use the pure decisions in [`control`]
//! (control admission, retry backoff, the proxy budget) and [`runtime`] (the
//! transition decision, the catch protocol, and the container-event engine).
//!
//! ## Examples
//!
//! ```
//! use darkmatter::lifecycle::registry::EventRegistry;
//! use darkmatter::lifecycle::{
//!     LifecycleParseOptions, LifecycleSignal, UnknownVerbPolicy, parse_lifecycle_config_with,
//! };
//! use serde_json::json;
//!
//! let frontmatter = json!({ "composed": [{ "info": "Starting..." }] });
//! let options = LifecycleParseOptions {
//!     is_known_effect: &|_| false,
//!     authored: None,
//!     unknown_verbs: UnknownVerbPolicy::Reject,
//!     registry: &EventRegistry::md(),
//! };
//! let config =
//!     parse_lifecycle_config_with(&frontmatter, std::path::Path::new("doc.md"), &options).unwrap();
//! assert_eq!(config.stack(LifecycleSignal::Start).len(), 1);
//! ```

// rustfmt doesn't support let-chains yet, so nested ifs are required
#![allow(clippy::collapsible_if)]

use std::path::Path;

use biscuit_terminal::components::status::StatusState;

use crate::markdown::compose::expression::{Expr, ExpressionFinder, parse, parse_condition};

use self::actions::{
    CommunicationAction, CommunicationChannel, ExpressionFunctionAction, IgnoredAction, LifecycleAction,
    LifecycleActionKind, LifecycleControlAction, LifecycleNode, LifecyclePath, RetryBackoff, RuntimeSet,
    RuntimeSetError, ShellAction, SideEffectAction, WithOverlay, WithOverlayError, all_lifecycle_verbs,
    expression_function_signature, is_known_lifecycle_verb, rewrite_to_positional, side_effect_signature,
};
use self::error::{ActionExprError, LifecycleError};

mod action_shape;
pub mod actions;
pub mod authored_order;
pub mod cancellation;
pub mod context;
pub mod control;
pub mod duration;
pub mod effects;
pub mod envelope;
pub mod error;
pub mod executor;
pub mod md_host;
mod operands;
mod parse;
pub mod policy;
pub mod preflight;
pub mod registry;
pub mod run;
pub mod run_guard;
pub mod runtime;
pub mod schema_projection;
pub mod services;
pub mod shell_template;
mod source_map;
mod validate;

use action_shape::*;
#[doc(hidden)]
pub use action_shape::{action_value_to_expr, closest_spelling};
pub use authored_order::AuthoredOrder;
pub use context::{LifecycleCause, LifecycleErrorInfo, LifecycleEvaluationReason, LifecycleTiming};
pub use effects::{LifecycleEffects, dispatch_communication, render_prose_line, render_status_line};
pub use envelope::{
    DIAGNOSTIC_SNAPSHOT_SCHEMA_VERSION, DiagnosticCatalog, DiagnosticCause, DiagnosticSnapshot, SourceAppendix,
    SourceExcerpt,
};
pub use error::LifecycleExprError;
pub use operands::{
    ActionCall, OperandLocation, OperandVerdict, TemplateOperand, action_call, operand_form, template_operands, validate_action_operands,
    validate_action_operands_with,
    validate_expression_function_operands, validate_operands, validate_operands_with,
};
pub use services::{LifecycleService, LifecycleServices, MissingServiceError};
pub use parse::{
    LOOP_ITERATION_KEYS, LifecycleParseOptions, UnknownVerbPolicy, parse_lifecycle_config_with,
    parse_loop_concerns_with, parse_single_action_with, parse_task_action_stack_with,
    scan_removed_validation_keys,
};
#[doc(hidden)]
pub use parse::{action_type, runs_a_shell, with_mapping_shape};
#[doc(hidden)]
pub use source_map::{AuthoredValue, LifecycleSourceMap};
#[doc(hidden)]
pub use validate::{Surface, event_surfaces, walk_stack};
pub use validate::{
    collect_lifecycle_shell_commands, collect_lifecycle_shell_commands_for,
    reject_incompatible_calls, validate_no_nested_spans_in_literals, validate_no_nested_spans_in_literals_in,
};

/// Complete lifecycle configuration for a document.
///
/// Parsed from the frontmatter keys the host's registry names for each
/// [`LifecycleSignal`], and the lifecycle concerns of its iteration gate.
///
/// ## Examples
///
/// ```yaml
/// start:
///   - message: "Starting..."
/// success:
///   effect: "crowd-applause"
///   say: "Composition complete"
/// ```
#[derive(Debug, Clone, Default, PartialEq)]
pub struct LifecycleConfig {
    /// Typed, validated stacks keyed by event.
    pub stacks: LifecycleStacks,
    /// Whether `cancelled` was authored, including an empty stack. Sibling
    /// cancellation opts in by presence, independently of cleanup actions.
    pub cancelled_declared: bool,
}

/// Per-event typed stacks.
///
/// An omitted event, `[]`, and `{}` contain no actions. The configuration
/// separately retains cancellation opt-in presence. `cancelled: null` is
/// invalid; other events retain their null-as-empty behavior.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct LifecycleStacks {
    #[allow(missing_docs)]
    pub initialize: Vec<LifecycleNode>,
    #[allow(missing_docs)]
    pub start: Vec<LifecycleNode>,
    #[allow(missing_docs)]
    pub success: Vec<LifecycleNode>,
    #[allow(missing_docs)]
    pub blocked: Vec<LifecycleNode>,
    #[allow(missing_docs)]
    pub failure: Vec<LifecycleNode>,
    #[allow(missing_docs)]
    pub finalize: Vec<LifecycleNode>,
    #[allow(missing_docs)]
    pub cancelled: Vec<LifecycleNode>,
    /// The `loop:` block's lifecycle concerns: its loose verb keys or `gate:`.
    pub loop_gate: Vec<LifecycleNode>,
}

/// Lifecycle event signal types.
///
/// The eight lifecycle signals, in deterministic iteration order:
/// `Initialize`, `Start`, `Success`, `Blocked`, `Failure`, `Finalize`, `Loop`,
/// `Cancelled`. This order is used by validators that walk every event
/// surface (nested-span scan, `err` availability check) and is exposed via
/// [`Self::ALL`].
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum LifecycleSignal {
    /// Prompt file has been identified and frontmatter has parsed, before
    /// user `$schema` validation and shell pre-flight checks.
    Initialize,

    /// Pre-flight checks have all passed; about to invoke the agent.
    Start,

    /// Agentic loop completed without error.
    Success,

    /// Pre-flight checks failed; agent will not be invoked.
    Blocked,

    /// Agentic loop returned an error.
    Failure,

    /// Once per iteration, immediately after the terminal `success`/
    /// `failure`, `blocked`, or `cancelled` completes.
    Finalize,

    /// Post-`finalize` "iterate again?" gate; evaluates the document's
    /// `loop` while/until condition.
    Loop,

    /// Execution was interrupted or a running leg opted into sibling cancellation.
    Cancelled,
}

impl LifecycleSignal {
    /// All signals in deterministic iteration order.
    ///
    /// Used by validators that walk every event surface. New variants
    /// should be appended to the end so existing diagnostic order is
    /// preserved.
    pub const ALL: [Self; 8] = [
        Self::Initialize,
        Self::Start,
        Self::Success,
        Self::Blocked,
        Self::Failure,
        Self::Finalize,
        Self::Loop,
        Self::Cancelled,
    ];

    /// Returns the frontmatter property name for this signal.
    ///
    /// ## Examples
    ///
    /// ```
    /// # use darkmatter::lifecycle::LifecycleSignal;
    /// assert_eq!(LifecycleSignal::Start.property_name(), "start");
    /// assert_eq!(LifecycleSignal::Loop.property_name(), "loop");
    /// ```
    pub fn property_name(&self) -> &'static str {
        match self {
            Self::Initialize => "initialize",
            Self::Start => "start",
            Self::Success => "success",
            Self::Blocked => "blocked",
            Self::Failure => "failure",
            Self::Finalize => "finalize",
            Self::Loop => "loop",
            Self::Cancelled => "cancelled",
        }
    }

    /// Returns the status state for this signal.
    ///
    /// ## Examples
    ///
    /// ```
    /// # use darkmatter::lifecycle::LifecycleSignal;
    /// # use biscuit_terminal::components::status::StatusState;
    /// assert_eq!(LifecycleSignal::Start.status_state(), StatusState::Info);
    /// assert_eq!(LifecycleSignal::Success.status_state(), StatusState::Success);
    /// assert_eq!(LifecycleSignal::Blocked.status_state(), StatusState::Error);
    /// assert_eq!(LifecycleSignal::Failure.status_state(), StatusState::Error);
    /// ```
    pub fn status_state(&self) -> StatusState {
        match self {
            Self::Initialize => StatusState::Info,
            Self::Start => StatusState::Info,
            Self::Success => StatusState::Success,
            Self::Blocked | Self::Failure | Self::Cancelled => StatusState::Error,
            Self::Finalize => StatusState::Info,
            Self::Loop => StatusState::Info,
        }
    }

    /// Whether this event can ever observe an error and therefore may
    /// legitimately reference the lifecycle-stack-only `err` global.
    ///
    /// Per the spec's `err` static-scan rule:
    /// - `blocked`, `failure`, and `cancelled` always carry an error.
    /// - `finalize` *optionally* carries an error (success path: no error;
    ///   failure path: error present).
    /// - `initialize`, `start`, `success`, `loop` never carry an error.
    pub const fn can_carry_error(self) -> bool {
        matches!(self, Self::Blocked | Self::Failure | Self::Cancelled | Self::Finalize)
    }

    /// Whether an *unintentional* action error during this event's stack
    /// must route the run to the `failure` event.
    ///
    /// Per the spec's action error-propagation table, setup-phase events
    /// (`initialize`, `start`, `blocked`) propagate an errored action to
    /// `failure` so the agent is never invoked with a broken environment.
    /// Terminal-phase events (`success`, `failure`, `cancelled`, `finalize`, `loop`) log
    /// the error but leave the composition outcome unchanged.
    ///
    /// This governs only unintentional action errors. The explicit `Error`
    /// lifecycle action is a deliberate author choice and follows the
    /// separate "Where valid" transition table.
    pub const fn routes_action_error_to_failure(self) -> bool {
        matches!(self, Self::Initialize | Self::Start | Self::Blocked)
    }
}

impl LifecycleConfig {
    /// Returns the typed stack for a given signal; empty when the event is
    /// absent.
    ///
    /// For [`LifecycleSignal::Loop`] this is the `loop:` block's lifecycle
    /// concerns.
    ///
    /// ## Examples
    ///
    /// ```
    /// # use darkmatter::lifecycle::{LifecycleConfig, LifecycleSignal};
    /// let config = LifecycleConfig::default();
    /// assert!(config.stack(LifecycleSignal::Start).is_empty());
    /// ```
    pub fn stack(&self, signal: LifecycleSignal) -> &[LifecycleNode] {
        match signal {
            LifecycleSignal::Initialize => &self.stacks.initialize,
            LifecycleSignal::Start => &self.stacks.start,
            LifecycleSignal::Success => &self.stacks.success,
            LifecycleSignal::Blocked => &self.stacks.blocked,
            LifecycleSignal::Failure => &self.stacks.failure,
            LifecycleSignal::Finalize => &self.stacks.finalize,
            LifecycleSignal::Loop => &self.stacks.loop_gate,
            LifecycleSignal::Cancelled => &self.stacks.cancelled,
        }
    }

    /// Every `prep` target written as a literal, in event and stack order:
    /// the targets a dry run lists without running. A target written as an
    /// interpolation is known only when its event fires.
    pub fn literal_prep_targets(&self) -> Vec<String> {
        let mut targets = Vec::new();
        for signal in LifecycleSignal::ALL {
            actions::for_each_action(self.stack(signal), &mut |action| {
                if let LifecycleActionKind::LifecycleControl(actions::LifecycleControlAction::Prep {
                    target: Expr::StringLiteral(target),
                    ..
                }) = &action.kind
                    && ExpressionFinder::find_all_plain(target).is_empty()
                    && !targets.contains(target)
                {
                    targets.push(target.clone());
                }
            });
        }
        targets
    }

    /// Mutable access to the typed stack for a given signal.
    ///
    /// Used by the pre-flight shell resolution pass to stamp resolved
    /// command bytes back into `ShellAction::command` / `on_error` so the
    /// approved command equals the executed command.
    pub fn stack_mut(&mut self, signal: LifecycleSignal) -> &mut Vec<LifecycleNode> {
        match signal {
            LifecycleSignal::Initialize => &mut self.stacks.initialize,
            LifecycleSignal::Start => &mut self.stacks.start,
            LifecycleSignal::Success => &mut self.stacks.success,
            LifecycleSignal::Blocked => &mut self.stacks.blocked,
            LifecycleSignal::Failure => &mut self.stacks.failure,
            LifecycleSignal::Finalize => &mut self.stacks.finalize,
            LifecycleSignal::Loop => &mut self.stacks.loop_gate,
            LifecycleSignal::Cancelled => &mut self.stacks.cancelled,
        }
    }

    /// Returns `true` when there are no actions or explicit cancellation opt-in.
    ///
    /// ## Examples
    ///
    /// ```
    /// # use darkmatter::lifecycle::LifecycleConfig;
    /// let config = LifecycleConfig::default();
    /// assert!(config.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        !self.cancelled_declared && LifecycleSignal::ALL
            .iter()
            .all(|signal| self.stack(*signal).is_empty())
    }
}

#[cfg(test)]
mod tests;
